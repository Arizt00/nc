use crate::primitive_cli::PrimitiveCliStandardOptions;
use crate::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(crate) fn primitive_read_program(
    path: &str,
) -> Result<(String, ncc_ast::Program), Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
    let tokens = ncc_lexer::lex(&src)?;
    let ast = ncc_parser::parse(tokens)?;
    Ok((src, ast))
}

fn primitive_detect_target_from_program(program: &ncc_ast::Program) -> Option<String> {
    for container in &program.containers {
        let Some(primitive) = container.as_primitive_at() else {
            continue;
        };
        for section in primitive.sections {
            if let ncc_ast::PrimitiveAtSection::Target(arch) = section {
                let normalized = arch.to_ascii_lowercase();
                if normalized == "x86_64" {
                    return Some("x86_64-unknown-none".to_string());
                }
                if normalized == "aarch64" {
                    return Some("aarch64-unknown-none".to_string());
                }
                if normalized == "riscv64" {
                    return Some("riscv64-unknown-none".to_string());
                }
            }
        }
    }
    None
}

fn primitive_collect_anchor_symbols(primitive: &ncc_ast::PrimitiveAtContainer) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for section in &primitive.sections {
        match section {
            ncc_ast::PrimitiveAtSection::Anchor(items) => {
                for item in items {
                    out.insert(item.clone());
                }
            }
            ncc_ast::PrimitiveAtSection::AnchorBlock(section) => {
                out.insert(section.name.join("."));
            }
            _ => {}
        }
    }
    out
}

fn primitive_sorted_symbols(
    primitive: &ncc_ast::PrimitiveAtContainer,
    kind: ncc_ast::PrimitiveDirectiveKind,
) -> Vec<String> {
    let mut values: Vec<String> = primitive_collect_directive_symbols(primitive, kind)
        .into_iter()
        .collect();
    values.sort();
    values
}

