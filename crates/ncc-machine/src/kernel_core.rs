use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::error::Error;
use std::fmt;

use crate::elf_loader::{load_user_elf, parse_user_elf, LoadedUserElf, UserLoadOptions};

pub const KERNEL_PAGE_SIZE: usize = 4096;
pub type FrameId = usize;
pub type ProcessId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelCoreError {
    InvalidConfiguration(&'static str),
    OutOfFrames,
    InvalidFrame(FrameId),
    FrameStillShared {
        frame: FrameId,
        references: u32,
    },
    AddressOverflow,
    MappingExists(u64),
    MappingMissing(u64),
    ProtectionFault {
        address: u64,
        operation: &'static str,
    },
    InvalidElf(&'static str),
    UnsupportedElf {
        class: u8,
        machine: u16,
    },
    ProcessExists(ProcessId),
    ProcessMissing(ProcessId),
    InvalidProcessState(ProcessId),
    CpuOutOfRange(usize),
    InvalidPriority(u8),
    InvalidAffinity,
    SchedulerTaskMissing(ProcessId),
    SchedulerTaskExists(ProcessId),
    LockNotOwned {
        lock: u64,
        pid: ProcessId,
    },
    InvalidSyscall(u32),
}

impl fmt::Display for KernelCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) | Self::InvalidElf(message) => f.write_str(message),
            Self::OutOfFrames => f.write_str("physical frame allocator is exhausted"),
            Self::InvalidFrame(frame) => write!(f, "invalid physical frame {frame}"),
            Self::FrameStillShared { frame, references } => {
                write!(
                    f,
                    "physical frame {frame} still has {references} references"
                )
            }
            Self::AddressOverflow => f.write_str("address calculation overflow"),
            Self::MappingExists(address) => {
                write!(f, "virtual mapping already exists at {address:#x}")
            }
            Self::MappingMissing(address) => write!(f, "virtual mapping is absent at {address:#x}"),
            Self::ProtectionFault { address, operation } => {
                write!(f, "{operation} protection fault at {address:#x}")
            }
            Self::UnsupportedElf { class, machine } => {
                write!(f, "unsupported ELF class {class} or machine {machine}")
            }
            Self::ProcessExists(pid) => write!(f, "process {pid} already exists"),
            Self::ProcessMissing(pid) => write!(f, "process {pid} does not exist"),
            Self::InvalidProcessState(pid) => {
                write!(f, "process {pid} is not runnable in its current state")
            }
            Self::CpuOutOfRange(cpu) => write!(f, "CPU {cpu} is outside the scheduler topology"),
            Self::InvalidPriority(priority) => {
                write!(f, "scheduler priority {priority} is outside 0..31")
            }
            Self::InvalidAffinity => f.write_str("scheduler affinity mask selects no online CPU"),
            Self::SchedulerTaskMissing(pid) => write!(f, "scheduler task {pid} does not exist"),
            Self::SchedulerTaskExists(pid) => write!(f, "scheduler task {pid} already exists"),
            Self::LockNotOwned { lock, pid } => {
                write!(f, "task {pid} does not own kernel lock {lock}")
            }
            Self::InvalidSyscall(number) => write!(f, "unknown syscall {number}"),
        }
    }
}

impl Error for KernelCoreError {}

#[derive(Debug, Clone)]
struct PhysicalFrame {
    bytes: Box<[u8; KERNEL_PAGE_SIZE]>,
    references: u32,
    allocated: bool,
}

#[derive(Debug, Clone)]
pub struct PhysicalMemory {
    frames: Vec<PhysicalFrame>,
    free: VecDeque<FrameId>,
}

impl PhysicalMemory {
    pub fn new(frame_count: usize) -> Result<Self, KernelCoreError> {
        if frame_count == 0 {
            return Err(KernelCoreError::InvalidConfiguration(
                "physical memory requires at least one frame",
            ));
        }
        let frames = (0..frame_count)
            .map(|_| PhysicalFrame {
                bytes: Box::new([0; KERNEL_PAGE_SIZE]),
                references: 0,
                allocated: false,
            })
            .collect();
        Ok(Self {
            frames,
            free: (0..frame_count).collect(),
        })
    }

    pub fn capacity_bytes(&self) -> usize {
        self.frames.len() * KERNEL_PAGE_SIZE
    }

    pub fn free_frames(&self) -> usize {
        self.free.len()
    }

    pub fn allocate(&mut self) -> Result<FrameId, KernelCoreError> {
        let id = self.free.pop_front().ok_or(KernelCoreError::OutOfFrames)?;
        let frame = &mut self.frames[id];
        frame.bytes.fill(0);
        frame.references = 1;
        frame.allocated = true;
        Ok(id)
    }

    pub fn clone_frame(&mut self, source: FrameId) -> Result<FrameId, KernelCoreError> {
        let bytes = *self.frame(source)?.bytes;
        let destination = self.allocate()?;
        self.frames[destination].bytes.copy_from_slice(&bytes);
        Ok(destination)
    }

    pub fn retain(&mut self, id: FrameId) -> Result<(), KernelCoreError> {
        let frame = self.frame_mut(id)?;
        frame.references =
            frame
                .references
                .checked_add(1)
                .ok_or(KernelCoreError::InvalidConfiguration(
                    "frame reference overflow",
                ))?;
        Ok(())
    }

    pub fn release(&mut self, id: FrameId) -> Result<(), KernelCoreError> {
        let frame = self.frame_mut(id)?;
        frame.references -= 1;
        if frame.references == 0 {
            frame.allocated = false;
            frame.bytes.fill(0);
            self.free.push_back(id);
        }
        Ok(())
    }

    pub fn references(&self, id: FrameId) -> Result<u32, KernelCoreError> {
        Ok(self.frame(id)?.references)
    }

    pub fn read(
        &self,
        id: FrameId,
        offset: usize,
        output: &mut [u8],
    ) -> Result<(), KernelCoreError> {
        let end = offset
            .checked_add(output.len())
            .filter(|end| *end <= KERNEL_PAGE_SIZE)
            .ok_or(KernelCoreError::AddressOverflow)?;
        output.copy_from_slice(&self.frame(id)?.bytes[offset..end]);
        Ok(())
    }

    pub fn write(
        &mut self,
        id: FrameId,
        offset: usize,
        input: &[u8],
    ) -> Result<(), KernelCoreError> {
        let end = offset
            .checked_add(input.len())
            .filter(|end| *end <= KERNEL_PAGE_SIZE)
            .ok_or(KernelCoreError::AddressOverflow)?;
        self.frame_mut(id)?.bytes[offset..end].copy_from_slice(input);
        Ok(())
    }

    fn frame(&self, id: FrameId) -> Result<&PhysicalFrame, KernelCoreError> {
        self.frames
            .get(id)
            .filter(|frame| frame.allocated)
            .ok_or(KernelCoreError::InvalidFrame(id))
    }

