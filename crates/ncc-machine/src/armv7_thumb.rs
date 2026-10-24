use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const EM_ARM: u16 = 40;
pub const EF_ARM_EABI_VER5: u32 = 0x0500_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbError {
    InvalidRegister(u8),
    ImmediateOutOfRange(&'static str),
    BranchOutOfRange(i64),
    UnalignedBranch(i64),
    InvalidRegisterList,
    InvalidRelocationOffset(usize),
    MissingSymbol(String),
    AddressOverflow,
    InvalidInstruction(u16),
    ExecutionLimit,
    MemoryFault(u32),
}

impl fmt::Display for ThumbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegister(register) => write!(f, "invalid Thumb register r{register}"),
            Self::ImmediateOutOfRange(kind) => {
                write!(f, "{kind} immediate is outside its encoding")
            }
            Self::BranchOutOfRange(offset) => {
                write!(f, "Thumb branch offset {offset} is out of range")
            }
            Self::UnalignedBranch(offset) => {
                write!(f, "Thumb branch offset {offset} is not halfword aligned")
            }
            Self::InvalidRegisterList => f.write_str("invalid Thumb push/pop register list"),
            Self::InvalidRelocationOffset(offset) => {
                write!(f, "relocation offset {offset} is outside .text")
            }
            Self::MissingSymbol(symbol) => write!(f, "unresolved ARM symbol '{symbol}'"),
            Self::AddressOverflow => f.write_str("ARM address calculation overflow"),
            Self::InvalidInstruction(word) => {
                write!(f, "unsupported Thumb instruction {word:#06x}")
            }
            Self::ExecutionLimit => f.write_str("Thumb execution step limit reached"),
            Self::MemoryFault(address) => write!(f, "Thumb memory fault at {address:#010x}"),
        }
    }
}

impl Error for ThumbError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ThumbRegister {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    Sp = 13,
    Lr = 14,
    Pc = 15,
}

impl ThumbRegister {
    pub fn from_u8(value: u8) -> Result<Self, ThumbError> {
        const REGISTERS: [ThumbRegister; 16] = [
            ThumbRegister::R0,
            ThumbRegister::R1,
            ThumbRegister::R2,
            ThumbRegister::R3,
            ThumbRegister::R4,
            ThumbRegister::R5,
            ThumbRegister::R6,
            ThumbRegister::R7,
            ThumbRegister::R8,
            ThumbRegister::R9,
            ThumbRegister::R10,
            ThumbRegister::R11,
            ThumbRegister::R12,
            ThumbRegister::Sp,
            ThumbRegister::Lr,
            ThumbRegister::Pc,
        ];
        REGISTERS
            .get(value as usize)
            .copied()
            .ok_or(ThumbError::InvalidRegister(value))
    }

    const fn bits(self) -> u16 {
        self as u16
    }

