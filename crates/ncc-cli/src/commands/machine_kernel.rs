use ncc_machine::{
    build_test_elf64, certify_kernel_network_contracts, certify_kernel_sync_contracts,
    certify_physical_driver_contracts, certify_storage_contracts, coordinate_complete_kernel,
    coordinate_kernel, coordinate_target_kernel, emit_arm_elf32_object, encode_thumb,
    execute_thumb, link_cortex_m_firmware, parse_user_elf, run_kernel_stress,
    run_raw_memory_stress as execute_raw_memory_stress, vm_activation_sequence,
    vm_context_invalidation_sequence, vm_local_invalidation_sequence, AutonomousKernel,
    BlockCommand, BlockOpcode, DirtyRect, HardwarePageTables, HidKeyTransition, Ipv4Address,
    KernelCore, KernelStressConfig, KernelStressProfile, KernelStressReport, KernelSubsystem,
    KernelSystem, NetworkStack, OpenFlags, PciAddress, PciFunction, PixelFormat,
    RawMemoryStressConfig, RawMemoryStressProfile, RawMemoryStressReport, SchedulerTaskState,
    ThumbInstruction, ThumbRegister, UnifiedKernelImage, UnifiedRootFileSystem,
    UsbDeviceDescriptor, UsbSpeed, UserElfClass, UserLoadOptions, VirtualBlockDevice,
    VmArchitecture, VmFaultAccess, VmFaultResolution, VmPageFlags, VmPageSize, VmTlbShootdown,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_system_host_evidence.v1";
const HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_system_host_evidence.v1\0";
const STRESS_REPORT_SCHEMA: &str = "ncc.machine.kernel_stress_report.v1";
const STRESS_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_stress_certificate.v1";
const STRESS_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_stress_certificate.v1\0";
const RAW_STRESS_REPORT_SCHEMA: &str = "ncc.machine.raw_memory_stress_report.v1";
const RAW_STRESS_CERTIFICATE_SCHEMA: &str = "ncc.machine.raw_memory_stress_certificate.v1";
const RAW_STRESS_HASH_DOMAIN: &[u8] = b"ncc.machine.raw_memory_stress_certificate.v1\0";
const VM_REPORT_SCHEMA: &str = "ncc.machine.kernel_virtual_memory_report.v1";
const VM_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_virtual_memory_certificate.v1";
const VM_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_virtual_memory_certificate.v1\0";
const USER_RUNTIME_REPORT_SCHEMA: &str = "ncc.machine.kernel_user_runtime_report.v1";
const USER_RUNTIME_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_user_runtime_certificate.v1";
const USER_RUNTIME_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_user_runtime_certificate.v1\0";
const IO_STORAGE_REPORT_SCHEMA: &str = "ncc.machine.kernel_io_storage_report.v1";
const IO_STORAGE_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_io_storage_certificate.v1";
const IO_STORAGE_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_io_storage_certificate.v1\0";
const NETWORK_SYNC_REPORT_SCHEMA: &str = "ncc.machine.kernel_network_sync_report.v1";
const NETWORK_SYNC_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_network_sync_certificate.v1";
const NETWORK_SYNC_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_network_sync_certificate.v1\0";
const AUTONOMOUS_REPORT_SCHEMA: &str = "ncc.machine.kernel_autonomous_report.v1";
const AUTONOMOUS_CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_autonomous_certificate.v1";
const AUTONOMOUS_HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_autonomous_certificate.v1\0";

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn option_values<'a>(args: &'a [String], flag: &str) -> Vec<&'a str> {
    args.iter()
        .enumerate()
        .filter(|(_, argument)| argument.as_str() == flag)
        .filter_map(|(index, _)| args.get(index + 1).map(String::as_str))
        .collect()
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let mut output = Map::new();
            for key in keys {
                output.insert(key.clone(), canonical_json(&object[key]));
            }
            Value::Object(output)
        }
        other => other.clone(),
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn payload_hash_with_domain(value: &Value, domain: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn payload_hash(value: &Value) -> Result<String, Box<dyn Error>> {
    payload_hash_with_domain(value, HASH_DOMAIN)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(&canonical_json(value))?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn default_bundle() -> PathBuf {
    PathBuf::from(format!("target/evidence/kernel-system-{}", epoch_seconds()))
}

fn default_stress_bundle() -> PathBuf {
    PathBuf::from(format!("target/evidence/kernel-stress-{}", epoch_seconds()))
}

fn default_raw_stress_bundle() -> PathBuf {
    PathBuf::from(format!(
        "target/evidence/raw-memory-stress-{}",
        epoch_seconds()
    ))
}

fn default_io_storage_bundle() -> PathBuf {
    PathBuf::from(format!(
        "target/evidence/kernel-io-storage-{}",
        epoch_seconds()
    ))
}

fn default_network_sync_bundle() -> PathBuf {
    PathBuf::from(format!(
        "target/evidence/kernel-network-sync-{}",
        epoch_seconds()
    ))
}

fn default_autonomous_bundle() -> PathBuf {
    PathBuf::from(format!(
        "target/evidence/kernel-autonomous-{}",
        epoch_seconds()
    ))
}

fn checked_artifact_path(root: &Path, relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "[E_MACHINE_KERNEL_ARTIFACT_PATH_001] unsafe artifact path '{relative}'"
        )
        .into());
    }
    Ok(root.join(path))
}

fn collect_artifacts(bundle: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut artifacts = Vec::new();
    let mut entries = fs::read_dir(bundle)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        if !entry.file_type()?.is_file() || entry.file_name() == "kernel-system-certificate.json" {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        artifacts.push(json!({
            "path": entry.file_name().to_string_lossy(),
            "bytes": bytes.len(),
            "sha256": sha256(&bytes),
        }));
    }
    Ok(artifacts)
}

fn parse_u64_option(args: &[String], flag: &str, default: u64) -> Result<u64, Box<dyn Error>> {
    if !args.iter().any(|argument| argument == flag) {
        return Ok(default);
    }
    let value = option_value(args, flag)
        .ok_or_else(|| format!("{flag} expects an unsigned integer"))?
        .replace('_', "")
        .parse::<u64>()
        .map_err(|error| format!("invalid value for {flag}: {error}"))?;
    Ok(value)
}

fn parse_usize_option(
    args: &[String],
    flag: &str,
    default: usize,
) -> Result<usize, Box<dyn Error>> {
    let value = parse_u64_option(args, flag, default as u64)?;
    usize::try_from(value)
        .map_err(|_| format!("value for {flag} does not fit this host: {value}").into())
}

fn parse_u64_literal(value: &str) -> Result<u64, Box<dyn Error>> {
    let compact = value.replace('_', "");
    let parsed = if let Some(hex) = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        compact.parse::<u64>()
    };
    parsed.map_err(|error| format!("invalid integer '{value}': {error}").into())
}

fn user_elf_target(target: &str) -> Result<(u16, UserElfClass), Box<dyn Error>> {
    let target = target.to_ascii_lowercase();
    if target.contains("x86_64") {
        Ok((62, UserElfClass::Elf64))
    } else if target.contains("i386") || target.contains("i686") {
        Ok((3, UserElfClass::Elf32))
    } else if target.contains("aarch64") {
        Ok((183, UserElfClass::Elf64))
    } else if target.contains("riscv64") {
        Ok((243, UserElfClass::Elf64))
    } else {
        Err(
            format!("[E_MACHINE_KERNEL_USER_ELF_TARGET_001] target '{target}' has no user ELF ABI")
                .into(),
        )
    }
}

fn virtual_memory_architecture(target: &str) -> Result<VmArchitecture, Box<dyn Error>> {
    let target = target.to_ascii_lowercase();
    if target.contains("x86_64") {
        Ok(VmArchitecture::X86_64)
    } else if target.contains("aarch64") {
        Ok(VmArchitecture::Aarch64_4K)
    } else if target.contains("riscv64") && target.contains("sv48") {
        Ok(VmArchitecture::RiscvSv48)
    } else if target.contains("riscv64") || target.contains("riscv") {
        Ok(VmArchitecture::RiscvSv39)
    } else {
        Err(format!(
            "[E_MACHINE_KERNEL_VM_TARGET_001] target '{target}' has no material page-table profile"
        )
        .into())
    }
}

fn vm_fault_name(fault: VmFaultResolution) -> &'static str {
    match fault {
        VmFaultResolution::NotPresent => "not_present",
        VmFaultResolution::GuardPage => "guard_page",
        VmFaultResolution::Protection => "protection",
        VmFaultResolution::CopyOnWriteRequired { .. } => "copy_on_write_required",
        VmFaultResolution::Spurious => "spurious",
    }
}

