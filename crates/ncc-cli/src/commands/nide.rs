use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NideIdeTarget {
    Vscode,
    JetBrains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NideDiagnosticFormat {
    Report,
    Jsonl,
    Ndiag,
    Terminal,
}

#[derive(Debug)]
struct NideDiagnosticAnalysis {
    phase: String,
    warnings: Vec<String>,
    diagnostics: Vec<ncc_diagnostics::CanonicalDiagnostic>,
}

#[derive(Debug)]
struct NideIdeInitOptions {
    project_dir: PathBuf,
    ide_targets: Vec<NideIdeTarget>,
    run_profiles: Vec<NideRunProfile>,
    run_targets: Vec<NideRunTarget>,
    toolchains: Vec<String>,
    source_file: Option<String>,
    force: bool,
    dry_run: bool,
    json: bool,
}

#[derive(Debug)]
struct NideIdeInitWrite {
    kind: &'static str,
    path: PathBuf,
    status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NideRunProfile {
    Auto,
    Interpreted,
    NativeFast,
    NativeFfi,
    Debug,
    Release,
    ReleaseFast,
    ReleaseSmall,
    UnsafeFast,
    Instrumented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NideRunTarget {
    id: String,
}

pub(crate) fn build_nide_outline_report(path: &str, ast: &ncc_ast::Program) -> serde_json::Value {
    let containers = ast
        .containers
        .iter()
        .map(|container| {
            serde_json::json!({
                "name": container.name,
                "context": context_label(&container.context),
                "visibility": format!("{:?}", container.visibility),
                "sections": container.sections.iter().map(section_kind_label).collect::<Vec<_>>(),
                "section_count": container.sections.len(),
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "mode": "nide.outline.v1",
        "file": path,
        "imports": ast.uses.iter().map(|u| if u.wildcard {
            format!("{}.*", u.path.join("."))
        } else {
            u.path.join(".")
        }).collect::<Vec<_>>(),
        "containers": containers,
    })
}

pub(crate) fn build_nide_symbols_report(path: &str, ast: &ncc_ast::Program) -> serde_json::Value {
    let containers = ast
        .containers
        .iter()
        .map(|container| {
            let mut symbols = Vec::new();
            for section in &container.sections {
                collect_section_symbols(section, &mut symbols);
            }
            serde_json::json!({
                "name": container.name,
                "context": context_label(&container.context),
                "symbols": symbols,
                "symbol_count": symbols.len(),
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "mode": "nide.symbols.v1",
        "file": path,
        "containers": containers,
    })
}

fn nide_error_entry(diagnostic: &ncc_diagnostics::CanonicalDiagnostic) -> serde_json::Value {
    let mut entry = crate::canonical_diagnostic_to_json(diagnostic);
    entry["hint"] = serde_json::Value::String(diagnostic.help.clone());
    entry
}

fn analyze_nide_diagnostics(path: &str, src: &str) -> NideDiagnosticAnalysis {
    let tokens = match ncc_lexer::lex(src) {
        Ok(tokens) => tokens,
        Err(err) => {
            return NideDiagnosticAnalysis {
                phase: "lexer".to_string(),
                warnings: Vec::new(),
                diagnostics: vec![err.canonical("lexer").with_file(path)],
            };
        }
    };

    let ast = match ncc_parser::parse(tokens) {
        Ok(ast) => ast,
        Err(err) => {
            return NideDiagnosticAnalysis {
                phase: "parser".to_string(),
                warnings: Vec::new(),
                diagnostics: vec![err.canonical("parser").with_file(path)],
            };
        }
    };

    match ncc_sema::analyze_canonical(&ast) {
        Ok(result) => {
            let diagnostics = result
                .warnings
                .iter()
                .map(|warning| {
                    ncc_diagnostics::CanonicalDiagnostic::from_legacy(warning, "sema", 0, 0)
                        .with_file(path)
                        .with_severity(ncc_diagnostics::DiagnosticSeverity::Warning)
                })
                .collect::<Vec<_>>();
            NideDiagnosticAnalysis {
                phase: "sema".to_string(),
                warnings: result.warnings,
                diagnostics,
            }
        }
        Err(err) => NideDiagnosticAnalysis {
            phase: "sema".to_string(),
            warnings: Vec::new(),
            diagnostics: vec![err.with_file(path)],
        },
    }
}

fn nide_diagnostics_report_from_analysis(
    path: &str,
    src: &str,
    analysis: &NideDiagnosticAnalysis,
) -> serde_json::Value {
    let is_error = |diagnostic: &&ncc_diagnostics::CanonicalDiagnostic| {
        matches!(
            diagnostic.severity,
            ncc_diagnostics::DiagnosticSeverity::Error | ncc_diagnostics::DiagnosticSeverity::Fatal
        )
    };
    let error_entries = analysis
        .diagnostics
        .iter()
        .filter(is_error)
        .map(nide_error_entry)
        .collect::<Vec<_>>();
    let warning_entries = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| !is_error(diagnostic))
        .map(nide_error_entry)
        .collect::<Vec<_>>();
    let errors = analysis
        .diagnostics
        .iter()
        .filter(is_error)
        .map(|diagnostic| diagnostic.render(src))
        .collect::<Vec<_>>();
    serde_json::json!({
        "mode": "nide.diagnostics.v1",
        "diagnostic_abi": "nc.diagnostic.machine.v2",
        "diagnostic_registry": ncc_diagnostics::DIAGNOSTIC_REGISTRY_SCHEMA,
        "diagnostic_registry_version": ncc_diagnostics::DIAGNOSTIC_REGISTRY_VERSION,
        "diagnostic_registry_entries": ncc_diagnostics::DIAGNOSTIC_REGISTRY.len(),
        "diagnostic_transport": "nc.diagnostic.jsonl.v1|nc.diagnostic.ndiag.v1",
        "file": path,
        "status": if errors.is_empty() { "ok" } else { "error" },
        "phase": analysis.phase,
        "warnings": analysis.warnings,
        "errors": errors,
        "warning_entries": warning_entries,
        "error_entries": error_entries,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_nide_diagnostics_report(path: &str, src: &str) -> serde_json::Value {
    let analysis = analyze_nide_diagnostics(path, src);
    nide_diagnostics_report_from_analysis(path, src, &analysis)
}

fn nide_init_help_text() -> &'static str {
    "Usage: nide init [--ide <vscode|jetbrains|all>] [--project <dir>] [--file <file.n|file.nc|file.nb|file.nbb|file.ncp>] [--profile <auto|interpreted|native-fast|release-fast,...>] [--target <host|triple,...>] [--toolchain <auto|ncc|stable|toolchain-id|path,...>] [--force] [--dry-run] [--json]\n\
     Generate IDE Run/Check/Format entries that delegate to ncc with optional toolchain profiles and target-specific Run configs.\n\
     Aliases: ncc ide init, nide init clion, nide init vscode\n\
     Examples:\n\
       nide init\n\
       nide init --ide vscode\n\
       nide init --ide jetbrains --file examples/helloWorld.n\n\
       ncc ide init --profile native-fast --target x86_64-unknown-linux-gnu --toolchain stable\n\
       ncc ide init --project . --force"
}

fn parse_nide_ide_targets(raw: &str) -> Result<Vec<NideIdeTarget>, Box<dyn std::error::Error>> {
    let mut targets = Vec::new();
    for item in raw.split(',') {
        let item = item.trim().to_ascii_lowercase();
        match item.as_str() {
            "" => {}
            "all" => {
                targets.clear();
                targets.push(NideIdeTarget::Vscode);
                targets.push(NideIdeTarget::JetBrains);
                return Ok(targets);
            }
            "vscode" | "vs-code" | "code" => {
                if !targets.contains(&NideIdeTarget::Vscode) {
                    targets.push(NideIdeTarget::Vscode);
                }
            }
            "jetbrains" | "idea" | "intellij" | "clion" | "rustrover" | "goland" => {
                if !targets.contains(&NideIdeTarget::JetBrains) {
                    targets.push(NideIdeTarget::JetBrains);
                }
            }
            other => {
                return Err(actionable_command_error(
                    "nide",
                    format!(
                        "nide init: unknown IDE target '{}'; expected vscode, jetbrains or all",
                        other
                    ),
                )
                .into());
            }
        }
    }
    if targets.is_empty() {
        return Err(actionable_command_error(
            "nide",
            "nide init: empty --ide target; expected vscode, jetbrains or all",
        )
        .into());
    }
    Ok(targets)
}

impl NideRunProfile {
    fn parse(raw: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" => Err(actionable_command_error(
                "nide",
                "nide init: empty --profile item; expected auto, interpreted, native-fast, debug, release, release-fast, release-small, unsafe-fast, instrumented or native-ffi",
            )
            .into()),
            "auto" | "default" | "host" => Ok(Self::Auto),
            "interpreted" | "runtime" => Ok(Self::Interpreted),
            "native" | "native-fast" | "real" => Ok(Self::NativeFast),
            "native-ffi" => Ok(Self::NativeFfi),
            "debug" => Ok(Self::Debug),
            "release" => Ok(Self::Release),
            "release-fast" | "releasefast" => Ok(Self::ReleaseFast),
            "release-small" | "releasesmall" => Ok(Self::ReleaseSmall),
            "unsafe-fast" | "unsafefast" => Ok(Self::UnsafeFast),
            "instrumented" => Ok(Self::Instrumented),
            other => Err(actionable_command_error(
                "nide",
                format!(
                    "nide init: unknown --profile '{}'; expected auto, interpreted, native-fast, debug, release, release-fast, release-small, unsafe-fast, instrumented or native-ffi",
                    other
                ),
            )
            .into()),
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Interpreted => "interpreted",
            Self::NativeFast => "native-fast",
            Self::NativeFfi => "native-ffi",
            Self::Debug => "debug",
            Self::Release => "release",
            Self::ReleaseFast => "release-fast",
            Self::ReleaseSmall => "release-small",
            Self::UnsafeFast => "unsafe-fast",
            Self::Instrumented => "instrumented",
        }
    }
}

impl NideRunTarget {
    fn parse(raw: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let normalized = raw.trim().to_ascii_lowercase();
        let id = match normalized.as_str() {
            "" => {
                return Err(actionable_command_error(
                    "nide",
                    "nide init: empty --target item; expected host or a supported target triple",
                )
                .into())
            }
            "host" | "auto" | "default" => "host",
            "linux-x64" | "linux-x86_64" | "x86_64-linux" | "x86_64-unknown-linux-gnu" => {
                "x86_64-unknown-linux-gnu"
            }
            "windows-x64" | "windows-x86_64" | "win64" | "x86_64-pc-windows-msvc" => {
                "x86_64-pc-windows-msvc"
            }
            "windows-gnu-x64" | "windows-x86_64-gnu" | "x86_64-pc-windows-gnu" => {
                "x86_64-pc-windows-gnu"
            }
            "macos-x64" | "macos-x86_64" | "darwin-x64" | "x86_64-apple-darwin" => {
                "x86_64-apple-darwin"
            }
            "macos-arm64" | "macos-aarch64" | "darwin-arm64" | "aarch64-apple-darwin"
            | "arm64-apple-darwin" => "aarch64-apple-darwin",
            "linux-arm64" | "linux-aarch64" | "arm64-linux" | "aarch64-unknown-linux-gnu"
            | "arm64-unknown-linux-gnu" => "aarch64-unknown-linux-gnu",
            "wasm32-wasi" | "wasi32" | "nc.portable" | "portable" => "wasm32-wasi",
            "x86_64-unknown-sovereign-nc" | "nc.sovereign" | "sovereign" => {
                "x86_64-unknown-sovereign-nc"
            }
            other => {
                return Err(actionable_command_error(
                    "nide",
                    format!(
                        "nide init: unknown --target '{}'; expected host, x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, x86_64-apple-darwin, aarch64-apple-darwin, aarch64-unknown-linux-gnu, wasm32-wasi or x86_64-unknown-sovereign-nc",
                        other
                    ),
                )
                .into())
            }
        };
        Ok(Self { id: id.to_string() })
    }

    fn id(&self) -> &str {
        &self.id
    }
}

fn parse_nide_run_profiles(raw: &str) -> Result<Vec<NideRunProfile>, Box<dyn std::error::Error>> {
    let mut profiles = Vec::new();
    for item in raw.split(',') {
        let profile = NideRunProfile::parse(item)?;
        if !profiles.contains(&profile) {
            profiles.push(profile);
        }
    }
    if profiles.is_empty() {
        return Err(actionable_command_error(
            "nide",
            "nide init: --profile requires at least one profile",
        )
        .into());
    }
    Ok(profiles)
}

fn parse_nide_run_targets(raw: &str) -> Result<Vec<NideRunTarget>, Box<dyn std::error::Error>> {
    let mut targets = Vec::new();
    for item in raw.split(',') {
        let target = NideRunTarget::parse(item)?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    if targets.is_empty() {
        return Err(actionable_command_error(
            "nide",
            "nide init: --target requires at least one target",
        )
        .into());
    }
    Ok(targets)
}

fn parse_nide_toolchains(raw: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut toolchains = Vec::new();
    for item in raw.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return Err(actionable_command_error(
                "nide",
                "nide init: empty --toolchain item; expected auto, ncc, stable, beta, nightly, a toolchain-id or a path",
            )
            .into());
        }
        if !toolchains.iter().any(|known| known == item) {
            toolchains.push(item.to_string());
        }
    }
    if toolchains.is_empty() {
        return Err(actionable_command_error(
            "nide",
            "nide init: --toolchain requires at least one item",
        )
        .into());
    }
    Ok(toolchains)
}

fn is_nide_source_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".n")
        || lower.ends_with(".nc")
        || lower.ends_with(".nb")
        || lower.ends_with(".nbb")
        || lower.ends_with(".ncp")
}

fn parse_nide_ide_init_options(
    args: &[String],
) -> Result<NideIdeInitOptions, Box<dyn std::error::Error>> {
    let mut targets = vec![NideIdeTarget::Vscode, NideIdeTarget::JetBrains];
    let mut run_profiles = vec![NideRunProfile::Auto];
    let mut run_targets = vec![NideRunTarget::parse("host")?];
    let mut toolchains = vec!["auto".to_string()];
    let mut project_dir = PathBuf::from(".");
    let mut source_file: Option<String> = None;
    let mut force = false;
    let mut dry_run = false;
    let mut json = false;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" | "help" => {
                println!("{}", nide_init_help_text());
                return Err("__NIDE_INIT_HELP_PRINTED__".into());
            }
            "--ide" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --ide requires a value")
                })?;
                targets = parse_nide_ide_targets(value)?;
                idx += 2;
            }
            "--profile" | "--run-profile" | "--run-profiles" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --profile requires a value")
                })?;
                run_profiles = parse_nide_run_profiles(value)?;
                idx += 2;
            }
            "--target" | "--run-target" | "--run-targets" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --target requires a value")
                })?;
                run_targets = parse_nide_run_targets(value)?;
                idx += 2;
            }
            "--toolchain" | "--toolchains" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --toolchain requires a value")
                })?;
                toolchains = parse_nide_toolchains(value)?;
                idx += 2;
            }
            "--project" | "--cwd" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --project requires a directory")
                })?;
                project_dir = PathBuf::from(value);
                idx += 2;
            }
            "--file" | "--main" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide init: --file requires a source path")
                })?;
                source_file = Some(value.clone());
                idx += 2;
            }
            "--force" => {
                force = true;
                idx += 1;
            }
            "--dry-run" | "--check" => {
                dry_run = true;
                idx += 1;
            }
            "--json" => {
                json = true;
                idx += 1;
            }
            "all" | "vscode" | "vs-code" | "code" | "jetbrains" | "idea" | "intellij" | "clion"
            | "rustrover" | "goland" => {
                targets = parse_nide_ide_targets(&args[idx])?;
                idx += 1;
            }
            other if is_nide_source_path(other) => {
                if source_file.is_some() {
                    return Err(actionable_command_error(
                        "nide",
                        "nide init: source file was provided more than once",
                    )
                    .into());
                }
                source_file = Some(other.to_string());
                idx += 1;
            }
            other => {
                return Err(actionable_command_error(
                    "nide",
                    format!("nide init: unknown option '{}'", other),
                )
                .into());
            }
        }
    }

    Ok(NideIdeInitOptions {
        project_dir,
        ide_targets: targets,
        run_profiles,
        run_targets,
        toolchains,
        source_file,
        force,
        dry_run,
        json,
    })
}

