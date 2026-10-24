use super::super::*;

pub(crate) fn run_optimize_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| is_help_token(arg)) {
        println!(
            "{}",
            command_help("optimize").expect("optimize help must exist")
        );
        return Ok(());
    }

    let path = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| actionable_command_error("optimize", "optimize expects <file.n>"))?;
    let json = args.iter().any(|arg| arg == "--json");
    let out_path = parse_tool_opt(args, "--out");
    let payload = build_optimizer_payload_from_source(path, args)?;
    let rendered = serde_json::to_string_pretty(&payload)?;

    if let Some(out) = out_path.as_deref() {
        ensure_output_parent_dir(out)?;
        fs::write(out, rendered.as_bytes())?;
    }

    if json || out_path.is_none() {
        println!("{rendered}");
    } else {
        println!("OK: optimize report {}", out_path.unwrap_or_default());
    }
    Ok(())
}