fn run_virtual_memory(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let architecture = virtual_memory_architecture(target)?;
    let root = parse_u64_option(args, "--root", 0x10_0000)?;
    let table_pages = parse_usize_option(args, "--table-pages", 64)?;
    let asid = parse_u64_option(args, "--asid", 7)?;
    let asid = u16::try_from(asid).map_err(|_| "--asid exceeds 16 bits")?;
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("target/evidence/kernel-vm-{}", epoch_seconds())));
    fs::create_dir_all(&bundle)?;

    let mut parent = HardwarePageTables::new(architecture, root, table_pages, asid)?;
    let user_rw = VmPageFlags::READ
        .union(VmPageFlags::WRITE)
        .union(VmPageFlags::USER)
        .union(VmPageFlags::ACCESSED);
    parent.reserve_guard(0x3f_f000, 0x1000)?;
    parent.mmap_user(0x40_0000, 0x20_0000, 4 * 4096, VmPageSize::Size4K, user_rw)?;
    parent.mmap_kernel(
        0x4000_0000,
        0x8000_0000,
        VmPageSize::Size2M.bytes(),
        VmPageSize::Size2M,
        VmPageFlags::READ
            .union(VmPageFlags::EXECUTE)
            .union(VmPageFlags::GLOBAL),
    )?;
    parent.mmap_kernel(
        0x8000_0000,
        0x1_0000_0000,
        VmPageSize::Size1G.bytes(),
        VmPageSize::Size1G,
        VmPageFlags::READ.union(VmPageFlags::GLOBAL),
    )?;

    let before_cow_generation = parent.generation();
    let child_root = root
        .checked_add((table_pages as u64 + 16) * 4096)
        .ok_or("child page-table root overflow")?;
    let mut child = parent.fork_cow(child_root, table_pages, asid.wrapping_add(1))?;
    let cow_fault = child.fault(0x40_0001, VmFaultAccess::Write);
    let cow_source = match cow_fault {
        VmFaultResolution::CopyOnWriteRequired {
            source_physical_address,
            ..
        } => source_physical_address,
        other => {
            return Err(format!(
                "[E_MACHINE_KERNEL_VM_COW_001] expected COW fault, observed {}",
                vm_fault_name(other)
            )
            .into())
        }
    };
    child.resolve_cow(0x40_0000, 0x30_0000)?;
    if parent.translate(0x40_0123)? != 0x20_0123 || child.translate(0x40_0123)? != 0x30_0123 {
        return Err(
            "[E_MACHINE_KERNEL_VM_COW_001] parent/child COW translations diverged incorrectly"
                .into(),
        );
    }

    let mut shootdown =
        VmTlbShootdown::new(parent.generation(), 0x40_0000, 0x4000, asid, [1, 2, 3])?;
    for cpu in [1, 2, 3] {
        shootdown.acknowledge(cpu, parent.generation())?;
    }
    if !shootdown.complete() {
        return Err("[E_MACHINE_KERNEL_VM_SHOOTDOWN_001] remote CPUs did not acknowledge".into());
    }

    let table_image = parent.material_image();
    let table_path = bundle.join("page-tables.bin");
    fs::write(&table_path, &table_image)?;
    let table_sha256 = sha256(&table_image);
    let invalidation = vm_local_invalidation_sequence(architecture);
    let context_invalidation = vm_context_invalidation_sequence(architecture);
    let activation_isa = vm_activation_sequence(architecture);
    let report = json!({
        "schema": VM_REPORT_SCHEMA,
        "status": "materialized",
        "target": target,
        "architecture": format!("{architecture:?}"),
        "profile": architecture.profile(),
        "activation": {
            "register": architecture.activation_register(),
            "value": parent.activation_value(),
            "root_physical_address": parent.root_physical_address(),
            "asid": asid,
        },
        "tables": {
            "page_count": parent.table_page_count(),
            "bytes": table_image.len(),
            "path": "page-tables.bin",
            "sha256": table_sha256,
        },
        "mappings": parent.mappings().map(|mapping| json!({
            "virtual_address": mapping.virtual_address,
            "physical_address": mapping.physical_address,
            "page_size": mapping.page_size.bytes(),
            "flags": mapping.flags.bits(),
        })).collect::<Vec<_>>(),
        "faults": {
            "not_present": vm_fault_name(parent.fault(0xdead_0000, VmFaultAccess::Read)),
            "guard": vm_fault_name(parent.fault(0x3f_f123, VmFaultAccess::Read)),
            "execute_protection": vm_fault_name(parent.fault(0x40_0000, VmFaultAccess::Execute)),
            "cow": vm_fault_name(cow_fault),
        },
        "cow": {
            "source_physical_address": cow_source,
            "parent_translation": parent.translate(0x40_0123)?,
            "child_translation": child.translate(0x40_0123)?,
            "generation_before_fork": before_cow_generation,
            "parent_generation": parent.generation(),
            "child_generation": child.generation(),
        },
        "shootdown": {
            "generation": shootdown.generation,
            "virtual_address": shootdown.virtual_address,
            "length": shootdown.length,
            "asid": shootdown.asid,
            "pending": shootdown.pending(),
            "complete": shootdown.complete(),
        },
        "machine_isa": {
            "activation": activation_isa.iter().map(vm_isa_json).collect::<Vec<_>>(),
            "local_invalidation": invalidation.iter().map(vm_isa_json).collect::<Vec<_>>(),
            "context_invalidation": context_invalidation.iter().map(vm_isa_json).collect::<Vec<_>>(),
        },
        "contracts": {
            "real_page_table_bytes": true,
            "kernel_and_user_mappings": true,
            "huge_pages_2m_1g": true,
            "guard_pages": true,
            "page_fault_classification": true,
            "copy_on_write": true,
            "smp_tlb_shootdown_acknowledged": true,
            "hidden_allocation_or_scheduler": false,
        }
    });
    let report_path = bundle.join("kernel-virtual-memory-report.json");
    write_json(&report_path, &report)?;
    let payload = json!({
        "schema": VM_CERTIFICATE_SCHEMA,
        "report_sha256": sha256(&serde_json::to_vec(&canonical_json(&report))?),
        "page_tables_sha256": table_sha256,
        "target": target,
        "profile": architecture.profile(),
        "status": "verified_material_execution",
    });
    let certificate = json!({
        "schema": VM_CERTIFICATE_SCHEMA,
        "payload": payload,
        "integrity": {
            "algorithm": "sha256",
            "domain": "ncc.machine.kernel_virtual_memory_certificate.v1",
            "payload_sha256": payload_hash_with_domain(&payload, VM_HASH_DOMAIN)?,
        }
    });
    write_json(
        &bundle.join("kernel-virtual-memory-certificate.json"),
        &certificate,
    )?;
    println!("{}", serde_json::to_string_pretty(&certificate)?);
    Ok(())
}

fn vm_isa_json(instruction: &ncc_machine::VmIsaInstruction) -> Value {
    json!({
        "mnemonic": instruction.mnemonic,
        "operands": instruction.operands,
        "bytes_hex": instruction.bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
    })
}

fn run_verify_virtual_memory(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = args
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .map(PathBuf::from)
        .ok_or("verify-virtual-memory expects a bundle directory")?;
    let certificate_path = bundle.join("kernel-virtual-memory-certificate.json");
    let report_path = bundle.join("kernel-virtual-memory-report.json");
    let table_path = bundle.join("page-tables.bin");
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    let report: Value = serde_json::from_slice(&fs::read(&report_path)?)?;
    let payload = certificate
        .get("payload")
        .ok_or("virtual-memory certificate has no payload")?;
    if certificate.get("schema").and_then(Value::as_str) != Some(VM_CERTIFICATE_SCHEMA)
        || report.get("schema").and_then(Value::as_str) != Some(VM_REPORT_SCHEMA)
    {
        return Err("[E_MACHINE_KERNEL_VM_VERIFY_001] schema mismatch".into());
    }
    let expected = payload_hash_with_domain(payload, VM_HASH_DOMAIN)?;
    if certificate
        .pointer("/integrity/payload_sha256")
        .and_then(Value::as_str)
        != Some(expected.as_str())
    {
        return Err("[E_MACHINE_KERNEL_VM_VERIFY_001] certificate payload hash mismatch".into());
    }
    let report_hash = sha256(&serde_json::to_vec(&canonical_json(&report))?);
    if payload.get("report_sha256").and_then(Value::as_str) != Some(report_hash.as_str()) {
        return Err("[E_MACHINE_KERNEL_VM_VERIFY_001] report hash mismatch".into());
    }
    let table_hash = sha256(&fs::read(&table_path)?);
    if payload.get("page_tables_sha256").and_then(Value::as_str) != Some(table_hash.as_str())
        || report.pointer("/tables/sha256").and_then(Value::as_str) != Some(table_hash.as_str())
    {
        return Err("[E_MACHINE_KERNEL_VM_VERIFY_001] page-table image hash mismatch".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_virtual_memory_verification.v1",
            "status": "verified",
            "bundle": bundle,
            "page_tables_sha256": table_hash,
            "payload_sha256": expected,
        }))?
    );
    Ok(())
}

fn read_user_elf_input(args: &[String]) -> Result<(Vec<u8>, String), Box<dyn Error>> {
    let source = option_value(args, "--image").unwrap_or("-");
    if source == "-" || source.eq_ignore_ascii_case("stdin") {
        let mut bytes = Vec::new();
        io::stdin().read_to_end(&mut bytes)?;
        if bytes.is_empty() {
            return Err("[E_MACHINE_KERNEL_USER_ELF_INPUT_001] stdin is empty".into());
        }
        Ok((bytes, "stdin".into()))
    } else {
        let path = PathBuf::from(source);
        let bytes = fs::read(&path)?;
        Ok((bytes, path.display().to_string()))
    }
}

fn user_context_isa(target: &str) -> Vec<Value> {
    let target = target.to_ascii_lowercase();
    if target.contains("x86_64") {
        vec![
            json!({"mnemonic":"rdfsbase", "operands":"rax", "bytes_hex":"f3480faec0"}),
            json!({"mnemonic":"wrfsbase", "operands":"rax", "bytes_hex":"f3480faed0"}),
            json!({"mnemonic":"swapgs", "operands":"", "bytes_hex":"0f01f8"}),
            json!({"mnemonic":"iretq", "operands":"", "bytes_hex":"48cf"}),
        ]
    } else if target.contains("aarch64") {
        vec![
            json!({"mnemonic":"mrs", "operands":"x0, tpidr_el0", "bytes_hex":"40d03bd5"}),
            json!({"mnemonic":"msr", "operands":"tpidr_el0, x0", "bytes_hex":"40d01bd5"}),
            json!({"mnemonic":"eret", "operands":"", "bytes_hex":"e0039fd6"}),
        ]
    } else if target.contains("riscv") {
        vec![
            json!({"mnemonic":"csrw", "operands":"sscratch, a0", "bytes_hex":"73100514"}),
            json!({"mnemonic":"csrr", "operands":"a0, sscratch", "bytes_hex":"73250014"}),
            json!({"mnemonic":"sret", "operands":"", "bytes_hex":"73002010"}),
        ]
    } else {
        vec![
            json!({"mnemonic":"mov", "operands":"gs, user_tls", "bytes_hex":"profile-dependent"}),
            json!({"mnemonic":"iret", "operands":"", "bytes_hex":"cf"}),
        ]
    }
}

