use core::fmt;

pub const AARCH64_STAGE2_PAGE_SIZE_4K: u64 = 1 << 12;
pub const AARCH64_STAGE2_PAGE_SIZE_16K: u64 = 1 << 14;
pub const AARCH64_STAGE2_PAGE_SIZE_64K: u64 = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aarch64SystemRegister {
    pub name: &'static str,
    pub op0: u8,
    pub op1: u8,
    pub crn: u8,
    pub crm: u8,
    pub op2: u8,
}

impl Aarch64SystemRegister {
    pub const fn encoding(self) -> u32 {
        ((self.op0 as u32 & 0x3) << 19)
            | ((self.op1 as u32 & 0x7) << 16)
            | ((self.crn as u32 & 0xf) << 12)
            | ((self.crm as u32 & 0xf) << 8)
            | ((self.op2 as u32 & 0x7) << 5)
    }

    pub const fn encode_mrs(self, destination: u8) -> u32 {
        0xd520_0000 | self.encoding() | destination as u32 & 0x1f
    }

    pub const fn encode_msr(self, source: u8) -> u32 {
        0xd500_0000 | self.encoding() | source as u32 & 0x1f
    }
}

macro_rules! el2_register {
    ($name:ident, $spelling:literal, $op0:literal, $op1:literal, $crn:literal, $crm:literal, $op2:literal) => {
        pub const $name: Aarch64SystemRegister = Aarch64SystemRegister {
            name: $spelling,
            op0: $op0,
            op1: $op1,
            crn: $crn,
            crm: $crm,
            op2: $op2,
        };
    };
}

el2_register!(SCTLR_EL2, "sctlr_el2", 3, 4, 1, 0, 0);
el2_register!(ACTLR_EL2, "actlr_el2", 3, 4, 1, 0, 1);
el2_register!(HCR_EL2, "hcr_el2", 3, 4, 1, 1, 0);
el2_register!(MDCR_EL2, "mdcr_el2", 3, 4, 1, 1, 1);
el2_register!(CPTR_EL2, "cptr_el2", 3, 4, 1, 1, 2);
el2_register!(HSTR_EL2, "hstr_el2", 3, 4, 1, 1, 3);
el2_register!(HACR_EL2, "hacr_el2", 3, 4, 1, 1, 7);
el2_register!(TTBR0_EL2, "ttbr0_el2", 3, 4, 2, 0, 0);
el2_register!(TCR_EL2, "tcr_el2", 3, 4, 2, 0, 2);
el2_register!(VTTBR_EL2, "vttbr_el2", 3, 4, 2, 1, 0);
el2_register!(VTCR_EL2, "vtcr_el2", 3, 4, 2, 1, 2);
el2_register!(VSTTBR_EL2, "vsttbr_el2", 3, 4, 2, 6, 0);
el2_register!(VSTCR_EL2, "vstcr_el2", 3, 4, 2, 6, 2);
el2_register!(SPSR_EL2, "spsr_el2", 3, 4, 4, 0, 0);
el2_register!(ELR_EL2, "elr_el2", 3, 4, 4, 0, 1);
el2_register!(ESR_EL2, "esr_el2", 3, 4, 5, 2, 0);
el2_register!(VSESR_EL2, "vsesr_el2", 3, 4, 5, 2, 3);
el2_register!(FAR_EL2, "far_el2", 3, 4, 6, 0, 0);
el2_register!(HPFAR_EL2, "hpfar_el2", 3, 4, 6, 0, 4);
el2_register!(MAIR_EL2, "mair_el2", 3, 4, 10, 2, 0);
el2_register!(AMAIR_EL2, "amair_el2", 3, 4, 10, 3, 0);
el2_register!(VBAR_EL2, "vbar_el2", 3, 4, 12, 0, 0);
el2_register!(RVBAR_EL2, "rvbar_el2", 3, 4, 12, 0, 1);
el2_register!(RMR_EL2, "rmr_el2", 3, 4, 12, 0, 2);
el2_register!(VPIDR_EL2, "vpidr_el2", 3, 4, 0, 0, 0);
el2_register!(VMPIDR_EL2, "vmpidr_el2", 3, 4, 0, 0, 5);
el2_register!(CNTVOFF_EL2, "cntvoff_el2", 3, 4, 14, 0, 3);
el2_register!(CNTHCTL_EL2, "cnthctl_el2", 3, 4, 14, 1, 0);
el2_register!(CNTHP_TVAL_EL2, "cnthp_tval_el2", 3, 4, 14, 2, 0);
el2_register!(CNTHP_CTL_EL2, "cnthp_ctl_el2", 3, 4, 14, 2, 1);
el2_register!(CNTHP_CVAL_EL2, "cnthp_cval_el2", 3, 4, 14, 2, 2);
el2_register!(ICH_HCR_EL2, "ich_hcr_el2", 3, 4, 12, 11, 0);
el2_register!(ICH_VMCR_EL2, "ich_vmcr_el2", 3, 4, 12, 11, 7);
el2_register!(ICH_VTR_EL2, "ich_vtr_el2", 3, 4, 12, 11, 1);
el2_register!(ICH_MISR_EL2, "ich_misr_el2", 3, 4, 12, 11, 2);
el2_register!(ICH_EISR_EL2, "ich_eisr_el2", 3, 4, 12, 11, 3);
el2_register!(ICH_ELRSR_EL2, "ich_elrsr_el2", 3, 4, 12, 11, 5);

