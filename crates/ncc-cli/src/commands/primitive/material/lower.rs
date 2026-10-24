use super::*;

pub(super) fn handle_lower(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let mut source_path: Option<String> = None;
    let mut emit_bytes = false;
    let mut emit_artifact = false;
    let mut out_path: Option<String> = None;
    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--emit-bytes" => {
                emit_bytes = true;
                idx += 1;
            }
            "--emit-artifact" => {
                emit_artifact = true;
                idx += 1;
            }
            "--out" => {
                out_path = Some(
                    rest.get(idx + 1)
                        .ok_or("primitive lower syntax: --out requires a path")?
                        .clone(),
                );
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("primitive lower: unknown option '{}'", other).into());
            }
            _ => {
                if source_path.is_some() {
                    return Err("primitive lower syntax: expected exactly one source path".into());
                }
                source_path = Some(rest[idx].clone());
                idx += 1;
            }
        }
    }
    let source_path = source_path.ok_or("primitive lower syntax: missing source path")?;
    let (src, program) = primitive_read_program(&source_path)?;
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
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )?;
    let raw = emit_primitive_boot_sector(&contract)?;
    let lowered_ops: Vec<String> = contract
        .ops
        .iter()
        .map(|op| primitive_material_op_kind(op).to_string())
        .collect();
    let mut lowered_op_sequences = Vec::new();
    let mut protocol_contracts = Vec::new();
    let mut event_hooks = Vec::new();
    let mut material_entity_contract_hooks = Vec::new();
    let mut semantic_probe_phases: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut semantic_probe_labels: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut probe_profile_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for op in &contract.ops {
        let lowered = primitive_material_op_lowering_bytes(op, entry_name.as_str())?;
        let lowered_hex = primitive_bytes_to_hex(&lowered);
        lowered_op_sequences.push(serde_json::json!({
            "kind": primitive_material_op_kind(op),
            "machine_bytes_hex": lowered_hex.clone(),
            "machine_bytes_len": lowered.len(),
        }));
        match op {
            _ if primitive_protocol_op_view(op).is_some() => {
                let view = primitive_protocol_op_view(op)
                    .expect("protocol view must exist for protocol material op");
                protocol_contracts.push(serde_json::json!({
                    "kind": primitive_material_op_kind(op),
                    "protocol": view.protocol,
                    "lowering_mode": primitive_protocol_lowering_mode(view.protocol),
                    "transport": primitive_protocol_transport(view.protocol),
                    "protocol_marker": format!("0x{:02X}", primitive_protocol_marker_byte(view.protocol)),
                    "controller": view.controller,
                    "peripheral": view.peripheral,
                    "phy": view.phy,
                    "lane": view.lane,
                    "channel": view.channel,
                    "window": view.window,
                    "window_addr": view.window_addr.map(|addr| format!("0x{addr:08X}")),
                    "data_port": view.data_port.map(|port| format!("0x{port:04X}")),
                    "status_port": view.status_port.map(|port| format!("0x{port:04X}")),
                    "control_port": view.control_port.map(|port| format!("0x{port:04X}")),
                    "config_addr_port": view.config_addr_port.map(|port| format!("0x{port:04X}")),
                    "config_data_port": view.config_data_port.map(|port| format!("0x{port:04X}")),
                    "status_offset": view.status_offset.map(|offset| format!("0x{offset:04X}")),
                    "command_offset": view.command_offset.map(|offset| format!("0x{offset:04X}")),
                    "ready_mask": view.ready_mask.map(|mask| format!("0x{mask:02X}")),
                    "write_mask": view.write_mask.map(|mask| format!("0x{mask:02X}")),
                    "config_command": view.config_command.map(|value| format!("0x{value:02X}")),
                    "compare_value": view.compare_value.map(|value| format!("0x{value:02X}")),
                    "lowering_prefix_hex": primitive_bytes_to_hex(primitive_protocol_lowering_prefix_bytes(view.protocol)),
                    "lowering_suffix_hex": primitive_bytes_to_hex(primitive_protocol_lowering_suffix_bytes(view.protocol)),
                    "lowering_semantics": primitive_protocol_lowering_semantics(view.protocol),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            event_op @ (PrimitiveMaterialOp::LegacyEventContract { .. }
            | PrimitiveMaterialOp::PhysicalEventContract { .. }
            | PrimitiveMaterialOp::AsyncHardwareEventContract { .. }) => {
                let view =
                    primitive_event_op_view(event_op).expect("event view must exist for event op");
                event_hooks.push(serde_json::json!({
                    "construct_kind": view.construct.as_str(),
                    "nominal_event_contract": view.construct.is_nominal(),
                    "async_hardware_event_construct": view.construct.is_async_nominal(),
                    "event_kind": view.kind.as_str(),
                    "material_entity": view.kind.material_entity(),
                    "subject": view.subject,
                    "async_hardware_event": view.kind.async_hardware_event(),
                    "marker": format!("0x{:02X}", view.kind.marker_byte()),
                    "lowering_prefix_hex": primitive_bytes_to_hex(view.kind.lowering_prefix_bytes()),
                    "lowering_suffix_hex": primitive_bytes_to_hex(view.kind.lowering_suffix_bytes()),
                    "lowering_semantics": view.kind.lowering_semantics(),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            PrimitiveMaterialOp::MaterialEntityContractHook {
                kind,
                semantic,
                symbol,
            } => {
                let profile = primitive_material_probe_profile_from_op(op).unwrap_or_else(|| {
                    primitive_material_probe_profile_for_semantic(semantic.as_str())
                });
                let semantic_phase = primitive_material_semantic_phase(semantic.as_str());
                *semantic_probe_phases
                    .entry(semantic_phase.to_string())
                    .or_insert(0) += 1;
                semantic_probe_labels.insert(semantic.clone());
                *probe_profile_counts
                    .entry(profile.profile_key().to_string())
                    .or_insert(0) += 1;
                material_entity_contract_hooks.push(serde_json::json!({
                    "kind_family": kind.as_str(),
                    "probe_profile": profile.profile_key(),
                    "semantic": semantic,
                    "semantic_phase": semantic_phase,
                    "symbol": symbol,
                    "marker": format!("0x{:02X}", kind.marker_byte()),
                    "semantic_marker": format!("0x{:02X}", primitive_material_semantic_marker_byte(semantic.as_str())),
                    "lowering_mode": primitive_material_probe_semantic_lowering_mode(semantic.as_str()),
                    "required_capability": primitive_material_probe_semantic_required_capability(semantic.as_str()),
                    "unsupported_diagnostic_code": primitive_material_probe_semantic_unsupported_diag(semantic.as_str()),
                    "lowering_prefix_hex": primitive_bytes_to_hex(primitive_material_probe_semantic_prefix_bytes(semantic.as_str()).unwrap_or_else(|| kind.lowering_prefix_bytes())),
                    "lowering_suffix_hex": primitive_bytes_to_hex(primitive_material_probe_semantic_suffix_bytes(semantic.as_str()).unwrap_or_else(|| kind.lowering_suffix_bytes())),
                    "lowering_semantics": kind.lowering_semantics(),
                    "semantic_lowering_semantics": primitive_material_semantic_lowering_semantics(semantic.as_str()),
                    "machine_bytes_hex": lowered_hex,
                }));
            }
            _ => {}
        }
    }
    let event_hook_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_event_op_view(op).is_some())
        .count();
    let protocol_contract_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_protocol_op_view(op).is_some())
        .count();
    let material_entity_contract_hook_ops = contract
        .ops
        .iter()
        .filter(|op| primitive_material_probe_profile_from_op(op).is_some())
        .count();
    let directive_kinds = primitive_directive_kind_labels(&program);
    let material_semantics = primitive_material_semantic_labels(&program);
    let event_contracts = primitive_event_contracts_to_json(
        &contract.event_contracts,
        &contract.side_effect_contracts,
    );
    let event_contract_audit = primitive_audit_event_contracts(&serde_json::json!({
        "event_contracts": event_contracts.clone(),
    }));
    let artifact_hash = primitive_sha256_hex_bytes(&raw);
    let artifact_path = if emit_artifact || out_path.is_some() {
        let out = out_path.unwrap_or_else(|| {
            let stem = PathBuf::from(&source_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("primitive")
                .to_string();
            format!("target/nc/primitive/{stem}.lowered.sov")
        });
        ensure_output_parent_dir(&out)?;
        fs::write(&out, &raw)?;
        Some(out)
    } else {
        None
    };
    let artifact_generated = artifact_path.is_some();
    let evidence_level = if artifact_generated {
        "artifact_generated"
    } else {
        "lowered"
    };
    let mut payload = serde_json::json!({
        "schema": "nc.primitive.lower.v1",
        "source": source_path.clone(),
        "target": target_spec.id.clone(),
        "environment": env_name.clone(),
        "environment_class": primitive_environment_class(&env_name),
        "transport": primitive_transport_for_target(&target_spec, &env_name),
        "entry": entry_name,
        "directive_kinds": directive_kinds,
        "material_semantics": material_semantics,
        "firmware_boot_model": firmware_boot_model,
        "event_contracts": event_contracts,
        "lowered_ops": lowered_ops,
        "lowered_op_sequences": lowered_op_sequences,
        "advanced_material_lowering": {
            "protocol_contract_ops": protocol_contract_ops,
            "event_hook_ops": event_hook_ops,
            "material_entity_contract_hook_ops": material_entity_contract_hook_ops,
            "event_contracts_total": event_contract_audit.total,
            "event_contracts_async": event_contract_audit.async_count,
            "event_contracts_verified": event_contract_audit.verified(),
            "event_contract_kind_counts": event_contract_audit.kind_counts,
            "semantic_probe_phases": semantic_probe_phases,
            "probe_profile_counts": probe_profile_counts,
            "semantic_probe_labels": semantic_probe_labels.into_iter().collect::<Vec<_>>(),
            "protocol_contracts": protocol_contracts,
            "event_hooks": event_hooks,
            "material_entity_contract_hooks": material_entity_contract_hooks,
        },
        "non_linear_event_contracts_audit": primitive_event_audit_to_json(&event_contract_audit),
        "path": ["primitive[@]", "primitive IR", "machine IR", "backend", "artifact"],
        "artifact_kind": "raw_boot_image",
        "artifact_size": raw.len(),
        "artifact_path": artifact_path.clone(),
        "artifact_hash": artifact_hash.clone(),
        "backend_bytes_len": raw.len(),
        "backend_bytes_sha256": artifact_hash,
        "operations": lowered_ops,
        "side_effects": ["port_or_mmio_side_effects_require_target_contract"],
        "evidence_level": evidence_level,
        "evidence_source": "primitive-lower",
        "claim": primitive_claim_for_evidence_level(evidence_level),
        "physical_hardware_certified": false,
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
    });
    if emit_bytes {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "backend_bytes_hex".to_string(),
                serde_json::json!(primitive_bytes_to_hex(&raw)),
            );
        }
    }
    if opts.md {
        println!("# primitive lower");
        println!("- source: {}", source_path);
        println!("- target: {}", payload["target"].as_str().unwrap_or(""));
        println!(
            "- evidence_level: {}",
            payload["evidence_level"].as_str().unwrap_or("")
        );
        println!(
            "- artifact_hash: {}",
            payload["artifact_hash"].as_str().unwrap_or("")
        );
        println!(
            "- backend_bytes_len: {}",
            payload["backend_bytes_len"].as_u64().unwrap_or(0)
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    }
    Ok(())
}
