use crate::{
    crc32, DeviceSnapshot, GuestArchitecture, MemoryPageSnapshot, MemorySnapshot, SnapshotId,
    VcpuSnapshot, VirtualDeviceClass, VmLifecycleError, VmPowerState, VmSnapshot,
};
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

pub const VM_MIGRATION_MAGIC: [u8; 8] = *b"NCMIGR01";
pub const VM_MIGRATION_VERSION: u16 = 1;
pub const VM_MIGRATION_MAX_FRAME_BYTES: usize = 1024 * 1024 * 1024;
pub const MIGRATION_FEATURE_PRECOPY: u64 = 1 << 0;
pub const MIGRATION_FEATURE_POSTCOPY: u64 = 1 << 1;
pub const MIGRATION_FEATURE_DIRTY_BITMAP: u64 = 1 << 2;
pub const MIGRATION_FEATURE_DIRTY_RING: u64 = 1 << 3;
pub const MIGRATION_FEATURE_DEVICE_VERSIONING: u64 = 1 << 4;
pub const MIGRATION_FEATURE_CLOCK_CONTINUITY: u64 = 1 << 5;
pub const MIGRATION_FEATURE_TRANSACTIONAL_COMMIT: u64 = 1 << 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    InvalidPhase {
        expected: &'static str,
        actual: MigrationPhase,
    },
    IncompatibleArchitecture,
    IncompatibleTarget,
    IncompatiblePageSize,
    IncompatibleMemory,
    IncompatibleVcpuCount,
    MissingFeature(u64),
    MissingDevice(String),
    DeviceVersion {
        id: String,
        version: u16,
    },
    DuplicateVcpu(u32),
    DuplicateDevice(String),
    InvalidIdentity,
    DirtyPage(u64),
    DirtyBitmap,
    Sequence {
        expected: u64,
        actual: u64,
    },
    Checksum {
        expected: u32,
        actual: u32,
    },
    Frame(&'static str),
    Limit(&'static str),
    CommitToken,
    Clock,
    Snapshot(String),
    Io(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPhase { expected, actual } => {
                write!(formatter, "migration phase {actual:?}, expected {expected}")
            }
            Self::IncompatibleArchitecture => {
                formatter.write_str("migration architecture mismatch")
            }
            Self::IncompatibleTarget => formatter.write_str("migration target mismatch"),
            Self::IncompatiblePageSize => formatter.write_str("migration page size mismatch"),
            Self::IncompatibleMemory => formatter.write_str("migration memory layout mismatch"),
            Self::IncompatibleVcpuCount => formatter.write_str("migration vCPU topology mismatch"),
            Self::MissingFeature(feature) => {
                write!(formatter, "migration feature {feature:#x} is unavailable")
            }
            Self::MissingDevice(id) => {
                write!(formatter, "destination lacks required device '{id}'")
            }
            Self::DeviceVersion { id, version } => {
                write!(
                    formatter,
                    "device '{id}' schema version {version} is incompatible"
                )
            }
            Self::DuplicateVcpu(id) => write!(formatter, "duplicate migrated vCPU {id}"),
            Self::DuplicateDevice(id) => write!(formatter, "duplicate migrated device '{id}'"),
            Self::InvalidIdentity => formatter.write_str("invalid migration identity"),
            Self::DirtyPage(page) => write!(formatter, "invalid dirty page index {page}"),
            Self::DirtyBitmap => formatter.write_str("invalid KVM dirty-page bitmap"),
            Self::Sequence { expected, actual } => {
                write!(
                    formatter,
                    "migration sequence {actual}, expected {expected}"
                )
            }
            Self::Checksum { expected, actual } => write!(
                formatter,
                "migration checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
            ),
            Self::Frame(reason) => write!(formatter, "invalid migration frame: {reason}"),
            Self::Limit(reason) => write!(formatter, "migration limit exceeded: {reason}"),
            Self::CommitToken => formatter.write_str("migration commit token mismatch"),
            Self::Clock => formatter.write_str("invalid virtual-clock migration state"),
            Self::Snapshot(reason) => write!(formatter, "migration snapshot failed: {reason}"),
            Self::Io(reason) => write!(formatter, "migration transport failed: {reason}"),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<std::io::Error> for MigrationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<VmLifecycleError> for MigrationError {
    fn from(value: VmLifecycleError) -> Self {
        Self::Snapshot(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MigrationPhase {
    Created = 0,
    Negotiated = 1,
    PreCopy = 2,
    StopAndCopy = 3,
    Prepared = 4,
    Committing = 5,
    Committed = 6,
    RolledBack = 7,
    Failed = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationClockPolicy {
    FreezeDuringTransfer,
    AdvanceByDowntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualClockSnapshot {
    pub guest_monotonic_ns: u64,
    pub guest_wall_clock_ns: u64,
    pub counter_frequency_hz: u64,
    pub counter_value: u128,
    pub paused: bool,
    pub accumulated_downtime_ns: u64,
}

impl VirtualClockSnapshot {
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.counter_frequency_hz == 0 {
            return Err(MigrationError::Clock);
        }
        Ok(())
    }

    pub fn restore(
        &self,
        downtime_ns: u64,
        policy: MigrationClockPolicy,
    ) -> Result<Self, MigrationError> {
        self.validate()?;
        let advance = match policy {
            MigrationClockPolicy::FreezeDuringTransfer => 0,
            MigrationClockPolicy::AdvanceByDowntime => downtime_ns,
        };
        let counter_advance = u128::from(advance)
            .checked_mul(u128::from(self.counter_frequency_hz))
            .ok_or(MigrationError::Clock)?
            / 1_000_000_000_u128;
        Ok(Self {
            guest_monotonic_ns: self
                .guest_monotonic_ns
                .checked_add(advance)
                .ok_or(MigrationError::Clock)?,
            guest_wall_clock_ns: self
                .guest_wall_clock_ns
                .checked_add(advance)
                .ok_or(MigrationError::Clock)?,
            counter_frequency_hz: self.counter_frequency_hz,
            counter_value: self
                .counter_value
                .checked_add(counter_advance)
                .ok_or(MigrationError::Clock)?,
            paused: false,
            accumulated_downtime_ns: self
                .accumulated_downtime_ns
                .checked_add(downtime_ns)
                .ok_or(MigrationError::Clock)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSchemaRange {
    pub id: String,
    pub class: VirtualDeviceClass,
    pub minimum_version: u16,
    pub maximum_version: u16,
    pub required: bool,
}

impl DeviceSchemaRange {
    pub fn accepts(&self, device: &DeviceSnapshot) -> bool {
        self.id == device.id
            && self.class == device.class
            && (self.minimum_version..=self.maximum_version).contains(&device.schema_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCapabilities {
    pub architecture: GuestArchitecture,
    pub target: String,
    pub page_size: u32,
    pub memory_len: u64,
    pub maximum_vcpus: u32,
    pub features: u64,
    pub devices: Vec<DeviceSchemaRange>,
}

impl MigrationCapabilities {
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.target.is_empty()
            || self.page_size < 256
            || self.page_size > 2 * 1024 * 1024
            || !self.page_size.is_power_of_two()
            || self.memory_len == 0
            || self.maximum_vcpus == 0
        {
            return Err(MigrationError::IncompatibleMemory);
        }
        let mut ids = BTreeSet::new();
        for device in &self.devices {
            if device.id.is_empty()
                || device.minimum_version == 0
                || device.minimum_version > device.maximum_version
                || !ids.insert(device.id.as_str())
            {
                return Err(MigrationError::DuplicateDevice(device.id.clone()));
            }
        }
        Ok(())
    }

    pub fn negotiate(&self, destination: &Self, vcpu_count: usize) -> Result<u64, MigrationError> {
        self.validate()?;
        destination.validate()?;
        if self.architecture != destination.architecture {
            return Err(MigrationError::IncompatibleArchitecture);
        }
        if self.target != destination.target {
            return Err(MigrationError::IncompatibleTarget);
        }
        if self.page_size != destination.page_size {
            return Err(MigrationError::IncompatiblePageSize);
        }
        if self.memory_len != destination.memory_len {
            return Err(MigrationError::IncompatibleMemory);
        }
        if vcpu_count == 0 || vcpu_count > destination.maximum_vcpus as usize {
            return Err(MigrationError::IncompatibleVcpuCount);
        }
        let required_features = MIGRATION_FEATURE_PRECOPY
            | MIGRATION_FEATURE_DIRTY_BITMAP
            | MIGRATION_FEATURE_DEVICE_VERSIONING
            | MIGRATION_FEATURE_CLOCK_CONTINUITY
            | MIGRATION_FEATURE_TRANSACTIONAL_COMMIT;
        let common = self.features & destination.features;
        if common & required_features != required_features {
            return Err(MigrationError::MissingFeature(required_features & !common));
        }
        for source in self.devices.iter().filter(|device| device.required) {
            let destination = destination
                .devices
                .iter()
                .find(|candidate| candidate.id == source.id && candidate.class == source.class)
                .ok_or_else(|| MigrationError::MissingDevice(source.id.clone()))?;
            if source.minimum_version > destination.maximum_version
                || destination.minimum_version > source.maximum_version
            {
                return Err(MigrationError::DeviceVersion {
                    id: source.id.clone(),
                    version: source.maximum_version,
                });
            }
        }
        Ok(common)
    }

    pub fn validate_devices(&self, devices: &[DeviceSnapshot]) -> Result<(), MigrationError> {
        let mut seen = BTreeSet::new();
        for device in devices {
            device.verify()?;
            if !seen.insert(device.id.as_str()) {
                return Err(MigrationError::DuplicateDevice(device.id.clone()));
            }
            let schema = self
                .devices
                .iter()
                .find(|schema| schema.id == device.id && schema.class == device.class);
            match schema {
                Some(schema) if schema.accepts(device) => {}
                Some(_) => {
                    return Err(MigrationError::DeviceVersion {
                        id: device.id.clone(),
                        version: device.schema_version,
                    })
                }
                None if device.required => {
                    return Err(MigrationError::MissingDevice(device.id.clone()))
                }
                None => {}
            }
        }
        for schema in self.devices.iter().filter(|schema| schema.required) {
            if !seen.contains(schema.id.as_str()) {
                return Err(MigrationError::MissingDevice(schema.id.clone()));
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, MigrationError> {
        self.validate()?;
        let mut writer = BinaryWriter::default();
        writer.u8(self.architecture as u8);
        writer.string(&self.target)?;
        writer.u32(self.page_size);
        writer.u64(self.memory_len);
        writer.u32(self.maximum_vcpus);
        writer.u64(self.features);
        writer.u64(self.devices.len() as u64);
        for device in &self.devices {
            writer.string(&device.id)?;
            writer.u32(device.class as u16 as u32);
            writer.u32(device.minimum_version as u32);
            writer.u32(device.maximum_version as u32);
            writer.u8(device.required as u8);
        }
        Ok(writer.bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MigrationError> {
        let mut reader = BinaryReader::new(bytes);
        let architecture = decode_guest_architecture(reader.u8()?)?;
        let target = reader.string(4096, "target")?;
        let page_size = reader.u32()?;
        let memory_len = reader.u64()?;
        let maximum_vcpus = reader.u32()?;
        let features = reader.u64()?;
        let count = reader.count(4096, "device schemas")?;
        let mut devices = Vec::with_capacity(count);
        for _ in 0..count {
            devices.push(DeviceSchemaRange {
                id: reader.string(4096, "device id")?,
                class: decode_device_class(
                    u16::try_from(reader.u32()?)
                        .map_err(|_| MigrationError::Frame("device class"))?,
                )?,
                minimum_version: u16::try_from(reader.u32()?)
                    .map_err(|_| MigrationError::Frame("device version"))?,
                maximum_version: u16::try_from(reader.u32()?)
                    .map_err(|_| MigrationError::Frame("device version"))?,
                required: reader.u8()? != 0,
            });
        }
        reader.finish()?;
        let capabilities = Self {
            architecture,
            target,
            page_size,
            memory_len,
            maximum_vcpus,
            features,
            devices,
        };
        capabilities.validate()?;
        Ok(capabilities)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationIdentity {
    pub machine_id: SnapshotId,
    pub session_id: SnapshotId,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyPageTracker {
    page_size: usize,
    memory_len: usize,
    generation: u64,
    bits: Vec<u64>,
}

impl DirtyPageTracker {
    pub fn new(
        memory_len: usize,
        page_size: usize,
        initially_dirty: bool,
    ) -> Result<Self, MigrationError> {
        if memory_len == 0
            || page_size < 256
            || page_size > 2 * 1024 * 1024
            || !page_size.is_power_of_two()
        {
            return Err(MigrationError::IncompatiblePageSize);
        }
        let pages = memory_len.div_ceil(page_size);
        let mut bits = vec![if initially_dirty { u64::MAX } else { 0 }; pages.div_ceil(64)];
        if initially_dirty && pages % 64 != 0 {
            let valid = pages % 64;
            *bits.last_mut().unwrap() = (1_u64 << valid) - 1;
        }
        Ok(Self {
            page_size,
            memory_len,
            generation: 0,
            bits,
        })
    }

    pub const fn page_size(&self) -> usize {
        self.page_size
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn page_count(&self) -> usize {
        self.memory_len.div_ceil(self.page_size)
    }

    pub fn dirty_count(&self) -> usize {
        self.bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn mark_page(&mut self, page: u64) -> Result<(), MigrationError> {
        let page = usize::try_from(page).map_err(|_| MigrationError::DirtyPage(page))?;
        if page >= self.page_count() {
            return Err(MigrationError::DirtyPage(page as u64));
        }
        self.bits[page / 64] |= 1_u64 << (page % 64);
        Ok(())
    }

    pub fn mark_range(&mut self, address: u64, length: u64) -> Result<(), MigrationError> {
        if length == 0 {
            return Ok(());
        }
        let end = address
            .checked_add(length)
            .ok_or(MigrationError::IncompatibleMemory)?;
        if end > self.memory_len as u64 {
            return Err(MigrationError::IncompatibleMemory);
        }
        let first = address as usize / self.page_size;
        let last = (end as usize - 1) / self.page_size;
        for page in first..=last {
            self.mark_page(page as u64)?;
        }
        Ok(())
    }

    pub fn ingest_kvm_bitmap(
        &mut self,
        base_page: u64,
        bitmap: &[u64],
    ) -> Result<(), MigrationError> {
        for (word_index, word) in bitmap.iter().copied().enumerate() {
            let mut pending = word;
            while pending != 0 {
                let bit = pending.trailing_zeros() as u64;
                let page = base_page
                    .checked_add(word_index as u64 * 64)
                    .and_then(|page| page.checked_add(bit))
                    .ok_or(MigrationError::DirtyBitmap)?;
                self.mark_page(page)?;
                pending &= pending - 1;
            }
        }
        Ok(())
    }

    pub fn ingest_dirty_gfns(
        &mut self,
        slot_base_gfn: u64,
        gfns: &[u64],
    ) -> Result<(), MigrationError> {
        for gfn in gfns {
            let page = slot_base_gfn
                .checked_add(*gfn)
                .ok_or(MigrationError::DirtyBitmap)?;
            self.mark_page(page)?;
        }
        Ok(())
    }

    pub fn capture_round(
        &mut self,
        memory: &[u8],
        sequence: u64,
    ) -> Result<MigrationMemoryRound, MigrationError> {
        if memory.len() != self.memory_len {
            return Err(MigrationError::IncompatibleMemory);
        }
        let full = self.dirty_count() == self.page_count();
        let mut pages = Vec::with_capacity(self.dirty_count());
        for page in 0..self.page_count() {
            if self.bits[page / 64] & (1_u64 << (page % 64)) == 0 {
                continue;
            }
            let start = page * self.page_size;
            let end = (start + self.page_size).min(memory.len());
            let data = memory[start..end].to_vec();
            pages.push(MemoryPageSnapshot {
                index: page as u64,
                checksum: crc32(&data),
                data,
            });
        }
        self.bits.fill(0);
        self.generation = self.generation.wrapping_add(1);
        Ok(MigrationMemoryRound {
            sequence,
            generation: self.generation,
            full,
            page_size: self.page_size as u32,
            memory_len: self.memory_len as u64,
            image_checksum: crc32(memory),
            pages,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationMemoryRound {
    pub sequence: u64,
    pub generation: u64,
    pub full: bool,
    pub page_size: u32,
    pub memory_len: u64,
    pub image_checksum: u32,
    pub pages: Vec<MemoryPageSnapshot>,
}

impl MigrationMemoryRound {
    pub fn apply(&self, memory: &mut Vec<u8>) -> Result<(), MigrationError> {
        let memory_len =
            usize::try_from(self.memory_len).map_err(|_| MigrationError::Limit("memory"))?;
        if self.full {
            memory.clear();
            memory.resize(memory_len, 0);
        } else if memory.len() != memory_len {
            return Err(MigrationError::IncompatibleMemory);
        }
        let page_size = self.page_size as usize;
        if page_size < 256 || !page_size.is_power_of_two() {
            return Err(MigrationError::IncompatiblePageSize);
        }
        let mut previous = None;
        for page in &self.pages {
            if previous.is_some_and(|value| page.index <= value)
                || crc32(&page.data) != page.checksum
            {
                return Err(MigrationError::DirtyPage(page.index));
            }
            previous = Some(page.index);
            let start = usize::try_from(page.index)
                .ok()
                .and_then(|page| page.checked_mul(page_size))
                .ok_or(MigrationError::DirtyPage(page.index))?;
            let end = start
                .checked_add(page.data.len())
                .ok_or(MigrationError::DirtyPage(page.index))?;
            if page.data.is_empty()
                || page.data.len() > page_size
                || end > memory.len()
                || (end != memory.len() && page.data.len() != page_size)
            {
                return Err(MigrationError::DirtyPage(page.index));
            }
            memory[start..end].copy_from_slice(&page.data);
        }
        let actual = crc32(memory);
        if actual != self.image_checksum {
            return Err(MigrationError::Checksum {
                expected: self.image_checksum,
                actual,
            });
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, MigrationError> {
        let mut writer = BinaryWriter::default();
        writer.u64(self.sequence);
        writer.u64(self.generation);
        writer.u8(self.full as u8);
        writer.raw(&[0; 3]);
        writer.u32(self.page_size);
        writer.u64(self.memory_len);
        writer.u32(self.image_checksum);
        writer.u64(self.pages.len() as u64);
        for page in &self.pages {
            writer.u64(page.index);
            writer.u32(page.checksum);
            writer.blob(&page.data)?;
        }
        Ok(writer.bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MigrationError> {
        let mut reader = BinaryReader::new(bytes);
        let sequence = reader.u64()?;
        let generation = reader.u64()?;
        let full = reader.u8()? != 0;
        reader.take(3)?;
        let page_size = reader.u32()?;
        let memory_len = reader.u64()?;
        let image_checksum = reader.u32()?;
        let count = reader.count(4 * 1024 * 1024, "page count")?;
        let mut pages = Vec::with_capacity(count);
        for _ in 0..count {
            pages.push(MemoryPageSnapshot {
                index: reader.u64()?,
                checksum: reader.u32()?,
                data: reader.blob(page_size as usize)?,
            });
        }
        reader.finish()?;
        Ok(Self {
            sequence,
            generation,
            full,
            page_size,
            memory_len,
            image_checksum,
            pages,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MigrationFrameKind {
    Hello = 1,
    Accept = 2,
    MemoryRound = 3,
    StopAndCopy = 4,
    Prepared = 5,
    Commit = 6,
    CommitAck = 7,
    Abort = 8,
    Heartbeat = 9,
}

impl MigrationFrameKind {
    fn from_u8(value: u8) -> Result<Self, MigrationError> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Accept),
            3 => Ok(Self::MemoryRound),
            4 => Ok(Self::StopAndCopy),
            5 => Ok(Self::Prepared),
            6 => Ok(Self::Commit),
            7 => Ok(Self::CommitAck),
            8 => Ok(Self::Abort),
            9 => Ok(Self::Heartbeat),
            _ => Err(MigrationError::Frame("unknown frame kind")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationFrame {
    pub kind: MigrationFrameKind,
    pub flags: u8,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

impl MigrationFrame {
    pub fn encode(&self) -> Result<Vec<u8>, MigrationError> {
        if self.payload.len() > VM_MIGRATION_MAX_FRAME_BYTES {
            return Err(MigrationError::Limit("frame payload"));
        }
        let mut bytes = Vec::with_capacity(36 + self.payload.len());
        bytes.extend_from_slice(&VM_MIGRATION_MAGIC);
        bytes.extend_from_slice(&VM_MIGRATION_VERSION.to_le_bytes());
        bytes.push(self.kind as u8);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes.extend_from_slice(&(self.payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&crc32(&self.payload).to_le_bytes());
        let header_checksum = crc32(&bytes);
        bytes.extend_from_slice(&header_checksum.to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MigrationError> {
        if bytes.len() < 36 || bytes.get(..8) != Some(&VM_MIGRATION_MAGIC) {
            return Err(MigrationError::Frame("bad magic or truncated header"));
        }
        let version = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        if version != VM_MIGRATION_VERSION {
            return Err(MigrationError::Frame("unsupported version"));
        }
        let kind = MigrationFrameKind::from_u8(bytes[10])?;
        let flags = bytes[11];
        let sequence = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        let payload_len = usize::try_from(u64::from_le_bytes(bytes[20..28].try_into().unwrap()))
            .map_err(|_| MigrationError::Limit("frame payload"))?;
        if payload_len > VM_MIGRATION_MAX_FRAME_BYTES || bytes.len() != 36 + payload_len {
            return Err(MigrationError::Frame("invalid payload length"));
        }
        let payload_checksum = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        let header_checksum = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        let actual_header = crc32(&bytes[..32]);
        if header_checksum != actual_header {
            return Err(MigrationError::Checksum {
                expected: header_checksum,
                actual: actual_header,
            });
        }
        let payload = bytes[36..].to_vec();
        let actual_payload = crc32(&payload);
        if payload_checksum != actual_payload {
            return Err(MigrationError::Checksum {
                expected: payload_checksum,
                actual: actual_payload,
            });
        }
        Ok(Self {
            kind,
            flags,
            sequence,
            payload,
        })
    }

    pub fn write_to(&self, mut writer: impl Write) -> Result<(), MigrationError> {
        let encoded = self.encode()?;
        writer.write_all(&(encoded.len() as u64).to_le_bytes())?;
        writer.write_all(&encoded)?;
        writer.flush()?;
        Ok(())
    }

    pub fn read_from(mut reader: impl Read) -> Result<Self, MigrationError> {
        let mut length = [0_u8; 8];
        reader.read_exact(&mut length)?;
        let length = usize::try_from(u64::from_le_bytes(length))
            .map_err(|_| MigrationError::Limit("framed message"))?;
        if length < 36 || length > VM_MIGRATION_MAX_FRAME_BYTES + 36 {
            return Err(MigrationError::Limit("framed message"));
        }
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes)?;
        Self::decode(&bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationFinalState {
    pub identity: MigrationIdentity,
    pub snapshot: VmSnapshot,
    pub clock: VirtualClockSnapshot,
    pub downtime_started_ns: u64,
    pub commit_token: u32,
}

impl MigrationFinalState {
    pub fn encode(&self) -> Result<Vec<u8>, MigrationError> {
        self.verify()?;
        let snapshot = self.snapshot.encode()?;
        let mut writer = BinaryWriter::default();
        writer.raw(&self.identity.machine_id.0);
        writer.raw(&self.identity.session_id.0);
        writer.u64(self.identity.generation);
        writer.u64(self.clock.guest_monotonic_ns);
        writer.u64(self.clock.guest_wall_clock_ns);
        writer.u64(self.clock.counter_frequency_hz);
        writer.u128(self.clock.counter_value);
        writer.u8(self.clock.paused as u8);
        writer.raw(&[0; 7]);
        writer.u64(self.clock.accumulated_downtime_ns);
        writer.u64(self.downtime_started_ns);
        writer.u32(self.commit_token);
        writer.blob(&snapshot)?;
        Ok(writer.bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, MigrationError> {
        let mut reader = BinaryReader::new(bytes);
        let machine_id = SnapshotId(reader.take(16)?.try_into().unwrap());
        let session_id = SnapshotId(reader.take(16)?.try_into().unwrap());
        let generation = reader.u64()?;
        let clock = VirtualClockSnapshot {
            guest_monotonic_ns: reader.u64()?,
            guest_wall_clock_ns: reader.u64()?,
            counter_frequency_hz: reader.u64()?,
            counter_value: reader.u128()?,
            paused: reader.u8()? != 0,
            accumulated_downtime_ns: {
                reader.take(7)?;
                reader.u64()?
            },
        };
        let downtime_started_ns = reader.u64()?;
        let commit_token = reader.u32()?;
        let snapshot = VmSnapshot::decode(
            &reader.blob(VM_MIGRATION_MAX_FRAME_BYTES)?,
            crate::SnapshotLimits::default(),
        )?;
        reader.finish()?;
        let state = Self {
            identity: MigrationIdentity {
                machine_id,
                session_id,
                generation,
            },
            snapshot,
            clock,
            downtime_started_ns,
            commit_token,
        };
        state.verify()?;
        Ok(state)
    }

    pub fn verify(&self) -> Result<(), MigrationError> {
        validate_identity(self.identity)?;
        self.clock.validate()?;
        self.snapshot.verify()?;
        if !self.clock.paused
            || self.snapshot.state != VmPowerState::Paused
            || self.snapshot.generation != self.identity.generation
            || self
                .snapshot
                .vcpus
                .iter()
                .any(|vcpu| vcpu.virtual_time_ns != self.clock.guest_monotonic_ns)
        {
            return Err(MigrationError::Clock);
        }
        self.verify_token()
    }

    pub fn verify_token(&self) -> Result<(), MigrationError> {
        let expected = migration_commit_token(self.identity, &self.snapshot, &self.clock)?;
        if self.commit_token != expected {
            return Err(MigrationError::CommitToken);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveMigrationSource {
    pub identity: MigrationIdentity,
    pub capabilities: MigrationCapabilities,
    pub phase: MigrationPhase,
    pub negotiated_features: u64,
    pub memory: Vec<u8>,
    pub vcpus: Vec<VcpuSnapshot>,
    pub devices: Vec<DeviceSnapshot>,
    pub clock: VirtualClockSnapshot,
    tracker: DirtyPageTracker,
    replica_memory: Vec<u8>,
    next_sequence: u64,
    prepared_token: Option<u32>,
}

impl LiveMigrationSource {
    pub fn new(
        identity: MigrationIdentity,
        capabilities: MigrationCapabilities,
        memory: Vec<u8>,
        vcpus: Vec<VcpuSnapshot>,
        devices: Vec<DeviceSnapshot>,
        clock: VirtualClockSnapshot,
    ) -> Result<Self, MigrationError> {
        capabilities.validate()?;
        validate_identity(identity)?;
        validate_runtime_state(&capabilities, &memory, &vcpus, &devices, &clock)?;
        Ok(Self {
            identity,
            tracker: DirtyPageTracker::new(memory.len(), capabilities.page_size as usize, true)?,
            replica_memory: vec![0; memory.len()],
            capabilities,
            phase: MigrationPhase::Created,
            negotiated_features: 0,
            memory,
            vcpus,
            devices,
            clock,
            next_sequence: 1,
            prepared_token: None,
        })
    }

    pub fn negotiate(
        &mut self,
        destination: &MigrationCapabilities,
    ) -> Result<u64, MigrationError> {
        require_phase(self.phase, MigrationPhase::Created, "Created")?;
        let features = self.capabilities.negotiate(destination, self.vcpus.len())?;
        self.negotiated_features = features;
        self.phase = MigrationPhase::Negotiated;
        Ok(features)
    }

    pub fn mark_dirty(&mut self, address: u64, length: u64) -> Result<(), MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::Created | MigrationPhase::Negotiated | MigrationPhase::PreCopy
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "Created, Negotiated or PreCopy",
                actual: self.phase,
            });
        }
        self.tracker.mark_range(address, length)
    }

    pub fn ingest_kvm_dirty_bitmap(
        &mut self,
        base_page: u64,
        bitmap: &[u64],
    ) -> Result<(), MigrationError> {
        self.require_dirty_tracking_phase()?;
        self.tracker.ingest_kvm_bitmap(base_page, bitmap)
    }

    pub fn ingest_dirty_ring_gfns(
        &mut self,
        slot_base_gfn: u64,
        gfns: &[u64],
    ) -> Result<(), MigrationError> {
        self.require_dirty_tracking_phase()?;
        self.tracker.ingest_dirty_gfns(slot_base_gfn, gfns)
    }

    pub fn dirty_page_count(&self) -> usize {
        self.tracker.dirty_count()
    }

    pub fn dirty_generation(&self) -> u64 {
        self.tracker.generation()
    }

    fn require_dirty_tracking_phase(&self) -> Result<(), MigrationError> {
        if matches!(
            self.phase,
            MigrationPhase::Created | MigrationPhase::Negotiated | MigrationPhase::PreCopy
        ) {
            Ok(())
        } else {
            Err(MigrationError::InvalidPhase {
                expected: "Created, Negotiated or PreCopy",
                actual: self.phase,
            })
        }
    }

    pub fn write_memory(&mut self, address: usize, bytes: &[u8]) -> Result<(), MigrationError> {
        let end = address
            .checked_add(bytes.len())
            .ok_or(MigrationError::IncompatibleMemory)?;
        self.memory
            .get_mut(address..end)
            .ok_or(MigrationError::IncompatibleMemory)?
            .copy_from_slice(bytes);
        self.mark_dirty(address as u64, bytes.len() as u64)
    }

    pub fn pre_copy_round(&mut self) -> Result<MigrationMemoryRound, MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::Negotiated | MigrationPhase::PreCopy
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "Negotiated or PreCopy",
                actual: self.phase,
            });
        }
        self.phase = MigrationPhase::PreCopy;
        let round = self
            .tracker
            .capture_round(&self.memory, self.next_sequence)?;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        round.apply(&mut self.replica_memory)?;
        Ok(round)
    }

    pub fn stop_and_copy(
        &mut self,
        host_monotonic_ns: u64,
    ) -> Result<MigrationFinalState, MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::PreCopy | MigrationPhase::Negotiated
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "PreCopy",
                actual: self.phase,
            });
        }
        self.phase = MigrationPhase::StopAndCopy;
        self.clock.paused = true;
        for vcpu in &mut self.vcpus {
            vcpu.virtual_time_ns = self.clock.guest_monotonic_ns;
        }
        validate_runtime_state(
            &self.capabilities,
            &self.memory,
            &self.vcpus,
            &self.devices,
            &self.clock,
        )?;
        let memory = MemorySnapshot::capture_incremental(
            &self.replica_memory,
            &self.memory,
            self.capabilities.page_size as usize,
        )?;
        let snapshot = VmSnapshot {
            id: migration_snapshot_id(self.identity),
            parent: SnapshotId::NIL,
            generation: self.identity.generation,
            timestamp_ns: host_monotonic_ns,
            state: VmPowerState::Paused,
            architecture: self.capabilities.architecture,
            target: self.capabilities.target.clone(),
            vcpus: self.vcpus.clone(),
            memory,
            devices: self.devices.clone(),
            metadata: BTreeMap::from([
                (
                    "migration.protocol".into(),
                    VM_MIGRATION_VERSION.to_string(),
                ),
                ("migration.session".into(), hex_id(self.identity.session_id)),
                ("migration.phase".into(), "stop-and-copy".into()),
                ("migration.vcpus".into(), self.vcpus.len().to_string()),
            ]),
        };
        snapshot.verify()?;
        let token = migration_commit_token(self.identity, &snapshot, &self.clock)?;
        let final_state = MigrationFinalState {
            identity: self.identity,
            snapshot,
            clock: self.clock.clone(),
            downtime_started_ns: host_monotonic_ns,
            commit_token: token,
        };
        final_state.verify()?;
        self.prepared_token = Some(token);
        self.phase = MigrationPhase::Prepared;
        Ok(final_state)
    }

    pub fn prepare_commit(&mut self, token: u32) -> Result<(), MigrationError> {
        require_phase(self.phase, MigrationPhase::Prepared, "Prepared")?;
        if self.prepared_token != Some(token) {
            return Err(MigrationError::CommitToken);
        }
        self.phase = MigrationPhase::Committing;
        Ok(())
    }

    pub fn finish_commit(&mut self, token: u32) -> Result<(), MigrationError> {
        require_phase(self.phase, MigrationPhase::Committing, "Committing")?;
        if self.prepared_token != Some(token) {
            return Err(MigrationError::CommitToken);
        }
        self.phase = MigrationPhase::Committed;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::Negotiated
                | MigrationPhase::PreCopy
                | MigrationPhase::StopAndCopy
                | MigrationPhase::Prepared
                | MigrationPhase::Committing
                | MigrationPhase::Failed
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "active migration",
                actual: self.phase,
            });
        }
        self.clock.paused = false;
        self.prepared_token = None;
        self.phase = MigrationPhase::RolledBack;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoredVmState {
    pub identity: MigrationIdentity,
    pub memory: Vec<u8>,
    pub vcpus: Vec<VcpuSnapshot>,
    pub devices: Vec<DeviceSnapshot>,
    pub clock: VirtualClockSnapshot,
    pub commit_token: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveMigrationDestination {
    pub capabilities: MigrationCapabilities,
    pub phase: MigrationPhase,
    pub negotiated_features: u64,
    pub memory: Vec<u8>,
    pub expected_sequence: u64,
    staged: Option<RestoredVmState>,
}

impl LiveMigrationDestination {
    pub fn new(capabilities: MigrationCapabilities) -> Result<Self, MigrationError> {
        capabilities.validate()?;
        Ok(Self {
            memory: Vec::new(),
            capabilities,
            phase: MigrationPhase::Created,
            negotiated_features: 0,
            expected_sequence: 1,
            staged: None,
        })
    }

    pub fn accept(
        &mut self,
        source: &MigrationCapabilities,
        vcpu_count: usize,
    ) -> Result<u64, MigrationError> {
        require_phase(self.phase, MigrationPhase::Created, "Created")?;
        let features = source.negotiate(&self.capabilities, vcpu_count)?;
        self.negotiated_features = features;
        self.phase = MigrationPhase::Negotiated;
        Ok(features)
    }

    pub fn apply_round(&mut self, round: &MigrationMemoryRound) -> Result<(), MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::Negotiated | MigrationPhase::PreCopy
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "Negotiated or PreCopy",
                actual: self.phase,
            });
        }
        if round.sequence != self.expected_sequence {
            return Err(MigrationError::Sequence {
                expected: self.expected_sequence,
                actual: round.sequence,
            });
        }
        if round.page_size != self.capabilities.page_size
            || round.memory_len != self.capabilities.memory_len
        {
            return Err(MigrationError::IncompatibleMemory);
        }
        round.apply(&mut self.memory)?;
        self.expected_sequence = self.expected_sequence.wrapping_add(1);
        self.phase = MigrationPhase::PreCopy;
        Ok(())
    }

    pub fn stage_final(
        &mut self,
        final_state: &MigrationFinalState,
        destination_host_ns: u64,
        policy: MigrationClockPolicy,
    ) -> Result<u32, MigrationError> {
        if !matches!(
            self.phase,
            MigrationPhase::Negotiated | MigrationPhase::PreCopy
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "PreCopy",
                actual: self.phase,
            });
        }
        final_state.verify()?;
        final_state
            .snapshot
            .assert_compatible(self.capabilities.architecture, &self.capabilities.target)?;
        self.capabilities
            .validate_devices(&final_state.snapshot.devices)?;
        validate_vcpus(
            self.capabilities.architecture,
            &final_state.snapshot.vcpus,
            self.capabilities.maximum_vcpus,
        )?;
        let memory = final_state.snapshot.restore_memory(Some(&self.memory))?;
        let downtime = destination_host_ns
            .checked_sub(final_state.downtime_started_ns)
            .ok_or(MigrationError::Clock)?;
        let clock = final_state.clock.restore(downtime, policy)?;
        self.staged = Some(RestoredVmState {
            identity: final_state.identity,
            memory,
            vcpus: final_state.snapshot.vcpus.clone(),
            devices: final_state.snapshot.devices.clone(),
            clock,
            commit_token: final_state.commit_token,
        });
        self.phase = MigrationPhase::Prepared;
        Ok(final_state.commit_token)
    }

    pub fn prepare_commit(&mut self, token: u32) -> Result<(), MigrationError> {
        require_phase(self.phase, MigrationPhase::Prepared, "Prepared")?;
        if self.staged.as_ref().map(|state| state.commit_token) != Some(token) {
            return Err(MigrationError::CommitToken);
        }
        self.phase = MigrationPhase::Committing;
        Ok(())
    }

    pub fn commit(&mut self, token: u32) -> Result<RestoredVmState, MigrationError> {
        require_phase(self.phase, MigrationPhase::Committing, "Committing")?;
        let state = self.staged.take().ok_or(MigrationError::CommitToken)?;
        if state.commit_token != token {
            self.staged = Some(state);
            return Err(MigrationError::CommitToken);
        }
        self.memory = state.memory.clone();
        self.phase = MigrationPhase::Committed;
        Ok(state)
    }

    pub fn rollback(&mut self) -> Result<(), MigrationError> {
        if matches!(
            self.phase,
            MigrationPhase::Committed | MigrationPhase::RolledBack
        ) {
            return Err(MigrationError::InvalidPhase {
                expected: "uncommitted migration",
                actual: self.phase,
            });
        }
        self.staged = None;
        self.memory.clear();
        self.phase = MigrationPhase::RolledBack;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationPolicy {
    pub maximum_pre_copy_rounds: u32,
    pub stop_dirty_pages: usize,
    pub clock_policy: MigrationClockPolicy,
}

impl Default for MigrationPolicy {
    fn default() -> Self {
        Self {
            maximum_pre_copy_rounds: 8,
            stop_dirty_pages: 64,
            clock_policy: MigrationClockPolicy::AdvanceByDowntime,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub rounds: u32,
    pub pages_transferred: u64,
    pub bytes_transferred: u64,
    pub downtime_ns: u64,
    pub commit_token: u32,
    pub source_phase: MigrationPhase,
    pub destination_phase: MigrationPhase,
}

pub fn migrate_transactionally(
    source: &mut LiveMigrationSource,
    destination: &mut LiveMigrationDestination,
    policy: MigrationPolicy,
    source_stop_ns: u64,
    destination_resume_ns: u64,
) -> Result<(RestoredVmState, MigrationReport), MigrationError> {
    let result = (|| {
        let source_features = source.negotiate(&destination.capabilities)?;
        let destination_features = destination.accept(&source.capabilities, source.vcpus.len())?;
        if source_features != destination_features {
            return Err(MigrationError::MissingFeature(
                source_features ^ destination_features,
            ));
        }
        let mut rounds = 0_u32;
        let mut pages = 0_u64;
        let mut bytes = 0_u64;
        for _ in 0..policy.maximum_pre_copy_rounds.max(1) {
            let round = source.pre_copy_round()?;
            pages += round.pages.len() as u64;
            bytes += round
                .pages
                .iter()
                .map(|page| page.data.len() as u64)
                .sum::<u64>();
            destination.apply_round(&round)?;
            rounds += 1;
            if source.tracker.dirty_count() <= policy.stop_dirty_pages && !round.full {
                break;
            }
            if source.tracker.dirty_count() == 0 {
                break;
            }
        }
        let final_state = source.stop_and_copy(source_stop_ns)?;
        pages += final_state.snapshot.memory.pages.len() as u64;
        bytes += final_state
            .snapshot
            .memory
            .pages
            .iter()
            .map(|page| page.data.len() as u64)
            .sum::<u64>();
        let token =
            destination.stage_final(&final_state, destination_resume_ns, policy.clock_policy)?;
        source.prepare_commit(token)?;
        destination.prepare_commit(token)?;
        let restored = destination.commit(token)?;
        source.finish_commit(token)?;
        let report = MigrationReport {
            rounds,
            pages_transferred: pages,
            bytes_transferred: bytes,
            downtime_ns: destination_resume_ns - source_stop_ns,
            commit_token: token,
            source_phase: source.phase,
            destination_phase: destination.phase,
        };
        Ok((restored, report))
    })();
    if result.is_err() {
        if !matches!(
            source.phase,
            MigrationPhase::Committed | MigrationPhase::RolledBack
        ) {
            source.phase = MigrationPhase::Failed;
            let _ = source.rollback();
        }
        if !matches!(
            destination.phase,
            MigrationPhase::Committed | MigrationPhase::RolledBack
        ) {
            destination.phase = MigrationPhase::Failed;
            let _ = destination.rollback();
        }
    }
    result
}

pub fn migrate_source_over_stream<S: Read + Write>(
    source: &mut LiveMigrationSource,
    stream: &mut S,
    policy: MigrationPolicy,
    source_stop_ns: u64,
) -> Result<MigrationReport, MigrationError> {
    migrate_source_over_stream_with_hook(source, stream, policy, source_stop_ns, |_, _| Ok(()))
}

pub fn migrate_source_over_stream_with_hook<S: Read + Write>(
    source: &mut LiveMigrationSource,
    stream: &mut S,
    policy: MigrationPolicy,
    source_stop_ns: u64,
    mut after_round: impl FnMut(&mut LiveMigrationSource, u32) -> Result<(), MigrationError>,
) -> Result<MigrationReport, MigrationError> {
    let result = (|| {
        MigrationFrame {
            kind: MigrationFrameKind::Hello,
            flags: 0,
            sequence: 0,
            payload: encode_hello(&source.capabilities, source.vcpus.len())?,
        }
        .write_to(&mut *stream)?;
        let accept = MigrationFrame::read_from(&mut *stream)?;
        if accept.kind != MigrationFrameKind::Accept || accept.sequence != 0 {
            return Err(MigrationError::Frame("expected migration accept"));
        }
        let (destination_capabilities, accepted_features) = decode_accept(&accept.payload)?;
        let source_features = source.negotiate(&destination_capabilities)?;
        if source_features != accepted_features {
            return Err(MigrationError::MissingFeature(
                source_features ^ accepted_features,
            ));
        }

        let mut rounds = 0_u32;
        let mut pages = 0_u64;
        let mut bytes = 0_u64;
        for _ in 0..policy.maximum_pre_copy_rounds.max(1) {
            let round = source.pre_copy_round()?;
            let round_pages = round.pages.len() as u64;
            let round_bytes = round
                .pages
                .iter()
                .map(|page| page.data.len() as u64)
                .sum::<u64>();
            MigrationFrame {
                kind: MigrationFrameKind::MemoryRound,
                flags: round.full as u8,
                sequence: round.sequence,
                payload: round.encode()?,
            }
            .write_to(&mut *stream)?;
            expect_empty_ack(stream, round.sequence)?;
            rounds += 1;
            pages += round_pages;
            bytes += round_bytes;
            after_round(source, rounds)?;
            if source.tracker.dirty_count() <= policy.stop_dirty_pages && !round.full {
                break;
            }
            if source.tracker.dirty_count() == 0 {
                break;
            }
        }

        let final_state = source.stop_and_copy(source_stop_ns)?;
        pages += final_state.snapshot.memory.pages.len() as u64;
        bytes += final_state
            .snapshot
            .memory
            .pages
            .iter()
            .map(|page| page.data.len() as u64)
            .sum::<u64>();
        let final_sequence = u64::from(rounds) + 1;
        MigrationFrame {
            kind: MigrationFrameKind::StopAndCopy,
            flags: 0,
            sequence: final_sequence,
            payload: final_state.encode()?,
        }
        .write_to(&mut *stream)?;
        let prepared = MigrationFrame::read_from(&mut *stream)?;
        let (prepared_token, destination_resume_ns) = decode_prepared(&prepared.payload)?;
        if prepared.kind != MigrationFrameKind::Prepared
            || prepared.sequence != final_sequence
            || prepared_token != final_state.commit_token
        {
            return Err(MigrationError::CommitToken);
        }
        source.prepare_commit(final_state.commit_token)?;
        MigrationFrame {
            kind: MigrationFrameKind::Commit,
            flags: 0,
            sequence: final_sequence + 1,
            payload: final_state.commit_token.to_le_bytes().to_vec(),
        }
        .write_to(&mut *stream)?;
        let committed = MigrationFrame::read_from(&mut *stream)?;
        if committed.kind != MigrationFrameKind::CommitAck
            || committed.sequence != final_sequence + 1
            || decode_token(&committed.payload)? != final_state.commit_token
        {
            return Err(MigrationError::CommitToken);
        }
        source.finish_commit(final_state.commit_token)?;
        Ok(MigrationReport {
            rounds,
            pages_transferred: pages,
            bytes_transferred: bytes,
            downtime_ns: destination_resume_ns
                .checked_sub(source_stop_ns)
                .ok_or(MigrationError::Clock)?,
            commit_token: final_state.commit_token,
            source_phase: source.phase,
            destination_phase: MigrationPhase::Committed,
        })
    })();
    if result.is_err()
        && !matches!(
            source.phase,
            MigrationPhase::Committed | MigrationPhase::RolledBack
        )
    {
        let _ = MigrationFrame {
            kind: MigrationFrameKind::Abort,
            flags: 0,
            sequence: 0,
            payload: Vec::new(),
        }
        .write_to(&mut *stream);
        source.phase = MigrationPhase::Failed;
        let _ = source.rollback();
    }
    result
}

pub fn migrate_destination_over_stream<S: Read + Write>(
    destination: &mut LiveMigrationDestination,
    stream: &mut S,
    destination_resume_ns: u64,
    clock_policy: MigrationClockPolicy,
) -> Result<RestoredVmState, MigrationError> {
    let result = (|| {
        let hello = MigrationFrame::read_from(&mut *stream)?;
        if hello.kind != MigrationFrameKind::Hello || hello.sequence != 0 {
            return Err(MigrationError::Frame("expected migration hello"));
        }
        let (source_capabilities, source_vcpus) = decode_hello(&hello.payload)?;
        let accepted_features = destination.accept(&source_capabilities, source_vcpus)?;
        MigrationFrame {
            kind: MigrationFrameKind::Accept,
            flags: 0,
            sequence: 0,
            payload: encode_accept(&destination.capabilities, accepted_features)?,
        }
        .write_to(&mut *stream)?;

        loop {
            let frame = MigrationFrame::read_from(&mut *stream)?;
            match frame.kind {
                MigrationFrameKind::MemoryRound => {
                    let round = MigrationMemoryRound::decode(&frame.payload)?;
                    if round.sequence != frame.sequence {
                        return Err(MigrationError::Sequence {
                            expected: frame.sequence,
                            actual: round.sequence,
                        });
                    }
                    destination.apply_round(&round)?;
                    MigrationFrame {
                        kind: MigrationFrameKind::Accept,
                        flags: 0,
                        sequence: frame.sequence,
                        payload: Vec::new(),
                    }
                    .write_to(&mut *stream)?;
                }
                MigrationFrameKind::StopAndCopy => {
                    let final_state = MigrationFinalState::decode(&frame.payload)?;
                    let token = destination.stage_final(
                        &final_state,
                        destination_resume_ns,
                        clock_policy,
                    )?;
                    MigrationFrame {
                        kind: MigrationFrameKind::Prepared,
                        flags: 0,
                        sequence: frame.sequence,
                        payload: encode_prepared(token, destination_resume_ns),
                    }
                    .write_to(&mut *stream)?;
                    let commit = MigrationFrame::read_from(&mut *stream)?;
                    if commit.kind != MigrationFrameKind::Commit
                        || commit.sequence != frame.sequence + 1
                        || decode_token(&commit.payload)? != token
                    {
                        return Err(MigrationError::CommitToken);
                    }
                    destination.prepare_commit(token)?;
                    let restored = destination.commit(token)?;
                    MigrationFrame {
                        kind: MigrationFrameKind::CommitAck,
                        flags: 0,
                        sequence: commit.sequence,
                        payload: token.to_le_bytes().to_vec(),
                    }
                    .write_to(&mut *stream)?;
                    return Ok(restored);
                }
                MigrationFrameKind::Heartbeat => {
                    MigrationFrame {
                        kind: MigrationFrameKind::Accept,
                        flags: 0,
                        sequence: frame.sequence,
                        payload: Vec::new(),
                    }
                    .write_to(&mut *stream)?;
                }
                MigrationFrameKind::Abort => {
                    return Err(MigrationError::Frame("source aborted migration"));
                }
                _ => return Err(MigrationError::Frame("unexpected migration frame")),
            }
        }
    })();
    if result.is_err()
        && !matches!(
            destination.phase,
            MigrationPhase::Committed | MigrationPhase::RolledBack
        )
    {
        destination.phase = MigrationPhase::Failed;
        let _ = destination.rollback();
    }
    result
}

fn encode_accept(
    capabilities: &MigrationCapabilities,
    features: u64,
) -> Result<Vec<u8>, MigrationError> {
    let mut writer = BinaryWriter::default();
    writer.u64(features);
    writer.blob(&capabilities.encode()?)?;
    Ok(writer.bytes)
}

fn encode_hello(
    capabilities: &MigrationCapabilities,
    vcpu_count: usize,
) -> Result<Vec<u8>, MigrationError> {
    if vcpu_count == 0 || vcpu_count > u32::MAX as usize {
        return Err(MigrationError::IncompatibleVcpuCount);
    }
    let mut writer = BinaryWriter::default();
    writer.u32(vcpu_count as u32);
    writer.blob(&capabilities.encode()?)?;
    Ok(writer.bytes)
}

fn decode_hello(bytes: &[u8]) -> Result<(MigrationCapabilities, usize), MigrationError> {
    let mut reader = BinaryReader::new(bytes);
    let vcpu_count = reader.u32()? as usize;
    if vcpu_count == 0 {
        return Err(MigrationError::IncompatibleVcpuCount);
    }
    let capabilities = MigrationCapabilities::decode(&reader.blob(VM_MIGRATION_MAX_FRAME_BYTES)?)?;
    reader.finish()?;
    Ok((capabilities, vcpu_count))
}

fn decode_accept(bytes: &[u8]) -> Result<(MigrationCapabilities, u64), MigrationError> {
    let mut reader = BinaryReader::new(bytes);
    let features = reader.u64()?;
    let capabilities = MigrationCapabilities::decode(&reader.blob(VM_MIGRATION_MAX_FRAME_BYTES)?)?;
    reader.finish()?;
    Ok((capabilities, features))
}

fn decode_token(bytes: &[u8]) -> Result<u32, MigrationError> {
    if bytes.len() != 4 {
        return Err(MigrationError::Frame("invalid commit token"));
    }
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn encode_prepared(token: u32, destination_resume_ns: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&token.to_le_bytes());
    bytes.extend_from_slice(&destination_resume_ns.to_le_bytes());
    bytes
}

fn decode_prepared(bytes: &[u8]) -> Result<(u32, u64), MigrationError> {
    if bytes.len() != 12 {
        return Err(MigrationError::Frame("invalid prepared state"));
    }
    Ok((
        u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        u64::from_le_bytes(bytes[4..].try_into().unwrap()),
    ))
}

fn expect_empty_ack(stream: &mut (impl Read + Write), sequence: u64) -> Result<(), MigrationError> {
    let frame = MigrationFrame::read_from(stream)?;
    if frame.kind != MigrationFrameKind::Accept
        || frame.sequence != sequence
        || !frame.payload.is_empty()
    {
        return Err(MigrationError::Frame("invalid round acknowledgement"));
    }
    Ok(())
}

fn decode_guest_architecture(value: u8) -> Result<GuestArchitecture, MigrationError> {
    match value {
        1 => Ok(GuestArchitecture::X86_64),
        2 => Ok(GuestArchitecture::I386),
        3 => Ok(GuestArchitecture::Aarch64),
        4 => Ok(GuestArchitecture::RiscV64),
        5 => Ok(GuestArchitecture::RiscV32),
        _ => Err(MigrationError::Frame("unknown guest architecture")),
    }
}

fn decode_device_class(value: u16) -> Result<VirtualDeviceClass, MigrationError> {
    match value {
        1 => Ok(VirtualDeviceClass::Uart),
        2 => Ok(VirtualDeviceClass::Ps2),
        3 => Ok(VirtualDeviceClass::Block),
        4 => Ok(VirtualDeviceClass::Network),
        5 => Ok(VirtualDeviceClass::Framebuffer),
        6 => Ok(VirtualDeviceClass::PciBus),
        7 => Ok(VirtualDeviceClass::VirtioTransport),
        8 => Ok(VirtualDeviceClass::VirtioBlock),
        9 => Ok(VirtualDeviceClass::VirtioNetwork),
        10 => Ok(VirtualDeviceClass::VirtioConsole),
        11 => Ok(VirtualDeviceClass::VirtioInput),
        12 => Ok(VirtualDeviceClass::VirtioGpu),
        13 => Ok(VirtualDeviceClass::Apic),
        14 => Ok(VirtualDeviceClass::IoApic),
        15 => Ok(VirtualDeviceClass::Timer),
        16 => Ok(VirtualDeviceClass::Firmware),
        0xffff => Ok(VirtualDeviceClass::Opaque),
        _ => Err(MigrationError::Frame("unknown device class")),
    }
}

fn validate_runtime_state(
    capabilities: &MigrationCapabilities,
    memory: &[u8],
    vcpus: &[VcpuSnapshot],
    devices: &[DeviceSnapshot],
    clock: &VirtualClockSnapshot,
) -> Result<(), MigrationError> {
    if memory.len() as u64 != capabilities.memory_len {
        return Err(MigrationError::IncompatibleMemory);
    }
    validate_vcpus(capabilities.architecture, vcpus, capabilities.maximum_vcpus)?;
    capabilities.validate_devices(devices)?;
    clock.validate()?;
    Ok(())
}

fn validate_identity(identity: MigrationIdentity) -> Result<(), MigrationError> {
    if identity.machine_id == SnapshotId::NIL || identity.session_id == SnapshotId::NIL {
        return Err(MigrationError::InvalidIdentity);
    }
    Ok(())
}

fn migration_snapshot_id(identity: MigrationIdentity) -> SnapshotId {
    let mut bytes = identity.machine_id.0;
    let generation = identity.generation.to_le_bytes();
    for index in 0..8 {
        bytes[index] ^= identity.session_id.0[index];
        bytes[index + 8] ^= generation[index];
    }
    if bytes == SnapshotId::NIL.0 {
        bytes[0] = 1;
    }
    SnapshotId(bytes)
}

fn validate_vcpus(
    architecture: GuestArchitecture,
    vcpus: &[VcpuSnapshot],
    maximum_vcpus: u32,
) -> Result<(), MigrationError> {
    if vcpus.is_empty() || vcpus.len() > maximum_vcpus as usize {
        return Err(MigrationError::IncompatibleVcpuCount);
    }
    let mut ids = BTreeSet::new();
    for vcpu in vcpus {
        if vcpu.registers.architecture != architecture {
            return Err(MigrationError::IncompatibleArchitecture);
        }
        if !ids.insert(vcpu.id) {
            return Err(MigrationError::DuplicateVcpu(vcpu.id));
        }
    }
    Ok(())
}

fn migration_commit_token(
    identity: MigrationIdentity,
    snapshot: &VmSnapshot,
    clock: &VirtualClockSnapshot,
) -> Result<u32, MigrationError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&identity.machine_id.0);
    bytes.extend_from_slice(&identity.session_id.0);
    bytes.extend_from_slice(&identity.generation.to_le_bytes());
    bytes.extend_from_slice(&clock.guest_monotonic_ns.to_le_bytes());
    bytes.extend_from_slice(&clock.counter_value.to_le_bytes());
    bytes.extend_from_slice(&snapshot.encode()?);
    Ok(crc32(&bytes))
}

fn require_phase(
    actual: MigrationPhase,
    expected: MigrationPhase,
    expected_name: &'static str,
) -> Result<(), MigrationError> {
    if actual != expected {
        return Err(MigrationError::InvalidPhase {
            expected: expected_name,
            actual,
        });
    }
    Ok(())
}

fn hex_id(id: SnapshotId) -> String {
    let mut text = String::with_capacity(32);
    for byte in id.0 {
        use fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[derive(Default)]
struct BinaryWriter {
    bytes: Vec<u8>,
}

impl BinaryWriter {
    fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.raw(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.raw(&value.to_le_bytes());
    }

    fn blob(&mut self, value: &[u8]) -> Result<(), MigrationError> {
        if value.len() > VM_MIGRATION_MAX_FRAME_BYTES {
            return Err(MigrationError::Limit("blob"));
        }
        self.u64(value.len() as u64);
        self.raw(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), MigrationError> {
        self.blob(value.as_bytes())
    }
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], MigrationError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(MigrationError::Frame("offset overflow"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(MigrationError::Frame("truncated payload"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, MigrationError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, MigrationError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, MigrationError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn u128(&mut self) -> Result<u128, MigrationError> {
        Ok(u128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }

    fn count(&mut self, limit: usize, label: &'static str) -> Result<usize, MigrationError> {
        let value = usize::try_from(self.u64()?).map_err(|_| MigrationError::Limit(label))?;
        if value > limit {
            return Err(MigrationError::Limit(label));
        }
        Ok(value)
    }

    fn blob(&mut self, limit: usize) -> Result<Vec<u8>, MigrationError> {
        let length = self.count(limit, "blob")?;
        Ok(self.take(length)?.to_vec())
    }

    fn string(&mut self, limit: usize, label: &'static str) -> Result<String, MigrationError> {
        String::from_utf8(self.blob(limit)?).map_err(|_| MigrationError::Frame(label))
    }

    fn finish(&self) -> Result<(), MigrationError> {
        if self.offset != self.bytes.len() {
            return Err(MigrationError::Frame("trailing payload"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisterBank;
    use std::io::Cursor;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    fn capabilities() -> MigrationCapabilities {
        MigrationCapabilities {
            architecture: GuestArchitecture::X86_64,
            target: "x86_64-unknown-linux-gnu".into(),
            page_size: 4096,
            memory_len: 4 * 4096,
            maximum_vcpus: 8,
            features: MIGRATION_FEATURE_PRECOPY
                | MIGRATION_FEATURE_DIRTY_BITMAP
                | MIGRATION_FEATURE_DIRTY_RING
                | MIGRATION_FEATURE_DEVICE_VERSIONING
                | MIGRATION_FEATURE_CLOCK_CONTINUITY
                | MIGRATION_FEATURE_TRANSACTIONAL_COMMIT,
            devices: vec![DeviceSchemaRange {
                id: "virtio-blk0".into(),
                class: VirtualDeviceClass::VirtioBlock,
                minimum_version: 1,
                maximum_version: 2,
                required: true,
            }],
        }
    }

    fn vcpus() -> Vec<VcpuSnapshot> {
        (0..4)
            .map(|id| {
                let mut registers = RegisterBank::new(GuestArchitecture::X86_64);
                registers.set("rip", 0x1000 + u128::from(id));
                registers.set("rsp", 0x8000 - u128::from(id) * 0x100);
                VcpuSnapshot {
                    id,
                    registers,
                    halted: id != 0,
                    pending_interrupts: vec![32 + id],
                    virtual_time_ns: 5_000,
                }
            })
            .collect()
    }

    fn source() -> LiveMigrationSource {
        LiveMigrationSource::new(
            MigrationIdentity {
                machine_id: SnapshotId::from_sequence(7, 1),
                session_id: SnapshotId::from_sequence(7, 99),
                generation: 12,
            },
            capabilities(),
            (0..4 * 4096).map(|index| (index % 251) as u8).collect(),
            vcpus(),
            vec![DeviceSnapshot::new(
                "virtio-blk0",
                VirtualDeviceClass::VirtioBlock,
                2,
                true,
                vec![1, 2, 3, 4],
            )],
            VirtualClockSnapshot {
                guest_monotonic_ns: 5_000,
                guest_wall_clock_ns: 1_700_000_000_000_000_000,
                counter_frequency_hz: 1_000_000_000,
                counter_value: 5_000,
                paused: false,
                accumulated_downtime_ns: 0,
            },
        )
        .unwrap()
    }

    #[test]
    fn dirty_tracker_ingests_bitmap_and_dirty_ring_without_losing_pages() {
        let mut tracker = DirtyPageTracker::new(8 * 4096, 4096, false).unwrap();
        tracker.ingest_kvm_bitmap(0, &[0b1000_0101]).unwrap();
        tracker.ingest_dirty_gfns(0, &[1, 6]).unwrap();
        assert_eq!(tracker.dirty_count(), 5);
        let memory = vec![0xa5; 8 * 4096];
        let round = tracker.capture_round(&memory, 1).unwrap();
        assert_eq!(
            round
                .pages
                .iter()
                .map(|page| page.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 6, 7]
        );
        let encoded = round.encode().unwrap();
        assert_eq!(MigrationMemoryRound::decode(&encoded).unwrap(), round);
    }

    #[test]
    fn live_source_ingests_host_dirty_logs_and_tracks_generations() {
        let mut source = source();
        source.negotiate(&capabilities()).unwrap();
        source.pre_copy_round().unwrap();
        assert_eq!(source.dirty_page_count(), 0);
        assert_eq!(source.dirty_generation(), 1);
        source.ingest_kvm_dirty_bitmap(0, &[0b0010]).unwrap();
        source.ingest_dirty_ring_gfns(0, &[3]).unwrap();
        assert_eq!(source.dirty_page_count(), 2);
        let round = source.pre_copy_round().unwrap();
        assert_eq!(round.generation, 2);
        assert_eq!(
            round
                .pages
                .iter()
                .map(|page| page.index)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn migration_rejects_nil_identity_duplicate_vcpus_and_unpaused_final_state() {
        let mut nil_identity = MigrationIdentity {
            machine_id: SnapshotId::NIL,
            session_id: SnapshotId::from_sequence(1, 2),
            generation: 1,
        };
        assert_eq!(
            LiveMigrationSource::new(
                nil_identity,
                capabilities(),
                vec![0; 4 * 4096],
                vcpus(),
                vec![DeviceSnapshot::new(
                    "virtio-blk0",
                    VirtualDeviceClass::VirtioBlock,
                    2,
                    true,
                    vec![1],
                )],
                VirtualClockSnapshot {
                    guest_monotonic_ns: 1,
                    guest_wall_clock_ns: 1,
                    counter_frequency_hz: 1_000_000_000,
                    counter_value: 1,
                    paused: false,
                    accumulated_downtime_ns: 0,
                },
            )
            .unwrap_err(),
            MigrationError::InvalidIdentity
        );

        nil_identity.machine_id = SnapshotId::from_sequence(1, 1);
        let mut duplicate_vcpus = vcpus();
        duplicate_vcpus.push(duplicate_vcpus[0].clone());
        assert!(matches!(
            LiveMigrationSource::new(
                nil_identity,
                capabilities(),
                vec![0; 4 * 4096],
                duplicate_vcpus,
                vec![DeviceSnapshot::new(
                    "virtio-blk0",
                    VirtualDeviceClass::VirtioBlock,
                    2,
                    true,
                    vec![1],
                )],
                VirtualClockSnapshot {
                    guest_monotonic_ns: 1,
                    guest_wall_clock_ns: 1,
                    counter_frequency_hz: 1_000_000_000,
                    counter_value: 1,
                    paused: false,
                    accumulated_downtime_ns: 0,
                },
            ),
            Err(MigrationError::DuplicateVcpu(0))
        ));

        let mut source = source();
        source.negotiate(&capabilities()).unwrap();
        source.pre_copy_round().unwrap();
        let mut final_state = source.stop_and_copy(500).unwrap();
        final_state.clock.paused = false;
        assert_eq!(final_state.verify().unwrap_err(), MigrationError::Clock);
    }

    #[test]
    fn frame_stream_roundtrip_rejects_corruption() {
        let frame = MigrationFrame {
            kind: MigrationFrameKind::MemoryRound,
            flags: 1,
            sequence: 9,
            payload: vec![7; 8192],
        };
        let mut wire = Vec::new();
        frame.write_to(&mut wire).unwrap();
        assert_eq!(
            MigrationFrame::read_from(Cursor::new(&wire)).unwrap(),
            frame
        );
        *wire.last_mut().unwrap() ^= 1;
        assert!(matches!(
            MigrationFrame::read_from(Cursor::new(&wire)),
            Err(MigrationError::Checksum { .. })
        ));
    }

    #[test]
    fn capability_manifest_roundtrip_is_canonical_and_bounded() {
        let value = capabilities();
        let encoded = value.encode().unwrap();
        assert_eq!(MigrationCapabilities::decode(&encoded).unwrap(), value);
        let mut corrupt = encoded;
        corrupt[0] = 0xff;
        assert!(matches!(
            MigrationCapabilities::decode(&corrupt),
            Err(MigrationError::Frame("unknown guest architecture"))
        ));
    }

    #[test]
    fn tcp_live_migration_runs_precopy_stop_commit_and_clock_continuity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let destination_thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut destination = LiveMigrationDestination::new(capabilities()).unwrap();
            let restored = migrate_destination_over_stream(
                &mut destination,
                &mut stream,
                20_000,
                MigrationClockPolicy::AdvanceByDowntime,
            )
            .unwrap();
            (destination.phase, restored)
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.set_nodelay(true).unwrap();
        let mut source = source();
        let report = migrate_source_over_stream_with_hook(
            &mut source,
            &mut stream,
            MigrationPolicy {
                maximum_pre_copy_rounds: 6,
                stop_dirty_pages: 0,
                clock_policy: MigrationClockPolicy::AdvanceByDowntime,
            },
            10_000,
            |source, round| {
                match round {
                    1 => source.write_memory(4096 + 17, &[0xaa, 0xbb])?,
                    2 => source.write_memory(8192 + 31, &[0xcc, 0xdd])?,
                    _ => {}
                }
                Ok(())
            },
        )
        .unwrap();
        let (destination_phase, restored) = destination_thread.join().unwrap();
        assert_eq!(source.phase, MigrationPhase::Committed);
        assert_eq!(destination_phase, MigrationPhase::Committed);
        assert_eq!(report.rounds, 3);
        assert_eq!(report.downtime_ns, 10_000);
        assert_eq!(&restored.memory[4096 + 17..4096 + 19], &[0xaa, 0xbb]);
        assert_eq!(&restored.memory[8192 + 31..8192 + 33], &[0xcc, 0xdd]);
        assert_eq!(restored.vcpus.len(), 4);
        assert_eq!(restored.clock.guest_monotonic_ns, 15_000);
        assert_eq!(restored.clock.accumulated_downtime_ns, 10_000);
    }

    #[test]
    fn four_vcpu_live_migration_preserves_memory_devices_interrupts_and_time() {
        let mut source = source();
        let expected_memory = {
            let mut memory = source.memory.clone();
            memory[5000..5004].copy_from_slice(&[9, 8, 7, 6]);
            memory
        };
        source.write_memory(5000, &[9, 8, 7, 6]).unwrap();
        let mut destination = LiveMigrationDestination::new(capabilities()).unwrap();
        let (restored, report) = migrate_transactionally(
            &mut source,
            &mut destination,
            MigrationPolicy::default(),
            10_000,
            10_750,
        )
        .unwrap();
        assert_eq!(restored.memory, expected_memory);
        assert_eq!(restored.vcpus.len(), 4);
        assert_eq!(restored.vcpus[3].pending_interrupts, vec![35]);
        assert_eq!(restored.devices[0].schema_version, 2);
        assert_eq!(restored.clock.guest_monotonic_ns, 5_750);
        assert_eq!(source.phase, MigrationPhase::Committed);
        assert_eq!(destination.phase, MigrationPhase::Committed);
        assert!(report.pages_transferred >= 4);
        assert_eq!(report.downtime_ns, 750);
    }

    #[test]
    fn failed_destination_schema_rolls_source_back_without_split_brain() {
        let mut source = source();
        let mut incompatible = capabilities();
        incompatible.devices[0].minimum_version = 3;
        incompatible.devices[0].maximum_version = 4;
        let mut destination = LiveMigrationDestination::new(incompatible).unwrap();
        assert!(migrate_transactionally(
            &mut source,
            &mut destination,
            MigrationPolicy::default(),
            10,
            20,
        )
        .is_err());
        assert_eq!(source.phase, MigrationPhase::RolledBack);
        assert_eq!(destination.phase, MigrationPhase::RolledBack);
        assert!(!source.clock.paused);
    }

    #[test]
    fn final_state_binary_roundtrip_retains_snapshot_and_commit_token() {
        let mut source = source();
        source.negotiate(&capabilities()).unwrap();
        source.pre_copy_round().unwrap();
        let final_state = source.stop_and_copy(1234).unwrap();
        let bytes = final_state.encode().unwrap();
        assert_eq!(MigrationFinalState::decode(&bytes).unwrap(), final_state);
    }
}