    fn frame_mut(&mut self, id: FrameId) -> Result<&mut PhysicalFrame, KernelCoreError> {
        self.frames
            .get_mut(id)
            .filter(|frame| frame.allocated)
            .ok_or(KernelCoreError::InvalidFrame(id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags(u16);

impl PageFlags {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);
    pub const GLOBAL: Self = Self(1 << 4);
    pub const DEVICE: Self = Self(1 << 5);
    pub const COW: Self = Self(1 << 6);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn bits(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMapping {
    pub frame: FrameId,
    pub flags: PageFlags,
}

#[derive(Debug, Clone, Default)]
pub struct AddressSpace {
    pages: BTreeMap<u64, PageMapping>,
    pub tlb_generation: u64,
}

impl AddressSpace {
    pub fn map(
        &mut self,
        address: u64,
        frame: FrameId,
        flags: PageFlags,
    ) -> Result<(), KernelCoreError> {
        if address as usize % KERNEL_PAGE_SIZE != 0 {
            return Err(KernelCoreError::InvalidConfiguration(
                "virtual mapping is not page aligned",
            ));
        }
        let page = address / KERNEL_PAGE_SIZE as u64;
        if self
            .pages
            .insert(page, PageMapping { frame, flags })
            .is_some()
        {
            return Err(KernelCoreError::MappingExists(address));
        }
        self.tlb_generation = self.tlb_generation.wrapping_add(1);
        Ok(())
    }

    pub fn map_zeroed(
        &mut self,
        memory: &mut PhysicalMemory,
        address: u64,
        flags: PageFlags,
    ) -> Result<FrameId, KernelCoreError> {
        let frame = memory.allocate()?;
        if let Err(error) = self.map(address, frame, flags) {
            let _ = memory.release(frame);
            return Err(error);
        }
        Ok(frame)
    }

    pub fn protect(&mut self, address: u64, flags: PageFlags) -> Result<(), KernelCoreError> {
        let page = address / KERNEL_PAGE_SIZE as u64;
        let mapping = self
            .pages
            .get_mut(&page)
            .ok_or(KernelCoreError::MappingMissing(address))?;
        mapping.flags = flags;
        self.tlb_generation = self.tlb_generation.wrapping_add(1);
        Ok(())
    }

    pub fn unmap(
        &mut self,
        memory: &mut PhysicalMemory,
        address: u64,
    ) -> Result<PageMapping, KernelCoreError> {
        let page = address / KERNEL_PAGE_SIZE as u64;
        let mapping = self
            .pages
            .remove(&page)
            .ok_or(KernelCoreError::MappingMissing(address))?;
        memory.release(mapping.frame)?;
        self.tlb_generation = self.tlb_generation.wrapping_add(1);
        Ok(mapping)
    }

    pub fn mapping(&self, address: u64) -> Result<PageMapping, KernelCoreError> {
        self.pages
            .get(&(address / KERNEL_PAGE_SIZE as u64))
            .copied()
            .ok_or(KernelCoreError::MappingMissing(address))
    }

    pub fn fork_cow(&mut self, memory: &mut PhysicalMemory) -> Result<Self, KernelCoreError> {
        let mut child = Self::default();
        for (page, parent_mapping) in &mut self.pages {
            memory.retain(parent_mapping.frame)?;
            if parent_mapping.flags.contains(PageFlags::WRITE) {
                parent_mapping.flags = parent_mapping
                    .flags
                    .without(PageFlags::WRITE)
                    .union(PageFlags::COW);
            }
            child.pages.insert(*page, *parent_mapping);
        }
        self.tlb_generation = self.tlb_generation.wrapping_add(1);
        child.tlb_generation = self.tlb_generation;
        Ok(child)
    }

    pub fn read(
        &self,
        memory: &PhysicalMemory,
        address: u64,
        output: &mut [u8],
    ) -> Result<(), KernelCoreError> {
        self.transfer_read(memory, address, output, PageFlags::READ)
    }

    pub fn read_execute(
        &self,
        memory: &PhysicalMemory,
        address: u64,
        output: &mut [u8],
    ) -> Result<(), KernelCoreError> {
        self.transfer_read(memory, address, output, PageFlags::EXECUTE)
    }

    fn transfer_read(
        &self,
        memory: &PhysicalMemory,
        mut address: u64,
        mut output: &mut [u8],
        required: PageFlags,
    ) -> Result<(), KernelCoreError> {
        while !output.is_empty() {
            let mapping = self.mapping(address)?;
            if !mapping.flags.contains(required) {
                return Err(KernelCoreError::ProtectionFault {
                    address,
                    operation: if required == PageFlags::EXECUTE {
                        "execute"
                    } else {
                        "read"
                    },
                });
            }
            let offset = address as usize % KERNEL_PAGE_SIZE;
            let amount = output.len().min(KERNEL_PAGE_SIZE - offset);
            let (chunk, rest) = output.split_at_mut(amount);
            memory.read(mapping.frame, offset, chunk)?;
            output = rest;
            address = address
                .checked_add(amount as u64)
                .ok_or(KernelCoreError::AddressOverflow)?;
        }
        Ok(())
    }

    pub fn write(
        &mut self,
        memory: &mut PhysicalMemory,
        mut address: u64,
        mut input: &[u8],
    ) -> Result<(), KernelCoreError> {
        while !input.is_empty() {
            let page = address / KERNEL_PAGE_SIZE as u64;
            let current = self.mapping(address)?;
            if current.flags.contains(PageFlags::COW) {
                let references = memory.references(current.frame)?;
                let replacement = if references > 1 {
                    let cloned = memory.clone_frame(current.frame)?;
                    memory.release(current.frame)?;
                    cloned
                } else {
                    current.frame
                };
                let mapping = self.pages.get_mut(&page).unwrap();
                mapping.frame = replacement;
                mapping.flags = mapping
                    .flags
                    .without(PageFlags::COW)
                    .union(PageFlags::WRITE);
                self.tlb_generation = self.tlb_generation.wrapping_add(1);
            }
            let mapping = self.mapping(address)?;
            if !mapping.flags.contains(PageFlags::WRITE) {
                return Err(KernelCoreError::ProtectionFault {
                    address,
                    operation: "write",
                });
            }
            let offset = address as usize % KERNEL_PAGE_SIZE;
            let amount = input.len().min(KERNEL_PAGE_SIZE - offset);
            memory.write(mapping.frame, offset, &input[..amount])?;
            input = &input[amount..];
            address = address
                .checked_add(amount as u64)
                .ok_or(KernelCoreError::AddressOverflow)?;
        }
        Ok(())
    }

    pub fn release_all(&mut self, memory: &mut PhysicalMemory) -> Result<(), KernelCoreError> {
        let frames = self
            .pages
            .values()
            .map(|mapping| mapping.frame)
            .collect::<Vec<_>>();
        self.pages.clear();
        for frame in frames {
            memory.release(frame)?;
        }
        self.tlb_generation = self.tlb_generation.wrapping_add(1);
        Ok(())
    }

    pub fn mapped_pages(&self) -> usize {
        self.pages.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfLoadSegment {
    pub virtual_address: u64,
    pub memory_size: u64,
    pub file_offset: usize,
    pub file_size: usize,
    pub flags: PageFlags,
    pub alignment: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedElf {
    pub class: ElfClass,
    pub machine: u16,
    pub entry: u64,
    pub segments: Vec<ElfLoadSegment>,
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, KernelCoreError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(KernelCoreError::InvalidElf("truncated ELF u16"))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, KernelCoreError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(KernelCoreError::InvalidElf("truncated ELF u32"))?;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, KernelCoreError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(KernelCoreError::InvalidElf("truncated ELF u64"))?;
    Ok(u64::from_le_bytes(raw.try_into().unwrap()))
}

pub fn parse_elf(
    image: &[u8],
    expected_machine: Option<u16>,
) -> Result<ParsedElf, KernelCoreError> {
    if image.len() < 52 || !image.starts_with(b"\x7fELF") {
        return Err(KernelCoreError::InvalidElf("missing ELF magic or header"));
    }
    let class = image[4];
    if image[5] != 1 {
        return Err(KernelCoreError::InvalidElf(
            "only little-endian ELF is supported",
        ));
    }
    let machine = read_u16(image, 18)?;
    if expected_machine.is_some_and(|expected| expected != machine) {
        return Err(KernelCoreError::UnsupportedElf { class, machine });
    }
    let elf_type = read_u16(image, 16)?;
    if !matches!(elf_type, 2 | 3) {
        return Err(KernelCoreError::InvalidElf("ELF must be ET_EXEC or ET_DYN"));
    }
    let (parsed_class, entry, phoff, phentsize, phnum) = match class {
        1 => (
            ElfClass::Elf32,
            read_u32(image, 24)? as u64,
            read_u32(image, 28)? as usize,
            read_u16(image, 42)? as usize,
            read_u16(image, 44)? as usize,
        ),
        2 => (
            ElfClass::Elf64,
            read_u64(image, 24)?,
            usize::try_from(read_u64(image, 32)?).map_err(|_| KernelCoreError::AddressOverflow)?,
            read_u16(image, 54)? as usize,
            read_u16(image, 56)? as usize,
        ),
        _ => return Err(KernelCoreError::UnsupportedElf { class, machine }),
    };
    let minimum = if class == 1 { 32 } else { 56 };
    if phentsize < minimum {
        return Err(KernelCoreError::InvalidElf(
            "program header entry is too small",
        ));
    }
    let table_size = phentsize
        .checked_mul(phnum)
        .ok_or(KernelCoreError::AddressOverflow)?;
    if phoff
        .checked_add(table_size)
        .filter(|end| *end <= image.len())
        .is_none()
    {
        return Err(KernelCoreError::InvalidElf(
            "program header table is truncated",
        ));
    }
    let mut segments = Vec::new();
    for index in 0..phnum {
        let offset = phoff + index * phentsize;
        if read_u32(image, offset)? != 1 {
            continue;
        }
        let (flags, file_offset, virtual_address, file_size, memory_size, alignment) = if class == 1
        {
            (
                read_u32(image, offset + 24)?,
                read_u32(image, offset + 4)? as u64,
                read_u32(image, offset + 8)? as u64,
                read_u32(image, offset + 16)? as u64,
                read_u32(image, offset + 20)? as u64,
                read_u32(image, offset + 28)? as u64,
            )
        } else {
            (
                read_u32(image, offset + 4)?,
                read_u64(image, offset + 8)?,
                read_u64(image, offset + 16)?,
                read_u64(image, offset + 32)?,
                read_u64(image, offset + 40)?,
                read_u64(image, offset + 48)?,
            )
        };
        if file_size > memory_size {
            return Err(KernelCoreError::InvalidElf(
                "ELF segment filesz exceeds memsz",
            ));
        }
        let file_offset =
            usize::try_from(file_offset).map_err(|_| KernelCoreError::AddressOverflow)?;
        let file_size = usize::try_from(file_size).map_err(|_| KernelCoreError::AddressOverflow)?;
        if file_offset
            .checked_add(file_size)
            .filter(|end| *end <= image.len())
            .is_none()
        {
            return Err(KernelCoreError::InvalidElf("ELF segment data is truncated"));
        }
        if memory_size == 0 {
            continue;
        }
        let mut page_flags = PageFlags::USER;
        if flags & 4 != 0 {
            page_flags = page_flags.union(PageFlags::READ);
        }
        if flags & 2 != 0 {
            page_flags = page_flags.union(PageFlags::WRITE);
        }
        if flags & 1 != 0 {
            page_flags = page_flags.union(PageFlags::EXECUTE);
        }
        segments.push(ElfLoadSegment {
            virtual_address,
            memory_size,
            file_offset,
            file_size,
            flags: page_flags,
            alignment,
        });
    }
    if segments.is_empty() {
        return Err(KernelCoreError::InvalidElf("ELF has no loadable segments"));
    }
    if !segments.iter().any(|segment| {
        entry >= segment.virtual_address
            && entry < segment.virtual_address.saturating_add(segment.memory_size)
            && segment.flags.contains(PageFlags::EXECUTE)
    }) {
        return Err(KernelCoreError::InvalidElf(
            "ELF entry is not inside an executable segment",
        ));
    }
    Ok(ParsedElf {
        class: parsed_class,
        machine,
        entry,
        segments,
    })
}

pub fn build_test_elf64(machine: u16, virtual_address: u64, code: &[u8]) -> Vec<u8> {
    let file_offset = 0x100usize;
    let mut image = vec![0u8; file_offset + code.len()];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    image[16..18].copy_from_slice(&2u16.to_le_bytes());
    image[18..20].copy_from_slice(&machine.to_le_bytes());
    image[20..24].copy_from_slice(&1u32.to_le_bytes());
    image[24..32].copy_from_slice(&virtual_address.to_le_bytes());
    image[32..40].copy_from_slice(&64u64.to_le_bytes());
    image[52..54].copy_from_slice(&64u16.to_le_bytes());
    image[54..56].copy_from_slice(&56u16.to_le_bytes());
    image[56..58].copy_from_slice(&1u16.to_le_bytes());
    image[64..68].copy_from_slice(&1u32.to_le_bytes());
    image[68..72].copy_from_slice(&5u32.to_le_bytes());
    image[72..80].copy_from_slice(&(file_offset as u64).to_le_bytes());
    image[80..88].copy_from_slice(&virtual_address.to_le_bytes());
    image[88..96].copy_from_slice(&virtual_address.to_le_bytes());
    image[96..104].copy_from_slice(&(code.len() as u64).to_le_bytes());
    image[104..112].copy_from_slice(&(code.len() as u64).to_le_bytes());
    image[112..120].copy_from_slice(&1u64.to_le_bytes());
    image[file_offset..].copy_from_slice(code);
    image
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserContext {
    pub registers: [u64; 32],
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub thread_pointer: u64,
    pub flags: u64,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub id: ProcessId,
    pub parent: Option<ProcessId>,
    pub name: String,
    pub state: ProcessState,
    pub priority: u8,
    pub affinity: Option<usize>,
    pub affinity_mask: u128,
    pub context: UserContext,
    pub address_space: AddressSpace,
    pub loaded_image: Option<LoadedUserElf>,
    pub exit_code: Option<i32>,
    pub open_handles: BTreeMap<u32, u64>,
}

pub const SCHEDULER_PRIORITY_LEVELS: usize = 32;
pub const SCHEDULER_DEFAULT_PRIORITY: u8 = 16;
pub const SCHEDULER_DEFAULT_QUANTUM_NS: u64 = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerTaskState {
    Ready,
    Running,
    Blocked,
    Sleeping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTaskInfo {
    pub pid: ProcessId,
    pub base_priority: u8,
    pub effective_priority: u8,
    pub affinity_mask: u128,
    pub state: SchedulerTaskState,
    pub cpu: Option<usize>,
    pub wake_deadline_ns: Option<u64>,
    pub runtime_ns: u64,
}

#[derive(Debug, Clone)]
struct CpuRunQueue {
    ready: [VecDeque<ProcessId>; SCHEDULER_PRIORITY_LEVELS],
    current: Option<ProcessId>,
    quantum_remaining_ns: u64,
    switches: u64,
    preemptions: u64,
}

impl CpuRunQueue {
    fn new() -> Self {
        Self {
            ready: std::array::from_fn(|_| VecDeque::new()),
            current: None,
            quantum_remaining_ns: 0,
            switches: 0,
            preemptions: 0,
        }
    }

    fn depth(&self) -> usize {
        self.ready.iter().map(VecDeque::len).sum::<usize>() + usize::from(self.current.is_some())
    }

    fn highest_ready_priority(&self) -> Option<u8> {
        self.ready
            .iter()
            .position(|queue| !queue.is_empty())
            .map(|priority| priority as u8)
    }
}

#[derive(Debug, Clone, Default)]
struct SchedulerLock {
    owner: Option<ProcessId>,
    waiters: VecDeque<ProcessId>,
}

#[derive(Debug, Clone)]
pub struct MulticoreScheduler {
    cpus: Vec<CpuRunQueue>,
    tasks: BTreeMap<ProcessId, SchedulerTaskInfo>,
    sleepers: BinaryHeap<Reverse<(u64, ProcessId)>>,
    wait_queues: BTreeMap<u64, VecDeque<ProcessId>>,
    locks: BTreeMap<u64, SchedulerLock>,
    pub ticks: u64,
    pub now_ns: u64,
    pub migrations: u64,
    default_quantum_ns: u64,
}

impl MulticoreScheduler {
    pub fn new(cpu_count: usize) -> Result<Self, KernelCoreError> {
        if cpu_count == 0 || cpu_count > 128 {
            return Err(KernelCoreError::InvalidConfiguration(
                "scheduler requires 1..128 CPUs",
            ));
        }
        Ok(Self {
            cpus: (0..cpu_count).map(|_| CpuRunQueue::new()).collect(),
            tasks: BTreeMap::new(),
            sleepers: BinaryHeap::new(),
            wait_queues: BTreeMap::new(),
            locks: BTreeMap::new(),
            ticks: 0,
            now_ns: 0,
            migrations: 0,
            default_quantum_ns: SCHEDULER_DEFAULT_QUANTUM_NS,
        })
    }

    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    pub fn set_default_quantum_ns(&mut self, quantum_ns: u64) -> Result<(), KernelCoreError> {
        if quantum_ns == 0 {
            return Err(KernelCoreError::InvalidConfiguration(
                "scheduler quantum must be non-zero",
            ));
        }
        self.default_quantum_ns = quantum_ns;
        Ok(())
    }

    pub fn register(
        &mut self,
        pid: ProcessId,
        priority: u8,
        affinity_mask: u128,
    ) -> Result<(), KernelCoreError> {
        self.validate_priority(priority)?;
        self.validate_affinity(affinity_mask)?;
        if self.tasks.contains_key(&pid) {
            return Err(KernelCoreError::SchedulerTaskExists(pid));
        }
        self.tasks.insert(
            pid,
            SchedulerTaskInfo {
                pid,
                base_priority: priority,
                effective_priority: priority,
                affinity_mask,
                state: SchedulerTaskState::Blocked,
                cpu: None,
                wake_deadline_ns: None,
                runtime_ns: 0,
            },
        );
        Ok(())
    }

    pub fn enqueue(
        &mut self,
        pid: ProcessId,
        affinity: Option<usize>,
    ) -> Result<usize, KernelCoreError> {
        let mask = self.single_affinity_mask(affinity)?;
        if !self.tasks.contains_key(&pid) {
            self.register(pid, SCHEDULER_DEFAULT_PRIORITY, mask)?;
        } else if affinity.is_some() {
            self.set_affinity(pid, mask)?;
        }
        self.make_ready(pid)
    }

    pub fn enqueue_with_priority(
        &mut self,
        pid: ProcessId,
        priority: u8,
        affinity_mask: u128,
    ) -> Result<usize, KernelCoreError> {
        if self.tasks.contains_key(&pid) {
            self.set_priority(pid, priority)?;
            self.set_affinity(pid, affinity_mask)?;
        } else {
            self.register(pid, priority, affinity_mask)?;
        }
        self.make_ready(pid)
    }

    pub fn set_priority(&mut self, pid: ProcessId, priority: u8) -> Result<(), KernelCoreError> {
        self.validate_priority(priority)?;
        let was_ready = self
            .tasks
            .get(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?
            .state
            == SchedulerTaskState::Ready;
        self.remove_from_cpu_queues(pid);
        let task = self.tasks.get_mut(&pid).unwrap();
        task.base_priority = priority;
        task.effective_priority = priority;
        if was_ready {
            self.make_ready(pid)?;
        }
        Ok(())
    }

    pub fn set_affinity(
        &mut self,
        pid: ProcessId,
        affinity_mask: u128,
    ) -> Result<(), KernelCoreError> {
        self.validate_affinity(affinity_mask)?;
        let state = self
            .tasks
            .get(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?
            .state;
        self.remove_from_cpu_queues(pid);
        let task = self.tasks.get_mut(&pid).unwrap();
        task.affinity_mask = affinity_mask;
        task.cpu = None;
        if state == SchedulerTaskState::Ready || state == SchedulerTaskState::Running {
            self.make_ready(pid)?;
        }
        Ok(())
    }

    pub fn schedule(&mut self, cpu: usize) -> Result<Option<ProcessId>, KernelCoreError> {
        self.validate_cpu(cpu)?;
        if let Some(current) = self.cpus[cpu].current.take() {
            if let Some(task) = self.tasks.get_mut(&current) {
                task.state = SchedulerTaskState::Ready;
                task.cpu = Some(cpu);
                let priority = task.effective_priority as usize;
                self.cpus[cpu].ready[priority].push_back(current);
            }
        }
        self.dispatch(cpu)
    }

    pub fn timer_tick(
        &mut self,
        cpu: usize,
        elapsed_ns: u64,
    ) -> Result<Option<ProcessId>, KernelCoreError> {
        self.validate_cpu(cpu)?;
        if elapsed_ns == 0 {
            return Ok(self.cpus[cpu].current);
        }
        self.now_ns = self.now_ns.saturating_add(elapsed_ns);
        self.ticks = self.ticks.wrapping_add(1);
        self.wake_expired()?;
        let current = self.cpus[cpu].current;
        if let Some(pid) = current {
            if let Some(task) = self.tasks.get_mut(&pid) {
                task.runtime_ns = task.runtime_ns.saturating_add(elapsed_ns);
            }
            self.cpus[cpu].quantum_remaining_ns = self.cpus[cpu]
                .quantum_remaining_ns
                .saturating_sub(elapsed_ns);
        }
        let higher_ready = match (
            self.cpus[cpu].highest_ready_priority(),
            current.and_then(|pid| self.tasks.get(&pid).map(|task| task.effective_priority)),
        ) {
            (Some(ready), Some(running)) => ready < running,
            (Some(_), None) => true,
            _ => false,
        };
        if current.is_none() || higher_ready || self.cpus[cpu].quantum_remaining_ns == 0 {
            if current.is_some() {
                self.cpus[cpu].preemptions = self.cpus[cpu].preemptions.wrapping_add(1);
            }
            self.schedule(cpu)
        } else {
            Ok(current)
        }
    }

    pub fn sleep(&mut self, pid: ProcessId, duration_ns: u64) -> Result<u64, KernelCoreError> {
        if duration_ns == 0 {
            return Err(KernelCoreError::InvalidConfiguration(
                "scheduler sleep duration must be non-zero",
            ));
        }
        let deadline = self.now_ns.saturating_add(duration_ns);
        self.remove_from_cpu_queues(pid);
        let task = self
            .tasks
            .get_mut(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?;
        task.state = SchedulerTaskState::Sleeping;
        task.cpu = None;
        task.wake_deadline_ns = Some(deadline);
        self.sleepers.push(Reverse((deadline, pid)));
        Ok(deadline)
    }

    pub fn wake(&mut self, pid: ProcessId) -> Result<usize, KernelCoreError> {
        let task = self
            .tasks
            .get_mut(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?;
        task.wake_deadline_ns = None;
        self.make_ready(pid)
    }

    pub fn block_on(&mut self, pid: ProcessId, wait_queue: u64) -> Result<(), KernelCoreError> {
        self.remove_from_cpu_queues(pid);
        let task = self
            .tasks
            .get_mut(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?;
        task.state = SchedulerTaskState::Blocked;
        task.cpu = None;
        let queue = self.wait_queues.entry(wait_queue).or_default();
        if !queue.contains(&pid) {
            queue.push_back(pid);
        }
        Ok(())
    }

    pub fn wake_one(&mut self, wait_queue: u64) -> Result<Option<ProcessId>, KernelCoreError> {
        let pid = self
            .wait_queues
            .get_mut(&wait_queue)
            .and_then(VecDeque::pop_front);
        if let Some(pid) = pid {
            self.make_ready(pid)?;
        }
        Ok(pid)
    }

    pub fn wake_all(&mut self, wait_queue: u64) -> Result<Vec<ProcessId>, KernelCoreError> {
        let pids = self
            .wait_queues
            .remove(&wait_queue)
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        for pid in &pids {
            self.make_ready(*pid)?;
        }
        Ok(pids)
    }

    pub fn lock(&mut self, lock: u64, pid: ProcessId) -> Result<bool, KernelCoreError> {
        let task_priority = self
            .tasks
            .get(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?
            .effective_priority;
        let owner = self.locks.entry(lock).or_default().owner;
        match owner {
            None => {
                self.locks.get_mut(&lock).unwrap().owner = Some(pid);
                Ok(true)
            }
            Some(owner) if owner == pid => Err(KernelCoreError::InvalidConfiguration(
                "kernel mutexes are not recursive",
            )),
            Some(owner) => {
                let owner_was_ready = self
                    .tasks
                    .get(&owner)
                    .is_some_and(|task| task.state == SchedulerTaskState::Ready);
                if let Some(owner_task) = self.tasks.get_mut(&owner) {
                    owner_task.effective_priority =
                        owner_task.effective_priority.min(task_priority);
                }
                if owner_was_ready {
                    self.make_ready(owner)?;
                }
                self.block_on(pid, lock)?;
                let waiters = &mut self.locks.get_mut(&lock).unwrap().waiters;
                if !waiters.contains(&pid) {
                    waiters.push_back(pid);
                }
                Ok(false)
            }
        }
    }

    pub fn unlock(
        &mut self,
        lock: u64,
        pid: ProcessId,
    ) -> Result<Option<ProcessId>, KernelCoreError> {
        let state = self
            .locks
            .get_mut(&lock)
            .ok_or(KernelCoreError::LockNotOwned { lock, pid })?;
        if state.owner != Some(pid) {
            return Err(KernelCoreError::LockNotOwned { lock, pid });
        }
        let next = state.waiters.pop_front();
        state.owner = next;
        let owner_was_ready = self
            .tasks
            .get(&pid)
            .is_some_and(|task| task.state == SchedulerTaskState::Ready);
        if let Some(owner) = self.tasks.get_mut(&pid) {
            owner.effective_priority = owner.base_priority;
        }
        if owner_was_ready {
            self.make_ready(pid)?;
        }
        if let Some(next) = next {
            if let Some(queue) = self.wait_queues.get_mut(&lock) {
                queue.retain(|waiter| *waiter != next);
            }
            self.make_ready(next)?;
        }
        Ok(next)
    }

    pub fn rebalance(&mut self) -> Result<usize, KernelCoreError> {
        let mut moved = 0;
        loop {
            let Some((source, source_depth)) = self
                .cpus
                .iter()
                .enumerate()
                .map(|(cpu, queue)| (cpu, queue.depth()))
                .max_by_key(|(_, depth)| *depth)
            else {
                break;
            };
            let (destination, destination_depth) = self
                .cpus
                .iter()
                .enumerate()
                .map(|(cpu, queue)| (cpu, queue.depth()))
                .min_by_key(|(_, depth)| *depth)
                .unwrap();
            if source == destination || source_depth <= destination_depth + 1 {
                break;
            }
            let mut candidate = None;
            for priority in (0..SCHEDULER_PRIORITY_LEVELS).rev() {
                if let Some(position) = self.cpus[source].ready[priority].iter().position(|pid| {
                    self.tasks
                        .get(pid)
                        .is_some_and(|task| task.affinity_mask & (1u128 << destination) != 0)
                }) {
                    candidate = self.cpus[source].ready[priority].remove(position);
                    break;
                }
            }
            let Some(pid) = candidate else {
                break;
            };
            let priority = self.tasks[&pid].effective_priority as usize;
            self.tasks.get_mut(&pid).unwrap().cpu = Some(destination);
            self.cpus[destination].ready[priority].push_back(pid);
            self.migrations = self.migrations.wrapping_add(1);
            moved += 1;
        }
        Ok(moved)
    }

    pub fn current(&self, cpu: usize) -> Result<Option<ProcessId>, KernelCoreError> {
        Ok(self
            .cpus
            .get(cpu)
            .ok_or(KernelCoreError::CpuOutOfRange(cpu))?
            .current)
    }

    pub fn task(&self, pid: ProcessId) -> Result<&SchedulerTaskInfo, KernelCoreError> {
        self.tasks
            .get(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))
    }

    pub fn remove(&mut self, pid: ProcessId) {
        self.remove_from_cpu_queues(pid);
        for queue in self.wait_queues.values_mut() {
            queue.retain(|candidate| *candidate != pid);
        }
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.state = SchedulerTaskState::Blocked;
            task.cpu = None;
        }
    }

    pub fn unregister(&mut self, pid: ProcessId) {
        self.remove(pid);
        self.tasks.remove(&pid);
    }

    pub fn switches(&self, cpu: usize) -> Result<u64, KernelCoreError> {
        Ok(self
            .cpus
            .get(cpu)
            .ok_or(KernelCoreError::CpuOutOfRange(cpu))?
            .switches)
    }

    pub fn preemptions(&self, cpu: usize) -> Result<u64, KernelCoreError> {
        Ok(self
            .cpus
            .get(cpu)
            .ok_or(KernelCoreError::CpuOutOfRange(cpu))?
            .preemptions)
    }

    pub fn queue_depth(&self, cpu: usize) -> Result<usize, KernelCoreError> {
        Ok(self
            .cpus
            .get(cpu)
            .ok_or(KernelCoreError::CpuOutOfRange(cpu))?
            .depth())
    }

    fn dispatch(&mut self, cpu: usize) -> Result<Option<ProcessId>, KernelCoreError> {
        for priority in 0..SCHEDULER_PRIORITY_LEVELS {
            while let Some(pid) = self.cpus[cpu].ready[priority].pop_front() {
                let Some(task) = self.tasks.get_mut(&pid) else {
                    continue;
                };
                if task.state != SchedulerTaskState::Ready
                    || task.affinity_mask & (1u128 << cpu) == 0
                {
                    continue;
                }
                task.state = SchedulerTaskState::Running;
                task.cpu = Some(cpu);
                self.cpus[cpu].current = Some(pid);
                self.cpus[cpu].quantum_remaining_ns = self.default_quantum_ns;
                self.cpus[cpu].switches = self.cpus[cpu].switches.wrapping_add(1);
                return Ok(Some(pid));
            }
        }
        self.cpus[cpu].current = None;
        self.cpus[cpu].quantum_remaining_ns = 0;
        Ok(None)
    }

    fn make_ready(&mut self, pid: ProcessId) -> Result<usize, KernelCoreError> {
        self.remove_from_cpu_queues(pid);
        let task = self
            .tasks
            .get(&pid)
            .ok_or(KernelCoreError::SchedulerTaskMissing(pid))?;
        let cpu = self.select_cpu(task.affinity_mask)?;
        let priority = task.effective_priority as usize;
        let task = self.tasks.get_mut(&pid).unwrap();
        task.state = SchedulerTaskState::Ready;
        task.cpu = Some(cpu);
        task.wake_deadline_ns = None;
        self.cpus[cpu].ready[priority].push_back(pid);
        Ok(cpu)
    }

    fn wake_expired(&mut self) -> Result<(), KernelCoreError> {
        let mut ready = Vec::new();
        while let Some(Reverse((deadline, pid))) = self.sleepers.peek().copied() {
            if deadline > self.now_ns {
                break;
            }
            self.sleepers.pop();
            if self.tasks.get(&pid).is_some_and(|task| {
                task.state == SchedulerTaskState::Sleeping
                    && task.wake_deadline_ns == Some(deadline)
            }) {
                ready.push(pid);
            }
        }
        for pid in ready {
            self.make_ready(pid)?;
        }
        Ok(())
    }

    fn remove_from_cpu_queues(&mut self, pid: ProcessId) {
        for queue in &mut self.cpus {
            for priority in &mut queue.ready {
                priority.retain(|candidate| *candidate != pid);
            }
            if queue.current == Some(pid) {
                queue.current = None;
                queue.quantum_remaining_ns = 0;
            }
        }
    }

    fn select_cpu(&self, affinity_mask: u128) -> Result<usize, KernelCoreError> {
        self.cpus
            .iter()
            .enumerate()
            .filter(|(cpu, _)| affinity_mask & (1u128 << cpu) != 0)
            .min_by_key(|(_, queue)| queue.depth())
            .map(|(cpu, _)| cpu)
            .ok_or(KernelCoreError::InvalidAffinity)
    }

    fn single_affinity_mask(&self, affinity: Option<usize>) -> Result<u128, KernelCoreError> {
        match affinity {
            Some(cpu) if cpu < self.cpus.len() => Ok(1u128 << cpu),
            Some(cpu) => Err(KernelCoreError::CpuOutOfRange(cpu)),
            None if self.cpus.len() == 128 => Ok(u128::MAX),
            None => Ok((1u128 << self.cpus.len()) - 1),
        }
    }

    fn validate_cpu(&self, cpu: usize) -> Result<(), KernelCoreError> {
        if cpu < self.cpus.len() {
            Ok(())
        } else {
            Err(KernelCoreError::CpuOutOfRange(cpu))
        }
    }

    fn validate_priority(&self, priority: u8) -> Result<(), KernelCoreError> {
        if usize::from(priority) < SCHEDULER_PRIORITY_LEVELS {
            Ok(())
        } else {
            Err(KernelCoreError::InvalidPriority(priority))
        }
    }

    fn validate_affinity(&self, affinity_mask: u128) -> Result<(), KernelCoreError> {
        let online = if self.cpus.len() == 128 {
            u128::MAX
        } else {
            (1u128 << self.cpus.len()) - 1
        };
        if affinity_mask != 0 && affinity_mask & !online == 0 {
            Ok(())
        } else {
            Err(KernelCoreError::InvalidAffinity)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CoreSyscall {
    GetPid = 0,
    Yield = 1,
    Exit = 2,
    MapAnonymous = 3,
    Unmap = 4,
}

impl TryFrom<u32> for CoreSyscall {
    type Error = KernelCoreError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::GetPid),
            1 => Ok(Self::Yield),
            2 => Ok(Self::Exit),
            3 => Ok(Self::MapAnonymous),
            4 => Ok(Self::Unmap),
            other => Err(KernelCoreError::InvalidSyscall(other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KernelCore {
    pub memory: PhysicalMemory,
    pub processes: BTreeMap<ProcessId, Process>,
    pub scheduler: MulticoreScheduler,
    next_pid: ProcessId,
}

impl KernelCore {
    pub fn new(frame_count: usize, cpu_count: usize) -> Result<Self, KernelCoreError> {
        Ok(Self {
            memory: PhysicalMemory::new(frame_count)?,
            processes: BTreeMap::new(),
            scheduler: MulticoreScheduler::new(cpu_count)?,
            next_pid: 1,
        })
    }

    pub fn load_process(
        &mut self,
        name: impl Into<String>,
        image: &[u8],
        expected_machine: Option<u16>,
        affinity: Option<usize>,
    ) -> Result<ProcessId, KernelCoreError> {
        let name = name.into();
        let parsed = parse_user_elf(image, expected_machine)?;
        let mut options = UserLoadOptions::for_class(parsed.class);
        options.argv = vec![name.clone()];
        self.load_process_configured(
            name,
            image,
            expected_machine,
            affinity,
            SCHEDULER_DEFAULT_PRIORITY,
            options,
        )
    }

    pub fn load_process_configured(
        &mut self,
        name: impl Into<String>,
        image: &[u8],
        expected_machine: Option<u16>,
        affinity: Option<usize>,
        priority: u8,
        options: UserLoadOptions,
    ) -> Result<ProcessId, KernelCoreError> {
        let name = name.into();
        let affinity_mask = match affinity {
            Some(cpu) if cpu < self.scheduler.cpu_count() => 1u128 << cpu,
            Some(cpu) => return Err(KernelCoreError::CpuOutOfRange(cpu)),
            None if self.scheduler.cpu_count() == 128 => u128::MAX,
            None => (1u128 << self.scheduler.cpu_count()) - 1,
        };
        let pid = self.next_pid;
        self.next_pid = self
            .next_pid
            .checked_add(1)
            .ok_or(KernelCoreError::AddressOverflow)?;
        let mut address_space = AddressSpace::default();
        let loaded = match load_user_elf(
            image,
            expected_machine,
            &options,
            &mut address_space,
            &mut self.memory,
        ) {
            Ok(loaded) => loaded,
            Err(error) => {
                let _ = address_space.release_all(&mut self.memory);
                return Err(error);
            }
        };
        let process = Process {
            id: pid,
            parent: None,
            name,
            state: ProcessState::Ready,
            priority,
            affinity,
            affinity_mask,
            context: UserContext {
                instruction_pointer: loaded.entry,
                stack_pointer: loaded.stack_pointer,
                thread_pointer: loaded.thread_pointer,
                ..UserContext::default()
            },
            address_space,
            loaded_image: Some(loaded),
            exit_code: None,
            open_handles: BTreeMap::new(),
        };
        self.processes.insert(pid, process);
        if let Err(error) = self
            .scheduler
            .enqueue_with_priority(pid, priority, affinity_mask)
        {
            if let Some(mut process) = self.processes.remove(&pid) {
                let _ = process.address_space.release_all(&mut self.memory);
            }
            return Err(error);
        }
        Ok(pid)
    }

    pub fn fork_cow(&mut self, parent: ProcessId) -> Result<ProcessId, KernelCoreError> {
        let child_id = self.next_pid;
        self.next_pid = self
            .next_pid
            .checked_add(1)
            .ok_or(KernelCoreError::AddressOverflow)?;
        let parent_process = self
            .processes
            .get_mut(&parent)
            .ok_or(KernelCoreError::ProcessMissing(parent))?;
        let child_space = parent_process.address_space.fork_cow(&mut self.memory)?;
        let mut child_context = parent_process.context.clone();
        child_context.registers[0] = 0;
        parent_process.context.registers[0] = child_id as u64;
        let child = Process {
            id: child_id,
            parent: Some(parent),
            name: format!("{}:{child_id}", parent_process.name),
            state: ProcessState::Ready,
            priority: parent_process.priority,
            affinity: parent_process.affinity,
            affinity_mask: parent_process.affinity_mask,
            context: child_context,
            address_space: child_space,
            loaded_image: parent_process.loaded_image.clone(),
            exit_code: None,
            open_handles: parent_process.open_handles.clone(),
        };
        let priority = child.priority;
        let affinity_mask = child.affinity_mask;
        self.processes.insert(child_id, child);
        self.scheduler
            .enqueue_with_priority(child_id, priority, affinity_mask)?;
        Ok(child_id)
    }

    pub fn copy_from_user(
        &self,
        pid: ProcessId,
        address: u64,
        output: &mut [u8],
    ) -> Result<(), KernelCoreError> {
        self.processes
            .get(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?
            .address_space
            .read(&self.memory, address, output)
    }

    pub fn copy_to_user(
        &mut self,
        pid: ProcessId,
        address: u64,
        input: &[u8],
    ) -> Result<(), KernelCoreError> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        process
            .address_space
            .write(&mut self.memory, address, input)
    }

    pub fn set_state(
        &mut self,
        pid: ProcessId,
        state: ProcessState,
    ) -> Result<(), KernelCoreError> {
        let (priority, affinity_mask) = self
            .processes
            .get(&pid)
            .map(|process| (process.priority, process.affinity_mask))
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        if state == ProcessState::Blocked || state == ProcessState::Zombie {
            if state == ProcessState::Zombie {
                self.scheduler.unregister(pid);
            } else {
                self.scheduler.remove(pid);
            }
        } else if state == ProcessState::Ready {
            self.scheduler
                .enqueue_with_priority(pid, priority, affinity_mask)?;
        }
        self.processes.get_mut(&pid).unwrap().state = state;
        Ok(())
    }

    pub fn exit_process(&mut self, pid: ProcessId, code: i32) -> Result<(), KernelCoreError> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        process.address_space.release_all(&mut self.memory)?;
        process.state = ProcessState::Zombie;
        process.exit_code = Some(code);
        self.scheduler.unregister(pid);
        Ok(())
    }

    pub fn schedule_cpu(&mut self, cpu: usize) -> Result<Option<ProcessId>, KernelCoreError> {
        let next = self.scheduler.schedule(cpu)?;
        self.sync_scheduler_states();
        Ok(next)
    }

    pub fn timer_interrupt(
        &mut self,
        cpu: usize,
        elapsed_ns: u64,
    ) -> Result<Option<ProcessId>, KernelCoreError> {
        let next = self.scheduler.timer_tick(cpu, elapsed_ns)?;
        self.sync_scheduler_states();
        Ok(next)
    }

    pub fn sleep_process(
        &mut self,
        pid: ProcessId,
        duration_ns: u64,
    ) -> Result<u64, KernelCoreError> {
        self.processes
            .get(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        let deadline = self.scheduler.sleep(pid, duration_ns)?;
        self.sync_scheduler_states();
        Ok(deadline)
    }

    pub fn wake_process(&mut self, pid: ProcessId) -> Result<usize, KernelCoreError> {
        self.processes
            .get(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        let cpu = self.scheduler.wake(pid)?;
        self.sync_scheduler_states();
        Ok(cpu)
    }

    pub fn wait_process(&mut self, pid: ProcessId, queue: u64) -> Result<(), KernelCoreError> {
        self.processes
            .get(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        self.scheduler.block_on(pid, queue)?;
        self.sync_scheduler_states();
        Ok(())
    }

    pub fn wake_one_process(&mut self, queue: u64) -> Result<Option<ProcessId>, KernelCoreError> {
        let pid = self.scheduler.wake_one(queue)?;
        self.sync_scheduler_states();
        Ok(pid)
    }

    pub fn set_process_priority(
        &mut self,
        pid: ProcessId,
        priority: u8,
    ) -> Result<(), KernelCoreError> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        self.scheduler.set_priority(pid, priority)?;
        process.priority = priority;
        Ok(())
    }

    pub fn set_process_affinity(
        &mut self,
        pid: ProcessId,
        affinity_mask: u128,
    ) -> Result<(), KernelCoreError> {
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or(KernelCoreError::ProcessMissing(pid))?;
        self.scheduler.set_affinity(pid, affinity_mask)?;
        process.affinity_mask = affinity_mask;
        process.affinity = affinity_mask
            .is_power_of_two()
            .then(|| affinity_mask.trailing_zeros() as usize);
        Ok(())
    }

    pub fn lock_process(&mut self, lock: u64, pid: ProcessId) -> Result<bool, KernelCoreError> {
        let acquired = self.scheduler.lock(lock, pid)?;
        self.sync_scheduler_states();
        Ok(acquired)
    }

    pub fn unlock_process(
        &mut self,
        lock: u64,
        pid: ProcessId,
    ) -> Result<Option<ProcessId>, KernelCoreError> {
        let next = self.scheduler.unlock(lock, pid)?;
        self.sync_scheduler_states();
        Ok(next)
    }

    pub fn rebalance(&mut self) -> Result<usize, KernelCoreError> {
        self.scheduler.rebalance()
    }

    fn sync_scheduler_states(&mut self) {
        for process in self.processes.values_mut() {
            if process.state == ProcessState::Zombie {
                continue;
            }
            process.state = match self.scheduler.tasks.get(&process.id).map(|task| task.state) {
                Some(SchedulerTaskState::Running) => ProcessState::Running,
                Some(SchedulerTaskState::Ready) => ProcessState::Ready,
                Some(SchedulerTaskState::Blocked | SchedulerTaskState::Sleeping) | None => {
                    ProcessState::Blocked
                }
            };
        }
    }

    pub fn syscall(
        &mut self,
        pid: ProcessId,
        number: u32,
        arguments: [u64; 6],
    ) -> Result<i64, KernelCoreError> {
        let syscall = CoreSyscall::try_from(number)?;
        match syscall {
            CoreSyscall::GetPid => Ok(pid as i64),
            CoreSyscall::Yield => {
                self.processes
                    .get(&pid)
                    .ok_or(KernelCoreError::ProcessMissing(pid))?;
                let cpu = self.scheduler.task(pid)?.cpu.unwrap_or(0);
                self.schedule_cpu(cpu)?;
                Ok(0)
            }
            CoreSyscall::Exit => {
                self.exit_process(pid, arguments[0] as i32)?;
                Ok(0)
            }
            CoreSyscall::MapAnonymous => {
                let address = arguments[0];
                let process = self
                    .processes
                    .get_mut(&pid)
                    .ok_or(KernelCoreError::ProcessMissing(pid))?;
                process.address_space.map_zeroed(
                    &mut self.memory,
                    address,
                    PageFlags::READ
                        .union(PageFlags::WRITE)
                        .union(PageFlags::USER),
                )?;
                Ok(address as i64)
            }
            CoreSyscall::Unmap => {
                let process = self
                    .processes
                    .get_mut(&pid)
                    .ok_or(KernelCoreError::ProcessMissing(pid))?;
                process
                    .address_space
                    .unmap(&mut self.memory, arguments[0])?;
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elf_loader_maps_executable_and_user_stack() {
        let image = build_test_elf64(62, 0x400000, &[0x31, 0xc0, 0xc3]);
        let mut kernel = KernelCore::new(64, 2).unwrap();
        let pid = kernel
            .load_process("init", &image, Some(62), Some(0))
            .unwrap();
        let process = &kernel.processes[&pid];
        assert_eq!(process.context.instruction_pointer, 0x400000);
        assert_eq!(process.address_space.mapped_pages(), 17);
        let mut code = [0u8; 3];
        process
            .address_space
            .read_execute(&kernel.memory, 0x400000, &mut code)
            .unwrap();
        assert_eq!(code, [0x31, 0xc0, 0xc3]);
        assert!(matches!(
            kernel.copy_to_user(pid, 0x400000, &[0x90]),
            Err(KernelCoreError::ProtectionFault { .. })
        ));
    }

    #[test]
    fn fork_cow_materializes_private_page_on_first_write() {
        let image = build_test_elf64(62, 0x400000, &[0xc3]);
        let mut kernel = KernelCore::new(64, 2).unwrap();
        let parent = kernel
            .load_process("parent", &image, Some(62), None)
            .unwrap();
        kernel
            .syscall(
                parent,
                CoreSyscall::MapAnonymous as u32,
                [0x600000, 0, 0, 0, 0, 0],
            )
            .unwrap();
        kernel.copy_to_user(parent, 0x600000, b"parent").unwrap();
        let shared = kernel.processes[&parent]
            .address_space
            .mapping(0x600000)
            .unwrap()
            .frame;
        let child = kernel.fork_cow(parent).unwrap();
        assert_eq!(kernel.memory.references(shared).unwrap(), 2);
        kernel.copy_to_user(child, 0x600000, b"child!").unwrap();
        let mut parent_bytes = [0; 6];
        let mut child_bytes = [0; 6];
        kernel
            .copy_from_user(parent, 0x600000, &mut parent_bytes)
            .unwrap();
        kernel
            .copy_from_user(child, 0x600000, &mut child_bytes)
            .unwrap();
        assert_eq!(&parent_bytes, b"parent");
        assert_eq!(&child_bytes, b"child!");
        assert_ne!(
            kernel.processes[&parent]
                .address_space
                .mapping(0x600000)
                .unwrap()
                .frame,
            kernel.processes[&child]
                .address_space
                .mapping(0x600000)
                .unwrap()
                .frame
        );
    }

    #[test]
    fn multicore_scheduler_balances_blocks_wakes_and_counts_switches() {
        let image = build_test_elf64(62, 0x400000, &[0xc3]);
        let mut kernel = KernelCore::new(96, 2).unwrap();
        let a = kernel.load_process("a", &image, Some(62), None).unwrap();
        let b = kernel.load_process("b", &image, Some(62), None).unwrap();
        assert_eq!(kernel.scheduler.schedule(0).unwrap(), Some(a));
        assert_eq!(kernel.scheduler.schedule(1).unwrap(), Some(b));
        kernel.set_state(a, ProcessState::Blocked).unwrap();
        assert_eq!(kernel.scheduler.current(0).unwrap(), None);
        kernel.set_state(a, ProcessState::Ready).unwrap();
        assert_eq!(kernel.scheduler.schedule(0).unwrap(), Some(a));
        assert_eq!(kernel.scheduler.switches(0).unwrap(), 2);
    }

    #[test]
    fn scheduler_preempts_by_priority_and_wakes_deadline_sleepers() {
        let mut scheduler = MulticoreScheduler::new(2).unwrap();
        scheduler.enqueue_with_priority(10, 20, 0b01).unwrap();
        assert_eq!(scheduler.schedule(0).unwrap(), Some(10));
        scheduler.enqueue_with_priority(11, 3, 0b01).unwrap();
        assert_eq!(scheduler.timer_tick(0, 1).unwrap(), Some(11));
        assert_eq!(scheduler.preemptions(0).unwrap(), 1);
        assert_eq!(scheduler.timer_tick(0, 1).unwrap(), Some(11));
        scheduler.sleep(11, 2_000).unwrap();
        assert_eq!(scheduler.schedule(0).unwrap(), Some(10));
        assert_eq!(scheduler.timer_tick(0, 2_000).unwrap(), Some(11));
        assert_eq!(
            scheduler.task(11).unwrap().state,
            SchedulerTaskState::Running
        );
        assert!(scheduler.task(11).unwrap().runtime_ns >= 1);
    }

    #[test]
    fn scheduler_wait_queues_locks_affinity_and_balance_are_material() {
        let mut scheduler = MulticoreScheduler::new(4).unwrap();
        scheduler.enqueue_with_priority(1, 18, 0b1111).unwrap();
        scheduler.enqueue_with_priority(2, 2, 0b1111).unwrap();
        scheduler.enqueue_with_priority(3, 12, 0b1111).unwrap();
        assert!(scheduler.lock(0x44, 1).unwrap());
        assert!(!scheduler.lock(0x44, 2).unwrap());
        assert_eq!(scheduler.task(1).unwrap().effective_priority, 2);
        assert_eq!(
            scheduler.task(2).unwrap().state,
            SchedulerTaskState::Blocked
        );
        assert_eq!(scheduler.unlock(0x44, 1).unwrap(), Some(2));
        assert_eq!(scheduler.task(1).unwrap().effective_priority, 18);
        assert_eq!(scheduler.task(2).unwrap().state, SchedulerTaskState::Ready);

        scheduler.block_on(3, 0x99).unwrap();
        assert_eq!(scheduler.wake_one(0x99).unwrap(), Some(3));
        assert_eq!(scheduler.task(3).unwrap().state, SchedulerTaskState::Ready);
        scheduler.set_affinity(3, 0b1000).unwrap();
        assert_eq!(scheduler.task(3).unwrap().cpu, Some(3));

        scheduler.register(4, 24, 0b1111).unwrap();
        scheduler.tasks.get_mut(&4).unwrap().state = SchedulerTaskState::Ready;
        scheduler.tasks.get_mut(&4).unwrap().cpu = Some(0);
        scheduler.cpus[0].ready[24].push_back(4);
        scheduler.register(5, 24, 0b1111).unwrap();
        scheduler.tasks.get_mut(&5).unwrap().state = SchedulerTaskState::Ready;
        scheduler.tasks.get_mut(&5).unwrap().cpu = Some(0);
        scheduler.cpus[0].ready[24].push_back(5);
        assert!(scheduler.rebalance().unwrap() >= 1);
        assert!(scheduler.migrations >= 1);
    }

    #[test]
    fn core_syscall_contract_maps_unmaps_yields_and_exits() {
        let image = build_test_elf64(62, 0x400000, &[0xc3]);
        let mut kernel = KernelCore::new(64, 1).unwrap();
        let pid = kernel
            .load_process("sys", &image, Some(62), Some(0))
            .unwrap();
        assert_eq!(kernel.syscall(pid, 0, [0; 6]).unwrap(), pid as i64);
        assert_eq!(
            kernel.syscall(pid, 3, [0x700000, 0, 0, 0, 0, 0]).unwrap(),
            0x700000
        );
        assert_eq!(
            kernel.syscall(pid, 4, [0x700000, 0, 0, 0, 0, 0]).unwrap(),
            0
        );
        assert_eq!(kernel.syscall(pid, 2, [17, 0, 0, 0, 0, 0]).unwrap(), 0);
        assert_eq!(kernel.processes[&pid].exit_code, Some(17));
    }

    #[test]
    fn malformed_elf_is_rejected_before_any_mapping() {
        let mut image = build_test_elf64(62, 0x400000, &[0xc3]);
        image[104..112].copy_from_slice(&0u64.to_le_bytes());
        assert!(matches!(
            parse_elf(&image, Some(62)),
            Err(KernelCoreError::InvalidElf(_))
        ));
    }
}
