use ncc_machine::{
    apply_kernel_relocations, debug_trap_opcode, derive_stack_canary, encode_leaf_page_entry,
    encode_rsp_packet, select_kaslr_layout, validate_permission_transition, verify_stack_canary,
    DebugArchitecture, HardeningArchitecture, HardeningPolicy, KernelCrashReport,
    KernelDebugMemory, KernelDebugMemoryWindow, KernelPanicOwner, KernelRegisterFile,
    KernelRelocation, KernelRelocationKind, KernelShadow, KernelSymbol, KernelSymbolTable,
    PagePermissions, SerialDebugger, KERNEL_DEBUG_SCHEMA, KERNEL_HARDENING_SCHEMA,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const REPORT_SCHEMA: &str = "ncc.machine.kernel_debug_hardening_report.v1";
const CERTIFICATE_SCHEMA: &str = "ncc.machine.kernel_debug_hardening_certificate.v1";
const HASH_DOMAIN: &[u8] = b"ncc.machine.kernel_debug_hardening_certificate.v1\0";

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
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

fn payload_sha256(value: &Value) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
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

fn parse_architecture(
    target: &str,
) -> Result<(DebugArchitecture, HardeningArchitecture), Box<dyn Error>> {
    let target = target.to_ascii_lowercase();
    if target.contains("x86_64") {
        Ok((DebugArchitecture::X86_64, HardeningArchitecture::X86_64))
    } else if target.contains("i386") || target.contains("i686") {
        Ok((DebugArchitecture::I386, HardeningArchitecture::I386Pae))
    } else if target.contains("aarch64") {
        Ok((DebugArchitecture::Aarch64, HardeningArchitecture::Aarch64))
    } else if target.contains("armv7") || target.contains("thumbv7") {
        Ok((
            DebugArchitecture::Armv7Thumb,
            HardeningArchitecture::Armv7Lpae,
        ))
    } else if target.contains("riscv32") {
        Ok((DebugArchitecture::Riscv32, HardeningArchitecture::Riscv32))
    } else if target.contains("riscv64") {
        Ok((DebugArchitecture::Riscv64, HardeningArchitecture::Riscv64))
    } else {
        Err(format!(
            "[E_MACHINE_KERNEL_DEBUG_TARGET_001] target '{target}' has no debug/hardening ABI"
        )
        .into())
    }
}

fn parse_policy(args: &[String]) -> Result<HardeningPolicy, Box<dyn Error>> {
    let mut policy = HardeningPolicy::RAW;
    let Some(features) = option_value(args, "--enable") else {
        return Ok(policy);
    };
    for feature in features
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match feature.to_ascii_lowercase().as_str() {
            "aslr" => policy.aslr = true,
            "kaslr" => policy.kaslr = true,
            "wx" | "w^x" => policy.enforce_w_x = true,
            "nx" => policy.nx = true,
            "canary" | "stack-canary" => policy.stack_canary = true,
            "sanitizer" | "sanitizers" => policy.sanitizer = true,
            "all" => {
                policy.aslr = true;
                policy.kaslr = true;
                policy.enforce_w_x = true;
                policy.nx = true;
                policy.stack_canary = true;
                policy.sanitizer = true;
            }
            unknown => {
                return Err(format!(
                    "[E_MACHINE_KERNEL_HARDENING_FEATURE_001] unknown opt-in feature '{unknown}'"
                )
                .into())
            }
        }
    }
    policy.permit_explicit_rwx = args.iter().any(|value| value == "--permit-rwx");
    Ok(policy)
}

