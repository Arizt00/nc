use super::super::*;

pub(crate) fn run_ncup_init_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut explicit_home: Option<String> = None;
    let mut default_toolchain: Option<String> = None;
    let mut toolchain_channel: Option<String> = None;
    let mut toolchain_target: Option<String> = None;
    let mut profile = "default".to_string();
    let mut profile_explicit = false;
    let mut enterprise_mirror: Option<String> = None;
    let mut enterprise_trust_key: Option<String> = None;
    let mut force = false;
    let mut no_prompt = false;
    let mut skip_first_run = false;
    let mut json_mode = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--home" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --home requires a path",
                    )
                })?;
                explicit_home = Some(value.clone());
                idx += 2;
            }
            "--default-toolchain" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --default-toolchain requires an id",
                    )
                })?;
                default_toolchain = Some(value.clone());
                idx += 2;
            }
            "--channel" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --channel requires <stable|beta|nightly>",
                    )
                })?;
                toolchain_channel = Some(value.clone());
                idx += 2;
            }
            "--target" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --target requires <triple>",
                    )
                })?;
                toolchain_target = Some(value.clone());
                idx += 2;
            }
            "--profile" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --profile requires a value",
                    )
                })?;
                profile = value.clone();
                profile_explicit = true;
                idx += 2;
            }
            "--enterprise-mirror" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --enterprise-mirror requires a value",
                    )
                })?;
                enterprise_mirror = Some(value.clone());
                idx += 2;
            }
            "--enterprise-trust-key" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "ncup-init",
                        "ncup-init syntax: --enterprise-trust-key requires a value",
                    )
                })?;
                enterprise_trust_key = Some(value.clone());
                idx += 2;
            }
            "--force" => {
                force = true;
                idx += 1;
            }
            "--yes" | "--non-interactive" => {
                no_prompt = true;
                idx += 1;
            }
            "--skip-first-run" => {
                skip_first_run = true;
                idx += 1;
            }
            "--json" => {
                json_mode = true;
                idx += 1;
            }
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("ncup-init").expect("ncup-init help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "ncup-init",
                    format!("ncup-init: unknown option '{}'", other),
                )
                .into());
            }
        }
    }

    let interactive_wizard_available =
        !json_mode && !no_prompt && io::stdin().is_terminal() && io::stdout().is_terminal();
    let mut interactive_wizard_used = false;
    if interactive_wizard_available {
        let default_toolchain_id = default_phase10_toolchain_id();
        let (default_channel, default_target) = split_toolchain_id(&default_toolchain_id)
            .unwrap_or_else(|| ("stable".to_string(), "x86_64-unknown-linux-gnu".to_string()));

        if !profile_explicit {
            let selected =
                prompt_ncup_init_value("ncup-init profile (minimal|default|complete)", &profile)?;
            profile = selected;
            interactive_wizard_used = true;
        }
        if default_toolchain.is_none() {
            if toolchain_channel.is_none() {
                let selected = prompt_ncup_init_value(
                    "ncup-init channel (stable|beta|nightly)",
                    &default_channel,
                )?;
                toolchain_channel = Some(selected);
                interactive_wizard_used = true;
            }
            if toolchain_target.is_none() {
                let selected =
                    prompt_ncup_init_value("ncup-init default target triple", &default_target)?;
                toolchain_target = Some(selected);
                interactive_wizard_used = true;
            }
        }
    }

    validate_ncup_profile(&profile)?;
    let home = resolve_ncup_home(explicit_home.as_deref());
    ensure_ncup_layout(&home)?;
    write_ncup_supply_chain_root_trust(&home)?;
    let enterprise_meta = configure_ncup_enterprise_trust(
        &home,
        enterprise_mirror.as_deref(),
        enterprise_trust_key.as_deref(),
    )?;

    let chosen_toolchain = resolve_ncup_init_toolchain_id(
        default_toolchain.as_deref(),
        toolchain_channel.as_deref(),
        toolchain_target.as_deref(),
    )?;

    let state_path = ncup_state_path(&home);
    if state_path.exists() && !force {
        return Err(actionable_command_error(
            "ncup-init",
            format!(
                "ncup-init: '{}' already exists; rerun with --force to rebootstrap",
                state_path.to_string_lossy()
            ),
        )
        .into());
    }

    let mut state = default_ncup_state(&home, &chosen_toolchain);
    let init_install_result: Result<(), Box<dyn std::error::Error>> = (|| {
        let bin_dir = install_toolchain_shims(&home, &chosen_toolchain, &profile)?;
        upsert_toolchain_state_entry(&mut state, &chosen_toolchain, &profile, &bin_dir);
        state["default_toolchain"] = serde_json::Value::String(chosen_toolchain.clone());
        save_ncup_state(&home, &state)?;
        sync_default_toolchain_bin_dir(&home, &chosen_toolchain)?;
        Ok(())
    })();
    if let Err(err) = init_install_result {
        record_ncup_install_telemetry(
            &home,
            "ncup-init",
            "failure",
            Some(&chosen_toolchain),
            Some(&profile),
            Some(&err.to_string()),
        );
        return Err(err);
    }

    let config = serde_json::json!({
        "schema": "ncup.config.v1",
        "version": "2.0.0",
        "state": "frozen",
        "default_toolchain": chosen_toolchain,
        "profile": profile,
        "mode_policy": build_phase10_mode_policy_manifest(),
        "managed_bins": official_phase10_tool_bins(),
        "layout_dirs": NCUP_LAYOUT_DIRS_OFFICIAL,
        "project_override_file": NCUP_PROJECT_OVERRIDE_FILE,
        "supply_chain": {
            "schema": "ncup.supply.chain.v1",
            "version": "2.0.0",
            "state": "frozen",
            "verify_before_install": true,
            "hash_algorithm": NCUP_SUPPLY_CHAIN_HASH_ALGO,
            "signature_algorithm": NCUP_SUPPLY_CHAIN_SIGN_ALGO,
            "root_key_id": NCUP_SUPPLY_CHAIN_ROOT_KEY_ID,
            "root_key_fingerprint_sha256": sha256_hex_string(NCUP_SUPPLY_CHAIN_ROOT_KEY_MATERIAL),
            "embedded_root_key": true,
            "enterprise": {
                "enabled": enterprise_meta.is_some(),
                "mirror": enterprise_meta
                    .as_ref()
                    .and_then(|v| v.get("mirror"))
                    .and_then(|v| v.as_str()),
                "trust_key_id": enterprise_meta
                    .as_ref()
                    .and_then(|v| v.get("key_id"))
                    .and_then(|v| v.as_str()),
                "trust_key_env_override": NCUP_ENTERPRISE_TRUST_KEY_ENV
            }
        },
        "telemetry": {
            "schema": "ncup.telemetry.policy.v1",
            "version": "2.0.0",
            "state": "mutable",
            "opt_in": false,
            "mode": "opt-in-only",
            "metrics": ["install_outcome", "version", "target"],
            "never_collect": ["source_code", "source_paths", "file_contents"]
        }
    });
    fs::write(
        ncup_config_path(&home),
        serde_json::to_string_pretty(&config)?,
    )?;

    record_ncup_install_telemetry(
        &home,
        "ncup-init",
        "success",
        Some(&chosen_toolchain),
        Some(&profile),
        None,
    );

    let doctor_summary = run_ncup_doctor_for_home(&home, !json_mode)?;
    let first_run_demo = run_ncup_init_first_run_demo(&home, &chosen_toolchain, skip_first_run)?;
    let post_install_guide = vec![
        format!("source {}", home.join("env.sh").to_string_lossy()),
        "ncup doctor".to_string(),
        "ncc run ~/.ncup/tmp/first-run-demo/hello.n".to_string(),
        "ncup toolchain list --available".to_string(),
    ];

    let summary = serde_json::json!({
        "schema": "ncup.init.first-run.v1",
        "version": "2.0.0",
        "status": "ok",
        "home": home.to_string_lossy().to_string(),
        "default_toolchain": state.get("default_toolchain"),
        "profile": profile,
        "state_file": ncup_state_path(&home).to_string_lossy().to_string(),
        "env_script": home.join("env.sh").to_string_lossy().to_string(),
        "env_script_windows": home.join("env.bat").to_string_lossy().to_string(),
        "layout_dirs": NCUP_LAYOUT_DIRS_OFFICIAL,
        "project_override_file": NCUP_PROJECT_OVERRIDE_FILE,
        "supply_chain_root_key_id": NCUP_SUPPLY_CHAIN_ROOT_KEY_ID,
        "enterprise_mode": enterprise_meta.is_some(),
        "enterprise_mirror": enterprise_meta
            .as_ref()
            .and_then(|v| v.get("mirror"))
            .and_then(|v| v.as_str()),
        "first_run": {
            "interactive_wizard_used": interactive_wizard_used,
            "selected_channel": split_toolchain_id(&chosen_toolchain).map(|(ch, _)| ch),
            "selected_target": split_toolchain_id(&chosen_toolchain).map(|(_, tgt)| tgt),
            "doctor": doctor_summary,
            "demo": first_run_demo,
            "post_install_guide": post_install_guide,
        }
    });
    if json_mode {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!(
            "OK: ncup-init home={} default_toolchain={} profile={}",
            home.to_string_lossy(),
            summary
                .get("default_toolchain")
                .and_then(|v| v.as_str())
                .unwrap_or("-"),
            summary
                .get("profile")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
        );
        println!(
            "hint: source {} to activate PATH in current shell",
            home.join("env.sh").to_string_lossy()
        );
        println!("hint: first-run doctor and demo validation completed");
        println!("post-install guide:");
        println!("  1. source {}", home.join("env.sh").to_string_lossy());
        println!("  2. ncup doctor");
        println!(
            "  3. ncc run {}",
            home.join("tmp")
                .join("first-run-demo")
                .join("hello.n")
                .to_string_lossy()
        );
        println!("  4. ncup toolchain list --available");
    }
    Ok(())
}

