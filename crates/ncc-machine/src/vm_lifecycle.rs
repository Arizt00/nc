use crate::crc32;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const VM_SNAPSHOT_MAGIC: [u8; 8] = *b"NCSNAP02";
pub const VM_SNAPSHOT_VERSION: u16 = 2;
pub const VM_SNAPSHOT_ENDIAN_LITTLE: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmLifecycleError {
    InvalidTransition {
        from: VmPowerState,
        to: VmPowerState,
    },
    SnapshotFormat(&'static str),
    SnapshotLimit(&'static str),
    SnapshotChecksum {
        expected: u32,
        actual: u32,
    },
    SnapshotBaseMismatch,
    SnapshotArchitectureMismatch,
    SnapshotTargetMismatch,
    InvalidPageSize,
    InvalidMemoryPage,
    DuplicateVcpu(u32),
    DuplicateDevice(String),
    DeviceChecksum(String),
    DebugAddress,
    BreakpointExists(u64),
    BreakpointMissing(u64),
    HardwareSlot(u8),
    WatchpointLength,
    Io(String),
}

impl fmt::Display for VmLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(
                    formatter,
                    "invalid VM lifecycle transition {from:?} -> {to:?}"
                )
            }
            Self::SnapshotFormat(reason) => write!(formatter, "invalid VM snapshot: {reason}"),
            Self::SnapshotLimit(reason) => write!(formatter, "VM snapshot exceeds limit: {reason}"),
            Self::SnapshotChecksum { expected, actual } => write!(
                formatter,
                "VM snapshot checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
            ),
            Self::SnapshotBaseMismatch => formatter.write_str("incremental snapshot base mismatch"),
            Self::SnapshotArchitectureMismatch => {
                formatter.write_str("snapshot architecture does not match the VM")
            }
            Self::SnapshotTargetMismatch => {
                formatter.write_str("snapshot target does not match the VM")
            }
            Self::InvalidPageSize => formatter.write_str("invalid snapshot page size"),
            Self::InvalidMemoryPage => formatter.write_str("invalid snapshot memory page"),
            Self::DuplicateVcpu(id) => write!(formatter, "duplicate snapshot vCPU {id}"),
            Self::DuplicateDevice(id) => write!(formatter, "duplicate snapshot device '{id}'"),
            Self::DeviceChecksum(id) => write!(formatter, "snapshot device '{id}' is corrupted"),
            Self::DebugAddress => formatter.write_str("debug memory address is outside the target"),
            Self::BreakpointExists(address) => {
                write!(formatter, "breakpoint already exists at {address:#x}")
            }
            Self::BreakpointMissing(address) => {
                write!(formatter, "breakpoint does not exist at {address:#x}")
            }
            Self::HardwareSlot(slot) => {
                write!(formatter, "invalid or occupied hardware slot {slot}")
            }
            Self::WatchpointLength => {
                formatter.write_str("watchpoint length must be 1, 2, 4, or 8")
            }
            Self::Io(message) => write!(formatter, "snapshot I/O failed: {message}"),
        }
    }
}

impl Error for VmLifecycleError {}

impl From<std::io::Error> for VmLifecycleError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmPowerState {
    Created = 0,
    Running = 1,
    Paused = 2,
    Suspended = 3,
    Resetting = 4,
    Stopped = 5,
    Crashed = 6,
}