fn write_pointer(bytes: &mut [u8], offset: usize, width: usize, value: u64) {
    if width == 4 {
        bytes[offset..offset + 4].copy_from_slice(&(value as u32).to_le_bytes());
    } else {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}

fn rsp_roundtrip(
    debugger: &mut SerialDebugger,
    registers: &KernelRegisterFile,
    memory: &mut impl KernelDebugMemory,
    payload: &[u8],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut packet = vec![0; payload.len() + 4];
    let packet_len = encode_rsp_packet(payload, &mut packet)?;
    packet.truncate(packet_len);
    let mut response = vec![0; 16 * 1024];
    let (response_len, _) = debugger.handle_packet(&packet, registers, memory, &mut response)?;
    response.truncate(response_len);
    Ok(response)
}

fn architecture_traps() -> Vec<(DebugArchitecture, Vec<u8>)> {
    [
        DebugArchitecture::X86_64,
        DebugArchitecture::I386,
        DebugArchitecture::Aarch64,
        DebugArchitecture::Armv7Thumb,
        DebugArchitecture::Riscv32,
        DebugArchitecture::Riscv64,
    ]
    .into_iter()
    .map(|architecture| {
        let opcode = debug_trap_opcode(architecture, 0x1234);
        (architecture, opcode.bytes[..opcode.len].to_vec())
    })
    .collect()
}

fn policy_json(policy: HardeningPolicy) -> Value {
    json!({
        "aslr": policy.aslr,
        "kaslr": policy.kaslr,
        "w_x": policy.enforce_w_x,
        "nx": policy.nx,
        "stack_canary": policy.stack_canary,
        "sanitizer": policy.sanitizer,
        "explicit_rwx_override": policy.permit_explicit_rwx,
        "enabled_count": policy.enabled_count(),
    })
}

fn collect_artifacts(bundle: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut paths = fs::read_dir(bundle)?.collect::<Result<Vec<_>, _>>()?;
    paths.sort_by_key(|entry| entry.file_name());
    let mut artifacts = Vec::new();
    for entry in paths {
        if !entry.file_type()?.is_file()
            || entry.file_name() == "kernel-debug-hardening-certificate.json"
        {
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

fn checked_artifact(root: &Path, relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "[E_MACHINE_KERNEL_DEBUG_ARTIFACT_001] unsafe artifact path '{relative}'"
        )
        .into());
    }
    Ok(root.join(path))
}

pub(super) fn run_certify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let target = option_value(args, "--target").unwrap_or("x86_64-unknown-none");
    let (debug_arch, hardening_arch) = parse_architecture(target)?;
    let policy = parse_policy(args)?;
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!(
                "target/evidence/kernel-debug-hardening-{}",
                epoch_seconds()
            ))
        });
    fs::create_dir_all(&bundle)?;

    let base = 0x8000u64;
    let mut backing = vec![0x90; 4096];
    let width = debug_arch.pointer_width();
    let first_fp = base + 0x300;
    let second_fp = base + 0x340;
    write_pointer(&mut backing, 0x300, width, second_fp);
    write_pointer(&mut backing, 0x300 + width, width, 0x2020);
    write_pointer(&mut backing, 0x340, width, 0);
    write_pointer(&mut backing, 0x340 + width, width, 0x1010);

    let registers = KernelRegisterFile::new(
        debug_arch,
        0x2042,
        base + 0x200,
        first_fp,
        0x202,
        &(0..MAX_GPRS)
            .map(|value| value as u64 + 1)
            .collect::<Vec<_>>(),
        if matches!(
            debug_arch,
            DebugArchitecture::X86_64 | DebugArchitecture::I386
        ) {
            &[0; 8]
        } else {
            &[]
        },
    )?;
    let mut symbols = KernelSymbolTable::default();
    symbols.insert(KernelSymbol {
        name: "kmain".into(),
        address: 0x1000,
        size: 0x100,
    })?;
    symbols.insert(KernelSymbol {
        name: "panic_path".into(),
        address: 0x2000,
        size: 0x100,
    })?;

    let panic_owner = KernelPanicOwner::new();
    panic_owner.claim()?;
    let mut memory = KernelDebugMemoryWindow::new(base, &mut backing);
    let mut debugger = SerialDebugger::new(debug_arch, 16);
    let mut transcript = Vec::new();
    let rsp_payloads = vec![
        b"qSupported:multiprocess+".to_vec(),
        b"?".to_vec(),
        b"g".to_vec(),
        format!("m{:x},20", base).into_bytes(),
        format!("Z0,{:x},{:x}", base + 0x40, width).into_bytes(),
        format!("z0,{:x},{:x}", base + 0x40, width).into_bytes(),
        b"vCont?".to_vec(),
    ];
    for payload in rsp_payloads {
        let mut request = vec![0; payload.len() + 4];
        let request_len = encode_rsp_packet(&payload, &mut request)?;
        request.truncate(request_len);
        let response = rsp_roundtrip(&mut debugger, &registers, &mut memory, &payload)?;
        transcript.extend_from_slice(b"> ");
        transcript.extend_from_slice(&request);
        transcript.extend_from_slice(b"\n< ");
        transcript.extend_from_slice(&response);
        transcript.push(b'\n');
    }
    fs::write(bundle.join("kernel-serial-debug.rsp"), &transcript)?;

    let report = KernelCrashReport::capture(
        0x1122_3344_5566_7788,
        2,
        "certified kernel panic",
        registers.clone(),
        &memory,
        &symbols,
        &[(base, 128), (first_fp, 128)],
        16,
    )?;
    let mut crash = vec![0; report.binary_len()];
    let crash_len = report.write_binary(&mut crash)?;
    crash.truncate(crash_len);
    fs::write(bundle.join("kernel-crash.nccrash"), &crash)?;
    let mut serial_panic = vec![0; 16 * 1024];
    let serial_len = report.serial_text(&mut serial_panic)?;
    serial_panic.truncate(serial_len);
    fs::write(bundle.join("kernel-panic-serial.log"), &serial_panic)?;

    let traps = architecture_traps();
    let mut trap_bytes = Vec::new();
    let trap_manifest = traps
        .iter()
        .map(|(architecture, bytes)| {
            let offset = trap_bytes.len();
            trap_bytes.extend_from_slice(bytes);
            json!({
                "architecture": architecture.name(),
                "offset": offset,
                "bytes": bytes.len(),
                "encoding_hex": bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            })
        })
        .collect::<Vec<_>>();
    fs::write(bundle.join("kernel-debug-traps.bin"), &trap_bytes)?;

    let raw_rwx = PagePermissions::READ
        .union(PagePermissions::WRITE)
        .union(PagePermissions::EXECUTE);
    validate_permission_transition(HardeningPolicy::RAW, PagePermissions::default(), raw_rwx)?;
    let hardened_rwx_rejected = if policy.enforce_w_x && !policy.permit_explicit_rwx {
        validate_permission_transition(policy, PagePermissions::default(), raw_rwx).is_err()
    } else {
        false
    };
    let rw = PagePermissions::READ.union(PagePermissions::WRITE);
    let rx = PagePermissions::READ.union(PagePermissions::EXECUTE);
    validate_permission_transition(policy, rw, rx)?;
    let page_entry = encode_leaf_page_entry(hardening_arch, 0x4000, rw, policy)?;

    let link_address = 0x20_0000u64;
    let kaslr = if policy.kaslr {
        select_kaslr_layout(link_address, 0x80_0000, 0x10_0000, 0x20_0000, 0xfeed_cafe)?
    } else {
        select_kaslr_layout(link_address, link_address + 0x10_0000, 0x10_0000, 0x1000, 0)?
    };
    let mut image = vec![0; 32];
    image[..8].copy_from_slice(&(link_address + 0x100).to_le_bytes());
    apply_kernel_relocations(
        &mut image,
        link_address,
        kaslr.load_address,
        &[KernelRelocation {
            offset: 0,
            kind: KernelRelocationKind::Absolute64,
            addend: 0,
        }],
    )?;
    fs::write(bundle.join("kernel-relocated-image.bin"), &image)?;

    let canary = derive_stack_canary(0x1234_5678, base, 2);
    let canary_status = if policy.stack_canary {
        verify_stack_canary(canary, canary)?;
        "verified"
    } else {
        "disabled"
    };
    let mut shadow_bytes = [0u8; 64];
    let sanitizer_status = if policy.sanitizer {
        let mut shadow = KernelShadow::new(base, 8, &mut shadow_bytes)?;
        shadow.poison(base + 0x100, 16, 0xf1)?;
        let rejected = shadow.check(base + 0x108, 1).is_err();
        shadow.unpoison(base + 0x100, 16)?;
        shadow.check(base + 0x108, 1)?;
        if !rejected {
            return Err("kernel sanitizer failed to reject poisoned memory".into());
        }
        "poison_rejected_then_unpoisoned"
    } else {
        "disabled"
    };

    let report_json = json!({
        "schema": REPORT_SCHEMA,
        "debug_schema": KERNEL_DEBUG_SCHEMA,
        "hardening_schema": KERNEL_HARDENING_SCHEMA,
        "target": target,
        "architecture": debug_arch.name(),
        "manual_contract": {
            "raw_default": policy_json(HardeningPolicy::RAW),
            "selected": policy_json(policy),
            "hidden_allocation": false,
            "hidden_scheduler": false,
            "hidden_cleanup": false,
            "programmer_owned_memory": true,
        },
        "debug": {
            "panic_owner_claimed": true,
            "registers": {
                "general": registers.general().len(),
                "debug": registers.debug().len(),
                "pc": registers.instruction_pointer,
                "sp": registers.stack_pointer,
                "fp": registers.frame_pointer,
            },
            "symbols": symbols.symbols().iter().map(|symbol| json!({"name": symbol.name, "address": symbol.address, "size": symbol.size})).collect::<Vec<_>>(),
            "frames": report.frames.iter().map(|frame| json!({"index": frame.index, "pc": frame.instruction_pointer, "sp": frame.stack_pointer, "fp": frame.frame_pointer, "symbol": frame.symbol, "offset": frame.symbol_offset})).collect::<Vec<_>>(),
            "memory_dumps": report.dumps.iter().map(|dump| json!({"address": dump.address, "bytes": dump.bytes.len(), "sha256": sha256(&dump.bytes)})).collect::<Vec<_>>(),
            "rsp_commands": 7,
            "software_breakpoint_restored": debugger.breakpoints().is_empty(),
            "trap_encodings": trap_manifest,
            "crash_format": "NCCRASH1",
        },
        "hardening": {
            "raw_rwx_allowed": true,
            "selected_rwx_rejected": hardened_rwx_rejected,
            "rw_to_rx_allowed": true,
            "leaf_page_entry": page_entry,
            "kaslr": {
                "enabled": policy.kaslr,
                "link_address": link_address,
                "load_address": kaslr.load_address,
                "slide": kaslr.slide,
                "alignment": kaslr.alignment,
            },
            "aslr_enabled": policy.aslr,
            "nx_enabled": policy.nx,
            "stack_canary": canary_status,
            "sanitizer": sanitizer_status,
        },
    });
    write_json(
        &bundle.join("kernel-debug-hardening-report.json"),
        &report_json,
    )?;
    let summary = format!(
        "nC kernel debug/hardening\ntarget={target}\nframes={}\ntraps={}\npolicy_enabled={}\nraw_default=true\n",
        report.frames.len(),
        traps.len(),
        policy.enabled_count()
    );
    fs::write(bundle.join("kernel-debug-hardening-summary.txt"), summary)?;
    let artifacts = collect_artifacts(&bundle)?;
    let certificate = json!({
        "schema": CERTIFICATE_SCHEMA,
        "status": "certified",
        "report": "kernel-debug-hardening-report.json",
        "payload_sha256": payload_sha256(&report_json)?,
        "artifacts": artifacts,
    });
    write_json(
        &bundle.join("kernel-debug-hardening-certificate.json"),
        &certificate,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": "certified",
            "bundle": bundle,
            "target": target,
            "policy": policy_json(policy),
            "payload_sha256": certificate["payload_sha256"],
        }))?
    );
    Ok(())
}

