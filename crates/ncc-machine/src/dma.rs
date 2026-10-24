//! Manual DMA authority shared by kernel lowering and machine emission.
//!
//! These types describe caller-owned memory. They never allocate, pin pages,
//! choose an IOVA, infer coherency, or recover a device implicitly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaDirection {
    ToDevice,
    FromDevice,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DmaCoherency {
    Coherent,
    NonCoherent,
}

pub const DMA_REGION_PINNED: u64 = 1 << 0;
pub const DMA_REGION_COHERENT: u64 = 1 << 1;
pub const DMA_REGION_DEVICE_READ: u64 = 1 << 2;
pub const DMA_REGION_DEVICE_WRITE: u64 = 1 << 3;
pub const DMA_REGION_BOUNCE: u64 = 1 << 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct ManualPinnedRegion {
    pub virtual_address: u64,
    pub physical_address: u64,
    pub bytes: u64,
    pub page_size: u64,
    pub flags: u64,
}

impl ManualPinnedRegion {
    pub const fn from_raw_parts(
        virtual_address: u64,
        physical_address: u64,
        bytes: u64,
        page_size: u64,
        flags: u64,
    ) -> Self {
        Self {
            virtual_address,
            physical_address,
            bytes,
            page_size,
            flags,
        }
    }

    pub const fn is_marked_pinned(self) -> bool {
        self.flags & DMA_REGION_PINNED != 0
    }