fn normalize_project_dir(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    if !absolute.is_dir() {
        return Err(actionable_command_error(
            "nide",
            format!(
                "nide init: project directory '{}' does not exist",
                absolute.display()
            ),
        )
        .into());
    }
    Ok(absolute)
}

fn slash_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn configured_vscode_file_arg(source_file: &str) -> String {
    let normalized = slash_path(source_file);
    if Path::new(source_file).is_absolute() {
        normalized
    } else {
        format!(
            "${{workspaceFolder}}/{}",
            normalized.trim_start_matches("./")
        )
    }
}

fn configured_jetbrains_file_arg(source_file: &str) -> String {
    let normalized = slash_path(source_file);
    if Path::new(source_file).is_absolute() {
        normalized
    } else {
        format!("$PROJECT_DIR$/{}", normalized.trim_start_matches("./"))
    }
}

fn nide_host_runtime() -> JetBrainsScriptRuntime {
    JetBrainsScriptRuntime::current_host()
}

fn nide_runtime_for_project(project_dir: &Path) -> JetBrainsScriptRuntime {
    if cfg!(windows) {
        return JetBrainsScriptRuntime::Cmd;
    }
    let path = project_dir.to_string_lossy().replace('\\', "/");
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("/mnt/")
        || lower.starts_with("//wsl.localhost/")
        || lower.starts_with("//wsl$/")
        || lower.as_bytes().get(1) == Some(&b':')
    {
        JetBrainsScriptRuntime::Cmd
    } else {
        nide_host_runtime()
    }
}

