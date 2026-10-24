use super::super::*;

fn local_surface_doctor_payload() -> serde_json::Value {
    serde_json::json!({
        "schema": "ncc.local.doctor.v1",
        "component": "local",
        "status": "closed",
        "local_surface": "closed",
        "local_section": "official",
        "stc_legacy": "preserved",
        "stc_to_local_alias": "closed",
        "const_local": "closed",
        "type_local": "closed",
        "visibility": "closed",
        "metadata": "closed",
        "site_integration": "documented_future_lowering_hook",
        "diagnostics": {
            "duplicate_symbol": "E_LOCAL_DUP_SYMBOL_001",
            "duplicate_type": "E_LOCAL_TYPE_DUP_001",
            "unknown_type": "E_LOCAL_TYPE_UNKNOWN_001",
            "const_mutation": "E_LOCAL_CONST_MUTATION_001",
            "mode_forbidden": "E_LOCAL_MODE_FORBIDDEN_001"
        },
        "mode_gates": {
            "normal_high_level": "allowed",
            "runtime": "allowed",
            "compiler": "allowed",
            "system": "allowed",
            "security": "allowed",
            "inspect": "allowed",
            "ai": "allowed",
            "machine": "forbidden",
            "primitive": "forbidden",
            "kernel": "forbidden",
            "basal_nb": "not_touched"
        },
        "next": {
            "unified_doctors": "planned_after_local",
            "pointer_model": "planned_after_unified_doctors"
        }
    })
}

fn local_surface_manifest_payload() -> serde_json::Value {
    serde_json::json!({
        "schema": "ncc.local.manifest.v1",
        "component": "local",
        "official_section": "local:",
        "legacy_section": "stc:",
        "status": "closed",
        "decl_kinds": [
            "var",
            "const",
            "type_alias",
            "type_record",
            "metadata"
        ],
        "syntax": {
            "var": "name: Type = value; | name >> Type : value;",
            "const": "const NAME: Type = value; | const NAME >> Type : value;",
            "type_alias": "type Name: BaseType;",
            "type_record": "type Name { field: Type; }",
            "metadata": "metadata key: value;"
        },
        "positive_examples": [
            "examples/local/local_basic_state.n",
            "examples/local/local_const_values.n",
            "examples/local/local_type_alias.n",
            "examples/local/local_type_record.n",
            "examples/local/local_array_with_const_len.n",
            "examples/local/local_runtime_region_ptr.n",
            "examples/local/local_visibility.n",
            "examples/local/stc_legacy_still_works.n",
            "examples/local/local_system_state_policy.n",
            "examples/local/local_macro_const_type_integration.n"
        ],
        "negative_examples": [
            "examples/local/negative/local_duplicate_symbol_fail.n",
            "examples/local/negative/local_duplicate_type_fail.n",
            "examples/local/negative/local_const_mutation_fail.n",
            "examples/local/negative/local_unknown_type_fail.n",
            "examples/local/negative/local_any_leak_fail.n",
            "examples/local/negative/local_forbidden_in_machine_fail.nm",
            "examples/local/negative/local_forbidden_in_primitive_fail.ncp",
            "examples/local/negative/local_forbidden_in_kernel_fail.n"
        ],
        "docs": [
            "docs/spec/local/local_overview.md",
            "docs/spec/local/local_vs_stc.md",
            "docs/spec/local/local_const.md",
            "docs/spec/local/local_type.md",
            "docs/spec/local/local_visibility.md",
            "docs/spec/local/local_site.md",
            "docs/spec/local/local_mode_gates.md",
            "docs/spec/local/local_diagnostics.md"
        ]
    })
}

pub(crate) fn run_local_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let json = args.iter().any(|arg| arg == "--json");
    let sub = args
        .iter()
        .find(|arg| arg.as_str() != "--json")
        .map(|s| s.as_str())
        .unwrap_or("doctor");
    for arg in args {
        if arg != "--json" && arg != sub {
            return Err(format!("local: unknown option '{}'", arg).into());
        }
    }
    match sub {
        "doctor" => {
            let payload = local_surface_doctor_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("local doctor: closed");
                println!("local: official");
                println!("stc: legacy alias preserved");
                println!("mode gates: machine/primitive/kernel forbidden; basal not touched");
            }
        }
        "manifest" => {
            let payload = local_surface_manifest_payload();
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("local manifest: closed");
                println!("official: local:");
                println!("legacy: stc:");
                println!("decls: var, const, type alias, type record, metadata");
            }
        }
        "-h" | "--help" | "help" => {
            if let Some(help) = command_help("local") {
                println!("{help}");
            }
        }
        other => {
            return Err(format!(
                "local: unknown subcommand '{}'; expected doctor or manifest",
                other
            )
            .into());
        }
    }
    Ok(())
}
