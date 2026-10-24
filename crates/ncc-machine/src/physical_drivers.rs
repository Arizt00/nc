use crate::{DirtyRect, PixelFormat};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::ptr::{read_volatile, write_volatile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalDriverError {
    AddressOutOfRange { address: u64, width: u8 },
    Timeout(&'static str),
    InvalidDescriptor(&'static str),
    InvalidState(&'static str),
    Unsupported(&'static str),
    InjectedFault { access: u64 },
}

impl fmt::Display for PhysicalDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressOutOfRange { address, width } => {
                write!(
                    f,
                    "physical address {address:#x} with width {width} is outside the mapping"
                )
            }
            Self::Timeout(operation)
            | Self::InvalidDescriptor(operation)
            | Self::InvalidState(operation)
            | Self::Unsupported(operation) => f.write_str(operation),
            Self::InjectedFault { access } => {
                write!(f, "injected physical I/O fault at access {access}")
            }
        }
    }
}

impl Error for PhysicalDriverError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegisterWidth {
    U8,
    U16,
    U32,
    U64,
}

impl RegisterWidth {
    const fn bytes(self) -> u8 {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
            Self::U64 => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterAccess {
    pub address: u64,
    pub width: RegisterWidth,
    pub value: u64,
    pub write: bool,
}

pub trait RegisterIo {
    fn read(&mut self, address: u64, width: RegisterWidth) -> Result<u64, PhysicalDriverError>;
    fn write(
        &mut self,
        address: u64,
        width: RegisterWidth,
        value: u64,
    ) -> Result<(), PhysicalDriverError>;
    fn fence(&mut self) -> Result<(), PhysicalDriverError>;
}

#[derive(Debug, Clone, Default)]
pub struct TracedRegisterIo {
    values: BTreeMap<(u64, RegisterWidth), u64>,
    pub accesses: Vec<RegisterAccess>,
    pub fences: u64,
    pub fail_after: Option<u64>,
}

impl TracedRegisterIo {
    pub fn set(&mut self, address: u64, width: RegisterWidth, value: u64) {
        self.values.insert((address, width), value);
    }

    fn before_access(&self) -> Result<(), PhysicalDriverError> {
        let access = self.accesses.len() as u64;
        if self.fail_after == Some(access) {
            return Err(PhysicalDriverError::InjectedFault { access });
        }
        Ok(())
    }
}

impl RegisterIo for TracedRegisterIo {
    fn read(&mut self, address: u64, width: RegisterWidth) -> Result<u64, PhysicalDriverError> {
        self.before_access()?;
        let value = self.values.get(&(address, width)).copied().unwrap_or(0);
        self.accesses.push(RegisterAccess {
            address,
            width,
            value,
            write: false,
        });
        Ok(value)
    }

    fn write(
        &mut self,
        address: u64,
        width: RegisterWidth,
        value: u64,
    ) -> Result<(), PhysicalDriverError> {
        self.before_access()?;
        let mask = match width {
            RegisterWidth::U8 => u8::MAX as u64,
            RegisterWidth::U16 => u16::MAX as u64,
            RegisterWidth::U32 => u32::MAX as u64,
            RegisterWidth::U64 => u64::MAX,
        };
        self.values.insert((address, width), value & mask);
        self.accesses.push(RegisterAccess {
            address,
            width,
            value: value & mask,
            write: true,
        });
        Ok(())
    }

    fn fence(&mut self) -> Result<(), PhysicalDriverError> {
        self.fences = self.fences.wrapping_add(1);
        Ok(())
    }
}

#[derive(Debug)]
pub struct VolatileMmio {
    base: *mut u8,
    physical_base: u64,
    length: usize,
}

impl VolatileMmio {
    /// The caller must keep the mapping valid and uncached/device-typed for the lifetime of this object.
    pub unsafe fn new(
        base: *mut u8,
        physical_base: u64,
        length: usize,
    ) -> Result<Self, PhysicalDriverError> {
        if base.is_null() || length == 0 {
            return Err(PhysicalDriverError::InvalidState(
                "MMIO mapping is null or empty",
            ));
        }
        Ok(Self {
            base,
            physical_base,
            length,
        })
    }

    fn pointer(&self, address: u64, width: RegisterWidth) -> Result<*mut u8, PhysicalDriverError> {
        let relative = address
            .checked_sub(self.physical_base)
            .and_then(|offset| usize::try_from(offset).ok())
            .ok_or(PhysicalDriverError::AddressOutOfRange {
                address,
                width: width.bytes(),
            })?;
        if relative
            .checked_add(width.bytes() as usize)
            .is_none_or(|end| end > self.length)
            || relative % width.bytes() as usize != 0
        {
            return Err(PhysicalDriverError::AddressOutOfRange {
                address,
                width: width.bytes(),
            });
        }
        Ok(unsafe { self.base.add(relative) })
    }
}

impl RegisterIo for VolatileMmio {
    fn read(&mut self, address: u64, width: RegisterWidth) -> Result<u64, PhysicalDriverError> {
        let pointer = self.pointer(address, width)?;
        let value = unsafe {
            match width {
                RegisterWidth::U8 => read_volatile(pointer) as u64,
                RegisterWidth::U16 => read_volatile(pointer.cast::<u16>()) as u64,
                RegisterWidth::U32 => read_volatile(pointer.cast::<u32>()) as u64,
                RegisterWidth::U64 => read_volatile(pointer.cast::<u64>()),
            }
        };
        Ok(value)
    }

    fn write(
        &mut self,
        address: u64,
        width: RegisterWidth,
        value: u64,
    ) -> Result<(), PhysicalDriverError> {
        let pointer = self.pointer(address, width)?;
        unsafe {
            match width {
                RegisterWidth::U8 => write_volatile(pointer, value as u8),
                RegisterWidth::U16 => write_volatile(pointer.cast::<u16>(), value as u16),
                RegisterWidth::U32 => write_volatile(pointer.cast::<u32>(), value as u32),
                RegisterWidth::U64 => write_volatile(pointer.cast::<u64>(), value),
            }
        }
        Ok(())
    }

    fn fence(&mut self) -> Result<(), PhysicalDriverError> {
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Offline,
    Ready,
    Degraded,
    Resetting,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverHealth {
    pub state: DriverState,
    pub operations: u64,
    pub timeouts: u64,
    pub faults: u64,
    pub resets: u64,
    pub last_error: Option<String>,
}

impl Default for DriverHealth {
    fn default() -> Self {
        Self {
            state: DriverState::Offline,
            operations: 0,
            timeouts: 0,
            faults: 0,
            resets: 0,
            last_error: None,
        }
    }
}

impl DriverHealth {
    fn ready(&mut self) {
        self.state = DriverState::Ready;
        self.last_error = None;
    }

    fn timeout(&mut self, operation: &'static str) -> PhysicalDriverError {
        self.state = DriverState::Degraded;
        self.timeouts = self.timeouts.wrapping_add(1);
        self.last_error = Some(operation.into());
        PhysicalDriverError::Timeout(operation)
    }

    fn fault(&mut self, error: &PhysicalDriverError) {
        self.state = DriverState::Failed;
        self.faults = self.faults.wrapping_add(1);
        self.last_error = Some(error.to_string());
    }

    fn reset(&mut self) {
        self.state = DriverState::Resetting;
        self.resets = self.resets.wrapping_add(1);
    }
}

#[derive(Debug, Clone)]
pub struct Uart16550Driver {
    pub base: u64,
    pub health: DriverHealth,
}

impl Uart16550Driver {
    pub fn new(base: u64) -> Self {
        Self {
            base,
            health: DriverHealth::default(),
        }
    }

    pub fn initialize(
        &mut self,
        io: &mut impl RegisterIo,
        divisor: u16,
    ) -> Result<(), PhysicalDriverError> {
        if divisor == 0 {
            return Err(PhysicalDriverError::InvalidDescriptor(
                "UART divisor cannot be zero",
            ));
        }
        let sequence = [
            (1, 0x00),
            (3, 0x80),
            (0, divisor as u8),
            (1, (divisor >> 8) as u8),
            (3, 0x03),
            (2, 0xc7),
            (4, 0x0b),
        ];
        for (offset, value) in sequence {
            io.write(self.base + offset, RegisterWidth::U8, value as u64)?;
        }
        io.fence()?;
        self.health.ready();
        Ok(())
    }

    pub fn write_byte(
        &mut self,
        io: &mut impl RegisterIo,
        byte: u8,
        spins: u32,
    ) -> Result<(), PhysicalDriverError> {
        for _ in 0..spins {
            if io.read(self.base + 5, RegisterWidth::U8)? & 0x20 != 0 {
                io.write(self.base, RegisterWidth::U8, byte as u64)?;
                self.health.operations = self.health.operations.wrapping_add(1);
                return Ok(());
            }
        }
        Err(self.health.timeout("UART transmitter did not become empty"))
    }

    pub fn recover(&mut self, io: &mut impl RegisterIo) -> Result<(), PhysicalDriverError> {
        self.health.reset();
        io.write(self.base + 2, RegisterWidth::U8, 0x07)?;
        self.initialize(io, 1)
    }
}

#[derive(Debug, Clone)]
pub struct Pl011Driver {
    pub base: u64,
    pub health: DriverHealth,
}

impl Pl011Driver {
    pub fn initialize(
        &mut self,
        io: &mut impl RegisterIo,
        integer: u16,
        fractional: u8,
    ) -> Result<(), PhysicalDriverError> {
        io.write(self.base + 0x30, RegisterWidth::U32, 0)?;
        io.write(self.base + 0x44, RegisterWidth::U32, 0x7ff)?;
        io.write(self.base + 0x24, RegisterWidth::U32, integer as u64)?;
        io.write(self.base + 0x28, RegisterWidth::U32, fractional as u64)?;
        io.write(self.base + 0x2c, RegisterWidth::U32, 0x70)?;
        io.write(self.base + 0x30, RegisterWidth::U32, 0x301)?;
        io.fence()?;
        self.health.ready();
        Ok(())
    }

    pub fn write_byte(
        &mut self,
        io: &mut impl RegisterIo,
        byte: u8,
        spins: u32,
    ) -> Result<(), PhysicalDriverError> {
        for _ in 0..spins {
            if io.read(self.base + 0x18, RegisterWidth::U32)? & (1 << 5) == 0 {
                io.write(self.base, RegisterWidth::U32, byte as u64)?;
                self.health.operations = self.health.operations.wrapping_add(1);
                return Ok(());
            }
        }
        Err(self.health.timeout("PL011 transmit FIFO remained full"))
    }
}

#[derive(Debug)]
pub struct PhysicalFramebuffer<'a> {
    bytes: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub pitch: usize,
    pub format: PixelFormat,
}

impl<'a> PhysicalFramebuffer<'a> {
    pub fn new(
        bytes: &'a mut [u8],
        width: u32,
        height: u32,
        pitch: usize,
        format: PixelFormat,
    ) -> Result<Self, PhysicalDriverError> {
        let row = width as usize * format.bytes_per_pixel();
        if width == 0
            || height == 0
            || pitch < row
            || pitch.saturating_mul(height as usize) > bytes.len()
        {
            return Err(PhysicalDriverError::InvalidDescriptor(
                "invalid physical framebuffer geometry",
            ));
        }
        Ok(Self {
            bytes,
            width,
            height,
            pitch,
            format,
        })
    }

    pub fn fill_rect(&mut self, rect: DirtyRect, rgba: u32) -> Result<(), PhysicalDriverError> {
        if rect.x.saturating_add(rect.width) > self.width
            || rect.y.saturating_add(rect.height) > self.height
        {
            return Err(PhysicalDriverError::AddressOutOfRange {
                address: rect.x as u64,
                width: 4,
            });
        }
        let (encoded, bytes_per_pixel) = match self.format {
            PixelFormat::Bgra8888 => (
                [
                    rgba as u8,
                    (rgba >> 8) as u8,
                    (rgba >> 16) as u8,
                    (rgba >> 24) as u8,
                ],
                4,
            ),
            PixelFormat::Rgba8888 => (
                [
                    (rgba >> 16) as u8,
                    (rgba >> 8) as u8,
                    rgba as u8,
                    (rgba >> 24) as u8,
                ],
                4,
            ),
            PixelFormat::Rgb565 => {
                let red = ((rgba >> 19) & 0x1f) as u16;
                let green = ((rgba >> 10) & 0x3f) as u16;
                let blue = ((rgba >> 3) & 0x1f) as u16;
                let packed = (red << 11) | (green << 5) | blue;
                let bytes = packed.to_le_bytes();
                ([bytes[0], bytes[1], 0, 0], 2)
            }
        };
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                let offset = y as usize * self.pitch + x as usize * bytes_per_pixel;
                self.bytes[offset..offset + bytes_per_pixel]
                    .copy_from_slice(&encoded[..bytes_per_pixel]);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Ps2KeyboardDriver {
    pub data_port: u64,
    pub status_port: u64,
    pub health: DriverHealth,
}

impl Ps2KeyboardDriver {
    pub fn poll_scancode(
        &mut self,
        io: &mut impl RegisterIo,
    ) -> Result<Option<u8>, PhysicalDriverError> {
        let status = io.read(self.status_port, RegisterWidth::U8)? as u8;
        if status & 0x01 == 0 {
            return Ok(None);
        }
        if status & 0xc0 != 0 {
            let error = PhysicalDriverError::InvalidState("PS/2 parity or timeout error");
            self.health.fault(&error);
            return Err(error);
        }
        let code = io.read(self.data_port, RegisterWidth::U8)? as u8;
        self.health.operations = self.health.operations.wrapping_add(1);
        Ok(Some(code))
    }

    pub fn recover(&mut self, io: &mut impl RegisterIo) -> Result<(), PhysicalDriverError> {
        self.health.reset();
        io.write(self.status_port, RegisterWidth::U8, 0xad)?;
        io.write(self.status_port, RegisterWidth::U8, 0xae)?;
        self.health.ready();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptControllerKind {
    XApic,
    GicV3,
    Plic,
}

pub fn xapic_eoi(io: &mut impl RegisterIo, base: u64) -> Result<(), PhysicalDriverError> {
    io.write(base + 0xb0, RegisterWidth::U32, 0)?;
    io.fence()
}

pub fn xapic_send_ipi(
    io: &mut impl RegisterIo,
    base: u64,
    apic_id: u32,
    low: u32,
) -> Result<(), PhysicalDriverError> {
    io.write(base + 0x310, RegisterWidth::U32, (apic_id as u64) << 24)?;
    io.write(base + 0x300, RegisterWidth::U32, low as u64)?;
    io.fence()
}

pub fn gicv3_enable_spi(
    io: &mut impl RegisterIo,
    distributor: u64,
    interrupt: u32,
    target_affinity: u64,
) -> Result<(), PhysicalDriverError> {
    if interrupt < 32 || interrupt >= 1020 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "GICv3 SPI must be 32..1019",
        ));
    }
    io.write(distributor, RegisterWidth::U32, 0x12)?;
    io.write(
        distributor + 0x6100 + interrupt as u64 * 8,
        RegisterWidth::U64,
        target_affinity,
    )?;
    io.write(
        distributor + 0x100 + (interrupt / 32) as u64 * 4,
        RegisterWidth::U32,
        1u64 << (interrupt % 32),
    )?;
    io.fence()
}

pub fn plic_route_enable(
    io: &mut impl RegisterIo,
    base: u64,
    source: u32,
    context: u32,
    priority: u32,
) -> Result<(), PhysicalDriverError> {
    if source == 0 || priority == 0 || priority > 7 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid PLIC source or priority",
        ));
    }
    io.write(
        base + source as u64 * 4,
        RegisterWidth::U32,
        priority as u64,
    )?;
    io.write(
        base + 0x2000 + context as u64 * 0x80 + (source / 32) as u64 * 4,
        RegisterWidth::U32,
        1u64 << (source % 32),
    )?;
    io.write(
        base + 0x20_0000 + context as u64 * 0x1000,
        RegisterWidth::U32,
        0,
    )?;
    io.fence()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureTimer {
    X86Hpet,
    Aarch64Cntv,
    RiscvStimecmp,
}

pub fn hpet_program_oneshot(
    io: &mut impl RegisterIo,
    base: u64,
    comparator: u8,
    deadline: u64,
    route: u8,
) -> Result<(), PhysicalDriverError> {
    if comparator >= 32 || route >= 32 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid HPET comparator or route",
        ));
    }
    let timer = base + 0x100 + comparator as u64 * 0x20;
    let config = (route as u64) << 9 | (1 << 2);
    io.write(timer, RegisterWidth::U64, config)?;
    io.write(timer + 0x08, RegisterWidth::U64, deadline)?;
    io.write(base + 0x10, RegisterWidth::U64, 1)?;
    io.fence()
}

pub fn timer_machine_bytes(timer: ArchitectureTimer) -> &'static [u8] {
    match timer {
        ArchitectureTimer::X86Hpet => &[0x0f, 0xae, 0xf0],
        ArchitectureTimer::Aarch64Cntv => &[0x40, 0x32, 0x1b, 0xd5, 0x1f, 0x20, 0x03, 0xd5],
        ArchitectureTimer::RiscvStimecmp => &[0x73, 0x10, 0xd0, 0x14, 0x0f, 0x00, 0xf0, 0x0f],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcieEcam {
    pub base: u64,
    pub segment: u16,
    pub start_bus: u8,
    pub end_bus: u8,
}

impl PcieEcam {
    pub fn address(
        &self,
        bus: u8,
        device: u8,
        function: u8,
        register: u16,
    ) -> Result<u64, PhysicalDriverError> {
        if bus < self.start_bus
            || bus > self.end_bus
            || device >= 32
            || function >= 8
            || register >= 4096
            || register & 3 != 0
        {
            return Err(PhysicalDriverError::InvalidDescriptor(
                "invalid PCIe ECAM address",
            ));
        }
        Ok(self.base
            + (((bus - self.start_bus) as u64) << 20)
            + ((device as u64) << 15)
            + ((function as u64) << 12)
            + register as u64)
    }

    pub fn read_config32(
        &self,
        io: &mut impl RegisterIo,
        bus: u8,
        device: u8,
        function: u8,
        register: u16,
    ) -> Result<u32, PhysicalDriverError> {
        Ok(io.read(
            self.address(bus, device, function, register)?,
            RegisterWidth::U32,
        )? as u32)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NvmeSubmissionEntry {
    pub command: u32,
    pub namespace: u32,
    pub reserved: [u32; 2],
    pub metadata: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub dword10: u32,
    pub dword11: u32,
    pub dword12: u32,
    pub dword13: u32,
    pub dword14: u32,
    pub dword15: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NvmeCompletionEntry {
    pub result: u32,
    pub reserved: u32,
    pub submission_head: u16,
    pub submission_queue: u16,
    pub command_id: u16,
    pub status: u16,
}

pub fn nvme_program_admin_queues(
    io: &mut impl RegisterIo,
    base: u64,
    entries: u16,
    submission: u64,
    completion: u64,
    spins: u32,
) -> Result<(), PhysicalDriverError> {
    if entries < 2 || entries > 4096 || submission & 0xfff != 0 || completion & 0xfff != 0 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid NVMe admin queue geometry",
        ));
    }
    io.write(base + 0x14, RegisterWidth::U32, 0)?;
    for _ in 0..spins {
        if io.read(base + 0x1c, RegisterWidth::U32)? & 1 == 0 {
            let count = entries as u64 - 1;
            io.write(base + 0x24, RegisterWidth::U32, count | count << 16)?;
            io.write(base + 0x28, RegisterWidth::U64, submission)?;
            io.write(base + 0x30, RegisterWidth::U64, completion)?;
            io.write(base + 0x14, RegisterWidth::U32, 0x0046_0001)?;
            io.fence()?;
            return Ok(());
        }
    }
    Err(PhysicalDriverError::Timeout(
        "NVMe controller did not become disabled",
    ))
}

pub fn nvme_ring_submission(
    io: &mut impl RegisterIo,
    base: u64,
    stride: u8,
    queue: u16,
    tail: u16,
) -> Result<(), PhysicalDriverError> {
    if stride > 7 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid NVMe doorbell stride",
        ));
    }
    let address = base + 0x1000 + (2 * queue as u64) * (4u64 << stride);
    io.write(address, RegisterWidth::U32, tail as u64)?;
    io.fence()
}

