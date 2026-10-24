use core::ptr;
use core::sync::atomic::{fence, AtomicU64, Ordering};

pub const MACHINE_RAW_MEMORY_SCHEMA: &str = "nc.machine.raw_memory.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineRawMemoryPrimitive {
    Memcpy,
    Memmove,
    Memset,
    VolatileLoad,
    VolatileStore,
    AtomicLoad,
    AtomicStore,
    AtomicReadModifyWrite,
    Fence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineRawInstructionEncoding {
    pub mnemonic: &'static str,
    pub bytes_little_endian: &'static [u8],
    pub purpose: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineRawMemoryPlan {
    pub schema: &'static str,
    pub target: &'static str,
    pub architecture: &'static str,
    pub pointer_bits: u16,
    pub abi: &'static str,
    pub memcpy: &'static [&'static str],
    pub memmove: &'static [&'static str],
    pub memset: &'static [&'static str],
    pub volatile: &'static [&'static str],
    pub atomics: &'static [&'static str],
    pub encodings: &'static [MachineRawInstructionEncoding],
    pub hidden_bounds_checks: bool,
    pub hidden_alias_checks: bool,
    pub overlap_safe_memmove: bool,
}

const X86_64_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding("cld", &[0xfc], "forward string-memory direction"),
    encoding("std", &[0xfd], "backward string-memory direction"),
    encoding("rep movsb", &[0xf3, 0xa4], "byte copy or move loop"),
    encoding("rep stosb", &[0xf3, 0xaa], "byte fill loop"),
    encoding(
        "mfence",
        &[0x0f, 0xae, 0xf0],
        "sequentially-consistent fence",
    ),
    encoding(
        "lock xadd qword ptr [rdi], rax",
        &[0xf0, 0x48, 0x0f, 0xc1, 0x07],
        "atomic read-modify-write",
    ),
];

const I386_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding("cld", &[0xfc], "forward string-memory direction"),
    encoding("std", &[0xfd], "backward string-memory direction"),
    encoding("rep movsb", &[0xf3, 0xa4], "byte copy or move loop"),
    encoding("rep stosb", &[0xf3, 0xaa], "byte fill loop"),
    encoding(
        "mfence",
        &[0x0f, 0xae, 0xf0],
        "sequentially-consistent fence",
    ),
    encoding(
        "lock xadd dword ptr [edi], eax",
        &[0xf0, 0x0f, 0xc1, 0x07],
        "atomic read-modify-write",
    ),
];

const AARCH64_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding("ldrb w3, [x1]", &[0x23, 0x00, 0x40, 0x39], "byte load"),
    encoding("strb w3, [x0]", &[0x03, 0x00, 0x00, 0x39], "byte store"),
    encoding(
        "dmb ish",
        &[0xbf, 0x3b, 0x03, 0xd5],
        "inner-shareable memory fence",
    ),
    encoding(
        "yield",
        &[0x3f, 0x20, 0x03, 0xd5],
        "exclusive-loop contention hint",
    ),
];

const ARMV7_THUMB_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding("ldrb r3, [r1]", &[0x0b, 0x78], "Thumb byte load"),
    encoding("strb r3, [r0]", &[0x03, 0x70], "Thumb byte store"),
    encoding(
        "dmb sy",
        &[0xbf, 0xf3, 0x5f, 0x8f],
        "Thumb-2 full-system memory fence",
    ),
];

const RISCV32_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding(
        "lbu t0, 0(a1)",
        &[0x83, 0xc2, 0x05, 0x00],
        "unsigned byte load",
    ),
    encoding("sb t0, 0(a0)", &[0x23, 0x00, 0x55, 0x00], "byte store"),
    encoding(
        "fence rw, rw",
        &[0x0f, 0x00, 0x30, 0x03],
        "read/write memory fence",
    ),
    encoding(
        "amoadd.w.aqrl a0, a1, (a2)",
        &[0x2f, 0x25, 0xb6, 0x06],
        "A-extension atomic read-modify-write",
    ),
];

const RISCV64_ENCODINGS: &[MachineRawInstructionEncoding] = &[
    encoding(
        "lbu t0, 0(a1)",
        &[0x83, 0xc2, 0x05, 0x00],
        "unsigned byte load",
    ),
    encoding("sb t0, 0(a0)", &[0x23, 0x00, 0x55, 0x00], "byte store"),
    encoding(
        "fence rw, rw",
        &[0x0f, 0x00, 0x30, 0x03],
        "read/write memory fence",
    ),
    encoding(
        "amoadd.d.aqrl a0, a1, (a2)",
        &[0x2f, 0x35, 0xb6, 0x06],
        "A-extension atomic read-modify-write",
    ),
];

