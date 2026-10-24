use super::super::*;

pub(crate) fn run_dx_profile_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("dx-profile").expect("dx-profile help must exist")
        );
        return Ok(());
    }
    if args.len() < 2 {
        return Err(actionable_command_error(
            "dx-profile",
            "dx-profile syntax: dx-profile <check|run> <file.n> [--strict-source] [--main-profile <profile>] [--out <path>]",
        )
        .into());
    }

    let action = args[0].as_str();
    if action != "check" && action != "run" {
        let suggestion = suggestion_suffix(action, &["check", "run"]);
        return Err(actionable_command_error(
            "dx-profile",
            format!(
                "dx-profile: unknown action '{}'; expected check/run{}",
                action, suggestion
            ),
        )
        .into());
    }
    let path = args[1].clone();

    let mut strict_source = false;
    let mut main_profile: Option<String> = None;
    let mut out_path: Option<String> = None;
    let mut idx = 2usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--strict-source" => {
                strict_source = true;
                idx += 1;
            }
            "--main-profile" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "dx-profile",
                        "dx-profile syntax: --main-profile requires a value",
                    )
                })?;
                main_profile = Some(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "dx-profile",
                        "dx-profile syntax: --out requires a file path",
                    )
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("dx-profile").expect("dx-profile help must exist")
                );
                return Ok(());
            }
            other => {
                return Err(actionable_command_error(
                    "dx-profile",
                    format!("dx-profile: unknown option '{}'", other),
                )
                .into())
            }
        }
    }

    let started_at = Instant::now();
    let mut stage_timings: Vec<(&'static str, u128)> = Vec::new();

    let read_started = Instant::now();
    let src = match fs::read_to_string(&path) {
        Ok(src) => {
            stage_timings.push(("read_source_ms", read_started.elapsed().as_millis()));
            src
        }
        Err(err) => {
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "error": {
                    "code": "NCC-DX-0101",
                    "stage": "read_source",
                    "message": err.to_string(),
                    "hint": "verify file path and filesystem permissions"
                },
                "durations_ms": {},
                "source_warnings_total": 0,
                "sema_warnings_total": 0,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0101] dx-profile failed at read_source".into());
        }
    };

    let lex_started = Instant::now();
    let tokens = match ncc_lexer::lex(&src) {
        Ok(tokens) => {
            stage_timings.push(("lex_ms", lex_started.elapsed().as_millis()));
            tokens
        }
        Err(err) => {
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "error": {
                    "code": "NCC-DX-0201",
                    "stage": "lex",
                    "message": err.message,
                    "line": err.line,
                    "column": err.column,
                    "hint": "review invalid symbols/tokens near reported location"
                },
                "durations_ms": {
                    "read_source_ms": stage_timings.iter().find(|(k, _)| *k == "read_source_ms").map(|(_, v)| *v).unwrap_or(0)
                },
                "source_warnings_total": 0,
                "sema_warnings_total": 0,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0201] dx-profile failed at lex".into());
        }
    };

    let parse_started = Instant::now();
    let ast = match ncc_parser::parse(tokens) {
        Ok(ast) => {
            stage_timings.push(("parse_ms", parse_started.elapsed().as_millis()));
            ast
        }
        Err(err) => {
            let mut durations = serde_json::Map::new();
            for (k, v) in &stage_timings {
                durations.insert((*k).to_string(), serde_json::json!(*v));
            }
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "error": {
                    "code": "NCC-DX-0301",
                    "stage": "parse",
                    "message": err.message,
                    "line": err.line,
                    "column": err.column,
                    "hint": "review section syntax and delimiters near reported location"
                },
                "durations_ms": serde_json::Value::Object(durations),
                "source_warnings_total": 0,
                "sema_warnings_total": 0,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0301] dx-profile failed at parse".into());
        }
    };

    let source_started = Instant::now();
    let source_warnings = match validate_source_conventions(&ast, &path, strict_source) {
        Ok(warnings) => {
            stage_timings.push((
                "source_conventions_ms",
                source_started.elapsed().as_millis(),
            ));
            warnings
        }
        Err(err) => {
            let mut durations = serde_json::Map::new();
            for (k, v) in &stage_timings {
                durations.insert((*k).to_string(), serde_json::json!(*v));
            }
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "error": {
                    "code": "NCC-DX-0151",
                    "stage": "source_conventions",
                    "message": err.to_string(),
                    "hint": "review source naming/path conventions and strict-source policy"
                },
                "durations_ms": serde_json::Value::Object(durations),
                "source_warnings_total": 0,
                "sema_warnings_total": 0,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0151] dx-profile failed at source_conventions".into());
        }
    };
    let source_warnings_total = source_warnings.len();

    let sema_started = Instant::now();
    let sema = match ncc_sema::analyze(&ast) {
        Ok(result) => {
            stage_timings.push(("sema_ms", sema_started.elapsed().as_millis()));
            result
        }
        Err(err) => {
            let mut durations = serde_json::Map::new();
            for (k, v) in &stage_timings {
                durations.insert((*k).to_string(), serde_json::json!(*v));
            }
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "error": {
                    "code": "NCC-DX-0401",
                    "stage": "sema",
                    "message": err,
                    "hint": "review type contracts, visibility and context authority"
                },
                "durations_ms": serde_json::Value::Object(durations),
                "source_warnings_total": source_warnings_total,
                "sema_warnings_total": 0,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0401] dx-profile failed at sema".into());
        }
    };
    let sema_warnings_total = sema.warnings.len();
    let sema_phase_us = sema
        .phase_timings
        .iter()
        .map(|timing| {
            (
                timing.phase.to_string(),
                serde_json::json!(timing.elapsed_us),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    let mut runtime_return: Option<String> = None;
    if action == "run" {
        let main_validate_started = Instant::now();
        if let Err(err) =
            ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
        {
            let mut durations = serde_json::Map::new();
            for (k, v) in &stage_timings {
                durations.insert((*k).to_string(), serde_json::json!(*v));
            }
            stage_timings.push((
                "main_entry_validation_ms",
                main_validate_started.elapsed().as_millis(),
            ));
            for (k, v) in &stage_timings {
                durations.insert((*k).to_string(), serde_json::json!(*v));
            }
            let report = serde_json::json!({
                "mode": "ncc.dx.profile.v1",
                "action": action,
                "file": path,
                "status": "error",
                "selected_main_profile": main_profile,
                "error": {
                    "code": "NCC-DX-0501",
                    "stage": "main_entry_validation",
                    "message": err,
                    "hint": "review main family resolution and profile selection"
                },
                "durations_ms": serde_json::Value::Object(durations),
                "source_warnings_total": source_warnings_total,
                "sema_warnings_total": sema_warnings_total,
                "elapsed_ms": started_at.elapsed().as_millis(),
            });
            let rendered = serde_json::to_string_pretty(&report)?;
            if let Some(out) = out_path {
                fs::write(&out, rendered.as_bytes())?;
                println!("OK: dx-profile {}", out);
            } else {
                println!("{}", rendered);
            }
            return Err("[NCC-DX-0501] dx-profile failed at main_entry_validation".into());
        }
        stage_timings.push((
            "main_entry_validation_ms",
            main_validate_started.elapsed().as_millis(),
        ));

        let runtime_started = Instant::now();
        match run_main_with_profile(&ast, main_profile.as_deref()) {
            Ok(ret) => {
                stage_timings.push(("runtime_ms", runtime_started.elapsed().as_millis()));
                runtime_return = Some(render_value(&ret));
            }
            Err(err) => {
                stage_timings.push(("runtime_ms", runtime_started.elapsed().as_millis()));
                let mut durations = serde_json::Map::new();
                for (k, v) in &stage_timings {
                    durations.insert((*k).to_string(), serde_json::json!(*v));
                }
                let report = serde_json::json!({
                    "mode": "ncc.dx.profile.v1",
                    "action": action,
                    "file": path,
                    "status": "error",
                    "selected_main_profile": main_profile,
                    "error": {
                        "code": "NCC-DX-0601",
                        "stage": "runtime",
                        "message": err.to_string(),
                        "hint": "review runtime preconditions for memory/network/system/security flows"
                    },
                    "durations_ms": serde_json::Value::Object(durations),
                    "source_warnings_total": source_warnings_total,
                    "sema_warnings_total": sema_warnings_total,
                    "elapsed_ms": started_at.elapsed().as_millis(),
                });
                let rendered = serde_json::to_string_pretty(&report)?;
                if let Some(out) = out_path {
                    fs::write(&out, rendered.as_bytes())?;
                    println!("OK: dx-profile {}", out);
                } else {
                    println!("{}", rendered);
                }
                return Err("[NCC-DX-0601] dx-profile failed at runtime".into());
            }
        }
    }

    let mut durations = serde_json::Map::new();
    for (k, v) in &stage_timings {
        durations.insert((*k).to_string(), serde_json::json!(*v));
    }
    let report = serde_json::json!({
        "mode": "ncc.dx.profile.v1",
        "action": action,
        "file": path,
        "status": "ok",
        "selected_main_profile": main_profile,
        "return_value": runtime_return,
        "durations_ms": serde_json::Value::Object(durations),
        "sema_phase_us": serde_json::Value::Object(sema_phase_us),
        "source_warnings_total": source_warnings_total,
        "sema_warnings_total": sema_warnings_total,
        "elapsed_ms": started_at.elapsed().as_millis(),
    });
    let rendered = serde_json::to_string_pretty(&report)?;
    if let Some(out) = out_path {
        fs::write(&out, rendered.as_bytes())?;
        println!("OK: dx-profile {}", out);
    } else {
        println!("{}", rendered);
    }
    Ok(())
}