el2_register!(MPIDR_EL1, "mpidr_el1", 3, 0, 0, 0, 5);
el2_register!(SCTLR_EL1, "sctlr_el1", 3, 0, 1, 0, 0);
el2_register!(TTBR0_EL1, "ttbr0_el1", 3, 0, 2, 0, 0);
el2_register!(TTBR1_EL1, "ttbr1_el1", 3, 0, 2, 0, 1);
el2_register!(TCR_EL1, "tcr_el1", 3, 0, 2, 0, 2);
el2_register!(VBAR_EL1, "vbar_el1", 3, 0, 12, 0, 0);
el2_register!(TPIDR_EL1, "tpidr_el1", 3, 0, 13, 0, 4);
el2_register!(ICC_PMR_EL1, "icc_pmr_el1", 3, 0, 4, 6, 0);
el2_register!(ICC_IAR1_EL1, "icc_iar1_el1", 3, 0, 12, 12, 0);
el2_register!(ICC_EOIR1_EL1, "icc_eoir1_el1", 3, 0, 12, 12, 1);
el2_register!(ICC_SRE_EL1, "icc_sre_el1", 3, 0, 12, 12, 5);
el2_register!(ICC_IGRPEN1_EL1, "icc_igrpen1_el1", 3, 0, 12, 12, 7);
el2_register!(ICC_SGI1R_EL1, "icc_sgi1r_el1", 3, 0, 12, 11, 5);
el2_register!(CNTFRQ_EL0, "cntfrq_el0", 3, 3, 14, 0, 0);
el2_register!(CNTPCT_EL0, "cntpct_el0", 3, 3, 14, 0, 1);
el2_register!(CNTVCT_EL0, "cntvct_el0", 3, 3, 14, 0, 2);
el2_register!(CNTV_TVAL_EL0, "cntv_tval_el0", 3, 3, 14, 3, 0);
el2_register!(CNTV_CTL_EL0, "cntv_ctl_el0", 3, 3, 14, 3, 1);
el2_register!(CNTV_CVAL_EL0, "cntv_cval_el0", 3, 3, 14, 3, 2);

