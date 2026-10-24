use core::fmt;

use crate::firmware::{FirmwareArchitecture, FirmwareTopology, KernelCpuTopologyEntryV1};
use crate::x86_apic::{
    ApicDeliveryMode, ApicDestinationMode, ApicDestinationShorthand, ApicInterruptCommand,
    ApicLevel, ApicTriggerMode,
};

pub const SMP_MAX_CPUS: usize = 256;
pub const SMP_STACK_ALIGNMENT: u64 = 16;
pub const X86_AP_TRAMPOLINE_LIMIT: u64 = 0x10_0000;
pub const X86_AP_MAILBOX_ADDRESS: u64 = 0x6000;
pub const X86_AP_TRAMPOLINE_ADDRESS: u64 = 0x7000;
pub const X86_AP_TRAMPOLINE_PAGE_SIZE: usize = 0x1000;
pub const X86_INIT_ASSERT_DELAY_US: u32 = 10_000;
pub const X86_SIPI_DELAY_US: u32 = 200;
pub const PSCI_VERSION_0_2: u32 = 0x0000_0002;
pub const PSCI_FN_VERSION: u32 = 0x8400_0000;
pub const PSCI_FN_CPU_SUSPEND_64: u32 = 0xc400_0001;
pub const PSCI_FN_CPU_OFF: u32 = 0x8400_0002;
pub const PSCI_FN_CPU_ON_64: u32 = 0xc400_0003;
pub const PSCI_FN_AFFINITY_INFO_64: u32 = 0xc400_0004;
pub const PSCI_FN_SYSTEM_OFF: u32 = 0x8400_0008;
pub const PSCI_FN_SYSTEM_RESET: u32 = 0x8400_0009;
pub const SBI_EXT_TIME: u64 = 0x5449_4d45;
pub const SBI_EXT_IPI: u64 = 0x0073_5049;
pub const SBI_EXT_RFENCE: u64 = 0x5246_4e43;
pub const SBI_EXT_HSM: u64 = 0x0048_534d;
pub const SBI_HSM_HART_START: u64 = 0;
pub const SBI_HSM_HART_STOP: u64 = 1;
pub const SBI_HSM_HART_STATUS: u64 = 2;
pub const SBI_HSM_HART_SUSPEND: u64 = 3;
pub const SBI_RFENCE_REMOTE_FENCE_I: u64 = 0;
pub const SBI_RFENCE_REMOTE_SFENCE_VMA: u64 = 1;
pub const SBI_RFENCE_REMOTE_SFENCE_VMA_ASID: u64 = 2;
pub const KERNEL_SMP_PLATFORM_MAGIC: u64 = u64::from_le_bytes(*b"NCCSMP1\0");
pub const SECONDARY_CPU_MAILBOX_MAGIC: u64 = u64::from_le_bytes(*b"NCCAPM1\0");
pub const PSCI_SUCCESS: i64 = 0;
pub const PSCI_NOT_SUPPORTED: i64 = -1;
pub const PSCI_INVALID_PARAMETERS: i64 = -2;
pub const PSCI_DENIED: i64 = -3;
pub const PSCI_ALREADY_ON: i64 = -4;
pub const PSCI_ON_PENDING: i64 = -5;
pub const PSCI_INTERNAL_FAILURE: i64 = -6;
pub const PSCI_NOT_PRESENT: i64 = -7;
pub const PSCI_DISABLED: i64 = -8;
pub const PSCI_INVALID_ADDRESS: i64 = -9;
pub const PSCI_AFFINITY_ON: i64 = 0;
pub const PSCI_AFFINITY_OFF: i64 = 1;
pub const PSCI_AFFINITY_ON_PENDING: i64 = 2;
pub const SBI_SUCCESS: i64 = 0;
pub const SBI_ERR_FAILED: i64 = -1;
pub const SBI_ERR_NOT_SUPPORTED: i64 = -2;
pub const SBI_ERR_INVALID_PARAM: i64 = -3;
pub const SBI_ERR_DENIED: i64 = -4;
pub const SBI_ERR_INVALID_ADDRESS: i64 = -5;
pub const SBI_ERR_ALREADY_AVAILABLE: i64 = -6;
pub const SBI_ERR_ALREADY_STARTED: i64 = -7;
pub const SBI_ERR_ALREADY_STOPPED: i64 = -8;
pub const SBI_HSM_STARTED: i64 = 0;
pub const SBI_HSM_STOPPED: i64 = 1;
pub const SBI_HSM_START_PENDING: i64 = 2;
pub const SBI_HSM_STOP_PENDING: i64 = 3;
pub const SBI_HSM_SUSPENDED: i64 = 4;
pub const SBI_HSM_SUSPEND_PENDING: i64 = 5;
pub const SBI_HSM_RESUME_PENDING: i64 = 6;