pub(crate) fn primitive_firmware_boot_model_json(
    primitive: &ncc_ast::PrimitiveAtContainer,
) -> serde_json::Value {
    use ncc_ast::PrimitiveDirectiveKind as K;

    let firmware_region = primitive_sorted_symbols(primitive, K::FirmwareRegion);
    let boot_rom_region = primitive_sorted_symbols(primitive, K::BootRomRegion);
    let bios_region = primitive_sorted_symbols(primitive, K::BiosRegion);
    let uefi_handoff = primitive_sorted_symbols(primitive, K::UefiHandoff);
    let reset_vector = primitive_sorted_symbols(primitive, K::ResetVector);
    let boot_window = primitive_sorted_symbols(primitive, K::BootWindow);
    let boot_anchor = primitive_sorted_symbols(primitive, K::BootAnchor);
    let firmware_table = primitive_sorted_symbols(primitive, K::FirmwareTable);
    let device_tree = primitive_sorted_symbols(primitive, K::DeviceTree);
    let acpi_table = primitive_sorted_symbols(primitive, K::AcpiTable);
    let microcode_state = primitive_sorted_symbols(primitive, K::MicrocodeState);
    let anchors = primitive_collect_anchor_symbols(primitive);

    let has_extended_firmware_directive = !boot_rom_region.is_empty()
        || !bios_region.is_empty()
        || !uefi_handoff.is_empty()
        || !reset_vector.is_empty()
        || !boot_window.is_empty()
        || !boot_anchor.is_empty()
        || !firmware_table.is_empty()
        || !device_tree.is_empty()
        || !acpi_table.is_empty()
        || !microcode_state.is_empty();

    let reset_vector_requires_boot_window = reset_vector.is_empty() || !boot_window.is_empty();
    let boot_window_requires_reset_vector = boot_window.is_empty() || !reset_vector.is_empty();
    let reset_vector_requires_boot_anchor = reset_vector.is_empty() || !boot_anchor.is_empty();
    let boot_anchor_resolves_to_anchor = boot_anchor.iter().all(|symbol| anchors.contains(symbol));
    let bios_requires_reset_and_window =
        bios_region.is_empty() || (!reset_vector.is_empty() && !boot_window.is_empty());
    let firmware_table_requires_platform_table =
        firmware_table.is_empty() || !device_tree.is_empty() || !acpi_table.is_empty();
    let platform_table_requires_firmware_table =
        (device_tree.is_empty() && acpi_table.is_empty()) || !firmware_table.is_empty();
    let uefi_requires_firmware_table = uefi_handoff.is_empty() || !firmware_table.is_empty();
    let microcode_requires_firmware_table =
        microcode_state.is_empty() || !firmware_table.is_empty();
    let bootrom_requires_reset_and_window =
        boot_rom_region.is_empty() || (!reset_vector.is_empty() && !boot_window.is_empty());
    let extended_requires_firmware_region =
        !has_extended_firmware_directive || !firmware_region.is_empty();

    let verified = reset_vector_requires_boot_window
        && boot_window_requires_reset_vector
        && reset_vector_requires_boot_anchor
        && boot_anchor_resolves_to_anchor
        && bios_requires_reset_and_window
        && firmware_table_requires_platform_table
        && platform_table_requires_firmware_table
        && uefi_requires_firmware_table
        && microcode_requires_firmware_table
        && bootrom_requires_reset_and_window
        && extended_requires_firmware_region;

    serde_json::json!({
        "firmwareRegion": firmware_region,
        "bootRomRegion": boot_rom_region,
        "biosRegion": bios_region,
        "uefiHandoff": uefi_handoff,
        "resetVector": reset_vector,
        "bootWindow": boot_window,
        "bootAnchor": boot_anchor,
        "firmwareTable": firmware_table,
        "deviceTree": device_tree,
        "acpiTable": acpi_table,
        "microcodeState": microcode_state,
        "anchorSymbols": anchors.into_iter().collect::<Vec<_>>(),
        "contract_checks": {
            "extended_requires_firmware_region": extended_requires_firmware_region,
            "reset_vector_requires_boot_window": reset_vector_requires_boot_window,
            "boot_window_requires_reset_vector": boot_window_requires_reset_vector,
            "reset_vector_requires_boot_anchor": reset_vector_requires_boot_anchor,
            "boot_anchor_resolves_to_anchor": boot_anchor_resolves_to_anchor,
            "bios_requires_reset_and_window": bios_requires_reset_and_window,
            "firmware_table_requires_platform_table": firmware_table_requires_platform_table,
            "platform_table_requires_firmware_table": platform_table_requires_firmware_table,
            "uefi_requires_firmware_table": uefi_requires_firmware_table,
            "microcode_requires_firmware_table": microcode_requires_firmware_table,
            "bootrom_requires_reset_and_window": bootrom_requires_reset_and_window
        },
        "verified": verified
    })
}

pub(super) fn primitive_resolve_target_spec_for_program(
    program: &ncc_ast::Program,
    opts: &PrimitiveCliStandardOptions,
) -> Result<(PrimitiveTargetSpec, String), Box<dyn std::error::Error>> {
    let target_id = if let Some(target) = opts.target.as_deref() {
        target.to_string()
    } else {
        primitive_detect_target_from_program(program)
            .unwrap_or_else(|| "x86_64-unknown-none".to_string())
    };
    let spec = load_primitive_target_spec(&target_id)?;
    let env = if let Some(value) = opts.environment.as_deref() {
        value.to_string()
    } else {
        spec.field("environment")
            .ok_or("E_PRIMITIVE_ENV_MISSING_001: primitive[@] requires an explicit material environment")?
            .to_string()
    };
    Ok((spec, env))
}

fn primitive_is_qemu_emulated_environment(target: &PrimitiveTargetSpec, environment: &str) -> bool {
    let env_lower = environment.to_ascii_lowercase();
    if env_lower.contains("qemu") || env_lower.contains("emulated") {
        return true;
    }
    if target.id.to_ascii_lowercase().contains("qemu") {
        return true;
    }
    target
        .field("environment")
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            lower.contains("qemu") || lower.contains("emulated")
        })
        .unwrap_or(false)
}

pub(super) fn primitive_enforce_strict_qemu_certification_guard(
    opts: &PrimitiveCliStandardOptions,
    target: &PrimitiveTargetSpec,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if opts.strict && primitive_is_qemu_emulated_environment(target, environment) {
        return Err(
            "E_PRIMITIVE_QEMU_NOT_REAL_HW_001: QEMU environment cannot certify this operation as physical hardware".into(),
        );
    }
    Ok(())
}

