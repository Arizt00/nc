use super::*;

fn primitive_allowlist_contains_device(device: &str) -> bool {
    let path = workspace_repo_root().join("primitive.hardware.allowlist.json");
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    raw.contains(&format!("\"{device}\""))
        || (device.starts_with("/dev/pts/") && raw.contains("\"/dev/pts/\""))
}

fn primitive_real_blocked_payload(
    source_path: &str,
    target_spec: &PrimitiveTargetSpec,
    env_name: &str,
    device: Option<&str>,
    command: &str,
    code: &str,
    message: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": format!("nc.primitive.{command}.real.v1"),
        "status": "blocked",
        "code": code,
        "message": message,
        "source": source_path,
        "target": target_spec.id,
        "environment": env_name,
        "environment_class": primitive_environment_class(env_name),
        "transport": primitive_transport_for_target(target_spec, env_name),
        "device": device,
        "device_opened": false,
        "bytes_written": 0,
        "bytes_read": 0,
        "expected_pattern": "nC primitive alive",
        "observed_pattern": null,
        "pass": false,
        "evidence_level": "checked",
        "evidence_source": format!("primitive-{command}-blocked"),
        "claim": primitive_claim_for_evidence_level("checked"),
        "physical_hardware_certified": false,
        "hash": primitive_sha256_hex_string(&format!("{source_path}|{}|{env_name}|{command}|{code}", target_spec.id)),
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
    })
}

pub(super) fn handle_run(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive run syntax: missing source path")?;
    let (src, program) = primitive_read_program(source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    let device = opts.device.as_deref();
    let payload = if let Some(device) = device {
        let device_path = PathBuf::from(device);
        if !device_path.exists() {
            primitive_real_blocked_payload(
                source_path,
                &target_spec,
                &env_name,
                Some(device),
                "run",
                "E_PRIMITIVE_REAL_HARDWARE_MISSING_001",
                "requested primitive hardware device does not exist on this host",
            )
        } else if !primitive_allowlist_contains_device(device) {
            primitive_real_blocked_payload(
                source_path,
                &target_spec,
                &env_name,
                Some(device),
                "run",
                "E_PRIMITIVE_PERMISSION_001",
                "real primitive writes require primitive.hardware.allowlist.json to name the device explicitly",
            )
        } else {
            primitive_real_blocked_payload(
                source_path,
                &target_spec,
                &env_name,
                Some(device),
                "run",
                "E_PRIMITIVE_REAL_HARDWARE_NOT_VERIFIED_001",
                "device is allowlisted, but this run was not accompanied by readback/capture evidence",
            )
        }
    } else {
        primitive_real_blocked_payload(
            source_path,
            &target_spec,
            &env_name,
            None,
            "run",
            "E_PRIMITIVE_REAL_HARDWARE_MISSING_001",
            "primitive real run requires --device and an allowlisted serial test device",
        )
    };
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "blocked: primitive run {} ({})",
            payload["code"]
                .as_str()
                .unwrap_or("E_PRIMITIVE_REAL_HARDWARE_MISSING_001"),
            payload["message"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

pub(super) fn handle_capture(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive capture syntax: missing source path")?;
    let (src, program) = primitive_read_program(source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    let device = opts.device.as_deref();
    let payload = primitive_real_blocked_payload(
        source_path,
        &target_spec,
        &env_name,
        device,
        "capture",
        if device.is_some() {
            "E_PRIMITIVE_REAL_HARDWARE_NOT_VERIFIED_001"
        } else {
            "E_PRIMITIVE_REAL_HARDWARE_MISSING_001"
        },
        "primitive capture requires an external serial log/readback artifact before hardware verification can be claimed",
    );
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "blocked: primitive capture {}",
            payload["code"]
                .as_str()
                .unwrap_or("E_PRIMITIVE_REAL_HARDWARE_MISSING_001")
        );
    }
    Ok(())
}

pub(super) fn handle_verify(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive verify syntax: missing source path")?;
    let evidence_path = opts
        .evidence_path
        .as_deref()
        .ok_or("primitive verify syntax: --evidence <path> is required")?;
    let raw = fs::read_to_string(evidence_path)?;
    let evidence: serde_json::Value = serde_json::from_str(&raw)?;
    let level = evidence
        .get("evidence_level")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let physical_claim = evidence
        .get("physical_hardware_certified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let observed = evidence
        .get("observed_pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let verified = level == "hardware_verified" && physical_claim && !observed.is_empty();
    let status = if level == "hardware_verified" && !verified {
        "blocked"
    } else {
        "ok"
    };
    let payload = serde_json::json!({
        "schema": "nc.primitive.verify.real.v1",
        "source": source_path,
        "evidence": evidence_path,
        "status": status,
        "evidence_level": level,
        "claim": evidence.get("claim").cloned().unwrap_or_else(|| serde_json::json!("contract_only")),
        "hardware_claim_classification_ok": level != "hardware_verified" || verified,
        "physical_hardware_certified": verified,
        "hash": primitive_sha256_hex_string(&raw),
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
        "diagnostics": if status == "blocked" {
            serde_json::json!([{
                "code": "E_PRIMITIVE_REAL_HARDWARE_NOT_VERIFIED_001",
                "message": "hardware_verified evidence requires observed readback/capture data"
            }])
        } else {
            serde_json::json!([])
        },
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
