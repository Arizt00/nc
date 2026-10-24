use super::*;

mod doctor_cmd;
mod target_cmd;

pub(super) fn handle_doctor(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    doctor_cmd::handle_doctor(sub_args)
}

pub(super) fn handle_target(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    target_cmd::handle_target(sub_args)
}
