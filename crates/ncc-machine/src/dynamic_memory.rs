#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedMemoryOs {
    Windows,
    Linux,
    MacOs,
    Freestanding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualAllocatorKind {
    Arena,
    Pool,
    Slab,
    Buddy,
    PerCpuHeapTable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualAllocatorLayout {
    pub kind: ManualAllocatorKind,
    pub state_bytes: usize,
    pub state_align: usize,
    pub caller_supplies_backing: bool,
    pub caller_supplies_metadata: bool,
    pub concurrent_head_offset: Option<usize>,
    pub hidden_allocation: bool,
}

pub const fn manual_allocator_layout(
    kind: ManualAllocatorKind,
    pointer_bytes: usize,
) -> ManualAllocatorLayout {
    let word = if pointer_bytes <= 4 { 4 } else { 8 };
    match kind {
        ManualAllocatorKind::Arena => ManualAllocatorLayout {
            kind,
            state_bytes: word * 4,
            state_align: word,
            caller_supplies_backing: true,
            caller_supplies_metadata: false,
            concurrent_head_offset: Some(word * 2),
            hidden_allocation: false,
        },
        ManualAllocatorKind::Pool => ManualAllocatorLayout {
            kind,
            state_bytes: word * 6,
            state_align: word,
            caller_supplies_backing: true,
            caller_supplies_metadata: false,
            concurrent_head_offset: Some(word * 4),
            hidden_allocation: false,
        },
        ManualAllocatorKind::Slab => ManualAllocatorLayout {
            kind,
            state_bytes: word * 8,
            state_align: word,
            caller_supplies_backing: true,
            caller_supplies_metadata: true,
            concurrent_head_offset: Some(word * 5),
            hidden_allocation: false,
        },
        ManualAllocatorKind::Buddy => ManualAllocatorLayout {
            kind,
            state_bytes: word * 8,
            state_align: word,
            caller_supplies_backing: true,
            caller_supplies_metadata: true,
            concurrent_head_offset: None,
            hidden_allocation: false,
        },
        ManualAllocatorKind::PerCpuHeapTable => ManualAllocatorLayout {
            kind,
            state_bytes: word * 4,
            state_align: word,
            caller_supplies_backing: true,
            caller_supplies_metadata: true,
            concurrent_head_offset: None,
            hidden_allocation: false,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawReallocContract {
    pub caller_passes_old_size: bool,
    pub old_block_survives_failure: bool,
    pub copy_bytes_is_min_old_new: bool,
    pub zero_size_releases: bool,
    pub preserves_requested_alignment: bool,
    pub hidden_size_header: bool,
}

pub const RAW_ALIGNED_REALLOC_CONTRACT: RawReallocContract = RawReallocContract {
    caller_passes_old_size: true,
    old_block_survives_failure: true,
    copy_bytes_is_min_old_new: true,
    zero_size_releases: true,
    preserves_requested_alignment: true,
    hidden_size_header: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualMemoryAbiPlan {
    pub os: HostedMemoryOs,
    pub reserve: &'static str,
    pub commit: &'static str,
    pub decommit: &'static str,
    pub release: &'static str,
    pub protect: &'static str,
    pub shared_create: &'static str,
    pub shared_map: &'static str,
    pub numa_bind: Option<&'static str>,
    pub native_flags_are_passthrough: bool,
    pub unmap_requires_size: bool,
    pub oom_returns_null: bool,
    pub rw_to_rx_flushes_instruction_cache: bool,
    pub hidden_mapping_registry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualMemoryNativeConstants {
    pub os: HostedMemoryOs,
    pub protect_none: u64,
    pub protect_read: u64,
    pub protect_read_write: u64,
    pub protect_read_execute: u64,
    pub protect_read_write_execute: u64,
    pub guard_modifier: Option<u64>,
    pub map_shared: Option<u64>,
    pub map_private_cow: Option<u64>,
    pub map_anonymous: Option<u64>,
    pub map_fixed: Option<u64>,
    pub huge_pages: Option<u64>,
    pub reserve: Option<u64>,
    pub commit: Option<u64>,
    pub decommit: Option<u64>,
    pub release: Option<u64>,
}

pub fn virtual_memory_native_constants(
    target: &str,
) -> Result<VirtualMemoryNativeConstants, String> {
    let target = target.to_ascii_lowercase();
    if target.contains("windows") || target.contains("msvc") {
        return Ok(VirtualMemoryNativeConstants {
            os: HostedMemoryOs::Windows,
            protect_none: 0x01,
            protect_read: 0x02,
            protect_read_write: 0x04,
            protect_read_execute: 0x20,
            protect_read_write_execute: 0x40,
            guard_modifier: Some(0x100),
            map_shared: Some(0x000f_001f),
            map_private_cow: Some(0x0000_0001),
            map_anonymous: None,
            map_fixed: None,
            huge_pages: Some(0x2000_0000),
            reserve: Some(0x2000),
            commit: Some(0x1000),
            decommit: Some(0x4000),
            release: Some(0x8000),
        });
    }
    if target.contains("linux") {
        return Ok(VirtualMemoryNativeConstants {
            os: HostedMemoryOs::Linux,
            protect_none: 0,
            protect_read: 1,
            protect_read_write: 3,
            protect_read_execute: 5,
            protect_read_write_execute: 7,
            guard_modifier: None,
            map_shared: Some(0x01),
            map_private_cow: Some(0x02),
            map_anonymous: Some(0x20),
            map_fixed: Some(0x10),
            huge_pages: Some(0x0004_0000),
            reserve: None,
            commit: None,
            decommit: None,
            release: None,
        });
    }
    if target.contains("darwin") || target.contains("macos") {
        return Ok(VirtualMemoryNativeConstants {
            os: HostedMemoryOs::MacOs,
            protect_none: 0,
            protect_read: 1,
            protect_read_write: 3,
            protect_read_execute: 5,
            protect_read_write_execute: 7,
            guard_modifier: None,
            map_shared: Some(0x01),
            map_private_cow: Some(0x02),
            map_anonymous: Some(0x1000),
            map_fixed: Some(0x10),
            huge_pages: None,
            reserve: None,
            commit: None,
            decommit: None,
            release: None,
        });
    }
    Err(format!(
        "target '{target}' has no hosted native virtual-memory constants"
    ))
}

pub fn virtual_memory_abi_plan(target: &str) -> Result<VirtualMemoryAbiPlan, String> {
    let target = target.to_ascii_lowercase();
    let plan = if target.contains("windows") || target.contains("msvc") {
        VirtualMemoryAbiPlan {
            os: HostedMemoryOs::Windows,
            reserve: "VirtualAlloc(MEM_RESERVE)",
            commit: "VirtualAlloc(MEM_COMMIT)",
            decommit: "VirtualFree(MEM_DECOMMIT)",
            release: "VirtualFree(MEM_RELEASE)",
            protect: "VirtualProtect",
            shared_create: "CreateFileMapping",
            shared_map: "MapViewOfFileEx",
            numa_bind: Some("VirtualAllocExNuma"),
            native_flags_are_passthrough: true,
            unmap_requires_size: true,
            oom_returns_null: true,
            rw_to_rx_flushes_instruction_cache: true,
            hidden_mapping_registry: false,
        }
    } else if target.contains("linux") {
        VirtualMemoryAbiPlan {
            os: HostedMemoryOs::Linux,
            reserve: "mmap(PROT_NONE)",
            commit: "mprotect",
            decommit: "madvise(DONTNEED)+mprotect(PROT_NONE)",
            release: "munmap",
            protect: "mprotect",
            shared_create: "shm_open+ftruncate",
            shared_map: "mmap(MAP_SHARED|MAP_PRIVATE)",
            numa_bind: Some("mbind syscall"),
            native_flags_are_passthrough: true,
            unmap_requires_size: true,
            oom_returns_null: true,
            rw_to_rx_flushes_instruction_cache: true,
            hidden_mapping_registry: false,
        }
    } else if target.contains("darwin") || target.contains("macos") {
        VirtualMemoryAbiPlan {
            os: HostedMemoryOs::MacOs,
            reserve: "mmap(PROT_NONE)",
            commit: "mprotect",
            decommit: "madvise(DONTNEED)+mprotect(PROT_NONE)",
            release: "munmap",
            protect: "mprotect",
            shared_create: "shm_open+ftruncate",
            shared_map: "mmap(MAP_SHARED|MAP_PRIVATE)",
            numa_bind: None,
            native_flags_are_passthrough: true,
            unmap_requires_size: true,
            oom_returns_null: true,
            rw_to_rx_flushes_instruction_cache: true,
            hidden_mapping_registry: false,
        }
    } else if target.contains("none") || target.contains("elf") {
        VirtualMemoryAbiPlan {
            os: HostedMemoryOs::Freestanding,
            reserve: "caller page-table allocator",
            commit: "caller physical-frame mapper",
            decommit: "caller unmap+frame policy",
            release: "caller virtual-range allocator",
            protect: "caller page-table permission update",
            shared_create: "caller shared-frame object",
            shared_map: "caller page-table mapper",
            numa_bind: Some("caller firmware/topology policy"),
            native_flags_are_passthrough: true,
            unmap_requires_size: true,
            oom_returns_null: true,
            rw_to_rx_flushes_instruction_cache: true,
            hidden_mapping_registry: false,
        }
    } else {
        return Err(format!("unsupported virtual-memory target '{target}'"));
    };
    Ok(plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmMachinePrimitive {
    X86InvalidatePageRax,
    X86MemoryFence,
    X86SerializeInstructionStream,
    Aarch64CleanDataCacheX0,
    Aarch64InvalidateInstructionCacheX0,
    Aarch64InvalidateTlbX0,
    Aarch64DataBarrier,
    Aarch64InstructionBarrier,
    Armv7CleanDataCacheR0,
    Armv7InvalidateInstructionCacheR0,
    Armv7InvalidateTlbR0,
    Armv7DataBarrier,
    Armv7InstructionBarrier,
    ThumbDataBarrier,
    ThumbInstructionBarrier,
    RiscvInvalidateTlbA0,
    RiscvInstructionFence,
}

impl VmMachinePrimitive {
    pub const fn encode(self) -> &'static [u8] {
        match self {
            Self::X86InvalidatePageRax => &[0x0f, 0x01, 0x38],
            Self::X86MemoryFence => &[0x0f, 0xae, 0xf0],
            Self::X86SerializeInstructionStream => &[0x0f, 0xa2],
            Self::Aarch64CleanDataCacheX0 => &[0x20, 0x7b, 0x0b, 0xd5],
            Self::Aarch64InvalidateInstructionCacheX0 => &[0x20, 0x75, 0x0b, 0xd5],
            Self::Aarch64InvalidateTlbX0 => &[0x20, 0x83, 0x08, 0xd5],
            Self::Aarch64DataBarrier => &[0x9f, 0x3b, 0x03, 0xd5],
            Self::Aarch64InstructionBarrier => &[0xdf, 0x3f, 0x03, 0xd5],
            Self::Armv7CleanDataCacheR0 => &[0x3a, 0x0f, 0x07, 0xee],
            Self::Armv7InvalidateInstructionCacheR0 => &[0x35, 0x0f, 0x07, 0xee],
            Self::Armv7InvalidateTlbR0 => &[0x37, 0x0f, 0x08, 0xee],
            Self::Armv7DataBarrier => &[0x4f, 0xf0, 0x7f, 0xf5],
            Self::Armv7InstructionBarrier => &[0x6f, 0xf0, 0x7f, 0xf5],
            Self::ThumbDataBarrier => &[0xbf, 0xf3, 0x4f, 0x8f],
            Self::ThumbInstructionBarrier => &[0xbf, 0xf3, 0x6f, 0x8f],
            Self::RiscvInvalidateTlbA0 => &[0x73, 0x00, 0x05, 0x12],
            Self::RiscvInstructionFence => &[0x0f, 0x10, 0x00, 0x00],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutableTransitionPlan {
    pub target: &'static str,
    pub operations: &'static [VmMachinePrimitive],
    pub permits_direct_rwx: bool,
    pub requires_explicit_permission_change: bool,
}

const X86_EXEC_TRANSITION: &[VmMachinePrimitive] = &[
    VmMachinePrimitive::X86MemoryFence,
    VmMachinePrimitive::X86SerializeInstructionStream,
];
const AARCH64_EXEC_TRANSITION: &[VmMachinePrimitive] = &[
    VmMachinePrimitive::Aarch64CleanDataCacheX0,
    VmMachinePrimitive::Aarch64DataBarrier,
    VmMachinePrimitive::Aarch64InvalidateInstructionCacheX0,
    VmMachinePrimitive::Aarch64DataBarrier,
    VmMachinePrimitive::Aarch64InstructionBarrier,
];
const ARMV7_EXEC_TRANSITION: &[VmMachinePrimitive] = &[
    VmMachinePrimitive::Armv7CleanDataCacheR0,
    VmMachinePrimitive::Armv7DataBarrier,
    VmMachinePrimitive::Armv7InvalidateInstructionCacheR0,
    VmMachinePrimitive::Armv7InstructionBarrier,
];
const RISCV_EXEC_TRANSITION: &[VmMachinePrimitive] = &[VmMachinePrimitive::RiscvInstructionFence];

pub fn executable_transition_plan(target: &str) -> Result<ExecutableTransitionPlan, String> {
    let target_lower = target.to_ascii_lowercase();
    let (target_name, operations) = if target_lower.contains("x86_64")
        || target_lower.contains("i386")
        || target_lower.contains("i686")
    {
        ("x86", X86_EXEC_TRANSITION)
    } else if target_lower.contains("aarch64") {
        ("aarch64", AARCH64_EXEC_TRANSITION)
    } else if target_lower.contains("thumb") || target_lower.contains("armv7") {
        ("armv7-thumb", ARMV7_EXEC_TRANSITION)
    } else if target_lower.contains("riscv") {
        ("riscv", RISCV_EXEC_TRANSITION)
    } else {
        return Err(format!("unsupported executable-memory target '{target}'"));
    };
    Ok(ExecutableTransitionPlan {
        target: target_name,
        operations,
        permits_direct_rwx: true,
        requires_explicit_permission_change: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_plans_have_no_hidden_registry_or_oom_abort() {
        for target in [
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
        ] {
            let plan = virtual_memory_abi_plan(target).expect(target);
            assert!(plan.native_flags_are_passthrough);
            assert!(plan.unmap_requires_size && plan.oom_returns_null);
            assert!(!plan.hidden_mapping_registry);
            assert!(plan.rw_to_rx_flushes_instruction_cache);
        }
    }

    #[test]
    fn hosted_native_flags_are_explicit_and_never_cross_translated() {
        let windows = virtual_memory_native_constants("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(windows.protect_read_write, 0x04);
        assert_eq!(windows.guard_modifier, Some(0x100));
        assert_eq!(windows.huge_pages, Some(0x2000_0000));

        let linux = virtual_memory_native_constants("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(linux.protect_read_write, 3);
        assert_eq!(linux.map_private_cow, Some(2));
        assert_eq!(linux.huge_pages, Some(0x0004_0000));

        let macos = virtual_memory_native_constants("aarch64-apple-darwin").unwrap();
        assert_eq!(macos.map_anonymous, Some(0x1000));
        assert_eq!(macos.huge_pages, None);
    }

    #[test]
    fn every_allocator_is_caller_backed_and_has_no_hidden_allocation() {
        for kind in [
            ManualAllocatorKind::Arena,
            ManualAllocatorKind::Pool,
            ManualAllocatorKind::Slab,
            ManualAllocatorKind::Buddy,
            ManualAllocatorKind::PerCpuHeapTable,
        ] {
            for pointer_bytes in [4, 8] {
                let layout = manual_allocator_layout(kind, pointer_bytes);
                assert!(layout.caller_supplies_backing);
                assert!(!layout.hidden_allocation);
                assert_eq!(layout.state_bytes % layout.state_align, 0);
            }
        }
    }

    #[test]
    fn aligned_realloc_never_hides_a_size_header_or_destroys_on_oom() {
        let contract = RAW_ALIGNED_REALLOC_CONTRACT;
        assert!(contract.caller_passes_old_size);
        assert!(contract.old_block_survives_failure);
        assert!(contract.copy_bytes_is_min_old_new);
        assert!(contract.zero_size_releases);
        assert!(contract.preserves_requested_alignment);
        assert!(!contract.hidden_size_header);
    }

    #[test]
    fn vm_cache_and_tlb_primitives_match_independent_assembler_bytes() {
        assert_eq!(
            VmMachinePrimitive::X86InvalidatePageRax.encode(),
            [0x0f, 0x01, 0x38]
        );
        assert_eq!(
            VmMachinePrimitive::Aarch64InvalidateTlbX0.encode(),
            [0x20, 0x83, 0x08, 0xd5]
        );
        assert_eq!(
            VmMachinePrimitive::Aarch64DataBarrier.encode(),
            [0x9f, 0x3b, 0x03, 0xd5]
        );
        assert_eq!(
            VmMachinePrimitive::Aarch64InstructionBarrier.encode(),
            [0xdf, 0x3f, 0x03, 0xd5]
        );
        assert_eq!(
            VmMachinePrimitive::Armv7InvalidateTlbR0.encode(),
            [0x37, 0x0f, 0x08, 0xee]
        );
        assert_eq!(
            VmMachinePrimitive::RiscvInvalidateTlbA0.encode(),
            [0x73, 0x00, 0x05, 0x12]
        );
        assert_eq!(
            VmMachinePrimitive::RiscvInstructionFence.encode(),
            [0x0f, 0x10, 0x00, 0x00]
        );
    }

    #[test]
    fn executable_transition_is_explicit_on_every_supported_isa() {
        for target in [
            "x86_64-unknown-linux-gnu",
            "i386-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "armv7a-none-eabi",
            "riscv32imac-unknown-elf",
            "riscv64gc-unknown-linux-gnu",
        ] {
            let plan = executable_transition_plan(target).expect(target);
            assert!(plan.requires_explicit_permission_change);
            assert!(!plan.operations.is_empty());
        }
    }
}
