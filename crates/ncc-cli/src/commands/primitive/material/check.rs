use super::*;

pub(super) fn handle_check(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive check syntax: missing source path")?;
    let (src, program) = primitive_read_program(source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    primitive_enforce_strict_qemu_certification_guard(&opts, &target_spec, &env_name)?;
    let entry =
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".to_string());
    validate_primitive_material_build_requirements(
        &program,
        Some(target_spec.id.as_str()),
        Some(env_name.as_str()),
        Some("link/x86_64_baremetal.ld"),
        Some(entry.as_str()),
    )?;
    let payload = serde_json::json!({
        "schema": "nc.primitive.check.v1",
        "source": source_path,
        "status": "ok",
        "target": target_spec.id,
        "environment": env_name,
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "OK: primitive check {} (target={} environment={})",
            source_path, payload["target"], payload["environment"]
        );
    }
    Ok(())
}
