use ncc_ast::SourceExecutionContract;

pub const RAW_OPTIMIZER_SCHEMA: &str = "nc.optimizer.raw.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawAliasClass {
    NoAlias,
    MustAlias,
    MayAlias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawMemoryIdentity {
    Unknown,
    Escaped,
    SealedStack { frame: u64, slot: u64 },
    SealedObject { symbol: String },
    Absolute(u128),
    Mmio(u128),
    Dma { domain: u32, iova: u128 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMemoryAccess {
    pub identity: RawMemoryIdentity,
    pub offset: i128,
    pub bytes: u128,
    pub write: bool,
    pub volatile: bool,
    pub atomic: bool,
}

impl RawMemoryAccess {
    pub fn ordinary(identity: RawMemoryIdentity, offset: i128, bytes: u128, write: bool) -> Self {
        Self {
            identity,
            offset,
            bytes,
            write,
            volatile: false,
            atomic: false,
        }
    }

    pub const fn is_observable_barrier(&self) -> bool {
        self.volatile
            || self.atomic
            || matches!(
                self.identity,
                RawMemoryIdentity::Mmio(_) | RawMemoryIdentity::Dma { .. }
            )
    }

    pub const fn has_material_range_identity(&self) -> bool {
        matches!(
            self.identity,
            RawMemoryIdentity::SealedStack { .. }
                | RawMemoryIdentity::SealedObject { .. }
                | RawMemoryIdentity::Absolute(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOptimizerContract {
    pub schema: &'static str,
    pub mode: &'static str,
    pub default_alias: RawAliasClass,
    pub emit_tbaa: bool,
    pub emit_noalias: bool,
    pub emit_alias_scopes: bool,
    pub raw_gep_inbounds: bool,
    pub hidden_runtime_alias_checks: bool,
    pub preserve_volatile: bool,
    pub preserve_atomics: bool,
    pub preserve_memmove_overlap: bool,
    pub explicit_simd_permitted: bool,
}

impl RawOptimizerContract {
    pub fn for_source(contract: &SourceExecutionContract) -> Self {
        debug_assert!(contract.is_manual_raw());
        Self {
            schema: RAW_OPTIMIZER_SCHEMA,
            mode: contract.mode.as_str(),
            default_alias: RawAliasClass::MayAlias,
            emit_tbaa: false,
            emit_noalias: false,
            emit_alias_scopes: false,
            raw_gep_inbounds: false,
            hidden_runtime_alias_checks: false,
            preserve_volatile: true,
            preserve_atomics: true,
            preserve_memmove_overlap: true,
            explicit_simd_permitted: true,
        }
    }

    pub fn classify(&self, left: &RawMemoryAccess, right: &RawMemoryAccess) -> RawAliasClass {
        if left.is_observable_barrier() || right.is_observable_barrier() {
            return RawAliasClass::MayAlias;
        }

        match (&left.identity, &right.identity) {
            (
                RawMemoryIdentity::SealedStack {
                    frame: lf,
                    slot: ls,
                },
                RawMemoryIdentity::SealedStack {
                    frame: rf,
                    slot: rs,
                },
            ) if lf != rf || ls != rs => RawAliasClass::NoAlias,
            (
                RawMemoryIdentity::SealedObject {
                    symbol: left_symbol,
                },
                RawMemoryIdentity::SealedObject {
                    symbol: right_symbol,
                },
            ) if left_symbol != right_symbol => RawAliasClass::NoAlias,
            (RawMemoryIdentity::Absolute(left_base), RawMemoryIdentity::Absolute(right_base)) => {
                classify_numeric_ranges(*left_base, left, *right_base, right)
            }
            (
                RawMemoryIdentity::SealedStack {
                    frame: lf,
                    slot: ls,
                },
                RawMemoryIdentity::SealedStack {
                    frame: rf,
                    slot: rs,
                },
            ) if lf == rf && ls == rs => classify_relative_ranges(left, right),
            (
                RawMemoryIdentity::SealedObject {
                    symbol: left_symbol,
                },
                RawMemoryIdentity::SealedObject {
                    symbol: right_symbol,
                },
            ) if left_symbol == right_symbol => classify_relative_ranges(left, right),
            _ => self.default_alias,
        }
    }

    pub fn can_forward_store_to_load(
        &self,
        store: &RawMemoryAccess,
        load: &RawMemoryAccess,
    ) -> bool {
        store.write
            && !load.write
            && !store.is_observable_barrier()
            && !load.is_observable_barrier()
            && self.classify(store, load) == RawAliasClass::MustAlias
    }

    pub fn can_eliminate_overwritten_store(
        &self,
        previous: &RawMemoryAccess,
        replacement: &RawMemoryAccess,
    ) -> bool {
        previous.write
            && replacement.write
            && !previous.is_observable_barrier()
            && !replacement.is_observable_barrier()
            && self.classify(previous, replacement) == RawAliasClass::MustAlias
    }

    pub fn can_vectorize_without_hidden_checks(
        &self,
        accesses: &[RawMemoryAccess],
        loop_carried_scalar_dependency: bool,
        iteration_ranges_proven_disjoint: bool,
    ) -> bool {
        if loop_carried_scalar_dependency
            || !iteration_ranges_proven_disjoint
            || accesses.iter().any(RawMemoryAccess::is_observable_barrier)
            || accesses
                .iter()
                .any(|access| !access.has_material_range_identity())
        {
            return false;
        }
        for (index, left) in accesses.iter().enumerate() {
            for right in accesses.iter().skip(index + 1) {
                if !(left.write || right.write) {
                    continue;
                }
                if self.classify(left, right) != RawAliasClass::NoAlias {
                    return false;
                }
            }
        }
        true
    }
}

fn classify_relative_ranges(left: &RawMemoryAccess, right: &RawMemoryAccess) -> RawAliasClass {
    let Some((left_start, left_end)) = relative_range(left) else {
        return RawAliasClass::MayAlias;
    };
    let Some((right_start, right_end)) = relative_range(right) else {
        return RawAliasClass::MayAlias;
    };
    if left_start == right_start && left_end == right_end {
        RawAliasClass::MustAlias
    } else if left_end <= right_start || right_end <= left_start {
        RawAliasClass::NoAlias
    } else {
        RawAliasClass::MayAlias
    }
}

fn classify_numeric_ranges(
    left_base: u128,
    left: &RawMemoryAccess,
    right_base: u128,
    right: &RawMemoryAccess,
) -> RawAliasClass {
    let Some(left_start) = add_signed(left_base, left.offset) else {
        return RawAliasClass::MayAlias;
    };
    let Some(right_start) = add_signed(right_base, right.offset) else {
        return RawAliasClass::MayAlias;
    };
    let Some(left_end) = left_start.checked_add(left.bytes.max(1)) else {
        return RawAliasClass::MayAlias;
    };
    let Some(right_end) = right_start.checked_add(right.bytes.max(1)) else {
        return RawAliasClass::MayAlias;
    };
    if left_start == right_start && left_end == right_end {
        RawAliasClass::MustAlias
    } else if left_end <= right_start || right_end <= left_start {
        RawAliasClass::NoAlias
    } else {
        RawAliasClass::MayAlias
    }
}

fn relative_range(access: &RawMemoryAccess) -> Option<(i128, i128)> {
    let bytes = i128::try_from(access.bytes.max(1)).ok()?;
    Some((access.offset, access.offset.checked_add(bytes)?))
}

fn add_signed(base: u128, offset: i128) -> Option<u128> {
    if offset >= 0 {
        base.checked_add(offset as u128)
    } else {
        base.checked_sub(offset.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> RawOptimizerContract {
        RawOptimizerContract::for_source(&SourceExecutionContract::manual_raw())
    }

    #[test]
    fn raw_unknown_escaped_mmio_dma_and_volatile_never_gain_noalias() {
        let policy = policy();
        let identities = [
            RawMemoryIdentity::Unknown,
            RawMemoryIdentity::Escaped,
            RawMemoryIdentity::Mmio(0xfee0_0000),
            RawMemoryIdentity::Dma {
                domain: 1,
                iova: 0x1000,
            },
        ];
        for identity in identities {
            let left = RawMemoryAccess::ordinary(identity, 0, 8, true);
            let right = RawMemoryAccess::ordinary(RawMemoryIdentity::Unknown, 0, 8, false);
            assert_eq!(policy.classify(&left, &right), RawAliasClass::MayAlias);
        }
        let dma = RawMemoryAccess::ordinary(
            RawMemoryIdentity::Dma {
                domain: 7,
                iova: 0x4000,
            },
            0,
            64,
            true,
        );
        assert!(dma.is_observable_barrier());
        let mut volatile =
            RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x1000), 0, 8, true);
        volatile.volatile = true;
        let ordinary = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x2000), 0, 8, false);
        assert_eq!(
            policy.classify(&volatile, &ordinary),
            RawAliasClass::MayAlias
        );
    }

    #[test]
    fn only_materially_sealed_or_numeric_disjoint_ranges_gain_noalias() {
        let policy = policy();
        let a = RawMemoryAccess::ordinary(
            RawMemoryIdentity::SealedObject { symbol: "a".into() },
            0,
            8,
            true,
        );
        let b = RawMemoryAccess::ordinary(
            RawMemoryIdentity::SealedObject { symbol: "b".into() },
            0,
            8,
            false,
        );
        assert_eq!(policy.classify(&a, &b), RawAliasClass::NoAlias);

        let left = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x1000), 0, 16, true);
        let right = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x1008), 0, 16, false);
        assert_eq!(policy.classify(&left, &right), RawAliasClass::MayAlias);
        let far = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x2000), 0, 16, false);
        assert_eq!(policy.classify(&left, &far), RawAliasClass::NoAlias);
    }

    #[test]
    fn dce_forwarding_and_vectorization_require_exact_proofs() {
        let policy = policy();
        let store = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x1000), 0, 8, true);
        let load = RawMemoryAccess::ordinary(RawMemoryIdentity::Absolute(0x1000), 0, 8, false);
        assert!(policy.can_forward_store_to_load(&store, &load));
        assert!(policy.can_eliminate_overwritten_store(&store, &store));

        let escaped = RawMemoryAccess::ordinary(RawMemoryIdentity::Escaped, 0, 8, true);
        assert!(!policy.can_forward_store_to_load(&escaped, &load));
        assert!(!policy.can_eliminate_overwritten_store(&escaped, &escaped));
        assert!(!policy.can_vectorize_without_hidden_checks(&[escaped, load], false, true));
        assert!(!policy.can_vectorize_without_hidden_checks(&[store.clone()], false, false));
        assert!(policy.can_vectorize_without_hidden_checks(&[store], false, true));
    }

    #[test]
    fn llvm_raw_contract_forbids_provenance_metadata_and_hidden_versioning() {
        let policy = policy();
        assert_eq!(policy.schema, RAW_OPTIMIZER_SCHEMA);
        assert!(!policy.emit_tbaa);
        assert!(!policy.emit_noalias);
        assert!(!policy.emit_alias_scopes);
        assert!(!policy.raw_gep_inbounds);
        assert!(!policy.hidden_runtime_alias_checks);
        assert!(policy.preserve_volatile);
        assert!(policy.preserve_atomics);
        assert!(policy.preserve_memmove_overlap);
        let pipeline = crate::mct_llvm_pass_pipeline(crate::MctLlvmOptimizationLevelIR::O3);
        for required in [
            "raw-aa-mayalias-default",
            "raw-tbaa-noalias-suppression",
            "raw-observable-memory-barriers",
        ] {
            assert!(pipeline
                .passes
                .iter()
                .any(|pass| pass.name == required && pass.enabled));
        }
    }
}