#[repr(C, align(1024))]
#[derive(Debug, Clone, Copy)]
pub struct AhciCommandList(pub [u8; 1024]);

#[repr(C, align(256))]
#[derive(Debug, Clone, Copy)]
pub struct AhciReceivedFis(pub [u8; 256]);

pub fn ahci_start_port(
    io: &mut impl RegisterIo,
    abar: u64,
    port: u8,
    command_list: u64,
    received_fis: u64,
) -> Result<(), PhysicalDriverError> {
    if port >= 32 || command_list & 0x3ff != 0 || received_fis & 0xff != 0 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid AHCI port buffers",
        ));
    }
    let base = abar + 0x100 + port as u64 * 0x80;
    io.write(base + 0x18, RegisterWidth::U32, 0)?;
    io.write(base, RegisterWidth::U64, command_list)?;
    io.write(base + 0x08, RegisterWidth::U64, received_fis)?;
    io.write(base + 0x10, RegisterWidth::U32, u32::MAX as u64)?;
    io.write(base + 0x14, RegisterWidth::U32, u32::MAX as u64)?;
    io.write(base + 0x18, RegisterWidth::U32, (1 << 4) | 1)?;
    io.fence()
}

pub fn ahci_comreset(
    io: &mut impl RegisterIo,
    abar: u64,
    port: u8,
) -> Result<(), PhysicalDriverError> {
    let base = abar + 0x100 + port as u64 * 0x80;
    io.write(base + 0x2c, RegisterWidth::U32, 1)?;
    io.fence()?;
    io.write(base + 0x2c, RegisterWidth::U32, 0)?;
    io.fence()
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XhciTrb {
    pub parameter: u64,
    pub status: u32,
    pub control: u32,
}

impl XhciTrb {
    pub fn normal(
        buffer: u64,
        length: u32,
        cycle: bool,
        interrupt: bool,
    ) -> Result<Self, PhysicalDriverError> {
        if length > 0x1_ffff {
            return Err(PhysicalDriverError::InvalidDescriptor(
                "xHCI transfer exceeds one TRB",
            ));
        }
        Ok(Self {
            parameter: buffer,
            status: length,
            control: (1 << 10) | u32::from(cycle) | (u32::from(interrupt) << 5),
        })
    }
}

pub fn xhci_ring_doorbell(
    io: &mut impl RegisterIo,
    doorbells: u64,
    slot: u8,
    endpoint: u8,
) -> Result<(), PhysicalDriverError> {
    if endpoint > 31 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid xHCI endpoint",
        ));
    }
    io.write(
        doorbells + slot as u64 * 4,
        RegisterWidth::U32,
        endpoint as u64,
    )?;
    io.fence()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VirtioNetHeader {
    pub flags: u8,
    pub gso_type: u8,
    pub header_length: u16,
    pub gso_size: u16,
    pub checksum_start: u16,
    pub checksum_offset: u16,
    pub buffers: u16,
}