pub(crate) fn run_ncup_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("ncup").expect("ncup help must exist"));
        return Ok(());
    }
    let default_args;
    let args = if args.is_empty() {
        default_args = ncup_default_update_args();
        default_args.as_slice()
    } else {
        args
    };
    let action = args
        .first()
        .expect("ncup default invocation always supplies an action");

    match action.as_str() {
        "init" => {
            run_ncup_init_command(&args[1..])?;
        }
        "home" => {
            let home = resolve_ncup_home(None);
            println!("{}", home.to_string_lossy());
        }
        "model" => {
            let mut list_mode = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--json" => list_mode = false,
                    "--list" => list_mode = true,
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!(
                                "ncup model: unknown option '{}'; expected --json or --list",
                                other
                            ),
                        )
                        .into());
                    }
                }
            }
            let model = build_phase10_installer_manifest();
            if list_mode {
                for bin in official_phase10_tool_bins() {
                    println!("{}", bin);
                }
            } else {
                println!("{}", serde_json::to_string_pretty(&model)?);
            }
        }
        "install" => {
            let mut explicit_home: Option<String> = None;
            let mut profile = "default".to_string();
            let mut set_default = false;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup install: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--profile" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup install: --profile requires a value",
                            )
                        })?;
                        profile = value.clone();
                        i += 2;
                    }
                    "--set-default" => {
                        set_default = true;
                        i += 1;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup install: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error("ncup", "ncup install <stable|beta|nightly|toolchain-id> [--profile <minimal|default|complete>] [--set-default]").into());
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            validate_ncup_profile(&profile)?;
            let toolchain_id = resolve_channel_or_toolchain_id(&positional[0])?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let install_result: Result<(), Box<dyn std::error::Error>> = (|| {
                let bin_dir = install_toolchain_shims(&home, &toolchain_id, &profile)?;
                upsert_toolchain_state_entry(&mut state, &toolchain_id, &profile, &bin_dir);
                if set_default {
                    state["default_toolchain"] = serde_json::Value::String(toolchain_id.clone());
                    sync_default_toolchain_bin_dir(&home, &toolchain_id)?;
                }
                save_ncup_state(&home, &state)?;
                Ok(())
            })();
            if let Err(err) = install_result {
                record_ncup_install_telemetry(
                    &home,
                    "ncup install",
                    "failure",
                    Some(&toolchain_id),
                    Some(&profile),
                    Some(&err.to_string()),
                );
                return Err(err);
            }
            record_ncup_install_telemetry(
                &home,
                "ncup install",
                "success",
                Some(&toolchain_id),
                Some(&profile),
                None,
            );
            println!(
                "OK: ncup installed {} profile={} home={}",
                toolchain_id,
                profile,
                home.to_string_lossy()
            );
        }
        "default" => {
            let mut explicit_home: Option<String> = None;
            let mut profile = "default".to_string();
            let mut positional: Vec<String> = Vec::new();
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup default: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--profile" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup default: --profile requires a value",
                            )
                        })?;
                        profile = value.clone();
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup default: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error(
                    "ncup",
                    "ncup default <stable|beta|nightly|toolchain-id>",
                )
                .into());
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            validate_ncup_profile(&profile)?;
            let toolchain_id = resolve_channel_or_toolchain_id(&positional[0])?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            ensure_installed_toolchain(&home, &mut state, &toolchain_id, &profile)?;
            state["default_toolchain"] = serde_json::Value::String(toolchain_id.clone());
            save_ncup_state(&home, &state)?;
            sync_default_toolchain_bin_dir(&home, &toolchain_id)?;
            println!("OK: ncup default toolchain {}", toolchain_id);
        }
        "toolchain" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error(
                    "ncup",
                    "ncup toolchain syntax: toolchain <list|install|default|which> [args]",
                )
            })?;
            let mut explicit_home: Option<String> = None;
            let mut profile = "default".to_string();
            let mut set_default = false;
            let mut channel: Option<String> = None;
            let mut target: Option<String> = None;
            let mut explicit_cwd: Option<String> = None;
            let mut list_available = false;
            let mut require_component: Option<String> = None;
            let mut json_output = false;
            let mut positional: Vec<String> = Vec::new();

            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--profile" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --profile requires a value",
                            )
                        })?;
                        profile = value.clone();
                        i += 2;
                    }
                    "--set-default" => {
                        set_default = true;
                        i += 1;
                    }
                    "--channel" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --channel requires a value",
                            )
                        })?;
                        channel = Some(value.clone());
                        i += 2;
                    }
                    "--target" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --target requires a value",
                            )
                        })?;
                        target = Some(value.clone());
                        i += 2;
                    }
                    "--cwd" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --cwd requires a path",
                            )
                        })?;
                        explicit_cwd = Some(value.clone());
                        i += 2;
                    }
                    "--available" => {
                        list_available = true;
                        i += 1;
                    }
                    "--require-component" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup toolchain: --require-component requires a value",
                            )
                        })?;
                        require_component = Some(value.clone());
                        i += 2;
                    }
                    "--json" => {
                        json_output = true;
                        i += 1;
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup toolchain: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;

            match sub.as_str() {
                "list" => {
                    if !positional.is_empty() {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain list does not accept positional arguments",
                        )
                        .into());
                    }
                    if set_default || channel.is_some() || target.is_some() || profile != "default"
                    {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain list accepts only --home and optional --available",
                        )
                        .into());
                    }
                    if explicit_cwd.is_some() {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain list does not accept --cwd",
                        )
                        .into());
                    }
                    if require_component.is_some() || json_output {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain list does not accept --require-component or --json",
                        )
                        .into());
                    }
                    let state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
                    let default_toolchain = state
                        .get("default_toolchain")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let installed_rows = state
                        .get("installed_toolchains")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let mut installed_ids: std::collections::BTreeSet<String> =
                        std::collections::BTreeSet::new();
                    for row in &installed_rows {
                        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
                            installed_ids.insert(id.to_string());
                        }
                    }
                    if list_available {
                        for id in official_toolchain_ids() {
                            if id == default_toolchain {
                                println!("* {} [default]", id);
                            } else if installed_ids.contains(&id) {
                                println!("= {} [installed]", id);
                            } else {
                                println!("  {}", id);
                            }
                        }
                        return Ok(());
                    }
                    for row in installed_rows {
                        let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        if id == default_toolchain {
                            println!("* {}", id);
                        } else {
                            println!("  {}", id);
                        }
                    }
                }
                "install" => {
                    if list_available {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain install does not accept --available",
                        )
                        .into());
                    }
                    let toolchain_id = resolve_toolchain_install_id(
                        &positional,
                        channel.as_deref(),
                        target.as_deref(),
                    )?;
                    if explicit_cwd.is_some() {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain install does not accept --cwd",
                        )
                        .into());
                    }
                    if require_component.is_some() || json_output {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain install does not accept --require-component or --json",
                        )
                        .into());
                    }
                    validate_toolchain_id_known(&toolchain_id)?;
                    validate_ncup_profile(&profile)?;
                    let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
                    let install_result: Result<(), Box<dyn std::error::Error>> = (|| {
                        let bin_dir = install_toolchain_shims(&home, &toolchain_id, &profile)?;
                        upsert_toolchain_state_entry(&mut state, &toolchain_id, &profile, &bin_dir);
                        if set_default {
                            state["default_toolchain"] =
                                serde_json::Value::String(toolchain_id.clone());
                            sync_default_toolchain_bin_dir(&home, &toolchain_id)?;
                        }
                        save_ncup_state(&home, &state)?;
                        Ok(())
                    })(
                    );
                    if let Err(err) = install_result {
                        record_ncup_install_telemetry(
                            &home,
                            "ncup toolchain install",
                            "failure",
                            Some(&toolchain_id),
                            Some(&profile),
                            Some(&err.to_string()),
                        );
                        return Err(err);
                    }
                    record_ncup_install_telemetry(
                        &home,
                        "ncup toolchain install",
                        "success",
                        Some(&toolchain_id),
                        Some(&profile),
                        None,
                    );
                    println!(
                        "OK: ncup installed {} profile={} home={}",
                        &toolchain_id,
                        profile,
                        home.to_string_lossy()
                    );
                }
                "default" => {
                    if set_default || channel.is_some() || target.is_some() || list_available {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain default only accepts --home and positional <id>",
                        )
                        .into());
                    }
                    if explicit_cwd.is_some() {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain default does not accept --cwd",
                        )
                        .into());
                    }
                    if require_component.is_some() || json_output {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain default does not accept --require-component or --json",
                        )
                        .into());
                    }
                    if positional.len() > 1 {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain default accepts a single positional <id>",
                        )
                        .into());
                    }
                    let toolchain_id = positional.first().ok_or_else(|| {
                        actionable_command_error("ncup", "ncup toolchain default <id>")
                    })?;
                    let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
                    let installed = state
                        .get("installed_toolchains")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if !installed
                        .iter()
                        .any(|row| row.get("id").and_then(|v| v.as_str()) == Some(toolchain_id))
                    {
                        return Err(format!(
                            "ncup: toolchain '{}' is not installed; run `ncup toolchain install {}` first",
                            toolchain_id, toolchain_id
                        )
                        .into());
                    }
                    state["default_toolchain"] = serde_json::Value::String(toolchain_id.clone());
                    save_ncup_state(&home, &state)?;
                    sync_default_toolchain_bin_dir(&home, toolchain_id)?;
                    println!("OK: ncup default toolchain {}", toolchain_id);
                }
                "which" => {
                    if set_default || channel.is_some() || target.is_some() || list_available {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain which only accepts --home, optional --cwd, optional --require-component, optional --json and optional positional <id>",
                        )
                        .into());
                    }
                    if positional.len() > 1 {
                        return Err(actionable_command_error(
                            "ncup",
                            "ncup toolchain which accepts at most one positional <id>",
                        )
                        .into());
                    }
                    let state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
                    let cwd = if let Some(raw) = explicit_cwd.as_deref() {
                        PathBuf::from(raw)
                    } else {
                        env::current_dir()?
                    };
                    let (toolchain_id, resolution_source, resolution_detail) =
                        if let Some(value) = positional.first() {
                            (
                                resolve_channel_or_toolchain_id(value)?,
                                "explicit".to_string(),
                                None,
                            )
                        } else {
                            resolve_effective_toolchain(&state, &cwd)?
                        };
                    ensure_toolchain_present_in_state(&state, &toolchain_id)?;
                    if let Some(required_component) = require_component.as_deref() {
                        ensure_toolchain_has_component(&state, &toolchain_id, required_component)?;
                    }
                    let path = home.join("toolchains").join(&toolchain_id).join("bin");
                    if json_output {
                        let payload = serde_json::json!({
                            "schema": "ncup.toolchain.resolution.v1",
                            "version": "2.0.0",
                            "state": "frozen",
                            "toolchain_id": toolchain_id,
                            "source": resolution_source,
                            "source_detail": resolution_detail,
                            "bin_dir": path.to_string_lossy().to_string(),
                            "cwd": cwd.to_string_lossy().to_string(),
                            "project_override_file": NCUP_PROJECT_OVERRIDE_FILE,
                            "env_var": NCUP_TOOLCHAIN_ENV,
                            "resolution_order": ["local_override", "env_var", "global_default"]
                        });
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                    } else {
                        println!("{}", path.to_string_lossy());
                    }
                }
                other => {
                    return Err(actionable_command_error(
                        "ncup",
                        format!("ncup toolchain: unknown action '{}'", other),
                    )
                    .into());
                }
            }
        }
        "which" => {
            let mut explicit_home: Option<String> = None;
            let mut explicit_cwd: Option<String> = None;
            let mut explicit_toolchain: Option<String> = None;
            let mut json_output = false;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup which: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--cwd" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup which: --cwd requires a path")
                        })?;
                        explicit_cwd = Some(value.clone());
                        i += 2;
                    }
                    "--toolchain" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup which: --toolchain requires a value",
                            )
                        })?;
                        explicit_toolchain = Some(value.clone());
                        i += 2;
                    }
                    "--json" => {
                        json_output = true;
                        i += 1;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup which: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error(
                    "ncup",
                    "ncup which <bin> [--toolchain <stable|beta|nightly|toolchain-id>] [--cwd <path>] [--home <path>] [--json]",
                )
                .into());
            }
            let managed_bin = positional.first().cloned().unwrap_or_default();
            validate_managed_bin_name(&managed_bin)?;
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let cwd = if let Some(raw) = explicit_cwd.as_deref() {
                PathBuf::from(raw)
            } else {
                env::current_dir()?
            };
            let (toolchain_id, resolution_source, resolution_detail) =
                if let Some(raw) = explicit_toolchain.as_deref() {
                    (
                        resolve_channel_or_toolchain_id(raw)?,
                        "explicit".to_string(),
                        None,
                    )
                } else {
                    resolve_effective_toolchain(&state, &cwd)?
                };
            ensure_toolchain_present_in_state(&state, &toolchain_id)?;
            if let Some(component) = component_for_managed_bin(&managed_bin) {
                ensure_toolchain_has_component(&state, &toolchain_id, component)?;
            }
            let bin_path = home.join("toolchains").join(&toolchain_id).join("bin");
            let bin_path = ncup_managed_bin_path(&bin_path, &managed_bin);
            if !bin_path.exists() || !bin_path.is_file() {
                return Err(format!(
                    "ncup: managed bin '{}' is missing for toolchain '{}' at {}; run `ncup update --home {}`",
                    managed_bin,
                    toolchain_id,
                    bin_path.to_string_lossy(),
                    home.to_string_lossy()
                )
                .into());
            }
            if json_output {
                let payload = serde_json::json!({
                    "schema": "ncup.which.v1",
                    "version": "2.0.0",
                    "state": "frozen",
                    "bin": managed_bin,
                    "path": bin_path.to_string_lossy().to_string(),
                    "toolchain_id": toolchain_id,
                    "source": resolution_source,
                    "source_detail": resolution_detail,
                    "cwd": cwd.to_string_lossy().to_string(),
                    "project_override_file": NCUP_PROJECT_OVERRIDE_FILE,
                    "env_var": NCUP_TOOLCHAIN_ENV,
                    "resolution_order": ["local_override", "env_var", "global_default"],
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{}", bin_path.to_string_lossy());
            }
        }
        "target" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error("ncup", "ncup target syntax: target add <triple> [--toolchain <stable|beta|nightly|toolchain-id>] [--home <path>]")
            })?;
            if sub.as_str() != "add" {
                return Err(actionable_command_error(
                    "ncup",
                    format!("ncup target: unknown action '{}'; expected add", sub),
                )
                .into());
            }
            let mut explicit_home: Option<String> = None;
            let mut explicit_toolchain: Option<String> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup target add: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--toolchain" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup target add: --toolchain requires a value",
                            )
                        })?;
                        explicit_toolchain = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup target add: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error("ncup", "ncup target add <triple>").into());
            }
            let target_triple = positional[0].clone();
            validate_toolchain_target_known(&target_triple)?;
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let cwd = env::current_dir()?;
            let toolchain_id = if let Some(raw) = explicit_toolchain.as_deref() {
                resolve_channel_or_toolchain_id(raw)?
            } else {
                let (resolved, _, _) = resolve_effective_toolchain(&state, &cwd)?;
                resolved
            };
            if let Some((channel, _)) = split_toolchain_id(&toolchain_id) {
                validate_target_supported_by_channel(
                    &channel,
                    &target_triple,
                    &format!("toolchain '{}'", toolchain_id),
                )?;
            }
            ensure_installed_toolchain(&home, &mut state, &toolchain_id, "default")?;
            let idx = find_installed_toolchain_index(&state, &toolchain_id).ok_or_else(|| {
                format!(
                    "ncup: toolchain '{}' missing from state after install",
                    toolchain_id
                )
            })?;
            let rows = state
                .get_mut("installed_toolchains")
                .and_then(|v| v.as_array_mut())
                .ok_or_else(|| "ncup: installed_toolchains state is invalid".to_string())?;
            let row = rows.get_mut(idx).ok_or_else(|| {
                format!("ncup: installed toolchain row index {} out of bounds", idx)
            })?;
            let mut merged_targets: BTreeSet<String> = row
                .get("targets")
                .and_then(|v| v.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<BTreeSet<String>>()
                })
                .unwrap_or_default();
            merged_targets.insert(target_triple.clone());
            row["targets"] = serde_json::Value::Array(
                merged_targets
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            );
            let target_stamp = home.join("toolchains").join(&toolchain_id).join("targets");
            fs::create_dir_all(&target_stamp)?;
            fs::write(
                target_stamp.join(format!("{}.enabled", target_triple)),
                b"enabled\n",
            )?;
            save_ncup_state(&home, &state)?;
            println!(
                "OK: ncup target add {} toolchain={}",
                target_triple, toolchain_id
            );
        }
        "component" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error(
                    "ncup",
                    "ncup component syntax: component add <name> [--toolchain <stable|beta|nightly|toolchain-id>] [--home <path>]",
                )
            })?;
            if sub.as_str() != "add" {
                return Err(actionable_command_error(
                    "ncup",
                    format!("ncup component: unknown action '{}'; expected add", sub),
                )
                .into());
            }
            let mut explicit_home: Option<String> = None;
            let mut explicit_toolchain: Option<String> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup component add: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--toolchain" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup component add: --toolchain requires a value",
                            )
                        })?;
                        explicit_toolchain = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup component add: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error("ncup", "ncup component add <name>").into());
            }
            let component = positional[0].clone();
            validate_toolchain_component_known(&component)?;
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let cwd = env::current_dir()?;
            let toolchain_id = if let Some(raw) = explicit_toolchain.as_deref() {
                resolve_channel_or_toolchain_id(raw)?
            } else {
                let (resolved, _, _) = resolve_effective_toolchain(&state, &cwd)?;
                resolved
            };
            ensure_installed_toolchain(&home, &mut state, &toolchain_id, "default")?;
            let idx = find_installed_toolchain_index(&state, &toolchain_id).ok_or_else(|| {
                format!(
                    "ncup: toolchain '{}' missing from state after install",
                    toolchain_id
                )
            })?;
            let rows = state
                .get_mut("installed_toolchains")
                .and_then(|v| v.as_array_mut())
                .ok_or_else(|| "ncup: installed_toolchains state is invalid".to_string())?;
            let row = rows.get_mut(idx).ok_or_else(|| {
                format!("ncup: installed toolchain row index {} out of bounds", idx)
            })?;
            let mut merged_components: BTreeSet<String> = row
                .get("components")
                .and_then(|v| v.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<BTreeSet<String>>()
                })
                .unwrap_or_default();
            merged_components.insert(component.clone());
            row["components"] = serde_json::Value::Array(
                merged_components
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            );
            save_ncup_state(&home, &state)?;
            println!(
                "OK: ncup component add {} toolchain={}",
                component, toolchain_id
            );
        }
        "override" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error(
                    "ncup",
                    "ncup override syntax: override set <stable|beta|nightly|toolchain-id> [--cwd <path>]",
                )
            })?;
            if sub.as_str() != "set" {
                return Err(actionable_command_error(
                    "ncup",
                    format!("ncup override: unknown action '{}'; expected set", sub),
                )
                .into());
            }
            let mut explicit_cwd: Option<String> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--cwd" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup override set: --cwd requires a path",
                            )
                        })?;
                        explicit_cwd = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup override set: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            if positional.len() != 1 {
                return Err(actionable_command_error(
                    "ncup",
                    "ncup override set <stable|beta|nightly|toolchain-id>",
                )
                .into());
            }
            let resolved = resolve_channel_or_toolchain_id(&positional[0])?;
            let cwd = if let Some(raw) = explicit_cwd.as_deref() {
                PathBuf::from(raw)
            } else {
                env::current_dir()?
            };
            fs::create_dir_all(&cwd)?;
            let override_path = cwd.join(NCUP_PROJECT_OVERRIDE_FILE);
            let payload = format!("[toolchain]\nid = \"{}\"\n", resolved);
            fs::write(&override_path, payload.as_bytes())?;
            println!(
                "OK: ncup override set {} file={}",
                resolved,
                override_path.to_string_lossy()
            );
        }
        "update" => {
            if let Some(opts) = parse_ncup_direct_update_options(args)? {
                run_ncup_direct_update_command(opts)?;
                return Ok(());
            }
            let mut explicit_home: Option<String> = None;
            let mut explicit_cwd: Option<String> = None;
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup update: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "--cwd" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup update: --cwd requires a path")
                        })?;
                        explicit_cwd = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup update: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let cwd = if let Some(raw) = explicit_cwd.as_deref() {
                PathBuf::from(raw)
            } else {
                env::current_dir()?
            };
            let (active_toolchain, resolved_from, resolved_detail) =
                resolve_effective_toolchain(&state, &cwd)?;
            ensure_toolchain_present_in_state(&state, &active_toolchain)?;

            let active_idx =
                find_installed_toolchain_index(&state, &active_toolchain).ok_or_else(|| {
                    format!(
                        "ncup: active toolchain '{}' is missing from installed state",
                        active_toolchain
                    )
                })?;
            let active_profile = state
                .get("installed_toolchains")
                .and_then(|v| v.as_array())
                .and_then(|rows| rows.get(active_idx))
                .and_then(|row| row.get("profile"))
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();

            let default_toolchain = state
                .get("default_toolchain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let toolchain_dir = home.join("toolchains").join(&active_toolchain);
            if !toolchain_dir.exists() || !toolchain_dir.is_dir() {
                return Err(format!(
                    "ncup: active toolchain '{}' directory is missing at {}",
                    active_toolchain,
                    toolchain_dir.to_string_lossy()
                )
                .into());
            }

            let before_revision = toolchain_revision_marker(&toolchain_dir)?;
            let epoch = epoch_seconds_now();
            let tx_id = format!(
                "upd-{}-{}",
                epoch,
                sanitize_for_path_component(&active_toolchain)
            );
            let snapshot_dir = home
                .join("manifests")
                .join("history")
                .join("toolchains")
                .join(sanitize_for_path_component(&active_toolchain))
                .join(&tx_id)
                .join("before");
            if snapshot_dir.exists() {
                fs::remove_dir_all(&snapshot_dir)?;
            }
            if let Some(parent) = snapshot_dir.parent() {
                fs::create_dir_all(parent)?;
            }
            copy_dir_recursive(&toolchain_dir, &snapshot_dir)?;

            let state_backup_raw = serde_json::to_string_pretty(&state)?;

            let mut tx_error: Option<String> = None;
            let tx_result: Result<(), Box<dyn std::error::Error>> = (|| {
                let bin_dir = install_toolchain_shims(&home, &active_toolchain, &active_profile)?;
                upsert_toolchain_state_entry(
                    &mut state,
                    &active_toolchain,
                    &active_profile,
                    &bin_dir,
                );
                if default_toolchain == active_toolchain {
                    sync_default_toolchain_bin_dir(&home, &active_toolchain)?;
                }
                save_ncup_state(&home, &state)?;
                Ok(())
            })();
            if let Err(err) = tx_result {
                tx_error = Some(err.to_string());
            }

            if let Some(err_message) = tx_error {
                fs::write(ncup_state_path(&home), state_backup_raw.as_bytes())?;
                restore_toolchain_from_snapshot(&home, &active_toolchain, &snapshot_dir)?;
                if default_toolchain == active_toolchain {
                    sync_default_toolchain_bin_dir(&home, &active_toolchain)?;
                }
                let rolled_back_revision =
                    toolchain_revision_marker(&home.join("toolchains").join(&active_toolchain))?;
                append_ncup_update_history_entry(
                    &home,
                    serde_json::json!({
                        "id": tx_id,
                        "timestamp_epoch": epoch,
                        "toolchain_id": active_toolchain,
                        "profile": active_profile,
                        "resolved_from": resolved_from,
                        "resolved_detail": resolved_detail,
                        "status": "rolled_back",
                        "rollback_applied": true,
                        "before_revision": before_revision,
                        "after_revision": rolled_back_revision,
                        "snapshot_dir": snapshot_dir.to_string_lossy().to_string(),
                        "error": err_message,
                    }),
                )?;
                return Err(format!(
                    "ncup update: update transaction failed for '{}' and rollback was applied; snapshot={} cause={}",
                    active_toolchain,
                    snapshot_dir.to_string_lossy(),
                    err_message
                )
                .into());
            }

            let after_revision =
                toolchain_revision_marker(&home.join("toolchains").join(&active_toolchain))?;
            append_ncup_update_history_entry(
                &home,
                serde_json::json!({
                    "id": tx_id,
                    "timestamp_epoch": epoch,
                    "toolchain_id": active_toolchain,
                    "profile": active_profile,
                    "resolved_from": resolved_from,
                    "resolved_detail": resolved_detail,
                    "status": "ok",
                    "rollback_applied": false,
                    "before_revision": before_revision,
                    "after_revision": after_revision,
                    "snapshot_dir": snapshot_dir.to_string_lossy().to_string(),
                }),
            )?;

            println!(
                "OK: ncup update home={} toolchain={} source={}",
                home.to_string_lossy(),
                active_toolchain,
                resolved_from
            );
        }
        "self" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error("ncup", "ncup self syntax: self update [--home <path>]")
            })?;
            if sub.as_str() != "update" {
                return Err(actionable_command_error(
                    "ncup",
                    format!("ncup self: unknown action '{}'; expected update", sub),
                )
                .into());
            }
            let mut explicit_home: Option<String> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup self update: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup self update: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let info = serde_json::json!({
                "schema": "ncup.self.update.v1",
                "version": "2.0.0",
                "state": "frozen",
                "home": home.to_string_lossy().to_string(),
                "updated_at_epoch": epoch_seconds_now(),
                "status": "ok",
                "mode": "managed-placeholder"
            });
            fs::write(
                home.join("manifests").join("self-update.v1.json"),
                serde_json::to_string_pretty(&info)?,
            )?;
            println!("OK: ncup self update home={}", home.to_string_lossy());
        }
        "doctor" => {
            let mut explicit_home: Option<String> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--home" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup doctor: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        idx += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup doctor: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            run_ncup_doctor_for_home(&home, true)?;
        }
        "verify" => {
            let mut explicit_home: Option<String> = None;
            let mut explicit_toolchain: Option<String> = None;
            let mut idx = 1usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--home" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error("ncup", "ncup verify: --home requires a path")
                        })?;
                        explicit_home = Some(value.clone());
                        idx += 2;
                    }
                    "--toolchain" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup verify: --toolchain requires a value",
                            )
                        })?;
                        explicit_toolchain = Some(resolve_channel_or_toolchain_id(value)?);
                        idx += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup verify: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            verify_ncup_supply_chain_policy(&home)?;
            let config = load_ncup_config(&home)?;
            let (_, key_material, _) = effective_ncup_supply_chain_key(&home, &config)?;
            let state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let installed = state
                .get("installed_toolchains")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut verified = 0usize;
            for row in installed {
                let Some(id) = row.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                if let Some(ref requested) = explicit_toolchain {
                    if requested != id {
                        continue;
                    }
                }
                let toolchain_dir = home.join("toolchains").join(id);
                verify_toolchain_integrity_bundle(&toolchain_dir, &key_material)?;
                verified += 1;
            }
            if explicit_toolchain.is_some() && verified == 0 {
                return Err("ncup verify: requested toolchain is not installed".into());
            }
            println!(
                "OK: ncup verify home={} verified_toolchains={}",
                home.to_string_lossy(),
                verified
            );
        }
        "telemetry" => {
            let sub = args.get(1).ok_or_else(|| {
                actionable_command_error(
                    "ncup",
                    "ncup telemetry syntax: telemetry <status|enable|disable> [--home <path>] [--json]",
                )
            })?;
            let mut explicit_home: Option<String> = None;
            let mut json_output = false;
            let mut idx = 2usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--home" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup telemetry: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        idx += 2;
                    }
                    "--json" => {
                        json_output = true;
                        idx += 1;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup telemetry: unknown option '{}'", flag),
                        )
                        .into());
                    }
                }
            }

            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;

            match sub.as_str() {
                "status" => {
                    let payload = ncup_telemetry_status_payload(&home);
                    if json_output {
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                    } else {
                        println!(
                            "ncup telemetry status: enabled={} source={}",
                            payload
                                .get("enabled")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            payload
                                .get("source")
                                .and_then(|v| v.as_str())
                                .unwrap_or("config")
                        );
                        println!(
                            "events: {}",
                            payload
                                .get("events_path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-")
                        );
                        println!(
                            "policy: opt-in only, never collects source_code/source_paths/file_contents"
                        );
                    }
                }
                "enable" => {
                    set_ncup_telemetry_opt_in(&home, true)?;
                    let payload = ncup_telemetry_status_payload(&home);
                    if json_output {
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                    } else {
                        println!("OK: ncup telemetry enabled (opt-in)");
                    }
                }
                "disable" => {
                    set_ncup_telemetry_opt_in(&home, false)?;
                    let payload = ncup_telemetry_status_payload(&home);
                    if json_output {
                        println!("{}", serde_json::to_string_pretty(&payload)?);
                    } else {
                        println!("OK: ncup telemetry disabled");
                    }
                }
                other => {
                    return Err(actionable_command_error(
                        "ncup",
                        format!(
                            "ncup telemetry: unknown action '{}'; expected status|enable|disable",
                            other
                        ),
                    )
                    .into());
                }
            }
        }
        "uninstall" => {
            let mut explicit_home: Option<String> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--home" => {
                        let value = args.get(i + 1).ok_or_else(|| {
                            actionable_command_error(
                                "ncup",
                                "ncup uninstall: --home requires a path",
                            )
                        })?;
                        explicit_home = Some(value.clone());
                        i += 2;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("ncup").expect("ncup help must exist"));
                        return Ok(());
                    }
                    flag if flag.starts_with("--") => {
                        return Err(actionable_command_error(
                            "ncup",
                            format!("ncup uninstall: unknown option '{}'", flag),
                        )
                        .into());
                    }
                    value => {
                        positional.push(value.to_string());
                        i += 1;
                    }
                }
            }
            let home = resolve_ncup_home(explicit_home.as_deref());
            ensure_ncup_layout(&home)?;
            let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
            let target = if let Some(raw) = positional.first() {
                resolve_channel_or_toolchain_id(raw)?
            } else {
                state
                    .get("default_toolchain")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            if target.is_empty() {
                return Err("ncup uninstall: no default toolchain configured".into());
            }
            let mut rows = state
                .get("installed_toolchains")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let before = rows.len();
            rows.retain(|row| row.get("id").and_then(|v| v.as_str()) != Some(&target));
            if rows.len() == before {
                return Err(
                    format!("ncup uninstall: toolchain '{}' is not installed", target).into(),
                );
            }
            state["installed_toolchains"] = serde_json::Value::Array(rows);
            let toolchain_dir = home.join("toolchains").join(&target);
            if toolchain_dir.exists() {
                fs::remove_dir_all(&toolchain_dir)?;
            }
            let default_toolchain = state
                .get("default_toolchain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if default_toolchain == target {
                let next_default = state
                    .get("installed_toolchains")
                    .and_then(|v| v.as_array())
                    .and_then(|rows| rows.first())
                    .and_then(|row| row.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                state["default_toolchain"] = serde_json::Value::String(next_default.clone());
                if !next_default.is_empty() {
                    sync_default_toolchain_bin_dir(&home, &next_default)?;
                } else {
                    for bin in official_phase10_tool_bins() {
                        let dst = ncup_managed_bin_path(&home.join("bin"), bin);
                        if dst.exists() {
                            fs::remove_file(dst)?;
                        }
                    }
                    write_ncup_env_scripts(&home)?;
                }
            }
            save_ncup_state(&home, &state)?;
            println!(
                "OK: ncup uninstall {} home={}",
                target,
                home.to_string_lossy()
            );
        }
        other => {
            return Err(actionable_command_error(
                "ncup",
                format!(
                    "ncup: unknown action '{}'; expected init/home/model/install/default/toolchain/which/target/component/override/update/self/doctor/verify/telemetry/uninstall",
                    other
                ),
            )
            .into());
        }
    }

    Ok(())
}

