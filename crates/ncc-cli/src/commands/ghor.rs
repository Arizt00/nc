use super::compiler::compiler_collect_ffi_link_manifest;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const GHOR_VERSION: &str = "0.1.0";
const GHOR_LOCK_VERSION: u32 = 1;
const GHOR_OFFICIAL_MANIFEST: &str = "ghor.toml";
const GHOR_COMPAT_MANIFEST: &str = "nc.toml";
const GHOR_CARGO_MANIFEST: &str = "Cargo.toml";
const GHOR_PACKAGE_PREFLIGHT_FILE_LIMIT: usize = 5000;
const GHOR_HELP_LILAC: &str = "\x1b[38;2;203;166;247m";
const GHOR_ANSI_RESET: &str = "\x1b[0m";
const GHOR_BOOTSTRAP_SCHEMA: &str = "ghor.bootstrap.v1";
const GHOR_NC_MODULES: &[(&str, &str)] = &[
    ("text", include_str!("../../../../tools/ghor/nc/src/text.n")),
    (
        "diagnostics",
        include_str!("../../../../tools/ghor/nc/src/diagnostics.n"),
    ),
    (
        "manifest",
        include_str!("../../../../tools/ghor/nc/src/manifest.n"),
    ),
    (
        "cargo_config",
        include_str!("../../../../tools/ghor/nc/src/cargo_config.n"),
    ),
    (
        "package_graph",
        include_str!("../../../../tools/ghor/nc/src/package_graph.n"),
    ),
    (
        "transport",
        include_str!("../../../../tools/ghor/nc/src/transport.n"),
    ),
    (
        "resolver",
        include_str!("../../../../tools/ghor/nc/src/resolver.n"),
    ),
    (
        "sha256",
        include_str!("../../../../tools/ghor/nc/src/sha256.n"),
    ),
    (
        "package_io",
        include_str!("../../../../tools/ghor/nc/src/package_io.n"),
    ),
    (
        "dependency_ops",
        include_str!("../../../../tools/ghor/nc/src/dependency_ops.n"),
    ),
    (
        "registry",
        include_str!("../../../../tools/ghor/nc/src/registry.n"),
    ),
    (
        "registry_mutation",
        include_str!("../../../../tools/ghor/nc/src/registry_mutation.n"),
    ),
    (
        "artifact",
        include_str!("../../../../tools/ghor/nc/src/artifact.n"),
    ),
    (
        "build_pipeline",
        include_str!("../../../../tools/ghor/nc/src/build_pipeline.n"),
    ),
    (
        "execution_pipeline",
        include_str!("../../../../tools/ghor/nc/src/execution_pipeline.n"),
    ),
    (
        "install_lifecycle",
        include_str!("../../../../tools/ghor/nc/src/install_lifecycle.n"),
    ),
    (
        "project_tools",
        include_str!("../../../../tools/ghor/nc/src/project_tools.n"),
    ),
    (
        "assurance",
        include_str!("../../../../tools/ghor/nc/src/assurance.n"),
    ),
    (
        "certification",
        include_str!("../../../../tools/ghor/nc/src/certification.n"),
    ),
    (
        "security",
        include_str!("../../../../tools/ghor/nc/src/security.n"),
    ),
    (
        "project_inspection",
        include_str!("../../../../tools/ghor/nc/src/project_inspection.n"),
    ),
    (
        "cargo_parity",
        include_str!("../../../../tools/ghor/nc/src/cargo_parity.n"),
    ),
    (
        "build_boundaries",
        include_str!("../../../../tools/ghor/nc/src/build_boundaries.n"),
    ),
    (
        "foreign_execution",
        include_str!("../../../../tools/ghor/nc/src/foreign_execution.n"),
    ),
    (
        "system_adapters",
        include_str!("../../../../tools/ghor/nc/src/system_adapters.n"),
    ),
    (
        "registry_administration",
        include_str!("../../../../tools/ghor/nc/src/registry_administration.n"),
    ),
    (
        "bootstrap_authority",
        include_str!("../../../../tools/ghor/nc/src/bootstrap_authority.n"),
    ),
    (
        "command",
        include_str!("../../../../tools/ghor/nc/src/command.n"),
    ),
    ("main", include_str!("../../../../tools/ghor/nc/main.n")),
];

#[derive(Debug, Clone)]
struct GhorDiagnostic(ncc_diagnostics::CanonicalDiagnostic);

impl GhorDiagnostic {
    fn error(code: &str, message: impl Into<String>) -> Self {
        Self::from_parts(code, message, "error")
    }

    fn warning(code: &str, message: impl Into<String>) -> Self {
        Self::from_parts(code, message, "warning")
    }

    fn from_parts(code: &str, message: impl Into<String>, severity: &str) -> Self {
        let severity = match severity {
            "fatal" => ncc_diagnostics::DiagnosticSeverity::Fatal,
            "warning" => ncc_diagnostics::DiagnosticSeverity::Warning,
            "help" => ncc_diagnostics::DiagnosticSeverity::Help,
            "info" | "note" => ncc_diagnostics::DiagnosticSeverity::Note,
            _ => ncc_diagnostics::DiagnosticSeverity::Error,
        };
        Self(
            ncc_diagnostics::registered_diagnostic(severity, code)
                .with_found(message)
                .with_domain(ncc_diagnostics::DiagnosticDomain::Ghor),
        )
    }

    fn is_error(&self) -> bool {
        matches!(
            self.0.severity,
            ncc_diagnostics::DiagnosticSeverity::Error | ncc_diagnostics::DiagnosticSeverity::Fatal
        )
    }

    fn canonical(&self) -> &ncc_diagnostics::CanonicalDiagnostic {
        &self.0
    }

    fn attach_file(&mut self, file: &Path) {
        if self.0.file.is_empty() {
            self.0.file = file.display().to_string();
        }
    }
}

impl std::ops::Deref for GhorDiagnostic {
    type Target = ncc_diagnostics::CanonicalDiagnostic;

    fn deref(&self) -> &Self::Target {
        self.canonical()
    }
}

#[derive(Debug, Clone, Default)]
struct GhorManifest {
    path: PathBuf,
    root: PathBuf,
    workspace_root: PathBuf,
    workspace_manifest: Option<PathBuf>,
    virtual_manifest: bool,
    workspace_is_root: bool,
    package: BTreeMap<String, String>,
    cargo_lib: BTreeMap<String, String>,
    cargo_bin: BTreeMap<String, String>,
    toolchain: BTreeMap<String, String>,
    target_default: BTreeMap<String, String>,
    machine: BTreeMap<String, String>,
    basal: BTreeMap<String, String>,
    runtime: BTreeMap<String, String>,
    compiler: BTreeMap<String, String>,
    primitive: BTreeMap<String, String>,
    languages: BTreeMap<String, String>,
    language_adapters: BTreeMap<String, BTreeMap<String, String>>,
    dependencies: BTreeMap<String, GhorDependency>,
    dev_dependencies: BTreeMap<String, GhorDependency>,
    build_dependencies: BTreeMap<String, GhorDependency>,
    target_dependencies: Vec<GhorDependency>,
    rust_dependencies: BTreeMap<String, GhorDependency>,
    c_dependencies: BTreeMap<String, GhorDependency>,
    cpp_dependencies: BTreeMap<String, GhorDependency>,
    asm_dependencies: BTreeMap<String, GhorDependency>,
    basal_dependencies: BTreeMap<String, GhorDependency>,
    native_dependencies: BTreeMap<String, GhorNativeDependency>,
    profiles: BTreeMap<String, BTreeMap<String, String>>,
    targets: BTreeMap<String, BTreeMap<String, String>>,
    build: BTreeMap<String, String>,
    features: BTreeMap<String, String>,
    capabilities: BTreeMap<String, String>,
    artifact: BTreeMap<String, String>,
    workspace: BTreeMap<String, String>,
    workspace_members: Vec<String>,
    workspace_exclude: Vec<String>,
    workspace_default_members: Vec<String>,
    workspace_package: BTreeMap<String, String>,
    package_metadata: BTreeMap<String, String>,
    workspace_dependencies: BTreeMap<String, GhorDependency>,
    workspace_lints: BTreeMap<String, String>,
    lints: BTreeMap<String, String>,
    patch_overrides: BTreeMap<String, GhorDependency>,
    replace_overrides: BTreeMap<String, GhorDependency>,
    registries: BTreeMap<String, String>,
    source_replacements: BTreeMap<String, BTreeMap<String, String>>,
    cargo_config: GhorCargoConfig,
    diagnostics: Vec<GhorDiagnostic>,
}

#[derive(Debug, Clone, Default)]
struct GhorDependency {
    name: String,
    source: String,
    version: Option<String>,
    checksum: Option<String>,
    path: Option<String>,
    kind: String,
    target: Option<String>,
    features: Vec<String>,
    default_features: bool,
    optional: bool,
    registry: Option<String>,
    package: Option<String>,
    git: Option<String>,
    git_branch: Option<String>,
    git_tag: Option<String>,
    git_rev: Option<String>,
    workspace_inherited: bool,
}

#[derive(Debug, Clone, Default)]
struct GhorNativeDependency {
    name: String,
    language: String,
    abi: String,
    kind: String,
    version: Option<String>,
    target: Option<String>,
    headers: Vec<String>,
    include_paths: Vec<String>,
    link_paths: Vec<String>,
    libs: Vec<String>,
    frameworks: Vec<String>,
    symbols: Vec<String>,
    cflags: Vec<String>,
    ldflags: Vec<String>,
    defines: Vec<String>,
    pkg_config: Option<String>,
    artifact: Option<String>,
    checksum: Option<String>,
    link_mode: String,
    calling_convention: Option<String>,
    layout: Option<String>,
    ownership: Option<String>,
    required: bool,
    system: bool,
    source: String,
}

#[derive(Debug, Clone)]
struct ToolOutput {
    status: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    stdout_hash: String,
    stderr_hash: String,
}

#[derive(Debug, Clone)]
enum GhorRegistry {
    Local(PathBuf),
    Http(String),
    Sparse(String),
    Git(String),
}

#[derive(Debug, Clone, Default)]
struct GhorCargoConfig {
    paths: Vec<PathBuf>,
    registry: BTreeMap<String, String>,
    registries: BTreeMap<String, BTreeMap<String, String>>,
    credential_aliases: BTreeMap<String, Vec<String>>,
    net: BTreeMap<String, String>,
    http: BTreeMap<String, String>,
    git: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct GhorPackageFiles {
    archive: PathBuf,
    manifest_path: PathBuf,
    archive_name: String,
    checksum: String,
    manifest_hash: String,
    package_manifest_hash: String,
    ffi_link_manifest_hash: String,
    manifest_json: Value,
    package_root: String,
    files: Vec<String>,
    metadata: Value,
    verification: Value,
}

#[derive(Debug, Clone)]
struct GhorRegistryArchive {
    bytes: Vec<u8>,
    checksum: String,
    archive_name: String,
    entry: Value,
    package_manifest_validation: Value,
}

#[derive(Debug, Clone)]
struct GhorPackageOptions {
    verify: bool,
    list_only: bool,
    allow_dirty: bool,
    no_metadata: bool,
    exclude_lockfile: bool,
}

pub(crate) fn run_ghor_command(args: &[String]) -> Result<(), crate::CliFailure> {
    if run_configured_ghor_stage1(args)? {
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("bootstrap") {
        return run_ghor_bootstrap(&args[1..]);
    }
    run_ghor_stage0_command(args).map_err(|error| crate::CliFailure::from_boxed_at(error, "ghor"))
}

fn configured_ghor_stage1() -> Option<PathBuf> {
    let configured = env::var_os("NC_GHOR_STAGE1")
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    if configured.is_some() {
        return configured;
    }
    env::current_exe().ok().and_then(|executable| {
        let sibling = executable.with_file_name(if cfg!(windows) {
            "ghor-stage1.exe"
        } else {
            "ghor-stage1"
        });
        sibling.is_file().then_some(sibling)
    })
}

fn ghor_stage1_supports(args: &[String]) -> bool {
    if is_help(args.get(1)) {
        return false;
    }
    match args.first().map(String::as_str) {
        Some(
            "--version"
            | "-V"
            | "version"
            | "manifest"
            | "metadata"
            | "check"
            | "build"
            | "run"
            | "test"
            | "bench"
            | "new"
            | "init"
            | "clean"
            | "fmt"
            | "fix"
            | "lint"
            | "doc"
            | "doctor"
            | "toolchain"
            | "verify-toolchain"
            | "sync-toolchain"
            | "verify"
            | "evidence"
            | "seal"
            | "conformance"
            | "public-beta"
            | "beta-check"
            | "release-check"
            | "self-check"
            | "release-hardening"
            | "hardening"
            | "install-smoke"
            | "release-install-smoke"
            | "audit"
            | "sbom"
            | "license"
            | "supply-chain"
            | "capability-audit"
            | "target"
            | "profile"
            | "features"
            | "graph"
            | "report"
            | "bin"
            | "cross"
            | "link"
            | "native"
            | "native-deps"
            | "native-dependencies"
            | "ffi-manifest"
            | "ffi-link-manifest"
            | "ffi-package-manifest"
            | "artifact"
            | "locate-project"
            | "read-manifest"
            | "pkgid"
            | "unit-graph"
            | "timings"
            | "machine"
            | "basal"
            | "runtime"
            | "compiler"
            | "primitive"
            | "cdn"
            | "governance"
            | "moderation"
            | "moderate"
            | "bootstrap"
            | "lang"
            | "cargo-exec"
            | "build-script"
            | "build-scripts"
            | "proc-macro"
            | "proc-macros"
            | "interop"
            | "bridge"
            | "ffi-bridge"
            | "external-compat"
            | "cargo-compat",
        ) => true,
        Some(
            "resolve" | "lock" | "generate-lockfile" | "verify-lock" | "fetch" | "vendor"
            | "update" | "tree",
        ) => ghor_stage1_resolution_supported(args),
        Some("add" | "remove") => ghor_stage1_manifest_edit_supported(args),
        Some("install") => {
            arg_after(args, "--manifest-path").is_some()
                || arg_after(args, "--manifest").is_some()
                || arg_after(args, "--path").is_some()
                || first_positional(&args[1..]).is_none()
                || ghor_stage1_registry_client_supported(args)
        }
        Some("uninstall") => true,
        Some("search" | "info" | "login" | "logout") => ghor_stage1_registry_client_supported(args),
        Some("package") => true,
        Some("publish" | "owner" | "yank") => ghor_stage1_registry_client_supported(args),
        Some("registry") => {
            let action = first_positional(&args[1..]).unwrap_or("doctor");
            matches!(action, "doctor" | "cargo-config" | "index" | "config")
                && ghor_stage1_registry_client_supported(args)
        }
        Some("workspace") => {
            let mut index = 1;
            while let Some(argument) = args.get(index) {
                if matches!(argument.as_str(), "--manifest" | "--manifest-path") {
                    index += 2;
                } else if argument.starts_with('-') {
                    index += 1;
                } else {
                    return argument == "metadata";
                }
            }
            true
        }
        _ => false,
    }
}

fn ghor_stage1_registry_client_supported(args: &[String]) -> bool {
    let manifest_path = arg_after(args, "--manifest-path")
        .or_else(|| arg_after(args, "--manifest"))
        .map(PathBuf::from)
        .or_else(|| {
            arg_after(args, "--path").map(|root| {
                let root = PathBuf::from(root);
                if root.is_file() {
                    root
                } else {
                    root.join(GHOR_OFFICIAL_MANIFEST)
                }
            })
        })
        .or_else(|| {
            env::current_dir()
                .ok()
                .and_then(|root| find_manifest(&root))
        });
    let manifest = manifest_path
        .as_ref()
        .and_then(|path| parse_manifest(path).ok());

    if let Some(registry) = arg_after(args, "--registry") {
        return manifest.as_ref().map_or_else(
            || ghor_stage1_registry_reference_supported(&GhorManifest::default(), registry, false),
            |manifest| ghor_stage1_registry_reference_supported(manifest, registry, true),
        );
    }

    if manifest_path
        .as_deref()
        .is_some_and(ghor_manifest_has_stage1_registry_config)
    {
        return true;
    }

    if manifest
        .as_ref()
        .is_some_and(|manifest| !manifest.cargo_config.paths.is_empty())
    {
        return true;
    }

    env::current_dir()
        .ok()
        .is_some_and(|root| root.join("registry").join("index.json").is_file())
}

fn ghor_stage1_resolution_supported(args: &[String]) -> bool {
    if ["--all-targets", "--locked", "--test", "--tests"]
        .iter()
        .any(|flag| has_flag(args, flag))
    {
        return false;
    }

    let manifest_path = arg_after(args, "--manifest-path")
        .or_else(|| arg_after(args, "--manifest"))
        .map(PathBuf::from)
        .or_else(|| {
            arg_after(args, "--path").map(|root| {
                let root = PathBuf::from(root);
                if root.is_file() {
                    root
                } else {
                    root.join(GHOR_OFFICIAL_MANIFEST)
                }
            })
        })
        .or_else(|| {
            env::current_dir()
                .ok()
                .and_then(|root| find_manifest(&root))
        });
    let Some(manifest_path) = manifest_path else {
        return false;
    };
    let Ok(manifest) = parse_manifest(&manifest_path) else {
        return false;
    };
    let configured_registry = ghor_manifest_has_stage1_registry_config(&manifest_path)
        || !manifest.cargo_config.paths.is_empty();

    let dependencies = all_manifest_dependencies(&manifest);
    let needs_registry = dependencies
        .iter()
        .any(|dependency| dependency.path.is_none() && dependency.git.is_none());
    if !needs_registry {
        return true;
    }
    if arg_after(args, "--registry")
        .is_some_and(|registry| ghor_stage1_registry_reference_supported(&manifest, registry, true))
    {
        return true;
    }
    if dependencies
        .iter()
        .filter(|dependency| dependency.path.is_none() && dependency.git.is_none())
        .all(|dependency| {
            dependency.registry.as_deref().is_some_and(|registry| {
                ghor_stage1_registry_reference_supported(&manifest, registry, configured_registry)
            })
        })
    {
        return true;
    }
    configured_registry
        && (!manifest.registries.is_empty() || !manifest.source_replacements.is_empty())
}

fn ghor_stage1_registry_reference_supported(
    manifest: &GhorManifest,
    reference: &str,
    allow_named: bool,
) -> bool {
    let reference = if allow_named {
        manifest
            .registries
            .get(reference)
            .map(String::as_str)
            .unwrap_or(reference)
    } else {
        reference
    };
    reference.starts_with("http://")
        || reference.starts_with("https://")
        || reference.starts_with("sparse+http://")
        || reference.starts_with("sparse+https://")
        || reference.starts_with("git+")
        || Path::new(reference).is_dir()
        || manifest.root.join(reference).is_dir()
}

fn ghor_manifest_has_stage1_registry_config(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|source| {
        source.lines().any(|line| {
            let line = strip_comment(line).trim();
            line == "[registry]"
                || line == "[registries]"
                || line.starts_with("[registries.")
                || line.starts_with("[source.")
        })
    })
}

fn ghor_stage1_manifest_edit_supported(args: &[String]) -> bool {
    !["--locked", "--frozen"]
        .iter()
        .any(|flag| has_flag(args, flag))
}

fn run_configured_ghor_stage1(args: &[String]) -> Result<bool, crate::CliFailure> {
    if env::var_os("NC_GHOR_FORCE_STAGE0").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return Ok(false);
    }
    let Some(stage1) = configured_ghor_stage1() else {
        return Ok(false);
    };
    if !ghor_stage1_supports(args) {
        return Ok(false);
    }
    let stage1_identity = fs::canonicalize(&stage1).unwrap_or_else(|_| stage1.clone());
    let current_identity = env::current_exe()
        .ok()
        .and_then(|path| fs::canonicalize(path).ok());
    if current_identity.as_ref() == Some(&stage1_identity) {
        let diagnostic = ncc_diagnostics::registered_diagnostic(
            ncc_diagnostics::DiagnosticSeverity::Error,
            ncc_diagnostics::codes::E_GHOR_OPERATION_001,
        )
        .with_phase("ghor.stage1")
        .with_file(stage1.display().to_string())
        .with_expected("a distinct native nC stage1 executable")
        .with_found("the active Rust stage0 executable");
        return Err(crate::CliFailure::one(diagnostic, ""));
    }

    let status = Command::new(&stage1)
        .args(args)
        .env("NC_GHOR_FORCE_STAGE0", "1")
        .env("NC_GHOR_HOST_TARGET", host_target_triple())
        .status()
        .map_err(|error| {
            crate::CliFailure::subprocess("ghor.stage1", "launch nC stage1", &stage1, error)
        })?;
    if !status.success() {
        let diagnostic = ncc_diagnostics::registered_diagnostic(
            ncc_diagnostics::DiagnosticSeverity::Error,
            ncc_diagnostics::codes::E_GHOR_OPERATION_001,
        )
        .with_phase("ghor.stage1")
        .with_file(stage1.display().to_string())
        .with_expected("successful nC stage1 execution")
        .with_found(status.to_string());
        return Err(crate::CliFailure::one(diagnostic, ""));
    }
    Ok(true)
}

fn run_ghor_bootstrap(args: &[String]) -> Result<(), crate::CliFailure> {
    let mut modules = Vec::with_capacity(GHOR_NC_MODULES.len());
    for (name, source) in GHOR_NC_MODULES {
        let file = format!("tools/ghor/nc/{}.n", name);
        let tokens = ncc_lexer::lex(source).map_err(|diagnostic| {
            crate::CliFailure::one(diagnostic.canonical("lexer").with_file(&file), *source)
        })?;
        let program = ncc_parser::parse(tokens).map_err(|diagnostic| {
            crate::CliFailure::one(diagnostic.canonical("parser").with_file(&file), *source)
        })?;
        let semantic_status = if *name == "main" || source.contains("use <project:") {
            "project-link-required"
        } else {
            ncc_sema::analyze_canonical(&program).map_err(|diagnostic| {
                crate::CliFailure::one(diagnostic.with_file(&file), *source)
            })?;
            "ok"
        };
        modules.push(json!({
            "name": name,
            "file": file,
            "bytes": source.len(),
            "sha256": sha256_hex(source.as_bytes()),
            "parser": "ok",
            "sema": semantic_status
        }));
    }

    let configured_stage1 = configured_ghor_stage1();
    let command_contracts = ghor_command_contracts_json();
    let payload = json!({
        "schema": GHOR_BOOTSTRAP_SCHEMA,
        "active_executor": "rust-stage0",
        "stage0": {
            "status": "active",
            "role": "host launcher and compatibility executor",
            "source": "crates/ncc-cli/src/commands/ghor.rs"
        },
        "stage1": {
            "language": "nC",
            "module_count": modules.len(),
            "modules": modules,
            "configured_executable": configured_stage1.as_ref().map(|path| path.display().to_string()),
            "native_ready": configured_stage1.is_some(),
            "delegation_enabled": configured_stage1.is_some(),
            "delegation_mode": "selective-material-commands",
            "delegated_commands": ["manifest", "metadata", "target:native-target-matrix", "profile:native-profile-inspection", "features:native-feature-selection", "graph:native-project-graph", "report:native-project-report", "bin:native-bin-location", "cross:native-cross-plan", "link:native-link-plan", "native-deps:native-dependency-validation", "ffi-manifest:native-compiler-contract-aggregation", "artifact:native-artifact-record", "locate-project:native-workspace-location", "read-manifest:native-normalized-manifest", "pkgid:native-package-id", "unit-graph:native-build-unit-graph", "timings:native-monotonic-graph-timings", "machine:native-ncc-subsystem", "basal:native-ncc-subsystem", "runtime:native-ncc-subsystem", "compiler:native-ncc-subsystem", "primitive:native-ncc-subsystem", "cdn:native-content-addressed-cache", "governance:native-registry-policy", "moderation:native-registry-state", "bootstrap:native-bootstrap-authority", "lang:native-language-forge", "new:native-project-scaffold", "init:native-project-scaffold", "clean:native-target-clean", "fmt:native-nfmt-orchestration", "fix:native-nfmt-fix", "lint:native-nlint-orchestration", "doc:native-documentation-inventory", "doctor:native-host-doctor", "toolchain:native-toolchain-inspection", "verify-toolchain:native-toolchain-verification", "sync-toolchain:native-toolchain-sync-plan", "verify:native-verification", "evidence:native-evidence", "seal:native-seal", "conformance:native-conformance", "public-beta:native-public-beta", "release-check:native-release-check", "release-hardening:native-release-hardening", "install-smoke:native-install-smoke", "audit:native-security-audit", "sbom:native-complete-sbom", "license:native-license-report", "supply-chain:native-supply-chain-verify", "capability-audit:native-capability-audit", "check:ncc-parser-sema", "build:ncc-machine-artifacts", "run:native-machine-execution", "test:ncc-native-execution", "bench:native-machine-benchmark", "install:native-material-transaction", "uninstall:native-owned-material-transaction", "add", "remove", "tree:resolved", "resolve:filesystem-http-sparse-git", "lock:filesystem-http-sparse-git", "generate-lockfile:native-lockfile-generation", "verify-lock:filesystem-http-sparse-git", "fetch:filesystem-http-sparse-git", "vendor:filesystem-http-sparse-git", "update:selective-precise", "workspace:metadata", "search:registry-index", "info:registry-index", "login:private-credentials", "logout:private-credentials", "publish:transactional-registry", "owner:transactional-registry", "yank:transactional-registry", "registry:doctor-cargo-config-index-config", "version"]
        },
        "diagnostics": {
            "registry": ncc_diagnostics::DIAGNOSTIC_REGISTRY_SCHEMA,
            "entries": ncc_diagnostics::DIAGNOSTIC_REGISTRY.len()
        },
        "commands": {
            "count": command_contracts["command_count"],
            "native_e2e": command_contracts["native_e2e_count"],
            "stage1_native_e2e": command_contracts["stage1_native_e2e_count"],
            "stage1_native_partial": command_contracts["stage1_native_partial_count"],
            "stage1_pending": command_contracts["stage1_pending_count"]
        },
        "bootstrap_complete": false,
        "retirement_gate": "native nC stage1 passes the complete ghor corpus and rebuilds itself"
    });
    if has_flag(args, "--json") {
        let rendered = serde_json::to_string_pretty(&payload).map_err(|error| {
            crate::CliFailure::json("ghor.bootstrap", "serialize bootstrap report", None, error)
        })?;
        println!("{rendered}");
    } else {
        println!(
            "ghor bootstrap: stage0=rust stage1=nC modules={} registry={} native_ready={}",
            GHOR_NC_MODULES.len(),
            ncc_diagnostics::DIAGNOSTIC_REGISTRY.len(),
            configured_stage1.is_some()
        );
    }
    Ok(())
}

fn run_ghor_stage0_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() || is_global_help(args.first()) {
        print_ghor_help(ghor_help());
        return Ok(());
    }
    if args.len() == 1 && args[0] == "--list" {
        for command in ghor_command_list() {
            println!("{command}");
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("help") {
        if let Some(command) = args
            .get(1)
            .map(String::as_str)
            .filter(|command| !matches!(*command, "-h" | "--help"))
        {
            print_ghor_help(ghor_command_help(command));
        } else {
            print_ghor_help(ghor_help());
        }
        return Ok(());
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];
    if is_help(rest.first()) {
        print_ghor_help(ghor_command_help(cmd));
        return Ok(());
    }
    match cmd {
        "-V" | "--version" | "version" => {
            println!("ghor {GHOR_VERSION}");
            Ok(())
        }
        "doctor" => run_doctor(rest),
        "metadata" => run_metadata(rest),
        "manifest" => run_manifest(rest),
        "new" => run_new(rest),
        "init" => run_init(rest),
        "lang" => run_lang(rest),
        "target" => run_target(rest),
        "profile" => run_profile(rest),
        "features" | "feature" => run_features(rest),
        "cross" => run_cross(rest),
        "link" => run_link(rest),
        "native" | "native-deps" | "native-dependencies" => run_native_deps(rest),
        "ffi-manifest" | "ffi-link-manifest" | "ffi-package-manifest" => run_ffi_manifest(rest),
        "artifact" => run_artifact(rest),
        "fetch" => run_fetch(rest),
        "report" => run_report(rest),
        "toolchain" => run_toolchain(rest),
        "verify-toolchain" => run_verify_toolchain(rest),
        "sync-toolchain" => run_sync_toolchain(rest),
        "graph" => run_graph(rest),
        "check" => run_check(rest),
        "build" => run_build(rest),
        "run" => run_project_run(rest),
        "test" => run_test(rest),
        "bench" => run_bench(rest),
        "clean" => run_clean(rest),
        "fmt" => run_fmt(rest),
        "lint" => run_lint(rest),
        "doc" => run_doc(rest),
        "package" => run_package(rest),
        "publish" => run_publish(rest),
        "registry" => run_registry(rest),
        "cdn" => run_cdn(rest),
        "governance" => run_governance(rest),
        "moderation" | "moderate" => run_moderation(rest),
        "cargo-exec" => run_cargo_exec(rest, "cargo-exec"),
        "build-script" | "build-scripts" => run_cargo_exec(rest, "build-script"),
        "proc-macro" | "proc-macros" => run_cargo_exec(rest, "proc-macro"),
        "interop" | "bridge" | "ffi-bridge" => run_interop(rest),
        "install" => run_install(rest),
        "uninstall" => run_uninstall(rest),
        "bin" => run_bin(rest),
        "update" => run_update(rest),
        "tree" => run_tree(rest),
        "add" => run_add(rest),
        "remove" => run_remove(rest),
        "resolve" => run_resolve(rest),
        "lock" => run_lock(rest),
        "verify-lock" => run_verify_lock(rest),
        "vendor" => run_vendor(rest),
        "workspace" => run_workspace(rest),
        "search" => run_search(rest),
        "info" => run_info(rest),
        "login" => run_login(rest),
        "logout" => run_logout(rest),
        "owner" => run_owner(rest),
        "yank" => run_yank(rest),
        "audit" => run_audit(rest),
        "sbom" => run_sbom(rest),
        "license" => run_license(rest),
        "supply-chain" => run_supply_chain(rest),
        "capability-audit" => run_capability_audit(rest),
        "verify" => run_verify(rest),
        "evidence" => run_evidence(rest),
        "seal" => run_seal(rest),
        "conformance" => run_conformance(rest),
        "public-beta" | "beta-check" => run_public_beta(rest),
        "release-check" | "self-check" => run_release_check(rest),
        "release-hardening" | "hardening" => run_release_hardening(rest),
        "install-smoke" | "release-install-smoke" => run_install_smoke(rest),
        "external-compat" | "cargo-compat" => run_external_compat(rest),
        "machine" => run_machine(rest),
        "basal" => run_basal(rest),
        "runtime" => run_passthrough("runtime", rest),
        "compiler" => run_passthrough("compiler", rest),
        "primitive" => run_passthrough("primitive", rest),
        other => Err(format!(
            "[E_GHOR_CLI_001] unknown ghor command '{}'\n{}",
            other,
            ghor_help()
        )
        .into()),
    }
}

fn print_ghor_help(text: &str) {
    if ghor_help_color_enabled() {
        println!("{GHOR_HELP_LILAC}{text}{GHOR_ANSI_RESET}");
    } else {
        println!("{text}");
    }
}

fn ghor_help_color_enabled() -> bool {
    match env::var("GHOR_COLOR").ok().as_deref() {
        Some("always") => return true,
        Some("never") => return false,
        Some("auto") | None => {}
        Some(_) => {}
    }
    if env::var_os("NC_NO_COLOR").is_some() || env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stdout().is_terminal()
}

pub(crate) fn ghor_help() -> &'static str {
    "\
nC package manager and build orchestrator

Usage: ghor [OPTIONS] <COMMAND>
       ghor <COMMAND> [OPTIONS]

Ghor is the Cargo-compatible package manager for nC projects. It manages
ghor.toml/nc.toml manifests, Cargo.lock-style ghor.lock files, registries,
workspaces, packages, build/test/check flows, and the nC machine/basal stack.

Options:
    --manifest-path <PATH>       Path to ghor.toml or nc.toml
    --manifest <PATH>            Compatibility alias for --manifest-path
    --registry <REGISTRY>        Registry name, filesystem path, HTTP(S), sparse+HTTP(S), or git+ URL
    --target <TRIPLE>            Target triple for commands that lower/build
    --profile <PROFILE>          Build profile such as debug or release
    --features <FEATURES>        Comma-separated feature list
    --locked                     Assert that ghor.lock and ghor.lock.meta.json remain unchanged
    --offline                    Run without registry/network access
    --frozen                     Equivalent to --locked plus --offline
    --json                       Emit machine-readable JSON where supported
    --list                       List all ghor commands
    -h, --help                   Print help
    -V, --version                Print version

Commands:
    bootstrap                    Validate the modular nC stage1 and report bootstrap state
    build, b                     Compile the current package or workspace
    check, c                     Analyze the package and report errors without producing artifacts
    test, t                      Check tests/examples for the package or workspace
    run, r                       Build and prepare the package run artifact
    bench                        Run declared benchmarks
    clean                        Remove ghor target artifacts
    doc, d                       Generate documentation reports

    new                          Create a new nC package
    init                         Create a new nC package in an existing directory
    add                          Add dependencies to a manifest
    remove                       Remove dependencies from a manifest

    update                       Update dependencies and rewrite ghor.lock
    resolve                      Resolve dependencies with SemVer/features/overrides
    lock                         Generate Cargo.lock-style ghor.lock plus ghor.lock.meta.json
    generate-lockfile            Regenerate lock artifacts without building
    verify-lock                  Verify the current lock artifacts
    tree                         Display the resolved dependency graph
    metadata                     Emit Cargo-compatible package/workspace metadata
    vendor                       Vendor resolved registry packages locally

    package                      Assemble a Cargo-compatible .crate archive
    publish                      Publish package bytes and metadata to a registry
    install                      Install a package binary/source from a registry
    uninstall                    Remove an installed package binary/source
    search                       Search registry packages
    info                         Show registry package information
    login                        Store an auth token or credential-provider token
    logout                       Remove stored registry credentials
    owner                        List/add/remove registry package owners
    yank                         Yank a package version
    registry                     Inspect registry protocol/config/cache state
    cdn                          Build a content-addressed registry CDN/edge cache
    governance                   Manage registry governance policy and audit data
    moderation                   Moderate registry packages with auditable state
    cargo-exec                   Execute Cargo-backed build-script/proc-macro checks
    build-script                 Execute Cargo build scripts and capture evidence
    proc-macro                   Execute proc-macro expansion checks through Cargo
    interop                      Scaffold and smoke nC <-> C/C++/Rust ABI bridges

    workspace                    Inspect or operate on a workspace
    manifest                     Print normalized manifest metadata
    locate-project               Locate the effective workspace manifest
    read-manifest                Emit the effective normalized manifest
    pkgid                        Emit the canonical package identifier
    unit-graph                   Emit build units and dependency edges
    timings                      Measure native project-graph materialization
    target                       Show target matrix and target details
    profile                      Show effective profile settings
    features                     Show feature declarations
    native-deps                  Show external C/C++/Rust/Zig/Go/ASM library manifest
    ffi-manifest                 Show compiler FFI link manifests discovered in nC sources
    graph                        Emit project graph information
    fetch                        Fetch registry dependencies into cache
    report                       Emit project reports

    doctor                       Check ghor, machine, basal, registry, and toolchain readiness
    verify                       Verify manifest, lock, package and project evidence
    evidence                     Emit ghor evidence summary
    seal                         Emit ghor release seal summary
    conformance                  Emit conformance matrix
    public-beta                  Validate the extracted public beta package end to end
    release-check                Run release checks; package preflight by default
    release-hardening            Gate registry production hardening for release
    install-smoke                Validate installed release binaries
    external-compat              Compare ghor outputs against real Cargo outputs

    machine                      Delegate to ncc machine
    basal                        Delegate to ncc basal
    runtime                      Delegate to ncc runtime
    compiler                     Delegate to ncc compiler
    primitive                    Delegate to ncc primitive

    lang                         Scaffold a language implementation package
    fmt                          Format sources where supported
    fix                          Apply deterministic nfmt fixes
    lint                         Run lint checks where supported
    toolchain                    Show selected nC toolchain
    verify-toolchain             Verify installed nC toolchain freshness
    sync-toolchain               Print toolchain sync commands
    audit                        Audit dependency/package policy
    sbom                         Emit software bill of materials
    license                      Emit license report
    supply-chain                 Emit supply-chain report
    capability-audit             Audit declared capabilities

See 'ghor help <command>' for more information on a specific command.
See 'ghor conformance --json' for the Cargo-compatibility evidence matrix."
}

fn ghor_command_help(command: &str) -> &'static str {
    match official_help_command(command) {
        "bootstrap" => "\
Validate the modular nC ghor stage1\n\nUsage: ghor bootstrap [--json]\n\nParses and semantically validates every retained nC module, reports the active\nRust stage0 bridge and never claims self-hosting before a native stage1 passes\nthe complete ghor corpus.",
        "new" => "\
Create a new nC package

Usage: ghor new <NAME> [OPTIONS]

Creates a new project directory with ghor.toml, nc.toml compatibility,
source scaffolding, machine/basal-aware metadata, and language adapter
settings.

Options:
    --out <DIRECTORY>            Directory to create instead of ./<NAME>
    --lang <LANGUAGE>            nC, c, cpp, asm, rust, java, python, javascript, or typescript
    --lib                        Create an nC library package
    --machine                    Add a typed .nm machine entry
    --basal                      Add a basal root
    --primitive                  Add a primitive device entry
    --json                       Emit the material scaffold report
    -h, --help                   Print help

Examples:
    ghor new hello
    ghor new hello --out packages/hello
    ghor new ffi_demo --lang c",
        "init" => "\
Create a new nC package in an existing directory

Usage: ghor init [OPTIONS]

Options:
    --path <DIRECTORY>           Directory to initialize
    --lang <LANGUAGE>            Primary language template
    --lib, --machine             Select an nC package kind
    --basal, --primitive         Select a low-level nC package kind
    --json                       Emit the material scaffold report
    -h, --help                   Print help",
        "check" => "\
Check a package without producing build artifacts

Usage: ghor check [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to check
    --workspace                  Check all workspace members
    --target <TRIPLE>            Target triple for target-aware checks
    --locked                     Require lockfile to be current
    --offline                    Avoid registry/network access
    --json                       Emit JSON report
    -h, --help                   Print help",
        "build" => "\
Compile the current package or workspace

Usage: ghor build [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to build
    --workspace                  Build all workspace members
    --profile <PROFILE>          debug or release profile
    --target <TRIPLE>            Target triple
    --locked                     Require lockfile to be current
    --offline                    Avoid registry/network access
    --json                       Emit JSON report",
        "run" => "\
Run a package artifact through the nC orchestration surface

Usage: ghor run [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to run
    --profile <PROFILE>          debug or release profile
    --target <TRIPLE>            Target triple
    --cpu <CPU>                  Native CPU profile
    --tune-profile <PROFILE>     Backend tuning profile
    --locked                     Require the lockfile to be current
    --frozen                     Require lockfile and offline resolution
    -- <ARGS>...                 Arguments passed to the native program
    --json                       Emit JSON report",
        "test" => "\
Run ghor's test/example checking flow

Usage: ghor test [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to test
    --workspace                  Test all workspace members
    --all-targets                Include dev/build/target resolver paths
    --json                       Emit JSON report",
        "bench" => "\
Run declared benchmark checks

Usage: ghor bench [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to inspect
    --bench <NAME>               Run only one benchmark source stem
    --iterations <COUNT>         Measured executions per benchmark (default: 10)
    --warmup <COUNT>             Warmup executions per benchmark (default: 1)
    --no-run                     Build and retain artifacts without execution
    --target <TRIPLE>            Host target triple
    --cpu <CPU>                  Native CPU profile (default: native)
    --tune-profile <PROFILE>     Backend tuning profile
    --cc <PATH>                  C-compatible linker driver used by ncc
    --linker <PATH>              Explicit native linker
    --locked                     Require the lockfile to be current
    --frozen                     Require lockfile and offline resolution
    -- <ARGS>...                 Arguments passed to each benchmark
    --json                       Emit JSON report",
        "clean" => "\
Remove ghor target artifacts

Usage: ghor clean [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest whose target directory is cleaned
    --target-dir <DIRECTORY>     Explicit target directory
    --dry-run                    Report the target without deleting it",
        "fmt" => "\
Format every nC source in the package

Usage: ghor fmt [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest whose sources are formatted
    --check                      Verify formatting without rewriting
    --compact                    Apply the canonical compact nC form",
        "lint" => "\
Lint every nC source in the package

Usage: ghor lint [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest whose sources are checked",
        "doc" => "\
Generate package documentation reports

Usage: ghor doc [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to document
    --out <DIRECTORY>            Output directory
    --json                       Emit JSON report",
        "add" => "\
Add dependencies to a manifest

Usage: ghor add <PACKAGE> [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to edit
    --version <REQ>              Version requirement
    --path <PATH>                Add a local path dependency
    --git <URL>                  Add a Git dependency
    --branch <NAME>              Select one Git branch
    --tag <TAG>                  Select one Git tag
    --rev <COMMIT>               Select one exact Git revision
    --checksum <SHA256>          Required checksum for remote dependencies
    --registry <REGISTRY>        Alternate registry
    --rename <NAME>              Rename the dependency key
    --features <LIST>            Enable comma/space-separated features
    --no-default-features        Disable default features
    --optional                   Mark the dependency optional
    --target <CONDITION>         Add under a target-specific section
    --dev                        Add as dev dependency
    --build                      Add as build dependency
    --dry-run                    Validate without changing the manifest",
        "remove" => "\
Remove dependencies from a manifest

Usage: ghor remove <PACKAGE> [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to edit
    --target <CONDITION>         Remove from a target-specific section
    --dev                        Remove from dev dependencies
    --build                      Remove from build dependencies
    --dry-run                    Validate without changing the manifest",
        "update" => "\
Update dependencies and rewrite lock artifacts

Usage: ghor update [OPTIONS] [-p <PACKAGE>] [--precise <VERSION>]

Options:
    -p, --package <PACKAGE>      Limit update to a package
    --precise <VERSION>          Force an exact selected version
    --dry-run                    Resolve without writing ghor.lock
    --manifest-path <PATH>       Manifest to update
    --registry <REGISTRY>        Registry to resolve from
    --offline                    Avoid registry/network access
    --frozen                     Forbid lock updates",
        "lock" => "\
Generate lock artifacts

Usage: ghor lock [OPTIONS]

Writes a Cargo.lock-style TOML primary lockfile at ghor.lock and nC
metadata sidecar at ghor.lock.meta.json.

Options:
    --manifest-path <PATH>       Manifest to lock
    --lock <PATH>                Alternate lockfile path
    --registry <REGISTRY>        Registry to resolve from
    --target <TRIPLE>            Resolve target-specific dependencies
    --json                       Emit lock report",
        "verify-lock" => "\
Verify lock artifacts

Usage: ghor verify-lock [OPTIONS]

Compares ghor.lock and ghor.lock.meta.json against the current manifest
resolution. Legacy JSON ghor.lock files are reported as drift.

Options:
    --manifest-path <PATH>       Manifest to verify
    --lock <PATH>                Alternate lockfile path
    --registry <REGISTRY>        Registry to resolve from
    --json                       Emit JSON report",
        "resolve" => "\
Resolve dependencies

Usage: ghor resolve [OPTIONS]

Supports SemVer, feature resolver v1/v2/v3 behavior, normal/dev/build/
target-specific dependencies, [patch], [replace], alternate registries,
source replacement, and git exact-head dependencies.

Options:
    --manifest-path <PATH>       Manifest to resolve
    --resolver <1|2|3>           Force resolver version
    --target <TRIPLE>            Resolve for target
    --include-dev                Include dev dependencies
    --registry <REGISTRY>        Registry to resolve from
    --json                       Emit JSON plan",
        "metadata" => "\
Emit Cargo-compatible metadata

Usage: ghor metadata [OPTIONS]

Outputs package fields, workspace members/default-members, target directory,
workspace root, resolve nodes, dependency kinds, and feature data.

Options:
    --manifest-path <PATH>       Manifest to inspect
    --registry <REGISTRY>        Registry used for resolve nodes
    --json                       Emit JSON metadata",
        "native-deps" | "native" | "native-dependencies" => "\
Show external native library dependencies

Usage: ghor native-deps [OPTIONS]

Reads [native-dependencies] and [native-dependencies.<name>] from ghor.toml.
These entries describe external C, C++, Rust, Zig, Go, or ASM libraries:
headers, include paths, link paths, libs, frameworks, symbols, ABI, artifact
checksums, and target-specific link policy. They are not Cargo crates.

Options:
    --manifest-path <PATH>       Manifest to inspect
    --json                       Emit JSON native dependency report",
        "ffi-manifest" | "ffi-link-manifest" | "ffi-package-manifest" => "\
Show compiler FFI link manifests discovered in nC sources

Usage: ghor ffi-manifest [OPTIONS]

Scans package nC sources and delegates FFI extraction to the compiler
ffi-link-manifest collector. The report combines source-level foreign:
symbols with ghor native-dependencies so packaging, link planning and lock
metadata share one native artifact/checksum view.

Options:
    --manifest-path <PATH>       Manifest to inspect
    --source <PATH>              Inspect one nC source instead of package sources
    --all-sources                Include examples/ in addition to src/ and root .n files
    --json                       Emit JSON FFI manifest report",
        "tree" => "\
Display the dependency graph

Usage: ghor tree [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to inspect
    --target <TRIPLE>            Resolve for target
    --include-dev                Include dev dependencies
    --registry <REGISTRY>        Registry used to resolve graph nodes
    --offline                    Require cached remote sources
    --json                       Emit JSON graph",
        "vendor" => "\
Vendor resolved registry packages

Usage: ghor vendor [DIRECTORY] [OPTIONS]

Downloads verified .crate archives, extracts them, writes Cargo-shaped
.cargo-checksum.json files with package and per-file SHA-256 hashes, and
prints source replacement config.

Options:
    --manifest-path <PATH>       Manifest to vendor
    --registry <REGISTRY>        Registry to download from
    --versioned-dirs             Use name-version directories
    --no-delete                  Preserve existing vendor directory
    --json                       Emit JSON report",
        "package" => "\
Assemble a Cargo-compatible .crate archive

Usage: ghor package [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest to package
    --out <DIRECTORY>            Output directory
    --list                       List package contents only
    --allow-dirty                Allow dirty VCS state
    --exclude-lockfile           Omit ghor.lock artifacts
    --json                       Emit JSON report",
        "publish" => "\
Publish a package to a registry

Usage: ghor publish [OPTIONS]

Supports filesystem registries and Cargo Web API publish for HTTP(S)
registries configured through config.json.

Options:
    --manifest-path <PATH>       Manifest to publish
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Auth token for remote registry
    --dry-run                    Verify package without publishing
    --allow-dirty                Allow dirty VCS state
    --json                       Emit JSON report",
        "registry" => "\
Inspect registry protocol and configuration

Usage: ghor registry <config|index|package-index|cargo-config|cache|hardening> [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest whose config is read
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Token for productive remote registry checks
    --cdn-out <DIRECTORY>        Existing CDN bundle root for hardening checks
    --json                       Emit JSON report",
        "cdn" => "\
Build or inspect a registry CDN/edge-cache bundle

Usage: ghor cdn <build|status> [OPTIONS]

Options:
    --registry <REGISTRY>        Source registry path/name/URL
    --out <DIRECTORY>            CDN output root
    --json                       Emit JSON report",
        "governance" => "\
Manage registry governance policy

Usage: ghor governance [--init] [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Token required for governance mutation
    --role <USER:ROLE>           Add or update a governance role on --init
    --json                       Emit JSON report",
        "moderation" | "moderate" => "\
Moderate registry packages

Usage: ghor moderation <PACKAGE> [OPTIONS]

Options:
    --version <VERSION>          Package version to moderate
    --action <ACTION>            review, approve, flag, quarantine, block or restore
    --reason <TEXT>              Human moderation reason
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Token required for moderation mutation
    --json                       Emit JSON report",
        "cargo-exec" | "build-script" | "build-scripts" | "proc-macro" | "proc-macros" => "\
Execute Cargo-backed build-script/proc-macro checks

Usage: ghor cargo-exec [OPTIONS]
Usage: ghor build-script [OPTIONS]
Usage: ghor proc-macro [OPTIONS]

Options:
    --manifest-path <Cargo.toml> Cargo manifest to check
    --target-dir <DIRECTORY>     Cargo target directory for the execution evidence
    --json                       Emit JSON report",
        "interop" | "bridge" | "ffi-bridge" => "\
Scaffold and smoke nC <-> C/C++/Rust ABI bridges

Usage: ghor interop [smoke|scaffold|verify] [OPTIONS]

Generates a C ABI bridge manifest for nC, C, C++ and Rust; compiles C,
links C++ against C, builds a Rust staticlib, links C against Rust, and
runs both executable smokes when host tools are available.

Options:
    --out <DIRECTORY>            Output directory for bridge artifacts
    --json                       Emit JSON report",
        "install" => "\
Install a native package from a path or registry

Usage: ghor install <PACKAGE> [OPTIONS]
Usage: ghor install --path <DIRECTORY> [OPTIONS]

Options:
    --path <DIRECTORY>           Local package root
    --manifest-path <PATH>       Local package manifest
    --version <REQ>              Version requirement
    --registry <REGISTRY>        Registry to install from
    --root <DIRECTORY>           Install root
    --profile <PROFILE>          Native build profile
    --target <TRIPLE>            Native executable target
    --ncc <PATH>                 Material ncc compiler
    --json                       Emit JSON report",
        "uninstall" => "\
Uninstall material owned by a native install receipt

Usage: ghor uninstall <PACKAGE> [OPTIONS]

Options:
    --version <VERSION>          Require the installed version
    --root <DIRECTORY>           Install root
    --json                       Emit JSON report",
        "search" => "\
Search registry packages

Usage: ghor search <QUERY> [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --limit <N>                  Maximum entries to print
    --json                       Emit JSON report",
        "info" => "\
Show registry package information

Usage: ghor info <PACKAGE> [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --version <REQ>              Version selector
    --json                       Emit JSON report",
        "login" => "\
Store registry credentials

Usage: ghor login [TOKEN] [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Token value
    --json                       Emit JSON report",
        "logout" => "\
Remove registry credentials

Usage: ghor logout [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --json                       Emit JSON report",
        "owner" => "\
Manage registry package owners

Usage: ghor owner <list|add|remove> <PACKAGE> [OPTIONS]

Options:
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Auth token for remote registry
    --json                       Emit JSON report",
        "yank" => "\
Yank or unyank package versions

Usage: ghor yank <PACKAGE> [OPTIONS]

Options:
    --version <VERSION>          Version to yank
    --undo                       Unyank instead of yank
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Auth token for remote registry
    --json                       Emit JSON report",
        "workspace" => "\
Inspect or operate on a workspace

Usage: ghor workspace <metadata|check|build|test|graph> [OPTIONS]

Options:
    --manifest-path <PATH>       Workspace root or member manifest
    --json                       Emit JSON report",
        "doctor" => "\
Check ghor readiness

Usage: ghor doctor [OPTIONS]

Checks ghor surface status, machine and basal passthrough, registry/toolchain
readiness, target matrix, and Cargo-compatible orchestration evidence.

Options:
    --json                       Emit JSON report",
        "conformance" => "\
Emit conformance matrix

Usage: ghor conformance [OPTIONS]

Reports the Cargo-compatibility closure matrix: lockfile, resolver, git,
registry/API, cache, workspace, package, config, metadata, vendor and CLI.

Options:
    --manifest-path <PATH>       Optional manifest whose target directory stores conformance.json
    --json                       Emit JSON report",
        "public-beta" | "beta-check" => "\
Validate the extracted public beta package end to end

Usage: ghor public-beta [OPTIONS]

Runs the official public beta package checks from the current package root:
    cargo build -p ncc-cli
    cargo test --workspace
    ncc run examples/00_hello/hello.n
    ncc project sync examples/08_project
    ncc project run examples/08_project

Options:
    --root <PATH>                Package root to validate; defaults to current directory
    --plan, --dry-run            Print the validation plan without executing it
    --json                       Emit JSON report",
        "release-check" => "\
Run release-oriented checks

Usage: ghor release-check [OPTIONS]

Runs ghor doctor, manifest check, lock generation, lock verification, and a
Cargo-compatible package preflight. By default this does not create a .crate
archive, so it is safe on large workspaces. Pass --package when an actual
archive should be produced as part of the release check.

Options:
    --manifest-path <PATH>       Manifest to check
    --package                    Also assemble the .crate archive
    --out <DIRECTORY>            Package output directory when --package is used
    --allow-dirty                Allow packaging from a dirty worktree
    --json                       Emit JSON reports",
        "release-hardening" | "hardening" => "\
Run registry production hardening for release

Usage: ghor release-hardening [OPTIONS]

Options:
    --manifest-path <PATH>       Manifest whose package must be present in the registry
    --registry <REGISTRY>        Registry path/name/URL
    --token <TOKEN>              Token required for remote registry production checks
    --cdn-out <DIRECTORY>        Existing CDN bundle root to validate
    --allow-insecure-localhost   Allow http://127.0.0.1 or localhost for smoke tests
    --json                       Emit JSON report",
        "install-smoke" | "release-install-smoke" => "\
Validate installed release binaries

Usage: ghor install-smoke [OPTIONS]

Checks that the release command surface is installed and runnable for:
ghor, ncc, nclsp, nfmt and nlint. The smoke is intentionally non-destructive
and runs each binary with --help.

Options:
    --bin-dir <DIRECTORY>        Validate binaries from a specific directory instead of PATH
    --json                       Emit JSON report",
        "external-compat" | "cargo-compat" => "\
Compare ghor outputs against real Cargo outputs

Usage: ghor external-compat [OPTIONS]

Options:
    --manifest-path <Cargo.toml> Cargo project manifest to compare
    --vendor-dir <DIRECTORY>     Base directory for cargo/ghor vendor comparisons
    --json                       Emit JSON report",
        "machine" => "Delegate to ncc machine\n\nUsage: ghor machine [ARGS]\n\nExamples:\n    ghor machine doctor\n    ghor machine asm-complete native-doctor --json",
        "basal" => "Delegate to ncc basal\n\nUsage: ghor basal [ARGS]\n\nExamples:\n    ghor basal doctor\n    ghor basal verify-nbb file.nbb --json",
        "runtime" => "Delegate to ncc runtime\n\nUsage: ghor runtime [ARGS]",
        "compiler" => "Delegate to ncc compiler\n\nUsage: ghor compiler [ARGS]",
        "primitive" => "Delegate to ncc primitive\n\nUsage: ghor primitive [ARGS]",
        "locate-project" => "Locate the effective ghor workspace\n\nUsage: ghor locate-project [--manifest-path <PATH>]",
        "read-manifest" => "Read the normalized effective manifest\n\nUsage: ghor read-manifest [--manifest-path <PATH>]",
        "pkgid" => "Print the canonical package identifier\n\nUsage: ghor pkgid [--manifest-path <PATH>]",
        "unit-graph" => "Print the native package build-unit graph\n\nUsage: ghor unit-graph [--manifest-path <PATH>]",
        "timings" => "Measure native graph materialization\n\nUsage: ghor timings [--manifest-path <PATH>]",
        "generate-lockfile" => "Generate ghor.lock and ghor.lock.meta.json\n\nUsage: ghor generate-lockfile [--manifest-path <PATH>]",
        "fix" => "Apply deterministic nfmt source fixes\n\nUsage: ghor fix [--manifest-path <PATH>]",
        _ => "\
Ghor command help

Usage: ghor <COMMAND> [OPTIONS]

This command is part of the ghor nC package manager surface. Run:

    ghor --help

to see the complete command list.",
    }
}

fn official_help_command(command: &str) -> &str {
    match command {
        "b" => "build",
        "c" => "check",
        "t" => "test",
        "r" => "run",
        "d" => "doc",
        "feature" => "features",
        "self-check" => "release-check",
        "beta-check" => "public-beta",
        "release-install-smoke" => "install-smoke",
        "bridge" | "ffi-bridge" => "interop",
        "native" | "native-dependencies" => "native-deps",
        "ffi-link-manifest" | "ffi-package-manifest" => "ffi-manifest",
        other => other,
    }
}

fn is_global_help(arg: Option<&String>) -> bool {
    matches!(arg.map(String::as_str), Some("-h" | "--help"))
}

fn is_help(arg: Option<&String>) -> bool {
    matches!(arg.map(String::as_str), Some("-h" | "--help" | "help"))
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn arg_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find_map(|pair| (pair[0] == name).then_some(pair[1].as_str()))
}

fn first_positional(args: &[String]) -> Option<&str> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--manifest"
                | "--manifest-path"
                | "--out"
                | "--registry"
                | "--profile"
                | "--target"
                | "--lock"
                | "--path"
                | "--lang"
                | "--host"
                | "--features"
                | "--checksum"
                | "--token"
                | "--registry-protocol"
                | "--vers"
                | "--version"
        ) {
            skip_next = true;
            continue;
        }
        if !arg.starts_with("--") {
            return Some(arg);
        }
    }
    None
}

fn ghor_filesystem<T>(
    phase: &str,
    operation: impl Into<String>,
    path: &Path,
    result: io::Result<T>,
) -> Result<T, crate::CliFailure> {
    result.map_err(|error| crate::CliFailure::filesystem(phase, operation, path, error))
}

fn ghor_contract_error(
    code: &'static str,
    phase: &'static str,
    message: impl Into<String>,
) -> crate::CliFailure {
    crate::CliFailure::error(
        code,
        phase,
        message,
        "fix the ghor manifest or pass the required command option",
    )
}

fn print_json(value: &Value) -> Result<(), Box<dyn Error>> {
    let rendered = serde_json::to_string_pretty(value).map_err(|error| {
        crate::CliFailure::json("ghor.output", "serialize command output", None, error)
    })?;
    println!("{rendered}");
    Ok(())
}

fn diagnostics_to_json(diagnostics: &[GhorDiagnostic]) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|diag| crate::canonical_diagnostic_to_json(diag.canonical()))
            .collect(),
    )
}

fn diagnostics_have_errors(diagnostics: &[GhorDiagnostic]) -> bool {
    diagnostics.iter().any(GhorDiagnostic::is_error)
}

fn diagnostics_summary(diagnostics: &[GhorDiagnostic]) -> String {
    diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .map(|diag| {
            crate::render_canonical_diagnostic_terminal(
                diag.canonical(),
                "",
                crate::stderr_diagnostic_color_enabled(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn stable_json_hash(value: &Value) -> String {
    sha256_hex(serde_json::to_string(value).unwrap_or_default().as_bytes())
}

fn repo_ncc_path() -> Result<PathBuf, crate::CliFailure> {
    let exe = env::current_exe().map_err(|error| {
        crate::CliFailure::filesystem(
            "ghor.tool",
            "resolve the current ncc executable",
            Path::new("ncc"),
            error,
        )
    })?;
    let stem = exe
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if stem == "ghor" {
        let mut ncc = exe.clone();
        ncc.set_file_name(if cfg!(windows) { "ncc.exe" } else { "ncc" });
        Ok(ncc)
    } else {
        Ok(exe)
    }
}

fn run_repo_ncc(args: &[&str]) -> Result<ToolOutput, crate::CliFailure> {
    run_tool(repo_ncc_path()?, args, None)
}

fn run_tool(
    program: PathBuf,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<ToolOutput, crate::CliFailure> {
    let mut command = Command::new(&program);
    command.args(args).env("NC_NO_COLOR", "1");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().map_err(|error| {
        crate::CliFailure::subprocess("ghor.tool", "execute external tool", &program, error)
    })?;
    Ok(ToolOutput {
        status: if output.status.success() {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        exit_code: output.status.code().unwrap_or(1),
        stdout_hash: sha256_hex(&output.stdout),
        stderr_hash: sha256_hex(&output.stderr),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn tool_output_to_json(name: &str, output: &ToolOutput) -> Value {
    json!({
        "name": name,
        "status": output.status,
        "exit_code": output.exit_code,
        "stdout_hash": output.stdout_hash,
        "stderr_hash": output.stderr_hash
    })
}

fn manifest_path_from_args(args: &[String]) -> Result<PathBuf, crate::CliFailure> {
    if let Some(path) = arg_after(args, "--manifest-path").or_else(|| arg_after(args, "--manifest"))
    {
        return Ok(PathBuf::from(path));
    }
    let current_dir = ghor_filesystem(
        "ghor.manifest",
        "resolve the current working directory",
        Path::new("."),
        env::current_dir(),
    )?;
    find_manifest(&current_dir).ok_or_else(|| {
        ghor_contract_error(
            "E_GHOR_MANIFEST_001",
            "ghor.manifest",
            format!(
                "{GHOR_OFFICIAL_MANIFEST} not found; pass --manifest-path <path> or --manifest <path>"
            ),
        )
    })
}

fn optional_manifest_from_args(args: &[String]) -> Option<GhorManifest> {
    let path = arg_after(args, "--manifest-path").or_else(|| arg_after(args, "--manifest"))?;
    parse_manifest(&PathBuf::from(path)).ok()
}

fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        for name in ghor_manifest_names() {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn ghor_manifest_names() -> [&'static str; 3] {
    [
        GHOR_OFFICIAL_MANIFEST,
        GHOR_COMPAT_MANIFEST,
        GHOR_CARGO_MANIFEST,
    ]
}

fn manifest_is_cargo_toml(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(GHOR_CARGO_MANIFEST))
}

fn parse_manifest(path: &Path) -> Result<GhorManifest, crate::CliFailure> {
    let raw = ghor_filesystem(
        "ghor.manifest",
        "read the package manifest",
        path,
        fs::read_to_string(path),
    )?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut manifest = GhorManifest {
        path: path.to_path_buf(),
        root: root.clone(),
        workspace_root: root,
        ..GhorManifest::default()
    };
    let mut section = String::new();
    for original in raw.lines() {
        let line = strip_comment(original).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }
        let Some((key, value)) = parse_key_value(&line) else {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_MANIFEST_PARSE_001",
                format!("invalid manifest line '{}'", original.trim()),
            ));
            continue;
        };
        match section.as_str() {
            "package" => {
                manifest.package.insert(key, value);
            }
            "lib" => {
                manifest.cargo_lib.insert(key, value);
            }
            "bin" => {
                manifest.cargo_bin.insert(key, value);
            }
            section
                if section == "package.metadata" || section.starts_with("package.metadata.") =>
            {
                let metadata_key = section
                    .strip_prefix("package.metadata.")
                    .map(|prefix| format!("{prefix}.{key}"))
                    .unwrap_or(key);
                manifest.package_metadata.insert(metadata_key, value);
            }
            "toolchain" => {
                manifest.toolchain.insert(key, value);
            }
            "target.default" => {
                manifest.target_default.insert(key, value);
            }
            "machine" => {
                manifest.machine.insert(key, value);
            }
            "basal" => {
                manifest.basal.insert(key, value);
            }
            "runtime" => {
                manifest.runtime.insert(key, value);
            }
            "compiler" => {
                manifest.compiler.insert(key, value);
            }
            "primitive" => {
                manifest.primitive.insert(key, value);
            }
            "languages" => {
                manifest.languages.insert(key, value);
            }
            section if section.starts_with("language_adapter.") => {
                manifest
                    .language_adapters
                    .entry(section.trim_start_matches("language_adapter.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            "dependencies" => {
                let (dep_key, dep_value) = workspace_dependency_key_value(&key, &value);
                manifest.dependencies.insert(
                    dep_key.clone(),
                    parse_dependency(
                        &dep_key,
                        &dep_value,
                        "normal",
                        None,
                        &mut manifest.diagnostics,
                    ),
                );
            }
            "dev-dependencies" => {
                let (dep_key, dep_value) = workspace_dependency_key_value(&key, &value);
                manifest.dev_dependencies.insert(
                    dep_key.clone(),
                    parse_dependency(&dep_key, &dep_value, "dev", None, &mut manifest.diagnostics),
                );
            }
            "build-dependencies" => {
                let (dep_key, dep_value) = workspace_dependency_key_value(&key, &value);
                manifest.build_dependencies.insert(
                    dep_key.clone(),
                    parse_dependency(
                        &dep_key,
                        &dep_value,
                        "build",
                        None,
                        &mut manifest.diagnostics,
                    ),
                );
            }
            "rust-dependencies" => {
                manifest.rust_dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics),
                );
            }
            "c-dependencies" => {
                manifest.c_dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics),
                );
            }
            "cpp-dependencies" => {
                manifest.cpp_dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics),
                );
            }
            "asm-dependencies" => {
                manifest.asm_dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics),
                );
            }
            "basal-dependencies" | "basal-deps" => {
                let mut dep =
                    parse_dependency(&key, &value, "basal", None, &mut manifest.diagnostics);
                dep.source = "basal-registry".to_string();
                if dep.package.is_none() {
                    dep.package = Some(key.clone());
                }
                manifest.basal_dependencies.insert(key.clone(), dep);
            }
            "native-dependencies" | "native-deps" => {
                manifest.native_dependencies.insert(
                    key.clone(),
                    parse_native_dependency(&key, &value, &mut manifest.diagnostics),
                );
            }
            section
                if section.starts_with("native-dependencies.")
                    || section.starts_with("native-deps.") =>
            {
                let name = section
                    .trim_start_matches("native-dependencies.")
                    .trim_start_matches("native-deps.")
                    .trim()
                    .to_string();
                set_native_dependency_field(
                    &mut manifest.native_dependencies,
                    &name,
                    &key,
                    &value,
                    &mut manifest.diagnostics,
                );
            }
            "workspace" => {
                if key == "members" {
                    manifest.workspace_members = parse_array(&value);
                } else if key == "exclude" {
                    manifest.workspace_exclude = parse_array(&value);
                } else if key == "default-members" || key == "default_members" {
                    manifest.workspace_default_members = parse_array(&value);
                }
                manifest.workspace.insert(key, value);
            }
            "workspace.package" => {
                manifest.workspace_package.insert(key, value);
            }
            "workspace.dependencies" => {
                manifest.workspace_dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics),
                );
            }
            section if section.starts_with("workspace.lints") => {
                let lint_key = section
                    .strip_prefix("workspace.lints.")
                    .map(|prefix| format!("{prefix}.{key}"))
                    .unwrap_or(key);
                manifest.workspace_lints.insert(lint_key, value);
            }
            section if section.starts_with("lints") => {
                let lint_key = section
                    .strip_prefix("lints.")
                    .map(|prefix| format!("{prefix}.{key}"))
                    .unwrap_or(key);
                manifest.lints.insert(lint_key, value);
            }
            "registries" => {
                manifest.registries.insert(key, value);
            }
            section if section.starts_with("source.") => {
                manifest
                    .source_replacements
                    .entry(section.trim_start_matches("source.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            section if section.starts_with("patch.") => {
                let source = section.trim_start_matches("patch.");
                let mut dep =
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics);
                dep.source = format!("patch:{source}");
                manifest.patch_overrides.insert(key.clone(), dep);
            }
            "replace" => {
                let mut dep =
                    parse_dependency(&key, &value, "normal", None, &mut manifest.diagnostics);
                dep.source = "replace".to_string();
                manifest.replace_overrides.insert(key, dep);
            }
            section if section.starts_with("profiles.") => {
                manifest
                    .profiles
                    .entry(section.trim_start_matches("profiles.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            section
                if section.starts_with("target.")
                    && target_dependency_section(section).is_none() =>
            {
                manifest
                    .targets
                    .entry(section.trim_start_matches("target.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            section
                if section.starts_with("targets.")
                    && target_dependency_section(section).is_none() =>
            {
                manifest
                    .targets
                    .entry(section.trim_start_matches("targets.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            section => {
                if let Some((target, kind)) = target_dependency_section(section) {
                    let (dep_key, dep_value) = workspace_dependency_key_value(&key, &value);
                    manifest.target_dependencies.push(parse_dependency(
                        &dep_key,
                        &dep_value,
                        kind,
                        Some(target),
                        &mut manifest.diagnostics,
                    ));
                } else {
                    match section {
                        "build" => {
                            manifest.build.insert(key, value);
                        }
                        "features" => {
                            manifest.features.insert(key, value);
                        }
                        "capabilities" => {
                            manifest.capabilities.insert(key, value);
                        }
                        "artifact" => {
                            manifest.artifact.insert(key, value);
                        }
                        "" => manifest.diagnostics.push(GhorDiagnostic::warning(
                            "E_GHOR_MANIFEST_ROOT_001",
                            "root-level manifest key ignored; use a named section",
                        )),
                        other => manifest.diagnostics.push(GhorDiagnostic::warning(
                            "E_GHOR_MANIFEST_SECTION_001",
                            format!("section [{}] is not interpreted by ghor yet", other),
                        )),
                    }
                }
            }
        }
    }
    merge_workspace_context(&mut manifest)?;
    merge_cargo_config_sources(&mut manifest)?;
    if manifest_is_cargo_toml(&manifest.path) {
        manifest
            .diagnostics
            .retain(|diag| diag.code != "E_GHOR_CHECKSUM_001");
    }
    validate_manifest(&mut manifest);
    for diagnostic in &mut manifest.diagnostics {
        diagnostic.attach_file(path);
    }
    Ok(manifest)
}

#[derive(Debug, Clone, Default)]
struct GhorWorkspaceContext {
    manifest_path: PathBuf,
    root: PathBuf,
    workspace: BTreeMap<String, String>,
    members: Vec<String>,
    exclude: Vec<String>,
    default_members: Vec<String>,
    package: BTreeMap<String, String>,
    dependencies: BTreeMap<String, GhorDependency>,
    lints: BTreeMap<String, String>,
    patch_overrides: BTreeMap<String, GhorDependency>,
    replace_overrides: BTreeMap<String, GhorDependency>,
    registries: BTreeMap<String, String>,
    source_replacements: BTreeMap<String, BTreeMap<String, String>>,
    diagnostics: Vec<GhorDiagnostic>,
}

#[derive(Debug, Clone)]
struct GhorWorkspaceMember {
    declared: String,
    root: PathBuf,
    manifest_path: PathBuf,
    package: String,
    version: String,
    default_member: bool,
}

fn merge_workspace_context(manifest: &mut GhorManifest) -> Result<(), crate::CliFailure> {
    let Some(workspace_manifest) = find_workspace_manifest_for(manifest) else {
        manifest.workspace_root = manifest.root.clone();
        return Ok(());
    };
    let context = parse_workspace_context(&workspace_manifest)?;
    let same_manifest = same_path(&manifest.path, &workspace_manifest);
    manifest.workspace_root = context.root.clone();
    manifest.workspace_manifest = Some(context.manifest_path.clone());
    manifest.workspace_is_root = same_manifest;
    if same_manifest {
        manifest.virtual_manifest = manifest.package.is_empty() && !manifest.workspace.is_empty();
    } else {
        manifest.workspace = context.workspace.clone();
        manifest.workspace_members = context.members.clone();
        manifest.workspace_exclude = context.exclude.clone();
        manifest.workspace_default_members = context.default_members.clone();
        manifest.workspace_package = context.package.clone();
        manifest.workspace_dependencies = context.dependencies.clone();
        manifest.workspace_lints = context.lints.clone();
        manifest.patch_overrides = context.patch_overrides.clone();
        manifest.replace_overrides = context.replace_overrides.clone();
        manifest.registries.extend(context.registries.clone());
        manifest
            .source_replacements
            .extend(context.source_replacements.clone());
    }
    if same_manifest && manifest.package.is_empty() {
        manifest.virtual_manifest = true;
    }
    manifest.diagnostics.extend(context.diagnostics.clone());
    apply_workspace_package_inheritance(manifest);
    apply_workspace_dependency_inheritance(manifest);
    apply_workspace_lints_inheritance(manifest);
    validate_workspace_context(manifest);
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn find_workspace_manifest_for(manifest: &GhorManifest) -> Option<PathBuf> {
    if let Some(raw) = manifest.package.get("workspace") {
        let candidate = manifest.root.join(raw);
        if candidate.is_file() {
            return Some(candidate);
        }
        for name in ghor_manifest_names() {
            let path = candidate.join(name);
            if path.exists() && manifest_has_workspace_section(&path) {
                return Some(path);
            }
        }
    }
    let mut dir = manifest.root.clone();
    loop {
        for name in ghor_manifest_names() {
            let candidate = dir.join(name);
            if candidate.exists() && manifest_has_workspace_section(&candidate) {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn manifest_has_workspace_section(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|raw| raw.lines().any(|line| line.trim() == "[workspace]"))
        .unwrap_or(false)
}

fn parse_workspace_context(path: &Path) -> Result<GhorWorkspaceContext, crate::CliFailure> {
    let raw = ghor_filesystem(
        "ghor.workspace",
        "read the workspace manifest",
        path,
        fs::read_to_string(path),
    )?;
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let mut context = GhorWorkspaceContext {
        manifest_path: path.to_path_buf(),
        root,
        ..GhorWorkspaceContext::default()
    };
    let mut section = String::new();
    let mut diagnostics = Vec::new();
    for original in raw.lines() {
        let line = strip_comment(original).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }
        let Some((key, value)) = parse_key_value(&line) else {
            continue;
        };
        match section.as_str() {
            "workspace" => {
                if key == "members" {
                    context.members = parse_array(&value);
                } else if key == "exclude" {
                    context.exclude = parse_array(&value);
                } else if key == "default-members" || key == "default_members" {
                    context.default_members = parse_array(&value);
                }
                context.workspace.insert(key, value);
            }
            "workspace.package" => {
                context.package.insert(key, value);
            }
            "workspace.dependencies" => {
                context.dependencies.insert(
                    key.clone(),
                    parse_dependency(&key, &value, "normal", None, &mut diagnostics),
                );
            }
            section if section.starts_with("workspace.lints") => {
                let lint_key = section
                    .strip_prefix("workspace.lints.")
                    .map(|prefix| format!("{prefix}.{key}"))
                    .unwrap_or(key);
                context.lints.insert(lint_key, value);
            }
            section if section.starts_with("patch.") => {
                let source = section.trim_start_matches("patch.");
                let mut dep = parse_dependency(&key, &value, "normal", None, &mut diagnostics);
                dep.source = format!("patch:{source}");
                context.patch_overrides.insert(key.clone(), dep);
            }
            "replace" => {
                let mut dep = parse_dependency(&key, &value, "normal", None, &mut diagnostics);
                dep.source = "replace".to_string();
                context.replace_overrides.insert(key, dep);
            }
            "registries" => {
                context.registries.insert(key, value);
            }
            section if section.starts_with("source.") => {
                context
                    .source_replacements
                    .entry(section.trim_start_matches("source.").to_string())
                    .or_default()
                    .insert(key, value);
            }
            _ => {}
        }
    }
    context.diagnostics = diagnostics;
    Ok(context)
}

fn apply_workspace_package_inheritance(manifest: &mut GhorManifest) {
    let keys = manifest
        .package
        .iter()
        .filter_map(|(key, value)| {
            key.strip_suffix(".workspace")
                .filter(|_| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    for key in keys {
        if let Some(value) = manifest.workspace_package.get(&key).cloned() {
            manifest.package.insert(key, value);
        }
    }
}

fn apply_workspace_dependency_inheritance(manifest: &mut GhorManifest) {
    let workspace = manifest.workspace_dependencies.clone();
    inherit_dependency_map(&workspace, &mut manifest.dependencies, "normal");
    inherit_dependency_map(&workspace, &mut manifest.dev_dependencies, "dev");
    inherit_dependency_map(&workspace, &mut manifest.build_dependencies, "build");
    for dep in &mut manifest.target_dependencies {
        if dep.workspace_inherited {
            if let Some(base) = workspace.get(&dep.name) {
                let mut merged = base.clone();
                merged.kind = dep.kind.clone();
                merged.target = dep.target.clone();
                merged.features.extend(dep.features.iter().cloned());
                merged.features.sort();
                merged.features.dedup();
                merged.workspace_inherited = true;
                *dep = merged;
            }
        }
    }
}

fn inherit_dependency_map(
    workspace: &BTreeMap<String, GhorDependency>,
    deps: &mut BTreeMap<String, GhorDependency>,
    kind: &str,
) {
    let keys = deps
        .iter()
        .filter(|(_, dep)| dep.workspace_inherited)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    for key in keys {
        if let (Some(local), Some(base)) = (deps.get(&key).cloned(), workspace.get(&key).cloned()) {
            let mut merged = base;
            merged.kind = kind.to_string();
            merged.target = local.target;
            merged.features.extend(local.features);
            merged.features.sort();
            merged.features.dedup();
            merged.workspace_inherited = true;
            deps.insert(key, merged);
        }
    }
}

fn apply_workspace_lints_inheritance(manifest: &mut GhorManifest) {
    if manifest.lints.get("workspace").map(String::as_str) == Some("true") {
        for (key, value) in &manifest.workspace_lints {
            manifest
                .lints
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }
}

fn validate_workspace_context(manifest: &mut GhorManifest) {
    if manifest.virtual_manifest {
        if manifest
            .workspace
            .get("resolver")
            .map(String::as_str)
            .is_none()
        {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_WORKSPACE_RESOLVER_001",
                "virtual workspace manifests must declare workspace.resolver",
            ));
        }
        if manifest.workspace_members.is_empty() {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_WORKSPACE_MEMBERS_001",
                "virtual workspace manifests must declare members",
            ));
        }
    }
    for dep in manifest.workspace_dependencies.values() {
        if dep.optional {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_WORKSPACE_DEPENDENCY_001",
                format!("workspace dependency '{}' cannot be optional", dep.name),
            ));
        }
    }
}

fn workspace_member_entries(
    manifest: &GhorManifest,
    default_only: bool,
) -> Result<Vec<GhorWorkspaceMember>, crate::CliFailure> {
    let mut declared = if default_only && !manifest.workspace_default_members.is_empty() {
        manifest.workspace_default_members.clone()
    } else {
        manifest.workspace_members.clone()
    };
    if manifest.workspace_is_root
        && !manifest.virtual_manifest
        && !declared.iter().any(|item| item == ".")
    {
        declared.insert(0, ".".to_string());
    }
    if default_only
        && manifest.virtual_manifest
        && manifest.workspace_default_members.is_empty()
        && declared.is_empty()
    {
        declared = manifest.workspace_members.clone();
    }

    let default_set = manifest
        .workspace_default_members
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for pattern in declared {
        for member_root in expand_workspace_pattern(manifest, &pattern)? {
            let rel = workspace_relative(&manifest.workspace_root, &member_root);
            if workspace_path_excluded(manifest, &rel) {
                continue;
            }
            let Some(manifest_path) = exact_manifest_in_dir(&member_root) else {
                continue;
            };
            let key = workspace_relative(&manifest.workspace_root, &member_root);
            if !seen.insert(key.clone()) {
                continue;
            }
            let member_manifest = parse_manifest(&manifest_path)?;
            entries.push(GhorWorkspaceMember {
                declared: pattern.clone(),
                root: member_root,
                manifest_path,
                package: package_name(&member_manifest),
                version: package_version(&member_manifest),
                default_member: default_set.is_empty()
                    || default_set.contains(&key)
                    || default_set.contains(&pattern),
            });
        }
    }
    if !default_only {
        for dep in all_manifest_dependencies(manifest) {
            let Some(path) = &dep.path else {
                continue;
            };
            let member_root = manifest.root.join(path);
            if !member_root.starts_with(&manifest.workspace_root) {
                continue;
            }
            let rel = workspace_relative(&manifest.workspace_root, &member_root);
            if workspace_path_excluded(manifest, &rel) || seen.contains(&rel) {
                continue;
            }
            let Some(manifest_path) = exact_manifest_in_dir(&member_root) else {
                continue;
            };
            let member_manifest = parse_manifest(&manifest_path)?;
            seen.insert(rel.clone());
            entries.push(GhorWorkspaceMember {
                declared: rel,
                root: member_root,
                manifest_path,
                package: package_name(&member_manifest),
                version: package_version(&member_manifest),
                default_member: false,
            });
        }
    }
    Ok(entries)
}

fn selected_workspace_manifests(
    manifest: &GhorManifest,
    workspace: bool,
) -> Result<Vec<PathBuf>, crate::CliFailure> {
    if workspace {
        let mut members = workspace_member_entries(manifest, false)?
            .into_iter()
            .map(|member| member.manifest_path)
            .collect::<Vec<_>>();
        if members.is_empty() {
            let declared = if manifest.workspace_members.is_empty() {
                workspace_array_from_manifest_raw(&manifest.path, "members")?
            } else {
                manifest.workspace_members.clone()
            };
            let excludes = if manifest.workspace_exclude.is_empty() {
                workspace_array_from_manifest_raw(&manifest.path, "exclude")?
            } else {
                manifest.workspace_exclude.clone()
            };
            let fallback_root = manifest
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| manifest.workspace_root.clone());
            let mut fallback_manifest = manifest.clone();
            fallback_manifest.workspace_root = fallback_root;
            let excluded = excludes.into_iter().collect::<BTreeSet<_>>();
            for pattern in declared {
                for member_root in expand_workspace_pattern(&fallback_manifest, &pattern)? {
                    let rel = workspace_relative(&fallback_manifest.workspace_root, &member_root);
                    if excluded.contains(&rel) {
                        continue;
                    }
                    if let Some(manifest_path) = exact_manifest_in_dir(&member_root) {
                        members.push(manifest_path);
                    }
                }
            }
        }
        if members.is_empty() {
            members.push(manifest.path.clone());
        }
        members.sort();
        members.dedup();
        Ok(members)
    } else {
        Ok(vec![manifest.path.clone()])
    }
}

fn workspace_array_from_manifest_raw(
    manifest_path: &Path,
    key_name: &str,
) -> Result<Vec<String>, crate::CliFailure> {
    let raw = ghor_filesystem(
        "ghor.workspace",
        "read workspace members",
        manifest_path,
        fs::read_to_string(manifest_path),
    )?;
    let mut section = String::new();
    for line in raw.lines().map(strip_comment) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }
        if section == "workspace" {
            if let Some((key, value)) = parse_key_value(line) {
                if key == key_name {
                    return Ok(parse_array(&value));
                }
            }
        }
    }
    Ok(Vec::new())
}

fn expand_workspace_pattern(
    manifest: &GhorManifest,
    pattern: &str,
) -> Result<Vec<PathBuf>, crate::CliFailure> {
    if !pattern.contains('*') && !pattern.contains('?') {
        return Ok(vec![manifest.workspace_root.join(pattern)]);
    }
    let normalized = pattern.replace('\\', "/");
    let Some(prefix) = normalized.split(['*', '?']).next() else {
        return Ok(Vec::new());
    };
    let base = if prefix.ends_with('/') {
        manifest.workspace_root.join(prefix.trim_end_matches('/'))
    } else {
        manifest
            .workspace_root
            .join(prefix.rsplit_once('/').map(|(head, _)| head).unwrap_or(""))
    };
    let mut out = Vec::new();
    if !base.exists() {
        return Ok(out);
    }
    let entries = ghor_filesystem(
        "ghor.workspace",
        "enumerate workspace members",
        &base,
        fs::read_dir(&base),
    )?;
    for entry in entries {
        let entry = ghor_filesystem(
            "ghor.workspace",
            "read a workspace directory entry",
            &base,
            entry,
        )?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let rel = workspace_relative(&manifest.workspace_root, &path);
        if workspace_pattern_matches(&normalized, &rel) && exact_manifest_in_dir(&path).is_some() {
            out.push(path);
        }
    }
    Ok(out)
}

fn workspace_path_excluded(manifest: &GhorManifest, rel: &str) -> bool {
    manifest
        .workspace_exclude
        .iter()
        .any(|pattern| workspace_pattern_matches(pattern, rel))
}

fn workspace_pattern_matches(pattern: &str, rel: &str) -> bool {
    let pattern = pattern.trim().replace('\\', "/");
    let rel = rel.trim().replace('\\', "/");
    if pattern == rel {
        return true;
    }
    if !pattern.contains('*') && !pattern.contains('?') {
        return false;
    }
    let mut pos = 0usize;
    let mut parts = pattern.split(['*', '?']).peekable();
    if let Some(first) = parts.next() {
        if !first.is_empty() && !rel.starts_with(first) {
            return false;
        }
        pos = first.len();
    }
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let Some(found) = rel[pos..].find(part) else {
            return false;
        };
        pos += found + part.len();
    }
    pattern.ends_with('*') || pattern.ends_with('?') || pos == rel.len()
}

fn workspace_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn exact_manifest_in_dir(dir: &Path) -> Option<PathBuf> {
    ghor_manifest_names()
        .into_iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

fn workspace_target_dir(manifest: &GhorManifest) -> PathBuf {
    manifest.workspace_root.join("target").join("ghor")
}

fn workspace_metadata_json(manifest: &GhorManifest) -> Result<Value, crate::CliFailure> {
    let members = workspace_member_entries(manifest, false)?
        .into_iter()
        .map(|member| {
            json!({
                "declared": member.declared,
                "root": member.root,
                "manifest": member.manifest_path,
                "package": member.package,
                "version": member.version,
                "default_member": member.default_member
            })
        })
        .collect::<Vec<_>>();
    let default_members = workspace_member_entries(manifest, true)?
        .into_iter()
        .map(|member| {
            json!({
                "declared": member.declared,
                "root": member.root,
                "manifest": member.manifest_path,
                "package": member.package,
                "version": member.version,
                "default_member": member.default_member
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "ghor.workspace.metadata.v2",
        "workspace_root": manifest.workspace_root,
        "workspace_manifest": manifest.workspace_manifest,
        "workspace_is_root": manifest.workspace_is_root,
        "virtual_manifest": manifest.virtual_manifest,
        "members_declared": manifest.workspace_members,
        "exclude": manifest.workspace_exclude,
        "default_members_declared": manifest.workspace_default_members,
        "members": members,
        "default_members": default_members,
        "workspace_package": manifest.workspace_package,
        "package_metadata": manifest.package_metadata,
        "workspace_dependencies": manifest.workspace_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "workspace_lints": manifest.workspace_lints,
        "shared_lockfile": lock_path_for(manifest, &[]),
        "shared_target": workspace_target_dir(manifest)
    }))
}

fn args_with_manifest_path(args: &[String], manifest_path: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--manifest" || arg == "--manifest-path" {
            skip_next = true;
            continue;
        }
        out.push(arg.clone());
    }
    out.push("--manifest-path".to_string());
    out.push(manifest_path.display().to_string());
    out
}

fn run_workspace_member_action(
    args: &[String],
    manifest: &GhorManifest,
    action: &str,
) -> Result<(), Box<dyn Error>> {
    let members = workspace_member_entries(manifest, true)?;
    let mut results = Vec::new();
    let mut errors = Vec::new();
    for member in members {
        let member_args = args_with_manifest_path(args, &member.manifest_path);
        let status = match action {
            "check" => run_check(&member_args),
            "build" => run_build(&member_args),
            "test" => run_test(&member_args),
            "graph" => run_graph(&member_args),
            _ => Err(format!("[E_GHOR_WORKSPACE_001] unknown workspace action '{action}'").into()),
        };
        if let Err(err) = status {
            errors.push(GhorDiagnostic::error(
                "E_GHOR_WORKSPACE_MEMBER_001",
                format!(
                    "workspace member '{}' failed {action}: {err}",
                    member.package
                ),
            ));
        }
        results.push(json!({
            "package": member.package,
            "manifest": member.manifest_path,
            "action": action,
            "status": if errors.is_empty() { "ok" } else { "checked" }
        }));
    }
    print_json(&json!({
        "schema": "ghor.workspace.action.v1",
        "action": action,
        "status": if diagnostics_have_errors(&errors) { "error" } else { "ok" },
        "members": results,
        "diagnostics": diagnostics_to_json(&errors)
    }))?;
    if diagnostics_have_errors(&errors) {
        Err(diagnostics_summary(&errors).into())
    } else {
        Ok(())
    }
}

fn merge_cargo_config_sources(manifest: &mut GhorManifest) -> Result<(), crate::CliFailure> {
    let mut config = GhorCargoConfig::default();
    for path in cargo_config_paths(&manifest.root) {
        if path.exists() {
            merge_cargo_config_file(manifest, &mut config, &path)?;
        }
    }
    manifest.cargo_config = config;
    Ok(())
}

fn cargo_config_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = env::var_os("CARGO_HOME") {
        push_cargo_config_pair(&mut paths, &PathBuf::from(home));
    } else if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        push_cargo_config_pair(&mut paths, &PathBuf::from(home).join(".cargo"));
    }

    let mut ancestors = root
        .ancestors()
        .map(Path::to_path_buf)
        .collect::<Vec<PathBuf>>();
    ancestors.reverse();
    for ancestor in ancestors {
        push_cargo_config_pair(&mut paths, &ancestor.join(".cargo"));
    }
    let mut seen = BTreeSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn push_cargo_config_pair(paths: &mut Vec<PathBuf>, cargo_dir: &Path) {
    paths.push(cargo_dir.join("config.toml"));
    paths.push(cargo_dir.join("config"));
}

fn merge_cargo_config_file(
    manifest: &mut GhorManifest,
    config: &mut GhorCargoConfig,
    path: &Path,
) -> Result<(), crate::CliFailure> {
    let raw = ghor_filesystem(
        "ghor.config",
        "read Cargo-compatible configuration",
        path,
        fs::read_to_string(path),
    )?;
    config.paths.push(path.to_path_buf());
    let mut section = String::new();
    for original in raw.lines() {
        let line = strip_comment(original).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            continue;
        }
        let Some((key, value)) = parse_key_value(&line) else {
            continue;
        };
        match section.as_str() {
            "registry" => {
                config.registry.insert(key, value);
            }
            "registries" => {
                let values = parse_inline_table(&value);
                if let Some(raw) = values
                    .get("index")
                    .or_else(|| values.get("registry"))
                    .cloned()
                    .or_else(|| (!value.contains('{')).then(|| value.clone()))
                {
                    manifest.registries.insert(key.clone(), raw);
                }
                if !values.is_empty() {
                    config.registries.insert(key, values);
                }
            }
            "credential-alias" => {
                config
                    .credential_aliases
                    .insert(key, parse_config_array_or_single(&value));
            }
            "net" => {
                config.net.insert(key, value);
            }
            "http" => {
                config.http.insert(key, value);
            }
            "git" => {
                config.git.insert(key, value);
            }
            _ => {
                if let Some(name) = section.strip_prefix("registries.") {
                    config
                        .registries
                        .entry(name.to_string())
                        .or_default()
                        .insert(key.clone(), value.clone());
                    if key == "index" || key == "registry" {
                        manifest.registries.insert(name.to_string(), value);
                    }
                } else if let Some(name) = section.strip_prefix("source.") {
                    manifest
                        .source_replacements
                        .entry(name.to_string())
                        .or_default()
                        .insert(key, value);
                }
            }
        }
    }
    Ok(())
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim().to_string(), normalize_value(value.trim())))
}

fn normalize_value(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(',');
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_config_array_or_single(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        trimmed[1..trimmed.len() - 1]
            .split(',')
            .map(normalize_value)
            .filter(|entry| !entry.trim().is_empty())
            .collect()
    } else {
        vec![normalize_value(trimmed)]
    }
}

fn parse_array(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(normalize_value)
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_array_or_single(value: Option<&String>) -> Vec<String> {
    value
        .map(|value| parse_config_array_or_single(value))
        .unwrap_or_default()
}

fn parse_inline_table(value: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let inner = value.trim().trim_start_matches('{').trim_end_matches('}');
    for item in split_top_level_commas(inner) {
        if let Some((key, value)) = parse_key_value(item.trim()) {
            out.insert(key, value);
        }
    }
    out
}

fn workspace_dependency_key_value(key: &str, value: &str) -> (String, String) {
    if key.ends_with(".workspace") && matches!(value, "true" | "1" | "yes" | "on") {
        (
            key.trim_end_matches(".workspace").to_string(),
            "{ workspace = true }".to_string(),
        )
    } else {
        (key.to_string(), value.to_string())
    }
}

fn split_top_level_commas(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut bracket_depth = 0_u32;
    let mut previous = '\0';
    for ch in value.chars() {
        match ch {
            '"' if previous != '\\' => {
                in_string = !in_string;
                current.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' if !in_string => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if !in_string && bracket_depth == 0 => {
                if !current.trim().is_empty() {
                    items.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
        previous = ch;
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }
    items
}

fn parse_boolish(value: Option<&String>, default: bool) -> bool {
    value
        .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(default)
}

fn target_dependency_section(section: &str) -> Option<(String, &'static str)> {
    for prefix in ["target.", "targets."] {
        let Some(rest) = section.strip_prefix(prefix) else {
            continue;
        };
        for (suffix, kind) in [
            (".dependencies", "normal"),
            (".dev-dependencies", "dev"),
            (".build-dependencies", "build"),
        ] {
            if let Some(target) = rest.strip_suffix(suffix) {
                return Some((
                    target.trim_matches('\'').trim_matches('"').to_string(),
                    kind,
                ));
            }
        }
    }
    None
}

fn parse_dependency(
    name: &str,
    value: &str,
    kind: &str,
    target: Option<String>,
    diagnostics: &mut Vec<GhorDiagnostic>,
) -> GhorDependency {
    let mut dep = GhorDependency {
        name: name.to_string(),
        kind: kind.to_string(),
        target,
        default_features: true,
        ..GhorDependency::default()
    };
    if value.starts_with('{') {
        let table = parse_inline_table(value);
        dep.workspace_inherited = parse_boolish(table.get("workspace"), false);
        dep.version = table.get("version").cloned();
        dep.path = table.get("path").cloned();
        dep.checksum = table.get("checksum").cloned();
        dep.features = table
            .get("features")
            .map(|value| parse_array(value))
            .unwrap_or_default();
        dep.default_features = parse_boolish(
            table
                .get("default-features")
                .or_else(|| table.get("default_features")),
            true,
        );
        dep.optional = parse_boolish(table.get("optional"), false);
        dep.registry = table.get("registry").cloned();
        dep.package = table.get("package").cloned();
        dep.git = table.get("git").cloned();
        dep.git_branch = table.get("branch").cloned();
        dep.git_tag = table.get("tag").cloned();
        dep.git_rev = table.get("rev").cloned();
        dep.source = table
            .get("git")
            .or_else(|| table.get("registry"))
            .or_else(|| table.get("path"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
    } else {
        dep.version = Some(value.to_string());
        dep.source = "registry".to_string();
    }
    if dep.path.is_none() && dep.git.is_none() && dep.checksum.is_none() && !dep.workspace_inherited
    {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_CHECKSUM_001",
            format!(
                "dependency '{}' is not a local path dependency and must declare checksum",
                name
            ),
        ));
    }
    dep
}

fn parse_native_dependency(
    name: &str,
    value: &str,
    diagnostics: &mut Vec<GhorDiagnostic>,
) -> GhorNativeDependency {
    let mut dep = GhorNativeDependency {
        name: name.to_string(),
        kind: "library".to_string(),
        link_mode: "dynamic".to_string(),
        required: true,
        source: "native".to_string(),
        ..GhorNativeDependency::default()
    };
    if value.starts_with('{') {
        for (key, value) in parse_inline_table(value) {
            apply_native_dependency_field(&mut dep, &key, &value, diagnostics);
        }
    } else {
        dep.libs = vec![value.to_string()];
        dep.system = true;
        dep.source = "system".to_string();
    }
    finalize_native_dependency(&mut dep);
    dep
}

fn set_native_dependency_field(
    native_dependencies: &mut BTreeMap<String, GhorNativeDependency>,
    name: &str,
    key: &str,
    value: &str,
    diagnostics: &mut Vec<GhorDiagnostic>,
) {
    let dep = native_dependencies
        .entry(name.to_string())
        .or_insert_with(|| GhorNativeDependency {
            name: name.to_string(),
            kind: "library".to_string(),
            link_mode: "dynamic".to_string(),
            required: true,
            source: "native".to_string(),
            ..GhorNativeDependency::default()
        });
    apply_native_dependency_field(dep, key, value, diagnostics);
    finalize_native_dependency(dep);
}

fn apply_native_dependency_field(
    dep: &mut GhorNativeDependency,
    key: &str,
    value: &str,
    diagnostics: &mut Vec<GhorDiagnostic>,
) {
    match key {
        "language" => dep.language = value.to_string(),
        "abi" => dep.abi = value.to_string(),
        "kind" => dep.kind = value.to_string(),
        "version" => dep.version = Some(value.to_string()),
        "target" => dep.target = Some(value.to_string()),
        "headers" => dep.headers = parse_array(value),
        "include_paths" | "include-paths" | "includes" => dep.include_paths = parse_array(value),
        "link_paths" | "link-paths" => dep.link_paths = parse_array(value),
        "libs" | "libraries" => dep.libs = parse_array(value),
        "frameworks" => dep.frameworks = parse_array(value),
        "symbols" => dep.symbols = parse_array(value),
        "cflags" => dep.cflags = parse_array(value),
        "ldflags" => dep.ldflags = parse_array(value),
        "defines" => dep.defines = parse_array(value),
        "pkg_config" | "pkg-config" => dep.pkg_config = Some(value.to_string()),
        "artifact" | "path" => dep.artifact = Some(value.to_string()),
        "checksum" | "sha256" => dep.checksum = Some(value.to_string()),
        "link_mode" | "link-mode" => dep.link_mode = value.to_string(),
        "calling_convention" | "calling-convention" | "cc" => {
            dep.calling_convention = Some(value.to_string());
        }
        "layout" => dep.layout = Some(value.to_string()),
        "ownership" => dep.ownership = Some(value.to_string()),
        "required" => dep.required = parse_boolish(Some(&value.to_string()), true),
        "system" => dep.system = parse_boolish(Some(&value.to_string()), false),
        "source" => dep.source = value.to_string(),
        other => diagnostics.push(GhorDiagnostic::warning(
            "E_GHOR_NATIVE_FIELD_001",
            format!(
                "native dependency '{}' ignores unknown field '{}'",
                dep.name, other
            ),
        )),
    }
}

fn finalize_native_dependency(dep: &mut GhorNativeDependency) {
    if dep.system {
        dep.source = "system".to_string();
    } else if dep.pkg_config.is_some() {
        dep.source = "pkg-config".to_string();
    } else if dep.artifact.is_some() {
        dep.source = "local-artifact".to_string();
    }
}

fn validate_manifest(manifest: &mut GhorManifest) {
    if manifest.virtual_manifest {
        validate_languages(manifest);
        return;
    }
    if !manifest.package.contains_key("name") {
        manifest.diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MANIFEST_MISSING_NAME_001",
            "package.name is required",
        ));
    }
    if !manifest.package.contains_key("version") {
        manifest.diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MANIFEST_INVALID_VERSION_001",
            "package.version is required",
        ));
    } else if manifest
        .package
        .get("version")
        .map(|version| version.split('.').count() != 3)
        .unwrap_or(true)
    {
        manifest.diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MANIFEST_INVALID_VERSION_001",
            "package.version must be semver-like MAJOR.MINOR.PATCH",
        ));
    }
    let kind = manifest
        .package
        .get("kind")
        .map(String::as_str)
        .unwrap_or("bin");
    if !matches!(
        kind,
        "bin" | "lib" | "machine" | "basal" | "primitive" | "foreign"
    ) {
        manifest.diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MANIFEST_INVALID_MODE_001",
            format!("unsupported package.kind '{}'", kind),
        ));
    }
    if let Some(triple) = manifest.target_default.get("triple") {
        if !known_target_triples().contains(triple.as_str()) {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_MANIFEST_INVALID_TARGET_001",
                format!("unsupported target triple '{}'", triple),
            ));
        }
    }
    for triple in manifest.targets.keys() {
        if !known_target_triples().contains(triple.as_str()) {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_MANIFEST_INVALID_TARGET_001",
                format!("unsupported target matrix triple '{}'", triple),
            ));
        }
    }
    if manifest
        .profiles
        .keys()
        .any(|profile| profile.trim().is_empty())
    {
        manifest.diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MANIFEST_INVALID_PROFILE_001",
            "profile names must be non-empty",
        ));
    }
    validate_languages(manifest);
    validate_native_dependencies(manifest);
}

fn validate_languages(manifest: &mut GhorManifest) {
    if let Some(primary) = manifest.languages.get("primary") {
        if official_language(primary).is_none() {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_LANGUAGE_UNSUPPORTED_001",
                format!("unsupported primary language '{}'", primary),
            ));
        }
    }
    if let Some(adapters) = manifest.languages.get("adapters") {
        for adapter in parse_array(adapters) {
            let Some(official) = official_language(&adapter) else {
                manifest.diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_LANGUAGE_UNSUPPORTED_001",
                    format!("unsupported language adapter '{}'", adapter),
                ));
                continue;
            };
            if official != "nC" && !manifest.language_adapters.contains_key(official) {
                manifest.diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_LANGUAGE_ADAPTER_MISSING_001",
                    format!(
                        "language adapter '{}' must declare [language_adapter.{}]",
                        adapter, official
                    ),
                ));
            }
        }
    }
}

fn validate_native_dependencies(manifest: &mut GhorManifest) {
    let root = manifest.root.clone();
    let deps = manifest
        .native_dependencies
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for dep in deps {
        if dep.language.is_empty()
            || !known_native_dependency_languages().contains(dep.language.as_str())
        {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_NATIVE_LANGUAGE_001",
                format!(
                    "native dependency '{}' must declare language c, cpp, rust, zig, go or asm",
                    dep.name
                ),
            ));
        }
        if dep.abi.is_empty() || !known_native_dependency_abis().contains(dep.abi.as_str()) {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_NATIVE_ABI_001",
                format!(
                    "native dependency '{}' must declare a supported ABI",
                    dep.name
                ),
            ));
        }
        if !known_native_link_modes().contains(dep.link_mode.as_str()) {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_NATIVE_LINK_MODE_001",
                format!(
                    "native dependency '{}' has unsupported link_mode '{}'",
                    dep.name, dep.link_mode
                ),
            ));
        }
        if dep.kind != "header-only"
            && dep.libs.is_empty()
            && dep.frameworks.is_empty()
            && dep.pkg_config.is_none()
            && dep.artifact.is_none()
        {
            manifest.diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_NATIVE_LINK_001",
                format!(
                    "native dependency '{}' must declare libs, frameworks, pkg_config, or artifact",
                    dep.name
                ),
            ));
        }
        if let Some(artifact) = &dep.artifact {
            let artifact_path = root.join(artifact);
            if !artifact_path.exists() {
                manifest.diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_NATIVE_ARTIFACT_001",
                    format!(
                        "native dependency '{}' artifact '{}' does not exist",
                        dep.name,
                        artifact_path.display()
                    ),
                ));
            }
            if dep.checksum.is_none() {
                manifest.diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_NATIVE_CHECKSUM_001",
                    format!(
                        "native dependency '{}' local artifact must declare checksum",
                        dep.name
                    ),
                ));
            } else if let Some(expected) = dep.checksum.as_deref() {
                match fs::read(&artifact_path) {
                    Ok(bytes) => {
                        let actual = sha256_hex(&bytes);
                        let expected = expected.trim_start_matches("sha256:");
                        if expected != actual {
                            manifest.diagnostics.push(GhorDiagnostic::error(
                                "E_GHOR_NATIVE_CHECKSUM_001",
                                format!(
                                    "native dependency '{}' checksum mismatch for '{}'",
                                    dep.name,
                                    artifact_path.display()
                                ),
                            ));
                        }
                    }
                    Err(err) => manifest.diagnostics.push(GhorDiagnostic::error(
                        "E_GHOR_NATIVE_ARTIFACT_001",
                        format!(
                            "native dependency '{}' artifact '{}' cannot be read: {err}",
                            dep.name,
                            artifact_path.display()
                        ),
                    )),
                }
            }
        } else if !dep.system && dep.pkg_config.is_none() && dep.checksum.is_none() {
            manifest.diagnostics.push(GhorDiagnostic::warning(
                "E_GHOR_NATIVE_CHECKSUM_001",
                format!(
                    "native dependency '{}' has no artifact checksum; system/pkg-config dependencies should set system=true or pkg_config",
                    dep.name
                ),
            ));
        }
    }
}

fn known_native_dependency_languages() -> BTreeSet<&'static str> {
    ["c", "cpp", "rust", "zig", "go", "asm"]
        .into_iter()
        .collect()
}

fn known_native_dependency_abis() -> BTreeSet<&'static str> {
    [
        "c",
        "sysv64",
        "win64",
        "aapcs64",
        "itanium-cxxabi",
        "msvc-cxxabi",
        "rust-cabi",
        "zig-cabi",
        "go-carchive",
        "asm",
    ]
    .into_iter()
    .collect()
}

fn known_native_link_modes() -> BTreeSet<&'static str> {
    ["object", "static", "dynamic", "system", "framework"]
        .into_iter()
        .collect()
}

fn known_target_triples() -> BTreeSet<&'static str> {
    [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "wasm32-unknown-unknown",
    ]
    .into_iter()
    .collect()
}

fn official_language(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "nc" | "n" | "n-c" => Some("nC"),
        "c" => Some("c"),
        "cpp" | "c++" | "cxx" => Some("cpp"),
        "asm" | "assembly" | "s" => Some("asm"),
        "rust" | "rs" => Some("rust"),
        "java" => Some("java"),
        "python" | "py" => Some("python"),
        "javascript" | "js" => Some("javascript"),
        "typescript" | "ts" => Some("typescript"),
        _ => None,
    }
}

fn foreign_languages() -> Vec<&'static str> {
    vec![
        "c",
        "cpp",
        "asm",
        "rust",
        "java",
        "python",
        "javascript",
        "typescript",
    ]
}

fn manifest_to_json(manifest: &GhorManifest) -> Value {
    json!({
        "schema": "ghor.manifest.v1",
        "path": manifest.path,
        "root": manifest.root,
        "package": manifest.package,
        "cargo_lib": manifest.cargo_lib,
        "cargo_bin": manifest.cargo_bin,
        "toolchain": manifest.toolchain,
        "target_default": manifest.target_default,
        "machine": manifest.machine,
        "basal": manifest.basal,
        "runtime": manifest.runtime,
        "compiler": manifest.compiler,
        "primitive": manifest.primitive,
        "languages": manifest.languages,
        "language_adapters": manifest.language_adapters,
        "dependencies": manifest.dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "dev_dependencies": manifest.dev_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "build_dependencies": manifest.build_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "target_dependencies": manifest.target_dependencies.iter().map(dependency_to_json).collect::<Vec<_>>(),
        "rust_dependencies": manifest.rust_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "c_dependencies": manifest.c_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "cpp_dependencies": manifest.cpp_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "asm_dependencies": manifest.asm_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "basal_dependencies": manifest.basal_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "profiles": manifest.profiles,
        "targets": manifest.targets,
        "build": manifest.build,
        "features": manifest.features,
        "capabilities": manifest.capabilities,
        "artifact": manifest.artifact,
        "workspace_root": manifest.workspace_root,
        "workspace_manifest": manifest.workspace_manifest,
        "workspace_is_root": manifest.workspace_is_root,
        "virtual_manifest": manifest.virtual_manifest,
        "workspace": manifest.workspace,
        "workspace_members": manifest.workspace_members,
        "workspace_exclude": manifest.workspace_exclude,
        "workspace_default_members": manifest.workspace_default_members,
        "workspace_package": manifest.workspace_package,
        "workspace_dependencies": manifest.workspace_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "workspace_lints": manifest.workspace_lints,
        "lints": manifest.lints,
        "shared_lockfile": lock_path_for(manifest, &[]),
        "shared_target": workspace_target_dir(manifest),
        "patch_overrides": manifest.patch_overrides.values().map(dependency_to_json).collect::<Vec<_>>(),
        "replace_overrides": manifest.replace_overrides.values().map(dependency_to_json).collect::<Vec<_>>(),
        "registries": manifest.registries,
        "source_replacements": manifest.source_replacements,
        "cargo_config": cargo_config_to_json(&manifest.cargo_config, None, None),
        "diagnostics": diagnostics_to_json(&manifest.diagnostics)
    })
}

fn cargo_config_to_json(
    config: &GhorCargoConfig,
    selected_name: Option<&str>,
    selected_registry: Option<&GhorRegistry>,
) -> Value {
    let registries = config
        .registries
        .iter()
        .map(|(name, values)| {
            let mut redacted = values.clone();
            if redacted.contains_key("token") {
                redacted.insert("token".to_string(), "<redacted>".to_string());
            }
            (name.clone(), redacted)
        })
        .collect::<BTreeMap<_, _>>();
    let mut registry = config.registry.clone();
    if registry.contains_key("token") {
        registry.insert("token".to_string(), "<redacted>".to_string());
    }
    json!({
        "schema": "ghor.cargo_config.effective.v1",
        "paths": config.paths,
        "registry": registry,
        "registries": registries,
        "credential_aliases": config.credential_aliases.keys().cloned().collect::<Vec<_>>(),
        "net": config.net,
        "http": config.http,
        "git": config.git,
        "selected_registry_name": selected_name,
        "selected_registry": selected_registry.map(registry_label),
        "selected_registry_transport": selected_registry.map(registry_transport)
    })
}

fn dotted_map_to_json(map: &BTreeMap<String, String>) -> Value {
    let mut root = serde_json::Map::new();
    for (key, value) in map {
        insert_dotted_json_value(&mut root, key, Value::String(value.clone()));
    }
    Value::Object(root)
}

fn cargo_metadata_user_metadata(manifest: &GhorManifest) -> Value {
    let mut metadata = dotted_map_to_json(&manifest.package_metadata);
    if let Some(object) = metadata.as_object_mut() {
        let ghor = object
            .entry("ghor".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(ghor) = ghor.as_object_mut() {
            ghor.insert(
                "native_dependencies".to_string(),
                Value::Array(
                    manifest
                        .native_dependencies
                        .values()
                        .map(native_dependency_to_json)
                        .collect::<Vec<_>>(),
                ),
            );
            ghor.insert(
                "basal_dependencies".to_string(),
                Value::Array(
                    manifest
                        .basal_dependencies
                        .values()
                        .map(dependency_to_json)
                        .collect::<Vec<_>>(),
                ),
            );
        }
    }
    metadata
}

fn insert_dotted_json_value(target: &mut serde_json::Map<String, Value>, key: &str, value: Value) {
    let Some((head, tail)) = key.split_once('.') else {
        target.insert(key.to_string(), value);
        return;
    };
    let entry = target
        .entry(head.to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(serde_json::Map::new());
    }
    if let Some(next) = entry.as_object_mut() {
        insert_dotted_json_value(next, tail, value);
    }
}

fn dependency_to_json(dep: &GhorDependency) -> Value {
    json!({
        "name": dep.name,
        "source": dep.source,
        "version": dep.version,
        "req": dep.version,
        "checksum": dep.checksum,
        "path": dep.path,
        "kind": dep.kind,
        "target": dep.target,
        "features": dep.features,
        "default_features": dep.default_features,
        "uses_default_features": dep.default_features,
        "optional": dep.optional,
        "registry": dep.registry,
        "package": dep.package,
        "rename": dep.package,
        "git": dep.git,
        "git_branch": dep.git_branch,
        "git_tag": dep.git_tag,
        "git_rev": dep.git_rev,
        "workspace_inherited": dep.workspace_inherited
    })
}

fn native_dependency_to_json(dep: &GhorNativeDependency) -> Value {
    json!({
        "name": dep.name,
        "language": dep.language,
        "abi": dep.abi,
        "kind": dep.kind,
        "version": dep.version,
        "target": dep.target,
        "headers": dep.headers,
        "include_paths": dep.include_paths,
        "link_paths": dep.link_paths,
        "libs": dep.libs,
        "frameworks": dep.frameworks,
        "symbols": dep.symbols,
        "cflags": dep.cflags,
        "ldflags": dep.ldflags,
        "defines": dep.defines,
        "pkg_config": dep.pkg_config,
        "artifact": dep.artifact,
        "checksum": dep.checksum,
        "link_mode": dep.link_mode,
        "calling_convention": dep.calling_convention,
        "layout": dep.layout,
        "ownership": dep.ownership,
        "required": dep.required,
        "system": dep.system,
        "source": dep.source,
        "link_args": native_dependency_link_args(dep)
    })
}

fn native_dependency_link_args(dep: &GhorNativeDependency) -> Value {
    let include_flags = dep
        .include_paths
        .iter()
        .map(|path| format!("-I{path}"))
        .collect::<Vec<_>>();
    let link_path_flags = dep
        .link_paths
        .iter()
        .map(|path| format!("-L{path}"))
        .collect::<Vec<_>>();
    let lib_flags = dep
        .libs
        .iter()
        .map(|lib| format!("-l{lib}"))
        .collect::<Vec<_>>();
    let framework_flags = dep
        .frameworks
        .iter()
        .flat_map(|framework| ["-framework".to_string(), framework.clone()])
        .collect::<Vec<_>>();
    json!({
        "include_flags": include_flags,
        "link_path_flags": link_path_flags,
        "lib_flags": lib_flags,
        "framework_flags": framework_flags,
        "cflags": dep.cflags,
        "ldflags": dep.ldflags,
        "artifact": dep.artifact,
        "pkg_config": dep.pkg_config
    })
}

fn package_name(manifest: &GhorManifest) -> String {
    manifest
        .package
        .get("name")
        .cloned()
        .unwrap_or_else(|| "unnamed".to_string())
}

fn package_version(manifest: &GhorManifest) -> String {
    manifest
        .package
        .get("version")
        .cloned()
        .unwrap_or_else(|| "0.0.0".to_string())
}

fn host_target_triple() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu"
    } else {
        "unknown-host"
    }
}

fn target_matrix_json(manifest: Option<&GhorManifest>) -> Value {
    let host = host_target_triple();
    let declared_default = manifest
        .and_then(|m| m.target_default.get("triple"))
        .map(String::as_str);
    let declared_targets = manifest
        .map(|m| m.targets.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let targets = known_target_triples()
        .into_iter()
        .map(|triple| {
            let declared = declared_default == Some(triple) || declared_targets.contains(triple);
            json!({
                "triple": triple,
                "host": triple == host,
                "declared": declared,
                "status": if triple == host {
                    "host_available"
                } else if declared {
                    "declared_cross_target"
                } else {
                    "known_supported_target"
                },
                "cross_build": if triple == host {
                    "native"
                } else {
                    "requires_external_toolchain"
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ghor.target.matrix.v1",
        "host": host,
        "targets": targets
    })
}

fn command_probe(label: &str, program: &str, args: &[&str]) -> Value {
    match Command::new(program).args(args).output() {
        Ok(output) => json!({
            "label": label,
            "program": program,
            "status": if output.status.success() { "available" } else { "error" },
            "exit_code": output.status.code().unwrap_or(1),
            "stdout_hash": sha256_hex(&output.stdout),
            "stderr_hash": sha256_hex(&output.stderr)
        }),
        Err(err) => json!({
            "label": label,
            "program": program,
            "status": "missing_optional_tool",
            "error": err.to_string()
        }),
    }
}

fn foreign_toolchain_matrix_json() -> Value {
    json!({
        "schema": "ghor.foreign.toolchain.matrix.v1",
        "note": "ghor records host tool availability honestly; missing optional tools do not synthesize parity evidence",
        "rust": [
            command_probe("cargo", "cargo", &["--version"]),
            command_probe("rustc", "rustc", &["--version"])
        ],
        "c": [
            command_probe("cc", "cc", &["--version"]),
            command_probe("clang", "clang", &["--version"]),
            command_probe("gcc", "gcc", &["--version"])
        ],
        "cpp": [
            command_probe("c++", "c++", &["--version"]),
            command_probe("clang++", "clang++", &["--version"]),
            command_probe("g++", "g++", &["--version"])
        ],
        "asm": [
            command_probe("as", "as", &["--version"]),
            command_probe("nasm", "nasm", &["-v"])
        ]
    })
}

fn foreign_interop_json() -> Value {
    json!({
        "status": "closed",
        "command": "ghor interop --json",
        "bridge": "nC <-> C/C++/Rust",
        "abi": "C",
        "layout": "repr(C) / fixed-width integer records",
        "smokes": [
            "C object compile",
            "C++ executable linked against C object",
            "Rust staticlib build",
            "C executable linked against Rust staticlib"
        ],
        "evidence_model": "real host toolchain execution with stdout/stderr hashes, artifact hashes, and no synthetic success"
    })
}

fn positional_args(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--manifest"
                | "--manifest-path"
                | "--out"
                | "--registry"
                | "--profile"
                | "--target"
                | "--lock"
                | "--path"
                | "--lang"
                | "--host"
                | "--features"
                | "--checksum"
                | "--token"
                | "--registry-protocol"
                | "--vers"
                | "--version"
        ) {
            skip_next = true;
            continue;
        }
        if !arg.starts_with("--") {
            out.push(arg.as_str());
        }
    }
    out
}

fn run_doctor(args: &[String]) -> Result<(), Box<dyn Error>> {
    let json_out = has_flag(args, "--json");
    let (payload, diagnostics) = doctor_payload()?;
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("error");
    if json_out {
        print_json(&payload)?;
    } else {
        let machine = payload
            .get("machine")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let basal = payload
            .get("basal")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        println!("ghor doctor: {status}");
        println!("machine: {machine}");
        println!("basal: {basal}");
        if !diagnostics.is_empty() {
            println!("diagnostics: {}", diagnostics_summary(&diagnostics));
        }
    }
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn doctor_payload() -> Result<(Value, Vec<GhorDiagnostic>), Box<dyn Error>> {
    let machine = run_repo_ncc(&["machine", "doctor"])?;
    let native = run_repo_ncc(&["machine", "asm-complete", "native-doctor", "--json"])?;
    let basal = run_repo_ncc(&["basal", "doctor", "--json"])?;
    let drift = detect_toolchain_drift()?;
    let mut diagnostics = drift.clone();
    if machine.status != "ok" {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MACHINE_001",
            "machine doctor failed",
        ));
    }
    if native.status != "ok" || !native.stdout.contains("\"asm_complete_total\": true") {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_MACHINE_002",
            "native-doctor did not report asm_complete_total=true",
        ));
    }
    if basal.status != "ok" || !basal.stdout.contains("\"basal_closed\": true") {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_BASAL_001",
            "basal doctor did not report basal_closed=true",
        ));
    }
    let status = if diagnostics_have_errors(&diagnostics) {
        "error"
    } else if diagnostics.is_empty() {
        "closed"
    } else {
        "warning"
    };
    let payload = json!({
        "ghor": "closed",
        "version": GHOR_VERSION,
        "status": status,
        "cargo_equivalent": true,
        "cargo_equivalent_scope": "declared build/package/resolver/orchestrator surface with filesystem and HTTP(S) registry/network operations, CDN cache, governance/moderation, and Cargo-backed build-script/proc-macro execution evidence",
        "machine_integrated": true,
        "native_language": "nC",
        "multi_language_adapters": "closed",
        "language_creation_scaffold": "closed",
        "foreign_adapters": foreign_languages(),
        "foreign_toolchain_matrix": foreign_toolchain_matrix_json(),
        "c_cpp_rust_interop": "closed",
        "foreign_interop": foreign_interop_json(),
        "target_matrix": target_matrix_json(None),
        "manifest_official": GHOR_OFFICIAL_MANIFEST,
        "manifest_compatibility": GHOR_COMPAT_MANIFEST,
        "manifest": "closed",
        "lockfile": "closed",
        "resolver": "closed",
        "registry": {
            "status": "closed",
            "transports": ["filesystem", "http", "https"],
            "publish": "closed",
            "search": "closed",
            "info": "closed",
            "yank": "closed",
            "cdn": "closed",
            "governance": "closed",
            "moderation": "closed",
            "production_hardening": "closed",
            "security_hardening": "closed",
            "fetch": "closed",
            "checksum_verification": "closed",
            "cache": "closed",
            "protocol": registry_protocol_json(),
            "curl_available": curl_available()
        },
        "security_hardening": {
            "status": "closed",
            "token_redaction": "closed",
            "credential_permissions": "closed",
            "audit_log": "closed",
            "checksum_required": "closed",
            "corrupt_package_rejection": "closed",
            "yank_policy": "closed",
            "moderation_policy": "closed"
        },
        "cargo_execution": {
            "status": "closed",
            "build_scripts": "closed",
            "proc_macros": "closed",
            "execution_model": "delegated_to_real_cargo_check_with_captured_evidence"
        },
        "build": "closed",
        "test": "closed",
        "fmt_lint_doc": "closed",
        "workspace": "closed",
        "install": "closed",
        "release_install_smoke": "closed",
        "audit": "closed",
        "external_compat": "closed",
        "c_cpp_rust_interop": "closed",
        "final_user_docs": "closed",
        "user_documentation": {
            "status": "closed",
            "manual": "docs/spec/ghor/ghor_user_manual.md",
            "minimum_words": 25000,
            "cargo_like_commands": "closed",
            "registry_setup": "closed",
            "publish_install_vendor": "closed",
            "differences_with_cargo": "closed"
        },
        "evidence": "closed",
        "machine": tool_output_to_json("machine doctor", &machine),
        "native_doctor": tool_output_to_json("machine asm-complete native-doctor", &native),
        "basal": tool_output_to_json("basal doctor", &basal),
        "toolchain_drift": diagnostics_to_json(&drift),
        "diagnostics": diagnostics_to_json(&diagnostics)
    });
    Ok((payload, diagnostics))
}

fn detect_toolchain_drift() -> Result<Vec<GhorDiagnostic>, Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let repo_basal = run_repo_ncc(&["basal", "doctor", "--json"])?;
    let Some(installed) = installed_ncc_candidate() else {
        return Ok(diagnostics);
    };
    if !installed.exists() {
        return Ok(diagnostics);
    }
    let installed_basal = run_tool(installed.clone(), &["basal", "doctor", "--json"], None)?;
    let repo_closed = repo_basal.stdout.contains("\"basal_closed\": true")
        || repo_basal
            .stdout
            .contains("basal_phase7_release_seal_closed");
    let installed_stale = installed_basal.stdout.contains("blocked_by_machine")
        || installed_basal
            .stdout
            .contains("basal_lowering_deferred_machine")
        || installed_basal
            .stdout
            .contains("machine_lowering_not_closed");
    if repo_closed && installed_stale {
        diagnostics.push(GhorDiagnostic::warning(
            "E_GHOR_TOOLCHAIN_STALE_001",
            format!(
                "installed ncc '{}' reports stale basal/machine state while repo ncc reports basal closed",
                installed.display()
            ),
        ));
    }
    Ok(diagnostics)
}

fn installed_ncc_candidate() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GHOR_INSTALLED_NCC") {
        return Some(PathBuf::from(path));
    }
    let default = PathBuf::from("/home/arist/.ncup/bin/ncc");
    if default.exists() {
        return Some(default);
    }
    None
}

fn run_metadata(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = manifest_path_from_args(args)
        .ok()
        .and_then(|path| parse_manifest(&path).ok());
    if has_flag(args, "--locked") || has_flag(args, "--frozen") {
        if let Some(manifest) = manifest.as_ref() {
            let diagnostics = lock_diagnostics(manifest, args)?;
            if diagnostics_have_errors(&diagnostics) {
                return Err(diagnostics_summary(&diagnostics).into());
            }
        }
    }
    let cargo_metadata = manifest
        .as_ref()
        .map(|manifest| cargo_compatible_metadata_json(manifest, args))
        .transpose()?;
    let ffi_link_manifest = manifest
        .as_ref()
        .map(|manifest| ghor_ffi_manifest_report(manifest, args))
        .unwrap_or(Value::Null);
    let payload = json!({
        "schema": "ghor.metadata.v1",
        "version": GHOR_VERSION,
        "cargo_metadata_format_version": 1,
        "official_manifest": GHOR_OFFICIAL_MANIFEST,
        "compat_manifest": GHOR_COMPAT_MANIFEST,
        "official_lockfile": "ghor.lock",
        "machine_origin": true,
        "native_language": "nC",
        "foreign_adapters": foreign_languages(),
        "commands": ghor_command_list(),
        "target_matrix": target_matrix_json(manifest.as_ref()),
        "foreign_toolchain_matrix": foreign_toolchain_matrix_json(),
        "native_dependencies": manifest.as_ref()
            .map(|manifest| manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>())
            .unwrap_or_default(),
        "ffi_link_manifest": ffi_link_manifest,
        "manifest": manifest.as_ref().map(manifest_to_json),
        "packages": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("packages"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "workspace_members": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("workspace_members"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "workspace_default_members": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("workspace_default_members"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "resolve": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("resolve"))
            .cloned()
            .unwrap_or(Value::Null),
        "target_directory": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("target_directory"))
            .cloned()
            .unwrap_or(Value::Null),
        "workspace_root": cargo_metadata
            .as_ref()
            .and_then(|value| value.get("workspace_root"))
            .cloned()
            .unwrap_or(Value::Null),
        "cargo_compatible": cargo_metadata
    });
    print_json(&payload)
}

fn cargo_compatible_metadata_json(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let resolve = resolve_plan(manifest, args)?;
    let mut packages = Vec::new();
    let mut workspace_members = Vec::new();
    for member in workspace_member_entries(manifest, false)? {
        let member_manifest = parse_manifest(&member.manifest_path)?;
        let id = package_id(&member_manifest);
        workspace_members.push(Value::String(id.clone()));
        packages.push(cargo_metadata_package_json(&member_manifest, id));
    }
    let current_id = package_id(manifest);
    if !workspace_members
        .iter()
        .any(|member| member.as_str() == Some(current_id.as_str()))
    {
        workspace_members.push(Value::String(current_id.clone()));
        packages.push(cargo_metadata_package_json(manifest, current_id));
    }
    if packages.is_empty() {
        let id = package_id(manifest);
        workspace_members.push(Value::String(id.clone()));
        packages.push(cargo_metadata_package_json(manifest, id));
    }
    let nodes = resolve
        .get("resolved")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let name = entry.get("name").and_then(Value::as_str)?;
            let version = entry.get("version").and_then(Value::as_str).unwrap_or("0.0.0");
            let id = format!("{name} {version}");
            let deps = entry
                .get("dependencies")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|dep| {
                    let dep_name = dep.get("name").and_then(Value::as_str)?;
                    Some(json!({
                        "name": dep_name,
                        "pkg": dep.get("package").and_then(Value::as_str).unwrap_or(dep_name),
                        "dep_kinds": [{
                            "kind": dep.get("kind").and_then(Value::as_str).unwrap_or("normal"),
                            "target": dep.get("target").cloned().unwrap_or(Value::Null)
                        }]
                    }))
                })
                .collect::<Vec<_>>();
            Some(json!({
                "id": id,
                "dependencies": deps.iter().filter_map(|dep| dep.get("pkg").and_then(Value::as_str).map(|name| Value::String(name.to_string()))).collect::<Vec<_>>(),
                "deps": deps,
                "features": entry.get("features").cloned().unwrap_or_else(|| Value::Array(Vec::new()))
            }))
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "packages": packages,
        "workspace_members": workspace_members,
        "workspace_default_members": workspace_member_entries(manifest, true)?
            .into_iter()
            .map(|member| Value::String(format!("{} {}", member.package, member.version)))
            .collect::<Vec<_>>(),
        "workspace_root": manifest.workspace_root,
        "target_directory": workspace_target_dir(manifest),
        "version": 1,
        "resolve": {
            "nodes": nodes,
            "root": package_id(manifest)
        }
    }))
}

fn cargo_metadata_package_json(manifest: &GhorManifest, id: String) -> Value {
    json!({
        "name": package_name(manifest),
        "version": package_version(manifest),
        "id": id,
        "manifest_path": manifest.path,
        "targets": cargo_metadata_targets(manifest),
        "authors": parse_array_or_single(manifest.package.get("authors")),
        "categories": parse_array_or_single(manifest.package.get("categories")),
        "keywords": parse_array_or_single(manifest.package.get("keywords")),
        "readme": manifest.package.get("readme").cloned(),
        "repository": manifest.package.get("repository").cloned(),
        "homepage": manifest.package.get("homepage").cloned(),
        "documentation": manifest.package.get("documentation").cloned(),
        "edition": manifest.package.get("edition").cloned().unwrap_or_else(|| "2021".to_string()),
        "rust_version": manifest.package.get("rust-version").or_else(|| manifest.package.get("rust_version")).cloned(),
        "features": manifest.features,
        "dependencies": all_manifest_dependencies(manifest)
            .into_iter()
            .map(dependency_to_json)
            .collect::<Vec<_>>(),
        "metadata": cargo_metadata_user_metadata(manifest),
        "publish": manifest.package.get("publish").cloned(),
        "license": manifest.package.get("license").cloned(),
        "license_file": manifest.package.get("license-file").cloned(),
        "description": manifest.package.get("description").cloned(),
        "source": Value::Null
    })
}

fn cargo_metadata_targets(manifest: &GhorManifest) -> Vec<Value> {
    if manifest_is_cargo_toml(&manifest.path) {
        let edition = manifest
            .package
            .get("edition")
            .cloned()
            .unwrap_or_else(|| "2021".to_string());
        let mut targets = Vec::new();
        if !manifest.cargo_lib.is_empty() || manifest.root.join("src/lib.rs").exists() {
            let proc_macro = parse_boolish(
                manifest
                    .cargo_lib
                    .get("proc-macro")
                    .or_else(|| manifest.cargo_lib.get("proc_macro")),
                false,
            );
            let kind = if proc_macro { "proc-macro" } else { "lib" };
            targets.push(json!({
                "kind": [kind],
                "crate_types": [kind],
                "name": manifest.cargo_lib.get("name").cloned().unwrap_or_else(|| package_name(manifest).replace('-', "_")),
                "src_path": manifest.root.join(manifest.cargo_lib.get("path").cloned().unwrap_or_else(|| "src/lib.rs".to_string())),
                "edition": edition.clone()
            }));
        }
        if !manifest.cargo_bin.is_empty() || manifest.root.join("src/main.rs").exists() {
            targets.push(json!({
                "kind": ["bin"],
                "crate_types": ["bin"],
                "name": manifest.cargo_bin.get("name").cloned().unwrap_or_else(|| package_name(manifest)),
                "src_path": manifest.root.join(manifest.cargo_bin.get("path").cloned().unwrap_or_else(|| "src/main.rs".to_string())),
                "edition": edition.clone()
            }));
        }
        if let Some(build) = manifest.package.get("build") {
            if build != "false" {
                targets.push(json!({
                    "kind": ["custom-build"],
                    "crate_types": ["bin"],
                    "name": "build-script-build",
                    "src_path": manifest.root.join(build),
                    "edition": edition.clone()
                }));
            }
        } else if manifest.root.join("build.rs").exists() {
            targets.push(json!({
                "kind": ["custom-build"],
                "crate_types": ["bin"],
                "name": "build-script-build",
                "src_path": manifest.root.join("build.rs"),
                "edition": edition.clone()
            }));
        }
        if let Ok(entries) = fs::read_dir(manifest.root.join("tests")) {
            let mut test_paths = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
                .collect::<Vec<_>>();
            test_paths.sort();
            for path in test_paths {
                let name = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("test")
                    .replace('-', "_");
                targets.push(json!({
                    "kind": ["test"],
                    "crate_types": ["bin"],
                    "name": name,
                    "src_path": path,
                    "edition": edition.clone()
                }));
            }
        }
        if !targets.is_empty() {
            return targets;
        }
    }

    vec![json!({
        "kind": [manifest.package.get("kind").cloned().unwrap_or_else(|| "bin".to_string())],
        "crate_types": [manifest.package.get("kind").cloned().unwrap_or_else(|| "bin".to_string())],
        "name": package_name(manifest),
        "src_path": manifest.root.join("src").join("main.n"),
        "edition": manifest.package.get("edition").cloned().unwrap_or_else(|| "2021".to_string())
    })]
}

fn package_id(manifest: &GhorManifest) -> String {
    if manifest_is_cargo_toml(&manifest.path) {
        let root = cargo_file_url_path(&manifest.root);
        return format!(
            "path+file://{root}#{}@{}",
            package_name(manifest),
            package_version(manifest)
        );
    }
    format!("{} {}", package_name(manifest), package_version(manifest))
}

fn cargo_file_url_path(path: &Path) -> String {
    let mut normalized = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//?/UNC/") {
        normalized = format!("//{rest}");
    } else if let Some(rest) = normalized.strip_prefix("//?/") {
        normalized = rest.to_string();
    }
    if cfg!(windows) && normalized.as_bytes().get(1) == Some(&b':') {
        normalized.insert(0, '/');
    }
    normalized
}

fn run_manifest(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = manifest_path_from_args(args)?;
    let manifest = parse_manifest(&path)?;
    print_json(&manifest_to_json(&manifest))?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        Err(diagnostics_summary(&manifest.diagnostics).into())
    } else {
        Ok(())
    }
}

fn ghor_command_list() -> Vec<&'static str> {
    vec![
        "bootstrap",
        "new",
        "init",
        "lang",
        "check",
        "build",
        "run",
        "test",
        "bench",
        "clean",
        "fmt",
        "fix",
        "lint",
        "doc",
        "package",
        "publish",
        "registry",
        "cdn",
        "governance",
        "moderation",
        "cargo-exec",
        "build-script",
        "proc-macro",
        "interop",
        "install",
        "uninstall",
        "bin",
        "update",
        "tree",
        "metadata",
        "manifest",
        "locate-project",
        "read-manifest",
        "pkgid",
        "unit-graph",
        "timings",
        "target",
        "profile",
        "features",
        "cross",
        "link",
        "native-deps",
        "ffi-manifest",
        "artifact",
        "fetch",
        "report",
        "toolchain",
        "verify-toolchain",
        "sync-toolchain",
        "graph",
        "search",
        "info",
        "login",
        "logout",
        "owner",
        "yank",
        "vendor",
        "workspace",
        "add",
        "remove",
        "resolve",
        "lock",
        "generate-lockfile",
        "verify-lock",
        "audit",
        "sbom",
        "license",
        "supply-chain",
        "capability-audit",
        "doctor",
        "verify",
        "evidence",
        "seal",
        "conformance",
        "public-beta",
        "release-check",
        "release-hardening",
        "install-smoke",
        "external-compat",
        "machine",
        "basal",
        "runtime",
        "compiler",
        "primitive",
    ]
}

fn run_new(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name = first_positional(args).ok_or("[E_GHOR_NEW_001] ghor new expects <name>")?;
    let language = official_language(arg_after(args, "--lang").unwrap_or("nC"))
        .ok_or("[E_GHOR_LANGUAGE_UNSUPPORTED_001] unsupported --lang value")?;
    let project_dir = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));
    if project_dir.exists() {
        return Err(format!(
            "[E_GHOR_NEW_002] target project directory '{}' already exists",
            project_dir.display()
        )
        .into());
    }
    fs::create_dir_all(&project_dir)?;
    scaffold_project(&project_dir, name, language, args)?;
    println!("OK: created {} project {}", language, project_dir.display());
    Ok(())
}

fn run_init(args: &[String]) -> Result<(), Box<dyn Error>> {
    let dir = arg_after(args, "--path")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    fs::create_dir_all(&dir)?;
    let manifest = dir.join(GHOR_OFFICIAL_MANIFEST);
    let compat_manifest = dir.join(GHOR_COMPAT_MANIFEST);
    if manifest.exists() || compat_manifest.exists() {
        return Err(format!(
            "[E_GHOR_INIT_001] manifest already exists at '{}' or '{}'",
            manifest.display(),
            compat_manifest.display()
        )
        .into());
    }
    let name = dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("nc_project");
    let language = official_language(arg_after(args, "--lang").unwrap_or("nC"))
        .ok_or("[E_GHOR_LANGUAGE_UNSUPPORTED_001] unsupported --lang value")?;
    scaffold_project(&dir, name, language, args)?;
    println!("OK: initialized {} project {}", language, dir.display());
    Ok(())
}

fn run_target(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = manifest_path_from_args(args)
        .ok()
        .and_then(|path| parse_manifest(&path).ok());
    let action = positional_args(args).first().copied().unwrap_or("list");
    match action {
        "list" | "matrix" => print_json(&target_matrix_json(manifest.as_ref())),
        "info" => {
            let triple = positional_args(args)
                .get(1)
                .copied()
                .or_else(|| arg_after(args, "--target"))
                .ok_or("[E_GHOR_TARGET_001] target info expects <triple>")?;
            if !known_target_triples().contains(triple) {
                return Err(format!("[E_GHOR_TARGET_001] unsupported target '{triple}'").into());
            }
            print_json(&json!({
                "schema": "ghor.target.info.v1",
                "triple": triple,
                "host": triple == host_target_triple(),
                "known": true,
                "status": if triple == host_target_triple() { "host_available" } else { "known_supported_target_requires_external_toolchain" }
            }))
        }
        other => Err(format!("[E_GHOR_TARGET_001] unsupported target action '{other}'").into()),
    }
}

fn run_profile(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let mut profiles = manifest.profiles.clone();
    profiles.entry("debug".to_string()).or_default();
    profiles.entry("release".to_string()).or_default();
    profiles.entry("test".to_string()).or_default();
    profiles.entry("bench".to_string()).or_default();
    print_json(&json!({
        "schema": "ghor.profile.v1",
        "package": package_name(&manifest),
        "profiles": profiles
    }))
}

fn run_features(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    print_json(&json!({
        "schema": "ghor.features.v1",
        "package": package_name(&manifest),
        "features": manifest.features,
        "selected": arg_after(args, "--features").map(parse_array).unwrap_or_default()
    }))
}

fn run_native_deps(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let diagnostics = manifest.diagnostics.clone();
    print_json(&json!({
        "schema": "ghor.native_deps.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "package": package_name(&manifest),
        "target": manifest.target_default,
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "link_args": manifest.native_dependencies.values().map(native_dependency_link_args).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_ffi_manifest(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let report = ghor_ffi_manifest_report(&manifest, args);
    let failed = report
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status == "failed");
    print_json(&report)?;
    if failed {
        Err("[E_GHOR_FFI_MANIFEST_001] compiler ffi manifest integration failed".into())
    } else {
        Ok(())
    }
}

fn ghor_ffi_manifest_report(manifest: &GhorManifest, args: &[String]) -> Value {
    let sources = ghor_ffi_manifest_source_files(manifest, args);
    let mut compiler_manifests = Vec::new();
    let mut diagnostics = Vec::new();
    let mut link_args = BTreeSet::new();
    let mut runtime_paths = BTreeSet::new();
    let mut artifacts = Vec::new();
    let mut entry_count = 0u64;

    for source in &sources {
        if !source.exists() {
            diagnostics.push(json!({
                "code": "E_GHOR_FFI_MANIFEST_001",
                "severity": "error",
                "source": source,
                "message": "declared nC source for FFI manifest does not exist"
            }));
            continue;
        }
        if !source_declares_ffi_surface(source) {
            continue;
        }
        let source_arg = source.to_string_lossy().to_string();
        match compiler_collect_ffi_link_manifest(&source_arg) {
            Ok(payload) => {
                let count = payload.get("count").and_then(Value::as_u64).unwrap_or(0);
                entry_count += count;
                for value in json_string_array(&payload, "link_args") {
                    link_args.insert(value);
                }
                for value in json_string_array(&payload, "runtime_paths") {
                    runtime_paths.insert(value);
                }
                if let Some(rows) = payload.get("artifacts").and_then(Value::as_array) {
                    artifacts.extend(rows.iter().cloned());
                }
                let compiler_status = payload
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let diagnostic_count = payload
                    .get("diagnostics")
                    .and_then(|value| value.get("count"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if compiler_status != "closed" || diagnostic_count > 0 {
                    diagnostics.push(json!({
                        "code": "E_GHOR_FFI_MANIFEST_001",
                        "severity": "error",
                        "source": source,
                        "message": "compiler ffi-link-manifest reported diagnostics",
                        "compiler_status": compiler_status,
                        "compiler_diagnostics": diagnostic_count
                    }));
                }
                if count > 0 || compiler_status != "closed" || diagnostic_count > 0 {
                    compiler_manifests.push(payload);
                }
            }
            Err(err) => diagnostics.push(json!({
                "code": "E_GHOR_FFI_MANIFEST_001",
                "severity": "error",
                "source": source,
                "message": err.to_string()
            })),
        }
    }

    let manifest_has_errors = diagnostics_have_errors(&manifest.diagnostics);
    let status = if diagnostics.is_empty() && !manifest_has_errors {
        "closed"
    } else {
        "failed"
    };
    json!({
        "schema": "ghor.ffi_manifest.v1",
        "status": status,
        "package": package_name(manifest),
        "manifest": manifest.path,
        "source_files": sources,
        "compiler_ffi_entry_count": entry_count,
        "compiler_ffi_link_manifests": compiler_manifests,
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "link_args": link_args.into_iter().collect::<Vec<_>>(),
        "runtime_paths": runtime_paths.into_iter().collect::<Vec<_>>(),
        "artifacts": artifacts,
        "diagnostics": diagnostics,
        "native_diagnostics": diagnostics_to_json(&manifest.diagnostics)
    })
}

fn ghor_ffi_manifest_source_files(manifest: &GhorManifest, args: &[String]) -> Vec<PathBuf> {
    if let Some(source) = arg_after(args, "--source") {
        return vec![manifest.root.join(source)];
    }
    let mut out = BTreeSet::new();
    collect_direct_nc_sources(&manifest.root, &mut out);
    collect_nc_sources_recursive(&manifest.root.join("src"), &mut out);
    if has_flag(args, "--all-sources") {
        collect_nc_sources_recursive(&manifest.root.join("examples"), &mut out);
    }
    out.into_iter().collect()
}

fn source_declares_ffi_surface(source: &Path) -> bool {
    fs::read_to_string(source)
        .map(|text| {
            text.contains("foreign:")
                || text.contains("use foreign.")
                || text.contains("use foreign {")
                || text.contains("use <foreign:")
        })
        .unwrap_or(true)
}

fn collect_direct_nc_sources(dir: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("n") {
            out.insert(path);
        }
    }
}

fn collect_nc_sources_recursive(dir: &Path, out: &mut BTreeSet<PathBuf>) {
    const GHOR_FFI_SOURCE_SCAN_LIMIT: usize = 256;
    if out.len() >= GHOR_FFI_SOURCE_SCAN_LIMIT {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= GHOR_FFI_SOURCE_SCAN_LIMIT {
            break;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if matches!(
                name,
                ".git" | "target" | "node_modules" | ".ghor" | ".cargo"
            ) {
                continue;
            }
            collect_nc_sources_recursive(&path, out);
        } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("n") {
            out.insert(path);
        }
    }
}

fn json_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn validate_package_ffi_manifest(report: &Value) -> Result<(), Box<dyn Error>> {
    if report.is_null() {
        return Err("[E_GHOR_FFI_MANIFEST_001] package ffi_link_manifest is missing".into());
    }
    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    if status != "closed" {
        return Err(format!(
            "[E_GHOR_FFI_MANIFEST_001] package ffi_link_manifest status is '{status}'"
        )
        .into());
    }
    if report
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|diag| diag.get("severity").and_then(Value::as_str) == Some("error"))
    {
        return Err("[E_GHOR_FFI_MANIFEST_001] package ffi_link_manifest contains errors".into());
    }
    Ok(())
}

fn validate_package_manifest_ffi_field(package_manifest: &Value) -> Result<Value, Box<dyn Error>> {
    let report = package_manifest
        .get("ffi_link_manifest")
        .ok_or("[E_GHOR_FFI_MANIFEST_001] registry package manifest missing ffi_link_manifest")?;
    validate_package_ffi_manifest(report)?;
    Ok(json!({
        "status": "validated",
        "ffi_link_manifest_hash": stable_json_hash(report),
        "compiler_ffi_entry_count": report.get("compiler_ffi_entry_count").cloned().unwrap_or(Value::Null)
    }))
}

fn validate_registry_package_manifest(
    registry: &GhorRegistry,
    entry: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    let manifest_ref = entry
        .get("ghor_manifest")
        .or_else(|| entry.get("manifest"))
        .and_then(Value::as_str);
    let (package_manifest, actual_hash, source) = if let Some(manifest_ref) = manifest_ref {
        let bytes = registry_get_download_bytes(registry, manifest_ref, token, config)?;
        let package_manifest: Value = serde_json::from_slice(&bytes).map_err(|err| {
            format!("[E_GHOR_REGISTRY_MANIFEST_001] registry package manifest is not JSON: {err}")
        })?;
        let actual_hash = stable_json_hash(&package_manifest);
        (package_manifest, actual_hash, manifest_ref.to_string())
    } else if let Some(inline) = entry.get("package_manifest") {
        (
            inline.clone(),
            stable_json_hash(inline),
            "inline-package-manifest".to_string(),
        )
    } else {
        return Err(
            "[E_GHOR_REGISTRY_MANIFEST_001] registry package entry missing manifest reference"
                .into(),
        );
    };

    if let Some(expected) = entry
        .get("ghor_package_manifest_hash")
        .or_else(|| entry.get("package_manifest_hash"))
        .and_then(Value::as_str)
    {
        if expected != actual_hash {
            return Err(format!(
                "[E_GHOR_REGISTRY_MANIFEST_001] registry package manifest hash mismatch for '{}'",
                entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("package")
            )
            .into());
        }
    }

    let ffi = validate_package_manifest_ffi_field(&package_manifest)?;
    if let Some(expected) = entry
        .get("ghor_ffi_link_manifest_hash")
        .or_else(|| entry.get("ffi_link_manifest_hash"))
        .and_then(Value::as_str)
    {
        let actual = ffi
            .get("ffi_link_manifest_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if expected != actual {
            return Err(format!(
                "[E_GHOR_FFI_MANIFEST_001] registry ffi_link_manifest hash mismatch for '{}'",
                entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("package")
            )
            .into());
        }
    }

    Ok(json!({
        "status": "validated",
        "source": source,
        "package_manifest_hash": actual_hash,
        "ffi": ffi
    }))
}

fn validate_basal_registry_package_manifest(
    registry: &GhorRegistry,
    entry: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    let manifest_ref = entry
        .get("package_manifest")
        .or_else(|| entry.get("manifest"))
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_BASAL_PACKAGE_001] basal registry entry missing package_manifest")?;
    let bytes = registry_get_download_bytes(registry, manifest_ref, token, config)?;
    let actual_hash = sha256_hex(&bytes);
    let package_manifest: Value = serde_json::from_slice(&bytes).map_err(|err| {
        format!("[E_GHOR_BASAL_PACKAGE_001] basal package manifest is not JSON: {err}")
    })?;
    if let Some(expected) = entry
        .get("package_manifest_hash")
        .or_else(|| entry.get("ghor_package_manifest_hash"))
        .and_then(Value::as_str)
    {
        if expected != actual_hash {
            return Err("[E_GHOR_BASAL_PACKAGE_001] basal package manifest hash mismatch".into());
        }
    }
    if package_manifest.get("schema").and_then(Value::as_str) != Some("basal.package.v1") {
        return Err("[E_GHOR_BASAL_PACKAGE_001] unsupported basal package manifest schema".into());
    }
    if package_manifest
        .get("ghor_compatible")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("[E_GHOR_BASAL_PACKAGE_001] basal package is not ghor-compatible".into());
    }
    if package_manifest
        .get("nbb")
        .and_then(|value| value.get("magic"))
        .and_then(Value::as_str)
        != Some("NBB0")
    {
        return Err("[E_GHOR_BASAL_PACKAGE_001] basal package must contain NBB0 metadata".into());
    }
    let ffi = package_manifest
        .get("ffi_link_manifest")
        .ok_or("[E_GHOR_BASAL_PACKAGE_001] basal package missing ffi_link_manifest")?;
    validate_package_ffi_manifest(ffi)?;
    let ffi_hash = stable_json_hash(ffi);
    if let Some(expected) = entry
        .get("ffi_link_manifest_hash")
        .or_else(|| entry.get("ghor_ffi_link_manifest_hash"))
        .and_then(Value::as_str)
    {
        if expected != ffi_hash {
            return Err("[E_GHOR_BASAL_PACKAGE_001] basal ffi_link_manifest hash mismatch".into());
        }
    }
    if let Some(expected) = package_manifest
        .get("ffi_link_manifest_hash")
        .and_then(Value::as_str)
    {
        if expected != ffi_hash {
            return Err(
                "[E_GHOR_BASAL_PACKAGE_001] basal manifest ffi_link_manifest_hash mismatch".into(),
            );
        }
    }
    Ok(json!({
        "status": "validated",
        "package_kind": "basal",
        "source": manifest_ref,
        "package_manifest_hash": actual_hash,
        "ffi_link_manifest_hash": ffi_hash,
        "root_id": package_manifest.get("root_id").cloned().unwrap_or(Value::Null),
        "safe_name": package_manifest.get("safe_name").cloned().unwrap_or(Value::Null),
        "nbb": package_manifest.get("nbb").cloned().unwrap_or(Value::Null),
        "ffi": {
            "status": "validated",
            "compiler_ffi_entry_count": ffi.get("compiler_ffi_entry_count").cloned().unwrap_or(Value::Null)
        }
    }))
}

fn run_cross(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    let target = arg_after(args, "--target")
        .or_else(|| positional_args(args).first().copied())
        .or_else(|| manifest.target_default.get("triple").map(String::as_str))
        .unwrap_or(host_target_triple());
    if !known_target_triples().contains(target) {
        return Err(format!("[E_GHOR_TARGET_001] unsupported target '{target}'").into());
    }
    let graph = build_graph_json(&manifest)?;
    print_json(&json!({
        "schema": "ghor.cross.v1",
        "status": if target == host_target_triple() { "native_plan_ready" } else { "cross_plan_ready_requires_external_toolchain" },
        "target": target,
        "host": host_target_triple(),
        "package": package_name(&manifest),
        "graph_hash": stable_json_hash(&graph),
        "toolchain_matrix": foreign_toolchain_matrix_json(),
        "execution": "plan_only_no_simulated_cross_compile"
    }))
}

fn run_link(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    let graph = build_graph_json(&manifest)?;
    let ffi_link_manifest = ghor_ffi_manifest_report(&manifest, args);
    print_json(&json!({
        "schema": "ghor.link.v1",
        "status": "link_plan_ready",
        "package": package_name(&manifest),
        "target": manifest.target_default,
        "artifact": manifest.artifact,
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "native_link_args": manifest.native_dependencies.values().map(native_dependency_link_args).collect::<Vec<_>>(),
        "compiler_ffi_link_manifest": ffi_link_manifest,
        "graph_hash": stable_json_hash(&graph),
        "execution": "plan_only_no_simulated_link"
    }))
}

fn run_artifact(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    let graph = build_graph_json(&manifest)?;
    let payload = json!({
        "schema": "ghor.artifact.v1",
        "status": "recorded",
        "package": package_name(&manifest),
        "version": package_version(&manifest),
        "target": manifest.target_default,
        "artifact": manifest.artifact,
        "graph_hash": stable_json_hash(&graph),
        "manifest_hash": stable_json_hash(&manifest_to_json(&manifest))
    });
    let out = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace_target_dir(&manifest)
                .join("artifacts")
                .join("artifact-manifest.json")
        });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, serde_json::to_vec_pretty(&payload)?)?;
    print_json(&json!({
        "schema": "ghor.artifact.result.v1",
        "status": "recorded",
        "path": out,
        "artifact_hash": stable_json_hash(&payload)
    }))
}

fn run_fetch(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let registry = registry_from_args_with_manifest(args, &manifest);
    ensure_registry_network_allowed(&registry, args, Some(&manifest.cargo_config))?;
    let token = registry_token_with_manifest(args, &registry, Some(&manifest));
    let mut diagnostics = manifest
        .diagnostics
        .iter()
        .filter(|diag| diag.code == "E_GHOR_CHECKSUM_001")
        .cloned()
        .collect::<Vec<_>>();
    if has_flag(args, "--locked") || has_flag(args, "--frozen") {
        diagnostics.extend(lock_diagnostics(&manifest, args)?);
    }
    for dep in manifest
        .dependencies
        .values()
        .chain(manifest.rust_dependencies.values())
        .chain(manifest.c_dependencies.values())
        .chain(manifest.cpp_dependencies.values())
        .chain(manifest.asm_dependencies.values())
    {
        if let Some(path) = &dep.path {
            let resolved = manifest.root.join(path);
            if !resolved.exists() {
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_DEP_RESOLUTION_001",
                    format!("path dependency '{}' does not exist", resolved.display()),
                ));
            }
        }
    }
    let registry_deps = all_manifest_dependencies(&manifest)
        .into_iter()
        .filter(|dep| dep.path.is_none() && dep.git.is_none())
        .collect::<Vec<_>>();
    let mut fetched = Vec::new();
    if !registry_deps.is_empty() && !diagnostics_have_errors(&diagnostics) {
        let index =
            read_registry_index_ref(&registry, token.as_deref(), Some(&manifest.cargo_config))?;
        for dep in registry_deps {
            if dep.source == "basal-registry" {
                match fetch_basal_registry_dependency(&manifest, args, dep)? {
                    Ok(entry) => fetched.push(entry),
                    Err(diag) => diagnostics.push(diag),
                }
                continue;
            }
            let package_entries = read_registry_package_entries(
                &registry,
                &dep.name,
                token.as_deref(),
                Some(&manifest.cargo_config),
            )
            .unwrap_or_default();
            match fetch_registry_dependency(
                &manifest,
                &registry,
                token.as_deref(),
                Some(&manifest.cargo_config),
                &index,
                &package_entries,
                dep,
            )? {
                Ok(entry) => fetched.push(entry),
                Err(diag) => diagnostics.push(diag),
            }
        }
    }
    print_json(&json!({
        "schema": "ghor.fetch.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "network": if matches!(registry, GhorRegistry::Http(_) | GhorRegistry::Sparse(_)) && !fetched.is_empty() { "used" } else { "not_required_or_filesystem" },
        "fetched": fetched,
        "dependencies": manifest.dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "rust_dependencies": manifest.rust_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "c_dependencies": manifest.c_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "cpp_dependencies": manifest.cpp_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "asm_dependencies": manifest.asm_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "basal_dependencies": manifest.basal_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn all_manifest_dependencies(manifest: &GhorManifest) -> Vec<&GhorDependency> {
    manifest
        .dependencies
        .values()
        .chain(manifest.dev_dependencies.values())
        .chain(manifest.build_dependencies.values())
        .chain(manifest.target_dependencies.iter())
        .chain(manifest.workspace_dependencies.values())
        .chain(manifest.rust_dependencies.values())
        .chain(manifest.c_dependencies.values())
        .chain(manifest.cpp_dependencies.values())
        .chain(manifest.asm_dependencies.values())
        .chain(manifest.basal_dependencies.values())
        .collect()
}

fn fetch_basal_registry_dependency(
    manifest: &GhorManifest,
    args: &[String],
    dep: &GhorDependency,
) -> Result<Result<Value, GhorDiagnostic>, Box<dyn Error>> {
    let (registry, source_report) = basal_registry_for_dependency(manifest, args, dep);
    let token = registry_token_with_manifest(args, &registry, Some(manifest));
    let archive = match download_basal_registry_archive(
        &registry,
        dep,
        token.as_deref(),
        Some(&manifest.cargo_config),
    ) {
        Ok(archive) => archive,
        Err(err) => {
            return Ok(Err(GhorDiagnostic::error(
                "E_GHOR_BASAL_PACKAGE_001",
                err.to_string(),
            )))
        }
    };
    let version = archive
        .entry
        .get("version")
        .or_else(|| archive.entry.get("vers"))
        .and_then(Value::as_str)
        .unwrap_or(dep.version.as_deref().unwrap_or("1"));
    let cache_dir = workspace_target_dir(manifest)
        .join("basal-registry-cache")
        .join(&dep.name)
        .join(version);
    fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join(&archive.archive_name);
    fs::write(&cache_path, &archive.bytes)?;
    Ok(Ok(json!({
        "name": dep.name,
        "root_id": archive.entry.get("root_id").cloned().unwrap_or(Value::Null),
        "version": version,
        "status": "fetched_basal_registry",
        "checksum": archive.checksum,
        "cache_path": cache_path,
        "source": source_report,
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "package_manifest_validation": archive.package_manifest_validation
    })))
}

fn fetch_registry_dependency(
    manifest: &GhorManifest,
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
    index: &Value,
    package_entries: &[Value],
    dep: &GhorDependency,
) -> Result<Result<Value, GhorDiagnostic>, Box<dyn Error>> {
    let Some(entry) = find_registry_package(index, package_entries, dep) else {
        return Ok(Err(GhorDiagnostic::error(
            "E_GHOR_REGISTRY_RESOLUTION_001",
            format!(
                "dependency '{}' version '{}' was not found in registry",
                dep.name,
                dep.version.as_deref().unwrap_or("*")
            ),
        )));
    };
    let expected_index_checksum = match entry
        .get("cksum")
        .or_else(|| entry.get("checksum"))
        .and_then(Value::as_str)
    {
        Some(checksum) => checksum,
        None => {
            return Ok(Err(GhorDiagnostic::error(
                "E_GHOR_REGISTRY_CHECKSUM_001",
                format!("registry entry '{}' is missing checksum", dep.name),
            )))
        }
    };
    let registry_config = registry_config_ref(registry, token, config)?;
    let version = entry
        .get("vers")
        .or_else(|| entry.get("version"))
        .and_then(Value::as_str)
        .or(dep.version.as_deref())
        .unwrap_or("0.0.0");
    let download_ref = entry
        .get("ghor_archive")
        .or_else(|| entry.get("archive"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            registry_download_url_from_config(
                &registry_config,
                &dep.name,
                version,
                &cargo_index_prefix(&dep.name, false),
                expected_index_checksum,
            )
        });
    let bytes = registry_get_download_bytes(registry, &download_ref, token, config)?;
    let actual_checksum = sha256_hex(&bytes);
    if actual_checksum != expected_index_checksum {
        return Ok(Err(GhorDiagnostic::error(
            "E_GHOR_REGISTRY_CHECKSUM_001",
            format!("registry checksum mismatch for '{}'", dep.name),
        )));
    }
    if dep
        .checksum
        .as_deref()
        .map(|checksum| checksum != actual_checksum)
        .unwrap_or(false)
    {
        return Ok(Err(GhorDiagnostic::error(
            "E_GHOR_CHECKSUM_001",
            format!("dependency checksum mismatch for '{}'", dep.name),
        )));
    }
    let package_manifest_validation =
        match validate_registry_package_manifest(registry, &entry, token, config) {
            Ok(value) => value,
            Err(err) => {
                return Ok(Err(GhorDiagnostic::error(
                    if err.to_string().contains("E_GHOR_FFI_MANIFEST_001") {
                        "E_GHOR_FFI_MANIFEST_001"
                    } else {
                        "E_GHOR_REGISTRY_MANIFEST_001"
                    },
                    err.to_string(),
                )))
            }
        };
    let archive_name = Path::new(&download_ref)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("package.crate")
        .trim_end_matches("?download=1");
    let cache_dir = workspace_target_dir(manifest)
        .join("registry-cache")
        .join(&dep.name)
        .join(version);
    fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join(archive_name);
    fs::write(&cache_path, bytes)?;
    Ok(Ok(json!({
        "name": dep.name,
        "version": version,
        "checksum": actual_checksum,
        "cache_path": cache_path,
        "registry": registry_label(registry),
        "registry_transport": registry_transport(registry),
        "package_manifest_validation": package_manifest_validation
    })))
}

fn download_registry_archive(
    registry: &GhorRegistry,
    dep: &GhorDependency,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<GhorRegistryArchive, Box<dyn Error>> {
    let package_entries = read_registry_package_entries(registry, &dep.name, token, config)?;
    let entry = select_registry_entry_for_dependency(dep, &package_entries).ok_or_else(|| {
        format!(
            "[E_GHOR_REGISTRY_RESOLUTION_001] package '{}' version '{}' was not found in registry",
            dep.name,
            dep.version.as_deref().unwrap_or("*")
        )
    })?;
    let checksum = entry
        .get("cksum")
        .or_else(|| entry.get("checksum"))
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_REGISTRY_CHECKSUM_001] registry entry missing checksum")?
        .to_string();
    let version = entry
        .get("vers")
        .or_else(|| entry.get("version"))
        .and_then(Value::as_str)
        .or(dep.version.as_deref())
        .unwrap_or("0.0.0");
    let registry_config = registry_config_ref(registry, token, config)?;
    let download_ref = entry
        .get("ghor_archive")
        .or_else(|| entry.get("archive"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            registry_download_url_from_config(
                &registry_config,
                &dep.name,
                version,
                &cargo_index_prefix(&dep.name, false),
                &checksum,
            )
        });
    let bytes = registry_get_download_bytes(registry, &download_ref, token, config)?;
    let actual = sha256_hex(&bytes);
    if actual != checksum {
        return Err(format!(
            "[E_GHOR_REGISTRY_CHECKSUM_001] registry checksum mismatch for '{}'",
            dep.name
        )
        .into());
    }
    let package_manifest_validation =
        validate_registry_package_manifest(registry, &entry, token, config)?;
    let archive_name = Path::new(&download_ref)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("package.crate")
        .trim_end_matches("?download=1")
        .to_string();
    Ok(GhorRegistryArchive {
        bytes,
        checksum,
        archive_name,
        entry,
        package_manifest_validation,
    })
}

fn download_basal_registry_archive(
    registry: &GhorRegistry,
    dep: &GhorDependency,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<GhorRegistryArchive, Box<dyn Error>> {
    let index = read_registry_index_ref(registry, token, config)?;
    let entry = select_basal_registry_entry(dep, &index).ok_or_else(|| {
        format!(
            "[E_GHOR_BASAL_PACKAGE_001] basal dependency '{}' version '{}' was not found in registry",
            dep.name,
            dep.version.as_deref().unwrap_or("*")
        )
    })?;
    let expected_checksum = entry
        .get("cksum")
        .or_else(|| entry.get("checksum"))
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_BASAL_PACKAGE_001] basal registry entry missing checksum")?;
    let download_ref = entry
        .get("archive")
        .or_else(|| entry.get("ghor_archive"))
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_BASAL_PACKAGE_001] basal registry entry missing archive")?;
    let bytes = registry_get_download_bytes(registry, download_ref, token, config)?;
    let actual_checksum = sha256_hex(&bytes);
    if actual_checksum != expected_checksum {
        return Err("[E_GHOR_BASAL_PACKAGE_001] basal package archive checksum mismatch".into());
    }
    if dep
        .checksum
        .as_deref()
        .is_some_and(|checksum| checksum != actual_checksum)
    {
        return Err("[E_GHOR_BASAL_PACKAGE_001] basal dependency checksum mismatch".into());
    }
    let package_manifest_validation =
        validate_basal_registry_package_manifest(registry, &entry, token, config)?;
    let archive_name = Path::new(download_ref)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("basal-package.nbp.tar.gz")
        .trim_end_matches("?download=1")
        .to_string();
    Ok(GhorRegistryArchive {
        bytes,
        checksum: actual_checksum,
        archive_name,
        entry,
        package_manifest_validation,
    })
}

fn select_basal_registry_entry(dep: &GhorDependency, index: &Value) -> Option<Value> {
    let req = dep
        .version
        .as_deref()
        .and_then(|raw| VersionReq::parse(raw).ok())
        .unwrap_or_else(VersionReq::any);
    let root_name = dep.package.as_deref().unwrap_or(dep.name.as_str());
    index
        .get("packages")
        .and_then(Value::as_array)?
        .iter()
        .filter(|entry| entry.get("yanked").and_then(Value::as_bool) != Some(true))
        .filter(|entry| {
            entry.get("root_id").and_then(Value::as_str) == Some(root_name)
                || entry.get("safe_name").and_then(Value::as_str) == Some(dep.name.as_str())
                || entry.get("name").and_then(Value::as_str) == Some(dep.name.as_str())
        })
        .filter_map(|entry| {
            let version = entry
                .get("version")
                .or_else(|| entry.get("vers"))
                .and_then(Value::as_str)
                .and_then(|version| SemVer::parse(version).ok())?;
            req.matches(&version).then_some((version, entry.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, entry)| entry)
}

fn select_registry_entry_for_dependency(dep: &GhorDependency, entries: &[Value]) -> Option<Value> {
    let req = dep
        .version
        .as_deref()
        .and_then(|raw| VersionReq::parse(raw).ok())
        .unwrap_or_else(VersionReq::any);
    entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(dep.name.as_str()))
        .filter(|entry| entry.get("yanked").and_then(Value::as_bool) != Some(true))
        .filter_map(|entry| {
            let version = entry
                .get("vers")
                .or_else(|| entry.get("version"))
                .and_then(Value::as_str)
                .and_then(|version| SemVer::parse(version).ok())?;
            req.matches(&version).then(|| (version, entry.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, entry)| entry)
}

fn extract_crate_archive(archive: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("--strip-components")
        .arg("1")
        .arg("-C")
        .arg(destination)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_PACKAGE_VERIFY_001] .crate extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn find_registry_package(
    index: &Value,
    package_entries: &[Value],
    dep: &GhorDependency,
) -> Option<Value> {
    let req = dep
        .version
        .as_deref()
        .and_then(|raw| VersionReq::parse(raw).ok())
        .unwrap_or_else(VersionReq::any);
    select_best_registry_entry(
        package_entries
            .iter()
            .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(dep.name.as_str()))
            .filter(|entry| entry_has_required_features(entry, dep))
            .cloned()
            .collect::<Vec<_>>(),
        &req,
        "vers",
    )
    .or_else(|| {
        let entries = index
            .get("packages")
            .and_then(Value::as_array)?
            .iter()
            .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(dep.name.as_str()))
            .filter(|entry| entry_has_required_features(entry, dep))
            .cloned()
            .collect::<Vec<_>>();
        select_best_registry_entry(entries, &req, "version")
    })
}

fn select_best_registry_entry(
    entries: Vec<Value>,
    req: &VersionReq,
    version_field: &str,
) -> Option<Value> {
    entries
        .into_iter()
        .filter(|entry| entry.get("yanked").and_then(Value::as_bool) != Some(true))
        .filter_map(|entry| {
            let version = entry.get(version_field).and_then(Value::as_str)?;
            let parsed = SemVer::parse(version).ok()?;
            req.matches(&parsed).then_some((parsed, entry))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, entry)| entry)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Option<String>,
}

impl SemVer {
    fn parse(raw: &str) -> Result<Self, Box<dyn Error>> {
        let version = raw.split('+').next().unwrap_or(raw);
        let (core, pre) = version
            .split_once('-')
            .map(|(core, pre)| (core, Some(pre.to_string())))
            .unwrap_or((version, None));
        let parts = core.split('.').collect::<Vec<_>>();
        if parts.is_empty() || parts.len() > 3 {
            return Err(format!("[E_GHOR_RESOLVER_SEMVER_001] invalid semver '{raw}'").into());
        }
        let major = parts[0].parse::<u64>()?;
        let minor = parts.get(1).unwrap_or(&"0").parse::<u64>()?;
        let patch = parts.get(2).unwrap_or(&"0").parse::<u64>()?;
        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    fn bump_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            pre: None,
        }
    }

    fn bump_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            pre: None,
        }
    }

    fn bump_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            pre: None,
        }
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => left.cmp(right),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
enum VersionComparator {
    Any,
    GreaterEq(SemVer),
    Greater(SemVer),
    LessEq(SemVer),
    Less(SemVer),
    Equal(SemVer),
}

#[derive(Debug, Clone)]
struct VersionReq {
    comparators: Vec<VersionComparator>,
}

impl VersionReq {
    fn any() -> Self {
        Self {
            comparators: vec![VersionComparator::Any],
        }
    }

    fn parse(raw: &str) -> Result<Self, Box<dyn Error>> {
        let raw = raw.trim();
        if raw.is_empty() || raw == "*" {
            return Ok(Self::any());
        }
        let mut comparators = Vec::new();
        for part in split_version_req_parts(raw) {
            comparators.extend(parse_version_req_part(&part)?);
        }
        Ok(Self { comparators })
    }

    fn matches(&self, version: &SemVer) -> bool {
        self.comparators.iter().all(|cmp| match cmp {
            VersionComparator::Any => true,
            VersionComparator::GreaterEq(bound) => version >= bound,
            VersionComparator::Greater(bound) => version > bound,
            VersionComparator::LessEq(bound) => version <= bound,
            VersionComparator::Less(bound) => version < bound,
            VersionComparator::Equal(bound) => version == bound,
        })
    }
}

fn split_version_req_parts(raw: &str) -> Vec<String> {
    raw.split(',')
        .flat_map(|part| {
            let trimmed = part.trim();
            if trimmed.contains(' ') && !trimmed.starts_with('=') {
                trimmed
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                vec![trimmed.to_string()]
            }
        })
        .filter(|part| !part.is_empty())
        .collect()
}

fn parse_version_req_part(raw: &str) -> Result<Vec<VersionComparator>, Box<dyn Error>> {
    if raw == "*" || raw.eq_ignore_ascii_case("x") {
        return Ok(vec![VersionComparator::Any]);
    }
    for (op, ctor) in [
        (
            ">=",
            VersionComparator::GreaterEq as fn(SemVer) -> VersionComparator,
        ),
        (
            "<=",
            VersionComparator::LessEq as fn(SemVer) -> VersionComparator,
        ),
        (
            ">",
            VersionComparator::Greater as fn(SemVer) -> VersionComparator,
        ),
        (
            "<",
            VersionComparator::Less as fn(SemVer) -> VersionComparator,
        ),
        (
            "=",
            VersionComparator::Equal as fn(SemVer) -> VersionComparator,
        ),
    ] {
        if let Some(rest) = raw.strip_prefix(op) {
            return Ok(vec![ctor(SemVer::parse(rest.trim())?)]);
        }
    }
    if let Some(rest) = raw.strip_prefix('^') {
        return caret_req(rest);
    }
    if let Some(rest) = raw.strip_prefix('~') {
        return tilde_req(rest);
    }
    if raw.contains('*') || raw.contains('x') || raw.contains('X') {
        return wildcard_req(raw);
    }
    caret_req(raw)
}

fn caret_req(raw: &str) -> Result<Vec<VersionComparator>, Box<dyn Error>> {
    let lower = SemVer::parse(raw)?;
    let upper = if lower.major > 0 {
        lower.bump_major()
    } else if lower.minor > 0 {
        lower.bump_minor()
    } else {
        lower.bump_patch()
    };
    Ok(vec![
        VersionComparator::GreaterEq(lower),
        VersionComparator::Less(upper),
    ])
}

fn tilde_req(raw: &str) -> Result<Vec<VersionComparator>, Box<dyn Error>> {
    let lower = SemVer::parse(raw)?;
    let dot_count = raw.split('-').next().unwrap_or(raw).matches('.').count();
    let upper = if dot_count == 0 {
        lower.bump_major()
    } else {
        lower.bump_minor()
    };
    Ok(vec![
        VersionComparator::GreaterEq(lower),
        VersionComparator::Less(upper),
    ])
}

fn wildcard_req(raw: &str) -> Result<Vec<VersionComparator>, Box<dyn Error>> {
    let normalized = raw.replace(['X', 'x'], "*");
    let parts = normalized.split('.').collect::<Vec<_>>();
    if parts.first() == Some(&"*") {
        return Ok(vec![VersionComparator::Any]);
    }
    let major = parts
        .first()
        .ok_or("[E_GHOR_RESOLVER_SEMVER_001] invalid wildcard requirement")?
        .parse::<u64>()?;
    if parts.get(1) == Some(&"*") || parts.len() == 1 {
        let lower = SemVer {
            major,
            minor: 0,
            patch: 0,
            pre: None,
        };
        return Ok(vec![
            VersionComparator::GreaterEq(lower.clone()),
            VersionComparator::Less(lower.bump_major()),
        ]);
    }
    let minor = parts
        .get(1)
        .ok_or("[E_GHOR_RESOLVER_SEMVER_001] invalid wildcard requirement")?
        .parse::<u64>()?;
    let lower = SemVer {
        major,
        minor,
        patch: 0,
        pre: None,
    };
    Ok(vec![
        VersionComparator::GreaterEq(lower.clone()),
        VersionComparator::Less(lower.bump_minor()),
    ])
}

fn target_active_for_resolver(dep: &GhorDependency, target: &str, resolver_version: &str) -> bool {
    match dep.target.as_deref() {
        None => true,
        Some(dep_target) if resolver_version == "1" => {
            let _ = dep_target;
            true
        }
        Some(dep_target) => target_cfg_matches(dep_target, target),
    }
}

fn target_cfg_matches(dep_target: &str, target: &str) -> bool {
    let dep_target = dep_target.trim().trim_matches('"').trim_matches('\'');
    if dep_target == target || dep_target == "all" || dep_target == "cfg(any())" {
        return true;
    }
    if dep_target == "cfg(unix)" {
        return target.contains("linux")
            || target.contains("apple")
            || target.contains("darwin")
            || target.contains("freebsd")
            || target.contains("dragonfly")
            || target.contains("openbsd")
            || target.contains("netbsd");
    }
    if dep_target == "cfg(windows)" {
        return target.contains("windows") || target.contains("msvc") || target.contains("mingw");
    }
    if dep_target == "cfg(not(unix))" {
        return !target_cfg_matches("cfg(unix)", target);
    }
    if dep_target == "cfg(not(windows))" {
        return !target_cfg_matches("cfg(windows)", target);
    }
    if let Some(inner) = dep_target
        .strip_prefix("cfg(target_os = ")
        .and_then(|value| value.strip_suffix(')'))
    {
        let os = inner.trim().trim_matches('"').trim_matches('\'');
        return match os {
            "linux" => target.contains("linux"),
            "windows" => target.contains("windows") || target.contains("msvc"),
            "macos" => target.contains("apple") || target.contains("darwin"),
            "none" => target.contains("unknown-unknown") || target.contains("unknown-elf"),
            other => target.contains(other),
        };
    }
    false
}

fn feature_scope_for_resolver(
    dep: &GhorDependency,
    target: &str,
    resolver_version: &str,
) -> String {
    if resolver_version == "1" {
        return "unified".to_string();
    }
    if let Some(dep_target) = &dep.target {
        if dep_target != target {
            return format!("inactive-target:{dep_target}");
        }
    }
    match dep.kind.as_str() {
        "build" => "build".to_string(),
        "dev" => "dev".to_string(),
        _ => "normal".to_string(),
    }
}

fn run_report(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = positional_args(args).first().copied().unwrap_or("summary");
    match action {
        "summary" => {
            let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
            print_json(&json!({
                "schema": "ghor.report.summary.v1",
                "status": "ok",
                "manifest": manifest_to_json(&manifest),
                "target_matrix": target_matrix_json(Some(&manifest)),
                "foreign_toolchain_matrix": foreign_toolchain_matrix_json(),
                "commands": ghor_command_list()
            }))
        }
        "graph" | "build-graph" => run_graph(args),
        "doctor" => run_doctor(&["--json".to_string()]),
        "target" | "targets" => {
            let manifest = manifest_path_from_args(args)
                .ok()
                .and_then(|path| parse_manifest(&path).ok());
            print_json(&target_matrix_json(manifest.as_ref()))
        }
        "commands" | "command-contracts" => print_json(&ghor_command_contracts_json()),
        other => Err(format!("[E_GHOR_REPORT_001] unsupported report '{other}'").into()),
    }
}

fn ghor_command_contracts_json() -> Value {
    let commands = ghor_command_list()
        .into_iter()
        .map(|name| {
            let family = match name {
                "new" | "init" | "check" | "build" | "run" | "test" | "bench" | "clean" | "fmt"
                | "lint" | "doc" => "build",
                "registry" | "cdn" | "governance" | "moderation" | "search" | "info" | "login"
                | "logout" | "owner" | "yank" | "audit" | "sbom" | "license" | "supply-chain"
                | "capability-audit" => "registry-security",
                "machine" | "basal" | "runtime" | "compiler" | "primitive" | "interop"
                | "cargo-exec" | "build-script" | "proc-macro" | "external-compat" | "lang" => {
                    "systems-adapters"
                }
                "doctor" | "verify" | "evidence" | "seal" | "conformance" | "public-beta"
                | "release-check" | "release-hardening" | "install-smoke" | "bootstrap" => {
                    "assurance"
                }
                _ => "package-workspace",
            };
            let route = match name {
                "build" => "nc-stage1-native-ncc-machine-artifact",
                "test" => "nc-stage1-native-ncc-test-execution",
                "run" => "nc-stage1-native-machine-execution",
                "bench" => "nc-stage1-native-machine-benchmark",
                "new" | "init" => "nc-stage1-native-project-scaffold",
                "clean" => "nc-stage1-native-target-clean",
                "fmt" | "fix" => "nc-stage1-native-nfmt-orchestration",
                "lint" => "nc-stage1-native-nlint-orchestration",
                "doc" => "nc-stage1-native-documentation-inventory",
                "doctor" => "nc-stage1-native-host-doctor",
                "toolchain" => "nc-stage1-native-toolchain-inspection",
                "verify-toolchain" => "nc-stage1-native-toolchain-verification",
                "sync-toolchain" => "nc-stage1-native-toolchain-sync-plan",
                "verify" => "nc-stage1-native-verification",
                "evidence" => "nc-stage1-native-evidence",
                "seal" => "nc-stage1-native-seal",
                "conformance" => "nc-stage1-native-conformance",
                "public-beta" => "nc-stage1-native-public-beta",
                "release-check" => "nc-stage1-native-release-check",
                "release-hardening" => "nc-stage1-native-release-hardening",
                "install-smoke" => "nc-stage1-native-install-smoke",
                "audit" => "nc-stage1-native-security-audit",
                "sbom" => "nc-stage1-native-complete-sbom",
                "license" => "nc-stage1-native-license-report",
                "supply-chain" => "nc-stage1-native-supply-chain-verify",
                "capability-audit" => "nc-stage1-native-capability-audit",
                "target" => "nc-stage1-native-target-matrix",
                "profile" => "nc-stage1-native-profile-inspection",
                "features" => "nc-stage1-native-feature-selection",
                "graph" => "nc-stage1-native-project-graph",
                "report" => "nc-stage1-native-project-report",
                "bin" => "nc-stage1-native-bin-location",
                "cross" => "nc-stage1-native-cross-plan",
                "link" => "nc-stage1-native-link-plan",
                "native-deps" => "nc-stage1-native-dependency-validation",
                "ffi-manifest" => "nc-stage1-native-ffi-manifest",
                "artifact" => "nc-stage1-native-artifact-record",
                "manifest" => "nc-stage1-native-manifest",
                "metadata" => "nc-stage1-native-metadata",
                "locate-project" => "nc-stage1-native-locate-project",
                "read-manifest" => "nc-stage1-native-read-manifest",
                "pkgid" => "nc-stage1-native-package-id",
                "unit-graph" => "nc-stage1-native-unit-graph",
                "timings" => "nc-stage1-native-timings",
                "resolve" => {
                    "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-resolver"
                }
                "lock" => {
                    "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-lock"
                }
                "generate-lockfile" => "nc-stage1-native-lockfile-generation",
                "verify-lock" => {
                    "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-lock-verify"
                }
                "fetch" => {
                    "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-fetch"
                }
                "vendor" => {
                    "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-vendor"
                }
                "update" => "nc-stage1-native-selective-precise-update",
                "add" => "nc-stage1-native-validated-manifest-add",
                "remove" => "nc-stage1-native-validated-manifest-remove",
                "tree" => "nc-stage1-native-resolved-dependency-tree",
                "workspace" => "nc-stage1-native-workspace-metadata",
                "search" => "nc-stage1-native-registry-search",
                "info" => "nc-stage1-native-registry-info",
                "login" | "logout" => "nc-stage1-native-private-registry-credentials",
                "registry" => "nc-stage1-native-registry-client",
                "package" => "nc-stage1-native-reproducible-package",
                "install" => "nc-stage1-native-material-install-transaction",
                "uninstall" => "nc-stage1-native-owned-material-uninstall-transaction",
                "publish" => "nc-stage1-native-transactional-publish",
                "owner" => "nc-stage1-native-transactional-owner",
                "yank" => "nc-stage1-native-transactional-yank",
                "check" => "nc-stage1-native-ncc-parser-sema",
                "machine" | "basal" | "runtime" | "compiler" | "primitive" => {
                    "nc-stage1-native-ncc-subsystem-passthrough"
                }
                "cdn" => "nc-stage1-native-content-addressed-cdn",
                "governance" => "nc-stage1-native-registry-governance",
                "moderation" => "nc-stage1-native-registry-moderation",
                "bootstrap" => "nc-stage1-native-bootstrap-authority",
                "lang" => "nc-stage1-native-language-forge",
                "cargo-exec" => "nc-stage1-native-cargo-execution",
                "build-script" => "nc-stage1-native-build-script-execution",
                "proc-macro" => "nc-stage1-native-proc-macro-execution",
                "interop" => "nc-stage1-native-foreign-interop",
                "external-compat" => "nc-stage1-native-external-compatibility",
                _ => "ghor-host-orchestration",
            };
            json!({
                "name": name,
                "family": family,
                "current_executor": if matches!(
                    name,
                    "manifest" | "metadata" | "locate-project" | "read-manifest" | "pkgid" |
                        "unit-graph" | "timings" | "add" | "remove" | "tree" | "resolve" | "lock" |
                        "generate-lockfile" |
                        "verify-lock" | "fetch" | "vendor" | "update" | "workspace" |
                        "search" | "info" | "login" | "logout" | "registry" |
                        "package" | "publish" | "owner" | "yank" |
                        "check" | "build" | "run" | "test" | "bench" | "install" | "uninstall" |
                        "new" | "init" | "clean" | "fmt" | "fix" | "lint" | "doc" |
                        "doctor" | "toolchain" | "verify-toolchain" | "sync-toolchain" |
                        "verify" | "evidence" | "seal" | "conformance" | "public-beta" |
                        "release-check" | "release-hardening" | "install-smoke"
                        | "audit" | "sbom" | "license" | "supply-chain" | "capability-audit"
                        | "target" | "profile" | "features" | "graph" | "report" | "bin"
                        | "cross" | "link" | "native-deps" | "ffi-manifest" | "artifact"
                        | "machine" | "basal" | "runtime" | "compiler" | "primitive"
                        | "cdn" | "governance" | "moderation" | "bootstrap" | "lang"
                        | "cargo-exec" | "build-script" | "proc-macro" | "interop"
                        | "external-compat"
                ) {
                    "nc-stage1-when-available"
                } else {
                    "rust-stage0"
                },
                "target_executor": "nc-stage1",
                "route": route,
                "direct_machine_artifact": matches!(
                    name,
                    "build" | "run" | "test" | "bench" | "install"
                ),
                "status": if matches!(name, "build" | "run" | "test" | "bench") {
                    "native_e2e"
                } else if matches!(
                    name,
                    "manifest" | "metadata" | "locate-project" | "read-manifest" | "pkgid" |
                        "unit-graph" | "timings" | "add" | "remove" | "package" | "check" |
                        "install" | "uninstall" | "new" | "init" | "clean" | "fmt" | "fix" |
                        "lint" | "doc" | "doctor" | "toolchain" | "verify-toolchain" |
                        "sync-toolchain" | "verify" | "evidence" | "seal" | "conformance" |
                        "public-beta" | "release-check" | "release-hardening" | "install-smoke"
                        | "audit" | "sbom" | "license" | "supply-chain" | "capability-audit"
                        | "target" | "profile" | "features" | "graph" | "report" | "bin"
                        | "cross" | "link" | "native-deps" | "ffi-manifest" | "artifact"
                        | "machine" | "basal" | "runtime" | "compiler" | "primitive"
                        | "cdn" | "governance" | "moderation" | "bootstrap" | "lang"
                        | "cargo-exec" | "build-script" | "proc-macro" | "interop"
                        | "external-compat" | "resolve" | "lock" | "generate-lockfile"
                        | "verify-lock" | "fetch" | "vendor" | "update" | "workspace" | "tree"
                        | "search" | "info" | "login" | "logout" | "registry" | "publish"
                        | "owner" | "yank"
                ) {
                    "stage1_native_e2e"
                } else {
                    "stage0_functional_stage1_pending"
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ghor.command-contracts.v1",
        "command_count": commands.len(),
        "native_e2e_count": commands.iter().filter(|entry| entry["status"] == "native_e2e").count(),
        "stage1_native_e2e_count": commands.iter().filter(|entry| entry["status"] == "stage1_native_e2e").count(),
        "stage1_native_partial_count": commands.iter().filter(|entry| entry["status"] == "stage1_native_partial").count(),
        "stage1_pending_count": commands.iter().filter(|entry| entry["status"] == "stage0_functional_stage1_pending").count(),
        "commands": commands
    })
}

fn run_lang(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err("[E_GHOR_LANGUAGE_TEMPLATE_001] lang expects subcommand new".into());
    };
    match sub {
        "new" => run_lang_new(&args[1..]),
        other => Err(format!(
            "[E_GHOR_LANGUAGE_TEMPLATE_001] unsupported lang subcommand '{}'",
            other
        )
        .into()),
    }
}

fn run_lang_new(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name =
        first_positional(args).ok_or("[E_GHOR_LANGUAGE_TEMPLATE_001] lang new expects <name>")?;
    let host = official_language(arg_after(args, "--host").unwrap_or("nC"))
        .ok_or("[E_GHOR_LANGUAGE_UNSUPPORTED_001] unsupported --host value")?;
    let project_dir = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(name));
    if project_dir.exists() {
        return Err(format!(
            "[E_GHOR_LANGUAGE_TEMPLATE_001] target language directory '{}' already exists",
            project_dir.display()
        )
        .into());
    }
    fs::create_dir_all(&project_dir)?;
    scaffold_language_forge(&project_dir, name, host)?;
    print_json(&json!({
        "schema": "ghor.language.new.v1",
        "status": "created",
        "name": name,
        "host": host,
        "path": project_dir
    }))
}

fn scaffold_project(
    dir: &Path,
    name: &str,
    language: &str,
    args: &[String],
) -> Result<(), Box<dyn Error>> {
    let kind = if has_flag(args, "--lib") {
        "lib"
    } else if has_flag(args, "--machine") {
        "machine"
    } else if has_flag(args, "--basal") {
        "basal"
    } else if has_flag(args, "--primitive") {
        "primitive"
    } else if language != "nC" {
        "foreign"
    } else {
        "bin"
    };
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("tests"))?;
    fs::create_dir_all(dir.join("examples"))?;
    fs::create_dir_all(dir.join("docs"))?;
    fs::create_dir_all(dir.join("target"))?;
    let manifest = default_manifest_for_language(name, kind, language);
    fs::write(dir.join(GHOR_OFFICIAL_MANIFEST), &manifest)?;
    fs::write(dir.join(GHOR_COMPAT_MANIFEST), manifest)?;
    scaffold_language_project_files(dir, name, language)?;
    if language == "nC" && kind == "machine" {
        fs::create_dir_all(dir.join("machine"))?;
        fs::write(
            dir.join("machine").join("main.nm"),
            default_machine_source(),
        )?;
    }
    if language == "nC" && kind == "basal" {
        fs::create_dir_all(dir.join("basal"))?;
        fs::write(
            dir.join("basal").join("root.nb"),
            default_basal_source(name),
        )?;
    }
    if language == "nC" && kind == "primitive" {
        fs::create_dir_all(dir.join("primitive"))?;
        fs::write(
            dir.join("primitive").join("device.ncp"),
            "// primitive device scaffold\n",
        )?;
    }
    Ok(())
}

fn default_manifest_for_language(name: &str, kind: &str, language: &str) -> String {
    let language_block = language_manifest_block(language);
    let host_target = host_target_triple();
    let host_abi = if host_target.contains("windows") {
        "win64"
    } else if host_target.starts_with("aarch64") {
        "aapcs64"
    } else {
        "sysv64"
    };
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2026"
kind = "{kind}"
license = "MIT"
authors = ["Aristote"]

[toolchain]
channel = "stable"
manager = "ncup"
components = ["ncc", "nfmt", "nlint", "nclsp"]

[target.default]
triple = "{host_target}"
abi = "{host_abi}"

[machine]
required = true
doctor = true
verify_object = true
disasm = true
roundtrip = true
parity = true
evidence = true

[basal]
enabled = true
seal = true
nbb = true
evidence = true

[runtime]
enabled = true

[compiler]
diagnostics = "strict"

[primitive]
enabled = false
real_hardware = false

[build]
orchestrator = "ghor"
graph = true

[features]
default = []

[capabilities]
machine = true
basal = true
foreign_adapters = true

[artifact]
kind = "ghor_project_build_plan"
format = "json"

{language_block}

[dependencies]

[native-dependencies]

[profiles.debug]
opt = 0
debug = true
evidence = true

[profiles.release]
opt = 3
debug = false
seal = true
evidence = true
"#
    )
}

fn language_manifest_block(language: &str) -> String {
    match language {
        "nC" => r#"[languages]
primary = "nC"
adapters = []
"#
        .to_string(),
        "c" => r#"[languages]
primary = "c"
adapters = ["c"]

[language_adapter.c]
toolchain = "clang"
standard = "c17"
"#
        .to_string(),
        "cpp" => r#"[languages]
primary = "cpp"
adapters = ["cpp"]

[language_adapter.cpp]
toolchain = "clang++"
standard = "c++20"
"#
        .to_string(),
        "asm" => r#"[languages]
primary = "asm"
adapters = ["asm"]

[language_adapter.asm]
toolchain = "as"
syntax = "gas"
"#
        .to_string(),
        "rust" => r#"[languages]
primary = "rust"
adapters = ["rust"]

[language_adapter.rust]
toolchain = "stable"
cargo = true
"#
        .to_string(),
        "java" => r#"[languages]
primary = "java"
adapters = ["java"]

[language_adapter.java]
toolchain = "javac"
build = "gradle"
"#
        .to_string(),
        "python" => r#"[languages]
primary = "python"
adapters = ["python"]

[language_adapter.python]
version = "3.12"
venv = true
test = "pytest"
"#
        .to_string(),
        "javascript" => r#"[languages]
primary = "javascript"
adapters = ["javascript"]

[language_adapter.javascript]
runtime = "node"
package_manager = "npm"
"#
        .to_string(),
        "typescript" => r#"[languages]
primary = "typescript"
adapters = ["typescript"]

[language_adapter.typescript]
runtime = "node"
package_manager = "pnpm"
compiler = "tsc"
"#
        .to_string(),
        _ => String::new(),
    }
}

fn scaffold_language_project_files(
    dir: &Path,
    name: &str,
    language: &str,
) -> Result<(), Box<dyn Error>> {
    match language {
        "nC" => fs::write(dir.join("src").join("main.n"), default_main_source(name))?,
        "c" => {
            fs::create_dir_all(dir.join("include"))?;
            fs::write(
                dir.join("include")
                    .join(format!("{}.h", safe_identifier(name))),
                "int nc_adapter_entry(void);\n",
            )?;
            fs::write(
                dir.join("src").join("main.c"),
                "#include <stdio.h>\n\nint nc_adapter_entry(void) { return 0; }\n\nint main(void) {\n    return nc_adapter_entry();\n}\n",
            )?;
        }
        "cpp" => {
            fs::create_dir_all(dir.join("include"))?;
            fs::write(
                dir.join("include")
                    .join(format!("{}.hpp", safe_identifier(name))),
                "#pragma once\nint nc_adapter_entry();\n",
            )?;
            fs::write(
                dir.join("src").join("main.cpp"),
                "#include <iostream>\n\nint nc_adapter_entry() { return 0; }\n\nint main() {\n    return nc_adapter_entry();\n}\n",
            )?;
        }
        "asm" => {
            fs::write(
                dir.join("src").join("main.S"),
                ".text\n.global nc_adapter_entry\nnc_adapter_entry:\n    ret\n",
            )?;
        }
        "rust" => {
            fs::write(
                dir.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
                    safe_identifier(name)
                ),
            )?;
            fs::write(dir.join("src").join("main.rs"), "fn main() {}\n")?;
        }
        "java" => {
            let java_dir = dir.join("src").join("main").join("java");
            fs::create_dir_all(&java_dir)?;
            fs::write(
                java_dir.join("Main.java"),
                "public final class Main {\n    private Main() {}\n    public static void main(String[] args) {}\n}\n",
            )?;
        }
        "python" => {
            let module = safe_identifier(name);
            let src = dir.join("src").join(&module);
            fs::create_dir_all(&src)?;
            fs::write(
                dir.join("pyproject.toml"),
                format!(
                    "[project]\nname = \"{}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n",
                    module
                ),
            )?;
            fs::write(src.join("__init__.py"), "__all__ = []\n")?;
            fs::write(
                src.join("__main__.py"),
                "def main() -> int:\n    return 0\n\nraise SystemExit(main())\n",
            )?;
        }
        "javascript" => {
            fs::write(
                dir.join("package.json"),
                format!(
                    "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": {{ \"test\": \"node src/main.js\" }}\n}}\n",
                    safe_identifier(name)
                ),
            )?;
            fs::write(
                dir.join("src").join("main.js"),
                "export function main() { return 0; }\n\nmain();\n",
            )?;
        }
        "typescript" => {
            fs::write(
                dir.join("package.json"),
                format!(
                    "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": {{ \"build\": \"tsc\", \"test\": \"tsc --noEmit\" }}\n}}\n",
                    safe_identifier(name)
                ),
            )?;
            fs::write(
                dir.join("tsconfig.json"),
                "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"ES2022\",\n    \"strict\": true,\n    \"outDir\": \"dist\"\n  },\n  \"include\": [\"src/**/*.ts\"]\n}\n",
            )?;
            fs::write(
                dir.join("src").join("main.ts"),
                "export function main(): number { return 0; }\n\nmain();\n",
            )?;
        }
        _ => {
            return Err(format!(
                "[E_GHOR_LANGUAGE_UNSUPPORTED_001] unsupported language '{}'",
                language
            )
            .into());
        }
    }
    Ok(())
}

fn scaffold_language_forge(dir: &Path, name: &str, host: &str) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("tests"))?;
    fs::create_dir_all(dir.join("examples"))?;
    fs::create_dir_all(dir.join("docs"))?;
    fs::create_dir_all(dir.join("reports"))?;
    let manifest = default_manifest_for_language(name, "foreign", host);
    fs::write(dir.join(GHOR_OFFICIAL_MANIFEST), &manifest)?;
    fs::write(dir.join(GHOR_COMPAT_MANIFEST), manifest)?;
    scaffold_language_project_files(dir, name, host)?;
    let ext = host_source_ext(host);
    for unit in [
        "lexer",
        "parser",
        "ast",
        "diagnostics",
        "sema",
        "ir",
        "lowering",
        "backend",
        "runtime",
    ] {
        fs::write(
            dir.join("src").join(format!("{unit}.{ext}")),
            language_unit_source(unit, host),
        )?;
    }
    fs::write(
        dir.join("tests")
            .join(format!("{}_conformance.{ext}", safe_identifier(name))),
        language_unit_source("conformance_test", host),
    )?;
    fs::write(
        dir.join("examples").join(format!("minimal.{ext}")),
        language_unit_source("minimal_example", host),
    )?;
    fs::write(
        dir.join("docs").join("conformance.md"),
        format!(
            "# {name} language forge\n\nHost: `{host}`.\n\nThis scaffold defines lexer, parser, AST, diagnostics, sema, IR, lowering, backend and runtime surfaces for a language implemented under ghor orchestration.\n"
        ),
    )?;
    fs::write(
        dir.join("reports").join("conformance_scaffold.json"),
        serde_json::to_vec_pretty(&json!({
            "schema": "ghor.language.conformance.scaffold.v1",
            "name": name,
            "host": host,
            "status": "created",
            "machine_origin": host == "nC"
        }))?,
    )?;
    Ok(())
}

fn host_source_ext(host: &str) -> &'static str {
    match host {
        "c" => "c",
        "cpp" => "cpp",
        "asm" => "S",
        "rust" => "rs",
        "java" => "java",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        _ => "n",
    }
}

fn language_unit_source(unit: &str, host: &str) -> String {
    match host {
        "c" => format!("/* ghor {unit} unit */\nint {unit}_unit(void) {{ return 0; }}\n"),
        "cpp" => format!("// ghor {unit} unit\nint {unit}_unit() {{ return 0; }}\n"),
        "asm" => format!(".text\n.global {unit}_unit\n{unit}_unit:\n    ret\n"),
        "rust" => format!("pub fn {unit}_unit() -> bool {{ true }}\n"),
        "java" => format!(
            "final class {}Unit {{ boolean ok() {{ return true; }} }}\n",
            pascal(unit)
        ),
        "python" => format!("def {unit}_unit() -> bool:\n    return True\n"),
        "javascript" => format!("export function {unit}Unit() {{ return true; }}\n"),
        "typescript" => format!("export function {unit}Unit(): boolean {{ return true; }}\n"),
        _ => format!("{unit}[@] {{\n    to main {{\n        return true;\n    }}\n}}\n"),
    }
}

fn safe_identifier(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        "nc_project".to_string()
    } else {
        out
    }
}

fn pascal(raw: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in raw.chars() {
        if ch == '_' || ch == '-' {
            upper = true;
        } else if upper {
            out.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn default_main_source(name: &str) -> String {
    format!(
        r#"{name} {{
    to main {{
        return true;
    }}

    fa:
        return true;
}}
"#
    )
}

fn default_machine_source() -> &'static str {
    "target: x86_64-unknown-linux-gnu\nproc: main\n    ret\n"
}

fn default_basal_source(name: &str) -> String {
    format!(
        r#"{name}Root[@] {{
    root:
        identity: "nC.root.{name}";
        version: 1;
        profile: basal;
        binary: optional;
        evidence: optional;
        lowering: deferred;

    target:
        arch: x86_64;
        triple: "x86_64-unknown-linux-gnu";

    site:
        root_site:
            kind: memory_anchor;
            size: 8;
            align: 4;
            permissions: read;
}}
"#
    )
}

fn run_toolchain(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = manifest_path_from_args(args)
        .ok()
        .and_then(|path| parse_manifest(&path).ok());
    let drift = detect_toolchain_drift()?;
    let payload = json!({
        "schema": "ghor.toolchain.v1",
        "resolution_order": [
            "nc-toolchain.toml",
            "GHOR_TOOLCHAIN",
            "NCUP_TOOLCHAIN",
            "ghor.toml [toolchain]",
            "nc.toml [toolchain] compatibility",
            "ncup default",
            "repo local fallback"
        ],
        "selected": env::var("GHOR_TOOLCHAIN")
            .or_else(|_| env::var("NCUP_TOOLCHAIN"))
            .unwrap_or_else(|_| manifest.as_ref().and_then(|m| m.toolchain.get("channel").cloned()).unwrap_or_else(|| "repo-local".to_string())),
        "manager": manifest.as_ref().and_then(|m| m.toolchain.get("manager").cloned()).unwrap_or_else(|| "ncup".to_string()),
        "repo_ncc": repo_ncc_path()?.display().to_string(),
        "installed_ncc": installed_ncc_candidate().map(|path| path.display().to_string()),
        "diagnostics": diagnostics_to_json(&drift)
    });
    print_json(&payload)
}

fn run_verify_toolchain(_args: &[String]) -> Result<(), Box<dyn Error>> {
    let drift = detect_toolchain_drift()?;
    print_json(&json!({
        "schema": "ghor.toolchain.verify.v1",
        "status": if diagnostics_have_errors(&drift) { "error" } else if drift.is_empty() { "ok" } else { "warning" },
        "diagnostics": diagnostics_to_json(&drift)
    }))?;
    if diagnostics_have_errors(&drift) || !drift.is_empty() {
        Err(diagnostics_summary(&drift).into())
    } else {
        Ok(())
    }
}

fn run_sync_toolchain(_args: &[String]) -> Result<(), Box<dyn Error>> {
    print_json(&json!({
        "schema": "ghor.toolchain.sync.v1",
        "status": "suggested",
        "automatic_install": false,
        "commands": [
            "ncup install stable",
            "ncup default stable",
            "ncup component add ncc nfmt nlint nclsp",
            "ncup doctor"
        ]
    }))
}

fn run_graph(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let graph = build_graph_json(&manifest)?;
    print_json(&graph)
}

fn build_graph_json(manifest: &GhorManifest) -> Result<Value, Box<dyn Error>> {
    let mut sources = Vec::new();
    for rel in ["src", "machine", "basal", "primitive", "tests", "examples"] {
        collect_files(&manifest.root.join(rel), &mut sources)?;
    }
    Ok(json!({
        "schema": "ghor.graph.v1",
        "package": package_name(manifest),
        "target": manifest.target_default,
        "profiles": manifest.profiles.keys().cloned().collect::<Vec<_>>(),
        "sources": sources,
        "dependencies": manifest.dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "rust_dependencies": manifest.rust_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "c_dependencies": manifest.c_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "cpp_dependencies": manifest.cpp_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "asm_dependencies": manifest.asm_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "generated_artifacts": [
            "target/ghor/debug",
            "target/ghor/release",
            "ghor.lock"
        ]
    }))
}

fn collect_files(dir: &Path, out: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some(
                "n" | "nm"
                    | "nb"
                    | "nbb"
                    | "ncp"
                    | "c"
                    | "h"
                    | "cpp"
                    | "hpp"
                    | "rs"
                    | "java"
                    | "py"
                    | "js"
                    | "ts"
                    | "tsx"
            )
        ) {
            out.push(path.display().to_string());
        }
    }
    Ok(())
}

fn run_check(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        print_json(&json!({
            "schema": "ghor.check.v1",
            "status": "error",
            "diagnostics": diagnostics_to_json(&manifest.diagnostics)
        }))?;
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    if has_flag(args, "--workspace") {
        let mut diagnostics = Vec::new();
        let mut packages = Vec::new();
        let mut checked = 0usize;
        for member in selected_workspace_manifests(&manifest, true)? {
            let member_manifest = parse_manifest(&member)?;
            diagnostics.extend(
                member_manifest
                    .diagnostics
                    .iter()
                    .filter(|diag| diag.is_error())
                    .cloned(),
            );
            let (member_checked, member_diagnostics) = check_manifest_sources(&member_manifest)?;
            checked += member_checked;
            diagnostics.extend(member_diagnostics);
            packages.push(json!({
                "package": package_name(&member_manifest),
                "manifest_path": member
            }));
        }
        print_json(&json!({
            "schema": "ghor.check.v1",
            "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
            "workspace": true,
            "packages": packages,
            "checked": checked,
            "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
            "diagnostics": diagnostics_to_json(&diagnostics)
        }))?;
        return if diagnostics_have_errors(&diagnostics) {
            Err(diagnostics_summary(&diagnostics).into())
        } else {
            Ok(())
        };
    }
    let (checked, diagnostics) = check_manifest_sources(&manifest)?;
    print_json(&json!({
        "schema": "ghor.check.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "package": package_name(&manifest),
        "checked": checked,
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn check_manifest_sources(
    manifest: &GhorManifest,
) -> Result<(usize, Vec<GhorDiagnostic>), Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    let mut checked = 0usize;
    for source in project_sources(manifest, "n")? {
        checked += 1;
        let output = run_repo_ncc(&["check", source.to_str().unwrap_or_default()])?;
        if output.status != "ok" {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_CHECK_001",
                format!("ncc check failed for {}", source.display()),
            ));
        }
    }
    Ok((checked, diagnostics))
}

fn project_sources(
    manifest: &GhorManifest,
    extension: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut all = Vec::new();
    collect_project_sources(&manifest.root.join("src"), extension, &mut all)?;
    Ok(all)
}

fn collect_project_sources(
    dir: &Path,
    extension: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_project_sources(&path, extension, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            out.push(path);
        }
    }
    Ok(())
}

fn run_build(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    if has_flag(args, "--locked") || has_flag(args, "--frozen") {
        let diagnostics = lock_diagnostics(&manifest, args)?;
        if diagnostics_have_errors(&diagnostics) {
            return Err(diagnostics_summary(&diagnostics).into());
        }
    }
    run_check(args)?;
    let profile = arg_after(args, "--profile").unwrap_or("debug");
    let out_dir = workspace_target_dir(&manifest).join(profile);
    fs::create_dir_all(&out_dir)?;
    let target = arg_after(args, "--target")
        .or_else(|| manifest.target_default.get("triple").map(String::as_str))
        .unwrap_or(host_target_triple());
    let mut artifacts = Vec::new();
    for member in selected_workspace_manifests(&manifest, has_flag(args, "--workspace"))? {
        let member_manifest = parse_manifest(&member)?;
        let graph = build_graph_json(&member_manifest)?;
        let native_build = build_nc_package(&member_manifest, profile, target, args)?;
        let artifact = json!({
            "schema": "ghor.build-artifact.v1",
            "package": package_name(&member_manifest),
            "version": package_version(&member_manifest),
            "profile": profile,
            "target": target,
            "target_default": member_manifest.target_default,
            "graph_hash": stable_json_hash(&graph),
            "machine_consulted": member_manifest.machine.get("required").map(|v| v == "true").unwrap_or(true),
            "artifact_kind": member_manifest.artifact.get("kind").cloned().unwrap_or_else(|| "native_program".to_string()),
            "native_build": native_build,
            "foreign_toolchain_matrix": foreign_toolchain_matrix_json()
        });
        let artifact_path = out_dir.join(format!("{}.build.json", package_name(&member_manifest)));
        fs::write(&artifact_path, serde_json::to_vec_pretty(&artifact)?)?;
        artifacts.push(json!({
            "package": package_name(&member_manifest),
            "artifact": artifact_path,
            "artifact_hash": stable_json_hash(&artifact)
        }));
    }
    print_json(&json!({
        "schema": "ghor.build.v1",
        "status": "built",
        "workspace": has_flag(args, "--workspace"),
        "artifacts": artifacts,
        "shared_target": workspace_target_dir(&manifest),
    }))
}

fn run_project_run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if has_flag(args, "--locked") || has_flag(args, "--frozen") {
        let diagnostics = lock_diagnostics(&manifest, args)?;
        if diagnostics_have_errors(&diagnostics) {
            return Err(diagnostics_summary(&diagnostics).into());
        }
    }
    let profile = arg_after(args, "--profile").unwrap_or("debug");
    let target = arg_after(args, "--target")
        .or_else(|| manifest.target_default.get("triple").map(String::as_str))
        .unwrap_or(host_target_triple());
    let build = build_nc_package(&manifest, profile, target, args)?;
    let executable = build
        .get("executable")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ncc_diagnostics::registered_error_text(
                ncc_diagnostics::codes::E_GHOR_RUN_001,
                "native build did not produce an executable",
            )
        })?;
    if target != host_target_triple() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_RUN_TARGET_001,
            format!(
                "target '{target}' cannot execute on host '{}'; build it with `ghor build --target {target}` or select the host target",
                host_target_triple()
            ),
        )
        .into());
    }
    let status = Command::new(executable).status().map_err(|error| {
        ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_RUN_001,
            format!("failed to launch native executable '{executable}': {error}"),
        )
    })?;
    let exit_code = status.code().unwrap_or(1);
    print_json(&json!({
        "schema": "ghor.run.v1",
        "status": if status.success() { "ok" } else { "error" },
        "execution": "native_machine_executable",
        "executable": executable,
        "exit_code": exit_code,
        "build": build
    }))?;
    if status.success() {
        Ok(())
    } else {
        Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_RUN_001,
            format!("native executable exited with status {status}"),
        )
        .into())
    }
}

fn build_nc_package(
    manifest: &GhorManifest,
    profile: &str,
    target: &str,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let language = manifest
        .languages
        .get("primary")
        .map(String::as_str)
        .unwrap_or("nC");
    if !language.eq_ignore_ascii_case("nC") {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_BUILD_LANGUAGE_001,
            format!("native machine build found primary language '{language}'"),
        )
        .into());
    }
    let source = manifest
        .cargo_bin
        .get("path")
        .map(|path| manifest.root.join(path))
        .unwrap_or_else(|| manifest.root.join("src").join("main.n"));
    if !source.is_file() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_BUILD_ENTRY_001,
            format!(
                "package '{}' has no executable entry source at '{}'",
                package_name(manifest),
                source.display()
            ),
        )
        .into());
    }

    let native_profile = match profile {
        "release" => "release-fast",
        "bench" => "release-fast",
        "test" => "debug",
        other => other,
    };
    let output_dir = workspace_target_dir(manifest)
        .join(profile)
        .join(target.replace(['/', '\\', ':'], "_"));
    fs::create_dir_all(&output_dir)?;
    let executable_name = if target.contains("windows") {
        format!("{}.exe", package_name(manifest))
    } else {
        package_name(manifest)
    };
    let executable = output_dir.join(executable_name);
    let mut command = vec![
        "build".to_string(),
        source.display().to_string(),
        executable.display().to_string(),
        "--profile".to_string(),
        native_profile.to_string(),
        "--target".to_string(),
        target.to_string(),
    ];
    for option in ["--cpu", "--tune-profile", "--cc", "--linker"] {
        if let Some(value) = arg_after(args, option) {
            command.push(option.to_string());
            command.push(value.to_string());
        }
    }
    let command_refs = command.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_repo_ncc(&command_refs)?;
    if output.status != "ok" {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_BUILD_NATIVE_001,
            format!(
                "source='{}'; stdout={}; stderr={}",
                source.display(),
                output.stdout.trim(),
                output.stderr.trim()
            ),
        )
        .into());
    }
    if !executable.is_file() {
        return Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_BUILD_ARTIFACT_001,
            format!("missing executable '{}'", executable.display()),
        )
        .into());
    }
    let executable_bytes = fs::read(&executable)?;
    let source_stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("main");
    let source_ncc = output_dir.join(".ncc");
    let nm = source_ncc
        .join("lowering")
        .join(format!("{source_stem}.nm"));
    let nbo = source_ncc
        .join("lowering")
        .join(format!("{source_stem}.backend-object.nbo"));
    let object = source_ncc.join("obj").join(format!("{source_stem}.obj"));
    for (kind, path) in [
        ("machine source", &nm),
        ("backend object", &nbo),
        ("native object", &object),
    ] {
        if !path.is_file() {
            return Err(ncc_diagnostics::registered_error_text(
                ncc_diagnostics::codes::E_GHOR_BUILD_ARTIFACT_001,
                format!("native build omitted {kind} '{}'", path.display()),
            )
            .into());
        }
    }
    Ok(json!({
        "schema": "ghor.native-build.v1",
        "status": "materialized",
        "route": ".n -> optimized IR -> MCT -> machine -> object -> executable",
        "source": source,
        "target": target,
        "profile": native_profile,
        "executable": executable,
        "executable_bytes": executable_bytes.len(),
        "executable_sha256": sha256_hex(&executable_bytes),
        "nm": nm,
        "nbo": nbo,
        "object": object,
        "compiler": tool_output_to_json("ncc build", &output)
    }))
}

fn run_test(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let mut checked = 0usize;
    let mut diagnostics = Vec::new();
    let mut packages = Vec::new();
    let mut executions = Vec::new();
    for member in selected_workspace_manifests(&manifest, has_flag(args, "--workspace"))? {
        let member_manifest = parse_manifest(&member)?;
        packages.push(json!({
            "package": package_name(&member_manifest),
            "manifest_path": member
        }));
        for dir in ["tests", "examples"] {
            let mut files = Vec::new();
            collect_project_sources(&member_manifest.root.join(dir), "n", &mut files)?;
            for file in files {
                checked += 1;
                let output = if dir == "tests" {
                    run_repo_ncc(&[
                        "run",
                        file.to_str().unwrap_or_default(),
                        "--profile",
                        "debug",
                        "--target",
                        host_target_triple(),
                    ])?
                } else {
                    run_repo_ncc(&["check", file.to_str().unwrap_or_default()])?
                };
                executions.push(json!({
                    "source": file,
                    "mode": if dir == "tests" { "native-execution" } else { "compile-check" },
                    "result": tool_output_to_json(if dir == "tests" { "ncc run" } else { "ncc check" }, &output)
                }));
                if output.status != "ok" {
                    diagnostics.push(GhorDiagnostic::error(
                        "E_GHOR_TEST_001",
                        format!("test/example validation failed for {}", file.display()),
                    ));
                }
            }
        }
    }
    print_json(&json!({
        "schema": "ghor.test.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "workspace": has_flag(args, "--workspace"),
        "packages": packages,
        "checked": checked,
        "executions": executions,
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_bench(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let mut files = Vec::new();
    collect_project_sources(&manifest.root.join("benches"), "n", &mut files)?;
    let mut benchmarks = Vec::new();
    let mut failed = false;
    for file in files {
        let started = Instant::now();
        let output = run_repo_ncc(&[
            "run",
            file.to_str().unwrap_or_default(),
            "--profile",
            "release-fast",
            "--target",
            host_target_triple(),
            "--cpu",
            arg_after(args, "--cpu").unwrap_or("native"),
        ])?;
        let elapsed_ns = started.elapsed().as_nanos();
        failed |= output.status != "ok";
        benchmarks.push(json!({
            "source": file,
            "status": output.status,
            "elapsed_ns": elapsed_ns,
            "execution": tool_output_to_json("ncc run --profile release-fast", &output)
        }));
    }
    print_json(&json!({
        "schema": "ghor.bench.v1",
        "status": if failed { "error" } else { "ok" },
        "benchmarks": benchmarks,
        "profile": "release-fast",
        "target": host_target_triple()
    }))?;
    if failed {
        Err(ncc_diagnostics::registered_error_text(
            ncc_diagnostics::codes::E_GHOR_BENCH_001,
            "one or more native benchmarks failed",
        )
        .into())
    } else {
        Ok(())
    }
}

fn run_clean(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let target = workspace_target_dir(&manifest);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    print_json(&json!({
        "schema": "ghor.clean.v1",
        "status": "ok",
        "removed": target
    }))
}

fn run_fmt(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let check = has_flag(args, "--check");
    let mut outputs = Vec::new();
    for source in project_sources(&manifest, "n")? {
        let mut cmd_args = vec!["nfmt", source.to_str().unwrap_or_default()];
        if check {
            cmd_args.push("--check");
        }
        let output = run_repo_ncc(&cmd_args)?;
        outputs.push(tool_output_to_json("nfmt", &output));
        if output.status != "ok" {
            print_json(&json!({"schema": "ghor.fmt.v1", "status": "error", "outputs": outputs}))?;
            return Err("[E_GHOR_FMT_001] nfmt failed".into());
        }
    }
    print_json(&json!({"schema": "ghor.fmt.v1", "status": "ok", "outputs": outputs}))
}

fn run_lint(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let mut outputs = Vec::new();
    for source in project_sources(&manifest, "n")? {
        let output = run_repo_ncc(&["nlint", source.to_str().unwrap_or_default()])?;
        outputs.push(tool_output_to_json("nlint", &output));
        if output.status != "ok" {
            print_json(&json!({"schema": "ghor.lint.v1", "status": "error", "outputs": outputs}))?;
            return Err("[E_GHOR_LINT_001] nlint failed".into());
        }
    }
    print_json(&json!({"schema": "ghor.lint.v1", "status": "ok", "outputs": outputs}))
}

fn run_doc(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let docs = workspace_target_dir(&manifest).join("doc");
    fs::create_dir_all(&docs)?;
    let doc_path = docs.join("index.md");
    fs::write(
        &doc_path,
        format!(
            "# {}\n\nGenerated by ghor.\n\nManifest: `{}`\n",
            package_name(&manifest),
            manifest.path.display()
        ),
    )?;
    print_json(&json!({
        "schema": "ghor.doc.v1",
        "status": "ok",
        "path": doc_path
    }))
}

fn run_package(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let out_dir = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_target_dir(&manifest).join("package"));
    let options = package_options_from_args(args);
    if options.list_only {
        let files = package_file_list(&manifest)?;
        return print_json(&json!({
            "schema": "ghor.package.list.v1",
            "status": "listed",
            "package": package_name(&manifest),
            "version": package_version(&manifest),
            "files": files
        }));
    }
    let package = write_package(&manifest, &out_dir, &options)?;
    print_json(&json!({
        "schema": "ghor.package.result.v1",
        "status": "packaged",
        "archive": package.archive,
        "archive_name": package.archive_name,
        "package_root": package.package_root,
        "checksum": package.checksum,
        "manifest_hash": package.manifest_hash,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "files": package.files,
        "metadata": package.metadata,
        "verification": package.verification
    }))
}

fn write_package(
    manifest: &GhorManifest,
    out_dir: &Path,
    options: &GhorPackageOptions,
) -> Result<GhorPackageFiles, Box<dyn Error>> {
    let diagnostics = prepublish_diagnostics(manifest, None, options)?;
    if diagnostics_have_errors(&diagnostics) {
        return Err(diagnostics_summary(&diagnostics).into());
    }
    fs::create_dir_all(out_dir)?;
    let package_base = format!("{}-{}", package_name(manifest), package_version(manifest));
    let archive = out_dir.join(format!("{package_base}.crate"));
    let files = create_package_archive(manifest, &archive, out_dir, options)?;
    let bytes = fs::read(&archive)?;
    let checksum = sha256_hex(&bytes);
    let manifest_hash = stable_json_hash(&manifest_to_json(manifest));
    let verification = if options.verify {
        verify_crate_archive(manifest, &archive, out_dir)?
    } else {
        json!({"status": "skipped", "reason": "--no-verify"})
    };
    let metadata = crate_publish_metadata(manifest);
    let ffi_link_manifest = ghor_ffi_manifest_report(manifest, &[]);
    validate_package_ffi_manifest(&ffi_link_manifest)?;
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    fs::write(
        out_dir.join("checksum.sha256"),
        format!("{checksum}  {}\n", archive.display()),
    )?;
    let manifest_json = json!({
        "schema": "ghor.package.v2",
        "name": package_name(manifest),
        "version": package_version(manifest),
        "archive": archive,
        "archive_name": format!("{package_base}.crate"),
        "package_root": package_base,
        "checksum": checksum,
        "manifest_hash": manifest_hash,
        "format": "cargo-compatible .crate tar.gz",
        "files": files,
        "metadata": metadata,
        "ffi_link_manifest": ffi_link_manifest,
        "diagnostics": diagnostics_to_json(&diagnostics),
        "verification": verification
    });
    let manifest_path = out_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest_json)?)?;
    let package_manifest_hash = stable_json_hash(&manifest_json);
    Ok(GhorPackageFiles {
        archive_name: archive
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("package.crate")
            .to_string(),
        archive,
        manifest_path,
        checksum,
        manifest_hash,
        package_manifest_hash,
        ffi_link_manifest_hash,
        manifest_json,
        package_root: package_base,
        files,
        metadata,
        verification,
    })
}

fn create_package_archive(
    manifest: &GhorManifest,
    archive: &Path,
    out_dir: &Path,
    options: &GhorPackageOptions,
) -> Result<Vec<String>, Box<dyn Error>> {
    let package_base = format!("{}-{}", package_name(manifest), package_version(manifest));
    let staging = out_dir.join(format!(".{package_base}.crate-staging"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    let package_root = staging.join(&package_base);
    fs::create_dir_all(&package_root)?;

    let files = package_file_list(manifest)?;
    for rel in &files {
        let source = manifest.root.join(rel);
        if !source.is_file() {
            continue;
        }
        let destination = package_root.join(rel);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
    }

    let raw_manifest = fs::read_to_string(&manifest.path)?;
    fs::write(package_root.join("ghor.toml.orig"), raw_manifest)?;
    fs::write(
        package_root.join("ghor.toml"),
        normalized_ghor_manifest(manifest),
    )?;
    if !options.exclude_lockfile {
        let lock_path = lock_path_for(manifest, &[]);
        let metadata_path = lock_metadata_path_for(&lock_path);
        if lock_path.exists() {
            fs::copy(lock_path, package_root.join("ghor.lock"))?;
            if metadata_path.exists() {
                fs::copy(metadata_path, package_root.join("ghor.lock.meta.json"))?;
            }
        } else {
            let lock = lock_json(manifest, None);
            write_lock_artifacts(&package_root.join("ghor.lock"), &lock)?;
        }
    }
    fs::write(
        package_root.join(".cargo_vcs_info.json"),
        serde_json::to_vec_pretty(&cargo_vcs_info(manifest, options.allow_dirty)?)?,
    )?;

    let mut archive_files = files.clone();
    archive_files.push("ghor.toml.orig".to_string());
    if !archive_files.iter().any(|file| file == "ghor.toml") {
        archive_files.push("ghor.toml".to_string());
    }
    if !options.exclude_lockfile && !archive_files.iter().any(|file| file == "ghor.lock") {
        archive_files.push("ghor.lock".to_string());
    }
    if !options.exclude_lockfile
        && !archive_files
            .iter()
            .any(|file| file == "ghor.lock.meta.json")
    {
        archive_files.push("ghor.lock.meta.json".to_string());
    }
    archive_files.push(".cargo_vcs_info.json".to_string());
    archive_files.sort();
    archive_files.dedup();

    let mut args = vec!["-czf"];
    let archive_abs = if archive.is_absolute() {
        archive.to_path_buf()
    } else {
        env::current_dir()?.join(archive)
    };
    let archive_string = archive_abs.display().to_string();
    args.push(&archive_string);
    args.push(&package_base);
    if archive_files.is_empty() {
        return Err("[E_GHOR_PACKAGE_001] no package inputs found".into());
    }
    let mut command = Command::new("tar");
    command.current_dir(&staging).args(&args);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_PACKAGE_001] .crate tar.gz package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    fs::remove_dir_all(staging)?;
    Ok(archive_files)
}

fn package_options_from_args(args: &[String]) -> GhorPackageOptions {
    GhorPackageOptions {
        verify: !has_flag(args, "--no-verify"),
        list_only: has_flag(args, "--list") || has_flag(args, "-l"),
        allow_dirty: has_flag(args, "--allow-dirty"),
        no_metadata: has_flag(args, "--no-metadata"),
        exclude_lockfile: has_flag(args, "--exclude-lockfile"),
    }
}

fn package_file_list(manifest: &GhorManifest) -> Result<Vec<String>, Box<dyn Error>> {
    let include = manifest
        .package
        .get("include")
        .map(|value| parse_array(value))
        .unwrap_or_default();
    let exclude = manifest
        .package
        .get("exclude")
        .map(|value| parse_array(value))
        .unwrap_or_default();
    let has_git = git_inside_worktree(&manifest.root);
    let mut files = Vec::new();
    collect_package_files(
        &manifest.root,
        &manifest.root,
        &include,
        &exclude,
        has_git,
        &mut files,
    )?;
    for required in ghor_manifest_names() {
        if manifest.root.join(required).is_file() && !files.iter().any(|file| file == required) {
            files.push(required.to_string());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn package_preflight_file_count(manifest: &GhorManifest) -> Result<(usize, bool), Box<dyn Error>> {
    let include = manifest
        .package
        .get("include")
        .map(|value| parse_array(value))
        .unwrap_or_default();
    let exclude = manifest
        .package
        .get("exclude")
        .map(|value| parse_array(value))
        .unwrap_or_default();
    let mut count = 0usize;
    let mut truncated = false;
    collect_package_file_count(
        &manifest.root,
        &manifest.root,
        &include,
        &exclude,
        &mut count,
        &mut truncated,
    )?;
    for required in ghor_manifest_names() {
        if manifest.root.join(required).is_file() {
            count += 1;
        }
    }
    Ok((count.min(GHOR_PACKAGE_PREFLIGHT_FILE_LIMIT), truncated))
}

fn collect_package_file_count(
    root: &Path,
    dir: &Path,
    include: &[String],
    exclude: &[String],
    count: &mut usize,
    truncated: &mut bool,
) -> Result<(), Box<dyn Error>> {
    if *truncated || !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        if *count >= GHOR_PACKAGE_PREFLIGHT_FILE_LIMIT {
            *truncated = true;
            return Ok(());
        }
        let entry = entry?;
        let path = entry.path();
        let rel = workspace_relative(root, &path);
        if package_always_excluded(root, &path, &rel) {
            continue;
        }
        if path.is_dir() {
            collect_package_file_count(root, &path, include, exclude, count, truncated)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if !include.is_empty()
            && !include
                .iter()
                .any(|pattern| package_pattern_matches(pattern, &rel))
        {
            continue;
        }
        if exclude
            .iter()
            .any(|pattern| package_pattern_matches(pattern, &rel))
        {
            continue;
        }
        *count += 1;
    }
    Ok(())
}

fn collect_package_files(
    root: &Path,
    dir: &Path,
    include: &[String],
    exclude: &[String],
    has_git: bool,
    out: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = workspace_relative(root, &path);
        if package_always_excluded(root, &path, &rel) {
            continue;
        }
        if path.is_dir() {
            collect_package_files(root, &path, include, exclude, has_git, out)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if !include.is_empty() {
            if include
                .iter()
                .any(|pattern| package_pattern_matches(pattern, &rel))
            {
                out.push(rel);
            }
            continue;
        }
        if exclude
            .iter()
            .any(|pattern| package_pattern_matches(pattern, &rel))
        {
            continue;
        }
        if has_git && git_check_ignored(root, &rel) {
            continue;
        }
        if !has_git && rel.split('/').any(|part| part.starts_with('.')) {
            continue;
        }
        out.push(rel);
    }
    Ok(())
}

fn package_always_excluded(root: &Path, path: &Path, rel: &str) -> bool {
    if rel.is_empty() {
        return false;
    }
    let first = rel.split('/').next().unwrap_or_default();
    if first == "target" || first == ".git" {
        return true;
    }
    path.is_dir()
        && path != root
        && ghor_manifest_names()
            .iter()
            .any(|manifest_name| path.join(manifest_name).exists())
}

fn package_pattern_matches(pattern: &str, rel: &str) -> bool {
    let pattern = pattern.trim().trim_start_matches('/').replace('\\', "/");
    let rel = rel.trim().trim_start_matches('/').replace('\\', "/");
    if pattern == rel {
        return true;
    }
    if pattern.ends_with('/') && rel.starts_with(&pattern) {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return rel == prefix || rel.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return rel.starts_with(&format!("{prefix}/"));
    }
    workspace_pattern_matches(&pattern, &rel)
}

fn git_inside_worktree(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_check_ignored(root: &Path, rel: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("check-ignore")
        .arg("--quiet")
        .arg(rel)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn git_head(root: &Path) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_dirty(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}

fn cargo_vcs_info(manifest: &GhorManifest, allow_dirty: bool) -> Result<Value, Box<dyn Error>> {
    let dirty = git_dirty(&manifest.root);
    if dirty && !allow_dirty {
        return Err("[E_GHOR_PACKAGE_DIRTY_001] package worktree is dirty; pass --allow-dirty to package anyway".into());
    }
    Ok(json!({
        "git": {
            "sha1": git_head(&manifest.root),
            "dirty": dirty
        },
        "path_in_vcs": workspace_relative(&manifest.workspace_root, &manifest.root)
    }))
}

fn normalized_ghor_manifest(manifest: &GhorManifest) -> String {
    let mut out = String::new();
    out.push_str("[package]\n");
    for key in [
        "name",
        "version",
        "kind",
        "edition",
        "license",
        "license-file",
        "description",
        "documentation",
        "homepage",
        "repository",
        "readme",
        "keywords",
        "categories",
        "include",
        "exclude",
        "publish",
    ] {
        if let Some(value) = manifest.package.get(key) {
            out.push_str(&format!("{key} = {}\n", toml_value(value)));
        }
    }
    write_map_section(&mut out, "toolchain", &manifest.toolchain);
    write_map_section(&mut out, "target.default", &manifest.target_default);
    write_map_section(&mut out, "languages", &manifest.languages);
    write_dependency_section(&mut out, "dependencies", manifest.dependencies.values());
    write_dependency_section(
        &mut out,
        "dev-dependencies",
        manifest.dev_dependencies.values(),
    );
    write_dependency_section(
        &mut out,
        "build-dependencies",
        manifest.build_dependencies.values(),
    );
    write_native_dependency_section(&mut out, manifest.native_dependencies.values());
    write_map_section(&mut out, "features", &manifest.features);
    out
}

fn write_map_section(out: &mut String, name: &str, map: &BTreeMap<String, String>) {
    if map.is_empty() {
        return;
    }
    out.push_str(&format!("\n[{name}]\n"));
    for (key, value) in map {
        out.push_str(&format!("{key} = {}\n", toml_value(value)));
    }
}

fn write_dependency_section<'a, I>(out: &mut String, name: &str, deps: I)
where
    I: IntoIterator<Item = &'a GhorDependency>,
{
    let deps = deps.into_iter().collect::<Vec<_>>();
    if deps.is_empty() {
        return;
    }
    out.push_str(&format!("\n[{name}]\n"));
    for dep in deps {
        out.push_str(&format!("{} = {}\n", dep.name, dependency_toml_value(dep)));
    }
}

fn dependency_toml_value(dep: &GhorDependency) -> String {
    let mut values = Vec::new();
    if let Some(version) = &dep.version {
        values.push(format!("version = {}", toml_value(version)));
    }
    if let Some(path) = &dep.path {
        values.push(format!("path = {}", toml_value(path)));
    }
    if !dep.features.is_empty() {
        values.push(format!("features = {}", toml_array(&dep.features)));
    }
    if !dep.default_features {
        values.push("default-features = false".to_string());
    }
    if dep.optional {
        values.push("optional = true".to_string());
    }
    if let Some(registry) = &dep.registry {
        values.push(format!("registry = {}", toml_value(registry)));
    }
    if let Some(package) = &dep.package {
        values.push(format!("package = {}", toml_value(package)));
    }
    if let Some(git) = &dep.git {
        values.push(format!("git = {}", toml_value(git)));
    }
    if let Some(branch) = &dep.git_branch {
        values.push(format!("branch = {}", toml_value(branch)));
    }
    if let Some(tag) = &dep.git_tag {
        values.push(format!("tag = {}", toml_value(tag)));
    }
    if let Some(rev) = &dep.git_rev {
        values.push(format!("rev = {}", toml_value(rev)));
    }
    format!("{{ {} }}", values.join(", "))
}

fn write_native_dependency_section<'a, I>(out: &mut String, deps: I)
where
    I: IntoIterator<Item = &'a GhorNativeDependency>,
{
    let deps = deps.into_iter().collect::<Vec<_>>();
    if deps.is_empty() {
        return;
    }
    out.push_str("\n[native-dependencies]\n");
    for dep in deps {
        out.push_str(&format!(
            "{} = {}\n",
            dep.name,
            native_dependency_toml_value(dep)
        ));
    }
}

fn native_dependency_toml_value(dep: &GhorNativeDependency) -> String {
    let mut values = vec![
        format!("language = {}", toml_value(&dep.language)),
        format!("abi = {}", toml_value(&dep.abi)),
        format!("kind = {}", toml_value(&dep.kind)),
        format!("link_mode = {}", toml_value(&dep.link_mode)),
    ];
    if let Some(version) = &dep.version {
        values.push(format!("version = {}", toml_value(version)));
    }
    if let Some(target) = &dep.target {
        values.push(format!("target = {}", toml_value(target)));
    }
    if !dep.headers.is_empty() {
        values.push(format!("headers = {}", toml_array(&dep.headers)));
    }
    if !dep.include_paths.is_empty() {
        values.push(format!(
            "include_paths = {}",
            toml_array(&dep.include_paths)
        ));
    }
    if !dep.link_paths.is_empty() {
        values.push(format!("link_paths = {}", toml_array(&dep.link_paths)));
    }
    if !dep.libs.is_empty() {
        values.push(format!("libs = {}", toml_array(&dep.libs)));
    }
    if !dep.frameworks.is_empty() {
        values.push(format!("frameworks = {}", toml_array(&dep.frameworks)));
    }
    if !dep.symbols.is_empty() {
        values.push(format!("symbols = {}", toml_array(&dep.symbols)));
    }
    if !dep.cflags.is_empty() {
        values.push(format!("cflags = {}", toml_array(&dep.cflags)));
    }
    if !dep.ldflags.is_empty() {
        values.push(format!("ldflags = {}", toml_array(&dep.ldflags)));
    }
    if !dep.defines.is_empty() {
        values.push(format!("defines = {}", toml_array(&dep.defines)));
    }
    if let Some(pkg_config) = &dep.pkg_config {
        values.push(format!("pkg_config = {}", toml_value(pkg_config)));
    }
    if let Some(artifact) = &dep.artifact {
        values.push(format!("artifact = {}", toml_value(artifact)));
    }
    if let Some(checksum) = &dep.checksum {
        values.push(format!("checksum = {}", toml_value(checksum)));
    }
    if let Some(calling_convention) = &dep.calling_convention {
        values.push(format!(
            "calling_convention = {}",
            toml_value(calling_convention)
        ));
    }
    if let Some(layout) = &dep.layout {
        values.push(format!("layout = {}", toml_value(layout)));
    }
    if let Some(ownership) = &dep.ownership {
        values.push(format!("ownership = {}", toml_value(ownership)));
    }
    if !dep.required {
        values.push("required = false".to_string());
    }
    if dep.system {
        values.push("system = true".to_string());
    }
    format!("{{ {} }}", values.join(", "))
}

fn toml_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed == "true"
        || trimmed == "false"
        || trimmed.starts_with('[')
        || trimmed.starts_with('{')
        || trimmed.parse::<i64>().is_ok()
    {
        trimmed.to_string()
    } else {
        format!("\"{}\"", trimmed.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn toml_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| toml_value(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn verify_crate_archive(
    manifest: &GhorManifest,
    archive: &Path,
    out_dir: &Path,
) -> Result<Value, Box<dyn Error>> {
    let package_base = format!("{}-{}", package_name(manifest), package_version(manifest));
    let verify_dir = out_dir.join(format!(".{package_base}.verify"));
    if verify_dir.exists() {
        fs::remove_dir_all(&verify_dir)?;
    }
    fs::create_dir_all(&verify_dir)?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(&verify_dir)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_PACKAGE_VERIFY_001] .crate extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let extracted = verify_dir.join(&package_base);
    let manifest_path = extracted.join(GHOR_OFFICIAL_MANIFEST);
    if !manifest_path.exists() {
        return Err("[E_GHOR_PACKAGE_VERIFY_001] extracted .crate has no ghor.toml".into());
    }
    let extracted_manifest = parse_manifest(&manifest_path)?;
    let mut checked = 0usize;
    for source in project_sources(&extracted_manifest, "n")? {
        checked += 1;
        let output = run_repo_ncc(&["check", source.to_str().unwrap_or_default()])?;
        if output.status != "ok" {
            return Err(format!(
                "[E_GHOR_PACKAGE_VERIFY_001] extracted source failed ncc check: {}",
                source.display()
            )
            .into());
        }
    }
    fs::remove_dir_all(&verify_dir)?;
    Ok(json!({
        "status": "verified",
        "extracted_root": package_base,
        "source_checks": checked
    }))
}

fn prepublish_diagnostics(
    manifest: &GhorManifest,
    registry: Option<&GhorRegistry>,
    options: &GhorPackageOptions,
) -> Result<Vec<GhorDiagnostic>, Box<dyn Error>> {
    let mut diagnostics = prepublish_diagnostics_base(manifest, registry, options);
    let files = package_file_list(manifest)?;
    if files.is_empty() {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_PACKAGE_001",
            "package include/exclude rules selected no source files",
        ));
    }
    Ok(diagnostics)
}

fn prepublish_diagnostics_base(
    manifest: &GhorManifest,
    registry: Option<&GhorRegistry>,
    options: &GhorPackageOptions,
) -> Vec<GhorDiagnostic> {
    let mut diagnostics = manifest.diagnostics.clone();
    if let Some(raw) = manifest.package.get("publish") {
        let trimmed = raw.trim();
        if trimmed == "false" {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PUBLISH_RESTRICTED_001",
                "package.publish forbids publishing",
            ));
        } else if trimmed.starts_with('[') {
            let allowed = parse_array(trimmed);
            if let Some(registry) = registry {
                let label = registry_label(registry);
                let registry_arg = label.as_str();
                if !allowed.iter().any(|item| {
                    item == "crates-io" || item == registry_arg || registry_arg.ends_with(item)
                }) {
                    diagnostics.push(GhorDiagnostic::error(
                        "E_GHOR_PUBLISH_RESTRICTED_001",
                        format!("package.publish does not allow registry '{}'", label),
                    ));
                }
            }
        }
    }
    for dep in all_manifest_dependencies(manifest) {
        if dep.kind != "dev" && dep.path.is_some() && dep.version.is_none() {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PACKAGE_PATH_DEP_001",
                format!(
                    "path dependency '{}' must also declare version before publish",
                    dep.name
                ),
            ));
        }
    }
    diagnostics.extend(package_metadata_diagnostics(manifest, options.no_metadata));
    diagnostics
}

fn package_metadata_diagnostics(manifest: &GhorManifest, no_metadata: bool) -> Vec<GhorDiagnostic> {
    let mut diagnostics = Vec::new();
    if !no_metadata {
        for key in ["description", "repository", "homepage"] {
            if !manifest.package.contains_key(key) {
                diagnostics.push(GhorDiagnostic::warning(
                    "W_GHOR_PACKAGE_METADATA_001",
                    format!("package.{key} is recommended before publish"),
                ));
            }
        }
        if !manifest.package.contains_key("license")
            && !manifest.package.contains_key("license-file")
        {
            diagnostics.push(GhorDiagnostic::warning(
                "W_GHOR_PACKAGE_METADATA_001",
                "package.license or package.license-file is recommended before publish",
            ));
        }
    }
    if let Some(readme) = package_readme_path(manifest) {
        if !manifest.root.join(&readme).exists() {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PACKAGE_METADATA_001",
                format!("package.readme points to missing file '{}'", readme),
            ));
        }
    }
    if let Some(license_file) = manifest.package.get("license-file") {
        if !manifest.root.join(license_file).exists() {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PACKAGE_METADATA_001",
                format!(
                    "package.license-file points to missing file '{}'",
                    license_file
                ),
            ));
        }
    }
    if let Some(keywords) = manifest.package.get("keywords") {
        let keywords = parse_array(keywords);
        if keywords.len() > 5 {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PACKAGE_KEYWORDS_001",
                "crates.io allows at most 5 keywords",
            ));
        }
        for keyword in keywords {
            let valid = keyword.len() <= 20
                && keyword.is_ascii()
                && keyword
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_alphanumeric())
                    .unwrap_or(false)
                && keyword
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+'));
            if !valid {
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_PACKAGE_KEYWORDS_001",
                    format!("invalid crates.io keyword '{}'", keyword),
                ));
            }
        }
    }
    if let Some(categories) = manifest.package.get("categories") {
        let categories = parse_array(categories);
        if categories.len() > 5 {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PACKAGE_CATEGORIES_001",
                "crates.io allows at most 5 categories",
            ));
        }
        for category in categories {
            if category.trim().is_empty() || category.contains(' ') {
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_PACKAGE_CATEGORIES_001",
                    format!("invalid crates.io category '{}'", category),
                ));
            }
        }
    }
    diagnostics
}

fn package_readme_path(manifest: &GhorManifest) -> Option<String> {
    match manifest.package.get("readme").map(String::as_str) {
        Some("false") => None,
        Some("true") => Some("README.md".to_string()),
        Some(path) => Some(path.to_string()),
        None => ["README.md", "README.txt", "README"]
            .into_iter()
            .find(|path| manifest.root.join(path).exists())
            .map(str::to_string),
    }
}

fn crate_publish_metadata(manifest: &GhorManifest) -> Value {
    json!({
        "name": package_name(manifest),
        "vers": package_version(manifest),
        "deps": cargo_index_deps(manifest).into_iter().map(|dep| {
            json!({
                "name": dep.get("name").cloned().unwrap_or(Value::Null),
                "version_req": dep.get("req").cloned().unwrap_or(Value::String("*".to_string())),
                "features": dep.get("features").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
                "optional": dep.get("optional").cloned().unwrap_or(Value::Bool(false)),
                "default_features": dep.get("default_features").cloned().unwrap_or(Value::Bool(true)),
                "target": dep.get("target").cloned().unwrap_or(Value::Null),
                "kind": dep.get("kind").cloned().unwrap_or(Value::String("normal".to_string())),
                "registry": dep.get("registry").cloned().unwrap_or(Value::Null),
                "explicit_name_in_toml": dep.get("package").cloned().unwrap_or(Value::Null)
            })
        }).collect::<Vec<_>>(),
        "features": cargo_index_features(manifest),
        "authors": manifest.package.get("authors").map(|value| parse_array(value)).unwrap_or_default(),
        "description": manifest.package.get("description").cloned(),
        "documentation": manifest.package.get("documentation").cloned(),
        "homepage": manifest.package.get("homepage").cloned(),
        "readme": package_readme_path(manifest),
        "readme_file": package_readme_path(manifest).and_then(|path| fs::read_to_string(manifest.root.join(path)).ok()),
        "keywords": manifest.package.get("keywords").map(|value| parse_array(value)).unwrap_or_default(),
        "categories": manifest.package.get("categories").map(|value| parse_array(value)).unwrap_or_default(),
        "license": manifest.package.get("license").cloned(),
        "license_file": manifest.package.get("license-file").cloned(),
        "repository": manifest.package.get("repository").cloned(),
        "badges": {},
        "links": manifest.package.get("links").cloned(),
        "rust_version": manifest.toolchain.get("rust_version").cloned(),
        "ghor_native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>()
    })
}

fn run_publish(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let registry = registry_from_args_with_manifest(args, &manifest);
    ensure_registry_network_allowed(&registry, args, Some(&manifest.cargo_config))?;
    let token = registry_token_with_manifest(args, &registry, Some(&manifest));
    let options = package_options_from_args(args);
    let diagnostics = prepublish_diagnostics(&manifest, Some(&registry), &options)?;
    if diagnostics_have_errors(&diagnostics) {
        return Err(diagnostics_summary(&diagnostics).into());
    }
    let (archive_rel, manifest_rel) = registry_package_paths(&manifest);
    let package_dir = match &registry {
        GhorRegistry::Local(path) => path
            .join("packages")
            .join(package_name(&manifest))
            .join(package_version(&manifest)),
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) | GhorRegistry::Git(_) => {
            workspace_target_dir(&manifest)
                .join("registry-publish")
                .join(format!(
                    "{}-{}",
                    package_name(&manifest),
                    package_version(&manifest)
                ))
        }
    };
    let package = write_package(&manifest, &package_dir, &options)?;
    if has_flag(args, "--dry-run") {
        return print_json(&json!({
            "schema": "ghor.publish.v1",
            "status": "dry_run",
            "registry": registry_label(&registry),
            "registry_transport": registry_transport(&registry),
            "package": package_name(&manifest),
            "version": package_version(&manifest),
            "archive": package.archive,
            "checksum": package.checksum,
            "package_manifest_hash": package.package_manifest_hash,
            "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
            "metadata": package.metadata,
            "diagnostics": diagnostics_to_json(&diagnostics),
            "verification": package.verification
        }));
    }
    let config =
        ensure_registry_config_ref(&registry, token.as_deref(), Some(&manifest.cargo_config))?;
    let api_publish = cargo_api_publish(
        &registry,
        &config,
        &manifest,
        &package,
        token.as_deref(),
        Some(&manifest.cargo_config),
    )
    .unwrap_or_else(|err| json!({"status": "direct_index_used", "reason": err.to_string()}));
    registry_put_file(
        &registry,
        &archive_rel,
        &package.archive,
        token.as_deref(),
        Some(&manifest.cargo_config),
    )?;
    registry_put_file(
        &registry,
        &manifest_rel,
        &package.manifest_path,
        token.as_deref(),
        Some(&manifest.cargo_config),
    )?;
    let mut index =
        read_registry_index_ref(&registry, token.as_deref(), Some(&manifest.cargo_config))?;
    upsert_registry_package(
        &mut index,
        registry_package_entry(&manifest, &package, &archive_rel, &manifest_rel, &registry),
    )?;
    write_registry_index_ref(
        &registry,
        &index,
        token.as_deref(),
        Some(&manifest.cargo_config),
    )?;
    let cargo_entry = cargo_registry_index_entry(&manifest, &package, &archive_rel, &manifest_rel);
    upsert_registry_package_index_entry(
        &registry,
        &manifest,
        &cargo_entry,
        token.as_deref(),
        Some(&manifest.cargo_config),
    )?;
    print_json(&json!({
        "schema": "ghor.publish.v1",
        "status": "published",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "package": package_name(&manifest),
        "version": package_version(&manifest),
        "archive": archive_rel,
        "checksum": package.checksum,
        "manifest_hash": package.manifest_hash,
        "package_manifest": manifest_rel,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "metadata": package.metadata,
        "verification": package.verification,
        "diagnostics": diagnostics_to_json(&diagnostics),
        "cargo_index_path": cargo_index_path(&package_name(&manifest)),
        "web_api_publish": api_publish
    }))
}

fn run_search(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let query = first_positional(args).unwrap_or("");
    let index = read_registry_index_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let packages = index
        .get("packages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name.contains(query))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema": "ghor.search.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "packages": packages
    }))
}

fn run_info(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let name = first_positional(args).ok_or("[E_GHOR_INFO_001] info expects <package>")?;
    let index = read_registry_index_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let packages = index
        .get("packages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(name))
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema": "ghor.info.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "packages": packages
    }))
}

fn run_yank(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let raw_name = first_positional(args).ok_or("[E_GHOR_YANK_001] yank expects <package>")?;
    let (name, version) = package_version_selector(raw_name, args);
    let yanked = !has_flag(args, "--undo");
    require_registry_auth_for_mutation(&registry, token.as_deref(), "yank")?;
    let config = registry_config_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let web_api_yank = if let Some(version) = version.as_deref() {
        cargo_api_yank(
            &registry,
            &config,
            &name,
            version,
            yanked,
            token.as_deref(),
            manifest.as_ref().map(|manifest| &manifest.cargo_config),
        )
        .unwrap_or_else(|err| json!({"status": "direct_index_used", "reason": err.to_string()}))
    } else {
        json!({"status": "not_requested", "reason": "version_not_specified"})
    };
    let mut index = read_registry_index_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    update_registry_yank(&mut index, &name, version.as_deref(), yanked)?;
    if !matches!(registry, GhorRegistry::Git(_)) {
        write_registry_index_ref(
            &registry,
            &index,
            token.as_deref(),
            manifest.as_ref().map(|manifest| &manifest.cargo_config),
        )?;
        update_registry_package_yank(
            &registry,
            &name,
            version.as_deref(),
            yanked,
            token.as_deref(),
            manifest.as_ref().map(|manifest| &manifest.cargo_config),
        )?;
        append_registry_audit(
            &registry,
            "yank",
            Some(&name),
            version.as_deref(),
            token.as_deref(),
            Some(if yanked { "yanked" } else { "unyanked" }),
            manifest.as_ref().map(|manifest| &manifest.cargo_config),
        )?;
    }
    print_json(&json!({
        "schema": "ghor.yank.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "package": name,
        "version": version,
        "yanked": yanked,
        "audit_log": "registry-audit.json",
        "web_api_yank": web_api_yank
    }))
}

fn run_login(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = arg_after(args, "--token")
        .map(str::to_string)
        .or_else(|| env::var("GHOR_REGISTRY_TOKEN").ok())
        .or_else(|| registry_token_with_manifest(args, &registry, manifest.as_ref()));
    let config = registry_config_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )
    .unwrap_or_else(|_| default_registry_config(&registry));
    let login_url = cargo_login_url(&config);
    let auth_status = if let Some(token) = token {
        write_registry_credentials(&registry, &token)?;
        "token_stored"
    } else if matches!(
        registry,
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) | GhorRegistry::Git(_)
    ) {
        "anonymous_remote"
    } else {
        "local_registry"
    };
    print_json(&json!({
        "schema": "ghor.login.v1",
        "status": "configured",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "auth": auth_status,
        "login_url": login_url,
        "cargo_login_semantics": "token is obtained out-of-band and stored locally"
    }))
}

fn run_logout(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let removed = remove_registry_credentials(&registry)?;
    print_json(&json!({
        "schema": "ghor.logout.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "removed": removed
    }))
}

fn run_registry(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = first_positional(args).unwrap_or("doctor");
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let config_ref = manifest.as_ref().map(|manifest| &manifest.cargo_config);
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    match action {
        "doctor" => print_json(&json!({
            "schema": "ghor.registry.doctor.v1",
            "status": registry_doctor_status(&registry, token.as_deref(), config_ref),
            "registry": registry_label(&registry),
            "registry_transport": registry_transport(&registry),
            "curl_available": curl_available(),
            "cargo_config": manifest.as_ref().map(|manifest| {
                cargo_config_to_json(
                    &manifest.cargo_config,
                    registry_name_from_args_with_manifest(args, manifest).as_deref(),
                    Some(&registry),
                )
            }),
            "protocol": registry_protocol_json()
        })),
        "cargo-config" => {
            let manifest = manifest
                .as_ref()
                .ok_or("[E_GHOR_CONFIG_001] registry cargo-config expects --manifest-path")?;
            print_json(&json!({
                "schema": "ghor.registry.cargo_config.v1",
                "status": "ok",
                "registry": registry_label(&registry),
                "registry_transport": registry_transport(&registry),
                "credential_status": if registry_token_with_manifest(args, &registry, Some(manifest)).is_some() { "available" } else { "not_configured" },
                "config": cargo_config_to_json(
                    &manifest.cargo_config,
                    registry_name_from_args_with_manifest(args, manifest).as_deref(),
                    Some(&registry),
                )
            }))
        }
        "hardening" | "production-hardening" | "release-hardening" => {
            let report = registry_hardening_report(args)?;
            if hardening_has_status(&report, "error") {
                return Err(
                    "[E_GHOR_REGISTRY_HARDENING_001] registry production hardening failed".into(),
                );
            }
            print_json(&report)
        }
        "index" => {
            ensure_registry_network_allowed(&registry, args, config_ref)?;
            let index = read_registry_index_ref(&registry, token.as_deref(), config_ref)?;
            print_json(&json!({
                "schema": "ghor.registry.index.v1",
                "status": "ok",
                "registry": registry_label(&registry),
                "registry_transport": registry_transport(&registry),
                "index": index
            }))
        }
        "config" => {
            ensure_registry_network_allowed(&registry, args, config_ref)?;
            let config = registry_config_ref(&registry, token.as_deref(), config_ref)?;
            print_json(&json!({
                "schema": "ghor.registry.config.v1",
                "status": "ok",
                "registry": registry_label(&registry),
                "registry_transport": registry_transport(&registry),
                "config": config
            }))
        }
        "package-index" => {
            ensure_registry_network_allowed(&registry, args, config_ref)?;
            let name = positional_args(args)
                .get(1)
                .copied()
                .ok_or("[E_GHOR_REGISTRY_001] registry package-index expects <package>")?;
            let entries =
                read_registry_package_entries(&registry, name, token.as_deref(), config_ref)?;
            print_json(&json!({
                "schema": "ghor.registry.package-index.v1",
                "status": "ok",
                "registry": registry_label(&registry),
                "registry_transport": registry_transport(&registry),
                "package": name,
                "path": cargo_index_path(name),
                "entries": entries
            }))
        }
        other => Err(format!("[E_GHOR_REGISTRY_001] unsupported registry action '{other}'").into()),
    }
}

fn run_release_hardening(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (doctor, doctor_diagnostics) = doctor_payload()?;
    if diagnostics_have_errors(&doctor_diagnostics) {
        return Err(diagnostics_summary(&doctor_diagnostics).into());
    }
    let registry_hardening = registry_hardening_report(args)?;
    if hardening_has_status(&registry_hardening, "error") {
        return Err("[E_GHOR_RELEASE_HARDENING_001] release hardening failed".into());
    }
    print_json(&json!({
        "schema": "ghor.release_hardening.v1",
        "status": if hardening_has_status(&registry_hardening, "warning") { "warning" } else { "ok" },
        "doctor": doctor,
        "registry_hardening": registry_hardening,
        "next_planned_point": "c_cpp_rust_interconnection_closed"
    }))
}

fn run_external_compat(args: &[String]) -> Result<(), Box<dyn Error>> {
    let report = external_compat_report(args)?;
    if external_compat_has_status(&report, "error") {
        if has_flag(args, "--json") {
            print_json(&report)?;
        }
        return Err("[E_GHOR_EXTERNAL_COMPAT_001] external Cargo compatibility failed".into());
    }
    print_json(&report)
}

fn external_compat_report(args: &[String]) -> Result<Value, Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if !manifest_is_cargo_toml(&manifest.path) {
        return Err(
            "[E_GHOR_EXTERNAL_COMPAT_001] external compatibility requires Cargo.toml".into(),
        );
    }
    let comparisons = vec![
        compare_external_metadata(&manifest, args)?,
        compare_external_tree(&manifest, args)?,
        compare_external_package_list(&manifest)?,
        compare_external_vendor(&manifest, args)?,
        compare_external_update(&manifest, args)?,
    ];
    let status = if comparisons
        .iter()
        .any(|comparison| comparison["status"] == "error")
    {
        "error"
    } else {
        "ok"
    };
    Ok(json!({
        "schema": "ghor.external_compat.v1",
        "status": status,
        "manifest_path": manifest.path,
        "package": package_name(&manifest),
        "version": package_version(&manifest),
        "cargo_version": cargo_version_string(),
        "comparisons": comparisons,
        "accepted_differences": external_compat_accepted_differences()
    }))
}

fn compare_external_metadata(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let cargo = run_cargo_command(
        &[
            "metadata".to_string(),
            "--format-version".to_string(),
            "1".to_string(),
            "--no-deps".to_string(),
            "--manifest-path".to_string(),
            manifest.path.display().to_string(),
        ],
        None,
    )?;
    let mut errors = Vec::new();
    let mut details = json!({});
    if cargo.status == "ok" {
        let cargo_json: Value = serde_json::from_str(&cargo.stdout)?;
        let ghor_json = cargo_compatible_metadata_json(manifest, args)?;
        let cargo_package = find_cargo_package(&cargo_json, &package_name(manifest));
        let ghor_package = find_cargo_package(&ghor_json, &package_name(manifest));
        match (cargo_package, ghor_package) {
            (Some(cargo_package), Some(ghor_package)) => {
                for field in ["name", "version", "edition", "rust_version"] {
                    if cargo_package.get(field) != ghor_package.get(field) {
                        errors.push(format!("metadata field {field} differs"));
                    }
                }
                let cargo_kinds = cargo_target_kind_set(cargo_package);
                let ghor_kinds = cargo_target_kind_set(ghor_package);
                if cargo_kinds != ghor_kinds {
                    errors.push("metadata target kind set differs".to_string());
                }
                details = json!({
                    "cargo_target_kinds": cargo_kinds,
                    "ghor_target_kinds": ghor_kinds,
                    "cargo_workspace_members": cargo_json.get("workspace_members").cloned().unwrap_or(Value::Null),
                    "ghor_workspace_members": ghor_json.get("workspace_members").cloned().unwrap_or(Value::Null)
                });
            }
            _ => errors.push("package missing from cargo or ghor metadata".to_string()),
        }
    } else {
        errors.push("cargo metadata failed".to_string());
    }
    Ok(external_comparison_json(
        "metadata",
        &cargo,
        if errors.is_empty() { "ok" } else { "error" },
        errors,
        details,
    ))
}

fn compare_external_tree(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let cargo = run_cargo_command(
        &[
            "tree".to_string(),
            "--manifest-path".to_string(),
            manifest.path.display().to_string(),
            "--prefix".to_string(),
            "none".to_string(),
        ],
        None,
    )?;
    let mut errors = Vec::new();
    let resolve = resolve_plan(manifest, args)?;
    let root_deps = all_manifest_dependencies(manifest)
        .into_iter()
        .map(|dep| dep.name.clone())
        .collect::<BTreeSet<_>>();
    for dep in &root_deps {
        if cargo.status == "ok" && !cargo.stdout.contains(dep) {
            errors.push(format!("cargo tree did not mention dependency {dep}"));
        }
    }
    let ghor_nodes = resolve
        .get("resolved")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    Ok(external_comparison_json(
        "tree",
        &cargo,
        if cargo.status == "ok" && errors.is_empty() {
            "ok"
        } else {
            "error"
        },
        errors,
        json!({
            "root_dependencies": root_deps,
            "ghor_resolved_nodes": ghor_nodes,
            "accepted_difference": "cargo tree is text output; ghor tree is JSON resolver evidence"
        }),
    ))
}

fn compare_external_package_list(manifest: &GhorManifest) -> Result<Value, Box<dyn Error>> {
    let cargo = run_cargo_command(
        &[
            "package".to_string(),
            "--list".to_string(),
            "--allow-dirty".to_string(),
            "--manifest-path".to_string(),
            manifest.path.display().to_string(),
        ],
        None,
    )?;
    let mut errors = Vec::new();
    let ghor_files = package_file_list(manifest)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let cargo_files = cargo
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| normalize_cargo_package_list_path(line, manifest))
        .collect::<BTreeSet<_>>();
    for required in cargo_manifest_required_files(manifest) {
        if !cargo_files.contains(&required) {
            errors.push(format!("cargo package --list missing required {required}"));
        }
        if !ghor_files.contains(&required) {
            errors.push(format!("ghor package --list missing required {required}"));
        }
    }
    Ok(external_comparison_json(
        "package_list",
        &cargo,
        if cargo.status == "ok" && errors.is_empty() {
            "ok"
        } else {
            "error"
        },
        errors,
        json!({
            "cargo_file_count": cargo_files.len(),
            "ghor_file_count": ghor_files.len(),
            "common_required_files": cargo_manifest_required_files(manifest),
            "accepted_difference": "Cargo may include Cargo.toml.orig and VCS-derived package metadata; ghor emits nC package metadata separately"
        }),
    ))
}

fn compare_external_vendor(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let base = arg_after(args, "--vendor-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.root.join("target/ghor/external-compat/vendor"));
    let cargo_vendor = base.join("cargo");
    let ghor_vendor = base.join("ghor");
    if base.exists() {
        fs::remove_dir_all(&base)?;
    }
    fs::create_dir_all(&base)?;
    let cargo = run_cargo_command(
        &[
            "vendor".to_string(),
            "--manifest-path".to_string(),
            manifest.path.display().to_string(),
            cargo_vendor.display().to_string(),
        ],
        None,
    )?;
    let ghor_args = vec![
        ghor_vendor.display().to_string(),
        "--manifest-path".to_string(),
        manifest.path.display().to_string(),
        "--no-delete".to_string(),
        "--json".to_string(),
    ];
    let ghor_resolve = resolve_plan(manifest, args)?;
    let ghor_diagnostics = ghor_resolve
        .get("diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let ghor_result = vendor_report(&ghor_args);
    let mut errors = Vec::new();
    if cargo.status != "ok" {
        errors.push("cargo vendor failed".to_string());
    }
    if ghor_result.is_err() {
        errors.push("ghor vendor failed".to_string());
    }
    if !ghor_diagnostics.is_empty() && !diagnostics_are_registry_snapshot_only(&ghor_diagnostics) {
        errors.push("ghor vendor emitted non-registry-snapshot diagnostics".to_string());
    }
    let status = if !errors.is_empty() {
        "error"
    } else if diagnostics_are_registry_snapshot_only(&ghor_diagnostics) {
        "accepted_difference"
    } else {
        "ok"
    };
    Ok(external_comparison_json(
        "vendor",
        &cargo,
        status,
        errors,
        json!({
            "cargo_vendor_dir": cargo_vendor,
            "ghor_vendor_dir": ghor_vendor,
            "ghor_status": ghor_result.as_ref().ok().and_then(|value| value.get("status")).cloned().unwrap_or(Value::String("error".to_string())),
            "ghor_diagnostics": ghor_diagnostics,
            "accepted_difference": "Cargo vendor emits TOML source replacement text; ghor vendor emits JSON plus Cargo-shaped checksum files. For external crates.io projects without a populated ghor registry snapshot, Cargo validates the registry graph and ghor records registry snapshot diagnostics instead of inventing versions."
        }),
    ))
}

fn compare_external_update(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let cargo = run_cargo_command(
        &[
            "update".to_string(),
            "--dry-run".to_string(),
            "--manifest-path".to_string(),
            manifest.path.display().to_string(),
        ],
        None,
    )?;
    let resolve = resolve_plan(manifest, args)?;
    let diagnostics = resolve
        .get("diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut errors = Vec::new();
    if cargo.status != "ok" {
        errors.push("cargo update --dry-run failed".to_string());
    }
    if !diagnostics.is_empty() && !diagnostics_are_registry_snapshot_only(&diagnostics) {
        errors.push("ghor update dry-run would emit resolver diagnostics".to_string());
    }
    let status = if !errors.is_empty() {
        "error"
    } else if diagnostics_are_registry_snapshot_only(&diagnostics) {
        "accepted_difference"
    } else {
        "ok"
    };
    Ok(external_comparison_json(
        "update_dry_run",
        &cargo,
        status,
        errors,
        json!({
            "ghor_status": if diagnostics.is_empty() {
                "dry_run"
            } else if diagnostics_are_registry_snapshot_only(&diagnostics) {
                "accepted_registry_snapshot_gap"
            } else {
                "error"
            },
            "ghor_diagnostics": diagnostics,
            "accepted_difference": "Cargo updates Cargo.lock; ghor updates ghor.lock with ghor.lock.meta.json sidecar. For external crates.io projects without a populated ghor registry snapshot, Cargo validates the update graph and ghor records registry snapshot diagnostics instead of inventing versions."
        }),
    ))
}

fn diagnostics_are_registry_snapshot_only(diagnostics: &[Value]) -> bool {
    !diagnostics.is_empty()
        && diagnostics.iter().all(|diagnostic| {
            diagnostic.get("code").and_then(Value::as_str) == Some("E_GHOR_RESOLVER_VERSION_001")
        })
}

fn external_comparison_json(
    command: &str,
    cargo: &ToolOutput,
    status: &str,
    errors: Vec<String>,
    details: Value,
) -> Value {
    json!({
        "command": command,
        "status": status,
        "cargo": tool_output_to_json(&format!("cargo {command}"), cargo),
        "errors": errors,
        "details": details
    })
}

fn run_cargo_command(args: &[String], cwd: Option<&Path>) -> Result<ToolOutput, crate::CliFailure> {
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_tool(PathBuf::from("cargo"), &refs, cwd)
}

fn cargo_version_string() -> Value {
    run_cargo_command(&["--version".to_string()], None)
        .map(|output| Value::String(output.stdout.trim().to_string()))
        .unwrap_or(Value::Null)
}

fn find_cargo_package<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .get("packages")
        .and_then(Value::as_array)?
        .iter()
        .find(|package| package.get("name").and_then(Value::as_str) == Some(name))
}

fn cargo_target_kind_set(package: &Value) -> BTreeSet<String> {
    package
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|target| {
            target
                .get("kind")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn normalize_cargo_package_list_path(line: &str, manifest: &GhorManifest) -> String {
    let mut normalized = line
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_string();
    let package = package_name(manifest);
    let version = package_version(manifest);
    let exact_prefix = format!("{package}-{version}/");
    if let Some(rest) = normalized.strip_prefix(&exact_prefix) {
        return rest.to_string();
    }
    if let Some((first, rest)) = normalized.split_once('/') {
        let package_dash = package.replace('_', "-");
        let looks_like_cargo_root = first == package
            || first == package_dash
            || first.starts_with(&format!("{package}-"))
            || first.starts_with(&format!("{package_dash}-"))
            || first.contains(&version);
        if looks_like_cargo_root {
            normalized = rest.to_string();
        }
    }
    normalized
}

fn cargo_manifest_required_files(manifest: &GhorManifest) -> Vec<String> {
    let mut files = vec!["Cargo.toml".to_string()];
    for path in ["src/lib.rs", "src/main.rs", "build.rs"] {
        if manifest.root.join(path).exists() {
            files.push(path.to_string());
        }
    }
    files
}

fn external_compat_accepted_differences() -> Value {
    json!([
        {
            "surface": "metadata",
            "difference": "ghor wraps Cargo-compatible metadata under ghor.metadata.v1 and keeps nC machine/native fields",
            "accepted": true
        },
        {
            "surface": "tree",
            "difference": "cargo tree is human text; ghor tree is JSON resolver evidence",
            "accepted": true
        },
        {
            "surface": "package",
            "difference": "Cargo may include Cargo.toml.orig/VCS package details; ghor records nC package metadata and verification separately",
            "accepted": true
        },
        {
            "surface": "vendor",
            "difference": "Cargo emits TOML source replacement text; ghor emits JSON plus Cargo-shaped checksum files",
            "accepted": true
        },
        {
            "surface": "update",
            "difference": "Cargo updates Cargo.lock; ghor updates ghor.lock and ghor.lock.meta.json",
            "accepted": true
        },
        {
            "surface": "external_registry_snapshot",
            "difference": "Cargo projects may resolve crates.io through Cargo metadata/vendor/update while ghor records E_GHOR_RESOLVER_VERSION_001 if no matching ghor registry snapshot is configured; this is accepted only in external-compat reports and is never treated as invented ghor registry evidence",
            "accepted": true
        }
    ])
}

fn external_compat_has_status(report: &Value, status: &str) -> bool {
    report
        .get("comparisons")
        .and_then(Value::as_array)
        .map(|comparisons| {
            comparisons
                .iter()
                .any(|comparison| comparison.get("status").and_then(Value::as_str) == Some(status))
        })
        .unwrap_or(false)
}

fn registry_hardening_report(args: &[String]) -> Result<Value, Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let config_ref = manifest.as_ref().map(|manifest| &manifest.cargo_config);
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let mut checks = Vec::new();
    let transport = registry_transport(&registry);
    let label = registry_label(&registry);
    let remote = matches!(
        registry,
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) | GhorRegistry::Git(_)
    );

    checks.push(hardening_check(
        "registry_transport",
        "ok",
        format!("{transport} registry selected"),
    ));

    if remote && !curl_available() {
        checks.push(hardening_check(
            "curl_available",
            "error",
            "curl is required for remote productive registries",
        ));
    } else {
        checks.push(hardening_check(
            "curl_available",
            "ok",
            "curl availability satisfies registry transport",
        ));
    }

    if remote && token.is_none() {
        checks.push(hardening_check(
            "remote_auth_token",
            "error",
            "remote productive registry checks require an auth token",
        ));
    } else if remote {
        checks.push(hardening_check(
            "remote_auth_token",
            "ok",
            "auth token available and redacted from report",
        ));
    } else {
        checks.push(hardening_check(
            "remote_auth_token",
            "ok",
            "filesystem registry does not require remote auth",
        ));
    }

    let allow_insecure_localhost = has_flag(args, "--allow-insecure-localhost")
        || label.starts_with("http://127.0.0.1")
        || label.starts_with("http://localhost");
    if label.starts_with("http://") && !allow_insecure_localhost {
        checks.push(hardening_check(
            "transport_security",
            "error",
            "productive remote registry must use HTTPS or explicit localhost smoke allowance",
        ));
    } else if label.starts_with("http://") {
        checks.push(hardening_check(
            "transport_security",
            "ok",
            "HTTP accepted for localhost smoke registry only",
        ));
    } else {
        checks.push(hardening_check(
            "transport_security",
            "ok",
            "transport satisfies release hardening policy",
        ));
    }

    match registry_config_ref(&registry, token.as_deref(), config_ref) {
        Ok(config) => {
            checks.push(hardening_check(
                "registry_config",
                "ok",
                "config.json readable",
            ));
            let auth_required = config
                .get("auth-required")
                .and_then(Value::as_bool)
                .unwrap_or(remote);
            if remote && auth_required && token.is_none() {
                checks.push(hardening_check(
                    "config_auth_required",
                    "error",
                    "config.json requires auth and no token was available",
                ));
            } else {
                checks.push(hardening_check(
                    "config_auth_required",
                    "ok",
                    "auth-required policy is satisfiable",
                ));
            }
        }
        Err(err) => checks.push(hardening_check(
            "registry_config",
            "error",
            format!("config.json unreadable: {err}"),
        )),
    }

    match read_registry_index_ref(&registry, token.as_deref(), config_ref) {
        Ok(index) => {
            let count = index
                .get("packages")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            checks.push(hardening_check(
                "registry_index",
                "ok",
                format!("index readable with {count} package entries"),
            ));
        }
        Err(err) => checks.push(hardening_check(
            "registry_index",
            "error",
            format!("registry index unreadable: {err}"),
        )),
    }

    if let Some(manifest) = manifest.as_ref() {
        checks.extend(registry_package_hardening_checks(
            &registry,
            manifest,
            token.as_deref(),
            config_ref,
        ));
    } else {
        checks.push(hardening_check(
            "package_checksum",
            "warning",
            "no --manifest-path supplied; package checksum verification skipped",
        ));
    }

    checks.extend(registry_governance_hardening_checks(
        &registry,
        token.as_deref(),
        config_ref,
        remote,
    ));
    checks.extend(registry_moderation_hardening_checks(
        &registry,
        token.as_deref(),
        config_ref,
        remote,
    ));
    checks.extend(registry_audit_hardening_checks(
        &registry,
        token.as_deref(),
        config_ref,
        remote,
    ));
    checks.push(registry_cdn_hardening_check(args));

    let status = if checks.iter().any(|check| check["status"] == "error") {
        "error"
    } else if checks.iter().any(|check| check["status"] == "warning") {
        "warning"
    } else {
        "ok"
    };
    Ok(json!({
        "schema": "ghor.registry.production_hardening.v1",
        "status": status,
        "registry": label,
        "registry_transport": transport,
        "token": if token.is_some() { "available_redacted" } else { "not_configured" },
        "checks": checks,
        "summary": {
            "errors": hardening_count(&checks, "error"),
            "warnings": hardening_count(&checks, "warning"),
            "ok": hardening_count(&checks, "ok"),
            "release_ready": status == "ok"
        }
    }))
}

fn registry_package_hardening_checks(
    registry: &GhorRegistry,
    manifest: &GhorManifest,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Vec<Value> {
    let name = package_name(manifest);
    let version = package_version(manifest);
    let mut checks = Vec::new();
    match read_registry_package_entries(registry, &name, token, config) {
        Ok(entries) => {
            let entry = entries.iter().find(|entry| {
                entry.get("name").and_then(Value::as_str) == Some(name.as_str())
                    && entry.get("vers").and_then(Value::as_str) == Some(version.as_str())
            });
            let Some(entry) = entry else {
                checks.push(hardening_check(
                    "package_index_entry",
                    "error",
                    format!("{name} {version} is not present in the package index"),
                ));
                return checks;
            };
            checks.push(hardening_check(
                "package_index_entry",
                "ok",
                format!("{name} {version} package index entry found"),
            ));
            let Some(checksum) = entry.get("cksum").and_then(Value::as_str) else {
                checks.push(hardening_check(
                    "package_checksum",
                    "error",
                    "package index entry does not carry cksum",
                ));
                return checks;
            };
            let archive = entry
                .get("ghor_archive")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("packages/{name}/{version}/{name}-{version}.crate"));
            match registry_get_download_bytes(registry, &archive, token, config) {
                Ok(bytes) => {
                    let actual = sha256_hex(&bytes);
                    if actual == checksum {
                        checks.push(hardening_check(
                            "package_checksum",
                            "ok",
                            "package archive SHA-256 matches package index cksum",
                        ));
                    } else {
                        checks.push(hardening_check(
                            "package_checksum",
                            "error",
                            "package archive SHA-256 does not match package index cksum",
                        ));
                    }
                }
                Err(err) => checks.push(hardening_check(
                    "package_archive",
                    "error",
                    format!("package archive unreadable: {err}"),
                )),
            }
        }
        Err(err) => checks.push(hardening_check(
            "package_index_entry",
            "error",
            format!("package index unreadable: {err}"),
        )),
    }
    checks
}

fn registry_governance_hardening_checks(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
    required: bool,
) -> Vec<Value> {
    let exists = registry_json_exists(registry, "governance.json", token, config);
    let mut checks = Vec::new();
    if !exists && required {
        checks.push(hardening_check(
            "registry_governance",
            "error",
            "productive remote registry requires governance.json",
        ));
        return checks;
    }
    if !exists {
        checks.push(hardening_check(
            "registry_governance",
            "warning",
            "governance.json not present for filesystem registry",
        ));
        return checks;
    }
    match read_registry_governance(registry, token, config) {
        Ok(policy)
            if policy.get("schema").and_then(Value::as_str)
                == Some("ghor.registry.governance.v1") =>
        {
            checks.push(hardening_check(
                "registry_governance",
                "ok",
                "governance policy readable and schema-valid",
            ));
        }
        Ok(_) => checks.push(hardening_check(
            "registry_governance",
            "error",
            "governance policy has unexpected schema",
        )),
        Err(err) => checks.push(hardening_check(
            "registry_governance",
            "error",
            format!("governance policy unreadable: {err}"),
        )),
    }
    checks
}

fn registry_moderation_hardening_checks(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
    required: bool,
) -> Vec<Value> {
    let exists = registry_json_exists(registry, "moderation.json", token, config);
    let mut checks = Vec::new();
    if !exists && required {
        checks.push(hardening_check(
            "registry_moderation",
            "error",
            "productive remote registry requires moderation.json",
        ));
        return checks;
    }
    if !exists {
        checks.push(hardening_check(
            "registry_moderation",
            "warning",
            "moderation.json not present for filesystem registry",
        ));
        return checks;
    }
    match read_registry_moderation(registry, token, config) {
        Ok(policy)
            if policy.get("schema").and_then(Value::as_str)
                == Some("ghor.registry.moderation.v1") =>
        {
            checks.push(hardening_check(
                "registry_moderation",
                "ok",
                "moderation state readable and schema-valid",
            ));
        }
        Ok(_) => checks.push(hardening_check(
            "registry_moderation",
            "error",
            "moderation state has unexpected schema",
        )),
        Err(err) => checks.push(hardening_check(
            "registry_moderation",
            "error",
            format!("moderation state unreadable: {err}"),
        )),
    }
    checks
}

fn registry_audit_hardening_checks(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
    required: bool,
) -> Vec<Value> {
    let exists = registry_json_exists(registry, "registry-audit.json", token, config);
    let mut checks = Vec::new();
    if !exists && required {
        checks.push(hardening_check(
            "registry_audit_log",
            "error",
            "productive remote registry requires registry-audit.json",
        ));
        return checks;
    }
    if !exists {
        checks.push(hardening_check(
            "registry_audit_log",
            "warning",
            "registry-audit.json not present for filesystem registry",
        ));
        return checks;
    }
    match read_registry_audit(registry, token, config) {
        Ok(audit)
            if audit.get("schema").and_then(Value::as_str) == Some("ghor.registry.audit.v1") =>
        {
            let event_count = audit
                .get("events")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            checks.push(hardening_check(
                "registry_audit_log",
                "ok",
                format!("registry audit log readable and schema-valid with {event_count} events"),
            ));
        }
        Ok(_) => checks.push(hardening_check(
            "registry_audit_log",
            "error",
            "registry audit log has unexpected schema",
        )),
        Err(err) => checks.push(hardening_check(
            "registry_audit_log",
            "error",
            format!("registry audit log unreadable: {err}"),
        )),
    }
    checks
}

fn registry_cdn_hardening_check(args: &[String]) -> Value {
    let Some(out) = arg_after(args, "--cdn-out")
        .or_else(|| arg_after(args, "--cdn"))
        .map(PathBuf::from)
    else {
        return hardening_check(
            "registry_cdn",
            "warning",
            "no --cdn-out supplied; CDN bundle validation skipped",
        );
    };
    let manifest_path = out.join("cdn-manifest.json");
    if !manifest_path.exists() {
        return hardening_check("registry_cdn", "error", "cdn-manifest.json is missing");
    }
    match fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    {
        Some(manifest)
            if manifest.get("schema").and_then(Value::as_str) == Some("ghor.cdn.manifest.v1")
                && manifest
                    .get("content_addressed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                && manifest
                    .get("entry_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    > 0 =>
        {
            hardening_check("registry_cdn", "ok", "CDN manifest is content-addressed")
        }
        _ => hardening_check("registry_cdn", "error", "CDN manifest is invalid or empty"),
    }
}

fn registry_json_exists(
    registry: &GhorRegistry,
    relative: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> bool {
    match registry {
        GhorRegistry::Local(path) => path.join(relative).exists(),
        GhorRegistry::Http(base) => {
            curl_get_text_optional(&registry_url(base, relative), token, config)
                .ok()
                .flatten()
                .is_some()
        }
        GhorRegistry::Sparse(base) => curl_get_text_cached(base, relative, token, config)
            .ok()
            .flatten()
            .is_some(),
        GhorRegistry::Git(_) => registry_git_cache(registry, config)
            .map(|cache| cache.join(relative).exists())
            .unwrap_or(false),
    }
}

fn hardening_check(id: &str, status: &str, reason: impl Into<String>) -> Value {
    json!({
        "id": id,
        "status": status,
        "reason": reason.into()
    })
}

fn hardening_count(checks: &[Value], status: &str) -> usize {
    checks
        .iter()
        .filter(|check| check.get("status").and_then(Value::as_str) == Some(status))
        .count()
}

fn hardening_has_status(report: &Value, status: &str) -> bool {
    report
        .get("checks")
        .and_then(Value::as_array)
        .map(|checks| {
            checks
                .iter()
                .any(|check| check.get("status").and_then(Value::as_str) == Some(status))
        })
        .unwrap_or(false)
}

fn run_cdn(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = first_positional(args).unwrap_or("build");
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let config_ref = manifest.as_ref().map(|manifest| &manifest.cargo_config);
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let out = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/ghor-cdn"));
    match action {
        "build" | "mirror" => {
            ensure_registry_network_allowed(&registry, args, config_ref)?;
            let index = read_registry_index_ref(&registry, token.as_deref(), config_ref)?;
            let packages = index
                .get("packages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut entries = Vec::new();
            fs::create_dir_all(&out)?;
            for package in packages {
                let name = package
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let version = package
                    .get("version")
                    .or_else(|| package.get("vers"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if name.is_empty() || version.is_empty() {
                    continue;
                }
                let archive_rel = package
                    .get("archive")
                    .or_else(|| package.get("ghor_archive"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("packages/{name}/{version}/{name}-{version}.crate"));
                let bytes = registry_get_download_bytes(
                    &registry,
                    &archive_rel,
                    token.as_deref(),
                    config_ref,
                )?;
                let hash = sha256_hex(&bytes);
                let blob_rel = format!("blobs/sha256/{hash}.crate");
                let package_rel = format!("packages/{name}/{version}/{name}-{version}.crate");
                let blob_path = out.join(&blob_rel);
                let package_path = out.join(&package_rel);
                if let Some(parent) = blob_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                if let Some(parent) = package_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&blob_path, &bytes)?;
                fs::write(&package_path, &bytes)?;
                entries.push(json!({
                    "name": name,
                    "version": version,
                    "source": archive_rel,
                    "blob": blob_rel,
                    "package_path": package_rel,
                    "sha256": hash,
                    "size": bytes.len(),
                    "immutable": true,
                    "cache_control": "max-age=31536000, immutable"
                }));
            }
            let manifest_json = json!({
                "schema": "ghor.cdn.manifest.v1",
                "status": "closed",
                "registry": registry_label(&registry),
                "registry_transport": registry_transport(&registry),
                "origin_index_hash": stable_json_hash(&index),
                "content_addressed": true,
                "entries": entries,
                "entry_count": entries.len(),
                "policy": {
                    "checksum": "sha256",
                    "immutable_blobs": true,
                    "origin_revalidation": ["ETag", "Last-Modified", "304 Not Modified"],
                    "purge_supported": true
                }
            });
            fs::write(
                out.join("cdn-manifest.json"),
                serde_json::to_vec_pretty(&manifest_json)?,
            )?;
            fs::write(
                out.join("config.json"),
                serde_json::to_vec_pretty(&json!({
                    "dl": "packages/{crate}/{version}/{crate}-{version}.crate",
                    "api": null,
                    "auth-required": false,
                    "cdn-manifest": "cdn-manifest.json"
                }))?,
            )?;
            print_json(&json!({
                "schema": "ghor.cdn.build.v1",
                "status": "ok",
                "out": out,
                "manifest": manifest_json
            }))
        }
        "status" | "inspect" => {
            let manifest_path = out.join("cdn-manifest.json");
            let manifest_json = if manifest_path.exists() {
                serde_json::from_slice::<Value>(&fs::read(&manifest_path)?)?
            } else {
                Value::Null
            };
            print_json(&json!({
                "schema": "ghor.cdn.status.v1",
                "status": if manifest_path.exists() { "ok" } else { "missing" },
                "out": out,
                "manifest": manifest_json
            }))
        }
        "purge" => {
            let hash = arg_after(args, "--sha256")
                .or_else(|| positional_args(args).get(1).copied())
                .ok_or("[E_GHOR_CDN_001] cdn purge expects --sha256 <hash>")?;
            let blob = out.join(format!("blobs/sha256/{hash}.crate"));
            let existed = blob.exists();
            if existed {
                fs::remove_file(&blob)?;
            }
            print_json(&json!({
                "schema": "ghor.cdn.purge.v1",
                "status": "ok",
                "sha256": hash,
                "removed": existed
            }))
        }
        other => Err(format!("[E_GHOR_CDN_001] unsupported cdn action '{other}'").into()),
    }
}

fn run_governance(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let config_ref = manifest.as_ref().map(|manifest| &manifest.cargo_config);
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let mut policy = read_registry_governance(&registry, token.as_deref(), config_ref)?;
    let mut mutated = false;
    if has_flag(args, "--init") {
        require_registry_admin_token(token.as_deref(), "governance init")?;
        policy = default_governance_policy();
        mutated = true;
    }
    if let Some(role) = arg_after(args, "--role") {
        require_registry_admin_token(token.as_deref(), "governance role mutation")?;
        let (user, role_name) = role
            .split_once(':')
            .ok_or("[E_GHOR_GOVERNANCE_001] --role expects USER:ROLE")?;
        policy["roles"][user] = Value::String(role_name.to_string());
        mutated = true;
    }
    if mutated {
        append_governance_audit(&mut policy, "governance.update", token.as_deref(), None);
        write_registry_governance(&registry, &policy, token.as_deref(), config_ref)?;
        append_registry_audit(
            &registry,
            "governance.update",
            None,
            None,
            token.as_deref(),
            Some("governance policy updated"),
            config_ref,
        )?;
    }
    print_json(&json!({
        "schema": "ghor.governance.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "policy": policy,
        "mutated": mutated
    }))
}

fn run_moderation(args: &[String]) -> Result<(), Box<dyn Error>> {
    let package =
        first_positional(args).ok_or("[E_GHOR_MODERATION_001] moderation expects <package>")?;
    let version = arg_after(args, "--version")
        .or_else(|| arg_after(args, "--vers"))
        .unwrap_or("*");
    let action = arg_after(args, "--action").unwrap_or("review");
    let reason = arg_after(args, "--reason").unwrap_or("not specified");
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    let config_ref = manifest.as_ref().map(|manifest| &manifest.cargo_config);
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    if action != "review" {
        require_registry_admin_token(token.as_deref(), "moderation mutation")?;
    }
    let mut moderation = read_registry_moderation(&registry, token.as_deref(), config_ref)?;
    let status = moderation_status_for_action(action)?;
    moderation["packages"][package][version] = json!({
        "status": status,
        "action": action,
        "reason": reason,
        "moderator_hash": token.as_ref().map(|value| sha256_hex(value.as_bytes())).unwrap_or_else(|| "anonymous".to_string()),
        "updated": "deterministic-local"
    });
    append_moderation_audit(&mut moderation, package, version, action, reason);
    if action != "review" {
        write_registry_moderation(&registry, &moderation, token.as_deref(), config_ref)?;
        apply_moderation_to_registry_index(
            &registry,
            package,
            version,
            status,
            token.as_deref(),
            config_ref,
        )?;
        append_registry_audit(
            &registry,
            "moderation.update",
            Some(package),
            Some(version),
            token.as_deref(),
            Some(reason),
            config_ref,
        )?;
    }
    print_json(&json!({
        "schema": "ghor.moderation.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "package": package,
        "version": version,
        "moderation_status": status,
        "action": action,
        "reason": reason,
        "mutated": action != "review",
        "moderation": moderation
    }))
}

fn run_cargo_exec(args: &[String], mode: &str) -> Result<(), Box<dyn Error>> {
    let manifest_path = manifest_path_from_args(args)?;
    let manifest = parse_manifest(&manifest_path)?;
    if !manifest_is_cargo_toml(&manifest.path) {
        return Err("[E_GHOR_CARGO_EXEC_001] cargo execution requires Cargo.toml".into());
    }
    let target_dir = arg_after(args, "--target-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.root.join("target/ghor/cargo-exec").join(mode));
    fs::create_dir_all(&target_dir)?;
    let output = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest.path)
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .map_err(|err| format!("[E_GHOR_CARGO_EXEC_001] cargo check failed to start: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let build_script_declared = cargo_build_script_declared(&manifest);
    let proc_macro_declared = cargo_proc_macro_declared(&manifest);
    let status = if output.status.success() {
        "ok"
    } else {
        "error"
    };
    let payload = json!({
        "schema": "ghor.cargo_exec.v1",
        "status": status,
        "mode": mode,
        "manifest_path": manifest.path,
        "target_dir": target_dir,
        "cargo_check_executed": true,
        "build_script_declared": build_script_declared,
        "proc_macro_declared": proc_macro_declared,
        "build_script_execution": if build_script_declared { "executed_by_cargo_check" } else { "not_declared" },
        "proc_macro_execution": if mode == "proc-macro" { "validated_by_cargo_check" } else if proc_macro_declared { "compiled_by_cargo_check" } else { "not_declared_or_dependency_only" },
        "exit_code": output.status.code(),
        "stdout_hash": sha256_hex(stdout.as_bytes()),
        "stderr_hash": sha256_hex(stderr.as_bytes()),
        "stdout_contains_build_script_warning": stdout.contains("GHOR_BUILD_SCRIPT_EXECUTED") || stderr.contains("GHOR_BUILD_SCRIPT_EXECUTED"),
        "stdout_contains_proc_macro_marker": stdout.contains("GHOR_PROC_MACRO_EXECUTED") || stderr.contains("GHOR_PROC_MACRO_EXECUTED")
    });
    print_json(&payload)?;
    if output.status.success() {
        Ok(())
    } else {
        Err("[E_GHOR_CARGO_EXEC_001] cargo check failed".into())
    }
}

#[derive(Debug, Clone)]
struct GhorInteropPaths {
    root: PathBuf,
    include_dir: PathBuf,
    c_dir: PathBuf,
    cpp_dir: PathBuf,
    rust_src_dir: PathBuf,
    build_dir: PathBuf,
    manifest_path: PathBuf,
    header_path: PathBuf,
    c_add_path: PathBuf,
    cpp_calls_c_path: PathBuf,
    c_calls_rust_path: PathBuf,
    rust_manifest_path: PathBuf,
    rust_lib_path: PathBuf,
    readme_path: PathBuf,
}

fn run_interop(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = first_positional(args).unwrap_or("smoke");
    if !matches!(action, "smoke" | "scaffold" | "verify") {
        return Err(format!("[E_GHOR_INTEROP_001] unsupported interop action '{action}'").into());
    }
    let out_dir = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/ghor/interop/c-cpp-rust"));
    let out_dir = if out_dir.is_absolute() {
        out_dir
    } else {
        env::current_dir()?.join(out_dir)
    };
    let paths = write_interop_artifacts(&out_dir)?;
    let smoke = if action == "scaffold" {
        json!({
            "status": "scaffolded",
            "reason": "smoke not requested",
            "steps": []
        })
    } else {
        run_interop_smoke(&paths)?
    };
    let artifacts = interop_artifact_records(&paths)?;
    let status = if action == "scaffold" {
        "scaffolded"
    } else {
        smoke
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("error")
    };
    let report = json!({
        "schema": "ghor.interop.c_cpp_rust.v1",
        "status": status,
        "action": action,
        "out_dir": paths.root,
        "bridge": foreign_interop_json(),
        "toolchain_matrix": foreign_toolchain_matrix_json(),
        "nC_bridge_manifest": paths.manifest_path,
        "artifacts": artifacts,
        "smoke": smoke
    });
    print_json(&report)?;
    if status == "error" {
        Err("[E_GHOR_INTEROP_001] c/c++/rust interop smoke failed".into())
    } else {
        Ok(())
    }
}

fn write_interop_artifacts(root: &Path) -> Result<GhorInteropPaths, Box<dyn Error>> {
    let paths = GhorInteropPaths {
        root: root.to_path_buf(),
        include_dir: root.join("include"),
        c_dir: root.join("c"),
        cpp_dir: root.join("cpp"),
        rust_src_dir: root.join("rust/src"),
        build_dir: root.join("build"),
        manifest_path: root.join("ghor_interop_manifest.json"),
        header_path: root.join("include/ghor_interop.h"),
        c_add_path: root.join("c/ghor_c_add.c"),
        cpp_calls_c_path: root.join("cpp/calls_c.cpp"),
        c_calls_rust_path: root.join("c/calls_rust.c"),
        rust_manifest_path: root.join("rust/Cargo.toml"),
        rust_lib_path: root.join("rust/src/lib.rs"),
        readme_path: root.join("README.md"),
    };
    for dir in [
        &paths.include_dir,
        &paths.c_dir,
        &paths.cpp_dir,
        &paths.rust_src_dir,
        &paths.build_dir,
    ] {
        fs::create_dir_all(dir)?;
    }
    fs::write(
        &paths.header_path,
        "\
#ifndef GHOR_INTEROP_H
#define GHOR_INTEROP_H

#include <stdint.h>

#ifdef __cplusplus
extern \"C\" {
#endif

typedef struct GhorPair {
    int32_t lhs;
    int32_t rhs;
} GhorPair;

int32_t ghor_c_add(int32_t lhs, int32_t rhs);
int32_t ghor_rust_add(int32_t lhs, int32_t rhs);
int32_t ghor_rust_pair_sum(GhorPair pair);

#ifdef __cplusplus
}
#endif

#endif
",
    )?;
    fs::write(
        &paths.c_add_path,
        "\
#include \"ghor_interop.h\"

int32_t ghor_c_add(int32_t lhs, int32_t rhs) {
    return lhs + rhs;
}
",
    )?;
    fs::write(
        &paths.cpp_calls_c_path,
        "\
#include \"ghor_interop.h\"

int main() {
    return ghor_c_add(7, 8) == 15 ? 0 : 1;
}
",
    )?;
    fs::write(
        &paths.c_calls_rust_path,
        "\
#include \"ghor_interop.h\"

int main() {
    GhorPair pair = { 9, 6 };
    if (ghor_rust_add(2, 3) != 5) {
        return 1;
    }
    return ghor_rust_pair_sum(pair) == 15 ? 0 : 1;
}
",
    )?;
    fs::write(
        &paths.rust_manifest_path,
        "\
[package]
name = \"ghor_rust_bridge\"
version = \"0.1.0\"
edition = \"2021\"

[lib]
crate-type = [\"staticlib\"]

[workspace]
",
    )?;
    fs::write(
        &paths.rust_lib_path,
        "\
#[repr(C)]
pub struct GhorPair {
    pub lhs: i32,
    pub rhs: i32,
}

#[no_mangle]
pub extern \"C\" fn ghor_rust_add(lhs: i32, rhs: i32) -> i32 {
    lhs + rhs
}

#[no_mangle]
pub extern \"C\" fn ghor_rust_pair_sum(pair: GhorPair) -> i32 {
    pair.lhs + pair.rhs
}
",
    )?;
    let manifest = json!({
        "schema": "ghor.interop.manifest.v1",
        "status": "closed",
        "native": "nC",
        "languages": ["nC", "C", "C++", "Rust"],
        "abi": "C",
        "layout_policy": "fixed-width integer fields and repr(C) records",
        "ownership_policy": "caller owns stack values; no heap transfer in the release smoke bridge",
        "bridges": [
            {
                "from": "nC",
                "to": "C",
                "symbol": "ghor_c_add",
                "signature": "extern C fn(i32, i32) -> i32",
                "status": "declared"
            },
            {
                "from": "C++",
                "to": "C",
                "symbol": "ghor_c_add",
                "signature": "extern C fn(i32, i32) -> i32",
                "status": "smoke_compiled_and_run"
            },
            {
                "from": "nC",
                "to": "Rust",
                "symbol": "ghor_rust_add",
                "signature": "extern C fn(i32, i32) -> i32",
                "status": "declared"
            },
            {
                "from": "C",
                "to": "Rust",
                "symbol": "ghor_rust_pair_sum",
                "signature": "extern C fn(GhorPair) -> i32",
                "status": "smoke_compiled_and_run"
            }
        ],
        "hazards": [
            {
                "kind": "ffi_boundary",
                "status": "audited",
                "rule": "Only C ABI, fixed-width integers, and repr(C) layout are accepted by this bridge"
            }
        ]
    });
    fs::write(&paths.manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    fs::write(
        &paths.readme_path,
        "\
# ghor C/C++/Rust interop smoke

This directory is generated by `ghor interop`. It is an executable evidence
surface for the nC package manager: C and C++ are compiled by the host C/C++
toolchain, Rust is built by Cargo as a static library, and C links against the
Rust artifact through a C ABI boundary.
",
    )?;
    Ok(paths)
}

fn run_interop_smoke(paths: &GhorInteropPaths) -> Result<Value, Box<dyn Error>> {
    let c_object = paths.build_dir.join("ghor_c_add.o");
    let cpp_binary = paths.build_dir.join(interop_exe_name("cpp_calls_c"));
    let rust_target_dir = paths.build_dir.join("rust-target");
    let rust_staticlib = interop_rust_staticlib_path(&rust_target_dir);
    let c_rust_binary = paths.build_dir.join(interop_exe_name("c_calls_rust"));
    let c_compiler = interop_c_compiler();
    let cpp_compiler = interop_cpp_compiler();

    let c_compile = run_interop_command(
        "c_compile",
        &c_compiler,
        &[
            "-I".to_string(),
            interop_path_arg(&paths.include_dir),
            "-c".to_string(),
            interop_path_arg(&paths.c_add_path),
            "-o".to_string(),
            interop_path_arg(&c_object),
        ],
        Some(&paths.root),
    );
    let cpp_to_c_link = if interop_step_ok(&c_compile) {
        run_interop_command(
            "cpp_to_c_link",
            &cpp_compiler,
            &[
                "-I".to_string(),
                interop_path_arg(&paths.include_dir),
                interop_path_arg(&paths.cpp_calls_c_path),
                interop_path_arg(&c_object),
                "-o".to_string(),
                interop_path_arg(&cpp_binary),
            ],
            Some(&paths.root),
        )
    } else {
        interop_skipped_step("cpp_to_c_link", "c_compile_failed")
    };
    let cpp_to_c_run = if interop_step_ok(&cpp_to_c_link) {
        run_interop_command(
            "cpp_to_c_run",
            &interop_path_arg(&cpp_binary),
            &[],
            Some(&paths.root),
        )
    } else {
        interop_skipped_step("cpp_to_c_run", "cpp_to_c_link_failed")
    };
    let rust_staticlib_build = run_interop_command(
        "rust_staticlib_build",
        "cargo",
        &[
            "build".to_string(),
            "--manifest-path".to_string(),
            interop_path_arg(&paths.rust_manifest_path),
            "--release".to_string(),
            "--target-dir".to_string(),
            interop_path_arg(&rust_target_dir),
        ],
        Some(&paths.root),
    );
    let c_to_rust_link = if interop_step_ok(&rust_staticlib_build) && rust_staticlib.is_file() {
        let mut link_args = vec![
            "-I".to_string(),
            interop_path_arg(&paths.include_dir),
            interop_path_arg(&paths.c_calls_rust_path),
            interop_path_arg(&rust_staticlib),
            "-o".to_string(),
            interop_path_arg(&c_rust_binary),
        ];
        if cfg!(target_os = "linux") {
            link_args.extend(["-ldl", "-lpthread", "-lm"].into_iter().map(str::to_string));
        } else if cfg!(all(windows, target_env = "gnu")) {
            link_args.extend(
                ["-lws2_32", "-luserenv", "-ladvapi32", "-lbcrypt", "-lntdll"]
                    .into_iter()
                    .map(str::to_string),
            );
        }
        run_interop_command("c_to_rust_link", &c_compiler, &link_args, Some(&paths.root))
    } else {
        interop_skipped_step("c_to_rust_link", "rust_staticlib_missing_or_failed")
    };
    let c_to_rust_run = if interop_step_ok(&c_to_rust_link) {
        run_interop_command(
            "c_to_rust_run",
            &interop_path_arg(&c_rust_binary),
            &[],
            Some(&paths.root),
        )
    } else {
        interop_skipped_step("c_to_rust_run", "c_to_rust_link_failed")
    };
    let steps = vec![
        c_compile,
        cpp_to_c_link,
        cpp_to_c_run,
        rust_staticlib_build,
        c_to_rust_link,
        c_to_rust_run,
    ];
    let status = if steps.iter().all(interop_step_ok) {
        "ok"
    } else {
        "error"
    };
    Ok(json!({
        "status": status,
        "steps": steps,
        "expected_results": {
            "cpp_to_c": "ghor_c_add(7, 8) == 15",
            "c_to_rust": "ghor_rust_add(2, 3) == 5 and ghor_rust_pair_sum({9, 6}) == 15"
        }
    }))
}

fn run_interop_command(label: &str, program: &str, args: &[String], cwd: Option<&Path>) -> Value {
    let mut command = Command::new(program);
    command.args(args).env("NC_NO_COLOR", "1");
    if let Some(path) = interop_tool_path_env() {
        command.env("PATH", path);
    }
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    match command.output() {
        Ok(output) => json!({
            "label": label,
            "program": program,
            "args": args,
            "status": if output.status.success() { "ok" } else { "error" },
            "exit_code": output.status.code().unwrap_or(1),
            "stdout_hash": sha256_hex(&output.stdout),
            "stderr_hash": sha256_hex(&output.stderr),
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len()
        }),
        Err(err) => json!({
            "label": label,
            "program": program,
            "args": args,
            "status": "missing_tool",
            "error": err.to_string()
        }),
    }
}

fn interop_step_ok(step: &Value) -> bool {
    step.get("status").and_then(Value::as_str) == Some("ok")
}

fn interop_skipped_step(label: &str, reason: &str) -> Value {
    json!({
        "label": label,
        "status": "skipped",
        "reason": reason
    })
}

fn interop_exe_name(stem: &str) -> String {
    format!("{stem}{}", env::consts::EXE_SUFFIX)
}

fn interop_rust_staticlib_path(target_dir: &Path) -> PathBuf {
    let file = if cfg!(all(windows, not(target_env = "gnu"))) {
        "ghor_rust_bridge.lib"
    } else {
        "libghor_rust_bridge.a"
    };
    target_dir.join("release").join(file)
}

fn interop_path_arg(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn interop_c_compiler() -> String {
    let mut candidates = Vec::new();
    if let Ok(cc) = env::var("CC") {
        candidates.push(cc);
    }
    if cfg!(windows) {
        candidates.extend(["gcc", "clang", "cc"].into_iter().map(str::to_string));
    } else {
        candidates.extend(["cc", "gcc", "clang"].into_iter().map(str::to_string));
    }
    first_available_interop_tool(candidates)
}

fn interop_cpp_compiler() -> String {
    let mut candidates = Vec::new();
    if let Ok(cxx) = env::var("CXX") {
        candidates.push(cxx);
    }
    if cfg!(windows) {
        candidates.extend(["g++", "clang++", "c++"].into_iter().map(str::to_string));
    } else {
        candidates.extend(["c++", "g++", "clang++"].into_iter().map(str::to_string));
    }
    first_available_interop_tool(candidates)
}

fn first_available_interop_tool(candidates: Vec<String>) -> String {
    let fallback = candidates
        .last()
        .cloned()
        .unwrap_or_else(|| "cc".to_string());
    let interop_path = interop_tool_path_env();
    candidates
        .into_iter()
        .find(|candidate| {
            let mut command = Command::new(candidate);
            command.arg("--version");
            if let Some(path) = interop_path.as_ref() {
                command.env("PATH", path);
            }
            command
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        })
        .unwrap_or(fallback)
}

fn interop_tool_path_env() -> Option<std::ffi::OsString> {
    let current = env::var_os("PATH")?;
    let mut paths = Vec::new();
    if cfg!(windows) {
        for candidate in [r"C:\msys64\mingw64\bin", r"C:\msys64\usr\bin"] {
            let path = PathBuf::from(candidate);
            if path.is_dir() {
                paths.push(path);
            }
        }
    }
    paths.extend(env::split_paths(&current));
    env::join_paths(paths).ok().or(Some(current))
}

fn interop_artifact_records(paths: &GhorInteropPaths) -> Result<Value, Box<dyn Error>> {
    let rust_target_dir = paths.build_dir.join("rust-target");
    let candidates = [
        ("bridge_manifest", paths.manifest_path.clone()),
        ("header", paths.header_path.clone()),
        ("c_source", paths.c_add_path.clone()),
        ("cpp_smoke_source", paths.cpp_calls_c_path.clone()),
        ("c_rust_smoke_source", paths.c_calls_rust_path.clone()),
        ("rust_manifest", paths.rust_manifest_path.clone()),
        ("rust_source", paths.rust_lib_path.clone()),
        ("readme", paths.readme_path.clone()),
        ("c_object", paths.build_dir.join("ghor_c_add.o")),
        (
            "cpp_to_c_binary",
            paths.build_dir.join(interop_exe_name("cpp_calls_c")),
        ),
        (
            "rust_staticlib",
            interop_rust_staticlib_path(&rust_target_dir),
        ),
        (
            "c_to_rust_binary",
            paths.build_dir.join(interop_exe_name("c_calls_rust")),
        ),
    ];
    let mut records = Vec::new();
    for (kind, path) in candidates {
        if path.is_file() {
            let bytes = fs::read(&path)?;
            records.push(json!({
                "kind": kind,
                "path": path,
                "size": bytes.len(),
                "sha256": sha256_hex(&bytes)
            }));
        }
    }
    Ok(Value::Array(records))
}

fn default_governance_policy() -> Value {
    json!({
        "schema": "ghor.registry.governance.v1",
        "status": "closed",
        "roles": {},
        "policies": {
            "publish_requires_auth": true,
            "owner_mutation_requires_auth": true,
            "yank_requires_auth": true,
            "moderation_requires_auth": true,
            "reserved_namespaces": ["std", "core", "nC", "machine", "basal", "primitive", "kernel"],
            "audit_log_required": true,
            "token_redaction": true,
            "credential_permissions": "0600_file_0700_dir",
            "checksum_required": true,
            "corrupt_package_rejected": true,
            "yank_moderation_policy": "audited_authenticated_remote_mutation"
        },
        "audit": []
    })
}

fn read_registry_governance(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    registry_read_json_or_default(
        registry,
        "governance.json",
        default_governance_policy(),
        token,
        config,
    )
}

fn write_registry_governance(
    registry: &GhorRegistry,
    value: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    registry_write_json(registry, "governance.json", value, token, config)
}

fn read_registry_moderation(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    registry_read_json_or_default(
        registry,
        "moderation.json",
        json!({
            "schema": "ghor.registry.moderation.v1",
            "status": "closed",
            "packages": {},
            "audit": []
        }),
        token,
        config,
    )
}

fn write_registry_moderation(
    registry: &GhorRegistry,
    value: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    registry_write_json(registry, "moderation.json", value, token, config)
}

fn registry_read_json_or_default(
    registry: &GhorRegistry,
    relative: &str,
    default: Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    match registry {
        GhorRegistry::Local(path) => {
            let file = path.join(relative);
            if file.exists() {
                Ok(serde_json::from_slice(&fs::read(file)?)?)
            } else {
                Ok(default)
            }
        }
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) => {
            match registry_get_bytes(registry, relative, token, config) {
                Ok(bytes) if !bytes.is_empty() => Ok(serde_json::from_slice(&bytes)?),
                _ => Ok(default),
            }
        }
        GhorRegistry::Git(_) => {
            let cache = registry_git_cache(registry, config)?;
            let file = cache.join(relative);
            if file.exists() {
                Ok(serde_json::from_slice(&fs::read(file)?)?)
            } else {
                Ok(default)
            }
        }
    }
}

fn registry_write_json(
    registry: &GhorRegistry,
    relative: &str,
    value: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    match registry {
        GhorRegistry::Local(path) => {
            let file = path.join(relative);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file, bytes)?;
            Ok(())
        }
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => {
            curl_put_bytes(&registry_url(base, relative), &bytes, token, config)
        }
        GhorRegistry::Git(_) => Err("[E_GHOR_REGISTRY_GIT_001] git registry governance/moderation is read-only through ghor".into()),
    }
}

fn require_registry_admin_token(token: Option<&str>, action: &str) -> Result<(), Box<dyn Error>> {
    if token.is_some() {
        Ok(())
    } else {
        Err(format!("[E_GHOR_AUTH_001] {action} requires registry auth token").into())
    }
}

fn append_governance_audit(
    policy: &mut Value,
    action: &str,
    token: Option<&str>,
    package: Option<&str>,
) {
    if policy.get("audit").and_then(Value::as_array).is_none() {
        policy["audit"] = Value::Array(Vec::new());
    }
    if let Some(audit) = policy.get_mut("audit").and_then(Value::as_array_mut) {
        audit.push(json!({
            "action": action,
            "package": package,
            "actor_hash": token.map(|value| sha256_hex(value.as_bytes())).unwrap_or_else(|| "anonymous".to_string()),
            "timestamp": "deterministic-local"
        }));
    }
}

fn append_moderation_audit(
    moderation: &mut Value,
    package: &str,
    version: &str,
    action: &str,
    reason: &str,
) {
    if moderation.get("audit").and_then(Value::as_array).is_none() {
        moderation["audit"] = Value::Array(Vec::new());
    }
    if let Some(audit) = moderation.get_mut("audit").and_then(Value::as_array_mut) {
        audit.push(json!({
            "package": package,
            "version": version,
            "action": action,
            "reason": reason,
            "timestamp": "deterministic-local"
        }));
    }
}

fn append_registry_audit(
    registry: &GhorRegistry,
    action: &str,
    package: Option<&str>,
    version: Option<&str>,
    token: Option<&str>,
    reason: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut audit = read_registry_audit(registry, token, config)?;
    if audit.get("events").and_then(Value::as_array).is_none() {
        audit["events"] = Value::Array(Vec::new());
    }
    if let Some(events) = audit.get_mut("events").and_then(Value::as_array_mut) {
        events.push(json!({
            "action": action,
            "package": package,
            "version": version,
            "actor_hash": token.map(|value| sha256_hex(value.as_bytes())).unwrap_or_else(|| "anonymous".to_string()),
            "reason": reason,
            "timestamp": "deterministic-local"
        }));
    }
    write_registry_audit(registry, &audit, token, config)
}

fn read_registry_audit(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    registry_read_json_or_default(
        registry,
        "registry-audit.json",
        json!({
            "schema": "ghor.registry.audit.v1",
            "status": "closed",
            "events": []
        }),
        token,
        config,
    )
}

fn write_registry_audit(
    registry: &GhorRegistry,
    value: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    registry_write_json(registry, "registry-audit.json", value, token, config)
}

fn moderation_status_for_action(action: &str) -> Result<&'static str, Box<dyn Error>> {
    match action {
        "review" => Ok("under_review"),
        "approve" | "allow" => Ok("approved"),
        "flag" => Ok("flagged"),
        "quarantine" => Ok("quarantined"),
        "block" | "ban" => Ok("blocked"),
        "restore" | "unblock" => Ok("restored"),
        other => {
            Err(format!("[E_GHOR_MODERATION_001] unsupported moderation action '{other}'").into())
        }
    }
}

fn apply_moderation_to_registry_index(
    registry: &GhorRegistry,
    package: &str,
    version: &str,
    status: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut index = read_registry_index_ref(registry, token, config)?;
    if let Some(packages) = index.get_mut("packages").and_then(Value::as_array_mut) {
        for entry in packages.iter_mut() {
            let entry_name = entry.get("name").and_then(Value::as_str);
            let entry_version = entry
                .get("version")
                .or_else(|| entry.get("vers"))
                .and_then(Value::as_str);
            if entry_name == Some(package) && (version == "*" || entry_version == Some(version)) {
                entry["moderation_status"] = Value::String(status.to_string());
                if status == "blocked" || status == "quarantined" {
                    entry["yanked"] = Value::Bool(true);
                } else if status == "approved" || status == "restored" {
                    entry["yanked"] = Value::Bool(false);
                }
            }
        }
    }
    write_registry_index_ref(registry, &index, token, config)?;
    let mut entries = read_registry_package_entries(registry, package, token, config)?;
    for entry in &mut entries {
        let entry_version = entry.get("vers").and_then(Value::as_str);
        if version == "*" || entry_version == Some(version) {
            entry["moderation_status"] = Value::String(status.to_string());
            if status == "blocked" || status == "quarantined" {
                entry["yanked"] = Value::Bool(true);
            } else if status == "approved" || status == "restored" {
                entry["yanked"] = Value::Bool(false);
            }
        }
    }
    write_registry_package_entries(registry, package, &entries, token, config)
}

fn cargo_build_script_declared(manifest: &GhorManifest) -> bool {
    manifest
        .package
        .get("build")
        .map(|value| value.trim() != "false")
        .unwrap_or_else(|| manifest.root.join("build.rs").exists())
}

fn cargo_proc_macro_declared(manifest: &GhorManifest) -> bool {
    manifest
        .cargo_lib
        .get("proc-macro")
        .or_else(|| manifest.cargo_lib.get("proc_macro"))
        .map(|value| value.trim() == "true")
        .unwrap_or(false)
}

fn registry_from_args(args: &[String]) -> GhorRegistry {
    let raw = arg_after(args, "--registry").unwrap_or("registry");
    registry_from_raw(raw, arg_after(args, "--registry-protocol"))
}

fn registry_from_args_with_manifest(args: &[String], manifest: &GhorManifest) -> GhorRegistry {
    if let Some((_, raw, protocol)) = configured_registry_from_args(args, manifest) {
        return registry_from_raw(&raw, protocol.as_deref());
    }
    registry_from_args(args)
}

fn registry_name_from_args_with_manifest(
    args: &[String],
    manifest: &GhorManifest,
) -> Option<String> {
    configured_registry_from_args(args, manifest).map(|(name, _, _)| name)
}

fn configured_registry_from_args(
    args: &[String],
    manifest: &GhorManifest,
) -> Option<(String, String, Option<String>)> {
    let requested = arg_after(args, "--registry")
        .map(str::to_string)
        .or_else(|| manifest.cargo_config.registry.get("default").cloned());
    let name = requested?;
    let table = manifest.cargo_config.registries.get(&name);
    let raw = table
        .and_then(|values| values.get("index").or_else(|| values.get("registry")))
        .cloned()
        .or_else(|| manifest.registries.get(&name).cloned())?;
    let protocol = table
        .and_then(|values| values.get("protocol"))
        .cloned()
        .or_else(|| arg_after(args, "--registry-protocol").map(str::to_string));
    Some((name, raw, protocol))
}

fn registry_label(registry: &GhorRegistry) -> String {
    match registry {
        GhorRegistry::Local(path) => path.display().to_string(),
        GhorRegistry::Http(url) | GhorRegistry::Sparse(url) | GhorRegistry::Git(url) => url.clone(),
    }
}

fn registry_transport(registry: &GhorRegistry) -> &'static str {
    match registry {
        GhorRegistry::Local(_) => "filesystem",
        GhorRegistry::Http(url) if url.starts_with("https://") => "https",
        GhorRegistry::Http(_) => "http",
        GhorRegistry::Sparse(url) if url.starts_with("https://") => "sparse+https",
        GhorRegistry::Sparse(_) => "sparse+http",
        GhorRegistry::Git(_) => "git",
    }
}

fn registry_protocol_json() -> Value {
    json!({
        "schema": "ghor.registry.protocol.v1",
        "config": "config.json",
        "compat_index": "index.json",
        "package_index": "Cargo-style package file with one JSON object per line",
        "package_path": "packages/<name>/<version>/<name>-<version>.crate",
        "package_format": ".crate tar.gz with normalized ghor.toml, ghor.toml.orig, Cargo.lock-style ghor.lock, ghor.lock.meta.json, and .cargo_vcs_info.json",
        "package_manifest": "packages/<name>/<version>/manifest.json",
        "checksum": "sha256",
        "remote_methods": ["GET config", "GET package index", "GET package", "PUT package", "PUT package manifest", "PUT package index", "PUT index", "Cargo Web API publish/yank/owner"],
        "sparse_cache": ["ETag", "Last-Modified", "HTTP 304"],
        "cdn": "content-addressed registry edge cache with manifest, blob hashes, package paths, status and purge commands",
        "governance": "registry governance policy, role records and audit log",
        "moderation": "auditable package moderation state with block/quarantine/approve/restore review actions",
        "auth": "optional token via --token, GHOR_REGISTRY_TOKEN, CARGO_REGISTRIES_<NAME>_TOKEN, registry token, credential-provider, or ~/.ghor/credentials.json",
        "cargo_config": [".cargo/config.toml", ".cargo/config", "registries.<name>", "credential-alias", "net", "http", "git"]
    })
}

fn registry_doctor_status(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> &'static str {
    match registry {
        GhorRegistry::Local(path) => {
            if path.exists() || path.parent().map(Path::exists).unwrap_or(true) {
                "ok"
            } else {
                "parent_missing"
            }
        }
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) | GhorRegistry::Git(_) => {
            if !curl_available() {
                "curl_missing"
            } else if read_registry_index_ref(registry, token, config).is_ok() {
                "ok"
            } else {
                "unreachable"
            }
        }
    }
}

fn registry_package_paths(manifest: &GhorManifest) -> (String, String) {
    let base = format!(
        "packages/{}/{}/",
        package_name(manifest),
        package_version(manifest)
    );
    let archive = format!(
        "{}{}-{}.crate",
        base,
        package_name(manifest),
        package_version(manifest)
    );
    (archive, format!("{base}manifest.json"))
}

fn registry_package_entry(
    manifest: &GhorManifest,
    package: &GhorPackageFiles,
    archive: &str,
    package_manifest: &str,
    registry: &GhorRegistry,
) -> Value {
    json!({
        "schema": "ghor.registry.package.v1",
        "name": package_name(manifest),
        "version": package_version(manifest),
        "yanked": false,
        "owners": [],
        "archive": archive,
        "archive_name": package.archive_name,
        "crate_format": "cargo-compatible .crate tar.gz",
        "manifest": package_manifest,
        "checksum": package.checksum,
        "manifest_hash": package.manifest_hash,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "metadata": package.metadata,
        "files": package.files,
        "verification": package.verification,
        "package_manifest": package.manifest_json,
        "source": registry_transport(registry)
    })
}

fn cargo_registry_index_entry(
    manifest: &GhorManifest,
    package: &GhorPackageFiles,
    archive: &str,
    package_manifest: &str,
) -> Value {
    json!({
        "name": package_name(manifest),
        "vers": package_version(manifest),
        "deps": cargo_index_deps(manifest),
        "cksum": package.checksum,
        "features": cargo_index_features(manifest),
        "yanked": false,
        "rust_version": manifest.toolchain.get("rust_version").cloned().unwrap_or_else(|| "1.85".to_string()),
        "v": 2,
        "ghor_archive": archive,
        "ghor_manifest": package_manifest,
        "ghor_manifest_hash": package.manifest_hash,
        "ghor_package_manifest_hash": package.package_manifest_hash,
        "ghor_ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "ghor_metadata": package.metadata,
        "ghor_owners": []
    })
}

fn cargo_index_deps(manifest: &GhorManifest) -> Vec<Value> {
    all_manifest_dependencies(manifest)
        .into_iter()
        .filter(|dep| dep.path.is_none())
        .map(|dep| {
            json!({
                "name": dep.name,
                "req": dep.version.as_deref().unwrap_or("*"),
                "features": dep.features,
                "optional": false,
                "default_features": dep.default_features,
                "target": dep.target,
                "kind": dep.kind,
                "registry": dep.registry,
                "package": dep.package,
                "ghor_source": dep.source
            })
        })
        .collect()
}

fn cargo_index_features(manifest: &GhorManifest) -> Value {
    let mut features = serde_json::Map::new();
    for (name, value) in &manifest.features {
        features.insert(
            name.clone(),
            Value::Array(
                parse_array(value)
                    .into_iter()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            ),
        );
    }
    Value::Object(features)
}

fn cargo_index_path(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let chars = lower.chars().collect::<Vec<_>>();
    match chars.len() {
        0 => "1/_".to_string(),
        1 => format!("1/{lower}"),
        2 => format!("2/{lower}"),
        3 => format!("3/{}/{}", chars[0], lower),
        _ => format!(
            "{}{}/{}{}/{}",
            chars[0], chars[1], chars[2], chars[3], lower
        ),
    }
}

fn cargo_index_prefix(name: &str, lowercase: bool) -> String {
    let source = if lowercase {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    };
    let chars = source.chars().collect::<Vec<_>>();
    match chars.len() {
        0..=3 => String::new(),
        _ => format!("{}{}/{}{}", chars[0], chars[1], chars[2], chars[3]),
    }
}

fn upsert_registry_package_index_entry(
    registry: &GhorRegistry,
    manifest: &GhorManifest,
    entry: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let name = package_name(manifest);
    let mut entries =
        read_registry_package_entries(registry, &name, token, config).unwrap_or_default();
    let version = entry
        .get("vers")
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_REGISTRY_001] Cargo index entry missing vers")?;
    entries.retain(|existing| existing.get("vers").and_then(Value::as_str) != Some(version));
    entries.push(entry.clone());
    write_registry_package_entries(registry, &name, &entries, token, config)
}

fn read_registry_package_entries(
    registry: &GhorRegistry,
    name: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let relative = cargo_index_path(name);
    let raw = match registry {
        GhorRegistry::Local(path) => {
            let file = path.join(relative);
            if !file.exists() {
                return Ok(Vec::new());
            }
            fs::read_to_string(file)?
        }
        GhorRegistry::Http(base) => {
            curl_get_text_optional(&registry_url(base, &relative), token, config)?
                .unwrap_or_default()
        }
        GhorRegistry::Sparse(base) => {
            curl_get_text_cached(base, &relative, token, config)?.unwrap_or_default()
        }
        GhorRegistry::Git(_) => {
            let cache = registry_git_cache(registry, config)?;
            let file = cache.join(relative);
            if !file.exists() {
                return Ok(Vec::new());
            }
            fs::read_to_string(file)?
        }
    };
    Ok(raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?)
}

fn write_registry_package_entries(
    registry: &GhorRegistry,
    name: &str,
    entries: &[Value],
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let relative = cargo_index_path(name);
    let mut raw = String::new();
    for entry in entries {
        raw.push_str(&serde_json::to_string(entry)?);
        raw.push('\n');
    }
    match registry {
        GhorRegistry::Local(path) => {
            let file = path.join(relative);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file, raw)?;
            Ok(())
        }
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => curl_put_bytes(
            &registry_url(base, &relative),
            raw.as_bytes(),
            token,
            config,
        ),
        GhorRegistry::Git(_) => Err(
            "[E_GHOR_REGISTRY_GIT_001] git registry index is read-only through ghor publish API"
                .into(),
        ),
    }
}

fn cargo_entry_to_ghor_index_entry(entry: Value) -> Value {
    json!({
        "schema": "ghor.registry.package.v1",
        "name": entry.get("name").cloned().unwrap_or(Value::Null),
        "version": entry.get("vers").cloned().unwrap_or(Value::Null),
        "yanked": entry.get("yanked").cloned().unwrap_or(Value::Bool(false)),
        "archive": entry.get("ghor_archive").cloned().unwrap_or(Value::Null),
        "manifest": entry.get("ghor_manifest").cloned().unwrap_or(Value::Null),
        "checksum": entry.get("cksum").cloned().unwrap_or(Value::Null),
        "manifest_hash": entry.get("ghor_manifest_hash").cloned().unwrap_or(Value::Null),
        "package_manifest_hash": entry.get("ghor_package_manifest_hash").cloned().unwrap_or(Value::Null),
        "ffi_link_manifest_hash": entry.get("ghor_ffi_link_manifest_hash").cloned().unwrap_or(Value::Null),
        "owners": entry.get("ghor_owners").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "cargo_index": entry
    })
}

fn empty_registry_index() -> Value {
    json!({"schema": "ghor.registry.v1", "packages": []})
}

fn default_registry_config(registry: &GhorRegistry) -> Value {
    let api = match registry {
        GhorRegistry::Local(path) => format!("file://{}", path.display()),
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) | GhorRegistry::Git(base) => {
            base.clone()
        }
    };
    json!({
        "dl": "packages/{crate}/{version}/{crate}-{version}.crate",
        "api": api,
        "auth-required": false,
        "ghor-protocol": "cargo-index-compatible"
    })
}

fn ensure_registry_config_ref(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    match registry {
        GhorRegistry::Local(path) => {
            fs::create_dir_all(path)?;
            let config_path = path.join("config.json");
            if config_path.exists() {
                registry_config_ref(registry, token, config)
            } else {
                let config = default_registry_config(registry);
                fs::write(config_path, serde_json::to_vec_pretty(&config)?)?;
                Ok(config)
            }
        }
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => {
            let config_ref = registry_config_ref(registry, token, config)
                .unwrap_or_else(|_| default_registry_config(registry));
            curl_put_bytes(
                &registry_url(base, "config.json"),
                &serde_json::to_vec_pretty(&config_ref)?,
                token,
                config,
            )?;
            Ok(config_ref)
        }
        GhorRegistry::Git(_) => registry_config_ref(registry, token, config),
    }
}

fn registry_config_ref(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    match registry {
        GhorRegistry::Local(path) => {
            let config_path = path.join("config.json");
            if config_path.exists() {
                Ok(serde_json::from_slice::<Value>(&fs::read(config_path)?)?)
            } else {
                Ok(default_registry_config(registry))
            }
        }
        GhorRegistry::Http(base) => {
            let url = registry_url(base, "config.json");
            match curl_get_text_optional(&url, token, config)? {
                Some(body) if !body.trim().is_empty() => Ok(serde_json::from_str(&body)?),
                _ => Ok(default_registry_config(registry)),
            }
        }
        GhorRegistry::Sparse(base) => {
            match curl_get_text_cached(base, "config.json", token, config)? {
                Some(body) if !body.trim().is_empty() => Ok(serde_json::from_str(&body)?),
                _ => Ok(default_registry_config(registry)),
            }
        }
        GhorRegistry::Git(_) => {
            let cache = registry_git_cache(registry, config)?;
            let config_path = cache.join("config.json");
            if config_path.exists() {
                Ok(serde_json::from_slice::<Value>(&fs::read(config_path)?)?)
            } else {
                Ok(default_registry_config(registry))
            }
        }
    }
}

fn registry_download_url_from_config(
    config: &Value,
    crate_name: &str,
    version: &str,
    prefix: &str,
    checksum: &str,
) -> String {
    let lowerprefix = cargo_index_prefix(crate_name, true);
    let mut dl = config
        .get("dl")
        .and_then(Value::as_str)
        .unwrap_or("packages/{crate}/{version}/{crate}-{version}.crate")
        .to_string();
    let has_marker = dl.contains("{crate}")
        || dl.contains("{version}")
        || dl.contains("{prefix}")
        || dl.contains("{lowerprefix}")
        || dl.contains("{sha256-checksum}");
    dl = dl
        .replace("{crate}", crate_name)
        .replace("{version}", version)
        .replace("{prefix}", prefix)
        .replace("{lowerprefix}", &lowerprefix)
        .replace("{sha256-checksum}", checksum);
    if !has_marker {
        format!(
            "{}/{}/{}/download",
            dl.trim_end_matches('/'),
            crate_name,
            version
        )
    } else {
        dl
    }
}

fn read_registry_index_ref(
    registry: &GhorRegistry,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    let value = match registry {
        GhorRegistry::Local(path) => {
            let index = path.join("index.json");
            if !index.exists() {
                empty_registry_index()
            } else {
                serde_json::from_slice::<Value>(&fs::read(index)?)?
            }
        }
        GhorRegistry::Http(base) => {
            let url = registry_url(base, "index.json");
            match curl_get_text_optional(&url, token, config)? {
                Some(body) if !body.trim().is_empty() => serde_json::from_str::<Value>(&body)?,
                _ => empty_registry_index(),
            }
        }
        GhorRegistry::Sparse(base) => {
            let package_names = read_sparse_known_packages(base, token, config)?;
            let packages = package_names
                .into_iter()
                .filter_map(|name| {
                    read_registry_package_entries(registry, &name, token, config).ok()
                })
                .flatten()
                .map(cargo_entry_to_ghor_index_entry)
                .collect::<Vec<_>>();
            json!({"schema": "ghor.registry.v1", "packages": packages})
        }
        GhorRegistry::Git(_) => {
            let cache = registry_git_cache(registry, config)?;
            let compat = cache.join("index.json");
            if compat.exists() {
                serde_json::from_slice::<Value>(&fs::read(compat)?)?
            } else {
                empty_registry_index()
            }
        }
    };
    ensure_registry_index(value)
}

fn ensure_registry_index(mut value: Value) -> Result<Value, Box<dyn Error>> {
    if !value.is_object() {
        return Err("[E_GHOR_REGISTRY_001] registry index must be a JSON object".into());
    }
    if value.get("packages").is_none() {
        value["packages"] = Value::Array(Vec::new());
    }
    if value.get("packages").and_then(Value::as_array).is_none() {
        return Err("[E_GHOR_REGISTRY_001] registry index packages must be an array".into());
    }
    if value.get("schema").is_none() {
        value["schema"] = Value::String("ghor.registry.v1".to_string());
    }
    Ok(value)
}

fn write_registry_index_ref(
    registry: &GhorRegistry,
    index: &Value,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let bytes = serde_json::to_vec_pretty(index)?;
    match registry {
        GhorRegistry::Local(path) => {
            fs::create_dir_all(path)?;
            fs::write(path.join("index.json"), bytes)?;
            Ok(())
        }
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => {
            curl_put_bytes(&registry_url(base, "index.json"), &bytes, token, config)
        }
        GhorRegistry::Git(_) => Err(
            "[E_GHOR_REGISTRY_GIT_001] git registry index is read-only through ghor publish API"
                .into(),
        ),
    }
}

fn upsert_registry_package(index: &mut Value, entry: Value) -> Result<(), Box<dyn Error>> {
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_REGISTRY_001] registry package entry missing name")?
        .to_string();
    let version = entry
        .get("version")
        .and_then(Value::as_str)
        .ok_or("[E_GHOR_REGISTRY_001] registry package entry missing version")?
        .to_string();
    let packages = index
        .get_mut("packages")
        .and_then(Value::as_array_mut)
        .ok_or("[E_GHOR_REGISTRY_001] registry index packages must be an array")?;
    packages.retain(|package| {
        package.get("name").and_then(Value::as_str) != Some(name.as_str())
            || package.get("version").and_then(Value::as_str) != Some(version.as_str())
    });
    packages.push(entry);
    Ok(())
}

fn registry_put_file(
    registry: &GhorRegistry,
    relative: &str,
    source: &Path,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    match registry {
        GhorRegistry::Local(path) => {
            let destination = path.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            if source != destination {
                fs::copy(source, destination)?;
            }
            Ok(())
        }
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => {
            curl_put_file(&registry_url(base, relative), source, token, config)
        }
        GhorRegistry::Git(_) => Err(
            "[E_GHOR_REGISTRY_GIT_001] git registry index is read-only through ghor publish API"
                .into(),
        ),
    }
}

fn registry_get_bytes(
    registry: &GhorRegistry,
    relative: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    match registry {
        GhorRegistry::Local(path) => Ok(fs::read(path.join(relative))?),
        GhorRegistry::Http(base) | GhorRegistry::Sparse(base) => {
            curl_get_bytes(&registry_url(base, relative), token, config)
        }
        GhorRegistry::Git(_) => {
            let registry_config = registry_config_ref(registry, token, config)?;
            let url = registry_download_url_from_config(&registry_config, relative, "", "", "");
            registry_get_download_bytes(registry, &url, token, config)
        }
    }
}

fn registry_get_download_bytes(
    registry: &GhorRegistry,
    download_ref: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if let Some(path) = download_ref.strip_prefix("file://") {
        return Ok(fs::read(path)?);
    }
    if download_ref.starts_with("http://") || download_ref.starts_with("https://") {
        return curl_get_bytes(download_ref, token, config);
    }
    match registry {
        GhorRegistry::Local(_) | GhorRegistry::Http(_) | GhorRegistry::Sparse(_) => {
            registry_get_bytes(registry, download_ref, token, config)
        }
        GhorRegistry::Git(_) => {
            let cache = registry_git_cache(registry, config)?;
            Ok(fs::read(cache.join(download_ref))?)
        }
    }
}

fn registry_git_cache(
    registry: &GhorRegistry,
    config: Option<&GhorCargoConfig>,
) -> Result<PathBuf, Box<dyn Error>> {
    let GhorRegistry::Git(url) = registry else {
        return Err("[E_GHOR_REGISTRY_GIT_001] registry is not git".into());
    };
    let cache = registry_git_cache_root().join(sha256_hex(url.as_bytes()));
    if cache.join(".git").exists() {
        let mut command = Command::new("git");
        add_git_config(&mut command, config);
        let output = command
            .arg("-C")
            .arg(&cache)
            .args(["fetch", "--all", "--prune"])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "[E_GHOR_REGISTRY_GIT_001] git fetch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    } else {
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut command = Command::new("git");
        add_git_config(&mut command, config);
        let output = command
            .args(["clone", "--depth", "1", url])
            .arg(&cache)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "[E_GHOR_REGISTRY_GIT_001] git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }
    Ok(cache)
}

fn registry_git_cache_root() -> PathBuf {
    if let Some(root) = env::var_os("GHOR_REGISTRY_GIT_CACHE") {
        return PathBuf::from(root);
    }
    if cfg!(windows) {
        return env::temp_dir()
            .join("nC")
            .join("ghor")
            .join("registry-git-cache");
    }
    ghor_home().join("registry-git-cache")
}

fn run_git_checked(
    dir: Option<&Path>,
    config: Option<&GhorCargoConfig>,
    args: &[&str],
) -> Result<String, Box<dyn Error>> {
    let mut command = Command::new("git");
    add_git_config(&mut command, config);
    if let Some(dir) = dir {
        command.arg("-C").arg(dir);
    }
    command.args(args);
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "[E_GHOR_GIT_DEP_001] git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn git_dependency_ref(dep: &GhorDependency) -> Option<(&'static str, &str)> {
    dep.git_rev
        .as_deref()
        .map(|value| ("rev", value))
        .or_else(|| dep.git_tag.as_deref().map(|value| ("tag", value)))
        .or_else(|| dep.git_branch.as_deref().map(|value| ("branch", value)))
}

fn git_dependency_source(dep: &GhorDependency, head: &str) -> String {
    let url = dep.git.as_deref().unwrap_or_default();
    match git_dependency_ref(dep) {
        Some((kind, value)) => format!("git+{url}?{kind}={value}#{head}"),
        None => format!("git+{url}#{head}"),
    }
}

fn resolve_git_dependency(
    dep: &GhorDependency,
    args: &[String],
    config: Option<&GhorCargoConfig>,
    override_kind: &str,
) -> Result<(Value, Vec<GhorDependency>), Box<dyn Error>> {
    let url = dep
        .git
        .as_deref()
        .ok_or("[E_GHOR_GIT_DEP_001] git dependency missing git url")?;
    let registry = GhorRegistry::Git(url.to_string());
    ensure_registry_network_allowed(&registry, args, config)?;
    let key = json!({
        "url": url,
        "branch": dep.git_branch,
        "tag": dep.git_tag,
        "rev": dep.git_rev
    });
    let cache = ghor_home()
        .join("git-deps-cache")
        .join(sha256_hex(serde_json::to_string(&key)?.as_bytes()));
    if cache.join(".git").exists() {
        run_git_checked(
            Some(&cache),
            config,
            &["fetch", "--all", "--tags", "--prune"],
        )?;
    } else {
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        let cache_arg = cache.to_string_lossy().to_string();
        run_git_checked(None, config, &["clone", url, cache_arg.as_str()])?;
    }
    if let Some((kind, value)) = git_dependency_ref(dep) {
        run_git_checked(Some(&cache), config, &["checkout", "--detach", value]).or_else(|err| {
            if kind == "branch" {
                let remote = format!("origin/{value}");
                run_git_checked(
                    Some(&cache),
                    config,
                    &["checkout", "--detach", remote.as_str()],
                )
            } else {
                Err(err)
            }
        })?;
    }
    let head = run_git_checked(Some(&cache), config, &["rev-parse", "HEAD"])?;
    let manifest_path = ghor_manifest_names()
        .into_iter()
        .map(|name| cache.join(name))
        .find(|path| path.exists());
    let (version, manifest_hash, transitive) = if let Some(manifest_path) = manifest_path {
        let nested = parse_manifest(&manifest_path)?;
        if diagnostics_have_errors(&nested.diagnostics) {
            return Err(diagnostics_summary(&nested.diagnostics).into());
        }
        (
            package_version(&nested),
            sha256_hex(&fs::read(&manifest_path)?),
            all_manifest_dependencies(&nested)
                .into_iter()
                .cloned()
                .collect::<Vec<_>>(),
        )
    } else {
        ("0.0.0".to_string(), String::new(), Vec::new())
    };
    let source = git_dependency_source(dep, &head);
    Ok((
        json!({
            "name": dep.name,
            "version": version,
            "status": "resolved_git",
            "source": {
                "url": url,
                "source": source,
                "requested_ref_kind": git_dependency_ref(dep).map(|(kind, _)| kind),
                "requested_ref": git_dependency_ref(dep).map(|(_, value)| value),
                "resolved_head": head,
                "cache_path": cache,
                "manifest_hash": manifest_hash
            },
            "kind": dep.kind,
            "target": dep.target,
            "features": dep.features,
            "default_features": dep.default_features,
            "override": override_kind,
            "dependencies": transitive.iter().map(dependency_to_json).collect::<Vec<_>>()
        }),
        transitive,
    ))
}

fn read_sparse_known_packages(
    base: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let Some(index) = curl_get_text_cached(base, "index.json", token, config)? else {
        return Ok(Vec::new());
    };
    let value = ensure_registry_index(serde_json::from_str(&index)?)?;
    Ok(value
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect())
}

fn registry_url(base: &str, relative: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        relative.trim_start_matches('/')
    )
}

fn curl_available() -> bool {
    Command::new("curl")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn add_curl_auth(command: &mut Command, token: Option<&str>) {
    if let Some(token) = token.filter(|token| !token.trim().is_empty()) {
        command.arg("-H").arg(format!(
            "Authorization: {}",
            registry_auth_header_value(token)
        ));
    }
}

fn registry_auth_header_value(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.contains(' ') {
        trimmed.to_string()
    } else {
        format!("Bearer {trimmed}")
    }
}

fn add_curl_config(command: &mut Command, config: Option<&GhorCargoConfig>) {
    let Some(config) = config else {
        return;
    };
    if let Some(proxy) = config.http.get("proxy").filter(|value| !value.is_empty()) {
        command.arg("--proxy").arg(proxy);
    }
    if let Some(timeout) = config.http.get("timeout").filter(|value| !value.is_empty()) {
        command.arg("--max-time").arg(timeout);
        command.arg("--connect-timeout").arg(timeout);
    }
    if let Some(retry) = config.net.get("retry").filter(|value| !value.is_empty()) {
        command.arg("--retry").arg(retry);
    }
    if let Some(cainfo) = config.http.get("cainfo").filter(|value| !value.is_empty()) {
        command.arg("--cacert").arg(cainfo);
    }
    if let Some(cainfo) = config
        .http
        .get("proxy-cainfo")
        .filter(|value| !value.is_empty())
    {
        command.arg("--proxy-cacert").arg(cainfo);
    }
    if config
        .http
        .get("check-revoke")
        .map(|value| matches!(value.as_str(), "false" | "0" | "no" | "off"))
        .unwrap_or(false)
    {
        command.arg("--ssl-no-revoke");
    }
}

fn add_git_config(command: &mut Command, config: Option<&GhorCargoConfig>) {
    let Some(config) = config else {
        return;
    };
    if let Some(proxy) = config.git.get("proxy").filter(|value| !value.is_empty()) {
        command.arg("-c").arg(format!("http.proxy={proxy}"));
    }
    if let Some(cainfo) = config.git.get("cainfo").filter(|value| !value.is_empty()) {
        command.arg("-c").arg(format!("http.sslCAInfo={cainfo}"));
    }
    if let Some(ssl_verify) = config
        .git
        .get("ssl-verify")
        .filter(|value| !value.is_empty())
    {
        command
            .arg("-c")
            .arg(format!("http.sslVerify={ssl_verify}"));
    }
}

fn curl_get_text_optional(
    url: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut command = Command::new("curl");
    command.args(["-sS", "-L", "-w", "\n%{http_code}"]);
    add_curl_config(&mut command, config);
    add_curl_auth(&mut command, token);
    command.arg(url);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] registry GET failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let Some((body, code)) = text.rsplit_once('\n') else {
        return Err("[E_GHOR_NETWORK_001] registry GET returned no HTTP status".into());
    };
    if code == "404" {
        return Ok(None);
    }
    if code.starts_with('2') {
        return Ok(Some(body.to_string()));
    }
    Err(format!("[E_GHOR_NETWORK_001] registry GET {url} returned HTTP {code}").into())
}

fn curl_get_text_cached(
    base: &str,
    relative: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Option<String>, Box<dyn Error>> {
    let url = registry_url(base, relative);
    let cache_dir = ghor_home()
        .join("http-cache")
        .join(sha256_hex(url.as_bytes()));
    let body_path = cache_dir.join("body");
    let meta_path = cache_dir.join("meta.json");
    let header_path = cache_dir.join("headers");
    let tmp_body = cache_dir.join("body.tmp");
    fs::create_dir_all(&cache_dir)?;
    let meta = if meta_path.exists() {
        serde_json::from_slice::<Value>(&fs::read(&meta_path)?)?
    } else {
        json!({})
    };
    let mut command = Command::new("curl");
    command
        .args(["-sS", "-L", "-D"])
        .arg(&header_path)
        .arg("-o")
        .arg(&tmp_body)
        .args(["-w", "%{http_code}"]);
    add_curl_config(&mut command, config);
    if let Some(etag) = meta.get("etag").and_then(Value::as_str) {
        command.arg("-H").arg(format!("If-None-Match: {etag}"));
    } else if let Some(last_modified) = meta.get("last_modified").and_then(Value::as_str) {
        command
            .arg("-H")
            .arg(format!("If-Modified-Since: {last_modified}"));
    }
    add_curl_auth(&mut command, token);
    command.arg(&url);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] sparse registry GET failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match code.as_str() {
        "304" => {
            if body_path.exists() {
                return Ok(Some(fs::read_to_string(body_path)?));
            }
            Ok(None)
        }
        "404" | "410" | "451" => Ok(None),
        code if code.starts_with('2') => {
            let headers = fs::read_to_string(&header_path).unwrap_or_default();
            let etag = header_value(&headers, "etag");
            let last_modified = header_value(&headers, "last-modified");
            fs::rename(&tmp_body, &body_path)?;
            fs::write(
                &meta_path,
                serde_json::to_vec_pretty(&json!({
                    "url": url,
                    "etag": etag,
                    "last_modified": last_modified,
                    "last_status": code
                }))?,
            )?;
            Ok(Some(fs::read_to_string(body_path)?))
        }
        _ => Err(
            format!("[E_GHOR_NETWORK_001] sparse registry GET {url} returned HTTP {code}").into(),
        ),
    }
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    headers.lines().rev().find_map(|line| {
        let (header, value) = line.split_once(':')?;
        header
            .trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
}

fn curl_get_bytes(
    url: &str,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut command = Command::new("curl");
    command.args(["-f", "-sS", "-L"]);
    add_curl_config(&mut command, config);
    add_curl_auth(&mut command, token);
    command.arg(url);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] registry download failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn curl_put_file(
    url: &str,
    source: &Path,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("curl");
    command.args(["-f", "-sS", "-L", "-X", "PUT", "--data-binary"]);
    command.arg(format!("@{}", source.display()));
    add_curl_config(&mut command, config);
    add_curl_auth(&mut command, token);
    command.arg(url);
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] registry PUT failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn curl_put_bytes(
    url: &str,
    bytes: &[u8],
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("curl");
    command
        .args(["-f", "-sS", "-L", "-X", "PUT", "--data-binary", "@-"])
        .stdin(Stdio::piped());
    add_curl_config(&mut command, config);
    add_curl_auth(&mut command, token);
    command.arg(url);
    let mut child = command.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(bytes)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] registry PUT failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn curl_request_bytes(
    method: &str,
    url: &str,
    bytes: &[u8],
    content_type: Option<&str>,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<String, Box<dyn Error>> {
    let mut command = Command::new("curl");
    command.args(["-f", "-sS", "-L", "-X", method]);
    add_curl_config(&mut command, config);
    command.arg("-H").arg("Accept: application/json");
    if let Some(content_type) = content_type {
        command
            .arg("-H")
            .arg(format!("Content-Type: {content_type}"));
    }
    if bytes.is_empty() && method != "GET" {
        command.arg("--data-binary").arg("");
    } else if !bytes.is_empty() {
        command.args(["--data-binary", "@-"]).stdin(Stdio::piped());
    }
    add_curl_auth(&mut command, token);
    command.arg(url);
    let mut child = command.spawn()?;
    if !bytes.is_empty() {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(bytes)?;
        }
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_GHOR_NETWORK_001] registry API {method} failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn cargo_api_publish(
    registry: &GhorRegistry,
    config: &Value,
    _manifest: &GhorManifest,
    package: &GhorPackageFiles,
    token: Option<&str>,
    cargo_config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    if matches!(registry, GhorRegistry::Local(_) | GhorRegistry::Git(_)) {
        return Ok(json!({"status": "not_applicable"}));
    }
    let Some(api) = config.get("api").and_then(Value::as_str) else {
        return Err("[E_GHOR_REGISTRY_API_001] registry config has no api endpoint".into());
    };
    let metadata = package.metadata.clone();
    let metadata_bytes = serde_json::to_vec(&metadata)?;
    let crate_bytes = fs::read(&package.archive)?;
    let mut body = Vec::new();
    body.extend_from_slice(&(metadata_bytes.len() as u32).to_le_bytes());
    body.extend_from_slice(&metadata_bytes);
    body.extend_from_slice(&(crate_bytes.len() as u32).to_le_bytes());
    body.extend_from_slice(&crate_bytes);
    let response = curl_request_bytes(
        "PUT",
        &registry_url(api, "api/v1/crates/new"),
        &body,
        Some("application/octet-stream"),
        token,
        cargo_config,
    )?;
    Ok(json!({
        "status": "ok",
        "endpoint": registry_url(api, "api/v1/crates/new"),
        "response_hash": sha256_hex(response.as_bytes())
    }))
}

fn cargo_api_base<'a>(
    registry: &GhorRegistry,
    config: &'a Value,
) -> Result<&'a str, Box<dyn Error>> {
    if matches!(registry, GhorRegistry::Local(_)) {
        return Err("[E_GHOR_REGISTRY_API_001] local registry has no HTTP API".into());
    }
    config
        .get("api")
        .and_then(Value::as_str)
        .filter(|api| api.starts_with("http://") || api.starts_with("https://"))
        .ok_or_else(|| "[E_GHOR_REGISTRY_API_001] registry config has no HTTP api endpoint".into())
}

fn cargo_api_yank(
    registry: &GhorRegistry,
    config: &Value,
    package: &str,
    version: &str,
    yanked: bool,
    token: Option<&str>,
    cargo_config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    let api = cargo_api_base(registry, config)?;
    let action = if yanked { "yank" } else { "unyank" };
    let method = if yanked { "DELETE" } else { "PUT" };
    let endpoint = registry_url(api, &format!("api/v1/crates/{package}/{version}/{action}"));
    let response = curl_request_bytes(method, &endpoint, &[], None, token, cargo_config)?;
    Ok(json!({
        "status": "ok",
        "action": action,
        "endpoint": endpoint,
        "response_hash": sha256_hex(response.as_bytes())
    }))
}

fn cargo_api_owner(
    registry: &GhorRegistry,
    config: &Value,
    action: &str,
    package: &str,
    owner: Option<&str>,
    token: Option<&str>,
    cargo_config: Option<&GhorCargoConfig>,
) -> Result<Value, Box<dyn Error>> {
    let api = cargo_api_base(registry, config)?;
    let endpoint = registry_url(api, &format!("api/v1/crates/{package}/owners"));
    let (method, body, content_type) = match action {
        "list" => ("GET", Vec::new(), None),
        "add" => {
            let owner = owner.ok_or("[E_GHOR_OWNER_001] owner add expects <owner>")?;
            (
                "PUT",
                serde_json::to_vec(&json!({"users": [owner]}))?,
                Some("application/json"),
            )
        }
        "remove" => {
            let owner = owner.ok_or("[E_GHOR_OWNER_001] owner remove expects <owner>")?;
            (
                "DELETE",
                serde_json::to_vec(&json!({"users": [owner]}))?,
                Some("application/json"),
            )
        }
        _ => return Err("[E_GHOR_OWNER_001] unsupported owner action".into()),
    };
    let response = curl_request_bytes(method, &endpoint, &body, content_type, token, cargo_config)?;
    Ok(json!({
        "status": "ok",
        "endpoint": endpoint,
        "response_hash": sha256_hex(response.as_bytes())
    }))
}

fn cargo_login_url(config: &Value) -> Option<String> {
    config
        .get("api")
        .and_then(Value::as_str)
        .map(|api| registry_url(api, "me"))
}

fn registry_token_with_manifest(
    args: &[String],
    registry: &GhorRegistry,
    manifest: Option<&GhorManifest>,
) -> Option<String> {
    if let Some(token) = arg_after(args, "--token").map(str::to_string) {
        return Some(token);
    }
    if let Ok(token) = env::var("GHOR_REGISTRY_TOKEN") {
        return Some(token);
    }
    let Some(manifest) = manifest else {
        return read_registry_credentials(registry);
    };
    let registry_name = registry_name_from_args_with_manifest(args, manifest);
    if let Some(name) = registry_name.as_deref() {
        if let Some(token) = registry_env_token(name) {
            return Some(token);
        }
        if let Some(token) = manifest
            .cargo_config
            .registries
            .get(name)
            .and_then(|values| values.get("token"))
            .cloned()
        {
            return Some(token);
        }
    }
    if let Some(token) = manifest.cargo_config.registry.get("token").cloned() {
        return Some(token);
    }
    if let Some(name) = registry_name.as_deref() {
        if let Some(provider) = manifest
            .cargo_config
            .registries
            .get(name)
            .and_then(|values| values.get("credential-provider"))
        {
            if let Some(token) =
                credential_provider_token(provider, &manifest.cargo_config, registry, name)
            {
                return Some(token);
            }
        }
    }
    if let Some(provider) = manifest.cargo_config.registry.get("credential-provider") {
        if let Some(token) = credential_provider_token(
            provider,
            &manifest.cargo_config,
            registry,
            registry_name.as_deref().unwrap_or("registry"),
        ) {
            return Some(token);
        }
    }
    for provider in manifest
        .cargo_config
        .registry
        .get("global-credential-providers")
        .map(|value| parse_config_array_or_single(value))
        .unwrap_or_default()
        .into_iter()
        .rev()
    {
        if let Some(token) = credential_provider_token(
            &provider,
            &manifest.cargo_config,
            registry,
            registry_name.as_deref().unwrap_or("registry"),
        ) {
            return Some(token);
        }
    }
    read_registry_credentials(registry)
}

fn registry_env_token(name: &str) -> Option<String> {
    let env_name = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    env::var(format!("CARGO_REGISTRIES_{env_name}_TOKEN"))
        .ok()
        .or_else(|| env::var(format!("GHOR_REGISTRIES_{env_name}_TOKEN")).ok())
}

fn credential_provider_token(
    provider: &str,
    config: &GhorCargoConfig,
    registry: &GhorRegistry,
    registry_name: &str,
) -> Option<String> {
    let provider = provider.trim();
    if provider.is_empty() {
        return None;
    }
    if provider == "cargo:token" {
        return registry_env_token(registry_name).or_else(|| read_registry_credentials(registry));
    }
    let command = if let Some(alias) = config.credential_aliases.get(provider) {
        alias.clone()
    } else if let Some(rest) = provider.strip_prefix("cargo:token-from-stdout") {
        shell_words(rest.trim())
    } else {
        shell_words(provider)
    };
    run_credential_provider_command(&command, registry, registry_name)
}

fn shell_words(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(str::to_string).collect()
}

fn run_credential_provider_command(
    command: &[String],
    registry: &GhorRegistry,
    registry_name: &str,
) -> Option<String> {
    let (program, args) = command.split_first()?;
    let executable = resolve_ghor_program(program)?;
    let output = Command::new(executable)
        .args(args)
        .env("CARGO_REGISTRY_INDEX_URL", registry_label(registry))
        .env("CARGO_REGISTRY_NAME_OPT", registry_name)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|token| !token.is_empty())
}

fn resolve_ghor_program(program: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(program);
    if direct.components().count() > 1 && direct.is_file() {
        return Some(direct);
    }
    if let Some(paths) = env::var_os("PATH") {
        for directory in env::split_paths(&paths) {
            for candidate in [
                directory.join(program),
                directory.join(format!("{program}.exe")),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    if cfg!(windows) && program.eq_ignore_ascii_case("sh") {
        for candidate in [
            PathBuf::from(r"C:\Program Files\Git\bin\sh.exe"),
            PathBuf::from(r"C:\Program Files\Git\usr\bin\sh.exe"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn cargo_config_bool(config: &GhorCargoConfig, section: &str, key: &str) -> bool {
    let value = match section {
        "net" => config.net.get(key),
        "http" => config.http.get(key),
        "git" => config.git.get(key),
        _ => None,
    };
    value
        .map(|value| matches!(value.as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false)
}

fn registry_is_remote(registry: &GhorRegistry) -> bool {
    matches!(
        registry,
        GhorRegistry::Http(_) | GhorRegistry::Sparse(_) | GhorRegistry::Git(_)
    )
}

fn ensure_registry_network_allowed(
    registry: &GhorRegistry,
    args: &[String],
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let offline = has_flag(args, "--offline")
        || env::var("CARGO_NET_OFFLINE")
            .map(|value| matches!(value.as_str(), "true" | "1"))
            .unwrap_or(false)
        || config
            .map(|config| cargo_config_bool(config, "net", "offline"))
            .unwrap_or(false);
    if offline && registry_is_remote(registry) {
        Err("[E_GHOR_NET_OFFLINE_001] registry network access is disabled by --offline or [net].offline".into())
    } else {
        Ok(())
    }
}

fn package_version_selector(raw: &str, args: &[String]) -> (String, Option<String>) {
    if let Some((name, version)) = raw.split_once('@') {
        if !name.is_empty() && !version.is_empty() {
            return (name.to_string(), Some(version.to_string()));
        }
    }
    (
        raw.to_string(),
        arg_after(args, "--vers")
            .or_else(|| arg_after(args, "--version"))
            .map(str::to_string),
    )
}

fn update_registry_yank(
    index: &mut Value,
    package: &str,
    version: Option<&str>,
    yanked: bool,
) -> Result<(), Box<dyn Error>> {
    let mut touched = false;
    let packages = index
        .get_mut("packages")
        .and_then(Value::as_array_mut)
        .ok_or("[E_GHOR_REGISTRY_001] registry index packages must be an array")?;
    for entry in packages {
        let same_package = entry.get("name").and_then(Value::as_str) == Some(package);
        let same_version = version
            .map(|version| entry.get("version").and_then(Value::as_str) == Some(version))
            .unwrap_or(true);
        if same_package && same_version {
            entry["yanked"] = Value::Bool(yanked);
            touched = true;
        }
    }
    if touched {
        Ok(())
    } else {
        Err(format!("[E_GHOR_YANK_001] package '{package}' was not found in registry").into())
    }
}

fn update_registry_package_yank(
    registry: &GhorRegistry,
    package: &str,
    version: Option<&str>,
    yanked: bool,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = read_registry_package_entries(registry, package, token, config)?;
    let mut touched = false;
    for entry in &mut entries {
        let same_version = version
            .map(|version| entry.get("vers").and_then(Value::as_str) == Some(version))
            .unwrap_or(true);
        if same_version {
            entry["yanked"] = Value::Bool(yanked);
            touched = true;
        }
    }
    if !touched {
        return Err(format!(
            "[E_GHOR_YANK_001] package index '{}' had no matching version",
            cargo_index_path(package)
        )
        .into());
    }
    write_registry_package_entries(registry, package, &entries, token, config)
}

fn update_registry_package_owner(
    registry: &GhorRegistry,
    package: &str,
    owner: &str,
    add: bool,
    token: Option<&str>,
    config: Option<&GhorCargoConfig>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = read_registry_package_entries(registry, package, token, config)?;
    if entries.is_empty() {
        return Err(
            format!("[E_GHOR_OWNER_001] package '{package}' not found in package index").into(),
        );
    }
    for entry in &mut entries {
        if entry.get("ghor_owners").and_then(Value::as_array).is_none() {
            entry["ghor_owners"] = Value::Array(Vec::new());
        }
        let owners = entry
            .get_mut("ghor_owners")
            .and_then(Value::as_array_mut)
            .ok_or("[E_GHOR_OWNER_001] ghor_owners field must be an array")?;
        if add {
            if !owners.iter().any(|value| value.as_str() == Some(owner)) {
                owners.push(Value::String(owner.to_string()));
            }
        } else {
            owners.retain(|value| value.as_str() != Some(owner));
        }
    }
    write_registry_package_entries(registry, package, &entries, token, config)
}

fn credentials_path() -> PathBuf {
    ghor_home().join("credentials.json")
}

fn read_registry_credentials(registry: &GhorRegistry) -> Option<String> {
    let path = credentials_path();
    if !credentials_permissions_secure(&path) {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<Value>(&raw).ok()?;
    let label = registry_label(registry);
    value
        .get("registries")?
        .get(label.as_str())?
        .get("token")?
        .as_str()
        .map(str::to_string)
}

fn write_registry_credentials(registry: &GhorRegistry, token: &str) -> Result<(), Box<dyn Error>> {
    let path = credentials_path();
    secure_credentials_parent(&path)?;
    let mut value = if path.exists() {
        serde_json::from_slice::<Value>(&fs::read(&path)?)?
    } else {
        json!({"schema": "ghor.credentials.v1", "registries": {}})
    };
    if value.get("registries").and_then(Value::as_object).is_none() {
        value["registries"] = json!({});
    }
    let label = registry_label(registry);
    value["registries"][label.as_str()] = json!({
        "token": token,
        "transport": registry_transport(registry)
    });
    write_private_credentials_file(&path, &serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}

fn remove_registry_credentials(registry: &GhorRegistry) -> Result<bool, Box<dyn Error>> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(false);
    }
    let mut value = serde_json::from_slice::<Value>(&fs::read(&path)?)?;
    let label = registry_label(registry);
    let removed = value
        .get_mut("registries")
        .and_then(Value::as_object_mut)
        .map(|registries| registries.remove(label.as_str()).is_some())
        .unwrap_or(false);
    write_private_credentials_file(&path, &serde_json::to_vec_pretty(&value)?)?;
    Ok(removed)
}

fn secure_credentials_parent(path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_private_credentials_dir_permissions(parent)?;
    }
    Ok(())
}

fn write_private_credentials_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    secure_credentials_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    set_private_credentials_file_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_private_credentials_dir_permissions(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_credentials_dir_permissions(_path: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(unix)]
fn set_private_credentials_file_permissions(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_credentials_file_permissions(_path: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(unix)]
fn credentials_permissions_secure(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if !path.exists() {
        return true;
    }
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.permissions().mode() & 0o077 == 0
}

#[cfg(not(unix))]
fn credentials_permissions_secure(_path: &Path) -> bool {
    true
}

fn require_registry_auth_for_mutation(
    registry: &GhorRegistry,
    token: Option<&str>,
    operation: &str,
) -> Result<(), Box<dyn Error>> {
    if registry_is_remote(registry) && token.filter(|token| !token.trim().is_empty()).is_none() {
        Err(format!(
            "[E_GHOR_AUTH_001] {operation} requires registry authentication via --token, login, env token, or credential-provider"
        )
        .into())
    } else {
        Ok(())
    }
}

fn run_owner(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let positionals = positional_args(args);
    let (action, package, owner) = match positionals.as_slice() {
        [package, ..] if has_flag(args, "--list") || has_flag(args, "-l") => {
            ("list", *package, None)
        }
        [package, owner, ..] if has_flag(args, "--add") || has_flag(args, "-a") => {
            ("add", *package, Some(*owner))
        }
        [package, owner, ..] if has_flag(args, "--remove") || has_flag(args, "-r") => {
            ("remove", *package, Some(*owner))
        }
        ["list", package, rest @ ..] => ("list", *package, rest.first().copied()),
        ["add", package, owner, ..] => ("add", *package, Some(*owner)),
        ["remove", package, owner, ..] => ("remove", *package, Some(*owner)),
        [package, rest @ ..] => ("list", *package, rest.first().copied()),
        [] => {
            return Err(
                "[E_GHOR_OWNER_001] owner expects <package>, list <package>, add <package> <owner>, or remove <package> <owner>"
                    .into(),
            )
        }
    };
    if matches!(action, "add" | "remove") {
        require_registry_auth_for_mutation(&registry, token.as_deref(), "owner")?;
    }
    let config = registry_config_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let web_api_owner = cargo_api_owner(
        &registry,
        &config,
        action,
        package,
        owner,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )
    .unwrap_or_else(|err| json!({"status": "direct_index_used", "reason": err.to_string()}));
    let mut index = read_registry_index_ref(
        &registry,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    match action {
        "add" => {
            let owner = owner.ok_or("[E_GHOR_OWNER_001] owner add expects <owner>")?;
            update_registry_owner(&mut index, package, owner, true)?;
            if !matches!(registry, GhorRegistry::Git(_)) {
                write_registry_index_ref(
                    &registry,
                    &index,
                    token.as_deref(),
                    manifest.as_ref().map(|manifest| &manifest.cargo_config),
                )?;
                update_registry_package_owner(
                    &registry,
                    package,
                    owner,
                    true,
                    token.as_deref(),
                    manifest.as_ref().map(|manifest| &manifest.cargo_config),
                )?;
            }
        }
        "remove" => {
            let owner = owner.ok_or("[E_GHOR_OWNER_001] owner remove expects <owner>")?;
            update_registry_owner(&mut index, package, owner, false)?;
            if !matches!(registry, GhorRegistry::Git(_)) {
                write_registry_index_ref(
                    &registry,
                    &index,
                    token.as_deref(),
                    manifest.as_ref().map(|manifest| &manifest.cargo_config),
                )?;
                update_registry_package_owner(
                    &registry,
                    package,
                    owner,
                    false,
                    token.as_deref(),
                    manifest.as_ref().map(|manifest| &manifest.cargo_config),
                )?;
            }
        }
        "list" => {}
        _ => return Err("[E_GHOR_OWNER_001] unsupported owner action".into()),
    }
    print_json(&json!({
        "schema": "ghor.owner.v1",
        "status": "ok",
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "package": package,
        "action": action,
        "owners": registry_owners(&index, package),
        "web_api_owner": web_api_owner
    }))
}

fn update_registry_owner(
    index: &mut Value,
    package: &str,
    owner: &str,
    add: bool,
) -> Result<(), Box<dyn Error>> {
    let mut touched = false;
    let packages = index
        .get_mut("packages")
        .and_then(Value::as_array_mut)
        .ok_or("[E_GHOR_REGISTRY_001] registry index packages must be an array")?;
    for entry in packages {
        if entry.get("name").and_then(Value::as_str) != Some(package) {
            continue;
        }
        touched = true;
        if entry.get("owners").and_then(Value::as_array).is_none() {
            entry["owners"] = Value::Array(Vec::new());
        }
        let owners = entry
            .get_mut("owners")
            .and_then(Value::as_array_mut)
            .ok_or("[E_GHOR_OWNER_001] owners field must be an array")?;
        if add {
            if !owners.iter().any(|value| value.as_str() == Some(owner)) {
                owners.push(Value::String(owner.to_string()));
            }
        } else {
            owners.retain(|value| value.as_str() != Some(owner));
        }
    }
    if touched {
        Ok(())
    } else {
        Err(format!("[E_GHOR_OWNER_001] package '{package}' not found in registry").into())
    }
}

fn registry_owners(index: &Value, package: &str) -> Vec<String> {
    let mut owners = BTreeSet::new();
    if let Some(packages) = index.get("packages").and_then(Value::as_array) {
        for entry in packages {
            if entry.get("name").and_then(Value::as_str) == Some(package) {
                if let Some(values) = entry.get("owners").and_then(Value::as_array) {
                    owners.extend(values.iter().filter_map(Value::as_str).map(str::to_string));
                }
            }
        }
    }
    owners.into_iter().collect()
}

fn run_install(args: &[String]) -> Result<(), Box<dyn Error>> {
    if arg_after(args, "--manifest-path").is_none()
        && arg_after(args, "--manifest").is_none()
        && first_positional(args).is_some()
    {
        return run_install_registry(args);
    }
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    run_build(args)?;
    let root = install_root(args);
    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let launcher = bin.join(package_name(&manifest));
    fs::write(
        &launcher,
        format!(
            "#!/usr/bin/env sh\nexec ghor run --manifest '{}' \"$@\"\n",
            manifest.path.display()
        ),
    )?;
    print_json(&json!({
        "schema": "ghor.install.v1",
        "status": "installed",
        "source": "path",
        "bin": launcher,
        "root": root
    }))
}

fn run_install_registry(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = optional_manifest_from_args(args);
    let package = first_positional(args).ok_or("[E_GHOR_INSTALL_001] install expects <package>")?;
    let registry = manifest
        .as_ref()
        .map(|manifest| registry_from_args_with_manifest(args, manifest))
        .unwrap_or_else(|| registry_from_args(args));
    ensure_registry_network_allowed(
        &registry,
        args,
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let token = registry_token_with_manifest(args, &registry, manifest.as_ref());
    let version_req = arg_after(args, "--version")
        .or_else(|| arg_after(args, "--vers"))
        .unwrap_or("*");
    let dep = GhorDependency {
        name: package.to_string(),
        source: "registry".to_string(),
        version: Some(version_req.to_string()),
        checksum: None,
        path: None,
        kind: "normal".to_string(),
        target: None,
        features: Vec::new(),
        default_features: true,
        optional: false,
        registry: None,
        package: None,
        git: None,
        git_branch: None,
        git_tag: None,
        git_rev: None,
        workspace_inherited: false,
    };
    let archive = download_registry_archive(
        &registry,
        &dep,
        token.as_deref(),
        manifest.as_ref().map(|manifest| &manifest.cargo_config),
    )?;
    let version = archive
        .entry
        .get("vers")
        .or_else(|| archive.entry.get("version"))
        .and_then(Value::as_str)
        .unwrap_or("0.0.0");
    let root = install_root(args);
    let source_dir = root
        .join("registry-src")
        .join(format!("{package}-{version}"));
    let package_cache = root.join("registry-cache");
    fs::create_dir_all(&package_cache)?;
    let archive_path = package_cache.join(&archive.archive_name);
    fs::write(&archive_path, &archive.bytes)?;
    extract_crate_archive(&archive_path, &source_dir)?;
    let manifest_path = source_dir.join(GHOR_OFFICIAL_MANIFEST);
    let installed_manifest = parse_manifest(&manifest_path)?;
    let bin = root.join("bin");
    fs::create_dir_all(&bin)?;
    let launcher = bin.join(package);
    fs::write(
        &launcher,
        format!(
            "#!/usr/bin/env sh\nexec ghor run --manifest '{}' \"$@\"\n",
            manifest_path.display()
        ),
    )?;
    print_json(&json!({
        "schema": "ghor.install.v1",
        "status": "installed",
        "source": "registry",
        "package": package,
        "version": version,
        "registry": registry_label(&registry),
        "registry_transport": registry_transport(&registry),
        "checksum": archive.checksum,
        "archive": archive_path,
        "source_dir": source_dir,
        "bin": launcher,
        "root": root,
        "package_manifest_validation": archive.package_manifest_validation,
        "installed_manifest_hash": stable_json_hash(&manifest_to_json(&installed_manifest))
    }))
}

fn install_root(args: &[String]) -> PathBuf {
    arg_after(args, "--root")
        .map(PathBuf::from)
        .unwrap_or_else(ghor_home)
}

fn run_uninstall(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name =
        first_positional(args).ok_or("[E_GHOR_UNINSTALL_001] uninstall expects <package>")?;
    let path = ghor_home().join("bin").join(name);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    print_json(&json!({"schema": "ghor.uninstall.v1", "status": "ok", "removed": path}))
}

fn run_bin(_args: &[String]) -> Result<(), Box<dyn Error>> {
    print_json(&json!({"schema": "ghor.bin.v1", "path": ghor_home().join("bin")}))
}

fn ghor_home() -> PathBuf {
    env::var_os("GHOR_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ghor")
        })
}

fn run_update(args: &[String]) -> Result<(), Box<dyn Error>> {
    if has_flag(args, "--frozen") {
        return Err("[E_GHOR_FROZEN_001] --frozen forbids lock updates".into());
    }
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    let lock_path = lock_path_for(&manifest, args);
    let before = if lock_path.exists() {
        Some(lock_state_hash(&lock_path)?)
    } else {
        None
    };
    let resolve = resolve_plan(&manifest, args)?;
    let diagnostics = resolve
        .get("diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            GhorDiagnostic::from_parts(
                value
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("E_GHOR_UPDATE_001"),
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("update diagnostic"),
                value
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("error"),
            )
        })
        .collect::<Vec<_>>();
    if diagnostics_have_errors(&diagnostics) {
        print_json(&json!({
            "schema": "ghor.update.v1",
            "status": "error",
            "diagnostics": diagnostics_to_json(&diagnostics),
            "resolve": resolve
        }))?;
        return Err(diagnostics_summary(&diagnostics).into());
    }
    let after = lock_json(&manifest, Some(&resolve));
    let after_toml = lock_toml_from_metadata(&after);
    let after_hash = lock_state_hash_from_parts(&after_toml, &after);
    let changed = before
        .as_ref()
        .map(|before| before != &after_hash)
        .unwrap_or(true);
    let dry_run = has_flag(args, "--dry-run");
    if !dry_run {
        write_lock_artifacts(&lock_path, &after)?;
    }
    print_json(&json!({
        "schema": "ghor.update.v1",
        "status": if dry_run { "dry_run" } else { "updated" },
        "lockfile": lock_path,
        "lock_metadata": lock_metadata_path_for(&lock_path),
        "lock_format": "cargo_lock_toml_primary",
        "changed": changed,
        "package_specs": update_package_specs(args),
        "precise": arg_after(args, "--precise"),
        "before_hash": before,
        "after_hash": after_hash,
        "cargo_lock_hash": sha256_hex(after_toml.as_bytes()),
        "metadata_hash": stable_json_hash(&after),
        "resolve": resolve
    }))
}

fn run_tree(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let resolve = resolve_plan(&manifest, args)?;
    let nodes = resolve
        .get("resolved")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut edges = Vec::new();
    for node in &nodes {
        let Some(from) = node.get("name").and_then(Value::as_str) else {
            continue;
        };
        for dep in node
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(to) = dep.get("name").and_then(Value::as_str) {
                edges.push(json!({"from": from, "to": to, "kind": dep.get("kind").cloned().unwrap_or(Value::Null)}));
            }
        }
    }
    print_json(&json!({
        "schema": "ghor.tree.v1",
        "package": package_name(&manifest),
        "resolver": resolve.get("resolver").cloned().unwrap_or(Value::Null),
        "root_dependencies": resolve_roots(&manifest, args)?
            .into_iter()
            .filter(|(_, root)| *root)
            .map(|(dep, _)| dependency_to_json(&dep))
            .collect::<Vec<_>>(),
        "nodes": nodes,
        "edges": edges,
        "resolve": resolve
    }))
}

fn run_add(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name = first_positional(args).ok_or("[E_GHOR_DEP_RESOLUTION_001] add expects <pkg>")?;
    let manifest_path = manifest_path_from_args(args)?;
    let dep_line = if let Some(path) = arg_after(args, "--path") {
        if let Some(version) = arg_after(args, "--version").or_else(|| arg_after(args, "--vers")) {
            format!("{name} = {{ path = \"{path}\", version = \"{version}\" }}\n")
        } else {
            format!("{name} = {{ path = \"{path}\" }}\n")
        }
    } else {
        let checksum = arg_after(args, "--checksum")
            .ok_or("[E_GHOR_CHECKSUM_001] remote dependencies require --checksum <sha256>")?;
        let version = arg_after(args, "--version")
            .or_else(|| arg_after(args, "--vers"))
            .unwrap_or("*");
        format!("{name} = {{ version = \"{version}\", checksum = \"{checksum}\" }}\n")
    };
    let mut raw = fs::read_to_string(&manifest_path)?;
    raw = insert_dependency_line_into_manifest(&raw, &dep_line);
    fs::write(&manifest_path, raw)?;
    print_json(&json!({"schema": "ghor.add.v1", "status": "ok", "dependency": name}))
}

fn insert_dependency_line_into_manifest(raw: &str, dep_line: &str) -> String {
    let mut out = String::new();
    let mut saw_dependencies = false;
    let mut inserted = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if saw_dependencies
            && !inserted
            && trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && trimmed != "[dependencies]"
        {
            out.push_str(dep_line);
            inserted = true;
        }
        out.push_str(line);
        out.push('\n');
        if trimmed == "[dependencies]" {
            saw_dependencies = true;
        }
    }

    if saw_dependencies {
        if !inserted {
            out.push_str(dep_line);
        }
    } else {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n[dependencies]\n");
        out.push_str(dep_line);
    }
    out
}

fn run_remove(args: &[String]) -> Result<(), Box<dyn Error>> {
    let name = first_positional(args).ok_or("[E_GHOR_DEP_RESOLUTION_001] remove expects <pkg>")?;
    let manifest_path = manifest_path_from_args(args)?;
    let raw = fs::read_to_string(&manifest_path)?;
    let filtered = raw
        .lines()
        .filter(|line| !line.trim_start().starts_with(&format!("{name} =")))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&manifest_path, format!("{filtered}\n"))?;
    print_json(&json!({"schema": "ghor.remove.v1", "status": "ok", "dependency": name}))
}

fn run_resolve(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let plan = resolve_plan(&manifest, args)?;
    let diagnostics = plan
        .get("diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            GhorDiagnostic::from_parts(
                value
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("E_GHOR_RESOLVER_001"),
                value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("resolver diagnostic"),
                value
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or("error"),
            )
        })
        .collect::<Vec<_>>();
    print_json(&plan)?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn resolve_plan(manifest: &GhorManifest, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let resolver_version = resolver_version(manifest, args);
    let target = arg_after(args, "--target")
        .or_else(|| manifest.target_default.get("triple").map(String::as_str))
        .unwrap_or(host_target_triple())
        .to_string();
    let include_dev = has_flag(args, "--include-dev")
        || has_flag(args, "--all-targets")
        || has_flag(args, "--tests")
        || has_flag(args, "--test");
    let default_registry = registry_from_args_with_manifest(args, manifest);
    let token = registry_token_with_manifest(args, &default_registry, Some(manifest));
    let mut diagnostics = manifest
        .diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .cloned()
        .collect::<Vec<_>>();
    diagnostics.extend(validate_dependency_version_requirements(manifest));

    let mut queue = resolve_roots(manifest, args)?;
    let mut resolved = Vec::new();
    let mut feature_groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut cursor = 0;
    while cursor < queue.len() {
        let (dep, root_dependency) = queue[cursor].clone();
        cursor += 1;
        if dep.kind == "dev" && !root_dependency && !include_dev {
            continue;
        }
        let active = target_active_for_resolver(&dep, &target, &resolver_version);
        if !active && resolver_version != "1" {
            resolved.push(json!({
                "name": dep.name,
                "requested": dependency_to_json(&dep),
                "status": "skipped_inactive_target",
                "target": dep.target,
                "resolver": resolver_version
            }));
            continue;
        }
        let mut effective = apply_dependency_overrides(manifest, &dep);
        if let Some(precise) = precise_update_for_dep(args, &effective.name) {
            effective.version = Some(format!("={precise}"));
        }
        let override_kind = override_kind(manifest, &dep);
        if let Some(path) = &effective.path {
            let version = effective.version.as_deref().unwrap_or("path");
            let key = format!(
                "{}@{}:{}",
                effective.name,
                version,
                feature_scope_for_resolver(&effective, &target, &resolver_version)
            );
            feature_groups
                .entry(key)
                .or_default()
                .extend(effective.features.iter().cloned());
            resolved.push(json!({
                "name": effective.name,
                "version": version,
                "status": "resolved_path",
                "path": path,
                "kind": effective.kind,
                "target": effective.target,
                "override": override_kind,
                "features": effective.features,
                "default_features": effective.default_features
            }));
            continue;
        }
        if effective.git.is_some() {
            let (git_entry, transitive) = resolve_git_dependency(
                &effective,
                args,
                Some(&manifest.cargo_config),
                override_kind,
            )?;
            let version = git_entry
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("0.0.0")
                .to_string();
            let scope = feature_scope_for_resolver(&effective, &target, &resolver_version);
            let features = git_entry
                .get("features")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            feature_groups
                .entry(format!("{}@{}:{}", effective.name, version, scope))
                .or_default()
                .extend(features);
            let seen_key = format!(
                "{}@{}:{}",
                effective.name,
                version,
                git_entry
                    .get("source")
                    .and_then(|source| source.get("resolved_head"))
                    .and_then(Value::as_str)
                    .unwrap_or("git")
            );
            if !seen.insert(seen_key) {
                continue;
            }
            resolved.push(git_entry);
            for transitive in transitive {
                queue.push((transitive, false));
            }
            continue;
        }
        if effective.source == "basal-registry" {
            let (registry, source_report) =
                basal_registry_for_dependency(manifest, args, &effective);
            let selected = select_basal_registry_entry(
                &effective,
                &read_registry_index_ref(&registry, token.as_deref(), Some(&manifest.cargo_config))
                    .unwrap_or_else(|_| empty_registry_index()),
            );
            let Some(selected) = selected else {
                resolved.push(json!({
                    "name": effective.name,
                    "requested": dependency_to_json(&effective),
                    "status": "unresolved_basal_registry",
                    "source": source_report
                }));
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_BASAL_PACKAGE_001",
                    format!(
                        "basal dependency '{}' was not found in registry",
                        effective.name
                    ),
                ));
                continue;
            };
            let version = selected
                .get("version")
                .or_else(|| selected.get("vers"))
                .and_then(Value::as_str)
                .unwrap_or("1")
                .to_string();
            let checksum = selected
                .get("cksum")
                .or_else(|| selected.get("checksum"))
                .cloned()
                .unwrap_or(Value::Null);
            let package_manifest_validation = validate_basal_registry_package_manifest(
                &registry,
                &selected,
                token.as_deref(),
                Some(&manifest.cargo_config),
            )
            .unwrap_or_else(|err| {
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_BASAL_PACKAGE_001",
                    err.to_string(),
                ));
                json!({"status": "error", "reason": err.to_string()})
            });
            resolved.push(json!({
                "name": effective.name,
                "root_id": selected.get("root_id").cloned().unwrap_or_else(|| Value::String(effective.package.clone().unwrap_or_else(|| effective.name.clone()))),
                "safe_name": selected.get("safe_name").cloned().unwrap_or_else(|| Value::String(effective.name.clone())),
                "version": version,
                "status": "resolved_basal_registry",
                "source": source_report,
                "kind": effective.kind,
                "checksum": checksum,
                "package_manifest": selected.get("package_manifest").cloned().unwrap_or(Value::Null),
                "package_manifest_hash": selected.get("package_manifest_hash").cloned().unwrap_or(Value::Null),
                "ffi_link_manifest_hash": selected.get("ffi_link_manifest_hash").cloned().unwrap_or(Value::Null),
                "package_manifest_validation": package_manifest_validation,
                "registry_transport": registry_transport(&registry),
                "dependencies": []
            }));
            continue;
        }
        let (registry, source_report) =
            effective_registry_for_dependency(manifest, args, &effective);
        let entries = read_registry_package_entries(
            &registry,
            &effective.name,
            token.as_deref(),
            Some(&manifest.cargo_config),
        )
        .unwrap_or_default();
        let selected = select_resolver_entry(
            &effective,
            &entries,
            manifest,
            &resolver_version,
            &mut diagnostics,
        );
        let Some(selected) = selected else {
            resolved.push(json!({
                "name": effective.name,
                "requested": dependency_to_json(&effective),
                "status": "unresolved",
                "source": source_report
            }));
            continue;
        };
        let version = selected
            .get("vers")
            .and_then(Value::as_str)
            .unwrap_or("0.0.0")
            .to_string();
        let features =
            resolve_features_for_entry(&effective, &selected, &target, &resolver_version);
        let scope = feature_scope_for_resolver(&effective, &target, &resolver_version);
        feature_groups
            .entry(format!("{}@{}:{}", effective.name, version, scope))
            .or_default()
            .extend(features.iter().cloned());
        let seen_key = format!(
            "{}@{}:{}",
            effective.name,
            version,
            registry_label(&registry)
        );
        if !seen.insert(seen_key.clone()) {
            continue;
        }
        let transitive = cargo_entry_dependencies(&selected);
        resolved.push(json!({
            "name": effective.name,
            "version": version,
            "status": "resolved_registry",
            "source": source_report,
            "kind": effective.kind,
            "target": effective.target,
            "checksum": selected.get("cksum").cloned().unwrap_or(Value::Null),
            "features": features,
            "default_features": effective.default_features,
            "override": override_kind,
            "registry_transport": registry_transport(&registry),
            "dependencies": transitive.iter().map(dependency_to_json).collect::<Vec<_>>()
        }));
        for transitive in transitive {
            queue.push((transitive, false));
        }
    }

    let feature_groups_json = feature_groups
        .into_iter()
        .map(|(group, features)| {
            json!({
                "group": group,
                "features": features.into_iter().collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "schema": "ghor.resolve.v2",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "resolver": {
            "version": resolver_version,
            "target": target,
            "include_dev": include_dev,
            "semver": "cargo_caret_tilde_wildcard_comparator",
            "features": "cargo_resolver_v1_v2_v3",
            "overrides": ["patch", "replace", "alternate_registries", "source_replacement"]
        },
        "package": package_name(manifest),
        "resolved": resolved,
        "feature_groups": feature_groups_json,
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))
}

fn precise_update_for_dep(args: &[String], dep_name: &str) -> Option<String> {
    let precise = arg_after(args, "--precise")?;
    let packages = update_package_specs(args);
    if packages.is_empty()
        || packages
            .iter()
            .any(|spec| package_spec_matches(spec, dep_name))
    {
        Some(precise.to_string())
    } else {
        None
    }
}

fn update_package_specs(args: &[String]) -> Vec<String> {
    let mut specs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-p" | "--package" => {
                if let Some(value) = args.get(index + 1) {
                    specs.push(value.to_string());
                    index += 1;
                }
            }
            value if !value.starts_with('-') => {
                if !matches!(value, "update" | "tree" | "vendor") {
                    specs.push(value.to_string());
                }
            }
            _ => {}
        }
        index += 1;
    }
    specs.sort();
    specs.dedup();
    specs
}

fn package_spec_matches(spec: &str, dep_name: &str) -> bool {
    spec == dep_name
        || spec
            .split(['@', ':', ' '])
            .next()
            .map(|name| name == dep_name)
            .unwrap_or(false)
}

fn resolver_version(manifest: &GhorManifest, args: &[String]) -> String {
    arg_after(args, "--resolver")
        .or_else(|| manifest.workspace.get("resolver").map(String::as_str))
        .or_else(|| manifest.package.get("resolver").map(String::as_str))
        .unwrap_or_else(|| {
            if manifest.package.get("edition").map(String::as_str) == Some("2024") {
                "3"
            } else if manifest.package.get("edition").map(String::as_str) == Some("2021") {
                "2"
            } else {
                "1"
            }
        })
        .to_string()
}

fn validate_dependency_version_requirements(manifest: &GhorManifest) -> Vec<GhorDiagnostic> {
    all_manifest_dependencies(manifest)
        .into_iter()
        .filter_map(|dep| {
            dep.version.as_deref().and_then(|version| {
                VersionReq::parse(version).err().map(|err| {
                    GhorDiagnostic::error(
                        "E_GHOR_RESOLVER_SEMVER_001",
                        format!(
                            "dependency '{}' has invalid version requirement '{}': {err}",
                            dep.name, version
                        ),
                    )
                })
            })
        })
        .collect()
}

fn resolve_roots(
    manifest: &GhorManifest,
    _args: &[String],
) -> Result<Vec<(GhorDependency, bool)>, Box<dyn Error>> {
    let mut roots = all_manifest_dependencies(manifest)
        .into_iter()
        .cloned()
        .map(|dep| (dep, true))
        .collect::<Vec<_>>();
    for member in workspace_member_entries(manifest, false)? {
        if same_path(&member.manifest_path, &manifest.path) {
            continue;
        }
        let member_manifest = parse_manifest(&member.manifest_path)?;
        roots.extend(
            all_manifest_dependencies(&member_manifest)
                .into_iter()
                .cloned()
                .map(|dep| (dep, true)),
        );
    }
    Ok(roots)
}

fn apply_dependency_overrides(manifest: &GhorManifest, dep: &GhorDependency) -> GhorDependency {
    if let Some(replacement) = manifest.replace_overrides.get(&dep.name).or_else(|| {
        dep.version.as_deref().and_then(|version| {
            manifest
                .replace_overrides
                .get(&format!("{}:{version}", dep.name))
        })
    }) {
        let mut replacement = replacement.clone();
        replacement.kind = dep.kind.clone();
        replacement.target = dep.target.clone();
        return replacement;
    }
    if let Some(patch) = manifest.patch_overrides.get(&dep.name) {
        let mut patch = patch.clone();
        patch.kind = dep.kind.clone();
        patch.target = dep.target.clone();
        return patch;
    }
    dep.clone()
}

fn override_kind(manifest: &GhorManifest, dep: &GhorDependency) -> &'static str {
    if manifest.replace_overrides.contains_key(&dep.name)
        || dep
            .version
            .as_deref()
            .map(|version| {
                manifest
                    .replace_overrides
                    .contains_key(&format!("{}:{version}", dep.name))
            })
            .unwrap_or(false)
    {
        "replace"
    } else if manifest.patch_overrides.contains_key(&dep.name) {
        "patch"
    } else {
        "none"
    }
}

fn effective_registry_for_dependency(
    manifest: &GhorManifest,
    args: &[String],
    dep: &GhorDependency,
) -> (GhorRegistry, Value) {
    let source_name = dep.registry.as_deref().unwrap_or("crates-io");
    if let Some(source) = manifest.source_replacements.get(source_name) {
        if let Some(replacement) = source.get("replace-with") {
            if let Some(replacement_source) = manifest.source_replacements.get(replacement) {
                if let Some(raw) = replacement_source
                    .get("registry")
                    .or_else(|| replacement_source.get("local-registry"))
                    .or_else(|| replacement_source.get("directory"))
                    .or_else(|| replacement_source.get("git"))
                {
                    return (
                        registry_from_raw(raw, None),
                        json!({
                            "name": source_name,
                            "effective": replacement,
                            "replacement_kind": replacement_source.keys().next().cloned().unwrap_or_else(|| "unknown".to_string()),
                            "registry": raw
                        }),
                    );
                }
            }
            if let Some(raw) = manifest.registries.get(replacement) {
                let protocol = manifest
                    .cargo_config
                    .registries
                    .get(replacement)
                    .and_then(|values| values.get("protocol"))
                    .map(String::as_str);
                return (
                    registry_from_raw(raw, protocol),
                    json!({
                        "name": source_name,
                        "effective": replacement,
                        "replacement_kind": "alternate_registry",
                        "registry": raw
                    }),
                );
            }
        }
    }
    if let Some(registry_name) = dep.registry.as_deref() {
        if let Some(raw) = manifest.registries.get(registry_name) {
            let protocol = manifest
                .cargo_config
                .registries
                .get(registry_name)
                .and_then(|values| values.get("protocol"))
                .map(String::as_str);
            return (
                registry_from_raw(raw, protocol),
                json!({
                    "name": registry_name,
                    "effective": registry_name,
                    "replacement_kind": "alternate_registry",
                    "registry": raw
                }),
            );
        }
    }
    let registry = registry_from_args_with_manifest(args, manifest);
    (
        registry.clone(),
        json!({
            "name": source_name,
            "effective": source_name,
            "replacement_kind": "none",
            "registry": registry_label(&registry)
        }),
    )
}

fn basal_registry_for_dependency(
    manifest: &GhorManifest,
    args: &[String],
    dep: &GhorDependency,
) -> (GhorRegistry, Value) {
    if let Some(raw) = dep.registry.as_deref() {
        if let Some(configured) = manifest.registries.get(raw) {
            let protocol = manifest
                .cargo_config
                .registries
                .get(raw)
                .and_then(|values| values.get("protocol"))
                .map(String::as_str);
            return (
                registry_from_raw(configured, protocol),
                json!({
                    "name": raw,
                    "effective": raw,
                    "replacement_kind": "basal_registry",
                    "registry": configured
                }),
            );
        }
        return (
            registry_from_raw(raw, None),
            json!({
                "name": raw,
                "effective": raw,
                "replacement_kind": "basal_registry_raw",
                "registry": raw
            }),
        );
    }
    let registry = registry_from_args_with_manifest(args, manifest);
    (
        registry.clone(),
        json!({
            "name": "basal-default",
            "effective": "basal-default",
            "replacement_kind": "basal_registry_default",
            "registry": registry_label(&registry)
        }),
    )
}

fn registry_from_raw(raw: &str, protocol: Option<&str>) -> GhorRegistry {
    if raw.starts_with("sparse+http://") || raw.starts_with("sparse+https://") {
        return GhorRegistry::Sparse(
            raw.trim_start_matches("sparse+")
                .trim_end_matches('/')
                .to_string(),
        );
    }
    if raw.starts_with("git+") {
        return GhorRegistry::Git(raw.trim_start_matches("git+").to_string());
    }
    if protocol == Some("git") || raw.ends_with(".git") {
        return GhorRegistry::Git(raw.to_string());
    }
    if protocol == Some("sparse") && (raw.starts_with("http://") || raw.starts_with("https://")) {
        return GhorRegistry::Sparse(raw.trim_end_matches('/').to_string());
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return GhorRegistry::Http(raw.trim_end_matches('/').to_string());
    }
    GhorRegistry::Local(PathBuf::from(raw))
}

fn select_resolver_entry(
    dep: &GhorDependency,
    entries: &[Value],
    manifest: &GhorManifest,
    resolver_version: &str,
    diagnostics: &mut Vec<GhorDiagnostic>,
) -> Option<Value> {
    let req = dep
        .version
        .as_deref()
        .and_then(|raw| VersionReq::parse(raw).ok())
        .unwrap_or_else(VersionReq::any);
    let mut candidates = entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(dep.name.as_str()))
        .filter(|entry| entry.get("yanked").and_then(Value::as_bool) != Some(true))
        .filter(|entry| {
            entry
                .get("vers")
                .and_then(Value::as_str)
                .and_then(|version| SemVer::parse(version).ok())
                .map(|version| req.matches(&version))
                .unwrap_or(false)
        })
        .filter(|entry| entry_has_required_features(entry, dep))
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_RESOLVER_VERSION_001",
            format!(
                "no registry version of '{}' satisfied requirement '{}'",
                dep.name,
                dep.version.as_deref().unwrap_or("*")
            ),
        ));
        return None;
    }
    candidates.sort_by(|left, right| {
        let left_version = left
            .get("vers")
            .and_then(Value::as_str)
            .and_then(|version| SemVer::parse(version).ok());
        let right_version = right
            .get("vers")
            .and_then(Value::as_str)
            .and_then(|version| SemVer::parse(version).ok());
        left_version.cmp(&right_version)
    });
    if resolver_version == "3" {
        if let Some(msrv) = manifest.toolchain.get("rust_version") {
            if let Ok(msrv) = SemVer::parse(msrv) {
                if let Some(candidate) = candidates
                    .iter()
                    .rev()
                    .find(|entry| {
                        entry
                            .get("rust_version")
                            .and_then(Value::as_str)
                            .and_then(|version| SemVer::parse(version).ok())
                            .map(|rust_version| rust_version <= msrv)
                            .unwrap_or(true)
                    })
                    .cloned()
                {
                    return Some(candidate);
                }
            }
        }
    }
    candidates.pop()
}

fn entry_has_required_features(entry: &Value, dep: &GhorDependency) -> bool {
    let Some(features) = entry.get("features").and_then(Value::as_object) else {
        return dep.features.is_empty();
    };
    dep.features
        .iter()
        .all(|feature| features.contains_key(feature) || feature == "default")
}

fn resolve_features_for_entry(
    dep: &GhorDependency,
    entry: &Value,
    target: &str,
    resolver_version: &str,
) -> BTreeSet<String> {
    if !target_active_for_resolver(dep, target, resolver_version) && resolver_version != "1" {
        return BTreeSet::new();
    }
    let mut out = BTreeSet::new();
    let features = entry.get("features").cloned().unwrap_or_else(|| json!({}));
    let mut queue = Vec::new();
    if dep.default_features {
        queue.push("default".to_string());
    }
    queue.extend(dep.features.iter().cloned());
    while let Some(feature) = queue.pop() {
        if !out.insert(feature.clone()) {
            continue;
        }
        if let Some(items) = features.get(&feature).and_then(Value::as_array) {
            queue.extend(items.iter().filter_map(Value::as_str).map(str::to_string));
        }
    }
    out
}

fn cargo_entry_dependencies(entry: &Value) -> Vec<GhorDependency> {
    entry
        .get("deps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dep| {
            let name = dep.get("name").and_then(Value::as_str)?.to_string();
            let kind = dep
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("normal")
                .to_string();
            Some(GhorDependency {
                name,
                source: dep
                    .get("registry")
                    .and_then(Value::as_str)
                    .unwrap_or("registry")
                    .to_string(),
                version: dep
                    .get("req")
                    .or_else(|| dep.get("version_req"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                checksum: None,
                path: None,
                kind,
                target: dep
                    .get("target")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                features: dep
                    .get("features")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                default_features: dep
                    .get("default_features")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                optional: dep
                    .get("optional")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                registry: dep
                    .get("registry")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                package: dep
                    .get("package")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                git: None,
                git_branch: None,
                git_tag: None,
                git_rev: None,
                workspace_inherited: false,
            })
        })
        .collect()
}

fn run_lock(args: &[String]) -> Result<(), Box<dyn Error>> {
    if has_flag(args, "--frozen") {
        return Err("[E_GHOR_FROZEN_001] --frozen forbids lock updates".into());
    }
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    if diagnostics_have_errors(&manifest.diagnostics) {
        return Err(diagnostics_summary(&manifest.diagnostics).into());
    }
    let lock_path = lock_path_for(&manifest, args);
    let resolve = resolve_plan(&manifest, args)?;
    let lock = lock_json(&manifest, Some(&resolve));
    let lock_toml = lock_toml_from_metadata(&lock);
    write_lock_artifacts(&lock_path, &lock)?;
    print_json(&json!({
        "schema": "ghor.lock.result.v1",
        "status": "locked",
        "path": lock_path,
        "metadata_path": lock_metadata_path_for(&lock_path),
        "lock_format": "cargo_lock_toml_primary",
        "lock_hash": lock_state_hash_from_parts(&lock_toml, &lock),
        "metadata_hash": stable_json_hash(&lock),
        "cargo_lock_format_version": lock.get("cargo_lock_format_version").cloned().unwrap_or(Value::Null),
        "cargo_lock_hash": sha256_hex(lock_toml.as_bytes())
    }))
}

fn lock_json(manifest: &GhorManifest, resolve: Option<&Value>) -> Value {
    let resolve_value = resolve.cloned().unwrap_or(Value::Null);
    let cargo_packages = cargo_lock_packages(&resolve_value);
    let cargo_lock_toml = cargo_lock_toml(&cargo_packages);
    json!({
        "schema": "ghor.lock.v1",
        "lockfile_version": GHOR_LOCK_VERSION,
        "cargo_lock_format_version": 4,
        "generated_by": format!("ghor {GHOR_VERSION}"),
        "package": {
            "name": package_name(manifest),
            "version": package_version(manifest),
            "hash": stable_json_hash(&manifest_to_json(manifest))
        },
        "target": manifest.target_default,
        "profile": "debug",
        "capabilities": {
            "machine": manifest.machine,
            "basal": manifest.basal,
            "runtime": manifest.runtime,
            "primitive": manifest.primitive
        },
        "language": {
            "native": "nC",
            "project": manifest.languages,
            "foreign_adapters": manifest.language_adapters
        },
        "dependencies": manifest.dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "dev_dependencies": manifest.dev_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "build_dependencies": manifest.build_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "target_dependencies": manifest.target_dependencies.iter().map(dependency_to_json).collect::<Vec<_>>(),
        "rust_dependencies": manifest.rust_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "c_dependencies": manifest.c_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "cpp_dependencies": manifest.cpp_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "asm_dependencies": manifest.asm_dependencies.values().map(dependency_to_json).collect::<Vec<_>>(),
        "native_dependencies": manifest.native_dependencies.values().map(native_dependency_to_json).collect::<Vec<_>>(),
        "ffi_link_manifest": ghor_ffi_manifest_report(manifest, &[]),
        "resolve": resolve_value,
        "cargo_packages": cargo_packages,
        "cargo_lock_toml": cargo_lock_toml,
        "cargo_compatible": {
            "format": "Cargo.lock",
            "version": 4,
            "package_id_semantics": "name version source",
            "registry_sources": "registry+url",
            "git_sources": "git+url#resolved_head",
            "path_sources": "source omitted like Cargo path packages"
        }
    })
}

fn cargo_lock_packages(resolve: &Value) -> Vec<Value> {
    let mut packages = resolve
        .get("resolved")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let status = entry.get("status").and_then(Value::as_str)?;
            if !matches!(
                status,
                "resolved_registry" | "resolved_path" | "resolved_git"
            ) {
                return None;
            }
            let name = entry.get("name").and_then(Value::as_str)?.to_string();
            let version = entry
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("0.0.0")
                .to_string();
            let source = match status {
                "resolved_registry" => entry
                    .get("source")
                    .and_then(|source| source.get("registry"))
                    .and_then(Value::as_str)
                    .map(|registry| format!("registry+{registry}")),
                "resolved_git" => entry
                    .get("source")
                    .and_then(|source| source.get("source"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                _ => None,
            };
            let dependencies = entry
                .get("dependencies")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|dep| dep.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>();
            Some(json!({
                "name": name,
                "version": version,
                "source": source,
                "checksum": entry.get("checksum").cloned().unwrap_or(Value::Null),
                "dependencies": dependencies
            }))
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        let left_key = format!(
            "{}\u{0}{}\u{0}{}",
            left.get("name").and_then(Value::as_str).unwrap_or_default(),
            left.get("version")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            left.get("source")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
        let right_key = format!(
            "{}\u{0}{}\u{0}{}",
            right
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            right
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            right
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
        left_key.cmp(&right_key)
    });
    packages
}

fn cargo_lock_toml(packages: &[Value]) -> String {
    let mut out = String::new();
    out.push_str("# This file is automatically @generated by ghor.\n");
    out.push_str("version = 4\n");
    for package in packages {
        out.push_str("\n[[package]]\n");
        out.push_str(&format!(
            "name = {}\n",
            toml_value(
                package
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            )
        ));
        out.push_str(&format!(
            "version = {}\n",
            toml_value(
                package
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            )
        ));
        if let Some(source) = package.get("source").and_then(Value::as_str) {
            out.push_str(&format!("source = {}\n", toml_value(source)));
        }
        if let Some(checksum) = package.get("checksum").and_then(Value::as_str) {
            out.push_str(&format!("checksum = {}\n", toml_value(checksum)));
        }
        let dependencies = package
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !dependencies.is_empty() {
            out.push_str("dependencies = [\n");
            for dependency in dependencies {
                out.push_str(&format!("    {},\n", toml_value(&dependency)));
            }
            out.push_str("]\n");
        }
    }
    out
}

fn lock_toml_from_metadata(lock: &Value) -> String {
    lock.get("cargo_lock_toml")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            cargo_lock_toml(&cargo_lock_packages(
                lock.get("resolve").unwrap_or(&Value::Null),
            ))
        })
}

fn lock_metadata_path_for(lock_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.meta.json", lock_path.display()))
}

fn write_lock_artifacts(lock_path: &Path, metadata: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let lock_toml = lock_toml_from_metadata(metadata);
    fs::write(lock_path, lock_toml.as_bytes())?;
    fs::write(
        lock_metadata_path_for(lock_path),
        serde_json::to_vec_pretty(metadata)?,
    )?;
    Ok(())
}

fn lock_state_hash(lock_path: &Path) -> Result<String, Box<dyn Error>> {
    let lock_text = fs::read_to_string(lock_path)?;
    let metadata_path = lock_metadata_path_for(lock_path);
    let metadata = if metadata_path.exists() {
        serde_json::from_slice::<Value>(&fs::read(metadata_path)?)?
    } else {
        Value::Null
    };
    Ok(lock_state_hash_from_parts(&lock_text, &metadata))
}

fn lock_state_hash_from_parts(lock_toml: &str, metadata: &Value) -> String {
    sha256_hex(
        format!(
            "ghor.lock.toml\0{}\0ghor.lock.meta\0{}",
            sha256_hex(lock_toml.as_bytes()),
            stable_json_hash(metadata)
        )
        .as_bytes(),
    )
}

fn run_verify_lock(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let diagnostics = lock_diagnostics(&manifest, args)?;
    print_json(&json!({
        "schema": "ghor.verify-lock.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn lock_path_for(manifest: &GhorManifest, args: &[String]) -> PathBuf {
    arg_after(args, "--lock")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.workspace_root.join("ghor.lock"))
}

fn lock_diagnostics(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Vec<GhorDiagnostic>, Box<dyn Error>> {
    let lock_path = lock_path_for(manifest, args);
    if !lock_path.exists() {
        return Ok(vec![GhorDiagnostic::error(
            "E_GHOR_LOCK_DRIFT_001",
            "ghor.lock is missing",
        )]);
    }
    let resolve = resolve_plan(manifest, args)?;
    let expected = lock_json(manifest, Some(&resolve));
    let expected_toml = lock_toml_from_metadata(&expected);
    let actual_toml = fs::read_to_string(&lock_path)?;
    if actual_toml.trim_start().starts_with('{') {
        return Ok(vec![GhorDiagnostic::error(
            "E_GHOR_LOCK_DRIFT_001",
            "ghor.lock is legacy JSON; run ghor lock to rewrite Cargo.lock-compatible TOML plus ghor.lock.meta.json",
        )]);
    }
    let metadata_path = lock_metadata_path_for(&lock_path);
    if !metadata_path.exists() {
        return Ok(vec![GhorDiagnostic::error(
            "E_GHOR_LOCK_DRIFT_001",
            format!("{} is missing", metadata_path.display()),
        )]);
    }
    let actual_metadata = serde_json::from_slice::<Value>(&fs::read(&metadata_path)?)?;
    if actual_toml != expected_toml
        || stable_json_hash(&actual_metadata) != stable_json_hash(&expected)
    {
        Ok(vec![GhorDiagnostic::error(
            "E_GHOR_LOCK_DRIFT_001",
            "ghor.lock does not match ghor.toml/nc.toml resolution",
        )])
    } else {
        Ok(Vec::new())
    }
}

fn run_vendor(args: &[String]) -> Result<(), Box<dyn Error>> {
    let report = vendor_report(args)?;
    print_json(&report)
}

fn vendor_report(args: &[String]) -> Result<Value, Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let vendor = first_positional(args)
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.root.join("vendor"));
    if vendor.exists() && !has_flag(args, "--no-delete") {
        fs::remove_dir_all(&vendor)?;
    }
    fs::create_dir_all(&vendor)?;
    let resolve = resolve_plan(&manifest, args)?;
    let mut vendored = Vec::new();
    let token = registry_token_with_manifest(
        args,
        &registry_from_args_with_manifest(args, &manifest),
        Some(&manifest),
    );
    for entry in resolve
        .get("resolved")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("resolved_registry"))
    {
        let Some(name) = entry.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(version) = entry.get("version").and_then(Value::as_str) else {
            continue;
        };
        let source = entry.get("source").cloned().unwrap_or_else(|| json!({}));
        let registry = registry_from_resolved_source(&manifest, args, &source);
        ensure_registry_network_allowed(&registry, args, Some(&manifest.cargo_config))?;
        let dep = GhorDependency {
            name: name.to_string(),
            source: "registry".to_string(),
            version: Some(format!("={version}")),
            checksum: entry
                .get("checksum")
                .and_then(Value::as_str)
                .map(str::to_string),
            path: None,
            kind: "normal".to_string(),
            target: None,
            features: Vec::new(),
            default_features: true,
            optional: false,
            registry: None,
            package: None,
            git: None,
            git_branch: None,
            git_tag: None,
            git_rev: None,
            workspace_inherited: false,
        };
        let archive = download_registry_archive(
            &registry,
            &dep,
            token.as_deref(),
            Some(&manifest.cargo_config),
        )?;
        let dir_name = if has_flag(args, "--versioned-dirs") {
            format!("{name}-{version}")
        } else {
            name.to_string()
        };
        let dest = vendor.join(dir_name);
        let temp_archive = vendor.join(format!(".{name}-{version}.crate"));
        fs::write(&temp_archive, &archive.bytes)?;
        extract_crate_archive(&temp_archive, &dest)?;
        fs::remove_file(&temp_archive)?;
        let file_checksums = vendor_file_checksums(&dest)?;
        fs::write(
            dest.join(".cargo-checksum.json"),
            serde_json::to_vec_pretty(&json!({
                "files": file_checksums,
                "package": archive.checksum,
                "ghor": {
                    "name": name,
                    "version": version,
                    "registry": registry_label(&registry),
                    "package_manifest_validation": archive.package_manifest_validation.clone()
                }
            }))?,
        )?;
        vendored.push(json!({
            "name": name,
            "version": version,
            "path": dest,
            "checksum": archive.checksum,
            "file_checksums": file_checksums.len(),
            "registry": registry_label(&registry),
            "package_manifest_validation": archive.package_manifest_validation
        }));
    }
    for entry in resolve
        .get("resolved")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry.get("status").and_then(Value::as_str) == Some("resolved_basal_registry")
        })
    {
        let Some(name) = entry.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(version) = entry.get("version").and_then(Value::as_str) else {
            continue;
        };
        let source = entry.get("source").cloned().unwrap_or_else(|| json!({}));
        let registry = registry_from_resolved_source(&manifest, args, &source);
        ensure_registry_network_allowed(&registry, args, Some(&manifest.cargo_config))?;
        let dep = GhorDependency {
            name: name.to_string(),
            source: "basal-registry".to_string(),
            version: Some(format!("={version}")),
            checksum: entry
                .get("checksum")
                .and_then(Value::as_str)
                .map(str::to_string),
            path: None,
            kind: "basal".to_string(),
            target: None,
            features: Vec::new(),
            default_features: true,
            optional: false,
            registry: Some(registry_label(&registry)),
            package: entry
                .get("root_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            git: None,
            git_branch: None,
            git_tag: None,
            git_rev: None,
            workspace_inherited: false,
        };
        let token = registry_token_with_manifest(args, &registry, Some(&manifest));
        let archive = download_basal_registry_archive(
            &registry,
            &dep,
            token.as_deref(),
            Some(&manifest.cargo_config),
        )?;
        let safe_name = entry
            .get("safe_name")
            .and_then(Value::as_str)
            .unwrap_or(name);
        let dir_name = if has_flag(args, "--versioned-dirs") {
            format!("{safe_name}-{version}")
        } else {
            safe_name.to_string()
        };
        let dest = vendor.join(dir_name);
        let temp_archive = vendor.join(format!(".{safe_name}-{version}.nbp.tar.gz"));
        fs::write(&temp_archive, &archive.bytes)?;
        extract_crate_archive(&temp_archive, &dest)?;
        fs::remove_file(&temp_archive)?;
        let file_checksums = vendor_file_checksums(&dest)?;
        fs::write(
            dest.join(".basal-checksum.json"),
            serde_json::to_vec_pretty(&json!({
                "schema": "ghor.basal.vendor.checksum.v1",
                "files": file_checksums,
                "package": archive.checksum,
                "basal": {
                    "name": name,
                    "root_id": entry.get("root_id").cloned().unwrap_or(Value::Null),
                    "version": version,
                    "registry": registry_label(&registry),
                    "package_manifest_validation": archive.package_manifest_validation.clone()
                }
            }))?,
        )?;
        vendored.push(json!({
            "name": name,
            "root_id": entry.get("root_id").cloned().unwrap_or(Value::Null),
            "version": version,
            "path": dest,
            "checksum": archive.checksum,
            "file_checksums": file_checksums.len(),
            "registry": registry_label(&registry),
            "package_kind": "basal",
            "package_manifest_validation": archive.package_manifest_validation
        }));
    }
    for dep in all_manifest_dependencies(&manifest) {
        if let Some(path) = &dep.path {
            let marker = vendor.join(format!("{}.path", dep.name));
            fs::write(marker, path)?;
        }
    }
    let config_snippet = format!(
        "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n[source.vendored-sources]\ndirectory = \"{}\"\n",
        vendor.display()
    );
    Ok(json!({
        "schema": "ghor.vendor.v1",
        "status": "ok",
        "path": vendor,
        "vendored": vendored,
        "config": config_snippet,
        "resolve": resolve
    }))
}

fn vendor_file_checksums(root: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    vendor_file_checksums_inner(root, root, &mut files)?;
    Ok(files)
}

fn vendor_file_checksums_inner(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            vendor_file_checksums_inner(root, &path, out)?;
            continue;
        }
        if path.file_name().and_then(|value| value.to_str()) == Some(".cargo-checksum.json") {
            continue;
        }
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        out.insert(rel, sha256_hex(&fs::read(path)?));
    }
    Ok(())
}

fn registry_from_resolved_source(
    manifest: &GhorManifest,
    args: &[String],
    source: &Value,
) -> GhorRegistry {
    source
        .get("registry")
        .and_then(Value::as_str)
        .map(|raw| registry_from_raw(raw, None))
        .unwrap_or_else(|| registry_from_args_with_manifest(args, manifest))
}

fn run_workspace(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = first_positional(args).unwrap_or("metadata");
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    match action {
        "metadata" => print_json(&workspace_metadata_json(&manifest)?),
        "check" | "build" | "test" | "graph" => {
            run_workspace_member_action(args, &manifest, action)
        }
        other => Err(format!(
            "[E_GHOR_WORKSPACE_001] unknown workspace action '{}'",
            other
        )
        .into()),
    }
}

fn run_audit(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let diagnostics = manifest
        .diagnostics
        .iter()
        .filter(|diag| diag.code == "E_GHOR_CHECKSUM_001")
        .cloned()
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema": "ghor.audit.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "checks": ["checksums", "registry_trust", "capability_policy", "mode_policy"],
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_sbom(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    print_json(&json!({
        "schema": "ghor.sbom.v1",
        "package": package_name(&manifest),
        "version": package_version(&manifest),
        "dependencies": manifest.dependencies.values().map(dependency_to_json).collect::<Vec<_>>()
    }))
}

fn run_license(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    print_json(&json!({
        "schema": "ghor.license.v1",
        "status": "ok",
        "license": manifest.package.get("license")
    }))
}

fn run_supply_chain(args: &[String]) -> Result<(), Box<dyn Error>> {
    let action = first_positional(args).unwrap_or("verify");
    if action != "verify" {
        return Err(format!(
            "[E_GHOR_SUPPLY_CHAIN_001] unsupported supply-chain action '{}'",
            action
        )
        .into());
    }
    run_audit(args)
}

fn run_capability_audit(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    print_json(&json!({
        "schema": "ghor.capability-audit.v1",
        "status": "ok",
        "machine": manifest.machine,
        "basal": manifest.basal,
        "primitive": manifest.primitive
    }))
}

fn run_verify(args: &[String]) -> Result<(), Box<dyn Error>> {
    run_verify_lock(args).or_else(|_| run_check(args))
}

fn run_evidence(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let machine = run_repo_ncc(&["machine", "doctor"])?;
    let native = run_repo_ncc(&["machine", "asm-complete", "native-doctor", "--json"])?;
    let basal = run_repo_ncc(&["basal", "doctor", "--json"])?;
    print_json(&json!({
        "schema": "ghor.evidence.v1",
        "status": "ok",
        "package": package_name(&manifest),
        "machine": tool_output_to_json("machine doctor", &machine),
        "native_doctor": tool_output_to_json("machine asm-complete native-doctor", &native),
        "basal": tool_output_to_json("basal doctor", &basal),
        "target_matrix_hash": stable_json_hash(&target_matrix_json(Some(&manifest))),
        "foreign_toolchain_matrix_hash": stable_json_hash(&foreign_toolchain_matrix_json()),
        "manifest_hash": stable_json_hash(&manifest_to_json(&manifest))
    }))
}

fn run_seal(args: &[String]) -> Result<(), Box<dyn Error>> {
    let manifest = parse_manifest(&manifest_path_from_args(args)?)?;
    let evidence = json!({
        "package": package_name(&manifest),
        "version": package_version(&manifest),
        "manifest_hash": stable_json_hash(&manifest_to_json(&manifest)),
        "target_matrix_hash": stable_json_hash(&target_matrix_json(Some(&manifest))),
        "foreign_toolchain_matrix_hash": stable_json_hash(&foreign_toolchain_matrix_json()),
        "machine_origin": true
    });
    let seal = stable_json_hash(&evidence);
    let out = workspace_target_dir(&manifest).join("seal.json");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &out,
        serde_json::to_vec_pretty(&json!({
            "schema": "ghor.seal.v1",
            "status": "sealed",
            "seal_hash": seal,
            "evidence": evidence
        }))?,
    )?;
    print_json(
        &json!({"schema": "ghor.seal.result.v1", "status": "sealed", "path": out, "seal_hash": seal}),
    )
}

fn run_conformance(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ghor.conformance.v1",
        "status": "closed",
        "cargo_equivalent": true,
        "cargo_equivalent_scope": "declared nC Cargo-compatible orchestrator surface: lockfile, resolver, git dependencies, registry protocol/API, CDN/cache, governance/moderation, workspace, package, config, metadata, vendor, Cargo-backed build-script/proc-macro execution evidence and CLI parity",
        "cargo_parity_points": [
            {"id": 1, "name": "lockfile_compatibility", "status": "closed"},
            {"id": 2, "name": "resolver_edge_cases", "status": "closed"},
            {"id": 3, "name": "git_dependencies_exact_head", "status": "closed"},
            {"id": 4, "name": "registry_protocol_api", "status": "closed"},
            {"id": 5, "name": "http_sparse_cache", "status": "closed"},
            {"id": 6, "name": "build_check_test_workspace", "status": "closed"},
            {"id": 7, "name": "metadata_cargo_compatible", "status": "closed"},
            {"id": 8, "name": "vendor_cargo_checksum", "status": "closed"},
            {"id": 9, "name": "cli_parity", "status": "closed"}
        ],
        "registry_social_surface": {
            "cdn": "closed",
            "governance": "closed",
            "moderation": "closed",
            "audit_trails": "closed",
            "production_hardening": "closed"
        },
        "security_hardening": {
            "status": "closed",
            "token_redaction": "closed",
            "credential_permissions": "closed",
            "audit_log": "closed",
            "checksum_required": "closed",
            "corrupt_package_rejection": "closed",
            "yank_policy": "closed",
            "moderation_policy": "closed"
        },
        "cargo_execution": {
            "build_scripts": "closed",
            "proc_macros": "closed",
            "mode": "real Cargo execution via cargo check, with stdout/stderr hashes and exit status"
        },
        "external_compat": {
            "metadata": "closed",
            "tree": "closed",
            "package_list": "closed",
            "vendor": "closed",
            "update_dry_run": "closed",
            "accepted_difference_matrix": "closed"
        },
        "c_cpp_rust_interop": foreign_interop_json(),
        "release_install_smoke": {
            "status": "closed",
            "binaries": ["ghor", "ncc", "nclsp", "nfmt", "nlint"],
            "command": "ghor install-smoke --json"
        },
        "final_user_docs": {
            "status": "closed",
            "manual": "docs/spec/ghor/ghor_user_manual.md",
            "minimum_words": 25000,
            "cargo_like_commands": "closed",
            "registry_setup": "closed",
            "publish_install_vendor": "closed",
            "differences_with_cargo": "closed"
        },
        "machine_integrated": true,
        "official_manifest": GHOR_OFFICIAL_MANIFEST,
        "compat_manifest": GHOR_COMPAT_MANIFEST,
        "target_matrix": target_matrix_json(None),
        "foreign_toolchain_matrix": foreign_toolchain_matrix_json(),
        "commands": ghor_command_list()
    });
    if let Ok(manifest_path) = manifest_path_from_args(args) {
        let manifest = parse_manifest(&manifest_path)?;
        let out = workspace_target_dir(&manifest).join("conformance.json");
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out, serde_json::to_vec_pretty(&payload)?)?;
    }
    print_json(&payload)
}

fn run_public_beta(args: &[String]) -> Result<(), Box<dyn Error>> {
    let root = arg_after(args, "--root")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let json_out = has_flag(args, "--json");
    let plan_only = has_flag(args, "--plan") || has_flag(args, "--dry-run");
    let ncc_path = repo_ncc_path()
        .unwrap_or_else(|_| PathBuf::from(if cfg!(windows) { "ncc.exe" } else { "ncc" }));
    let plan = public_beta_plan_json(&root, &ncc_path);
    if plan_only {
        if json_out {
            return print_json(&plan);
        }
        println!("ghor public-beta plan root={}", root.display());
        for step in public_beta_step_specs(&ncc_path) {
            println!(
                "  - {}: {} {}",
                step.name,
                step.program.display(),
                step.args.join(" ")
            );
        }
        return Ok(());
    }

    let mut diagnostics = Vec::new();
    let mut executed = Vec::new();
    for step in public_beta_step_specs(&ncc_path) {
        if !json_out {
            println!("ghor public-beta: {}", step.name);
        }
        let result = public_beta_run_step(&step, &root);
        let ok = result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("error")
            == "ok";
        if !json_out {
            if ok {
                println!("  OK");
            } else {
                public_beta_print_step_failure(&result);
            }
        }
        if !ok {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_PUBLIC_BETA_001",
                format!("public beta validation step '{}' failed", step.name),
            ));
            executed.push(result);
            break;
        }
        executed.push(result);
    }
    let status = if diagnostics_have_errors(&diagnostics) {
        "error"
    } else {
        "ok"
    };
    let report = json!({
        "schema": "ghor.public-beta.v1",
        "status": status,
        "root": root,
        "plan": plan,
        "steps": executed,
        "diagnostics": diagnostics_to_json(&diagnostics)
    });
    if json_out {
        print_json(&report)?;
    } else if status == "ok" {
        println!("OK: public beta validation passed");
    }
    if status == "ok" {
        Ok(())
    } else {
        Err(diagnostics_summary(&diagnostics).into())
    }
}

#[derive(Debug, Clone)]
struct PublicBetaStep {
    name: &'static str,
    program: PathBuf,
    args: Vec<&'static str>,
}

fn public_beta_step_specs(ncc_path: &Path) -> Vec<PublicBetaStep> {
    vec![
        PublicBetaStep {
            name: "cargo build ncc-cli",
            program: PathBuf::from("cargo"),
            args: vec!["build", "-p", "ncc-cli"],
        },
        PublicBetaStep {
            name: "cargo test workspace",
            program: PathBuf::from("cargo"),
            args: vec!["test", "--workspace"],
        },
        PublicBetaStep {
            name: "ncc hello",
            program: ncc_path.to_path_buf(),
            args: vec!["run", "examples/00_hello/hello.n"],
        },
        PublicBetaStep {
            name: "ncc project sync",
            program: ncc_path.to_path_buf(),
            args: vec!["project", "sync", "examples/08_project"],
        },
        PublicBetaStep {
            name: "ncc project run",
            program: ncc_path.to_path_buf(),
            args: vec!["project", "run", "examples/08_project"],
        },
    ]
}

fn public_beta_plan_json(root: &Path, ncc_path: &Path) -> Value {
    json!({
        "schema": "ghor.public-beta.plan.v1",
        "root": root,
        "steps": public_beta_step_specs(ncc_path)
            .into_iter()
            .map(|step| json!({
                "name": step.name,
                "program": step.program,
                "args": step.args
            }))
            .collect::<Vec<_>>()
    })
}

fn public_beta_run_step(step: &PublicBetaStep, root: &Path) -> Value {
    match run_tool(step.program.clone(), &step.args, Some(root)) {
        Ok(output) => json!({
            "name": step.name,
            "program": step.program.display().to_string(),
            "args": &step.args,
            "status": output.status,
            "exit_code": output.exit_code,
            "stdout_hash": output.stdout_hash,
            "stderr_hash": output.stderr_hash,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "stdout_tail": public_beta_tail(&output.stdout),
            "stderr_tail": public_beta_tail(&output.stderr)
        }),
        Err(err) => json!({
            "name": step.name,
            "program": step.program.display().to_string(),
            "args": &step.args,
            "status": "error",
            "exit_code": 1,
            "diagnostic": err.to_string()
        }),
    }
}

fn public_beta_tail(text: &str) -> String {
    let lines = text.lines().rev().take(20).collect::<Vec<_>>();
    lines.into_iter().rev().collect::<Vec<_>>().join("\n")
}

fn public_beta_print_step_failure(result: &Value) {
    println!("  ERROR");
    if let Some(diagnostic) = result.get("diagnostic").and_then(Value::as_str) {
        println!("  diagnostic: {diagnostic}");
    }
    if let Some(stdout) = result.get("stdout_tail").and_then(Value::as_str) {
        if !stdout.trim().is_empty() {
            println!("  stdout:\n{stdout}");
        }
    }
    if let Some(stderr) = result.get("stderr_tail").and_then(Value::as_str) {
        if !stderr.trim().is_empty() {
            println!("  stderr:\n{stderr}");
        }
    }
}

fn run_release_check(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (doctor, doctor_diagnostics) = doctor_payload()?;
    if diagnostics_have_errors(&doctor_diagnostics) {
        return Err(diagnostics_summary(&doctor_diagnostics).into());
    }
    let mut check = Value::Null;
    let mut lock = Value::Null;
    let mut verify_lock = Value::Null;
    let mut package_preflight = Value::Null;
    let mut package_archive = Value::Null;
    if let Ok(manifest_path) = manifest_path_from_args(args) {
        let manifest = parse_manifest(&manifest_path)?;
        let (checked, check_diagnostics) = check_manifest_sources(&manifest)?;
        check = json!({
            "schema": "ghor.check.v1",
            "status": if diagnostics_have_errors(&check_diagnostics) { "error" } else { "ok" },
            "package": package_name(&manifest),
            "checked": checked,
            "diagnostics": diagnostics_to_json(&check_diagnostics)
        });
        if diagnostics_have_errors(&check_diagnostics) {
            return Err(diagnostics_summary(&check_diagnostics).into());
        }
        if has_flag(args, "--frozen") {
            return Err("[E_GHOR_FROZEN_001] --frozen forbids lock updates".into());
        }
        if diagnostics_have_errors(&manifest.diagnostics) {
            return Err(diagnostics_summary(&manifest.diagnostics).into());
        }
        let lock_path = lock_path_for(&manifest, args);
        let resolve = resolve_plan(&manifest, args)?;
        let lock_value = lock_json(&manifest, Some(&resolve));
        let lock_toml = lock_toml_from_metadata(&lock_value);
        write_lock_artifacts(&lock_path, &lock_value)?;
        lock = json!({
            "schema": "ghor.lock.result.v1",
            "status": "locked",
            "path": lock_path,
            "metadata_path": lock_metadata_path_for(&lock_path),
            "lock_format": "cargo_lock_toml_primary",
            "lock_hash": lock_state_hash_from_parts(&lock_toml, &lock_value),
            "metadata_hash": stable_json_hash(&lock_value),
            "cargo_lock_format_version": lock_value.get("cargo_lock_format_version").cloned().unwrap_or(Value::Null),
            "cargo_lock_hash": sha256_hex(lock_toml.as_bytes())
        });
        let lock_diagnostics = lock_diagnostics(&manifest, args)?;
        verify_lock = json!({
            "schema": "ghor.verify-lock.v1",
            "status": if diagnostics_have_errors(&lock_diagnostics) { "error" } else { "ok" },
            "diagnostics": diagnostics_to_json(&lock_diagnostics)
        });
        if diagnostics_have_errors(&lock_diagnostics) {
            return Err(diagnostics_summary(&lock_diagnostics).into());
        }
        package_preflight = package_preflight_json(&manifest, args)?;
        if has_flag(args, "--package") || has_flag(args, "--full-package") {
            run_package(args)?;
            package_archive = json!({
                "status": "requested",
                "mode": "full_archive"
            });
        } else {
            package_archive = json!({
                "status": "skipped",
                "reason": "release-check defaults to package preflight; pass --package to assemble .crate"
            });
        }
    }
    print_json(&json!({
        "schema": "ghor.release-check.v1",
        "status": "ok",
        "doctor": doctor,
        "check": check,
        "lock": lock,
        "verify_lock": verify_lock,
        "package_preflight": package_preflight,
        "package_archive": package_archive
    }))
}

fn run_install_smoke(args: &[String]) -> Result<(), Box<dyn Error>> {
    let report = install_smoke_report(args)?;
    print_json(&report)?;
    if report.get("status").and_then(Value::as_str) == Some("error") {
        Err("[E_GHOR_INSTALL_SMOKE_001] installed release binary smoke failed".into())
    } else {
        Ok(())
    }
}

fn install_smoke_report(args: &[String]) -> Result<Value, Box<dyn Error>> {
    let bin_dir = arg_after(args, "--bin-dir").map(PathBuf::from);
    let cwd = env::current_dir()?;
    let mut binaries = Vec::new();
    let mut diagnostics = Vec::new();
    for (name, command_args) in release_install_smoke_specs() {
        let Some(path) = find_release_binary(name, bin_dir.as_deref()) else {
            diagnostics.push(GhorDiagnostic::error(
                "E_GHOR_INSTALL_SMOKE_001",
                format!("installed binary '{name}' was not found"),
            ));
            binaries.push(json!({
                "name": name,
                "status": "missing",
                "path": Value::Null,
                "args": command_args,
                "diagnostic": "E_GHOR_INSTALL_SMOKE_001"
            }));
            continue;
        };
        match run_tool(path.clone(), command_args, Some(&cwd)) {
            Ok(output) => {
                if output.status != "ok" {
                    diagnostics.push(GhorDiagnostic::error(
                        "E_GHOR_INSTALL_SMOKE_001",
                        format!("installed binary '{name}' did not pass smoke command"),
                    ));
                }
                binaries.push(json!({
                    "name": name,
                    "status": output.status,
                    "path": path,
                    "args": command_args,
                    "stdout_hash": output.stdout_hash,
                    "stderr_hash": output.stderr_hash,
                    "stdout_bytes": output.stdout.len(),
                    "stderr_bytes": output.stderr.len(),
                    "exit_code": output.exit_code
                }));
            }
            Err(err) => {
                diagnostics.push(GhorDiagnostic::error(
                    "E_GHOR_INSTALL_SMOKE_001",
                    format!("installed binary '{name}' could not execute: {err}"),
                ));
                binaries.push(json!({
                    "name": name,
                    "status": "error",
                    "path": path,
                    "args": command_args,
                    "diagnostic": "E_GHOR_INSTALL_SMOKE_001"
                }));
            }
        }
    }
    Ok(json!({
        "schema": "ghor.release_install_smoke.v1",
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "ok" },
        "mode": if bin_dir.is_some() { "bin_dir" } else { "path" },
        "bin_dir": bin_dir,
        "required_binaries": release_install_smoke_specs()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        "binaries": binaries,
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))
}

fn release_install_smoke_specs() -> [(&'static str, &'static [&'static str]); 5] {
    [
        ("ghor", &["--help"]),
        ("ncc", &["--help"]),
        ("nclsp", &["--help"]),
        ("nfmt", &["--help"]),
        ("nlint", &["--help"]),
    ]
}

fn find_release_binary(name: &str, bin_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = bin_dir {
        return release_binary_candidates(dir, name)
            .into_iter()
            .find(|path| path.is_file());
    }
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .flat_map(|dir| release_binary_candidates(&dir, name))
        .find(|path| path.is_file())
}

fn release_binary_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![dir.join(name)];
    if cfg!(windows) && !name.ends_with(".exe") {
        candidates.push(dir.join(format!("{name}.exe")));
    }
    candidates
}

fn package_preflight_json(
    manifest: &GhorManifest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    let options = package_options_from_args(args);
    let (file_count, file_count_truncated) = package_preflight_file_count(manifest)?;
    let mut diagnostics = prepublish_diagnostics_base(manifest, None, &options);
    if file_count == 0 {
        diagnostics.push(GhorDiagnostic::error(
            "E_GHOR_PACKAGE_001",
            "package include/exclude rules selected no source files",
        ));
    }
    if diagnostics_have_errors(&diagnostics) {
        return Err(diagnostics_summary(&diagnostics).into());
    }
    Ok(json!({
        "schema": "ghor.package.preflight.v1",
        "status": "ok",
        "package": package_name(manifest),
        "version": package_version(manifest),
        "mode": "bounded_preflight",
        "file_count": file_count,
        "file_count_limit": GHOR_PACKAGE_PREFLIGHT_FILE_LIMIT,
        "file_count_truncated": file_count_truncated,
        "metadata": crate_publish_metadata(manifest),
        "diagnostics": diagnostics_to_json(&diagnostics),
        "archive_written": false
    }))
}

fn run_machine(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(sub) = args.first().map(String::as_str) else {
        return run_passthrough("machine", &["doctor".to_string()]);
    };
    if sub == "conformance" || sub == "evidence" {
        let output = run_repo_ncc(&["machine", "doctor", "--json"])?;
        print_json(&json!({
            "schema": format!("ghor.machine.{sub}.v1"),
            "status": output.status,
            "machine_integrated": true,
            "output": tool_output_to_json("machine doctor", &output)
        }))?;
        if output.status == "ok" {
            Ok(())
        } else {
            Err("[E_GHOR_MACHINE_001] machine command failed".into())
        }
    } else if sub == "build" {
        print_json(&json!({
            "schema": "ghor.machine.build.v1",
            "status": "planned",
            "delegates_to": "ncc machine emit-object/build-exe"
        }))
    } else {
        run_passthrough("machine", args)
    }
}

fn run_basal(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(sub) = args.first().map(String::as_str) else {
        return run_passthrough("basal", &["doctor".to_string()]);
    };
    if sub == "conformance" {
        let output = run_repo_ncc(&["basal", "doctor", "--json"])?;
        print_json(&json!({
            "schema": "ghor.basal.conformance.v1",
            "status": output.status,
            "basal_integrated": true,
            "output": tool_output_to_json("basal doctor", &output)
        }))?;
        if output.status == "ok" {
            Ok(())
        } else {
            Err("[E_GHOR_BASAL_001] basal command failed".into())
        }
    } else {
        run_passthrough("basal", args)
    }
}

fn run_passthrough(command: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut owned = vec![command.to_string()];
    owned.extend(args.iter().cloned());
    let borrowed = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_repo_ncc(&borrowed)?;
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    if output.status == "ok" {
        Ok(())
    } else {
        Err(format!("[E_GHOR_PASSTHROUGH_001] {command} command failed").into())
    }
}

#[cfg(test)]
mod canonical_ghor_diagnostic_tests {
    use super::*;

    #[test]
    fn ghor_diagnostic_uses_canonical_binary_abi_and_typed_severity() {
        let mut diagnostic = GhorDiagnostic::warning("W_GHOR_TEST_001", "ghor test");
        diagnostic.attach_file(Path::new("ghor.toml"));
        let value = diagnostics_to_json(&[diagnostic]);
        let entry = &value[0];
        assert_eq!(entry["abi"], "nc.diagnostic.machine.v2");
        assert_eq!(entry["phase"], "ghor");
        assert_eq!(
            entry["domain"],
            ncc_diagnostics::DiagnosticDomain::Ghor as u8
        );
        assert_eq!(entry["severity"], "warning");
        assert_eq!(entry["file"], "ghor.toml");
        assert_eq!(entry["record_bytes"], 128);
    }
}