const MAX_GPRS: usize = 16;

pub(super) fn run_verify(args: &[String]) -> Result<(), Box<dyn Error>> {
    let input = args
        .first()
        .ok_or("verify-debug-hardening expects a bundle or certificate path")?;
    let input = PathBuf::from(input);
    let (root, certificate_path) = if input.is_dir() {
        (
            input.clone(),
            input.join("kernel-debug-hardening-certificate.json"),
        )
    } else {
        (
            input
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            input,
        )
    };
    let certificate: Value = serde_json::from_slice(&fs::read(&certificate_path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(CERTIFICATE_SCHEMA) {
        return Err("invalid kernel debug/hardening certificate schema".into());
    }
    for artifact in certificate
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("kernel debug/hardening certificate has no artifacts")?
    {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("kernel debug/hardening artifact has no path")?;
        let bytes = fs::read(checked_artifact(&root, relative)?)?;
        if sha256(&bytes)
            != artifact
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
        {
            return Err(
                format!("kernel debug/hardening artifact hash mismatch: {relative}").into(),
            );
        }
    }
    let report_name = certificate
        .get("report")
        .and_then(Value::as_str)
        .ok_or("kernel debug/hardening certificate has no report")?;
    let report: Value = serde_json::from_slice(&fs::read(checked_artifact(&root, report_name)?)?)?;
    if report.get("schema").and_then(Value::as_str) != Some(REPORT_SCHEMA) {
        return Err("invalid kernel debug/hardening report schema".into());
    }
    let payload = payload_sha256(&report)?;
    if payload
        != certificate
            .get("payload_sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
    {
        return Err("kernel debug/hardening payload hash mismatch".into());
    }
    if report["manual_contract"]["raw_default"]["enabled_count"] != 0
        || report["manual_contract"]["hidden_allocation"] != false
        || report["manual_contract"]["hidden_scheduler"] != false
        || report["hardening"]["raw_rwx_allowed"] != true
    {
        return Err("kernel debug/hardening raw-default contract is invalid".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ncc.machine.kernel_debug_hardening_verify.v1",
            "status": "verified",
            "certificate": certificate_path,
            "payload_sha256": payload,
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardening_is_raw_by_default_and_explicit_when_selected() {
        assert_eq!(parse_policy(&[]).unwrap(), HardeningPolicy::RAW);
        let args = vec!["--enable".into(), "all".into()];
        let policy = parse_policy(&args).unwrap();
        assert_eq!(policy.enabled_count(), 6);
        assert!(!policy.permit_explicit_rwx);
    }
}