pub(crate) fn primitive_reject_runtime_stdout_fallbacks(
    src: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let lower = src.to_ascii_lowercase();
    if lower.contains("runtime.") {
        return Err("E_PRIMITIVE_RUNTIME_001: runtime is not available in primitive[@]".into());
    }
    if lower.contains("readline(") || lower.contains("readint(") {
        return Err("E_PRIMITIVE_INPUT_001: human input is forbidden in primitive[@]".into());
    }
    if lower.contains("println(") || lower.contains("stdout") || lower.contains("runtime.output") {
        return Err("E_PRIMITIVE_STDOUT_FAKE_001: stdout/runtime output cannot be used as primitive hardware output".into());
    }
    Ok(())
}

pub(crate) fn primitive_default_entry_for_program(program: &ncc_ast::Program) -> Option<String> {
    let primitive = primitive_find_container(program).ok()?;
    primitive_boot_entry_name(&primitive, None)
}

pub(crate) fn primitive_bytes_to_hex_impl(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn primitive_sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    primitive_bytes_to_hex_impl(&hasher.finalize())
}

pub(crate) fn primitive_sha256_hex_string(value: &str) -> String {
    primitive_sha256_hex_bytes(value.as_bytes())
}

pub(crate) fn primitive_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

pub(crate) fn primitive_host_label() -> String {
    if let Ok(value) = env::var("HOSTNAME") {
        let value = value.trim();
        if !value.is_empty() {
            return value.to_string();
        }
    }
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown-host".to_string())
}

pub(crate) fn primitive_environment_class(environment: &str) -> &'static str {
    let lower = environment.to_ascii_lowercase();
    if lower.contains("qemu") || lower.contains("emulated") {
        "emulated"
    } else if lower.contains("freestanding") {
        "freestanding"
    } else if lower.contains("host") || lower.contains("real") || lower.contains("privileged") {
        "host"
    } else {
        "target-declared"
    }
}

pub(crate) fn primitive_transport_for_target(
    target: &PrimitiveTargetSpec,
    environment: &str,
) -> &'static str {
    let id = target.id.to_ascii_lowercase();
    let env = environment.to_ascii_lowercase();
    if id.contains("serial") || env.contains("serial") {
        "serial"
    } else if id.contains("portio") || env.contains("port") {
        "port-io"
    } else if id.contains("mmio") || env.contains("mmio") {
        "mmio"
    } else if env.contains("qemu") || env.contains("emulated") {
        "qemu-serial"
    } else {
        "artifact"
    }
}

pub(crate) fn primitive_claim_for_evidence_level(level: &str) -> &'static str {
    match level {
        "qemu_emulated" => "qemu_emulated",
        "freestanding_built" | "artifact_generated" => "freestanding_artifact",
        "hardware_observed" => "hardware_observed",
        "hardware_verified" => "hardware_verified",
        "parsed" | "checked" | "lowered" => "compiler_validated",
        _ => "contract_only",
    }
}

pub(crate) fn primitive_event_contracts_to_json_impl(
    events: &[PrimitiveEventContract],
    side_effect_contracts: &[String],
) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for event in events {
        let subject = event.subject_path.join(".");
        let mut side_effects: std::collections::BTreeSet<String> =
            side_effect_contracts.iter().cloned().collect();
        for default_effect in match event.kind {
            PrimitiveEventContractKind::Interrupt => {
                vec![
                    "async_interrupt_state_transition",
                    "control_flow_preemption",
                ]
            }
            PrimitiveEventContractKind::DmaCompletion => {
                vec!["async_dma_memory_visibility", "coherence_boundary_update"]
            }
            PrimitiveEventContractKind::WatchdogTrip => {
                vec!["watchdog_timeout_trip", "reset_or_fault_transition"]
            }
            PrimitiveEventContractKind::MmioReadClearsFlag => {
                vec!["mmio_read_clears_status_flag", "stateful_read_side_effect"]
            }
            PrimitiveEventContractKind::PeripheralStateChange => {
                vec![
                    "peripheral_state_transition",
                    "async_device_observable_change",
                ]
            }
            PrimitiveEventContractKind::PhysicalFault => {
                vec!["physical_fault_signal", "rupture_path_required"]
            }
        } {
            side_effects.insert(default_effect.to_string());
        }
        let side_effects: Vec<String> = side_effects.into_iter().collect();
        out.push(serde_json::json!({
            "kind": event.kind.as_str(),
            "construct_kind": event.construct.as_str(),
            "nominal_event_contract": event.construct.is_nominal(),
            "async_hardware_event_construct": event.construct.is_async_nominal(),
            "material_entity": event.kind.material_entity(),
            "subject": subject,
            "async_hardware_event": event.kind.async_hardware_event(),
            "side_effects": side_effects,
            "lowering_prefix_hex": primitive_bytes_to_hex_impl(event.kind.lowering_prefix_bytes()),
            "lowering_suffix_hex": primitive_bytes_to_hex_impl(event.kind.lowering_suffix_bytes()),
            "lowering_semantics": event.kind.lowering_semantics(),
        }));
    }
    out
}

