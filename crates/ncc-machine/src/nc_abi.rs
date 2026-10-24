use ncc_ast::{
    compute_binary_layout, BinaryBitFieldContract, BinaryEndian, BinaryLayoutContract,
    BinaryLayoutFieldInput, BinaryRepr, TypeContractKind,
};

pub const NC_RAW_ABI_VERSION: &str = "nC.raw.abi.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NcAbiTarget {
    X86_64,
    I386,
    AArch64,
    ArmV7,
    RiscV32,
    RiscV64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcAbiDataModel {
    Ilp32,
    Lp64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcAbiEndianness {
    Little,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcAbiVariant {
    SysV64,
    Win64,
    I386Cdecl,
    Aapcs64,
    Aapcs32HardFloat,
    RiscV32,
    RiscV64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcAbiValueKind {
    Integer,
    Float,
    Pointer,
    FunctionPointer,
    Aggregate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NcAbiValue {
    pub kind: NcAbiValueKind,
    pub size_bytes: usize,
    pub align_bytes: usize,
}

impl NcAbiValue {
    pub const fn scalar(kind: NcAbiValueKind, size_bytes: usize, align_bytes: usize) -> Self {
        Self {
            kind,
            size_bytes,
            align_bytes,
        }
    }

    pub const fn aggregate(size_bytes: usize, align_bytes: usize) -> Self {
        Self::scalar(NcAbiValueKind::Aggregate, size_bytes, align_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NcAbiLocation {
    Registers(Vec<&'static str>),
    Stack {
        offset_bytes: usize,
        size_bytes: usize,
        align_bytes: usize,
    },
    Indirect {
        pointer: Box<NcAbiLocation>,
        pointee_size_bytes: usize,
        pointee_align_bytes: usize,
    },
    HiddenSRet {
        pointer: Box<NcAbiLocation>,
        echo_register: Option<&'static str>,
    },
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcAbiArgumentPlan {
    pub index: usize,
    pub variadic: bool,
    pub value: NcAbiValue,
    pub location: NcAbiLocation,
    pub mirror_location: Option<NcAbiLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcAbiCallPlan {
    pub abi_version: &'static str,
    pub target: NcAbiTarget,
    pub variant: NcAbiVariant,
    pub arguments: Vec<NcAbiArgumentPlan>,
    pub return_location: NcAbiLocation,
    pub stack_argument_bytes: usize,
    pub stack_align_bytes: usize,
    pub shadow_space_bytes: usize,
    pub caller_materializes_sret: bool,
    pub caller_materializes_indirect_arguments: bool,
    pub varargs_float_register_count_register: Option<&'static str>,
    pub varargs_float_register_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcAbiContract {
    pub version: &'static str,
    pub target: NcAbiTarget,
    pub variant: NcAbiVariant,
    pub canonical_triple: &'static str,
    pub data_model: NcAbiDataModel,
    pub endianness: NcAbiEndianness,
    pub pointer_bits: u16,
    pub function_pointer_bits: u16,
    pub stack_align_bytes: usize,
    pub shadow_space_bytes: usize,
    pub integer_argument_registers: &'static [&'static str],
    pub float_argument_registers: &'static [&'static str],
    pub integer_return_registers: &'static [&'static str],
    pub float_return_registers: &'static [&'static str],
    pub sret_register: Option<&'static str>,
    pub callee_saved_registers: &'static [&'static str],
    pub tls_is_explicit: bool,
    pub per_cpu_is_explicit: bool,
    pub tls_base_register: &'static str,
    pub per_cpu_base_register: &'static str,
    pub aggregate_direct_limit_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcManualAbiContract {
    pub name: String,
    pub target: NcAbiTarget,
    pub canonical_triple: String,
    pub pointer_bits: u16,
    pub stack_align_bytes: usize,
    pub shadow_space_bytes: usize,
    pub red_zone_bytes: usize,
    pub integer_argument_registers: Vec<String>,
    pub float_argument_registers: Vec<String>,
    pub integer_return_registers: Vec<String>,
    pub float_return_registers: Vec<String>,
    pub callee_saved_registers: Vec<String>,
    pub clobbered_registers: Vec<String>,
    pub sret_register: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NcManualAbiLocation {
    Registers(Vec<String>),
    Stack {
        offset_bytes: usize,
        size_bytes: usize,
        align_bytes: usize,
    },
    HiddenSRet {
        register: Option<String>,
        stack_offset_bytes: Option<usize>,
    },
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcManualAbiArgumentPlan {
    pub index: usize,
    pub value: NcAbiValue,
    pub location: NcManualAbiLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcManualAbiCallPlan {
    pub abi_name: String,
    pub arguments: Vec<NcManualAbiArgumentPlan>,
    pub return_location: NcManualAbiLocation,
    pub stack_argument_bytes: usize,
    pub stack_align_bytes: usize,
    pub shadow_space_bytes: usize,
    pub red_zone_bytes: usize,
}

impl TryFrom<&ncc_ast::ManualAbiDecl> for NcManualAbiContract {
    type Error = String;

    fn try_from(source: &ncc_ast::ManualAbiDecl) -> Result<Self, Self::Error> {
        let base = nc_abi_contract_for_triple(&source.target)?;
        if source.pointer_bits != base.pointer_bits {
            return Err(format!(
                "E_NC_MANUAL_ABI_POINTER_001: '{}' declares {}-bit pointers for {}",
                source.name, source.pointer_bits, source.target
            ));
        }
        if source.stack_align_bytes == 0 || !source.stack_align_bytes.is_power_of_two() {
            return Err(format!(
                "E_NC_MANUAL_ABI_ALIGN_001: '{}' stack alignment must be a non-zero power of two",
                source.name
            ));
        }
        let contract = Self {
            name: source.name.clone(),
            target: base.target,
            canonical_triple: source.target.clone(),
            pointer_bits: source.pointer_bits,
            stack_align_bytes: source.stack_align_bytes,
            shadow_space_bytes: source.shadow_space_bytes,
            red_zone_bytes: source.red_zone_bytes,
            integer_argument_registers: source.integer_argument_registers.clone(),
            float_argument_registers: source.float_argument_registers.clone(),
            integer_return_registers: source.integer_return_registers.clone(),
            float_return_registers: source.float_return_registers.clone(),
            callee_saved_registers: source.callee_saved_registers.clone(),
            clobbered_registers: source.clobbered_registers.clone(),
            sret_register: source.sret_register.clone(),
        };
        contract.validate_registers()?;
        Ok(contract)
    }
}

impl NcManualAbiContract {
    fn register_valid_for_target(&self, register: &str) -> bool {
        let register = register.trim().to_ascii_lowercase();
        match self.target {
            NcAbiTarget::X86_64 => {
                matches!(
                    register.as_str(),
                    "rax"
                        | "rbx"
                        | "rcx"
                        | "rdx"
                        | "rsi"
                        | "rdi"
                        | "rbp"
                        | "rsp"
                        | "rip"
                        | "r8"
                        | "r9"
                        | "r10"
                        | "r11"
                        | "r12"
                        | "r13"
                        | "r14"
                        | "r15"
                ) || register
                    .strip_prefix("xmm")
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix("ymm")
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix("zmm")
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('k')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 8)
            }
            NcAbiTarget::I386 => {
                matches!(
                    register.as_str(),
                    "eax" | "ebx" | "ecx" | "edx" | "esi" | "edi" | "ebp" | "esp" | "eip"
                ) || register
                    .strip_prefix("xmm")
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| n < 8)
                    || register
                        .strip_prefix("st")
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 8)
                    || register
                        .strip_prefix("mm")
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 8)
            }
            NcAbiTarget::AArch64 => {
                matches!(register.as_str(), "sp" | "pc")
                    || register
                        .strip_prefix('x')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n <= 30)
                    || register
                        .strip_prefix('w')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n <= 30)
                    || register
                        .strip_prefix('v')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('z')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('p')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 16)
            }
            NcAbiTarget::ArmV7 => {
                matches!(register.as_str(), "sp" | "lr" | "pc" | "ip" | "fp")
                    || register
                        .strip_prefix('r')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n <= 15)
                    || register
                        .strip_prefix('s')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('d')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('q')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 16)
            }
            NcAbiTarget::RiscV32 | NcAbiTarget::RiscV64 => {
                matches!(
                    register.as_str(),
                    "zero"
                        | "ra"
                        | "sp"
                        | "gp"
                        | "tp"
                        | "t0"
                        | "t1"
                        | "t2"
                        | "t3"
                        | "t4"
                        | "t5"
                        | "t6"
                        | "s0"
                        | "s1"
                        | "s2"
                        | "s3"
                        | "s4"
                        | "s5"
                        | "s6"
                        | "s7"
                        | "s8"
                        | "s9"
                        | "s10"
                        | "s11"
                        | "a0"
                        | "a1"
                        | "a2"
                        | "a3"
                        | "a4"
                        | "a5"
                        | "a6"
                        | "a7"
                        | "ft0"
                        | "ft1"
                        | "ft2"
                        | "ft3"
                        | "ft4"
                        | "ft5"
                        | "ft6"
                        | "ft7"
                        | "ft8"
                        | "ft9"
                        | "ft10"
                        | "ft11"
                        | "fs0"
                        | "fs1"
                        | "fs2"
                        | "fs3"
                        | "fs4"
                        | "fs5"
                        | "fs6"
                        | "fs7"
                        | "fs8"
                        | "fs9"
                        | "fs10"
                        | "fs11"
                        | "fa0"
                        | "fa1"
                        | "fa2"
                        | "fa3"
                        | "fa4"
                        | "fa5"
                        | "fa6"
                        | "fa7"
                ) || register
                    .strip_prefix('x')
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| n < 32)
                    || register
                        .strip_prefix('f')
                        .and_then(|n| n.parse::<u8>().ok())
                        .is_some_and(|n| n < 32)
            }
        }
    }

    pub fn validate_registers(&self) -> Result<(), String> {
        for register in self
            .integer_argument_registers
            .iter()
            .chain(&self.float_argument_registers)
            .chain(&self.integer_return_registers)
            .chain(&self.float_return_registers)
            .chain(&self.callee_saved_registers)
            .chain(&self.clobbered_registers)
            .chain(self.sret_register.iter())
        {
            if !self.register_valid_for_target(register) {
                return Err(format!(
                    "E_NC_MANUAL_ABI_REGISTER_001: register '{}' is not valid for ABI '{}' target '{}'",
                    register, self.name, self.canonical_triple
                ));
            }
        }
        Ok(())
    }

    pub fn plan_call(
        &self,
        arguments: &[NcAbiValue],
        return_value: Option<NcAbiValue>,
    ) -> Result<NcManualAbiCallPlan, String> {
        self.validate_registers()?;
        let word = usize::from(self.pointer_bits).div_ceil(8).max(1);
        let mut integer_index = 0usize;
        let mut float_index = 0usize;
        let mut stack_bytes = self.shadow_space_bytes;
        let mut planned = Vec::with_capacity(arguments.len());
        for (index, value) in arguments.iter().copied().enumerate() {
            let bank = if value.kind == NcAbiValueKind::Float {
                &self.float_argument_registers
            } else {
                &self.integer_argument_registers
            };
            let bank_index = if value.kind == NcAbiValueKind::Float {
                &mut float_index
            } else {
                &mut integer_index
            };
            let words = value.size_bytes.max(1).div_ceil(word).max(1);
            let location = if *bank_index + words <= bank.len() {
                let registers = bank[*bank_index..*bank_index + words].to_vec();
                *bank_index += words;
                NcManualAbiLocation::Registers(registers)
            } else {
                let alignment = value.align_bytes.max(1).min(self.stack_align_bytes);
                stack_bytes = align_up(stack_bytes, alignment);
                let offset = stack_bytes;
                let size = align_up(value.size_bytes.max(word), word);
                stack_bytes += size;
                NcManualAbiLocation::Stack {
                    offset_bytes: offset,
                    size_bytes: size,
                    align_bytes: alignment,
                }
            };
            planned.push(NcManualAbiArgumentPlan {
                index,
                value,
                location,
            });
        }

        let return_location = match return_value {
            None => NcManualAbiLocation::Void,
            Some(value)
                if value.kind == NcAbiValueKind::Aggregate && value.size_bytes > 2 * word =>
            {
                if self.sret_register.is_none() {
                    stack_bytes = align_up(stack_bytes, word);
                    let offset = stack_bytes;
                    stack_bytes += word;
                    NcManualAbiLocation::HiddenSRet {
                        register: None,
                        stack_offset_bytes: Some(offset),
                    }
                } else {
                    NcManualAbiLocation::HiddenSRet {
                        register: self.sret_register.clone(),
                        stack_offset_bytes: None,
                    }
                }
            }
            Some(value) => {
                let bank = if value.kind == NcAbiValueKind::Float {
                    &self.float_return_registers
                } else {
                    &self.integer_return_registers
                };
                let words = value.size_bytes.max(1).div_ceil(word).max(1);
                if words > bank.len() {
                    return Err(format!(
                        "E_NC_MANUAL_ABI_RETURN_001: ABI '{}' has insufficient return registers",
                        self.name
                    ));
                }
                NcManualAbiLocation::Registers(bank[..words].to_vec())
            }
        };

        Ok(NcManualAbiCallPlan {
            abi_name: self.name.clone(),
            arguments: planned,
            return_location,
            stack_argument_bytes: align_up(stack_bytes, self.stack_align_bytes),
            stack_align_bytes: self.stack_align_bytes,
            shadow_space_bytes: self.shadow_space_bytes,
            red_zone_bytes: self.red_zone_bytes,
        })
    }
}