pub const AARCH64_EL2_REGISTERS: &[Aarch64SystemRegister] = &[
    SCTLR_EL2,
    ACTLR_EL2,
    HCR_EL2,
    MDCR_EL2,
    CPTR_EL2,
    HSTR_EL2,
    HACR_EL2,
    TTBR0_EL2,
    TCR_EL2,
    VTTBR_EL2,
    VTCR_EL2,
    VSTTBR_EL2,
    VSTCR_EL2,
    SPSR_EL2,
    ELR_EL2,
    ESR_EL2,
    VSESR_EL2,
    FAR_EL2,
    HPFAR_EL2,
    MAIR_EL2,
    AMAIR_EL2,
    VBAR_EL2,
    RVBAR_EL2,
    RMR_EL2,
    VPIDR_EL2,
    VMPIDR_EL2,
    CNTVOFF_EL2,
    CNTHCTL_EL2,
    CNTHP_TVAL_EL2,
    CNTHP_CTL_EL2,
    CNTHP_CVAL_EL2,
    ICH_HCR_EL2,
    ICH_VMCR_EL2,
    ICH_VTR_EL2,
    ICH_MISR_EL2,
    ICH_EISR_EL2,
    ICH_ELRSR_EL2,
];

pub const AARCH64_KERNEL_REGISTERS: &[Aarch64SystemRegister] = &[
    MPIDR_EL1,
    SCTLR_EL1,
    TTBR0_EL1,
    TTBR1_EL1,
    TCR_EL1,
    VBAR_EL1,
    TPIDR_EL1,
    ICC_PMR_EL1,
    ICC_IAR1_EL1,
    ICC_EOIR1_EL1,
    ICC_SRE_EL1,
    ICC_IGRPEN1_EL1,
    ICC_SGI1R_EL1,
    CNTFRQ_EL0,
    CNTPCT_EL0,
    CNTVCT_EL0,
    CNTV_TVAL_EL0,
    CNTV_CTL_EL0,
    CNTV_CVAL_EL0,
];

pub const fn aarch64_el2_register(name: &str) -> Option<Aarch64SystemRegister> {
    let mut index = 0;
    while index < AARCH64_EL2_REGISTERS.len() {
        let candidate = AARCH64_EL2_REGISTERS[index];
        if const_str_eq(candidate.name.as_bytes(), name.as_bytes()) {
            return Some(candidate);
        }
        index += 1;
    }
    None
}

pub const fn aarch64_system_register(name: &str) -> Option<Aarch64SystemRegister> {
    if let Some(register) = aarch64_el2_register(name) {
        return Some(register);
    }
    let mut index = 0;
    while index < AARCH64_KERNEL_REGISTERS.len() {
        let candidate = AARCH64_KERNEL_REGISTERS[index];
        if const_str_eq(candidate.name.as_bytes(), name.as_bytes()) {
            return Some(candidate);
        }
        index += 1;
    }
    None
}

const fn const_str_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aarch64El2Instruction {
    Hvc(u16),
    Smc(u16),
    Eret,
    Wfi,
    Wfe,
    TlbiVmalls12e1is,
    TlbiVmalls12e1,
    TlbiIpas2e1is(u8),
    TlbiIpas2le1is(u8),
    AtS12e1r(u8),
    AtS12e1w(u8),
}

