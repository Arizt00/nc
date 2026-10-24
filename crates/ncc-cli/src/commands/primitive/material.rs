use super::*;

mod build;
mod check;
mod clean;
mod lower;
mod map;
mod protocol_contract;
mod qemu;
mod report;
mod run;
mod validation;
mod witness;

pub(crate) use protocol_contract::{
    primitive_parse_address_ref_impl, primitive_parse_port_ref_impl,
    primitive_parse_protocol_contract_clause_impl, primitive_protocol_clause_to_op_impl,
    primitive_wait_status_port_for_path_impl,
};
pub(crate) use validation::validate_primitive_material_build_requirements_impl;

pub(super) fn handle_material_subcommand(
    sub: &str,
    sub_args: &[String],
) -> Result<bool, Box<dyn std::error::Error>> {
    match sub {
        "check" => {
            check::handle_check(sub_args)?;
            Ok(true)
        }
        "witness" => {
            witness::handle_witness(sub_args)?;
            Ok(true)
        }
        "map" => {
            map::handle_map(sub_args)?;
            Ok(true)
        }
        "lower" => {
            lower::handle_lower(sub_args)?;
            Ok(true)
        }
        "build" => {
            build::handle_build(sub_args)?;
            Ok(true)
        }
        "qemu" => {
            qemu::handle_qemu(sub_args)?;
            Ok(true)
        }
        "run" => {
            run::handle_run(sub_args)?;
            Ok(true)
        }
        "capture" => {
            run::handle_capture(sub_args)?;
            Ok(true)
        }
        "verify" => {
            run::handle_verify(sub_args)?;
            Ok(true)
        }
        "report" => {
            report::handle_report(sub_args)?;
            Ok(true)
        }
        "clean" => {
            clean::handle_clean()?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