const X86_64_INT_ARGS: &[&str] = &["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
const X86_64_FLOAT_ARGS: &[&str] = &[
    "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
];
const AARCH64_INT_ARGS: &[&str] = &["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"];
const AARCH64_FLOAT_ARGS: &[&str] = &["v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7"];
const ARMV7_INT_ARGS: &[&str] = &["r0", "r1", "r2", "r3"];
const ARMV7_FLOAT_ARGS: &[&str] = &["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7"];
const RISCV_INT_ARGS: &[&str] = &["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"];
const RISCV_FLOAT_ARGS: &[&str] = &["fa0", "fa1", "fa2", "fa3", "fa4", "fa5", "fa6", "fa7"];

pub fn nc_abi_contract(target: NcAbiTarget) -> NcAbiContract {
    match target {
        NcAbiTarget::X86_64 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::SysV64,
            canonical_triple: "x86_64-unknown-linux-gnu",
            data_model: NcAbiDataModel::Lp64,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 64,
            function_pointer_bits: 64,
            stack_align_bytes: 16,
            shadow_space_bytes: 0,
            integer_argument_registers: X86_64_INT_ARGS,
            float_argument_registers: X86_64_FLOAT_ARGS,
            integer_return_registers: &["rax", "rdx"],
            float_return_registers: &["xmm0", "xmm1"],
            sret_register: Some("rdi"),
            callee_saved_registers: &["rbx", "rbp", "r12", "r13", "r14", "r15"],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "fs.base",
            per_cpu_base_register: "gs.base",
            aggregate_direct_limit_bytes: 16,
        },
        NcAbiTarget::I386 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::I386Cdecl,
            canonical_triple: "i386-unknown-linux-gnu",
            data_model: NcAbiDataModel::Ilp32,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 32,
            function_pointer_bits: 32,
            stack_align_bytes: 4,
            shadow_space_bytes: 0,
            integer_argument_registers: &[],
            float_argument_registers: &[],
            integer_return_registers: &["eax", "edx"],
            float_return_registers: &["st0"],
            sret_register: None,
            callee_saved_registers: &["ebx", "esi", "edi", "ebp"],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "gs",
            per_cpu_base_register: "fs",
            aggregate_direct_limit_bytes: 8,
        },
        NcAbiTarget::AArch64 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::Aapcs64,
            canonical_triple: "aarch64-unknown-linux-gnu",
            data_model: NcAbiDataModel::Lp64,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 64,
            function_pointer_bits: 64,
            stack_align_bytes: 16,
            shadow_space_bytes: 0,
            integer_argument_registers: AARCH64_INT_ARGS,
            float_argument_registers: AARCH64_FLOAT_ARGS,
            integer_return_registers: &["x0", "x1"],
            float_return_registers: &["v0", "v1"],
            sret_register: Some("x8"),
            callee_saved_registers: &[
                "x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26", "x27", "x28", "x29",
            ],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "tpidr_el0",
            per_cpu_base_register: "tpidr_el1",
            aggregate_direct_limit_bytes: 16,
        },
        NcAbiTarget::ArmV7 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::Aapcs32HardFloat,
            canonical_triple: "armv7-unknown-linux-gnueabihf",
            data_model: NcAbiDataModel::Ilp32,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 32,
            function_pointer_bits: 32,
            // nC keeps the AAPCS32 public minimum while requiring a stronger
            // 16-byte call boundary for SIMD values and cross-target frames.
            stack_align_bytes: 16,
            shadow_space_bytes: 0,
            integer_argument_registers: ARMV7_INT_ARGS,
            float_argument_registers: ARMV7_FLOAT_ARGS,
            integer_return_registers: &["r0", "r1"],
            float_return_registers: &["s0", "s1"],
            sret_register: Some("r0"),
            callee_saved_registers: &["r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11"],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "tpidrurw",
            per_cpu_base_register: "tpidrprw",
            aggregate_direct_limit_bytes: 8,
        },
        NcAbiTarget::RiscV32 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::RiscV32,
            canonical_triple: "riscv32imac-unknown-none-elf",
            data_model: NcAbiDataModel::Ilp32,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 32,
            function_pointer_bits: 32,
            stack_align_bytes: 16,
            shadow_space_bytes: 0,
            integer_argument_registers: RISCV_INT_ARGS,
            float_argument_registers: RISCV_FLOAT_ARGS,
            integer_return_registers: &["a0", "a1"],
            float_return_registers: &["fa0", "fa1"],
            sret_register: Some("a0"),
            callee_saved_registers: &[
                "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
            ],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "tp",
            per_cpu_base_register: "sscratch",
            aggregate_direct_limit_bytes: 8,
        },
        NcAbiTarget::RiscV64 => NcAbiContract {
            version: NC_RAW_ABI_VERSION,
            target,
            variant: NcAbiVariant::RiscV64,
            canonical_triple: "riscv64gc-unknown-linux-gnu",
            data_model: NcAbiDataModel::Lp64,
            endianness: NcAbiEndianness::Little,
            pointer_bits: 64,
            function_pointer_bits: 64,
            stack_align_bytes: 16,
            shadow_space_bytes: 0,
            integer_argument_registers: RISCV_INT_ARGS,
            float_argument_registers: RISCV_FLOAT_ARGS,
            integer_return_registers: &["a0", "a1"],
            float_return_registers: &["fa0", "fa1"],
            sret_register: Some("a0"),
            callee_saved_registers: &[
                "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
            ],
            tls_is_explicit: true,
            per_cpu_is_explicit: true,
            tls_base_register: "tp",
            per_cpu_base_register: "sscratch",
            aggregate_direct_limit_bytes: 16,
        },
    }
}