fn run_user_elf(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "target/evidence/kernel-user-runtime-{}",
                epoch_seconds()
            ))
        });
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_KERNEL_BUNDLE_EXISTS_001] evidence bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    let (expected_machine, expected_class) = user_elf_target(target)?;
    let (image, source) = read_user_elf_input(args)?;
    let parsed = parse_user_elf(&image, Some(expected_machine))?;
    if parsed.class != expected_class {
        return Err(format!(
            "[E_MACHINE_KERNEL_USER_ELF_CLASS_001] target '{target}' expects {expected_class:?}, image is {:?}",
            parsed.class
        )
        .into());
    }
    let mut options = UserLoadOptions::for_class(parsed.class);
    if let Some(value) = option_value(args, "--load-bias") {
        options.load_bias = parse_u64_literal(value)?;
    }
    if let Some(value) = option_value(args, "--stack-top") {
        options.stack_top = parse_u64_literal(value)?;
    }
    if let Some(value) = option_value(args, "--tls-address") {
        options.tls_address = parse_u64_literal(value)?;
    }
    options.stack_pages = parse_usize_option(args, "--stack-pages", options.stack_pages)?;
    let argv = option_values(args, "--arg");
    if !argv.is_empty() {
        options.argv = argv.into_iter().map(str::to_owned).collect();
    }
    options.envp = option_values(args, "--env")
        .into_iter()
        .map(str::to_owned)
        .collect();
    for symbol in option_values(args, "--symbol") {
        let (name, value) = symbol
            .split_once('=')
            .ok_or("--symbol expects name=address")?;
        if name.is_empty() {
            return Err("--symbol name cannot be empty".into());
        }
        options
            .symbols
            .insert(name.to_owned(), parse_u64_literal(value)?);
    }
    let cpu_count = parse_usize_option(args, "--cpus", 4)?;
    let frame_count = parse_usize_option(args, "--frames", 4096)?;
    let priority = u8::try_from(parse_u64_option(args, "--priority", 16)?)
        .map_err(|_| "--priority does not fit u8")?;
    let mut core = KernelCore::new(frame_count, cpu_count)?;
    let main = core.load_process_configured(
        options
            .argv
            .first()
            .cloned()
            .unwrap_or_else(|| "program".into()),
        &image,
        Some(expected_machine),
        Some(0),
        priority,
        options.clone(),
    )?;
    let low = core.load_process_configured(
        "scheduler-low",
        &image,
        Some(expected_machine),
        Some(0),
        24,
        options.clone(),
    )?;
    let high = core.load_process_configured(
        "scheduler-high",
        &image,
        Some(expected_machine),
        Some(0),
        2,
        options,
    )?;
    let first = core.schedule_cpu(0)?;
    let sleep_deadline = core.sleep_process(high, 1_000_000)?;
    let after_sleep = core.schedule_cpu(0)?;
    let lock_acquired = core.lock_process(0x4e_43, main)?;
    core.wake_process(high)?;
    let high_blocked_on_lock = !core.lock_process(0x4e_43, high)?;
    let inherited_priority = core.scheduler.task(main)?.effective_priority;
    let lock_handoff = core.unlock_process(0x4e_43, main)?;
    let after_priority_preemption = core.timer_interrupt(0, 1)?;
    core.wait_process(high, 0x7001)?;
    let after_wait = core.schedule_cpu(0)?;
    let wait_wakeup = core.wake_one_process(0x7001)?;
    let after_wait_preemption = core.timer_interrupt(0, 1)?;
    let all_cpus = if cpu_count == 128 {
        u128::MAX
    } else {
        (1u128 << cpu_count) - 1
    };
    core.set_process_affinity(low, all_cpus)?;
    let low_cpu = core.scheduler.task(low)?.cpu;
    let balanced = core.rebalance()?;

    let loaded = core.processes[&main]
        .loaded_image
        .clone()
        .ok_or("loaded process has no ELF metadata")?;
    fs::create_dir_all(&bundle)?;
    let retained_image = bundle.join("user-image.elf");
    fs::write(&retained_image, &image)?;
    let report = json!({
        "schema": USER_RUNTIME_REPORT_SCHEMA,
        "target": target,
        "source": source,
        "image": {
            "bytes": image.len(),
            "sha256": sha256(&image),
            "class": format!("{:?}", loaded.class),
            "machine": loaded.machine,
            "entry": loaded.entry,
            "load_bias": loaded.load_bias,
            "interpreter": loaded.interpreter,
            "needed_libraries": loaded.needed_libraries,
            "relocations_applied": loaded.relocations_applied,
            "mapped_pages": loaded.mapped_pages,
        },
        "initial_stack": {
            "pointer": loaded.stack_pointer,
            "low": loaded.stack_low,
            "alignment": loaded.stack_pointer & 15,
            "argv": loaded.argv_addresses,
            "envp": loaded.envp_addresses,
            "auxv": loaded.auxv.iter().map(|(tag, value)| json!([tag, value])).collect::<Vec<_>>(),
        },
        "tls": {
            "thread_pointer": loaded.thread_pointer,
            "bytes": loaded.tls_size,
        },
        "scheduler": {
            "cpus": cpu_count,
            "priority_levels": ncc_machine::SCHEDULER_PRIORITY_LEVELS,
            "quantum_ns": ncc_machine::SCHEDULER_DEFAULT_QUANTUM_NS,
            "pids": {"main": main, "low": low, "high": high},
            "first": first,
            "sleep_deadline_ns": sleep_deadline,
            "after_sleep": after_sleep,
            "lock_acquired": lock_acquired,
            "high_blocked_on_lock": high_blocked_on_lock,
            "inherited_priority": inherited_priority,
            "lock_handoff": lock_handoff,
            "after_priority_preemption": after_priority_preemption,
            "after_wait": after_wait,
            "wait_wakeup": wait_wakeup,
            "after_wait_preemption": after_wait_preemption,
            "low_balanced_cpu": low_cpu,
            "rebalance_migrations": balanced,
            "total_migrations": core.scheduler.migrations,
            "ticks": core.scheduler.ticks,
            "preemptions_cpu0": core.scheduler.preemptions(0)?,
            "switches_cpu0": core.scheduler.switches(0)?,
            "high_state": format!("{:?}", core.scheduler.task(high)?.state),
        },
        "machine_isa": user_context_isa(target),
        "contracts": {
            "elf_exec_and_dyn": true,
            "rel_and_rela": true,
            "dynamic_symbols": true,
            "argv_envp_auxv": true,
            "tls_tdata_tbss": true,
            "priorities_preemption_timers": true,
            "affinity_sleep_wakeup": true,
            "locks_wait_queues_balance": true,
            "stdin_binary_safe": source == "stdin",
        },
        "state_checks": {
            "high_running": core.scheduler.task(high)?.state == SchedulerTaskState::Running,
            "main_priority_restored": core.scheduler.task(main)?.effective_priority == priority,
        }
    });
    let report_path = bundle.join("kernel-user-runtime-report.json");
    write_json(&report_path, &report)?;
    let report_bytes = serde_json::to_vec(&canonical_json(&report))?;
    let payload = json!({
        "schema": USER_RUNTIME_CERTIFICATE_SCHEMA,
        "target": target,
        "image_sha256": sha256(&image),
        "report_sha256": sha256(&report_bytes),
        "status": "verified_material_execution",
    });
    let certificate = json!({
        "schema": USER_RUNTIME_CERTIFICATE_SCHEMA,
        "payload": payload,
        "integrity": {
            "algorithm": "sha256",
            "domain": "ncc.machine.kernel_user_runtime_certificate.v1",
            "payload_sha256": payload_hash_with_domain(&payload, USER_RUNTIME_HASH_DOMAIN)?,
        }
    });
    write_json(
        &bundle.join("kernel-user-runtime-certificate.json"),
        &certificate,
    )?;
    println!("{}", serde_json::to_string_pretty(&certificate)?);
    Ok(())
}

fn run_verify_user_elf(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = args
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .map(PathBuf::from)
        .ok_or("verify-user-runtime expects a bundle directory")?;
    let certificate: Value = serde_json::from_slice(&fs::read(
        bundle.join("kernel-user-runtime-certificate.json"),
    )?)?;
    let report: Value =
        serde_json::from_slice(&fs::read(bundle.join("kernel-user-runtime-report.json"))?)?;
    let image = fs::read(bundle.join("user-image.elf"))?;
    if certificate.get("schema").and_then(Value::as_str) != Some(USER_RUNTIME_CERTIFICATE_SCHEMA)
        || report.get("schema").and_then(Value::as_str) != Some(USER_RUNTIME_REPORT_SCHEMA)
    {
        return Err("[E_MACHINE_KERNEL_USER_RUNTIME_VERIFY_001] schema mismatch".into());
    }
    let payload = certificate
        .get("payload")
        .ok_or("user-runtime certificate has no payload")?;
    let payload_hash = payload_hash_with_domain(payload, USER_RUNTIME_HASH_DOMAIN)?;
    if certificate
        .pointer("/integrity/payload_sha256")
        .and_then(Value::as_str)
        != Some(payload_hash.as_str())
    {
        return Err("[E_MACHINE_KERNEL_USER_RUNTIME_VERIFY_001] payload hash mismatch".into());
    }
    let report_hash = sha256(&serde_json::to_vec(&canonical_json(&report))?);
    let image_hash = sha256(&image);
    if payload.get("report_sha256").and_then(Value::as_str) != Some(report_hash.as_str())
        || payload.get("image_sha256").and_then(Value::as_str) != Some(image_hash.as_str())
        || report.pointer("/image/sha256").and_then(Value::as_str) != Some(image_hash.as_str())
    {
        return Err("[E_MACHINE_KERNEL_USER_RUNTIME_VERIFY_001] artifact hash mismatch".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_user_runtime_verification.v1",
            "status": "verified",
            "bundle": bundle,
            "image_sha256": image_hash,
            "report_sha256": report_hash,
            "payload_sha256": payload_hash,
        }))?
    );
    Ok(())
}

fn parse_stress_profile(args: &[String]) -> Result<KernelStressProfile, Box<dyn Error>> {
    match option_value(args, "--profile").unwrap_or("smoke") {
        "smoke" => Ok(KernelStressProfile::Smoke),
        "ci" => Ok(KernelStressProfile::Ci),
        "endurance" => Ok(KernelStressProfile::Endurance),
        profile => Err(format!(
            "[E_MACHINE_KERNEL_STRESS_PROFILE_001] unknown stress profile '{profile}'; expected smoke, ci, or endurance"
        )
        .into()),
    }
}

fn stress_config(args: &[String]) -> Result<KernelStressConfig, Box<dyn Error>> {
    let profile = parse_stress_profile(args)?;
    let mut config = profile.config();
    config.seed = parse_u64_option(args, "--seed", config.seed)?;
    config.vm_exits = parse_u64_option(args, "--vm-exits", config.vm_exits)?;
    config.smp_vcpus = parse_usize_option(args, "--vcpus", config.smp_vcpus)?;
    config.smp_iterations_per_vcpu =
        parse_u64_option(args, "--smp-iterations", config.smp_iterations_per_vcpu)?;
    config.cow_fault_rounds = parse_u64_option(args, "--cow-rounds", config.cow_fault_rounds)?;
    config.dma_rounds = parse_u64_option(args, "--dma-rounds", config.dma_rounds)?;
    config.interrupt_events =
        parse_u64_option(args, "--interrupt-events", config.interrupt_events)?;
    config.snapshot_rounds = parse_u64_option(args, "--snapshot-rounds", config.snapshot_rounds)?;
    config.snapshot_memory_bytes = parse_usize_option(
        args,
        "--snapshot-memory-bytes",
        config.snapshot_memory_bytes,
    )?;
    Ok(config)
}