const fn encoding(
    mnemonic: &'static str,
    bytes_little_endian: &'static [u8],
    purpose: &'static str,
) -> MachineRawInstructionEncoding {
    MachineRawInstructionEncoding {
        mnemonic,
        bytes_little_endian,
        purpose,
    }
}

pub fn machine_raw_memory_plan(target: &str) -> Result<MachineRawMemoryPlan, String> {
    let plan = match target {
        "x86_64-pc-windows-msvc" => plan(
            "x86_64-pc-windows-msvc",
            "x86_64",
            64,
            "win64",
            &["cld", "rep movsb"],
            &["cmp", "ja/jae", "std", "rep movsb", "cld"],
            &["cld", "rep stosb"],
            &["mov"],
            &["lock xadd", "lock cmpxchg", "mfence"],
            X86_64_ENCODINGS,
        ),
        "x86_64-unknown-linux-gnu" => plan(
            "x86_64-unknown-linux-gnu",
            "x86_64",
            64,
            "sysv64",
            &["cld", "rep movsb"],
            &["cmp", "ja/jae", "std", "rep movsb", "cld"],
            &["cld", "rep stosb"],
            &["mov"],
            &["lock xadd", "lock cmpxchg", "mfence"],
            X86_64_ENCODINGS,
        ),
        "x86_64-apple-darwin" => plan(
            "x86_64-apple-darwin",
            "x86_64",
            64,
            "sysv64",
            &["cld", "rep movsb"],
            &["cmp", "ja/jae", "std", "rep movsb", "cld"],
            &["cld", "rep stosb"],
            &["mov"],
            &["lock xadd", "lock cmpxchg", "mfence"],
            X86_64_ENCODINGS,
        ),
        "i386-unknown-linux-gnu" => plan(
            "i386-unknown-linux-gnu",
            "i386",
            32,
            "sysv32-cdecl",
            &["cld", "rep movsb"],
            &["cmp", "ja/jae", "std", "rep movsb", "cld"],
            &["cld", "rep stosb"],
            &["mov"],
            &["lock xadd", "lock cmpxchg", "mfence"],
            I386_ENCODINGS,
        ),
        "aarch64-unknown-linux-gnu" => plan(
            "aarch64-unknown-linux-gnu",
            "aarch64",
            64,
            "aapcs64",
            &["ldp/stp", "ldrb/strb"],
            &["cmp", "b.hs/b.lo", "ldrb/strb pre/post-index"],
            &["stp", "strb"],
            &["ldar", "stlr", "ldr", "str"],
            &["ldxr/stxr", "ldaxr/stlxr", "dmb ish"],
            AARCH64_ENCODINGS,
        ),
        "aarch64-apple-darwin" => plan(
            "aarch64-apple-darwin",
            "aarch64",
            64,
            "aapcs64",
            &["ldp/stp", "ldrb/strb"],
            &["cmp", "b.hs/b.lo", "ldrb/strb pre/post-index"],
            &["stp", "strb"],
            &["ldar", "stlr", "ldr", "str"],
            &["ldxr/stxr", "ldaxr/stlxr", "dmb ish"],
            AARCH64_ENCODINGS,
        ),
        "armv7-unknown-none-eabi" => plan(
            "armv7-unknown-none-eabi",
            "armv7a-thumb2",
            32,
            "aapcs32",
            &["ldmia/stmia", "ldrb/strb"],
            &["cmp", "bhs/blo", "ldrb/strb"],
            &["stmia", "strb"],
            &["ldr", "str"],
            &["ldrex/strex", "dmb sy"],
            ARMV7_THUMB_ENCODINGS,
        ),
        "riscv32imac-unknown-elf" => plan(
            "riscv32imac-unknown-elf",
            "riscv32imac",
            32,
            "riscv-ilp32",
            &["lw/sw", "lbu/sb"],
            &["bgeu/bltu", "lbu/sb"],
            &["sw", "sb"],
            &["lw", "sw"],
            &["lr.w/sc.w", "amo*.w", "fence rw,rw"],
            RISCV32_ENCODINGS,
        ),
        "riscv64-unknown-elf" => plan(
            "riscv64-unknown-elf",
            "riscv64",
            64,
            "riscv-lp64",
            &["ld/sd", "lbu/sb"],
            &["bgeu/bltu", "lbu/sb"],
            &["sd", "sb"],
            &["ld", "sd"],
            &["lr.d/sc.d", "amo*.d", "fence rw,rw"],
            RISCV64_ENCODINGS,
        ),
        "riscv64-unknown-linux-gnu" => plan(
            "riscv64-unknown-linux-gnu",
            "riscv64",
            64,
            "riscv-lp64",
            &["ld/sd", "lbu/sb"],
            &["bgeu/bltu", "lbu/sb"],
            &["sd", "sb"],
            &["ld", "sd"],
            &["lr.d/sc.d", "amo*.d", "fence rw,rw"],
            RISCV64_ENCODINGS,
        ),
        _ => {
            return Err(format!(
                "E_MACHINE_RAW_MEMORY_TARGET_001: unsupported target '{target}'"
            ))
        }
    };
    Ok(plan)
}

