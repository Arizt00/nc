use super::*;

pub(super) fn handle_build(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let mut source_path: Option<String> = None;
    let mut out_path: Option<String> = None;
    let mut entry: Option<String> = None;
    let mut linker: Option<String> = None;
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--out" => {
                out_path = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive build syntax: --out requires a path")?
                        .clone(),
                );
                idx += 2;
            }
            "--entry" => {
                entry = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive build syntax: --entry requires a symbol")?
                        .clone(),
                );
                idx += 2;
            }
            "--linker" => {
                linker = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive build syntax: --linker requires a path")?
                        .clone(),
                );
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("primitive build: unknown option '{}'", other).into());
            }
            _ => {
                if source_path.is_some() {
                    return Err("primitive build syntax: expected exactly one source path".into());
                }
                source_path = Some(rest[idx].clone());
                idx += 1;
            }
        }
    }
    let source_path = source_path.ok_or("primitive build syntax: missing source path")?;
    let (src, program) = primitive_read_program(&source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    primitive_enforce_strict_qemu_certification_guard(&opts, &target_spec, &env_name)?;
    let entry_name = entry.unwrap_or_else(|| {
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".to_string())
    });
    let linker_value = linker.unwrap_or_else(|| "link/x86_64_baremetal.ld".to_string());
    validate_primitive_material_build_requirements(
        &program,
        Some(target_spec.id.as_str()),
        Some(env_name.as_str()),
        Some(linker_value.as_str()),
        Some(entry_name.as_str()),
    )
    .map_err(|err| format!("E_PRIMITIVE_CLI_BUILD_001: primitive build failed ({err})"))?;
    if target_spec.field("arch") != Some("x86_64") {
        return Err(format!(
            "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: current primitive material lowering path is only available for x86_64 targets (got '{}')",
            target_spec.field("arch").unwrap_or("unknown")
        )
        .into());
    }
    if target_spec.field("runtime").unwrap_or("none") != "none" {
        return Err(
            "E_PRIMITIVE_RUNTIME_001: runtime is not available in primitive[@] build flow".into(),
        );
    }
    let image = build_primitive_boot_image_from_program(
        &program,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )?
    .ok_or(
        "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: primitive build failed to produce a material image",
    )?;

    let out = out_path.unwrap_or_else(|| {
        let stem = PathBuf::from(&source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("primitive")
            .to_string();
        format!("target/nc/primitive/{stem}.sov")
    });
    ensure_output_parent_dir(&out)?;
    fs::write(&out, &image).map_err(|err| {
        format!(
            "E_PRIMITIVE_CLI_BUILD_001: primitive build failed to write artifact '{}': {}",
            out, err
        )
    })?;
    require_non_empty_output_file("primitive build", &out).map_err(|err| {
        format!(
            "E_PRIMITIVE_CLI_BUILD_001: primitive build failed output verification for '{}': {}",
            out, err
        )
    })?;
    let artifact_hash = primitive_sha256_hex_bytes(&image);
    if opts.json {
        let evidence_level = if env_name == "freestanding" {
            "freestanding_built"
        } else {
            "artifact_generated"
        };
        let payload = serde_json::json!({
            "schema": "nc.primitive.build.v1",
            "source": source_path.clone(),
            "target": target_spec.id.clone(),
            "environment": env_name.clone(),
            "environment_class": primitive_environment_class(&env_name),
            "transport": primitive_transport_for_target(&target_spec, &env_name),
            "entry": entry_name.clone(),
            "out": out.clone(),
            "bytes": image.len(),
            "artifact_hash": artifact_hash,
            "evidence_level": evidence_level,
            "evidence_source": "primitive-build",
            "claim": primitive_claim_for_evidence_level(evidence_level),
            "physical_hardware_certified": false,
            "timestamp": primitive_timestamp_label(),
            "host": primitive_host_label(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("OK: primitive build {} sha256={}", out, artifact_hash);
    }
    Ok(())
}
