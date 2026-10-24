use super::super::*;

pub(crate) fn mct_diagnostic_to_json(diagnostic: &ncc_ir::MctDiagnosticIR) -> serde_json::Value {
    crate::canonical_diagnostic_to_json(diagnostic.canonical())
}

pub(crate) fn mct_backend_contract_to_json(
    contract: &ncc_ir::MctBackendContractIR,
) -> serde_json::Value {
    serde_json::json!({
        "provider": ncc_ir::render_mct_backend_provider(contract.provider),
        "status": ncc_ir::render_mct_backend_status(contract.status),
        "capabilities": contract.capabilities.iter().map(|capability| ncc_ir::render_mct_capability(*capability)).collect::<Vec<_>>(),
        "accepted_modes": contract.accepted_modes,
        "required_tools": contract.required_tools,
        "boundary": contract.boundary,
        "diagnostics": contract.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_unit_to_json(unit: &ncc_ir::MctModalUnitIR) -> serde_json::Value {
    serde_json::json!({
        "name": unit.name,
        "mode": unit.mode,
        "function_count": unit.function_count,
        "type_contract_count": unit.type_contract_count,
        "primary_provider": ncc_ir::render_mct_backend_provider(unit.primary_provider),
        "eligible_providers": unit.eligible_providers.iter().map(|provider| ncc_ir::render_mct_backend_provider(*provider)).collect::<Vec<_>>(),
        "diagnostics": unit.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(crate) fn mct_surface_descriptor_to_json(
    descriptor: &ncc_ir::MctSurfaceDescriptorIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": descriptor.schema,
        "construct": descriptor.construct,
        "container": descriptor.container,
        "symbol": descriptor.symbol,
        "nc_type": descriptor.nc_type,
        "visibility": descriptor.visibility,
        "flags": descriptor.flags,
        "payload": descriptor.payload,
        "bytes_len": descriptor.bytes.len(),
        "bytes_hex": mct_bytes_to_hex(&descriptor.bytes),
        "fingerprint": descriptor.fingerprint,
    })
}

pub(crate) fn mct_report_to_json(report: &ncc_ir::MctTranslationReportIR) -> serde_json::Value {
    serde_json::json!({
        "schema": report.schema,
        "status": report.status,
        "target": render_backend_target_label(&report.target),
        "target_triple": report.target_triple,
        "units": report.units.iter().map(mct_unit_to_json).collect::<Vec<_>>(),
        "backend_contracts": report.backend_contracts.iter().map(mct_backend_contract_to_json).collect::<Vec<_>>(),
        "diagnostics": report.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
        "full_llvm_parity": report.full_llvm_parity,
        "full_gcc_parity": report.full_gcc_parity,
        "full_rustc_parity": report.full_rustc_parity,
        "ncc_native_integrated": report.ncc_native_integrated,
        "surface_descriptor_count": report.surface_descriptor_count,
        "surface_descriptor_bytes": report.surface_descriptor_bytes,
        "surface_descriptors": report.surface_descriptors.iter().map(mct_surface_descriptor_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_llvm_function_to_json(function: &ncc_ir::MctLlvmFunctionIR) -> serde_json::Value {
    serde_json::json!({
        "source_name": function.source_name,
        "symbol_name": function.symbol_name,
        "visibility": ncc_ir::render_function_visibility(function.visibility),
        "return_type": function.return_type,
        "param_types": function.param_types,
        "status": ncc_ir::render_mct_llvm_function_status(function.status),
        "diagnostics": function.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_llvm_construct_coverage_row_to_json(
    row: &ncc_ir::MctLlvmConstructCoverageRowIR,
) -> serde_json::Value {
    serde_json::json!({
        "construct": row.construct,
        "family": row.family,
        "status": ncc_ir::render_mct_llvm_construct_representation(row.status),
        "present_in_module": row.present_in_module,
        "lowering": row.lowering,
        "diagnostic": row.diagnostic,
        "reason": row.reason,
        "evidence": row.evidence,
    })
}

pub(crate) fn mct_llvm_construct_coverage_to_json(
    coverage: &ncc_ir::MctLlvmConstructCoverageIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": coverage.schema,
        "status": coverage.status,
        "row_count": coverage.row_count,
        "represented_count": coverage.represented_count,
        "not_represented_count": coverage.not_represented_count,
        "forbidden_count": coverage.forbidden_count,
        "present_not_represented_count": coverage.present_not_represented_count,
        "rows": coverage.rows.iter().map(mct_llvm_construct_coverage_row_to_json).collect::<Vec<_>>(),
        "diagnostics": coverage.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_llvm_module_to_json(
    module: &ncc_ir::MctLlvmModuleIR,
    include_ir: bool,
    output_path: Option<&Path>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "schema": module.schema,
        "status": module.status,
        "target_triple": module.target_triple,
        "data_layout": module.data_layout,
        "pass_pipeline": mct_llvm_pass_pipeline_to_json(&module.pass_pipeline),
        "debug_metadata": mct_llvm_debug_metadata_to_json(&module.debug_metadata),
        "source_map": mct_source_map_to_json(&module.source_map),
        "target_attributes": mct_llvm_target_attributes_to_json(&module.target_attributes),
        "structs": module.structs.iter().map(mct_llvm_struct_to_json).collect::<Vec<_>>(),
        "dynamic_views": module.dynamic_views.iter().map(mct_llvm_dynamic_view_to_json).collect::<Vec<_>>(),
        "construct_coverage": mct_llvm_construct_coverage_to_json(&module.construct_coverage),
        "surface_descriptor_count": module.surface_descriptor_count,
        "surface_descriptor_bytes": module.surface_descriptor_bytes,
        "surface_descriptors": module.surface_descriptors.iter().map(mct_surface_descriptor_to_json).collect::<Vec<_>>(),
        "functions": module.functions.iter().map(mct_llvm_function_to_json).collect::<Vec<_>>(),
        "emitted_function_count": module.emitted_function_count,
        "rejected_function_count": module.rejected_function_count,
        "diagnostics": module.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
        "full_llvm_parity": module.full_llvm_parity,
        "output_path": output_path.map(|path| path.display().to_string()),
    });
    if include_ir {
        payload["llvm_ir"] = serde_json::Value::String(module.llvm_ir.clone());
    }
    payload
}

pub(crate) fn mct_llvm_target_attributes_to_json(
    attributes: &ncc_ir::MctLlvmTargetAttributesIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": attributes.schema,
        "status": attributes.status,
        "target_triple": attributes.target_triple,
        "object_format": attributes.object_format,
        "abi": attributes.abi,
        "debug_model": attributes.debug_model,
        "attribute_group": attributes.attribute_group,
        "target_cpu": attributes.target_cpu,
        "tune_cpu": attributes.tune_cpu,
        "target_features": attributes.target_features,
        "microarchitecture": attributes.microarchitecture,
        "fine_tuning_profile": attributes.fine_tuning_profile,
        "function_attributes": attributes.function_attributes.iter().map(|attribute| {
            serde_json::json!({
                "name": attribute.name,
                "value": attribute.value,
                "category": attribute.category,
                "llvm_ir": attribute.llvm_ir,
            })
        }).collect::<Vec<_>>(),
        "attribute_group_ir": attributes.attribute_group_ir,
        "applies_to_emitted_functions": attributes.applies_to_emitted_functions,
        "full_target_attribute_parity": attributes.full_target_attribute_parity,
        "diagnostics": attributes.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_llvm_attribute_catalog_to_json() -> serde_json::Value {
    let catalog = ncc_ir::mct_llvm_attribute_catalog();
    let function_count = catalog
        .iter()
        .filter(|entry| entry.function_position)
        .count();
    let parameter_count = catalog
        .iter()
        .filter(|entry| entry.parameter_position)
        .count();
    let return_count = catalog.iter().filter(|entry| entry.return_position).count();
    let parameterized_count = catalog
        .iter()
        .filter(|entry| {
            !matches!(
                entry.kind,
                ncc_ir::MctLlvmAttributeKindIR::Enum | ncc_ir::MctLlvmAttributeKindIR::StringBool
            )
        })
        .count();
    let sanitizer_or_hardening_count = catalog
        .iter()
        .filter(|entry| entry.policy == "toolchain_sanitizer_or_hardening")
        .count();
    let abi_or_pointer_count = catalog
        .iter()
        .filter(|entry| entry.policy == "abi_or_pointer_contract")
        .count();
    serde_json::json!({
        "schema": "nc.mct.llvm.attribute_catalog.v1",
        "status": if ncc_ir::mct_llvm_attribute_catalog_closed() {
            "mct_llvm_attribute_catalog_closed"
        } else {
            "mct_llvm_attribute_catalog_drift"
        },
        "source": ncc_ir::MCT_LLVM_ATTRIBUTE_CATALOG_SOURCE,
        "source_url": ncc_ir::MCT_LLVM_ATTRIBUTE_CATALOG_SOURCE_URL,
        "observed_date": ncc_ir::MCT_LLVM_ATTRIBUTE_CATALOG_OBSERVED_DATE,
        "expected_count": ncc_ir::MCT_LLVM_ATTRIBUTE_CATALOG_EXPECTED_COUNT,
        "catalog_count": catalog.len(),
        "summary": {
            "function_position": function_count,
            "parameter_position": parameter_count,
            "return_position": return_count,
            "parameterized": parameterized_count,
            "sanitizer_or_hardening": sanitizer_or_hardening_count,
            "abi_or_pointer_contract": abi_or_pointer_count,
        },
        "entries": catalog.iter().map(|entry| {
            serde_json::json!({
                "tablegen_name": entry.tablegen_name,
                "llvm_ir_name": entry.llvm_ir_name,
                "kind": ncc_ir::render_mct_llvm_attribute_kind(entry.kind),
                "positions": {
                    "function": entry.function_position,
                    "parameter": entry.parameter_position,
                    "return": entry.return_position,
                },
                "policy": entry.policy,
            })
        }).collect::<Vec<_>>(),
        "truth_boundary": {
            "attributes_td_catalog_covered": ncc_ir::mct_llvm_attribute_catalog_closed(),
            "all_entries_have_position_or_known_string_policy": catalog.iter().all(|entry| {
                entry.function_position
                    || entry.parameter_position
                    || entry.return_position
                    || entry.kind == ncc_ir::MctLlvmAttributeKindIR::StringBool
            }),
            "all_attributes_emitted_unconditionally": false,
            "reason": "MCT now catalogs every current Attributes.td attribute and validates its legal placement/policy. Emission remains driven by nC semantics, target profile and explicit lowering requests."
        }
    })
}

#[cfg(test)]
mod canonical_mct_diagnostic_tests {
    use super::*;

    #[test]
    fn mct_diagnostic_uses_canonical_binary_abi() {
        let diagnostic = ncc_ir::MctDiagnosticIR::new(
            "E_MCT_LLVM_TEST_001",
            ncc_ir::MctDiagnosticSeverityIR::Error,
            "mct test".to_string(),
        );
        let entry = mct_diagnostic_to_json(&diagnostic);
        assert_eq!(entry["abi"], "nc.diagnostic.machine.v2");
        assert_eq!(entry["phase"], "mct.llvm");
        assert_eq!(
            entry["domain"],
            ncc_diagnostics::DiagnosticDomain::Mct as u8
        );
        assert_eq!(entry["record_bytes"], 128);
        assert!(entry["machine_record_hex"]
            .as_str()
            .is_some_and(|hex| hex.starts_with("4e43444902008000")));
    }
}

pub(crate) fn mct_cpu_tuning_intent_to_json(intent: ncc_ir::MctCpuTuningIntentIR) -> &'static str {
    ncc_ir::render_mct_cpu_tuning_intent(intent)
}

pub(crate) fn mct_cpu_microarchitecture_to_json(
    profile: &ncc_ir::MctCpuMicroarchitectureIR,
) -> serde_json::Value {
    serde_json::json!({
        "id": profile.id,
        "status": profile.status,
        "target": profile.target,
        "vendor": profile.vendor,
        "family": profile.family,
        "llvm_target_cpu": profile.llvm_target_cpu,
        "llvm_tune_cpu": profile.llvm_tune_cpu,
        "target_features": profile.target_features,
        "preferred_vector_width_bits": profile.preferred_vector_width_bits,
        "issue_width": profile.issue_width,
        "reorder_window": profile.reorder_window,
        "load_latency_cycles": profile.load_latency_cycles,
        "store_forward_latency_cycles": profile.store_forward_latency_cycles,
        "branch_mispredict_penalty_cycles": profile.branch_mispredict_penalty_cycles,
        "cache_line_bytes": profile.cache_line_bytes,
        "supports_fma": profile.supports_fma,
        "supports_gather_scatter": profile.supports_gather_scatter,
        "scheduling_model": profile.scheduling_model,
        "notes": profile.notes,
    })
}

pub(crate) fn mct_cpu_fine_tuning_profile_to_json(
    profile: &ncc_ir::MctCpuFineTuningProfileIR,
) -> serde_json::Value {
    serde_json::json!({
        "id": profile.id,
        "intent": mct_cpu_tuning_intent_to_json(profile.intent),
        "opt_level_hint": profile.opt_level_hint,
        "inline_policy": profile.inline_policy,
        "unroll_policy": profile.unroll_policy,
        "vectorize_policy": profile.vectorize_policy,
        "slp_policy": profile.slp_policy,
        "prefetch_policy": profile.prefetch_policy,
        "branch_policy": profile.branch_policy,
        "scheduler_policy": profile.scheduler_policy,
    })
}

pub(crate) fn mct_cpu_scheduling_profile_to_json(
    profile: &ncc_ir::MctCpuSchedulingProfileIR,
) -> serde_json::Value {
    serde_json::json!({
        "microarchitecture_id": profile.microarchitecture_id,
        "fine_tuning_profile_id": profile.fine_tuning_profile_id,
        "target": profile.target,
        "llvm_target_cpu": profile.llvm_target_cpu,
        "llvm_tune_cpu": profile.llvm_tune_cpu,
        "opt_level_hint": profile.opt_level_hint,
        "issue_width": profile.issue_width,
        "preferred_vector_width_bits": profile.preferred_vector_width_bits,
        "load_latency_cycles": profile.load_latency_cycles,
        "branch_mispredict_penalty_cycles": profile.branch_mispredict_penalty_cycles,
        "unroll_policy": profile.unroll_policy,
        "vectorize_policy": profile.vectorize_policy,
        "scheduling_model": profile.scheduling_model,
        "llvm_args": profile.llvm_args,
        "diagnostics": profile.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn backend_simd_schedule_to_json(
    row: &ncc_ir::BackendSimdScheduleIR,
) -> serde_json::Value {
    serde_json::json!({
        "microarchitecture": ncc_ir::render_backend_simd_microarchitecture(row.microarchitecture),
        "feature": ncc_ir::render_backend_simd_feature(row.feature),
        "lane": ncc_ir::render_backend_simd_lane(row.lane),
        "op": ncc_ir::render_backend_simd_op(row.op),
        "latency_cycles": row.latency_cycles,
        "reciprocal_throughput_per_cycle": row.reciprocal_throughput_per_cycle,
        "issue_domain": row.issue_domain,
        "scheduling_note": row.scheduling_note,
    })
}

pub(crate) fn backend_native_cpu_tuning_bridge_to_json(
    bridge: &ncc_ir::BackendNativeCpuTuningBridgeIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": bridge.schema,
        "status": bridge.status,
        "phase": bridge.phase,
        "microarchitecture_id": bridge.microarchitecture_id,
        "target": bridge.target,
        "llvm_target_cpu": bridge.llvm_target_cpu,
        "llvm_tune_cpu": bridge.llvm_tune_cpu,
        "native_microarchitecture": bridge.native_microarchitecture.map(ncc_ir::render_backend_simd_microarchitecture),
        "schedule_row_count": bridge.schedule_row_count,
        "schedule_rows": bridge.schedule_rows.iter().map(backend_simd_schedule_to_json).collect::<Vec<_>>(),
        "object_disasm_evidence_required": bridge.object_disasm_evidence_required,
        "full_native_scheduler_parity": bridge.full_native_scheduler_parity,
        "diagnostics": bridge.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn backend_native_cpu_tuning_decision_to_json(
    decision: &ncc_ir::BackendNativeCpuTuningDecisionIR,
) -> serde_json::Value {
    serde_json::json!({
        "op": ncc_ir::render_backend_simd_op(decision.op),
        "vector_type": decision.vector_type,
        "feature": ncc_ir::render_backend_simd_feature(decision.feature),
        "lane": ncc_ir::render_backend_simd_lane(decision.lane),
        "selected_mnemonic": decision.selected_mnemonic,
        "selected_asm": decision.selected_asm,
        "selected_bytes_hex": decision.selected_bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "schedule": backend_simd_schedule_to_json(&decision.schedule),
        "selection_reason": decision.selection_reason,
    })
}

pub(crate) fn backend_native_cpu_tuning_policy_to_json(
    policy: &ncc_ir::BackendNativeCpuTuningPolicyIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": policy.schema,
        "status": policy.status,
        "phase": policy.phase,
        "microarchitecture_id": policy.microarchitecture_id,
        "fine_tuning_profile_id": policy.fine_tuning_profile_id,
        "target": policy.target,
        "native_microarchitecture": policy.native_microarchitecture.map(ncc_ir::render_backend_simd_microarchitecture),
        "preferred_vector_width_bits": policy.preferred_vector_width_bits,
        "vectorize_policy": policy.vectorize_policy,
        "scheduler_policy": policy.scheduler_policy,
        "decision_count": policy.decision_count,
        "decisions": policy.decisions.iter().map(backend_native_cpu_tuning_decision_to_json).collect::<Vec<_>>(),
        "full_native_scheduler_parity": policy.full_native_scheduler_parity,
        "diagnostics": policy.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_cpu_tuning_matrix_to_json(
    matrix: &ncc_ir::MctCpuTuningMatrixIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": matrix.schema,
        "status": matrix.status,
        "phase": matrix.phase,
        "microarchitecture_count": matrix.microarchitecture_count,
        "fine_tuning_profile_count": matrix.fine_tuning_profile_count,
        "scheduling_profile_count": matrix.scheduling_profile_count,
        "microarchitectures": matrix.microarchitectures.iter().map(mct_cpu_microarchitecture_to_json).collect::<Vec<_>>(),
        "fine_tuning_profiles": matrix.fine_tuning_profiles.iter().map(mct_cpu_fine_tuning_profile_to_json).collect::<Vec<_>>(),
        "scheduling_profiles": matrix.scheduling_profiles.iter().map(mct_cpu_scheduling_profile_to_json).collect::<Vec<_>>(),
        "full_llvm_cpu_universe": matrix.full_llvm_cpu_universe,
        "full_llvm_scheduler_parity": matrix.full_llvm_scheduler_parity,
        "diagnostics": matrix.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_official_target_matrix_to_json(
    matrix: &ncc_ir::MctBaseicalTargetMatrixIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": matrix.schema,
        "status": matrix.status,
        "phase": matrix.phase,
        "row_count": matrix.row_count,
        "closed_profile_count": matrix.closed_profile_count,
        "rows": matrix.rows.iter().map(|row| {
            serde_json::json!({
                "id": row.id,
                "status": row.status,
                "target": row.target,
                "calling_convention": row.calling_convention,
                "target_triple": row.target_triple,
                "target_family": row.target_family,
                "isa": row.isa,
                "abi": row.abi,
                "object_format": row.object_format,
                "relocation_model": row.relocation_model,
                "code_model": row.code_model,
                "endianness": row.endianness,
                "pointer_width": row.pointer_width,
                "runtime_model": row.runtime_model,
                "host_services": row.host_services,
                "debug_info_model": row.debug_info_model,
                "linker_model": row.linker_model,
                "support_tier": row.support_tier,
                "backend_kind": row.backend_kind,
                "target_cpu": row.target_cpu,
                "target_features": row.target_features,
                "llvm_attribute_status": row.llvm_attribute_status,
                "tuning_policy": row.tuning_policy,
                "smoke_status": row.smoke_status,
                "full_llvm_microarch_universe": row.full_llvm_microarch_universe,
                "diagnostics": row.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "full_llvm_microarch_universe": matrix.full_llvm_microarch_universe,
        "full_llvm_tuning_parity": matrix.full_llvm_tuning_parity,
        "diagnostics": matrix.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_llvm_debug_metadata_to_json(
    metadata: &ncc_ir::MctLlvmDebugMetadataIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": metadata.schema,
        "status": metadata.status,
        "enabled": metadata.enabled,
        "source_filename": metadata.source_filename,
        "source_directory": metadata.source_directory,
        "producer": metadata.producer,
        "dwarf_version": metadata.dwarf_version,
        "debug_info_version": metadata.debug_info_version,
        "emission_kind": metadata.emission_kind,
        "compile_unit_metadata": metadata.compile_unit_metadata,
        "file_metadata": metadata.file_metadata,
        "subprogram_count": metadata.subprogram_count,
        "local_variable_count": metadata.local_variable_count,
        "location_count": metadata.location_count,
        "basic_type_count": metadata.basic_type_count,
        "source_span_parity": metadata.source_span_parity,
        "full_dwarf_parity": metadata.full_dwarf_parity,
        "diagnostics": metadata.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_source_map_to_json(source_map: &ncc_ir::ProgramSourceMapIR) -> serde_json::Value {
    serde_json::json!({
        "schema": source_map.schema,
        "status": source_map.status,
        "file": source_map.file,
        "exact_span_count": source_map.exact_span_count,
        "synthetic_span_count": source_map.synthetic_span_count,
        "missing_span_count": source_map.missing_span_count,
        "statement_count": source_map.statement_count,
        "entries": source_map.entries.iter().map(mct_statement_source_span_to_json).collect::<Vec<_>>(),
        "diagnostics": source_map.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
    })
}

pub(crate) fn mct_statement_source_span_to_json(
    entry: &ncc_ir::StatementSourceSpanIR,
) -> serde_json::Value {
    serde_json::json!({
        "container": entry.container,
        "function": entry.function,
        "section": entry.section,
        "ordinal_path": entry.ordinal_path,
        "statement_kind": entry.statement_kind,
        "raw": entry.raw,
        "status": mct_source_span_status_to_json(entry.status),
        "span": entry.span.as_ref().map(mct_source_span_to_json),
    })
}

pub(crate) fn mct_source_span_to_json(span: &ncc_ir::SourceSpanIR) -> serde_json::Value {
    serde_json::json!({
        "file": span.file,
        "start_line": span.start_line,
        "start_column": span.start_column,
        "end_line": span.end_line,
        "end_column": span.end_column,
        "start_byte": span.start_byte,
        "end_byte": span.end_byte,
    })
}

pub(crate) fn mct_source_span_status_to_json(status: ncc_ir::SourceSpanStatusIR) -> &'static str {
    match status {
        ncc_ir::SourceSpanStatusIR::Exact => "exact",
        ncc_ir::SourceSpanStatusIR::SyntheticUnavailable => "synthetic_unavailable",
        ncc_ir::SourceSpanStatusIR::Missing => "missing",
    }
}

pub(crate) fn mct_llvm_debug_config_for_path(source: &str) -> ncc_ir::MctLlvmDebugConfigIR {
    let path = Path::new(source);
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(source)
        .to_string();
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|cwd| cwd.display().to_string())
                .unwrap_or_else(|_| ".".into())
        });
    ncc_ir::mct_llvm_debug_config_for_source(filename, directory)
}

pub(crate) fn mct_llvm_pass_pipeline_to_json(
    pipeline: &ncc_ir::MctLlvmPassPipelineIR,
) -> serde_json::Value {
    serde_json::json!({
        "schema": pipeline.schema,
        "status": pipeline.status,
        "owner": pipeline.owner,
        "level": ncc_ir::render_mct_llvm_optimization_level(pipeline.level),
        "profile": pipeline.profile,
        "clang_opt_arg": pipeline.clang_opt_arg,
        "opt_passes_arg": pipeline.opt_passes_arg,
        "clang_pass_manager_args": pipeline.clang_pass_manager_args,
        "clang_lto_args": pipeline.clang_lto_args,
        "clang_pgo_generate_args": pipeline.clang_pgo_generate_args,
        "clang_pgo_use_arg_template": pipeline.clang_pgo_use_arg_template,
        "clang_debug_args": pipeline.clang_debug_args,
        "pass_count": pipeline.pass_count,
        "passes": pipeline.passes.iter().map(|pass| {
            serde_json::json!({
                "name": pass.name,
                "stage": pass.stage,
                "purpose": pass.purpose,
                "enabled": pass.enabled
            })
        }).collect::<Vec<_>>(),
        "diagnostics": pipeline.diagnostics.iter().map(mct_diagnostic_to_json).collect::<Vec<_>>(),
        "full_llvm_pass_manager_parity": pipeline.full_llvm_pass_manager_parity,
    })
}

pub(crate) fn mct_llvm_struct_to_json(layout: &ncc_ir::MctLlvmStructIR) -> serde_json::Value {
    serde_json::json!({
        "source_name": layout.source_name,
        "symbol_name": layout.symbol_name,
        "llvm_type": layout.llvm_type,
        "field_count": layout.field_count,
        "size_bytes": layout.size_bytes,
        "align_bytes": layout.align_bytes,
        "fields": layout.fields.iter().map(|field| {
            serde_json::json!({
                "name": field.name,
                "nc_type": field.nc_type,
                "llvm_type": field.llvm_type,
                "index": field.index,
                "offset_bytes": field.offset_bytes,
                "size_bytes": field.size_bytes,
                "align_bytes": field.align_bytes
            })
        }).collect::<Vec<_>>()
    })
}

pub(crate) fn mct_llvm_dynamic_view_to_json(
    layout: &ncc_ir::MctLlvmDynamicViewIR,
) -> serde_json::Value {
    serde_json::json!({
        "source_type": layout.source_type,
        "kind": layout.kind,
        "llvm_type": layout.llvm_type,
        "element_nc_type": layout.element_nc_type,
        "element_llvm_type": layout.element_llvm_type,
        "field_count": layout.field_count,
        "size_bytes": layout.size_bytes,
        "align_bytes": layout.align_bytes,
        "has_capacity": layout.has_capacity,
    })
}

pub(crate) fn mct_dinarray_descriptor_abi_manifest() -> serde_json::Value {
    let closed_profiles = [
        ("nC.sysv64", ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
        ("nC.win64", ncc_ir::BackendLoweringProfileIR::nc_win64()),
        ("nC.aapcs64", ncc_ir::BackendLoweringProfileIR::nc_aapcs64()),
    ]
    .into_iter()
    .map(|(id, profile)| {
        serde_json::json!({
            "id": id,
            "status": "closed",
            "target_triple": profile.target_spec.target_triple,
            "abi": render_backend_abi_label(&profile.target_spec.abi),
            "object_format": render_backend_object_format_label(&profile.target_spec.object_format),
            "endianness": render_backend_endianness_label(&profile.target_spec.endianness),
            "pointer_width_bits": profile.target_spec.pointer_width,
            "descriptor_size_bytes": 24,
            "descriptor_align_bytes": 8,
            "field_offsets": {
                "data": 0,
                "len": 8,
                "capacity": 16
            }
        })
    })
    .collect::<Vec<_>>();

    serde_json::json!({
        "schema": "nc.mct.llvm.dinarray_descriptor_abi.v1",
        "status": "closed_for_hosted_64_bit_targets",
        "descriptor": {
            "kind": "dinArray",
            "official_llvm_type": "%nC.dinarray_<element> = type { ptr, i64, i64 }",
            "fields": [
                {
                    "name": "data",
                    "llvm_type": "ptr",
                    "offset_bytes": 0,
                    "meaning": "heap runtime backing pointer"
                },
                {
                    "name": "len",
                    "llvm_type": "i64",
                    "offset_bytes": 8,
                    "meaning": "live element count"
                },
                {
                    "name": "capacity",
                    "llvm_type": "i64",
                    "offset_bytes": 16,
                    "meaning": "allocated element capacity"
                }
            ],
            "descriptor_size_bytes": 24,
            "descriptor_align_bytes": 8,
            "ownership": {
                "backing_storage": "runtime_heap",
                "allocation": "malloc",
                "growth": "realloc",
                "release_boundary": "free",
                "automatic_finalizer_insertion": "deferred_to_descriptor_finalization_policy"
            }
        },
        "closed_profiles": closed_profiles,
        "deferred_profiles": [
            {
                "id": "nC.portable",
                "status": "deferred_allocator_abi",
                "reason": "WASI/portable allocator imports need an explicit runtime import contract before final descriptor ABI evidence is claimed."
            }
        ],
        "truth_boundary": {
            "descriptor_layout_closed": true,
            "hosted_heap_backing_closed": true,
            "automatic_destructor_or_finalizer_insertion": false,
            "full_portable_allocator_abi": false
        }
    })
}
