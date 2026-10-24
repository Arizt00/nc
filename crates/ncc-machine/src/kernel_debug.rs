use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

pub const KERNEL_DEBUG_SCHEMA: &str = "ncc.machine.kernel_debug.v1";
pub const CRASH_MAGIC: &[u8; 8] = b"NCCRASH1";
pub const MAX_GENERAL_REGISTERS: usize = 32;
pub const MAX_DEBUG_REGISTERS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugArchitecture {
    X86_64,
    I386,
    Aarch64,
    Armv7Thumb,
    Riscv32,
    Riscv64,
}

impl DebugArchitecture {
    pub const fn pointer_width(self) -> usize {
        match self {
            Self::I386 | Self::Armv7Thumb | Self::Riscv32 => 4,
            Self::X86_64 | Self::Aarch64 | Self::Riscv64 => 8,
        }
    }

    pub const fn id(self) -> u16 {
        match self {
            Self::X86_64 => 1,
            Self::I386 => 2,
            Self::Aarch64 => 3,
            Self::Armv7Thumb => 4,
            Self::Riscv32 => 5,
            Self::Riscv64 => 6,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::I386 => "i386",
            Self::Aarch64 => "aarch64",
            Self::Armv7Thumb => "armv7-thumb",
            Self::Riscv32 => "riscv32",
            Self::Riscv64 => "riscv64",
        }
    }

    pub const fn software_breakpoint(self) -> BreakpointOpcode {
        match self {
            Self::X86_64 | Self::I386 => BreakpointOpcode::new([0xcc, 0, 0, 0], 1),
            Self::Aarch64 => BreakpointOpcode::new([0x00, 0x00, 0x20, 0xd4], 4),
            Self::Armv7Thumb => BreakpointOpcode::new([0x00, 0xbe, 0, 0], 2),
            Self::Riscv32 | Self::Riscv64 => BreakpointOpcode::new([0x73, 0x00, 0x10, 0x00], 4),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelDebugError {
    InvalidConfiguration(&'static str),
    AddressOverflow,
    MemoryOutsideWindow { address: u64, bytes: usize },
    OutputTooSmall { required: usize, available: usize },
    InvalidPacket(&'static str),
    ChecksumMismatch { expected: u8, observed: u8 },
    InvalidHex,
    BreakpointExists(u64),
    BreakpointMissing(u64),
    BreakpointCapacity,
    UnsupportedBreakpoint(DebugArchitecture),
    InvalidRegisterCount { provided: usize, maximum: usize },
    SymbolOverlap,
    SymbolMissing(u64),
    PanicAlreadyCaptured,
}

impl fmt::Display for KernelDebugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) | Self::InvalidPacket(message) => {
                f.write_str(message)
            }
            Self::AddressOverflow => f.write_str("debug address arithmetic overflow"),
            Self::MemoryOutsideWindow { address, bytes } => write!(
                f,
                "debug memory access {address:#x}+{bytes:#x} is outside caller-owned memory"
            ),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "debug output requires {required} bytes, caller provided {available}"
            ),
            Self::ChecksumMismatch { expected, observed } => write!(
                f,
                "serial debugger checksum mismatch: expected {expected:02x}, observed {observed:02x}"
            ),
            Self::InvalidHex => f.write_str("serial debugger packet contains invalid hex"),
            Self::BreakpointExists(address) => {
                write!(f, "breakpoint already exists at {address:#x}")
            }
            Self::BreakpointMissing(address) => write!(f, "no breakpoint at {address:#x}"),
            Self::BreakpointCapacity => f.write_str("caller-selected breakpoint capacity reached"),
            Self::UnsupportedBreakpoint(arch) => {
                write!(f, "{} has no selected breakpoint form", arch.name())
            }
            Self::InvalidRegisterCount { provided, maximum } => write!(
                f,
                "register snapshot has {provided} values, maximum is {maximum}"
            ),
            Self::SymbolOverlap => f.write_str("kernel symbol address ranges overlap"),
            Self::SymbolMissing(address) => write!(f, "no symbol covers {address:#x}"),
            Self::PanicAlreadyCaptured => f.write_str("another CPU already owns kernel panic"),
        }
    }
}

