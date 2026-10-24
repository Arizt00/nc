use core::fmt;
use core::marker::PhantomData;

pub const X86_VIRTUALIZATION_PAGE_SIZE: usize = 4096;
pub const AMD_VMCB_CONTROL_AREA_SIZE: usize = 0x400;
pub const AMD_VMCB_STATE_SAVE_AREA_OFFSET: usize = 0x400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationVendor {
    Intel,
    Amd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationPrivilege {
    Ring0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationOperandContract {
    None,
    Memory64,
    Register64Memory64,
    Register64Register64Memory64,
    Register64Memory128,
    ImplicitRaxPhysicalAddress,
    ImplicitRaxRcx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86VirtualizationInstructionSpec {
    pub mnemonic: &'static str,
    pub vendor: VirtualizationVendor,
    pub feature: &'static str,
    pub opcode: &'static [u8],
    pub modrm_extension: Option<u8>,
    pub operands: VirtualizationOperandContract,
    pub privilege: VirtualizationPrivilege,
    pub updates_status_flags: bool,
}

pub const X86_VIRTUALIZATION_INSTRUCTIONS: &[X86VirtualizationInstructionSpec] = &[
    X86VirtualizationInstructionSpec {
        mnemonic: "vmxon",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0xf3, 0x0f, 0xc7],
        modrm_extension: Some(6),
        operands: VirtualizationOperandContract::Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmclear",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x66, 0x0f, 0xc7],
        modrm_extension: Some(6),
        operands: VirtualizationOperandContract::Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmptrld",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0xc7],
        modrm_extension: Some(6),
        operands: VirtualizationOperandContract::Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmptrst",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0xc7],
        modrm_extension: Some(7),
        operands: VirtualizationOperandContract::Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmread",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x78],
        modrm_extension: None,
        operands: VirtualizationOperandContract::Register64Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmwrite",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x79],
        modrm_extension: None,
        operands: VirtualizationOperandContract::Register64Register64Memory64,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "invept",
        vendor: VirtualizationVendor::Intel,
        feature: "ept",
        opcode: &[0x66, 0x0f, 0x38, 0x80],
        modrm_extension: None,
        operands: VirtualizationOperandContract::Register64Memory128,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "invvpid",
        vendor: VirtualizationVendor::Intel,
        feature: "vpid",
        opcode: &[0x66, 0x0f, 0x38, 0x81],
        modrm_extension: None,
        operands: VirtualizationOperandContract::Register64Memory128,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmcall",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x01, 0xc1],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmxoff",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x01, 0xc4],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmlaunch",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x01, 0xc2],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmresume",
        vendor: VirtualizationVendor::Intel,
        feature: "vmx",
        opcode: &[0x0f, 0x01, 0xc3],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: true,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmrun",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xd8],
        modrm_extension: None,
        operands: VirtualizationOperandContract::ImplicitRaxPhysicalAddress,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmmcall",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xd9],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmload",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xda],
        modrm_extension: None,
        operands: VirtualizationOperandContract::ImplicitRaxPhysicalAddress,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "vmsave",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xdb],
        modrm_extension: None,
        operands: VirtualizationOperandContract::ImplicitRaxPhysicalAddress,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "stgi",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xdc],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "clgi",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xdd],
        modrm_extension: None,
        operands: VirtualizationOperandContract::None,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
    X86VirtualizationInstructionSpec {
        mnemonic: "invlpga",
        vendor: VirtualizationVendor::Amd,
        feature: "svm",
        opcode: &[0x0f, 0x01, 0xdf],
        modrm_extension: None,
        operands: VirtualizationOperandContract::ImplicitRaxRcx,
        privilege: VirtualizationPrivilege::Ring0,
        updates_status_flags: false,
    },
];

pub fn x86_virtualization_instruction(
    mnemonic: &str,
) -> Option<&'static X86VirtualizationInstructionSpec> {
    X86_VIRTUALIZATION_INSTRUCTIONS
        .iter()
        .find(|spec| spec.mnemonic.eq_ignore_ascii_case(mnemonic))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmcsFieldWidth {
    Bits16 = 0,
    Bits64 = 1,
    Bits32 = 2,
    Natural = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmcsFieldClass {
    Control = 0,
    VmExit = 1,
    Guest = 2,
    Host = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmcsFieldAccess {
    Full = 0,
    High = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmcsFieldError {
    ReservedEncodingBits(u32),
    HighAccessRequires64BitWidth(u32),
    WidthMismatch {
        encoding: u32,
        expected: VmcsFieldWidth,
        actual: VmcsFieldWidth,
    },
}

impl fmt::Display for VmcsFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReservedEncodingBits(raw) => {
                write!(f, "VMCS field encoding 0x{raw:08x} sets reserved bits")
            }
            Self::HighAccessRequires64BitWidth(raw) => write!(
                f,
                "VMCS high-half access 0x{raw:08x} is valid only for a 64-bit field"
            ),
            Self::WidthMismatch {
                encoding,
                expected,
                actual,
            } => write!(
                f,
                "VMCS field 0x{encoding:08x} has width {actual:?}, expected {expected:?}"
            ),
        }
    }
}

