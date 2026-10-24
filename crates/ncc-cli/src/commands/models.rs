use super::super::*;

pub(crate) fn run_toolchain_model_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("toolchain-model").expect("toolchain-model help must exist")
        );
        return Ok(());
    }
    let mut mode = "json";
    for arg in args {
        match arg.as_str() {
            "--json" => mode = "json",
            "--list" => mode = "list",
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("toolchain-model").expect("toolchain-model help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "toolchain-model",
                    format!(
                        "toolchain-model: unknown option '{}'; expected --json or --list",
                        other
                    ),
                )
                .into());
            }
        }
    }

    let model = build_toolchain_model_manifest();
    match mode {
        "json" => {
            let rendered = serde_json::to_string_pretty(&model)?;
            println!("{}", rendered);
        }
        "list" => {
            let rows = model
                .get("toolchains")
                .and_then(|v| v.as_array())
                .ok_or("toolchain-model internal error: toolchains array missing")?;
            for row in rows {
                if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
                    println!("{}", id);
                }
            }
        }
        _ => unreachable!("toolchain-model mode is fully validated"),
    }

    Ok(())
}

pub(crate) fn run_phase10_toolchain_model_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("phase10-toolchain-model")
                .expect("phase10-toolchain-model help must exist")
        );
        return Ok(());
    }
    let mut mode = "json";
    for arg in args {
        match arg.as_str() {
            "--json" => mode = "json",
            "--list" => mode = "list",
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("phase10-toolchain-model")
                        .expect("phase10-toolchain-model help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "phase10-toolchain-model",
                    format!(
                        "phase10-toolchain-model: unknown option '{}'; expected --json or --list",
                        other
                    ),
                )
                .into());
            }
        }
    }

    let model = build_phase10_toolchain_model_manifest();
    match mode {
        "json" => {
            let rendered = serde_json::to_string_pretty(&model)?;
            println!("{}", rendered);
        }
        "list" => {
            let rows = model
                .get("toolchains")
                .and_then(|v| v.as_array())
                .ok_or("phase10-toolchain-model internal error: toolchains array missing")?;
            for row in rows {
                if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
                    println!("{}", id);
                }
            }
        }
        _ => unreachable!("phase10-toolchain-model mode is fully validated"),
    }

    Ok(())
}

pub(crate) fn run_stdlib_model_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("stdlib-model").expect("stdlib-model help must exist")
        );
        return Ok(());
    }
    let mut mode = "json";
    for arg in args {
        match arg.as_str() {
            "--json" => mode = "json",
            "--list" => mode = "list",
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("stdlib-model").expect("stdlib-model help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "stdlib-model",
                    format!(
                        "stdlib-model: unknown option '{}'; expected --json or --list",
                        other
                    ),
                )
                .into());
            }
        }
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (mut model, resolution_source, resolution_toolchain_id, resolution_manifest_path) =
        match resolve_toolchain_runtime_stdlib_manifest(&cwd) {
            Ok(Some((value, toolchain_id, manifest_path))) => (
                value,
                "toolchain_installation".to_string(),
                Some(toolchain_id),
                Some(manifest_path.to_string_lossy().to_string()),
            ),
            Ok(None) => (
                build_runtime_stdlib_manifest(),
                "embedded_fallback".to_string(),
                None,
                None,
            ),
            Err(_) => (
                build_runtime_stdlib_manifest(),
                "embedded_fallback".to_string(),
                None,
                None,
            ),
        };
    model["resolution"] = serde_json::json!({
        "source": resolution_source,
        "toolchain_id": resolution_toolchain_id,
        "manifest_path": resolution_manifest_path,
    });
    match mode {
        "json" => {
            let rendered = serde_json::to_string_pretty(&model)?;
            println!("{}", rendered);
        }
        "list" => {
            let modules = model
                .get("modules")
                .and_then(|v| v.as_array())
                .ok_or("stdlib-model internal error: modules array missing")?;
            for module in modules {
                if let Some(name) = module.as_str() {
                    println!("{}", name);
                }
            }
        }
        _ => unreachable!("stdlib-model mode is fully validated"),
    }

    Ok(())
}

fn resolve_toolchain_runtime_stdlib_manifest(
    cwd: &Path,
) -> Result<Option<(serde_json::Value, String, PathBuf)>, Box<dyn std::error::Error>> {
    let home_raw = match env::var("NCUP_HOME") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };
    let home = PathBuf::from(home_raw);
    if !home.exists() || !home.is_dir() {
        return Ok(None);
    }
    let state_path = ncup_state_path(&home);
    if !state_path.exists() || !state_path.is_file() {
        return Ok(None);
    }
    let state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
    let toolchain_id = if let Ok(raw) = env::var(NCUP_ACTIVE_TOOLCHAIN_ID_ENV) {
        if raw.trim().is_empty() {
            None
        } else {
            Some(resolve_channel_or_toolchain_id(raw.trim())?)
        }
    } else {
        None
    };
    let resolved_toolchain_id = if let Some(explicit) = toolchain_id {
        explicit
    } else {
        let (resolved, _, _) = resolve_effective_toolchain(&state, cwd)?;
        resolved
    };
    if find_installed_toolchain_index(&state, &resolved_toolchain_id).is_none() {
        return Ok(None);
    }
    let manifest_path = home
        .join("toolchains")
        .join(&resolved_toolchain_id)
        .join(NCUP_RUNTIME_STDLIB_MANIFEST_FILE);
    if !manifest_path.exists() || !manifest_path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&manifest_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(Some((value, resolved_toolchain_id, manifest_path)))
}

