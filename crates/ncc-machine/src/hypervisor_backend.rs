use core::fmt;

use crate::{GuestPagePermissions, GuestPhysicalAddress};

pub const HYPERVISOR_ABI_VERSION: u32 = 1;
pub const HYPERVISOR_MINIMUM_PAGE_SIZE: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HostHypervisorKind {
    Whpx = 1,
    Kvm = 2,
    Hvf = 3,
}

impl HostHypervisorKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Whpx => "whpx",
            Self::Kvm => "kvm",
            Self::Hvf => "hvf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HypervisorGuestArchitecture {
    X86_64 = 1,
    Aarch64 = 2,
    Riscv64 = 3,
}

impl HypervisorGuestArchitecture {
    pub const fn name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::Riscv64 => "riscv64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypervisorCapabilities {
    pub backend: HostHypervisorKind,
    pub architecture: HypervisorGuestArchitecture,
    pub api_version: u32,
    pub maximum_vcpus: u32,
    pub guest_physical_address_bits: u8,
    pub memory_slots: u32,
    pub user_memory: bool,
    pub read_only_memory: bool,
    pub executable_memory: bool,
    pub dirty_page_tracking: bool,
    pub interrupt_injection: bool,
    pub in_kernel_irqchip: bool,
    pub vcpu_cancel: bool,
    pub vcpu_state: bool,
    pub nested_paging: bool,
    pub device_assignment: bool,
    pub migration: bool,
}