pub(crate) fn run_ncup_direct_update_command(
    opts: NcupDirectUpdateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = opts.mode.expect("direct update mode checked by parser");
    let workspace = resolve_ncup_workspace_root(opts.workspace.as_deref())?;
    let distro = opts.distro.clone().unwrap_or_else(|| {
        if mode == NcupDirectUpdateMode::All {
            "auto-detect-user-distros".to_string()
        } else {
            mode.default_distro().to_string()
        }
    });
    let windows_apps =
        if mode.writes_windows_wrappers() || (mode == NcupDirectUpdateMode::All && cfg!(windows)) {
            Some(ncup_windows_apps_dir(
                opts.windows_apps.as_deref(),
                Some(&workspace),
            )?)
        } else {
            opts.windows_apps.as_deref().map(PathBuf::from)
        };
    let bins = ncup_direct_update_bins();
    let plan = serde_json::json!({
        "schema": "ncup.direct_update.plan.v1",
        "status": if opts.dry_run { "dry_run" } else { "ready" },
        "mode": mode.label(),
        "workspace": workspace.to_string_lossy().to_string(),
        "distro": distro.clone(),
        "bins": bins.clone(),
        "companion_artifacts": [GHOR_STAGE1_COMPANION],
        "actions": {
            "cargo_install_bins": mode.installs_wsl_bins(),
            "write_windows_cmd_wrappers": mode.writes_windows_wrappers(),
            "complete_native_suite": mode == NcupDirectUpdateMode::All,
            "refresh_all_user_wsl_distros_on_windows": mode == NcupDirectUpdateMode::All && cfg!(windows),
            "transactional_stage": mode == NcupDirectUpdateMode::All,
        },
        "windows_apps": windows_apps.as_ref().map(|p| p.to_string_lossy().to_string()),
    });

    if opts.dry_run {
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        } else {
            println!(
                "DRY-RUN: ncup update {} workspace={} distro={}",
                mode.label(),
                workspace.to_string_lossy(),
                distro
            );
        }
        return Ok(());
    }

    if mode == NcupDirectUpdateMode::All {
        let result =
            run_ncup_complete_update(&workspace, opts.distro.as_deref(), windows_apps.as_deref())?;
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!(
                "OK: ncup updated all {} binaries workspace={} host={}",
                complete_toolchain_bins().len(),
                workspace.to_string_lossy(),
                env::consts::OS
            );
        }
        return Ok(());
    }

    if mode.installs_wsl_bins() {
        run_ncup_cargo_install_bins(&workspace, mode, &distro)?;
    }
    if mode.writes_windows_wrappers() {
        let dir = windows_apps
            .as_ref()
            .expect("windows wrapper dir resolved above");
        fs::create_dir_all(dir)?;
        for bin in ncup_direct_update_bins() {
            ncup_write_windows_cmd_wrapper(&dir.join(format!("{bin}.cmd")), bin, &distro)?;
        }
    }

    let result = serde_json::json!({
        "schema": "ncup.direct_update.result.v1",
        "status": "ok",
        "mode": mode.label(),
        "workspace": workspace.to_string_lossy().to_string(),
        "distro": distro.clone(),
        "bins": ncup_direct_update_bins(),
        "companion_artifacts": [GHOR_STAGE1_COMPANION],
        "windows_apps": windows_apps.as_ref().map(|p| p.to_string_lossy().to_string()),
    });
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "OK: ncup update {} workspace={} bins={}",
            mode.label(),
            workspace.to_string_lossy(),
            ncup_direct_update_bins().len()
        );
    }
    Ok(())
}

