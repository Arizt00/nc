pub(crate) fn run_imports_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(subcommand) = args.first() else {
        println!("{}", crate::imports_help_text());
        return Ok(());
    };

    if crate::is_help_token(subcommand) {
        println!("{}", crate::imports_help_text());
        return Ok(());
    }

    let json = args.iter().any(|arg| arg == "--json");
    let payload = match subcommand.as_str() {
        "doctor" => crate::import_surface_doctor_payload(),
        "manifest" => crate::import_manifest_payload(None),
        "core" => crate::import_manifest_payload(Some(ncc_ast::UseDomain::Core)),
        "alloc" => crate::import_manifest_payload(Some(ncc_ast::UseDomain::Alloc)),
        "stoud" => crate::import_manifest_payload(Some(ncc_ast::UseDomain::Stoud)),
        "auth" => crate::import_manifest_payload(Some(ncc_ast::UseDomain::Auth)),
        "dom" => crate::import_manifest_payload(Some(ncc_ast::UseDomain::Dom)),
        "backend" | "backend-plan" => {
            let path = args.iter().skip(1).find_map(|arg| {
                if arg.starts_with("--") {
                    None
                } else {
                    Some(arg.as_str())
                }
            });
            crate::imports_backend_payload(path)?
        }
        "runtime-table" | "runtime-status" => {
            let path = args.iter().skip(1).find_map(|arg| {
                if arg.starts_with("--") {
                    None
                } else {
                    Some(arg.as_str())
                }
            });
            crate::imports_runtime_table_payload(path)?
        }
        "closure-plan" | "pending" => crate::imports_closure_plan_payload(),
        "expand" => {
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !arg.starts_with("--"))
                .ok_or("imports expand expects <file.n>")?;
            crate::imports_expand_payload(path)?
        }
        other => {
            return Err(format!(
                "imports: unknown subcommand '{}'; expected doctor, manifest, expand, backend, runtime-table, closure-plan, core, alloc, stoud, auth or dom",
                other
            )
            .into());
        }
    };

    if json
        || matches!(
            subcommand.as_str(),
            "doctor"
                | "manifest"
                | "core"
                | "alloc"
                | "stoud"
                | "auth"
                | "dom"
                | "expand"
                | "backend"
                | "backend-plan"
                | "runtime-table"
                | "runtime-status"
                | "closure-plan"
                | "pending"
        )
    {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("imports: closed");
    }
    Ok(())
}