#[allow(clippy::too_many_arguments)]
fn plan(
    target: &'static str,
    architecture: &'static str,
    pointer_bits: u16,
    abi: &'static str,
    memcpy: &'static [&'static str],
    memmove: &'static [&'static str],
    memset: &'static [&'static str],
    volatile: &'static [&'static str],
    atomics: &'static [&'static str],
    encodings: &'static [MachineRawInstructionEncoding],
) -> MachineRawMemoryPlan {
    MachineRawMemoryPlan {
        schema: MACHINE_RAW_MEMORY_SCHEMA,
        target,
        architecture,
        pointer_bits,
        abi,
        memcpy,
        memmove,
        memset,
        volatile,
        atomics,
        encodings,
        hidden_bounds_checks: false,
        hidden_alias_checks: false,
        overlap_safe_memmove: true,
    }
}

/// The caller owns validity, non-overlap, lifetime and synchronization.
pub unsafe fn execute_raw_memcpy(destination: *mut u8, source: *const u8, bytes: usize) {
    ptr::copy_nonoverlapping(source, destination, bytes);
}

/// The caller owns validity, lifetime and synchronization. Overlap is supported.
pub unsafe fn execute_raw_memmove(destination: *mut u8, source: *const u8, bytes: usize) {
    ptr::copy(source, destination, bytes);
}

/// The caller owns validity, lifetime and synchronization.
pub unsafe fn execute_raw_memset(destination: *mut u8, value: u8, bytes: usize) {
    ptr::write_bytes(destination, value, bytes);
}

/// The caller owns validity, alignment, lifetime, synchronization and address-space rules.
pub unsafe fn raw_volatile_load<T: Copy>(address: *const T) -> T {
    ptr::read_volatile(address)
}

/// The caller owns validity, alignment, lifetime, synchronization and address-space rules.
pub unsafe fn raw_volatile_store<T>(address: *mut T, value: T) {
    ptr::write_volatile(address, value);
}

pub fn raw_atomic_roundtrip(cell: &AtomicU64, value: u64) -> (u64, u64) {
    cell.store(value, Ordering::Release);
    let previous = cell.fetch_add(1, Ordering::AcqRel);
    fence(Ordering::SeqCst);
    (previous, cell.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGETS: [&str; 8] = [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "i386-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "armv7-unknown-none-eabi",
        "riscv32imac-unknown-elf",
        "riscv64-unknown-elf",
        "x86_64-apple-darwin",
    ];

    #[test]
    fn every_required_target_has_manual_memory_and_atomic_isa() {
        for target in TARGETS {
            let plan =
                machine_raw_memory_plan(target).unwrap_or_else(|error| panic!("{target}: {error}"));
            assert_eq!(plan.schema, MACHINE_RAW_MEMORY_SCHEMA);
            assert!(!plan.memcpy.is_empty());
            assert!(!plan.memmove.is_empty());
            assert!(!plan.memset.is_empty());
            assert!(!plan.volatile.is_empty());
            assert!(!plan.atomics.is_empty());
            assert!(!plan.encodings.is_empty());
            assert!(!plan.hidden_bounds_checks);
            assert!(!plan.hidden_alias_checks);
            assert!(plan.overlap_safe_memmove);
            assert!(plan
                .encodings
                .iter()
                .all(|row| !row.bytes_little_endian.is_empty()));
        }
    }

    #[test]
    fn memcpy_memmove_and_memset_match_byte_contracts() {
        let mut memory = (0u8..64).collect::<Vec<_>>();
        let base = memory.as_mut_ptr();
        unsafe {
            execute_raw_memcpy(base.add(32), base, 16);
        }
        assert_eq!(&memory[32..48], &(0u8..16).collect::<Vec<_>>());

        unsafe {
            execute_raw_memmove(base.add(4), base, 24);
        }
        assert_eq!(&memory[4..28], &(0u8..24).collect::<Vec<_>>());

        unsafe {
            execute_raw_memset(base.add(8), 0xa5, 12);
        }
        assert_eq!(&memory[8..20], &[0xa5; 12]);
    }

    #[test]
    fn volatile_and_atomic_operations_remain_observable() {
        let mut value = 7u64;
        unsafe {
            raw_volatile_store(&mut value, 11);
            assert_eq!(raw_volatile_load(&value), 11);
        }
        let cell = AtomicU64::new(0);
        assert_eq!(raw_atomic_roundtrip(&cell, 41), (41, 42));
    }
}