pub(crate) fn run_ncc_update_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let opts = parse_ncc_update_options(args)?;
    if opts.help {
        println!(
            "{}",
            command_help("update").expect("update help must exist")
        );
        return Ok(());
    }
    if opts.all_paths {
        return run_ncc_update_all_paths_command(&opts);
    }

    let home = resolve_ncup_home(opts.explicit_home.as_deref());
    ensure_ncup_layout(&home)?;
    let mut state = load_ncup_state(&home, &default_phase10_toolchain_id())?;
    let cwd = if let Some(raw) = opts.explicit_cwd.as_deref() {
        PathBuf::from(raw)
    } else {
        env::current_dir()?
    };
    let (toolchains, scope, scope_detail) = resolve_ncc_update_toolchains(&state, &cwd, &opts)?;
    let size = ncc_update_estimated_size(&home, &toolchains, &opts)?;
    let mode = opts.mode.clone().unwrap_or_else(ncc_update_host_mode_label);
    let plan = ncc_update_plan_payload(
        &home,
        &cwd,
        &toolchains,
        &mode,
        &scope,
        scope_detail.as_deref(),
        &size,
        &opts,
    );

    if opts.dry_run {
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        } else {
            print_ncc_update_plan(&plan);
            println!("DRY-RUN: no se ha modificado ningun binario.");
        }
        return Ok(());
    }

    if opts.json && !opts.yes {
        return Err("ncc update: --json update execution requires --yes or --dry-run".into());
    }
    if !opts.json {
        print_ncc_update_plan(&plan);
    }
    if !opts.yes && !prompt_ncc_update_authorization(&plan)? {
        println!("ABORTADO: no se ha actualizado ningun binario.");
        return Ok(());
    }

    let downloaded_bundle_dir = if let Some(download) = opts.download.as_deref() {
        Some(ncc_update_prepare_download_bundle(&home, download)?)
    } else {
        None
    };
    let bundle_dir = downloaded_bundle_dir
        .clone()
        .or_else(|| opts.bundle.as_deref().map(PathBuf::from));
    if let Some(bundle) = bundle_dir.as_deref() {
        if !bundle.exists() || !bundle.is_dir() {
            return Err(format!(
                "ncc update: bundle '{}' must be an existing directory",
                bundle.to_string_lossy()
            )
            .into());
        }
    }

    let mut entries = Vec::new();
    for toolchain_id in &toolchains {
        let profile = ncc_update_installed_profile(&state, toolchain_id).ok_or_else(|| {
            format!(
                "ncc update: toolchain '{}' is not installed in ncup state",
                toolchain_id
            )
        })?;
        let entry = run_ncc_update_toolchain_transaction(
            &home,
            &mut state,
            toolchain_id,
            &profile,
            &scope,
            scope_detail.as_deref(),
            bundle_dir.as_deref(),
        )?;
        entries.push(entry);
    }

    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "ncc.update.result.v1",
                "status": "ok",
                "home": home.to_string_lossy().to_string(),
                "mode": mode,
                "size_bytes": size.bytes,
                "size_human": ncc_update_format_bytes(size.bytes),
                "entries": entries,
            }))?
        );
    } else {
        println!(
            "OK: ncc update home={} toolchains={} peso={}",
            home.to_string_lossy(),
            toolchains.len(),
            ncc_update_format_bytes(size.bytes)
        );
    }
    Ok(())
}