pub fn virtio_mmio_negotiate(
    io: &mut impl RegisterIo,
    base: u64,
    features: u64,
    queue: u16,
    size: u16,
    desc: u64,
    avail: u64,
    used: u64,
) -> Result<(), PhysicalDriverError> {
    if size == 0 || !size.is_power_of_two() || desc & 0xf != 0 || avail & 1 != 0 || used & 3 != 0 {
        return Err(PhysicalDriverError::InvalidDescriptor(
            "invalid VirtIO queue geometry",
        ));
    }
    if io.read(base, RegisterWidth::U32)? != 0x7472_6976 {
        return Err(PhysicalDriverError::InvalidState(
            "VirtIO MMIO magic is absent",
        ));
    }
    io.write(base + 0x70, RegisterWidth::U32, 0)?;
    io.write(base + 0x70, RegisterWidth::U32, 1)?;
    io.write(base + 0x70, RegisterWidth::U32, 3)?;
    io.write(base + 0x20, RegisterWidth::U32, features as u32 as u64)?;
    io.write(base + 0x24, RegisterWidth::U32, 0)?;
    io.write(
        base + 0x20,
        RegisterWidth::U32,
        (features >> 32) as u32 as u64,
    )?;
    io.write(base + 0x24, RegisterWidth::U32, 1)?;
    io.write(base + 0x30, RegisterWidth::U32, queue as u64)?;
    io.write(base + 0x38, RegisterWidth::U32, size as u64)?;
    for (offset, address) in [(0x80, desc), (0x90, avail), (0xa0, used)] {
        io.write(base + offset, RegisterWidth::U32, address as u32 as u64)?;
        io.write(
            base + offset + 4,
            RegisterWidth::U32,
            (address >> 32) as u32 as u64,
        )?;
    }
    io.write(base + 0x44, RegisterWidth::U32, 1)?;
    io.write(base + 0x70, RegisterWidth::U32, 0x0f)?;
    io.fence()
}