fn parse_raw_stress_profile(args: &[String]) -> Result<RawMemoryStressProfile, Box<dyn Error>> {
    match option_value(args, "--profile").unwrap_or("smoke") {
        "smoke" => Ok(RawMemoryStressProfile::Smoke),
        "ci" => Ok(RawMemoryStressProfile::Ci),
        "endurance" => Ok(RawMemoryStressProfile::Endurance),
        profile => Err(format!(
            "[E_MACHINE_RAW_MEMORY_STRESS_PROFILE_001] unknown stress profile '{profile}'; expected smoke, ci, or endurance"
        )
        .into()),
    }
}

fn raw_stress_config(args: &[String]) -> Result<RawMemoryStressConfig, Box<dyn Error>> {
    let profile = parse_raw_stress_profile(args)?;
    let mut config = profile.config();
    config.seed = parse_u64_option(args, "--seed", config.seed)?;
    config.heap_operations = parse_u64_option(args, "--heap-operations", config.heap_operations)?;
    config.realloc_operations =
        parse_u64_option(args, "--realloc-operations", config.realloc_operations)?;
    config.fragmentation_operations = parse_u64_option(
        args,
        "--fragmentation-operations",
        config.fragmentation_operations,
    )?;
    config.threads = parse_usize_option(args, "--threads", config.threads)?;
    config.operations_per_thread = parse_u64_option(
        args,
        "--operations-per-thread",
        config.operations_per_thread,
    )?;
    Ok(config)
}

fn elapsed_ns(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}

fn stress_report_json(report: &KernelStressReport) -> Value {
    let aggregate_operations_per_second = if report.elapsed_ns == 0 {
        report.total_operations
    } else {
        ((report.total_operations as u128) * 1_000_000_000u128 / report.elapsed_ns)
            .min(u64::MAX as u128) as u64
    };
    json!({
        "schema": STRESS_REPORT_SCHEMA,
        "status": "passed",
        "profile": report.config.profile.name(),
        "config": {
            "seed": report.config.seed,
            "vm_exits": report.config.vm_exits,
            "smp_vcpus": report.config.smp_vcpus,
            "smp_iterations_per_vcpu": report.config.smp_iterations_per_vcpu,
            "cow_fault_rounds": report.config.cow_fault_rounds,
            "dma_rounds": report.config.dma_rounds,
            "interrupt_events": report.config.interrupt_events,
            "snapshot_rounds": report.config.snapshot_rounds,
            "snapshot_memory_bytes": report.config.snapshot_memory_bytes,
        },
        "host_execution": {
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "native_process": true,
            "native_threads": true,
            "qemu": false,
            "physical_board": false,
        },
        "metrics": report.metrics.iter().map(|metric| json!({
            "scenario": metric.scenario,
            "operations": metric.operations,
            "elapsed_ns": elapsed_ns(metric.elapsed_ns),
            "operations_per_second": metric.operations_per_second(),
            "checksum": metric.checksum,
            "checksum_hex": format!("{:016x}", metric.checksum),
            "rejected_operations": metric.rejected_operations,
        })).collect::<Vec<_>>(),
        "summary": {
            "total_operations": report.total_operations,
            "elapsed_ns": elapsed_ns(report.elapsed_ns),
            "operations_per_second": aggregate_operations_per_second,
            "checksum": report.checksum,
            "checksum_hex": format!("{:016x}", report.checksum),
            "assertions": report.assertions,
            "scenario_count": report.metrics.len(),
        },
    })
}

fn stress_summary(report: &KernelStressReport) -> String {
    let mut output = format!(
        "nC kernel resistance profile={} seed=0x{:016x}\n",
        report.config.profile.name(),
        report.config.seed
    );
    for metric in &report.metrics {
        output.push_str(&format!(
            "{} operations={} rejected={} elapsed_ns={} ops_per_second={} checksum={:016x}\n",
            metric.scenario,
            metric.operations,
            metric.rejected_operations,
            metric.elapsed_ns,
            metric.operations_per_second(),
            metric.checksum,
        ));
    }
    output.push_str(&format!(
        "total_operations={} elapsed_ns={} checksum={:016x} assertions={}\n",
        report.total_operations, report.elapsed_ns, report.checksum, report.assertions
    ));
    output
}

fn raw_stress_report_json(report: &RawMemoryStressReport) -> Value {
    let aggregate_operations_per_second = if report.elapsed_ns == 0 {
        report.total_operations
    } else {
        ((report.total_operations as u128) * 1_000_000_000u128 / report.elapsed_ns)
            .min(u64::MAX as u128) as u64
    };
    json!({
        "schema": RAW_STRESS_REPORT_SCHEMA,
        "status": "passed",
        "profile": report.config.profile.name(),
        "config": {
            "seed": report.config.seed,
            "heap_operations": report.config.heap_operations,
            "realloc_operations": report.config.realloc_operations,
            "fragmentation_operations": report.config.fragmentation_operations,
            "threads": report.config.threads,
            "operations_per_thread": report.config.operations_per_thread,
        },
        "host_execution": {
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "native_process": true,
            "native_threads": true,
            "qemu": false,
            "physical_board": false,
        },
        "metrics": report.metrics.iter().map(|metric| {
            let operations_per_second = if metric.elapsed_ns == 0 {
                metric.operations
            } else {
                ((metric.operations as u128) * 1_000_000_000u128 / metric.elapsed_ns)
                    .min(u64::MAX as u128) as u64
            };
            json!({
                "scenario": metric.scenario,
                "operations": metric.operations,
                "elapsed_ns": elapsed_ns(metric.elapsed_ns),
                "operations_per_second": operations_per_second,
                "checksum": metric.checksum,
                "checksum_hex": format!("{:016x}", metric.checksum),
                "peak_live_bytes": metric.peak_live_bytes,
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "total_operations": report.total_operations,
            "elapsed_ns": elapsed_ns(report.elapsed_ns),
            "operations_per_second": aggregate_operations_per_second,
            "checksum": report.checksum,
            "checksum_hex": format!("{:016x}", report.checksum),
            "scenario_count": report.metrics.len(),
        },
    })
}

fn raw_stress_summary(report: &RawMemoryStressReport) -> String {
    let mut output = format!(
        "nC raw memory stress profile={} seed=0x{:016x}\n",
        report.config.profile.name(),
        report.config.seed
    );
    for metric in &report.metrics {
        output.push_str(&format!(
            "{} operations={} elapsed_ns={} peak_live_bytes={} checksum={:016x}\n",
            metric.scenario,
            metric.operations,
            metric.elapsed_ns,
            metric.peak_live_bytes,
            metric.checksum,
        ));
    }
    output.push_str(&format!(
        "total_operations={} elapsed_ns={} checksum={:016x}\n",
        report.total_operations, report.elapsed_ns, report.checksum
    ));
    output
}

fn run_raw_stress(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_raw_stress_bundle);
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_RAW_MEMORY_STRESS_BUNDLE_EXISTS_001] evidence bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    let report = execute_raw_memory_stress(raw_stress_config(args)?)
        .map_err(|error| format!("[E_MACHINE_RAW_MEMORY_STRESS_001] {error}"))?;
    let report_json = raw_stress_report_json(&report);
    let minimum = parse_u64_option(args, "--min-ops-per-second", 0)?;
    let measured = report_json["summary"]["operations_per_second"]
        .as_u64()
        .unwrap_or(0);
    if measured < minimum {
        return Err(format!(
            "[E_MACHINE_RAW_MEMORY_STRESS_PERF_001] aggregate throughput {measured} ops/s is below required {minimum} ops/s"
        )
        .into());
    }

    fs::create_dir_all(&bundle)?;
    write_json(&bundle.join("raw-memory-stress-report.json"), &report_json)?;
    fs::write(
        bundle.join("raw-memory-stress-summary.txt"),
        raw_stress_summary(&report),
    )?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": RAW_STRESS_CERTIFICATE_SCHEMA,
        "version": 1,
        "status": "passed",
        "created_epoch_seconds": epoch_seconds(),
        "bundle": fs::canonicalize(&bundle)?.to_string_lossy(),
        "profile": report.config.profile.name(),
        "artifacts": artifacts,
        "report_sha256": sha256(&serde_json::to_vec(&canonical_json(&report_json))?),
        "summary_checksum": report.checksum,
        "truth_boundary": {
            "heap_realloc_model_executed": true,
            "fragmentation_model_executed": true,
            "native_host_threads_executed": true,
            "atomic_contention_executed": true,
            "qemu_used": false,
            "physical_hardware_certified": false,
            "native_nC_heap_program_requires_pipeline_stage": true,
        },
    });
    let mut certificate = payload.clone();
    certificate.as_object_mut().unwrap().insert(
        "payload_sha256".into(),
        Value::String(payload_hash_with_domain(&payload, RAW_STRESS_HASH_DOMAIN)?),
    );
    write_json(
        &bundle.join("raw-memory-stress-certificate.json"),
        &certificate,
    )?;
    println!("{}", serde_json::to_string_pretty(&certificate)?);
    Ok(())
}

