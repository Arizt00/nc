use super::super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NclspModeFamily {
    RootBasal,
    MachineLowLevel,
    PrimitiveLowLevel,
    RustCore,
    HighLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NclspEditorialProfile {
    Novice,
    Standard,
    Expert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NclspExtensionMigrationStage {
    WarningFirst,
    ErrorByProfile,
    Strict,
}

fn nclsp_family_from_context(ctx: &Option<ncc_ast::Context>) -> NclspModeFamily {
    match ctx {
        Some(ncc_ast::Context::Root) => NclspModeFamily::RootBasal,
        Some(ncc_ast::Context::Machine) => NclspModeFamily::MachineLowLevel,
        Some(ncc_ast::Context::Primitive) => NclspModeFamily::PrimitiveLowLevel,
        Some(ncc_ast::Context::Kernel)
        | Some(ncc_ast::Context::Inspect)
        | Some(ncc_ast::Context::Compiler) => NclspModeFamily::RustCore,
        _ => NclspModeFamily::HighLevel,
    }
}

fn nclsp_family_label(family: NclspModeFamily) -> &'static str {
    match family {
        NclspModeFamily::RootBasal => "root-basal",
        NclspModeFamily::MachineLowLevel => "machine-low-level",
        NclspModeFamily::PrimitiveLowLevel => "primitive-low-level",
        NclspModeFamily::RustCore => "rust-core",
        NclspModeFamily::HighLevel => "high-level",
    }
}

fn nclsp_is_low_level_family(family: NclspModeFamily) -> bool {
    matches!(
        family,
        NclspModeFamily::RootBasal
            | NclspModeFamily::MachineLowLevel
            | NclspModeFamily::PrimitiveLowLevel
    )
}

fn nclsp_family_required_extension(family: NclspModeFamily) -> &'static str {
    match family {
        NclspModeFamily::RootBasal => ".nb",
        NclspModeFamily::MachineLowLevel => ".nm",
        NclspModeFamily::PrimitiveLowLevel => ".ncp",
        NclspModeFamily::RustCore => ".n",
        NclspModeFamily::HighLevel => ".n",
    }
}

fn nclsp_family_accepted_extensions(family: NclspModeFamily) -> &'static [&'static str] {
    match family {
        NclspModeFamily::RootBasal => &[".nb"],
        NclspModeFamily::MachineLowLevel => &[".nm"],
        NclspModeFamily::PrimitiveLowLevel => &[".ncp"],
        NclspModeFamily::RustCore | NclspModeFamily::HighLevel => &[".n", ".nc"],
    }
}

fn nclsp_family_allowed_ides(family: NclspModeFamily) -> &'static [&'static str] {
    match family {
        NclspModeFamily::RootBasal
        | NclspModeFamily::MachineLowLevel
        | NclspModeFamily::PrimitiveLowLevel => &["goland", "clion", "ghidra", "vscode"],
        NclspModeFamily::RustCore => &["rustrover", "vscode", "intellij"],
        NclspModeFamily::HighLevel => &["intellij", "vscode", "rustrover"],
    }
}

fn nclsp_family_required_lane(family: NclspModeFamily) -> &'static str {
    match family {
        NclspModeFamily::RootBasal => "sovereign-core",
        NclspModeFamily::MachineLowLevel => "machine-core",
        NclspModeFamily::PrimitiveLowLevel => "primitive-core",
        NclspModeFamily::RustCore => "rust-core",
        NclspModeFamily::HighLevel => "hl-core",
    }
}

fn nclsp_family_min_profile(family: NclspModeFamily) -> &'static str {
    match family {
        NclspModeFamily::RootBasal
        | NclspModeFamily::MachineLowLevel
        | NclspModeFamily::PrimitiveLowLevel => "expert",
        NclspModeFamily::RustCore => "standard",
        NclspModeFamily::HighLevel => "novice",
    }
}

fn nclsp_profile_label(profile: NclspEditorialProfile) -> &'static str {
    match profile {
        NclspEditorialProfile::Novice => "novice",
        NclspEditorialProfile::Standard => "standard",
        NclspEditorialProfile::Expert => "expert",
    }
}

fn nclsp_profile_from_str(raw: &str) -> Option<NclspEditorialProfile> {
    match raw.to_ascii_lowercase().as_str() {
        "novice" => Some(NclspEditorialProfile::Novice),
        "standard" => Some(NclspEditorialProfile::Standard),
        "expert" => Some(NclspEditorialProfile::Expert),
        _ => None,
    }
}

fn nclsp_migration_stage_label(stage: NclspExtensionMigrationStage) -> &'static str {
    match stage {
        NclspExtensionMigrationStage::WarningFirst => "warning-first",
        NclspExtensionMigrationStage::ErrorByProfile => "error-by-profile",
        NclspExtensionMigrationStage::Strict => "strict",
    }
}