impl HypervisorCapabilities {
    pub fn validate_for(self, config: HypervisorVmConfig) -> Result<Self, HypervisorBackendError> {
        config.validate()?;
        if self.api_version < HYPERVISOR_ABI_VERSION {
            return Err(HypervisorBackendError::ApiVersion {
                required: HYPERVISOR_ABI_VERSION,
                available: self.api_version,
            });
        }
        if self.architecture != config.architecture {
            return Err(HypervisorBackendError::ArchitectureMismatch {
                requested: config.architecture,
                available: self.architecture,
            });
        }
        if config.vcpu_count > self.maximum_vcpus {
            return Err(HypervisorBackendError::TooManyVcpus {
                requested: config.vcpu_count,
                maximum: self.maximum_vcpus,
            });
        }
        if config.guest_physical_address_bits > self.guest_physical_address_bits {
            return Err(HypervisorBackendError::PhysicalAddressWidth {
                requested: config.guest_physical_address_bits,
                maximum: self.guest_physical_address_bits,
            });
        }
        if config.track_dirty_pages && !self.dirty_page_tracking {
            return Err(HypervisorBackendError::MissingCapability(
                "dirty_page_tracking",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypervisorVmConfig {
    pub architecture: HypervisorGuestArchitecture,
    pub vcpu_count: u32,
    pub guest_physical_address_bits: u8,
    pub track_dirty_pages: bool,
    pub nested_virtualization: bool,
}

impl HypervisorVmConfig {
    pub const X86_64_SINGLE_VCPU: Self = Self {
        architecture: HypervisorGuestArchitecture::X86_64,
        vcpu_count: 1,
        guest_physical_address_bits: 48,
        track_dirty_pages: true,
        nested_virtualization: false,
    };

    pub fn validate(self) -> Result<Self, HypervisorBackendError> {
        if self.vcpu_count == 0 {
            return Err(HypervisorBackendError::ZeroVcpus);
        }
        if !(32..=63).contains(&self.guest_physical_address_bits) {
            return Err(HypervisorBackendError::PhysicalAddressWidth {
                requested: self.guest_physical_address_bits,
                maximum: 63,
            });
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HypervisorVmState {
    Created,
    Configuring,
    Ready,
    Running,
    Paused,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HypervisorVcpuState {
    Created,
    Runnable,
    Running,
    Halted,
    Canceled,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypervisorMemorySlot {
    pub slot: u32,
    pub guest_start: GuestPhysicalAddress,
    pub host_virtual_address: u64,
    pub length: u64,
    pub permissions: GuestPagePermissions,
    pub track_dirty_pages: bool,
}

impl HypervisorMemorySlot {
    pub fn validate(self) -> Result<Self, HypervisorBackendError> {
        if self.length == 0 || self.length % HYPERVISOR_MINIMUM_PAGE_SIZE != 0 {
            return Err(HypervisorBackendError::InvalidMemoryLength(self.length));
        }
        if !self.guest_start.is_aligned(HYPERVISOR_MINIMUM_PAGE_SIZE)
            || self.host_virtual_address % HYPERVISOR_MINIMUM_PAGE_SIZE != 0
        {
            return Err(HypervisorBackendError::MisalignedMemory {
                guest: self.guest_start.raw(),
                host: self.host_virtual_address,
            });
        }
        if self.permissions.is_empty() {
            return Err(HypervisorBackendError::EmptyMemoryPermissions);
        }
        self.end()?;
        Ok(self)
    }

    pub fn end(self) -> Result<u64, HypervisorBackendError> {
        self.guest_start
            .raw()
            .checked_add(self.length)
            .ok_or(HypervisorBackendError::AddressOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PortableVmExitKind {
    MemoryAccess,
    IoPort,
    Mmio,
    Cpuid,
    Msr,
    Hypercall,
    Exception,
    InterruptWindow,
    Halt,
    Shutdown,
    Canceled,
    Debug,
    SystemRegister,
    WfiWfe,
    RiscvSbi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortableVmExit {
    pub kind: PortableVmExitKind,
    pub vcpu_id: u32,
    pub program_counter: u64,
    pub instruction_length: u8,
    pub qualification: u64,
    pub guest_physical_address: Option<u64>,
    pub guest_virtual_address: Option<u64>,
    pub port: Option<u16>,
    pub vector: Option<u32>,
}

impl PortableVmExit {
    pub const fn halt(vcpu_id: u32, program_counter: u64) -> Self {
        Self {
            kind: PortableVmExitKind::Halt,
            vcpu_id,
            program_counter,
            instruction_length: 1,
            qualification: 0,
            guest_physical_address: None,
            guest_virtual_address: None,
            port: None,
            vector: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableInterrupt {
    X86Vector(u8),
    ArmGic { intid: u32, priority: u8 },
    RiscvImsic { identity: u32 },
}

pub trait HostHypervisorBackend {
    type Partition: HypervisorPartition;

    fn kind(&self) -> HostHypervisorKind;
    fn probe(&self) -> Result<HypervisorCapabilities, HypervisorBackendError>;
    fn create_partition(
        &self,
        config: HypervisorVmConfig,
    ) -> Result<Self::Partition, HypervisorBackendError>;
}

pub trait HypervisorPartition {
    type Vcpu: HypervisorVirtualCpu;

    fn state(&self) -> HypervisorVmState;
    fn map_memory(&mut self, slot: HypervisorMemorySlot) -> Result<(), HypervisorBackendError>;
    fn unmap_memory(
        &mut self,
        guest_start: GuestPhysicalAddress,
        length: u64,
    ) -> Result<(), HypervisorBackendError>;
    fn create_vcpu(&mut self, id: u32) -> Result<Self::Vcpu, HypervisorBackendError>;
    fn inject_interrupt(
        &mut self,
        interrupt: PortableInterrupt,
    ) -> Result<(), HypervisorBackendError>;
    fn pause(&mut self) -> Result<(), HypervisorBackendError>;
    fn resume(&mut self) -> Result<(), HypervisorBackendError>;
    fn reset(&mut self) -> Result<(), HypervisorBackendError>;
}

pub trait HypervisorVirtualCpu {
    fn id(&self) -> u32;
    fn state(&self) -> HypervisorVcpuState;
    fn run(&mut self) -> Result<PortableVmExit, HypervisorBackendError>;
    fn cancel(&mut self) -> Result<(), HypervisorBackendError>;
    fn get_register(&self, register: u32) -> Result<u128, HypervisorBackendError>;
    fn set_register(&mut self, register: u32, value: u128) -> Result<(), HypervisorBackendError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypervisorBackendError {
    ApiUnavailable(&'static str),
    ApiVersion {
        required: u32,
        available: u32,
    },
    ArchitectureMismatch {
        requested: HypervisorGuestArchitecture,
        available: HypervisorGuestArchitecture,
    },
    ZeroVcpus,
    TooManyVcpus {
        requested: u32,
        maximum: u32,
    },
    PhysicalAddressWidth {
        requested: u8,
        maximum: u8,
    },
    MissingCapability(&'static str),
    InvalidState {
        operation: &'static str,
        state: HypervisorVmState,
    },
    InvalidVcpu(u32),
    DuplicateVcpu(u32),
    InvalidMemoryLength(u64),
    MisalignedMemory {
        guest: u64,
        host: u64,
    },
    EmptyMemoryPermissions,
    MemoryOverlap {
        start: u64,
        end: u64,
    },
    MemorySlotMissing(u64),
    AddressOverflow,
    ProviderStatus {
        operation: &'static str,
        status: i64,
    },
    Timeout {
        milliseconds: u64,
    },
    UnsupportedExit(u64),
}

impl fmt::Display for HypervisorBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiUnavailable(api) => write!(f, "hypervisor API '{api}' is unavailable"),
            Self::ApiVersion {
                required,
                available,
            } => write!(
                f,
                "hypervisor ABI version {available} is older than required version {required}"
            ),
            Self::ArchitectureMismatch {
                requested,
                available,
            } => write!(
                f,
                "guest architecture {requested:?} is unavailable on {available:?} backend"
            ),
            Self::ZeroVcpus => write!(f, "a virtual machine needs at least one vCPU"),
            Self::TooManyVcpus { requested, maximum } => {
                write!(f, "requested {requested} vCPUs, backend limit is {maximum}")
            }
            Self::PhysicalAddressWidth { requested, maximum } => write!(
                f,
                "requested {requested}-bit guest physical addresses, maximum is {maximum}"
            ),
            Self::MissingCapability(capability) => {
                write!(f, "hypervisor capability '{capability}' is required")
            }
            Self::InvalidState { operation, state } => {
                write!(
                    f,
                    "operation '{operation}' is invalid while VM is {state:?}"
                )
            }
            Self::InvalidVcpu(id) => write!(f, "vCPU {id} does not exist"),
            Self::DuplicateVcpu(id) => write!(f, "vCPU {id} already exists"),
            Self::InvalidMemoryLength(length) => {
                write!(f, "guest memory length {length:#x} is not page-sized")
            }
            Self::MisalignedMemory { guest, host } => write!(
                f,
                "guest {guest:#x} or host {host:#x} memory address is not page-aligned"
            ),
            Self::EmptyMemoryPermissions => write!(f, "guest memory needs permissions"),
            Self::MemoryOverlap { start, end } => {
                write!(f, "guest memory range {start:#x}..{end:#x} overlaps")
            }
            Self::MemorySlotMissing(address) => {
                write!(f, "guest memory slot at {address:#x} does not exist")
            }
            Self::AddressOverflow => write!(f, "guest address range overflows u64"),
            Self::ProviderStatus { operation, status } => {
                write!(
                    f,
                    "provider operation '{operation}' failed with {status:#x}"
                )
            }
            Self::Timeout { milliseconds } => {
                write!(f, "vCPU execution exceeded {milliseconds} ms")
            }
            Self::UnsupportedExit(reason) => write!(f, "unsupported VM exit {reason:#x}"),
        }
    }
}

impl std::error::Error for HypervisorBackendError {}

pub struct PortableVmTopology<const MEMORY_SLOTS: usize, const VCPUS: usize> {
    state: HypervisorVmState,
    memory: [Option<HypervisorMemorySlot>; MEMORY_SLOTS],
    vcpus: [Option<HypervisorVcpuState>; VCPUS],
}

impl<const MEMORY_SLOTS: usize, const VCPUS: usize> PortableVmTopology<MEMORY_SLOTS, VCPUS> {
    pub const fn new() -> Self {
        Self {
            state: HypervisorVmState::Created,
            memory: [None; MEMORY_SLOTS],
            vcpus: [None; VCPUS],
        }
    }

    pub const fn state(&self) -> HypervisorVmState {
        self.state
    }

    pub fn begin_configuration(&mut self) -> Result<(), HypervisorBackendError> {
        if self.state != HypervisorVmState::Created {
            return Err(HypervisorBackendError::InvalidState {
                operation: "begin_configuration",
                state: self.state,
            });
        }
        self.state = HypervisorVmState::Configuring;
        Ok(())
    }

    pub fn map_memory(&mut self, slot: HypervisorMemorySlot) -> Result<(), HypervisorBackendError> {
        if self.state != HypervisorVmState::Configuring {
            return Err(HypervisorBackendError::InvalidState {
                operation: "map_memory",
                state: self.state,
            });
        }
        let slot = slot.validate()?;
        let end = slot.end()?;
        for existing in self.memory.iter().flatten() {
            let existing_end = existing.end()?;
            if slot.guest_start.raw() < existing_end && end > existing.guest_start.raw() {
                return Err(HypervisorBackendError::MemoryOverlap {
                    start: slot.guest_start.raw(),
                    end,
                });
            }
        }
        let destination = self
            .memory
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(HypervisorBackendError::MissingCapability("memory_slots"))?;
        *destination = Some(slot);
        Ok(())
    }

    pub fn create_vcpu(&mut self, id: u32) -> Result<(), HypervisorBackendError> {
        if self.state != HypervisorVmState::Configuring {
            return Err(HypervisorBackendError::InvalidState {
                operation: "create_vcpu",
                state: self.state,
            });
        }
        let index = usize::try_from(id).map_err(|_| HypervisorBackendError::InvalidVcpu(id))?;
        let slot = self
            .vcpus
            .get_mut(index)
            .ok_or(HypervisorBackendError::InvalidVcpu(id))?;
        if slot.is_some() {
            return Err(HypervisorBackendError::DuplicateVcpu(id));
        }
        *slot = Some(HypervisorVcpuState::Created);
        Ok(())
    }

    pub fn finish_configuration(&mut self) -> Result<(), HypervisorBackendError> {
        if self.state != HypervisorVmState::Configuring {
            return Err(HypervisorBackendError::InvalidState {
                operation: "finish_configuration",
                state: self.state,
            });
        }
        if self.vcpus.iter().all(Option::is_none) {
            return Err(HypervisorBackendError::ZeroVcpus);
        }
        for state in self.vcpus.iter_mut().flatten() {
            *state = HypervisorVcpuState::Runnable;
        }
        self.state = HypervisorVmState::Ready;
        Ok(())
    }

    pub fn transition(&mut self, next: HypervisorVmState) -> Result<(), HypervisorBackendError> {
        let allowed = matches!(
            (self.state, next),
            (HypervisorVmState::Ready, HypervisorVmState::Running)
                | (HypervisorVmState::Running, HypervisorVmState::Paused)
                | (HypervisorVmState::Paused, HypervisorVmState::Running)
                | (HypervisorVmState::Ready, HypervisorVmState::Stopped)
                | (HypervisorVmState::Running, HypervisorVmState::Stopped)
                | (HypervisorVmState::Paused, HypervisorVmState::Stopped)
                | (_, HypervisorVmState::Failed)
        );
        if !allowed {
            return Err(HypervisorBackendError::InvalidState {
                operation: "transition",
                state: self.state,
            });
        }
        self.state = next;
        Ok(())
    }
}

impl<const MEMORY_SLOTS: usize, const VCPUS: usize> Default
    for PortableVmTopology<MEMORY_SLOTS, VCPUS>
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_contract_rejects_architecture_and_limit_mismatches() {
        let capabilities = HypervisorCapabilities {
            backend: HostHypervisorKind::Kvm,
            architecture: HypervisorGuestArchitecture::X86_64,
            api_version: HYPERVISOR_ABI_VERSION,
            maximum_vcpus: 2,
            guest_physical_address_bits: 48,
            memory_slots: 32,
            user_memory: true,
            read_only_memory: true,
            executable_memory: true,
            dirty_page_tracking: true,
            interrupt_injection: true,
            in_kernel_irqchip: true,
            vcpu_cancel: true,
            vcpu_state: true,
            nested_paging: true,
            device_assignment: true,
            migration: true,
        };
        capabilities
            .validate_for(HypervisorVmConfig::X86_64_SINGLE_VCPU)
            .unwrap();
        let mut invalid = HypervisorVmConfig::X86_64_SINGLE_VCPU;
        invalid.architecture = HypervisorGuestArchitecture::Aarch64;
        assert!(matches!(
            capabilities.validate_for(invalid),
            Err(HypervisorBackendError::ArchitectureMismatch { .. })
        ));
    }

    #[test]
    fn topology_enforces_order_overlap_and_vcpu_identity() {
        let mut topology = PortableVmTopology::<4, 2>::new();
        topology.begin_configuration().unwrap();
        topology
            .map_memory(HypervisorMemorySlot {
                slot: 0,
                guest_start: GuestPhysicalAddress::new(0),
                host_virtual_address: 0x1000,
                length: 0x2000,
                permissions: GuestPagePermissions::READ_WRITE_EXECUTE,
                track_dirty_pages: true,
            })
            .unwrap();
        assert!(matches!(
            topology.map_memory(HypervisorMemorySlot {
                slot: 1,
                guest_start: GuestPhysicalAddress::new(0x1000),
                host_virtual_address: 0x4000,
                length: 0x1000,
                permissions: GuestPagePermissions::READ_WRITE,
                track_dirty_pages: false,
            }),
            Err(HypervisorBackendError::MemoryOverlap { .. })
        ));
        topology.create_vcpu(0).unwrap();
        assert_eq!(
            topology.create_vcpu(0),
            Err(HypervisorBackendError::DuplicateVcpu(0))
        );
        topology.finish_configuration().unwrap();
        topology.transition(HypervisorVmState::Running).unwrap();
        topology.transition(HypervisorVmState::Paused).unwrap();
        topology.transition(HypervisorVmState::Running).unwrap();
        topology.transition(HypervisorVmState::Stopped).unwrap();
    }
}