    pub const fn coherency(self) -> DmaCoherency {
        if self.flags & DMA_REGION_COHERENT != 0 {
            DmaCoherency::Coherent
        } else {
            DmaCoherency::NonCoherent
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct DmaScatterGatherEntry {
    pub iova: u64,
    pub physical_address: u64,
    pub bytes: u64,
    pub flags: u64,
}

impl DmaScatterGatherEntry {
    pub const fn from_raw_parts(iova: u64, physical_address: u64, bytes: u64, flags: u64) -> Self {
        Self {
            iova,
            physical_address,
            bytes,
            flags,
        }
    }

    pub const fn end_iova_wrapping(self) -> u64 {
        self.iova.wrapping_add(self.bytes)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct ManualBounceBuffer {
    pub original_address: u64,
    pub bounce_address: u64,
    pub bytes: u64,
    pub direction: u64,
}

impl ManualBounceBuffer {
    pub const fn from_raw_parts(
        original_address: u64,
        bounce_address: u64,
        bytes: u64,
        direction: DmaDirection,
    ) -> Self {
        Self {
            original_address,
            bounce_address,
            bytes,
            direction: direction as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct RawIommuCommand {
    pub low: u64,
    pub high: u64,
}

impl RawIommuCommand {
    pub const fn from_words(low: u64, high: u64) -> Self {
        Self { low, high }
    }

    pub const fn to_le_bytes(self) -> [u8; 16] {
        let low = self.low.to_le_bytes();
        let high = self.high.to_le_bytes();
        [
            low[0], low[1], low[2], low[3], low[4], low[5], low[6], low[7], high[0], high[1],
            high[2], high[3], high[4], high[5], high[6], high[7],
        ]
    }
}

pub const INTEL_VTD_QI_CONTEXT_CACHE: u8 = 0x1;
pub const INTEL_VTD_QI_IOTLB: u8 = 0x2;
pub const INTEL_VTD_QI_DEVICE_IOTLB: u8 = 0x3;
pub const INTEL_VTD_QI_INTERRUPT_ENTRY_CACHE: u8 = 0x4;
pub const INTEL_VTD_QI_WAIT: u8 = 0x5;
pub const INTEL_VTD_QI_PASID_CACHE: u8 = 0x6;
pub const INTEL_VTD_QI_PI_CACHE: u8 = 0x7;
pub const INTEL_VTD_QI_EIOTLB: u8 = 0x8;
pub const INTEL_VTD_QI_PC: u8 = 0x9;

pub const ARM_SMMUV3_CMD_PREFETCH_CONFIG: u8 = 0x01;
pub const ARM_SMMUV3_CMD_CFGI_STE: u8 = 0x03;
pub const ARM_SMMUV3_CMD_CFGI_ALL: u8 = 0x04;
pub const ARM_SMMUV3_CMD_TLBI_S2_IPA: u8 = 0x2a;
pub const ARM_SMMUV3_CMD_TLBI_NSNH_ALL: u8 = 0x30;
pub const ARM_SMMUV3_CMD_ATC_INV: u8 = 0x40;
pub const ARM_SMMUV3_CMD_PRI_RESP: u8 = 0x41;
pub const ARM_SMMUV3_CMD_RESUME: u8 = 0x44;
pub const ARM_SMMUV3_CMD_STALL_TERM: u8 = 0x45;
pub const ARM_SMMUV3_CMD_SYNC: u8 = 0x46;

pub const RISCV_IOMMU_CMD_IOTINVAL: u8 = 1;
pub const RISCV_IOMMU_CMD_IOFENCE: u8 = 2;
pub const RISCV_IOMMU_CMD_IODIR: u8 = 3;
pub const RISCV_IOMMU_CMD_ATS: u8 = 4;

pub const fn arm_smmuv3_command(opcode: u8, word0_fields: u64, word1: u64) -> RawIommuCommand {
    RawIommuCommand::from_words((word0_fields & !0xff) | opcode as u64, word1)
}

pub const fn riscv_iommu_command(
    opcode: u8,
    function: u8,
    word0_fields: u64,
    word1: u64,
) -> RawIommuCommand {
    RawIommuCommand::from_words(
        (word0_fields & !0x7f) | (opcode as u64 & 0x7f) | ((function as u64 & 0x7) << 7),
        word1,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaCachePrimitive {
    X86ClflushRax,
    X86ClflushoptRax,
    X86ClwbRax,
    X86Sfence,
    X86Mfence,
    Aarch64DcIvacX0,
    Aarch64DcCvacX0,
    Aarch64DcCivacX0,
    Aarch64DsbOsh,
    Aarch64DsbOshst,
    Armv7DcimvacR0,
    Armv7DccmvacR0,
    Armv7DccimvacR0,
    Armv7Dsb,
    RiscvCboInvalA0,
    RiscvCboCleanA0,
    RiscvCboFlushA0,
    RiscvFenceRwRw,
}

impl DmaCachePrimitive {
    pub const fn encode(self) -> &'static [u8] {
        match self {
            Self::X86ClflushRax => &[0x0f, 0xae, 0x38],
            Self::X86ClflushoptRax => &[0x66, 0x0f, 0xae, 0x38],
            Self::X86ClwbRax => &[0x66, 0x0f, 0xae, 0x30],
            Self::X86Sfence => &[0x0f, 0xae, 0xf8],
            Self::X86Mfence => &[0x0f, 0xae, 0xf0],
            Self::Aarch64DcIvacX0 => &[0x20, 0x76, 0x08, 0xd5],
            Self::Aarch64DcCvacX0 => &[0x20, 0x7a, 0x0b, 0xd5],
            Self::Aarch64DcCivacX0 => &[0x20, 0x7e, 0x0b, 0xd5],
            Self::Aarch64DsbOsh => &[0x9f, 0x33, 0x03, 0xd5],
            Self::Aarch64DsbOshst => &[0x9f, 0x32, 0x03, 0xd5],
            Self::Armv7DcimvacR0 => &[0x36, 0x0f, 0x07, 0xee],
            Self::Armv7DccmvacR0 => &[0x3a, 0x0f, 0x07, 0xee],
            Self::Armv7DccimvacR0 => &[0x3e, 0x0f, 0x07, 0xee],
            Self::Armv7Dsb => &[0x4f, 0xf0, 0x7f, 0xf5],
            Self::RiscvCboInvalA0 => &[0x0f, 0x20, 0x05, 0x00],
            Self::RiscvCboCleanA0 => &[0x0f, 0x20, 0x15, 0x00],
            Self::RiscvCboFlushA0 => &[0x0f, 0x20, 0x25, 0x00],
            Self::RiscvFenceRwRw => &[0x0f, 0x00, 0x30, 0x03],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaCachePlan {
    pub target: &'static str,
    pub clean: &'static [DmaCachePrimitive],
    pub invalidate: &'static [DmaCachePrimitive],
    pub clean_invalidate: &'static [DmaCachePrimitive],
    pub required_extension: Option<&'static str>,
    pub caller_supplies_cache_line_bytes: bool,
    pub hidden_coherency_detection: bool,
}

const X86_CLEAN: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::X86ClflushRax,
    DmaCachePrimitive::X86Mfence,
];
const X86_INVALIDATE: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::X86ClflushRax,
    DmaCachePrimitive::X86Mfence,
];
const X86_FLUSH: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::X86ClflushRax,
    DmaCachePrimitive::X86Mfence,
];
const AARCH64_CLEAN: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Aarch64DcCvacX0,
    DmaCachePrimitive::Aarch64DsbOshst,
];
const AARCH64_INVALIDATE: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Aarch64DcIvacX0,
    DmaCachePrimitive::Aarch64DsbOsh,
];
const AARCH64_FLUSH: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Aarch64DcCivacX0,
    DmaCachePrimitive::Aarch64DsbOsh,
];
const ARMV7_CLEAN: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Armv7DccmvacR0,
    DmaCachePrimitive::Armv7Dsb,
];
const ARMV7_INVALIDATE: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Armv7DcimvacR0,
    DmaCachePrimitive::Armv7Dsb,
];
const ARMV7_FLUSH: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::Armv7DccimvacR0,
    DmaCachePrimitive::Armv7Dsb,
];
const RISCV_CLEAN: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::RiscvCboCleanA0,
    DmaCachePrimitive::RiscvFenceRwRw,
];
const RISCV_INVALIDATE: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::RiscvCboInvalA0,
    DmaCachePrimitive::RiscvFenceRwRw,
];
const RISCV_FLUSH: &[DmaCachePrimitive] = &[
    DmaCachePrimitive::RiscvCboFlushA0,
    DmaCachePrimitive::RiscvFenceRwRw,
];

pub fn dma_cache_plan(target: &str) -> Result<DmaCachePlan, String> {
    let target_lower = target.to_ascii_lowercase();
    let (target, clean, invalidate, clean_invalidate, required_extension) = if target_lower
        .contains("x86_64")
        || target_lower.contains("i386")
        || target_lower.contains("i686")
    {
        (
            "x86",
            X86_CLEAN,
            X86_INVALIDATE,
            X86_FLUSH,
            Some("CLFLUSH; CLWB/CLFLUSHOPT remain explicit machine operations"),
        )
    } else if target_lower.contains("aarch64") {
        (
            "aarch64",
            AARCH64_CLEAN,
            AARCH64_INVALIDATE,
            AARCH64_FLUSH,
            None,
        )
    } else if target_lower.contains("armv7") || target_lower.contains("thumb") {
        ("armv7", ARMV7_CLEAN, ARMV7_INVALIDATE, ARMV7_FLUSH, None)
    } else if target_lower.contains("riscv") {
        (
            "riscv",
            RISCV_CLEAN,
            RISCV_INVALIDATE,
            RISCV_FLUSH,
            Some("Zicbom"),
        )
    } else {
        return Err(format!("unsupported manual DMA cache target '{target}'"));
    };
    Ok(DmaCachePlan {
        target,
        clean,
        invalidate,
        clean_invalidate,
        required_extension,
        caller_supplies_cache_line_bytes: true,
        hidden_coherency_detection: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualDmaContract {
    pub caller_pins_pages: bool,
    pub caller_owns_descriptors: bool,
    pub caller_selects_iova: bool,
    pub caller_selects_coherency: bool,
    pub caller_performs_cache_maintenance: bool,
    pub caller_submits_iommu_commands: bool,
    pub caller_recovers_faults: bool,
    pub hidden_allocation: bool,
    pub hidden_bounds_checks: bool,
    pub hidden_recovery: bool,
}

pub const MANUAL_DMA_CONTRACT: ManualDmaContract = ManualDmaContract {
    caller_pins_pages: true,
    caller_owns_descriptors: true,
    caller_selects_iova: true,
    caller_selects_coherency: true,
    caller_performs_cache_maintenance: true,
    caller_submits_iommu_commands: true,
    caller_recovers_faults: true,
    hidden_allocation: false,
    hidden_bounds_checks: false,
    hidden_recovery: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_dma_layouts_are_stable_and_all_storage_is_caller_owned() {
        assert_eq!(core::mem::size_of::<ManualPinnedRegion>(), 40);
        assert_eq!(core::mem::size_of::<DmaScatterGatherEntry>(), 32);
        assert_eq!(core::mem::size_of::<ManualBounceBuffer>(), 32);
        assert_eq!(core::mem::size_of::<RawIommuCommand>(), 16);
        assert_eq!(core::mem::align_of::<RawIommuCommand>(), 16);
        assert!(MANUAL_DMA_CONTRACT.caller_pins_pages);
        assert!(MANUAL_DMA_CONTRACT.caller_selects_iova);
        assert!(!MANUAL_DMA_CONTRACT.hidden_allocation);
        assert!(!MANUAL_DMA_CONTRACT.hidden_bounds_checks);
        assert!(!MANUAL_DMA_CONTRACT.hidden_recovery);
    }

    #[test]
    fn cache_maintenance_bytes_cover_x86_arm_and_riscv() {
        assert_eq!(
            DmaCachePrimitive::X86ClflushRax.encode(),
            [0x0f, 0xae, 0x38]
        );
        assert_eq!(
            DmaCachePrimitive::X86ClwbRax.encode(),
            [0x66, 0x0f, 0xae, 0x30]
        );
        assert_eq!(
            DmaCachePrimitive::Aarch64DcCvacX0.encode(),
            [0x20, 0x7a, 0x0b, 0xd5]
        );
        assert_eq!(
            DmaCachePrimitive::Aarch64DcCivacX0.encode(),
            [0x20, 0x7e, 0x0b, 0xd5]
        );
        assert_eq!(
            DmaCachePrimitive::Armv7DccimvacR0.encode(),
            [0x3e, 0x0f, 0x07, 0xee]
        );
        assert_eq!(
            DmaCachePrimitive::RiscvCboCleanA0.encode(),
            [0x0f, 0x20, 0x15, 0x00]
        );
        assert_eq!(
            DmaCachePrimitive::RiscvCboFlushA0.encode(),
            [0x0f, 0x20, 0x25, 0x00]
        );
    }

    #[test]
    fn every_dma_target_requires_explicit_cache_line_size_and_policy() {
        for target in [
            "x86_64-unknown-none",
            "i386-unknown-none-elf",
            "aarch64-unknown-none",
            "armv7a-none-eabi",
            "riscv32imac-unknown-none-elf",
            "riscv64gc-unknown-none-elf",
        ] {
            let plan = dma_cache_plan(target).expect(target);
            assert!(plan.caller_supplies_cache_line_bytes);
            assert!(!plan.hidden_coherency_detection);
            assert!(!plan.clean.is_empty());
            assert!(!plan.invalidate.is_empty());
            assert!(!plan.clean_invalidate.is_empty());
        }
    }

    #[test]
    fn iommu_command_entries_preserve_all_caller_bits() {
        let raw = RawIommuCommand::from_words(0x1122_3344_5566_7788, 0x99aa_bbcc_ddee_ff00);
        assert_eq!(
            raw.to_le_bytes()[0..8],
            0x1122_3344_5566_7788u64.to_le_bytes()
        );
        assert_eq!(
            raw.to_le_bytes()[8..16],
            0x99aa_bbcc_ddee_ff00u64.to_le_bytes()
        );
        assert_eq!(arm_smmuv3_command(ARM_SMMUV3_CMD_SYNC, 0, 0).low, 0x46);
        assert_eq!(riscv_iommu_command(RISCV_IOMMU_CMD_IOFENCE, 0, 0, 0).low, 2);
    }
}
