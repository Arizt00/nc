use ncc_ast::{validate_atomic_order, AtomicOperation, MemoryOrder};

use crate::{
    encode_backend_atomic_rmw_aarch64_ordered, encode_backend_atomic_rmw_riscv,
    encode_backend_atomic_rmw_x86_64, BackendAtomicOrderingIR, BackendAtomicRmwOpIR,
    ScalarMemWidthIR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicTarget {
    X86_64,
    AArch64,
    RiscV32,
    RiscV64,
}

impl AtomicTarget {
    pub fn parse(target: &str) -> Option<Self> {
        let target = target.trim().to_ascii_lowercase();
        if target.starts_with("x86_64") || target.starts_with("amd64") {
            Some(Self::X86_64)
        } else if target.starts_with("aarch64") || target.starts_with("arm64") {
            Some(Self::AArch64)
        } else if target.starts_with("riscv32") {
            Some(Self::RiscV32)
        } else if target.starts_with("riscv64") {
            Some(Self::RiscV64)
        } else {
            None
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::AArch64 => "aarch64",
            Self::RiscV32 => "riscv32",
            Self::RiscV64 => "riscv64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicLoweringStrategy {
    NativeInline,
    CompilerOnly,
    SpinWait,
    EventWait,
    EventNotify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicPrimitiveEncoding {
    pub target: AtomicTarget,
    pub operation: AtomicOperation,
    pub order: Option<MemoryOrder>,
    pub mnemonic: &'static str,
    pub bytes: Vec<u8>,
    pub strategy: AtomicLoweringStrategy,
    pub compiler_barrier: bool,
}

pub fn lower_atomic_primitive(
    target: AtomicTarget,
    operation: AtomicOperation,
    order: Option<MemoryOrder>,
) -> Result<AtomicPrimitiveEncoding, &'static str> {
    match operation {
        AtomicOperation::NotifyOne
        | AtomicOperation::NotifyAll
        | AtomicOperation::VolatileLoad
        | AtomicOperation::VolatileStore => {
            if order.is_some() {
                return Err("E_ATOMIC_NOTIFY_ORDER_001");
            }
        }
        _ => {
            let order = order.ok_or("E_ATOMIC_ORDER_REQUIRED_001")?;
            validate_atomic_order(operation, order)?;
        }
    }

    let order = order.unwrap_or(MemoryOrder::Relaxed);
    let mut row = match target {
        AtomicTarget::X86_64 => lower_x86(operation, order),
        AtomicTarget::AArch64 => lower_aarch64(operation, order),
        AtomicTarget::RiscV32 => lower_riscv(operation, order, false),
        AtomicTarget::RiscV64 => lower_riscv(operation, order, true),
    }?;
    row.order = match operation {
        AtomicOperation::NotifyOne
        | AtomicOperation::NotifyAll
        | AtomicOperation::VolatileLoad
        | AtomicOperation::VolatileStore => None,
        _ => Some(order),
    };
    Ok(row)
}

fn row(
    target: AtomicTarget,
    operation: AtomicOperation,
    mnemonic: &'static str,
    bytes: Vec<u8>,
    strategy: AtomicLoweringStrategy,
    compiler_barrier: bool,
) -> AtomicPrimitiveEncoding {
    AtomicPrimitiveEncoding {
        target,
        operation,
        order: None,
        mnemonic,
        bytes,
        strategy,
        compiler_barrier,
    }
}

fn lower_x86(
    operation: AtomicOperation,
    order: MemoryOrder,
) -> Result<AtomicPrimitiveEncoding, &'static str> {
    let target = AtomicTarget::X86_64;
    if let Some(op) = backend_rmw_op(operation) {
        let bytes = encode_backend_atomic_rmw_x86_64(op, ScalarMemWidthIR::Qword)
            .ok_or("E_ATOMIC_RMW_WIDTH_001")?;
        return Ok(row(
            target,
            operation,
            backend_rmw_mnemonic(op, target),
            bytes,
            AtomicLoweringStrategy::NativeInline,
            true,
        ));
    }
    let result = match operation {
        AtomicOperation::Load => row(
            target,
            operation,
            "mov rax, [rdi]",
            vec![0x48, 0x8B, 0x07],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Store if order == MemoryOrder::SeqCst => row(
            target,
            operation,
            "xchg [rdi], rsi",
            vec![0x48, 0x87, 0x37],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Store => row(
            target,
            operation,
            "mov [rdi], rsi",
            vec![0x48, 0x89, 0x37],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Fence if order == MemoryOrder::SeqCst => row(
            target,
            operation,
            "mfence",
            vec![0x0F, 0xAE, 0xF0],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Fence | AtomicOperation::CompilerFence => row(
            target,
            operation,
            "compiler barrier",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::Wait => row(
            target,
            operation,
            "load/cmp/pause wait loop",
            vec![
                0x48, 0x8B, 0x07, // mov rax, [rdi]
                0x48, 0x39, 0xF0, // cmp rax, rsi
                0x75, 0x04, // jne done
                0xF3, 0x90, // pause
                0xEB, 0xF4, // jmp loop
            ],
            AtomicLoweringStrategy::SpinWait,
            true,
        ),
        AtomicOperation::NotifyOne | AtomicOperation::NotifyAll => row(
            target,
            operation,
            "spin-wait publish",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::VolatileLoad => row(
            target,
            operation,
            "mov rax, [rdi]",
            vec![0x48, 0x8B, 0x07],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::VolatileStore => row(
            target,
            operation,
            "mov [rdi], rsi",
            vec![0x48, 0x89, 0x37],
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        _ => return Err("E_ATOMIC_PRIMITIVE_RMW_ROUTE_001"),
    };
    Ok(result)
}

fn lower_aarch64(
    operation: AtomicOperation,
    order: MemoryOrder,
) -> Result<AtomicPrimitiveEncoding, &'static str> {
    let target = AtomicTarget::AArch64;
    if let Some(op) = backend_rmw_op(operation) {
        let mut bytes = encode_backend_atomic_rmw_aarch64_ordered(
            op,
            ScalarMemWidthIR::Qword,
            backend_order(order),
        )
        .ok_or("E_ATOMIC_RMW_WIDTH_001")?;
        if order == MemoryOrder::SeqCst {
            bytes.extend(word(0xD5033BBF)); // dmb ish
        }
        return Ok(row(
            target,
            operation,
            backend_rmw_mnemonic(op, target),
            bytes,
            AtomicLoweringStrategy::NativeInline,
            true,
        ));
    }
    let result = match operation {
        AtomicOperation::Load if order.has_acquire() => row(
            target,
            operation,
            "ldar x0, [x0]",
            word(0xC8DFFC00),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Load => row(
            target,
            operation,
            "ldr x0, [x0]",
            word(0xF9400000),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Store if order.has_release() => row(
            target,
            operation,
            "stlr x1, [x0]",
            word(0xC89FFC01),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Store => row(
            target,
            operation,
            "str x1, [x0]",
            word(0xF9000001),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Fence if order == MemoryOrder::Acquire => row(
            target,
            operation,
            "dmb ishld",
            word(0xD50339BF),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Fence if order == MemoryOrder::Release => row(
            target,
            operation,
            "dmb ishst",
            word(0xD5033ABF),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::Fence if order == MemoryOrder::Relaxed => row(
            target,
            operation,
            "compiler barrier",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::Fence => row(
            target,
            operation,
            "dmb ish",
            word(0xD5033BBF),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::CompilerFence => row(
            target,
            operation,
            "compiler barrier",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::Wait => row(
            target,
            operation,
            "sevl/wfe/ldar/cmp wait loop",
            words(&[
                0xD50320BF, // sevl: make the first wfe non-blocking
                0xD503205F, // loop: wfe
                0xC8DFFC02, // ldar x2, [x0]
                0xEB01005F, // cmp x2, x1
                0x54FFFFA0, // b.eq loop
            ]),
            AtomicLoweringStrategy::EventWait,
            true,
        ),
        AtomicOperation::NotifyOne | AtomicOperation::NotifyAll => row(
            target,
            operation,
            "sev",
            word(0xD503209F),
            AtomicLoweringStrategy::EventNotify,
            true,
        ),
        AtomicOperation::VolatileLoad => row(
            target,
            operation,
            "ldr x0, [x0]",
            word(0xF9400000),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::VolatileStore => row(
            target,
            operation,
            "str x1, [x0]",
            word(0xF9000001),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        _ => return Err("E_ATOMIC_PRIMITIVE_RMW_ROUTE_001"),
    };
    Ok(result)
}

fn lower_riscv(
    operation: AtomicOperation,
    order: MemoryOrder,
    rv64: bool,
) -> Result<AtomicPrimitiveEncoding, &'static str> {
    let target = if rv64 {
        AtomicTarget::RiscV64
    } else {
        AtomicTarget::RiscV32
    };
    if let Some(op) = backend_rmw_op(operation) {
        let width = if rv64 {
            ScalarMemWidthIR::Qword
        } else {
            ScalarMemWidthIR::Dword
        };
        let bytes = encode_backend_atomic_rmw_riscv(op, width, backend_order(order))
            .ok_or("E_ATOMIC_RMW_WIDTH_001")?;
        return Ok(row(
            target,
            operation,
            backend_rmw_mnemonic(op, target),
            bytes,
            AtomicLoweringStrategy::NativeInline,
            true,
        ));
    }
    let load = if rv64 { 0x00053503 } else { 0x00052503 };
    let store = if rv64 { 0x00B53023 } else { 0x00B52023 };
    let fence_acquire = 0x0230000F;
    let fence_release = 0x0310000F;
    let fence_full = 0x0330000F;
    let mut bytes = Vec::new();
    let result = match operation {
        AtomicOperation::Load => {
            bytes.extend(word(load));
            if order.has_acquire() {
                bytes.extend(word(if order == MemoryOrder::SeqCst {
                    fence_full
                } else {
                    fence_acquire
                }));
            }
            row(
                target,
                operation,
                if rv64 {
                    "ld + order fence"
                } else {
                    "lw + order fence"
                },
                bytes,
                AtomicLoweringStrategy::NativeInline,
                true,
            )
        }
        AtomicOperation::Store => {
            if order.has_release() {
                bytes.extend(word(if order == MemoryOrder::SeqCst {
                    fence_full
                } else {
                    fence_release
                }));
            }
            bytes.extend(word(store));
            row(
                target,
                operation,
                if rv64 {
                    "order fence + sd"
                } else {
                    "order fence + sw"
                },
                bytes,
                AtomicLoweringStrategy::NativeInline,
                true,
            )
        }
        AtomicOperation::Fence if order == MemoryOrder::Relaxed => row(
            target,
            operation,
            "compiler barrier",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::Fence => {
            let fence = match order {
                MemoryOrder::Acquire => fence_acquire,
                MemoryOrder::Release => fence_release,
                MemoryOrder::AcqRel | MemoryOrder::SeqCst => fence_full,
                MemoryOrder::Relaxed => unreachable!(),
            };
            row(
                target,
                operation,
                "fence",
                word(fence),
                AtomicLoweringStrategy::NativeInline,
                true,
            )
        }
        AtomicOperation::CompilerFence => row(
            target,
            operation,
            "compiler barrier",
            Vec::new(),
            AtomicLoweringStrategy::CompilerOnly,
            true,
        ),
        AtomicOperation::Wait => {
            let load_wait = riscv_i(0, 10, if rv64 { 0b011 } else { 0b010 }, 5, 0x03);
            row(
                target,
                operation,
                "load/bne/pause wait loop",
                words(&[
                    load_wait,
                    riscv_b(12, 11, 5, 0b001, 0x63),
                    0x0100000F,
                    riscv_j(-12, 0, 0x6F),
                ]),
                AtomicLoweringStrategy::SpinWait,
                true,
            )
        }
        AtomicOperation::NotifyOne | AtomicOperation::NotifyAll => row(
            target,
            operation,
            "fence rw,rw",
            word(fence_full),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::VolatileLoad => row(
            target,
            operation,
            if rv64 { "ld volatile" } else { "lw volatile" },
            word(load),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        AtomicOperation::VolatileStore => row(
            target,
            operation,
            if rv64 { "sd volatile" } else { "sw volatile" },
            word(store),
            AtomicLoweringStrategy::NativeInline,
            true,
        ),
        _ => return Err("E_ATOMIC_PRIMITIVE_RMW_ROUTE_001"),
    };
    Ok(result)
}

fn word(value: u32) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn backend_order(order: MemoryOrder) -> BackendAtomicOrderingIR {
    match order {
        MemoryOrder::Relaxed => BackendAtomicOrderingIR::Relaxed,
        MemoryOrder::Acquire => BackendAtomicOrderingIR::Acquire,
        MemoryOrder::Release => BackendAtomicOrderingIR::Release,
        MemoryOrder::AcqRel => BackendAtomicOrderingIR::AcqRel,
        MemoryOrder::SeqCst => BackendAtomicOrderingIR::SeqCst,
    }
}

fn backend_rmw_op(operation: AtomicOperation) -> Option<BackendAtomicRmwOpIR> {
    match operation {
        AtomicOperation::Exchange => Some(BackendAtomicRmwOpIR::Exchange),
        AtomicOperation::FetchAdd => Some(BackendAtomicRmwOpIR::Add),
        AtomicOperation::FetchSub => Some(BackendAtomicRmwOpIR::Sub),
        AtomicOperation::FetchAnd => Some(BackendAtomicRmwOpIR::And),
        AtomicOperation::FetchOr => Some(BackendAtomicRmwOpIR::Or),
        AtomicOperation::FetchXor => Some(BackendAtomicRmwOpIR::Xor),
        AtomicOperation::CompareExchangeStrong | AtomicOperation::CompareExchangeWeak => {
            Some(BackendAtomicRmwOpIR::CompareExchange)
        }
        _ => None,
    }
}

fn backend_rmw_mnemonic(op: BackendAtomicRmwOpIR, target: AtomicTarget) -> &'static str {
    match (target, op) {
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::Exchange) => "xchg",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::Add) => "lock xadd",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::Sub) => "neg + lock xadd",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::And) => "lock cmpxchg and loop",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::Or) => "lock cmpxchg or loop",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::Xor) => "lock cmpxchg xor loop",
        (AtomicTarget::X86_64, BackendAtomicRmwOpIR::CompareExchange) => "lock cmpxchg",
        (AtomicTarget::AArch64, _) => "ldaxr/stlxr RMW loop",
        (AtomicTarget::RiscV32 | AtomicTarget::RiscV64, BackendAtomicRmwOpIR::CompareExchange) => {
            "lr/sc compare_exchange"
        }
        (AtomicTarget::RiscV32 | AtomicTarget::RiscV64, _) => "AMO RMW",
    }
}

fn riscv_i(imm: i32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
    (((imm as u32) & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

fn riscv_b(imm: i32, rs2: u32, rs1: u32, funct3: u32, opcode: u32) -> u32 {
    let bits = (imm as u32) & 0x1FFF;
    (((bits >> 12) & 1) << 31)
        | (((bits >> 5) & 0x3F) << 25)
        | (rs2 << 20)
        | (rs1 << 15)
        | (funct3 << 12)
        | (((bits >> 1) & 0xF) << 8)
        | (((bits >> 11) & 1) << 7)
        | opcode
}

fn riscv_j(imm: i32, rd: u32, opcode: u32) -> u32 {
    let bits = (imm as u32) & 0x1F_FFFF;
    (((bits >> 20) & 1) << 31)
        | (((bits >> 1) & 0x3FF) << 21)
        | (((bits >> 11) & 1) << 20)
        | (((bits >> 12) & 0xFF) << 12)
        | (rd << 7)
        | opcode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_store_and_fence_orders_have_target_specific_bytes() {
        let x86 = lower_atomic_primitive(
            AtomicTarget::X86_64,
            AtomicOperation::Store,
            Some(MemoryOrder::SeqCst),
        )
        .unwrap();
        assert_eq!(x86.bytes, vec![0x48, 0x87, 0x37]);

        let arm_acquire = lower_atomic_primitive(
            AtomicTarget::AArch64,
            AtomicOperation::Load,
            Some(MemoryOrder::Acquire),
        )
        .unwrap();
        let arm_relaxed = lower_atomic_primitive(
            AtomicTarget::AArch64,
            AtomicOperation::Load,
            Some(MemoryOrder::Relaxed),
        )
        .unwrap();
        assert_ne!(arm_acquire.bytes, arm_relaxed.bytes);

        let rv_release = lower_atomic_primitive(
            AtomicTarget::RiscV64,
            AtomicOperation::Store,
            Some(MemoryOrder::Release),
        )
        .unwrap();
        assert_eq!(rv_release.bytes.len(), 8);
        assert_eq!(&rv_release.bytes[..4], &0x0310000Fu32.to_le_bytes());
    }

    #[test]
    fn wait_notify_selects_event_or_spin_contract_per_isa() {
        let arm_wait = lower_atomic_primitive(
            AtomicTarget::AArch64,
            AtomicOperation::Wait,
            Some(MemoryOrder::Acquire),
        )
        .unwrap();
        assert_eq!(arm_wait.strategy, AtomicLoweringStrategy::EventWait);
        let arm_notify =
            lower_atomic_primitive(AtomicTarget::AArch64, AtomicOperation::NotifyAll, None)
                .unwrap();
        assert_eq!(arm_notify.bytes, 0xD503209Fu32.to_le_bytes());
        let x86_wait = lower_atomic_primitive(
            AtomicTarget::X86_64,
            AtomicOperation::Wait,
            Some(MemoryOrder::Relaxed),
        )
        .unwrap();
        assert!(x86_wait.bytes.windows(2).any(|bytes| bytes == [0xF3, 0x90]));
        assert!(
            x86_wait.bytes.len() > 2,
            "wait must include value observation and a retry branch"
        );
    }

    #[test]
    fn every_declared_c11_rmw_has_real_bytes_on_all_atomic_targets() {
        for operation in [
            AtomicOperation::Exchange,
            AtomicOperation::FetchAdd,
            AtomicOperation::FetchSub,
            AtomicOperation::FetchAnd,
            AtomicOperation::FetchOr,
            AtomicOperation::FetchXor,
            AtomicOperation::CompareExchangeStrong,
            AtomicOperation::CompareExchangeWeak,
        ] {
            for target in [
                AtomicTarget::X86_64,
                AtomicTarget::AArch64,
                AtomicTarget::RiscV32,
                AtomicTarget::RiscV64,
            ] {
                let encoded = lower_atomic_primitive(target, operation, Some(MemoryOrder::AcqRel))
                    .expect("declared C11 RMW must lower");
                assert!(!encoded.bytes.is_empty(), "{target:?} {operation:?}");
                assert_eq!(encoded.strategy, AtomicLoweringStrategy::NativeInline);
            }
        }
    }

    #[test]
    fn invalid_c11_orders_stop_before_instruction_selection() {
        assert_eq!(
            lower_atomic_primitive(
                AtomicTarget::AArch64,
                AtomicOperation::Load,
                Some(MemoryOrder::Release)
            ),
            Err("E_ATOMIC_LOAD_ORDER_001")
        );
        assert_eq!(
            lower_atomic_primitive(
                AtomicTarget::RiscV64,
                AtomicOperation::Store,
                Some(MemoryOrder::Acquire)
            ),
            Err("E_ATOMIC_STORE_ORDER_001")
        );
    }
}
