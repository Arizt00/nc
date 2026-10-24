use super::*;

pub(super) fn handle_report(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let mut source_path: Option<String> = None;
    let mut md_out: Option<String> = None;
    let mut json_out: Option<String> = None;
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--md-out" => {
                md_out = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive report syntax: --md-out requires a path")?
                        .clone(),
                );
                idx += 2;
            }
            "--json-out" => {
                json_out = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive report syntax: --json-out requires a path")?
                        .clone(),
                );
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("primitive report: unknown option '{}'", other).into());
            }
            _ => {
                if source_path.is_some() {
                    return Err("primitive report syntax: expected exactly one source path".into());
                }
                source_path = Some(rest[idx].clone());
                idx += 1;
            }
        }
    }
    let source_path = source_path.ok_or("primitive report syntax: missing source path")?;
    let (src, program) = primitive_read_program(&source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    primitive_enforce_strict_qemu_certification_guard(&opts, &target_spec, &env_name)?;
    let witness = primitive_emit_witness_json(&source_path, &program, &target_spec, &env_name)?;
    let event_audit = primitive_audit_event_contracts(&witness);
    let target_label = witness["target"].as_str().unwrap_or("");
    let env_label = witness["material_environment"].as_str().unwrap_or("");
    let op_count = witness["operation_count"].as_u64().unwrap_or(0);
    let md = primitive_render_report_markdown(
        &source_path,
        target_label,
        env_label,
        op_count,
        &event_audit,
    );
    let report_json = serde_json::json!({
        "schema": "nc.primitive.report.v1",
        "source": source_path,
        "target": target_label,
        "material_environment": env_label,
        "operation_count": op_count,
        "event_audit": primitive_event_audit_to_json(&event_audit),
        "material_contract_checks": {
            "runtime_fallback_rejected": true,
            "stdout_fallback_rejected": true,
            "event_contracts_verified": event_audit.verified(),
            "non_linear_events_present": event_audit.total > 0
        },
        "witness": witness,
    });
    let md_path =
        md_out.unwrap_or_else(|| "reports/primitive/hardware_identity_uart_report.md".to_string());
    let json_path = json_out
        .unwrap_or_else(|| "reports/primitive/hardware_identity_uart_report.json".to_string());
    ensure_output_parent_dir(&md_path)?;
    ensure_output_parent_dir(&json_path)?;
    fs::write(&md_path, md.as_bytes()).map_err(|err| {
        format!(
            "E_PRIMITIVE_CLI_REPORT_001: failed to write primitive report markdown '{}': {}",
            md_path, err
        )
    })?;
    fs::write(
        &json_path,
        serde_json::to_string_pretty(&report_json)?.as_bytes(),
    )
    .map_err(|err| {
        format!(
            "E_PRIMITIVE_CLI_REPORT_001: failed to write primitive report json '{}': {}",
            json_path, err
        )
    })?;
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "md": md_path,
                "json": json_path
            }))?
        );
    } else {
        println!("OK: primitive report md={}", md_path);
        println!("OK: primitive report json={}", json_path);
    }
    Ok(())
}