impl Error for KernelDebugError {}

pub trait KernelDebugMemory {
    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), KernelDebugError>;
    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), KernelDebugError>;
}

#[derive(Debug)]
pub struct KernelDebugMemoryWindow<'a> {
    base: u64,
    bytes: &'a mut [u8],
}

impl<'a> KernelDebugMemoryWindow<'a> {
    pub fn new(base: u64, bytes: &'a mut [u8]) -> Self {
        Self { base, bytes }
    }

    fn range(&self, address: u64, len: usize) -> Result<std::ops::Range<usize>, KernelDebugError> {
        let offset = address
            .checked_sub(self.base)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(KernelDebugError::MemoryOutsideWindow {
                address,
                bytes: len,
            })?;
        let end = offset
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(KernelDebugError::MemoryOutsideWindow {
                address,
                bytes: len,
            })?;
        Ok(offset..end)
    }
}

impl KernelDebugMemory for KernelDebugMemoryWindow<'_> {
    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), KernelDebugError> {
        output.copy_from_slice(&self.bytes[self.range(address, output.len())?]);
        Ok(())
    }

    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), KernelDebugError> {
        let range = self.range(address, input.len())?;
        self.bytes[range].copy_from_slice(input);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRegisterFile {
    pub architecture: DebugArchitecture,
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub frame_pointer: u64,
    pub status: u64,
    general: [u64; MAX_GENERAL_REGISTERS],
    general_count: usize,
    debug: [u64; MAX_DEBUG_REGISTERS],
    debug_count: usize,
}

impl KernelRegisterFile {
    pub fn new(
        architecture: DebugArchitecture,
        instruction_pointer: u64,
        stack_pointer: u64,
        frame_pointer: u64,
        status: u64,
        general: &[u64],
        debug: &[u64],
    ) -> Result<Self, KernelDebugError> {
        if general.len() > MAX_GENERAL_REGISTERS {
            return Err(KernelDebugError::InvalidRegisterCount {
                provided: general.len(),
                maximum: MAX_GENERAL_REGISTERS,
            });
        }
        if debug.len() > MAX_DEBUG_REGISTERS {
            return Err(KernelDebugError::InvalidRegisterCount {
                provided: debug.len(),
                maximum: MAX_DEBUG_REGISTERS,
            });
        }
        let mut general_values = [0; MAX_GENERAL_REGISTERS];
        general_values[..general.len()].copy_from_slice(general);
        let mut debug_values = [0; MAX_DEBUG_REGISTERS];
        debug_values[..debug.len()].copy_from_slice(debug);
        Ok(Self {
            architecture,
            instruction_pointer,
            stack_pointer,
            frame_pointer,
            status,
            general: general_values,
            general_count: general.len(),
            debug: debug_values,
            debug_count: debug.len(),
        })
    }

    pub fn general(&self) -> &[u64] {
        &self.general[..self.general_count]
    }

    pub fn debug(&self) -> &[u64] {
        &self.debug[..self.debug_count]
    }

    pub fn set_debug(&mut self, index: usize, value: u64) -> Result<(), KernelDebugError> {
        if index >= MAX_DEBUG_REGISTERS {
            return Err(KernelDebugError::InvalidRegisterCount {
                provided: index + 1,
                maximum: MAX_DEBUG_REGISTERS,
            });
        }
        self.debug[index] = value;
        self.debug_count = self.debug_count.max(index + 1);
        Ok(())
    }

    fn rsp_values(&self) -> impl Iterator<Item = u64> + '_ {
        self.general()
            .iter()
            .copied()
            .chain([
                self.stack_pointer,
                self.frame_pointer,
                self.instruction_pointer,
                self.status,
            ])
            .chain(self.debug().iter().copied())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSymbol {
    pub name: String,
    pub address: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KernelSymbolTable {
    symbols: Vec<KernelSymbol>,
}

impl KernelSymbolTable {
    pub fn insert(&mut self, symbol: KernelSymbol) -> Result<(), KernelDebugError> {
        if symbol.name.is_empty() || symbol.size == 0 {
            return Err(KernelDebugError::InvalidConfiguration(
                "kernel symbol requires a name and non-zero size",
            ));
        }
        let end = symbol
            .address
            .checked_add(symbol.size)
            .ok_or(KernelDebugError::AddressOverflow)?;
        if self.symbols.iter().any(|current| {
            let current_end = current.address.saturating_add(current.size);
            symbol.address < current_end && current.address < end
        }) {
            return Err(KernelDebugError::SymbolOverlap);
        }
        self.symbols.push(symbol);
        self.symbols.sort_by_key(|value| value.address);
        Ok(())
    }

    pub fn lookup(&self, address: u64) -> Option<(&KernelSymbol, u64)> {
        self.symbols
            .iter()
            .rev()
            .find(|symbol| {
                address >= symbol.address && address < symbol.address.saturating_add(symbol.size)
            })
            .map(|symbol| (symbol, address - symbol.address))
    }

    pub fn symbols(&self) -> &[KernelSymbol] {
        &self.symbols
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    pub index: usize,
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub frame_pointer: u64,
    pub symbol: Option<String>,
    pub symbol_offset: u64,
}

fn read_pointer(
    memory: &impl KernelDebugMemory,
    address: u64,
    width: usize,
) -> Result<u64, KernelDebugError> {
    let mut bytes = [0u8; 8];
    memory.read(address, &mut bytes[..width])?;
    Ok(if width == 4 {
        u32::from_le_bytes(bytes[..4].try_into().expect("four bytes")) as u64
    } else {
        u64::from_le_bytes(bytes)
    })
}

pub fn unwind_frame_pointer(
    registers: &KernelRegisterFile,
    memory: &impl KernelDebugMemory,
    symbols: &KernelSymbolTable,
    maximum_frames: usize,
) -> Result<Vec<StackFrame>, KernelDebugError> {
    if maximum_frames == 0 {
        return Ok(Vec::new());
    }
    let width = registers.architecture.pointer_width();
    let mut frames = Vec::with_capacity(maximum_frames);
    let mut pc = registers.instruction_pointer;
    let mut sp = registers.stack_pointer;
    let mut fp = registers.frame_pointer;
    let mut visited = BTreeMap::new();
    for index in 0..maximum_frames {
        let (symbol, symbol_offset) = symbols
            .lookup(pc)
            .map(|(symbol, offset)| (Some(symbol.name.clone()), offset))
            .unwrap_or((None, 0));
        frames.push(StackFrame {
            index,
            instruction_pointer: pc,
            stack_pointer: sp,
            frame_pointer: fp,
            symbol,
            symbol_offset,
        });
        if fp == 0 || fp % width as u64 != 0 || visited.insert(fp, ()).is_some() {
            break;
        }
        let previous = read_pointer(memory, fp, width)?;
        let return_address = read_pointer(memory, fp + width as u64, width)?;
        if return_address == 0 || (previous != 0 && previous <= fp) {
            break;
        }
        sp = fp + (width * 2) as u64;
        fp = previous;
        pc = return_address;
    }
    Ok(frames)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointOpcode {
    pub bytes: [u8; 4],
    pub len: usize,
}

impl BreakpointOpcode {
    pub const fn new(bytes: [u8; 4], len: usize) -> Self {
        Self { bytes, len }
    }
}

pub fn debug_trap_opcode(architecture: DebugArchitecture, immediate: u16) -> BreakpointOpcode {
    match architecture {
        DebugArchitecture::Aarch64 => {
            let word = 0xd4200000u32 | ((immediate as u32) << 5);
            BreakpointOpcode::new(word.to_le_bytes(), 4)
        }
        DebugArchitecture::Armv7Thumb => BreakpointOpcode::new([immediate as u8, 0xbe, 0, 0], 2),
        _ => architecture.software_breakpoint(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointPatch {
    pub address: u64,
    pub architecture: DebugArchitecture,
    pub original: [u8; 4],
    pub replacement: [u8; 4],
    pub len: usize,
}

pub fn install_software_breakpoint(
    memory: &mut impl KernelDebugMemory,
    architecture: DebugArchitecture,
    address: u64,
    immediate: u16,
) -> Result<BreakpointPatch, KernelDebugError> {
    let opcode = debug_trap_opcode(architecture, immediate);
    let mut original = [0; 4];
    memory.read(address, &mut original[..opcode.len])?;
    memory.write(address, &opcode.bytes[..opcode.len])?;
    Ok(BreakpointPatch {
        address,
        architecture,
        original,
        replacement: opcode.bytes,
        len: opcode.len,
    })
}

pub fn remove_software_breakpoint(
    memory: &mut impl KernelDebugMemory,
    patch: &BreakpointPatch,
) -> Result<(), KernelDebugError> {
    memory.write(patch.address, &patch.original[..patch.len])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X86HardwareBreakpointKind {
    Execute,
    Write,
    ReadWrite,
}

pub fn configure_x86_hardware_breakpoint(
    registers: &mut KernelRegisterFile,
    slot: usize,
    address: u64,
    kind: X86HardwareBreakpointKind,
    width: usize,
) -> Result<(), KernelDebugError> {
    if !matches!(
        registers.architecture,
        DebugArchitecture::X86_64 | DebugArchitecture::I386
    ) || slot >= 4
        || !matches!(width, 1 | 2 | 4 | 8)
        || (width == 8 && registers.architecture == DebugArchitecture::I386)
    {
        return Err(KernelDebugError::InvalidConfiguration(
            "invalid x86 hardware breakpoint slot or width",
        ));
    }
    let rw = match kind {
        X86HardwareBreakpointKind::Execute => 0,
        X86HardwareBreakpointKind::Write => 1,
        X86HardwareBreakpointKind::ReadWrite => 3,
    };
    let len = match width {
        1 => 0,
        2 => 1,
        4 => 3,
        8 => 2,
        _ => unreachable!(),
    };
    registers.set_debug(slot, address)?;
    let mut dr7 = registers.debug.get(7).copied().unwrap_or(0);
    dr7 |= 1u64 << (slot * 2);
    dr7 &= !(0xfu64 << (16 + slot * 4));
    dr7 |= ((rw | (len << 2)) as u64) << (16 + slot * 4);
    registers.set_debug(7, dr7)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialDebugAction {
    None,
    Continue,
    Step,
    Detach,
    Kill,
}

#[derive(Debug)]
pub struct SerialDebugger {
    architecture: DebugArchitecture,
    breakpoints: Vec<BreakpointPatch>,
    breakpoint_capacity: usize,
    stop_signal: u8,
}

impl SerialDebugger {
    pub fn new(architecture: DebugArchitecture, breakpoint_capacity: usize) -> Self {
        Self {
            architecture,
            breakpoints: Vec::with_capacity(breakpoint_capacity),
            breakpoint_capacity,
            stop_signal: 5,
        }
    }

    pub fn breakpoints(&self) -> &[BreakpointPatch] {
        &self.breakpoints
    }

    pub fn handle_packet(
        &mut self,
        packet: &[u8],
        registers: &KernelRegisterFile,
        memory: &mut impl KernelDebugMemory,
        response: &mut [u8],
    ) -> Result<(usize, SerialDebugAction), KernelDebugError> {
        let payload = decode_rsp_packet(packet)?;
        let text = std::str::from_utf8(payload)
            .map_err(|_| KernelDebugError::InvalidPacket("RSP command is not ASCII"))?;
        let (reply, action) = if text == "?" {
            (
                format!("S{:02x}", self.stop_signal),
                SerialDebugAction::None,
            )
        } else if text.starts_with("qSupported") {
            (
                "PacketSize=1000;swbreak+;hwbreak+;vContSupported+;QStartNoAckMode+".to_string(),
                SerialDebugAction::None,
            )
        } else if text == "qAttached" {
            ("1".to_string(), SerialDebugAction::None)
        } else if text == "qC" {
            ("QC1".to_string(), SerialDebugAction::None)
        } else if text == "vCont?" {
            ("vCont;c;s".to_string(), SerialDebugAction::None)
        } else if text == "g" {
            (encode_registers_hex(registers), SerialDebugAction::None)
        } else if let Some(index) = text.strip_prefix('p') {
            let index =
                usize::from_str_radix(index, 16).map_err(|_| KernelDebugError::InvalidHex)?;
            let value =
                registers
                    .rsp_values()
                    .nth(index)
                    .ok_or(KernelDebugError::InvalidPacket(
                        "RSP register index is out of range",
                    ))?;
            (
                encode_value_hex(value, registers.architecture.pointer_width()),
                SerialDebugAction::None,
            )
        } else if let Some(arguments) = text.strip_prefix('m') {
            let (address, len) = parse_address_len(arguments)?;
            let mut bytes = vec![0; len];
            memory.read(address, &mut bytes)?;
            (encode_hex(&bytes), SerialDebugAction::None)
        } else if let Some(arguments) = text.strip_prefix('M') {
            let (range, encoded) = arguments
                .split_once(':')
                .ok_or(KernelDebugError::InvalidPacket("RSP M packet has no data"))?;
            let (address, len) = parse_address_len(range)?;
            let bytes = decode_hex(encoded.as_bytes())?;
            if bytes.len() != len {
                return Err(KernelDebugError::InvalidPacket(
                    "RSP M packet length does not match data",
                ));
            }
            memory.write(address, &bytes)?;
            ("OK".to_string(), SerialDebugAction::None)
        } else if let Some(arguments) = text.strip_prefix("Z0,") {
            let (address, _) = parse_address_len(arguments)?;
            if self
                .breakpoints
                .iter()
                .any(|value| value.address == address)
            {
                return Err(KernelDebugError::BreakpointExists(address));
            }
            if self.breakpoints.len() == self.breakpoint_capacity {
                return Err(KernelDebugError::BreakpointCapacity);
            }
            let patch = install_software_breakpoint(memory, self.architecture, address, 0)?;
            self.breakpoints.push(patch);
            ("OK".to_string(), SerialDebugAction::None)
        } else if let Some(arguments) = text.strip_prefix("z0,") {
            let (address, _) = parse_address_len(arguments)?;
            let index = self
                .breakpoints
                .iter()
                .position(|value| value.address == address)
                .ok_or(KernelDebugError::BreakpointMissing(address))?;
            let patch = self.breakpoints.remove(index);
            remove_software_breakpoint(memory, &patch)?;
            ("OK".to_string(), SerialDebugAction::None)
        } else if matches!(text, "c" | "vCont;c") {
            ("OK".to_string(), SerialDebugAction::Continue)
        } else if matches!(text, "s" | "vCont;s") {
            ("OK".to_string(), SerialDebugAction::Step)
        } else if text == "D" {
            ("OK".to_string(), SerialDebugAction::Detach)
        } else if text == "k" {
            ("OK".to_string(), SerialDebugAction::Kill)
        } else if text.starts_with('H') || text == "QStartNoAckMode" {
            ("OK".to_string(), SerialDebugAction::None)
        } else {
            (String::new(), SerialDebugAction::None)
        };
        let len = encode_rsp_packet(reply.as_bytes(), response)?;
        Ok((len, action))
    }
}

fn parse_address_len(value: &str) -> Result<(u64, usize), KernelDebugError> {
    let (address, len) = value
        .split_once(',')
        .ok_or(KernelDebugError::InvalidPacket(
            "RSP range requires address,length",
        ))?;
    let address = u64::from_str_radix(address, 16).map_err(|_| KernelDebugError::InvalidHex)?;
    let len = usize::from_str_radix(len, 16).map_err(|_| KernelDebugError::InvalidHex)?;
    Ok((address, len))
}

fn encode_registers_hex(registers: &KernelRegisterFile) -> String {
    registers
        .rsp_values()
        .map(|value| encode_value_hex(value, registers.architecture.pointer_width()))
        .collect()
}

fn encode_value_hex(value: u64, width: usize) -> String {
    encode_hex(&value.to_le_bytes()[..width])
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from_digit((byte >> 4) as u32, 16).expect("hex"));
        output.push(char::from_digit((byte & 0x0f) as u32, 16).expect("hex"));
    }
    output
}

fn decode_hex(bytes: &[u8]) -> Result<Vec<u8>, KernelDebugError> {
    if bytes.len() % 2 != 0 {
        return Err(KernelDebugError::InvalidHex);
    }
    bytes
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_value(pair[0])?;
            let low = hex_value(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_value(value: u8) -> Result<u8, KernelDebugError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(KernelDebugError::InvalidHex),
    }
}

pub fn encode_rsp_packet(payload: &[u8], output: &mut [u8]) -> Result<usize, KernelDebugError> {
    let required = payload.len() + 4;
    if output.len() < required {
        return Err(KernelDebugError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }
    output[0] = b'$';
    output[1..1 + payload.len()].copy_from_slice(payload);
    output[1 + payload.len()] = b'#';
    let checksum = payload
        .iter()
        .fold(0u8, |sum, value| sum.wrapping_add(*value));
    output[2 + payload.len()] = b"0123456789abcdef"[(checksum >> 4) as usize];
    output[3 + payload.len()] = b"0123456789abcdef"[(checksum & 0x0f) as usize];
    Ok(required)
}

pub fn decode_rsp_packet(packet: &[u8]) -> Result<&[u8], KernelDebugError> {
    if packet.len() < 4 || packet[0] != b'$' || packet[packet.len() - 3] != b'#' {
        return Err(KernelDebugError::InvalidPacket("invalid RSP frame"));
    }
    let payload = &packet[1..packet.len() - 3];
    let expected = payload
        .iter()
        .fold(0u8, |sum, value| sum.wrapping_add(*value));
    let observed =
        (hex_value(packet[packet.len() - 2])? << 4) | hex_value(packet[packet.len() - 1])?;
    if expected != observed {
        return Err(KernelDebugError::ChecksumMismatch { expected, observed });
    }
    Ok(payload)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDump {
    pub address: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelCrashReport {
    pub timestamp_ticks: u64,
    pub cpu_id: u32,
    pub reason: String,
    pub registers: KernelRegisterFile,
    pub frames: Vec<StackFrame>,
    pub dumps: Vec<MemoryDump>,
}

impl KernelCrashReport {
    pub fn capture(
        timestamp_ticks: u64,
        cpu_id: u32,
        reason: impl Into<String>,
        registers: KernelRegisterFile,
        memory: &impl KernelDebugMemory,
        symbols: &KernelSymbolTable,
        dump_ranges: &[(u64, usize)],
        maximum_frames: usize,
    ) -> Result<Self, KernelDebugError> {
        let frames = unwind_frame_pointer(&registers, memory, symbols, maximum_frames)?;
        let mut dumps = Vec::with_capacity(dump_ranges.len());
        for (address, len) in dump_ranges {
            let mut bytes = vec![0; *len];
            memory.read(*address, &mut bytes)?;
            dumps.push(MemoryDump {
                address: *address,
                bytes,
            });
        }
        Ok(Self {
            timestamp_ticks,
            cpu_id,
            reason: reason.into(),
            registers,
            frames,
            dumps,
        })
    }

    pub fn binary_len(&self) -> usize {
        8 + 2
            + 2
            + 8
            + 4
            + 4
            + self.reason.len()
            + 32
            + 4
            + (self.registers.general_count + self.registers.debug_count) * 8
            + 4
            + self.frames.len() * 32
            + 4
            + self
                .dumps
                .iter()
                .map(|dump| 12 + dump.bytes.len())
                .sum::<usize>()
    }

    pub fn write_binary(&self, output: &mut [u8]) -> Result<usize, KernelDebugError> {
        let required = self.binary_len();
        if output.len() < required {
            return Err(KernelDebugError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        let mut cursor = BinaryCursor::new(output);
        cursor.write(CRASH_MAGIC)?;
        cursor.u16(1)?;
        cursor.u16(self.registers.architecture.id())?;
        cursor.u64(self.timestamp_ticks)?;
        cursor.u32(self.cpu_id)?;
        cursor.u32(self.reason.len() as u32)?;
        cursor.write(self.reason.as_bytes())?;
        cursor.u64(self.registers.instruction_pointer)?;
        cursor.u64(self.registers.stack_pointer)?;
        cursor.u64(self.registers.frame_pointer)?;
        cursor.u64(self.registers.status)?;
        cursor.u16(self.registers.general_count as u16)?;
        cursor.u16(self.registers.debug_count as u16)?;
        for value in self
            .registers
            .general()
            .iter()
            .chain(self.registers.debug())
        {
            cursor.u64(*value)?;
        }
        cursor.u32(self.frames.len() as u32)?;
        for frame in &self.frames {
            cursor.u64(frame.instruction_pointer)?;
            cursor.u64(frame.stack_pointer)?;
            cursor.u64(frame.frame_pointer)?;
            cursor.u64(frame.symbol_offset)?;
        }
        cursor.u32(self.dumps.len() as u32)?;
        for dump in &self.dumps {
            cursor.u64(dump.address)?;
            cursor.u32(dump.bytes.len() as u32)?;
            cursor.write(&dump.bytes)?;
        }
        Ok(cursor.position)
    }

    pub fn serial_text(&self, output: &mut [u8]) -> Result<usize, KernelDebugError> {
        let mut text = format!(
            "PANIC cpu={} arch={} pc={:#x} sp={:#x} fp={:#x} reason={}\n",
            self.cpu_id,
            self.registers.architecture.name(),
            self.registers.instruction_pointer,
            self.registers.stack_pointer,
            self.registers.frame_pointer,
            self.reason
        );
        for frame in &self.frames {
            let symbol = frame.symbol.as_deref().unwrap_or("<unknown>");
            text.push_str(&format!(
                "#{:02} {:#018x} {}+{:#x}\n",
                frame.index, frame.instruction_pointer, symbol, frame.symbol_offset
            ));
        }
        if output.len() < text.len() {
            return Err(KernelDebugError::OutputTooSmall {
                required: text.len(),
                available: output.len(),
            });
        }
        output[..text.len()].copy_from_slice(text.as_bytes());
        Ok(text.len())
    }
}

struct BinaryCursor<'a> {
    output: &'a mut [u8],
    position: usize,
}

impl<'a> BinaryCursor<'a> {
    fn new(output: &'a mut [u8]) -> Self {
        Self {
            output,
            position: 0,
        }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), KernelDebugError> {
        let end = self
            .position
            .checked_add(value.len())
            .filter(|end| *end <= self.output.len())
            .ok_or(KernelDebugError::OutputTooSmall {
                required: self.position.saturating_add(value.len()),
                available: self.output.len(),
            })?;
        self.output[self.position..end].copy_from_slice(value);
        self.position = end;
        Ok(())
    }

    fn u16(&mut self, value: u16) -> Result<(), KernelDebugError> {
        self.write(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), KernelDebugError> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), KernelDebugError> {
        self.write(&value.to_le_bytes())
    }
}

#[derive(Debug, Default)]
pub struct KernelPanicOwner {
    captured: AtomicBool,
}

impl KernelPanicOwner {
    pub const fn new() -> Self {
        Self {
            captured: AtomicBool::new(false),
        }
    }

    pub fn claim(&self) -> Result<(), KernelDebugError> {
        self.captured
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| KernelDebugError::PanicAlreadyCaptured)
    }

    pub fn release_for_test_or_reboot(&self) {
        self.captured.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(payload: &[u8]) -> Vec<u8> {
        let mut output = vec![0; payload.len() + 4];
        let len = encode_rsp_packet(payload, &mut output).unwrap();
        output.truncate(len);
        output
    }

    #[test]
    fn breakpoints_and_rsp_patch_caller_memory_and_restore_it() {
        let mut backing = [0x90u8; 64];
        let mut memory = KernelDebugMemoryWindow::new(0x1000, &mut backing);
        let registers = KernelRegisterFile::new(
            DebugArchitecture::X86_64,
            0x1000,
            0x1020,
            0x1030,
            0x202,
            &[1, 2, 3],
            &[],
        )
        .unwrap();
        let mut debugger = SerialDebugger::new(DebugArchitecture::X86_64, 4);
        let mut response = [0; 4096];
        debugger
            .handle_packet(
                &packet(b"Z0,1004,1"),
                &registers,
                &mut memory,
                &mut response,
            )
            .unwrap();
        let mut byte = [0];
        memory.read(0x1004, &mut byte).unwrap();
        assert_eq!(byte, [0xcc]);
        debugger
            .handle_packet(
                &packet(b"z0,1004,1"),
                &registers,
                &mut memory,
                &mut response,
            )
            .unwrap();
        memory.read(0x1004, &mut byte).unwrap();
        assert_eq!(byte, [0x90]);
    }

    #[test]
    fn stack_trace_symbols_and_binary_crash_are_material() {
        let mut backing = [0u8; 256];
        let base = 0x8000u64;
        backing[0x40..0x48].copy_from_slice(&(base + 0x60).to_le_bytes());
        backing[0x48..0x50].copy_from_slice(&0x2020u64.to_le_bytes());
        backing[0x60..0x68].copy_from_slice(&0u64.to_le_bytes());
        backing[0x68..0x70].copy_from_slice(&0x1010u64.to_le_bytes());
        let memory = KernelDebugMemoryWindow::new(base, &mut backing);
        let mut symbols = KernelSymbolTable::default();
        symbols
            .insert(KernelSymbol {
                name: "kmain".into(),
                address: 0x1000,
                size: 0x100,
            })
            .unwrap();
        symbols
            .insert(KernelSymbol {
                name: "panic_path".into(),
                address: 0x2000,
                size: 0x100,
            })
            .unwrap();
        let registers = KernelRegisterFile::new(
            DebugArchitecture::X86_64,
            0x2042,
            base + 0x20,
            base + 0x40,
            0x202,
            &[1, 2, 3, 4],
            &[0; 8],
        )
        .unwrap();
        let report = KernelCrashReport::capture(
            99,
            2,
            "fatal page fault",
            registers,
            &memory,
            &symbols,
            &[(base, 32)],
            8,
        )
        .unwrap();
        assert_eq!(report.frames.len(), 3);
        assert_eq!(report.frames[0].symbol.as_deref(), Some("panic_path"));
        let mut binary = vec![0; report.binary_len()];
        let len = report.write_binary(&mut binary).unwrap();
        assert_eq!(&binary[..8], CRASH_MAGIC);
        assert_eq!(len, report.binary_len());
    }

    #[test]
    fn trap_opcodes_cover_every_kernel_architecture() {
        assert_eq!(
            debug_trap_opcode(DebugArchitecture::X86_64, 0).bytes[0],
            0xcc
        );
        assert_eq!(
            debug_trap_opcode(DebugArchitecture::Aarch64, 0x1234).bytes,
            [0x80, 0x46, 0x22, 0xd4]
        );
        assert_eq!(
            debug_trap_opcode(DebugArchitecture::Armv7Thumb, 0x42).bytes[..2],
            [0x42, 0xbe]
        );
        assert_eq!(
            debug_trap_opcode(DebugArchitecture::Riscv64, 0).bytes,
            [0x73, 0x00, 0x10, 0x00]
        );
    }
}