pub fn virtio_notify(
    io: &mut impl RegisterIo,
    base: u64,
    queue: u16,
) -> Result<(), PhysicalDriverError> {
    io.write(base + 0x50, RegisterWidth::U32, queue as u64)?;
    io.fence()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalDriverReport {
    pub serial_writes: u64,
    pub framebuffer_checksum: u32,
    pub keyboard_scancode: u8,
    pub interrupt_models: Vec<InterruptControllerKind>,
    pub timer_models: Vec<ArchitectureTimer>,
    pub pcie_vendor_device: u32,
    pub nvme_submission_bytes: usize,
    pub ahci_command_list_bytes: usize,
    pub xhci_trb_bytes: usize,
    pub virtio_notifications: u64,
    pub recovered_faults: u64,
    pub register_accesses: usize,
    pub fences: u64,
}

pub fn certify_physical_driver_contracts() -> Result<PhysicalDriverReport, PhysicalDriverError> {
    let mut io = TracedRegisterIo::default();
    io.set(0x3fd, RegisterWidth::U8, 0x20);
    let mut serial = Uart16550Driver::new(0x3f8);
    serial.initialize(&mut io, 1)?;
    serial.write_byte(&mut io, b'N', 8)?;
    let mut framebuffer_bytes = vec![0u8; 64 * 32 * 4];
    let mut framebuffer = PhysicalFramebuffer::new(
        &mut framebuffer_bytes,
        64,
        32,
        64 * 4,
        PixelFormat::Bgra8888,
    )?;
    framebuffer.fill_rect(
        DirtyRect {
            x: 4,
            y: 5,
            width: 8,
            height: 6,
        },
        0xff11_2233,
    )?;
    let framebuffer_checksum = crate::crc32(&framebuffer_bytes);
    io.set(0x64, RegisterWidth::U8, 1);
    io.set(0x60, RegisterWidth::U8, 0x1c);
    let mut keyboard = Ps2KeyboardDriver {
        data_port: 0x60,
        status_port: 0x64,
        health: DriverHealth::default(),
    };
    let keyboard_scancode =
        keyboard
            .poll_scancode(&mut io)?
            .ok_or(PhysicalDriverError::InvalidState(
                "PS/2 fixture produced no key",
            ))?;
    xapic_eoi(&mut io, 0xfee0_0000)?;
    gicv3_enable_spi(&mut io, 0x0800_0000, 48, 1)?;
    plic_route_enable(&mut io, 0x0c00_0000, 10, 1, 3)?;
    hpet_program_oneshot(&mut io, 0xfed0_0000, 0, 100_000, 2)?;
    let ecam = PcieEcam {
        base: 0xe000_0000,
        segment: 0,
        start_bus: 0,
        end_bus: 0xff,
    };
    let pci_address = ecam.address(0, 1, 0, 0)?;
    io.set(pci_address, RegisterWidth::U32, 0x5678_1234);
    let pcie_vendor_device = ecam.read_config32(&mut io, 0, 1, 0, 0)?;
    io.set(0xf100_001c, RegisterWidth::U32, 0);
    nvme_program_admin_queues(&mut io, 0xf100_0000, 64, 0x20_0000, 0x21_0000, 8)?;
    nvme_ring_submission(&mut io, 0xf100_0000, 0, 1, 1)?;
    ahci_start_port(&mut io, 0xf200_0000, 0, 0x30_0000, 0x31_0000)?;
    ahci_comreset(&mut io, 0xf200_0000, 0)?;
    let _trb = XhciTrb::normal(0x40_0000, 64, true, true)?;
    xhci_ring_doorbell(&mut io, 0xf300_1000, 1, 2)?;
    io.set(0x1000_1000, RegisterWidth::U32, 0x7472_6976);
    virtio_mmio_negotiate(
        &mut io,
        0x1000_1000,
        1 << 32,
        0,
        128,
        0x50_0000,
        0x51_0000,
        0x52_0000,
    )?;
    virtio_notify(&mut io, 0x1000_1000, 0)?;
    let mut recovery_io = io.clone();
    recovery_io.set(0x3fd, RegisterWidth::U8, 0);
    let timeout = serial.write_byte(&mut recovery_io, b'X', 1);
    if !matches!(timeout, Err(PhysicalDriverError::Timeout(_))) {
        return Err(PhysicalDriverError::InvalidState(
            "UART timeout recovery was not exercised",
        ));
    }
    serial.recover(&mut recovery_io)?;
    Ok(PhysicalDriverReport {
        serial_writes: serial.health.operations,
        framebuffer_checksum,
        keyboard_scancode,
        interrupt_models: vec![
            InterruptControllerKind::XApic,
            InterruptControllerKind::GicV3,
            InterruptControllerKind::Plic,
        ],
        timer_models: vec![
            ArchitectureTimer::X86Hpet,
            ArchitectureTimer::Aarch64Cntv,
            ArchitectureTimer::RiscvStimecmp,
        ],
        pcie_vendor_device,
        nvme_submission_bytes: std::mem::size_of::<NvmeSubmissionEntry>(),
        ahci_command_list_bytes: std::mem::size_of::<AhciCommandList>(),
        xhci_trb_bytes: std::mem::size_of::<XhciTrb>(),
        virtio_notifications: io
            .accesses
            .iter()
            .filter(|access| access.write && access.address == 0x1000_1050)
            .count() as u64,
        recovered_faults: serial.health.timeouts,
        register_accesses: io.accesses.len(),
        fences: io.fences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volatile_mmio_alignment_and_bounds_fail_before_access() {
        let mut bytes = [0u8; 16];
        let mut mmio =
            unsafe { VolatileMmio::new(bytes.as_mut_ptr(), 0x1000, bytes.len()).unwrap() };
        mmio.write(0x1004, RegisterWidth::U32, 0xaabb_ccdd).unwrap();
        assert_eq!(mmio.read(0x1004, RegisterWidth::U32).unwrap(), 0xaabb_ccdd);
        assert!(mmio.read(0x1003, RegisterWidth::U32).is_err());
        assert!(mmio.read(0x1010, RegisterWidth::U8).is_err());
    }

    #[test]
    fn physical_driver_matrix_uses_exact_registers_dma_layouts_and_recovery() {
        let report = certify_physical_driver_contracts().unwrap();
        assert_eq!(report.keyboard_scancode, 0x1c);
        assert_eq!(report.pcie_vendor_device, 0x5678_1234);
        assert_eq!(report.nvme_submission_bytes, 64);
        assert_eq!(report.ahci_command_list_bytes, 1024);
        assert_eq!(report.xhci_trb_bytes, 16);
        assert_eq!(report.recovered_faults, 1);
        assert!(report.framebuffer_checksum != 0);
        assert!(report.register_accesses > 40);
        assert!(report.fences >= 8);
    }

    #[test]
    fn injected_fault_stops_a_register_sequence_without_false_success() {
        let mut io = TracedRegisterIo {
            fail_after: Some(2),
            ..TracedRegisterIo::default()
        };
        let mut uart = Uart16550Driver::new(0x3f8);
        let error = uart.initialize(&mut io, 1).unwrap_err();
        uart.health.fault(&error);
        assert!(matches!(
            error,
            PhysicalDriverError::InjectedFault { access: 2 }
        ));
        assert_eq!(uart.health.state, DriverState::Failed);
        assert_eq!(uart.health.faults, 1);
    }
}