impl std::error::Error for VmcsFieldError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VmcsFieldEncoding(u32);

impl VmcsFieldEncoding {
    const RESERVED_MASK: u32 = !0x7fff | (1 << 12);

    pub const fn from_raw_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    pub fn try_from_raw(raw: u32) -> Result<Self, VmcsFieldError> {
        if raw & Self::RESERVED_MASK != 0 {
            return Err(VmcsFieldError::ReservedEncodingBits(raw));
        }
        let encoding = Self(raw);
        if encoding.access() == VmcsFieldAccess::High && encoding.width() != VmcsFieldWidth::Bits64
        {
            return Err(VmcsFieldError::HighAccessRequires64BitWidth(raw));
        }
        Ok(encoding)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn access(self) -> VmcsFieldAccess {
        if self.0 & 1 == 0 {
            VmcsFieldAccess::Full
        } else {
            VmcsFieldAccess::High
        }
    }

    pub const fn index(self) -> u16 {
        ((self.0 >> 1) & 0x1ff) as u16
    }

    pub const fn class(self) -> VmcsFieldClass {
        match (self.0 >> 10) & 0x3 {
            0 => VmcsFieldClass::Control,
            1 => VmcsFieldClass::VmExit,
            2 => VmcsFieldClass::Guest,
            _ => VmcsFieldClass::Host,
        }
    }

    pub const fn width(self) -> VmcsFieldWidth {
        match (self.0 >> 13) & 0x3 {
            0 => VmcsFieldWidth::Bits16,
            1 => VmcsFieldWidth::Bits64,
            2 => VmcsFieldWidth::Bits32,
            _ => VmcsFieldWidth::Natural,
        }
    }
}

pub trait VmcsValue: Copy + 'static {
    const WIDTH: VmcsFieldWidth;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsU16;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsU32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsU64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsNatural;

impl VmcsValue for VmcsU16 {
    const WIDTH: VmcsFieldWidth = VmcsFieldWidth::Bits16;
}
impl VmcsValue for VmcsU32 {
    const WIDTH: VmcsFieldWidth = VmcsFieldWidth::Bits32;
}
impl VmcsValue for VmcsU64 {
    const WIDTH: VmcsFieldWidth = VmcsFieldWidth::Bits64;
}
impl VmcsValue for VmcsNatural {
    const WIDTH: VmcsFieldWidth = VmcsFieldWidth::Natural;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsField<T: VmcsValue> {
    pub name: &'static str,
    pub encoding: VmcsFieldEncoding,
    marker: PhantomData<T>,
}

impl<T: VmcsValue> VmcsField<T> {
    pub const fn from_raw_unchecked(name: &'static str, raw: u32) -> Self {
        Self {
            name,
            encoding: VmcsFieldEncoding::from_raw_unchecked(raw),
            marker: PhantomData,
        }
    }

    pub fn try_from_raw(name: &'static str, raw: u32) -> Result<Self, VmcsFieldError> {
        let encoding = VmcsFieldEncoding::try_from_raw(raw)?;
        if encoding.width() != T::WIDTH {
            return Err(VmcsFieldError::WidthMismatch {
                encoding: raw,
                expected: T::WIDTH,
                actual: encoding.width(),
            });
        }
        Ok(Self {
            name,
            encoding,
            marker: PhantomData,
        })
    }
}

macro_rules! vmcs_fields {
    ($ty:ty; $($name:ident = $raw:expr),+ $(,)?) => {
        $(pub const $name: VmcsField<$ty> =
            VmcsField::from_raw_unchecked(stringify!($name), $raw);)+
    };
}

pub mod vmcs {
    use super::*;

    vmcs_fields!(VmcsU16;
        VIRTUAL_PROCESSOR_ID = 0x0000,
        POSTED_INTERRUPT_NOTIFICATION_VECTOR = 0x0002,
        EPTP_INDEX = 0x0004,
        GUEST_ES_SELECTOR = 0x0800,
        GUEST_CS_SELECTOR = 0x0802,
        GUEST_SS_SELECTOR = 0x0804,
        GUEST_DS_SELECTOR = 0x0806,
        GUEST_FS_SELECTOR = 0x0808,
        GUEST_GS_SELECTOR = 0x080a,
        GUEST_LDTR_SELECTOR = 0x080c,
        GUEST_TR_SELECTOR = 0x080e,
        GUEST_INTERRUPT_STATUS = 0x0810,
        PML_INDEX = 0x0812,
        HOST_ES_SELECTOR = 0x0c00,
        HOST_CS_SELECTOR = 0x0c02,
        HOST_SS_SELECTOR = 0x0c04,
        HOST_DS_SELECTOR = 0x0c06,
        HOST_FS_SELECTOR = 0x0c08,
        HOST_GS_SELECTOR = 0x0c0a,
        HOST_TR_SELECTOR = 0x0c0c,
    );

