use super::super::*;

pub(crate) fn run_nc_unified_doctor_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" | "help" => {
                if let Some(help) = command_help("doctor") {
                    println!("{help}");
                }
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "doctor",
                    format!("doctor: unknown option '{}'", other),
                )
                .into());
            }
        }
    }

    let payload = nc_unified_doctor_payload();
    if json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("ncc doctor: OK");
        println!("scope: normal_nc_high_level_only");
        println!("included: normal_nc, local, high-level, runtime, compiler, pointer_model");
        println!("excluded_by_design: basal, machine, primitive, kernel");
        println!("semantic_changes: pointer_model_officialized_normal_high_level");
    }
    Ok(())
}
