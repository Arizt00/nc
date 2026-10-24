use super::*;

pub(super) fn handle_target(sub_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (opts, rest) = primitive_parse_standard_options(sub_args)?;
    let maybe_name = rest.first().map(String::as_str);
    if let Some(name) = maybe_name {
        let spec = load_primitive_target_spec(name)?;
        let env_name = if let Some(value) = opts.environment.as_deref() {
            value.to_string()
        } else {
            spec.field("environment")
                .ok_or("E_PRIMITIVE_ENV_MISSING_001: primitive[@] requires an explicit material environment")?
                .to_string()
        };
        primitive_enforce_strict_qemu_certification_guard(&opts, &spec, &env_name)?;
        let payload = serde_json::json!({
            "name": spec.id,
            "path": spec.path.to_string_lossy(),
            "arch": spec.field("arch"),
            "isa": spec.field("isa"),
            "environment": spec.field("environment"),
            "runtime": spec.field("runtime"),
            "protocols": spec.protocols,
            "capabilities": spec.bool_fields,
        });
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("target: {}", payload["name"].as_str().unwrap_or(""));
            println!("environment: {}", env_name);
            println!(
                "protocols: {}",
                serde_json::to_string(&payload["protocols"])?
            );
        }
    } else {
        let targets = list_primitive_target_specs()?;
        let payload = serde_json::json!({
            "targets": targets.iter().map(|t| t.id.clone()).collect::<Vec<_>>()
        });
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            for target in targets {
                println!("{}", target.id);
            }
        }
    }
    Ok(())
}