impl Aarch64El2Instruction {
    pub const fn encode(self) -> u32 {
        match self {
            Self::Hvc(immediate) => 0xd400_0002 | ((immediate as u32) << 5),
            Self::Smc(immediate) => 0xd400_0003 | ((immediate as u32) << 5),
            Self::Eret => 0xd69f_03e0,
            Self::Wfi => 0xd503_207f,
            Self::Wfe => 0xd503_205f,
            Self::TlbiVmalls12e1is => 0xd50c_83df,
            Self::TlbiVmalls12e1 => 0xd50c_87df,
            Self::TlbiIpas2e1is(register) => 0xd50c_8020 | register as u32 & 0x1f,
            Self::TlbiIpas2le1is(register) => 0xd50c_80a0 | register as u32 & 0x1f,
            Self::AtS12e1r(register) => 0xd50c_7880 | register as u32 & 0x1f,
            Self::AtS12e1w(register) => 0xd50c_78a0 | register as u32 & 0x1f,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Aarch64Stage2Granule {
    Size4K,
    Size16K,
    Size64K,
}

impl Aarch64Stage2Granule {
    pub const fn bytes(self) -> u64 {
        match self {
            Self::Size4K => AARCH64_STAGE2_PAGE_SIZE_4K,
            Self::Size16K => AARCH64_STAGE2_PAGE_SIZE_16K,
            Self::Size64K => AARCH64_STAGE2_PAGE_SIZE_64K,
        }
    }

    const fn tg0(self) -> u64 {
        match self {
            Self::Size4K => 0,
            Self::Size64K => 1,
            Self::Size16K => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aarch64Stage2Config {
    pub granule: Aarch64Stage2Granule,
    pub input_address_bits: u8,
    pub physical_address_bits: u8,
    pub starting_level: u8,
    pub vmid_16bit: bool,
    pub hardware_access_flag: bool,
    pub hardware_dirty_state: bool,
}

impl Aarch64Stage2Config {
    pub const DEFAULT_4K_48BIT: Self = Self {
        granule: Aarch64Stage2Granule::Size4K,
        input_address_bits: 48,
        physical_address_bits: 48,
        starting_level: 0,
        vmid_16bit: true,
        hardware_access_flag: true,
        hardware_dirty_state: true,
    };

    pub fn vtcr_el2(self) -> Result<u64, Aarch64El2Error> {
        if !(32..=52).contains(&self.input_address_bits) {
            return Err(Aarch64El2Error::InvalidInputAddressBits(
                self.input_address_bits,
            ));
        }
        let ps = match self.physical_address_bits {
            32 => 0,
            36 => 1,
            40 => 2,
            42 => 3,
            44 => 4,
            48 => 5,
            52 => 6,
            bits => return Err(Aarch64El2Error::InvalidPhysicalAddressBits(bits)),
        };
        let sl0 = match (self.granule, self.starting_level) {
            (Aarch64Stage2Granule::Size4K, 2) => 0,
            (Aarch64Stage2Granule::Size4K, 1) => 1,
            (Aarch64Stage2Granule::Size4K, 0) => 2,
            (Aarch64Stage2Granule::Size4K, 3) => 3,
            (Aarch64Stage2Granule::Size16K, 2) => 0,
            (Aarch64Stage2Granule::Size16K, 1) => 1,
            (Aarch64Stage2Granule::Size16K, 0) => 2,
            (Aarch64Stage2Granule::Size64K, 2) => 0,
            (Aarch64Stage2Granule::Size64K, 1) => 1,
            pair => return Err(Aarch64El2Error::InvalidStartingLevel(pair.1)),
        };
        let t0sz = 64 - self.input_address_bits as u64;
        Ok(t0sz
            | (sl0 << 6)
            | (1 << 8)
            | (1 << 10)
            | (3 << 12)
            | (self.granule.tg0() << 14)
            | (ps << 16)
            | (u64::from(self.vmid_16bit) << 19)
            | (u64::from(self.hardware_access_flag) << 21)
            | (u64::from(self.hardware_dirty_state) << 22))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Aarch64Stage2Access {
    None = 0,
    Read = 1,
    Write = 2,
    ReadWrite = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Aarch64Stage2MemoryAttribute {
    DeviceNGnRnE = 0x0,
    DeviceNGnRE = 0x1,
    DeviceNGRE = 0x2,
    DeviceGRE = 0x3,
    NormalOuterNonCacheable = 0x4,
    NormalWriteThrough = 0xa,
    NormalWriteBack = 0xf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Aarch64Stage2Descriptor(u64);

impl Aarch64Stage2Descriptor {
    pub fn table(next_table: u64, granule: Aarch64Stage2Granule) -> Result<Self, Aarch64El2Error> {
        validate_output_address(next_table, granule)?;
        Ok(Self(next_table | 0b11))
    }

    pub fn block(
        output_address: u64,
        level: u8,
        granule: Aarch64Stage2Granule,
        access: Aarch64Stage2Access,
        memory: Aarch64Stage2MemoryAttribute,
        execute_never: bool,
    ) -> Result<Self, Aarch64El2Error> {
        if level == 0 || level > 2 {
            return Err(Aarch64El2Error::InvalidBlockLevel(level));
        }
        Self::leaf(output_address, granule, 0b01, access, memory, execute_never)
    }

    pub fn page(
        output_address: u64,
        granule: Aarch64Stage2Granule,
        access: Aarch64Stage2Access,
        memory: Aarch64Stage2MemoryAttribute,
        execute_never: bool,
    ) -> Result<Self, Aarch64El2Error> {
        Self::leaf(output_address, granule, 0b11, access, memory, execute_never)
    }

    fn leaf(
        output_address: u64,
        granule: Aarch64Stage2Granule,
        descriptor_type: u64,
        access: Aarch64Stage2Access,
        memory: Aarch64Stage2MemoryAttribute,
        execute_never: bool,
    ) -> Result<Self, Aarch64El2Error> {
        validate_output_address(output_address, granule)?;
        Ok(Self(
            output_address
                | descriptor_type
                | ((memory as u64) << 2)
                | ((access as u64) << 6)
                | (3 << 8)
                | (1 << 10)
                | (u64::from(execute_never) << 54),
        ))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

fn validate_output_address(
    output_address: u64,
    granule: Aarch64Stage2Granule,
) -> Result<(), Aarch64El2Error> {
    if output_address >= 1u64 << 52 {
        return Err(Aarch64El2Error::OutputAddressWidth(output_address));
    }
    if output_address & (granule.bytes() - 1) != 0 {
        return Err(Aarch64El2Error::MisalignedOutputAddress {
            address: output_address,
            alignment: granule.bytes(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aarch64VttbrEl2(u64);

impl Aarch64VttbrEl2 {
    pub fn new(root: u64, vmid: u16) -> Result<Self, Aarch64El2Error> {
        validate_output_address(root, Aarch64Stage2Granule::Size4K)?;
        Ok(Self((vmid as u64) << 48 | root))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aarch64El2ExitClass {
    Unknown,
    WfiOrWfe,
    TrappedSystemRegister,
    SupervisorCall,
    HypervisorCall,
    SecureMonitorCall,
    InstructionAbort,
    DataAbort,
    PcAlignment,
    SpAlignment,
    FloatingPoint,
    Sve,
    Breakpoint,
    SoftwareStep,
    Watchpoint,
    BreakpointInstruction,
    SError,
    Other(u8),
}

impl Aarch64El2ExitClass {
    pub const fn decode(esr_el2: u64) -> Self {
        match ((esr_el2 >> 26) & 0x3f) as u8 {
            0x00 => Self::Unknown,
            0x01 => Self::WfiOrWfe,
            0x07 => Self::FloatingPoint,
            0x19 => Self::Sve,
            0x18 => Self::TrappedSystemRegister,
            0x11 | 0x15 => Self::SupervisorCall,
            0x12 | 0x16 => Self::HypervisorCall,
            0x13 | 0x17 => Self::SecureMonitorCall,
            0x20 | 0x21 => Self::InstructionAbort,
            0x22 => Self::PcAlignment,
            0x24 | 0x25 => Self::DataAbort,
            0x26 => Self::SpAlignment,
            0x2f => Self::SError,
            0x30 | 0x31 => Self::Breakpoint,
            0x32 | 0x33 => Self::SoftwareStep,
            0x34 | 0x35 => Self::Watchpoint,
            0x3c => Self::BreakpointInstruction,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Aarch64El2ExitSnapshot {
    pub esr_el2: u64,
    pub far_el2: u64,
    pub hpfar_el2: u64,
    pub elr_el2: u64,
    pub spsr_el2: u64,
}

impl Aarch64El2ExitSnapshot {
    pub const fn exception_class(self) -> Aarch64El2ExitClass {
        Aarch64El2ExitClass::decode(self.esr_el2)
    }

    pub const fn instruction_length(self) -> u8 {
        if self.esr_el2 & (1 << 25) != 0 {
            4
        } else {
            2
        }
    }

    pub const fn syndrome(self) -> u32 {
        (self.esr_el2 & 0x01ff_ffff) as u32
    }

    pub const fn fault_ipa(self) -> u64 {
        ((self.hpfar_el2 & 0x0000_ffff_ffff_fff0) << 8) | (self.far_el2 & 0xfff)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aarch64El2Error {
    InvalidInputAddressBits(u8),
    InvalidPhysicalAddressBits(u8),
    InvalidStartingLevel(u8),
    InvalidBlockLevel(u8),
    OutputAddressWidth(u64),
    MisalignedOutputAddress { address: u64, alignment: u64 },
}

impl fmt::Display for Aarch64El2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInputAddressBits(bits) => {
                write!(f, "invalid EL2 input address width {bits}")
            }
            Self::InvalidPhysicalAddressBits(bits) => {
                write!(f, "invalid EL2 output address width {bits}")
            }
            Self::InvalidStartingLevel(level) => {
                write!(f, "invalid EL2 stage-2 starting level {level}")
            }
            Self::InvalidBlockLevel(level) => {
                write!(f, "invalid EL2 stage-2 block level {level}")
            }
            Self::OutputAddressWidth(address) => {
                write!(f, "EL2 output address {address:#x} exceeds 52 bits")
            }
            Self::MisalignedOutputAddress { address, alignment } => write!(
                f,
                "EL2 output address {address:#x} is not aligned to {alignment:#x}"
            ),
        }
    }
}

impl std::error::Error for Aarch64El2Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_register_encodings_match_llvm_mc() {
        assert_eq!(HCR_EL2.encode_mrs(0), 0xd53c_1100);
        assert_eq!(HCR_EL2.encode_msr(1), 0xd51c_1101);
        assert_eq!(VTCR_EL2.encode_mrs(2), 0xd53c_2142);
        assert_eq!(VTCR_EL2.encode_msr(3), 0xd51c_2143);
        assert_eq!(VTTBR_EL2.encode_mrs(4), 0xd53c_2104);
        assert_eq!(VTTBR_EL2.encode_msr(5), 0xd51c_2105);
        assert_eq!(ESR_EL2.encode_mrs(6), 0xd53c_5206);
        assert_eq!(FAR_EL2.encode_mrs(7), 0xd53c_6007);
        assert_eq!(HPFAR_EL2.encode_mrs(8), 0xd53c_6088);
    }

    #[test]
    fn privileged_instruction_encodings_match_llvm_mc() {
        assert_eq!(Aarch64El2Instruction::Hvc(0x1234).encode(), 0xd402_4682);
        assert_eq!(Aarch64El2Instruction::Eret.encode(), 0xd69f_03e0);
        assert_eq!(
            Aarch64El2Instruction::TlbiVmalls12e1is.encode(),
            0xd50c_83df
        );
        assert_eq!(
            Aarch64El2Instruction::TlbiIpas2e1is(9).encode(),
            0xd50c_8029
        );
        assert_eq!(Aarch64El2Instruction::AtS12e1r(10).encode(), 0xd50c_788a);
    }

    #[test]
    fn stage2_control_and_descriptors_preserve_hardware_fields() {
        let vtcr = Aarch64Stage2Config::DEFAULT_4K_48BIT.vtcr_el2().unwrap();
        assert_eq!(vtcr & 0x3f, 16);
        assert_eq!((vtcr >> 16) & 0x7, 5);
        assert_eq!((vtcr >> 19) & 1, 1);
        let page = Aarch64Stage2Descriptor::page(
            0x4000_0000,
            Aarch64Stage2Granule::Size4K,
            Aarch64Stage2Access::ReadWrite,
            Aarch64Stage2MemoryAttribute::NormalWriteBack,
            false,
        )
        .unwrap();
        assert_eq!(page.raw() & 0x3, 0x3);
        assert_eq!((page.raw() >> 6) & 0x3, 0x3);
        assert_eq!((page.raw() >> 10) & 1, 1);
    }

    #[test]
    fn exit_snapshot_decodes_class_length_and_fault_ipa() {
        let snapshot = Aarch64El2ExitSnapshot {
            esr_el2: (0x24 << 26) | (1 << 25) | 0x45,
            far_el2: 0xabc,
            hpfar_el2: 0x1234_50,
            elr_el2: 0x8000,
            spsr_el2: 0,
        };
        assert_eq!(snapshot.exception_class(), Aarch64El2ExitClass::DataAbort);
        assert_eq!(snapshot.instruction_length(), 4);
        assert_eq!(snapshot.fault_ipa() & 0xfff, 0xabc);
    }
}
