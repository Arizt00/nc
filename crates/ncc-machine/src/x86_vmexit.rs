use core::fmt;

use crate::{GuestPagePermissions, GuestPhysicalAddress};

pub const X86_EXCEPTION_VECTOR_COUNT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum X86ExceptionVector {
    DivideError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    CoprocessorSegmentOverrun = 9,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtection = 13,
    PageFault = 14,
    Reserved15 = 15,
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloatingPoint = 19,
    VirtualizationException = 20,
    ControlProtection = 21,
    Reserved22 = 22,
    Reserved23 = 23,
    Reserved24 = 24,
    Reserved25 = 25,
    Reserved26 = 26,
    Reserved27 = 27,
    HypervisorInjection = 28,
    VmmCommunication = 29,
    SecurityException = 30,
    Reserved31 = 31,
}

impl X86ExceptionVector {
    pub const fn from_vector(vector: u8) -> Option<Self> {
        if vector >= X86_EXCEPTION_VECTOR_COUNT as u8 {
            return None;
        }
        Some(unsafe { core::mem::transmute::<u8, Self>(vector) })
    }

    pub const fn delivers_error_code(self) -> bool {
        matches!(
            self,
            Self::DoubleFault
                | Self::InvalidTss
                | Self::SegmentNotPresent
                | Self::StackSegmentFault
                | Self::GeneralProtection
                | Self::PageFault
                | Self::AlignmentCheck
                | Self::ControlProtection
                | Self::VmmCommunication
                | Self::SecurityException
        )
    }

