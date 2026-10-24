use core::fmt;

pub const RISCV_H_EXTENSION_MISA_BIT: u64 = 1 << 7;
pub const RISCV_G_STAGE_ROOT_ALIGNMENT: u64 = 16 * 1024;
pub const RISCV_PAGE_SIZE_4K: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiscvHypervisorCsr {
    pub name: &'static str,
    pub address: u16,
    pub minimum_xlen: u8,
}

macro_rules! h_csr {
    ($name:ident, $spelling:literal, $address:literal, $xlen:literal) => {
        pub const $name: RiscvHypervisorCsr = RiscvHypervisorCsr {
            name: $spelling,
            address: $address,
            minimum_xlen: $xlen,
        };
    };
}

h_csr!(VSSTATUS, "vsstatus", 0x200, 32);
h_csr!(VSIE, "vsie", 0x204, 32);
h_csr!(VSTVEC, "vstvec", 0x205, 32);
h_csr!(VSSCRATCH, "vsscratch", 0x240, 32);
h_csr!(VSEPC, "vsepc", 0x241, 32);
h_csr!(VSCAUSE, "vscause", 0x242, 32);
h_csr!(VSTVAL, "vstval", 0x243, 32);
h_csr!(VSIP, "vsip", 0x244, 32);
h_csr!(VSATP, "vsatp", 0x280, 32);
h_csr!(HSTATUS, "hstatus", 0x600, 32);
h_csr!(HEDELEG, "hedeleg", 0x602, 32);
h_csr!(HIDELEG, "hideleg", 0x603, 32);
h_csr!(HIE, "hie", 0x604, 32);
h_csr!(HTIMEDELTA, "htimedelta", 0x605, 32);
h_csr!(HCOUNTEREN, "hcounteren", 0x606, 32);
h_csr!(HGEIE, "hgeie", 0x607, 32);
h_csr!(HENVCFG, "henvcfg", 0x60a, 32);
h_csr!(HTIMEDELTAH, "htimedeltah", 0x615, 32);
h_csr!(HENVCFGH, "henvcfgh", 0x61a, 32);
h_csr!(HTVAL, "htval", 0x643, 32);
h_csr!(HIP, "hip", 0x644, 32);
h_csr!(HVIP, "hvip", 0x645, 32);
h_csr!(HTINST, "htinst", 0x64a, 32);
h_csr!(HGATP, "hgatp", 0x680, 32);
h_csr!(HCONTEXT, "hcontext", 0x6a8, 32);
h_csr!(HGEIP, "hgeip", 0xe12, 32);

pub const RISCV_HYPERVISOR_CSRS: &[RiscvHypervisorCsr] = &[
    VSSTATUS,
    VSIE,
    VSTVEC,
    VSSCRATCH,
    VSEPC,
    VSCAUSE,
    VSTVAL,
    VSIP,
    VSATP,
    HSTATUS,
    HEDELEG,
    HIDELEG,
    HIE,
    HTIMEDELTA,
    HCOUNTEREN,
    HGEIE,
    HENVCFG,
    HTIMEDELTAH,
    HENVCFGH,
    HTVAL,
    HIP,
    HVIP,
    HTINST,
    HGATP,
    HCONTEXT,
    HGEIP,
];