    vmcs_fields!(VmcsU64;
        IO_BITMAP_A = 0x2000,
        IO_BITMAP_B = 0x2002,
        MSR_BITMAP = 0x2004,
        VM_EXIT_MSR_STORE_ADDRESS = 0x2006,
        VM_EXIT_MSR_LOAD_ADDRESS = 0x2008,
        VM_ENTRY_MSR_LOAD_ADDRESS = 0x200a,
        EXECUTIVE_VMCS_POINTER = 0x200c,
        PML_ADDRESS = 0x200e,
        TSC_OFFSET = 0x2010,
        VIRTUAL_APIC_ADDRESS = 0x2012,
        APIC_ACCESS_ADDRESS = 0x2014,
        POSTED_INTERRUPT_DESCRIPTOR_ADDRESS = 0x2016,
        VM_FUNCTION_CONTROLS = 0x2018,
        EPT_POINTER = 0x201a,
        EOI_EXIT_BITMAP0 = 0x201c,
        EOI_EXIT_BITMAP1 = 0x201e,
        EOI_EXIT_BITMAP2 = 0x2020,
        EOI_EXIT_BITMAP3 = 0x2022,
        EPTP_LIST_ADDRESS = 0x2024,
        VMREAD_BITMAP_ADDRESS = 0x2026,
        VMWRITE_BITMAP_ADDRESS = 0x2028,
        VIRTUALIZATION_EXCEPTION_INFORMATION_ADDRESS = 0x202a,
        XSS_EXIT_BITMAP = 0x202c,
        ENCLS_EXIT_BITMAP = 0x202e,
        SUB_PAGE_PERMISSION_TABLE_POINTER = 0x2030,
        TSC_MULTIPLIER = 0x2032,
        GUEST_PHYSICAL_ADDRESS = 0x2400,
        VMCS_LINK_POINTER = 0x2800,
        GUEST_IA32_DEBUGCTL = 0x2802,
        GUEST_IA32_PAT = 0x2804,
        GUEST_IA32_EFER = 0x2806,
        GUEST_IA32_PERF_GLOBAL_CTRL = 0x2808,
        GUEST_PDPTE0 = 0x280a,
        GUEST_PDPTE1 = 0x280c,
        GUEST_PDPTE2 = 0x280e,
        GUEST_PDPTE3 = 0x2810,
        GUEST_IA32_BNDCFGS = 0x2812,
        GUEST_IA32_RTIT_CTL = 0x2814,
        HOST_IA32_PAT = 0x2c00,
        HOST_IA32_EFER = 0x2c02,
        HOST_IA32_PERF_GLOBAL_CTRL = 0x2c04,
    );

    vmcs_fields!(VmcsU32;
        PIN_BASED_VM_EXECUTION_CONTROLS = 0x4000,
        PRIMARY_PROCESSOR_BASED_VM_EXECUTION_CONTROLS = 0x4002,
        EXCEPTION_BITMAP = 0x4004,
        PAGE_FAULT_ERROR_CODE_MASK = 0x4006,
        PAGE_FAULT_ERROR_CODE_MATCH = 0x4008,
        CR3_TARGET_COUNT = 0x400a,
        VM_EXIT_CONTROLS = 0x400c,
        VM_EXIT_MSR_STORE_COUNT = 0x400e,
        VM_EXIT_MSR_LOAD_COUNT = 0x4010,
        VM_ENTRY_CONTROLS = 0x4012,
        VM_ENTRY_MSR_LOAD_COUNT = 0x4014,
        VM_ENTRY_INTERRUPTION_INFORMATION_FIELD = 0x4016,
        VM_ENTRY_EXCEPTION_ERROR_CODE = 0x4018,
        VM_ENTRY_INSTRUCTION_LENGTH = 0x401a,
        TPR_THRESHOLD = 0x401c,
        SECONDARY_PROCESSOR_BASED_VM_EXECUTION_CONTROLS = 0x401e,
        PLE_GAP = 0x4020,
        PLE_WINDOW = 0x4022,
        VM_INSTRUCTION_ERROR = 0x4400,
        VM_EXIT_REASON = 0x4402,
        VM_EXIT_INTERRUPTION_INFORMATION = 0x4404,
        VM_EXIT_INTERRUPTION_ERROR_CODE = 0x4406,
        IDT_VECTORING_INFORMATION_FIELD = 0x4408,
        IDT_VECTORING_ERROR_CODE = 0x440a,
        VM_EXIT_INSTRUCTION_LENGTH = 0x440c,
        VM_EXIT_INSTRUCTION_INFORMATION = 0x440e,
        GUEST_ES_LIMIT = 0x4800,
        GUEST_CS_LIMIT = 0x4802,
        GUEST_SS_LIMIT = 0x4804,
        GUEST_DS_LIMIT = 0x4806,
        GUEST_FS_LIMIT = 0x4808,
        GUEST_GS_LIMIT = 0x480a,
        GUEST_LDTR_LIMIT = 0x480c,
        GUEST_TR_LIMIT = 0x480e,
        GUEST_GDTR_LIMIT = 0x4810,
        GUEST_IDTR_LIMIT = 0x4812,
        GUEST_ES_ACCESS_RIGHTS = 0x4814,
        GUEST_CS_ACCESS_RIGHTS = 0x4816,
        GUEST_SS_ACCESS_RIGHTS = 0x4818,
        GUEST_DS_ACCESS_RIGHTS = 0x481a,
        GUEST_FS_ACCESS_RIGHTS = 0x481c,
        GUEST_GS_ACCESS_RIGHTS = 0x481e,
        GUEST_LDTR_ACCESS_RIGHTS = 0x4820,
        GUEST_TR_ACCESS_RIGHTS = 0x4822,
        GUEST_INTERRUPTIBILITY_STATE = 0x4824,
        GUEST_ACTIVITY_STATE = 0x4826,
        GUEST_SMBASE = 0x4828,
        GUEST_SYSENTER_CS = 0x482a,
        VMX_PREEMPTION_TIMER_VALUE = 0x482e,
        HOST_SYSENTER_CS = 0x4c00,
    );