// Reset-vector entry for an x86_64 application processor. It enters protected
// mode, enables PAE and long mode with the mailbox CR3, then invokes the nC
// entry with its explicit context. Production lowering copies these bytes to
// X86_AP_TRAMPOLINE_ADDRESS; no external assembler participates in a build.
pub const X86_64_AP_TRAMPOLINE_BYTES: &[u8] = &[
    0xfa, 0xfc, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0x0f, 0x01, 0x16, 0x30, 0x71, 0x0f,
    0x20, 0xc0, 0x66, 0x83, 0xc8, 0x01, 0x0f, 0x22, 0xc0, 0x66, 0xea, 0x21, 0x70, 0x00, 0x00, 0x08,
    0x00, 0x66, 0xb8, 0x10, 0x00, 0x66, 0x8e, 0xd8, 0x66, 0x8e, 0xc0, 0x66, 0x8e, 0xd0, 0x66, 0x8e,
    0xe0, 0x66, 0x8e, 0xe8, 0x0f, 0x20, 0xe0, 0x83, 0xc8, 0x20, 0x0f, 0x22, 0xe0, 0xa1, 0x30, 0x60,
    0x00, 0x00, 0x0f, 0x22, 0xd8, 0xb9, 0x80, 0x00, 0x00, 0xc0, 0x0f, 0x32, 0x0d, 0x00, 0x01, 0x00,
    0x00, 0x0f, 0x30, 0x0f, 0x20, 0xc0, 0x0d, 0x00, 0x00, 0x00, 0x80, 0x0f, 0x22, 0xc0, 0xea, 0x65,
    0x70, 0x00, 0x00, 0x18, 0x00, 0x66, 0xb8, 0x10, 0x00, 0x66, 0x8e, 0xd8, 0x66, 0x8e, 0xc0, 0x66,
    0x8e, 0xd0, 0x66, 0x8e, 0xe0, 0x66, 0x8e, 0xe8, 0x48, 0xba, 0x4e, 0x43, 0x43, 0x41, 0x50, 0x4d,
    0x31, 0x00, 0x48, 0x8b, 0x04, 0x25, 0x00, 0x60, 0x00, 0x00, 0x48, 0x39, 0xd0, 0x75, 0x70, 0x83,
    0x3c, 0x25, 0x14, 0x60, 0x00, 0x00, 0x01, 0x75, 0x66, 0x48, 0x83, 0x3c, 0x25, 0x08, 0x60, 0x00,
    0x00, 0x00, 0x74, 0x5b, 0x48, 0x8b, 0x24, 0x25, 0x18, 0x60, 0x00, 0x00, 0x48, 0x85, 0xe4, 0x74,
    0x4e, 0x48, 0x83, 0xe4, 0xf0, 0x31, 0xed, 0x4c, 0x8b, 0x24, 0x25, 0x28, 0x60, 0x00, 0x00, 0x4c,
    0x8b, 0x2c, 0x25, 0x20, 0x60, 0x00, 0x00, 0x44, 0x8b, 0x34, 0x25, 0x10, 0x60, 0x00, 0x00, 0x4c,
    0x8b, 0x3c, 0x25, 0x48, 0x60, 0x00, 0x00, 0x4d, 0x85, 0xed, 0x74, 0x23, 0xc7, 0x04, 0x25, 0x14,
    0x60, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0f, 0xae, 0xf0, 0x4c, 0x89, 0xe7, 0x41, 0xff, 0xd5,
    0x4d, 0x85, 0xff, 0x74, 0x06, 0x44, 0x89, 0xf7, 0x41, 0xff, 0xd7, 0xfa, 0xf4, 0xeb, 0xfc, 0xc7,
    0x04, 0x25, 0x14, 0x60, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x0f, 0xae, 0xf0, 0xeb, 0xec, 0x90,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xcf, 0x00,
    0xff, 0xff, 0x00, 0x00, 0x00, 0x92, 0xcf, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xaf, 0x00,
    0x1f, 0x00, 0x10, 0x71, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86ApTrampolineImage {
    pub load_address: u64,
    pub mailbox_address: u64,
    pub bytes: &'static [u8],
}

pub fn x86_64_ap_trampoline_image() -> Result<X86ApTrampolineImage, SmpError> {
    let bytes = X86_64_AP_TRAMPOLINE_BYTES;
    let valid_page = !bytes.is_empty() && bytes.len() <= X86_AP_TRAMPOLINE_PAGE_SIZE;
    let valid_entry = bytes.starts_with(&[0xfa, 0xfc, 0x31, 0xc0]);
    let valid_layout = X86_AP_MAILBOX_ADDRESS & 0xfff == 0
        && X86_AP_TRAMPOLINE_ADDRESS & 0xfff == 0
        && X86_AP_TRAMPOLINE_ADDRESS < X86_AP_TRAMPOLINE_LIMIT
        && X86_AP_MAILBOX_ADDRESS + 0x1000 == X86_AP_TRAMPOLINE_ADDRESS;
    if !valid_page || !valid_entry || !valid_layout {
        return Err(SmpError::InvalidTrampolineImage);
    }
    Ok(X86ApTrampolineImage {
        load_address: X86_AP_TRAMPOLINE_ADDRESS,
        mailbox_address: X86_AP_MAILBOX_ADDRESS,
        bytes,
    })
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpArchitecture {
    X86 = 1,
    Aarch64 = 2,
    RiscV = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpStartMethod {
    X86InitSipi = 1,
    PsciSmc = 2,
    PsciHvc = 3,
    SbiHsm = 4,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpInterruptModel {
    XApic = 1,
    X2Apic = 2,
    GicV3 = 3,
    Plic = 4,
    Imsic = 5,
    AplicImsic = 6,
    PlicImsic = 7,
    Aplic = 8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelSmpPlatformV1 {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub architecture: u32,
    pub flags: u32,
    pub start_method: u32,
    pub interrupt_model: u32,
    pub primary_controller_base: u64,
    pub secondary_controller_base: u64,
    pub tertiary_controller_base: u64,
    pub timer_frequency_hz: u64,
    pub redistributor_stride: u64,
    pub context_stride: u64,
    pub source_count: u32,
    pub context_count: u32,
    pub feature_flags: u64,
    pub reserved: u64,
}

impl KernelSmpPlatformV1 {
    pub const BYTE_SIZE: usize = 104;
    pub const FLAG_DISCOVERED_FROM_ACPI: u32 = 1 << 0;
    pub const FLAG_DISCOVERED_FROM_FDT: u32 = 1 << 1;
    pub const FLAG_CONTROLLER_COHERENT: u32 = 1 << 2;
    pub const FLAG_TIMER_INVARIANT: u32 = 1 << 3;
    pub const FEATURE_TLB_RANGE: u64 = 1 << 0;
    pub const FEATURE_TLB_ASID: u64 = 1 << 1;
    pub const FEATURE_IPI: u64 = 1 << 2;
    pub const FEATURE_PER_CPU_TIMER: u64 = 1 << 3;
    pub const FEATURE_MSI: u64 = 1 << 4;

    pub fn new(
        architecture: SmpArchitecture,
        start_method: SmpStartMethod,
        interrupt_model: SmpInterruptModel,
    ) -> Self {
        Self {
            magic: KERNEL_SMP_PLATFORM_MAGIC,
            version: 1,
            size: Self::BYTE_SIZE as u32,
            architecture: architecture as u32,
            flags: 0,
            start_method: start_method as u32,
            interrupt_model: interrupt_model as u32,
            primary_controller_base: 0,
            secondary_controller_base: 0,
            tertiary_controller_base: 0,
            timer_frequency_hz: 0,
            redistributor_stride: 0,
            context_stride: 0,
            source_count: 0,
            context_count: 0,
            feature_flags: 0,
            reserved: 0,
        }
    }

    pub fn validate(self) -> Result<Self, SmpError> {
        if self.magic != KERNEL_SMP_PLATFORM_MAGIC || self.version != 1 {
            return Err(SmpError::InvalidPlatformDescriptor);
        }
        if self.size as usize != Self::BYTE_SIZE || self.primary_controller_base & 0xfff != 0 {
            return Err(SmpError::InvalidPlatformDescriptor);
        }
        if self.flags & Self::FLAG_TIMER_INVARIANT != 0 && self.timer_frequency_hz == 0 {
            return Err(SmpError::InvalidTimerFrequency);
        }
        let pairing = match (self.architecture, self.start_method, self.interrupt_model) {
            (1, 1, 1 | 2) => true,
            (2, 2 | 3, 3) => self.secondary_controller_base != 0,
            (3, 4, 4 | 5 | 6 | 7 | 8) => true,
            _ => false,
        };
        if !pairing {
            return Err(SmpError::InvalidPlatformDescriptor);
        }
        Ok(self)
    }
}

const _: [(); KernelSmpPlatformV1::BYTE_SIZE] = [(); core::mem::size_of::<KernelSmpPlatformV1>()];

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryCpuMailboxState {
    Empty = 0,
    Prepared = 1,
    Entered = 2,
    Online = 3,
    Parked = 4,
    Failed = 5,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondaryCpuMailboxV1 {
    pub magic: u64,
    pub generation: u64,
    pub logical_id: u32,
    pub state: u32,
    pub stack_top: u64,
    pub entry: u64,
    pub context: u64,
    pub address_space_root: u64,
    pub descriptor_table: u64,
    pub interrupt_table: u64,
    pub park_callback: u64,
}

impl SecondaryCpuMailboxV1 {
    pub const BYTE_SIZE: usize = 80;

    pub fn prepared(
        generation: u64,
        logical_id: u32,
        stack_top: u64,
        entry: u64,
        context: u64,
    ) -> Result<Self, SmpError> {
        if generation == 0
            || stack_top == 0
            || stack_top & (SMP_STACK_ALIGNMENT - 1) != 0
            || entry == 0
        {
            return Err(SmpError::InvalidMailbox);
        }
        Ok(Self {
            magic: SECONDARY_CPU_MAILBOX_MAGIC,
            generation,
            logical_id,
            state: SecondaryCpuMailboxState::Prepared as u32,
            stack_top,
            entry,
            context,
            address_space_root: 0,
            descriptor_table: 0,
            interrupt_table: 0,
            park_callback: 0,
        })
    }
}

const _: [(); SecondaryCpuMailboxV1::BYTE_SIZE] =
    [(); core::mem::size_of::<SecondaryCpuMailboxV1>()];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmpError {
    EmptyTopology,
    TooManyCpus(usize),
    DuplicateLogicalId(u32),
    DuplicateHardwareId(u64),
    MissingBootstrap,
    MultipleBootstrap,
    UnknownCpu(u32),
    CpuDisabled(u32),
    CpuAlreadyOnline(u32),
    CpuNotStarted(u32),
    InvalidStack { base: u64, size: u64 },
    StackOverlap { first: u32, second: u32 },
    InvalidTrampoline(u64),
    InvalidTrampolineImage,
    InvalidInterrupt(u32),
    InvalidControllerBase(u64),
    InvalidTimerFrequency,
    InvalidShootdownGeneration { expected: u64, observed: u64 },
    ShootdownAlreadyAcknowledged(u32),
    InvalidPlatformDescriptor,
    InvalidMailbox,
    FirmwareProtocol { protocol: &'static str, status: i64 },
    FirmwareTimeout { protocol: &'static str, target: u64 },
    OperationBusy(&'static str),
    ArithmeticOverflow,
}

impl fmt::Display for SmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTopology => write!(f, "SMP topology contains no processors"),
            Self::TooManyCpus(count) => write!(
                f,
                "SMP topology has {count} processors; maximum is {SMP_MAX_CPUS}"
            ),
            Self::DuplicateLogicalId(id) => write!(f, "duplicate logical CPU id {id}"),
            Self::DuplicateHardwareId(id) => write!(f, "duplicate hardware CPU id {id:#x}"),
            Self::MissingBootstrap => write!(f, "SMP topology has no bootstrap processor"),
            Self::MultipleBootstrap => write!(f, "SMP topology has multiple bootstrap processors"),
            Self::UnknownCpu(id) => write!(f, "unknown logical CPU id {id}"),
            Self::CpuDisabled(id) => write!(f, "logical CPU {id} is disabled by firmware"),
            Self::CpuAlreadyOnline(id) => write!(f, "logical CPU {id} is already online"),
            Self::CpuNotStarted(id) => {
                write!(f, "logical CPU {id} has not reached the online state")
            }
            Self::InvalidStack { base, size } => {
                write!(f, "invalid per-CPU stack base={base:#x} size={size:#x}")
            }
            Self::StackOverlap { first, second } => {
                write!(f, "per-CPU stacks overlap for CPUs {first} and {second}")
            }
            Self::InvalidTrampoline(address) => {
                write!(f, "invalid x86 AP trampoline address {address:#x}")
            }
            Self::InvalidTrampolineImage => write!(f, "invalid x86_64 AP trampoline image"),
            Self::InvalidInterrupt(vector) => write!(f, "invalid SMP interrupt/vector {vector:#x}"),
            Self::InvalidControllerBase(base) => {
                write!(f, "invalid interrupt-controller base {base:#x}")
            }
            Self::InvalidTimerFrequency => write!(f, "timer frequency must be non-zero"),
            Self::InvalidShootdownGeneration { expected, observed } => write!(
                f,
                "TLB shootdown generation mismatch: expected {expected}, observed {observed}"
            ),
            Self::ShootdownAlreadyAcknowledged(cpu) => {
                write!(f, "CPU {cpu} acknowledged the TLB shootdown twice")
            }
            Self::InvalidPlatformDescriptor => {
                write!(f, "invalid physical SMP platform descriptor")
            }
            Self::InvalidMailbox => write!(f, "invalid secondary CPU mailbox"),
            Self::FirmwareProtocol { protocol, status } => {
                write!(f, "{protocol} returned status {status}")
            }
            Self::FirmwareTimeout { protocol, target } => {
                write!(
                    f,
                    "{protocol} timed out while waiting for target {target:#x}"
                )
            }
            Self::OperationBusy(operation) => write!(f, "{operation} is already in progress"),
            Self::ArithmeticOverflow => write!(f, "SMP address arithmetic overflow"),
        }
    }
}

impl std::error::Error for SmpError {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalCpuState {
    FirmwareDisabled,
    Offline,
    Starting,
    Online,
    Parked,
    Failed,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PerCpuStack {
    pub guard_base: u64,
    pub stack_base: u64,
    pub stack_top: u64,
    pub size: u64,
}

impl PerCpuStack {
    pub fn new(stack_base: u64, size: u64, guard_size: u64) -> Result<Self, SmpError> {
        if size == 0
            || stack_base & (SMP_STACK_ALIGNMENT - 1) != 0
            || size & (SMP_STACK_ALIGNMENT - 1) != 0
            || guard_size & (SMP_STACK_ALIGNMENT - 1) != 0
        {
            return Err(SmpError::InvalidStack {
                base: stack_base,
                size,
            });
        }
        let stack_top = stack_base
            .checked_add(size)
            .ok_or(SmpError::ArithmeticOverflow)?;
        let guard_base = stack_base
            .checked_sub(guard_size)
            .ok_or(SmpError::InvalidStack {
                base: stack_base,
                size,
            })?;
        Ok(Self {
            guard_base,
            stack_base,
            stack_top,
            size,
        })
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.guard_base < other.stack_top && other.guard_base < self.stack_top
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalCpu {
    pub logical_id: u32,
    pub hardware_id: u64,
    pub firmware_id: u64,
    pub numa_node: u32,
    pub package_id: u16,
    pub core_id: u16,
    pub thread_id: u16,
    pub bootstrap: bool,
    pub state: PhysicalCpuState,
    pub stack: Option<PerCpuStack>,
    pub local_base: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalSmpTopology {
    pub architecture: FirmwareArchitecture,
    pub cpus: Vec<PhysicalCpu>,
    pub interrupt_controller: Option<String>,
    pub interrupt_controller_base: Option<u64>,
    pub timer_frequency_hz: Option<u64>,
}

impl PhysicalSmpTopology {
    pub fn from_firmware(
        architecture: FirmwareArchitecture,
        firmware: &FirmwareTopology,
    ) -> Result<Self, SmpError> {
        if firmware.cpus.is_empty() {
            return Err(SmpError::EmptyTopology);
        }
        if firmware.cpus.len() > SMP_MAX_CPUS {
            return Err(SmpError::TooManyCpus(firmware.cpus.len()));
        }
        let mut cpus = Vec::with_capacity(firmware.cpus.len());
        for (logical_id, cpu) in firmware.cpus.iter().enumerate() {
            let hierarchy = firmware.hierarchy.get(&cpu.hardware_id).copied();
            cpus.push(PhysicalCpu {
                logical_id: logical_id as u32,
                hardware_id: cpu.hardware_id,
                firmware_id: cpu.firmware_id,
                numa_node: cpu.numa_node,
                package_id: hierarchy.map_or(0, |value| value.package_id),
                core_id: hierarchy.map_or(logical_id as u16, |value| value.core_id),
                thread_id: hierarchy.map_or(0, |value| value.thread_id),
                bootstrap: cpu.bootstrap,
                state: if !cpu.enabled {
                    PhysicalCpuState::FirmwareDisabled
                } else if cpu.bootstrap {
                    PhysicalCpuState::Online
                } else {
                    PhysicalCpuState::Offline
                },
                stack: None,
                local_base: 0,
            });
        }
        let result = Self {
            architecture,
            cpus,
            interrupt_controller: firmware.interrupt_controller.clone(),
            interrupt_controller_base: firmware.interrupt_controller_base,
            timer_frequency_hz: firmware.timer_frequency_hz,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn from_entries(
        architecture: FirmwareArchitecture,
        entries: &[KernelCpuTopologyEntryV1],
    ) -> Result<Self, SmpError> {
        let cpus = entries
            .iter()
            .map(|entry| PhysicalCpu {
                logical_id: entry.logical_id,
                hardware_id: entry.hardware_id,
                firmware_id: entry.firmware_id,
                numa_node: entry.numa_node,
                package_id: entry.package_id,
                core_id: entry.core_id,
                thread_id: entry.thread_id,
                bootstrap: entry.flags & KernelCpuTopologyEntryV1::FLAG_BOOTSTRAP != 0,
                state: if entry.flags & KernelCpuTopologyEntryV1::FLAG_ENABLED == 0 {
                    PhysicalCpuState::FirmwareDisabled
                } else if entry.flags & KernelCpuTopologyEntryV1::FLAG_BOOTSTRAP != 0 {
                    PhysicalCpuState::Online
                } else {
                    PhysicalCpuState::Offline
                },
                stack: None,
                local_base: 0,
            })
            .collect();
        let result = Self {
            architecture,
            cpus,
            interrupt_controller: None,
            interrupt_controller_base: None,
            timer_frequency_hz: None,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), SmpError> {
        if self.cpus.is_empty() {
            return Err(SmpError::EmptyTopology);
        }
        if self.cpus.len() > SMP_MAX_CPUS {
            return Err(SmpError::TooManyCpus(self.cpus.len()));
        }
        let bootstrap_count = self.cpus.iter().filter(|cpu| cpu.bootstrap).count();
        if bootstrap_count == 0 {
            return Err(SmpError::MissingBootstrap);
        }
        if bootstrap_count != 1 {
            return Err(SmpError::MultipleBootstrap);
        }
        for (index, cpu) in self.cpus.iter().enumerate() {
            if self.cpus[..index]
                .iter()
                .any(|other| other.logical_id == cpu.logical_id)
            {
                return Err(SmpError::DuplicateLogicalId(cpu.logical_id));
            }
            if self.cpus[..index]
                .iter()
                .any(|other| other.hardware_id == cpu.hardware_id)
            {
                return Err(SmpError::DuplicateHardwareId(cpu.hardware_id));
            }
        }
        for first in 0..self.cpus.len() {
            let Some(first_stack) = self.cpus[first].stack else {
                continue;
            };
            for second in first + 1..self.cpus.len() {
                if let Some(second_stack) = self.cpus[second].stack {
                    if first_stack.overlaps(second_stack) {
                        return Err(SmpError::StackOverlap {
                            first: self.cpus[first].logical_id,
                            second: self.cpus[second].logical_id,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn assign_stacks(
        &mut self,
        region_base: u64,
        stack_size: u64,
        guard_size: u64,
    ) -> Result<(), SmpError> {
        let stride = stack_size
            .checked_add(guard_size)
            .ok_or(SmpError::ArithmeticOverflow)?;
        for (index, cpu) in self.cpus.iter_mut().enumerate() {
            let guard_base = region_base
                .checked_add(
                    (index as u64)
                        .checked_mul(stride)
                        .ok_or(SmpError::ArithmeticOverflow)?,
                )
                .ok_or(SmpError::ArithmeticOverflow)?;
            let stack_base = guard_base
                .checked_add(guard_size)
                .ok_or(SmpError::ArithmeticOverflow)?;
            cpu.stack = Some(PerCpuStack::new(stack_base, stack_size, guard_size)?);
        }
        self.validate()
    }

    pub fn cpu(&self, logical_id: u32) -> Result<&PhysicalCpu, SmpError> {
        self.cpus
            .iter()
            .find(|cpu| cpu.logical_id == logical_id)
            .ok_or(SmpError::UnknownCpu(logical_id))
    }

    pub fn cpu_mut(&mut self, logical_id: u32) -> Result<&mut PhysicalCpu, SmpError> {
        self.cpus
            .iter_mut()
            .find(|cpu| cpu.logical_id == logical_id)
            .ok_or(SmpError::UnknownCpu(logical_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86StartupOperation {
    InitAssert { command: ApicInterruptCommand },
    DelayMicroseconds(u32),
    InitDeassert { command: ApicInterruptCommand },
    Startup { command: ApicInterruptCommand },
}

pub fn x86_init_sipi_sequence(
    apic_id: u32,
    trampoline: u64,
) -> Result<[X86StartupOperation; 8], SmpError> {
    if trampoline >= X86_AP_TRAMPOLINE_LIMIT || trampoline & 0xfff != 0 {
        return Err(SmpError::InvalidTrampoline(trampoline));
    }
    let init = ApicInterruptCommand {
        vector: 0,
        delivery_mode: ApicDeliveryMode::Init,
        destination_mode: ApicDestinationMode::Physical,
        level: ApicLevel::Assert,
        trigger_mode: ApicTriggerMode::Level,
        shorthand: ApicDestinationShorthand::None,
        destination: apic_id,
    };
    let deassert = ApicInterruptCommand {
        level: ApicLevel::Deassert,
        ..init
    };
    let sipi = ApicInterruptCommand {
        vector: (trampoline >> 12) as u8,
        delivery_mode: ApicDeliveryMode::Startup,
        level: ApicLevel::Assert,
        trigger_mode: ApicTriggerMode::Edge,
        ..init
    };
    Ok([
        X86StartupOperation::InitAssert { command: init },
        X86StartupOperation::DelayMicroseconds(X86_INIT_ASSERT_DELAY_US),
        X86StartupOperation::InitDeassert { command: deassert },
        X86StartupOperation::DelayMicroseconds(X86_SIPI_DELAY_US),
        X86StartupOperation::Startup { command: sipi },
        X86StartupOperation::DelayMicroseconds(X86_SIPI_DELAY_US),
        X86StartupOperation::Startup { command: sipi },
        X86StartupOperation::DelayMicroseconds(X86_SIPI_DELAY_US),
    ])
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsciConduit {
    Smc,
    Hvc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsciCall {
    pub conduit: PsciConduit,
    pub function: u32,
    pub args: [u64; 3],
}

impl PsciCall {
    pub const fn cpu_on(conduit: PsciConduit, target_mpidr: u64, entry: u64, context: u64) -> Self {
        Self {
            conduit,
            function: PSCI_FN_CPU_ON_64,
            args: [target_mpidr, entry, context],
        }
    }

    pub const fn affinity_info(conduit: PsciConduit, target_mpidr: u64, level: u64) -> Self {
        Self {
            conduit,
            function: PSCI_FN_AFFINITY_INFO_64,
            args: [target_mpidr, level, 0],
        }
    }

    pub const fn cpu_suspend(
        conduit: PsciConduit,
        power_state: u64,
        entry: u64,
        context: u64,
    ) -> Self {
        Self {
            conduit,
            function: PSCI_FN_CPU_SUSPEND_64,
            args: [power_state, entry, context],
        }
    }

    pub fn check_status(status: i64) -> Result<(), SmpError> {
        if status == PSCI_SUCCESS || status == PSCI_ALREADY_ON {
            Ok(())
        } else {
            Err(SmpError::FirmwareProtocol {
                protocol: "PSCI",
                status,
            })
        }
    }

    pub const fn affinity_is_online(status: i64) -> bool {
        status == PSCI_AFFINITY_ON
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbiCall {
    pub extension: u64,
    pub function: u64,
    pub args: [u64; 6],
}

impl SbiCall {
    pub const fn hart_start(hart_id: u64, entry: u64, opaque: u64) -> Self {
        Self {
            extension: SBI_EXT_HSM,
            function: SBI_HSM_HART_START,
            args: [hart_id, entry, opaque, 0, 0, 0],
        }
    }

    pub const fn hart_status(hart_id: u64) -> Self {
        Self {
            extension: SBI_EXT_HSM,
            function: SBI_HSM_HART_STATUS,
            args: [hart_id, 0, 0, 0, 0, 0],
        }
    }

    pub const fn send_ipi(hart_mask: u64, hart_mask_base: u64) -> Self {
        Self {
            extension: SBI_EXT_IPI,
            function: 0,
            args: [hart_mask, hart_mask_base, 0, 0, 0, 0],
        }
    }

    pub const fn remote_sfence_vma(
        hart_mask: u64,
        hart_mask_base: u64,
        start: u64,
        size: u64,
    ) -> Self {
        Self {
            extension: SBI_EXT_RFENCE,
            function: SBI_RFENCE_REMOTE_SFENCE_VMA,
            args: [hart_mask, hart_mask_base, start, size, 0, 0],
        }
    }

    pub const fn remote_sfence_vma_asid(
        hart_mask: u64,
        hart_mask_base: u64,
        start: u64,
        size: u64,
        asid: u64,
    ) -> Self {
        Self {
            extension: SBI_EXT_RFENCE,
            function: SBI_RFENCE_REMOTE_SFENCE_VMA_ASID,
            args: [hart_mask, hart_mask_base, start, size, asid, 0],
        }
    }

    pub fn check_status(status: i64) -> Result<(), SmpError> {
        if status == SBI_SUCCESS || status == SBI_ERR_ALREADY_STARTED {
            Ok(())
        } else {
            Err(SmpError::FirmwareProtocol {
                protocol: "SBI",
                status,
            })
        }
    }

    pub const fn hart_is_online(status: i64) -> bool {
        matches!(
            status,
            SBI_HSM_STARTED | SBI_HSM_START_PENDING | SBI_HSM_RESUME_PENDING
        )
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct X86DescriptorTablePointer64 {
    pub limit: u16,
    pub base: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct X86Tss64 {
    pub reserved0: u32,
    pub rsp: [u64; 3],
    pub reserved1: u64,
    pub ist: [u64; 7],
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

impl X86Tss64 {
    pub const BYTE_SIZE: usize = 104;

    pub fn new(rsp0: u64, ist: [u64; 7]) -> Self {
        Self {
            rsp: [rsp0, 0, 0],
            ist,
            iomap_base: Self::BYTE_SIZE as u16,
            ..Self::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct X86TssDescriptor64 {
    pub low: u64,
    pub high: u64,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct X86GdtEntry64(pub u64);

impl X86GdtEntry64 {
    pub const fn null() -> Self {
        Self(0)
    }

    pub fn code(dpl: u8) -> Result<Self, SmpError> {
        if dpl > 3 {
            return Err(SmpError::InvalidInterrupt(dpl as u32));
        }
        Ok(Self(
            0x0000_0000_0000_ffff | (0x9a_u64 << 40) | ((dpl as u64) << 45) | (0xa_u64 << 52),
        ))
    }

    pub fn data(dpl: u8) -> Result<Self, SmpError> {
        if dpl > 3 {
            return Err(SmpError::InvalidInterrupt(dpl as u32));
        }
        Ok(Self(
            0x0000_0000_0000_ffff | (0x92_u64 << 40) | ((dpl as u64) << 45) | (0xc_u64 << 52),
        ))
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct X86IdtGate64 {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,
    pub attributes: u8,
    pub offset_mid: u16,
    pub offset_high: u32,
    pub reserved: u32,
}

impl X86IdtGate64 {
    pub fn interrupt(handler: u64, selector: u16, ist: u8, dpl: u8) -> Result<Self, SmpError> {
        Self::new(handler, selector, ist, dpl, false)
    }

    pub fn trap(handler: u64, selector: u16, ist: u8, dpl: u8) -> Result<Self, SmpError> {
        Self::new(handler, selector, ist, dpl, true)
    }

    fn new(handler: u64, selector: u16, ist: u8, dpl: u8, trap: bool) -> Result<Self, SmpError> {
        if handler == 0 || selector == 0 || ist > 7 || dpl > 3 {
            return Err(SmpError::InvalidInterrupt(dpl as u32));
        }
        Ok(Self {
            offset_low: handler as u16,
            selector,
            ist,
            attributes: 0x80 | (dpl << 5) | if trap { 0x0f } else { 0x0e },
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        })
    }

    pub const fn handler(self) -> u64 {
        self.offset_low as u64
            | ((self.offset_mid as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }
}

impl X86TssDescriptor64 {
    pub fn new(base: u64, limit: u32, dpl: u8) -> Result<Self, SmpError> {
        if dpl > 3 || limit > 0x000f_ffff {
            return Err(SmpError::InvalidInterrupt(dpl as u32));
        }
        let low = (limit as u64 & 0xffff)
            | ((base & 0x00ff_ffff) << 16)
            | (0x9_u64 << 40)
            | (1_u64 << 47)
            | ((dpl as u64) << 45)
            | (((limit as u64 >> 16) & 0xf) << 48)
            | (((base >> 24) & 0xff) << 56);
        Ok(Self {
            low,
            high: base >> 32,
        })
    }
}

const _: [(); X86Tss64::BYTE_SIZE] = [(); core::mem::size_of::<X86Tss64>()];
const _: [(); 10] = [(); core::mem::size_of::<X86DescriptorTablePointer64>()];
const _: [(); 8] = [(); core::mem::size_of::<X86GdtEntry64>()];
const _: [(); 16] = [(); core::mem::size_of::<X86IdtGate64>()];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GicV3Sgi1r(pub u64);

impl GicV3Sgi1r {
    pub fn new(
        affinity3: u8,
        affinity2: u8,
        affinity1: u8,
        target_list: u16,
        interrupt_id: u8,
        routing_mode: bool,
    ) -> Result<Self, SmpError> {
        if interrupt_id > 15 || (!routing_mode && target_list == 0) {
            return Err(SmpError::InvalidInterrupt(interrupt_id as u32));
        }
        Ok(Self(
            ((affinity3 as u64) << 48)
                | ((routing_mode as u64) << 40)
                | ((affinity2 as u64) << 32)
                | ((interrupt_id as u64) << 24)
                | ((affinity1 as u64) << 16)
                | target_list as u64,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GicV3Layout {
    pub distributor_base: u64,
    pub redistributor_base: u64,
    pub redistributor_stride: u64,
}

impl GicV3Layout {
    pub fn redistributor_for(self, cpu_index: u32) -> Result<u64, SmpError> {
        if self.distributor_base & 0xffff != 0
            || self.redistributor_base & 0xffff != 0
            || self.redistributor_stride == 0
        {
            return Err(SmpError::InvalidControllerBase(self.distributor_base));
        }
        self.redistributor_base
            .checked_add(
                u64::from(cpu_index)
                    .checked_mul(self.redistributor_stride)
                    .ok_or(SmpError::ArithmeticOverflow)?,
            )
            .ok_or(SmpError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlicLayout {
    pub base: u64,
    pub source_count: u32,
    pub context_count: u32,
}

impl PlicLayout {
    pub fn priority(self, source: u32) -> Result<u64, SmpError> {
        if source == 0 || source > self.source_count {
            return Err(SmpError::InvalidInterrupt(source));
        }
        self.base
            .checked_add(u64::from(source) * 4)
            .ok_or(SmpError::ArithmeticOverflow)
    }

    pub fn enable(self, context: u32, source: u32) -> Result<(u64, u32), SmpError> {
        if context >= self.context_count || source == 0 || source > self.source_count {
            return Err(SmpError::InvalidInterrupt(source));
        }
        let address = self
            .base
            .checked_add(0x2000)
            .and_then(|value| value.checked_add(u64::from(context) * 0x80))
            .and_then(|value| value.checked_add(u64::from(source / 32) * 4))
            .ok_or(SmpError::ArithmeticOverflow)?;
        Ok((address, 1 << (source % 32)))
    }

    pub fn threshold(self, context: u32) -> Result<u64, SmpError> {
        if context >= self.context_count {
            return Err(SmpError::InvalidInterrupt(context));
        }
        self.base
            .checked_add(0x20_0000 + u64::from(context) * 0x1000)
            .ok_or(SmpError::ArithmeticOverflow)
    }

    pub fn claim_complete(self, context: u32) -> Result<u64, SmpError> {
        self.threshold(context)?
            .checked_add(4)
            .ok_or(SmpError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImsicLayout {
    pub machine_base: u64,
    pub supervisor_base: u64,
    pub guest_base: u64,
    pub hart_stride: u64,
    pub guest_stride: u64,
}

impl ImsicLayout {
    pub fn interrupt_file(
        self,
        hart: u32,
        supervisor: bool,
        guest: Option<u32>,
    ) -> Result<u64, SmpError> {
        let base = if let Some(guest) = guest {
            self.guest_base
                .checked_add(
                    u64::from(guest)
                        .checked_mul(self.guest_stride)
                        .ok_or(SmpError::ArithmeticOverflow)?,
                )
                .ok_or(SmpError::ArithmeticOverflow)?
        } else if supervisor {
            self.supervisor_base
        } else {
            self.machine_base
        };
        base.checked_add(
            u64::from(hart)
                .checked_mul(self.hart_stride)
                .ok_or(SmpError::ArithmeticOverflow)?,
        )
        .ok_or(SmpError::ArithmeticOverflow)
    }

    pub fn msi(
        self,
        hart: u32,
        supervisor: bool,
        guest: Option<u32>,
        interrupt_id: u16,
    ) -> Result<(u64, u32), SmpError> {
        if interrupt_id == 0 || interrupt_id >= 2048 {
            return Err(SmpError::InvalidInterrupt(interrupt_id as u32));
        }
        Ok((
            self.interrupt_file(hart, supervisor, guest)?,
            interrupt_id as u32,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalTimer {
    X86TscDeadline { deadline: u64, vector: u8 },
    Aarch64Generic { compare_value: u64, control: u32 },
    RiscvSbi { absolute_time: u64 },
}

impl PhysicalTimer {
    pub fn after_ticks(
        architecture: FirmwareArchitecture,
        now: u64,
        ticks: u64,
        vector: u8,
    ) -> Result<Self, SmpError> {
        let deadline = now.checked_add(ticks).ok_or(SmpError::ArithmeticOverflow)?;
        match architecture {
            FirmwareArchitecture::X86 => {
                if vector < 0x10 {
                    return Err(SmpError::InvalidInterrupt(vector as u32));
                }
                Ok(Self::X86TscDeadline { deadline, vector })
            }
            FirmwareArchitecture::Aarch64 => Ok(Self::Aarch64Generic {
                compare_value: deadline,
                control: 1,
            }),
            FirmwareArchitecture::RiscV => Ok(Self::RiscvSbi {
                absolute_time: deadline,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlbShootdown {
    pub generation: u64,
    pub address: u64,
    pub length: u64,
    pub address_space: u64,
    targets: Vec<u64>,
    acknowledged: Vec<u64>,
}

impl TlbShootdown {
    pub fn new(
        generation: u64,
        address: u64,
        length: u64,
        address_space: u64,
        cpu_count: usize,
        initiator: u32,
    ) -> Result<Self, SmpError> {
        if cpu_count == 0 || cpu_count > SMP_MAX_CPUS || initiator as usize >= cpu_count {
            return Err(SmpError::UnknownCpu(initiator));
        }
        address
            .checked_add(length)
            .ok_or(SmpError::ArithmeticOverflow)?;
        let words = cpu_count.div_ceil(64);
        let mut targets = vec![u64::MAX; words];
        if cpu_count & 63 != 0 {
            targets[words - 1] = (1_u64 << (cpu_count & 63)) - 1;
        }
        targets[initiator as usize / 64] &= !(1_u64 << (initiator & 63));
        Ok(Self {
            generation,
            address,
            length,
            address_space,
            acknowledged: vec![0; words],
            targets,
        })
    }

    pub fn acknowledge(&mut self, cpu: u32, generation: u64) -> Result<bool, SmpError> {
        if generation != self.generation {
            return Err(SmpError::InvalidShootdownGeneration {
                expected: self.generation,
                observed: generation,
            });
        }
        let word = cpu as usize / 64;
        let bit = 1_u64 << (cpu & 63);
        if word >= self.targets.len() || self.targets[word] & bit == 0 {
            return Err(SmpError::UnknownCpu(cpu));
        }
        if self.acknowledged[word] & bit != 0 {
            return Err(SmpError::ShootdownAlreadyAcknowledged(cpu));
        }
        self.acknowledged[word] |= bit;
        Ok(self.complete())
    }

    pub fn pending(&self) -> usize {
        self.targets
            .iter()
            .zip(&self.acknowledged)
            .map(|(targets, acknowledged)| (targets & !acknowledged).count_ones() as usize)
            .sum()
    }

    pub fn complete(&self) -> bool {
        self.pending() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::FirmwareCpu;

    fn topology(architecture: FirmwareArchitecture) -> PhysicalSmpTopology {
        PhysicalSmpTopology::from_firmware(
            architecture,
            &FirmwareTopology {
                cpus: vec![
                    FirmwareCpu {
                        hardware_id: 0,
                        firmware_id: 0,
                        enabled: true,
                        bootstrap: true,
                        numa_node: 0,
                    },
                    FirmwareCpu {
                        hardware_id: 1,
                        firmware_id: 1,
                        enabled: true,
                        bootstrap: false,
                        numa_node: 0,
                    },
                    FirmwareCpu {
                        hardware_id: 2,
                        firmware_id: 2,
                        enabled: true,
                        bootstrap: false,
                        numa_node: 1,
                    },
                ],
                ..FirmwareTopology::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn physical_topology_assigns_non_overlapping_guarded_stacks() {
        let mut topology = topology(FirmwareArchitecture::X86);
        topology.assign_stacks(0x10_0000, 0x8000, 0x1000).unwrap();
        assert_eq!(topology.cpu(1).unwrap().stack.unwrap().stack_top, 0x112000);
        assert!(topology
            .cpus
            .windows(2)
            .all(|pair| !pair[0].stack.unwrap().overlaps(pair[1].stack.unwrap())));
    }

    #[test]
    fn x86_startup_is_init_deassert_and_two_sipi_commands() {
        let sequence = x86_init_sipi_sequence(7, 0x8000).unwrap();
        assert!(matches!(
            sequence[0],
            X86StartupOperation::InitAssert { .. }
        ));
        assert!(matches!(
            sequence[2],
            X86StartupOperation::InitDeassert { .. }
        ));
        for operation in [sequence[4], sequence[6]] {
            let X86StartupOperation::Startup { command } = operation else {
                panic!("missing SIPI")
            };
            assert_eq!(command.vector, 8);
            assert_eq!(command.destination, 7);
            assert_eq!(command.delivery_mode, ApicDeliveryMode::Startup);
            assert!(command.encode_x2apic().is_ok());
        }
        assert!(x86_init_sipi_sequence(1, 0x10_0000).is_err());
    }

    #[test]
    fn psci_and_sbi_hsm_calls_use_architectural_function_ids() {
        let psci = PsciCall::cpu_on(PsciConduit::Smc, 0x8000_0001, 0x4020_0000, 0x9000);
        assert_eq!(psci.function, PSCI_FN_CPU_ON_64);
        assert_eq!(psci.args, [0x8000_0001, 0x4020_0000, 0x9000]);
        let sbi = SbiCall::hart_start(3, 0x8020_1000, 0x8800_0000);
        assert_eq!(
            (sbi.extension, sbi.function),
            (SBI_EXT_HSM, SBI_HSM_HART_START)
        );
        assert_eq!(&sbi.args[..3], &[3, 0x8020_1000, 0x8800_0000]);
    }

    #[test]
    fn descriptor_interrupt_and_timer_layouts_are_material() {
        assert_eq!(core::mem::size_of::<X86Tss64>(), 104);
        assert_eq!(core::mem::size_of::<X86DescriptorTablePointer64>(), 10);
        assert_eq!(core::mem::size_of::<X86GdtEntry64>(), 8);
        assert_eq!(core::mem::size_of::<X86IdtGate64>(), 16);
        let tss = X86Tss64::new(0x9000, [0xa000, 0xb000, 0, 0, 0, 0, 0]);
        let descriptor = X86TssDescriptor64::new((&tss as *const X86Tss64) as u64, 103, 0).unwrap();
        assert_eq!((descriptor.low >> 40) & 0xf, 9);
        let code = X86GdtEntry64::code(0).unwrap();
        assert_eq!((code.0 >> 45) & 0x3, 0);
        assert_eq!((code.0 >> 53) & 0x1, 1);
        let gate = X86IdtGate64::interrupt(0xffff_8000_1234_5678, 8, 2, 0).unwrap();
        assert_eq!(gate.handler(), 0xffff_8000_1234_5678);
        assert_eq!(gate.attributes, 0x8e);
        let sgi = GicV3Sgi1r::new(1, 2, 3, 1 << 4, 9, false).unwrap();
        assert_eq!((sgi.0 >> 24) & 0xf, 9);
        let timer = PhysicalTimer::after_ticks(FirmwareArchitecture::Aarch64, 100, 25, 0).unwrap();
        assert_eq!(
            timer,
            PhysicalTimer::Aarch64Generic {
                compare_value: 125,
                control: 1
            }
        );
    }

    #[test]
    fn plic_imsic_and_tlb_shootdown_cover_all_target_cpus() {
        let plic = PlicLayout {
            base: 0x0c00_0000,
            source_count: 1023,
            context_count: 8,
        };
        assert_eq!(plic.priority(10).unwrap(), 0x0c00_0028);
        assert_eq!(plic.enable(1, 65).unwrap(), (0x0c00_2088, 2));
        assert_eq!(plic.claim_complete(2).unwrap(), 0x0c20_2004);
        let imsic = ImsicLayout {
            machine_base: 0x2400_0000,
            supervisor_base: 0x2800_0000,
            guest_base: 0x3000_0000,
            hart_stride: 0x1000,
            guest_stride: 0x1000_0000,
        };
        assert_eq!(imsic.msi(3, true, None, 17).unwrap(), (0x2800_3000, 17));
        let mut shootdown = TlbShootdown::new(9, 0x4000, 0x3000, 2, 3, 0).unwrap();
        assert_eq!(shootdown.pending(), 2);
        assert!(!shootdown.acknowledge(1, 9).unwrap());
        assert!(shootdown.acknowledge(2, 9).unwrap());
        assert!(shootdown.complete());
    }

    #[test]
    fn physical_platform_descriptor_binds_architecture_firmware_and_interrupt_model() {
        let mut x86 = KernelSmpPlatformV1::new(
            SmpArchitecture::X86,
            SmpStartMethod::X86InitSipi,
            SmpInterruptModel::X2Apic,
        );
        x86.primary_controller_base = 0xfee0_0000;
        x86.timer_frequency_hz = 3_000_000_000;
        x86.feature_flags = KernelSmpPlatformV1::FEATURE_IPI
            | KernelSmpPlatformV1::FEATURE_TLB_RANGE
            | KernelSmpPlatformV1::FEATURE_PER_CPU_TIMER;
        assert_eq!(x86.validate().unwrap().size, 104);

        let mut arm = KernelSmpPlatformV1::new(
            SmpArchitecture::Aarch64,
            SmpStartMethod::PsciSmc,
            SmpInterruptModel::GicV3,
        );
        arm.primary_controller_base = 0x0800_0000;
        arm.secondary_controller_base = 0x080a_0000;
        arm.redistributor_stride = 0x20_000;
        arm.timer_frequency_hz = 62_500_000;
        assert!(arm.validate().is_ok());

        arm.start_method = SmpStartMethod::SbiHsm as u32;
        assert_eq!(arm.validate(), Err(SmpError::InvalidPlatformDescriptor));
    }

    #[test]
    fn secondary_mailbox_and_firmware_statuses_are_explicit() {
        let mailbox = SecondaryCpuMailboxV1::prepared(7, 3, 0x20_0000, 0x40_0000, 99)
            .expect("valid AP mailbox");
        assert_eq!(mailbox.state, SecondaryCpuMailboxState::Prepared as u32);
        assert_eq!(core::mem::size_of::<SecondaryCpuMailboxV1>(), 80);
        assert_eq!(mailbox.park_callback, 0);

        let trampoline = x86_64_ap_trampoline_image().expect("x86 AP trampoline");
        assert_eq!(trampoline.load_address, 0x7000);
        assert_eq!(trampoline.mailbox_address, 0x6000);
        assert_eq!(trampoline.bytes.len(), 314);
        assert_eq!(&trampoline.bytes[..4], &[0xfa, 0xfc, 0x31, 0xc0]);
        assert_eq!(&trampoline.bytes[26..33], &[0xea, 0x21, 0x70, 0, 0, 8, 0]);
        assert!(PsciCall::check_status(PSCI_ALREADY_ON).is_ok());
        assert!(PsciCall::check_status(PSCI_DENIED).is_err());
        assert!(SbiCall::check_status(SBI_ERR_ALREADY_STARTED).is_ok());
        assert!(SbiCall::hart_is_online(SBI_HSM_START_PENDING));
        assert!(!SbiCall::hart_is_online(SBI_HSM_STOPPED));
    }
}