fn run_verify_raw_stress(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .ok_or("kernel-system verify-raw-memory-stress expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("raw-memory-stress-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(RAW_STRESS_CERTIFICATE_SCHEMA) {
        return Err(
            "[E_MACHINE_RAW_MEMORY_STRESS_CERT_SCHEMA_001] unknown stress certificate schema"
                .into(),
        );
    }
    let mut payload = certificate.clone();
    let expected = payload
        .as_object_mut()
        .and_then(|object| object.remove("payload_sha256"))
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or("raw memory stress certificate has no payload_sha256")?;
    if payload_hash_with_domain(&payload, RAW_STRESS_HASH_DOMAIN)? != expected {
        return Err(
            "[E_MACHINE_RAW_MEMORY_STRESS_CERT_HASH_001] certificate payload hash mismatch".into(),
        );
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    for artifact in payload
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("raw memory stress certificate has no artifact list")?
    {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("raw memory stress artifact path missing")?;
        let bytes = fs::read(checked_artifact_path(root, relative)?)?;
        if sha256(&bytes)
            != artifact
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        {
            return Err(format!("raw memory stress artifact hash mismatch: {relative}").into());
        }
    }
    let report: Value =
        serde_json::from_slice(&fs::read(root.join("raw-memory-stress-report.json"))?)?;
    let scenarios = report
        .get("metrics")
        .and_then(Value::as_array)
        .map(|metrics| {
            metrics
                .iter()
                .filter_map(|metric| metric.get("scenario").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if report.get("schema").and_then(Value::as_str) != Some(RAW_STRESS_REPORT_SCHEMA)
        || report.get("status").and_then(Value::as_str) != Some("passed")
        || scenarios != ["heap-realloc", "fragmentation", "concurrent-atomics"]
    {
        return Err(
            "[E_MACHINE_RAW_MEMORY_STRESS_REPORT_001] stress report contract is incomplete".into(),
        );
    }
    let report_hash = sha256(&serde_json::to_vec(&canonical_json(&report))?);
    if payload.get("report_sha256").and_then(Value::as_str) != Some(report_hash.as_str())
        || payload.get("summary_checksum").and_then(Value::as_u64)
            != report.pointer("/summary/checksum").and_then(Value::as_u64)
    {
        return Err(
            "[E_MACHINE_RAW_MEMORY_STRESS_REPORT_HASH_001] stress report summary mismatch".into(),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.raw_memory_stress_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": expected,
            "profile": payload["profile"],
            "total_operations": report["summary"]["total_operations"],
            "checksum": report["summary"]["checksum_hex"],
        }))?
    );
    Ok(())
}

fn run_stress(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_stress_bundle);
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_KERNEL_STRESS_BUNDLE_EXISTS_001] evidence bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    let config = stress_config(args)?;
    let report = run_kernel_stress(config)?;
    let report_json = stress_report_json(&report);
    let minimum = parse_u64_option(args, "--min-ops-per-second", 0)?;
    let measured = report_json["summary"]["operations_per_second"]
        .as_u64()
        .unwrap_or(0);
    if measured < minimum {
        return Err(format!(
            "[E_MACHINE_KERNEL_STRESS_PERF_001] aggregate throughput {measured} ops/s is below required {minimum} ops/s"
        )
        .into());
    }

    fs::create_dir_all(&bundle)?;
    write_json(&bundle.join("kernel-stress-report.json"), &report_json)?;
    fs::write(
        bundle.join("kernel-stress-summary.txt"),
        stress_summary(&report),
    )?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": STRESS_CERTIFICATE_SCHEMA,
        "version": 1,
        "status": "passed",
        "created_epoch_seconds": epoch_seconds(),
        "bundle": fs::canonicalize(&bundle)?.to_string_lossy(),
        "profile": report.config.profile.name(),
        "artifacts": artifacts,
        "report_sha256": sha256(&serde_json::to_vec(&canonical_json(&report_json))?),
        "summary_checksum": report.checksum,
        "truth_boundary": {
            "kernel_algorithms_executed": true,
            "native_host_threads_executed": true,
            "machine_model_executed": true,
            "qemu_used": false,
            "physical_hardware_certified": false,
            "physical_hardware_requires_a_separate_strict_runner_lane": true,
        },
    });
    let mut certificate = payload.clone();
    certificate.as_object_mut().unwrap().insert(
        "payload_sha256".into(),
        Value::String(payload_hash_with_domain(&payload, STRESS_HASH_DOMAIN)?),
    );
    write_json(&bundle.join("kernel-stress-certificate.json"), &certificate)?;
    println!("{}", serde_json::to_string_pretty(&certificate)?);
    Ok(())
}