pub(crate) fn primitive_emit_witness_json(
    source_path: &str,
    program: &ncc_ast::Program,
    target: &PrimitiveTargetSpec,
    environment: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let primitive = primitive_find_container(program)?;
    let firmware_boot_model = primitive_firmware_boot_model_json(&primitive);
    let entry_name = primitive_boot_entry_name(&primitive, None).ok_or(
        "E_PRIMITIVE_WITNESS_001: primitive material operation requires a material witness",
    )?;
    let contract = primitive_extract_boot_contract(
        &primitive,
        Some(entry_name.as_str()),
        PrimitiveLoweringPolicy {
            expert_profile: true,
            allow_unlowered_sensory: true,
        },
    )?;
    let directive_kinds = primitive_directive_kind_labels(program);
    let material_semantics = primitive_material_semantic_labels(program);

    let mut ops = Vec::new();
    for op in &contract.ops {
        let lowering_bytes =
            primitive_material_op_lowering_bytes(op, contract.entry_name.as_str())?;
        let lowering_hex = primitive_bytes_to_hex(&lowering_bytes);
        let mut row = match op {
            PrimitiveMaterialOp::PortWrite { port, value, .. } => serde_json::json!({
                "kind": "port_write",
                "material_entity": "portWindow",
                "port": format!("0x{port:04X}"),
                "width": 8,
                "value": format!("0x{value:02X}"),
                "volatility": true,
                "side_effects": ["device_state_change"],
            }),
            PrimitiveMaterialOp::PortRead { port } => serde_json::json!({
                "kind": "port_read",
                "material_entity": "portWindow",
                "port": format!("0x{port:04X}"),
                "width": 8,
                "volatility": true,
                "side_effects": ["read_may_affect_device_state"],
            }),
            PrimitiveMaterialOp::MmioWrite { addr, value } => serde_json::json!({
                "kind": "mmio_write",
                "material_entity": "mmioWindow",
                "address": format!("0x{addr:08X}"),
                "width": 8,
                "value": format!("0x{value:02X}"),
                "volatility": true,
                "side_effects": ["mmio_write_side_effect"],
            }),
            PrimitiveMaterialOp::MmioRead { addr } => serde_json::json!({
                "kind": "mmio_read",
                "material_entity": "mmioWindow",
                "address": format!("0x{addr:08X}"),
                "width": 8,
                "volatility": true,
                "side_effects": ["mmio_read_side_effect"],
            }),
            PrimitiveMaterialOp::VolatileWrite { addr, value } => serde_json::json!({
                "kind": "volatile_write",
                "material_entity": "registerWindow",
                "address": format!("0x{addr:08X}"),
                "width": 8,
                "value": format!("0x{value:02X}"),
                "volatility": true,
                "side_effects": ["volatile_write_side_effect"],
            }),
            PrimitiveMaterialOp::VolatileRead { addr } => serde_json::json!({
                "kind": "volatile_read",
                "material_entity": "registerWindow",
                "address": format!("0x{addr:08X}"),
                "width": 8,
                "volatility": true,
                "side_effects": ["volatile_read_side_effect"],
            }),
            PrimitiveMaterialOp::Fence => serde_json::json!({
                "kind": "fence",
                "material_entity": "timing_barrier",
                "required_fences": ["io"],
            }),
            PrimitiveMaterialOp::CpuPause => serde_json::json!({
                "kind": "cpu_pause",
                "material_entity": "timing_contract",
            }),
            PrimitiveMaterialOp::Halt => serde_json::json!({
                "kind": "halt",
                "material_entity": "control_state",
            }),
            PrimitiveMaterialOp::HaltLoop => serde_json::json!({
                "kind": "halt_loop",
                "material_entity": "control_state",
            }),
            PrimitiveMaterialOp::SensoryPrint => serde_json::json!({
                "kind": "sensory_emit",
                "material_entity": "sensory_channel",
            }),
            protocol_op @ (PrimitiveMaterialOp::ProtocolUart { .. }
            | PrimitiveMaterialOp::ProtocolPcie { .. }
            | PrimitiveMaterialOp::ProtocolUsb { .. }
            | PrimitiveMaterialOp::ProtocolSata { .. }
            | PrimitiveMaterialOp::ProtocolNvme { .. }
            | PrimitiveMaterialOp::ProtocolI2c { .. }
            | PrimitiveMaterialOp::ProtocolSpi { .. }
            | PrimitiveMaterialOp::ProtocolEthernet { .. }
            | PrimitiveMaterialOp::ProtocolHdmi { .. }
            | PrimitiveMaterialOp::ProtocolDisplayPort { .. }) => {
                let view = primitive_protocol_op_view(protocol_op)
                    .expect("protocol view must exist for protocol material op");
                serde_json::json!({
                    "kind": primitive_material_op_kind(protocol_op),
                    "material_entity": "protocol",
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
                })
            }
            event_op @ (PrimitiveMaterialOp::LegacyEventContract { .. }
            | PrimitiveMaterialOp::PhysicalEventContract { .. }
            | PrimitiveMaterialOp::AsyncHardwareEventContract { .. }) => {
                let view =
                    primitive_event_op_view(event_op).expect("event view must exist for event op");
                serde_json::json!({
                    "kind": "event_hook",
                    "construct_kind": view.construct.as_str(),
                    "nominal_event_contract": view.construct.is_nominal(),
                    "async_hardware_event_construct": view.construct.is_async_nominal(),
                    "material_entity": view.kind.material_entity(),
                    "event_kind": view.kind.as_str(),
                    "subject": view.subject,
                    "async_hardware_event": view.kind.async_hardware_event(),
                    "lowering_prefix_hex": primitive_bytes_to_hex(view.kind.lowering_prefix_bytes()),
                    "lowering_suffix_hex": primitive_bytes_to_hex(view.kind.lowering_suffix_bytes()),
                    "lowering_semantics": view.kind.lowering_semantics(),
                })
            }
            PrimitiveMaterialOp::SideEffectContract { effect } => serde_json::json!({
                "kind": "side_effect_contract",
                "material_entity": "side_effect_contract",
                "effect": effect,
            }),
            PrimitiveMaterialOp::MaterialEntityContractHook {
                kind,
                semantic,
                symbol,
            } => {
                let profile = primitive_material_probe_profile_from_op(op).unwrap_or_else(|| {
                    primitive_material_probe_profile_for_semantic(semantic.as_str())
                });
                serde_json::json!({
                    "probe_profile": profile.profile_key(),
                    "kind": "material_entity_contract_hook",
                    "material_entity": kind.as_str(),
                    "semantic": semantic,
                    "semantic_phase": primitive_material_semantic_phase(semantic.as_str()),
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
                })
            }
        };
        if let Some(obj) = row.as_object_mut() {
            obj.insert(
                "lowering_machine_bytes_hex".to_string(),
                serde_json::json!(lowering_hex),
            );
            obj.insert(
                "lowering_machine_bytes_len".to_string(),
                serde_json::json!(lowering_bytes.len()),
            );
            obj.insert(
                "lowering_stage".to_string(),
                serde_json::json!(["primitive IR", "machine IR", "backend bytes"]),
            );
        }
        ops.push(row);
    }

    let event_contracts = primitive_event_contracts_to_json(
        &contract.event_contracts,
        &contract.side_effect_contracts,
    );

    let witness_hash = primitive_sha256_hex_string(&serde_json::to_string(&ops)?);

    Ok(serde_json::json!({
        "schema": "nc.primitive.material.witness.v1",
        "source": source_path,
        "target": target.id,
        "material_environment": environment,
        "environment": environment,
        "construct_kind": "primitive[@]",
        "entry": contract.entry_name,
        "operation_count": contract.ops.len(),
        "operations": ops,
        "event_contracts": event_contracts,
        "directive_kinds": directive_kinds,
        "material_semantics": material_semantics,
        "firmware_boot_model": firmware_boot_model,
        "capabilities": target.bool_fields,
        "protocols": target.protocols,
        "lowering_path": ["primitive[@]", "primitive IR", "machine IR", "backend", "artifact"],
        "environment_class": primitive_environment_class(environment),
        "evidence_level": "lowered",
        "evidence_source": "compiler-witness",
        "transport": primitive_transport_for_target(target, environment),
        "artifact": null,
        "hash": witness_hash,
        "timestamp": primitive_timestamp_label(),
        "host": primitive_host_label(),
        "device": null,
        "claim": primitive_claim_for_evidence_level("lowered"),
        "physical_hardware_certified": false,
        "diagnostics": [],
    }))
}