fn nclsp_migration_stage_from_str(raw: &str) -> Option<NclspExtensionMigrationStage> {
    match raw.to_ascii_lowercase().as_str() {
        "warning-first" => Some(NclspExtensionMigrationStage::WarningFirst),
        "error-by-profile" => Some(NclspExtensionMigrationStage::ErrorByProfile),
        "strict" => Some(NclspExtensionMigrationStage::Strict),
        _ => None,
    }
}

fn nclsp_should_error_legacy_extension(
    profile: NclspEditorialProfile,
    stage: NclspExtensionMigrationStage,
) -> bool {
    match stage {
        NclspExtensionMigrationStage::WarningFirst => false,
        NclspExtensionMigrationStage::ErrorByProfile => {
            matches!(
                profile,
                NclspEditorialProfile::Standard | NclspEditorialProfile::Expert
            )
        }
        NclspExtensionMigrationStage::Strict => true,
    }
}

pub(crate) fn nclsp_validate_file(
    path: &str,
    ide: Option<&str>,
    profile: NclspEditorialProfile,
    migration_stage: NclspExtensionMigrationStage,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let src = fs::read_to_string(path)?;
    let tokens = ncc_lexer::lex(&src)?;
    let ast = ncc_parser::parse(tokens)?;

    let mut families: BTreeSet<NclspModeFamily> = BTreeSet::new();
    let mut contexts: Vec<String> = Vec::new();
    for container in &ast.containers {
        families.insert(nclsp_family_from_context(&container.context));
        contexts.push(context_label(&container.context));
    }

    let extension = PathBuf::from(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase()))
        .unwrap_or_else(|| "".to_string());

    if families.is_empty() {
        return Err("nclsp validate: source has no containers".into());
    }
    if families.len() > 1 {
        let names: Vec<&str> = families
            .iter()
            .map(|family| nclsp_family_label(*family))
            .collect();
        return Ok(serde_json::json!({
            "mode": "nclsp.validate.v1",
            "status": "error",
            "file": path,
            "contexts": contexts,
            "message": format!("mixed mode families are not allowed in one file: {:?}", names),
        }));
    }

    let family = *families.iter().next().expect("family set cannot be empty");
    let required_ext = nclsp_family_required_extension(family);
    let accepted_exts = nclsp_family_accepted_extensions(family);
    let allowed_ides = nclsp_family_allowed_ides(family);
    let required_lane = nclsp_family_required_lane(family);
    let min_profile = nclsp_family_min_profile(family);

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let extension_accepted = accepted_exts.iter().any(|ext| *ext == extension);
    if !extension_accepted {
        let is_legacy_low_level_n = nclsp_is_low_level_family(family) && extension == ".n";
        if is_legacy_low_level_n {
            let message = format!(
                "legacy extension '.n' for {} is in migration; official extension is '{}' (stage={}, profile={})",
                nclsp_family_label(family),
                required_ext,
                nclsp_migration_stage_label(migration_stage),
                nclsp_profile_label(profile)
            );
            if nclsp_should_error_legacy_extension(profile, migration_stage) {
                errors.push(message);
            } else {
                warnings.push(message);
            }
        } else {
            errors.push(format!(
                "invalid extension '{}' for mode family {}; expected '{}'",
                extension,
                nclsp_family_label(family),
                required_ext
            ));
        }
    }
    if let Some(requested_ide) = ide {
        let normalized = requested_ide.to_ascii_lowercase();
        if !allowed_ides.iter().any(|allowed| *allowed == normalized) {
            warnings.push(format!(
                "ide '{}' is not in recommended set for this mode family; recommended {:?}",
                requested_ide, allowed_ides
            ));
        }
    }

    let status = if errors.is_empty() { "ok" } else { "error" };
    Ok(serde_json::json!({
        "mode": "nclsp.validate.v1",
        "status": status,
        "file": path,
        "contexts": contexts,
        "required_extension": required_ext,
        "accepted_extensions": accepted_exts,
        "actual_extension": extension,
        "recommended_ides": allowed_ides,
        "required_lane": required_lane,
        "editorial_profile_min": min_profile,
        "requested_editorial_profile": nclsp_profile_label(profile),
        "extension_migration_stage": nclsp_migration_stage_label(migration_stage),
        "requested_ide": ide,
        "warnings": warnings,
        "errors": errors
    }))
}

pub(crate) fn run_nclsp_command(args: &[String]) -> Result<(), CliFailure> {
    run_nclsp_command_legacy(args)
        .map_err(|error| CliFailure::from_boxed_at(error, "tooling.nclsp"))
}

