use crate::{
    PixelFormat, Ps2KeyEvent, Uart16550, VirtualBlockDevice, VirtualDeviceError,
    VirtualFramebuffer, VirtualNetworkDevice,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{fence, Ordering};

pub const PCI_CONFIG_SPACE_BYTES: usize = 4096;
pub const PCI_VENDOR_VIRTIO: u16 = 0x1af4;
pub const VIRTIO_PCI_DEVICE_BASE: u16 = 0x1040;
pub const VIRTIO_F_VERSION_1: u64 = 1 << 32;
pub const VIRTIO_F_RING_INDIRECT_DESC: u64 = 1 << 28;
pub const VIRTIO_F_RING_EVENT_IDX: u64 = 1 << 29;
pub const VIRTIO_F_ACCESS_PLATFORM: u64 = 1 << 33;
pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;
pub const VIRTQ_DESC_F_INDIRECT: u16 = 4;
pub const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;
pub const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 1;
pub const VIRTIO_STATUS_DRIVER: u8 = 2;
pub const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
pub const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
pub const VIRTIO_STATUS_DEVICE_NEEDS_RESET: u8 = 64;
pub const VIRTIO_STATUS_FAILED: u8 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PciVirtioError {
    InvalidPciAddress,
    InvalidConfigAccess,
    DuplicatePciAddress,
    DeviceNotFound,
    InvalidBar,
    InvalidCapability,
    InvalidStatusTransition,
    FeatureNegotiationFailed,
    QueueNotReady,
    InvalidQueue,
    InvalidDescriptor,
    DescriptorCycle,
    GuestMemory { address: u64, length: usize },
    UnsupportedRequest(u32),
    Backend(String),
}

impl fmt::Display for PciVirtioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPciAddress => formatter.write_str("invalid PCI address"),
            Self::InvalidConfigAccess => formatter.write_str("invalid PCI configuration access"),
            Self::DuplicatePciAddress => formatter.write_str("duplicate PCI address"),
            Self::DeviceNotFound => formatter.write_str("PCI or VirtIO device not found"),
            Self::InvalidBar => formatter.write_str("invalid PCI BAR"),
            Self::InvalidCapability => formatter.write_str("invalid PCI capability chain"),
            Self::InvalidStatusTransition => {
                formatter.write_str("invalid VirtIO status transition")
            }
            Self::FeatureNegotiationFailed => {
                formatter.write_str("VirtIO driver selected unsupported features")
            }
            Self::QueueNotReady => formatter.write_str("VirtIO queue is not ready"),
            Self::InvalidQueue => formatter.write_str("invalid VirtIO queue"),
            Self::InvalidDescriptor => formatter.write_str("invalid VirtIO descriptor chain"),
            Self::DescriptorCycle => {
                formatter.write_str("VirtIO descriptor chain contains a cycle")
            }
            Self::GuestMemory { address, length } => write!(
                formatter,
                "guest memory access {address:#x}+{length:#x} is outside the mapped buffer"
            ),
            Self::UnsupportedRequest(request) => {
                write!(formatter, "unsupported VirtIO request {request}")
            }
            Self::Backend(message) => write!(formatter, "VirtIO backend error: {message}"),
        }
    }
}

impl Error for PciVirtioError {}