fn nide_profile_ids(opts: &NideIdeInitOptions) -> Vec<String> {
    opts.run_profiles
        .iter()
        .map(|profile| profile.id().to_string())
        .collect()
}

fn nide_target_ids(opts: &NideIdeInitOptions) -> Vec<String> {
    opts.run_targets
        .iter()
        .map(|target| target.id().to_string())
        .collect()
}

fn nide_toolchain_ids(opts: &NideIdeInitOptions) -> Vec<String> {
    opts.toolchains.clone()
}

fn nide_has_explicit_run_matrix(opts: &NideIdeInitOptions) -> bool {
    nide_profile_ids(opts) != ["auto"]
        || nide_target_ids(opts) != ["host"]
        || nide_toolchain_ids(opts) != ["auto"]
}

fn nide_matrix_slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn nide_vscode_script_command(
    runtime: JetBrainsScriptRuntime,
    action: JetBrainsScriptAction,
) -> String {
    let script = match action {
        JetBrainsScriptAction::Run => "nc-run",
        JetBrainsScriptAction::Check => "nc-check",
        JetBrainsScriptAction::Format => "nc-format",
        JetBrainsScriptAction::DebugAdapter => "nc-debug-adapter",
    };
    format!(
        "${{workspaceFolder}}/.ncc/ide/{script}.{}",
        runtime.extension()
    )
}

fn nide_vscode_task(
    label: &str,
    action: JetBrainsScriptAction,
    file_arg: &str,
    runtime: JetBrainsScriptRuntime,
    profile: &str,
    target: &str,
    toolchain: &str,
) -> serde_json::Value {
    let group = if matches!(action, JetBrainsScriptAction::Run) {
        serde_json::json!({"kind": "test", "isDefault": true})
    } else {
        serde_json::json!({"kind": "build", "isDefault": false})
    };
    serde_json::json!({
        "label": label,
        "type": "shell",
        "command": nide_vscode_script_command(runtime, action),
        "args": [file_arg, profile, target, toolchain],
        "options": {
            "cwd": "${workspaceFolder}"
        },
        "group": group,
        "problemMatcher": [],
        "presentation": {
            "reveal": "always",
            "panel": "dedicated",
            "clear": true
        }
    })
}

fn nide_vscode_input(id: &str, description: &str, values: Vec<String>) -> serde_json::Value {
    if values.len() <= 1 {
        serde_json::json!({
            "id": id,
            "type": "promptString",
            "description": description,
            "default": values.first().cloned().unwrap_or_default(),
        })
    } else {
        serde_json::json!({
            "id": id,
            "type": "pickString",
            "description": description,
            "options": values,
        })
    }
}

fn render_vscode_tasks(
    opts: &NideIdeInitOptions,
    runtime: JetBrainsScriptRuntime,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut tasks = vec![
        nide_vscode_task(
            "nC: Run Active File",
            JetBrainsScriptAction::Run,
            "${file}",
            runtime,
            "auto",
            "host",
            "auto",
        ),
        nide_vscode_task(
            "nC: Check Active File",
            JetBrainsScriptAction::Check,
            "${file}",
            runtime,
            "auto",
            "host",
            "auto",
        ),
        nide_vscode_task(
            "nC: Format Active File",
            JetBrainsScriptAction::Format,
            "${file}",
            runtime,
            "auto",
            "host",
            "auto",
        ),
    ];
    if nide_has_explicit_run_matrix(opts) {
        tasks.push(nide_vscode_task(
            "nC: Run Active File (Profile/Target)",
            JetBrainsScriptAction::Run,
            "${file}",
            runtime,
            "${input:ncProfile}",
            "${input:ncTarget}",
            "${input:ncToolchain}",
        ));
    }
    if let Some(source_file) = opts.source_file.as_deref() {
        let configured = configured_vscode_file_arg(source_file);
        tasks.push(nide_vscode_task(
            "nC: Run Configured File",
            JetBrainsScriptAction::Run,
            &configured,
            runtime,
            "auto",
            "host",
            "auto",
        ));
        if nide_has_explicit_run_matrix(opts) {
            tasks.push(nide_vscode_task(
                "nC: Run Configured File (Profile/Target)",
                JetBrainsScriptAction::Run,
                &configured,
                runtime,
                "${input:ncProfile}",
                "${input:ncTarget}",
                "${input:ncToolchain}",
            ));
        }
    }
    let mut payload = serde_json::json!({
        "version": "2.0.0",
        "tasks": tasks,
    });
    if nide_has_explicit_run_matrix(opts) {
        payload["inputs"] = serde_json::Value::Array(vec![
            nide_vscode_input("ncProfile", "nC execution profile", nide_profile_ids(opts)),
            nide_vscode_input("ncTarget", "nC backend target", nide_target_ids(opts)),
            nide_vscode_input(
                "ncToolchain",
                "nC toolchain or binary path",
                nide_toolchain_ids(opts),
            ),
        ]);
    }
    Ok(format!("{}\n", serde_json::to_string_pretty(&payload)?))
}

fn xml_escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[derive(Debug, Clone, Copy)]
enum JetBrainsScriptRuntime {
    Cmd,
    Sh,
}

#[derive(Debug, Clone, Copy)]
enum JetBrainsScriptAction {
    Run,
    Check,
    Format,
    DebugAdapter,
}

impl JetBrainsScriptAction {
    fn id(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Check => "check",
            Self::Format => "format",
            Self::DebugAdapter => "debug-adapter",
        }
    }
}

impl JetBrainsScriptRuntime {
    fn current_host() -> Self {
        if cfg!(windows) {
            Self::Cmd
        } else {
            Self::Sh
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Cmd => "cmd",
            Self::Sh => "sh",
        }
    }

    fn interpreter(self) -> String {
        match self {
            Self::Cmd => env::var("COMSPEC")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    env::var("SystemRoot")
                        .map(|root| format!(r"{root}\System32\cmd.exe"))
                        .unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".to_string())
                }),
            Self::Sh => env::var("SHELL")
                .ok()
                .filter(|value| value.ends_with("sh") && Path::new(value).is_file())
                .unwrap_or_else(|| {
                    if Path::new("/bin/sh").is_file() {
                        "/bin/sh".to_string()
                    } else {
                        "sh".to_string()
                    }
                }),
        }
    }

    fn interpreter_options(self) -> &'static str {
        match self {
            Self::Cmd => "/C",
            Self::Sh => "",
        }
    }
}

fn render_jetbrains_script(
    runtime: JetBrainsScriptRuntime,
    action: JetBrainsScriptAction,
) -> String {
    match runtime {
        JetBrainsScriptRuntime::Cmd => render_jetbrains_cmd_script(action),
        JetBrainsScriptRuntime::Sh => render_jetbrains_sh_script(action),
    }
}