#[derive(Debug, Clone)]
pub(super) struct PrimitiveEventAudit {
    pub(super) total: usize,
    pub(super) async_count: usize,
    pub(super) kind_counts: BTreeMap<String, usize>,
    pub(super) failures: Vec<String>,
}

impl PrimitiveEventAudit {
    pub(super) fn verified(&self) -> bool {
        self.failures.is_empty()
    }
}

pub(super) fn primitive_audit_event_contracts(witness: &serde_json::Value) -> PrimitiveEventAudit {
    let mut audit = PrimitiveEventAudit {
        total: 0,
        async_count: 0,
        kind_counts: BTreeMap::new(),
        failures: Vec::new(),
    };
    let events = witness
        .get("event_contracts")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    audit.total = events.len();
    for (idx, row) in events.iter().enumerate() {
        let kind = row
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        *audit.kind_counts.entry(kind.clone()).or_insert(0) += 1;
        if row
            .get("async_hardware_event")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            audit.async_count += 1;
        }
        if row
            .get("side_effects")
            .and_then(|v| v.as_array())
            .map(|list| list.is_empty())
            .unwrap_or(true)
        {
            audit
                .failures
                .push(format!("event_contract[{}] has empty side_effects", idx));
        }
        if kind == "mmioReadClearsFlag" {
            let subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            if !subject.contains('.') {
                audit.failures.push(format!(
                    "event_contract[{}] kind=mmioReadClearsFlag must use subject '<window>.<flag>'",
                    idx
                ));
            }
        }
    }
    audit
}