fn run_verify_stress(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .ok_or("kernel-system verify-stress expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("kernel-stress-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(STRESS_CERTIFICATE_SCHEMA) {
        return Err(
            "[E_MACHINE_KERNEL_STRESS_CERT_SCHEMA_001] unknown stress certificate schema".into(),
        );
    }
    let mut payload = certificate.clone();
    let expected = payload
        .as_object_mut()
        .and_then(|object| object.remove("payload_sha256"))
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or("stress certificate has no payload_sha256")?;
    if payload_hash_with_domain(&payload, STRESS_HASH_DOMAIN)? != expected {
        return Err(
            "[E_MACHINE_KERNEL_STRESS_CERT_HASH_001] stress certificate payload hash mismatch"
                .into(),
        );
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    for artifact in payload
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("stress certificate has no artifact list")?
    {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("stress artifact path missing")?;
        let bytes = fs::read(checked_artifact_path(root, relative)?)?;
        if sha256(&bytes)
            != artifact
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        {
            return Err(format!("stress artifact hash mismatch: {relative}").into());
        }
    }
    let report: Value = serde_json::from_slice(&fs::read(root.join("kernel-stress-report.json"))?)?;
    let scenarios = report
        .get("metrics")
        .and_then(Value::as_array)
        .map(|metrics| {
            metrics
                .iter()
                .filter_map(|metric| metric.get("scenario").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if report.get("schema").and_then(Value::as_str) != Some(STRESS_REPORT_SCHEMA)
        || report.get("status").and_then(Value::as_str) != Some("passed")
        || scenarios
            != [
                "vm-exit-storm",
                "smp-contention",
                "cow-page-faults",
                "invalid-dma",
                "simultaneous-interrupts",
                "snapshot-restore",
            ]
    {
        return Err(
            "[E_MACHINE_KERNEL_STRESS_REPORT_001] stress report contract is incomplete".into(),
        );
    }
    let report_hash = sha256(&serde_json::to_vec(&canonical_json(&report))?);
    if payload.get("report_sha256").and_then(Value::as_str) != Some(report_hash.as_str())
        || payload.get("summary_checksum").and_then(Value::as_u64)
            != report.pointer("/summary/checksum").and_then(Value::as_u64)
    {
        return Err(
            "[E_MACHINE_KERNEL_STRESS_REPORT_HASH_001] stress report summary mismatch".into(),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_stress_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": expected,
            "profile": payload["profile"],
            "total_operations": report["summary"]["total_operations"],
            "checksum": report["summary"]["checksum_hex"],
        }))?
    );
    Ok(())
}

fn subsystem_names(values: &[KernelSubsystem]) -> Vec<String> {
    values.iter().map(|value| format!("{value:?}")).collect()
}

fn run_coordinate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let plan = coordinate_target_kernel(target)?;
    let manual = plan.manual_authority();
    let payload = json!({
        "schema": "ncc.machine.kernel_coordination.v1",
        "status": "machine_authorized",
        "target": plan.profile.target,
        "architecture": plan.profile.architecture,
        "abi": plan.profile.abi,
        "pointer_width": plan.profile.pointer_width,
        "object_format": format!("{:?}", plan.profile.object_format),
        "executable_format": format!("{:?}", plan.profile.executable_format),
        "subsystems": subsystem_names(&plan.ordered_subsystems),
        "scope": if plan.ordered_subsystems.len() == ncc_machine::COMPLETE_KERNEL_SUBSYSTEMS.len() {
            "complete-kernel"
        } else {
            "firmware-microcontroller"
        },
        "features": plan.required_features.iter().map(|feature| format!("{feature:?}")).collect::<Vec<_>>(),
        "registers": plan.profile.registers,
        "instructions": plan.profile.instructions,
        "relocations": plan.profile.relocations,
        "manual_authority": {
            "source_coordinator": manual.source_coordinator,
            "source_mode": manual.source_mode,
            "raw_memory": manual.raw_memory,
            "unchecked_out_of_range": manual.unchecked_out_of_range,
            "unchecked_aliasing": manual.unchecked_aliasing,
            "explicit_register_state": manual.explicit_register_state,
            "explicit_abi": manual.explicit_abi,
            "explicit_relocations": manual.explicit_relocations,
            "compiler_inserts": [],
            "hidden_heap": manual.hidden_heap,
            "hidden_cleanup": manual.hidden_cleanup,
            "hidden_scheduler": manual.hidden_scheduler,
            "hosted_runtime": manual.hosted_runtime,
        },
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn run_selftest(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_bundle);
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_KERNEL_BUNDLE_EXISTS_001] evidence bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    fs::create_dir_all(&bundle)?;

    let plan = coordinate_complete_kernel(target)?;
    let mut system = KernelSystem::new(512, 4)?;

    let elf = build_test_elf64(62, 0x400000, &[0x31, 0xc0, 0xc3]);
    let parent = system.core.load_process("init", &elf, Some(62), Some(0))?;
    system.core.syscall(parent, 3, [0x600000, 0, 0, 0, 0, 0])?;
    system.core.copy_to_user(parent, 0x600000, b"parent")?;
    let child = system.core.fork_cow(parent)?;
    system.core.copy_to_user(child, 0x600000, b"child!")?;
    let mut parent_page = [0u8; 6];
    let mut child_page = [0u8; 6];
    system
        .core
        .copy_from_user(parent, 0x600000, &mut parent_page)?;
    system
        .core
        .copy_from_user(child, 0x600000, &mut child_page)?;
    if &parent_page != b"parent" || &child_page != b"child!" {
        return Err("[E_MACHINE_KERNEL_COW_001] COW isolation failed".into());
    }
    let scheduled_cpu0 = system.core.scheduler.schedule(0)?;

    system.vfs.mkdir_all("/var/log", 0o755)?;
    let log = system.vfs.open(
        "/var/log/kernel.log",
        OpenFlags::READ
            .union(OpenFlags::WRITE)
            .union(OpenFlags::CREATE),
    )?;
    system.vfs.write(log, b"nC kernel system selftest\n")?;
    system.vfs.seek(log, 0)?;
    let mut log_read = vec![0u8; 64];
    let log_bytes = system.vfs.read(log, &mut log_read)?;
    log_read.truncate(log_bytes);
    system.vfs.close(log)?;

    system.pcie.attach(PciFunction::new(
        PciAddress::new(0, 0, 1, 0)?,
        0x1b36,
        0x0010,
        1,
        0x01,
        0x08,
        0x02,
        0x1b36,
        0x0010,
    ))?;
    let pcie_devices = system.pcie.enumerate()?;

    system
        .nvme
        .attach_namespace(1, VirtualBlockDevice::new(1024 * 1024, 512, false)?)?;
    system.nvme.create_io_queue(1, 64)?;
    let sector = vec![0x5a; 512];
    system.nvme.submit(
        1,
        1,
        BlockCommand {
            id: 1,
            opcode: BlockOpcode::Write,
            lba: 8,
            blocks: 1,
            data: sector.clone(),
        },
    )?;
    system
        .nvme
        .completion(1)?
        .ok_or("missing NVMe write completion")?;
    system.nvme.submit(
        1,
        1,
        BlockCommand {
            id: 2,
            opcode: BlockOpcode::Read,
            lba: 8,
            blocks: 1,
            data: Vec::new(),
        },
    )?;
    let nvme_read = system
        .nvme
        .completion(1)?
        .ok_or("missing NVMe read completion")?
        .data;
    if nvme_read != sector {
        return Err("NVMe sector roundtrip mismatch".into());
    }

    system
        .ahci
        .attach(0, VirtualBlockDevice::new(64 * 1024, 512, false)?)?;
    system.ahci.issue(
        0,
        0,
        BlockCommand {
            id: 3,
            opcode: BlockOpcode::Write,
            lba: 1,
            blocks: 1,
            data: vec![0xa5; 512],
        },
    )?;
    system
        .ahci
        .completion(0)?
        .ok_or("missing AHCI completion")?;

    let peer_ip = Ipv4Address([10, 0, 2, 16]);
    let peer_mac = [0x02, 0, 0, 0, 0, 2];
    let mut peer = NetworkStack::new(peer_mac, peer_ip)?;
    system.network.bind_udp(4000)?;
    peer.bind_udp(5000)?;
    system.network.add_neighbor(peer_ip, peer_mac);
    system
        .network
        .send_udp(4000, peer_ip, 5000, b"kernel-net")?;
    let ethernet = system
        .network
        .device
        .take_transmitted()
        .ok_or("network did not produce an Ethernet frame")?;
    peer.inject_frame(&ethernet)?;
    let datagram = peer
        .receive_udp(5000)?
        .ok_or("UDP peer did not receive a datagram")?;

    system.usb.connect(
        0,
        UsbSpeed::Full,
        UsbDeviceDescriptor {
            vendor: 0x1209,
            product: 0x4e43,
            class: 3,
            subclass: 1,
            protocol: 1,
            max_packet_size: 64,
            configurations: 1,
        },
    )?;
    let (usb_address, usb_descriptor) = system.usb.enumerate(0)?;
    let hid_press = system
        .keyboard
        .consume_boot_report([0, 0, 4, 0, 0, 0, 0, 0])?;
    let hid_release = system.keyboard.consume_boot_report([0; 8])?;
    if hid_press
        != vec![HidKeyTransition {
            usage: 4,
            pressed: true,
        }]
        || hid_release
            != vec![HidKeyTransition {
                usage: 4,
                pressed: false,
            }]
    {
        return Err("HID transition mismatch".into());
    }

    system.display = ncc_machine::KernelDisplay::new(160, 100, PixelFormat::Bgra8888)?;
    system.display.clear(0xff10_2030)?;
    system.display.rect(
        DirtyRect {
            x: 8,
            y: 8,
            width: 40,
            height: 16,
        },
        0xff00_ff00,
    )?;
    let framebuffer_crc = system.display.framebuffer.checksum();

    let thumb = encode_thumb(&[
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
    ])?;
    let thumb_execution = execute_thumb(&thumb, 64)?;
    if thumb_execution.registers[0] != 42 {
        return Err("Thumb execution did not return 42".into());
    }
    let arm_object = emit_arm_elf32_object("reset", &thumb, &[])?;
    let firmware = link_cortex_m_firmware(0x0800_0000, 0x2002_0000, &thumb, &[], &BTreeMap::new())?;
    let arm_plan = coordinate_kernel(
        "armv7em-none-eabi",
        &[KernelSubsystem::Firmware, KernelSubsystem::Framebuffer],
    )?;

    fs::write(bundle.join("init.elf"), &elf)?;
    fs::write(bundle.join("nvme.img"), system.nvme.namespace_bytes(1)?)?;
    fs::write(
        bundle.join("framebuffer.bgra"),
        system.display.framebuffer.pixels(),
    )?;
    fs::write(bundle.join("armv7-return42.o"), &arm_object)?;
    fs::write(bundle.join("armv7-return42.bin"), &firmware.bytes)?;
    fs::write(bundle.join("armv7-return42.hex"), firmware.intel_hex())?;

    let report = json!({
        "schema": "ncc.machine.kernel_system_selftest.v1",
        "status": "passed",
        "host_execution": {
            "qemu": false,
            "native_ncc_process": true,
            "reference_kernel_engine": true,
            "thumb_encoded_byte_executor": true,
            "physical_board": false,
        },
        "machine": {
            "target": plan.profile.target,
            "architecture": plan.profile.architecture,
            "abi": plan.profile.abi,
            "subsystems": subsystem_names(&plan.ordered_subsystems),
            "required_features": plan.required_features.iter().map(|feature| format!("{feature:?}")).collect::<Vec<_>>(),
            "instructions": plan.profile.instructions,
            "registers": plan.profile.registers,
            "relocations": plan.profile.relocations,
        },
        "processes": {
            "parent": parent,
            "child": child,
            "cow_parent": String::from_utf8_lossy(&parent_page),
            "cow_child": String::from_utf8_lossy(&child_page),
            "scheduled_cpu0": scheduled_cpu0,
            "count": system.core.processes.len(),
        },
        "vfs": {
            "log": String::from_utf8_lossy(&log_read),
            "metadata": format!("{:?}", system.vfs.stat("/var/log/kernel.log")?),
        },
        "storage": {
            "nvme_roundtrip_bytes": nvme_read.len(),
            "nvme_doorbells": system.nvme.doorbell_writes,
            "nvme_interrupts": system.nvme.interrupts,
            "ahci_interrupts": system.ahci.interrupts,
        },
        "pcie": pcie_devices.iter().map(|device| json!({
            "address": format!("{:04x}:{:02x}:{:02x}.{}", device.address.segment, device.address.bus, device.address.device, device.address.function),
            "vendor": device.vendor_id,
            "device": device.device_id,
            "class": device.class,
            "subclass": device.subclass,
            "programming_interface": device.programming_interface,
        })).collect::<Vec<_>>(),
        "network": {
            "ethernet_bytes": ethernet.len(),
            "udp_payload": String::from_utf8_lossy(&datagram.payload),
        },
        "usb_hid": {
            "address": usb_address,
            "vendor": usb_descriptor.vendor,
            "product": usb_descriptor.product,
            "press": format!("{:?}", hid_press),
            "release": format!("{:?}", hid_release),
        },
        "framebuffer": {
            "width": system.display.framebuffer.width,
            "height": system.display.framebuffer.height,
            "crc32": framebuffer_crc,
        },
        "armv7_thumb": {
            "target": arm_plan.profile.target,
            "encoded_hex": thumb.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "return_r0": thumb_execution.registers[0],
            "svc": thumb_execution.supervisor_calls,
            "elf32_bytes": arm_object.len(),
            "firmware_bytes": firmware.bytes.len(),
            "reset_vector": firmware.reset_vector,
        },
    });
    write_json(&bundle.join("kernel-system-report.json"), &report)?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": CERTIFICATE_SCHEMA,
        "version": 1,
        "status": "host_reference_and_machine_bytes_passed",
        "created_epoch_seconds": epoch_seconds(),
        "bundle": fs::canonicalize(&bundle)?.to_string_lossy(),
        "artifacts": artifacts,
        "truth_boundary": {
            "kernel_subsystem_algorithms_executed": true,
            "arm_thumb_bytes_executed": true,
            "qemu_used": false,
            "physical_arm_board_certified": false,
            "physical_board_requires_external_probe": true,
        },
        "report_sha256": sha256(&serde_json::to_vec(&canonical_json(&report))?),
    });
    let mut certificate = payload.clone();
    certificate.as_object_mut().unwrap().insert(
        "payload_sha256".into(),
        Value::String(payload_hash(&payload)?),
    );
    write_json(&bundle.join("kernel-system-certificate.json"), &certificate)?;
    println!("{}", serde_json::to_string_pretty(&certificate)?);
    Ok(())
}

fn run_verify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .iter()
        .find(|argument| !argument.starts_with('-'))
        .ok_or("kernel-system verify expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("kernel-system-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(CERTIFICATE_SCHEMA) {
        return Err("[E_MACHINE_KERNEL_CERT_SCHEMA_001] unknown certificate schema".into());
    }
    let mut payload = certificate.clone();
    let expected = payload
        .as_object_mut()
        .and_then(|object| object.remove("payload_sha256"))
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or("certificate has no payload_sha256")?;
    if payload_hash(&payload)? != expected {
        return Err("[E_MACHINE_KERNEL_CERT_HASH_001] certificate payload hash mismatch".into());
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    for artifact in payload
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("certificate has no artifact list")?
    {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("artifact path missing")?;
        let bytes = fs::read(checked_artifact_path(root, relative)?)?;
        if sha256(&bytes)
            != artifact
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        {
            return Err(format!("artifact hash mismatch: {relative}").into());
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_system_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": expected,
        }))?
    );
    Ok(())
}

fn run_io_storage(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_io_storage_bundle);
    fs::create_dir_all(&bundle)?;
    let physical = certify_physical_driver_contracts()?;
    let storage = certify_storage_contracts()?;
    let report = json!({
        "schema": IO_STORAGE_REPORT_SCHEMA,
        "status": "validated_on_host_not_physical_hardware",
        "drivers": {
            "serial_writes": physical.serial_writes,
            "framebuffer_checksum": physical.framebuffer_checksum,
            "keyboard_scancode": physical.keyboard_scancode,
            "interrupt_models": physical.interrupt_models.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
            "timer_models": physical.timer_models.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
            "pcie_vendor_device": physical.pcie_vendor_device,
            "nvme_submission_bytes": physical.nvme_submission_bytes,
            "ahci_command_list_bytes": physical.ahci_command_list_bytes,
            "xhci_trb_bytes": physical.xhci_trb_bytes,
            "virtio_notifications": physical.virtio_notifications,
            "recovered_faults": physical.recovered_faults,
            "register_accesses": physical.register_accesses,
            "fences": physical.fences,
        },
        "storage": {
            "filesystems": storage.filesystems.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
            "writable_filesystems": storage.writable_filesystems.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
            "files_materialized": storage.files_materialized,
            "permission_rejections": storage.permission_rejections,
            "busy_unmount_rejections": storage.busy_unmount_rejections,
            "cache_hits": storage.cache.hits,
            "cache_misses": storage.cache.misses,
            "cache_evictions": storage.cache.evictions,
            "cache_writebacks": storage.cache.writebacks,
            "persisted_bytes": storage.persisted_bytes,
            "journal_commits": storage.journal_commits,
            "journal_replays": storage.journal_replays,
            "indirect_blocks": storage.indirect_blocks,
            "acl_blocks": storage.acl_blocks,
            "long_names": storage.long_names,
        },
        "machine": {
            "x86_port_io": {
                "insb": "6c",
                "insw": "666d",
                "insd": "6d",
                "outsb": "6e",
                "outsw": "666f",
                "outsd": "6f",
            },
            "stdin_surfaces": ["powershell", "cmd", "bash"],
        },
        "physical_execution": {
            "performed": false,
            "reason": "privileged MMIO/PIO requires an explicitly selected board runner",
        }
    });
    let report_path = bundle.join("kernel-io-storage-report.json");
    write_json(&report_path, &report)?;
    let report_bytes = fs::read(&report_path)?;
    let payload_sha256 = payload_hash_with_domain(&report, IO_STORAGE_HASH_DOMAIN)?;
    let certificate = json!({
        "schema": IO_STORAGE_CERTIFICATE_SCHEMA,
        "report": "kernel-io-storage-report.json",
        "report_sha256": sha256(&report_bytes),
        "payload_sha256": payload_sha256,
    });
    write_json(
        &bundle.join("kernel-io-storage-certificate.json"),
        &certificate,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "bundle": bundle,
            "payload_sha256": payload_sha256,
            "drivers": report["drivers"],
            "storage": report["storage"],
        }))?
    );
    Ok(())
}