fn render_jetbrains_cmd_script(action: JetBrainsScriptAction) -> String {
    let script = r#"@echo off
setlocal EnableExtensions
set "NCC_IDE_ACTION=__ACTION__"
set "TARGET=%~1"
if "%TARGET%"=="" set "TARGET=main.n"
set "PROFILE=%~2"
if "%PROFILE%"=="" set "PROFILE=%NCC_IDE_PROFILE%"
if "%PROFILE%"=="" set "PROFILE=auto"
set "RUN_TARGET=%~3"
if "%RUN_TARGET%"=="" set "RUN_TARGET=%NCC_IDE_TARGET%"
if "%RUN_TARGET%"=="" set "RUN_TARGET=host"
set "TOOLCHAIN=%~4"
if "%TOOLCHAIN%"=="" set "TOOLCHAIN=%NCC_IDE_TOOLCHAIN%"
if "%TOOLCHAIN%"=="" set "TOOLCHAIN=auto"

call :is_nc_source "%TARGET%"
if errorlevel 1 set "TARGET=main.n"
for %%F in ("%TARGET%") do set "TARGET_EXT=%%~xF"
if /I "%TARGET_EXT%"==".nm" if /I not "%NCC_IDE_ACTION%"=="check" (
    echo .nm Machine sources start only from a terminal. Use ncc machine ^<command^> ^<file.nm^>. 1>&2
    exit /b 2
)
call :resolve_ncc "%TOOLCHAIN%"

if /I "%NCC_IDE_ACTION%"=="format" (
    "%NCC_BIN%" fmt "%TARGET%"
    exit /b %ERRORLEVEL%
)

if /I "%NCC_IDE_ACTION%"=="debug-adapter" (
    if /I "%~5"=="stdio" (
        "%NCC_BIN%" debug-adapter
    ) else (
        "%NCC_BIN%" debug-adapter --source-map "%TARGET%" --json
    )
    exit /b %ERRORLEVEL%
)

if /I "%NCC_IDE_ACTION%"=="check" (
    if exist bridge.json (
        if /I "%RUN_TARGET%"=="host" (
            "%NCC_BIN%" project check "."
        ) else (
            "%NCC_BIN%" project check "." --target "%RUN_TARGET%"
        )
    ) else (
        call :cc_for_target "%RUN_TARGET%"
        if defined NCC_CC (
            "%NCC_BIN%" emit-backend "%TARGET%" --cc "%NCC_CC%"
        ) else (
            "%NCC_BIN%" check "%TARGET%"
        )
    )
    exit /b %ERRORLEVEL%
)

if exist bridge.json (
    if /I "%RUN_TARGET%"=="host" (
        "%NCC_BIN%" project run "." --show-return
    ) else (
        "%NCC_BIN%" project run "." --target "%RUN_TARGET%" --show-return
    )
    exit /b %ERRORLEVEL%
)

call :cc_for_target "%RUN_TARGET%"
call :run_file "%TARGET%" "%PROFILE%"
exit /b %ERRORLEVEL%

:resolve_ncc
set "NCC_BIN=ncc"
set "NCC_RESOLVED="
if "%~1"=="" exit /b 0
if /I "%~1"=="auto" exit /b 0
if /I "%~1"=="ncc" exit /b 0
if exist "%~1" (
    set "NCC_BIN=%~1"
    exit /b 0
)
for /f "usebackq delims=" %%P in (`ncup which ncc --toolchain "%~1" 2^>nul`) do (
    if not defined NCC_RESOLVED set "NCC_RESOLVED=%%P"
)
if defined NCC_RESOLVED set "NCC_BIN=%NCC_RESOLVED%"
exit /b 0

:is_nc_source
set "EXT=%~x1"
if /I "%EXT%"==".n" exit /b 0
if /I "%EXT%"==".nb" exit /b 0
if /I "%EXT%"==".nbb" exit /b 0
if /I "%EXT%"==".nm" exit /b 0
if /I "%EXT%"==".nc" exit /b 0
if /I "%EXT%"==".ncp" exit /b 0
exit /b 1

:cc_for_target
set "NCC_CC="
if "%~1"=="" exit /b 0
if /I "%~1"=="host" exit /b 0
if /I "%~1"=="auto" exit /b 0
if /I "%~1"=="x86_64-unknown-linux-gnu" set "NCC_CC=nC.sysv64"
if /I "%~1"=="linux-x64" set "NCC_CC=nC.sysv64"
if /I "%~1"=="linux-x86_64" set "NCC_CC=nC.sysv64"
if /I "%~1"=="x86_64-pc-windows-msvc" set "NCC_CC=nC.win64"
if /I "%~1"=="x86_64-pc-windows-gnu" set "NCC_CC=nC.win64"
if /I "%~1"=="windows-x64" set "NCC_CC=nC.win64"
if /I "%~1"=="windows-x86_64" set "NCC_CC=nC.win64"
if /I "%~1"=="x86_64-apple-darwin" set "NCC_CC=nC.sysv64"
if /I "%~1"=="macos-x64" set "NCC_CC=nC.sysv64"
if /I "%~1"=="macos-x86_64" set "NCC_CC=nC.sysv64"
if /I "%~1"=="aarch64-apple-darwin" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="arm64-apple-darwin" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="macos-arm64" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="aarch64-unknown-linux-gnu" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="arm64-unknown-linux-gnu" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="linux-arm64" set "NCC_CC=nC.aapcs64"
if /I "%~1"=="wasm32-wasi" set "NCC_CC=nC.portable"
if /I "%~1"=="x86_64-unknown-sovereign-nc" set "NCC_CC=nC.sovereign"
exit /b 0

:run_file
if /I "%~2"=="interpreted" (
    if defined NCC_CC (
        "%NCC_BIN%" run "%~1" --interpreted --show-return --cc "%NCC_CC%"
    ) else (
        "%NCC_BIN%" run "%~1" --interpreted --show-return
    )
    exit /b %ERRORLEVEL%
)
if /I "%~2"=="native-fast" (
    if defined NCC_CC (
        "%NCC_BIN%" run "%~1" --native-fast --show-return --cc "%NCC_CC%"
    ) else (
        "%NCC_BIN%" run "%~1" --native-fast --show-return
    )
    exit /b %ERRORLEVEL%
)
if /I "%~2"=="native" (
    if defined NCC_CC (
        "%NCC_BIN%" run "%~1" --native-fast --show-return --cc "%NCC_CC%"
    ) else (
        "%NCC_BIN%" run "%~1" --native-fast --show-return
    )
    exit /b %ERRORLEVEL%
)
if /I "%~2"=="native-ffi" (
    if defined NCC_CC (
        "%NCC_BIN%" run "%~1" --native-ffi --show-return --cc "%NCC_CC%"
    ) else (
        "%NCC_BIN%" run "%~1" --native-ffi --show-return
    )
    exit /b %ERRORLEVEL%
)
if /I "%~2"=="debug" goto run_with_profile
if /I "%~2"=="release" goto run_with_profile
if /I "%~2"=="release-fast" goto run_with_profile
if /I "%~2"=="release-small" goto run_with_profile
if /I "%~2"=="unsafe-fast" goto run_with_profile
if /I "%~2"=="instrumented" goto run_with_profile
if defined NCC_CC (
    "%NCC_BIN%" run "%~1" --show-return --cc "%NCC_CC%"
) else (
    "%NCC_BIN%" run "%~1" --show-return
)
exit /b %ERRORLEVEL%

:run_with_profile
if defined NCC_CC (
    "%NCC_BIN%" run "%~1" --profile "%~2" --show-return --cc "%NCC_CC%"
) else (
    "%NCC_BIN%" run "%~1" --profile "%~2" --show-return
)
exit /b %ERRORLEVEL%
"#;
    script
        .replace("__ACTION__", action.id())
        .replace('\n', "\r\n")
}

fn render_jetbrains_sh_script(action: JetBrainsScriptAction) -> String {
    let script = r#"#!/usr/bin/env sh
set -eu
action="__ACTION__"
target="${1:-main.n}"
profile="${2:-${NCC_IDE_PROFILE:-auto}}"
run_target="${3:-${NCC_IDE_TARGET:-host}}"
toolchain="${4:-${NCC_IDE_TOOLCHAIN:-auto}}"

target_ext="$(printf '%s' "$target" | tr '[:upper:]' '[:lower:]')"
case "$target_ext" in
  *.n|*.nb|*.nbb|*.nm|*.nc|*.ncp) ;;
  *) target="main.n" ;;