pub(super) fn primitive_event_audit_to_json(audit: &PrimitiveEventAudit) -> serde_json::Value {
    serde_json::json!({
        "total": audit.total,
        "async_count": audit.async_count,
        "verified": audit.verified(),
        "kind_counts": audit.kind_counts,
        "failures": audit.failures,
    })
}

pub(super) fn primitive_render_report_markdown(
    source_path: &str,
    target: &str,
    environment: &str,
    operation_count: u64,
    audit: &PrimitiveEventAudit,
) -> String {
    let mut out = String::new();
    out.push_str("# primitive report\n\n");
    out.push_str(&format!("- source: {}\n", source_path));
    out.push_str(&format!("- target: {}\n", target));
    out.push_str(&format!("- environment: {}\n", environment));
    out.push_str(&format!("- operation_count: {}\n", operation_count));
    out.push_str(&format!("- non_linear_event_contracts: {}\n", audit.total));
    out.push_str(&format!("- async_event_contracts: {}\n", audit.async_count));
    out.push_str(&format!(
        "- event_contracts_verified: {}\n",
        audit.verified()
    ));
    out.push_str("- qemu_environment_class: emulated if run under qemu\n");
    if !audit.kind_counts.is_empty() {
        out.push_str("\n## non_linear_event_kinds\n\n");
        for (kind, count) in &audit.kind_counts {
            out.push_str(&format!("- {}: {}\n", kind, count));
        }
    }
    if !audit.failures.is_empty() {
        out.push_str("\n## event_contract_failures\n\n");
        for failure in &audit.failures {
            out.push_str(&format!("- {}\n", failure));
        }
    }
    out
}