impl From<VirtualDeviceError> for PciVirtioError {
    fn from(value: VirtualDeviceError) -> Self {
        Self::Backend(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    pub fn new(segment: u16, bus: u8, device: u8, function: u8) -> Result<Self, PciVirtioError> {
        if device >= 32 || function >= 8 {
            return Err(PciVirtioError::InvalidPciAddress);
        }
        Ok(Self {
            segment,
            bus,
            device,
            function,
        })
    }

    pub const fn config_mechanism_one(self, register: u16) -> u32 {
        0x8000_0000
            | (self.bus as u32) << 16
            | (self.device as u32) << 11
            | (self.function as u32) << 8
            | (register as u32 & 0xfc)
    }

    pub fn decode_config_mechanism_one(value: u32) -> Option<(Self, u16)> {
        if value & 0x8000_0000 == 0 {
            return None;
        }
        Some((
            Self {
                segment: 0,
                bus: ((value >> 16) & 0xff) as u8,
                device: ((value >> 11) & 0x1f) as u8,
                function: ((value >> 8) & 0x07) as u8,
            },
            (value & 0xfc) as u16,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBarKind {
    Io,
    Memory32,
    Memory64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciBar {
    pub kind: PciBarKind,
    pub address: u64,
    pub size: u64,
    pub prefetchable: bool,
    probe: bool,
}

impl PciBar {
    pub fn new(
        kind: PciBarKind,
        address: u64,
        size: u64,
        prefetchable: bool,
    ) -> Result<Self, PciVirtioError> {
        if size < 4 || !size.is_power_of_two() || address & (size - 1) != 0 {
            return Err(PciVirtioError::InvalidBar);
        }
        if matches!(kind, PciBarKind::Io) && (address > u16::MAX as u64 || size > 0x1_0000) {
            return Err(PciVirtioError::InvalidBar);
        }
        Ok(Self {
            kind,
            address,
            size,
            prefetchable,
            probe: false,
        })
    }

    fn encoded_low(self) -> u32 {
        if self.probe {
            return match self.kind {
                PciBarKind::Io => (!(self.size as u32 - 1) & 0xffff_fffc) | 1,
                PciBarKind::Memory32 => {
                    (!(self.size as u32 - 1) & 0xffff_fff0) | if self.prefetchable { 8 } else { 0 }
                }
                PciBarKind::Memory64 => {
                    (!(self.size as u32 - 1) & 0xffff_fff0)
                        | 4
                        | if self.prefetchable { 8 } else { 0 }
                }
            };
        }
        match self.kind {
            PciBarKind::Io => (self.address as u32 & 0xffff_fffc) | 1,
            PciBarKind::Memory32 => {
                (self.address as u32 & 0xffff_fff0) | if self.prefetchable { 8 } else { 0 }
            }
            PciBarKind::Memory64 => {
                (self.address as u32 & 0xffff_fff0) | 4 | if self.prefetchable { 8 } else { 0 }
            }
        }
    }

    fn encoded_high(self) -> u32 {
        if self.probe {
            (!(self.size - 1) >> 32) as u32
        } else {
            (self.address >> 32) as u32
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsiMessage {
    pub address: u64,
    pub data: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsiState {
    pub enabled: bool,
    pub multiple_message_capable: u8,
    pub multiple_message_enable: u8,
    pub address: u64,
    pub data: u16,
    pub mask_bits: u32,
    pub pending_bits: u32,
}

impl Default for MsiState {
    fn default() -> Self {
        Self {
            enabled: false,
            multiple_message_capable: 0,
            multiple_message_enable: 0,
            address: 0xfee0_0000,
            data: 0,
            mask_bits: 0,
            pending_bits: 0,
        }
    }
}

impl MsiState {
    pub fn trigger(&mut self, vector: u16) -> Option<MsiMessage> {
        let count = 1u16
            << self
                .multiple_message_enable
                .min(self.multiple_message_capable);
        let index = vector % count;
        if !self.enabled || self.mask_bits & (1 << index) != 0 {
            self.pending_bits |= 1 << index;
            return None;
        }
        Some(MsiMessage {
            address: self.address,
            data: (self.data & !(count - 1)) as u32 | index as u32,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MsixTableEntry {
    pub address: u64,
    pub data: u32,
    pub masked: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsixState {
    pub enabled: bool,
    pub function_mask: bool,
    pub table_bar: u8,
    pub table_offset: u32,
    pub pba_bar: u8,
    pub pba_offset: u32,
    pub entries: Vec<MsixTableEntry>,
}

impl MsixState {
    pub fn new(vectors: usize) -> Result<Self, PciVirtioError> {
        if vectors == 0 || vectors > 2048 {
            return Err(PciVirtioError::InvalidCapability);
        }
        Ok(Self {
            enabled: false,
            function_mask: false,
            table_bar: 0,
            table_offset: 0,
            pba_bar: 0,
            pba_offset: 0x800,
            entries: vec![MsixTableEntry::default(); vectors],
        })
    }

    pub fn trigger(&mut self, vector: usize) -> Result<Option<MsiMessage>, PciVirtioError> {
        let entry = self
            .entries
            .get_mut(vector)
            .ok_or(PciVirtioError::InvalidCapability)?;
        if !self.enabled || self.function_mask || entry.masked {
            entry.pending = true;
            return Ok(None);
        }
        entry.pending = false;
        Ok(Some(MsiMessage {
            address: entry.address,
            data: entry.data,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PciFunction {
    pub address: PciAddress,
    pub bars: [Option<PciBar>; 6],
    pub msi: Option<MsiState>,
    pub msix: Option<MsixState>,
    config: [u8; PCI_CONFIG_SPACE_BYTES],
}

impl PciFunction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        address: PciAddress,
        vendor: u16,
        device: u16,
        revision: u8,
        class: u8,
        subclass: u8,
        programming_interface: u8,
        subsystem_vendor: u16,
        subsystem: u16,
    ) -> Self {
        let mut config = [0u8; PCI_CONFIG_SPACE_BYTES];
        config[0..2].copy_from_slice(&vendor.to_le_bytes());
        config[2..4].copy_from_slice(&device.to_le_bytes());
        config[8] = revision;
        config[9] = programming_interface;
        config[10] = subclass;
        config[11] = class;
        config[14] = 0;
        config[44..46].copy_from_slice(&subsystem_vendor.to_le_bytes());
        config[46..48].copy_from_slice(&subsystem.to_le_bytes());
        config[60] = 0xff;
        config[61] = 1;
        Self {
            address,
            bars: [None; 6],
            msi: None,
            msix: None,
            config,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        u16::from_le_bytes(self.config[0..2].try_into().unwrap())
    }

    pub fn device_id(&self) -> u16 {
        u16::from_le_bytes(self.config[2..4].try_into().unwrap())
    }

    pub fn set_bar(&mut self, index: usize, bar: PciBar) -> Result<(), PciVirtioError> {
        if index >= 6 || (matches!(bar.kind, PciBarKind::Memory64) && index == 5) {
            return Err(PciVirtioError::InvalidBar);
        }
        self.bars[index] = Some(bar);
        if matches!(bar.kind, PciBarKind::Memory64) {
            self.bars[index + 1] = None;
        }
        self.refresh_bars();
        Ok(())
    }

    pub fn install_msi(&mut self, offset: u8) -> Result<(), PciVirtioError> {
        if offset < 0x40 || offset as usize + 24 > 0x100 || offset & 3 != 0 {
            return Err(PciVirtioError::InvalidCapability);
        }
        self.msi = Some(MsiState::default());
        self.link_capability(offset, 0x05, 24)?;
        self.refresh_msi(offset);
        Ok(())
    }

    pub fn install_msix(&mut self, offset: u8, vectors: usize) -> Result<(), PciVirtioError> {
        if offset < 0x40 || offset as usize + 12 > 0x100 || offset & 3 != 0 {
            return Err(PciVirtioError::InvalidCapability);
        }
        self.msix = Some(MsixState::new(vectors)?);
        self.link_capability(offset, 0x11, 12)?;
        self.refresh_msix(offset);
        Ok(())
    }

    pub fn install_vendor_capability(
        &mut self,
        offset: u8,
        payload: &[u8],
    ) -> Result<(), PciVirtioError> {
        let length = payload.len() + 2;
        if offset < 0x40 || offset as usize + length > 0x100 || offset & 3 != 0 || length > 255 {
            return Err(PciVirtioError::InvalidCapability);
        }
        self.link_capability(offset, 0x09, length)?;
        self.config[offset as usize + 2..offset as usize + length].copy_from_slice(payload);
        Ok(())
    }

    fn link_capability(
        &mut self,
        offset: u8,
        capability_id: u8,
        length: usize,
    ) -> Result<(), PciVirtioError> {
        if self.config[offset as usize..offset as usize + length]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(PciVirtioError::InvalidCapability);
        }
        self.config[6] |= 0x10;
        self.config[offset as usize] = capability_id;
        let first = self.config[0x34];
        if first == 0 {
            self.config[0x34] = offset;
            return Ok(());
        }
        let mut current = first;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(current) {
                return Err(PciVirtioError::InvalidCapability);
            }
            let next = self.config[current as usize + 1];
            if next == 0 {
                self.config[current as usize + 1] = offset;
                break;
            }
            current = next;
        }
        Ok(())
    }

    fn refresh_bars(&mut self) {
        let mut index = 0;
        while index < 6 {
            if let Some(bar) = self.bars[index] {
                let offset = 0x10 + index * 4;
                self.config[offset..offset + 4].copy_from_slice(&bar.encoded_low().to_le_bytes());
                if matches!(bar.kind, PciBarKind::Memory64) {
                    self.config[offset + 4..offset + 8]
                        .copy_from_slice(&bar.encoded_high().to_le_bytes());
                    index += 1;
                }
            }
            index += 1;
        }
    }

    fn refresh_msi(&mut self, offset: u8) {
        if let Some(msi) = &self.msi {
            let start = offset as usize;
            let control = (msi.enabled as u16)
                | ((msi.multiple_message_capable as u16 & 7) << 1)
                | ((msi.multiple_message_enable as u16 & 7) << 4)
                | (1 << 7)
                | (1 << 8);
            self.config[start + 2..start + 4].copy_from_slice(&control.to_le_bytes());
            self.config[start + 4..start + 12].copy_from_slice(&msi.address.to_le_bytes());
            self.config[start + 12..start + 14].copy_from_slice(&msi.data.to_le_bytes());
            self.config[start + 16..start + 20].copy_from_slice(&msi.mask_bits.to_le_bytes());
            self.config[start + 20..start + 24].copy_from_slice(&msi.pending_bits.to_le_bytes());
        }
    }

    fn refresh_msix(&mut self, offset: u8) {
        if let Some(msix) = &self.msix {
            let start = offset as usize;
            let control = (msix.entries.len() as u16 - 1)
                | ((msix.function_mask as u16) << 14)
                | ((msix.enabled as u16) << 15);
            self.config[start + 2..start + 4].copy_from_slice(&control.to_le_bytes());
            let table = (msix.table_offset & !7) | (msix.table_bar as u32 & 7);
            let pba = (msix.pba_offset & !7) | (msix.pba_bar as u32 & 7);
            self.config[start + 4..start + 8].copy_from_slice(&table.to_le_bytes());
            self.config[start + 8..start + 12].copy_from_slice(&pba.to_le_bytes());
        }
    }

    pub fn read_config(&mut self, offset: u16, width: u8) -> Result<u32, PciVirtioError> {
        self.refresh_bars();
        if let Some(offset) = self.find_capability(0x05) {
            self.refresh_msi(offset);
        }
        if let Some(offset) = self.find_capability(0x11) {
            self.refresh_msix(offset);
        }
        let range = config_range(offset, width)?;
        let mut bytes = [0u8; 4];
        bytes[..range.len()].copy_from_slice(&self.config[range]);
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn write_config(
        &mut self,
        offset: u16,
        width: u8,
        value: u32,
    ) -> Result<(), PciVirtioError> {
        let range = config_range(offset, width)?;
        if range.start < 4 {
            return Ok(());
        }
        if offset >= 0x10 && offset < 0x28 && width == 4 && offset & 3 == 0 {
            let index = ((offset - 0x10) / 4) as usize;
            if let Some(mut bar) = self.bars[index] {
                if value == u32::MAX {
                    bar.probe = true;
                } else {
                    bar.probe = false;
                    match bar.kind {
                        PciBarKind::Io => bar.address = (value & 0xffff_fffc) as u64,
                        PciBarKind::Memory32 | PciBarKind::Memory64 => {
                            bar.address = (bar.address & 0xffff_ffff_0000_0000)
                                | (value & 0xffff_fff0) as u64;
                        }
                    }
                    bar.address &= !(bar.size - 1);
                }
                self.bars[index] = Some(bar);
                self.refresh_bars();
                return Ok(());
            }
            if index > 0 {
                if let Some(mut bar) = self.bars[index - 1] {
                    if matches!(bar.kind, PciBarKind::Memory64) {
                        if value == u32::MAX {
                            bar.probe = true;
                        } else {
                            bar.probe = false;
                            bar.address = ((value as u64) << 32) | (bar.address as u32 as u64);
                            bar.address &= !(bar.size - 1);
                        }
                        self.bars[index - 1] = Some(bar);
                        self.refresh_bars();
                        return Ok(());
                    }
                }
            }
        }
        let value_bytes = value.to_le_bytes();
        for (index, destination) in range.clone().enumerate() {
            match destination {
                4 | 5 => self.config[destination] = value_bytes[index],
                6 | 7 => self.config[destination] &= !value_bytes[index],
                _ if destination >= 0x40 => self.config[destination] = value_bytes[index],
                _ => {}
            }
        }
        self.sync_capability_writes();
        Ok(())
    }

    fn sync_capability_writes(&mut self) {
        if let (Some(offset), Some(msi)) = (self.find_capability(0x05), self.msi.as_mut()) {
            let start = offset as usize;
            let control = u16::from_le_bytes(self.config[start + 2..start + 4].try_into().unwrap());
            msi.enabled = control & 1 != 0;
            msi.multiple_message_enable = ((control >> 4) & 7) as u8;
            msi.address =
                u64::from_le_bytes(self.config[start + 4..start + 12].try_into().unwrap());
            msi.data = u16::from_le_bytes(self.config[start + 12..start + 14].try_into().unwrap());
            msi.mask_bits =
                u32::from_le_bytes(self.config[start + 16..start + 20].try_into().unwrap());
        }
        if let (Some(offset), Some(msix)) = (self.find_capability(0x11), self.msix.as_mut()) {
            let start = offset as usize;
            let control = u16::from_le_bytes(self.config[start + 2..start + 4].try_into().unwrap());
            msix.function_mask = control & (1 << 14) != 0;
            msix.enabled = control & (1 << 15) != 0;
        }
    }

    pub fn find_capability(&self, wanted: u8) -> Option<u8> {
        let mut current = self.config[0x34];
        let mut visited = BTreeSet::new();
        while current >= 0x40 && visited.insert(current) {
            if self.config[current as usize] == wanted {
                return Some(current);
            }
            current = self.config[current as usize + 1];
        }
        None
    }
}

fn config_range(offset: u16, width: u8) -> Result<std::ops::Range<usize>, PciVirtioError> {
    if !matches!(width, 1 | 2 | 4) || offset as usize + width as usize > PCI_CONFIG_SPACE_BYTES {
        return Err(PciVirtioError::InvalidConfigAccess);
    }
    if offset as usize & (width as usize - 1) != 0 {
        return Err(PciVirtioError::InvalidConfigAccess);
    }
    Ok(offset as usize..offset as usize + width as usize)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PciBus {
    pub functions: BTreeMap<PciAddress, PciFunction>,
    pub config_address: u32,
}

impl PciBus {
    pub fn attach(&mut self, function: PciFunction) -> Result<(), PciVirtioError> {
        if self.functions.insert(function.address, function).is_some() {
            return Err(PciVirtioError::DuplicatePciAddress);
        }
        Ok(())
    }

    pub fn detach(&mut self, address: PciAddress) -> Option<PciFunction> {
        self.functions.remove(&address)
    }

    pub fn config_read(
        &mut self,
        address: PciAddress,
        offset: u16,
        width: u8,
    ) -> Result<u32, PciVirtioError> {
        match self.functions.get_mut(&address) {
            Some(function) => function.read_config(offset, width),
            None => Ok(match width {
                1 => 0xff,
                2 => 0xffff,
                _ => u32::MAX,
            }),
        }
    }

    pub fn config_write(
        &mut self,
        address: PciAddress,
        offset: u16,
        width: u8,
        value: u32,
    ) -> Result<(), PciVirtioError> {
        let function = self
            .functions
            .get_mut(&address)
            .ok_or(PciVirtioError::DeviceNotFound)?;
        function.write_config(offset, width, value)
    }

    pub fn mechanism_one_write(
        &mut self,
        port: u16,
        width: u8,
        value: u32,
    ) -> Result<(), PciVirtioError> {
        if port == 0xcf8 && width == 4 {
            self.config_address = value;
            return Ok(());
        }
        if !(0xcfc..=0xcff).contains(&port) {
            return Err(PciVirtioError::InvalidConfigAccess);
        }
        let (address, register) = PciAddress::decode_config_mechanism_one(self.config_address)
            .ok_or(PciVirtioError::InvalidConfigAccess)?;
        self.config_write(address, register + (port - 0xcfc), width, value)
    }

    pub fn mechanism_one_read(&mut self, port: u16, width: u8) -> Result<u32, PciVirtioError> {
        if port == 0xcf8 && width == 4 {
            return Ok(self.config_address);
        }
        if !(0xcfc..=0xcff).contains(&port) {
            return Err(PciVirtioError::InvalidConfigAccess);
        }
        let Some((address, register)) =
            PciAddress::decode_config_mechanism_one(self.config_address)
        else {
            return Ok(u32::MAX);
        };
        self.config_read(address, register + (port - 0xcfc), width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum VirtioDeviceType {
    Network = 1,
    Block = 2,
    Console = 3,
    Gpu = 16,
    Input = 18,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VirtioFeatures {
    pub bits: u128,
}

impl VirtioFeatures {
    pub const fn from_low(bits: u64) -> Self {
        Self { bits: bits as u128 }
    }

    pub const fn contains(self, feature: u32) -> bool {
        feature < 128 && self.bits & (1u128 << feature) != 0
    }

    pub fn insert(&mut self, feature: u32) -> Result<(), PciVirtioError> {
        if feature >= 128 {
            return Err(PciVirtioError::FeatureNegotiationFailed);
        }
        self.bits |= 1u128 << feature;
        Ok(())
    }

    pub const fn page(self, select: u32) -> u32 {
        ((self.bits >> (select * 32)) & u32::MAX as u128) as u32
    }

    pub fn set_page(&mut self, select: u32, value: u32) -> Result<(), PciVirtioError> {
        if select >= 4 {
            return Err(PciVirtioError::FeatureNegotiationFailed);
        }
        let shift = select * 32;
        self.bits &= !((u32::MAX as u128) << shift);
        self.bits |= (value as u128) << shift;
        Ok(())
    }

    pub const fn is_subset_of(self, other: Self) -> bool {
        self.bits & !other.bits == 0
    }
}

pub trait VirtioGuestMemory {
    fn capacity(&self) -> usize;
    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), PciVirtioError>;
    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), PciVirtioError>;
}

impl VirtioGuestMemory for [u8] {
    fn capacity(&self) -> usize {
        self.len()
    }

    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), PciVirtioError> {
        let range = guest_range(self.len(), address, output.len())?;
        output.copy_from_slice(&self[range]);
        Ok(())
    }

    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), PciVirtioError> {
        let range = guest_range(self.len(), address, input.len())?;
        self[range].copy_from_slice(input);
        Ok(())
    }
}

impl VirtioGuestMemory for Vec<u8> {
    fn capacity(&self) -> usize {
        self.len()
    }

    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), PciVirtioError> {
        self.as_slice().read(address, output)
    }

    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), PciVirtioError> {
        self.as_mut_slice().write(address, input)
    }
}

fn guest_range(
    capacity: usize,
    address: u64,
    length: usize,
) -> Result<std::ops::Range<usize>, PciVirtioError> {
    let start =
        usize::try_from(address).map_err(|_| PciVirtioError::GuestMemory { address, length })?;
    let end = start
        .checked_add(length)
        .filter(|end| *end <= capacity)
        .ok_or(PciVirtioError::GuestMemory { address, length })?;
    Ok(start..end)
}

fn read_u16<M: VirtioGuestMemory + ?Sized>(
    memory: &M,
    address: u64,
) -> Result<u16, PciVirtioError> {
    let mut bytes = [0u8; 2];
    memory.read(address, &mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32<M: VirtioGuestMemory + ?Sized>(
    memory: &M,
    address: u64,
) -> Result<u32, PciVirtioError> {
    let mut bytes = [0u8; 4];
    memory.read(address, &mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64<M: VirtioGuestMemory + ?Sized>(
    memory: &M,
    address: u64,
) -> Result<u64, PciVirtioError> {
    let mut bytes = [0u8; 8];
    memory.read(address, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioDescriptor {
    pub address: u64,
    pub length: u32,
    pub flags: u16,
    pub next: u16,
}

impl VirtioDescriptor {
    pub fn read<M: VirtioGuestMemory + ?Sized>(
        memory: &M,
        table: u64,
        index: u16,
    ) -> Result<Self, PciVirtioError> {
        let address = table
            .checked_add(index as u64 * 16)
            .ok_or(PciVirtioError::InvalidDescriptor)?;
        Ok(Self {
            address: read_u64(memory, address)?,
            length: read_u32(memory, address + 8)?,
            flags: read_u16(memory, address + 12)?,
            next: read_u16(memory, address + 14)?,
        })
    }

    pub const fn writable(self) -> bool {
        self.flags & VIRTQ_DESC_F_WRITE != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioDescriptorChain {
    pub head: u16,
    pub descriptors: Vec<VirtioDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtQueue {
    pub maximum_size: u16,
    pub size: u16,
    pub descriptor_address: u64,
    pub driver_address: u64,
    pub device_address: u64,
    pub ready: bool,
    pub notification_enabled: bool,
    pub last_available_index: u16,
    pub used_index: u16,
}

impl VirtQueue {
    pub fn new(maximum_size: u16) -> Result<Self, PciVirtioError> {
        if maximum_size == 0 || maximum_size > 32768 || !maximum_size.is_power_of_two() {
            return Err(PciVirtioError::InvalidQueue);
        }
        Ok(Self {
            maximum_size,
            size: maximum_size,
            descriptor_address: 0,
            driver_address: 0,
            device_address: 0,
            ready: false,
            notification_enabled: true,
            last_available_index: 0,
            used_index: 0,
        })
    }

    pub fn configure(
        &mut self,
        size: u16,
        descriptor_address: u64,
        driver_address: u64,
        device_address: u64,
    ) -> Result<(), PciVirtioError> {
        if size == 0 || size > self.maximum_size || !size.is_power_of_two() {
            return Err(PciVirtioError::InvalidQueue);
        }
        if descriptor_address & 0xf != 0 || driver_address & 1 != 0 || device_address & 3 != 0 {
            return Err(PciVirtioError::InvalidQueue);
        }
        self.size = size;
        self.descriptor_address = descriptor_address;
        self.driver_address = driver_address;
        self.device_address = device_address;
        self.ready = true;
        self.last_available_index = 0;
        self.used_index = 0;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.ready = false;
        self.last_available_index = 0;
        self.used_index = 0;
    }

    pub fn pop_available<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &M,
    ) -> Result<Option<VirtioDescriptorChain>, PciVirtioError> {
        if !self.ready {
            return Err(PciVirtioError::QueueNotReady);
        }
        let available_index = read_u16(memory, self.driver_address + 2)?;
        if available_index == self.last_available_index {
            return Ok(None);
        }
        let ring_index = self.last_available_index & (self.size - 1);
        let head = read_u16(memory, self.driver_address + 4 + ring_index as u64 * 2)?;
        if head >= self.size {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        self.last_available_index = self.last_available_index.wrapping_add(1);
        Ok(Some(self.load_chain(memory, head)?))
    }

    fn load_chain<M: VirtioGuestMemory + ?Sized>(
        &self,
        memory: &M,
        head: u16,
    ) -> Result<VirtioDescriptorChain, PciVirtioError> {
        let first = VirtioDescriptor::read(memory, self.descriptor_address, head)?;
        if first.flags & VIRTQ_DESC_F_INDIRECT != 0 {
            if first.flags & VIRTQ_DESC_F_NEXT != 0
                || first.length == 0
                || first.length % 16 != 0
                || first.length / 16 > 32768
            {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            return self.load_indirect_chain(memory, head, first);
        }
        let mut descriptors = Vec::new();
        let mut visited = BTreeSet::new();
        let mut index = head;
        loop {
            if !visited.insert(index) {
                return Err(PciVirtioError::DescriptorCycle);
            }
            if index >= self.size || descriptors.len() >= self.size as usize {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            let descriptor = VirtioDescriptor::read(memory, self.descriptor_address, index)?;
            if descriptor.flags & VIRTQ_DESC_F_INDIRECT != 0 {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            descriptors.push(descriptor);
            if descriptor.flags & VIRTQ_DESC_F_NEXT == 0 {
                break;
            }
            index = descriptor.next;
        }
        Ok(VirtioDescriptorChain { head, descriptors })
    }

    fn load_indirect_chain<M: VirtioGuestMemory + ?Sized>(
        &self,
        memory: &M,
        head: u16,
        indirect: VirtioDescriptor,
    ) -> Result<VirtioDescriptorChain, PciVirtioError> {
        let entries = indirect.length / 16;
        let mut descriptors = Vec::new();
        let mut visited = BTreeSet::new();
        let mut index = 0u16;
        loop {
            if index as u32 >= entries || !visited.insert(index) {
                return Err(PciVirtioError::DescriptorCycle);
            }
            let descriptor = VirtioDescriptor::read(memory, indirect.address, index)?;
            if descriptor.flags & VIRTQ_DESC_F_INDIRECT != 0 {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            descriptors.push(descriptor);
            if descriptor.flags & VIRTQ_DESC_F_NEXT == 0 {
                break;
            }
            index = descriptor.next;
        }
        Ok(VirtioDescriptorChain { head, descriptors })
    }

    pub fn add_used<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
        head: u16,
        length: u32,
    ) -> Result<(), PciVirtioError> {
        if !self.ready {
            return Err(PciVirtioError::QueueNotReady);
        }
        let ring_index = self.used_index & (self.size - 1);
        let address = self.device_address + 4 + ring_index as u64 * 8;
        memory.write(address, &(head as u32).to_le_bytes())?;
        memory.write(address + 4, &length.to_le_bytes())?;
        // The used element must be globally visible before the device publishes used.idx.
        fence(Ordering::Release);
        self.used_index = self.used_index.wrapping_add(1);
        memory.write(self.device_address + 2, &self.used_index.to_le_bytes())?;
        Ok(())
    }

    pub fn finish_batch<M: VirtioGuestMemory + ?Sized>(
        &self,
        memory: &mut M,
        old_used_index: u16,
        event_index_negotiated: bool,
    ) -> Result<bool, PciVirtioError> {
        if !self.ready {
            return Err(PciVirtioError::QueueNotReady);
        }
        if event_index_negotiated {
            let used_event = read_u16(memory, self.driver_address + 4 + u64::from(self.size) * 2)?;
            memory.write(
                self.device_address + 4 + u64::from(self.size) * 8,
                &self.last_available_index.to_le_bytes(),
            )?;
            fence(Ordering::Release);
            return Ok(vring_need_event(
                used_event,
                self.used_index,
                old_used_index,
            ));
        }
        Ok(read_u16(memory, self.driver_address)? & VIRTQ_AVAIL_F_NO_INTERRUPT == 0)
    }
}

pub const fn vring_need_event(event_index: u16, new_index: u16, old_index: u16) -> bool {
    new_index.wrapping_sub(event_index).wrapping_sub(1) < new_index.wrapping_sub(old_index)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioPciTransport {
    pub device_type: VirtioDeviceType,
    pub offered_features: VirtioFeatures,
    pub driver_features: VirtioFeatures,
    pub device_status: u8,
    pub config_generation: u8,
    pub device_feature_select: u32,
    pub driver_feature_select: u32,
    pub queue_select: u16,
    pub msix_config_vector: u16,
    pub queues: Vec<VirtQueue>,
    pub notifications: VecDeque<u16>,
    pub isr_status: u8,
}

impl VirtioPciTransport {
    pub fn new(
        device_type: VirtioDeviceType,
        queue_sizes: &[u16],
        mut features: VirtioFeatures,
    ) -> Result<Self, PciVirtioError> {
        features.insert(32)?;
        let queues = queue_sizes
            .iter()
            .map(|size| VirtQueue::new(*size))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            device_type,
            offered_features: features,
            driver_features: VirtioFeatures::default(),
            device_status: 0,
            config_generation: 0,
            device_feature_select: 0,
            driver_feature_select: 0,
            queue_select: 0,
            msix_config_vector: u16::MAX,
            queues,
            notifications: VecDeque::new(),
            isr_status: 0,
        })
    }

    pub fn pci_function(
        &self,
        address: PciAddress,
        subsystem: u16,
        common_bar: PciBar,
        notify_bar: PciBar,
        device_bar: PciBar,
    ) -> Result<PciFunction, PciVirtioError> {
        let class = match self.device_type {
            VirtioDeviceType::Network => (0x02, 0x00),
            VirtioDeviceType::Block => (0x01, 0x00),
            VirtioDeviceType::Console => (0x07, 0x80),
            VirtioDeviceType::Gpu => (0x03, 0x80),
            VirtioDeviceType::Input => (0x09, 0x80),
        };
        let mut function = PciFunction::new(
            address,
            PCI_VENDOR_VIRTIO,
            VIRTIO_PCI_DEVICE_BASE + self.device_type as u16,
            1,
            class.0,
            class.1,
            0,
            PCI_VENDOR_VIRTIO,
            subsystem,
        );
        function.set_bar(0, common_bar)?;
        function.set_bar(2, notify_bar)?;
        function.set_bar(4, device_bar)?;
        function.install_msix(0x50, self.queues.len() + 1)?;
        function.install_vendor_capability(0x60, &virtio_pci_capability(1, 0, 0, 0x38, 0))?;
        function.install_vendor_capability(0x74, &virtio_pci_capability(2, 2, 0, 4, 4))?;
        function.install_vendor_capability(0x88, &virtio_pci_capability(3, 0, 0x38, 1, 0))?;
        function.install_vendor_capability(0x9c, &virtio_pci_capability(4, 4, 0, 0x100, 0))?;
        Ok(function)
    }

    pub fn set_status(&mut self, value: u8) -> Result<(), PciVirtioError> {
        if value == 0 {
            self.reset();
            return Ok(());
        }
        if value & self.device_status != self.device_status {
            return Err(PciVirtioError::InvalidStatusTransition);
        }
        if value & VIRTIO_STATUS_DRIVER != 0 && value & VIRTIO_STATUS_ACKNOWLEDGE == 0 {
            return Err(PciVirtioError::InvalidStatusTransition);
        }
        if value & VIRTIO_STATUS_FEATURES_OK != 0 {
            if value & VIRTIO_STATUS_DRIVER == 0
                || !self.driver_features.is_subset_of(self.offered_features)
                || !self.driver_features.contains(32)
            {
                self.device_status = value & !VIRTIO_STATUS_FEATURES_OK;
                return Err(PciVirtioError::FeatureNegotiationFailed);
            }
        }
        if value & VIRTIO_STATUS_DRIVER_OK != 0 && value & VIRTIO_STATUS_FEATURES_OK == 0 {
            return Err(PciVirtioError::InvalidStatusTransition);
        }
        self.device_status = value;
        Ok(())
    }

    pub fn ready(&self) -> bool {
        self.device_status
            & (VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK)
            == (VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK)
            && self.device_status & (VIRTIO_STATUS_FAILED | VIRTIO_STATUS_DEVICE_NEEDS_RESET) == 0
    }

    pub fn notify(&mut self, queue: u16) -> Result<(), PciVirtioError> {
        let selected = self
            .queues
            .get(queue as usize)
            .ok_or(PciVirtioError::InvalidQueue)?;
        if !self.ready() || !selected.ready {
            return Err(PciVirtioError::QueueNotReady);
        }
        self.notifications.push_back(queue);
        Ok(())
    }

    pub fn interrupt(&mut self, configuration: bool) {
        self.isr_status |= if configuration { 2 } else { 1 };
    }

    pub fn read_isr(&mut self) -> u8 {
        let value = self.isr_status;
        self.isr_status = 0;
        value
    }

    pub fn reset(&mut self) {
        self.driver_features = VirtioFeatures::default();
        self.device_status = 0;
        self.device_feature_select = 0;
        self.driver_feature_select = 0;
        self.queue_select = 0;
        self.notifications.clear();
        self.isr_status = 0;
        for queue in &mut self.queues {
            queue.reset();
        }
        self.config_generation = self.config_generation.wrapping_add(1);
    }
}

fn virtio_pci_capability(
    configuration_type: u8,
    bar: u8,
    offset: u32,
    length: u32,
    notify_multiplier: u32,
) -> Vec<u8> {
    let capability_length = if configuration_type == 2 { 20 } else { 16 };
    let mut bytes = vec![0u8; capability_length - 2];
    bytes[0] = capability_length as u8;
    bytes[1] = configuration_type;
    bytes[2] = bar;
    bytes[6..10].copy_from_slice(&offset.to_le_bytes());
    bytes[10..14].copy_from_slice(&length.to_le_bytes());
    if configuration_type == 2 {
        bytes[14..18].copy_from_slice(&notify_multiplier.to_le_bytes());
    }
    bytes
}

fn read_descriptor_bytes<M: VirtioGuestMemory + ?Sized>(
    memory: &M,
    descriptors: &[VirtioDescriptor],
    require_writable: bool,
) -> Result<Vec<u8>, PciVirtioError> {
    let mut bytes = Vec::new();
    for descriptor in descriptors {
        if descriptor.writable() != require_writable {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        let start = bytes.len();
        bytes.resize(start + descriptor.length as usize, 0);
        memory.read(descriptor.address, &mut bytes[start..])?;
    }
    Ok(bytes)
}

fn write_descriptor_bytes<M: VirtioGuestMemory + ?Sized>(
    memory: &mut M,
    descriptors: &[VirtioDescriptor],
    input: &[u8],
) -> Result<usize, PciVirtioError> {
    let mut written = 0;
    for descriptor in descriptors {
        if !descriptor.writable() {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        let length = (descriptor.length as usize).min(input.len().saturating_sub(written));
        memory.write(descriptor.address, &input[written..written + length])?;
        written += length;
        if written == input.len() {
            break;
        }
    }
    Ok(written)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioBlockDevice {
    pub transport: VirtioPciTransport,
    pub backend: VirtualBlockDevice,
    pub identifier: [u8; 20],
}

impl VirtioBlockDevice {
    pub fn new(backend: VirtualBlockDevice, identifier: &[u8]) -> Result<Self, PciVirtioError> {
        let mut id = [0u8; 20];
        let length = identifier.len().min(id.len());
        id[..length].copy_from_slice(&identifier[..length]);
        let features = VirtioFeatures::from_low(
            VIRTIO_F_RING_INDIRECT_DESC
                | VIRTIO_F_RING_EVENT_IDX
                | (1 << 9)
                | (1 << 13)
                | (1 << 14),
        );
        Ok(Self {
            transport: VirtioPciTransport::new(VirtioDeviceType::Block, &[256], features)?,
            backend,
            identifier: id,
        })
    }

    pub fn process_queue<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = self
            .transport
            .queues
            .get_mut(0)
            .ok_or(PciVirtioError::InvalidQueue)?;
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while let Some(chain) = queue.pop_available(memory)? {
            if chain.descriptors.len() < 2 {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            let header_descriptor = chain.descriptors[0];
            let status_descriptor = *chain.descriptors.last().unwrap();
            if header_descriptor.writable()
                || header_descriptor.length < 16
                || !status_descriptor.writable()
            {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            let mut header = [0u8; 16];
            memory.read(header_descriptor.address, &mut header)?;
            let request = u32::from_le_bytes(header[0..4].try_into().unwrap());
            let sector = u64::from_le_bytes(header[8..16].try_into().unwrap());
            let data_descriptors = &chain.descriptors[1..chain.descriptors.len() - 1];
            let result = match request {
                0 => {
                    if data_descriptors
                        .iter()
                        .any(|descriptor| !descriptor.writable())
                    {
                        Err(PciVirtioError::InvalidDescriptor)
                    } else {
                        let length = data_descriptors
                            .iter()
                            .map(|descriptor| descriptor.length as usize)
                            .sum();
                        let mut output = vec![0u8; length];
                        self.backend.read_sectors(sector, &mut output)?;
                        let written = write_descriptor_bytes(memory, data_descriptors, &output)?;
                        Ok(written as u32)
                    }
                }
                1 => {
                    let input = read_descriptor_bytes(memory, data_descriptors, false)?;
                    self.backend.write_sectors(sector, &input)?;
                    Ok(0)
                }
                4 => {
                    self.backend.flush();
                    Ok(0)
                }
                8 => Ok(write_descriptor_bytes(memory, data_descriptors, &self.identifier)? as u32),
                11 | 13 => {
                    let segments = read_descriptor_bytes(memory, data_descriptors, false)?;
                    if segments.len() % 16 != 0 {
                        return Err(PciVirtioError::InvalidDescriptor);
                    }
                    for segment in segments.chunks_exact(16) {
                        let sector = u64::from_le_bytes(segment[0..8].try_into().unwrap());
                        let count = u32::from_le_bytes(segment[8..12].try_into().unwrap());
                        let offset = sector
                            .checked_mul(self.backend.sector_size as u64)
                            .ok_or(PciVirtioError::InvalidDescriptor)?;
                        let length = count as usize * self.backend.sector_size as usize;
                        if request == 11 {
                            self.backend.discard(offset, length)?;
                        } else {
                            self.backend.write_zeroes(offset, length)?;
                        }
                    }
                    Ok(0)
                }
                other => Err(PciVirtioError::UnsupportedRequest(other)),
            };
            let (status, used_length) = match result {
                Ok(length) => (0u8, length.saturating_add(1)),
                Err(PciVirtioError::UnsupportedRequest(_)) => (2u8, 1),
                Err(error) => {
                    memory.write(status_descriptor.address, &[1])?;
                    queue.add_used(memory, chain.head, 1)?;
                    let raise_interrupt =
                        queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
                    if raise_interrupt {
                        self.transport.interrupt(false);
                    }
                    return Err(error);
                }
            };
            memory.write(status_descriptor.address, &[status])?;
            queue.add_used(memory, chain.head, used_length)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioNetworkDevice {
    pub transport: VirtioPciTransport,
    pub backend: VirtualNetworkDevice,
}

impl VirtioNetworkDevice {
    pub fn new(backend: VirtualNetworkDevice) -> Result<Self, PciVirtioError> {
        let features = VirtioFeatures::from_low(
            VIRTIO_F_RING_INDIRECT_DESC | VIRTIO_F_RING_EVENT_IDX | (1 << 5) | (1 << 16),
        );
        Ok(Self {
            transport: VirtioPciTransport::new(
                VirtioDeviceType::Network,
                &[256, 256, 64],
                features,
            )?,
            backend,
        })
    }

    pub fn process_tx<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = &mut self.transport.queues[1];
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while let Some(chain) = queue.pop_available(memory)? {
            let bytes = read_descriptor_bytes(memory, &chain.descriptors, false)?;
            if bytes.len() < 12 {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            self.backend.transmit(&bytes[12..])?;
            queue.add_used(memory, chain.head, bytes.len() as u32)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }

    pub fn process_rx<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = &mut self.transport.queues[0];
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while let Some(frame) = self.backend.receive() {
            let Some(chain) = queue.pop_available(memory)? else {
                self.backend.inject_received(&frame)?;
                break;
            };
            let mut packet = vec![0u8; 12];
            packet.extend_from_slice(&frame);
            let written = write_descriptor_bytes(memory, &chain.descriptors, &packet)?;
            if written != packet.len() {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            queue.add_used(memory, chain.head, written as u32)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioConsoleDevice {
    pub transport: VirtioPciTransport,
    pub backend: Uart16550,
}

impl VirtioConsoleDevice {
    pub fn new(backend: Uart16550) -> Result<Self, PciVirtioError> {
        Ok(Self {
            transport: VirtioPciTransport::new(
                VirtioDeviceType::Console,
                &[128, 128, 32, 32],
                VirtioFeatures::from_low(VIRTIO_F_RING_INDIRECT_DESC | VIRTIO_F_RING_EVENT_IDX),
            )?,
            backend,
        })
    }

    pub fn process_tx<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = &mut self.transport.queues[1];
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while let Some(chain) = queue.pop_available(memory)? {
            let bytes = read_descriptor_bytes(memory, &chain.descriptors, false)?;
            for byte in &bytes {
                self.backend.write(0, *byte)?;
            }
            queue.add_used(memory, chain.head, bytes.len() as u32)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }

    pub fn process_rx<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = &mut self.transport.queues[0];
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while self.backend.pending_rx() != 0 {
            let Some(chain) = queue.pop_available(memory)? else {
                break;
            };
            let capacity: usize = chain
                .descriptors
                .iter()
                .map(|descriptor| descriptor.length as usize)
                .sum();
            let mut bytes = Vec::new();
            while bytes.len() < capacity && self.backend.pending_rx() != 0 {
                bytes.push(self.backend.read(0)?);
            }
            let written = write_descriptor_bytes(memory, &chain.descriptors, &bytes)?;
            queue.add_used(memory, chain.head, written as u32)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioInputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioInputDevice {
    pub transport: VirtioPciTransport,
    pending: VecDeque<VirtioInputEvent>,
}

impl VirtioInputDevice {
    pub fn new() -> Result<Self, PciVirtioError> {
        Ok(Self {
            transport: VirtioPciTransport::new(
                VirtioDeviceType::Input,
                &[64, 8],
                VirtioFeatures::from_low(VIRTIO_F_RING_EVENT_IDX),
            )?,
            pending: VecDeque::new(),
        })
    }

    pub fn inject_key(&mut self, event: Ps2KeyEvent, linux_code: u16) {
        self.pending.push_back(VirtioInputEvent {
            event_type: 1,
            code: linux_code,
            value: event.pressed as u32,
        });
        self.pending.push_back(VirtioInputEvent {
            event_type: 0,
            code: 0,
            value: 0,
        });
    }

    pub fn process_events<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<usize, PciVirtioError> {
        let event_index_negotiated = self.transport.driver_features.contains(29);
        let queue = &mut self.transport.queues[0];
        let old_used_index = queue.used_index;
        let mut completed = 0;
        while let Some(event) = self.pending.pop_front() {
            let Some(chain) = queue.pop_available(memory)? else {
                self.pending.push_front(event);
                break;
            };
            let mut bytes = [0u8; 8];
            bytes[0..2].copy_from_slice(&event.event_type.to_le_bytes());
            bytes[2..4].copy_from_slice(&event.code.to_le_bytes());
            bytes[4..8].copy_from_slice(&event.value.to_le_bytes());
            if write_descriptor_bytes(memory, &chain.descriptors, &bytes)? != bytes.len() {
                return Err(PciVirtioError::InvalidDescriptor);
            }
            queue.add_used(memory, chain.head, bytes.len() as u32)?;
            completed += 1;
        }
        let raise_interrupt = queue.finish_batch(memory, old_used_index, event_index_negotiated)?;
        if completed != 0 && raise_interrupt {
            self.transport.interrupt(false);
        }
        Ok(completed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioGpuResource {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub backing: Vec<(u64, u32)>,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioGpuDevice {
    pub transport: VirtioPciTransport,
    pub scanout: Option<u32>,
    pub framebuffer: VirtualFramebuffer,
    pub resources: BTreeMap<u32, VirtioGpuResource>,
    pub flush_count: u64,
}

impl VirtioGpuDevice {
    pub fn new(framebuffer: VirtualFramebuffer) -> Result<Self, PciVirtioError> {
        Ok(Self {
            transport: VirtioPciTransport::new(
                VirtioDeviceType::Gpu,
                &[256, 16],
                VirtioFeatures::from_low(VIRTIO_F_RING_INDIRECT_DESC | VIRTIO_F_RING_EVENT_IDX),
            )?,
            scanout: None,
            framebuffer,
            resources: BTreeMap::new(),
            flush_count: 0,
        })
    }

    pub fn create_resource_2d(
        &mut self,
        id: u32,
        width: u32,
        height: u32,
        format: PixelFormat,
    ) -> Result<(), PciVirtioError> {
        if id == 0 || self.resources.contains_key(&id) || width == 0 || height == 0 {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        let length = width as usize * height as usize * format.bytes_per_pixel();
        self.resources.insert(
            id,
            VirtioGpuResource {
                id,
                width,
                height,
                format,
                backing: Vec::new(),
                pixels: vec![0; length],
            },
        );
        Ok(())
    }

    pub fn attach_backing(
        &mut self,
        id: u32,
        entries: &[(u64, u32)],
    ) -> Result<(), PciVirtioError> {
        let resource = self
            .resources
            .get_mut(&id)
            .ok_or(PciVirtioError::DeviceNotFound)?;
        if entries.is_empty() || entries.iter().any(|(_, length)| *length == 0) {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        resource.backing = entries.to_vec();
        Ok(())
    }

    pub fn transfer_to_host_2d<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        memory: &M,
        id: u32,
    ) -> Result<(), PciVirtioError> {
        let resource = self
            .resources
            .get_mut(&id)
            .ok_or(PciVirtioError::DeviceNotFound)?;
        let mut copied = 0;
        for (address, length) in &resource.backing {
            let length = (*length as usize).min(resource.pixels.len().saturating_sub(copied));
            memory.read(*address, &mut resource.pixels[copied..copied + length])?;
            copied += length;
            if copied == resource.pixels.len() {
                break;
            }
        }
        if copied != resource.pixels.len() {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        Ok(())
    }

    pub fn set_scanout(&mut self, id: u32) -> Result<(), PciVirtioError> {
        let resource = self
            .resources
            .get(&id)
            .ok_or(PciVirtioError::DeviceNotFound)?;
        if resource.width != self.framebuffer.width
            || resource.height != self.framebuffer.height
            || resource.format != self.framebuffer.format
        {
            return Err(PciVirtioError::InvalidDescriptor);
        }
        self.scanout = Some(id);
        Ok(())
    }

    pub fn resource_flush(&mut self, id: u32) -> Result<(), PciVirtioError> {
        if self.scanout != Some(id) {
            return Err(PciVirtioError::DeviceNotFound);
        }
        let resource = self
            .resources
            .get(&id)
            .ok_or(PciVirtioError::DeviceNotFound)?;
        for y in 0..resource.height {
            for x in 0..resource.width {
                let offset = y as usize * self.framebuffer.stride as usize
                    + x as usize * resource.format.bytes_per_pixel();
                let rgba = match resource.format {
                    PixelFormat::Rgba8888 => {
                        u32::from_be_bytes(resource.pixels[offset..offset + 4].try_into().unwrap())
                    }
                    PixelFormat::Bgra8888 => {
                        let pixel = &resource.pixels[offset..offset + 4];
                        u32::from_be_bytes([pixel[2], pixel[1], pixel[0], pixel[3]])
                    }
                    PixelFormat::Rgb565 => {
                        let value = u16::from_le_bytes(
                            resource.pixels[offset..offset + 2].try_into().unwrap(),
                        );
                        let red = ((value >> 11) & 0x1f) as u32 * 255 / 31;
                        let green = ((value >> 5) & 0x3f) as u32 * 255 / 63;
                        let blue = (value & 0x1f) as u32 * 255 / 31;
                        red << 24 | green << 16 | blue << 8 | 0xff
                    }
                };
                self.framebuffer.write_pixel(x, y, rgba)?;
            }
        }
        self.flush_count = self.flush_count.wrapping_add(1);
        Ok(())
    }

    pub fn destroy_resource(&mut self, id: u32) -> Result<(), PciVirtioError> {
        if self.resources.remove(&id).is_none() {
            return Err(PciVirtioError::DeviceNotFound);
        }
        if self.scanout == Some(id) {
            self.scanout = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_descriptor(memory: &mut [u8], table: u64, index: u16, descriptor: VirtioDescriptor) {
        let offset = table as usize + index as usize * 16;
        memory[offset..offset + 8].copy_from_slice(&descriptor.address.to_le_bytes());
        memory[offset + 8..offset + 12].copy_from_slice(&descriptor.length.to_le_bytes());
        memory[offset + 12..offset + 14].copy_from_slice(&descriptor.flags.to_le_bytes());
        memory[offset + 14..offset + 16].copy_from_slice(&descriptor.next.to_le_bytes());
    }

    fn configure_queue(queue: &mut VirtQueue, memory: &mut [u8], head: u16) {
        queue.configure(8, 0x1000, 0x1800, 0x1c00).unwrap();
        memory[0x1802..0x1804].copy_from_slice(&1u16.to_le_bytes());
        memory[0x1804..0x1806].copy_from_slice(&head.to_le_bytes());
    }

    fn ready_transport(transport: &mut VirtioPciTransport) {
        transport.driver_features = transport.offered_features;
        transport
            .set_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER)
            .unwrap();
        transport
            .set_status(
                VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK,
            )
            .unwrap();
        transport
            .set_status(
                VIRTIO_STATUS_ACKNOWLEDGE
                    | VIRTIO_STATUS_DRIVER
                    | VIRTIO_STATUS_FEATURES_OK
                    | VIRTIO_STATUS_DRIVER_OK,
            )
            .unwrap();
    }

    #[test]
    fn split_queue_event_index_publishes_next_kick_and_interrupt_contract() {
        let mut memory = vec![0u8; 0x4000];
        let mut queue = VirtQueue::new(8).unwrap();
        queue.configure(8, 0x1000, 0x1800, 0x1c00).unwrap();

        queue.last_available_index = 1;
        queue.used_index = 1;
        memory[0x1814..0x1816].copy_from_slice(&0_u16.to_le_bytes());
        assert!(queue.finish_batch(&mut memory, 0, true).unwrap());
        assert_eq!(
            u16::from_le_bytes(memory[0x1c44..0x1c46].try_into().unwrap()),
            1
        );

        queue.last_available_index = 2;
        queue.used_index = 2;
        memory[0x1814..0x1816].copy_from_slice(&1_u16.to_le_bytes());
        assert!(queue.finish_batch(&mut memory, 1, true).unwrap());
        assert_eq!(
            u16::from_le_bytes(memory[0x1c44..0x1c46].try_into().unwrap()),
            2
        );

        assert!(vring_need_event(0, 1, 0));
        assert!(vring_need_event(u16::MAX, 0, u16::MAX));
        assert!(!vring_need_event(0, 2, 1));
    }

    #[test]
    fn split_queue_legacy_interrupt_suppression_is_honored() {
        let mut memory = vec![0u8; 0x4000];
        let mut queue = VirtQueue::new(8).unwrap();
        queue.configure(8, 0x1000, 0x1800, 0x1c00).unwrap();
        queue.last_available_index = 1;
        queue.used_index = 1;

        memory[0x1800..0x1802].copy_from_slice(&VIRTQ_AVAIL_F_NO_INTERRUPT.to_le_bytes());
        assert!(!queue.finish_batch(&mut memory, 0, false).unwrap());
        memory[0x1800..0x1802].copy_from_slice(&0_u16.to_le_bytes());
        assert!(queue.finish_batch(&mut memory, 0, false).unwrap());
    }

    #[test]
    fn pci_configuration_bars_capabilities_msi_and_msix_are_material() {
        let address = PciAddress::new(0, 0, 1, 0).unwrap();
        let transport = VirtioPciTransport::new(
            VirtioDeviceType::Block,
            &[256],
            VirtioFeatures::from_low(VIRTIO_F_VERSION_1),
        )
        .unwrap();
        let function = transport
            .pci_function(
                address,
                2,
                PciBar::new(PciBarKind::Memory32, 0x1000_0000, 0x1000, false).unwrap(),
                PciBar::new(PciBarKind::Memory32, 0x1000_1000, 0x1000, false).unwrap(),
                PciBar::new(PciBarKind::Memory32, 0x1000_2000, 0x1000, true).unwrap(),
            )
            .unwrap();
        let mut bus = PciBus::default();
        bus.attach(function).unwrap();
        bus.mechanism_one_write(0xcf8, 4, address.config_mechanism_one(0))
            .unwrap();
        assert_eq!(bus.mechanism_one_read(0xcfc, 4).unwrap() & 0xffff, 0x1af4);
        bus.config_write(address, 0x10, 4, u32::MAX).unwrap();
        assert_eq!(
            bus.config_read(address, 0x10, 4).unwrap() & 0xffff_f000,
            0xffff_f000
        );
        let function = bus.functions.get(&address).unwrap();
        assert!(function.find_capability(0x11).is_some());
        assert!(function.find_capability(0x09).is_some());
    }

    #[test]
    fn virtqueue_direct_and_indirect_chains_detect_cycles_and_complete_used_ring() {
        let mut memory = vec![0u8; 0x4000];
        let mut queue = VirtQueue::new(8).unwrap();
        configure_queue(&mut queue, &mut memory, 0);
        write_descriptor(
            &mut memory,
            0x1000,
            0,
            VirtioDescriptor {
                address: 0x2000,
                length: 4,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            },
        );
        write_descriptor(
            &mut memory,
            0x1000,
            1,
            VirtioDescriptor {
                address: 0x2100,
                length: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );
        let chain = queue.pop_available(&memory).unwrap().unwrap();
        assert_eq!(chain.descriptors.len(), 2);
        queue.add_used(&mut memory, chain.head, 5).unwrap();
        assert_eq!(read_u16(&memory, 0x1c02).unwrap(), 1);

        let mut indirect_queue = VirtQueue::new(8).unwrap();
        memory[0x1802..0x1804].copy_from_slice(&1u16.to_le_bytes());
        memory[0x1804..0x1806].copy_from_slice(&0u16.to_le_bytes());
        indirect_queue.configure(8, 0x1000, 0x1800, 0x1c00).unwrap();
        write_descriptor(
            &mut memory,
            0x1000,
            0,
            VirtioDescriptor {
                address: 0x2400,
                length: 32,
                flags: VIRTQ_DESC_F_INDIRECT,
                next: 0,
            },
        );
        write_descriptor(
            &mut memory,
            0x2400,
            0,
            VirtioDescriptor {
                address: 0x2600,
                length: 8,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            },
        );
        write_descriptor(
            &mut memory,
            0x2400,
            1,
            VirtioDescriptor {
                address: 0x2700,
                length: 8,
                flags: 0,
                next: 0,
            },
        );
        assert_eq!(
            indirect_queue
                .pop_available(&memory)
                .unwrap()
                .unwrap()
                .descriptors
                .len(),
            2
        );
    }

    #[test]
    fn virtio_block_executes_write_read_flush_id_discard_and_status() {
        let backend = VirtualBlockDevice::new(8192, 512, false).unwrap();
        let mut block = VirtioBlockDevice::new(backend, b"ncc-virtio-block").unwrap();
        ready_transport(&mut block.transport);
        let mut memory = vec![0u8; 0x8000];
        configure_queue(&mut block.transport.queues[0], &mut memory, 0);
        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&1u32.to_le_bytes());
        header[8..16].copy_from_slice(&2u64.to_le_bytes());
        memory[0x3000..0x3010].copy_from_slice(&header);
        memory[0x3100..0x3300].fill(0x5a);
        write_descriptor(
            &mut memory,
            0x1000,
            0,
            VirtioDescriptor {
                address: 0x3000,
                length: 16,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            },
        );
        write_descriptor(
            &mut memory,
            0x1000,
            1,
            VirtioDescriptor {
                address: 0x3100,
                length: 512,
                flags: VIRTQ_DESC_F_NEXT,
                next: 2,
            },
        );
        write_descriptor(
            &mut memory,
            0x1000,
            2,
            VirtioDescriptor {
                address: 0x3400,
                length: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );
        assert_eq!(block.process_queue(&mut memory).unwrap(), 1);
        assert_eq!(memory[0x3400], 0);
        let mut sector = [0u8; 512];
        block.backend.read_sectors(2, &mut sector).unwrap();
        assert!(sector.iter().all(|byte| *byte == 0x5a));
    }

    #[test]
    fn virtio_network_console_input_and_gpu_move_real_guest_bytes() {
        let mut memory = vec![0u8; 0x10_000];
        let network_backend = VirtualNetworkDevice::new([0x02, 0, 0, 0, 0, 1], 1500, 8).unwrap();
        let mut network = VirtioNetworkDevice::new(network_backend).unwrap();
        ready_transport(&mut network.transport);
        configure_queue(&mut network.transport.queues[1], &mut memory, 0);
        let mut packet = vec![0u8; 12 + 64];
        packet[12..18].copy_from_slice(&[0xff; 6]);
        packet[18..24].copy_from_slice(&[0x02, 0, 0, 0, 0, 1]);
        memory[0x3000..0x3000 + packet.len()].copy_from_slice(&packet);
        write_descriptor(
            &mut memory,
            0x1000,
            0,
            VirtioDescriptor {
                address: 0x3000,
                length: packet.len() as u32,
                flags: 0,
                next: 0,
            },
        );
        assert_eq!(network.process_tx(&mut memory).unwrap(), 1);
        assert_eq!(network.backend.pending_tx(), 1);

        let mut console = VirtioConsoleDevice::new(Uart16550::new(0x3f8)).unwrap();
        ready_transport(&mut console.transport);
        memory[0x1802..0x1804].fill(0);
        configure_queue(&mut console.transport.queues[1], &mut memory, 0);
        memory[0x3500..0x3505].copy_from_slice(b"nC VM");
        write_descriptor(
            &mut memory,
            0x1000,
            0,
            VirtioDescriptor {
                address: 0x3500,
                length: 5,
                flags: 0,
                next: 0,
            },
        );
        assert_eq!(console.process_tx(&mut memory).unwrap(), 1);
        assert_eq!(console.backend.take_tx(), b"nC VM");

        let framebuffer = VirtualFramebuffer::new(2, 2, PixelFormat::Rgba8888).unwrap();
        let mut gpu = VirtioGpuDevice::new(framebuffer).unwrap();
        gpu.create_resource_2d(1, 2, 2, PixelFormat::Rgba8888)
            .unwrap();
        memory[0x4000..0x4010].copy_from_slice(&[
            0xff, 0, 0, 0xff, 0, 0xff, 0, 0xff, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ]);
        gpu.attach_backing(1, &[(0x4000, 16)]).unwrap();
        gpu.transfer_to_host_2d(&memory, 1).unwrap();
        gpu.set_scanout(1).unwrap();
        gpu.resource_flush(1).unwrap();
        assert_ne!(gpu.framebuffer.checksum(), 0);
        assert_eq!(gpu.framebuffer.take_dirty().len(), 4);
    }
}