esac
case "$target_ext" in
  *.nm)
    if [ "$action" != "check" ]; then
      printf '%s\n' '.nm Machine sources start only from a terminal. Use ncc machine <command> <file.nm>.' >&2
      exit 2
    fi
    ;;
esac

resolve_ncc() {
  candidate="${1:-auto}"
  if [ "$candidate" = "auto" ] || [ "$candidate" = "ncc" ] || [ -z "$candidate" ]; then
    printf '%s\n' "ncc"
    return 0
  fi
  if [ -f "$candidate" ] || [ -x "$candidate" ]; then
    printf '%s\n' "$candidate"
    return 0
  fi
  resolved="$(ncup which ncc --toolchain "$candidate" 2>/dev/null || true)"
  if [ -n "$resolved" ]; then
    printf '%s\n' "$resolved"
  else
    printf '%s\n' "ncc"
  fi
}

cc_for_target() {
  case "${1:-host}" in
    host|auto) printf '%s\n' "" ;;
    x86_64-unknown-linux-gnu|linux-x64|linux-x86_64) printf '%s\n' "nC.sysv64" ;;
    x86_64-pc-windows-msvc|x86_64-pc-windows-gnu|windows-x64|windows-x86_64) printf '%s\n' "nC.win64" ;;
    x86_64-apple-darwin|macos-x64|macos-x86_64) printf '%s\n' "nC.sysv64" ;;
    aarch64-apple-darwin|arm64-apple-darwin|macos-arm64|aarch64-unknown-linux-gnu|arm64-unknown-linux-gnu|linux-arm64) printf '%s\n' "nC.aapcs64" ;;
    wasm32-wasi) printf '%s\n' "nC.portable" ;;
    x86_64-unknown-sovereign-nc) printf '%s\n' "nC.sovereign" ;;
    *) printf '%s\n' "" ;;
  esac
}

run_file() {
  file="$1"
  selected_profile="$2"
  selected_cc="$(cc_for_target "$run_target")"
  cc_args=""
  if [ -n "$selected_cc" ]; then
    cc_args="--cc $selected_cc"
  fi
  case "$selected_profile" in
    interpreted)
      # shellcheck disable=SC2086
      "$ncc_bin" run "$file" --interpreted --show-return $cc_args
      ;;
    native|native-fast)
      # shellcheck disable=SC2086
      "$ncc_bin" run "$file" --native-fast --show-return $cc_args
      ;;
    native-ffi)
      # shellcheck disable=SC2086
      "$ncc_bin" run "$file" --native-ffi --show-return $cc_args
      ;;
    debug|release|release-fast|release-small|unsafe-fast|instrumented)
      # shellcheck disable=SC2086
      "$ncc_bin" run "$file" --profile "$selected_profile" --show-return $cc_args
      ;;
    *)
      # shellcheck disable=SC2086
      "$ncc_bin" run "$file" --show-return $cc_args
      ;;
  esac
}

ncc_bin="$(resolve_ncc "$toolchain")"

if [ "$action" = "format" ]; then
  "$ncc_bin" fmt "$target"
  exit $?
fi

if [ "$action" = "debug-adapter" ]; then
  if [ "${5:-}" = "stdio" ]; then
    "$ncc_bin" debug-adapter
  else
    "$ncc_bin" debug-adapter --source-map "$target" --json
  fi
  exit $?
fi

if [ "$action" = "check" ]; then
  if [ -f bridge.json ]; then
    if [ "$run_target" = "host" ] || [ "$run_target" = "auto" ]; then
      "$ncc_bin" project check .
    else
      "$ncc_bin" project check . --target "$run_target"
    fi
  else
    selected_cc="$(cc_for_target "$run_target")"
    if [ -n "$selected_cc" ]; then
      "$ncc_bin" emit-backend "$target" --cc "$selected_cc"
    else
      "$ncc_bin" check "$target"
    fi
  fi
  exit $?
fi

if [ -f bridge.json ]; then
  if [ "$run_target" = "host" ] || [ "$run_target" = "auto" ]; then
    "$ncc_bin" project run . --show-return
  else
    "$ncc_bin" project run . --target "$run_target" --show-return
  fi
  exit $?
fi

run_file "$target" "$profile"
"#;
    script.replace("__ACTION__", action.id())
}

fn render_jetbrains_shell_config(
    name: &str,
    script_path: &str,
    script_options: &str,
    runtime: JetBrainsScriptRuntime,
) -> String {
    let name = xml_escape_attr(name);
    let script_path = xml_escape_attr(script_path);
    let script_options = xml_escape_attr(script_options);
    let interpreter = xml_escape_attr(&runtime.interpreter());
    let interpreter_options = xml_escape_attr(runtime.interpreter_options());
    format!(
        "<component name=\"ProjectRunConfigurationManager\">\n\
  <configuration default=\"false\" name=\"{name}\" type=\"ShConfigurationType\" singleton=\"true\">\n\
    <option name=\"EXECUTE_IN_TERMINAL\" value=\"true\" />\n\
    <option name=\"INDEPENDENT_SCRIPT_PATH\" value=\"true\" />\n\
    <option name=\"SCRIPT_PATH\" value=\"{script_path}\" />\n\
    <option name=\"SCRIPT_OPTIONS\" value=\"{script_options}\" />\n\
    <option name=\"INDEPENDENT_SCRIPT_WORKING_DIRECTORY\" value=\"true\" />\n\
    <option name=\"SCRIPT_WORKING_DIRECTORY\" value=\"$PROJECT_DIR$\" />\n\
    <option name=\"INDEPENDENT_INTERPRETER_PATH\" value=\"true\" />\n\
    <option name=\"INTERPRETER_PATH\" value=\"{interpreter}\" />\n\
    <option name=\"INTERPRETER_OPTIONS\" value=\"{interpreter_options}\" />\n\
    <method v=\"2\" />\n\
  </configuration>\n\
</component>\n"
    )
}

fn nide_jetbrains_script_options(
    file_arg: &str,
    profile: &str,
    target: &str,
    toolchain: &str,
) -> String {
    format!("\"{file_arg}\" \"{profile}\" \"{target}\" \"{toolchain}\"")
}

fn nide_run_matrix_rows(opts: &NideIdeInitOptions) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    for profile in nide_profile_ids(opts) {
        for target in nide_target_ids(opts) {
            for toolchain in nide_toolchain_ids(opts) {
                rows.push((profile.clone(), target.clone(), toolchain));
            }
        }
    }
    rows
}

fn nide_jetbrains_matrix_file_name(
    prefix: &str,
    profile: &str,
    target: &str,
    toolchain: &str,
) -> String {
    format!(
        "{prefix}_{}_{}_{}.xml",
        nide_matrix_slug(profile),
        nide_matrix_slug(target),
        nide_matrix_slug(toolchain)
    )
}

fn nide_jetbrains_matrix_label(profile: &str, target: &str, toolchain: &str) -> String {
    format!("nC Run Active File [{profile} @ {target} / {toolchain}]")
}

