use serde::{Deserialize, Serialize};

pub const NC_RAW_API_VERSION: &str = "nC.raw.api.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawApiDomain {
    Pointer,
    Memory,
    VirtualMemory,
    Mmio,
    Dma,
    Callable,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RawApiAliasSpec {
    pub compact: &'static str,
    pub canonical: &'static str,
    pub arity: usize,
    pub domain: RawApiDomain,
    pub accepts_type_arguments: bool,
    pub unambiguous_without_type_arguments: bool,
}

macro_rules! raw {
    ($compact:literal, $canonical:literal, $arity:literal, $domain:ident, $typed:literal, $plain:literal) => {
        RawApiAliasSpec {
            compact: $compact,
            canonical: $canonical,
            arity: $arity,
            domain: RawApiDomain::$domain,
            accepts_type_arguments: $typed,
            unambiguous_without_type_arguments: $plain,
        }
    };
}

/// The compact spelling is source syntax; the canonical spelling is the only
/// name retained by AST, sema, IR and backends.
pub const NC_RAW_API_V1_ALIASES: &[RawApiAliasSpec] = &[
    raw!("rd", "ptrRead", 1, Pointer, true, true),
    raw!("wr", "ptrWrite", 2, Pointer, true, true),
    raw!("off", "ptrOffset", 2, Pointer, true, true),
    raw!("dist", "ptrDiff", 2, Pointer, true, true),
    raw!("nil", "ptrIsNull", 1, Pointer, false, true),
    raw!("from", "ptrFromAddr", 1, Pointer, true, true),
    raw!("cast", "ptrCast", 1, Pointer, true, true),
    raw!("adr", "ptrAddr", 1, Pointer, false, true),
    raw!("cat", "strConcatInto", 4, String, false, true),
    raw!("alloc", "alloc", 1, Memory, false, true),
    raw!("calloc", "calloc", 2, Memory, false, true),
    raw!("realloc", "realloc", 2, Memory, false, true),
    raw!("free", "free", 1, Memory, false, true),
    raw!("cp", "memcpy", 3, Memory, false, true),
    raw!("mv", "memmove", 3, Memory, false, true),
    raw!("fill", "memset", 3, Memory, false, true),
    raw!("cmp", "memcmp", 3, Memory, false, true),
    raw!("map", "vmMap", 6, VirtualMemory, false, true),
    raw!("protect", "vmProtect", 3, VirtualMemory, false, true),
    raw!("unmap", "vmUnmap", 2, VirtualMemory, false, true),
    raw!("exec", "vmExecutable", 2, VirtualMemory, false, true),
    raw!("closureAt", "closureAt", 2, Callable, false, true),
    raw!("asyncAt", "asyncAt", 2, Callable, false, true),
    raw!("callAt", "callAt", 2, Callable, false, true),
    raw!("mrd8", "mmio_read8", 1, Mmio, false, true),
    raw!("mrd16", "mmio_read16", 1, Mmio, false, true),
    raw!("mrd32", "mmio_read32", 1, Mmio, false, true),
    raw!("mrd64", "mmio_read64", 1, Mmio, false, true),
    raw!("mrd128", "mmio_read128", 2, Mmio, false, true),
    raw!("mrd256", "mmio_read256", 2, Mmio, false, true),
    raw!("mrd512", "mmio_read512", 2, Mmio, false, true),
    raw!("mwr8", "mmio_write8", 2, Mmio, false, true),
    raw!("mwr16", "mmio_write16", 2, Mmio, false, true),
    raw!("mwr32", "mmio_write32", 2, Mmio, false, true),
    raw!("mwr64", "mmio_write64", 2, Mmio, false, true),
    raw!("mwr128", "mmio_write128", 2, Mmio, false, true),
    raw!("mwr256", "mmio_write256", 2, Mmio, false, true),
    raw!("mwr512", "mmio_write512", 2, Mmio, false, true),
    raw!("dPin", "dma_pin_pages", 6, Dma, false, true),
    raw!("dUnpin", "dma_unpin_pages", 6, Dma, false, true),
    raw!("dSg", "dma_sg_write", 6, Dma, false, true),
    raw!("dTo", "dma_bounce_to_device", 3, Dma, false, true),
    raw!("dFrom", "dma_bounce_from_device", 3, Dma, false, true),
    raw!("dClean", "dma_cache_clean", 3, Dma, false, true),
    raw!("dInv", "dma_cache_invalidate", 3, Dma, false, true),
    raw!("dFlush", "dma_cache_flush", 3, Dma, false, true),
    raw!("dFence", "dma_fence", 0, Dma, false, true),
];

pub fn raw_api_v1_alias(name: &str, arity: usize) -> Option<&'static RawApiAliasSpec> {
    NC_RAW_API_V1_ALIASES
        .iter()
        .find(|spec| spec.arity == arity && spec.compact.eq_ignore_ascii_case(name))
}

pub fn raw_api_v1_typed_alias(name: &str) -> Option<&'static RawApiAliasSpec> {
    NC_RAW_API_V1_ALIASES
        .iter()
        .find(|spec| spec.accepts_type_arguments && spec.compact.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn raw_v1_aliases_are_unique_and_cover_every_material_domain() {
        let mut keys = HashSet::new();
        for spec in NC_RAW_API_V1_ALIASES {
            assert!(
                keys.insert((spec.compact.to_ascii_lowercase(), spec.arity)),
                "duplicate compact raw spelling: {} / {}",
                spec.compact,
                spec.arity
            );
            assert!(!spec.canonical.is_empty());
        }
        for domain in [
            RawApiDomain::Pointer,
            RawApiDomain::Memory,
            RawApiDomain::VirtualMemory,
            RawApiDomain::Mmio,
            RawApiDomain::Dma,
            RawApiDomain::Callable,
            RawApiDomain::String,
        ] {
            assert!(NC_RAW_API_V1_ALIASES
                .iter()
                .any(|spec| spec.domain == domain));
        }
    }
}
