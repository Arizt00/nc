use core::fmt;

pub const X86_LOCAL_APIC_BASE: u64 = 0xfee0_0000;
pub const X86_IO_APIC_BASE: u64 = 0xfec0_0000;
pub const X86_HPET_BASE: u64 = 0xfed0_0000;
pub const X86_LOCAL_APIC_PAGE_BYTES: usize = 4096;
pub const X86_LOCAL_APIC_REGISTER_BYTES: usize = 1024;
pub const X86_IO_APIC_REDIRECTION_ENTRIES: usize = 24;
pub const X86_HPET_MAX_TIMERS: usize = 32;
pub const IA32_APIC_BASE_MSR: u32 = 0x1b;
pub const IA32_TSC_DEADLINE_MSR: u32 = 0x6e0;
pub const X2APIC_MSR_BASE: u32 = 0x800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicError {
    InvalidRegister(u32),
    UnalignedRegister(u32),
    ReadOnlyRegister(u32),
    WriteOnlyRegister(u32),
    InvalidVector(u8),
    InvalidDeliveryMode(u8),
    InvalidDestination(u32),
    InvalidTimerDivisor(u32),
    InvalidTimerMode(u8),
    InvalidIoApicPin(usize),
    InvalidPitChannel(usize),
    InvalidPitAccessMode(u8),
    InvalidPitMode(u8),
    InvalidHpetTimer(usize),
    InvalidHpetPeriod,
    PendingQueueFull,
    InvalidVcpu(usize),
    InvalidStartupSequence { vcpu: usize },
    ArithmeticOverflow,
}

impl fmt::Display for ApicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegister(register) => write!(f, "invalid APIC register {register:#x}"),
            Self::UnalignedRegister(register) => {
                write!(f, "APIC register {register:#x} is not 16-byte aligned")
            }
            Self::ReadOnlyRegister(register) => {
                write!(f, "APIC register {register:#x} is read-only")
            }
            Self::WriteOnlyRegister(register) => {
                write!(f, "APIC register {register:#x} is write-only")
            }
            Self::InvalidVector(vector) => write!(f, "invalid APIC vector {vector:#x}"),
            Self::InvalidDeliveryMode(mode) => write!(f, "invalid APIC delivery mode {mode}"),
            Self::InvalidDestination(destination) => {
                write!(f, "invalid APIC destination {destination:#x}")
            }
            Self::InvalidTimerDivisor(divisor) => {
                write!(f, "invalid local APIC timer divisor {divisor}")
            }
            Self::InvalidTimerMode(mode) => write!(f, "invalid local APIC timer mode {mode}"),
            Self::InvalidIoApicPin(pin) => write!(f, "invalid IOAPIC input pin {pin}"),
            Self::InvalidPitChannel(channel) => write!(f, "invalid PIT channel {channel}"),
            Self::InvalidPitAccessMode(mode) => write!(f, "invalid PIT access mode {mode}"),
            Self::InvalidPitMode(mode) => write!(f, "invalid PIT operating mode {mode}"),
            Self::InvalidHpetTimer(timer) => write!(f, "invalid HPET timer {timer}"),
            Self::InvalidHpetPeriod => write!(f, "HPET period must be non-zero"),
            Self::PendingQueueFull => write!(f, "APIC pending-event queue is full"),
            Self::InvalidVcpu(vcpu) => write!(f, "invalid virtual CPU index {vcpu}"),
            Self::InvalidStartupSequence { vcpu } => {
                write!(f, "vCPU {vcpu} received SIPI without an accepted INIT")
            }
            Self::ArithmeticOverflow => write!(f, "APIC/timer arithmetic overflow"),
        }
    }
}