fn nide_ide_artifacts(
    project_dir: &Path,
    opts: &NideIdeInitOptions,
) -> Result<Vec<(&'static str, PathBuf, String)>, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();
    let runtime = nide_runtime_for_project(project_dir);
    let script_dir = project_dir.join(".ncc").join("ide");
    if opts.ide_targets.contains(&NideIdeTarget::Vscode)
        || opts.ide_targets.contains(&NideIdeTarget::JetBrains)
    {
        for script_runtime in [JetBrainsScriptRuntime::Cmd, JetBrainsScriptRuntime::Sh] {
            artifacts.push((
                "ide.script.run",
                script_dir.join(format!("nc-run.{}", script_runtime.extension())),
                render_jetbrains_script(script_runtime, JetBrainsScriptAction::Run),
            ));
            artifacts.push((
                "ide.script.check",
                script_dir.join(format!("nc-check.{}", script_runtime.extension())),
                render_jetbrains_script(script_runtime, JetBrainsScriptAction::Check),
            ));
            artifacts.push((
                "ide.script.format",
                script_dir.join(format!("nc-format.{}", script_runtime.extension())),
                render_jetbrains_script(script_runtime, JetBrainsScriptAction::Format),
            ));
            artifacts.push((
                "ide.script.debug_adapter",
                script_dir.join(format!("nc-debug-adapter.{}", script_runtime.extension())),
                render_jetbrains_script(script_runtime, JetBrainsScriptAction::DebugAdapter),
            ));
        }
    }
    if opts.ide_targets.contains(&NideIdeTarget::Vscode) {
        artifacts.push((
            "vscode.tasks",
            project_dir.join(".vscode").join("tasks.json"),
            render_vscode_tasks(opts, runtime)?,
        ));
    }
    if opts.ide_targets.contains(&NideIdeTarget::JetBrains) {
        let run_dir = project_dir.join(".idea").join("runConfigurations");
        let run_script = format!("$PROJECT_DIR$/.ncc/ide/nc-run.{}", runtime.extension());
        let check_script = format!("$PROJECT_DIR$/.ncc/ide/nc-check.{}", runtime.extension());
        let format_script = format!("$PROJECT_DIR$/.ncc/ide/nc-format.{}", runtime.extension());
        let debug_adapter_script = format!(
            "$PROJECT_DIR$/.ncc/ide/nc-debug-adapter.{}",
            runtime.extension()
        );
        artifacts.push((
            "jetbrains.run",
            run_dir.join("nC_Run_Active_File.xml"),
            render_jetbrains_shell_config(
                "nC Run Active File",
                &run_script,
                &nide_jetbrains_script_options("$FilePath$", "auto", "host", "auto"),
                runtime,
            ),
        ));
        artifacts.push((
            "jetbrains.check",
            run_dir.join("nC_Check_Active_File.xml"),
            render_jetbrains_shell_config(
                "nC Check Active File",
                &check_script,
                &nide_jetbrains_script_options("$FilePath$", "auto", "host", "auto"),
                runtime,
            ),
        ));
        artifacts.push((
            "jetbrains.format",
            run_dir.join("nC_Format_Active_File.xml"),
            render_jetbrains_shell_config(
                "nC Format Active File",
                &format_script,
                &nide_jetbrains_script_options("$FilePath$", "auto", "host", "auto"),
                runtime,
            ),
        ));
        artifacts.push((
            "jetbrains.debug_adapter",
            run_dir.join("nC_Debug_Adapter_Source_Map.xml"),
            render_jetbrains_shell_config(
                "nC Debug Adapter Source Map",
                &debug_adapter_script,
                &nide_jetbrains_script_options("$FilePath$", "debug", "host", "auto"),
                runtime,
            ),
        ));
        if nide_has_explicit_run_matrix(opts) {
            for (profile, target, toolchain) in nide_run_matrix_rows(opts) {
                if profile == "auto" && target == "host" && toolchain == "auto" {
                    continue;
                }
                artifacts.push((
                    "jetbrains.run.target",
                    run_dir.join(nide_jetbrains_matrix_file_name(
                        "nC_Run_Active_File",
                        &profile,
                        &target,
                        &toolchain,
                    )),
                    render_jetbrains_shell_config(
                        &nide_jetbrains_matrix_label(&profile, &target, &toolchain),
                        &run_script,
                        &nide_jetbrains_script_options("$FilePath$", &profile, &target, &toolchain),
                        runtime,
                    ),
                ));
            }
        }
        if let Some(source_file) = &opts.source_file {
            let configured = configured_jetbrains_file_arg(source_file);
            artifacts.push((
                "jetbrains.run.configured",
                run_dir.join("nC_Run_Configured_File.xml"),
                render_jetbrains_shell_config(
                    "nC Run Configured File",
                    &run_script,
                    &nide_jetbrains_script_options(&configured, "auto", "host", "auto"),
                    runtime,
                ),
            ));
            if nide_has_explicit_run_matrix(opts) {
                for (profile, target, toolchain) in nide_run_matrix_rows(opts) {
                    if profile == "auto" && target == "host" && toolchain == "auto" {
                        continue;
                    }
                    artifacts.push((
                        "jetbrains.run.configured.target",
                        run_dir.join(nide_jetbrains_matrix_file_name(
                            "nC_Run_Configured_File",
                            &profile,
                            &target,
                            &toolchain,
                        )),
                        render_jetbrains_shell_config(
                            &format!("nC Run Configured File [{profile} @ {target} / {toolchain}]"),
                            &run_script,
                            &nide_jetbrains_script_options(
                                &configured,
                                &profile,
                                &target,
                                &toolchain,
                            ),
                            runtime,
                        ),
                    ));
                }
            }
        }
    }
    Ok(artifacts)
}

fn nide_ide_stale_artifacts(project_dir: &Path, opts: &NideIdeInitOptions) -> Vec<PathBuf> {
    if !opts.ide_targets.contains(&NideIdeTarget::JetBrains)
        && !opts.ide_targets.contains(&NideIdeTarget::Vscode)
    {
        return Vec::new();
    }
    let script_dir = project_dir.join(".ncc").join("ide");
    let mut stale = Vec::new();
    for stem in ["nc-run", "nc-check", "nc-format", "nc-debug-adapter"] {
        stale.push(script_dir.join(format!("{stem}.ps1")));
    }
    stale
}

fn write_nide_ide_artifact(
    kind: &'static str,
    path: PathBuf,
    content: &str,
    force: bool,
    dry_run: bool,
) -> Result<NideIdeInitWrite, Box<dyn std::error::Error>> {
    let exists = path.exists();
    if exists && !force {
        return Ok(NideIdeInitWrite {
            kind,
            path,
            status: "skipped_exists",
        });
    }
    if dry_run {
        return Ok(NideIdeInitWrite {
            kind,
            path,
            status: if exists {
                "would_overwrite"
            } else {
                "would_write"
            },
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content.as_bytes())?;
    Ok(NideIdeInitWrite {
        kind,
        path,
        status: if force { "written" } else { "created" },
    })
}

fn nide_ide_target_labels(targets: &[NideIdeTarget]) -> Vec<&'static str> {
    targets
        .iter()
        .map(|target| match target {
            NideIdeTarget::Vscode => "vscode",
            NideIdeTarget::JetBrains => "jetbrains",
        })
        .collect()
}

fn run_nide_ide_init(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", nide_init_help_text());
        return Ok(());
    }
    let opts = match parse_nide_ide_init_options(args) {
        Ok(opts) => opts,
        Err(err) if err.to_string() == "__NIDE_INIT_HELP_PRINTED__" => return Ok(()),
        Err(err) => return Err(err),
    };
    let project_dir = normalize_project_dir(&opts.project_dir)?;
    let artifacts = nide_ide_artifacts(&project_dir, &opts)?;
    let mut writes = Vec::new();
    for (kind, path, content) in artifacts {
        writes.push(write_nide_ide_artifact(
            kind,
            path,
            &content,
            opts.force,
            opts.dry_run,
        )?);
    }
    if opts.force {
        for path in nide_ide_stale_artifacts(&project_dir, &opts) {
            if opts.dry_run {
                if path.exists() {
                    writes.push(NideIdeInitWrite {
                        kind: "jetbrains.script.stale",
                        path,
                        status: "would_remove",
                    });
                }
            } else if path.exists() {
                fs::remove_file(&path)?;
                writes.push(NideIdeInitWrite {
                    kind: "jetbrains.script.stale",
                    path,
                    status: "removed",
                });
            }
        }
    }

    let report = serde_json::json!({
        "schema": "nide.ide.init.v1",
        "status": "ok",
        "project": project_dir.to_string_lossy(),
        "ides": nide_ide_target_labels(&opts.ide_targets),
        "profiles": nide_profile_ids(&opts),
        "targets": nide_target_ids(&opts),
        "toolchains": nide_toolchain_ids(&opts),
        "source_file": opts.source_file.clone(),
        "force": opts.force,
        "dry_run": opts.dry_run,
        "artifacts": writes.iter().map(|write| {
            serde_json::json!({
                "kind": write.kind,
                "path": write.path.to_string_lossy(),
                "status": write.status,
            })
        }).collect::<Vec<_>>(),
        "commands": [
            "nide init",
            "nide init --ide vscode",
            "nide init --ide jetbrains --file examples/helloWorld.n",
            "nide init --profile native-fast --target x86_64-unknown-linux-gnu --toolchain stable",
            "ncc ide init --force"
        ]
    });

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for write in &writes {
            println!(
                "OK: nide init {} {} ({})",
                write.status,
                write.path.display(),
                write.kind
            );
        }
        println!("Hint: use --force to regenerate existing IDE files.");
    }
    Ok(())
}

pub(crate) fn run_nide_command(args: &[String]) -> Result<(), CliFailure> {
    run_nide_command_legacy(args).map_err(|error| CliFailure::from_boxed_at(error, "tooling.nide"))
}

