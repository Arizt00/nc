use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

pub const UART_16550_FIFO_BYTES: usize = 16;
pub const PS2_OUTPUT_BYTES: usize = 256;
pub const ETHERNET_HEADER_BYTES: usize = 14;
pub const ETHERNET_MAX_FRAME_BYTES: usize = 65_535;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualDeviceError {
    InvalidRegister(u64),
    InvalidValue(&'static str),
    OutOfRange {
        offset: u64,
        length: usize,
        capacity: usize,
    },
    ReadOnly,
    QueueFull,
    QueueEmpty,
    LinkDown,
    FrameTooLarge(usize),
    InvalidPixelFormat,
}

impl fmt::Display for VirtualDeviceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegister(register) => {
                write!(formatter, "invalid virtual device register {register:#x}")
            }
            Self::InvalidValue(reason) => {
                write!(formatter, "invalid virtual device value: {reason}")
            }
            Self::OutOfRange {
                offset,
                length,
                capacity,
            } => write!(
                formatter,
                "virtual device range {offset:#x}+{length:#x} exceeds capacity {capacity:#x}"
            ),
            Self::ReadOnly => formatter.write_str("virtual device is read-only"),
            Self::QueueFull => formatter.write_str("virtual device queue is full"),
            Self::QueueEmpty => formatter.write_str("virtual device queue is empty"),
            Self::LinkDown => formatter.write_str("virtual network link is down"),
            Self::FrameTooLarge(length) => {
                write!(
                    formatter,
                    "virtual network frame is too large: {length} bytes"
                )
            }
            Self::InvalidPixelFormat => formatter.write_str("invalid framebuffer pixel format"),
        }
    }
}

impl Error for VirtualDeviceError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteQueue<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    head: usize,
    tail: usize,
    length: usize,
}