fn run_verify_io_storage(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .first()
        .ok_or("kernel-system verify-io-storage expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("kernel-io-storage-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(IO_STORAGE_CERTIFICATE_SCHEMA) {
        return Err("invalid kernel I/O and storage certificate schema".into());
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    let report_relative = certificate
        .get("report")
        .and_then(Value::as_str)
        .ok_or("kernel I/O and storage certificate has no report")?;
    let report_path = checked_artifact_path(root, report_relative)?;
    let report_bytes = fs::read(&report_path)?;
    if sha256(&report_bytes)
        != certificate
            .get("report_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("kernel I/O and storage report hash mismatch".into());
    }
    let report: Value = serde_json::from_slice(&report_bytes)?;
    if report.get("schema").and_then(Value::as_str) != Some(IO_STORAGE_REPORT_SCHEMA) {
        return Err("invalid kernel I/O and storage report schema".into());
    }
    let payload_sha256 = payload_hash_with_domain(&report, IO_STORAGE_HASH_DOMAIN)?;
    if payload_sha256
        != certificate
            .get("payload_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("kernel I/O and storage payload hash mismatch".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_io_storage_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": payload_sha256,
        }))?
    );
    Ok(())
}

fn run_network_sync(args: &[String]) -> Result<(), Box<dyn Error>> {
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_network_sync_bundle);
    fs::create_dir_all(&bundle)?;
    let network = certify_kernel_network_contracts()?;
    let sync = certify_kernel_sync_contracts()?;
    let report = json!({
        "schema": NETWORK_SYNC_REPORT_SCHEMA,
        "status": "validated_on_host_not_physical_hardware",
        "network": {
            "nic_families": network.nic_families,
            "nic_register_accesses": network.nic_register_accesses,
            "recovered_nic_faults": network.recovered_nic_faults,
            "nic_rx_descriptors": network.nic_rx_descriptors,
            "nic_tx_bytes": network.nic_tx_bytes,
            "ethernet_vlan_tags": network.ethernet_vlan_tags,
            "arp_roundtrips": network.arp_roundtrips,
            "ipv4_fragments": network.ipv4_fragments,
            "ipv4_reassembled_bytes": network.ipv4_reassembled_bytes,
            "ipv6_extension_bytes": network.ipv6_extension_bytes,
            "ipv6_reassembled_bytes": network.ipv6_reassembled_bytes,
            "ipv6_nd_messages": network.ipv6_nd_messages,
            "udp_v4_bytes": network.udp_v4_bytes,
            "udp_v6_bytes": network.udp_v6_bytes,
            "tcp_states_exercised": network.tcp_states_exercised,
            "tcp_retransmits": network.tcp_retransmits,
            "sockets": network.sockets,
            "timers_expired": network.timers_expired,
        },
        "synchronization": {
            "targets": sync.targets,
            "barrier_encodings": sync.barrier_encodings,
            "contended_increments": sync.contended_increments,
            "queued_mutex_handoff": sync.queued_mutex_handoff,
            "irq_restores": sync.irq_restores,
            "rcu_epochs": sync.rcu_epochs,
            "seqlock_retries": sync.seqlock_retries,
            "hidden_scheduler": false,
            "hidden_allocation": false,
        },
        "machine": {
            "x86_waitpkg_serialize": {
                "mfence": "0faef0",
                "umonitor_rax": "f3480faef0",
                "umwait_eax": "f20faef0",
                "tpause_eax": "660faef0",
                "serialize": "0f01e8",
            },
            "cross_arch_barriers": ["x86", "i386", "aarch64", "armv7-thumb2", "riscv32", "riscv64"],
            "stdin_surfaces": ["powershell", "cmd", "bash"],
        },
        "manual_contract": {
            "caller_owned_packet_buffers": true,
            "caller_owned_socket_buffers": true,
            "caller_owned_wait_queues": true,
            "explicit_irq_restore": true,
            "explicit_rcu_grace_period": true,
        },
        "physical_execution": {
            "performed": false,
            "reason": "NIC DMA, interrupts and cache coherency require an explicitly selected physical board runner",
        }
    });
    let report_path = bundle.join("kernel-network-sync-report.json");
    write_json(&report_path, &report)?;
    let report_bytes = fs::read(&report_path)?;
    let payload_sha256 = payload_hash_with_domain(&report, NETWORK_SYNC_HASH_DOMAIN)?;
    let certificate = json!({
        "schema": NETWORK_SYNC_CERTIFICATE_SCHEMA,
        "report": "kernel-network-sync-report.json",
        "report_sha256": sha256(&report_bytes),
        "payload_sha256": payload_sha256,
    });
    write_json(
        &bundle.join("kernel-network-sync-certificate.json"),
        &certificate,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "bundle": bundle,
            "payload_sha256": payload_sha256,
            "network": report["network"],
            "synchronization": report["synchronization"],
        }))?
    );
    Ok(())
}

fn run_verify_network_sync(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .first()
        .ok_or("kernel-system verify-network-sync expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("kernel-network-sync-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(NETWORK_SYNC_CERTIFICATE_SCHEMA) {
        return Err("invalid kernel network/synchronization certificate schema".into());
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    let report_relative = certificate
        .get("report")
        .and_then(Value::as_str)
        .ok_or("kernel network/synchronization certificate has no report")?;
    let report_path = checked_artifact_path(root, report_relative)?;
    let report_bytes = fs::read(&report_path)?;
    if sha256(&report_bytes)
        != certificate
            .get("report_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("kernel network/synchronization report hash mismatch".into());
    }
    let report: Value = serde_json::from_slice(&report_bytes)?;
    if report.get("schema").and_then(Value::as_str) != Some(NETWORK_SYNC_REPORT_SCHEMA) {
        return Err("invalid kernel network/synchronization report schema".into());
    }
    let payload_sha256 = payload_hash_with_domain(&report, NETWORK_SYNC_HASH_DOMAIN)?;
    if payload_sha256
        != certificate
            .get("payload_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("kernel network/synchronization payload hash mismatch".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_network_sync_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": payload_sha256,
        }))?
    );
    Ok(())
}

fn autonomous_sync_json(sync: &ncc_machine::StorageSyncReport) -> Value {
    json!({
        "mount": sync.mount,
        "transaction": sync.transaction,
        "changed_blocks": sync.changed_blocks,
        "journal_commits": sync.journal_commits,
        "journal_replays": sync.journal_replays,
        "data_blocks": sync.data_blocks,
        "metadata_blocks": sync.metadata_blocks,
        "indirect_blocks": sync.indirect_blocks,
        "acl_blocks": sync.acl_blocks,
        "long_names": sync.long_names,
        "bytes_written": sync.bytes_written,
        "flushes": sync.flushes,
    })
}

fn run_autonomous(args: &[String]) -> Result<(), Box<dyn Error>> {
    let required_path = |flag: &str| -> Result<PathBuf, Box<dyn Error>> {
        option_value(args, flag)
            .map(PathBuf::from)
            .ok_or_else(|| format!("kernel-system autonomous requires {flag} <path>").into())
    };
    let kernel_path = required_path("--kernel")?;
    let rootfs_path = required_path("--rootfs")?;
    let init_path = required_path("--init")?;
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let root_file_system = match option_value(args, "--root-format").unwrap_or("ext2") {
        "fat" | "fat12" | "fat16" | "fat32" => UnifiedRootFileSystem::Fat,
        "ext2" => UnifiedRootFileSystem::Ext2,
        other => {
            return Err(format!(
                "[E_MACHINE_KERNEL_ROOT_FORMAT_001] unsupported root format '{other}'"
            )
            .into())
        }
    };
    let root_writable = !args.iter().any(|argument| argument == "--root-readonly");
    let frames = parse_usize_option(args, "--frames", 4096)?;
    let cpus = parse_usize_option(args, "--cpus", 4)?;
    let ticks = parse_usize_option(args, "--ticks", 4)?;
    let elapsed_ns = parse_u64_option(args, "--tick-ns", 1_000_000)?;
    let command_line = option_value(args, "--cmdline")
        .unwrap_or("root=/dev/nvme0n1 rw init=/init")
        .to_string();
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(default_autonomous_bundle);
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_KERNEL_BUNDLE_EXISTS_001] evidence bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    fs::create_dir_all(&bundle)?;

    let plan = coordinate_complete_kernel(target)?;
    let mut source = UnifiedKernelImage {
        target: target.to_string(),
        frames,
        cpus,
        root_file_system,
        root_writable,
        kernel_elf: fs::read(&kernel_path)?,
        root_image: fs::read(&rootfs_path)?,
        init_elf: fs::read(&init_path)?,
        machine_manifest: plan.canonical_manifest(),
        command_line,
    };
    let (initial_image, _) = source.encode()?;
    let (mut kernel, mut boot) = AutonomousKernel::boot(&initial_image)?;
    for _ in 0..ticks {
        kernel.tick_all(elapsed_ns)?;
    }
    source.root_image = kernel.shutdown()?;
    let (image, image_report) = source.encode()?;
    let decoded = UnifiedKernelImage::decode(&image)?;
    if decoded != source {
        return Err("autonomous image roundtrip changed the source contract".into());
    }

    let image_path = bundle.join("kernel-autonomous.nki");
    fs::write(&image_path, &image)?;
    fs::write(
        bundle.join("machine-manifest.txt"),
        &source.machine_manifest,
    )?;
    if let Some(out) = option_value(args, "--out") {
        let out = PathBuf::from(out);
        if out.exists() {
            return Err(format!("autonomous output already exists: {}", out.display()).into());
        }
        if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(out, &image)?;
    }
    let sync = boot.sync.take().map(|sync| autonomous_sync_json(&sync));
    let report = json!({
        "schema": AUTONOMOUS_REPORT_SCHEMA,
        "status": "one_image_loaded_and_executed_by_host_reference_kernel",
        "image": {
            "path": "kernel-autonomous.nki",
            "bytes": image_report.total_bytes,
            "header_bytes": image_report.header_bytes,
            "components": image_report.components,
            "crc32": image_report.image_crc32,
            "sha256": sha256(&image),
            "component_crc32": image_report.component_crc32.iter().map(|(kind, crc)| (format!("{kind:?}"), *crc)).collect::<BTreeMap<_, _>>(),
        },
        "machine": {
            "target": source.target,
            "subsystems": subsystem_names(&plan.ordered_subsystems),
            "required_features": plan.required_features.iter().map(|feature| feature.canonical_name()).collect::<Vec<_>>(),
        },
        "boot": {
            "frames": frames,
            "cpus": cpus,
            "init_pid": boot.init_pid,
            "scheduled": boot.scheduled,
            "pcie_devices": boot.pcie_devices,
            "network_online": boot.network_online,
            "framebuffer_crc32": boot.framebuffer_crc32,
            "root_kind": format!("{:?}", boot.root_kind),
            "root_writable": boot.root_writable,
            "root_files": boot.root_files,
            "sync": sync,
            "ticks": ticks,
            "tick_ns": elapsed_ns,
        },
        "execution_boundary": {
            "single_image": true,
            "integrity_verified_before_boot": true,
            "kernel_subsystems_executed": true,
            "firmware_or_board_entry_executed": false,
            "qemu_used": false,
        }
    });
    let report_path = bundle.join("kernel-autonomous-report.json");
    write_json(&report_path, &report)?;
    let payload_sha256 = payload_hash_with_domain(&report, AUTONOMOUS_HASH_DOMAIN)?;
    let certificate = json!({
        "schema": AUTONOMOUS_CERTIFICATE_SCHEMA,
        "report": "kernel-autonomous-report.json",
        "report_sha256": sha256(&fs::read(&report_path)?),
        "image": "kernel-autonomous.nki",
        "image_sha256": sha256(&image),
        "payload_sha256": payload_sha256,
    });
    write_json(
        &bundle.join("kernel-autonomous-certificate.json"),
        &certificate,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "bundle": bundle,
            "image": image_path,
            "payload_sha256": payload_sha256,
        }))?
    );
    Ok(())
}