fn run_nide_command_legacy(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("nide").expect("nide help must exist"));
        return Ok(());
    }
    if args.len() == 2 && args[0] == "help" && args[1] == "init" {
        println!("{}", nide_init_help_text());
        return Ok(());
    }
    if args.is_empty() {
        return Err(actionable_command_error(
            "nide",
            "nide syntax: nide <outline|symbols|diagnostics> <file.n> [--format <report|jsonl|ndiag|terminal>] [--out <path>] or nide init [--ide <vscode|jetbrains|all>]",
        )
        .into());
    }

    if args[0] == "init" {
        return run_nide_ide_init(&args[1..]);
    }

    let (action, path_idx) = if is_high_level_source_path(&args[0]) {
        ("symbols", 0usize)
    } else {
        (args[0].as_str(), 1usize)
    };

    let path = args
        .get(path_idx)
        .ok_or_else(|| actionable_command_error("nide", "nide syntax: missing <file.n> argument"))?
        .clone();

    let mut out_path: Option<String> = None;
    let mut diagnostic_format = NideDiagnosticFormat::Report;
    let mut idx = path_idx + 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" | "help" => {
                println!("{}", command_help("nide").expect("nide help must exist"));
                return Ok(());
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nide", "nide syntax: --out requires a file path")
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            "--format" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "nide",
                        "nide diagnostics: --format requires report|jsonl|ndiag|terminal",
                    )
                })?;
                diagnostic_format = match value.as_str() {
                    "report" | "json" => NideDiagnosticFormat::Report,
                    "jsonl" => NideDiagnosticFormat::Jsonl,
                    "ndiag" | "binary" => NideDiagnosticFormat::Ndiag,
                    "terminal" | "human" => NideDiagnosticFormat::Terminal,
                    other => {
                        return Err(actionable_command_error(
                            "nide",
                            format!(
                                "nide diagnostics: unknown format '{}'; expected report|jsonl|ndiag|terminal",
                                other
                            ),
                        )
                        .into())
                    }
                };
                idx += 2;
            }
            "--jsonl" => {
                diagnostic_format = NideDiagnosticFormat::Jsonl;
                idx += 1;
            }
            "--ndiag" => {
                diagnostic_format = NideDiagnosticFormat::Ndiag;
                idx += 1;
            }
            other => {
                return Err(actionable_command_error(
                    "nide",
                    format!("nide: unknown option '{}'", other),
                )
                .into())
            }
        }
    }

    if action != "diagnostics" && diagnostic_format != NideDiagnosticFormat::Report {
        return Err(actionable_command_error(
            "nide",
            "nide: --format/--jsonl/--ndiag are only valid for diagnostics",
        )
        .into());
    }

    if action == "diagnostics" {
        if diagnostic_format == NideDiagnosticFormat::Report
            && out_path.as_deref().is_some_and(|out| {
                Path::new(out)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("ndiag"))
            })
        {
            diagnostic_format = NideDiagnosticFormat::Ndiag;
        }
        let src = fs::read_to_string(&path)?;
        let analysis = analyze_nide_diagnostics(&path, &src);
        let bytes = match diagnostic_format {
            NideDiagnosticFormat::Report => serde_json::to_vec_pretty(
                &nide_diagnostics_report_from_analysis(&path, &src, &analysis),
            )?,
            NideDiagnosticFormat::Jsonl => {
                crate::canonical_diagnostics_to_jsonl(&analysis.diagnostics)?.into_bytes()
            }
            NideDiagnosticFormat::Ndiag => {
                crate::canonical_diagnostics_to_ndiag(&analysis.diagnostics)?
            }
            NideDiagnosticFormat::Terminal => analysis
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    crate::render_canonical_diagnostic_terminal(
                        diagnostic,
                        &src,
                        crate::stderr_diagnostic_color_enabled(),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes(),
        };
        if let Some(out) = out_path {
            fs::write(&out, &bytes)?;
            println!("OK: nide diagnostics {}", out);
        } else {
            io::stdout().write_all(&bytes)?;
            if diagnostic_format != NideDiagnosticFormat::Ndiag
                && !bytes.is_empty()
                && bytes.last() != Some(&b'\n')
            {
                println!();
            }
        }
        return Ok(());
    }

    let report = match action {
        "outline" => {
            let ast = load_ast_from_path(&path)?;
            build_nide_outline_report(&path, &ast)
        }
        "symbols" => {
            let ast = load_ast_from_path(&path)?;
            build_nide_symbols_report(&path, &ast)
        }
        "diagnostics" => unreachable!("diagnostics returns through its typed transport path"),
        other => {
            let suggestion =
                suggestion_suffix(other, &["outline", "symbols", "diagnostics", "init"]);
            return Err(actionable_command_error(
                "nide",
                format!(
                    "nide: unknown action '{}'; expected outline/symbols/diagnostics/init{}",
                    other, suggestion
                ),
            )
            .into());
        }
    };

    let rendered = serde_json::to_string_pretty(&report)?;
    if let Some(out) = out_path {
        fs::write(&out, rendered.as_bytes())?;
        println!("OK: nide {} {}", action, out);
    } else {
        println!("{}", rendered);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic_transport_temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir().join(format!("nide-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn diagnostics_keep_real_code_and_machine_packet() {
        let source = "sample {\n    type:\n        Item {\n\n            42;\n        }\n}\n";
        let report = build_nide_diagnostics_report("sample.n", source);
        let entry = &report["error_entries"][0];
        assert_eq!(entry["code"].as_str(), Some("E_TYPE_CONTRACT_SECTION_001"));
        assert_eq!(entry["phase"].as_str(), Some("parser.type"));
        assert_eq!(entry["line"].as_u64(), Some(5));
        assert_eq!(entry["column"].as_u64(), Some(13));
        assert_eq!(entry["abi"].as_str(), Some("nc.diagnostic.machine.v2"));
        assert_eq!(
            entry["machine_record_hex"].as_str().map(str::len),
            Some(ncc_diagnostics::DIAGNOSTIC_RECORD_BYTES * 2)
        );
        assert!(entry["machine_arena_hex"]
            .as_str()
            .is_some_and(|hex| !hex.is_empty()));
    }

    #[test]
    fn diagnostics_write_common_jsonl_and_ndiag_transports() {
        let dir = diagnostic_transport_temp_dir("canonical-transport");
        fs::create_dir_all(&dir).expect("create transport temp dir");
        let source_path = dir.join("sample.n");
        let jsonl_path = dir.join("sample.jsonl");
        let ndiag_path = dir.join("sample.ndiag");
        fs::write(
            &source_path,
            "sample {\n    type:\n        Item {\n\n            42;\n        }\n}\n",
        )
        .expect("write invalid fixture");

        run_nide_command(&[
            "diagnostics".to_string(),
            source_path.to_string_lossy().to_string(),
            "--format".to_string(),
            "jsonl".to_string(),
            "--out".to_string(),
            jsonl_path.to_string_lossy().to_string(),
        ])
        .expect("write JSONL diagnostics");
        let jsonl = fs::read_to_string(&jsonl_path).expect("read JSONL diagnostics");
        let lines = jsonl.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let entry: serde_json::Value = serde_json::from_str(lines[0]).expect("parse JSONL entry");
        assert_eq!(entry["transport"], "nc.diagnostic.jsonl.v1");
        assert_eq!(entry["code"], "E_TYPE_CONTRACT_SECTION_001");

        run_nide_command(&[
            "diagnostics".to_string(),
            source_path.to_string_lossy().to_string(),
            "--format".to_string(),
            "ndiag".to_string(),
            "--out".to_string(),
            ndiag_path.to_string_lossy().to_string(),
        ])
        .expect("write .ndiag diagnostics");
        let encoded = fs::read(&ndiag_path).expect("read .ndiag diagnostics");
        let decoded = ncc_diagnostics::decode_ndiag(&encoded).expect("decode .ndiag diagnostics");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].code, "E_TYPE_CONTRACT_SECTION_001");
        assert_eq!(decoded[0].phase, "parser.type");
        assert_eq!(decoded[0].file, source_path.to_string_lossy());

        fs::remove_dir_all(dir).expect("remove transport temp dir");
    }

    fn temp_project(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("nide-ide-init-{label}-{}-{stamp}", process::id()));
        fs::create_dir_all(&dir).expect("temp project dir");
        dir
    }

    #[test]
    fn nide_init_generates_vscode_tasks_and_jetbrains_run_configs() {
        let project = temp_project("all");
        let args = vec![
            "--project".to_string(),
            project.to_string_lossy().to_string(),
            "--file".to_string(),
            "examples/helloWorld.n".to_string(),
            "--json".to_string(),
        ];

        run_nide_ide_init(&args).expect("nide init should generate IDE artifacts");

        let tasks_path = project.join(".vscode").join("tasks.json");
        let tasks_raw = fs::read_to_string(&tasks_path).expect("tasks.json");
        let tasks: serde_json::Value =
            serde_json::from_str(&tasks_raw).expect("tasks.json must be valid JSON");
        let task_labels = tasks["tasks"]
            .as_array()
            .expect("tasks array")
            .iter()
            .filter_map(|task| task["label"].as_str())
            .collect::<Vec<_>>();
        assert!(task_labels.contains(&"nC: Run Active File"));
        assert!(task_labels.contains(&"nC: Check Active File"));
        assert!(task_labels.contains(&"nC: Format Active File"));
        assert!(task_labels.contains(&"nC: Run Configured File"));
        assert!(
            tasks_raw.contains("${workspaceFolder}/examples/helloWorld.n"),
            "configured VS Code task should be workspace-relative"
        );

        let run_config = fs::read_to_string(
            project
                .join(".idea")
                .join("runConfigurations")
                .join("nC_Run_Active_File.xml"),
        )
        .expect("JetBrains run config");
        assert!(run_config.contains("$PROJECT_DIR$/.ncc/ide/nc-run."));
        assert!(run_config.contains("&quot;$FilePath$&quot;"));
        assert!(
            run_config.contains("cmd.exe")
                || run_config.contains("/bin/sh")
                || run_config.contains("/bin/bash")
                || run_config.contains("/bin/zsh")
                || run_config.contains("value=\"sh\""),
            "JetBrains config should choose a host interpreter"
        );

        let script_ext = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(
            project
                .join(".ncc")
                .join("ide")
                .join("nc-run.cmd")
                .is_file(),
            "nide init should always generate Windows cmd scripts"
        );
        assert!(
            project.join(".ncc").join("ide").join("nc-run.sh").is_file(),
            "nide init should always generate Unix sh scripts"
        );
        let run_script = fs::read_to_string(
            project
                .join(".ncc")
                .join("ide")
                .join(format!("nc-run.{script_ext}")),
        )
        .expect("JetBrains run script");
        assert!(run_script.contains("run \"%~1\"") || run_script.contains("run \"$file\""));
        assert!(
            run_script.contains("resolve_ncc") || run_script.contains(":resolve_ncc"),
            "JetBrains run script should resolve ncc from auto/toolchain/path"
        );
        for ext in [".n", ".nb", ".nbb"] {
            assert!(
                run_script.contains(ext),
                "JetBrains run script should recognize {ext} sources"
            );
        }
        assert!(
            run_script.contains(".nm Machine sources start only from a terminal"),
            "JetBrains scripts must reject .nm Run/Debug and explain the terminal route"
        );
        assert!(
            run_script.contains("bridge.json"),
            "JetBrains run script should prefer project main when bridge.json exists"
        );

        let check_script = fs::read_to_string(
            project
                .join(".ncc")
                .join("ide")
                .join(format!("nc-check.{script_ext}")),
        )
        .expect("JetBrains check script");
        assert!(
            check_script.contains("project check"),
            "JetBrains check script should validate multi-file projects through bridge.json"
        );

        let configured_run = fs::read_to_string(
            project
                .join(".idea")
                .join("runConfigurations")
                .join("nC_Run_Configured_File.xml"),
        )
        .expect("JetBrains configured run config");
        assert!(configured_run.contains("$PROJECT_DIR$/examples/helloWorld.n"));

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn nide_runtime_for_wsl_mounted_windows_project_uses_cmd() {
        let runtime = nide_runtime_for_project(Path::new("/mnt/c/Users/example/project"));
        assert!(
            matches!(runtime, JetBrainsScriptRuntime::Cmd),
            "WSL-mounted Windows projects should generate JetBrains configs that use cmd.exe"
        );
    }

    #[test]
    fn nide_init_generates_profile_target_toolchain_run_matrix() {
        let project = temp_project("matrix");
        let args = vec![
            "--project".to_string(),
            project.to_string_lossy().to_string(),
            "--ide".to_string(),
            "all".to_string(),
            "--profile".to_string(),
            "interpreted,native-fast".to_string(),
            "--target".to_string(),
            "host,linux-x64,windows-x64".to_string(),
            "--toolchain".to_string(),
            "auto,stable".to_string(),
            "--json".to_string(),
        ];

        run_nide_ide_init(&args).expect("nide init should generate profile/target matrix");

        let run_dir = project.join(".idea").join("runConfigurations");
        assert!(
            run_dir
                .join("nC_Run_Active_File_native_fast_x86_64_unknown_linux_gnu_stable.xml")
                .is_file(),
            "JetBrains should get a stable/linux/native-fast run config"
        );
        assert!(
            run_dir
                .join("nC_Run_Active_File_interpreted_x86_64_pc_windows_msvc_auto.xml")
                .is_file(),
            "JetBrains should get a Windows target run config"
        );

        let linux_config = fs::read_to_string(
            run_dir.join("nC_Run_Active_File_native_fast_x86_64_unknown_linux_gnu_stable.xml"),
        )
        .expect("matrix run config");
        assert!(linux_config.contains("native-fast"));
        assert!(linux_config.contains("x86_64-unknown-linux-gnu"));
        assert!(linux_config.contains("stable"));

        let tasks_raw =
            fs::read_to_string(project.join(".vscode").join("tasks.json")).expect("VS Code tasks");
        let tasks: serde_json::Value =
            serde_json::from_str(&tasks_raw).expect("tasks.json must be valid JSON");
        let inputs = tasks["inputs"].as_array().expect("matrix inputs");
        assert!(
            inputs
                .iter()
                .any(|input| input["id"].as_str() == Some("ncProfile")),
            "VS Code tasks should expose profile picker"
        );
        assert!(tasks_raw.contains("native-fast"));
        assert!(tasks_raw.contains("x86_64-unknown-linux-gnu"));
        assert!(tasks_raw.contains("stable"));

        let script_ext = if cfg!(windows) { "cmd" } else { "sh" };
        let run_script = fs::read_to_string(
            project
                .join(".ncc")
                .join("ide")
                .join(format!("nc-run.{script_ext}")),
        )
        .expect("run script");
        assert!(run_script.contains("ncup which ncc --toolchain"));
        assert!(run_script.contains("project run"));
        assert!(run_script.contains("--target"));
        assert!(run_script.contains("nC.sysv64"));
        assert!(run_script.contains("nC.win64"));

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn nide_init_does_not_overwrite_vscode_tasks_without_force() {
        let project = temp_project("no-overwrite");
        let vscode_dir = project.join(".vscode");
        fs::create_dir_all(&vscode_dir).expect("vscode dir");
        let tasks_path = vscode_dir.join("tasks.json");
        fs::write(&tasks_path, b"{\"custom\":true}\n").expect("seed tasks");

        let args = vec![
            "--project".to_string(),
            project.to_string_lossy().to_string(),
            "--ide".to_string(),
            "vscode".to_string(),
        ];

        run_nide_ide_init(&args).expect("nide init should skip existing VS Code tasks");

        let tasks_raw = fs::read_to_string(&tasks_path).expect("tasks.json");
        assert_eq!(tasks_raw, "{\"custom\":true}\n");

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn nide_init_force_removes_stale_jetbrains_scripts() {
        let project = temp_project("stale-scripts");
        let ide_dir = project.join(".ncc").join("ide");
        fs::create_dir_all(&ide_dir).expect("ide dir");
        let stale_script = ide_dir.join("nc-run.ps1");
        fs::write(&stale_script, b"stale").expect("seed stale script");

        let args = vec![
            "--project".to_string(),
            project.to_string_lossy().to_string(),
            "--ide".to_string(),
            "jetbrains".to_string(),
            "--force".to_string(),
        ];

        run_nide_ide_init(&args).expect("nide init should remove stale scripts with force");

        assert!(
            !stale_script.exists(),
            "force regeneration should remove stale JetBrains runner scripts"
        );

        let _ = fs::remove_dir_all(project);
    }
}
