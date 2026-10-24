use super::*;

pub(super) fn handle_witness(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive witness syntax: missing source path")?;
    let (src, program) = primitive_read_program(source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    primitive_enforce_strict_qemu_certification_guard(&opts, &target_spec, &env_name)?;
    let witness = primitive_emit_witness_json(source_path, &program, &target_spec, &env_name)?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&witness)?);
    } else if opts.md {
        println!("# primitive witness");
        println!("- source: {}", source_path);
        println!("- target: {}", witness["target"].as_str().unwrap_or(""));
        println!(
            "- environment: {}",
            witness["material_environment"].as_str().unwrap_or("")
        );
        println!(
            "- operation_count: {}",
            witness["operation_count"].as_u64().unwrap_or(0)
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&witness)?);
    }
    Ok(())
}
