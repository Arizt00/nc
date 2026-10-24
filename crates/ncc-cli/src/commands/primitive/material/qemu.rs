use super::*;

fn primitive_qemu_serial_official(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub(super) fn handle_qemu(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let mut source_path: Option<String> = None;
    let mut expect = "nC primitive alive".to_string();
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--expect" => {
                expect = rest
                    .get(idx + 1)
                    .ok_or("primitive qemu syntax: --expect requires text")?
                    .clone();
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("primitive qemu: unknown option '{}'", other).into());
            }
            _ => {
                if source_path.is_some() {
                    return Err("primitive qemu syntax: expected exactly one source path".into());
                }
                source_path = Some(rest[idx].clone());
                idx += 1;
            }
        }
    }
    let source_path = source_path.ok_or("primitive qemu syntax: missing source path")?;
    if Command::new("qemu-system-x86_64")
        .arg("--version")
        .output()
        .is_err()
    {
        return Err("E_PRIMITIVE_CLI_QEMU_MISSING_001: qemu-system-x86_64 not found".into());
    }
    let (src, program) = primitive_read_program(&source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    if target_spec.field("arch") != Some("x86_64") {
        return Err(format!(
            "E_PRIMITIVE_QEMU_SMOKE_001: QEMU smoke currently supports x86_64 targets (got '{}')",
            target_spec.field("arch").unwrap_or("unknown")
        )
        .into());
    }
    let entry_name =
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".to_string());
    let image = build_primitive_boot_image_from_program(
        &program,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )?
    .ok_or("E_PRIMITIVE_LOWERING_UNAVAILABLE_001: primitive build failed to produce qemu image")?;
    let image_path = make_temp_path("primitive_qemu_image", "tmp.sov");
    ensure_output_parent_dir(image_path.to_string_lossy().as_ref())?;
    fs::write(&image_path, &image)?;
    let log_path = make_temp_path("primitive_qemu_serial", "serial.log");
    ensure_output_parent_dir(log_path.to_string_lossy().as_ref())?;
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-nographic")
        .arg("-serial")
        .arg(format!("file:{}", log_path.to_string_lossy()))
        .arg("-monitor")
        .arg("none")
        .arg("-no-reboot")
        .arg("-no-shutdown")
        .arg("-drive")
        .arg(format!(
            "format=raw,file={},if=floppy",
            image_path.to_string_lossy()
        ))
        .arg("-boot")
        .arg("a");
    let output = if Command::new("timeout").arg("--version").output().is_ok() {
        Command::new("timeout")
            .arg("12s")
            .arg("qemu-system-x86_64")
            .arg("-nographic")
            .arg("-serial")
            .arg(format!("file:{}", log_path.to_string_lossy()))
            .arg("-monitor")
            .arg("none")
            .arg("-no-reboot")
            .arg("-no-shutdown")
            .arg("-drive")
            .arg(format!(
                "format=raw,file={},if=floppy",
                image_path.to_string_lossy()
            ))
            .arg("-boot")
            .arg("a")
            .output()?
    } else {
        cmd.output()?
    };
    let serial = fs::read_to_string(&log_path).unwrap_or_default();
    let serial_official = primitive_qemu_serial_official(&serial);
    let expect_official = primitive_qemu_serial_official(&expect);
    if !serial_official.contains(&expect_official) {
        return Err(format!(
            "E_PRIMITIVE_QEMU_SMOKE_001: QEMU smoke test failed; expected '{}' in serial output",
            expect
        )
        .into());
    }
    let image_hash = primitive_sha256_hex_bytes(&image);
    let serial_hash = primitive_sha256_hex_string(&serial);
    let payload = serde_json::json!({
        "schema": "nc.primitive.qemu.v1",
        "status": "ok",
        "target": target_spec.id.clone(),
        "environment": env_name.clone(),
        "environment_class": "emulated",
        "transport": "qemu-serial",
        "physical_hardware_certified": false,
        "expect": expect,
        "serial_log": log_path.to_string_lossy(),
        "serial_log_hash": serial_hash,
        "artifact": image_path.to_string_lossy(),
        "artifact_hash": image_hash,
        "evidence_level": "qemu_emulated",
        "evidence_source": "qemu-serial-log",
        "claim": "qemu_emulated",
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
        "qemu_status": output.status.code(),
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("OK: primitive qemu smoke (emulated environment)");
        println!("serial_log={}", log_path.to_string_lossy());
    }
    Ok(())
}
