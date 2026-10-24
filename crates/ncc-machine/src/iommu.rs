use core::fmt;

pub const IOMMU_BASE_PAGE_SIZE: u64 = 4096;
pub const VFIO_API_VERSION: i32 = 0;
pub const VFIO_TYPE1_IOMMU: i32 = 1;
pub const VFIO_TYPE1V2_IOMMU: i32 = 3;
pub const VFIO_TYPE1_NESTING_IOMMU: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IommuArchitecture {
    IntelVtd,
    AmdVi,
    ArmSmmuv3,
    RiscvIommu,
    SoftwareViommu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IommuDomainId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct IommuDeviceId {
    pub segment: u16,
    pub requester_id: u16,
}

impl IommuDeviceId {
    pub const fn pci(segment: u16, bus: u8, device: u8, function: u8) -> Option<Self> {
        if device >= 32 || function >= 8 {
            return None;
        }
        Some(Self {
            segment,
            requester_id: (bus as u16) << 8 | (device as u16) << 3 | function as u16,
        })
    }

    pub const fn bus(self) -> u8 {
        (self.requester_id >> 8) as u8
    }

    pub const fn device(self) -> u8 {
        ((self.requester_id >> 3) & 0x1f) as u8
    }

    pub const fn function(self) -> u8 {
        (self.requester_id & 0x7) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct DmaPermissions(u8);

impl DmaPermissions {
    pub const NONE: Self = Self(0);
    pub const DEVICE_READ: Self = Self(1 << 0);
    pub const DEVICE_WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const READ_WRITE: Self = Self(Self::DEVICE_READ.0 | Self::DEVICE_WRITE.0);

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0x7)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaMapping {
    pub iova: u64,
    pub host_virtual_address: u64,
    pub host_physical_address: Option<u64>,
    pub length: u64,
    pub page_size: u64,
    pub permissions: DmaPermissions,
    pub pinned: bool,
    pub coherent: bool,
}

impl DmaMapping {
    pub fn validate(self) -> Result<Self, IommuError> {
        if self.length == 0 || self.page_size < IOMMU_BASE_PAGE_SIZE {
            return Err(IommuError::InvalidLength(self.length));
        }
        if !self.page_size.is_power_of_two()
            || self.length % self.page_size != 0
            || self.iova % self.page_size != 0
            || self.host_virtual_address % self.page_size != 0
            || self
                .host_physical_address
                .is_some_and(|address| address % self.page_size != 0)
        {
            return Err(IommuError::MisalignedMapping {
                iova: self.iova,
                host: self.host_virtual_address,
                page_size: self.page_size,
            });
        }
        if self.permissions.is_empty() {
            return Err(IommuError::EmptyPermissions);
        }
        self.end()?;
        Ok(self)
    }

    pub fn end(self) -> Result<u64, IommuError> {
        self.iova
            .checked_add(self.length)
            .ok_or(IommuError::AddressOverflow)
    }

    pub fn contains(self, iova: u64) -> bool {
        self.end().is_ok_and(|end| iova >= self.iova && iova < end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaTranslation {
    pub host_virtual_address: u64,
    pub host_physical_address: Option<u64>,
    pub remaining: u64,
    pub permissions: DmaPermissions,
    pub coherent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaRecoveryAction {
    QuiesceDevice,
    MaskMsix,
    DisableBusMaster,
    FlushIotlb(IotlbInvalidation),
    FunctionLevelReset,
    SecondaryBusReset,
    ReenableBusMaster,
    UnmaskMsix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmaRecoveryPlan<const ACTIONS: usize> {
    pub actions: [Option<DmaRecoveryAction>; ACTIONS],
    pub retry_limit: u8,
    pub timeout_ms: u32,
}

impl<const ACTIONS: usize> DmaRecoveryPlan<ACTIONS> {
    pub const fn new(
        actions: [Option<DmaRecoveryAction>; ACTIONS],
        retry_limit: u8,
        timeout_ms: u32,
    ) -> Self {
        Self {
            actions,
            retry_limit,
            timeout_ms,
        }
    }

    pub fn action_count(&self) -> usize {
        self.actions.iter().flatten().count()
    }

    pub fn validate(&self) -> Result<(), IommuError> {
        if self.action_count() == 0 {
            return Err(IommuError::RecoveryPlanEmpty);
        }
        if self.retry_limit == 0 || self.timeout_ms == 0 {
            return Err(IommuError::RecoveryPlanInvalid);
        }
        let mut saw_quiesce = false;
        let mut saw_mask = false;
        let mut saw_disable_bus_master = false;
        let mut saw_flush = false;
        let mut saw_reset = false;
        let mut saw_reenable = false;
        let mut saw_unmask = false;
        for action in self.actions.iter().flatten() {
            match action {
                DmaRecoveryAction::QuiesceDevice => saw_quiesce = true,
                DmaRecoveryAction::MaskMsix => saw_mask = true,
                DmaRecoveryAction::DisableBusMaster => saw_disable_bus_master = true,
                DmaRecoveryAction::FlushIotlb(_) => saw_flush = true,
                DmaRecoveryAction::FunctionLevelReset | DmaRecoveryAction::SecondaryBusReset => {
                    saw_reset = true
                }
                DmaRecoveryAction::ReenableBusMaster => saw_reenable = true,
                DmaRecoveryAction::UnmaskMsix => saw_unmask = true,
            }
        }
        if saw_quiesce
            && saw_mask
            && saw_disable_bus_master
            && saw_flush
            && saw_reset
            && saw_reenable
            && saw_unmask
        {
            Ok(())
        } else {
            Err(IommuError::RecoveryPlanIncomplete)
        }
    }
}

pub struct IommuDomain<const MAPPINGS: usize, const DEVICES: usize> {
    pub id: IommuDomainId,
    mappings: [Option<DmaMapping>; MAPPINGS],
    devices: [Option<IommuDeviceId>; DEVICES],
    generation: u64,
}

impl<const MAPPINGS: usize, const DEVICES: usize> IommuDomain<MAPPINGS, DEVICES> {
    pub const fn new(id: IommuDomainId) -> Self {
        Self {
            id,
            mappings: [None; MAPPINGS],
            devices: [None; DEVICES],
            generation: 0,
        }
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.iter().flatten().count()
    }

    pub fn device_count(&self) -> usize {
        self.devices.iter().flatten().count()
    }

    pub fn attach_device(&mut self, device: IommuDeviceId) -> Result<(), IommuError> {
        if self.devices.iter().flatten().any(|entry| *entry == device) {
            return Err(IommuError::DeviceAlreadyAttached(device));
        }
        let slot = self
            .devices
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(IommuError::DeviceCapacityExhausted)?;
        *slot = Some(device);
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn detach_device(&mut self, device: IommuDeviceId) -> Result<(), IommuError> {
        let slot = self
            .devices
            .iter_mut()
            .find(|entry| entry.is_some_and(|entry| entry == device))
            .ok_or(IommuError::DeviceNotAttached(device))?;
        *slot = None;
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn map(&mut self, mapping: DmaMapping) -> Result<(), IommuError> {
        let mapping = mapping.validate()?;
        let end = mapping.end()?;
        for existing in self.mappings.iter().flatten() {
            let existing_end = existing.end()?;
            if mapping.iova < existing_end && end > existing.iova {
                return Err(IommuError::MappingOverlap {
                    start: mapping.iova,
                    end,
                });
            }
        }
        let slot = self
            .mappings
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(IommuError::MappingCapacityExhausted)?;
        *slot = Some(mapping);
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn unmap(&mut self, iova: u64, length: u64) -> Result<DmaMapping, IommuError> {
        let slot = self
            .mappings
            .iter_mut()
            .find(|entry| entry.is_some_and(|mapping| mapping.iova == iova))
            .ok_or(IommuError::MappingNotPresent(iova))?;
        let mapping = slot.expect("matched mapping");
        if mapping.length != length {
            return Err(IommuError::UnmapLengthMismatch {
                expected: mapping.length,
                requested: length,
            });
        }
        *slot = None;
        self.generation = self.generation.wrapping_add(1);
        Ok(mapping)
    }

    pub fn translate(
        &self,
        iova: u64,
        access: DmaPermissions,
    ) -> Result<DmaTranslation, IommuError> {
        let mapping = self
            .mappings
            .iter()
            .flatten()
            .copied()
            .find(|mapping| mapping.contains(iova))
            .ok_or(IommuError::MappingNotPresent(iova))?;
        if !mapping.permissions.contains(access) {
            return Err(IommuError::PermissionDenied {
                iova,
                requested: access,
                available: mapping.permissions,
            });
        }
        if !mapping.pinned {
            return Err(IommuError::MemoryNotPinned(iova));
        }
        let offset = iova - mapping.iova;
        Ok(DmaTranslation {
            host_virtual_address: mapping.host_virtual_address + offset,
            host_physical_address: mapping
                .host_physical_address
                .map(|address| address + offset),
            remaining: mapping.length - offset,
            permissions: mapping.permissions,
            coherent: mapping.coherent,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IotlbInvalidation {
    Global,
    Domain(IommuDomainId),
    Device(IommuDeviceId),
    Address {
        domain: IommuDomainId,
        iova: u64,
        length: u64,
        page_selective: bool,
    },
    Process {
        domain: IommuDomainId,
        pasid: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuFaultReason {
    Unmapped,
    ReadDenied,
    WriteDenied,
    ExecuteDenied,
    InvalidContext,
    InvalidPageTable,
    AddressWidth,
    AtsTranslation,
    PriRequest,
    QueueOverflow,
    Hardware(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct IommuFault {
    pub device: IommuDeviceId,
    pub domain: IommuDomainId,
    pub pasid: Option<u32>,
    pub iova: u64,
    pub reason: IommuFaultReason,
    pub write: bool,
}

pub struct IommuQueue<T: Copy, const CAPACITY: usize> {
    entries: [Option<T>; CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
}

impl<T: Copy, const CAPACITY: usize> IommuQueue<T, CAPACITY> {
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, value: T) -> Result<(), IommuError> {
        if CAPACITY == 0 || self.len == CAPACITY {
            return Err(IommuError::QueueFull);
        }
        self.entries[self.tail] = Some(value);
        self.tail = (self.tail + 1) % CAPACITY;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 || CAPACITY == 0 {
            return None;
        }
        let value = self.entries[self.head].take();
        self.head = (self.head + 1) % CAPACITY;
        self.len -= 1;
        value
    }
}

impl<T: Copy, const CAPACITY: usize> Default for IommuQueue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViommuCommand {
    Attach(IommuDeviceId),
    Detach(IommuDeviceId),
    Map(DmaMapping),
    Unmap { iova: u64, length: u64 },
    Invalidate(IotlbInvalidation),
    Fence { write_barrier: bool },
}

pub struct VirtualIommu<
    const MAPPINGS: usize,
    const DEVICES: usize,
    const COMMANDS: usize,
    const FAULTS: usize,
> {
    pub architecture: IommuArchitecture,
    pub domain: IommuDomain<MAPPINGS, DEVICES>,
    pub commands: IommuQueue<ViommuCommand, COMMANDS>,
    pub faults: IommuQueue<IommuFault, FAULTS>,
    pub completed_commands: u64,
    pub invalidation_generation: u64,
}

impl<const MAPPINGS: usize, const DEVICES: usize, const COMMANDS: usize, const FAULTS: usize>
    VirtualIommu<MAPPINGS, DEVICES, COMMANDS, FAULTS>
{
    pub const fn new(architecture: IommuArchitecture, domain: IommuDomainId) -> Self {
        Self {
            architecture,
            domain: IommuDomain::new(domain),
            commands: IommuQueue::new(),
            faults: IommuQueue::new(),
            completed_commands: 0,
            invalidation_generation: 0,
        }
    }

    pub fn submit(&mut self, command: ViommuCommand) -> Result<(), IommuError> {
        self.commands.push(command)
    }

    pub fn process_one(&mut self) -> Result<bool, IommuError> {
        let Some(command) = self.commands.pop() else {
            return Ok(false);
        };
        match command {
            ViommuCommand::Attach(device) => self.domain.attach_device(device)?,
            ViommuCommand::Detach(device) => self.domain.detach_device(device)?,
            ViommuCommand::Map(mapping) => self.domain.map(mapping)?,
            ViommuCommand::Unmap { iova, length } => {
                self.domain.unmap(iova, length)?;
            }
            ViommuCommand::Invalidate(_) => {
                self.invalidation_generation = self.invalidation_generation.wrapping_add(1);
            }
            ViommuCommand::Fence { .. } => {}
        }
        self.completed_commands = self.completed_commands.wrapping_add(1);
        Ok(true)
    }

    pub fn process_all(&mut self) -> Result<u64, IommuError> {
        let before = self.completed_commands;
        while self.process_one()? {}
        Ok(self.completed_commands.wrapping_sub(before))
    }

    pub fn dma(
        &mut self,
        device: IommuDeviceId,
        iova: u64,
        access: DmaPermissions,
    ) -> Result<DmaTranslation, IommuError> {
        if !self
            .domain
            .devices
            .iter()
            .flatten()
            .any(|entry| *entry == device)
        {
            return Err(IommuError::DeviceNotAttached(device));
        }
        match self.domain.translate(iova, access) {
            Ok(translation) => Ok(translation),
            Err(error) => {
                let reason = match error {
                    IommuError::MappingNotPresent(_) => IommuFaultReason::Unmapped,
                    IommuError::PermissionDenied { .. }
                        if access.contains(DmaPermissions::DEVICE_WRITE) =>
                    {
                        IommuFaultReason::WriteDenied
                    }
                    IommuError::PermissionDenied { .. } => IommuFaultReason::ReadDenied,
                    _ => IommuFaultReason::Hardware(0xffff_ffff),
                };
                self.faults.push(IommuFault {
                    device,
                    domain: self.domain.id,
                    pasid: None,
                    iova,
                    reason,
                    write: access.contains(DmaPermissions::DEVICE_WRITE),
                })?;
                Err(error)
            }
        }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntelVtdRootEntry {
    pub low: u64,
    pub high: u64,
}

impl IntelVtdRootEntry {
    pub fn legacy(context_table_physical: u64) -> Result<Self, IommuError> {
        validate_table_address(context_table_physical, 4096)?;
        Ok(Self {
            low: context_table_physical | 1,
            high: 0,
        })
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntelVtdContextEntry {
    pub low: u64,
    pub high: u64,
}

impl IntelVtdContextEntry {
    pub fn second_level(
        root_physical: u64,
        domain: IommuDomainId,
        address_width: u8,
    ) -> Result<Self, IommuError> {
        validate_table_address(root_physical, 4096)?;
        let aw = match address_width {
            30 => 0,
            39 => 1,
            48 => 2,
            57 => 3,
            bits => return Err(IommuError::InvalidAddressWidth(bits)),
        };
        if domain.0 >= 1 << 16 {
            return Err(IommuError::InvalidDomain(domain));
        }
        Ok(Self {
            low: root_physical | 1,
            high: ((domain.0 as u64) << 8) | aw,
        })
    }
}

#[repr(C, align(32))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AmdViDeviceTableEntry {
    pub words: [u64; 4],
}

impl AmdViDeviceTableEntry {
    pub fn translated(
        root_physical: u64,
        domain: IommuDomainId,
        levels: u8,
    ) -> Result<Self, IommuError> {
        validate_table_address(root_physical, 4096)?;
        if !(1..=6).contains(&levels) {
            return Err(IommuError::InvalidPageTableLevels(levels));
        }
        if domain.0 >= 1 << 16 {
            return Err(IommuError::InvalidDomain(domain));
        }
        Ok(Self {
            words: [
                root_physical | 1 | ((levels as u64) << 9),
                (domain.0 as u64) | (1 << 61) | (1 << 62),
                0,
                0,
            ],
        })
    }
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmSmmuv3StreamTableEntry {
    pub words: [u64; 8],
}

impl ArmSmmuv3StreamTableEntry {
    pub fn stage2(vmid: u16, vttbr: u64, vtcr: u64) -> Result<Self, IommuError> {
        validate_table_address(vttbr & 0x0000_ffff_ffff_f000, 4096)?;
        let mut words = [0u64; 8];
        words[0] = 1 | (6 << 1);
        words[2] = vttbr;
        words[3] = vtcr | ((vmid as u64) << 48);
        Ok(Self { words })
    }
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmSmmuv3ContextDescriptor {
    pub words: [u64; 8],
}

impl ArmSmmuv3ContextDescriptor {
    pub fn stage1(asid: u16, ttbr0: u64, tcr: u64, mair: u64) -> Result<Self, IommuError> {
        validate_table_address(ttbr0 & 0x0000_ffff_ffff_f000, 4096)?;
        let mut words = [0u64; 8];
        words[0] = 1 | ((asid as u64) << 48) | (tcr << 8);
        words[1] = ttbr0;
        words[2] = mair;
        Ok(Self { words })
    }
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RiscvIommuDeviceContext {
    pub words: [u64; 8],
}

impl RiscvIommuDeviceContext {
    pub fn translated(iohgatp: u64, iosatp: u64, tc: u64, fault_queue_enabled: bool) -> Self {
        let mut words = [0u64; 8];
        words[0] = tc | 1 | (u64::from(fault_queue_enabled) << 1);
        words[1] = iohgatp;
        words[2] = iosatp;
        Self { words }
    }
}

fn validate_table_address(address: u64, alignment: u64) -> Result<(), IommuError> {
    if address == 0 || address % alignment != 0 {
        return Err(IommuError::InvalidTableAddress { address, alignment });
    }
    Ok(())
}

pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1 << 0;
pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 1 << 1;
pub const VFIO_DEVICE_FLAGS_RESET: u32 = 1 << 0;
pub const VFIO_DEVICE_FLAGS_PCI: u32 = 1 << 1;
pub const VFIO_IRQ_INFO_EVENTFD: u32 = 1 << 0;
pub const VFIO_IRQ_INFO_MASKABLE: u32 = 1 << 1;
pub const VFIO_IRQ_INFO_AUTOMASKED: u32 = 1 << 2;
pub const VFIO_IRQ_INFO_NORESIZE: u32 = 1 << 3;
pub const VFIO_IRQ_SET_DATA_NONE: u32 = 1 << 0;
pub const VFIO_IRQ_SET_DATA_BOOL: u32 = 1 << 1;
pub const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 1 << 2;
pub const VFIO_IRQ_SET_ACTION_MASK: u32 = 1 << 3;
pub const VFIO_IRQ_SET_ACTION_UNMASK: u32 = 1 << 4;
pub const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 1 << 5;
pub const VFIO_PCI_INTX_IRQ_INDEX: u32 = 0;
pub const VFIO_PCI_MSI_IRQ_INDEX: u32 = 1;
pub const VFIO_PCI_MSIX_IRQ_INDEX: u32 = 2;
pub const VFIO_PCI_ERR_IRQ_INDEX: u32 = 3;
pub const VFIO_PCI_REQ_IRQ_INDEX: u32 = 4;
pub const VFIO_REGION_INFO_FLAG_READ: u32 = 1 << 0;
pub const VFIO_REGION_INFO_FLAG_WRITE: u32 = 1 << 1;
pub const VFIO_REGION_INFO_FLAG_MMAP: u32 = 1 << 2;
pub const VFIO_REGION_INFO_FLAG_CAPS: u32 = 1 << 3;
pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1 << 0;
pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 1 << 1;
pub const VFIO_DMA_MAP_FLAG_VADDR: u32 = 1 << 2;
pub const VFIO_DMA_UNMAP_FLAG_GET_DIRTY_BITMAP: u32 = 1 << 0;
pub const VFIO_DMA_UNMAP_FLAG_ALL: u32 = 1 << 1;
pub const VFIO_DMA_UNMAP_FLAG_VADDR: u32 = 1 << 2;
pub const IOMMU_IOAS_MAP_FIXED_IOVA: u32 = 1 << 0;
pub const IOMMU_IOAS_MAP_WRITEABLE: u32 = 1 << 1;
pub const IOMMU_IOAS_MAP_READABLE: u32 = 1 << 2;

const fn linux_io(type_: u8, number: u8) -> u64 {
    ((type_ as u64) << 8) | number as u64
}

const VFIO_TYPE: u8 = b';';
const VFIO_BASE: u8 = 100;
const IOMMUFD_BASE: u8 = 0x80;

pub const VFIO_GET_API_VERSION: u64 = linux_io(VFIO_TYPE, VFIO_BASE);
pub const VFIO_CHECK_EXTENSION: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 1);
pub const VFIO_SET_IOMMU: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 2);
pub const VFIO_GROUP_GET_STATUS: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 3);
pub const VFIO_GROUP_SET_CONTAINER: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 4);
pub const VFIO_GROUP_UNSET_CONTAINER: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 5);
pub const VFIO_GROUP_GET_DEVICE_FD: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 6);
pub const VFIO_DEVICE_GET_INFO: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 7);
pub const VFIO_DEVICE_GET_REGION_INFO: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 8);
pub const VFIO_DEVICE_GET_IRQ_INFO: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 9);
pub const VFIO_DEVICE_SET_IRQS: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 10);
pub const VFIO_DEVICE_RESET: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 11);
pub const VFIO_IOMMU_GET_INFO: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 12);
pub const VFIO_IOMMU_MAP_DMA: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 13);
pub const VFIO_IOMMU_UNMAP_DMA: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 14);
pub const VFIO_DEVICE_FEATURE: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 17);
pub const VFIO_DEVICE_BIND_IOMMUFD: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 18);
pub const VFIO_DEVICE_ATTACH_IOMMUFD_PT: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 19);
pub const VFIO_DEVICE_DETACH_IOMMUFD_PT: u64 = linux_io(VFIO_TYPE, VFIO_BASE + 20);
pub const IOMMU_DESTROY: u64 = linux_io(VFIO_TYPE, IOMMUFD_BASE);
pub const IOMMU_IOAS_ALLOC: u64 = linux_io(VFIO_TYPE, IOMMUFD_BASE + 1);
pub const IOMMU_IOAS_COPY: u64 = linux_io(VFIO_TYPE, IOMMUFD_BASE + 3);
pub const IOMMU_IOAS_MAP: u64 = linux_io(VFIO_TYPE, IOMMUFD_BASE + 5);
pub const IOMMU_IOAS_UNMAP: u64 = linux_io(VFIO_TYPE, IOMMUFD_BASE + 6);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioGroupStatus {
    pub argsz: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioDeviceInfo {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
    pub cap_offset: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioRegionInfo {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIrqInfo {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIrqSet {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub start: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIrqSetEventFd {
    pub set: VfioIrqSet,
    pub event_fd: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIommuType1Info {
    pub argsz: u32,
    pub flags: u32,
    pub iova_pgsizes: u64,
    pub cap_offset: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIommuType1DmaMap {
    pub argsz: u32,
    pub flags: u32,
    pub vaddr: u64,
    pub iova: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioIommuType1DmaUnmap {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioDeviceBindIommufd {
    pub argsz: u32,
    pub flags: u32,
    pub iommufd: i32,
    pub out_devid: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioDeviceAttachIommufdPt {
    pub argsz: u32,
    pub flags: u32,
    pub pt_id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VfioDeviceDetachIommufdPt {
    pub argsz: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IommufdDestroy {
    pub size: u32,
    pub id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IommufdIoasAlloc {
    pub size: u32,
    pub flags: u32,
    pub out_ioas_id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IommufdIoasMap {
    pub size: u32,
    pub flags: u32,
    pub ioas_id: u32,
    pub reserved: u32,
    pub user_va: u64,
    pub length: u64,
    pub iova: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IommufdIoasUnmap {
    pub size: u32,
    pub ioas_id: u32,
    pub iova: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IommuError {
    InvalidLength(u64),
    MisalignedMapping {
        iova: u64,
        host: u64,
        page_size: u64,
    },
    EmptyPermissions,
    AddressOverflow,
    MappingOverlap {
        start: u64,
        end: u64,
    },
    MappingCapacityExhausted,
    MappingNotPresent(u64),
    UnmapLengthMismatch {
        expected: u64,
        requested: u64,
    },
    PermissionDenied {
        iova: u64,
        requested: DmaPermissions,
        available: DmaPermissions,
    },
    MemoryNotPinned(u64),
    DeviceAlreadyAttached(IommuDeviceId),
    DeviceNotAttached(IommuDeviceId),
    DeviceCapacityExhausted,
    QueueFull,
    RecoveryPlanEmpty,
    RecoveryPlanInvalid,
    RecoveryPlanIncomplete,
    InvalidTableAddress {
        address: u64,
        alignment: u64,
    },
    InvalidAddressWidth(u8),
    InvalidPageTableLevels(u8),
    InvalidDomain(IommuDomainId),
}

impl fmt::Display for IommuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(length) => write!(f, "invalid DMA mapping length {length:#x}"),
            Self::MisalignedMapping {
                iova,
                host,
                page_size,
            } => write!(
                f,
                "DMA mapping iova={iova:#x} host={host:#x} is not aligned to {page_size:#x}"
            ),
            Self::EmptyPermissions => write!(f, "DMA mapping has no permissions"),
            Self::AddressOverflow => write!(f, "DMA address range overflows u64"),
            Self::MappingOverlap { start, end } => {
                write!(f, "DMA mapping {start:#x}..{end:#x} overlaps")
            }
            Self::MappingCapacityExhausted => write!(f, "DMA mapping capacity exhausted"),
            Self::MappingNotPresent(iova) => write!(f, "IOVA {iova:#x} is not mapped"),
            Self::UnmapLengthMismatch {
                expected,
                requested,
            } => write!(
                f,
                "DMA unmap length {requested:#x} differs from mapping {expected:#x}"
            ),
            Self::PermissionDenied {
                iova,
                requested,
                available,
            } => write!(
                f,
                "DMA access at {iova:#x} requests {requested:?}, mapping has {available:?}"
            ),
            Self::MemoryNotPinned(iova) => write!(f, "DMA memory at {iova:#x} is not pinned"),
            Self::DeviceAlreadyAttached(device) => {
                write!(f, "IOMMU device {device:?} is already attached")
            }
            Self::DeviceNotAttached(device) => {
                write!(f, "IOMMU device {device:?} is not attached")
            }
            Self::DeviceCapacityExhausted => write!(f, "IOMMU device capacity exhausted"),
            Self::QueueFull => write!(f, "IOMMU command or fault queue is full"),
            Self::RecoveryPlanEmpty => write!(f, "DMA recovery plan is empty"),
            Self::RecoveryPlanInvalid => {
                write!(f, "DMA recovery plan has invalid timeout or retry limit")
            }
            Self::RecoveryPlanIncomplete => write!(
                f,
                "DMA recovery plan must quiesce, mask MSI-X, disable bus master, flush IOTLB, reset, re-enable bus master and unmask MSI-X"
            ),
            Self::InvalidTableAddress { address, alignment } => write!(
                f,
                "IOMMU table address {address:#x} is not aligned to {alignment:#x}"
            ),
            Self::InvalidAddressWidth(bits) => write!(f, "invalid IOMMU address width {bits}"),
            Self::InvalidPageTableLevels(levels) => {
                write!(f, "invalid IOMMU page-table level count {levels}")
            }
            Self::InvalidDomain(domain) => write!(f, "invalid IOMMU domain {domain:?}"),
        }
    }
}

impl std::error::Error for IommuError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(iova: u64, host: u64, permissions: DmaPermissions) -> DmaMapping {
        DmaMapping {
            iova,
            host_virtual_address: host,
            host_physical_address: Some(host + 0x1000_0000),
            length: 0x2000,
            page_size: 0x1000,
            permissions,
            pinned: true,
            coherent: true,
        }
    }

    #[test]
    fn domain_maps_translates_and_rejects_dma_violations() {
        let device = IommuDeviceId::pci(0, 2, 3, 1).unwrap();
        let mut domain = IommuDomain::<4, 2>::new(IommuDomainId(7));
        domain.attach_device(device).unwrap();
        domain
            .map(mapping(0x4000, 0x8000, DmaPermissions::DEVICE_READ))
            .unwrap();
        let translation = domain
            .translate(0x4800, DmaPermissions::DEVICE_READ)
            .unwrap();
        assert_eq!(translation.host_virtual_address, 0x8800);
        assert!(matches!(
            domain.translate(0x4800, DmaPermissions::DEVICE_WRITE),
            Err(IommuError::PermissionDenied { .. })
        ));
        assert!(matches!(
            domain.map(mapping(0x5000, 0xc000, DmaPermissions::READ_WRITE)),
            Err(IommuError::MappingOverlap { .. })
        ));
        domain.unmap(0x4000, 0x2000).unwrap();
        domain.detach_device(device).unwrap();
    }

    #[test]
    fn viommu_processes_command_and_fault_rings_materially() {
        let device = IommuDeviceId::pci(0, 0, 5, 0).unwrap();
        let mut iommu =
            VirtualIommu::<4, 2, 8, 4>::new(IommuArchitecture::SoftwareViommu, IommuDomainId(3));
        iommu.submit(ViommuCommand::Attach(device)).unwrap();
        iommu
            .submit(ViommuCommand::Map(mapping(
                0x1000,
                0x9000,
                DmaPermissions::READ_WRITE,
            )))
            .unwrap();
        iommu
            .submit(ViommuCommand::Invalidate(IotlbInvalidation::Domain(
                IommuDomainId(3),
            )))
            .unwrap();
        assert_eq!(iommu.process_all().unwrap(), 3);
        assert_eq!(iommu.invalidation_generation, 1);
        assert_eq!(
            iommu
                .dma(device, 0x1800, DmaPermissions::DEVICE_WRITE)
                .unwrap()
                .host_virtual_address,
            0x9800
        );
        assert!(iommu
            .dma(device, 0x8000, DmaPermissions::DEVICE_READ)
            .is_err());
        assert_eq!(iommu.faults.len(), 1);
    }

    #[test]
    fn separate_domains_reject_cross_device_dma_without_leaking_translation() {
        let device_a = IommuDeviceId::pci(0, 1, 0, 0).unwrap();
        let device_b = IommuDeviceId::pci(0, 2, 0, 0).unwrap();
        let mut domain_a =
            VirtualIommu::<2, 1, 4, 4>::new(IommuArchitecture::IntelVtd, IommuDomainId(11));
        let mut domain_b =
            VirtualIommu::<2, 1, 4, 4>::new(IommuArchitecture::AmdVi, IommuDomainId(12));

        domain_a.submit(ViommuCommand::Attach(device_a)).unwrap();
        domain_b.submit(ViommuCommand::Attach(device_b)).unwrap();
        domain_a
            .submit(ViommuCommand::Map(mapping(
                0x4000,
                0x10000,
                DmaPermissions::READ_WRITE,
            )))
            .unwrap();
        domain_b
            .submit(ViommuCommand::Map(mapping(
                0x4000,
                0x20000,
                DmaPermissions::READ_WRITE,
            )))
            .unwrap();
        domain_a.process_all().unwrap();
        domain_b.process_all().unwrap();

        assert!(matches!(
            domain_a.dma(device_b, 0x4000, DmaPermissions::DEVICE_READ),
            Err(IommuError::DeviceNotAttached(device)) if device == device_b
        ));
        assert_eq!(domain_a.faults.len(), 0);
        assert_eq!(
            domain_b
                .dma(device_b, 0x4000, DmaPermissions::DEVICE_READ)
                .unwrap()
                .host_virtual_address,
            0x20000
        );
    }

    #[test]
    fn unpinned_dma_fault_requires_explicit_recovery_commands() {
        let device = IommuDeviceId::pci(0, 3, 0, 0).unwrap();
        let mut iommu =
            VirtualIommu::<2, 1, 8, 4>::new(IommuArchitecture::ArmSmmuv3, IommuDomainId(21));
        let mut unpinned = mapping(0x8000, 0x30000, DmaPermissions::READ_WRITE);
        unpinned.pinned = false;
        unpinned.coherent = false;
        iommu.submit(ViommuCommand::Attach(device)).unwrap();
        iommu.submit(ViommuCommand::Map(unpinned)).unwrap();
        iommu.process_all().unwrap();
        let generation_before_fault = iommu.domain.generation();

        assert!(matches!(
            iommu.dma(device, 0x8000, DmaPermissions::DEVICE_WRITE),
            Err(IommuError::MemoryNotPinned(0x8000))
        ));
        assert_eq!(iommu.domain.generation(), generation_before_fault);
        assert_eq!(iommu.domain.mapping_count(), 1);
        assert_eq!(iommu.faults.len(), 1);
        let fault = iommu
            .faults
            .pop()
            .expect("fault must remain caller-visible");
        assert_eq!(fault.device, device);
        assert_eq!(fault.domain, IommuDomainId(21));
        assert_eq!(fault.iova, 0x8000);

        iommu
            .submit(ViommuCommand::Unmap {
                iova: 0x8000,
                length: 0x2000,
            })
            .unwrap();
        iommu
            .submit(ViommuCommand::Invalidate(IotlbInvalidation::Domain(
                IommuDomainId(21),
            )))
            .unwrap();
        iommu.process_all().unwrap();
        assert_eq!(iommu.domain.mapping_count(), 0);
        assert_eq!(iommu.invalidation_generation, 1);
        assert!(iommu.domain.generation() > generation_before_fault);
    }

    #[test]
    fn vendor_table_layouts_have_architectural_width_and_alignment() {
        assert_eq!(core::mem::size_of::<IntelVtdRootEntry>(), 16);
        assert_eq!(core::mem::align_of::<IntelVtdRootEntry>(), 16);
        assert_eq!(core::mem::size_of::<IntelVtdContextEntry>(), 16);
        assert_eq!(core::mem::size_of::<AmdViDeviceTableEntry>(), 32);
        assert_eq!(core::mem::align_of::<AmdViDeviceTableEntry>(), 32);
        assert_eq!(core::mem::size_of::<ArmSmmuv3StreamTableEntry>(), 64);
        assert_eq!(core::mem::align_of::<ArmSmmuv3StreamTableEntry>(), 64);
        assert_eq!(core::mem::size_of::<RiscvIommuDeviceContext>(), 64);
        assert_eq!(core::mem::align_of::<RiscvIommuDeviceContext>(), 64);
    }

    #[test]
    fn dma_recovery_plan_requires_full_driver_fault_cycle() {
        let domain = IommuDomainId(9);
        let device = IommuDeviceId::pci(0, 1, 0, 0).unwrap();
        let full = DmaRecoveryPlan::new(
            [
                Some(DmaRecoveryAction::QuiesceDevice),
                Some(DmaRecoveryAction::MaskMsix),
                Some(DmaRecoveryAction::DisableBusMaster),
                Some(DmaRecoveryAction::FlushIotlb(IotlbInvalidation::Address {
                    domain,
                    iova: 0x1000,
                    length: 0x1000,
                    page_selective: true,
                })),
                Some(DmaRecoveryAction::FunctionLevelReset),
                Some(DmaRecoveryAction::ReenableBusMaster),
                Some(DmaRecoveryAction::UnmaskMsix),
                Some(DmaRecoveryAction::FlushIotlb(IotlbInvalidation::Device(
                    device,
                ))),
            ],
            3,
            10_000,
        );
        full.validate().unwrap();

        let incomplete = DmaRecoveryPlan::new(
            [
                Some(DmaRecoveryAction::QuiesceDevice),
                Some(DmaRecoveryAction::FunctionLevelReset),
                None,
            ],
            1,
            1_000,
        );
        assert!(matches!(
            incomplete.validate(),
            Err(IommuError::RecoveryPlanIncomplete)
        ));
    }

    #[test]
    fn vfio_and_iommufd_abi_layout_and_ioctl_numbers_match_linux_uapi() {
        assert_eq!(VFIO_GET_API_VERSION, 0x3b64);
        assert_eq!(VFIO_DEVICE_RESET, 0x3b6f);
        assert_eq!(VFIO_IOMMU_MAP_DMA, 0x3b71);
        assert_eq!(VFIO_DEVICE_BIND_IOMMUFD, 0x3b76);
        assert_eq!(IOMMU_IOAS_ALLOC, 0x3b81);
        assert_eq!(IOMMU_IOAS_MAP, 0x3b85);
        assert_eq!(VFIO_REGION_INFO_FLAG_READ, 1);
        assert_eq!(VFIO_REGION_INFO_FLAG_WRITE, 2);
        assert_eq!(VFIO_REGION_INFO_FLAG_MMAP, 4);
        assert_eq!(VFIO_REGION_INFO_FLAG_CAPS, 8);
        assert_eq!(VFIO_IRQ_INFO_EVENTFD, 1);
        assert_eq!(VFIO_IRQ_SET_DATA_EVENTFD, 4);
        assert_eq!(VFIO_IRQ_SET_ACTION_TRIGGER, 32);
        assert_eq!(VFIO_PCI_MSIX_IRQ_INDEX, 2);
        assert_eq!(core::mem::size_of::<VfioGroupStatus>(), 8);
        assert_eq!(core::mem::size_of::<VfioDeviceInfo>(), 24);
        assert_eq!(core::mem::size_of::<VfioRegionInfo>(), 32);
        assert_eq!(core::mem::size_of::<VfioIommuType1Info>(), 24);
        assert_eq!(core::mem::size_of::<VfioIommuType1DmaMap>(), 32);
        assert_eq!(core::mem::size_of::<VfioIommuType1DmaUnmap>(), 24);
        assert_eq!(core::mem::size_of::<VfioIrqSet>(), 20);
        assert_eq!(core::mem::size_of::<VfioIrqSetEventFd>(), 24);
        assert_eq!(core::mem::size_of::<IommufdIoasMap>(), 40);
    }
}
