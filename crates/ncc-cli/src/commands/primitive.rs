use super::super::*;
use crate::primitive_cli::{primitive_help_text, primitive_parse_standard_options};

mod common;
mod doctor;
mod material;
mod material_scan;
mod protocol_registry;
mod target_registry;

pub(crate) use common::{
    primitive_bytes_to_hex_impl, primitive_default_entry_for_program, primitive_emit_witness_json,
    primitive_event_contracts_to_json_impl, primitive_firmware_boot_model_json,
    primitive_read_program, primitive_reject_runtime_stdout_fallbacks,
};
pub(crate) use material::validate_primitive_material_build_requirements_impl;
pub(crate) use material::{
    primitive_parse_address_ref_impl, primitive_parse_port_ref_impl,
    primitive_parse_protocol_contract_clause_impl, primitive_protocol_clause_to_op_impl,
    primitive_wait_status_port_for_path_impl,
};
pub(crate) use material_scan::{
    primitive_collect_all_words, primitive_collect_directive_kinds_impl,
    primitive_collect_protocol_requirements, primitive_directive_kind_labels_impl,
    primitive_has_rupture_section, primitive_material_semantic_labels_impl,
    primitive_material_semantic_name_impl, primitive_protocol_requires_channel,
    primitive_protocol_requires_controller, primitive_protocol_requires_lane,
    primitive_protocol_requires_peripheral, primitive_protocol_requires_phy,
};
pub(crate) use protocol_registry::{
    primitive_normalize_protocol_word, primitive_protocol_diag_code,
    primitive_protocol_lowering_mode, primitive_protocol_lowering_prefix_bytes,
    primitive_protocol_lowering_semantics, primitive_protocol_lowering_suffix_bytes,
    primitive_protocol_marker_byte, primitive_protocol_transport,
};
#[cfg(test)]
pub(crate) use target_registry::primitive_target_path;
pub(crate) use target_registry::{
    list_primitive_target_specs, load_primitive_target_spec, primitive_resource_path,
    PrimitiveTargetSpec,
};

use common::{
    primitive_audit_event_contracts, primitive_claim_for_evidence_level,
    primitive_enforce_strict_qemu_certification_guard, primitive_environment_class,
    primitive_event_audit_to_json, primitive_host_label, primitive_render_report_markdown,
    primitive_resolve_target_spec_for_program, primitive_sha256_hex_bytes,
    primitive_sha256_hex_string, primitive_timestamp_label, primitive_transport_for_target,
};

pub(crate) fn run_primitive_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || is_help_token(&args[0]) {
        println!("{}", primitive_help_text());
        return Ok(());
    }

    let sub = args[0].as_str();
    let sub_args = &args[1..];

    match sub {
        "doctor" => doctor::handle_doctor(sub_args)?,
        "target" => doctor::handle_target(sub_args)?,
        _ => {
            if !material::handle_material_subcommand(sub, sub_args)? {
                return Err(actionable_command_error(
                    "primitive",
                    format!("unknown primitive command '{}'", sub),
                )
                .into());
            }
        }
    }

    Ok(())
}