pub fn nc_abi_contract_for_triple(triple: &str) -> Result<NcAbiContract, String> {
    let lower = triple.trim().to_ascii_lowercase();
    let target = if lower.starts_with("x86_64") || lower.starts_with("amd64") {
        NcAbiTarget::X86_64
    } else if lower.starts_with("i386")
        || lower.starts_with("i486")
        || lower.starts_with("i586")
        || lower.starts_with("i686")
    {
        NcAbiTarget::I386
    } else if lower.starts_with("aarch64") || lower.starts_with("arm64") {
        NcAbiTarget::AArch64
    } else if lower.starts_with("armv7") || lower.starts_with("thumbv7") {
        NcAbiTarget::ArmV7
    } else if lower.starts_with("riscv32") {
        NcAbiTarget::RiscV32
    } else if lower.starts_with("riscv64") {
        NcAbiTarget::RiscV64
    } else {
        return Err(format!(
            "E_NC_ABI_TARGET_001: target '{triple}' has no {} contract",
            NC_RAW_ABI_VERSION
        ));
    };
    let mut contract = nc_abi_contract(target);
    if target == NcAbiTarget::X86_64 && lower.contains("windows") {
        contract.variant = NcAbiVariant::Win64;
        contract.canonical_triple = "x86_64-pc-windows-msvc";
        contract.integer_argument_registers = &["rcx", "rdx", "r8", "r9"];
        contract.float_argument_registers = &["xmm0", "xmm1", "xmm2", "xmm3"];
        contract.integer_return_registers = &["rax"];
        contract.float_return_registers = &["xmm0"];
        contract.sret_register = Some("rcx");
        contract.shadow_space_bytes = 32;
        contract.callee_saved_registers = &[
            "rbx", "rbp", "rdi", "rsi", "r12", "r13", "r14", "r15", "xmm6", "xmm7", "xmm8", "xmm9",
            "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
        ];
        contract.aggregate_direct_limit_bytes = 8;
    } else if target == NcAbiTarget::I386 && lower.contains("windows") {
        contract.canonical_triple = "i686-pc-windows-msvc";
        contract.tls_base_register = "fs";
        contract.per_cpu_base_register = "gs";
    }
    Ok(contract)
}