impl VmPowerState {
    fn from_u8(value: u8) -> Result<Self, VmLifecycleError> {
        match value {
            0 => Ok(Self::Created),
            1 => Ok(Self::Running),
            2 => Ok(Self::Paused),
            3 => Ok(Self::Suspended),
            4 => Ok(Self::Resetting),
            5 => Ok(Self::Stopped),
            6 => Ok(Self::Crashed),
            _ => Err(VmLifecycleError::SnapshotFormat("unknown power state")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmResetKind {
    Warm,
    Cold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmLifecycleEvent {
    pub sequence: u64,
    pub from: VmPowerState,
    pub to: VmPowerState,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmLifecycle {
    state: VmPowerState,
    sequence: u64,
    boot_count: u64,
    reset_count: u64,
    pending_reset: Option<VmResetKind>,
    events: VecDeque<VmLifecycleEvent>,
    event_capacity: usize,
}

impl Default for VmLifecycle {
    fn default() -> Self {
        Self::new(256)
    }
}

impl VmLifecycle {
    pub fn new(event_capacity: usize) -> Self {
        Self {
            state: VmPowerState::Created,
            sequence: 0,
            boot_count: 0,
            reset_count: 0,
            pending_reset: None,
            events: VecDeque::with_capacity(event_capacity.min(4096)),
            event_capacity,
        }
    }

    pub const fn state(&self) -> VmPowerState {
        self.state
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn boot_count(&self) -> u64 {
        self.boot_count
    }

    pub const fn reset_count(&self) -> u64 {
        self.reset_count
    }

    pub const fn pending_reset(&self) -> Option<VmResetKind> {
        self.pending_reset
    }

    pub fn events(&self) -> impl Iterator<Item = &VmLifecycleEvent> {
        self.events.iter()
    }

    fn transition(
        &mut self,
        target: VmPowerState,
        reason: impl Into<String>,
    ) -> Result<(), VmLifecycleError> {
        let valid = matches!(
            (self.state, target),
            (VmPowerState::Created, VmPowerState::Running)
                | (VmPowerState::Created, VmPowerState::Stopped)
                | (VmPowerState::Running, VmPowerState::Paused)
                | (VmPowerState::Running, VmPowerState::Suspended)
                | (VmPowerState::Running, VmPowerState::Resetting)
                | (VmPowerState::Running, VmPowerState::Stopped)
                | (VmPowerState::Running, VmPowerState::Crashed)
                | (VmPowerState::Paused, VmPowerState::Running)
                | (VmPowerState::Paused, VmPowerState::Suspended)
                | (VmPowerState::Paused, VmPowerState::Resetting)
                | (VmPowerState::Paused, VmPowerState::Stopped)
                | (VmPowerState::Paused, VmPowerState::Crashed)
                | (VmPowerState::Suspended, VmPowerState::Running)
                | (VmPowerState::Suspended, VmPowerState::Paused)
                | (VmPowerState::Suspended, VmPowerState::Resetting)
                | (VmPowerState::Suspended, VmPowerState::Stopped)
                | (VmPowerState::Resetting, VmPowerState::Running)
                | (VmPowerState::Resetting, VmPowerState::Paused)
                | (VmPowerState::Resetting, VmPowerState::Stopped)
                | (VmPowerState::Stopped, VmPowerState::Running)
                | (VmPowerState::Crashed, VmPowerState::Resetting)
                | (VmPowerState::Crashed, VmPowerState::Stopped)
        );
        if !valid {
            return Err(VmLifecycleError::InvalidTransition {
                from: self.state,
                to: target,
            });
        }
        let from = self.state;
        self.state = target;
        self.sequence = self.sequence.wrapping_add(1);
        if self.event_capacity != 0 {
            while self.events.len() >= self.event_capacity {
                self.events.pop_front();
            }
            self.events.push_back(VmLifecycleEvent {
                sequence: self.sequence,
                from,
                to: target,
                reason: reason.into(),
            });
        }
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), VmLifecycleError> {
        if !matches!(self.state, VmPowerState::Created | VmPowerState::Stopped) {
            return Err(VmLifecycleError::InvalidTransition {
                from: self.state,
                to: VmPowerState::Running,
            });
        }
        self.transition(VmPowerState::Running, "start")?;
        self.boot_count = self.boot_count.wrapping_add(1);
        Ok(())
    }

    pub fn pause(&mut self, reason: impl Into<String>) -> Result<(), VmLifecycleError> {
        self.transition(VmPowerState::Paused, reason)
    }

    pub fn resume(&mut self) -> Result<(), VmLifecycleError> {
        self.transition(VmPowerState::Running, "resume")
    }

    pub fn suspend(&mut self) -> Result<(), VmLifecycleError> {
        self.transition(VmPowerState::Suspended, "suspend")
    }

    pub fn wake(&mut self, paused: bool) -> Result<(), VmLifecycleError> {
        self.transition(
            if paused {
                VmPowerState::Paused
            } else {
                VmPowerState::Running
            },
            "wake",
        )
    }

    pub fn begin_reset(&mut self, kind: VmResetKind) -> Result<(), VmLifecycleError> {
        self.transition(VmPowerState::Resetting, format!("reset:{kind:?}"))?;
        self.pending_reset = Some(kind);
        Ok(())
    }

    pub fn complete_reset(&mut self, paused: bool) -> Result<(), VmLifecycleError> {
        if self.pending_reset.is_none() {
            return Err(VmLifecycleError::InvalidTransition {
                from: self.state,
                to: if paused {
                    VmPowerState::Paused
                } else {
                    VmPowerState::Running
                },
            });
        }
        self.reset_count = self.reset_count.wrapping_add(1);
        self.pending_reset = None;
        self.transition(
            if paused {
                VmPowerState::Paused
            } else {
                VmPowerState::Running
            },
            "reset-complete",
        )
    }

    pub fn stop(&mut self, reason: impl Into<String>) -> Result<(), VmLifecycleError> {
        self.pending_reset = None;
        self.transition(VmPowerState::Stopped, reason)
    }

    pub fn crash(&mut self, reason: impl Into<String>) -> Result<(), VmLifecycleError> {
        self.pending_reset = None;
        self.transition(VmPowerState::Crashed, reason)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GuestArchitecture {
    X86_64 = 1,
    I386 = 2,
    Aarch64 = 3,
    RiscV64 = 4,
    RiscV32 = 5,
}

impl GuestArchitecture {
    fn from_u8(value: u8) -> Result<Self, VmLifecycleError> {
        match value {
            1 => Ok(Self::X86_64),
            2 => Ok(Self::I386),
            3 => Ok(Self::Aarch64),
            4 => Ok(Self::RiscV64),
            5 => Ok(Self::RiscV32),
            _ => Err(VmLifecycleError::SnapshotFormat("unknown architecture")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterBank {
    pub architecture: GuestArchitecture,
    pub values: BTreeMap<String, u128>,
}

impl RegisterBank {
    pub fn new(architecture: GuestArchitecture) -> Self {
        Self {
            architecture,
            values: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, name: impl Into<String>, value: u128) {
        self.values.insert(name.into(), value);
    }

    pub fn get(&self, name: &str) -> Option<u128> {
        self.values.get(name).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcpuSnapshot {
    pub id: u32,
    pub registers: RegisterBank,
    pub halted: bool,
    pub pending_interrupts: Vec<u32>,
    pub virtual_time_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SnapshotKind {
    Full = 0,
    Incremental = 1,
}

impl SnapshotKind {
    fn from_u8(value: u8) -> Result<Self, VmLifecycleError> {
        match value {
            0 => Ok(Self::Full),
            1 => Ok(Self::Incremental),
            _ => Err(VmLifecycleError::SnapshotFormat("unknown snapshot kind")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPageSnapshot {
    pub index: u64,
    pub data: Vec<u8>,
    pub checksum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub kind: SnapshotKind,
    pub page_size: u32,
    pub memory_len: u64,
    pub base_checksum: u32,
    pub image_checksum: u32,
    pub pages: Vec<MemoryPageSnapshot>,
}

impl MemorySnapshot {
    fn validate_page_size(page_size: usize) -> Result<(), VmLifecycleError> {
        if page_size < 256 || page_size > 2 * 1024 * 1024 || !page_size.is_power_of_two() {
            return Err(VmLifecycleError::InvalidPageSize);
        }
        Ok(())
    }

    pub fn capture_full(memory: &[u8], page_size: usize) -> Result<Self, VmLifecycleError> {
        Self::validate_page_size(page_size)?;
        let pages = memory
            .chunks(page_size)
            .enumerate()
            .map(|(index, data)| MemoryPageSnapshot {
                index: index as u64,
                data: data.to_vec(),
                checksum: crc32(data),
            })
            .collect();
        Ok(Self {
            kind: SnapshotKind::Full,
            page_size: page_size as u32,
            memory_len: memory.len() as u64,
            base_checksum: 0,
            image_checksum: crc32(memory),
            pages,
        })
    }

    pub fn capture_incremental(
        base: &[u8],
        memory: &[u8],
        page_size: usize,
    ) -> Result<Self, VmLifecycleError> {
        Self::validate_page_size(page_size)?;
        if base.len() != memory.len() {
            return Err(VmLifecycleError::SnapshotBaseMismatch);
        }
        let pages = memory
            .chunks(page_size)
            .zip(base.chunks(page_size))
            .enumerate()
            .filter_map(|(index, (current, previous))| {
                (current != previous).then(|| MemoryPageSnapshot {
                    index: index as u64,
                    data: current.to_vec(),
                    checksum: crc32(current),
                })
            })
            .collect();
        Ok(Self {
            kind: SnapshotKind::Incremental,
            page_size: page_size as u32,
            memory_len: memory.len() as u64,
            base_checksum: crc32(base),
            image_checksum: crc32(memory),
            pages,
        })
    }

    pub fn restore(&self, base: Option<&[u8]>) -> Result<Vec<u8>, VmLifecycleError> {
        let memory_len = self.verify_structure()?;
        let mut memory = match self.kind {
            SnapshotKind::Full => vec![0; memory_len],
            SnapshotKind::Incremental => {
                let base = base.ok_or(VmLifecycleError::SnapshotBaseMismatch)?;
                if base.len() != memory_len || crc32(base) != self.base_checksum {
                    return Err(VmLifecycleError::SnapshotBaseMismatch);
                }
                base.to_vec()
            }
        };
        let page_size = self.page_size as usize;
        for page in &self.pages {
            let start = usize::try_from(page.index)
                .ok()
                .and_then(|index| index.checked_mul(page_size))
                .ok_or(VmLifecycleError::InvalidMemoryPage)?;
            let end = start
                .checked_add(page.data.len())
                .ok_or(VmLifecycleError::InvalidMemoryPage)?;
            memory[start..end].copy_from_slice(&page.data);
        }
        if crc32(&memory) != self.image_checksum {
            return Err(VmLifecycleError::SnapshotChecksum {
                expected: self.image_checksum,
                actual: crc32(&memory),
            });
        }
        Ok(memory)
    }

    fn verify_structure(&self) -> Result<usize, VmLifecycleError> {
        let page_size = self.page_size as usize;
        Self::validate_page_size(page_size)?;
        let memory_len = usize::try_from(self.memory_len)
            .map_err(|_| VmLifecycleError::SnapshotLimit("memory length"))?;
        if memory_len == 0 {
            return Err(VmLifecycleError::SnapshotFormat(
                "snapshot memory cannot be empty",
            ));
        }
        let page_count = memory_len
            .checked_add(page_size - 1)
            .ok_or(VmLifecycleError::SnapshotLimit("memory page count"))?
            / page_size;
        if self.kind == SnapshotKind::Full {
            if self.base_checksum != 0 || self.pages.len() != page_count {
                return Err(VmLifecycleError::InvalidMemoryPage);
            }
        }
        let mut previous = None;
        for (position, page) in self.pages.iter().enumerate() {
            if previous.is_some_and(|value| page.index <= value) {
                return Err(VmLifecycleError::InvalidMemoryPage);
            }
            previous = Some(page.index);
            let index =
                usize::try_from(page.index).map_err(|_| VmLifecycleError::InvalidMemoryPage)?;
            if index >= page_count || (self.kind == SnapshotKind::Full && index != position) {
                return Err(VmLifecycleError::InvalidMemoryPage);
            }
            let start = index
                .checked_mul(page_size)
                .ok_or(VmLifecycleError::InvalidMemoryPage)?;
            let expected_len = page_size.min(memory_len - start);
            if page.data.len() != expected_len || crc32(&page.data) != page.checksum {
                return Err(VmLifecycleError::InvalidMemoryPage);
            }
        }
        Ok(memory_len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum VirtualDeviceClass {
    Uart = 1,
    Ps2 = 2,
    Block = 3,
    Network = 4,
    Framebuffer = 5,
    PciBus = 6,
    VirtioTransport = 7,
    VirtioBlock = 8,
    VirtioNetwork = 9,
    VirtioConsole = 10,
    VirtioInput = 11,
    VirtioGpu = 12,
    Apic = 13,
    IoApic = 14,
    Timer = 15,
    Firmware = 16,
    Opaque = 0xffff,
}

impl VirtualDeviceClass {
    fn from_u16(value: u16) -> Result<Self, VmLifecycleError> {
        match value {
            1 => Ok(Self::Uart),
            2 => Ok(Self::Ps2),
            3 => Ok(Self::Block),
            4 => Ok(Self::Network),
            5 => Ok(Self::Framebuffer),
            6 => Ok(Self::PciBus),
            7 => Ok(Self::VirtioTransport),
            8 => Ok(Self::VirtioBlock),
            9 => Ok(Self::VirtioNetwork),
            10 => Ok(Self::VirtioConsole),
            11 => Ok(Self::VirtioInput),
            12 => Ok(Self::VirtioGpu),
            13 => Ok(Self::Apic),
            14 => Ok(Self::IoApic),
            15 => Ok(Self::Timer),
            16 => Ok(Self::Firmware),
            0xffff => Ok(Self::Opaque),
            _ => Err(VmLifecycleError::SnapshotFormat("unknown device class")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub id: String,
    pub class: VirtualDeviceClass,
    pub schema_version: u16,
    pub required: bool,
    pub data: Vec<u8>,
    pub checksum: u32,
}

impl DeviceSnapshot {
    pub fn new(
        id: impl Into<String>,
        class: VirtualDeviceClass,
        schema_version: u16,
        required: bool,
        data: Vec<u8>,
    ) -> Self {
        let checksum = crc32(&data);
        Self {
            id: id.into(),
            class,
            schema_version,
            required,
            data,
            checksum,
        }
    }

    pub fn verify(&self) -> Result<(), VmLifecycleError> {
        if self.id.trim().is_empty() {
            return Err(VmLifecycleError::SnapshotFormat("empty device id"));
        }
        if self.schema_version == 0 {
            return Err(VmLifecycleError::SnapshotFormat(
                "device schema version must be nonzero",
            ));
        }
        if crc32(&self.data) != self.checksum {
            return Err(VmLifecycleError::DeviceChecksum(self.id.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BreakpointArchitecture {
    X86 = 1,
    Aarch64 = 2,
    RiscV = 3,
}

impl BreakpointArchitecture {
    pub const fn instruction(self) -> &'static [u8] {
        match self {
            Self::X86 => &[0xcc],
            Self::Aarch64 => &[0x00, 0x00, 0x20, 0xd4],
            Self::RiscV => &[0x73, 0x00, 0x10, 0x00],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BreakpointKind {
    Software = 1,
    Execute = 2,
    Read = 3,
    Write = 4,
    ReadWrite = 5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breakpoint {
    pub id: u32,
    pub address: u64,
    pub length: u8,
    pub kind: BreakpointKind,
    pub enabled: bool,
    pub hardware_slot: Option<u8>,
    pub original_bytes: Vec<u8>,
    pub hit_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugStopReason {
    SoftwareBreakpoint { id: u32, address: u64 },
    HardwareBreakpoint { id: u32, address: u64 },
    Watchpoint { id: u32, address: u64, write: bool },
    SingleStep { vcpu: u32, address: u64 },
    Exception { vector: u8, error_code: Option<u32> },
    Interrupt { vector: u8 },
    GuestShutdown,
    GuestCrash(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugEvent {
    pub sequence: u64,
    pub vcpu: u32,
    pub reason: DebugStopReason,
}

pub trait DebugMemory {
    fn read_debug_memory(&self, address: u64, output: &mut [u8]) -> Result<(), VmLifecycleError>;
    fn write_debug_memory(&mut self, address: u64, input: &[u8]) -> Result<(), VmLifecycleError>;
}

impl DebugMemory for [u8] {
    fn read_debug_memory(&self, address: u64, output: &mut [u8]) -> Result<(), VmLifecycleError> {
        let start = usize::try_from(address).map_err(|_| VmLifecycleError::DebugAddress)?;
        let end = start
            .checked_add(output.len())
            .filter(|end| *end <= self.len())
            .ok_or(VmLifecycleError::DebugAddress)?;
        output.copy_from_slice(&self[start..end]);
        Ok(())
    }

    fn write_debug_memory(&mut self, address: u64, input: &[u8]) -> Result<(), VmLifecycleError> {
        let start = usize::try_from(address).map_err(|_| VmLifecycleError::DebugAddress)?;
        let end = start
            .checked_add(input.len())
            .filter(|end| *end <= self.len())
            .ok_or(VmLifecycleError::DebugAddress)?;
        self[start..end].copy_from_slice(input);
        Ok(())
    }
}

impl DebugMemory for Vec<u8> {
    fn read_debug_memory(&self, address: u64, output: &mut [u8]) -> Result<(), VmLifecycleError> {
        self.as_slice().read_debug_memory(address, output)
    }

    fn write_debug_memory(&mut self, address: u64, input: &[u8]) -> Result<(), VmLifecycleError> {
        self.as_mut_slice().write_debug_memory(address, input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmDebugger {
    architecture: BreakpointArchitecture,
    next_id: u32,
    breakpoints: BTreeMap<u32, Breakpoint>,
    by_address: BTreeMap<u64, u32>,
    pending_reinsert: Option<u32>,
    single_step: bool,
    sequence: u64,
    events: VecDeque<DebugEvent>,
    event_capacity: usize,
}

impl VmDebugger {
    pub fn new(architecture: BreakpointArchitecture, event_capacity: usize) -> Self {
        Self {
            architecture,
            next_id: 1,
            breakpoints: BTreeMap::new(),
            by_address: BTreeMap::new(),
            pending_reinsert: None,
            single_step: false,
            sequence: 0,
            events: VecDeque::with_capacity(event_capacity.min(4096)),
            event_capacity,
        }
    }

    pub fn breakpoints(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints.values()
    }

    pub fn events(&self) -> impl Iterator<Item = &DebugEvent> {
        self.events.iter()
    }

    pub const fn single_step_enabled(&self) -> bool {
        self.single_step
    }

    pub fn set_single_step(&mut self, enabled: bool) {
        self.single_step = enabled;
    }

    pub fn add_software_breakpoint<M: DebugMemory + ?Sized>(
        &mut self,
        memory: &mut M,
        address: u64,
    ) -> Result<u32, VmLifecycleError> {
        if self.by_address.contains_key(&address) {
            return Err(VmLifecycleError::BreakpointExists(address));
        }
        let instruction = self.architecture.instruction();
        let mut original_bytes = vec![0; instruction.len()];
        memory.read_debug_memory(address, &mut original_bytes)?;
        memory.write_debug_memory(address, instruction)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.breakpoints.insert(
            id,
            Breakpoint {
                id,
                address,
                length: instruction.len() as u8,
                kind: BreakpointKind::Software,
                enabled: true,
                hardware_slot: None,
                original_bytes,
                hit_count: 0,
            },
        );
        self.by_address.insert(address, id);
        Ok(id)
    }

    pub fn add_hardware_breakpoint(
        &mut self,
        address: u64,
        length: u8,
        kind: BreakpointKind,
        slot: u8,
    ) -> Result<u32, VmLifecycleError> {
        if slot >= 4
            || self
                .breakpoints
                .values()
                .any(|breakpoint| breakpoint.hardware_slot == Some(slot))
        {
            return Err(VmLifecycleError::HardwareSlot(slot));
        }
        if !matches!(length, 1 | 2 | 4 | 8)
            || matches!(kind, BreakpointKind::Software)
            || (matches!(kind, BreakpointKind::Execute) && length != 1)
        {
            return Err(VmLifecycleError::WatchpointLength);
        }
        if self.by_address.contains_key(&address) {
            return Err(VmLifecycleError::BreakpointExists(address));
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.breakpoints.insert(
            id,
            Breakpoint {
                id,
                address,
                length,
                kind,
                enabled: true,
                hardware_slot: Some(slot),
                original_bytes: Vec::new(),
                hit_count: 0,
            },
        );
        self.by_address.insert(address, id);
        Ok(id)
    }

    pub fn remove_breakpoint<M: DebugMemory + ?Sized>(
        &mut self,
        memory: &mut M,
        id: u32,
    ) -> Result<Breakpoint, VmLifecycleError> {
        let breakpoint = self
            .breakpoints
            .remove(&id)
            .ok_or(VmLifecycleError::BreakpointMissing(id as u64))?;
        self.by_address.remove(&breakpoint.address);
        if breakpoint.kind == BreakpointKind::Software && breakpoint.enabled {
            memory.write_debug_memory(breakpoint.address, &breakpoint.original_bytes)?;
        }
        if self.pending_reinsert == Some(id) {
            self.pending_reinsert = None;
        }
        Ok(breakpoint)
    }

    pub fn begin_software_breakpoint_resume<M: DebugMemory + ?Sized>(
        &mut self,
        memory: &mut M,
        address: u64,
    ) -> Result<u32, VmLifecycleError> {
        let id = self
            .by_address
            .get(&address)
            .copied()
            .ok_or(VmLifecycleError::BreakpointMissing(address))?;
        let breakpoint = self
            .breakpoints
            .get_mut(&id)
            .ok_or(VmLifecycleError::BreakpointMissing(address))?;
        if breakpoint.kind != BreakpointKind::Software || !breakpoint.enabled {
            return Err(VmLifecycleError::BreakpointMissing(address));
        }
        memory.write_debug_memory(address, &breakpoint.original_bytes)?;
        breakpoint.hit_count = breakpoint.hit_count.wrapping_add(1);
        self.pending_reinsert = Some(id);
        self.single_step = true;
        Ok(id)
    }

    pub fn complete_single_step<M: DebugMemory + ?Sized>(
        &mut self,
        memory: &mut M,
    ) -> Result<Option<u32>, VmLifecycleError> {
        let Some(id) = self.pending_reinsert.take() else {
            return Ok(None);
        };
        let breakpoint = self
            .breakpoints
            .get(&id)
            .ok_or(VmLifecycleError::BreakpointMissing(id as u64))?;
        if breakpoint.enabled {
            memory.write_debug_memory(breakpoint.address, self.architecture.instruction())?;
        }
        self.single_step = false;
        Ok(Some(id))
    }

    pub fn match_memory_access(&self, address: u64, length: u8, write: bool) -> Option<u32> {
        let access_end = address.checked_add(length as u64)?;
        self.breakpoints.values().find_map(|breakpoint| {
            if !breakpoint.enabled
                || !matches!(
                    breakpoint.kind,
                    BreakpointKind::Read | BreakpointKind::Write | BreakpointKind::ReadWrite
                )
                || (write && breakpoint.kind == BreakpointKind::Read)
                || (!write && breakpoint.kind == BreakpointKind::Write)
            {
                return None;
            }
            let breakpoint_end = breakpoint.address.checked_add(breakpoint.length as u64)?;
            (address < breakpoint_end && breakpoint.address < access_end).then_some(breakpoint.id)
        })
    }

    pub fn record_stop(&mut self, vcpu: u32, reason: DebugStopReason) {
        self.sequence = self.sequence.wrapping_add(1);
        if self.event_capacity != 0 {
            while self.events.len() >= self.event_capacity {
                self.events.pop_front();
            }
            self.events.push_back(DebugEvent {
                sequence: self.sequence,
                vcpu,
                reason,
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotId(pub [u8; 16]);

impl SnapshotId {
    pub const NIL: Self = Self([0; 16]);

    pub fn from_sequence(machine_id: u64, sequence: u64) -> Self {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&machine_id.to_le_bytes());
        bytes[8..].copy_from_slice(&sequence.to_le_bytes());
        Self(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmSnapshot {
    pub id: SnapshotId,
    pub parent: SnapshotId,
    pub generation: u64,
    pub timestamp_ns: u64,
    pub state: VmPowerState,
    pub architecture: GuestArchitecture,
    pub target: String,
    pub vcpus: Vec<VcpuSnapshot>,
    pub memory: MemorySnapshot,
    pub devices: Vec<DeviceSnapshot>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotLimits {
    pub max_snapshot_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_pages: usize,
    pub max_vcpus: usize,
    pub max_devices: usize,
    pub max_device_bytes: usize,
    pub max_string_bytes: usize,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_snapshot_bytes: 16 * 1024 * 1024 * 1024usize,
            max_memory_bytes: 8 * 1024 * 1024 * 1024usize,
            max_pages: 4 * 1024 * 1024,
            max_vcpus: 4096,
            max_devices: 65536,
            max_device_bytes: 1024 * 1024 * 1024,
            max_string_bytes: 1024 * 1024,
        }
    }
}

#[derive(Default)]
struct SnapshotWriter {
    bytes: Vec<u8>,
}

impl SnapshotWriter {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn blob(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.raw(value);
    }

    fn string(&mut self, value: &str) {
        self.blob(value.as_bytes());
    }
}

struct SnapshotReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    limits: SnapshotLimits,
}

impl<'a> SnapshotReader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], VmLifecycleError> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(VmLifecycleError::SnapshotFormat("truncated data"))?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn u8(&mut self) -> Result<u8, VmLifecycleError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, VmLifecycleError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, VmLifecycleError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, VmLifecycleError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn u128(&mut self) -> Result<u128, VmLifecycleError> {
        Ok(u128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }

    fn blob(&mut self, limit: usize) -> Result<Vec<u8>, VmLifecycleError> {
        let length = usize::try_from(self.u64()?)
            .map_err(|_| VmLifecycleError::SnapshotLimit("blob length"))?;
        if length > limit {
            return Err(VmLifecycleError::SnapshotLimit("blob length"));
        }
        Ok(self.take(length)?.to_vec())
    }

    fn string(&mut self) -> Result<String, VmLifecycleError> {
        String::from_utf8(self.blob(self.limits.max_string_bytes)?)
            .map_err(|_| VmLifecycleError::SnapshotFormat("string is not UTF-8"))
    }

    fn count(&mut self, limit: usize, label: &'static str) -> Result<usize, VmLifecycleError> {
        let count =
            usize::try_from(self.u64()?).map_err(|_| VmLifecycleError::SnapshotLimit(label))?;
        if count > limit {
            return Err(VmLifecycleError::SnapshotLimit(label));
        }
        Ok(count)
    }
}

impl VmSnapshot {
    pub fn verify(&self) -> Result<(), VmLifecycleError> {
        if self.id == SnapshotId::NIL {
            return Err(VmLifecycleError::SnapshotFormat("nil snapshot id"));
        }
        if self.target.trim().is_empty() {
            return Err(VmLifecycleError::SnapshotFormat("empty snapshot target"));
        }
        if self.vcpus.is_empty() {
            return Err(VmLifecycleError::SnapshotFormat("empty vCPU topology"));
        }
        let mut vcpu_ids = BTreeSet::new();
        for vcpu in &self.vcpus {
            if !vcpu_ids.insert(vcpu.id) {
                return Err(VmLifecycleError::DuplicateVcpu(vcpu.id));
            }
            if vcpu.registers.architecture != self.architecture {
                return Err(VmLifecycleError::SnapshotArchitectureMismatch);
            }
        }
        let mut ids = BTreeMap::new();
        for device in &self.devices {
            if ids.insert(device.id.as_str(), ()).is_some() {
                return Err(VmLifecycleError::DuplicateDevice(device.id.clone()));
            }
            device.verify()?;
        }
        if self.metadata.keys().any(|key| key.trim().is_empty()) {
            return Err(VmLifecycleError::SnapshotFormat("empty metadata key"));
        }
        self.memory.verify_structure()?;
        if self.memory.kind == SnapshotKind::Full {
            self.memory.restore(None)?;
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, VmLifecycleError> {
        self.verify()?;
        let mut writer = SnapshotWriter::default();
        writer.raw(&VM_SNAPSHOT_MAGIC);
        writer.u16(VM_SNAPSHOT_VERSION);
        writer.u8(VM_SNAPSHOT_ENDIAN_LITTLE);
        writer.u8(0);
        writer.raw(&self.id.0);
        writer.raw(&self.parent.0);
        writer.u64(self.generation);
        writer.u64(self.timestamp_ns);
        writer.u8(self.state as u8);
        writer.u8(self.architecture as u8);
        writer.u8(self.memory.kind as u8);
        writer.u8(0);
        writer.string(&self.target);

        writer.u64(self.vcpus.len() as u64);
        for vcpu in &self.vcpus {
            if vcpu.registers.architecture != self.architecture {
                return Err(VmLifecycleError::SnapshotArchitectureMismatch);
            }
            writer.u32(vcpu.id);
            writer.u8(vcpu.halted as u8);
            writer.u8(vcpu.registers.architecture as u8);
            writer.u16(0);
            writer.u64(vcpu.virtual_time_ns);
            writer.u64(vcpu.pending_interrupts.len() as u64);
            for interrupt in &vcpu.pending_interrupts {
                writer.u32(*interrupt);
            }
            writer.u64(vcpu.registers.values.len() as u64);
            for (name, value) in &vcpu.registers.values {
                writer.string(name);
                writer.u128(*value);
            }
        }

        writer.u32(self.memory.page_size);
        writer.u64(self.memory.memory_len);
        writer.u32(self.memory.base_checksum);
        writer.u32(self.memory.image_checksum);
        writer.u64(self.memory.pages.len() as u64);
        for page in &self.memory.pages {
            writer.u64(page.index);
            writer.u32(page.checksum);
            writer.blob(&page.data);
        }

        writer.u64(self.devices.len() as u64);
        for device in &self.devices {
            writer.string(&device.id);
            writer.u16(device.class as u16);
            writer.u16(device.schema_version);
            writer.u8(device.required as u8);
            writer.u8(0);
            writer.u16(0);
            writer.u32(device.checksum);
            writer.blob(&device.data);
        }

        writer.u64(self.metadata.len() as u64);
        for (key, value) in &self.metadata {
            writer.string(key);
            writer.string(value);
        }
        let checksum = crc32(&writer.bytes);
        writer.u32(checksum);
        Ok(writer.bytes)
    }

    pub fn decode(bytes: &[u8], limits: SnapshotLimits) -> Result<Self, VmLifecycleError> {
        if bytes.len() > limits.max_snapshot_bytes {
            return Err(VmLifecycleError::SnapshotLimit("file size"));
        }
        if bytes.len() < VM_SNAPSHOT_MAGIC.len() + 4 {
            return Err(VmLifecycleError::SnapshotFormat("truncated header"));
        }
        let body_len = bytes.len() - 4;
        let expected = u32::from_le_bytes(bytes[body_len..].try_into().unwrap());
        let actual = crc32(&bytes[..body_len]);
        if expected != actual {
            return Err(VmLifecycleError::SnapshotChecksum { expected, actual });
        }
        let mut reader = SnapshotReader {
            bytes: &bytes[..body_len],
            offset: 0,
            limits,
        };
        if reader.take(8)? != VM_SNAPSHOT_MAGIC {
            return Err(VmLifecycleError::SnapshotFormat("bad magic"));
        }
        if reader.u16()? != VM_SNAPSHOT_VERSION {
            return Err(VmLifecycleError::SnapshotFormat("unsupported version"));
        }
        if reader.u8()? != VM_SNAPSHOT_ENDIAN_LITTLE {
            return Err(VmLifecycleError::SnapshotFormat("unsupported endian"));
        }
        reader.u8()?;
        let id = SnapshotId(reader.take(16)?.try_into().unwrap());
        let parent = SnapshotId(reader.take(16)?.try_into().unwrap());
        let generation = reader.u64()?;
        let timestamp_ns = reader.u64()?;
        let state = VmPowerState::from_u8(reader.u8()?)?;
        let architecture = GuestArchitecture::from_u8(reader.u8()?)?;
        let memory_kind = SnapshotKind::from_u8(reader.u8()?)?;
        reader.u8()?;
        let target = reader.string()?;

        let vcpu_count = reader.count(limits.max_vcpus, "vCPU count")?;
        let mut vcpus = Vec::with_capacity(vcpu_count);
        for _ in 0..vcpu_count {
            let id = reader.u32()?;
            let halted = reader.u8()? != 0;
            let register_architecture = GuestArchitecture::from_u8(reader.u8()?)?;
            reader.u16()?;
            let virtual_time_ns = reader.u64()?;
            let pending_count = reader.count(65536, "pending interrupt count")?;
            let mut pending_interrupts = Vec::with_capacity(pending_count);
            for _ in 0..pending_count {
                pending_interrupts.push(reader.u32()?);
            }
            let register_count = reader.count(65536, "register count")?;
            let mut registers = RegisterBank::new(register_architecture);
            for _ in 0..register_count {
                let name = reader.string()?;
                if registers.values.insert(name, reader.u128()?).is_some() {
                    return Err(VmLifecycleError::SnapshotFormat("duplicate register"));
                }
            }
            vcpus.push(VcpuSnapshot {
                id,
                registers,
                halted,
                pending_interrupts,
                virtual_time_ns,
            });
        }

        let page_size = reader.u32()?;
        MemorySnapshot::validate_page_size(page_size as usize)?;
        let memory_len = reader.u64()?;
        if memory_len > limits.max_memory_bytes as u64 {
            return Err(VmLifecycleError::SnapshotLimit("memory length"));
        }
        let base_checksum = reader.u32()?;
        let image_checksum = reader.u32()?;
        let page_count = reader.count(limits.max_pages, "memory page count")?;
        let mut pages = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            let index = reader.u64()?;
            let checksum = reader.u32()?;
            let data = reader.blob(page_size as usize)?;
            pages.push(MemoryPageSnapshot {
                index,
                data,
                checksum,
            });
        }
        let memory = MemorySnapshot {
            kind: memory_kind,
            page_size,
            memory_len,
            base_checksum,
            image_checksum,
            pages,
        };

        let device_count = reader.count(limits.max_devices, "device count")?;
        let mut devices = Vec::with_capacity(device_count);
        let mut device_ids = BTreeMap::new();
        for _ in 0..device_count {
            let id = reader.string()?;
            if device_ids.insert(id.clone(), ()).is_some() {
                return Err(VmLifecycleError::DuplicateDevice(id));
            }
            let class = VirtualDeviceClass::from_u16(reader.u16()?)?;
            let schema_version = reader.u16()?;
            let required = reader.u8()? != 0;
            reader.u8()?;
            reader.u16()?;
            let checksum = reader.u32()?;
            let data = reader.blob(limits.max_device_bytes)?;
            let device = DeviceSnapshot {
                id,
                class,
                schema_version,
                required,
                data,
                checksum,
            };
            device.verify()?;
            devices.push(device);
        }

        let metadata_count = reader.count(65536, "metadata count")?;
        let mut metadata = BTreeMap::new();
        for _ in 0..metadata_count {
            let key = reader.string()?;
            let value = reader.string()?;
            if metadata.insert(key, value).is_some() {
                return Err(VmLifecycleError::SnapshotFormat("duplicate metadata key"));
            }
        }
        if reader.offset != reader.bytes.len() {
            return Err(VmLifecycleError::SnapshotFormat("trailing data"));
        }
        let snapshot = Self {
            id,
            parent,
            generation,
            timestamp_ns,
            state,
            architecture,
            target,
            vcpus,
            memory,
            devices,
            metadata,
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub fn restore_memory(&self, base: Option<&[u8]>) -> Result<Vec<u8>, VmLifecycleError> {
        self.memory.restore(base)
    }

    pub fn assert_compatible(
        &self,
        architecture: GuestArchitecture,
        target: &str,
    ) -> Result<(), VmLifecycleError> {
        if self.architecture != architecture {
            return Err(VmLifecycleError::SnapshotArchitectureMismatch);
        }
        if self.target != target {
            return Err(VmLifecycleError::SnapshotTargetMismatch);
        }
        Ok(())
    }
}

pub fn persist_snapshot_atomic(path: &Path, snapshot: &VmSnapshot) -> Result<(), VmLifecycleError> {
    let bytes = snapshot.encode()?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(VmLifecycleError::SnapshotFormat("invalid snapshot path"))?;
    let mut temporary = None;
    for attempt in 0..128u32 {
        let candidate = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            attempt
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(&bytes)?;
                file.sync_all()?;
                temporary = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let temporary = temporary.ok_or_else(|| {
        VmLifecycleError::Io("could not reserve an atomic snapshot temporary file".into())
    })?;
    if !path.exists() {
        fs::rename(&temporary, path)?;
        sync_directory(parent)?;
        return Ok(());
    }
    let backup = unique_backup_path(parent, file_name)?;
    fs::rename(path, &backup)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::rename(&backup, path);
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    fs::remove_file(backup)?;
    sync_directory(parent)?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), VmLifecycleError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), VmLifecycleError> {
    Ok(())
}

fn unique_backup_path(parent: &Path, file_name: &str) -> Result<PathBuf, VmLifecycleError> {
    for attempt in 0..128u32 {
        let candidate = parent.join(format!(
            ".{file_name}.{}.{}.bak",
            std::process::id(),
            attempt
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(VmLifecycleError::Io(
        "could not reserve an atomic snapshot backup file".into(),
    ))
}

pub fn load_snapshot(path: &Path, limits: SnapshotLimits) -> Result<VmSnapshot, VmLifecycleError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > limits.max_snapshot_bytes as u64 {
        return Err(VmLifecycleError::SnapshotLimit("file size"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?.read_to_end(&mut bytes)?;
    VmSnapshot::decode(&bytes, limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_snapshot(memory: MemorySnapshot) -> VmSnapshot {
        let mut registers = RegisterBank::new(GuestArchitecture::X86_64);
        registers.set("rip", 0x1000);
        registers.set("rax", 42);
        VmSnapshot {
            id: SnapshotId::from_sequence(7, 2),
            parent: SnapshotId::NIL,
            generation: 2,
            timestamp_ns: 123_456,
            state: VmPowerState::Paused,
            architecture: GuestArchitecture::X86_64,
            target: "x86_64-unknown-none".into(),
            vcpus: vec![VcpuSnapshot {
                id: 0,
                registers,
                halted: false,
                pending_interrupts: vec![32, 48],
                virtual_time_ns: 99,
            }],
            memory,
            devices: vec![
                DeviceSnapshot::new("uart0", VirtualDeviceClass::Uart, 1, true, vec![1, 2, 3]),
                DeviceSnapshot::new(
                    "virtio-blk0",
                    VirtualDeviceClass::VirtioBlock,
                    1,
                    true,
                    vec![4, 5, 6, 7],
                ),
            ],
            metadata: BTreeMap::from([
                ("firmware".into(), "uefi".into()),
                ("vm".into(), "test".into()),
            ]),
        }
    }

    #[test]
    fn lifecycle_enforces_pause_resume_reset_crash_and_restart() {
        let mut lifecycle = VmLifecycle::new(16);
        lifecycle.start().unwrap();
        lifecycle.pause("debugger").unwrap();
        lifecycle.resume().unwrap();
        lifecycle.begin_reset(VmResetKind::Warm).unwrap();
        lifecycle.complete_reset(true).unwrap();
        lifecycle.resume().unwrap();
        lifecycle.crash("triple fault").unwrap();
        lifecycle.begin_reset(VmResetKind::Cold).unwrap();
        lifecycle.complete_reset(false).unwrap();
        lifecycle.stop("poweroff").unwrap();
        lifecycle.start().unwrap();
        assert_eq!(lifecycle.state(), VmPowerState::Running);
        assert_eq!(lifecycle.boot_count(), 2);
        assert_eq!(lifecycle.reset_count(), 2);
        assert_eq!(lifecycle.events().count(), 11);
        assert!(lifecycle.pause("pause").is_ok());
        assert!(matches!(
            lifecycle.start(),
            Err(VmLifecycleError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn full_and_incremental_memory_snapshots_restore_exact_bytes() {
        let mut base = vec![0u8; 16 * 1024 + 13];
        base[17] = 7;
        base[8192] = 9;
        let full = MemorySnapshot::capture_full(&base, 4096).unwrap();
        assert_eq!(full.restore(None).unwrap(), base);

        let mut changed = base.clone();
        changed[17] = 11;
        changed[12_400] = 99;
        let delta = MemorySnapshot::capture_incremental(&base, &changed, 4096).unwrap();
        assert_eq!(delta.pages.len(), 2);
        assert_eq!(delta.restore(Some(&base)).unwrap(), changed);
        let mut wrong_base = base.clone();
        wrong_base[1] = 1;
        assert_eq!(
            delta.restore(Some(&wrong_base)).unwrap_err(),
            VmLifecycleError::SnapshotBaseMismatch
        );
    }

    #[test]
    fn snapshot_binary_roundtrip_rejects_corruption_and_persists_atomically() {
        let memory = (0..10_000).map(|value| value as u8).collect::<Vec<_>>();
        let snapshot = test_snapshot(MemorySnapshot::capture_full(&memory, 4096).unwrap());
        let encoded = snapshot.encode().unwrap();
        let decoded = VmSnapshot::decode(&encoded, SnapshotLimits::default()).unwrap();
        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.restore_memory(None).unwrap(), memory);
        let mut corrupted = encoded;
        corrupted[90] ^= 0x80;
        assert!(matches!(
            VmSnapshot::decode(&corrupted, SnapshotLimits::default()),
            Err(VmLifecycleError::SnapshotChecksum { .. })
        ));

        let directory = std::env::temp_dir().join(format!(
            "ncc-machine-snapshot-{}-{}",
            std::process::id(),
            snapshot.generation
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("vm.ncsnap");
        persist_snapshot_atomic(&path, &snapshot).unwrap();
        persist_snapshot_atomic(&path, &snapshot).unwrap();
        assert_eq!(
            load_snapshot(&path, SnapshotLimits::default()).unwrap(),
            snapshot
        );
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn snapshot_contract_rejects_duplicate_ids_and_unversioned_devices() {
        let memory = MemorySnapshot::capture_full(&vec![0; 8192], 4096).unwrap();
        let mut snapshot = test_snapshot(memory.clone());
        snapshot.vcpus.push(snapshot.vcpus[0].clone());
        assert_eq!(
            snapshot.encode().unwrap_err(),
            VmLifecycleError::DuplicateVcpu(0)
        );

        let mut snapshot = test_snapshot(memory);
        snapshot.devices[0].schema_version = 0;
        assert_eq!(
            snapshot.encode().unwrap_err(),
            VmLifecycleError::SnapshotFormat("device schema version must be nonzero")
        );
    }

    #[test]
    fn incremental_snapshot_validates_every_page_before_persistence() {
        let base = vec![0x11; 4096 * 3 + 17];
        let mut changed = base.clone();
        changed[4096 + 7] = 0x22;
        changed[4096 * 3 + 16] = 0x33;
        let delta = MemorySnapshot::capture_incremental(&base, &changed, 4096).unwrap();
        let snapshot = test_snapshot(delta.clone());
        let encoded = snapshot.encode().unwrap();
        let decoded = VmSnapshot::decode(&encoded, SnapshotLimits::default()).unwrap();
        assert_eq!(decoded.restore_memory(Some(&base)).unwrap(), changed);

        let mut malformed = delta;
        malformed.pages[0].index = 99;
        assert_eq!(
            test_snapshot(malformed).encode().unwrap_err(),
            VmLifecycleError::InvalidMemoryPage
        );
    }

    #[test]
    fn debugger_restores_and_reinserts_breakpoints_and_matches_watchpoints() {
        let mut memory = vec![0x90u8; 64];
        let mut debugger = VmDebugger::new(BreakpointArchitecture::X86, 8);
        let software = debugger.add_software_breakpoint(&mut memory, 8).unwrap();
        assert_eq!(memory[8], 0xcc);
        assert_eq!(
            debugger
                .begin_software_breakpoint_resume(&mut memory, 8)
                .unwrap(),
            software
        );
        assert_eq!(memory[8], 0x90);
        assert!(debugger.single_step_enabled());
        assert_eq!(
            debugger.complete_single_step(&mut memory).unwrap(),
            Some(software)
        );
        assert_eq!(memory[8], 0xcc);

        let watch = debugger
            .add_hardware_breakpoint(16, 4, BreakpointKind::ReadWrite, 0)
            .unwrap();
        assert_eq!(debugger.match_memory_access(18, 2, true), Some(watch));
        assert_eq!(debugger.match_memory_access(20, 1, false), None);
        debugger.record_stop(
            0,
            DebugStopReason::SoftwareBreakpoint {
                id: software,
                address: 8,
            },
        );
        assert_eq!(debugger.events().count(), 1);
        debugger.remove_breakpoint(&mut memory, software).unwrap();
        assert_eq!(memory[8], 0x90);
    }
}