    vmcs_fields!(VmcsNatural;
        CR0_GUEST_HOST_MASK = 0x6000,
        CR4_GUEST_HOST_MASK = 0x6002,
        CR0_READ_SHADOW = 0x6004,
        CR4_READ_SHADOW = 0x6006,
        CR3_TARGET_VALUE0 = 0x6008,
        CR3_TARGET_VALUE1 = 0x600a,
        CR3_TARGET_VALUE2 = 0x600c,
        CR3_TARGET_VALUE3 = 0x600e,
        EXIT_QUALIFICATION = 0x6400,
        IO_RCX = 0x6402,
        IO_RSI = 0x6404,
        IO_RDI = 0x6406,
        IO_RIP = 0x6408,
        GUEST_LINEAR_ADDRESS = 0x640a,
        GUEST_CR0 = 0x6800,
        GUEST_CR3 = 0x6802,
        GUEST_CR4 = 0x6804,
        GUEST_ES_BASE = 0x6806,
        GUEST_CS_BASE = 0x6808,
        GUEST_SS_BASE = 0x680a,
        GUEST_DS_BASE = 0x680c,
        GUEST_FS_BASE = 0x680e,
        GUEST_GS_BASE = 0x6810,
        GUEST_LDTR_BASE = 0x6812,
        GUEST_TR_BASE = 0x6814,
        GUEST_GDTR_BASE = 0x6816,
        GUEST_IDTR_BASE = 0x6818,
        GUEST_DR7 = 0x681a,
        GUEST_RSP = 0x681c,
        GUEST_RIP = 0x681e,
        GUEST_RFLAGS = 0x6820,
        GUEST_PENDING_DEBUG_EXCEPTIONS = 0x6822,
        GUEST_SYSENTER_ESP = 0x6824,
        GUEST_SYSENTER_EIP = 0x6826,
        HOST_CR0 = 0x6c00,
        HOST_CR3 = 0x6c02,
        HOST_CR4 = 0x6c04,
        HOST_FS_BASE = 0x6c06,
        HOST_GS_BASE = 0x6c08,
        HOST_TR_BASE = 0x6c0a,
        HOST_GDTR_BASE = 0x6c0c,
        HOST_IDTR_BASE = 0x6c0e,
        HOST_SYSENTER_ESP = 0x6c10,
        HOST_SYSENTER_EIP = 0x6c12,
        HOST_RSP = 0x6c14,
        HOST_RIP = 0x6c16,
    );
}

#[repr(C, align(4096))]
#[derive(Clone)]
pub struct IntelVmcsRegion {
    bytes: [u8; X86_VIRTUALIZATION_PAGE_SIZE],
}

impl IntelVmcsRegion {
    pub fn new(revision_id: u32, shadow: bool) -> Self {
        let mut region = Self {
            bytes: [0; X86_VIRTUALIZATION_PAGE_SIZE],
        };
        region.initialize(revision_id, shadow);
        region
    }

    pub fn initialize(&mut self, revision_id: u32, shadow: bool) {
        self.bytes.fill(0);
        let header = (revision_id & 0x7fff_ffff) | if shadow { 1 << 31 } else { 0 };
        self.bytes[0..4].copy_from_slice(&header.to_le_bytes());
    }

    pub fn revision_id(&self) -> u32 {
        u32::from_le_bytes(self.bytes[0..4].try_into().expect("fixed VMCS header")) & 0x7fff_ffff
    }