fn align_up(value: usize, align: usize) -> usize {
    value.div_ceil(align.max(1)) * align.max(1)
}

impl NcAbiContract {
    pub fn pointer_bytes(&self) -> usize {
        usize::from(self.pointer_bits) / 8
    }

    pub fn function_pointer(&self) -> NcAbiValue {
        let bytes = usize::from(self.function_pointer_bits) / 8;
        NcAbiValue::scalar(NcAbiValueKind::FunctionPointer, bytes, bytes)
    }

    pub fn plan_call(
        &self,
        fixed_argument_count: usize,
        arguments: &[NcAbiValue],
        return_value: Option<NcAbiValue>,
    ) -> Result<NcAbiCallPlan, String> {
        if fixed_argument_count > arguments.len() {
            return Err("E_NC_ABI_VARARGS_001: fixed argument count exceeds argument count".into());
        }

        let needs_sret = return_value.is_some_and(|value| {
            value.kind == NcAbiValueKind::Aggregate
                && value.size_bytes > self.aggregate_direct_limit_bytes
        });
        let sret_consumes_first_integer_register = needs_sret
            && self.sret_register.is_some_and(|sret| {
                self.integer_argument_registers
                    .first()
                    .is_some_and(|first| *first == sret)
            });
        let mut int_index = usize::from(sret_consumes_first_integer_register);
        let mut float_index = 0usize;
        let mut stack_bytes = if needs_sret && self.sret_register.is_none() {
            self.shadow_space_bytes + self.pointer_bytes()
        } else {
            self.shadow_space_bytes
        };
        let mut win64_slot_index = usize::from(sret_consumes_first_integer_register);
        let mut planned = Vec::with_capacity(arguments.len());
        let mut caller_materializes_indirect_arguments = false;

        for (index, value) in arguments.iter().copied().enumerate() {
            let variadic = index >= fixed_argument_count;
            let indirect = value.kind == NcAbiValueKind::Aggregate
                && value.size_bytes > self.aggregate_direct_limit_bytes;
            let carrier = if indirect {
                caller_materializes_indirect_arguments = true;
                NcAbiValue::scalar(
                    NcAbiValueKind::Pointer,
                    self.pointer_bytes(),
                    self.pointer_bytes(),
                )
            } else {
                value
            };
            let words = carrier
                .size_bytes
                .max(1)
                .div_ceil(self.pointer_bytes())
                .max(1);
            let (base_location, mirror_location) = if self.variant == NcAbiVariant::Win64 {
                let slot = win64_slot_index;
                win64_slot_index += words;
                if words == 1 && slot < self.integer_argument_registers.len() {
                    let is_float = carrier.kind == NcAbiValueKind::Float;
                    let primary = if is_float && !variadic {
                        NcAbiLocation::Registers(vec![self.float_argument_registers[slot]])
                    } else {
                        NcAbiLocation::Registers(vec![self.integer_argument_registers[slot]])
                    };
                    let mirror = (is_float && variadic).then(|| {
                        NcAbiLocation::Registers(vec![self.float_argument_registers[slot]])
                    });
                    (primary, mirror)
                } else {
                    let align = carrier.align_bytes.max(1).min(self.stack_align_bytes);
                    stack_bytes = align_up(stack_bytes, align);
                    let offset = stack_bytes;
                    let size = align_up(
                        carrier.size_bytes.max(self.pointer_bytes()),
                        self.pointer_bytes(),
                    );
                    stack_bytes += size;
                    (
                        NcAbiLocation::Stack {
                            offset_bytes: offset,
                            size_bytes: size,
                            align_bytes: align,
                        },
                        None,
                    )
                }
            } else {
                let float_bank = carrier.kind == NcAbiValueKind::Float
                    && (!variadic || self.variant == NcAbiVariant::SysV64);
                let bank = if float_bank {
                    self.float_argument_registers
                } else {
                    self.integer_argument_registers
                };
                let bank_index = if float_bank {
                    &mut float_index
                } else {
                    &mut int_index
                };
                if !bank.is_empty() && *bank_index + words <= bank.len() {
                    let regs = bank[*bank_index..*bank_index + words].to_vec();
                    *bank_index += words;
                    (NcAbiLocation::Registers(regs), None)
                } else {
                    let align = carrier.align_bytes.max(1).min(self.stack_align_bytes);
                    stack_bytes = align_up(stack_bytes, align);
                    let offset = stack_bytes;
                    let size = align_up(
                        carrier.size_bytes.max(self.pointer_bytes()),
                        self.pointer_bytes(),
                    );
                    stack_bytes += size;
                    (
                        NcAbiLocation::Stack {
                            offset_bytes: offset,
                            size_bytes: size,
                            align_bytes: align,
                        },
                        None,
                    )
                }
            };
            let location = if indirect {
                NcAbiLocation::Indirect {
                    pointer: Box::new(base_location),
                    pointee_size_bytes: value.size_bytes,
                    pointee_align_bytes: value.align_bytes,
                }
            } else {
                base_location
            };
            planned.push(NcAbiArgumentPlan {
                index,
                variadic,
                value,
                location,
                mirror_location,
            });
        }

        let return_location = match return_value {
            None => NcAbiLocation::Void,
            Some(value)
                if value.kind == NcAbiValueKind::Aggregate
                    && value.size_bytes > self.aggregate_direct_limit_bytes =>
            {
                let pointer = if let Some(reg) = self.sret_register {
                    NcAbiLocation::Registers(vec![reg])
                } else {
                    NcAbiLocation::Stack {
                        offset_bytes: 0,
                        size_bytes: self.pointer_bytes(),
                        align_bytes: self.pointer_bytes(),
                    }
                };
                NcAbiLocation::HiddenSRet {
                    pointer: Box::new(pointer),
                    echo_register: (self.target == NcAbiTarget::X86_64).then_some("rax"),
                }
            }
            Some(value) => {
                let regs = if value.kind == NcAbiValueKind::Float {
                    self.float_return_registers
                } else {
                    self.integer_return_registers
                };
                let words = value
                    .size_bytes
                    .max(1)
                    .div_ceil(self.pointer_bytes())
                    .max(1);
                if words > regs.len() {
                    return Err(format!(
                        "E_NC_ABI_RETURN_001: {}-byte return exceeds direct register contract for {:?}",
                        value.size_bytes, self.target
                    ));
                }
                NcAbiLocation::Registers(regs[..words].to_vec())
            }
        };
        let caller_materializes_sret = matches!(return_location, NcAbiLocation::HiddenSRet { .. });

        Ok(NcAbiCallPlan {
            abi_version: self.version,
            target: self.target,
            variant: self.variant,
            arguments: planned,
            return_location,
            stack_argument_bytes: align_up(stack_bytes, self.stack_align_bytes),
            stack_align_bytes: self.stack_align_bytes,
            shadow_space_bytes: self.shadow_space_bytes,
            caller_materializes_sret,
            caller_materializes_indirect_arguments,
            varargs_float_register_count_register: (self.variant == NcAbiVariant::SysV64
                && fixed_argument_count < arguments.len())
            .then_some("al"),
            varargs_float_register_count: if self.variant == NcAbiVariant::SysV64 {
                float_index
            } else {
                0
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcAbiCertification {
    pub target: NcAbiTarget,
    pub abi_version: &'static str,
    pub checks: Vec<&'static str>,
}

pub fn certify_nc_abi_layout(target: NcAbiTarget) -> Result<NcAbiCertification, String> {
    let abi = nc_abi_contract(target);
    if !abi.tls_is_explicit
        || !abi.per_cpu_is_explicit
        || abi.tls_base_register.is_empty()
        || abi.per_cpu_base_register.is_empty()
    {
        return Err(format!(
            "E_NC_ABI_STORAGE_BASE_001: TLS/per-CPU base is not explicit for {target:?}"
        ));
    }
    let pointer = abi.pointer_bytes();
    let scalar = BinaryLayoutFieldInput {
        name: "tag".into(),
        size_bytes: 1,
        align_bytes: 1,
        integer_bits: Some(8),
    };
    let pointer_field = BinaryLayoutFieldInput {
        name: "next".into(),
        size_bytes: pointer,
        align_bytes: pointer,
        integer_bits: None,
    };
    let tail = BinaryLayoutFieldInput {
        name: "tail".into(),
        size_bytes: 1,
        align_bytes: 1,
        integer_bits: Some(8),
    };

    let repr_c = BinaryLayoutContract {
        explicit: true,
        repr: BinaryRepr::C,
        ..BinaryLayoutContract::default()
    };
    let c_layout = compute_binary_layout(
        TypeContractKind::Struct,
        &repr_c,
        &[scalar.clone(), pointer_field.clone()],
    )?;
    if c_layout.fields[1].offset_bytes != pointer || c_layout.align_bytes != pointer {
        return Err(format!(
            "E_NC_ABI_LAYOUT_001: invalid repr(C) layout for {target:?}"
        ));
    }

    let packed = BinaryLayoutContract {
        explicit: true,
        repr: BinaryRepr::C,
        packed: true,
        ..BinaryLayoutContract::default()
    };
    let packed_layout = compute_binary_layout(
        TypeContractKind::Struct,
        &packed,
        &[scalar.clone(), pointer_field.clone()],
    )?;
    if packed_layout.fields[1].offset_bytes != 1 || packed_layout.align_bytes != 1 {
        return Err(format!(
            "E_NC_ABI_LAYOUT_002: invalid packed layout for {target:?}"
        ));
    }

    let explicit_align = BinaryLayoutContract {
        explicit: true,
        repr: BinaryRepr::C,
        align_bytes: Some(pointer * 2),
        endian: BinaryEndian::Little,
        ..BinaryLayoutContract::default()
    };
    let aligned_layout = compute_binary_layout(
        TypeContractKind::Struct,
        &explicit_align,
        &[scalar.clone(), pointer_field.clone()],
    )?;
    if aligned_layout.align_bytes != pointer * 2
        || aligned_layout.size_bytes % (pointer * 2) != 0
        || aligned_layout.endian != BinaryEndian::Little
    {
        return Err(format!(
            "E_NC_ABI_LAYOUT_006: invalid align/endian layout for {target:?}"
        ));
    }

    let union_layout = compute_binary_layout(
        TypeContractKind::Union,
        &repr_c,
        &[scalar.clone(), pointer_field.clone()],
    )?;
    if union_layout
        .fields
        .iter()
        .any(|field| field.offset_bytes != 0)
        || union_layout.size_bytes != pointer
    {
        return Err(format!(
            "E_NC_ABI_LAYOUT_003: invalid union layout for {target:?}"
        ));
    }

    let bitfields = BinaryLayoutContract {
        explicit: true,
        repr: BinaryRepr::C,
        bitfields: vec![BinaryBitFieldContract {
            field: "tag".into(),
            storage: "tag".into(),
            bit_offset: 1,
            bit_width: 3,
        }],
        ..BinaryLayoutContract::default()
    };
    let bitfield_layout = compute_binary_layout(
        TypeContractKind::Struct,
        &bitfields,
        std::slice::from_ref(&scalar),
    )?;
    if bitfield_layout.fields[0].bit_offset != Some(1)
        || bitfield_layout.fields[0].bit_width != Some(3)
    {
        return Err(format!(
            "E_NC_ABI_LAYOUT_004: invalid bitfield layout for {target:?}"
        ));
    }

    let flexible = BinaryLayoutContract {
        explicit: true,
        repr: BinaryRepr::C,
        flexible_field: Some("tail".into()),
        ..BinaryLayoutContract::default()
    };
    let flexible_layout =
        compute_binary_layout(TypeContractKind::Struct, &flexible, &[pointer_field, tail])?;
    if !flexible_layout.fields[1].flexible || flexible_layout.fields[1].size_bytes != 0 {
        return Err(format!(
            "E_NC_ABI_LAYOUT_005: invalid flexible layout for {target:?}"
        ));
    }

    let call = abi.plan_call(
        1,
        &[
            abi.function_pointer(),
            NcAbiValue::scalar(NcAbiValueKind::Float, 8, 8),
            NcAbiValue::aggregate(abi.aggregate_direct_limit_bytes + 1, pointer),
        ],
        Some(NcAbiValue::aggregate(
            abi.aggregate_direct_limit_bytes + 1,
            pointer,
        )),
    )?;
    if !call.arguments[1].variadic
        || !call.caller_materializes_indirect_arguments
        || !call.caller_materializes_sret
    {
        return Err(format!(
            "E_NC_ABI_CALL_001: invalid varargs/sret plan for {target:?}"
        ));
    }

    Ok(NcAbiCertification {
        target,
        abi_version: NC_RAW_ABI_VERSION,
        checks: vec![
            "repr_c",
            "packed",
            "align_endian",
            "union",
            "bitfield",
            "flexible_array",
            "function_pointer",
            "varargs",
            "aggregate_sret",
            "tls_explicit",
            "per_cpu_explicit",
        ],
    })
}

pub fn certify_all_nc_abi_layouts() -> Result<Vec<NcAbiCertification>, String> {
    [
        NcAbiTarget::X86_64,
        NcAbiTarget::I386,
        NcAbiTarget::AArch64,
        NcAbiTarget::ArmV7,
        NcAbiTarget::RiscV32,
        NcAbiTarget::RiscV64,
    ]
    .into_iter()
    .map(certify_nc_abi_layout)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certifies_layout_and_call_contract_for_all_six_targets() {
        let reports = certify_all_nc_abi_layouts().expect("ABI certification must pass");
        assert_eq!(reports.len(), 6);
        assert!(reports.iter().all(|report| {
            report.abi_version == NC_RAW_ABI_VERSION && report.checks.len() == 11
        }));
    }

    #[test]
    fn large_aggregate_is_always_caller_materialized() {
        for target in [
            NcAbiTarget::X86_64,
            NcAbiTarget::I386,
            NcAbiTarget::AArch64,
            NcAbiTarget::ArmV7,
            NcAbiTarget::RiscV32,
            NcAbiTarget::RiscV64,
        ] {
            let abi = nc_abi_contract(target);
            let large = NcAbiValue::aggregate(abi.aggregate_direct_limit_bytes + 8, 8);
            let plan = abi.plan_call(1, &[large], Some(large)).unwrap();
            assert!(matches!(
                plan.arguments[0].location,
                NcAbiLocation::Indirect { .. }
            ));
            assert!(matches!(
                plan.return_location,
                NcAbiLocation::HiddenSRet { .. }
            ));
        }
    }

    #[test]
    fn sret_never_aliases_the_first_user_argument() {
        for target in [
            NcAbiTarget::X86_64,
            NcAbiTarget::I386,
            NcAbiTarget::AArch64,
            NcAbiTarget::ArmV7,
            NcAbiTarget::RiscV32,
            NcAbiTarget::RiscV64,
        ] {
            let abi = nc_abi_contract(target);
            let word = abi.pointer_bytes();
            let plan = abi
                .plan_call(
                    1,
                    &[NcAbiValue::scalar(NcAbiValueKind::Integer, word, word)],
                    Some(NcAbiValue::aggregate(
                        abi.aggregate_direct_limit_bytes + 1,
                        word,
                    )),
                )
                .unwrap();
            let NcAbiLocation::HiddenSRet { pointer, .. } = &plan.return_location else {
                panic!("{target:?} must use sret");
            };
            assert_ne!(pointer.as_ref(), &plan.arguments[0].location, "{target:?}");
        }
    }

    #[test]
    fn x86_64_varargs_reports_the_material_xmm_count() {
        let abi = nc_abi_contract(NcAbiTarget::X86_64);
        let float = NcAbiValue::scalar(NcAbiValueKind::Float, 8, 8);
        let plan = abi.plan_call(1, &[float, float, float], None).unwrap();
        assert_eq!(plan.varargs_float_register_count_register, Some("al"));
        assert_eq!(plan.varargs_float_register_count, 3);
        assert!(plan
            .arguments
            .iter()
            .all(|arg| matches!(arg.location, NcAbiLocation::Registers(_))));
    }

    #[test]
    fn x86_64_windows_uses_its_explicit_nc_win64_variant() {
        let abi = nc_abi_contract_for_triple("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(abi.variant, NcAbiVariant::Win64);
        assert_eq!(abi.integer_argument_registers, ["rcx", "rdx", "r8", "r9"]);
        assert_eq!(abi.sret_register, Some("rcx"));
        assert_eq!(abi.shadow_space_bytes, 32);
        assert_eq!(abi.tls_base_register, "fs.base");
        assert_eq!(abi.per_cpu_base_register, "gs.base");
        let float = NcAbiValue::scalar(NcAbiValueKind::Float, 8, 8);
        let plan = abi.plan_call(0, &[float], None).unwrap();
        assert_eq!(plan.varargs_float_register_count_register, None);
        assert_eq!(plan.shadow_space_bytes, 32);
        assert!(matches!(
            plan.arguments[0].location,
            NcAbiLocation::Registers(ref registers) if registers == &["rcx"]
        ));
        assert!(matches!(
            plan.arguments[0].mirror_location,
            Some(NcAbiLocation::Registers(ref registers)) if registers == &["xmm0"]
        ));

        let integer = NcAbiValue::scalar(NcAbiValueKind::Integer, 8, 8);
        let positional = abi
            .plan_call(5, &[integer, float, integer, float, integer], None)
            .unwrap();
        assert!(matches!(
            positional.arguments[0].location,
            NcAbiLocation::Registers(ref registers) if registers == &["rcx"]
        ));
        assert!(matches!(
            positional.arguments[1].location,
            NcAbiLocation::Registers(ref registers) if registers == &["xmm1"]
        ));
        assert!(matches!(
            positional.arguments[4].location,
            NcAbiLocation::Stack {
                offset_bytes: 32,
                ..
            }
        ));
        assert_eq!(positional.stack_argument_bytes, 48);
    }

    #[test]
    fn i386_windows_uses_cdecl_ilp32_and_win32_segment_contract() {
        let abi = nc_abi_contract_for_triple("i686-pc-windows-msvc").unwrap();
        assert_eq!(abi.target, NcAbiTarget::I386);
        assert_eq!(abi.variant, NcAbiVariant::I386Cdecl);
        assert_eq!(abi.canonical_triple, "i686-pc-windows-msvc");
        assert_eq!(abi.pointer_bits, 32);
        assert_eq!(abi.stack_align_bytes, 4);
        assert!(abi.integer_argument_registers.is_empty());
        assert_eq!(abi.tls_base_register, "fs");
        assert_eq!(abi.per_cpu_base_register, "gs");
    }

    #[test]
    fn manual_win64_contract_validates_and_plans_real_locations() {
        let declaration = ncc_ast::ManualAbiDecl {
            name: "hostWin64".into(),
            target: "x86_64-pc-windows-msvc".into(),
            pointer_bits: 64,
            stack_align_bytes: 16,
            shadow_space_bytes: 32,
            red_zone_bytes: 0,
            integer_argument_registers: ["rcx", "rdx", "r8", "r9"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            float_argument_registers: ["xmm0", "xmm1", "xmm2", "xmm3"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            integer_return_registers: vec!["rax".into()],
            float_return_registers: vec!["xmm0".into()],
            callee_saved_registers: vec!["rbx".into(), "rbp".into()],
            clobbered_registers: vec!["rax".into(), "r10".into(), "r11".into()],
            sret_register: Some("rcx".into()),
        };
        let contract = NcManualAbiContract::try_from(&declaration).expect("manual ABI");
        let integer = NcAbiValue::scalar(NcAbiValueKind::Integer, 8, 8);
        let plan = contract
            .plan_call(
                &[integer, integer, integer, integer, integer],
                Some(integer),
            )
            .expect("manual call plan");
        assert!(matches!(
            &plan.arguments[0].location,
            NcManualAbiLocation::Registers(registers) if registers == &["rcx"]
        ));
        assert!(matches!(
            &plan.arguments[4].location,
            NcManualAbiLocation::Stack {
                offset_bytes: 32,
                ..
            }
        ));
        assert!(matches!(
            &plan.return_location,
            NcManualAbiLocation::Registers(registers) if registers == &["rax"]
        ));
        assert_eq!(plan.stack_argument_bytes, 48);
    }
}
