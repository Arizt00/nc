use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryLayer {
    Core,
    Alloc,
    Stoud,
}

impl LibraryLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Alloc => "alloc",
            Self::Stoud => "stoud",
        }
    }

    pub const fn requires_os(self) -> bool {
        matches!(self, Self::Stoud)
    }

    pub const fn requires_heap(self) -> bool {
        matches!(self, Self::Alloc | Self::Stoud)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreCapability {
    Types,
    Pointer,
    Region,
    Bytes,
    Layout,
    Endian,
    Volatile,
    Atomic,
    Math,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocCapability {
    Heap,
    Buffer,
    String,
    Vector,
    List,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoudCapability {
    Console,
    FileSystem,
    Network,
    Process,
    HostedThread,
    Time,
    Window,
    RuntimeService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct LibraryCapabilitySpec {
    pub layer: LibraryLayer,
    pub name: &'static str,
    pub requires_heap: bool,
    pub requires_os: bool,
    pub allowed_freestanding: bool,
}

pub const LIBRARY_CAPABILITY_SPECS: &[LibraryCapabilitySpec] = &[
    LibraryCapabilitySpec {
        layer: LibraryLayer::Core,
        name: "types",
        requires_heap: false,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Core,
        name: "memory",
        requires_heap: false,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Core,
        name: "atomic",
        requires_heap: false,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Core,
        name: "volatile",
        requires_heap: false,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Core,
        name: "math",
        requires_heap: false,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Alloc,
        name: "heap",
        requires_heap: true,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Alloc,
        name: "collections",
        requires_heap: true,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Alloc,
        name: "string",
        requires_heap: true,
        requires_os: false,
        allowed_freestanding: true,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "io",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "fs",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "net",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "process",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "thread",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "time",
        requires_heap: false,
        requires_os: true,
        allowed_freestanding: false,
    },
    LibraryCapabilitySpec {
        layer: LibraryLayer::Stoud,
        name: "ui",
        requires_heap: true,
        requires_os: true,
        allowed_freestanding: false,
    },
];

/// Classifies canonical lowered module paths. Legacy `nC.runtime.*` names remain
/// ABI-stable; their ownership is determined here rather than by their spelling.
pub fn classify_canonical_library_path(path: &[String]) -> LibraryLayer {
    let normalized = path
        .iter()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let joined = normalized.join(".");

    if joined.starts_with("nc.core.") || joined.starts_with("core.") {
        return LibraryLayer::Core;
    }
    if joined.starts_with("nc.alloc.") || joined.starts_with("alloc.") {
        return LibraryLayer::Alloc;
    }

    let family = normalized
        .windows(2)
        .find_map(|parts| (parts[0] == "runtime").then_some(parts[1].as_str()));
    match family {
        Some(
            "ptr" | "region" | "bytes" | "layout" | "align" | "endian" | "slice" | "span"
            | "atomic",
        ) => LibraryLayer::Core,
        Some(
            "heap" | "alloc" | "buffer" | "string" | "array" | "list" | "queue" | "vector"
            | "option" | "result" | "error",
        ) => LibraryLayer::Alloc,
        _ if joined.starts_with("nc.math.") => LibraryLayer::Core,
        _ if joined.starts_with("nc.vector.core") => LibraryLayer::Alloc,
        _ => LibraryLayer::Stoud,
    }
}

pub fn validate_freestanding_library_path(
    path: &[String],
    allocator_available: bool,
) -> Result<(), &'static str> {
    match classify_canonical_library_path(path) {
        LibraryLayer::Core => Ok(()),
        LibraryLayer::Alloc if allocator_available => Ok(()),
        LibraryLayer::Alloc => Err("E_KERNEL_ALLOC_UNAVAILABLE_001"),
        LibraryLayer::Stoud => Err("E_KERNEL_HOSTED_DEPENDENCY_001"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOrder {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

impl MemoryOrder {
    pub const ALL: [Self; 5] = [
        Self::Relaxed,
        Self::Acquire,
        Self::Release,
        Self::AcqRel,
        Self::SeqCst,
    ];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "relaxed" | "memory_order_relaxed" => Some(Self::Relaxed),
            "acquire" | "memory_order_acquire" => Some(Self::Acquire),
            "release" | "memory_order_release" => Some(Self::Release),
            "acq_rel" | "acqrel" | "memory_order_acq_rel" => Some(Self::AcqRel),
            "seq_cst" | "seqcst" | "memory_order_seq_cst" => Some(Self::SeqCst),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Acquire => "acquire",
            Self::Release => "release",
            Self::AcqRel => "acq_rel",
            Self::SeqCst => "seq_cst",
        }
    }

    pub const fn valid_for_load(self) -> bool {
        matches!(self, Self::Relaxed | Self::Acquire | Self::SeqCst)
    }

    pub const fn valid_for_store(self) -> bool {
        matches!(self, Self::Relaxed | Self::Release | Self::SeqCst)
    }

    pub const fn valid_for_cas_failure(self) -> bool {
        self.valid_for_load()
    }

    pub const fn has_acquire(self) -> bool {
        matches!(self, Self::Acquire | Self::AcqRel | Self::SeqCst)
    }

    pub const fn has_release(self) -> bool {
        matches!(self, Self::Release | Self::AcqRel | Self::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomicOperation {
    Load,
    Store,
    Exchange,
    FetchAdd,
    FetchSub,
    FetchAnd,
    FetchOr,
    FetchXor,
    CompareExchangeStrong,
    CompareExchangeWeak,
    Fence,
    CompilerFence,
    VolatileLoad,
    VolatileStore,
    Wait,
    NotifyOne,
    NotifyAll,
}

pub fn validate_atomic_order(op: AtomicOperation, order: MemoryOrder) -> Result<(), &'static str> {
    match op {
        AtomicOperation::Load | AtomicOperation::Wait if !order.valid_for_load() => {
            Err("E_ATOMIC_LOAD_ORDER_001")
        }
        AtomicOperation::Store if !order.valid_for_store() => Err("E_ATOMIC_STORE_ORDER_001"),
        AtomicOperation::NotifyOne | AtomicOperation::NotifyAll => Err("E_ATOMIC_NOTIFY_ORDER_001"),
        AtomicOperation::VolatileLoad | AtomicOperation::VolatileStore => {
            Err("E_VOLATILE_ORDER_001")
        }
        _ => Ok(()),
    }
}

pub fn validate_compare_exchange_orders(
    success: MemoryOrder,
    failure: MemoryOrder,
) -> Result<(), &'static str> {
    if !failure.valid_for_cas_failure() {
        return Err("E_ATOMIC_CAS_FAILURE_ORDER_001");
    }
    let allowed = match success {
        MemoryOrder::Relaxed => matches!(failure, MemoryOrder::Relaxed),
        MemoryOrder::Acquire => matches!(failure, MemoryOrder::Relaxed | MemoryOrder::Acquire),
        MemoryOrder::Release => matches!(failure, MemoryOrder::Relaxed),
        MemoryOrder::AcqRel => matches!(failure, MemoryOrder::Relaxed | MemoryOrder::Acquire),
        MemoryOrder::SeqCst => matches!(
            failure,
            MemoryOrder::Relaxed | MemoryOrder::Acquire | MemoryOrder::SeqCst
        ),
    };
    allowed
        .then_some(())
        .ok_or("E_ATOMIC_CAS_FAILURE_STRONGER_001")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c11_order_legality_is_explicit() {
        assert!(validate_atomic_order(AtomicOperation::Load, MemoryOrder::Acquire).is_ok());
        assert_eq!(
            validate_atomic_order(AtomicOperation::Load, MemoryOrder::Release),
            Err("E_ATOMIC_LOAD_ORDER_001")
        );
        assert!(validate_atomic_order(AtomicOperation::Store, MemoryOrder::Release).is_ok());
        assert_eq!(
            validate_atomic_order(AtomicOperation::Store, MemoryOrder::Acquire),
            Err("E_ATOMIC_STORE_ORDER_001")
        );
    }

    #[test]
    fn c11_compare_exchange_failure_cannot_release_or_exceed_success() {
        assert!(
            validate_compare_exchange_orders(MemoryOrder::SeqCst, MemoryOrder::Acquire).is_ok()
        );
        assert_eq!(
            validate_compare_exchange_orders(MemoryOrder::AcqRel, MemoryOrder::SeqCst),
            Err("E_ATOMIC_CAS_FAILURE_STRONGER_001")
        );
        assert_eq!(
            validate_compare_exchange_orders(MemoryOrder::SeqCst, MemoryOrder::Release),
            Err("E_ATOMIC_CAS_FAILURE_ORDER_001")
        );
    }

    #[test]
    fn canonical_runtime_names_do_not_blur_library_ownership() {
        let atomic = vec!["nC".into(), "runtime".into(), "atomic".into()];
        let heap = vec!["nC".into(), "runtime".into(), "heap".into()];
        let fs = vec!["nC".into(), "runtime".into(), "fs".into()];
        assert_eq!(classify_canonical_library_path(&atomic), LibraryLayer::Core);
        assert_eq!(classify_canonical_library_path(&heap), LibraryLayer::Alloc);
        assert_eq!(classify_canonical_library_path(&fs), LibraryLayer::Stoud);
        assert_eq!(
            validate_freestanding_library_path(&fs, true),
            Err("E_KERNEL_HOSTED_DEPENDENCY_001")
        );
    }
}