impl std::error::Error for ApicError {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicDeliveryMode {
    Fixed = 0,
    LowestPriority = 1,
    Smi = 2,
    Nmi = 4,
    Init = 5,
    Startup = 6,
    ExtInt = 7,
}

impl ApicDeliveryMode {
    pub fn from_bits(bits: u8) -> Result<Self, ApicError> {
        match bits {
            0 => Ok(Self::Fixed),
            1 => Ok(Self::LowestPriority),
            2 => Ok(Self::Smi),
            4 => Ok(Self::Nmi),
            5 => Ok(Self::Init),
            6 => Ok(Self::Startup),
            7 => Ok(Self::ExtInt),
            value => Err(ApicError::InvalidDeliveryMode(value)),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicDestinationMode {
    Physical = 0,
    Logical = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicLevel {
    Deassert = 0,
    Assert = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicTriggerMode {
    Edge = 0,
    Level = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicDestinationShorthand {
    None = 0,
    SelfOnly = 1,
    AllIncludingSelf = 2,
    AllExcludingSelf = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApicInterruptCommand {
    pub vector: u8,
    pub delivery_mode: ApicDeliveryMode,
    pub destination_mode: ApicDestinationMode,
    pub level: ApicLevel,
    pub trigger_mode: ApicTriggerMode,
    pub shorthand: ApicDestinationShorthand,
    pub destination: u32,
}

impl ApicInterruptCommand {
    pub fn validate(self) -> Result<Self, ApicError> {
        if matches!(
            self.delivery_mode,
            ApicDeliveryMode::Fixed | ApicDeliveryMode::LowestPriority | ApicDeliveryMode::ExtInt
        ) && self.vector < 0x10
        {
            return Err(ApicError::InvalidVector(self.vector));
        }
        Ok(self)
    }

    pub fn encode_xapic(self) -> Result<(u32, u32), ApicError> {
        let command = self.validate()?;
        if command.destination > u8::MAX as u32 {
            return Err(ApicError::InvalidDestination(command.destination));
        }
        let low = command.vector as u32
            | ((command.delivery_mode as u32) << 8)
            | ((command.destination_mode as u32) << 11)
            | ((command.level as u32) << 14)
            | ((command.trigger_mode as u32) << 15)
            | ((command.shorthand as u32) << 18);
        Ok(((command.destination & 0xff) << 24, low))
    }

    pub fn encode_x2apic(self) -> Result<u64, ApicError> {
        let command = self.validate()?;
        let (_, low) = command.encode_xapic_relaxed_destination();
        Ok(((command.destination as u64) << 32) | low as u64)
    }

    fn encode_xapic_relaxed_destination(self) -> (u32, u32) {
        let low = self.vector as u32
            | ((self.delivery_mode as u32) << 8)
            | ((self.destination_mode as u32) << 11)
            | ((self.level as u32) << 14)
            | ((self.trigger_mode as u32) << 15)
            | ((self.shorthand as u32) << 18);
        ((self.destination & 0xff) << 24, low)
    }

    pub fn decode_xapic(high: u32, low: u32) -> Result<Self, ApicError> {
        Self::decode((high >> 24) & 0xff, low)
    }

    pub fn decode_x2apic(value: u64) -> Result<Self, ApicError> {
        Self::decode((value >> 32) as u32, value as u32)
    }

    fn decode(destination: u32, low: u32) -> Result<Self, ApicError> {
        let command = Self {
            vector: low as u8,
            delivery_mode: ApicDeliveryMode::from_bits(((low >> 8) & 7) as u8)?,
            destination_mode: if low & (1 << 11) == 0 {
                ApicDestinationMode::Physical
            } else {
                ApicDestinationMode::Logical
            },
            level: if low & (1 << 14) == 0 {
                ApicLevel::Deassert
            } else {
                ApicLevel::Assert
            },
            trigger_mode: if low & (1 << 15) == 0 {
                ApicTriggerMode::Edge
            } else {
                ApicTriggerMode::Level
            },
            shorthand: match (low >> 18) & 3 {
                0 => ApicDestinationShorthand::None,
                1 => ApicDestinationShorthand::SelfOnly,
                2 => ApicDestinationShorthand::AllIncludingSelf,
                _ => ApicDestinationShorthand::AllExcludingSelf,
            },
            destination,
        };
        command.validate()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalApicTimerMode {
    OneShot = 0,
    Periodic = 1,
    TscDeadline = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApicTimerExpiration {
    pub vector: u8,
    pub expirations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalApicTimer {
    pub vector: u8,
    pub mode: LocalApicTimerMode,
    pub masked: bool,
    pub divisor: u32,
    pub initial_count: u32,
    pub current_count: u32,
    pub tsc_deadline: u64,
    pub total_expirations: u64,
}

impl Default for LocalApicTimer {
    fn default() -> Self {
        Self {
            vector: 0,
            mode: LocalApicTimerMode::OneShot,
            masked: true,
            divisor: 2,
            initial_count: 0,
            current_count: 0,
            tsc_deadline: 0,
            total_expirations: 0,
        }
    }
}

impl LocalApicTimer {
    pub fn set_lvt(&mut self, value: u32) -> Result<(), ApicError> {
        let vector = value as u8;
        if vector != 0 && vector < 0x10 {
            return Err(ApicError::InvalidVector(vector));
        }
        self.vector = vector;
        self.masked = value & (1 << 16) != 0;
        self.mode = match (value >> 17) & 3 {
            0 => LocalApicTimerMode::OneShot,
            1 => LocalApicTimerMode::Periodic,
            2 => LocalApicTimerMode::TscDeadline,
            mode => return Err(ApicError::InvalidTimerMode(mode as u8)),
        };
        Ok(())
    }

    pub fn lvt(&self) -> u32 {
        self.vector as u32 | (u32::from(self.masked) << 16) | ((self.mode as u32) << 17)
    }

    pub fn set_divide_configuration(&mut self, raw: u32) -> Result<(), ApicError> {
        self.divisor = match raw & 0b1011 {
            0x0 => 2,
            0x1 => 4,
            0x2 => 8,
            0x3 => 16,
            0x8 => 32,
            0x9 => 64,
            0xa => 128,
            0xb => 1,
            _ => return Err(ApicError::InvalidTimerDivisor(raw)),
        };
        Ok(())
    }

    pub fn divide_configuration(&self) -> u32 {
        match self.divisor {
            1 => 0xb,
            2 => 0x0,
            4 => 0x1,
            8 => 0x2,
            16 => 0x3,
            32 => 0x8,
            64 => 0x9,
            128 => 0xa,
            _ => 0,
        }
    }

    pub fn arm(&mut self, initial_count: u32) {
        self.initial_count = initial_count;
        self.current_count = initial_count;
        if self.mode != LocalApicTimerMode::TscDeadline {
            self.tsc_deadline = 0;
        }
    }

    pub fn set_tsc_deadline(&mut self, deadline: u64) {
        self.tsc_deadline = deadline;
        self.current_count = 0;
    }

    pub fn advance_bus_ticks(
        &mut self,
        bus_ticks: u64,
    ) -> Result<Option<ApicTimerExpiration>, ApicError> {
        if self.masked
            || self.vector < 0x10
            || self.mode == LocalApicTimerMode::TscDeadline
            || self.current_count == 0
        {
            return Ok(None);
        }
        let timer_ticks = bus_ticks / self.divisor as u64;
        if timer_ticks == 0 {
            return Ok(None);
        }
        let current = self.current_count as u64;
        if timer_ticks < current {
            self.current_count -= timer_ticks as u32;
            return Ok(None);
        }
        let expirations = if self.mode == LocalApicTimerMode::Periodic && self.initial_count != 0 {
            let remaining = timer_ticks - current;
            let period = self.initial_count as u64;
            let expirations = 1 + remaining / period;
            let remainder = remaining % period;
            self.current_count = if remainder == 0 {
                self.initial_count
            } else {
                (period - remainder) as u32
            };
            expirations
        } else {
            self.current_count = 0;
            1
        };
        self.total_expirations = self
            .total_expirations
            .checked_add(expirations)
            .ok_or(ApicError::ArithmeticOverflow)?;
        Ok(Some(ApicTimerExpiration {
            vector: self.vector,
            expirations,
        }))
    }

    pub fn observe_tsc(
        &mut self,
        current_tsc: u64,
    ) -> Result<Option<ApicTimerExpiration>, ApicError> {
        if self.masked
            || self.vector < 0x10
            || self.mode != LocalApicTimerMode::TscDeadline
            || self.tsc_deadline == 0
            || current_tsc < self.tsc_deadline
        {
            return Ok(None);
        }
        self.tsc_deadline = 0;
        self.total_expirations = self
            .total_expirations
            .checked_add(1)
            .ok_or(ApicError::ArithmeticOverflow)?;
        Ok(Some(ApicTimerExpiration {
            vector: self.vector,
            expirations: 1,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalApic {
    pub id: u32,
    pub version: u8,
    pub software_enabled: bool,
    pub x2apic_enabled: bool,
    pub task_priority: u8,
    pub logical_destination: u32,
    pub destination_format: u32,
    pub spurious_vector: u8,
    pub error_status: u32,
    pub timer: LocalApicTimer,
    irr: [u32; 8],
    isr: [u32; 8],
    tmr: [u32; 8],
    icr_high: u32,
}

impl LocalApic {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            version: 0x14,
            software_enabled: false,
            x2apic_enabled: false,
            task_priority: 0,
            logical_destination: 0,
            destination_format: 0xffff_ffff,
            spurious_vector: 0xff,
            error_status: 0,
            timer: LocalApicTimer::default(),
            irr: [0; 8],
            isr: [0; 8],
            tmr: [0; 8],
            icr_high: 0,
        }
    }

    pub fn apic_base_msr(&self, bootstrap_processor: bool) -> u64 {
        X86_LOCAL_APIC_BASE
            | (u64::from(bootstrap_processor) << 8)
            | (u64::from(self.x2apic_enabled) << 10)
            | (1 << 11)
    }

    pub fn set_apic_base_msr(&mut self, value: u64) {
        self.software_enabled = value & (1 << 11) != 0;
        self.x2apic_enabled = value & (1 << 10) != 0;
    }

    pub fn read_mmio(&self, register: u32) -> Result<u32, ApicError> {
        validate_apic_register(register)?;
        match register {
            0x020 => Ok(self.id << 24),
            0x030 => Ok(self.version as u32 | (6 << 16)),
            0x080 => Ok(self.task_priority as u32),
            0x0a0 => Ok(self.processor_priority() as u32),
            0x0b0 => Err(ApicError::WriteOnlyRegister(register)),
            0x0d0 => Ok(self.logical_destination),
            0x0e0 => Ok(self.destination_format),
            0x0f0 => Ok(self.spurious_vector as u32 | (u32::from(self.software_enabled) << 8)),
            0x100..=0x170 => Ok(self.isr[((register - 0x100) / 0x10) as usize]),
            0x180..=0x1f0 => Ok(self.tmr[((register - 0x180) / 0x10) as usize]),
            0x200..=0x270 => Ok(self.irr[((register - 0x200) / 0x10) as usize]),
            0x280 => Ok(self.error_status),
            0x300 => Ok(0),
            0x310 => Ok(self.icr_high),
            0x320 => Ok(self.timer.lvt()),
            0x380 => Ok(self.timer.initial_count),
            0x390 => Ok(self.timer.current_count),
            0x3e0 => Ok(self.timer.divide_configuration()),
            _ => Err(ApicError::InvalidRegister(register)),
        }
    }

    pub fn write_mmio(
        &mut self,
        register: u32,
        value: u32,
    ) -> Result<Option<ApicInterruptCommand>, ApicError> {
        validate_apic_register(register)?;
        match register {
            0x020 | 0x030 | 0x0a0 | 0x100..=0x270 | 0x390 => {
                Err(ApicError::ReadOnlyRegister(register))
            }
            0x080 => {
                self.task_priority = value as u8;
                Ok(None)
            }
            0x0b0 => {
                self.eoi();
                Ok(None)
            }
            0x0d0 => {
                self.logical_destination = value;
                Ok(None)
            }
            0x0e0 => {
                self.destination_format = value;
                Ok(None)
            }
            0x0f0 => {
                let vector = value as u8;
                if vector < 0x10 {
                    return Err(ApicError::InvalidVector(vector));
                }
                self.spurious_vector = vector;
                self.software_enabled = value & (1 << 8) != 0;
                Ok(None)
            }
            0x280 => {
                self.error_status = 0;
                Ok(None)
            }
            0x300 => Ok(Some(ApicInterruptCommand::decode_xapic(
                self.icr_high,
                value,
            )?)),
            0x310 => {
                self.icr_high = value & 0xff00_0000;
                Ok(None)
            }
            0x320 => {
                self.timer.set_lvt(value)?;
                Ok(None)
            }
            0x380 => {
                self.timer.arm(value);
                Ok(None)
            }
            0x3e0 => {
                self.timer.set_divide_configuration(value)?;
                Ok(None)
            }
            _ => Err(ApicError::InvalidRegister(register)),
        }
    }

    pub fn read_x2apic(&self, msr: u32) -> Result<u64, ApicError> {
        let register = x2apic_msr_to_register(msr)?;
        if register == 0x300 {
            return Err(ApicError::WriteOnlyRegister(register));
        }
        Ok(self.read_mmio(register)? as u64)
    }

    pub fn write_x2apic(
        &mut self,
        msr: u32,
        value: u64,
    ) -> Result<Option<ApicInterruptCommand>, ApicError> {
        if msr == 0x83f {
            return Ok(Some(
                ApicInterruptCommand {
                    vector: value as u8,
                    delivery_mode: ApicDeliveryMode::Fixed,
                    destination_mode: ApicDestinationMode::Physical,
                    level: ApicLevel::Assert,
                    trigger_mode: ApicTriggerMode::Edge,
                    shorthand: ApicDestinationShorthand::SelfOnly,
                    destination: self.id,
                }
                .validate()?,
            ));
        }
        let register = x2apic_msr_to_register(msr)?;
        if register == 0x300 {
            return Ok(Some(ApicInterruptCommand::decode_x2apic(value)?));
        }
        self.write_mmio(register, value as u32)
    }

    pub fn accept_interrupt(&mut self, vector: u8, level: bool) -> Result<(), ApicError> {
        if vector < 0x10 {
            return Err(ApicError::InvalidVector(vector));
        }
        set_vector(&mut self.irr, vector, true);
        set_vector(&mut self.tmr, vector, level);
        Ok(())
    }

    pub fn acknowledge(&mut self) -> Option<u8> {
        let vector = highest_vector(&self.irr)?;
        if vector & 0xf0 <= self.task_priority & 0xf0 {
            return None;
        }
        set_vector(&mut self.irr, vector, false);
        set_vector(&mut self.isr, vector, true);
        Some(vector)
    }

    pub fn eoi(&mut self) -> Option<u8> {
        let vector = highest_vector(&self.isr)?;
        set_vector(&mut self.isr, vector, false);
        Some(vector)
    }

    pub fn processor_priority(&self) -> u8 {
        self.task_priority
            .max(highest_vector(&self.isr).unwrap_or(0) & 0xf0)
    }
}

fn validate_apic_register(register: u32) -> Result<(), ApicError> {
    if register >= X86_LOCAL_APIC_REGISTER_BYTES as u32 {
        return Err(ApicError::InvalidRegister(register));
    }
    if register & 0xf != 0 {
        return Err(ApicError::UnalignedRegister(register));
    }
    Ok(())
}

fn x2apic_msr_to_register(msr: u32) -> Result<u32, ApicError> {
    if !(X2APIC_MSR_BASE..=0x83f).contains(&msr) {
        return Err(ApicError::InvalidRegister(msr));
    }
    Ok((msr - X2APIC_MSR_BASE) << 4)
}

fn set_vector(bitmap: &mut [u32; 8], vector: u8, value: bool) {
    let word = (vector / 32) as usize;
    let bit = 1_u32 << (vector % 32);
    if value {
        bitmap[word] |= bit;
    } else {
        bitmap[word] &= !bit;
    }
}

fn highest_vector(bitmap: &[u32; 8]) -> Option<u8> {
    for word in (0..bitmap.len()).rev() {
        let value = bitmap[word];
        if value != 0 {
            return Some((word * 32 + (31 - value.leading_zeros()) as usize) as u8);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoApicRedirectionEntry {
    pub vector: u8,
    pub delivery_mode: ApicDeliveryMode,
    pub destination_mode: ApicDestinationMode,
    pub active_low: bool,
    pub remote_irr: bool,
    pub trigger_mode: ApicTriggerMode,
    pub masked: bool,
    pub destination: u8,
}

impl Default for IoApicRedirectionEntry {
    fn default() -> Self {
        Self {
            vector: 0,
            delivery_mode: ApicDeliveryMode::Fixed,
            destination_mode: ApicDestinationMode::Physical,
            active_low: false,
            remote_irr: false,
            trigger_mode: ApicTriggerMode::Edge,
            masked: true,
            destination: 0,
        }
    }
}

impl IoApicRedirectionEntry {
    pub fn encode(self) -> Result<u64, ApicError> {
        if !self.masked && self.vector < 0x10 {
            return Err(ApicError::InvalidVector(self.vector));
        }
        Ok(self.vector as u64
            | ((self.delivery_mode as u64) << 8)
            | ((self.destination_mode as u64) << 11)
            | (u64::from(self.active_low) << 13)
            | (u64::from(self.remote_irr) << 14)
            | ((self.trigger_mode as u64) << 15)
            | (u64::from(self.masked) << 16)
            | ((self.destination as u64) << 56))
    }

    pub fn decode(value: u64) -> Result<Self, ApicError> {
        let entry = Self {
            vector: value as u8,
            delivery_mode: ApicDeliveryMode::from_bits(((value >> 8) & 7) as u8)?,
            destination_mode: if value & (1 << 11) == 0 {
                ApicDestinationMode::Physical
            } else {
                ApicDestinationMode::Logical
            },
            active_low: value & (1 << 13) != 0,
            remote_irr: value & (1 << 14) != 0,
            trigger_mode: if value & (1 << 15) == 0 {
                ApicTriggerMode::Edge
            } else {
                ApicTriggerMode::Level
            },
            masked: value & (1 << 16) != 0,
            destination: (value >> 56) as u8,
        };
        entry.encode()?;
        Ok(entry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoApic {
    pub id: u8,
    pub arbitration_id: u8,
    pub selector: u8,
    pub pins: [IoApicRedirectionEntry; X86_IO_APIC_REDIRECTION_ENTRIES],
    asserted: [bool; X86_IO_APIC_REDIRECTION_ENTRIES],
}

impl IoApic {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            arbitration_id: id,
            selector: 0,
            pins: [IoApicRedirectionEntry::default(); X86_IO_APIC_REDIRECTION_ENTRIES],
            asserted: [false; X86_IO_APIC_REDIRECTION_ENTRIES],
        }
    }

    pub fn select(&mut self, register: u8) {
        self.selector = register;
    }

    pub fn read_window(&self) -> Result<u32, ApicError> {
        match self.selector {
            0x00 => Ok((self.id as u32) << 24),
            0x01 => Ok(0x11 | (((X86_IO_APIC_REDIRECTION_ENTRIES - 1) as u32) << 16)),
            0x02 => Ok((self.arbitration_id as u32) << 24),
            0x10..=0x3f => {
                let index = ((self.selector - 0x10) / 2) as usize;
                let encoded = self.pins[index].encode()?;
                Ok(if self.selector & 1 == 0 {
                    encoded as u32
                } else {
                    (encoded >> 32) as u32
                })
            }
            register => Err(ApicError::InvalidRegister(register as u32)),
        }
    }

    pub fn write_window(&mut self, value: u32) -> Result<(), ApicError> {
        match self.selector {
            0x00 => {
                self.id = (value >> 24) as u8;
                Ok(())
            }
            0x01 | 0x02 => Err(ApicError::ReadOnlyRegister(self.selector as u32)),
            0x10..=0x3f => {
                let index = ((self.selector - 0x10) / 2) as usize;
                let mut encoded = self.pins[index].encode()?;
                if self.selector & 1 == 0 {
                    encoded = (encoded & 0xffff_ffff_0000_0000) | value as u64;
                } else {
                    encoded = (encoded & 0x0000_0000_ffff_ffff) | ((value as u64) << 32);
                }
                self.pins[index] = IoApicRedirectionEntry::decode(encoded)?;
                Ok(())
            }
            register => Err(ApicError::InvalidRegister(register as u32)),
        }
    }

    pub fn set_irq(
        &mut self,
        pin: usize,
        electrical_level: bool,
    ) -> Result<Option<ApicInterruptCommand>, ApicError> {
        let entry = *self.pins.get(pin).ok_or(ApicError::InvalidIoApicPin(pin))?;
        let active = if entry.active_low {
            !electrical_level
        } else {
            electrical_level
        };
        let was_active = self.asserted[pin];
        self.asserted[pin] = active;
        if entry.masked
            || !active
            || (entry.trigger_mode == ApicTriggerMode::Edge && was_active)
            || (entry.trigger_mode == ApicTriggerMode::Level && entry.remote_irr)
        {
            return Ok(None);
        }
        if entry.trigger_mode == ApicTriggerMode::Level {
            self.pins[pin].remote_irr = true;
        }
        Ok(Some(
            ApicInterruptCommand {
                vector: entry.vector,
                delivery_mode: entry.delivery_mode,
                destination_mode: entry.destination_mode,
                level: ApicLevel::Assert,
                trigger_mode: entry.trigger_mode,
                shorthand: ApicDestinationShorthand::None,
                destination: entry.destination as u32,
            }
            .validate()?,
        ))
    }

    pub fn eoi(&mut self, vector: u8) {
        for entry in &mut self.pins {
            if entry.vector == vector && entry.trigger_mode == ApicTriggerMode::Level {
                entry.remote_irr = false;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicEvent {
    Interrupt { vector: u8, level: bool },
    Nmi,
    Smi,
    Init,
    Startup { vector: u8, entry: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualCpuLifecycle {
    Reset,
    WaitForSipi,
    Runnable { entry: u64 },
    Halted,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualCpuTopology {
    pub apic_id: u32,
    pub package: u16,
    pub core: u16,
    pub thread: u16,
}

impl VirtualCpuTopology {
    pub const fn new(apic_id: u32, package: u16, core: u16, thread: u16) -> Self {
        Self {
            apic_id,
            package,
            core,
            thread,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApicPendingQueue<const CAPACITY: usize> {
    entries: [Option<ApicEvent>; CAPACITY],
}

impl<const CAPACITY: usize> Default for ApicPendingQueue<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> ApicPendingQueue<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
        }
    }

    pub fn push(&mut self, event: ApicEvent) -> Result<(), ApicError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(ApicError::PendingQueueFull)?;
        *slot = Some(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<ApicEvent> {
        let index = self.entries.iter().position(Option::is_some)?;
        let event = self.entries[index].take();
        self.entries[..].rotate_left(1);
        self.entries[CAPACITY.saturating_sub(1)] = None;
        event
    }

    pub fn len(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApicBus<const VCPUS: usize, const EVENTS: usize> {
    pub local_apics: [LocalApic; VCPUS],
    pub topology: [VirtualCpuTopology; VCPUS],
    pub lifecycle: [VirtualCpuLifecycle; VCPUS],
    pending: [ApicPendingQueue<EVENTS>; VCPUS],
}

impl<const VCPUS: usize, const EVENTS: usize> ApicBus<VCPUS, EVENTS> {
    pub fn new(topology: [VirtualCpuTopology; VCPUS]) -> Self {
        let local_apics = core::array::from_fn(|index| LocalApic::new(topology[index].apic_id));
        let lifecycle = core::array::from_fn(|index| {
            if index == 0 {
                VirtualCpuLifecycle::Runnable { entry: 0 }
            } else {
                VirtualCpuLifecycle::WaitForSipi
            }
        });
        Self {
            local_apics,
            topology,
            lifecycle,
            pending: core::array::from_fn(|_| ApicPendingQueue::new()),
        }
    }

    pub fn route_command(
        &mut self,
        source_vcpu: usize,
        command: ApicInterruptCommand,
    ) -> Result<usize, ApicError> {
        if source_vcpu >= VCPUS {
            return Err(ApicError::InvalidVcpu(source_vcpu));
        }
        let command = command.validate()?;
        let mut candidates = [false; VCPUS];
        for (index, local) in self.local_apics.iter().enumerate() {
            candidates[index] = match command.shorthand {
                ApicDestinationShorthand::SelfOnly => index == source_vcpu,
                ApicDestinationShorthand::AllIncludingSelf => true,
                ApicDestinationShorthand::AllExcludingSelf => index != source_vcpu,
                ApicDestinationShorthand::None => match command.destination_mode {
                    ApicDestinationMode::Physical => local.id == command.destination,
                    ApicDestinationMode::Logical => {
                        local.logical_destination & command.destination != 0
                    }
                },
            };
        }
        if command.delivery_mode == ApicDeliveryMode::LowestPriority {
            let selected = (0..VCPUS)
                .filter(|index| candidates[*index])
                .min_by_key(|index| self.local_apics[*index].processor_priority());
            candidates.fill(false);
            if let Some(selected) = selected {
                candidates[selected] = true;
            }
        }

        let mut delivered = 0;
        for (index, selected) in candidates.into_iter().enumerate() {
            if !selected {
                continue;
            }
            let event = match command.delivery_mode {
                ApicDeliveryMode::Fixed
                | ApicDeliveryMode::LowestPriority
                | ApicDeliveryMode::ExtInt => {
                    self.local_apics[index].accept_interrupt(
                        command.vector,
                        command.trigger_mode == ApicTriggerMode::Level,
                    )?;
                    ApicEvent::Interrupt {
                        vector: command.vector,
                        level: command.trigger_mode == ApicTriggerMode::Level,
                    }
                }
                ApicDeliveryMode::Nmi => ApicEvent::Nmi,
                ApicDeliveryMode::Smi => ApicEvent::Smi,
                ApicDeliveryMode::Init => {
                    self.lifecycle[index] = VirtualCpuLifecycle::WaitForSipi;
                    ApicEvent::Init
                }
                ApicDeliveryMode::Startup => {
                    if self.lifecycle[index] != VirtualCpuLifecycle::WaitForSipi {
                        return Err(ApicError::InvalidStartupSequence { vcpu: index });
                    }
                    let entry = (command.vector as u64) << 12;
                    self.lifecycle[index] = VirtualCpuLifecycle::Runnable { entry };
                    ApicEvent::Startup {
                        vector: command.vector,
                        entry,
                    }
                }
            };
            self.pending[index].push(event)?;
            delivered += 1;
        }
        Ok(delivered)
    }

    pub fn route_ioapic(
        &mut self,
        ioapic: &mut IoApic,
        pin: usize,
        level: bool,
    ) -> Result<usize, ApicError> {
        let Some(command) = ioapic.set_irq(pin, level)? else {
            return Ok(0);
        };
        self.route_command(0, command)
    }

    pub fn advance_local_timer(&mut self, vcpu: usize, bus_ticks: u64) -> Result<u64, ApicError> {
        let local = self
            .local_apics
            .get_mut(vcpu)
            .ok_or(ApicError::InvalidVcpu(vcpu))?;
        let Some(expiration) = local.timer.advance_bus_ticks(bus_ticks)? else {
            return Ok(0);
        };
        self.pending[vcpu].push(ApicEvent::Interrupt {
            vector: expiration.vector,
            level: false,
        })?;
        Ok(expiration.expirations)
    }

    pub fn observe_tsc(&mut self, vcpu: usize, tsc: u64) -> Result<bool, ApicError> {
        let local = self
            .local_apics
            .get_mut(vcpu)
            .ok_or(ApicError::InvalidVcpu(vcpu))?;
        let Some(expiration) = local.timer.observe_tsc(tsc)? else {
            return Ok(false);
        };
        self.pending[vcpu].push(ApicEvent::Interrupt {
            vector: expiration.vector,
            level: false,
        })?;
        Ok(true)
    }

    pub fn pop_event(&mut self, vcpu: usize) -> Result<Option<ApicEvent>, ApicError> {
        Ok(self
            .pending
            .get_mut(vcpu)
            .ok_or(ApicError::InvalidVcpu(vcpu))?
            .pop())
    }

    pub fn pending_events(&self, vcpu: usize) -> Result<usize, ApicError> {
        Ok(self
            .pending
            .get(vcpu)
            .ok_or(ApicError::InvalidVcpu(vcpu))?
            .len())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitAccessMode {
    Latch = 0,
    LowByte = 1,
    HighByte = 2,
    LowHigh = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitOperatingMode {
    InterruptOnTerminalCount = 0,
    HardwareRetriggerableOneShot = 1,
    RateGenerator = 2,
    SquareWave = 3,
    SoftwareTriggeredStrobe = 4,
    HardwareTriggeredStrobe = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PitChannel {
    pub reload: u16,
    pub counter: u32,
    pub access: PitAccessMode,
    pub mode: PitOperatingMode,
    pub bcd: bool,
    pub output: bool,
    pub gate: bool,
    write_low_pending: bool,
    read_low_pending: bool,
    latched: Option<u16>,
}

impl Default for PitChannel {
    fn default() -> Self {
        Self {
            reload: 0,
            counter: 0x1_0000,
            access: PitAccessMode::LowHigh,
            mode: PitOperatingMode::InterruptOnTerminalCount,
            bcd: false,
            output: false,
            gate: true,
            write_low_pending: true,
            read_low_pending: true,
            latched: None,
        }
    }
}

impl PitChannel {
    fn effective_reload(&self) -> u32 {
        if self.reload == 0 {
            0x1_0000
        } else {
            self.reload as u32
        }
    }

    fn load_counter(&mut self) {
        self.counter = self.effective_reload();
        self.output = matches!(
            self.mode,
            PitOperatingMode::RateGenerator | PitOperatingMode::SquareWave
        );
    }

    fn write(&mut self, value: u8) {
        match self.access {
            PitAccessMode::LowByte => {
                self.reload = (self.reload & 0xff00) | value as u16;
                self.load_counter();
            }
            PitAccessMode::HighByte => {
                self.reload = (self.reload & 0x00ff) | ((value as u16) << 8);
                self.load_counter();
            }
            PitAccessMode::LowHigh => {
                if self.write_low_pending {
                    self.reload = (self.reload & 0xff00) | value as u16;
                } else {
                    self.reload = (self.reload & 0x00ff) | ((value as u16) << 8);
                    self.load_counter();
                }
                self.write_low_pending = !self.write_low_pending;
            }
            PitAccessMode::Latch => {}
        }
    }

    fn read(&mut self) -> u8 {
        let value = self.latched.unwrap_or(self.counter as u16);
        let result = match self.access {
            PitAccessMode::HighByte => (value >> 8) as u8,
            PitAccessMode::LowHigh if !self.read_low_pending => (value >> 8) as u8,
            _ => value as u8,
        };
        if self.access == PitAccessMode::LowHigh {
            self.read_low_pending = !self.read_low_pending;
            if self.read_low_pending {
                self.latched = None;
            }
        } else {
            self.latched = None;
        }
        result
    }

    fn advance(&mut self, ticks: u64) -> u64 {
        if !self.gate || ticks == 0 {
            return 0;
        }
        let counter = self.counter.max(1) as u64;
        if ticks < counter {
            self.counter -= ticks as u32;
            return 0;
        }
        match self.mode {
            PitOperatingMode::RateGenerator | PitOperatingMode::SquareWave => {
                let remaining = ticks - counter;
                let period = self.effective_reload() as u64;
                let pulses = 1 + remaining / period;
                let remainder = remaining % period;
                self.counter = if remainder == 0 {
                    period as u32
                } else {
                    (period - remainder) as u32
                };
                pulses
            }
            _ => {
                self.counter = 0;
                self.output = true;
                1
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pit8254 {
    pub channels: [PitChannel; 3],
}

impl Default for Pit8254 {
    fn default() -> Self {
        Self {
            channels: [PitChannel::default(); 3],
        }
    }
}

impl Pit8254 {
    pub fn write_control(&mut self, control: u8) -> Result<(), ApicError> {
        let channel = ((control >> 6) & 3) as usize;
        if channel == 3 {
            return self.read_back(control);
        }
        let access_bits = (control >> 4) & 3;
        if access_bits == 0 {
            self.channels[channel].latched = Some(self.channels[channel].counter as u16);
            return Ok(());
        }
        let mode_bits = (control >> 1) & 7;
        let mode_bits = if mode_bits >= 6 {
            mode_bits - 4
        } else {
            mode_bits
        };
        self.channels[channel].access = match access_bits {
            1 => PitAccessMode::LowByte,
            2 => PitAccessMode::HighByte,
            3 => PitAccessMode::LowHigh,
            value => return Err(ApicError::InvalidPitAccessMode(value)),
        };
        self.channels[channel].mode = match mode_bits {
            0 => PitOperatingMode::InterruptOnTerminalCount,
            1 => PitOperatingMode::HardwareRetriggerableOneShot,
            2 => PitOperatingMode::RateGenerator,
            3 => PitOperatingMode::SquareWave,
            4 => PitOperatingMode::SoftwareTriggeredStrobe,
            5 => PitOperatingMode::HardwareTriggeredStrobe,
            value => return Err(ApicError::InvalidPitMode(value)),
        };
        self.channels[channel].bcd = control & 1 != 0;
        self.channels[channel].write_low_pending = true;
        self.channels[channel].read_low_pending = true;
        Ok(())
    }

    fn read_back(&mut self, control: u8) -> Result<(), ApicError> {
        if control & (1 << 5) == 0 {
            for channel in 0..3 {
                if control & (1 << (channel + 1)) == 0 {
                    self.channels[channel].latched = Some(self.channels[channel].counter as u16);
                }
            }
        }
        Ok(())
    }

    pub fn write_channel(&mut self, channel: usize, value: u8) -> Result<(), ApicError> {
        self.channels
            .get_mut(channel)
            .ok_or(ApicError::InvalidPitChannel(channel))?
            .write(value);
        Ok(())
    }

    pub fn read_channel(&mut self, channel: usize) -> Result<u8, ApicError> {
        Ok(self
            .channels
            .get_mut(channel)
            .ok_or(ApicError::InvalidPitChannel(channel))?
            .read())
    }

    pub fn advance(&mut self, ticks: u64) -> [u64; 3] {
        core::array::from_fn(|channel| self.channels[channel].advance(ticks))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpetTimer {
    pub enabled: bool,
    pub periodic: bool,
    pub interrupt_enabled: bool,
    pub level_triggered: bool,
    pub route: u8,
    pub fsb_enabled: bool,
    pub fsb_address: u64,
    pub fsb_data: u32,
    pub comparator: u64,
    pub period: u64,
    pub interrupts: u64,
}

impl Default for HpetTimer {
    fn default() -> Self {
        Self {
            enabled: false,
            periodic: false,
            interrupt_enabled: false,
            level_triggered: false,
            route: 0,
            fsb_enabled: false,
            fsb_address: 0,
            fsb_data: 0,
            comparator: u64::MAX,
            period: 0,
            interrupts: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hpet<const TIMERS: usize> {
    pub period_femtoseconds: u32,
    pub counter_64bit: bool,
    pub legacy_replacement: bool,
    pub enabled: bool,
    pub main_counter: u64,
    pub interrupt_status: u64,
    pub timers: [HpetTimer; TIMERS],
}

impl<const TIMERS: usize> Hpet<TIMERS> {
    pub fn new(period_femtoseconds: u32) -> Result<Self, ApicError> {
        if period_femtoseconds == 0 || TIMERS == 0 || TIMERS > X86_HPET_MAX_TIMERS {
            return Err(ApicError::InvalidHpetPeriod);
        }
        Ok(Self {
            period_femtoseconds,
            counter_64bit: true,
            legacy_replacement: false,
            enabled: false,
            main_counter: 0,
            interrupt_status: 0,
            timers: [HpetTimer::default(); TIMERS],
        })
    }

    pub fn capabilities(&self) -> u64 {
        1 | (((TIMERS - 1) as u64) << 8)
            | (u64::from(self.counter_64bit) << 13)
            | (u64::from(true) << 15)
            | ((self.period_femtoseconds as u64) << 32)
    }

    pub fn configure_timer(&mut self, timer: usize, value: HpetTimer) -> Result<(), ApicError> {
        if value.periodic && value.period == 0 {
            return Err(ApicError::InvalidHpetPeriod);
        }
        *self
            .timers
            .get_mut(timer)
            .ok_or(ApicError::InvalidHpetTimer(timer))? = value;
        Ok(())
    }

    pub fn advance(&mut self, ticks: u64) -> Result<u64, ApicError> {
        if !self.enabled || ticks == 0 {
            return Ok(0);
        }
        let previous = self.main_counter;
        self.main_counter = self
            .main_counter
            .checked_add(ticks)
            .ok_or(ApicError::ArithmeticOverflow)?;
        let mut fired = 0_u64;
        for (index, timer) in self.timers.iter_mut().enumerate() {
            if !timer.enabled
                || !timer.interrupt_enabled
                || timer.comparator > self.main_counter
                || timer.comparator <= previous
            {
                continue;
            }
            let expirations = if timer.periodic {
                let expirations = 1 + (self.main_counter - timer.comparator) / timer.period;
                timer.comparator = timer
                    .comparator
                    .checked_add(expirations * timer.period)
                    .ok_or(ApicError::ArithmeticOverflow)?;
                expirations
            } else {
                timer.enabled = false;
                1
            };
            timer.interrupts = timer
                .interrupts
                .checked_add(expirations)
                .ok_or(ApicError::ArithmeticOverflow)?;
            self.interrupt_status |= 1_u64 << index;
            fired |= 1_u64 << index;
        }
        Ok(fired)
    }

    pub fn clear_interrupt(&mut self, timer: usize) -> Result<(), ApicError> {
        if timer >= TIMERS {
            return Err(ApicError::InvalidHpetTimer(timer));
        }
        self.interrupt_status &= !(1_u64 << timer);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualTsc {
    pub frequency_hz: u64,
    pub offset: i64,
    pub scale_numerator: u64,
    pub scale_denominator: u64,
    pub value: u64,
}

impl VirtualTsc {
    pub fn new(frequency_hz: u64) -> Result<Self, ApicError> {
        if frequency_hz == 0 {
            return Err(ApicError::InvalidHpetPeriod);
        }
        Ok(Self {
            frequency_hz,
            offset: 0,
            scale_numerator: 1,
            scale_denominator: 1,
            value: 0,
        })
    }

    pub fn set_scale(&mut self, numerator: u64, denominator: u64) -> Result<(), ApicError> {
        if numerator == 0 || denominator == 0 {
            return Err(ApicError::InvalidHpetPeriod);
        }
        self.scale_numerator = numerator;
        self.scale_denominator = denominator;
        Ok(())
    }

    pub fn advance_host_ticks(&mut self, host_ticks: u64) -> Result<u64, ApicError> {
        let scaled = (host_ticks as u128)
            .checked_mul(self.scale_numerator as u128)
            .ok_or(ApicError::ArithmeticOverflow)?
            / self.scale_denominator as u128;
        let scaled = u64::try_from(scaled).map_err(|_| ApicError::ArithmeticOverflow)?;
        self.value = self
            .value
            .checked_add(scaled)
            .ok_or(ApicError::ArithmeticOverflow)?;
        Ok(self.read())
    }

    pub fn read(&self) -> u64 {
        if self.offset >= 0 {
            self.value.wrapping_add(self.offset as u64)
        } else {
            self.value.wrapping_sub(self.offset.unsigned_abs())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xapic_x2apic_icr_roundtrip_and_init_sipi_are_material() {
        let topology = [
            VirtualCpuTopology::new(0, 0, 0, 0),
            VirtualCpuTopology::new(1, 0, 1, 0),
        ];
        let mut bus = ApicBus::<2, 8>::new(topology);
        let init = ApicInterruptCommand {
            vector: 0,
            delivery_mode: ApicDeliveryMode::Init,
            destination_mode: ApicDestinationMode::Physical,
            level: ApicLevel::Assert,
            trigger_mode: ApicTriggerMode::Level,
            shorthand: ApicDestinationShorthand::None,
            destination: 1,
        };
        let encoded = init.encode_x2apic().unwrap();
        assert_eq!(ApicInterruptCommand::decode_x2apic(encoded).unwrap(), init);
        assert_eq!(bus.route_command(0, init).unwrap(), 1);
        assert_eq!(bus.pop_event(1).unwrap(), Some(ApicEvent::Init));

        let sipi = ApicInterruptCommand {
            vector: 0x08,
            delivery_mode: ApicDeliveryMode::Startup,
            trigger_mode: ApicTriggerMode::Edge,
            ..init
        };
        assert_eq!(bus.route_command(0, sipi).unwrap(), 1);
        assert_eq!(
            bus.lifecycle[1],
            VirtualCpuLifecycle::Runnable { entry: 0x8000 }
        );
        assert_eq!(
            bus.pop_event(1).unwrap(),
            Some(ApicEvent::Startup {
                vector: 8,
                entry: 0x8000
            })
        );
    }

    #[test]
    fn local_apic_priorities_eoi_and_all_timer_modes_are_deterministic() {
        let mut apic = LocalApic::new(7);
        apic.write_mmio(0x0f0, 0x1ff).unwrap();
        apic.accept_interrupt(0x31, false).unwrap();
        apic.accept_interrupt(0x72, true).unwrap();
        assert_eq!(apic.acknowledge(), Some(0x72));
        assert_eq!(apic.processor_priority(), 0x70);
        assert_eq!(apic.eoi(), Some(0x72));
        assert_eq!(apic.acknowledge(), Some(0x31));

        apic.timer.set_lvt(0x40 | (1 << 17)).unwrap();
        apic.timer.set_divide_configuration(0xb).unwrap();
        apic.timer.arm(10);
        let expiration = apic.timer.advance_bus_ticks(35).unwrap().unwrap();
        assert_eq!(expiration.expirations, 3);
        assert_eq!(apic.timer.current_count, 5);

        apic.timer.set_lvt(0x41 | (2 << 17)).unwrap();
        apic.timer.set_tsc_deadline(1000);
        assert!(apic.timer.observe_tsc(999).unwrap().is_none());
        assert_eq!(apic.timer.observe_tsc(1000).unwrap().unwrap().vector, 0x41);
    }

    #[test]
    fn ioapic_routes_edge_and_level_inputs_with_remote_irr_and_eoi() {
        let mut ioapic = IoApic::new(2);
        ioapic.pins[16] = IoApicRedirectionEntry {
            vector: 0x52,
            destination: 1,
            masked: false,
            trigger_mode: ApicTriggerMode::Level,
            ..IoApicRedirectionEntry::default()
        };
        let topology = [
            VirtualCpuTopology::new(0, 0, 0, 0),
            VirtualCpuTopology::new(1, 0, 1, 0),
        ];
        let mut bus = ApicBus::<2, 8>::new(topology);
        assert_eq!(bus.route_ioapic(&mut ioapic, 16, true).unwrap(), 1);
        assert_eq!(bus.route_ioapic(&mut ioapic, 16, true).unwrap(), 0);
        assert_eq!(
            bus.pop_event(1).unwrap(),
            Some(ApicEvent::Interrupt {
                vector: 0x52,
                level: true
            })
        );
        ioapic.eoi(0x52);
        bus.route_ioapic(&mut ioapic, 16, false).unwrap();
        assert_eq!(bus.route_ioapic(&mut ioapic, 16, true).unwrap(), 1);
    }

    #[test]
    fn pit_hpet_and_virtual_tsc_cover_one_shot_periodic_and_scaled_time() {
        let mut pit = Pit8254::default();
        pit.write_control(0b00_11_010_0).unwrap();
        pit.write_channel(0, 10).unwrap();
        pit.write_channel(0, 0).unwrap();
        assert_eq!(pit.advance(9)[0], 0);
        assert_eq!(pit.advance(1)[0], 1);
        assert_eq!(pit.advance(20)[0], 2);

        let mut hpet = Hpet::<3>::new(100_000_000).unwrap();
        hpet.enabled = true;
        hpet.configure_timer(
            1,
            HpetTimer {
                enabled: true,
                periodic: true,
                interrupt_enabled: true,
                comparator: 10,
                period: 10,
                route: 16,
                ..HpetTimer::default()
            },
        )
        .unwrap();
        assert_eq!(hpet.advance(35).unwrap(), 1 << 1);
        assert_eq!(hpet.timers[1].interrupts, 3);
        assert_eq!(hpet.timers[1].comparator, 40);

        let mut tsc = VirtualTsc::new(3_000_000_000).unwrap();
        tsc.set_scale(3, 2).unwrap();
        tsc.offset = -5;
        assert_eq!(tsc.advance_host_ticks(10).unwrap(), 10);
    }
}