pub fn riscv_hypervisor_csr(name: &str) -> Option<RiscvHypervisorCsr> {
    RISCV_HYPERVISOR_CSRS
        .iter()
        .copied()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiscvHypervisorInstruction {
    Csrr { rd: u8, csr: RiscvHypervisorCsr },
    Csrw { csr: RiscvHypervisorCsr, rs: u8 },
    HfenceVvma { virtual_address: u8, asid: u8 },
    HfenceGvma { guest_address: u8, vmid: u8 },
    HinvalVvma { virtual_address: u8, asid: u8 },
    HinvalGvma { guest_address: u8, vmid: u8 },
    HlvB { rd: u8, address: u8 },
    HlvBu { rd: u8, address: u8 },
    HlvH { rd: u8, address: u8 },
    HlvHu { rd: u8, address: u8 },
    HlvW { rd: u8, address: u8 },
    HlvWu { rd: u8, address: u8 },
    HlvD { rd: u8, address: u8 },
    HsvB { source: u8, address: u8 },
    HsvH { source: u8, address: u8 },
    HsvW { source: u8, address: u8 },
    HsvD { source: u8, address: u8 },
}

impl RiscvHypervisorInstruction {
    pub fn encode(self) -> Result<u32, RiscvHypervisorError> {
        match self {
            Self::Csrr { rd, csr } => {
                validate_register(rd)?;
                Ok((csr.address as u32) << 20 | (2 << 12) | (rd as u32) << 7 | 0x73)
            }
            Self::Csrw { csr, rs } => {
                validate_register(rs)?;
                Ok((csr.address as u32) << 20 | (rs as u32) << 15 | (1 << 12) | 0x73)
            }
            Self::HfenceVvma {
                virtual_address,
                asid,
            } => encode_fence(0x2200_0073, virtual_address, asid),
            Self::HfenceGvma {
                guest_address,
                vmid,
            } => encode_fence(0x6200_0073, guest_address, vmid),
            Self::HinvalVvma {
                virtual_address,
                asid,
            } => encode_fence(0x2600_0073, virtual_address, asid),
            Self::HinvalGvma {
                guest_address,
                vmid,
            } => encode_fence(0x6600_0073, guest_address, vmid),
            Self::HlvB { rd, address } => encode_hlv(0x6000_4073, rd, address),
            Self::HlvBu { rd, address } => encode_hlv(0x6010_4073, rd, address),
            Self::HlvH { rd, address } => encode_hlv(0x6400_4073, rd, address),
            Self::HlvHu { rd, address } => encode_hlv(0x6410_4073, rd, address),
            Self::HlvW { rd, address } => encode_hlv(0x6800_4073, rd, address),
            Self::HlvWu { rd, address } => encode_hlv(0x6810_4073, rd, address),
            Self::HlvD { rd, address } => encode_hlv(0x6c00_4073, rd, address),
            Self::HsvB { source, address } => encode_hsv(0x6200_4073, source, address),
            Self::HsvH { source, address } => encode_hsv(0x6600_4073, source, address),
            Self::HsvW { source, address } => encode_hsv(0x6a00_4073, source, address),
            Self::HsvD { source, address } => encode_hsv(0x6e00_4073, source, address),
        }
    }
}

fn validate_register(register: u8) -> Result<(), RiscvHypervisorError> {
    if register < 32 {
        Ok(())
    } else {
        Err(RiscvHypervisorError::InvalidRegister(register))
    }
}

fn encode_fence(base: u32, first: u8, second: u8) -> Result<u32, RiscvHypervisorError> {
    validate_register(first)?;
    validate_register(second)?;
    Ok(base | (first as u32) << 15 | (second as u32) << 20)
}

fn encode_hlv(base: u32, rd: u8, address: u8) -> Result<u32, RiscvHypervisorError> {
    validate_register(rd)?;
    validate_register(address)?;
    Ok(base | (rd as u32) << 7 | (address as u32) << 15)
}

fn encode_hsv(base: u32, source: u8, address: u8) -> Result<u32, RiscvHypervisorError> {
    validate_register(source)?;
    validate_register(address)?;
    Ok(base | (address as u32) << 15 | (source as u32) << 20)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RiscvGStageMode {
    Bare,
    Sv32x4,
    Sv39x4,
    Sv48x4,
    Sv57x4,
}

impl RiscvGStageMode {
    pub const fn xlen(self) -> u8 {
        match self {
            Self::Sv32x4 => 32,
            _ => 64,
        }
    }

    pub const fn levels(self) -> u8 {
        match self {
            Self::Bare => 0,
            Self::Sv32x4 => 2,
            Self::Sv39x4 => 3,
            Self::Sv48x4 => 4,
            Self::Sv57x4 => 5,
        }
    }

    const fn rv64_encoding(self) -> Option<u64> {
        match self {
            Self::Bare => Some(0),
            Self::Sv39x4 => Some(8),
            Self::Sv48x4 => Some(9),
            Self::Sv57x4 => Some(10),
            Self::Sv32x4 => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RiscvHgatp(u64);

impl RiscvHgatp {
    pub fn rv64(
        mode: RiscvGStageMode,
        vmid: u16,
        root_physical: u64,
    ) -> Result<Self, RiscvHypervisorError> {
        let mode = mode
            .rv64_encoding()
            .ok_or(RiscvHypervisorError::ModeXlenMismatch)?;
        if vmid >= 1 << 14 {
            return Err(RiscvHypervisorError::InvalidVmid(vmid as u32));
        }
        validate_root(root_physical)?;
        let ppn = root_physical >> 12;
        if ppn >= 1u64 << 44 {
            return Err(RiscvHypervisorError::PhysicalAddressWidth(root_physical));
        }
        Ok(Self((mode << 60) | ((vmid as u64) << 44) | ppn))
    }

    pub fn rv32(
        mode: RiscvGStageMode,
        vmid: u8,
        root_physical: u64,
    ) -> Result<Self, RiscvHypervisorError> {
        if !matches!(mode, RiscvGStageMode::Bare | RiscvGStageMode::Sv32x4) {
            return Err(RiscvHypervisorError::ModeXlenMismatch);
        }
        if vmid >= 1 << 7 {
            return Err(RiscvHypervisorError::InvalidVmid(vmid as u32));
        }
        validate_root(root_physical)?;
        let ppn = root_physical >> 12;
        if ppn >= 1u64 << 22 {
            return Err(RiscvHypervisorError::PhysicalAddressWidth(root_physical));
        }
        let encoded_mode = u64::from(mode == RiscvGStageMode::Sv32x4);
        Ok(Self((encoded_mode << 31) | ((vmid as u64) << 22) | ppn))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

fn validate_root(root_physical: u64) -> Result<(), RiscvHypervisorError> {
    if root_physical & (RISCV_G_STAGE_ROOT_ALIGNMENT - 1) != 0 {
        return Err(RiscvHypervisorError::MisalignedRoot(root_physical));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiscvGStagePte(u64);

impl RiscvGStagePte {
    pub fn table(next_table: u64) -> Result<Self, RiscvHypervisorError> {
        if next_table & (RISCV_PAGE_SIZE_4K - 1) != 0 {
            return Err(RiscvHypervisorError::MisalignedPage(next_table));
        }
        Ok(Self(((next_table >> 12) << 10) | 1))
    }

    pub fn leaf(
        output_address: u64,
        read: bool,
        write: bool,
        execute: bool,
        user: bool,
        global: bool,
        accessed: bool,
        dirty: bool,
        pbmt: u8,
    ) -> Result<Self, RiscvHypervisorError> {
        if output_address & (RISCV_PAGE_SIZE_4K - 1) != 0 {
            return Err(RiscvHypervisorError::MisalignedPage(output_address));
        }
        if write && !read {
            return Err(RiscvHypervisorError::WriteWithoutRead);
        }
        if pbmt > 3 {
            return Err(RiscvHypervisorError::InvalidPbmt(pbmt));
        }
        Ok(Self(
            ((output_address >> 12) << 10)
                | 1
                | (u64::from(read) << 1)
                | (u64::from(write) << 2)
                | (u64::from(execute) << 3)
                | (u64::from(user) << 4)
                | (u64::from(global) << 5)
                | (u64::from(accessed) << 6)
                | (u64::from(dirty) << 7)
                | ((pbmt as u64) << 61),
        ))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiscvHypervisorTrap {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    UserEnvironmentCall,
    SupervisorEnvironmentCall,
    VirtualSupervisorEnvironmentCall,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    GuestInstructionPageFault,
    GuestLoadPageFault,
    VirtualInstruction,
    GuestStorePageFault,
    Interrupt(u64),
    Other(u64),
}

impl RiscvHypervisorTrap {
    pub const fn decode(scause: u64, xlen: u8) -> Self {
        let interrupt_bit = if xlen == 32 { 1u64 << 31 } else { 1u64 << 63 };
        if scause & interrupt_bit != 0 {
            return Self::Interrupt(scause & !interrupt_bit);
        }
        match scause {
            0 => Self::InstructionAddressMisaligned,
            1 => Self::InstructionAccessFault,
            2 => Self::IllegalInstruction,
            3 => Self::Breakpoint,
            4 => Self::LoadAddressMisaligned,
            5 => Self::LoadAccessFault,
            6 => Self::StoreAddressMisaligned,
            7 => Self::StoreAccessFault,
            8 => Self::UserEnvironmentCall,
            9 => Self::SupervisorEnvironmentCall,
            10 => Self::VirtualSupervisorEnvironmentCall,
            12 => Self::InstructionPageFault,
            13 => Self::LoadPageFault,
            15 => Self::StorePageFault,
            20 => Self::GuestInstructionPageFault,
            21 => Self::GuestLoadPageFault,
            22 => Self::VirtualInstruction,
            23 => Self::GuestStorePageFault,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RiscvHypervisorExitSnapshot {
    pub scause: u64,
    pub sepc: u64,
    pub stval: u64,
    pub htval: u64,
    pub htinst: u64,
    pub hstatus: u64,
}

impl RiscvHypervisorExitSnapshot {
    pub const fn trap(self, xlen: u8) -> RiscvHypervisorTrap {
        RiscvHypervisorTrap::decode(self.scause, xlen)
    }

    pub const fn guest_physical_address(self) -> u64 {
        (self.htval << 2) | (self.stval & 0x3)
    }

    pub const fn guest_virtual_mode(self) -> bool {
        self.hstatus & (1 << 7) != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiscvHypervisorError {
    InvalidRegister(u8),
    InvalidVmid(u32),
    ModeXlenMismatch,
    MisalignedRoot(u64),
    MisalignedPage(u64),
    PhysicalAddressWidth(u64),
    WriteWithoutRead,
    InvalidPbmt(u8),
}

impl fmt::Display for RiscvHypervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegister(register) => write!(f, "invalid RISC-V register x{register}"),
            Self::InvalidVmid(vmid) => write!(f, "invalid RISC-V VMID {vmid}"),
            Self::ModeXlenMismatch => write!(f, "G-stage mode does not match XLEN"),
            Self::MisalignedRoot(root) => write!(
                f,
                "G-stage root {root:#x} is not aligned to four base pages"
            ),
            Self::MisalignedPage(page) => {
                write!(f, "G-stage page {page:#x} is not 4 KiB aligned")
            }
            Self::PhysicalAddressWidth(address) => {
                write!(
                    f,
                    "G-stage physical address {address:#x} exceeds mode width"
                )
            }
            Self::WriteWithoutRead => write!(f, "RISC-V leaf PTE cannot set W without R"),
            Self::InvalidPbmt(pbmt) => write!(f, "invalid PBMT encoding {pbmt}"),
        }
    }
}

impl std::error::Error for RiscvHypervisorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csr_and_h_extension_words_match_llvm_mc() {
        assert_eq!(
            RiscvHypervisorInstruction::Csrr {
                rd: 1,
                csr: HSTATUS
            }
            .encode()
            .unwrap(),
            0x6000_20f3
        );
        assert_eq!(
            RiscvHypervisorInstruction::Csrw {
                csr: HSTATUS,
                rs: 2
            }
            .encode()
            .unwrap(),
            0x6001_1073
        );
        assert_eq!(
            RiscvHypervisorInstruction::HfenceVvma {
                virtual_address: 5,
                asid: 6
            }
            .encode()
            .unwrap(),
            0x2262_8073
        );
        assert_eq!(
            RiscvHypervisorInstruction::HfenceGvma {
                guest_address: 7,
                vmid: 8
            }
            .encode()
            .unwrap(),
            0x6283_8073
        );
    }

    #[test]
    fn virtual_memory_access_words_match_llvm_mc() {
        assert_eq!(
            RiscvHypervisorInstruction::HlvB { rd: 9, address: 10 }
                .encode()
                .unwrap(),
            0x6005_44f3
        );
        assert_eq!(
            RiscvHypervisorInstruction::HlvD {
                rd: 21,
                address: 22
            }
            .encode()
            .unwrap(),
            0x6c0b_4af3
        );
        assert_eq!(
            RiscvHypervisorInstruction::HsvD {
                source: 29,
                address: 30
            }
            .encode()
            .unwrap(),
            0x6fdf_4073
        );
        assert_eq!(
            RiscvHypervisorInstruction::HinvalGvma {
                guest_address: 7,
                vmid: 8
            }
            .encode()
            .unwrap(),
            0x6683_8073
        );
    }

    #[test]
    fn hgatp_and_g_stage_pte_validate_architectural_rules() {
        let hgatp = RiscvHgatp::rv64(RiscvGStageMode::Sv48x4, 0x123, 0x4000).unwrap();
        assert_eq!(hgatp.raw() >> 60, 9);
        assert_eq!((hgatp.raw() >> 44) & 0x3fff, 0x123);
        let pte = RiscvGStagePte::leaf(0x8000_0000, true, true, true, false, false, true, true, 0)
            .unwrap();
        assert_eq!(pte.raw() & 0xff, 0xcf);
        assert_eq!(
            RiscvGStagePte::leaf(0, false, true, false, false, false, true, true, 0),
            Err(RiscvHypervisorError::WriteWithoutRead)
        );
    }

    #[test]
    fn guest_page_fault_snapshot_recovers_address() {
        let snapshot = RiscvHypervisorExitSnapshot {
            scause: 21,
            sepc: 0x8000,
            stval: 2,
            htval: 0x1234,
            htinst: 0,
            hstatus: 1 << 7,
        };
        assert_eq!(snapshot.trap(64), RiscvHypervisorTrap::GuestLoadPageFault);
        assert_eq!(snapshot.guest_physical_address(), 0x48d2);
        assert!(snapshot.guest_virtual_mode());
    }
}
