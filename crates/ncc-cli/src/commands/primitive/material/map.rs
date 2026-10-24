use super::*;

pub(super) fn handle_map(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let source_path = rest
        .first()
        .ok_or("primitive map syntax: missing source path")?;
    let (src, program) = primitive_read_program(source_path)?;
    primitive_reject_runtime_stdout_fallbacks(&src)?;
    ncc_sema::analyze(&program).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let (target_spec, env_name) = primitive_resolve_target_spec_for_program(&program, &opts)?;
    primitive_enforce_strict_qemu_certification_guard(&opts, &target_spec, &env_name)?;
    let entry_name =
        primitive_default_entry_for_program(&program).unwrap_or_else(|| "kernelEntry".to_string());
    validate_primitive_material_build_requirements(
        &program,
        Some(target_spec.id.as_str()),
        Some(env_name.as_str()),
        Some("link/x86_64_baremetal.ld"),
        Some(entry_name.as_str()),
    )?;
    let primitive = primitive_find_container(&program)?;
    let firmware_boot_model = primitive_firmware_boot_model_json(&primitive);
    let mut anchors = Vec::new();
    let mut windows = Vec::new();
    let mut signals = Vec::new();
    let mut ports = Vec::new();
    let mut entries = Vec::new();
    for section in &primitive.sections {
        match section {
            ncc_ast::PrimitiveAtSection::AnchorBlock(s) => anchors.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::WindowBlock(s) => windows.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::SignalBlock(s) => signals.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::Port(s) => ports.push(s.name.join(".")),
            ncc_ast::PrimitiveAtSection::Entry(s) => entries.push(s.name.join(".")),
            _ => {}
        }
    }
    let directive_kinds = primitive_directive_kind_labels(&program);
    let material_semantics = primitive_material_semantic_labels(&program);
    let protocols_required: Vec<String> = primitive_collect_protocol_requirements(&program)
        .into_iter()
        .collect();
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )?;
    let event_contracts = primitive_event_contracts_to_json(
        &contract.event_contracts,
        &contract.side_effect_contracts,
    );
    let map_hash = primitive_sha256_hex_string(&serde_json::to_string(&serde_json::json!({
        "anchors": anchors.clone(),
        "windows": windows.clone(),
        "signals": signals.clone(),
        "ports": ports.clone(),
        "entries": entries.clone(),
        "protocols_required": protocols_required.clone(),
        "event_contracts": event_contracts.clone(),
    }))?);
    let payload = serde_json::json!({
        "schema": "nc.primitive.map.v1",
        "source": source_path,
        "target": target_spec.id,
        "environment": env_name,
        "environment_class": primitive_environment_class(&env_name),
        "transport": primitive_transport_for_target(&target_spec, &env_name),
        "entry": entry_name,
        "anchors": anchors,
        "windows": windows.clone(),
        "signals": signals,
        "ports": ports,
        "entries": entries,
        "mmio_windows": windows,
        "buses": [],
        "firmware_regions": firmware_boot_model.clone(),
        "unsafe_regions": [],
        "directive_kinds": directive_kinds,
        "material_semantics": material_semantics,
        "protocols_required": protocols_required,
        "event_contracts": event_contracts,
        "evidence_level": "checked",
        "evidence_source": "primitive-map",
        "real_hardware_claim": false,
        "claim": primitive_claim_for_evidence_level("checked"),
        "hash": map_hash,
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
    });
    if opts.md {
        println!("# primitive map");
        println!("- source: {}", source_path);
        println!("- target: {}", payload["target"].as_str().unwrap_or(""));
        println!(
            "- evidence_level: {}",
            payload["evidence_level"].as_str().unwrap_or("")
        );
        println!("- claim: {}", payload["claim"].as_str().unwrap_or(""));
        println!("- hash: {}", payload["hash"].as_str().unwrap_or(""));
    } else {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    }
    Ok(())
}