impl<const CAPACITY: usize> Default for ByteQueue<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> ByteQueue<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            head: 0,
            tail: 0,
            length: 0,
        }
    }

    pub fn push(&mut self, byte: u8) -> Result<(), VirtualDeviceError> {
        if self.length == CAPACITY {
            return Err(VirtualDeviceError::QueueFull);
        }
        self.bytes[self.tail] = byte;
        self.tail = (self.tail + 1) % CAPACITY.max(1);
        self.length += 1;
        Ok(())
    }

    pub fn extend(&mut self, bytes: &[u8]) -> Result<(), VirtualDeviceError> {
        if bytes.len() > CAPACITY.saturating_sub(self.length) {
            return Err(VirtualDeviceError::QueueFull);
        }
        for byte in bytes {
            self.push(*byte)?;
        }
        Ok(())
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.length == 0 {
            return None;
        }
        let byte = self.bytes[self.head];
        self.head = (self.head + 1) % CAPACITY.max(1);
        self.length -= 1;
        Some(byte)
    }

    pub fn drain(&mut self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.length);
        while let Some(byte) = self.pop() {
            bytes.push(byte);
        }
        bytes
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.length = 0;
        self.bytes.fill(0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uart16550 {
    pub base_port: u16,
    pub interrupt_enable: u8,
    pub fifo_control: u8,
    pub line_control: u8,
    pub modem_control: u8,
    pub modem_status: u8,
    pub scratch: u8,
    pub divisor: u16,
    pub overrun: bool,
    receive: ByteQueue<256>,
    transmit: ByteQueue<4096>,
}

impl Uart16550 {
    pub fn new(base_port: u16) -> Self {
        Self {
            base_port,
            interrupt_enable: 0,
            fifo_control: 0,
            line_control: 0x03,
            modem_control: 0,
            modem_status: 0xb0,
            scratch: 0,
            divisor: 1,
            overrun: false,
            receive: ByteQueue::new(),
            transmit: ByteQueue::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.base_port);
    }

    pub fn inject_rx(&mut self, bytes: &[u8]) -> Result<(), VirtualDeviceError> {
        for byte in bytes {
            if self.receive.push(*byte).is_err() {
                self.overrun = true;
                return Err(VirtualDeviceError::QueueFull);
            }
        }
        Ok(())
    }

    pub fn take_tx(&mut self) -> Vec<u8> {
        self.transmit.drain()
    }

    pub fn pending_rx(&self) -> usize {
        self.receive.len()
    }

    pub fn pending_tx(&self) -> usize {
        self.transmit.len()
    }

    pub fn interrupt_pending(&self) -> bool {
        (self.interrupt_enable & 0x01 != 0 && !self.receive.is_empty())
            || (self.interrupt_enable & 0x02 != 0 && self.transmit.is_empty())
            || (self.interrupt_enable & 0x04 != 0 && self.overrun)
    }

    fn divisor_latch_access(&self) -> bool {
        self.line_control & 0x80 != 0
    }

    fn line_status(&self) -> u8 {
        let mut status = 0x60;
        if !self.receive.is_empty() {
            status |= 0x01;
        }
        if self.overrun {
            status |= 0x02;
        }
        status
    }

    fn interrupt_identification(&self) -> u8 {
        if self.interrupt_enable & 0x04 != 0 && self.overrun {
            0x06
        } else if self.interrupt_enable & 0x01 != 0 && !self.receive.is_empty() {
            0x04
        } else if self.interrupt_enable & 0x02 != 0 && self.transmit.is_empty() {
            0x02
        } else {
            0x01
        }
    }

    pub fn read(&mut self, offset: u8) -> Result<u8, VirtualDeviceError> {
        match offset {
            0 if self.divisor_latch_access() => Ok(self.divisor as u8),
            0 => Ok(self.receive.pop().unwrap_or(0)),
            1 if self.divisor_latch_access() => Ok((self.divisor >> 8) as u8),
            1 => Ok(self.interrupt_enable),
            2 => Ok(self.interrupt_identification()),
            3 => Ok(self.line_control),
            4 => Ok(self.modem_control),
            5 => Ok(self.line_status()),
            6 => Ok(self.modem_status),
            7 => Ok(self.scratch),
            _ => Err(VirtualDeviceError::InvalidRegister(offset as u64)),
        }
    }

    pub fn write(&mut self, offset: u8, value: u8) -> Result<(), VirtualDeviceError> {
        match offset {
            0 if self.divisor_latch_access() => {
                self.divisor = (self.divisor & 0xff00) | value as u16;
            }
            0 => self.transmit.push(value)?,
            1 if self.divisor_latch_access() => {
                self.divisor = (self.divisor & 0x00ff) | ((value as u16) << 8);
            }
            1 => self.interrupt_enable = value & 0x0f,
            2 => {
                self.fifo_control = value;
                if value & 0x02 != 0 {
                    self.receive.clear();
                    self.overrun = false;
                }
                if value & 0x04 != 0 {
                    self.transmit.clear();
                }
            }
            3 => self.line_control = value,
            4 => self.modem_control = value,
            5 | 6 => return Err(VirtualDeviceError::InvalidRegister(offset as u64)),
            7 => self.scratch = value,
            _ => return Err(VirtualDeviceError::InvalidRegister(offset as u64)),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ps2Key {
    Escape,
    Digit(u8),
    Letter(u8),
    Enter,
    Backspace,
    Tab,
    Space,
    LeftShift,
    RightShift,
    LeftControl,
    LeftAlt,
    CapsLock,
    Up,
    Down,
    Left,
    Right,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ps2KeyEvent {
    pub key: Ps2Key,
    pub pressed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ps2Controller {
    pub command_byte: u8,
    pub output_port: u8,
    pub keyboard_enabled: bool,
    pub scanning_enabled: bool,
    pub scancode_set: u8,
    pub leds: u8,
    pub system_flag: bool,
    expect_command_byte: bool,
    expect_output_port: bool,
    expect_scancode_set: bool,
    expect_leds: bool,
    output: ByteQueue<PS2_OUTPUT_BYTES>,
}

impl Default for Ps2Controller {
    fn default() -> Self {
        Self::new()
    }
}

impl Ps2Controller {
    pub const fn new() -> Self {
        Self {
            command_byte: 0,
            output_port: 0x01,
            keyboard_enabled: true,
            scanning_enabled: true,
            scancode_set: 2,
            leds: 0,
            system_flag: true,
            expect_command_byte: false,
            expect_output_port: false,
            expect_scancode_set: false,
            expect_leds: false,
            output: ByteQueue::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn status(&self) -> u8 {
        let mut status = 0;
        if !self.output.is_empty() {
            status |= 0x01;
        }
        if self.system_flag {
            status |= 0x04;
        }
        status
    }

    pub fn read_data(&mut self) -> u8 {
        self.output.pop().unwrap_or(0)
    }

    pub fn write_command(&mut self, command: u8) -> Result<(), VirtualDeviceError> {
        match command {
            0x20 => self.output.push(self.command_byte)?,
            0x60 => self.expect_command_byte = true,
            0xaa => self.output.push(0x55)?,
            0xab => self.output.push(0x00)?,
            0xad => self.keyboard_enabled = false,
            0xae => self.keyboard_enabled = true,
            0xd0 => self.output.push(self.output_port)?,
            0xd1 => self.expect_output_port = true,
            _ => {
                return Err(VirtualDeviceError::InvalidValue(
                    "unsupported i8042 command",
                ))
            }
        }
        Ok(())
    }

    pub fn write_data(&mut self, value: u8) -> Result<(), VirtualDeviceError> {
        if self.expect_command_byte {
            self.command_byte = value;
            self.expect_command_byte = false;
            return Ok(());
        }
        if self.expect_output_port {
            self.output_port = value;
            self.expect_output_port = false;
            return Ok(());
        }
        if self.expect_scancode_set {
            self.expect_scancode_set = false;
            if value == 0 {
                self.output.extend(&[0xfa, self.scancode_set])?;
            } else if (1..=3).contains(&value) {
                self.scancode_set = value;
                self.output.push(0xfa)?;
            } else {
                self.output.push(0xfe)?;
            }
            return Ok(());
        }
        if self.expect_leds {
            self.leds = value & 0x07;
            self.expect_leds = false;
            self.output.push(0xfa)?;
            return Ok(());
        }
        match value {
            0xff => {
                self.scanning_enabled = false;
                self.output.extend(&[0xfa, 0xaa])?;
            }
            0xf4 => {
                self.scanning_enabled = true;
                self.output.push(0xfa)?;
            }
            0xf5 => {
                self.scanning_enabled = false;
                self.output.push(0xfa)?;
            }
            0xf0 => {
                self.expect_scancode_set = true;
                self.output.push(0xfa)?;
            }
            0xed => {
                self.expect_leds = true;
                self.output.push(0xfa)?;
            }
            0xee => self.output.push(0xee)?,
            _ => self.output.push(0xfe)?,
        }
        Ok(())
    }

    pub fn inject_key(&mut self, event: Ps2KeyEvent) -> Result<(), VirtualDeviceError> {
        if !self.keyboard_enabled || !self.scanning_enabled {
            return Ok(());
        }
        let (extended, code) = set2_scancode(event.key)?;
        if extended {
            self.output.push(0xe0)?;
        }
        if !event.pressed {
            self.output.push(0xf0)?;
        }
        self.output.push(code)?;
        Ok(())
    }

    pub fn pending_bytes(&self) -> usize {
        self.output.len()
    }
}

fn set2_scancode(key: Ps2Key) -> Result<(bool, u8), VirtualDeviceError> {
    let value = match key {
        Ps2Key::Escape => (false, 0x76),
        Ps2Key::Digit(digit) => {
            let codes = [0x45, 0x16, 0x1e, 0x26, 0x25, 0x2e, 0x36, 0x3d, 0x3e, 0x46];
            let code = *codes
                .get(digit as usize)
                .ok_or(VirtualDeviceError::InvalidValue("PS/2 digit must be 0..9"))?;
            (false, code)
        }
        Ps2Key::Letter(letter) => {
            const CODES: [u8; 26] = [
                0x1c, 0x32, 0x21, 0x23, 0x24, 0x2b, 0x34, 0x33, 0x43, 0x3b, 0x42, 0x4b, 0x3a, 0x31,
                0x44, 0x4d, 0x15, 0x2d, 0x1b, 0x2c, 0x3c, 0x2a, 0x1d, 0x22, 0x35, 0x1a,
            ];
            let normalized = letter.to_ascii_lowercase();
            let index = normalized
                .checked_sub(b'a')
                .ok_or(VirtualDeviceError::InvalidValue(
                    "PS/2 letter must be ASCII a..z",
                ))? as usize;
            let code = *CODES.get(index).ok_or(VirtualDeviceError::InvalidValue(
                "PS/2 letter must be ASCII a..z",
            ))?;
            (false, code)
        }
        Ps2Key::Enter => (false, 0x5a),
        Ps2Key::Backspace => (false, 0x66),
        Ps2Key::Tab => (false, 0x0d),
        Ps2Key::Space => (false, 0x29),
        Ps2Key::LeftShift => (false, 0x12),
        Ps2Key::RightShift => (false, 0x59),
        Ps2Key::LeftControl => (false, 0x14),
        Ps2Key::LeftAlt => (false, 0x11),
        Ps2Key::CapsLock => (false, 0x58),
        Ps2Key::Up => (true, 0x75),
        Ps2Key::Down => (true, 0x72),
        Ps2Key::Left => (true, 0x6b),
        Ps2Key::Right => (true, 0x74),
        Ps2Key::Delete => (true, 0x71),
    };
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualBlockDevice {
    bytes: Vec<u8>,
    pub sector_size: u32,
    pub read_only: bool,
    pub dirty: bool,
    pub generation: u64,
    pub flush_count: u64,
    pub read_bytes: u64,
    pub written_bytes: u64,
}

impl VirtualBlockDevice {
    pub fn new(
        capacity: usize,
        sector_size: u32,
        read_only: bool,
    ) -> Result<Self, VirtualDeviceError> {
        if sector_size == 0 || !sector_size.is_power_of_two() {
            return Err(VirtualDeviceError::InvalidValue(
                "sector size must be a non-zero power of two",
            ));
        }
        if capacity == 0 || capacity % sector_size as usize != 0 {
            return Err(VirtualDeviceError::InvalidValue(
                "block capacity must contain whole sectors",
            ));
        }
        Ok(Self {
            bytes: vec![0; capacity],
            sector_size,
            read_only,
            dirty: false,
            generation: 0,
            flush_count: 0,
            read_bytes: 0,
            written_bytes: 0,
        })
    }

    pub fn from_bytes(
        bytes: Vec<u8>,
        sector_size: u32,
        read_only: bool,
    ) -> Result<Self, VirtualDeviceError> {
        let mut device = Self::new(bytes.len(), sector_size, read_only)?;
        device.bytes = bytes;
        Ok(device)
    }

    pub fn capacity(&self) -> usize {
        self.bytes.len()
    }

    pub fn sectors(&self) -> u64 {
        self.bytes.len() as u64 / self.sector_size as u64
    }

    fn range(
        &self,
        offset: u64,
        length: usize,
    ) -> Result<std::ops::Range<usize>, VirtualDeviceError> {
        let start = usize::try_from(offset).map_err(|_| VirtualDeviceError::OutOfRange {
            offset,
            length,
            capacity: self.bytes.len(),
        })?;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(VirtualDeviceError::OutOfRange {
                offset,
                length,
                capacity: self.bytes.len(),
            })?;
        Ok(start..end)
    }

    pub fn read(&mut self, offset: u64, output: &mut [u8]) -> Result<(), VirtualDeviceError> {
        let range = self.range(offset, output.len())?;
        output.copy_from_slice(&self.bytes[range]);
        self.read_bytes = self.read_bytes.saturating_add(output.len() as u64);
        Ok(())
    }

    pub fn write(&mut self, offset: u64, input: &[u8]) -> Result<(), VirtualDeviceError> {
        if self.read_only {
            return Err(VirtualDeviceError::ReadOnly);
        }
        let range = self.range(offset, input.len())?;
        self.bytes[range].copy_from_slice(input);
        self.dirty = true;
        self.generation = self.generation.wrapping_add(1);
        self.written_bytes = self.written_bytes.saturating_add(input.len() as u64);
        Ok(())
    }

    pub fn read_sectors(
        &mut self,
        sector: u64,
        output: &mut [u8],
    ) -> Result<(), VirtualDeviceError> {
        if output.len() % self.sector_size as usize != 0 {
            return Err(VirtualDeviceError::InvalidValue(
                "sector read length must be aligned",
            ));
        }
        let offset =
            sector
                .checked_mul(self.sector_size as u64)
                .ok_or(VirtualDeviceError::OutOfRange {
                    offset: sector,
                    length: output.len(),
                    capacity: self.bytes.len(),
                })?;
        self.read(offset, output)
    }

    pub fn write_sectors(&mut self, sector: u64, input: &[u8]) -> Result<(), VirtualDeviceError> {
        if input.len() % self.sector_size as usize != 0 {
            return Err(VirtualDeviceError::InvalidValue(
                "sector write length must be aligned",
            ));
        }
        let offset =
            sector
                .checked_mul(self.sector_size as u64)
                .ok_or(VirtualDeviceError::OutOfRange {
                    offset: sector,
                    length: input.len(),
                    capacity: self.bytes.len(),
                })?;
        self.write(offset, input)
    }

    pub fn discard(&mut self, offset: u64, length: usize) -> Result<(), VirtualDeviceError> {
        if self.read_only {
            return Err(VirtualDeviceError::ReadOnly);
        }
        let range = self.range(offset, length)?;
        self.bytes[range].fill(0);
        self.dirty = true;
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn write_zeroes(&mut self, offset: u64, length: usize) -> Result<(), VirtualDeviceError> {
        self.discard(offset, length)
    }

    pub fn flush(&mut self) {
        self.dirty = false;
        self.flush_count = self.flush_count.wrapping_add(1);
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkCounters {
    pub tx_packets: u64,
    pub tx_bytes: u64,
    pub tx_dropped: u64,
    pub rx_packets: u64,
    pub rx_bytes: u64,
    pub rx_dropped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualNetworkDevice {
    pub mac: [u8; 6],
    pub mtu: u16,
    pub link_up: bool,
    pub promiscuous: bool,
    pub queue_capacity: usize,
    pub counters: NetworkCounters,
    transmit: VecDeque<Vec<u8>>,
    receive: VecDeque<Vec<u8>>,
}

impl VirtualNetworkDevice {
    pub fn new(mac: [u8; 6], mtu: u16, queue_capacity: usize) -> Result<Self, VirtualDeviceError> {
        if mac[0] & 1 != 0 || mac == [0; 6] {
            return Err(VirtualDeviceError::InvalidValue(
                "MAC must be non-zero and unicast",
            ));
        }
        if mtu < 68 || queue_capacity == 0 {
            return Err(VirtualDeviceError::InvalidValue(
                "invalid network MTU or queue",
            ));
        }
        Ok(Self {
            mac,
            mtu,
            link_up: true,
            promiscuous: false,
            queue_capacity,
            counters: NetworkCounters::default(),
            transmit: VecDeque::with_capacity(queue_capacity),
            receive: VecDeque::with_capacity(queue_capacity),
        })
    }

    pub fn transmit(&mut self, frame: &[u8]) -> Result<(), VirtualDeviceError> {
        if !self.link_up {
            return Err(VirtualDeviceError::LinkDown);
        }
        if frame.len() < ETHERNET_HEADER_BYTES
            || frame.len()
                > (self.mtu as usize + ETHERNET_HEADER_BYTES).min(ETHERNET_MAX_FRAME_BYTES)
        {
            self.counters.tx_dropped = self.counters.tx_dropped.saturating_add(1);
            return Err(VirtualDeviceError::FrameTooLarge(frame.len()));
        }
        if self.transmit.len() == self.queue_capacity {
            self.counters.tx_dropped = self.counters.tx_dropped.saturating_add(1);
            return Err(VirtualDeviceError::QueueFull);
        }
        self.transmit.push_back(frame.to_vec());
        self.counters.tx_packets = self.counters.tx_packets.saturating_add(1);
        self.counters.tx_bytes = self.counters.tx_bytes.saturating_add(frame.len() as u64);
        Ok(())
    }

    pub fn take_transmitted(&mut self) -> Option<Vec<u8>> {
        self.transmit.pop_front()
    }

    pub fn inject_received(&mut self, frame: &[u8]) -> Result<(), VirtualDeviceError> {
        if !self.link_up {
            return Err(VirtualDeviceError::LinkDown);
        }
        if frame.len() < ETHERNET_HEADER_BYTES || frame.len() > ETHERNET_MAX_FRAME_BYTES {
            self.counters.rx_dropped = self.counters.rx_dropped.saturating_add(1);
            return Err(VirtualDeviceError::FrameTooLarge(frame.len()));
        }
        let destination_matches = frame[0..6] == self.mac
            || frame[0..6] == [0xff; 6]
            || frame[0] & 1 != 0
            || self.promiscuous;
        if !destination_matches {
            self.counters.rx_dropped = self.counters.rx_dropped.saturating_add(1);
            return Ok(());
        }
        if self.receive.len() == self.queue_capacity {
            self.counters.rx_dropped = self.counters.rx_dropped.saturating_add(1);
            return Err(VirtualDeviceError::QueueFull);
        }
        self.receive.push_back(frame.to_vec());
        self.counters.rx_packets = self.counters.rx_packets.saturating_add(1);
        self.counters.rx_bytes = self.counters.rx_bytes.saturating_add(frame.len() as u64);
        Ok(())
    }

    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.receive.pop_front()
    }

    pub fn pending_tx(&self) -> usize {
        self.transmit.len()
    }

    pub fn pending_rx(&self) -> usize {
        self.receive.len()
    }

    pub fn reset(&mut self) {
        self.transmit.clear();
        self.receive.clear();
        self.counters = NetworkCounters::default();
        self.link_up = true;
        self.promiscuous = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra8888,
    Rgba8888,
    Rgb565,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Bgra8888 | Self::Rgba8888 => 4,
            Self::Rgb565 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualFramebuffer {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pixels: Vec<u8>,
    dirty: Vec<DirtyRect>,
}

impl VirtualFramebuffer {
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Result<Self, VirtualDeviceError> {
        if width == 0 || height == 0 {
            return Err(VirtualDeviceError::InvalidValue(
                "framebuffer dimensions must be non-zero",
            ));
        }
        let stride = width.checked_mul(format.bytes_per_pixel() as u32).ok_or(
            VirtualDeviceError::InvalidValue("framebuffer stride overflow"),
        )?;
        let length = usize::try_from(stride.checked_mul(height).ok_or(
            VirtualDeviceError::InvalidValue("framebuffer size overflow"),
        )?)
        .map_err(|_| VirtualDeviceError::InvalidValue("framebuffer exceeds host size"))?;
        Ok(Self {
            width,
            height,
            stride,
            format,
            pixels: vec![0; length],
            dirty: Vec::new(),
        })
    }

    pub fn reset(&mut self) {
        self.pixels.fill(0);
        self.dirty.clear();
        self.dirty.push(DirtyRect {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        });
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn write_pixel(&mut self, x: u32, y: u32, rgba: u32) -> Result<(), VirtualDeviceError> {
        if x >= self.width || y >= self.height {
            return Err(VirtualDeviceError::OutOfRange {
                offset: y as u64 * self.stride as u64 + x as u64,
                length: self.format.bytes_per_pixel(),
                capacity: self.pixels.len(),
            });
        }
        let offset = y as usize * self.stride as usize + x as usize * self.format.bytes_per_pixel();
        match self.format {
            PixelFormat::Bgra8888 => {
                self.pixels[offset..offset + 4].copy_from_slice(&[
                    (rgba >> 8) as u8,
                    (rgba >> 16) as u8,
                    (rgba >> 24) as u8,
                    rgba as u8,
                ]);
            }
            PixelFormat::Rgba8888 => {
                self.pixels[offset..offset + 4].copy_from_slice(&rgba.to_be_bytes());
            }
            PixelFormat::Rgb565 => {
                let red = ((rgba >> 27) & 0x1f) as u16;
                let green = ((rgba >> 18) & 0x3f) as u16;
                let blue = ((rgba >> 11) & 0x1f) as u16;
                self.pixels[offset..offset + 2]
                    .copy_from_slice(&((red << 11) | (green << 5) | blue).to_le_bytes());
            }
        }
        self.mark_dirty(DirtyRect {
            x,
            y,
            width: 1,
            height: 1,
        });
        Ok(())
    }

    pub fn fill_rect(&mut self, rect: DirtyRect, rgba: u32) -> Result<(), VirtualDeviceError> {
        let end_x = rect
            .x
            .checked_add(rect.width)
            .ok_or(VirtualDeviceError::InvalidValue("rectangle overflow"))?;
        let end_y = rect
            .y
            .checked_add(rect.height)
            .ok_or(VirtualDeviceError::InvalidValue("rectangle overflow"))?;
        if rect.width == 0 || rect.height == 0 || end_x > self.width || end_y > self.height {
            return Err(VirtualDeviceError::InvalidValue(
                "rectangle is empty or outside framebuffer",
            ));
        }
        for y in rect.y..end_y {
            for x in rect.x..end_x {
                self.write_pixel_without_dirty(x, y, rgba);
            }
        }
        self.mark_dirty(rect);
        Ok(())
    }

    fn write_pixel_without_dirty(&mut self, x: u32, y: u32, rgba: u32) {
        let offset = y as usize * self.stride as usize + x as usize * self.format.bytes_per_pixel();
        match self.format {
            PixelFormat::Bgra8888 => {
                self.pixels[offset..offset + 4].copy_from_slice(&[
                    (rgba >> 8) as u8,
                    (rgba >> 16) as u8,
                    (rgba >> 24) as u8,
                    rgba as u8,
                ]);
            }
            PixelFormat::Rgba8888 => {
                self.pixels[offset..offset + 4].copy_from_slice(&rgba.to_be_bytes());
            }
            PixelFormat::Rgb565 => {
                let red = ((rgba >> 27) & 0x1f) as u16;
                let green = ((rgba >> 18) & 0x3f) as u16;
                let blue = ((rgba >> 11) & 0x1f) as u16;
                self.pixels[offset..offset + 2]
                    .copy_from_slice(&((red << 11) | (green << 5) | blue).to_le_bytes());
            }
        }
    }

    fn mark_dirty(&mut self, rect: DirtyRect) {
        if self.dirty.len() >= 256 {
            self.dirty.clear();
            self.dirty.push(DirtyRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            });
        } else {
            self.dirty.push(rect);
        }
    }

    pub fn take_dirty(&mut self) -> Vec<DirtyRect> {
        std::mem::take(&mut self.dirty)
    }

    pub fn checksum(&self) -> u32 {
        crc32(&self.pixels)
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uart_16550_registers_fifo_interrupts_and_reset_are_material() {
        let mut uart = Uart16550::new(0x3f8);
        uart.write(3, 0x80).unwrap();
        uart.write(0, 0x03).unwrap();
        uart.write(1, 0x00).unwrap();
        uart.write(3, 0x03).unwrap();
        assert_eq!(uart.divisor, 3);
        uart.inject_rx(b"K").unwrap();
        uart.write(1, 0x01).unwrap();
        assert!(uart.interrupt_pending());
        assert_eq!(uart.read(5).unwrap() & 1, 1);
        assert_eq!(uart.read(0).unwrap(), b'K');
        uart.write(0, b'O').unwrap();
        uart.write(0, b'K').unwrap();
        assert_eq!(uart.take_tx(), b"OK");
        uart.reset();
        assert_eq!(uart.divisor, 1);
    }

    #[test]
    fn ps2_controller_commands_and_set2_press_release_are_exact() {
        let mut controller = Ps2Controller::new();
        controller.write_command(0xaa).unwrap();
        assert_eq!(controller.read_data(), 0x55);
        controller
            .inject_key(Ps2KeyEvent {
                key: Ps2Key::Letter(b'a'),
                pressed: true,
            })
            .unwrap();
        controller
            .inject_key(Ps2KeyEvent {
                key: Ps2Key::Letter(b'a'),
                pressed: false,
            })
            .unwrap();
        assert_eq!(controller.read_data(), 0x1c);
        assert_eq!(controller.read_data(), 0xf0);
        assert_eq!(controller.read_data(), 0x1c);
        controller.write_data(0xf0).unwrap();
        assert_eq!(controller.read_data(), 0xfa);
        controller.write_data(0).unwrap();
        assert_eq!(controller.read_data(), 0xfa);
        assert_eq!(controller.read_data(), 2);
    }

    #[test]
    fn block_device_covers_sector_io_flush_discard_zeroes_and_bounds() {
        let mut block = VirtualBlockDevice::new(4096, 512, false).unwrap();
        let mut sector = [0u8; 512];
        sector[..8].copy_from_slice(b"NCCBLOCK");
        block.write_sectors(2, &sector).unwrap();
        let mut output = [0u8; 512];
        block.read_sectors(2, &mut output).unwrap();
        assert_eq!(&output[..8], b"NCCBLOCK");
        assert!(block.dirty);
        block.flush();
        assert!(!block.dirty);
        block.discard(1024, 512).unwrap();
        block.read_sectors(2, &mut output).unwrap();
        assert!(output.iter().all(|byte| *byte == 0));
        assert!(matches!(
            block.write(4090, &[0; 8]),
            Err(VirtualDeviceError::OutOfRange { .. })
        ));
    }

    #[test]
    fn network_filters_frames_tracks_counters_and_preserves_payloads() {
        let mac = [0x02, 0, 0, 0, 0, 1];
        let mut network = VirtualNetworkDevice::new(mac, 1500, 4).unwrap();
        let mut frame = vec![0u8; 64];
        frame[..6].copy_from_slice(&mac);
        frame[6..12].copy_from_slice(&[0x02, 0, 0, 0, 0, 2]);
        frame[12..14].copy_from_slice(&0x88b5u16.to_be_bytes());
        frame[14..17].copy_from_slice(b"nC!");
        network.transmit(&frame).unwrap();
        assert_eq!(network.take_transmitted().unwrap(), frame);
        network.inject_received(&frame).unwrap();
        assert_eq!(network.receive().unwrap(), frame);
        assert_eq!(network.counters.tx_packets, 1);
        assert_eq!(network.counters.rx_packets, 1);
    }

    #[test]
    fn framebuffer_formats_dirty_tracking_and_crc_cover_real_pixels() {
        let mut framebuffer = VirtualFramebuffer::new(4, 3, PixelFormat::Bgra8888).unwrap();
        framebuffer
            .fill_rect(
                DirtyRect {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                0xff11_2233,
            )
            .unwrap();
        assert_ne!(framebuffer.checksum(), crc32(&vec![0; 4 * 3 * 4]));
        assert_eq!(framebuffer.take_dirty().len(), 1);
        assert!(matches!(
            framebuffer.write_pixel(4, 0, 0),
            Err(VirtualDeviceError::OutOfRange { .. })
        ));
    }
}