    pub fn is_shadow(&self) -> bool {
        u32::from_le_bytes(self.bytes[0..4].try_into().expect("fixed VMCS header")) & (1 << 31) != 0
    }

    pub fn abort_indicator(&self) -> u32 {
        u32::from_le_bytes(self.bytes[4..8].try_into().expect("fixed VMCS header"))
    }

    pub fn bytes(&self) -> &[u8; X86_VIRTUALIZATION_PAGE_SIZE] {
        &self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u8; X86_VIRTUALIZATION_PAGE_SIZE] {
        &mut self.bytes
    }
}

#[repr(C, align(4096))]
#[derive(Clone)]
pub struct IntelVmxonRegion {
    bytes: [u8; X86_VIRTUALIZATION_PAGE_SIZE],
}

impl IntelVmxonRegion {
    pub fn new(revision_id: u32) -> Self {
        let mut bytes = [0; X86_VIRTUALIZATION_PAGE_SIZE];
        bytes[0..4].copy_from_slice(&(revision_id & 0x7fff_ffff).to_le_bytes());
        Self { bytes }
    }

    pub fn bytes(&self) -> &[u8; X86_VIRTUALIZATION_PAGE_SIZE] {
        &self.bytes
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InveptDescriptor {
    pub ept_pointer: u64,
    pub reserved: u64,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InvvpidDescriptor {
    pub vpid: u16,
    pub reserved: [u8; 6],
    pub linear_address: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum InveptType {
    SingleContext = 1,
    AllContexts = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum InvvpidType {
    IndividualAddress = 0,
    SingleContext = 1,
    AllContexts = 2,
    SingleContextRetainingGlobals = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmcbArea {
    Control,
    StateSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmcbAccess {
    ReadWrite,
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmcbError {
    FieldOutOfRange {
        name: &'static str,
        offset: usize,
        width: usize,
    },
    ReadOnlyField(&'static str),
}

impl fmt::Display for VmcbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldOutOfRange {
                name,
                offset,
                width,
            } => write!(
                f,
                "VMCB field {name} at 0x{offset:x} with width {width} leaves the 4 KiB VMCB"
            ),
            Self::ReadOnlyField(name) => write!(f, "VMCB field {name} is hardware read-only"),
        }
    }
}

impl std::error::Error for VmcbError {}

pub trait VmcbValue: Copy + Sized + 'static {
    const WIDTH: usize;
    fn decode(bytes: &[u8]) -> Self;
    fn encode(self, bytes: &mut [u8]);
}

macro_rules! vmcb_integer_value {
    ($ty:ty) => {
        impl VmcbValue for $ty {
            const WIDTH: usize = core::mem::size_of::<$ty>();

            fn decode(bytes: &[u8]) -> Self {
                <$ty>::from_le_bytes(bytes.try_into().expect("typed VMCB field width"))
            }

            fn encode(self, bytes: &mut [u8]) {
                bytes.copy_from_slice(&self.to_le_bytes());
            }
        }
    };
}

vmcb_integer_value!(u8);
vmcb_integer_value!(u16);
vmcb_integer_value!(u32);
vmcb_integer_value!(u64);
vmcb_integer_value!(u128);

impl<const N: usize> VmcbValue for [u8; N] {
    const WIDTH: usize = N;

    fn decode(bytes: &[u8]) -> Self {
        bytes.try_into().expect("typed VMCB byte array width")
    }

    fn encode(self, bytes: &mut [u8]) {
        bytes.copy_from_slice(&self);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VmcbSegment {
    pub selector: u16,
    pub attributes: u16,
    pub limit: u32,
    pub base: u64,
}

impl VmcbValue for VmcbSegment {
    const WIDTH: usize = 16;

    fn decode(bytes: &[u8]) -> Self {
        Self {
            selector: u16::from_le_bytes(bytes[0..2].try_into().expect("segment selector")),
            attributes: u16::from_le_bytes(bytes[2..4].try_into().expect("segment attributes")),
            limit: u32::from_le_bytes(bytes[4..8].try_into().expect("segment limit")),
            base: u64::from_le_bytes(bytes[8..16].try_into().expect("segment base")),
        }
    }

    fn encode(self, bytes: &mut [u8]) {
        bytes[0..2].copy_from_slice(&self.selector.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.attributes.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.limit.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.base.to_le_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcbField<T: VmcbValue> {
    pub name: &'static str,
    pub offset: usize,
    pub area: VmcbArea,
    pub access: VmcbAccess,
    marker: PhantomData<T>,
}

impl<T: VmcbValue> VmcbField<T> {
    pub const fn new(
        name: &'static str,
        offset: usize,
        area: VmcbArea,
        access: VmcbAccess,
    ) -> Self {
        Self {
            name,
            offset,
            area,
            access,
            marker: PhantomData,
        }
    }

    pub const fn absolute_offset(self) -> usize {
        match self.area {
            VmcbArea::Control => self.offset,
            VmcbArea::StateSave => AMD_VMCB_STATE_SAVE_AREA_OFFSET + self.offset,
        }
    }
}

macro_rules! vmcb_control_fields {
    ($($name:ident: $ty:ty = ($offset:expr, $access:ident)),+ $(,)?) => {
        $(pub const $name: VmcbField<$ty> = VmcbField::new(
            stringify!($name), $offset, VmcbArea::Control, VmcbAccess::$access
        );)+
    };
}

macro_rules! vmcb_state_fields {
    ($($name:ident: $ty:ty = ($offset:expr, $access:ident)),+ $(,)?) => {
        $(pub const $name: VmcbField<$ty> = VmcbField::new(
            stringify!($name), $offset, VmcbArea::StateSave, VmcbAccess::$access
        );)+
    };
}

pub mod vmcb {
    use super::*;

    vmcb_control_fields!(
        INTERCEPT_CR_READ: u16 = (0x000, ReadWrite),
        INTERCEPT_CR_WRITE: u16 = (0x002, ReadWrite),
        INTERCEPT_DR_READ: u16 = (0x004, ReadWrite),
        INTERCEPT_DR_WRITE: u16 = (0x006, ReadWrite),
        INTERCEPT_EXCEPTIONS: u32 = (0x008, ReadWrite),
        INTERCEPT_MISC1: u32 = (0x00c, ReadWrite),
        INTERCEPT_MISC2: u32 = (0x010, ReadWrite),
        INTERCEPT_MISC3: u32 = (0x014, ReadWrite),
        PAUSE_FILTER_THRESHOLD: u16 = (0x03c, ReadWrite),
        PAUSE_FILTER_COUNT: u16 = (0x03e, ReadWrite),
        IOPM_BASE_PHYSICAL_ADDRESS: u64 = (0x040, ReadWrite),
        MSRPM_BASE_PHYSICAL_ADDRESS: u64 = (0x048, ReadWrite),
        TSC_OFFSET: u64 = (0x050, ReadWrite),
        GUEST_ASID_AND_TLB_CONTROL: u64 = (0x058, ReadWrite),
        VIRTUAL_INTERRUPT_CONTROL: u64 = (0x060, ReadWrite),
        INTERRUPT_SHADOW: u64 = (0x068, ReadWrite),
        EXIT_CODE: u64 = (0x070, ReadOnly),
        EXIT_INFO1: u64 = (0x078, ReadOnly),
        EXIT_INFO2: u64 = (0x080, ReadOnly),
        EXIT_INTERRUPT_INFO: u64 = (0x088, ReadOnly),
        NESTED_PAGING_CONTROL: u64 = (0x090, ReadWrite),
        AVIC_APIC_BAR: u64 = (0x098, ReadWrite),
        GHCB_PHYSICAL_ADDRESS: u64 = (0x0a0, ReadWrite),
        EVENT_INJECTION: u64 = (0x0a8, ReadWrite),
        NESTED_CR3: u64 = (0x0b0, ReadWrite),
        VIRTUALIZATION_CONTROLS: u64 = (0x0b8, ReadWrite),
        CLEAN_BITS: u32 = (0x0c0, ReadWrite),
        NEXT_RIP: u64 = (0x0c8, ReadOnly),
        INSTRUCTION_BYTES: [u8; 16] = (0x0d0, ReadOnly),
        AVIC_APIC_BACKING_PAGE: u64 = (0x0e0, ReadWrite),
        AVIC_LOGICAL_TABLE: u64 = (0x0f0, ReadWrite),
        AVIC_PHYSICAL_TABLE: u64 = (0x0f8, ReadWrite),
        VMSA_POINTER: u64 = (0x108, ReadWrite),
        VMGEXIT_RAX: u64 = (0x110, ReadWrite),
        VIRTUAL_TOM: u64 = (0x118, ReadWrite),
        TLB_INVALIDATION_CONTROL: u64 = (0x120, ReadWrite),
        BUS_LOCK_THRESHOLD: u16 = (0x128, ReadWrite),
        UPDATE_IRR: u8 = (0x134, ReadWrite),
        ALLOWED_SEV_FEATURES: u64 = (0x138, ReadWrite),
        GUEST_SEV_FEATURES: u64 = (0x140, ReadWrite),
        REQUESTED_IRR: [u8; 32] = (0x150, ReadWrite),
    );

    vmcb_state_fields!(
        ES: VmcbSegment = (0x000, ReadWrite),
        CS: VmcbSegment = (0x010, ReadWrite),
        SS: VmcbSegment = (0x020, ReadWrite),
        DS: VmcbSegment = (0x030, ReadWrite),
        FS: VmcbSegment = (0x040, ReadWrite),
        GS: VmcbSegment = (0x050, ReadWrite),
        GDTR: VmcbSegment = (0x060, ReadWrite),
        LDTR: VmcbSegment = (0x070, ReadWrite),
        IDTR: VmcbSegment = (0x080, ReadWrite),
        TR: VmcbSegment = (0x090, ReadWrite),
        CPL: u8 = (0x0cb, ReadWrite),
        EFER: u64 = (0x0d0, ReadWrite),
        CR4: u64 = (0x148, ReadWrite),
        CR3: u64 = (0x150, ReadWrite),
        CR0: u64 = (0x158, ReadWrite),
        DR7: u64 = (0x160, ReadWrite),
        DR6: u64 = (0x168, ReadWrite),
        RFLAGS: u64 = (0x170, ReadWrite),
        RIP: u64 = (0x178, ReadWrite),
        RSP: u64 = (0x1d8, ReadWrite),
        RAX: u64 = (0x1f8, ReadWrite),
        STAR: u64 = (0x200, ReadWrite),
        LSTAR: u64 = (0x208, ReadWrite),
        CSTAR: u64 = (0x210, ReadWrite),
        SFMASK: u64 = (0x218, ReadWrite),
        KERNEL_GS_BASE: u64 = (0x220, ReadWrite),
        SYSENTER_CS: u64 = (0x228, ReadWrite),
        SYSENTER_ESP: u64 = (0x230, ReadWrite),
        SYSENTER_EIP: u64 = (0x238, ReadWrite),
        CR2: u64 = (0x240, ReadWrite),
        G_PAT: u64 = (0x268, ReadWrite),
        DBGCTL: u64 = (0x270, ReadWrite),
        BR_FROM: u64 = (0x278, ReadWrite),
        BR_TO: u64 = (0x280, ReadWrite),
        LAST_EXCP_FROM: u64 = (0x288, ReadWrite),
        LAST_EXCP_TO: u64 = (0x290, ReadWrite),
        SPEC_CTRL: u64 = (0x2e0, ReadWrite),
    );
}

#[repr(C, align(4096))]
#[derive(Clone)]
pub struct AmdVmcb {
    bytes: [u8; X86_VIRTUALIZATION_PAGE_SIZE],
}

impl Default for AmdVmcb {
    fn default() -> Self {
        Self::new()
    }
}

impl AmdVmcb {
    pub const fn new() -> Self {
        Self {
            bytes: [0; X86_VIRTUALIZATION_PAGE_SIZE],
        }
    }

    fn range<T: VmcbValue>(field: VmcbField<T>) -> Result<core::ops::Range<usize>, VmcbError> {
        let start = field.absolute_offset();
        let end = start.saturating_add(T::WIDTH);
        if end > X86_VIRTUALIZATION_PAGE_SIZE {
            return Err(VmcbError::FieldOutOfRange {
                name: field.name,
                offset: start,
                width: T::WIDTH,
            });
        }
        Ok(start..end)
    }

    pub fn read<T: VmcbValue>(&self, field: VmcbField<T>) -> Result<T, VmcbError> {
        let range = Self::range(field)?;
        Ok(T::decode(&self.bytes[range]))
    }

    pub fn write<T: VmcbValue>(&mut self, field: VmcbField<T>, value: T) -> Result<(), VmcbError> {
        if field.access == VmcbAccess::ReadOnly {
            return Err(VmcbError::ReadOnlyField(field.name));
        }
        let range = Self::range(field)?;
        value.encode(&mut self.bytes[range]);
        Ok(())
    }

    pub fn bytes(&self) -> &[u8; X86_VIRTUALIZATION_PAGE_SIZE] {
        &self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u8; X86_VIRTUALIZATION_PAGE_SIZE] {
        &mut self.bytes
    }
}

pub const fn is_x86_virtualization_page_aligned(physical_address: u64) -> bool {
    physical_address & (X86_VIRTUALIZATION_PAGE_SIZE as u64 - 1) == 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X86VirtualizationHostCapabilities {
    pub architecture_supported: bool,
    pub vendor_id: String,
    pub hypervisor_present: bool,
    pub vmx: bool,
    pub svm: bool,
    pub svm_revision: Option<u8>,
    pub svm_asid_count: Option<u32>,
}

impl X86VirtualizationHostCapabilities {
    pub fn detect() -> Self {
        detect_x86_virtualization_host_capabilities()
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn detect_x86_virtualization_host_capabilities() -> X86VirtualizationHostCapabilities {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__cpuid, __cpuid_count};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__cpuid, __cpuid_count};

    let leaf0 = __cpuid(0);
    let mut vendor = Vec::with_capacity(12);
    vendor.extend_from_slice(&leaf0.ebx.to_le_bytes());
    vendor.extend_from_slice(&leaf0.edx.to_le_bytes());
    vendor.extend_from_slice(&leaf0.ecx.to_le_bytes());
    let vendor_id = String::from_utf8_lossy(&vendor).into_owned();

    let leaf1 = __cpuid(1);
    let vmx = leaf1.ecx & (1 << 5) != 0;
    let hypervisor_present = leaf1.ecx & (1 << 31) != 0;

    let extended_max = __cpuid(0x8000_0000).eax;
    let svm = extended_max >= 0x8000_0001 && __cpuid(0x8000_0001).ecx & (1 << 2) != 0;
    let (svm_revision, svm_asid_count) = if svm && extended_max >= 0x8000_000a {
        let leaf = __cpuid_count(0x8000_000a, 0);
        (Some((leaf.eax & 0xff) as u8), Some(leaf.ebx))
    } else {
        (None, None)
    };

    X86VirtualizationHostCapabilities {
        architecture_supported: true,
        vendor_id,
        hypervisor_present,
        vmx,
        svm,
        svm_revision,
        svm_asid_count,
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn detect_x86_virtualization_host_capabilities() -> X86VirtualizationHostCapabilities {
    X86VirtualizationHostCapabilities {
        architecture_supported: false,
        vendor_id: String::new(),
        hypervisor_present: false,
        vmx: false,
        svm: false,
        svm_revision: None,
        svm_asid_count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmcs_and_vmcb_are_exactly_one_aligned_page() {
        assert_eq!(core::mem::size_of::<IntelVmcsRegion>(), 4096);
        assert_eq!(core::mem::align_of::<IntelVmcsRegion>(), 4096);
        assert_eq!(core::mem::size_of::<IntelVmxonRegion>(), 4096);
        assert_eq!(core::mem::align_of::<IntelVmxonRegion>(), 4096);
        assert_eq!(core::mem::size_of::<AmdVmcb>(), 4096);
        assert_eq!(core::mem::align_of::<AmdVmcb>(), 4096);
    }

    #[test]
    fn vmcs_header_preserves_revision_shadow_and_abort_slots() {
        let region = IntelVmcsRegion::new(0x9234_5678, true);
        assert_eq!(region.revision_id(), 0x1234_5678);
        assert!(region.is_shadow());
        assert_eq!(region.abort_indicator(), 0);
    }

    #[test]
    fn vmcs_field_encoding_is_validated_and_typed() {
        let guest_rip = VmcsField::<VmcsNatural>::try_from_raw("guest_rip", 0x681e).unwrap();
        assert_eq!(guest_rip.encoding.class(), VmcsFieldClass::Guest);
        assert_eq!(guest_rip.encoding.width(), VmcsFieldWidth::Natural);
        assert!(VmcsField::<VmcsU32>::try_from_raw("bad", 0x681e).is_err());
        assert!(VmcsFieldEncoding::try_from_raw(0x1000).is_err());
    }

    #[test]
    fn vmcb_fields_roundtrip_at_architectural_offsets() {
        let mut vmcb = AmdVmcb::new();
        vmcb.write(vmcb::RIP, 0x1122_3344_5566_7788).unwrap();
        vmcb.write(
            vmcb::CS,
            VmcbSegment {
                selector: 8,
                attributes: 0xa09b,
                limit: 0xffff_ffff,
                base: 0,
            },
        )
        .unwrap();
        assert_eq!(vmcb.read(vmcb::RIP).unwrap(), 0x1122_3344_5566_7788);
        assert_eq!(vmcb.read(vmcb::CS).unwrap().selector, 8);
        assert_eq!(vmcb::RIP.absolute_offset(), 0x578);
    }

    #[test]
    fn vmcb_rejects_host_writes_to_hardware_output_fields() {
        let mut vmcb = AmdVmcb::new();
        assert_eq!(
            vmcb.write(vmcb::EXIT_CODE, 1),
            Err(VmcbError::ReadOnlyField("EXIT_CODE"))
        );
    }

    #[test]
    fn invalidate_descriptors_have_hardware_width_and_alignment() {
        assert_eq!(core::mem::size_of::<InveptDescriptor>(), 16);
        assert_eq!(core::mem::align_of::<InveptDescriptor>(), 16);
        assert_eq!(core::mem::size_of::<InvvpidDescriptor>(), 16);
        assert_eq!(core::mem::align_of::<InvvpidDescriptor>(), 16);
    }

    #[test]
    fn instruction_catalog_covers_complete_vmcs_vmcb_lifecycle() {
        for mnemonic in [
            "vmxon", "vmclear", "vmptrld", "vmptrst", "vmread", "vmwrite", "invept", "invvpid",
            "vmcall", "vmxoff", "vmlaunch", "vmresume", "vmrun", "vmmcall", "vmload", "vmsave",
            "stgi", "clgi", "invlpga",
        ] {
            assert!(
                x86_virtualization_instruction(mnemonic).is_some(),
                "missing {mnemonic}"
            );
        }
        assert_eq!(
            x86_virtualization_instruction("vmxon").unwrap().opcode,
            &[0xf3, 0x0f, 0xc7]
        );
    }
}