    pub const fn class(self) -> ExceptionClass {
        match self {
            Self::DoubleFault => ExceptionClass::DoubleFault,
            Self::PageFault => ExceptionClass::PageFault,
            Self::InvalidTss
            | Self::SegmentNotPresent
            | Self::StackSegmentFault
            | Self::GeneralProtection => ExceptionClass::Contributory,
            _ => ExceptionClass::Benign,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionClass {
    Benign,
    Contributory,
    PageFault,
    DoubleFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmEventKind {
    ExternalInterrupt = 0,
    NonMaskableInterrupt = 2,
    HardwareException = 3,
    SoftwareInterrupt = 4,
    PrivilegedSoftwareException = 5,
    SoftwareException = 6,
    OtherEvent = 7,
}

impl VmEventKind {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::ExternalInterrupt),
            2 => Some(Self::NonMaskableInterrupt),
            3 => Some(Self::HardwareException),
            4 => Some(Self::SoftwareInterrupt),
            5 => Some(Self::PrivilegedSoftwareException),
            6 => Some(Self::SoftwareException),
            7 => Some(Self::OtherEvent),
            _ => None,
        }
    }

    pub const fn is_software(self) -> bool {
        matches!(
            self,
            Self::SoftwareInterrupt | Self::PrivilegedSoftwareException | Self::SoftwareException
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmEvent {
    pub vector: u8,
    pub kind: VmEventKind,
    pub error_code: Option<u32>,
    pub instruction_length: Option<u32>,
}

impl VmEvent {
    pub const fn external_interrupt(vector: u8) -> Self {
        Self {
            vector,
            kind: VmEventKind::ExternalInterrupt,
            error_code: None,
            instruction_length: None,
        }
    }

    pub const fn exception(vector: X86ExceptionVector, error_code: Option<u32>) -> Self {
        Self {
            vector: vector as u8,
            kind: VmEventKind::HardwareException,
            error_code,
            instruction_length: None,
        }
    }

    pub const fn nmi() -> Self {
        Self {
            vector: X86ExceptionVector::NonMaskableInterrupt as u8,
            kind: VmEventKind::NonMaskableInterrupt,
            error_code: None,
            instruction_length: None,
        }
    }

    pub fn validate(self) -> Result<Self, VmExitError> {
        if self.kind == VmEventKind::HardwareException {
            let exception = X86ExceptionVector::from_vector(self.vector)
                .ok_or(VmExitError::InvalidExceptionVector(self.vector))?;
            if exception.delivers_error_code() != self.error_code.is_some() {
                return Err(VmExitError::InvalidEventErrorCode {
                    vector: self.vector,
                    required: exception.delivers_error_code(),
                });
            }
        }
        if self.kind.is_software() && self.instruction_length.is_none() {
            return Err(VmExitError::SoftwareEventNeedsInstructionLength);
        }
        Ok(self)
    }

    pub fn intel_interruption_information(self) -> Result<u32, VmExitError> {
        let event = self.validate()?;
        Ok(event.vector as u32
            | ((event.kind as u32) << 8)
            | (u32::from(event.error_code.is_some()) << 11)
            | (1 << 31))
    }

    pub fn amd_event_injection(self) -> Result<u64, VmExitError> {
        let info = self.intel_interruption_information()? as u64;
        Ok(info | ((self.error_code.unwrap_or(0) as u64) << 32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmInterruptionInformation {
    pub valid: bool,
    pub vector: u8,
    pub kind: Option<VmEventKind>,
    pub error_code_valid: bool,
    pub nmi_unblocking: bool,
    pub raw: u32,
}

impl VmInterruptionInformation {
    pub const fn decode(raw: u32) -> Self {
        Self {
            valid: raw & (1 << 31) != 0,
            vector: raw as u8,
            kind: VmEventKind::from_raw(((raw >> 8) & 0x7) as u8),
            error_code_valid: raw & (1 << 11) != 0,
            nmi_unblocking: raw & (1 << 12) != 0,
            raw,
        }
    }

    pub fn event(
        self,
        error_code: Option<u32>,
        instruction_length: Option<u32>,
    ) -> Result<Option<VmEvent>, VmExitError> {
        if !self.valid {
            return Ok(None);
        }
        let kind = self
            .kind
            .ok_or(VmExitError::InvalidEventType((self.raw >> 8) as u8 & 0x7))?;
        let event = VmEvent {
            vector: self.vector,
            kind,
            error_code: if self.error_code_valid {
                Some(error_code.ok_or(VmExitError::MissingExitErrorCode)?)
            } else {
                None
            },
            instruction_length,
        };
        Ok(Some(event.validate()?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmVendor {
    Intel,
    Amd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmExitKind {
    ExceptionOrNmi,
    ExternalInterrupt,
    TripleFault,
    InitSignal,
    StartupIpi,
    InterruptWindow,
    NmiWindow,
    TaskSwitch,
    Cpuid,
    Halt,
    InvalidatePage,
    ReadTimestamp,
    Hypercall,
    ControlRegisterAccess,
    DebugRegisterAccess,
    PortIo,
    MsrRead,
    MsrWrite,
    InvalidGuestState,
    Pause,
    ApicAccess,
    EptViolation,
    EptMisconfiguration,
    NestedPageFault,
    PreemptionTimer,
    BusLock,
    Shutdown,
    SvmInstruction,
    Avic,
    Security,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntelVmExitReason {
    pub basic: u16,
    pub entry_failure: bool,
    pub pending_mtf: bool,
    pub from_vmx_root: bool,
    pub enclave_mode: bool,
    pub raw: u32,
}

impl IntelVmExitReason {
    pub const fn decode(raw: u32) -> Self {
        Self {
            basic: (raw & 0xffff) as u16,
            entry_failure: raw & (1 << 31) != 0,
            pending_mtf: raw & (1 << 28) != 0,
            from_vmx_root: raw & (1 << 29) != 0,
            enclave_mode: raw & (1 << 27) != 0,
            raw,
        }
    }

    pub const fn kind(self) -> VmExitKind {
        match self.basic {
            0 => VmExitKind::ExceptionOrNmi,
            1 => VmExitKind::ExternalInterrupt,
            2 => VmExitKind::TripleFault,
            3 => VmExitKind::InitSignal,
            4 => VmExitKind::StartupIpi,
            7 => VmExitKind::InterruptWindow,
            8 => VmExitKind::NmiWindow,
            9 => VmExitKind::TaskSwitch,
            10 => VmExitKind::Cpuid,
            12 => VmExitKind::Halt,
            14 => VmExitKind::InvalidatePage,
            16 | 51 => VmExitKind::ReadTimestamp,
            18 => VmExitKind::Hypercall,
            28 => VmExitKind::ControlRegisterAccess,
            29 => VmExitKind::DebugRegisterAccess,
            30 => VmExitKind::PortIo,
            31 => VmExitKind::MsrRead,
            32 => VmExitKind::MsrWrite,
            33 | 34 => VmExitKind::InvalidGuestState,
            40 => VmExitKind::Pause,
            44 | 45 | 56 => VmExitKind::ApicAccess,
            48 => VmExitKind::EptViolation,
            49 => VmExitKind::EptMisconfiguration,
            52 => VmExitKind::PreemptionTimer,
            60 | 66 => VmExitKind::Security,
            74 => VmExitKind::BusLock,
            _ => VmExitKind::Unknown,
        }
    }

    pub const fn mnemonic(self) -> &'static str {
        match self.basic {
            0 => "exception_or_nmi",
            1 => "external_interrupt",
            2 => "triple_fault",
            3 => "init",
            4 => "sipi",
            5 => "io_smi",
            6 => "other_smi",
            7 => "interrupt_window",
            8 => "nmi_window",
            9 => "task_switch",
            10 => "cpuid",
            11 => "getsec",
            12 => "hlt",
            13 => "invd",
            14 => "invlpg",
            15 => "rdpmc",
            16 => "rdtsc",
            17 => "rsm",
            18 => "vmcall",
            19 => "vmclear",
            20 => "vmlaunch",
            21 => "vmptrld",
            22 => "vmptrst",
            23 => "vmread",
            24 => "vmresume",
            25 => "vmwrite",
            26 => "vmxoff",
            27 => "vmxon",
            28 => "cr_access",
            29 => "dr_access",
            30 => "io_instruction",
            31 => "rdmsr",
            32 => "wrmsr",
            33 => "invalid_guest_state",
            34 => "msr_load_failure",
            36 => "mwait",
            37 => "monitor_trap_flag",
            39 => "monitor",
            40 => "pause",
            41 => "machine_check",
            43 => "tpr_below_threshold",
            44 => "apic_access",
            45 => "virtualized_eoi",
            46 => "gdtr_idtr_access",
            47 => "ldtr_tr_access",
            48 => "ept_violation",
            49 => "ept_misconfiguration",
            50 => "invept",
            51 => "rdtscp",
            52 => "preemption_timer",
            53 => "invvpid",
            54 => "wbinvd",
            55 => "xsetbv",
            56 => "apic_write",
            57 => "rdrand",
            58 => "invpcid",
            59 => "vmfunc",
            60 => "encls",
            61 => "rdseed",
            62 => "pml_full",
            63 => "xsaves",
            64 => "xrstors",
            65 => "pconfig",
            66 => "spp_event",
            67 => "umwait",
            68 => "tpause",
            69 => "loadiwkey",
            74 => "bus_lock",
            75 => "notification",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmdVmExitReason {
    pub code: u64,
}

impl AmdVmExitReason {
    pub const fn kind(self) -> VmExitKind {
        match self.code {
            0x000..=0x01f => VmExitKind::ControlRegisterAccess,
            0x020..=0x03f => VmExitKind::DebugRegisterAccess,
            0x040..=0x05f => VmExitKind::ExceptionOrNmi,
            0x060 => VmExitKind::ExternalInterrupt,
            0x061 => VmExitKind::ExceptionOrNmi,
            0x063 => VmExitKind::InitSignal,
            0x064 => VmExitKind::ExternalInterrupt,
            0x072 => VmExitKind::Cpuid,
            0x078 => VmExitKind::Halt,
            0x07b => VmExitKind::PortIo,
            0x07c => VmExitKind::MsrRead,
            0x07d => VmExitKind::TaskSwitch,
            0x07f => VmExitKind::Shutdown,
            0x080..=0x087 => VmExitKind::SvmInstruction,
            0x08b | 0x08c => VmExitKind::Pause,
            0x400..=0x402 => VmExitKind::Avic,
            0x403 => VmExitKind::InvalidGuestState,
            0x404 => VmExitKind::BusLock,
            0x800 => VmExitKind::NestedPageFault,
            0x4010..=0x401f => VmExitKind::Security,
            _ => VmExitKind::Unknown,
        }
    }

    pub const fn mnemonic(self) -> &'static str {
        match self.code {
            0x060 => "intr",
            0x061 => "nmi",
            0x062 => "smi",
            0x063 => "init",
            0x064 => "vintr",
            0x072 => "cpuid",
            0x078 => "hlt",
            0x07b => "ioio",
            0x07c => "msr",
            0x07d => "task_switch",
            0x07f => "shutdown",
            0x080 => "vmrun",
            0x081 => "vmmcall",
            0x082 => "vmload",
            0x083 => "vmsave",
            0x084 => "stgi",
            0x085 => "clgi",
            0x086 => "skinit",
            0x087 => "rdtscp",
            0x088 => "icebp",
            0x089 => "wbinvd",
            0x08a => "monitor",
            0x08b => "mwait",
            0x08c => "mwait_conditional",
            0x08d => "xsetbv",
            0x08e => "rdpru",
            0x400 => "avic_incomplete_ipi",
            0x401 => "avic_no_acceleration",
            0x402 => "vmgexit",
            0x403 => "invalid",
            0x404 => "bus_lock",
            0x800 => "nested_page_fault",
            _ => "unknown",
        }
    }

    pub const fn exception_vector(self) -> Option<u8> {
        if self.code >= 0x040 && self.code <= 0x05f {
            Some((self.code - 0x040) as u8)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestMemoryAccess {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub user: bool,
}

impl GuestMemoryAccess {
    pub const fn required_permissions(self) -> GuestPagePermissions {
        let mut bits = 0;
        if self.read {
            bits |= GuestPagePermissions::READ.bits();
        }
        if self.write {
            bits |= GuestPagePermissions::WRITE.bits();
        }
        if self.execute {
            bits |= GuestPagePermissions::EXECUTE.bits();
        }
        if self.user && self.execute {
            bits |= GuestPagePermissions::USER_EXECUTE.bits();
        }
        GuestPagePermissions::from_bits(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EptViolationQualification {
    pub access: GuestMemoryAccess,
    pub ept_permissions: GuestPagePermissions,
    pub guest_linear_address_valid: bool,
    pub guest_linear_translation: bool,
    pub user_mode_linear_address: bool,
    pub linear_page_writable: bool,
    pub linear_page_execute_disabled: bool,
    pub nmi_unblocking: bool,
    pub raw: u64,
}

impl EptViolationQualification {
    pub const fn decode(raw: u64) -> Self {
        let mut ept_permissions = GuestPagePermissions::from_bits(((raw >> 3) & 0x7) as u8);
        if raw & (1 << 10) != 0 {
            ept_permissions = ept_permissions.union(GuestPagePermissions::USER_EXECUTE);
        }
        Self {
            access: GuestMemoryAccess {
                read: raw & 1 != 0,
                write: raw & 2 != 0,
                execute: raw & 4 != 0,
                user: raw & (1 << 9) != 0,
            },
            ept_permissions,
            guest_linear_address_valid: raw & (1 << 7) != 0,
            guest_linear_translation: raw & (1 << 8) != 0,
            user_mode_linear_address: raw & (1 << 9) != 0,
            linear_page_writable: raw & (1 << 10) != 0,
            linear_page_execute_disabled: raw & (1 << 11) != 0,
            nmi_unblocking: raw & (1 << 12) != 0,
            raw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedPageFaultQualification {
    pub present: bool,
    pub access: GuestMemoryAccess,
    pub reserved_bit: bool,
    pub final_translation: bool,
    pub guest_page_table_walk: bool,
    pub raw: u64,
}

impl NestedPageFaultQualification {
    pub const fn decode(raw: u64) -> Self {
        Self {
            present: raw & 1 != 0,
            access: GuestMemoryAccess {
                read: raw & 2 == 0 && raw & (1 << 4) == 0,
                write: raw & 2 != 0,
                execute: raw & (1 << 4) != 0,
                user: raw & (1 << 2) != 0,
            },
            reserved_bit: raw & (1 << 3) != 0,
            final_translation: raw & (1 << 32) != 0,
            guest_page_table_walk: raw & (1 << 33) != 0,
            raw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Out,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoExitQualification {
    pub port: u16,
    pub size: u8,
    pub direction: IoDirection,
    pub string: bool,
    pub repeat: bool,
    pub immediate_port: bool,
    pub segment: u8,
    pub address_size: u8,
    pub raw: u64,
}

impl IoExitQualification {
    pub fn decode_intel(raw: u64) -> Result<Self, VmExitError> {
        let size = match raw & 0x7 {
            0 => 1,
            1 => 2,
            3 => 4,
            value => return Err(VmExitError::InvalidIoSize(value as u8)),
        };
        Ok(Self {
            port: (raw >> 16) as u16,
            size,
            direction: if raw & (1 << 3) != 0 {
                IoDirection::In
            } else {
                IoDirection::Out
            },
            string: raw & (1 << 4) != 0,
            repeat: raw & (1 << 5) != 0,
            immediate_port: raw & (1 << 6) != 0,
            segment: 0,
            address_size: 0,
            raw,
        })
    }

    pub fn decode_amd(raw: u64) -> Result<Self, VmExitError> {
        let size = match (raw >> 4) & 0x7 {
            1 => 1,
            2 => 2,
            4 => 4,
            value => return Err(VmExitError::InvalidIoSize(value as u8)),
        };
        Ok(Self {
            port: (raw >> 16) as u16,
            size,
            direction: if raw & 1 != 0 {
                IoDirection::In
            } else {
                IoDirection::Out
            },
            string: raw & (1 << 2) != 0,
            repeat: raw & (1 << 3) != 0,
            immediate_port: false,
            segment: ((raw >> 10) & 0x7) as u8,
            address_size: ((raw >> 7) & 0x7) as u8,
            raw,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlRegisterAccessType {
    MoveToControlRegister,
    MoveFromControlRegister,
    Clts,
    Lmsw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRegisterExitQualification {
    pub control_register: u8,
    pub access: ControlRegisterAccessType,
    pub general_register: u8,
    pub lmsw_memory_operand: bool,
    pub lmsw_source: u16,
    pub raw: u64,
}

impl ControlRegisterExitQualification {
    pub const fn decode_intel(raw: u64) -> Self {
        Self {
            control_register: (raw & 0xf) as u8,
            access: match (raw >> 4) & 0x3 {
                0 => ControlRegisterAccessType::MoveToControlRegister,
                1 => ControlRegisterAccessType::MoveFromControlRegister,
                2 => ControlRegisterAccessType::Clts,
                _ => ControlRegisterAccessType::Lmsw,
            },
            general_register: ((raw >> 8) & 0xf) as u8,
            lmsw_memory_operand: raw & (1 << 6) != 0,
            lmsw_source: (raw >> 16) as u16,
            raw,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntelVmExitSnapshot {
    pub reason: u64,
    pub instruction_length: u64,
    pub qualification: u64,
    pub guest_physical_address: u64,
    pub guest_linear_address: u64,
    pub interruption_information: u64,
    pub interruption_error_code: u64,
    pub idt_vectoring_information: u64,
    pub idt_vectoring_error_code: u64,
    pub guest_rip: u64,
    pub guest_rflags: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AmdVmExitSnapshot {
    pub exit_code: u64,
    pub exit_info1: u64,
    pub exit_info2: u64,
    pub exit_interrupt_information: u64,
    pub next_rip: u64,
    pub guest_rip: u64,
    pub guest_rflags: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmExit {
    pub vendor: VmVendor,
    pub kind: VmExitKind,
    pub entry_failure: bool,
    pub raw_reason: u64,
    pub mnemonic: &'static str,
    pub qualification: u64,
    pub guest_physical_address: Option<GuestPhysicalAddress>,
    pub guest_linear_address: Option<u64>,
    pub instruction_length: u32,
    pub interruption: Option<VmEvent>,
    pub vectoring: Option<VmEvent>,
    pub guest_rip: u64,
    pub guest_rflags: u64,
}

impl IntelVmExitSnapshot {
    pub fn decode(self) -> Result<VmExit, VmExitError> {
        let reason = IntelVmExitReason::decode(self.reason as u32);
        let interruption = VmInterruptionInformation::decode(self.interruption_information as u32)
            .event(
                Some(self.interruption_error_code as u32),
                Some(self.instruction_length as u32),
            )?;
        let vectoring = VmInterruptionInformation::decode(self.idt_vectoring_information as u32)
            .event(
                Some(self.idt_vectoring_error_code as u32),
                Some(self.instruction_length as u32),
            )?;
        Ok(VmExit {
            vendor: VmVendor::Intel,
            kind: reason.kind(),
            entry_failure: reason.entry_failure,
            raw_reason: self.reason as u64,
            mnemonic: reason.mnemonic(),
            qualification: self.qualification,
            guest_physical_address: matches!(
                reason.kind(),
                VmExitKind::EptViolation | VmExitKind::EptMisconfiguration
            )
            .then_some(GuestPhysicalAddress::new(self.guest_physical_address)),
            guest_linear_address: (self.guest_linear_address != 0)
                .then_some(self.guest_linear_address),
            instruction_length: self.instruction_length as u32,
            interruption,
            vectoring,
            guest_rip: self.guest_rip,
            guest_rflags: self.guest_rflags,
        })
    }
}

impl AmdVmExitSnapshot {
    pub fn decode(self) -> Result<VmExit, VmExitError> {
        let reason = AmdVmExitReason {
            code: self.exit_code,
        };
        let info = VmInterruptionInformation::decode(self.exit_interrupt_information as u32);
        let interruption =
            info.event(Some((self.exit_interrupt_information >> 32) as u32), None)?;
        let instruction_length = self.next_rip.saturating_sub(self.guest_rip) as u32;
        Ok(VmExit {
            vendor: VmVendor::Amd,
            kind: reason.kind(),
            entry_failure: reason.kind() == VmExitKind::InvalidGuestState,
            raw_reason: self.exit_code,
            mnemonic: reason.mnemonic(),
            qualification: self.exit_info1,
            guest_physical_address: (reason.kind() == VmExitKind::NestedPageFault)
                .then_some(GuestPhysicalAddress::new(self.exit_info2)),
            guest_linear_address: None,
            instruction_length,
            interruption,
            vectoring: None,
            guest_rip: self.guest_rip,
            guest_rflags: self.guest_rflags,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuestRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmExitAction {
    Resume,
    Advance,
    Inject(VmEvent),
    AdvanceAndInject(VmEvent),
    Yield,
    Halt,
    Shutdown,
    Unhandled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmLoopStopReason {
    Halted,
    Shutdown,
    TripleFault,
    Yielded,
    ExitBudgetExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmLoopOutcome {
    pub stop_reason: VmLoopStopReason,
    pub exits: u64,
    pub injected_events: u64,
    pub advanced_instructions: u64,
    pub last_exit: Option<VmExit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmExitError {
    InvalidExceptionVector(u8),
    InvalidEventType(u8),
    InvalidEventErrorCode { vector: u8, required: bool },
    SoftwareEventNeedsInstructionLength,
    MissingExitErrorCode,
    InvalidIoSize(u8),
    EventQueueFull,
    TripleFault,
    ZeroExitBudget,
    Engine(&'static str),
    Handler(&'static str),
}

impl fmt::Display for VmExitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExceptionVector(vector) => {
                write!(f, "invalid architectural exception vector {vector}")
            }
            Self::InvalidEventType(kind) => write!(f, "invalid VM event type {kind}"),
            Self::InvalidEventErrorCode { vector, required } => write!(
                f,
                "event vector {vector} {} an error code",
                if *required {
                    "requires"
                } else {
                    "does not accept"
                }
            ),
            Self::SoftwareEventNeedsInstructionLength => {
                write!(f, "software event injection needs instruction length")
            }
            Self::MissingExitErrorCode => write!(f, "VM-exit omitted a required error code"),
            Self::InvalidIoSize(size) => write!(f, "invalid intercepted IO width encoding {size}"),
            Self::EventQueueFull => write!(f, "pending VM event queue is full"),
            Self::TripleFault => write!(f, "exception delivery escalated to triple fault"),
            Self::ZeroExitBudget => write!(f, "VM-exit loop needs a non-zero exit budget"),
            Self::Engine(message) => write!(f, "VM execution engine failed: {message}"),
            Self::Handler(message) => write!(f, "VM-exit handler failed: {message}"),
        }
    }
}

impl std::error::Error for VmExitError {}

pub struct PendingVmEvents<const CAPACITY: usize> {
    events: [Option<VmEvent>; CAPACITY],
    delivering: Option<VmEvent>,
}

impl<const CAPACITY: usize> Default for PendingVmEvents<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> PendingVmEvents<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            events: [None; CAPACITY],
            delivering: None,
        }
    }

    pub fn len(&self) -> usize {
        self.events.iter().flatten().count() + usize::from(self.delivering.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn priority(event: VmEvent) -> u8 {
        match event.kind {
            VmEventKind::NonMaskableInterrupt => 0,
            VmEventKind::HardwareException => 1,
            VmEventKind::ExternalInterrupt => 2,
            _ => 3,
        }
    }

    pub fn push(&mut self, event: VmEvent) -> Result<(), VmExitError> {
        let event = event.validate()?;
        let slot = self
            .events
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(VmExitError::EventQueueFull)?;
        *slot = Some(event);
        Ok(())
    }

    pub fn pop_highest_priority(&mut self) -> Option<VmEvent> {
        let index = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.map(|event| (index, Self::priority(event))))
            .min_by_key(|(_, priority)| *priority)
            .map(|(index, _)| index)?;
        let event = self.events[index].take();
        self.delivering = event;
        event
    }

    pub fn delivering(&self) -> Option<VmEvent> {
        self.delivering
    }

    pub fn requeue_delivery(&mut self) -> Result<(), VmExitError> {
        let Some(event) = self.delivering.take() else {
            return Ok(());
        };
        self.push(event)
    }

    pub fn complete_delivery(&mut self) {
        self.delivering = None;
    }

    pub fn delivery_fault(&mut self, new_event: VmEvent) -> Result<(), VmExitError> {
        let Some(delivering) = self.delivering.take() else {
            return self.push(new_event);
        };
        let delivering_exception = if delivering.kind == VmEventKind::HardwareException {
            X86ExceptionVector::from_vector(delivering.vector)
        } else {
            None
        };
        let new_exception = if new_event.kind == VmEventKind::HardwareException {
            X86ExceptionVector::from_vector(new_event.vector)
        } else {
            None
        };

        if delivering_exception == Some(X86ExceptionVector::DoubleFault) {
            return Err(VmExitError::TripleFault);
        }
        let double_fault = matches!(
            (
                delivering_exception.map(X86ExceptionVector::class),
                new_exception.map(X86ExceptionVector::class)
            ),
            (
                Some(ExceptionClass::Contributory),
                Some(ExceptionClass::Contributory | ExceptionClass::PageFault)
            ) | (
                Some(ExceptionClass::PageFault),
                Some(ExceptionClass::Contributory | ExceptionClass::PageFault)
            )
        );
        if double_fault {
            self.push(VmEvent::exception(X86ExceptionVector::DoubleFault, Some(0)))
        } else {
            self.push(new_event)
        }
    }
}

pub trait VmExecutionEngine {
    fn enter(&mut self) -> Result<VmExit, VmExitError>;
    fn registers(&self) -> GuestRegisters;
    fn set_registers(&mut self, registers: GuestRegisters) -> Result<(), VmExitError>;
    fn advance_rip(&mut self, instruction_length: u32) -> Result<(), VmExitError>;
    fn inject_event(&mut self, event: VmEvent) -> Result<(), VmExitError>;

    fn event_injection_ready(&self, _event: VmEvent) -> bool {
        true
    }

    fn request_event_window(&mut self, _event: VmEvent) -> Result<(), VmExitError> {
        Ok(())
    }
}

pub trait VmExitHandler<E: VmExecutionEngine, const EVENTS: usize> {
    fn handle(
        &mut self,
        engine: &mut E,
        exit: &VmExit,
        pending: &mut PendingVmEvents<EVENTS>,
    ) -> Result<VmExitAction, VmExitError>;
}

pub fn run_vm_exit_loop<E: VmExecutionEngine, H: VmExitHandler<E, EVENTS>, const EVENTS: usize>(
    engine: &mut E,
    handler: &mut H,
    pending: &mut PendingVmEvents<EVENTS>,
    exit_budget: u64,
) -> Result<VmLoopOutcome, VmExitError> {
    if exit_budget == 0 {
        return Err(VmExitError::ZeroExitBudget);
    }
    let mut outcome = VmLoopOutcome {
        stop_reason: VmLoopStopReason::ExitBudgetExhausted,
        exits: 0,
        injected_events: 0,
        advanced_instructions: 0,
        last_exit: None,
    };

    while outcome.exits < exit_budget {
        if pending.delivering().is_none() {
            if let Some(event) = pending.pop_highest_priority() {
                if engine.event_injection_ready(event) {
                    if let Err(error) = engine.inject_event(event) {
                        pending.requeue_delivery()?;
                        return Err(error);
                    }
                    outcome.injected_events += 1;
                } else {
                    pending.requeue_delivery()?;
                    engine.request_event_window(event)?;
                }
            }
        }

        let exit = engine.enter()?;
        outcome.exits += 1;
        outcome.last_exit = Some(exit);
        let mut interruption_consumed_by_delivery = false;
        if pending.delivering().is_some() {
            if let Some(vectoring) = exit.vectoring {
                if let Some(fault) = exit.interruption {
                    pending.delivery_fault(fault)?;
                    interruption_consumed_by_delivery = true;
                } else {
                    pending.complete_delivery();
                    pending.push(vectoring)?;
                }
            } else if exit.entry_failure {
                pending.requeue_delivery()?;
            } else {
                pending.complete_delivery();
            }
        }
        if exit.kind == VmExitKind::TripleFault {
            outcome.stop_reason = VmLoopStopReason::TripleFault;
            return Ok(outcome);
        }

        let action = handler.handle(engine, &exit, pending)?;
        match action {
            VmExitAction::Resume => {}
            VmExitAction::Advance => {
                engine.advance_rip(exit.instruction_length)?;
                outcome.advanced_instructions += 1;
            }
            VmExitAction::Inject(event) => pending.push(event)?,
            VmExitAction::AdvanceAndInject(event) => {
                engine.advance_rip(exit.instruction_length)?;
                outcome.advanced_instructions += 1;
                pending.push(event)?;
            }
            VmExitAction::Yield => {
                outcome.stop_reason = VmLoopStopReason::Yielded;
                return Ok(outcome);
            }
            VmExitAction::Halt => {
                outcome.stop_reason = VmLoopStopReason::Halted;
                return Ok(outcome);
            }
            VmExitAction::Shutdown => {
                outcome.stop_reason = VmLoopStopReason::Shutdown;
                return Ok(outcome);
            }
            VmExitAction::Unhandled => {
                if interruption_consumed_by_delivery {
                    continue;
                } else if let Some(event) = exit.interruption {
                    pending.delivery_fault(event)?;
                } else {
                    return Err(VmExitError::Handler(
                        "exit was unhandled and contains no reinjectable event",
                    ));
                }
            }
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_and_amd_exit_snapshots_decode_material_fields() {
        let intel = IntelVmExitSnapshot {
            reason: 48,
            instruction_length: 3,
            qualification: 0b11 | (1 << 7),
            guest_physical_address: 0x1234_5000,
            guest_linear_address: 0xffff_8000_1234_5000,
            guest_rip: 0x1000,
            guest_rflags: 2,
            ..IntelVmExitSnapshot::default()
        }
        .decode()
        .unwrap();
        assert_eq!(intel.kind, VmExitKind::EptViolation);
        assert_eq!(intel.mnemonic, "ept_violation");
        assert_eq!(
            intel.guest_physical_address,
            Some(GuestPhysicalAddress::new(0x1234_5000))
        );
        let ept = EptViolationQualification::decode(intel.qualification);
        assert!(ept.access.read && ept.access.write);
        assert!(ept.guest_linear_address_valid);

        let amd = AmdVmExitSnapshot {
            exit_code: 0x800,
            exit_info1: 2 | (1 << 32),
            exit_info2: 0x8000,
            next_rip: 0x2003,
            guest_rip: 0x2000,
            guest_rflags: 2,
            ..AmdVmExitSnapshot::default()
        }
        .decode()
        .unwrap();
        assert_eq!(amd.kind, VmExitKind::NestedPageFault);
        assert_eq!(amd.instruction_length, 3);
        let npf = NestedPageFaultQualification::decode(amd.qualification);
        assert!(npf.access.write && npf.final_translation);
    }

    #[test]
    fn event_encodings_and_exception_escalation_follow_architecture() {
        let page_fault = VmEvent::exception(X86ExceptionVector::PageFault, Some(5));
        assert_eq!(
            page_fault.intel_interruption_information().unwrap(),
            14 | (3 << 8) | (1 << 11) | (1 << 31)
        );
        assert_eq!(
            page_fault.amd_event_injection().unwrap(),
            (5_u64 << 32) | 14 | (3 << 8) | (1 << 11) | (1 << 31)
        );

        let mut queue = PendingVmEvents::<4>::new();
        queue
            .push(VmEvent::exception(
                X86ExceptionVector::GeneralProtection,
                Some(0),
            ))
            .unwrap();
        assert_eq!(
            queue.pop_highest_priority().unwrap().vector,
            X86ExceptionVector::GeneralProtection as u8
        );
        queue
            .delivery_fault(VmEvent::exception(X86ExceptionVector::PageFault, Some(0)))
            .unwrap();
        assert_eq!(
            queue.pop_highest_priority().unwrap().vector,
            X86ExceptionVector::DoubleFault as u8
        );
        assert_eq!(
            queue.delivery_fault(VmEvent::exception(
                X86ExceptionVector::GeneralProtection,
                Some(0),
            )),
            Err(VmExitError::TripleFault)
        );
    }

    #[derive(Default)]
    struct ScriptedEngine {
        exits: [Option<VmExit>; 4],
        next: usize,
        registers: GuestRegisters,
        injections: [Option<VmEvent>; 4],
        injection_count: usize,
        advances: u64,
        injection_ready: bool,
        window_requests: u64,
    }

    impl VmExecutionEngine for ScriptedEngine {
        fn enter(&mut self) -> Result<VmExit, VmExitError> {
            let exit = self
                .exits
                .get_mut(self.next)
                .and_then(Option::take)
                .ok_or(VmExitError::Engine("script exhausted"))?;
            self.next += 1;
            Ok(exit)
        }

        fn registers(&self) -> GuestRegisters {
            self.registers
        }

        fn set_registers(&mut self, registers: GuestRegisters) -> Result<(), VmExitError> {
            self.registers = registers;
            Ok(())
        }

        fn advance_rip(&mut self, instruction_length: u32) -> Result<(), VmExitError> {
            self.registers.rip += instruction_length as u64;
            self.advances += 1;
            Ok(())
        }

        fn inject_event(&mut self, event: VmEvent) -> Result<(), VmExitError> {
            self.injections[self.injection_count] = Some(event);
            self.injection_count += 1;
            Ok(())
        }

        fn event_injection_ready(&self, _event: VmEvent) -> bool {
            self.injection_ready
        }

        fn request_event_window(&mut self, _event: VmEvent) -> Result<(), VmExitError> {
            self.window_requests += 1;
            Ok(())
        }
    }

    struct ScriptedHandler;

    impl VmExitHandler<ScriptedEngine, 4> for ScriptedHandler {
        fn handle(
            &mut self,
            engine: &mut ScriptedEngine,
            exit: &VmExit,
            pending: &mut PendingVmEvents<4>,
        ) -> Result<VmExitAction, VmExitError> {
            match exit.kind {
                VmExitKind::EptViolation => Ok(VmExitAction::Resume),
                VmExitKind::Cpuid => {
                    let mut registers = engine.registers();
                    registers.rax = 0x756e_6547;
                    engine.set_registers(registers)?;
                    pending.push(VmEvent::external_interrupt(0x20))?;
                    Ok(VmExitAction::Advance)
                }
                VmExitKind::Halt => Ok(VmExitAction::Halt),
                _ => Ok(VmExitAction::Unhandled),
            }
        }
    }

    fn scripted_exit(kind: VmExitKind, instruction_length: u32) -> VmExit {
        VmExit {
            vendor: VmVendor::Intel,
            kind,
            entry_failure: false,
            raw_reason: 0,
            mnemonic: "scripted",
            qualification: 0,
            guest_physical_address: None,
            guest_linear_address: None,
            instruction_length,
            interruption: None,
            vectoring: None,
            guest_rip: 0,
            guest_rflags: 2,
        }
    }

    #[test]
    fn complete_loop_retries_memory_advances_emulation_injects_and_halts() {
        let mut engine = ScriptedEngine {
            exits: [
                Some(scripted_exit(VmExitKind::EptViolation, 0)),
                Some(scripted_exit(VmExitKind::Cpuid, 2)),
                Some(scripted_exit(VmExitKind::Halt, 1)),
                None,
            ],
            registers: GuestRegisters {
                rip: 0x1000,
                ..GuestRegisters::default()
            },
            injection_ready: true,
            ..ScriptedEngine::default()
        };
        let mut pending = PendingVmEvents::<4>::new();
        let outcome = run_vm_exit_loop(&mut engine, &mut ScriptedHandler, &mut pending, 8).unwrap();
        assert_eq!(outcome.stop_reason, VmLoopStopReason::Halted);
        assert_eq!(outcome.exits, 3);
        assert_eq!(outcome.advanced_instructions, 1);
        assert_eq!(outcome.injected_events, 1);
        assert_eq!(engine.registers.rip, 0x1002);
        assert_eq!(
            engine.injections[0],
            Some(VmEvent::external_interrupt(0x20))
        );
    }

    struct InterruptWindowHandler;

    impl VmExitHandler<ScriptedEngine, 4> for InterruptWindowHandler {
        fn handle(
            &mut self,
            engine: &mut ScriptedEngine,
            exit: &VmExit,
            _pending: &mut PendingVmEvents<4>,
        ) -> Result<VmExitAction, VmExitError> {
            match exit.kind {
                VmExitKind::InterruptWindow => {
                    engine.injection_ready = true;
                    Ok(VmExitAction::Resume)
                }
                VmExitKind::Halt => Ok(VmExitAction::Halt),
                _ => Ok(VmExitAction::Unhandled),
            }
        }
    }

    #[test]
    fn blocked_interrupt_is_retained_until_the_guest_window_opens() {
        let mut engine = ScriptedEngine {
            exits: [
                Some(scripted_exit(VmExitKind::InterruptWindow, 0)),
                Some(scripted_exit(VmExitKind::Halt, 1)),
                None,
                None,
            ],
            ..ScriptedEngine::default()
        };
        let interrupt = VmEvent::external_interrupt(0x31);
        let mut pending = PendingVmEvents::<4>::new();
        pending.push(interrupt).unwrap();

        let outcome =
            run_vm_exit_loop(&mut engine, &mut InterruptWindowHandler, &mut pending, 4).unwrap();

        assert_eq!(outcome.stop_reason, VmLoopStopReason::Halted);
        assert_eq!(engine.window_requests, 1);
        assert_eq!(engine.injection_count, 1);
        assert_eq!(engine.injections[0], Some(interrupt));
        assert!(pending.is_empty());
        assert_eq!(pending.delivering(), None);
    }

    struct EntryFailureHandler;

    impl VmExitHandler<ScriptedEngine, 4> for EntryFailureHandler {
        fn handle(
            &mut self,
            _engine: &mut ScriptedEngine,
            exit: &VmExit,
            _pending: &mut PendingVmEvents<4>,
        ) -> Result<VmExitAction, VmExitError> {
            match exit.kind {
                VmExitKind::InvalidGuestState => Ok(VmExitAction::Resume),
                VmExitKind::Halt => Ok(VmExitAction::Halt),
                _ => Ok(VmExitAction::Unhandled),
            }
        }
    }

    #[test]
    fn vm_entry_failure_requeues_the_not_delivered_event() {
        let mut failed_entry = scripted_exit(VmExitKind::InvalidGuestState, 0);
        failed_entry.entry_failure = true;
        let mut engine = ScriptedEngine {
            exits: [
                Some(failed_entry),
                Some(scripted_exit(VmExitKind::Halt, 1)),
                None,
                None,
            ],
            injection_ready: true,
            ..ScriptedEngine::default()
        };
        let interrupt = VmEvent::external_interrupt(0x40);
        let mut pending = PendingVmEvents::<4>::new();
        pending.push(interrupt).unwrap();

        let outcome =
            run_vm_exit_loop(&mut engine, &mut EntryFailureHandler, &mut pending, 4).unwrap();

        assert_eq!(outcome.stop_reason, VmLoopStopReason::Halted);
        assert_eq!(engine.injection_count, 2);
        assert_eq!(engine.injections[0], Some(interrupt));
        assert_eq!(engine.injections[1], Some(interrupt));
        assert!(pending.is_empty());
        assert_eq!(pending.delivering(), None);
    }

    struct DeliveryFaultHandler;

    impl VmExitHandler<ScriptedEngine, 4> for DeliveryFaultHandler {
        fn handle(
            &mut self,
            _engine: &mut ScriptedEngine,
            exit: &VmExit,
            _pending: &mut PendingVmEvents<4>,
        ) -> Result<VmExitAction, VmExitError> {
            if exit.kind == VmExitKind::Halt {
                Ok(VmExitAction::Halt)
            } else {
                Ok(VmExitAction::Unhandled)
            }
        }
    }

    #[test]
    fn vectoring_fault_escalates_once_and_reinjects_double_fault() {
        let general_protection = VmEvent::exception(X86ExceptionVector::GeneralProtection, Some(0));
        let mut delivery_exit = scripted_exit(VmExitKind::ExceptionOrNmi, 0);
        delivery_exit.vectoring = Some(general_protection);
        delivery_exit.interruption =
            Some(VmEvent::exception(X86ExceptionVector::PageFault, Some(0)));
        let mut engine = ScriptedEngine {
            exits: [
                Some(delivery_exit),
                Some(scripted_exit(VmExitKind::Halt, 1)),
                None,
                None,
            ],
            injection_ready: true,
            ..ScriptedEngine::default()
        };
        let mut pending = PendingVmEvents::<4>::new();
        pending.push(general_protection).unwrap();

        let outcome =
            run_vm_exit_loop(&mut engine, &mut DeliveryFaultHandler, &mut pending, 4).unwrap();

        assert_eq!(outcome.stop_reason, VmLoopStopReason::Halted);
        assert_eq!(engine.injection_count, 2);
        assert_eq!(engine.injections[0], Some(general_protection));
        assert_eq!(
            engine.injections[1],
            Some(VmEvent::exception(X86ExceptionVector::DoubleFault, Some(0)))
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn io_and_control_qualifications_decode_without_string_parsing() {
        let io = IoExitQualification::decode_intel((0x3f8 << 16) | (1 << 6)).unwrap();
        assert_eq!(io.port, 0x3f8);
        assert_eq!(io.size, 1);
        assert_eq!(io.direction, IoDirection::Out);
        assert!(io.immediate_port);
        let cr = ControlRegisterExitQualification::decode_intel(3 | (1 << 4) | (9 << 8));
        assert_eq!(cr.control_register, 3);
        assert_eq!(
            cr.access,
            ControlRegisterAccessType::MoveFromControlRegister
        );
        assert_eq!(cr.general_register, 9);
    }
}
