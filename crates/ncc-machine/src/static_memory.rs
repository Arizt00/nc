use ncc_ast::{StorageContract, StorageDuration, StorageLinkage, StorageSection};

use crate::{machine_profile, MachineBinaryFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSectionFamily {
    Elf,
    Coff,
    MachO,
    Firmware,
    Wasm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSectionKind {
    Data,
    Bss,
    ReadOnlyData,
    TlsData,
    TlsBss,
    PerCpuData,
    PerCpuBss,
    InitArray,
    FiniArray,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStorageSectionPlan {
    pub family: NativeSectionFamily,
    pub kind: NativeSectionKind,
    pub segment: Option<&'static str>,
    pub name: String,
    pub align_bytes: usize,
    pub alloc: bool,
    pub writable: bool,
    pub executable: bool,
    pub tls: bool,
    pub zero_fill: bool,
    pub discardable_after_init: bool,
    pub linker_gc_eligible: bool,
    pub comdat_group: Option<String>,
    pub linkage: StorageLinkage,
}

pub fn native_storage_section_plan(
    target: &str,
    symbol: &str,
    contract: &StorageContract,
    initialized: bool,
    natural_align: usize,
) -> Result<NativeStorageSectionPlan, String> {
    let family = native_section_family(target)?;
    let align_bytes = contract.align_bytes.unwrap_or(natural_align.max(1));
    if !align_bytes.is_power_of_two() {
        return Err(format!(
            "E_MACHINE_STORAGE_ALIGN_001: symbol '{symbol}' requested non-power-of-two alignment {align_bytes}"
        ));
    }
    let kind = if contract.custom_section.is_some() {
        NativeSectionKind::Custom
    } else {
        match (contract.duration, contract.section, initialized) {
            (StorageDuration::ThreadLocal, _, true) => NativeSectionKind::TlsData,
            (StorageDuration::ThreadLocal, _, false) => NativeSectionKind::TlsBss,
            (StorageDuration::PerCpu, _, true) => NativeSectionKind::PerCpuData,
            (StorageDuration::PerCpu, _, false) => NativeSectionKind::PerCpuBss,
            (_, StorageSection::ReadOnlyData, _) => NativeSectionKind::ReadOnlyData,
            (_, StorageSection::Bss, _) => NativeSectionKind::Bss,
            (_, StorageSection::Data, _) => NativeSectionKind::Data,
            (_, StorageSection::Automatic, true) => NativeSectionKind::Data,
            (_, StorageSection::Automatic, false) => NativeSectionKind::Bss,
        }
    };
    let (segment, default_name) = native_section_spelling(family, kind);
    let name = contract.custom_section.clone().unwrap_or_else(|| {
        if family == NativeSectionFamily::MachO {
            default_name.to_string()
        } else {
            format!("{default_name}.{}", sanitize_section_component(symbol))
        }
    });
    let zero_fill = matches!(
        kind,
        NativeSectionKind::Bss | NativeSectionKind::TlsBss | NativeSectionKind::PerCpuBss
    ) || (kind == NativeSectionKind::Custom
        && (contract.section == StorageSection::Bss
            || (contract.section == StorageSection::Automatic && !initialized)));
    let writable = contract.section != StorageSection::ReadOnlyData;
    Ok(NativeStorageSectionPlan {
        family,
        kind,
        segment,
        name,
        align_bytes,
        alloc: true,
        writable,
        executable: false,
        tls: contract.duration == StorageDuration::ThreadLocal,
        zero_fill,
        discardable_after_init: false,
        linker_gc_eligible: contract.linkage != StorageLinkage::External,
        comdat_group: contract.comdat.clone(),
        linkage: contract.linkage,
    })
}

pub fn native_lifecycle_section_plan(
    target: &str,
    constructor: bool,
    priority: u16,
) -> Result<NativeStorageSectionPlan, String> {
    let family = native_section_family(target)?;
    let kind = if constructor {
        NativeSectionKind::InitArray
    } else {
        NativeSectionKind::FiniArray
    };
    let (segment, default_name) = native_section_spelling(family, kind);
    let name = match family {
        NativeSectionFamily::Coff => {
            if constructor {
                format!(".CRT$XCU{priority:05}")
            } else {
                format!(".CRT$XTU{priority:05}")
            }
        }
        NativeSectionFamily::MachO => default_name.to_string(),
        NativeSectionFamily::Elf | NativeSectionFamily::Firmware | NativeSectionFamily::Wasm => {
            format!("{default_name}.{priority:05}")
        }
    };
    Ok(NativeStorageSectionPlan {
        family,
        kind,
        segment,
        name,
        align_bytes: pointer_bytes_for_target(target)?,
        alloc: true,
        writable: true,
        executable: false,
        tls: false,
        zero_fill: false,
        discardable_after_init: false,
        linker_gc_eligible: false,
        comdat_group: None,
        linkage: StorageLinkage::Internal,
    })
}

fn native_section_family(target: &str) -> Result<NativeSectionFamily, String> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") || lower.contains("msvc") || lower.contains("mingw") {
        return Ok(NativeSectionFamily::Coff);
    }
    if lower.contains("darwin") || lower.contains("apple") || lower.contains("macos") {
        return Ok(NativeSectionFamily::MachO);
    }
    if lower.starts_with("wasm32") {
        return Ok(NativeSectionFamily::Wasm);
    }
    let profile = machine_profile(target)
        .ok_or_else(|| format!("E_MACHINE_STORAGE_TARGET_001: unsupported target '{target}'"))?;
    Ok(match profile.object_format {
        MachineBinaryFormat::Elf32 | MachineBinaryFormat::Elf64 => NativeSectionFamily::Elf,
        MachineBinaryFormat::Coff64 => NativeSectionFamily::Coff,
        MachineBinaryFormat::MachO64 => NativeSectionFamily::MachO,
        MachineBinaryFormat::RawFirmware => NativeSectionFamily::Firmware,
    })
}

fn native_section_spelling(
    family: NativeSectionFamily,
    kind: NativeSectionKind,
) -> (Option<&'static str>, &'static str) {
    match (family, kind) {
        (NativeSectionFamily::MachO, NativeSectionKind::ReadOnlyData) => {
            (Some("__TEXT"), "__const")
        }
        (NativeSectionFamily::MachO, NativeSectionKind::Bss) => (Some("__DATA"), "__bss"),
        (NativeSectionFamily::MachO, NativeSectionKind::TlsData) => {
            (Some("__DATA"), "__thread_data")
        }
        (NativeSectionFamily::MachO, NativeSectionKind::TlsBss) => (Some("__DATA"), "__thread_bss"),
        (NativeSectionFamily::MachO, NativeSectionKind::PerCpuData) => (Some("__DATA"), "__percpu"),
        (NativeSectionFamily::MachO, NativeSectionKind::PerCpuBss) => {
            (Some("__DATA"), "__percpu_bss")
        }
        (NativeSectionFamily::MachO, NativeSectionKind::InitArray) => {
            (Some("__DATA"), "__mod_init_func")
        }
        (NativeSectionFamily::MachO, NativeSectionKind::FiniArray) => {
            (Some("__DATA"), "__mod_term_func")
        }
        (NativeSectionFamily::MachO, _) => (Some("__DATA"), "__data"),
        (NativeSectionFamily::Coff, NativeSectionKind::ReadOnlyData) => (None, ".rdata"),
        (NativeSectionFamily::Coff, NativeSectionKind::Bss) => (None, ".bss"),
        (NativeSectionFamily::Coff, NativeSectionKind::TlsData) => (None, ".tls$A"),
        (NativeSectionFamily::Coff, NativeSectionKind::TlsBss) => (None, ".tls$B"),
        (NativeSectionFamily::Coff, NativeSectionKind::PerCpuData) => (None, ".data$percpu"),
        (NativeSectionFamily::Coff, NativeSectionKind::PerCpuBss) => (None, ".bss$percpu"),
        (NativeSectionFamily::Coff, NativeSectionKind::InitArray) => (None, ".CRT$XCU"),
        (NativeSectionFamily::Coff, NativeSectionKind::FiniArray) => (None, ".CRT$XTU"),
        (NativeSectionFamily::Coff, _) => (None, ".data"),
        (_, NativeSectionKind::ReadOnlyData) => (None, ".rodata"),
        (_, NativeSectionKind::Bss) => (None, ".bss"),
        (_, NativeSectionKind::TlsData) => (None, ".tdata"),
        (_, NativeSectionKind::TlsBss) => (None, ".tbss"),
        (_, NativeSectionKind::PerCpuData) => (None, ".data..percpu"),
        (_, NativeSectionKind::PerCpuBss) => (None, ".bss..percpu"),
        (_, NativeSectionKind::InitArray) => (None, ".init_array"),
        (_, NativeSectionKind::FiniArray) => (None, ".fini_array"),
        _ => (None, ".data"),
    }
}

fn sanitize_section_component(symbol: &str) -> String {
    symbol
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn pointer_bytes_for_target(target: &str) -> Result<usize, String> {
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("wasm32") {
        return Ok(4);
    }
    if (lower.contains("x86_64") || lower.contains("aarch64") || lower.contains("arm64"))
        && (lower.contains("windows")
            || lower.contains("darwin")
            || lower.contains("apple")
            || lower.contains("macos")
            || lower.contains("ios"))
    {
        return Ok(8);
    }
    machine_profile(target)
        .map(|profile| usize::from(profile.pointer_width) / 8)
        .ok_or_else(|| format!("E_MACHINE_STORAGE_TARGET_001: unsupported target '{target}'"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleEntry {
    pub module: String,
    pub symbol: String,
    pub priority: u16,
    pub source_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecyclePlan {
    pub constructors: Vec<LifecycleEntry>,
    pub destructors: Vec<LifecycleEntry>,
}

pub fn order_lifecycle_entries(
    mut constructors: Vec<LifecycleEntry>,
    mut destructors: Vec<LifecycleEntry>,
) -> LifecyclePlan {
    let key = |entry: &LifecycleEntry| {
        (
            entry.priority,
            entry.module.clone(),
            entry.source_order,
            entry.symbol.clone(),
        )
    };
    constructors.sort_by_key(&key);
    destructors.sort_by_key(&key);
    destructors.reverse();
    LifecyclePlan {
        constructors,
        destructors,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackProbeStrategy {
    None,
    WindowsChkstk,
    InlinePageTouch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardPageOwner {
    HostedOsThread,
    KernelOrLoader,
    FirmwareLinkContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnwindEncoding {
    DwarfCfi,
    WindowsPdataXdata,
    MachOCompactOrDwarf,
    ArmEhabi,
    FirmwareNone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicStackOp {
    SaveStackPointer,
    AlignSize(usize),
    ProbePages { page_bytes: usize },
    AdjustStack,
    RestoreStackPointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineStackPlan {
    pub target: String,
    pub stack_align_bytes: usize,
    pub page_bytes: usize,
    pub probe_threshold_bytes: usize,
    pub probe_strategy: StackProbeStrategy,
    pub probe_symbol: Option<&'static str>,
    pub guard_page_owner: GuardPageOwner,
    pub unwind: UnwindEncoding,
    pub dynamic_ops: Vec<DynamicStackOp>,
}

pub fn machine_stack_plan(target: &str) -> Result<MachineStackPlan, String> {
    let lower = target.to_ascii_lowercase();
    let hosted_windows = lower.contains("windows") || lower.contains("msvc");
    let hosted_macos = lower.contains("darwin") || lower.contains("apple");
    let firmware = lower.contains("eabi") || lower.contains("firmware");
    let (architecture, stack_align_bytes) = if lower.contains("x86_64") {
        ("x86_64", 16)
    } else if lower.contains("i386") || lower.contains("i686") {
        ("i386", 16)
    } else if lower.contains("aarch64") {
        ("aarch64", 16)
    } else if lower.contains("armv7") || lower.contains("thumbv7") {
        ("armv7", 8)
    } else if lower.contains("riscv64") || lower.contains("riscv32") {
        ("riscv", 16)
    } else {
        return Err(format!(
            "E_MACHINE_STACK_TARGET_001: unsupported target '{target}'"
        ));
    };
    let probe_strategy = if hosted_windows {
        StackProbeStrategy::WindowsChkstk
    } else if firmware {
        StackProbeStrategy::None
    } else {
        StackProbeStrategy::InlinePageTouch
    };
    let probe_symbol = if hosted_windows {
        Some(if architecture == "i386" {
            "_chkstk"
        } else {
            "__chkstk"
        })
    } else {
        None
    };
    let guard_page_owner = if firmware {
        GuardPageOwner::FirmwareLinkContract
    } else if lower.contains("none") || lower.contains("elf") && !lower.contains("linux") {
        GuardPageOwner::KernelOrLoader
    } else {
        GuardPageOwner::HostedOsThread
    };
    let unwind = if hosted_windows {
        UnwindEncoding::WindowsPdataXdata
    } else if hosted_macos {
        UnwindEncoding::MachOCompactOrDwarf
    } else if firmware {
        if architecture == "armv7" {
            UnwindEncoding::ArmEhabi
        } else {
            UnwindEncoding::FirmwareNone
        }
    } else {
        UnwindEncoding::DwarfCfi
    };
    Ok(MachineStackPlan {
        target: target.to_string(),
        stack_align_bytes,
        page_bytes: 4096,
        probe_threshold_bytes: 4096,
        probe_strategy,
        probe_symbol,
        guard_page_owner,
        unwind,
        dynamic_ops: vec![
            DynamicStackOp::SaveStackPointer,
            DynamicStackOp::AlignSize(stack_align_bytes),
            DynamicStackOp::ProbePages { page_bytes: 4096 },
            DynamicStackOp::AdjustStack,
            DynamicStackOp::RestoreStackPointer,
        ],
    })
}

pub fn encode_fixed_stack_adjust(
    target: &str,
    bytes: usize,
    allocate: bool,
) -> Result<Vec<u8>, String> {
    let plan = machine_stack_plan(target)?;
    if bytes % plan.stack_align_bytes != 0 {
        return Err(format!(
            "E_MACHINE_STACK_ALIGN_001: stack adjustment {bytes} is not aligned to {} bytes",
            plan.stack_align_bytes
        ));
    }
    let lower = target.to_ascii_lowercase();
    if lower.contains("x86_64") {
        let mut out = vec![0x48];
        if bytes <= 0x7f {
            out.extend_from_slice(&[0x83, if allocate { 0xec } else { 0xc4 }, bytes as u8]);
        } else {
            let immediate = u32::try_from(bytes)
                .map_err(|_| "E_MACHINE_STACK_RANGE_001: x86 stack frame exceeds imm32")?;
            out.extend_from_slice(&[0x81, if allocate { 0xec } else { 0xc4 }]);
            out.extend_from_slice(&immediate.to_le_bytes());
        }
        return Ok(out);
    }
    if lower.contains("i386") || lower.contains("i686") {
        let mut out = Vec::new();
        if bytes <= 0x7f {
            out.extend_from_slice(&[0x83, if allocate { 0xec } else { 0xc4 }, bytes as u8]);
        } else {
            let immediate = u32::try_from(bytes)
                .map_err(|_| "E_MACHINE_STACK_RANGE_001: i386 stack frame exceeds imm32")?;
            out.extend_from_slice(&[0x81, if allocate { 0xec } else { 0xc4 }]);
            out.extend_from_slice(&immediate.to_le_bytes());
        }
        return Ok(out);
    }
    if lower.contains("aarch64") {
        let (imm12, shift) = if bytes <= 4095 {
            (bytes as u32, 0)
        } else if bytes % 4096 == 0 && bytes / 4096 <= 4095 {
            ((bytes / 4096) as u32, 1)
        } else {
            return Err("E_MACHINE_STACK_RANGE_001: AArch64 stack adjustment is not encodable as add/sub immediate".to_string());
        };
        let base = if allocate { 0xd100_03ff } else { 0x9100_03ff };
        return Ok((base | (shift << 22) | (imm12 << 10))
            .to_le_bytes()
            .to_vec());
    }
    if lower.contains("armv7") || lower.contains("thumbv7") {
        if bytes % 4 != 0 || bytes / 4 > 127 {
            return Err(
                "E_MACHINE_STACK_RANGE_001: Thumb stack adjustment requires imm7*4".to_string(),
            );
        }
        let base = if allocate { 0xb080u16 } else { 0xb000u16 };
        return Ok((base | (bytes as u16 / 4)).to_le_bytes().to_vec());
    }
    if lower.contains("riscv") {
        let signed = if allocate {
            -(bytes as i64)
        } else {
            bytes as i64
        };
        if !(-2048..=2047).contains(&signed) {
            return Err(
                "E_MACHINE_STACK_RANGE_001: RISC-V stack adjustment exceeds addi imm12".to_string(),
            );
        }
        let immediate = (signed as u32) & 0x0fff;
        let instruction = (immediate << 20) | (2 << 15) | (2 << 7) | 0x13;
        return Ok(instruction.to_le_bytes().to_vec());
    }
    Err(format!(
        "E_MACHINE_STACK_TARGET_001: unsupported target '{target}'"
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackPrimitive {
    MarkToAbiReturn,
    RestoreFromAbiReturn,
    ProbeCurrentPage,
}

/// Encodes the irreducible stack primitives used by raw VLA lowering. The
/// mark/restore register is the integer ABI return register (RAX/EAX, X0, R0
/// or A0); page probing performs a real read from the current stack page.
pub fn encode_stack_primitive(target: &str, primitive: StackPrimitive) -> Result<Vec<u8>, String> {
    let lower = target.to_ascii_lowercase();
    let bytes = if lower.contains("x86_64") {
        match primitive {
            StackPrimitive::MarkToAbiReturn => vec![0x48, 0x89, 0xe0],
            StackPrimitive::RestoreFromAbiReturn => vec![0x48, 0x89, 0xc4],
            StackPrimitive::ProbeCurrentPage => vec![0xf6, 0x04, 0x24, 0x00],
        }
    } else if lower.contains("i386") || lower.contains("i686") {
        match primitive {
            StackPrimitive::MarkToAbiReturn => vec![0x89, 0xe0],
            StackPrimitive::RestoreFromAbiReturn => vec![0x89, 0xc4],
            StackPrimitive::ProbeCurrentPage => vec![0xf6, 0x04, 0x24, 0x00],
        }
    } else if lower.contains("aarch64") {
        let instruction = match primitive {
            StackPrimitive::MarkToAbiReturn => 0x9100_03e0u32, // add x0, sp, #0
            StackPrimitive::RestoreFromAbiReturn => 0x9100_001fu32, // add sp, x0, #0
            StackPrimitive::ProbeCurrentPage => 0xf940_03ffu32, // ldr xzr, [sp]
        };
        instruction.to_le_bytes().to_vec()
    } else if lower.contains("thumbv7") {
        match primitive {
            StackPrimitive::MarkToAbiReturn => 0x4668u16.to_le_bytes().to_vec(),
            StackPrimitive::RestoreFromAbiReturn => 0x4685u16.to_le_bytes().to_vec(),
            StackPrimitive::ProbeCurrentPage => 0x9b00u16.to_le_bytes().to_vec(),
        }
    } else if lower.contains("armv7") {
        let instruction = match primitive {
            StackPrimitive::MarkToAbiReturn => 0xe1a0_000du32, // mov r0, sp
            StackPrimitive::RestoreFromAbiReturn => 0xe1a0_d000u32, // mov sp, r0
            StackPrimitive::ProbeCurrentPage => 0xe59d_c000u32, // ldr r12, [sp]
        };
        instruction.to_le_bytes().to_vec()
    } else if lower.contains("riscv") {
        let instruction = match primitive {
            StackPrimitive::MarkToAbiReturn => 0x0001_0513u32, // addi a0, sp, 0
            StackPrimitive::RestoreFromAbiReturn => 0x0005_0113u32, // addi sp, a0, 0
            StackPrimitive::ProbeCurrentPage => 0x0001_0003u32, // lb zero, 0(sp)
        };
        instruction.to_le_bytes().to_vec()
    } else {
        return Err(format!(
            "E_MACHINE_STACK_TARGET_001: unsupported target '{target}'"
        ));
    };
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_sections_cover_bss_tls_percpu_and_custom_alignment() {
        let tls = StorageContract {
            duration: StorageDuration::ThreadLocal,
            align_bytes: Some(64),
            ..StorageContract::default()
        };
        let plan = native_storage_section_plan("x86_64-unknown-linux-gnu", "errno", &tls, false, 8)
            .expect("TLS plan");
        assert_eq!(plan.kind, NativeSectionKind::TlsBss);
        assert!(plan.name.starts_with(".tbss."));
        assert!(plan.tls && plan.zero_fill);
        assert_eq!(plan.align_bytes, 64);

        let percpu = StorageContract {
            duration: StorageDuration::PerCpu,
            ..StorageContract::default()
        };
        let plan = native_storage_section_plan("aarch64-unknown-none", "ticks", &percpu, true, 8)
            .expect("per-CPU plan");
        assert_eq!(plan.kind, NativeSectionKind::PerCpuData);
        assert!(plan.name.starts_with(".data..percpu."));

        let android = native_storage_section_plan(
            "aarch64-linux-android29",
            "blocklist",
            &StorageContract::default(),
            true,
            8,
        )
        .expect("Android ELF storage plan");
        assert_eq!(android.family, NativeSectionFamily::Elf);
        assert!(android.name.starts_with(".data."));
        assert_eq!(android.align_bytes, 8);

        let ios = native_storage_section_plan(
            "aarch64-apple-darwin",
            "uncall_rules",
            &StorageContract::default(),
            true,
            8,
        )
        .expect("Apple ARM64 Mach-O storage plan");
        assert_eq!(ios.family, NativeSectionFamily::MachO);
        assert_eq!(ios.segment, Some("__DATA"));
        assert_eq!(ios.name, "__data");
        assert_eq!(ios.align_bytes, 8);

        let lifecycle = native_lifecycle_section_plan("aarch64-apple-darwin", true, 100)
            .expect("Apple ARM64 lifecycle plan");
        assert_eq!(lifecycle.kind, NativeSectionKind::InitArray);
        assert_eq!(lifecycle.name, "__mod_init_func");
        assert_eq!(lifecycle.align_bytes, 8);

        let wasm = native_storage_section_plan(
            "wasm32-wasi",
            "linear_state",
            &StorageContract::default(),
            true,
            4,
        )
        .expect("WASM32 storage plan");
        assert_eq!(wasm.family, NativeSectionFamily::Wasm);
        assert_eq!(wasm.name, ".data.linear_state");
        assert_eq!(wasm.align_bytes, 4);

        let wasm_lifecycle = native_lifecycle_section_plan("wasm32-wasi", false, 200)
            .expect("WASM32 lifecycle plan");
        assert_eq!(wasm_lifecycle.kind, NativeSectionKind::FiniArray);
        assert_eq!(wasm_lifecycle.name, ".fini_array.00200");
        assert_eq!(wasm_lifecycle.align_bytes, 4);
    }

    #[test]
    fn lifecycle_order_is_stable_and_destructors_are_reversed() {
        let entry = |module: &str, symbol: &str, priority, source_order| LifecycleEntry {
            module: module.to_string(),
            symbol: symbol.to_string(),
            priority,
            source_order,
        };
        let plan = order_lifecycle_entries(
            vec![entry("b", "b.init", 200, 0), entry("a", "a.init", 100, 0)],
            vec![entry("a", "a.fini", 100, 0), entry("b", "b.fini", 200, 0)],
        );
        assert_eq!(plan.constructors[0].symbol, "a.init");
        assert_eq!(plan.destructors[0].symbol, "b.fini");
    }

    #[test]
    fn stack_adjustment_encoders_emit_real_target_bytes() {
        assert_eq!(
            encode_fixed_stack_adjust("x86_64-unknown-linux-gnu", 32, true).unwrap(),
            vec![0x48, 0x83, 0xec, 0x20]
        );
        assert_eq!(
            encode_fixed_stack_adjust("i386-unknown-linux-gnu", 16, false).unwrap(),
            vec![0x83, 0xc4, 0x10]
        );
        assert_eq!(
            encode_fixed_stack_adjust("aarch64-unknown-none", 16, true).unwrap(),
            0xd100_43ffu32.to_le_bytes()
        );
        assert_eq!(
            encode_fixed_stack_adjust("armv7a-unknown-none-eabi", 16, true).unwrap(),
            0xb084u16.to_le_bytes()
        );
        assert_eq!(
            encode_fixed_stack_adjust("riscv64-unknown-elf", 16, true).unwrap(),
            0xff01_0113u32.to_le_bytes()
        );
    }

    #[test]
    fn hosted_and_kernel_stack_contracts_name_guard_owner_and_unwind() {
        let windows = machine_stack_plan("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(windows.probe_symbol, Some("__chkstk"));
        assert_eq!(windows.unwind, UnwindEncoding::WindowsPdataXdata);
        assert_eq!(windows.guard_page_owner, GuardPageOwner::HostedOsThread);

        let kernel = machine_stack_plan("aarch64-unknown-none").unwrap();
        assert_eq!(kernel.probe_strategy, StackProbeStrategy::InlinePageTouch);
        assert_eq!(kernel.guard_page_owner, GuardPageOwner::KernelOrLoader);
        assert_eq!(kernel.unwind, UnwindEncoding::DwarfCfi);
    }

    #[test]
    fn dynamic_stack_primitives_have_real_bytes_on_every_supported_isa() {
        assert_eq!(
            encode_stack_primitive("x86_64-unknown-linux-gnu", StackPrimitive::MarkToAbiReturn)
                .unwrap(),
            vec![0x48, 0x89, 0xe0]
        );
        assert_eq!(
            encode_stack_primitive("aarch64-unknown-none", StackPrimitive::ProbeCurrentPage)
                .unwrap(),
            0xf940_03ffu32.to_le_bytes()
        );
        assert_eq!(
            encode_stack_primitive(
                "thumbv7a-unknown-none-eabi",
                StackPrimitive::RestoreFromAbiReturn
            )
            .unwrap(),
            0x4685u16.to_le_bytes()
        );
        assert_eq!(
            encode_stack_primitive(
                "riscv32imac-unknown-none-elf",
                StackPrimitive::MarkToAbiReturn
            )
            .unwrap(),
            0x0001_0513u32.to_le_bytes()
        );
    }
}