pub(crate) fn run_backend_model_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("backend-model").expect("backend-model help must exist")
        );
        return Ok(());
    }
    let mut mode = "json";
    let mut industrial_only = false;
    let mut host_smoke = false;
    let mut simd_smoke = false;
    for arg in args {
        match arg.as_str() {
            "--json" => mode = "json",
            "--list" => mode = "list",
            "--industrial" => industrial_only = true,
            "--host-smoke" => {
                host_smoke = true;
                industrial_only = true;
            }
            "--simd-smoke" => {
                simd_smoke = true;
                industrial_only = true;
            }
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("backend-model").expect("backend-model help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "backend-model",
                    format!(
                        "backend-model: unknown option '{}'; expected --json, --list, --industrial, --host-smoke or --simd-smoke",
                        other
                    ),
                )
                .into());
            }
        }
    }
    if host_smoke && simd_smoke {
        return Err(actionable_command_error(
            "backend-model",
            "backend-model: choose one smoke mode: --host-smoke or --simd-smoke",
        )
        .into());
    }

    let model = if simd_smoke {
        if mode == "list" {
            build_backend_simd_host_smoke_plan_manifest()
        } else {
            build_backend_simd_host_smoke_report()
        }
    } else if host_smoke {
        if mode == "list" {
            build_backend_atomic_host_smoke_plan_manifest()
        } else {
            build_backend_atomic_host_smoke_report()
        }
    } else {
        build_backend_model_manifest()
    };
    let model = if industrial_only && !host_smoke && !simd_smoke {
        model
            .get("industrial_backend")
            .cloned()
            .ok_or("backend-model internal error: industrial_backend object missing")?
    } else {
        model
    };
    match mode {
        "json" => {
            let rendered = serde_json::to_string_pretty(&model)?;
            println!("{}", rendered);
        }
        "list" => {
            if simd_smoke {
                let rows = model
                    .get("rows")
                    .and_then(|value| value.as_array())
                    .ok_or("backend-model internal error: SIMD smoke rows array missing")?;
                for row in rows {
                    let id = row
                        .get("row_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let target = row
                        .get("target")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let feature = row
                        .get("feature")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let op = row
                        .get("op")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let lane = row
                        .get("lane")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    println!("{id}\t{target}\t{feature}\t{lane}\t{op}");
                }
            } else if host_smoke {
                let rows = model
                    .get("rows")
                    .and_then(|value| value.as_array())
                    .ok_or("backend-model internal error: host smoke rows array missing")?;
                for row in rows {
                    let id = row
                        .get("row_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let target = row
                        .get("target")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let op = row
                        .get("op")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let width = row
                        .get("width")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    let ordering = row
                        .get("ordering")
                        .and_then(|value| value.as_str())
                        .unwrap_or("-");
                    println!("{id} -> {target} {op} {width} {ordering}");
                }
                return Ok(());
            }
            if industrial_only {
                let object = model
                    .as_object()
                    .ok_or("backend-model internal error: industrial model object missing")?;
                for key in object.keys() {
                    println!("{key}");
                }
                return Ok(());
            }
            let rows = model
                .get("profiles")
                .and_then(|v| v.as_array())
                .ok_or("backend-model internal error: profiles array missing")?;
            for row in rows {
                let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                let triple = row
                    .get("target_triple")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let tier = row.get("tier").and_then(|v| v.as_str()).unwrap_or("-");
                println!("{} -> {} [{}]", id, triple, tier);
            }
        }
        _ => unreachable!("backend-model mode is fully validated"),
    }

    Ok(())
}

pub(crate) fn run_release_train_model_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("release-train-model").expect("release-train-model help must exist")
        );
        return Ok(());
    }
    let mut mode = "json";
    for arg in args {
        match arg.as_str() {
            "--json" => mode = "json",
            "--list" => mode = "list",
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("release-train-model")
                        .expect("release-train-model help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "release-train-model",
                    format!(
                        "release-train-model: unknown option '{}'; expected --json or --list",
                        other
                    ),
                )
                .into());
            }
        }
    }

    let model = build_release_train_model_manifest();
    match mode {
        "json" => {
            let rendered = serde_json::to_string_pretty(&model)?;
            println!("{}", rendered);
        }
        "list" => {
            let rows = model
                .get("release_train")
                .and_then(|v| v.get("channels"))
                .and_then(|v| v.as_array())
                .ok_or("release-train-model internal error: release_train.channels missing")?;
            for row in rows {
                let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                let cadence = row
                    .get("cadence_days")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let support = row
                    .get("support_model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let breaking_allowed = row
                    .get("breaking_allowed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                println!(
                    "{} cadence={}d support={} breaking_allowed={}",
                    name, cadence, support, breaking_allowed
                );
            }
        }
        _ => unreachable!("release-train-model mode is fully validated"),
    }

    Ok(())
}
