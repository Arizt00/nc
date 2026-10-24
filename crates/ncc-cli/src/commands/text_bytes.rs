pub(crate) fn run_text_bytes_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| crate::is_help_token(arg)) {
        crate::print_ncc_help(
            crate::command_help("text-bytes").expect("text-bytes help must exist"),
        );
        return Ok(());
    }

    let mut json = false;
    let mut path: Option<&str> = None;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            other if other.starts_with("--") => {
                return Err(crate::actionable_command_error(
                    "text-bytes",
                    format!("text-bytes: unknown option '{}'", other),
                )
                .into());
            }
            other => {
                if path.is_some() {
                    return Err(crate::actionable_command_error(
                        "text-bytes",
                        "text-bytes expects exactly one source file",
                    )
                    .into());
                }
                path = Some(other);
            }
        }
    }

    let path =
        path.ok_or_else(|| crate::actionable_command_error("text-bytes", "missing source file"))?;
    let payload = crate::text_literal_bytes_payload(path)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "text-bytes: {} literal(s), {} payload byte(s), {} nul-terminated byte(s)",
        payload
            .get("literal_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        payload
            .get("payload_bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0),
        payload
            .get("nul_terminated_bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(literals) = payload
        .get("literals")
        .and_then(serde_json::Value::as_array)
    {
        for literal in literals {
            let idx = literal
                .get("literal_index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let line = literal
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let column = literal
                .get("column")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let hex = literal
                .get("bytes_hex")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            println!("literal[{idx}] @{line}:{column} bytes={hex}");
        }
    }
    Ok(())
}