    const fn low(self) -> bool {
        self as u8 <= 7
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThumbCondition {
    Eq = 0,
    Ne = 1,
    Cs = 2,
    Cc = 3,
    Mi = 4,
    Pl = 5,
    Vs = 6,
    Vc = 7,
    Hi = 8,
    Ls = 9,
    Ge = 10,
    Lt = 11,
    Gt = 12,
    Le = 13,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThumbSpecialRegister {
    Apsr = 0,
    Iapsr = 1,
    Eapsr = 2,
    Xpsr = 3,
    Ipsr = 5,
    Epsr = 6,
    Iepsr = 7,
    Msp = 8,
    Psp = 9,
    Primask = 16,
    Basepri = 17,
    BasepriMax = 18,
    Faultmask = 19,
    Control = 20,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbInstruction {
    Nop,
    Yield,
    MovImmediate {
        destination: ThumbRegister,
        immediate: u8,
    },
    MoveRegister {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    CompareImmediate {
        register: ThumbRegister,
        immediate: u8,
    },
    CompareRegister {
        left: ThumbRegister,
        right: ThumbRegister,
    },
    AddImmediate {
        register: ThumbRegister,
        immediate: u8,
    },
    SubImmediate {
        register: ThumbRegister,
        immediate: u8,
    },
    AndRegister {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    OrRegister {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    XorRegister {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    TestRegister {
        left: ThumbRegister,
        right: ThumbRegister,
    },
    MultiplyRegister {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    ShiftLeftImmediate {
        destination: ThumbRegister,
        source: ThumbRegister,
        immediate: u8,
    },
    ShiftRightImmediate {
        destination: ThumbRegister,
        source: ThumbRegister,
        immediate: u8,
    },
    ShiftArithmeticRightImmediate {
        destination: ThumbRegister,
        source: ThumbRegister,
        immediate: u8,
    },
    ReverseBits {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    CountLeadingZeros {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    SaturatingAdd {
        destination: ThumbRegister,
        left: ThumbRegister,
        right: ThumbRegister,
    },
    SaturatingSub {
        destination: ThumbRegister,
        left: ThumbRegister,
        right: ThumbRegister,
    },
    SignedSaturate {
        destination: ThumbRegister,
        bits: u8,
        source: ThumbRegister,
    },
    UnsignedSaturate {
        destination: ThumbRegister,
        bits: u8,
        source: ThumbRegister,
    },
    ReverseBytes {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    ReverseBytesInHalfwords {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    ReverseSignedHalfword {
        destination: ThumbRegister,
        source: ThumbRegister,
    },
    LoadWord {
        target: ThumbRegister,
        base: ThumbRegister,
        word_offset: u8,
    },
    StoreWord {
        source: ThumbRegister,
        base: ThumbRegister,
        word_offset: u8,
    },
    LoadByte {
        target: ThumbRegister,
        base: ThumbRegister,
        offset: u8,
    },
    StoreByte {
        source: ThumbRegister,
        base: ThumbRegister,
        offset: u8,
    },
    LoadHalf {
        target: ThumbRegister,
        base: ThumbRegister,
        half_offset: u8,
    },
    StoreHalf {
        source: ThumbRegister,
        base: ThumbRegister,
        half_offset: u8,
    },
    Push {
        low_register_mask: u8,
        link: bool,
    },
    Pop {
        low_register_mask: u8,
        pc: bool,
    },
    Branch {
        offset: i32,
    },
    BranchConditional {
        condition: ThumbCondition,
        offset: i16,
    },
    BranchLink {
        offset: i32,
    },
    BranchExchange(ThumbRegister),
    BranchLinkExchange(ThumbRegister),
    MoveWide {
        destination: ThumbRegister,
        immediate: u16,
    },
    MoveTop {
        destination: ThumbRegister,
        immediate: u16,
    },
    SupervisorCall(u8),
    HypervisorCall(u16),
    SecureMonitorCall(u8),
    Breakpoint(u8),
    Undefined(u8),
    DebugHint(u8),
    ReadSpecial {
        destination: ThumbRegister,
        special: ThumbSpecialRegister,
    },
    WriteSpecial {
        special: ThumbSpecialRegister,
        source: ThumbRegister,
    },
    LoadExclusive {
        target: ThumbRegister,
        base: ThumbRegister,
        word_offset: u8,
    },
    StoreExclusive {
        status: ThumbRegister,
        source: ThumbRegister,
        base: ThumbRegister,
        word_offset: u8,
    },
    ClearExclusive,
    DataMemoryBarrier,
    DataSyncBarrier,
    InstructionSyncBarrier,
    WaitForInterrupt,
    WaitForEvent,
    SendEvent,
    SetEndianBig,
    SetEndianLittle,
    ChangeProcessorState {
        enable: bool,
        affect_async_abort: bool,
        affect_irq: bool,
        affect_fiq: bool,
    },
    DisableInterrupts,
    EnableInterrupts,
}

fn push_halfword(output: &mut Vec<u8>, word: u16) {
    output.extend_from_slice(&word.to_le_bytes());
}

fn low_register(register: ThumbRegister) -> Result<u16, ThumbError> {
    register
        .low()
        .then_some(register.bits())
        .ok_or(ThumbError::InvalidRegister(register as u8))
}

fn encode_branch32(offset: i64, link: bool) -> Result<[u8; 4], ThumbError> {
    if offset & 1 != 0 {
        return Err(ThumbError::UnalignedBranch(offset));
    }
    if !(-(1 << 24)..(1 << 24)).contains(&offset) {
        return Err(ThumbError::BranchOutOfRange(offset));
    }
    let value = (offset as i32 as u32) & 0x01ff_fffe;
    let s = (value >> 24) & 1;
    let i1 = (value >> 23) & 1;
    let i2 = (value >> 22) & 1;
    let imm10 = (value >> 12) & 0x03ff;
    let imm11 = (value >> 1) & 0x07ff;
    let j1 = (!(i1 ^ s)) & 1;
    let j2 = (!(i2 ^ s)) & 1;
    let first = 0xf000 | (s << 10) | imm10;
    let second_base = if link { 0xd000 } else { 0x9000 };
    let second = second_base | (j1 << 13) | (j2 << 11) | imm11;
    let mut bytes = [0u8; 4];
    bytes[..2].copy_from_slice(&(first as u16).to_le_bytes());
    bytes[2..].copy_from_slice(&(second as u16).to_le_bytes());
    Ok(bytes)
}

fn encode_mov_wide(destination: ThumbRegister, immediate: u16, top: bool) -> [u8; 4] {
    let i = ((immediate >> 11) & 1) as u16;
    let imm4 = (immediate >> 12) & 0x0f;
    let imm3 = (immediate >> 8) & 0x07;
    let imm8 = immediate & 0xff;
    let first = (if top { 0xf2c0 } else { 0xf240 }) | (i << 10) | imm4;
    let second = (imm3 << 12) | (destination.bits() << 8) | imm8;
    let mut bytes = [0u8; 4];
    bytes[..2].copy_from_slice(&first.to_le_bytes());
    bytes[2..].copy_from_slice(&second.to_le_bytes());
    bytes
}

pub fn encode_thumb(instructions: &[ThumbInstruction]) -> Result<Vec<u8>, ThumbError> {
    let mut output = Vec::new();
    for instruction in instructions {
        match *instruction {
            ThumbInstruction::Nop => push_halfword(&mut output, 0xbf00),
            ThumbInstruction::Yield => push_halfword(&mut output, 0xbf10),
            ThumbInstruction::MovImmediate {
                destination,
                immediate,
            } => push_halfword(
                &mut output,
                0x2000 | (low_register(destination)? << 8) | immediate as u16,
            ),
            ThumbInstruction::MoveRegister {
                destination,
                source,
            } => {
                let destination = destination.bits();
                let source = source.bits();
                push_halfword(
                    &mut output,
                    0x4600 | ((destination & 8) << 4) | (source << 3) | (destination & 7),
                );
            }
            ThumbInstruction::CompareImmediate {
                register,
                immediate,
            } => push_halfword(
                &mut output,
                0x2800 | (low_register(register)? << 8) | immediate as u16,
            ),
            ThumbInstruction::CompareRegister { left, right } => {
                push_halfword(
                    &mut output,
                    0x4280 | (low_register(right)? << 3) | low_register(left)?,
                );
            }
            ThumbInstruction::AddImmediate {
                register,
                immediate,
            } => push_halfword(
                &mut output,
                0x3000 | (low_register(register)? << 8) | immediate as u16,
            ),
            ThumbInstruction::SubImmediate {
                register,
                immediate,
            } => push_halfword(
                &mut output,
                0x3800 | (low_register(register)? << 8) | immediate as u16,
            ),
            ThumbInstruction::AndRegister {
                destination,
                source,
            }
            | ThumbInstruction::OrRegister {
                destination,
                source,
            }
            | ThumbInstruction::XorRegister {
                destination,
                source,
            }
            | ThumbInstruction::TestRegister {
                left: destination,
                right: source,
            }
            | ThumbInstruction::MultiplyRegister {
                destination,
                source,
            } => {
                let base = match instruction {
                    ThumbInstruction::AndRegister { .. } => 0x4000,
                    ThumbInstruction::XorRegister { .. } => 0x4040,
                    ThumbInstruction::TestRegister { .. } => 0x4200,
                    ThumbInstruction::OrRegister { .. } => 0x4300,
                    ThumbInstruction::MultiplyRegister { .. } => 0x4340,
                    _ => unreachable!(),
                };
                push_halfword(
                    &mut output,
                    base | (low_register(source)? << 3) | low_register(destination)?,
                );
            }
            ThumbInstruction::ShiftLeftImmediate {
                destination,
                source,
                immediate,
            }
            | ThumbInstruction::ShiftRightImmediate {
                destination,
                source,
                immediate,
            }
            | ThumbInstruction::ShiftArithmeticRightImmediate {
                destination,
                source,
                immediate,
            } => {
                if immediate > 31
                    || (immediate == 0
                        && !matches!(instruction, ThumbInstruction::ShiftLeftImmediate { .. }))
                {
                    return Err(ThumbError::ImmediateOutOfRange("Thumb shift"));
                }
                let base = match instruction {
                    ThumbInstruction::ShiftLeftImmediate { .. } => 0x0000,
                    ThumbInstruction::ShiftRightImmediate { .. } => 0x0800,
                    ThumbInstruction::ShiftArithmeticRightImmediate { .. } => 0x1000,
                    _ => unreachable!(),
                };
                push_halfword(
                    &mut output,
                    base | ((immediate as u16) << 6)
                        | (low_register(source)? << 3)
                        | low_register(destination)?,
                );
            }
            ThumbInstruction::ReverseBits {
                destination,
                source,
            }
            | ThumbInstruction::CountLeadingZeros {
                destination,
                source,
            } => {
                let destination = destination.bits();
                let source = source.bits();
                if destination == ThumbRegister::Pc.bits() || source == ThumbRegister::Pc.bits() {
                    return Err(ThumbError::InvalidRegister(ThumbRegister::Pc as u8));
                }
                let (first_base, second_base) = match instruction {
                    ThumbInstruction::ReverseBits { .. } => (0xfa90, 0xf0a0),
                    ThumbInstruction::CountLeadingZeros { .. } => (0xfab0, 0xf080),
                    _ => unreachable!(),
                };
                push_halfword(&mut output, first_base | source);
                push_halfword(&mut output, second_base | (destination << 8) | source);
            }
            ThumbInstruction::SaturatingAdd {
                destination,
                left,
                right,
            }
            | ThumbInstruction::SaturatingSub {
                destination,
                left,
                right,
            } => {
                if matches!(destination, ThumbRegister::Pc)
                    || matches!(left, ThumbRegister::Pc)
                    || matches!(right, ThumbRegister::Pc)
                {
                    return Err(ThumbError::InvalidRegister(ThumbRegister::Pc as u8));
                }
                let second_base = match instruction {
                    ThumbInstruction::SaturatingAdd { .. } => 0xf080,
                    ThumbInstruction::SaturatingSub { .. } => 0xf0a0,
                    _ => unreachable!(),
                };
                push_halfword(&mut output, 0xfa80 | right.bits());
                push_halfword(
                    &mut output,
                    second_base | (destination.bits() << 8) | left.bits(),
                );
            }
            ThumbInstruction::SignedSaturate {
                destination,
                bits,
                source,
            } => {
                if !(1..=32).contains(&bits)
                    || matches!(destination, ThumbRegister::Pc)
                    || matches!(source, ThumbRegister::Pc)
                {
                    return Err(ThumbError::ImmediateOutOfRange("Thumb SSAT"));
                }
                push_halfword(&mut output, 0xf300 | source.bits());
                push_halfword(&mut output, (destination.bits() << 8) | u16::from(bits - 1));
            }
            ThumbInstruction::UnsignedSaturate {
                destination,
                bits,
                source,
            } => {
                if bits > 31
                    || matches!(destination, ThumbRegister::Pc)
                    || matches!(source, ThumbRegister::Pc)
                {
                    return Err(ThumbError::ImmediateOutOfRange("Thumb USAT"));
                }
                push_halfword(&mut output, 0xf380 | source.bits());
                push_halfword(&mut output, (destination.bits() << 8) | u16::from(bits));
            }
            ThumbInstruction::ReverseBytes {
                destination,
                source,
            }
            | ThumbInstruction::ReverseBytesInHalfwords {
                destination,
                source,
            }
            | ThumbInstruction::ReverseSignedHalfword {
                destination,
                source,
            } => {
                let base = match instruction {
                    ThumbInstruction::ReverseBytes { .. } => 0xba00,
                    ThumbInstruction::ReverseBytesInHalfwords { .. } => 0xba40,
                    ThumbInstruction::ReverseSignedHalfword { .. } => 0xbac0,
                    _ => unreachable!(),
                };
                push_halfword(
                    &mut output,
                    base | (low_register(source)? << 3) | low_register(destination)?,
                );
            }
            ThumbInstruction::LoadWord {
                target,
                base,
                word_offset,
            }
            | ThumbInstruction::StoreWord {
                source: target,
                base,
                word_offset,
            } => {
                if word_offset > 31 {
                    return Err(ThumbError::ImmediateOutOfRange("word load/store"));
                }
                let load = matches!(instruction, ThumbInstruction::LoadWord { .. });
                push_halfword(
                    &mut output,
                    (if load { 0x6800 } else { 0x6000 })
                        | ((word_offset as u16) << 6)
                        | (low_register(base)? << 3)
                        | low_register(target)?,
                );
            }
            ThumbInstruction::LoadByte {
                target,
                base,
                offset,
            }
            | ThumbInstruction::StoreByte {
                source: target,
                base,
                offset,
            } => {
                if offset > 31 {
                    return Err(ThumbError::ImmediateOutOfRange("byte load/store"));
                }
                let load = matches!(instruction, ThumbInstruction::LoadByte { .. });
                push_halfword(
                    &mut output,
                    (if load { 0x7800 } else { 0x7000 })
                        | ((offset as u16) << 6)
                        | (low_register(base)? << 3)
                        | low_register(target)?,
                );
            }
            ThumbInstruction::LoadHalf {
                target,
                base,
                half_offset,
            }
            | ThumbInstruction::StoreHalf {
                source: target,
                base,
                half_offset,
            } => {
                if half_offset > 31 {
                    return Err(ThumbError::ImmediateOutOfRange("halfword load/store"));
                }
                let load = matches!(instruction, ThumbInstruction::LoadHalf { .. });
                push_halfword(
                    &mut output,
                    (if load { 0x8800 } else { 0x8000 })
                        | ((half_offset as u16) << 6)
                        | (low_register(base)? << 3)
                        | low_register(target)?,
                );
            }
            ThumbInstruction::Push {
                low_register_mask,
                link,
            } => {
                if low_register_mask == 0 && !link {
                    return Err(ThumbError::InvalidRegisterList);
                }
                push_halfword(
                    &mut output,
                    0xb400 | low_register_mask as u16 | (u16::from(link) << 8),
                );
            }
            ThumbInstruction::Pop {
                low_register_mask,
                pc,
            } => {
                if low_register_mask == 0 && !pc {
                    return Err(ThumbError::InvalidRegisterList);
                }
                push_halfword(
                    &mut output,
                    0xbc00 | low_register_mask as u16 | (u16::from(pc) << 8),
                );
            }
            ThumbInstruction::Branch { offset } => {
                if offset & 1 != 0 {
                    return Err(ThumbError::UnalignedBranch(offset as i64));
                }
                if !(-2048..=2046).contains(&offset) {
                    output.extend_from_slice(&encode_branch32(offset as i64, false)?);
                } else {
                    push_halfword(&mut output, 0xe000 | (((offset >> 1) as u16) & 0x07ff));
                }
            }
            ThumbInstruction::BranchConditional { condition, offset } => {
                if offset & 1 != 0 {
                    return Err(ThumbError::UnalignedBranch(offset as i64));
                }
                if !(-256..=254).contains(&offset) {
                    return Err(ThumbError::BranchOutOfRange(offset as i64));
                }
                push_halfword(
                    &mut output,
                    0xd000 | ((condition as u16) << 8) | (((offset >> 1) as u16) & 0xff),
                );
            }
            ThumbInstruction::BranchLink { offset } => {
                output.extend_from_slice(&encode_branch32(offset as i64, true)?)
            }
            ThumbInstruction::BranchExchange(register) => {
                push_halfword(&mut output, 0x4700 | (register.bits() << 3));
            }
            ThumbInstruction::BranchLinkExchange(register) => {
                push_halfword(&mut output, 0x4780 | (register.bits() << 3));
            }
            ThumbInstruction::MoveWide {
                destination,
                immediate,
            } => {
                output.extend_from_slice(&encode_mov_wide(destination, immediate, false));
            }
            ThumbInstruction::MoveTop {
                destination,
                immediate,
            } => {
                output.extend_from_slice(&encode_mov_wide(destination, immediate, true));
            }
            ThumbInstruction::SupervisorCall(immediate) => {
                push_halfword(&mut output, 0xdf00 | immediate as u16)
            }
            ThumbInstruction::HypervisorCall(immediate) => {
                push_halfword(&mut output, 0xf7e0 | (immediate >> 12));
                push_halfword(&mut output, 0x8000 | (immediate & 0x0fff));
            }
            ThumbInstruction::SecureMonitorCall(immediate) => {
                if immediate > 15 {
                    return Err(ThumbError::ImmediateOutOfRange("Thumb SMC"));
                }
                push_halfword(&mut output, 0xf7f0 | immediate as u16);
                push_halfword(&mut output, 0x8000);
            }
            ThumbInstruction::Breakpoint(immediate) => {
                push_halfword(&mut output, 0xbe00 | immediate as u16)
            }
            ThumbInstruction::Undefined(immediate) => {
                push_halfword(&mut output, 0xde00 | immediate as u16)
            }
            ThumbInstruction::DebugHint(immediate) => {
                if immediate > 15 {
                    return Err(ThumbError::ImmediateOutOfRange("Thumb DBG"));
                }
                push_halfword(&mut output, 0xf3af);
                push_halfword(&mut output, 0x80f0 | immediate as u16);
            }
            ThumbInstruction::ReadSpecial {
                destination,
                special,
            } => {
                output.extend_from_slice(&[0xef, 0xf3]);
                push_halfword(
                    &mut output,
                    0x8000 | (destination.bits() << 8) | special as u16,
                );
            }
            ThumbInstruction::WriteSpecial { special, source } => {
                push_halfword(&mut output, 0xf380 | source.bits());
                push_halfword(&mut output, 0x8800 | special as u16);
            }
            ThumbInstruction::LoadExclusive {
                target,
                base,
                word_offset,
            } => {
                push_halfword(&mut output, 0xe850 | base.bits());
                push_halfword(
                    &mut output,
                    (target.bits() << 12) | 0x0f00 | word_offset as u16,
                );
            }
            ThumbInstruction::StoreExclusive {
                status,
                source,
                base,
                word_offset,
            } => {
                push_halfword(&mut output, 0xe840 | base.bits());
                push_halfword(
                    &mut output,
                    (source.bits() << 12) | (status.bits() << 8) | word_offset as u16,
                );
            }
            ThumbInstruction::ClearExclusive => {
                output.extend_from_slice(&[0xbf, 0xf3, 0x2f, 0x8f]);
            }
            ThumbInstruction::DataMemoryBarrier => {
                output.extend_from_slice(&[0xbf, 0xf3, 0x5f, 0x8f])
            }
            ThumbInstruction::DataSyncBarrier => {
                output.extend_from_slice(&[0xbf, 0xf3, 0x4f, 0x8f])
            }
            ThumbInstruction::InstructionSyncBarrier => {
                output.extend_from_slice(&[0xbf, 0xf3, 0x6f, 0x8f])
            }
            ThumbInstruction::WaitForInterrupt => push_halfword(&mut output, 0xbf30),
            ThumbInstruction::WaitForEvent => push_halfword(&mut output, 0xbf20),
            ThumbInstruction::SendEvent => push_halfword(&mut output, 0xbf40),
            ThumbInstruction::SetEndianBig => push_halfword(&mut output, 0xb658),
            ThumbInstruction::SetEndianLittle => push_halfword(&mut output, 0xb650),
            ThumbInstruction::ChangeProcessorState {
                enable,
                affect_async_abort,
                affect_irq,
                affect_fiq,
            } => {
                let mask = ((affect_async_abort as u16) << 2)
                    | ((affect_irq as u16) << 1)
                    | affect_fiq as u16;
                if mask == 0 {
                    return Err(ThumbError::ImmediateOutOfRange("cps effect mask"));
                }
                push_halfword(&mut output, (if enable { 0xb660 } else { 0xb670 }) | mask);
            }
            ThumbInstruction::DisableInterrupts => push_halfword(&mut output, 0xb672),
            ThumbInstruction::EnableInterrupts => push_halfword(&mut output, 0xb662),
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmRelocationKind {
    Abs32,
    Rel32,
    ThmCall,
    ThmJump24,
    MovwAbsNc,
    MovtAbs,
    Prel31,
}

impl ArmRelocationKind {
    pub const fn elf_type(self) -> u8 {
        match self {
            Self::Abs32 => 2,
            Self::Rel32 => 3,
            Self::ThmCall => 10,
            Self::Prel31 => 42,
            Self::MovwAbsNc => 47,
            Self::MovtAbs => 48,
            Self::ThmJump24 => 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmRelocation {
    pub offset: usize,
    pub kind: ArmRelocationKind,
    pub symbol: String,
    pub addend: i64,
}

fn read_halfword(bytes: &[u8], offset: usize) -> Result<u16, ThumbError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(ThumbError::InvalidRelocationOffset(offset))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

pub fn apply_arm_relocations(
    text: &mut [u8],
    text_address: u32,
    relocations: &[ArmRelocation],
    symbols: &BTreeMap<String, u32>,
) -> Result<(), ThumbError> {
    for relocation in relocations {
        let symbol = *symbols
            .get(&relocation.symbol)
            .ok_or_else(|| ThumbError::MissingSymbol(relocation.symbol.clone()))?
            as i64;
        let place = (text_address as usize)
            .checked_add(relocation.offset)
            .ok_or(ThumbError::AddressOverflow)? as i64;
        match relocation.kind {
            ArmRelocationKind::Abs32 | ArmRelocationKind::Rel32 | ArmRelocationKind::Prel31 => {
                let end = relocation
                    .offset
                    .checked_add(4)
                    .filter(|end| *end <= text.len())
                    .ok_or(ThumbError::InvalidRelocationOffset(relocation.offset))?;
                let value = match relocation.kind {
                    ArmRelocationKind::Abs32 => symbol + relocation.addend,
                    ArmRelocationKind::Rel32 => symbol + relocation.addend - place,
                    ArmRelocationKind::Prel31 => (symbol + relocation.addend - place) & 0x7fff_ffff,
                    _ => unreachable!(),
                };
                text[relocation.offset..end].copy_from_slice(&(value as u32).to_le_bytes());
            }
            ArmRelocationKind::ThmCall | ArmRelocationKind::ThmJump24 => {
                if relocation.offset + 4 > text.len() {
                    return Err(ThumbError::InvalidRelocationOffset(relocation.offset));
                }
                // ELF Thumb function symbols may carry the architectural state bit.
                let branch_target = symbol & !1;
                let displacement = branch_target + relocation.addend - (place + 4);
                let bytes =
                    encode_branch32(displacement, relocation.kind == ArmRelocationKind::ThmCall)?;
                text[relocation.offset..relocation.offset + 4].copy_from_slice(&bytes);
            }
            ArmRelocationKind::MovwAbsNc | ArmRelocationKind::MovtAbs => {
                let second = read_halfword(text, relocation.offset + 2)?;
                let register = ThumbRegister::from_u8(((second >> 8) & 0xf) as u8)?;
                let absolute = symbol
                    .checked_add(relocation.addend)
                    .ok_or(ThumbError::AddressOverflow)? as u32;
                let immediate = if relocation.kind == ArmRelocationKind::MovwAbsNc {
                    absolute as u16
                } else {
                    (absolute >> 16) as u16
                };
                let bytes = encode_mov_wide(
                    register,
                    immediate,
                    relocation.kind == ArmRelocationKind::MovtAbs,
                );
                if relocation.offset + 4 > text.len() {
                    return Err(ThumbError::InvalidRelocationOffset(relocation.offset));
                }
                text[relocation.offset..relocation.offset + 4].copy_from_slice(&bytes);
            }
        }
    }
    Ok(())
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn append_elf32_symbol(
    output: &mut Vec<u8>,
    name: u32,
    value: u32,
    size: u32,
    info: u8,
    section: u16,
) {
    output.extend_from_slice(&name.to_le_bytes());
    output.extend_from_slice(&value.to_le_bytes());
    output.extend_from_slice(&size.to_le_bytes());
    output.push(info);
    output.push(0);
    output.extend_from_slice(&section.to_le_bytes());
}

#[allow(clippy::too_many_arguments)]
fn write_section_header(
    bytes: &mut [u8],
    offset: usize,
    name: u32,
    kind: u32,
    flags: u32,
    file_offset: u32,
    size: u32,
    link: u32,
    info: u32,
    alignment: u32,
    entry_size: u32,
) {
    write_u32(bytes, offset, name);
    write_u32(bytes, offset + 4, kind);
    write_u32(bytes, offset + 8, flags);
    write_u32(bytes, offset + 16, file_offset);
    write_u32(bytes, offset + 20, size);
    write_u32(bytes, offset + 24, link);
    write_u32(bytes, offset + 28, info);
    write_u32(bytes, offset + 32, alignment);
    write_u32(bytes, offset + 36, entry_size);
}

pub fn emit_arm_elf32_object(
    entry_symbol: &str,
    text: &[u8],
    relocations: &[ArmRelocation],
) -> Result<Vec<u8>, ThumbError> {
    let mut encoded_text = text.to_vec();
    let mut undefined = BTreeSet::new();
    for relocation in relocations {
        if relocation.offset + 4 > text.len() {
            return Err(ThumbError::InvalidRelocationOffset(relocation.offset));
        }
        match relocation.kind {
            ArmRelocationKind::Abs32 | ArmRelocationKind::Rel32 => {
                encoded_text[relocation.offset..relocation.offset + 4]
                    .copy_from_slice(&(relocation.addend as u32).to_le_bytes());
            }
            ArmRelocationKind::Prel31 => {
                encoded_text[relocation.offset..relocation.offset + 4]
                    .copy_from_slice(&((relocation.addend as u32) & 0x7fff_ffff).to_le_bytes());
            }
            ArmRelocationKind::ThmCall | ArmRelocationKind::ThmJump24 => {
                // R_ARM_THM_CALL/JUMP24 are REL relocations. The encoded
                // immediate carries A-P's architectural PC bias; an API
                // addend of zero is therefore represented as -4 in-place.
                let bytes = encode_branch32(
                    relocation.addend - 4,
                    relocation.kind == ArmRelocationKind::ThmCall,
                )?;
                encoded_text[relocation.offset..relocation.offset + 4].copy_from_slice(&bytes);
            }
            ArmRelocationKind::MovwAbsNc | ArmRelocationKind::MovtAbs => {
                let second = read_halfword(&encoded_text, relocation.offset + 2)?;
                let register = ThumbRegister::from_u8(((second >> 8) & 0xf) as u8)?;
                let addend = relocation.addend as u32;
                let immediate = if relocation.kind == ArmRelocationKind::MovwAbsNc {
                    addend as u16
                } else {
                    (addend >> 16) as u16
                };
                let bytes = encode_mov_wide(
                    register,
                    immediate,
                    relocation.kind == ArmRelocationKind::MovtAbs,
                );
                encoded_text[relocation.offset..relocation.offset + 4].copy_from_slice(&bytes);
            }
        }
        undefined.insert(relocation.symbol.clone());
    }
    let mut strtab = vec![0u8];
    let entry_name = strtab.len() as u32;
    strtab.extend_from_slice(entry_symbol.as_bytes());
    strtab.push(0);
    let mut symbol_names = BTreeMap::new();
    for symbol in &undefined {
        let offset = strtab.len() as u32;
        strtab.extend_from_slice(symbol.as_bytes());
        strtab.push(0);
        symbol_names.insert(symbol.clone(), offset);
    }
    let mut symtab = vec![0u8; 16];
    append_elf32_symbol(&mut symtab, 0, 0, 0, 3, 1);
    append_elf32_symbol(&mut symtab, entry_name, 1, text.len() as u32, 0x12, 1);
    let mut symbol_indices = BTreeMap::new();
    for (index, symbol) in undefined.iter().enumerate() {
        let symbol_index = (3 + index) as u32;
        symbol_indices.insert(symbol.clone(), symbol_index);
        append_elf32_symbol(&mut symtab, symbol_names[symbol], 0, 0, 0x10, 0);
    }
    let mut rel_text = Vec::with_capacity(relocations.len() * 8);
    for relocation in relocations {
        rel_text.extend_from_slice(&(relocation.offset as u32).to_le_bytes());
        let info = (symbol_indices[&relocation.symbol] << 8) | relocation.kind.elf_type() as u32;
        rel_text.extend_from_slice(&info.to_le_bytes());
    }
    let shstrtab = b"\0.text\0.rel.text\0.symtab\0.strtab\0.shstrtab\0";
    const HEADER_SIZE: usize = 52;
    let text_offset = align_up(HEADER_SIZE, 4);
    let rel_offset = align_up(text_offset + encoded_text.len(), 4);
    let symtab_offset = align_up(rel_offset + rel_text.len(), 4);
    let strtab_offset = symtab_offset + symtab.len();
    let shstrtab_offset = strtab_offset + strtab.len();
    let section_offset = align_up(shstrtab_offset + shstrtab.len(), 4);
    let mut output = vec![0u8; section_offset + 6 * 40];
    output[..4].copy_from_slice(b"\x7fELF");
    output[4] = 1;
    output[5] = 1;
    output[6] = 1;
    output[7] = 0;
    write_u16(&mut output, 16, 1);
    write_u16(&mut output, 18, EM_ARM);
    write_u32(&mut output, 20, 1);
    write_u32(&mut output, 32, section_offset as u32);
    write_u32(&mut output, 36, EF_ARM_EABI_VER5);
    write_u16(&mut output, 40, HEADER_SIZE as u16);
    write_u16(&mut output, 46, 40);
    write_u16(&mut output, 48, 6);
    write_u16(&mut output, 50, 5);
    output[text_offset..text_offset + encoded_text.len()].copy_from_slice(&encoded_text);
    output[rel_offset..rel_offset + rel_text.len()].copy_from_slice(&rel_text);
    output[symtab_offset..symtab_offset + symtab.len()].copy_from_slice(&symtab);
    output[strtab_offset..strtab_offset + strtab.len()].copy_from_slice(&strtab);
    output[shstrtab_offset..shstrtab_offset + shstrtab.len()].copy_from_slice(shstrtab);
    write_section_header(
        &mut output,
        section_offset + 40,
        1,
        1,
        6,
        text_offset as u32,
        encoded_text.len() as u32,
        0,
        0,
        2,
        0,
    );
    write_section_header(
        &mut output,
        section_offset + 80,
        7,
        9,
        0,
        rel_offset as u32,
        rel_text.len() as u32,
        3,
        1,
        4,
        8,
    );
    write_section_header(
        &mut output,
        section_offset + 120,
        17,
        2,
        0,
        symtab_offset as u32,
        symtab.len() as u32,
        4,
        2,
        4,
        16,
    );
    write_section_header(
        &mut output,
        section_offset + 160,
        25,
        3,
        0,
        strtab_offset as u32,
        strtab.len() as u32,
        0,
        0,
        1,
        0,
    );
    write_section_header(
        &mut output,
        section_offset + 200,
        33,
        3,
        0,
        shstrtab_offset as u32,
        shstrtab.len() as u32,
        0,
        0,
        1,
        0,
    );
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CortexMFirmware {
    pub flash_base: u32,
    pub initial_stack: u32,
    pub reset_vector: u32,
    pub bytes: Vec<u8>,
}

impl CortexMFirmware {
    pub fn intel_hex(&self) -> String {
        fn record(address: u16, kind: u8, data: &[u8]) -> String {
            let mut sum = data.len() as u8;
            sum = sum
                .wrapping_add((address >> 8) as u8)
                .wrapping_add(address as u8)
                .wrapping_add(kind);
            let mut line = format!(":{:02X}{address:04X}{kind:02X}", data.len());
            for byte in data {
                sum = sum.wrapping_add(*byte);
                line.push_str(&format!("{byte:02X}"));
            }
            line.push_str(&format!("{:02X}\n", (!sum).wrapping_add(1)));
            line
        }

        let mut output = String::new();
        let mut upper = u16::MAX;
        for (index, chunk) in self.bytes.chunks(16).enumerate() {
            let address = self.flash_base.wrapping_add((index * 16) as u32);
            let next_upper = (address >> 16) as u16;
            if next_upper != upper {
                output.push_str(&record(0, 4, &next_upper.to_be_bytes()));
                upper = next_upper;
            }
            output.push_str(&record(address as u16, 0, chunk));
        }
        output.push_str(":00000001FF\n");
        output
    }
}

pub fn link_cortex_m_firmware(
    flash_base: u32,
    initial_stack: u32,
    text: &[u8],
    relocations: &[ArmRelocation],
    symbols: &BTreeMap<String, u32>,
) -> Result<CortexMFirmware, ThumbError> {
    const VECTOR_BYTES: usize = 16 * 4;
    if flash_base & 0x7f != 0 || initial_stack & 7 != 0 {
        return Err(ThumbError::ImmediateOutOfRange("Cortex-M vector alignment"));
    }
    let text_address = flash_base
        .checked_add(VECTOR_BYTES as u32)
        .ok_or(ThumbError::AddressOverflow)?;
    let mut linked = text.to_vec();
    apply_arm_relocations(&mut linked, text_address, relocations, symbols)?;
    let mut bytes = vec![0u8; VECTOR_BYTES + linked.len()];
    bytes[..4].copy_from_slice(&initial_stack.to_le_bytes());
    let reset_vector = text_address | 1;
    bytes[4..8].copy_from_slice(&reset_vector.to_le_bytes());
    bytes[VECTOR_BYTES..].copy_from_slice(&linked);
    Ok(CortexMFirmware {
        flash_base,
        initial_stack,
        reset_vector,
        bytes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbExecution {
    pub registers: [u32; 16],
    pub negative: bool,
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
    pub steps: u64,
    pub supervisor_calls: Vec<u8>,
    pub breakpoints: Vec<u8>,
}

fn sign_extend(value: u32, bits: u32) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

fn condition_holds(condition: u8, state: &ThumbExecution) -> bool {
    match condition {
        0 => state.zero,
        1 => !state.zero,
        2 => state.carry,
        3 => !state.carry,
        4 => state.negative,
        5 => !state.negative,
        6 => state.overflow,
        7 => !state.overflow,
        8 => state.carry && !state.zero,
        9 => !state.carry || state.zero,
        10 => state.negative == state.overflow,
        11 => state.negative != state.overflow,
        12 => !state.zero && state.negative == state.overflow,
        13 => state.zero || state.negative != state.overflow,
        _ => false,
    }
}

pub fn execute_thumb(text: &[u8], max_steps: u64) -> Result<ThumbExecution, ThumbError> {
    let mut state = ThumbExecution {
        registers: [0; 16],
        negative: false,
        zero: false,
        carry: false,
        overflow: false,
        steps: 0,
        supervisor_calls: Vec::new(),
        breakpoints: Vec::new(),
    };
    state.registers[14] = u32::MAX;
    while state.steps < max_steps {
        let pc = state.registers[15] as usize;
        let word = read_halfword(text, pc).map_err(|_| ThumbError::MemoryFault(pc as u32))?;
        state.registers[15] = state.registers[15].wrapping_add(2);
        state.steps += 1;
        match word {
            0xbf00 | 0xbf10 | 0xbf20 | 0xbf30 | 0xbf40 | 0xb662 | 0xb672 => {}
            value if value & 0xf800 == 0x2000 => {
                let register = ((value >> 8) & 7) as usize;
                let result = (value & 0xff) as u32;
                state.registers[register] = result;
                state.zero = result == 0;
                state.negative = result & 0x8000_0000 != 0;
            }
            value if value & 0xf800 == 0x2800 => {
                let register = ((value >> 8) & 7) as usize;
                let right = (value & 0xff) as u32;
                let left = state.registers[register];
                let result = left.wrapping_sub(right);
                state.zero = result == 0;
                state.negative = result & 0x8000_0000 != 0;
                state.carry = left >= right;
                state.overflow = ((left ^ right) & (left ^ result) & 0x8000_0000) != 0;
            }
            value if value & 0xf800 == 0x3000 => {
                let register = ((value >> 8) & 7) as usize;
                let right = (value & 0xff) as u32;
                let left = state.registers[register];
                let (result, carry) = left.overflowing_add(right);
                state.registers[register] = result;
                state.zero = result == 0;
                state.negative = result & 0x8000_0000 != 0;
                state.carry = carry;
                state.overflow = (!(left ^ right) & (left ^ result) & 0x8000_0000) != 0;
            }
            value if value & 0xf800 == 0x3800 => {
                let register = ((value >> 8) & 7) as usize;
                let right = (value & 0xff) as u32;
                let left = state.registers[register];
                let result = left.wrapping_sub(right);
                state.registers[register] = result;
                state.zero = result == 0;
                state.negative = result & 0x8000_0000 != 0;
                state.carry = left >= right;
                state.overflow = ((left ^ right) & (left ^ result) & 0x8000_0000) != 0;
            }
            value if value & 0xff87 == 0x4700 => {
                let register = ((value >> 3) & 0xf) as usize;
                let target = state.registers[register];
                if target == u32::MAX {
                    return Ok(state);
                }
                if target & 1 == 0 {
                    return Err(ThumbError::MemoryFault(target));
                }
                state.registers[15] = target & !1;
            }
            value if value & 0xf800 == 0xe000 => {
                let offset = sign_extend(((value & 0x07ff) as u32) << 1, 12);
                state.registers[15] = state.registers[15].wrapping_add(offset as u32);
            }
            value if value & 0xf000 == 0xd000 && (value >> 8) & 0xf < 14 => {
                if condition_holds(((value >> 8) & 0xf) as u8, &state) {
                    let offset = sign_extend(((value & 0xff) as u32) << 1, 9);
                    state.registers[15] = state.registers[15].wrapping_add(offset as u32);
                }
            }
            value if value & 0xff00 == 0xdf00 => state.supervisor_calls.push(value as u8),
            value if value & 0xff00 == 0xbe00 => state.breakpoints.push(value as u8),
            value if value & 0xff00 == 0xde00 => return Err(ThumbError::InvalidInstruction(value)),
            value => return Err(ThumbError::InvalidInstruction(value)),
        }
    }
    Err(ThumbError::ExecutionLimit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumb2_bit_reverse_count_and_byte_reverse_match_llvm_bytes() {
        let bytes = encode_thumb(&[
            ThumbInstruction::ReverseBits {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::CountLeadingZeros {
                destination: ThumbRegister::R2,
                source: ThumbRegister::R3,
            },
            ThumbInstruction::ReverseBytes {
                destination: ThumbRegister::R4,
                source: ThumbRegister::R5,
            },
            ThumbInstruction::ReverseBytesInHalfwords {
                destination: ThumbRegister::R6,
                source: ThumbRegister::R7,
            },
            ThumbInstruction::ReverseSignedHalfword {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
        ])
        .expect("Thumb-2 bit manipulation");
        assert_eq!(
            bytes,
            vec![
                0x91, 0xfa, 0xa1, 0xf0, 0xb3, 0xfa, 0x83, 0xf2, 0x2c, 0xba, 0x7e, 0xba, 0xc8, 0xba,
            ]
        );
    }

    #[test]
    fn thumb16_thumb2_barriers_and_control_encodings_are_exact() {
        let bytes = encode_thumb(&[
            ThumbInstruction::Nop,
            ThumbInstruction::MovImmediate {
                destination: ThumbRegister::R0,
                immediate: 42,
            },
            ThumbInstruction::AddImmediate {
                register: ThumbRegister::R0,
                immediate: 1,
            },
            ThumbInstruction::SubImmediate {
                register: ThumbRegister::R0,
                immediate: 1,
            },
            ThumbInstruction::BranchExchange(ThumbRegister::Lr),
            ThumbInstruction::DataMemoryBarrier,
            ThumbInstruction::DataSyncBarrier,
            ThumbInstruction::InstructionSyncBarrier,
            ThumbInstruction::WaitForInterrupt,
            ThumbInstruction::DisableInterrupts,
        ])
        .unwrap();
        assert_eq!(
            bytes,
            [
                0x00, 0xbf, 0x2a, 0x20, 0x01, 0x30, 0x01, 0x38, 0x70, 0x47, 0xbf, 0xf3, 0x5f, 0x8f,
                0xbf, 0xf3, 0x4f, 0x8f, 0xbf, 0xf3, 0x6f, 0x8f, 0x30, 0xbf, 0x72, 0xb6,
            ]
        );
    }

    #[test]
    fn thumb2_hypervisor_and_secure_monitor_calls_match_clang() {
        let bytes = encode_thumb(&[
            ThumbInstruction::ClearExclusive,
            ThumbInstruction::HypervisorCall(0x1234),
            ThumbInstruction::SecureMonitorCall(7),
            ThumbInstruction::BranchExchange(ThumbRegister::Lr),
        ])
        .unwrap();
        assert_eq!(
            bytes,
            [0xbf, 0xf3, 0x2f, 0x8f, 0xe1, 0xf7, 0x34, 0x82, 0xf7, 0xf7, 0x00, 0x80, 0x70, 0x47,]
        );
        assert!(matches!(
            encode_thumb(&[ThumbInstruction::SecureMonitorCall(16)]),
            Err(ThumbError::ImmediateOutOfRange("Thumb SMC"))
        ));
    }

    #[test]
    fn thumb2_debug_hint_matches_clang() {
        assert_eq!(
            encode_thumb(&[ThumbInstruction::DebugHint(3)]).expect("Thumb DBG must encode"),
            [0xaf, 0xf3, 0xf3, 0x80]
        );
        assert_eq!(
            encode_thumb(&[ThumbInstruction::DebugHint(16)]),
            Err(ThumbError::ImmediateOutOfRange("Thumb DBG"))
        );
    }

    #[test]
    fn thumb_cps_masks_match_clang() {
        let bytes = encode_thumb(&[
            ThumbInstruction::ChangeProcessorState {
                enable: false,
                affect_async_abort: false,
                affect_irq: true,
                affect_fiq: true,
            },
            ThumbInstruction::ChangeProcessorState {
                enable: false,
                affect_async_abort: true,
                affect_irq: true,
                affect_fiq: true,
            },
            ThumbInstruction::ChangeProcessorState {
                enable: true,
                affect_async_abort: false,
                affect_irq: true,
                affect_fiq: true,
            },
        ])
        .expect("CPS masks must encode");
        assert_eq!(bytes, [0x73, 0xb6, 0x77, 0xb6, 0x63, 0xb6]);
        assert_eq!(
            encode_thumb(&[ThumbInstruction::ChangeProcessorState {
                enable: false,
                affect_async_abort: false,
                affect_irq: false,
                affect_fiq: false,
            }]),
            Err(ThumbError::ImmediateOutOfRange("cps effect mask"))
        );
    }

    #[test]
    fn kernel_register_atomic_and_special_register_encodings_match_clang() {
        let bytes = encode_thumb(&[
            ThumbInstruction::ReadSpecial {
                destination: ThumbRegister::R0,
                special: ThumbSpecialRegister::Primask,
            },
            ThumbInstruction::WriteSpecial {
                special: ThumbSpecialRegister::Primask,
                source: ThumbRegister::R0,
            },
            ThumbInstruction::ReadSpecial {
                destination: ThumbRegister::R1,
                special: ThumbSpecialRegister::Control,
            },
            ThumbInstruction::WriteSpecial {
                special: ThumbSpecialRegister::Control,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::LoadExclusive {
                target: ThumbRegister::R0,
                base: ThumbRegister::R1,
                word_offset: 0,
            },
            ThumbInstruction::StoreExclusive {
                status: ThumbRegister::R2,
                source: ThumbRegister::R0,
                base: ThumbRegister::R1,
                word_offset: 0,
            },
            ThumbInstruction::ClearExclusive,
            ThumbInstruction::AndRegister {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::OrRegister {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::XorRegister {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::MultiplyRegister {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R1,
            },
            ThumbInstruction::ShiftLeftImmediate {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R0,
                immediate: 3,
            },
            ThumbInstruction::ShiftRightImmediate {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R0,
                immediate: 4,
            },
            ThumbInstruction::ShiftArithmeticRightImmediate {
                destination: ThumbRegister::R0,
                source: ThumbRegister::R0,
                immediate: 5,
            },
            ThumbInstruction::BranchLinkExchange(ThumbRegister::R3),
        ])
        .unwrap();
        assert_eq!(
            bytes,
            [
                0xef, 0xf3, 0x10, 0x80, 0x80, 0xf3, 0x10, 0x88, 0xef, 0xf3, 0x14, 0x81, 0x81, 0xf3,
                0x14, 0x88, 0x51, 0xe8, 0x00, 0x0f, 0x41, 0xe8, 0x00, 0x02, 0xbf, 0xf3, 0x2f, 0x8f,
                0x08, 0x40, 0x08, 0x43, 0x48, 0x40, 0x48, 0x43, 0xc0, 0x00, 0x00, 0x09, 0x40, 0x11,
                0x98, 0x47,
            ]
        );
    }

    #[test]
    fn movw_movt_and_branch_relocations_materialize_full_addresses() {
        let mut text = encode_thumb(&[
            ThumbInstruction::MoveWide {
                destination: ThumbRegister::R4,
                immediate: 0,
            },
            ThumbInstruction::MoveTop {
                destination: ThumbRegister::R4,
                immediate: 0,
            },
            ThumbInstruction::BranchLink { offset: 0 },
        ])
        .unwrap();
        let relocations = vec![
            ArmRelocation {
                offset: 0,
                kind: ArmRelocationKind::MovwAbsNc,
                symbol: "data".into(),
                addend: 0,
            },
            ArmRelocation {
                offset: 4,
                kind: ArmRelocationKind::MovtAbs,
                symbol: "data".into(),
                addend: 0,
            },
            ArmRelocation {
                offset: 8,
                kind: ArmRelocationKind::ThmCall,
                symbol: "callee".into(),
                addend: 0,
            },
        ];
        let symbols =
            BTreeMap::from([("data".into(), 0x1234_5678), ("callee".into(), 0x0800_0201)]);
        apply_arm_relocations(&mut text, 0x0800_0100, &relocations, &symbols).unwrap();
        assert_eq!(
            &text[..4],
            &encode_mov_wide(ThumbRegister::R4, 0x5678, false)
        );
        assert_eq!(
            &text[4..8],
            &encode_mov_wide(ThumbRegister::R4, 0x1234, true)
        );
        assert_ne!(&text[8..12], &[0x00, 0xf0, 0x00, 0xf8]);
    }

    #[test]
    fn elf32_arm_object_has_eabi_symbols_and_rel_section() {
        let text = encode_thumb(&[
            ThumbInstruction::BranchLink { offset: 0 },
            ThumbInstruction::BranchExchange(ThumbRegister::Lr),
        ])
        .unwrap();
        let object = emit_arm_elf32_object(
            "reset",
            &text,
            &[ArmRelocation {
                offset: 0,
                kind: ArmRelocationKind::ThmCall,
                symbol: "clock_init".into(),
                addend: 0,
            }],
        )
        .unwrap();
        assert!(object.starts_with(b"\x7fELF\x01\x01\x01"));
        assert_eq!(
            u16::from_le_bytes(object[18..20].try_into().unwrap()),
            EM_ARM
        );
        assert_eq!(
            u32::from_le_bytes(object[36..40].try_into().unwrap()),
            EF_ARM_EABI_VER5
        );
        assert_eq!(
            &object[52..56],
            &encode_branch32(-4, true).unwrap(),
            "ELF32 REL must retain the architectural Thumb PC addend in .text"
        );
        assert!(object.windows(9).any(|window| window == b".rel.text"));
        assert!(object.windows(10).any(|window| window == b"clock_init"));
    }

    #[test]
    fn cortex_m_vector_binary_and_intel_hex_are_bootable_artifacts() {
        let text = encode_thumb(&[
            ThumbInstruction::MovImmediate {
                destination: ThumbRegister::R0,
                immediate: 42,
            },
            ThumbInstruction::BranchExchange(ThumbRegister::Lr),
        ])
        .unwrap();
        let firmware =
            link_cortex_m_firmware(0x0800_0000, 0x2002_0000, &text, &[], &BTreeMap::new()).unwrap();
        assert_eq!(firmware.reset_vector, 0x0800_0041);
        assert_eq!(
            u32::from_le_bytes(firmware.bytes[..4].try_into().unwrap()),
            0x2002_0000
        );
        let hex = firmware.intel_hex();
        assert!(hex.starts_with(":020000040800F2"));
        assert!(hex.ends_with(":00000001FF\n"));
    }

    #[test]
    fn deterministic_thumb_executor_runs_real_encoded_bytes_outside_qemu() {
        let text = encode_thumb(&[
            ThumbInstruction::MovImmediate {
                destination: ThumbRegister::R0,
                immediate: 40,
            },
            ThumbInstruction::AddImmediate {
                register: ThumbRegister::R0,
                immediate: 2,
            },
            ThumbInstruction::SupervisorCall(7),
            ThumbInstruction::BranchExchange(ThumbRegister::Lr),
        ])
        .unwrap();
        let execution = execute_thumb(&text, 32).unwrap();
        assert_eq!(execution.registers[0], 42);
        assert_eq!(execution.supervisor_calls, vec![7]);
        assert_eq!(execution.steps, 4);
    }
}