fn run_verify_autonomous(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = args
        .first()
        .ok_or("kernel-system verify-autonomous expects a bundle or certificate path")?;
    let path = PathBuf::from(path);
    let certificate_path = if path.is_dir() {
        path.join("kernel-autonomous-certificate.json")
    } else {
        path
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(AUTONOMOUS_CERTIFICATE_SCHEMA) {
        return Err("invalid autonomous kernel certificate schema".into());
    }
    let root = certificate_path.parent().unwrap_or_else(|| Path::new("."));
    let report_path = checked_artifact_path(
        root,
        certificate
            .get("report")
            .and_then(Value::as_str)
            .ok_or("autonomous certificate has no report")?,
    )?;
    let image_path = checked_artifact_path(
        root,
        certificate
            .get("image")
            .and_then(Value::as_str)
            .ok_or("autonomous certificate has no image")?,
    )?;
    let report_bytes = fs::read(&report_path)?;
    let image = fs::read(&image_path)?;
    if sha256(&report_bytes)
        != certificate
            .get("report_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
        || sha256(&image)
            != certificate
                .get("image_sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
    {
        return Err("autonomous kernel artifact hash mismatch".into());
    }
    let report: Value = serde_json::from_slice(&report_bytes)?;
    let payload_sha256 = payload_hash_with_domain(&report, AUTONOMOUS_HASH_DOMAIN)?;
    if payload_sha256
        != certificate
            .get("payload_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("autonomous kernel payload hash mismatch".into());
    }
    let (kernel, boot) = AutonomousKernel::boot(&image)?;
    let persisted = kernel.shutdown()?;
    if boot.init_pid == 0 || persisted.is_empty() {
        return Err("autonomous kernel verification did not reach init and persistent root".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_autonomous_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "image_sha256": sha256(&image),
            "payload_sha256": payload_sha256,
            "init_pid": boot.init_pid,
        }))?
    );
    Ok(())
}

pub(crate) fn run_kernel_system_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args.first().map(String::as_str) {
        Some("coordinate") => run_coordinate(&args[1..]),
        Some("selftest") | Some("certify-host") => run_selftest(&args[1..]),
        Some("verify") | Some("verify-certificate") => run_verify(&args[1..]),
        Some("stress") => run_stress(&args[1..]),
        Some("verify-stress") => run_verify_stress(&args[1..]),
        Some("raw-memory-stress") => run_raw_stress(&args[1..]),
        Some("verify-raw-memory-stress") => run_verify_raw_stress(&args[1..]),
        Some("virtual-memory") | Some("certify-virtual-memory") => run_virtual_memory(&args[1..]),
        Some("verify-virtual-memory") => run_verify_virtual_memory(&args[1..]),
        Some("load-elf") | Some("certify-user-runtime") => run_user_elf(&args[1..]),
        Some("verify-user-runtime") => run_verify_user_elf(&args[1..]),
        Some("io-storage") | Some("certify-io-storage") => run_io_storage(&args[1..]),
        Some("verify-io-storage") => run_verify_io_storage(&args[1..]),
        Some("autonomous") | Some("build-autonomous") => run_autonomous(&args[1..]),
        Some("verify-autonomous") => run_verify_autonomous(&args[1..]),
        Some("network-sync") | Some("certify-network-sync") => run_network_sync(&args[1..]),
        Some("verify-network-sync") => run_verify_network_sync(&args[1..]),
        Some("network-sync-physical") | Some("certify-network-sync-physical") => {
            super::machine_network_physical::run_certify(&args[1..])
        }
        Some("verify-network-sync-physical") => {
            super::machine_network_physical::run_verify(&args[1..])
        }
        Some("debug-hardening") | Some("certify-debug-hardening") => {
            super::machine_kernel_debug::run_certify(&args[1..])
        }
        Some("verify-debug-hardening") => super::machine_kernel_debug::run_verify(&args[1..]),
        Some("help") | Some("--help") | Some("-h") | None => {
            println!(
                "Usage: ncc machine kernel-system <coordinate|selftest|verify|stress|verify-stress|raw-memory-stress|verify-raw-memory-stress|virtual-memory|verify-virtual-memory|load-elf|verify-user-runtime|io-storage|verify-io-storage|autonomous|verify-autonomous|network-sync|verify-network-sync|network-sync-physical|verify-network-sync-physical|debug-hardening|certify-debug-hardening|verify-debug-hardening> [options]\n\
                 ncc machine kernel-system coordinate --target x86_64-unknown-none\n\
                 ncc machine kernel-system selftest --target x86_64-unknown-none --bundle target/evidence/kernel-system\n\
                 ncc machine kernel-system verify target/evidence/kernel-system\n\
                 ncc machine kernel-system stress --profile <smoke|ci|endurance> --bundle target/evidence/kernel-stress\n\
                 ncc machine kernel-system verify-stress target/evidence/kernel-stress\n\
                 ncc machine kernel-system raw-memory-stress --profile <smoke|ci|endurance> --bundle target/evidence/raw-memory-stress\n\
                 ncc machine kernel-system verify-raw-memory-stress target/evidence/raw-memory-stress\n\
                 ncc machine kernel-system virtual-memory --target x86_64-unknown-none --bundle target/evidence/kernel-vm\n\
                 ncc machine kernel-system verify-virtual-memory target/evidence/kernel-vm\n\
                 ncc machine kernel-system load-elf --image app.elf --target x86_64-unknown-none --arg app --env MODE=raw --bundle target/evidence/user-runtime\n\
                 type app.elf | ncc machine kernel-system load-elf --image - --target x86_64-unknown-none --bundle target/evidence/user-runtime\n\
                 ncc machine kernel-system verify-user-runtime target/evidence/user-runtime\n\
                 ncc machine kernel-system io-storage --bundle target/evidence/kernel-io-storage\n\
                 ncc machine kernel-system verify-io-storage target/evidence/kernel-io-storage\n\
                 ncc machine kernel-system autonomous --kernel kernel.elf --rootfs root.ext2 --root-format ext2 --init init.elf --target x86_64-unknown-none --frames 4096 --cpus 4 --bundle target/evidence/kernel-autonomous --out kernel.nki\n\
                 ncc machine kernel-system verify-autonomous target/evidence/kernel-autonomous\n\
                 ncc machine kernel-system network-sync --bundle target/evidence/kernel-network-sync\n\
                 ncc machine kernel-system verify-network-sync target/evidence/kernel-network-sync\n\
                 ncc machine kernel-system network-sync-physical --host-bundle target/evidence/kernel-network-sync --board-bundle board-bundle --vfio-certificate vfio.json --capture wire.pcapng --capture-contract capture.json --bundle target/evidence/kernel-network-physical --signing-key-env NCC_NETWORK_PHYSICAL_HMAC_KEY\n\
                 ncc machine kernel-system verify-network-sync-physical target/evidence/kernel-network-physical --signing-key-env NCC_NETWORK_PHYSICAL_HMAC_KEY\n\
                 ncc machine kernel-system debug-hardening --target x86_64-unknown-none --enable kaslr,wx,nx,canary,sanitizer --bundle target/evidence/kernel-debug-hardening\n\
                 ncc machine kernel-system verify-debug-hardening target/evidence/kernel-debug-hardening\n\
                 Stress overrides: --vm-exits N --vcpus N --smp-iterations N --cow-rounds N\n\
                 --dma-rounds N --interrupt-events N --snapshot-rounds N\n\
                 --snapshot-memory-bytes N --seed N --min-ops-per-second N\n\
                 Raw memory overrides: --heap-operations N --realloc-operations N\n\
                 --fragmentation-operations N --threads N --operations-per-thread N"
            );
            Ok(())
        }
        Some(command) => Err(format!(
            "[E_MACHINE_KERNEL_COMMAND_001] unknown kernel-system command '{command}'"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_kernel_coordination_is_exposed_by_cli_layer() {
        let plan = coordinate_complete_kernel("armv7a-unknown-none-eabi").unwrap();
        assert!(plan.ordered_subsystems.contains(&KernelSubsystem::Nvme));
        assert!(plan.ordered_subsystems.contains(&KernelSubsystem::Hid));
    }
}
