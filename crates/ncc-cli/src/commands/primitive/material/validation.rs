use super::*;
pub(crate) fn validate_primitive_material_build_requirements_impl(
    program: &ncc_ast::Program,
    target: Option<&str>,
    environment: Option<&str>,
    linker: Option<&str>,
    entry: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !program_has_primitive_container(program) {
        return Ok(());
    }

    let target = target.ok_or(
        "E_PRIMITIVE_TARGET_MISSING_001: primitive material execution requires target (--target <triple>)",
    )?;
    let target_spec = load_primitive_target_spec(target)?;
    let runtime = target_spec.field("runtime").unwrap_or_default();
    if runtime != "none" {
        return Err(format!(
            "E_PRIMITIVE_TARGET_UNSUPPORTED_001: target '{}' must declare runtime = \"none\" for primitive material execution",
            target
        )
        .into());
    }

    let declared_environment = target_spec.field("environment").ok_or(
        "E_PRIMITIVE_ENV_MISSING_001: primitive[@] requires an explicit material environment",
    )?;
    if let Some(requested_env) = environment {
        if requested_env != declared_environment {
            return Err(format!(
                "E_PRIMITIVE_ENV_UNSUPPORTED_001: current environment '{}' does not match target material environment '{}'",
                requested_env,
                declared_environment
            )
            .into());
        }
    }

    let supports_port_io = target_spec.has_capability("supports_port_io");
    let supports_mmio = target_spec.has_capability("supports_mmio");
    let supports_volatile = target_spec.has_capability("supports_volatile");
    let supports_fence = target_spec.has_capability("supports_fence");
    let supports_dma = target_spec.has_capability("supports_dma");
    let supports_cache_model = target_spec.has_capability("supports_cache_model");
    let supports_tlb_model = target_spec.has_capability("supports_tlb_model");
    let supports_pipeline_model = target_spec.has_capability("supports_pipeline_model");
    let supports_firmware_model = target_spec.has_capability("supports_firmware_model");
    let supports_electrical_model = target_spec.has_capability("supports_electrical_model");
    let supports_clock_model = target_spec.has_capability("supports_clock_model");
    let supports_edge_model = target_spec.has_capability("supports_edge_model");
    let supports_metastability_model = target_spec.has_capability("supports_metastability_model");
    let supports_thermal_model = target_spec.has_capability("supports_thermal_model");
    let supports_power_model = target_spec.has_capability("supports_power_model");
    let supports_power_gate_model =
        target_spec.has_capability_or("supports_power_gate_model", supports_power_model);
    let supports_thermal_state_model =
        target_spec.has_capability_or("supports_thermal_state_model", supports_thermal_model);
    let supports_throttle_state_model =
        target_spec.has_capability_or("supports_throttle_state_model", supports_thermal_model);
    let supports_cache_state_model =
        target_spec.has_capability_or("supports_cache_state_model", supports_cache_model);
    let supports_tlb_state_model =
        target_spec.has_capability_or("supports_tlb_state_model", supports_tlb_model);
    let supports_pipeline_state_model =
        target_spec.has_capability_or("supports_pipeline_state_model", supports_pipeline_model);
    let supports_branch_predictor_model =
        target_spec.has_capability_or("supports_branch_predictor_model", supports_pipeline_model);
    let supports_reorder_model =
        target_spec.has_capability_or("supports_reorder_model", supports_pipeline_model);
    let supports_out_of_order_model =
        target_spec.has_capability_or("supports_out_of_order_model", supports_pipeline_model);
    let supports_firmware_region_model =
        target_spec.has_capability_or("supports_firmware_region_model", supports_firmware_model);
    let supports_boot_rom_region_model =
        target_spec.has_capability_or("supports_boot_rom_region_model", supports_firmware_model);
    let supports_bios_region_model =
        target_spec.has_capability_or("supports_bios_region_model", supports_firmware_model);
    let supports_uefi_handoff_model =
        target_spec.has_capability_or("supports_uefi_handoff_model", supports_firmware_model);
    let supports_reset_vector_model =
        target_spec.has_capability_or("supports_reset_vector_model", supports_firmware_model);
    let supports_boot_window_model =
        target_spec.has_capability_or("supports_boot_window_model", supports_firmware_model);
    let supports_boot_anchor_model =
        target_spec.has_capability_or("supports_boot_anchor_model", supports_firmware_model);
    let supports_firmware_table_model =
        target_spec.has_capability_or("supports_firmware_table_model", supports_firmware_model);
    let supports_device_tree_model =
        target_spec.has_capability_or("supports_device_tree_model", supports_firmware_model);
    let supports_acpi_table_model =
        target_spec.has_capability_or("supports_acpi_table_model", supports_firmware_model);
    let supports_microcode_state_model =
        target_spec.has_capability_or("supports_microcode_state_model", supports_firmware_model);

    let required_protocols = primitive_collect_protocol_requirements(program);
    let directive_kinds = primitive_collect_directive_kinds(program);
    let all_words = primitive_collect_all_words(program);
    let has_controller_directive =
        directive_kinds.contains(&ncc_ast::PrimitiveDirectiveKind::Controller);
    let has_peripheral_directive =
        directive_kinds.contains(&ncc_ast::PrimitiveDirectiveKind::Peripheral);
    let has_phy_directive = directive_kinds.contains(&ncc_ast::PrimitiveDirectiveKind::Phy);
    let has_lane_directive = directive_kinds.contains(&ncc_ast::PrimitiveDirectiveKind::Lane);
    let has_channel_directive = directive_kinds.contains(&ncc_ast::PrimitiveDirectiveKind::Channel);
    for protocol in &required_protocols {
        if !target_spec.supports_protocol(protocol) {
            return Err(format!(
                "E_PRIMITIVE_PROTOCOL_ENV_001: protocol '{}' is not available in current material environment '{}' ({}: protocol '{}' is unsupported by target '{}')",
                protocol,
                declared_environment,
                primitive_protocol_diag_code(protocol),
                protocol,
                target
            )
            .into());
        }

        let protocol_transport = primitive_protocol_transport(protocol.as_str());
        if protocol_transport == "port_io" && !supports_port_io {
            return Err(format!(
                "E_PRIMITIVE_PORT_IO_ENV_001: port I/O is not available in this environment '{}' (E_PRIMITIVE_PORT_UNSUPPORTED_001: protocol '{}' lowering requires port I/O transport but target '{}' does not expose supports_port_io)",
                declared_environment, protocol, target
            )
            .into());
        }
        if protocol_transport == "mmio" && !supports_mmio {
            return Err(format!(
                "E_PRIMITIVE_MMIO_ENV_001: MMIO mapping is not declared for this environment '{}' (E_PRIMITIVE_MMIO_UNSUPPORTED_001: protocol '{}' lowering requires MMIO transport but target '{}' does not expose supports_mmio)",
                declared_environment, protocol, target
            )
            .into());
        }

        if primitive_protocol_requires_controller(protocol) && !has_controller_directive {
            return Err(format!(
                "E_PRIMITIVE_TARGET_MATERIAL_001: protocol '{}' requires explicit controller declaration",
                protocol
            )
            .into());
        }
        if primitive_protocol_requires_peripheral(protocol) && !has_peripheral_directive {
            return Err(format!(
                "E_PRIMITIVE_TARGET_MATERIAL_001: protocol '{}' requires explicit peripheral declaration",
                protocol
            )
            .into());
        }
        if primitive_protocol_requires_phy(protocol) && !has_phy_directive {
            return Err(format!(
                "E_PRIMITIVE_TARGET_MATERIAL_001: protocol '{}' requires explicit phy declaration",
                protocol
            )
            .into());
        }
        if primitive_protocol_requires_lane(protocol) && !has_lane_directive {
            return Err(format!(
                "E_PRIMITIVE_TARGET_MATERIAL_001: protocol '{}' requires explicit lane declaration",
                protocol
            )
            .into());
        }
        if primitive_protocol_requires_channel(protocol) && !has_channel_directive {
            return Err(format!(
                "E_PRIMITIVE_TARGET_MATERIAL_001: protocol '{}' requires explicit channel declaration",
                protocol
            )
            .into());
        }
    }

    if has_phy_directive && !has_controller_directive {
        return Err(
            "E_PRIMITIVE_TARGET_MATERIAL_001: phy declarations require explicit controller declaration"
                .into(),
        );
    }
    if has_lane_directive && !has_phy_directive {
        return Err(
            "E_PRIMITIVE_TARGET_MATERIAL_001: lane declarations require explicit phy declaration"
                .into(),
        );
    }
    if has_channel_directive && !has_controller_directive && !has_peripheral_directive {
        return Err("E_PRIMITIVE_TARGET_MATERIAL_001: channel declarations require explicit controller or peripheral declaration".into());
    }

    let mentions_sensor_surface = all_words.iter().any(|word| word.starts_with("sensor"));
    if mentions_sensor_surface && !has_peripheral_directive {
        return Err("E_PRIMITIVE_TARGET_MATERIAL_001: sensor surface requires explicit peripheral declaration".into());
    }

    let mentions_motor_surface = all_words.iter().any(|word| word.starts_with("motor"));
    if mentions_motor_surface && !has_controller_directive {
        return Err(
            "E_PRIMITIVE_TARGET_MATERIAL_001: motor surface requires explicit controller declaration"
                .into(),
        );
    }

    let mentions_radio_surface = all_words.iter().any(|word| word.starts_with("radio"));
    if mentions_radio_surface && !target_spec.has_capability("supports_rf") {
        return Err(
            "E_PRIMITIVE_TARGET_CAPABILITY_001: radio surface requires target RF capability".into(),
        );
    }

    let mentions_display_surface = all_words
        .iter()
        .any(|word| word.starts_with("screen") || word.starts_with("display"));
    let has_display_link_protocol = required_protocols
        .iter()
        .any(|protocol| matches!(protocol.as_str(), "HDMI" | "DISPLAYPORT"));
    if mentions_display_surface && !has_display_link_protocol {
        return Err(
            "E_PRIMITIVE_DISPLAY_001: display surface requires explicit HDMI/DisplayPort protocol contract"
                .into(),
        );
    }

    let uses_electrical_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Voltage
                | ncc_ast::PrimitiveDirectiveKind::Current
                | ncc_ast::PrimitiveDirectiveKind::Noise
        )
    });
    if uses_electrical_directive && !supports_electrical_model {
        return Err(
            "E_PRIMITIVE_ELECTRICAL_001: electrical property is not supported by current target model"
                .into(),
        );
    }

    let uses_clock_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Clock
                | ncc_ast::PrimitiveDirectiveKind::Cycle
                | ncc_ast::PrimitiveDirectiveKind::Latency
                | ncc_ast::PrimitiveDirectiveKind::Timeout
                | ncc_ast::PrimitiveDirectiveKind::Watchdog
        )
    });
    let uses_latency_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::Latency));
    if uses_latency_directive && !supports_clock_model {
        return Err("E_PRIMITIVE_LATENCY_001: latency contract cannot be satisfied by current target (E_PRIMITIVE_CLOCK_001: timing contract requires declared hardware clock)".into());
    }
    if uses_clock_directive && !supports_clock_model {
        return Err(
            "E_PRIMITIVE_CLOCK_001: timing contract requires declared hardware clock".into(),
        );
    }

    let uses_edge_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::Edge));
    if uses_edge_directive && !supports_edge_model {
        return Err(
            "E_PRIMITIVE_EDGE_001: edge-sensitive operation requires explicit edge contract".into(),
        );
    }

    let uses_metastability_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Metastability
                | ncc_ast::PrimitiveDirectiveKind::MetastabilityRisk
        )
    });
    if uses_metastability_directive && !supports_metastability_model {
        return Err(
            "E_PRIMITIVE_METASTABILITY_001: operation may enter metastable state without synchronization contract".into(),
        );
    }

    let uses_thermal_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Temperature
                | ncc_ast::PrimitiveDirectiveKind::ThermalState
                | ncc_ast::PrimitiveDirectiveKind::ThrottleState
                | ncc_ast::PrimitiveDirectiveKind::ThermalViolation
        )
    });
    if uses_thermal_directive && !supports_thermal_model {
        return Err("E_PRIMITIVE_THERMAL_001: thermal state violates primitive contract".into());
    }
    let uses_thermal_state_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::ThermalState));
    if uses_thermal_state_directive && !supports_thermal_state_model {
        return Err(
            "E_PRIMITIVE_THERMAL_001: thermalState lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_throttle_state_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::ThrottleState));
    if uses_throttle_state_directive && !supports_throttle_state_model {
        return Err(
            "E_PRIMITIVE_THERMAL_001: throttleState lowering is unsupported by current target model"
                .into(),
        );
    }

    let uses_power_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Power
                | ncc_ast::PrimitiveDirectiveKind::PowerGate
                | ncc_ast::PrimitiveDirectiveKind::ThrottleState
        )
    });
    if uses_power_directive && !supports_power_model {
        return Err(
            "E_PRIMITIVE_POWER_001: power state does not permit requested operation".into(),
        );
    }
    let uses_power_gate_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::PowerGate));
    if uses_power_gate_directive && !supports_power_gate_model {
        return Err(
            "E_PRIMITIVE_POWER_001: powerGate lowering is unsupported by current target model"
                .into(),
        );
    }

    let uses_interrupt_line = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::InterruptLine));
    if uses_interrupt_line && !target_spec.has_capability("supports_interrupts") {
        return Err("E_PRIMITIVE_INTERRUPT_001: hardware interrupt/event must be declared".into());
    }

    let uses_dma_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::DmaChannel));
    if uses_dma_directive && !supports_dma {
        return Err(
            "E_PRIMITIVE_DMA_001: DMA operation requires explicit memory/coherence contract".into(),
        );
    }

    let uses_firmware_model_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::FirmwareRegion
                | ncc_ast::PrimitiveDirectiveKind::BootRomRegion
                | ncc_ast::PrimitiveDirectiveKind::BiosRegion
                | ncc_ast::PrimitiveDirectiveKind::UefiHandoff
                | ncc_ast::PrimitiveDirectiveKind::ResetVector
                | ncc_ast::PrimitiveDirectiveKind::BootWindow
                | ncc_ast::PrimitiveDirectiveKind::BootAnchor
                | ncc_ast::PrimitiveDirectiveKind::FirmwareTable
                | ncc_ast::PrimitiveDirectiveKind::DeviceTree
                | ncc_ast::PrimitiveDirectiveKind::AcpiTable
                | ncc_ast::PrimitiveDirectiveKind::MicrocodeState
        )
    });

    let uses_bios_region = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::BiosRegion));
    let bios_state = target_spec.field("firmware_bios").unwrap_or("unsupported");
    if uses_bios_region && bios_state != "supported" && bios_state != "emulated" {
        return Err("E_PRIMITIVE_FIRMWARE_ENV_001: firmware/boot environment does not expose requested capability".into());
    }

    let uses_uefi_handoff = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::UefiHandoff));
    let uefi_state = target_spec.field("firmware_uefi").unwrap_or("unsupported");
    if uses_uefi_handoff && uefi_state != "supported" && uefi_state != "emulated" {
        return Err("E_PRIMITIVE_FIRMWARE_ENV_001: firmware/boot environment does not expose requested capability".into());
    }

    if uses_firmware_model_directive && !supports_firmware_model {
        return Err("E_PRIMITIVE_FIRMWARE_001: firmware region or handoff is not declared".into());
    }
    let uses_firmware_region_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::FirmwareRegion));
    if uses_firmware_region_directive && !supports_firmware_region_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: firmwareRegion lowering is unsupported by current target model"
                .into(),
        );
    }

    let uses_bootrom_region = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::BootRomRegion));
    if uses_bootrom_region && !supports_boot_rom_region_model {
        return Err(
            "E_PRIMITIVE_BOOTROM_001: bootRomRegion lowering is unsupported by current target model"
                .into(),
        );
    }
    let bootrom_state = target_spec
        .field("firmware_bootrom")
        .unwrap_or("unsupported");
    if uses_bootrom_region && bootrom_state != "supported" && bootrom_state != "emulated" {
        return Err("E_PRIMITIVE_BOOTROM_001: boot ROM operation requires boot ROM support".into());
    }
    if uses_bios_region && !supports_bios_region_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_ENV_001: biosRegion lowering is unsupported by current target model"
                .into(),
        );
    }
    if uses_uefi_handoff && !supports_uefi_handoff_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_ENV_001: uefiHandoff lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_reset_vector = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::ResetVector));
    if uses_reset_vector && !supports_reset_vector_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: resetVector lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_boot_window = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::BootWindow));
    if uses_boot_window && !supports_boot_window_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: bootWindow lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_boot_anchor = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::BootAnchor));
    if uses_boot_anchor && !supports_boot_anchor_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: bootAnchor lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_firmware_table = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::FirmwareTable));
    if uses_firmware_table && !supports_firmware_table_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: firmwareTable lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_device_tree = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::DeviceTree));
    if uses_device_tree && !supports_device_tree_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: deviceTree lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_acpi_table = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::AcpiTable));
    if uses_acpi_table && !supports_acpi_table_model {
        return Err(
            "E_PRIMITIVE_FIRMWARE_001: acpiTable lowering is unsupported by current target model"
                .into(),
        );
    }

    let uses_physical_fault_contract = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::PhysicalFault
                | ncc_ast::PrimitiveDirectiveKind::BusFault
                | ncc_ast::PrimitiveDirectiveKind::SignalLoss
                | ncc_ast::PrimitiveDirectiveKind::TimingViolation
                | ncc_ast::PrimitiveDirectiveKind::ThermalViolation
                | ncc_ast::PrimitiveDirectiveKind::MetastabilityRisk
                | ncc_ast::PrimitiveDirectiveKind::DmaViolation
                | ncc_ast::PrimitiveDirectiveKind::CacheCoherenceViolation
                | ncc_ast::PrimitiveDirectiveKind::MmioSideEffectViolation
        )
    });
    if uses_physical_fault_contract && !primitive_has_rupture_section(program) {
        return Err(
            "E_PRIMITIVE_PHYSICAL_FAULT_001: physical fault path must use rupture/dev primitive"
                .into(),
        );
    }

    if program
        .containers
        .iter()
        .flat_map(|c| c.sections.iter())
        .any(|s| matches!(s, ncc_ast::Section::Timing(_)))
        && !target_spec.has_capability("supports_interrupts")
    {
        return Err(
            "E_PRIMITIVE_TIMING_ENV_001: timing contract cannot be guaranteed in this environment"
                .into(),
        );
    }

    if !supports_dma
        && required_protocols
            .iter()
            .any(|p| matches!(p.as_str(), "ETHERNET" | "NVME" | "PCIE"))
    {
        return Err(
            "E_PRIMITIVE_DMA_001: DMA operation requires explicit memory/coherence contract".into(),
        );
    }
    let uses_cache_state_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::CacheState
                | ncc_ast::PrimitiveDirectiveKind::CacheCoherenceViolation
        )
    });
    if !supports_cache_model
        && (uses_cache_state_directive
            || all_words.iter().any(|w| w == "cache" || w == "coherence"))
    {
        return Err(
            "E_PRIMITIVE_CACHE_001: cache state/coherence must be declared for this operation"
                .into(),
        );
    }
    let uses_tlb_state_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::TlbState));
    if uses_tlb_state_directive && !supports_tlb_state_model {
        return Err(
            "E_PRIMITIVE_TLB_001: tlbState lowering is unsupported by current target model".into(),
        );
    }
    if !supports_tlb_model && (uses_tlb_state_directive || all_words.iter().any(|w| w == "tlb")) {
        return Err(
            "E_PRIMITIVE_TLB_001: TLB-dependent operation requires target TLB model".into(),
        );
    }
    if uses_cache_state_directive && !supports_cache_state_model {
        return Err(
            "E_PRIMITIVE_CACHE_001: cacheState lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_pipeline_state_directive_only = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::PipelineState));
    if uses_pipeline_state_directive_only && !supports_pipeline_state_model {
        return Err(
            "E_PRIMITIVE_TARGET_CAPABILITY_001: pipelineState lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_branch_predictor_state_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::BranchPredictorState));
    if uses_branch_predictor_state_directive && !supports_branch_predictor_model {
        return Err(
            "E_PRIMITIVE_TARGET_CAPABILITY_001: branchPredictorState lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_reorder_state_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::ReorderState));
    if uses_reorder_state_directive && !supports_reorder_model {
        return Err(
            "E_PRIMITIVE_TARGET_CAPABILITY_001: reorderState lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_out_of_order_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::OutOfOrder));
    if uses_out_of_order_directive && !supports_out_of_order_model {
        return Err(
            "E_PRIMITIVE_TARGET_CAPABILITY_001: outOfOrder lowering is unsupported by current target model"
                .into(),
        );
    }
    let uses_pipeline_state_directive = directive_kinds.iter().any(|kind| {
        matches!(
            kind,
            ncc_ast::PrimitiveDirectiveKind::Pipeline
                | ncc_ast::PrimitiveDirectiveKind::PipelineState
                | ncc_ast::PrimitiveDirectiveKind::BranchPredictorState
                | ncc_ast::PrimitiveDirectiveKind::ReorderState
                | ncc_ast::PrimitiveDirectiveKind::OutOfOrder
        )
    });
    if !supports_pipeline_model
        && (uses_pipeline_state_directive
            || all_words.iter().any(|w| {
                matches!(
                    w.as_str(),
                    "pipeline" | "branchpredictor" | "reorder" | "outoforder"
                )
            }))
    {
        return Err("E_PRIMITIVE_TARGET_CAPABILITY_001: target capability is missing".into());
    }
    if !supports_firmware_model
        && all_words
            .iter()
            .any(|w| w == "firmware" || w == "bios" || w == "uefi" || w == "bootrom")
    {
        return Err("E_PRIMITIVE_FIRMWARE_ENV_001: firmware/boot environment does not expose requested capability".into());
    }
    let uses_microcode_directive = directive_kinds
        .iter()
        .any(|kind| matches!(kind, ncc_ast::PrimitiveDirectiveKind::MicrocodeState));
    if uses_microcode_directive && !supports_microcode_state_model {
        return Err(
            "E_PRIMITIVE_MICROCODE_001: microcodeState lowering is unsupported by current target model"
                .into(),
        );
    }
    if (uses_microcode_directive || all_words.iter().any(|w| w == "microcode"))
        && target_spec.field("firmware_microcode").unwrap_or("opaque") != "target_declared"
    {
        return Err(
            "E_PRIMITIVE_MICROCODE_001: microcode state is opaque or not controllable on this target"
                .into(),
        );
    }

    let primitive_contract = program
        .containers
        .iter()
        .find_map(|container| container.as_primitive_at())
        .map(|primitive| {
            primitive_extract_boot_contract(
                &primitive,
                None,
                PrimitiveLoweringPolicy {
                    expert_profile: true,
                    allow_unlowered_sensory: true,
                },
            )
        })
        .transpose()?;

    let mut needs_port_io = false;
    let mut needs_mmio = false;
    let mut needs_volatile = false;
    let mut needs_fence = false;
    if let Some(contract) = primitive_contract {
        for op in &contract.ops {
            match op {
                PrimitiveMaterialOp::PortWrite { .. } | PrimitiveMaterialOp::PortRead { .. } => {
                    needs_port_io = true;
                }
                PrimitiveMaterialOp::MmioWrite { .. } | PrimitiveMaterialOp::MmioRead { .. } => {
                    needs_mmio = true;
                    needs_volatile = true;
                }
                PrimitiveMaterialOp::VolatileWrite { .. }
                | PrimitiveMaterialOp::VolatileRead { .. } => {
                    needs_volatile = true;
                }
                PrimitiveMaterialOp::Fence => {
                    needs_fence = true;
                }
                PrimitiveMaterialOp::Halt
                | PrimitiveMaterialOp::HaltLoop
                | PrimitiveMaterialOp::CpuPause
                | PrimitiveMaterialOp::SensoryPrint
                | PrimitiveMaterialOp::LegacyEventContract { .. }
                | PrimitiveMaterialOp::PhysicalEventContract { .. }
                | PrimitiveMaterialOp::AsyncHardwareEventContract { .. }
                | PrimitiveMaterialOp::SideEffectContract { .. }
                | PrimitiveMaterialOp::MaterialEntityContractHook { .. } => {}
                PrimitiveMaterialOp::ProtocolUart { .. }
                | PrimitiveMaterialOp::ProtocolPcie { .. }
                | PrimitiveMaterialOp::ProtocolI2c { .. }
                | PrimitiveMaterialOp::ProtocolSpi { .. } => {
                    needs_port_io = true;
                    needs_fence = true;
                }
                PrimitiveMaterialOp::ProtocolUsb { .. }
                | PrimitiveMaterialOp::ProtocolSata { .. }
                | PrimitiveMaterialOp::ProtocolNvme { .. }
                | PrimitiveMaterialOp::ProtocolEthernet { .. }
                | PrimitiveMaterialOp::ProtocolHdmi { .. }
                | PrimitiveMaterialOp::ProtocolDisplayPort { .. } => {
                    needs_mmio = true;
                    needs_volatile = true;
                    needs_fence = true;
                }
            }
        }
    }

    if needs_port_io && !supports_port_io {
        if declared_environment
            .to_ascii_lowercase()
            .contains("privileged")
        {
            return Err(format!(
                "E_PRIMITIVE_PORT_IO_PERMISSION_001: port I/O requires an explicit privileged host capability and allowlist entry for target '{}'",
                target
            )
            .into());
        }
        return Err(format!(
            "E_PRIMITIVE_PORT_IO_ENV_001: port I/O is not available in this environment '{}' (E_PRIMITIVE_PORT_UNSUPPORTED_001: target '{}' does not support port I/O required by primitive contract)",
            declared_environment, target
        )
        .into());
    }
    if needs_mmio && !supports_mmio {
        if declared_environment.to_ascii_lowercase().contains("mmio") {
            return Err(format!(
                "E_PRIMITIVE_MMIO_PERMISSION_001: MMIO requires an explicit safe mapping capability and allowlist entry for target '{}'",
                target
            )
            .into());
        }
        return Err(format!(
            "E_PRIMITIVE_MMIO_ENV_001: MMIO mapping is not declared for this environment '{}' (E_PRIMITIVE_MMIO_UNSUPPORTED_001: target '{}' does not support MMIO required by primitive contract)",
            declared_environment, target
        )
        .into());
    }
    if needs_volatile && !supports_volatile {
        return Err(format!(
            "E_PRIMITIVE_TARGET_UNSUPPORTED_001: target '{}' does not support volatile memory semantics required by primitive contract",
            target
        )
        .into());
    }
    if needs_fence && !supports_fence {
        return Err(format!(
            "E_PRIMITIVE_TARGET_UNSUPPORTED_001: target '{}' does not support fence semantics required by primitive contract",
            target
        )
        .into());
    }

    let linker = linker.ok_or(
        "E_PRIMITIVE_CLI_LINKER_001: linker script not found (E_PRIMITIVE_LINKER_MISSING_001: freestanding primitive build requires linker script (--linker link/x86_64_baremetal.ld))",
    )?;
    let linker_path_raw = PathBuf::from(linker);
    let linker_path = if linker_path_raw.is_absolute() {
        linker_path_raw
    } else {
        primitive_resource_path(&linker_path_raw)
    };
    if !linker_path.exists() {
        return Err(format!(
            "E_PRIMITIVE_CLI_LINKER_001: linker script '{}' not found (E_PRIMITIVE_LINKER_MISSING_001)",
            linker_path.to_string_lossy()
        )
        .into());
    }
    let linker_src = fs::read_to_string(&linker_path)?;
    for required in [
        "ENTRY(_start)",
        ".text",
        ".rodata",
        ".data",
        ".bss",
        ".stack",
        ".boot",
        ".primitive.contracts",
    ] {
        if !linker_src.contains(required) {
            return Err(format!(
                "E_PRIMITIVE_CLI_LINKER_001: linker script '{}' is missing required section/directive '{}' (E_PRIMITIVE_LINKER_MISSING_001)",
                linker,
                required
            )
            .into());
        }
    }

    if entry.is_none() {
        return Err(
            "E_PRIMITIVE_ENTRY_MISSING_001: primitive material build requires entry point (--entry <symbol>)"
                .into(),
        );
    }

    Ok(())
}