fn run_nclsp_command_legacy(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("nclsp").expect("nclsp help must exist"));
        return Ok(());
    }
    let action = args.first().ok_or_else(|| {
        actionable_command_error("nclsp", "nclsp syntax: nclsp <policy|validate> [args]")
    })?;

    match action.as_str() {
        "policy" => {
            let mut mode = "json";
            for arg in &args[1..] {
                match arg.as_str() {
                    "--json" => mode = "json",
                    "--list" => mode = "list",
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("nclsp").expect("nclsp help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "nclsp",
                            format!(
                                "nclsp policy: unknown option '{}'; expected --json or --list",
                                other
                            ),
                        )
                        .into());
                    }
                }
            }
            let policy = build_phase10_mode_policy_manifest();
            if mode == "list" {
                let rows = policy
                    .get("mode_rules")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                for row in rows {
                    let family = row
                        .get("mode_family")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let ext = row
                        .get("official_extension")
                        .or_else(|| row.get("required_extension"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let ides = row
                        .get("recommended_ides")
                        .or_else(|| row.get("allowed_ides"))
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    let lane = row
                        .get("required_lane")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    let profile = row
                        .get("editorial_profile_min")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    println!(
                        "{} ext={} lane={} profile={} ides={}",
                        family, ext, lane, profile, ides
                    );
                }
            } else {
                println!("{}", serde_json::to_string_pretty(&policy)?);
            }
        }
        "validate" => {
            let path = args.get(1).ok_or_else(|| {
                actionable_command_error(
                    "nclsp",
                    "nclsp validate syntax: nclsp validate <file> [--ide <name>] [--profile <novice|standard|expert>] [--migration-stage <warning-first|error-by-profile|strict>] [--json]",
                )
            })?;
            let mut ide: Option<String> = None;
            let mut json_mode = false;
            let mut requested_profile = NclspEditorialProfile::Novice;
            let mut migration_stage = NclspExtensionMigrationStage::WarningFirst;
            let mut idx = 2usize;
            while idx < args.len() {
                match args[idx].as_str() {
                    "--ide" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error(
                                "nclsp",
                                "nclsp validate: --ide requires a value",
                            )
                        })?;
                        ide = Some(value.clone());
                        idx += 2;
                    }
                    "--profile" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error(
                                "nclsp",
                                "nclsp validate: --profile requires novice|standard|expert",
                            )
                        })?;
                        requested_profile = nclsp_profile_from_str(value).ok_or_else(|| {
                            actionable_command_error(
                                "nclsp",
                                format!(
                                    "nclsp validate: invalid profile '{}'; expected novice|standard|expert",
                                    value
                                ),
                            )
                        })?;
                        idx += 2;
                    }
                    "--migration-stage" => {
                        let value = args.get(idx + 1).ok_or_else(|| {
                            actionable_command_error(
                                "nclsp",
                                "nclsp validate: --migration-stage requires warning-first|error-by-profile|strict",
                            )
                        })?;
                        migration_stage = nclsp_migration_stage_from_str(value).ok_or_else(|| {
                            actionable_command_error(
                                "nclsp",
                                format!(
                                    "nclsp validate: invalid migration stage '{}'; expected warning-first|error-by-profile|strict",
                                    value
                                ),
                            )
                        })?;
                        idx += 2;
                    }
                    "--json" => {
                        json_mode = true;
                        idx += 1;
                    }
                    "-h" | "--help" | "help" => {
                        println!("{}", command_help("nclsp").expect("nclsp help must exist"));
                        return Ok(());
                    }
                    other => {
                        return Err(actionable_command_error(
                            "nclsp",
                            format!("nclsp validate: unknown option '{}'", other),
                        )
                        .into());
                    }
                }
            }
            let report =
                nclsp_validate_file(path, ide.as_deref(), requested_profile, migration_stage)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let status = report
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("error");
                if status == "ok" {
                    println!(
                        "OK: nclsp validate file={} ext={} required={} ide={} profile={} migration-stage={}",
                        path,
                        report
                            .get("actual_extension")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-"),
                        report
                            .get("required_extension")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-"),
                        ide.as_deref().unwrap_or("-"),
                        nclsp_profile_label(requested_profile),
                        nclsp_migration_stage_label(migration_stage),
                    );
                    if let Some(warnings) = report.get("warnings").and_then(|v| v.as_array()) {
                        for warning in warnings {
                            if let Some(msg) = warning.as_str() {
                                eprintln!("warning: {}", msg);
                            }
                        }
                    }
                } else {
                    let rendered = serde_json::to_string_pretty(&report)?;
                    return Err(rendered.into());
                }
            }
        }
        other => {
            return Err(actionable_command_error(
                "nclsp",
                format!(
                    "nclsp: unknown action '{}'; expected policy/validate",
                    other
                ),
            )
            .into());
        }
    }

    Ok(())
}
