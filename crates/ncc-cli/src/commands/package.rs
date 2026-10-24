use super::super::*;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PACKAGE_MANIFEST: &str = "ncc.package.json";
const RESERVED_PRIMITIVE_PACKAGE_MANIFEST: &str = "package.ncp";
const PACKAGE_LOCK: &str = "ncc.lock";
const PACKAGE_SCHEMA: &str = "ncc.package.v1";
const PACKAGE_LOCK_SCHEMA: &str = "ncc.package.lock.v1";
const PACKAGE_RESOLVE_SCHEMA: &str = "ncc.package.resolve.v1";

#[derive(Debug, Default)]
struct PackageArgs {
    path: Option<PathBuf>,
    out: Option<PathBuf>,
    import: Option<String>,
    version: Option<String>,
    git: Option<String>,
    dep_path: Option<String>,
    profile: Option<String>,
    cc: Option<String>,
    cpu: Option<String>,
    tune_profile: Option<String>,
    linker: Option<String>,
    entry: Option<String>,
    force: bool,
    json: bool,
    no_exe: bool,
    positionals: Vec<String>,
}

fn package_filesystem<T>(
    phase: &str,
    operation: impl Into<String>,
    path: &Path,
    result: std::io::Result<T>,
) -> Result<T, CliFailure> {
    result.map_err(|error| CliFailure::filesystem(phase, operation, path, error))
}

fn package_serialize_json(
    value: &serde_json::Value,
    phase: &str,
    operation: impl Into<String>,
    path: Option<&Path>,
    pretty: bool,
) -> Result<String, CliFailure> {
    let result = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    result.map_err(|error| CliFailure::json(phase, operation, path, error))
}

fn package_contract_error(
    code: &'static str,
    phase: &'static str,
    message: impl Into<String>,
) -> CliFailure {
    CliFailure::error(
        code,
        phase,
        message,
        "fix the package metadata or regenerate it with the corresponding `ncc package` command",
    )
}

pub(crate) fn run_package_command(args: &[String]) -> Result<(), CliFailure> {
    if args.is_empty() || (args.len() == 1 && is_help_token(&args[0])) {
        println!(
            "{}",
            command_help("package").expect("package help must exist")
        );
        return Ok(());
    }

    let action = args[0].as_str();
    let opts = parse_package_args(&args[1..])?;
    match action {
        "init" | "new" => package_init(&opts),
        "add" => package_add(&opts),
        "remove" | "rm" => package_remove(&opts),
        "manifest" => package_manifest(&opts),
        "resolve" => package_resolve(&opts),
        "lock" => package_lock(&opts),
        "verify-lock" => package_verify_lock(&opts),
        "check" => package_check_command(&opts),
        "build" => package_build(&opts),
        "run" => package_run(&opts),
        "clean" => package_clean(&opts),
        "doctor" => package_doctor(&opts),
        "-h" | "--help" | "help" => {
            println!("{}", command_help("package").expect("package help must exist"));
            Ok(())
        }
        other => Err(actionable_command_error(
            "package",
            format!(
                "package: unknown action '{}'; expected init/add/remove/manifest/resolve/lock/verify-lock/check/build/run/clean/doctor",
                other
            ),
        )
        .into()),
    }
}

fn parse_package_args(args: &[String]) -> Result<PackageArgs, CliFailure> {
    let mut opts = PackageArgs::default();
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" | "help" => {
                opts.positionals.push("--help".to_string());
                idx += 1;
            }
            "--path" | "--workspace" => {
                opts.path = Some(PathBuf::from(package_required_value(args, idx, "--path")?));
                idx += 2;
            }
            "--out" => {
                opts.out = Some(PathBuf::from(package_required_value(args, idx, "--out")?));
                idx += 2;
            }
            "--import" => {
                opts.import = Some(package_required_value(args, idx, "--import")?);
                idx += 2;
            }
            "--version" | "--require" => {
                opts.version = Some(package_required_value(args, idx, "--version")?);
                idx += 2;
            }
            "--git" => {
                opts.git = Some(package_required_value(args, idx, "--git")?);
                idx += 2;
            }
            "--dep-path" | "--local" => {
                opts.dep_path = Some(package_required_value(args, idx, "--dep-path")?);
                idx += 2;
            }
            "--profile" => {
                opts.profile = Some(package_required_value(args, idx, "--profile")?);
                idx += 2;
            }
            "--cc" => {
                opts.cc = Some(package_required_value(args, idx, "--cc")?);
                idx += 2;
            }
            "--cpu" => {
                opts.cpu = Some(package_required_value(args, idx, "--cpu")?);
                idx += 2;
            }
            "--tune-profile" => {
                opts.tune_profile = Some(package_required_value(args, idx, "--tune-profile")?);
                idx += 2;
            }
            "--linker" => {
                opts.linker = Some(package_required_value(args, idx, "--linker")?);
                idx += 2;
            }
            "--entry" => {
                opts.entry = Some(package_required_value(args, idx, "--entry")?);
                idx += 2;
            }
            "--force" => {
                opts.force = true;
                idx += 1;
            }
            "--json" => {
                opts.json = true;
                idx += 1;
            }
            "--no-exe" | "--check-only" => {
                opts.no_exe = true;
                idx += 1;
            }
            other if other.starts_with("--") => {
                return Err(actionable_command_error(
                    "package",
                    format!("package: unknown option '{}'", other),
                )
                .into());
            }
            other => {
                opts.positionals.push(other.to_string());
                idx += 1;
            }
        }
    }
    Ok(opts)
}

fn package_required_value(args: &[String], idx: usize, flag: &str) -> Result<String, CliFailure> {
    args.get(idx + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| {
            actionable_command_error("package", format!("package: {} requires a value", flag))
                .into()
        })
}

fn package_init(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_init(opts)?;
    let name = opts
        .positionals
        .first()
        .cloned()
        .or_else(|| {
            root.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| "app".to_string());
    let clean_name = package_clean_name(&name);
    let manifest_path = root.join(PACKAGE_MANIFEST);
    let reserved_manifest_path = root.join(RESERVED_PRIMITIVE_PACKAGE_MANIFEST);
    if reserved_manifest_path.exists() && !opts.force {
        return Err(actionable_command_error(
            "package",
            format!(
                "package init: '{}' uses the .ncp extension reserved for primitive[@]/physical hardware sources; use '{}' for package metadata",
                reserved_manifest_path.display(),
                manifest_path.display()
            ),
        )
        .into());
    }
    if manifest_path.exists() && !opts.force {
        return Err(actionable_command_error(
            "package",
            format!(
                "package init: {} already exists; pass --force to replace package metadata",
                manifest_path.display()
            ),
        )
        .into());
    }

    package_create_workspace_dirs(&root)?;
    let source_dir = root.join("src");
    package_filesystem(
        "package.init",
        "create source directory",
        &source_dir,
        fs::create_dir_all(&source_dir),
    )?;

    let manifest = package_default_manifest(&clean_name);
    package_write_json(&manifest_path, &manifest)?;

    let main_path = root.join("src").join("main.n");
    if !main_path.exists() || opts.force {
        package_filesystem(
            "package.init",
            "write package main source",
            &main_path,
            fs::write(
                &main_path,
                package_default_main_source(&clean_name).as_bytes(),
            ),
        )?;
    }

    let readme_path = root.join("README.md");
    if !readme_path.exists() || opts.force {
        let readme = format!(
                "# {}\n\nProyecto nC gestionado por `ncc package`.\n\nComandos utiles:\n\n```powershell\nncc package check --path .\nncc package build --path .\nncc package run --path .\n```\n",
                clean_name
            );
        package_filesystem(
            "package.init",
            "write package README",
            &readme_path,
            fs::write(&readme_path, readme.as_bytes()),
        )?;
    }

    let report = serde_json::json!({
        "schema": "ncc.package.init.v1",
        "status": "created",
        "root": root.display().to_string(),
        "manifest": manifest_path.display().to_string(),
        "main": main_path.display().to_string(),
        "artifact_dirs": package_workspace_dir_names(),
    });
    if opts.json {
        println!(
            "{}",
            package_serialize_json(
                &report,
                "package.init",
                "serialize package initialization report",
                None,
                true,
            )?
        );
    } else {
        println!(
            "OK: package init {} main={}",
            manifest_path.display(),
            main_path.display()
        );
    }
    Ok(())
}

fn package_add(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest_path = root.join(PACKAGE_MANIFEST);
    let mut manifest = package_read_manifest(&root)?;
    let spec = opts.positionals.first().ok_or_else(|| {
        actionable_command_error("package", "package add: missing dependency name")
    })?;
    let dep = package_dependency_from_spec(spec, opts)?;
    let dep_name = dep
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or(spec)
        .to_string();
    let deps = package_dependencies_mut(&mut manifest)?;
    deps.retain(|existing| {
        existing
            .get("name")
            .and_then(|value| value.as_str())
            .map(|name| name != dep_name)
            .unwrap_or(true)
    });
    deps.push(dep);
    deps.sort_by(|a, b| {
        a.get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .cmp(b.get("name").and_then(|value| value.as_str()).unwrap_or(""))
    });
    package_write_json(&manifest_path, &manifest)?;
    if opts.json {
        println!(
            "{}",
            package_serialize_json(
                &manifest,
                "package.add",
                "serialize updated package manifest",
                Some(&manifest_path),
                true,
            )?
        );
    } else {
        println!(
            "OK: package add {} -> {}",
            dep_name,
            manifest_path.display()
        );
    }
    Ok(())
}

fn package_remove(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest_path = root.join(PACKAGE_MANIFEST);
    let mut manifest = package_read_manifest(&root)?;
    let dep_name = opts.positionals.first().ok_or_else(|| {
        actionable_command_error("package", "package remove: missing dependency name")
    })?;
    let deps = package_dependencies_mut(&mut manifest)?;
    let before = deps.len();
    deps.retain(|existing| {
        existing
            .get("name")
            .and_then(|value| value.as_str())
            .map(|name| name != dep_name)
            .unwrap_or(true)
    });
    if before == deps.len() {
        return Err(actionable_command_error(
            "package",
            format!("package remove: dependency '{}' is not present", dep_name),
        )
        .into());
    }
    package_write_json(&manifest_path, &manifest)?;
    println!(
        "OK: package remove {} -> {}",
        dep_name,
        manifest_path.display()
    );
    Ok(())
}

fn package_manifest(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let manifest_path = root.join(PACKAGE_MANIFEST);
    println!(
        "{}",
        package_serialize_json(
            &manifest,
            "package.manifest",
            "serialize package manifest",
            Some(&manifest_path),
            true,
        )?
    );
    Ok(())
}

fn package_resolve(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let resolved = package_resolve_payload(&root, &manifest)?;
    if let Some(out) = &opts.out {
        package_write_json(out, &resolved)?;
        println!("OK: package resolve {}", out.display());
    } else {
        println!(
            "{}",
            package_serialize_json(
                &resolved,
                "package.resolve",
                "serialize dependency resolution",
                None,
                true,
            )?
        );
    }
    Ok(())
}

fn package_lock(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let lock = package_lock_payload(&root, &manifest)?;
    let out = opts.out.clone().unwrap_or_else(|| root.join(PACKAGE_LOCK));
    package_write_json(&out, &lock)?;
    if opts.json {
        println!(
            "{}",
            package_serialize_json(
                &lock,
                "package.lock",
                "serialize package lock",
                Some(&out),
                true,
            )?
        );
    } else {
        println!("OK: package lock {}", out.display());
    }
    Ok(())
}

fn package_verify_lock(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let expected = package_lock_payload(&root, &manifest)?;
    let lock_path = opts.out.clone().unwrap_or_else(|| root.join(PACKAGE_LOCK));
    let lock_bytes = package_filesystem(
        "package.verify-lock",
        "read package lock",
        &lock_path,
        fs::read(&lock_path),
    )?;
    let actual: serde_json::Value = serde_json::from_slice(&lock_bytes).map_err(|error| {
        CliFailure::json(
            "package.verify-lock",
            "parse package lock JSON",
            Some(&lock_path),
            error,
        )
    })?;
    if actual.get("schema").and_then(|value| value.as_str()) != Some(PACKAGE_LOCK_SCHEMA) {
        return Err(package_contract_error(
            "E_PACKAGE_LOCK_SCHEMA_001",
            "package.verify-lock",
            "package lock schema must be 'ncc.package.lock.v1'",
        ));
    }
    if actual != expected {
        return Err(actionable_command_error(
            "package",
            "package verify-lock failed: lock drift detected; regenerate with `ncc package lock`",
        )
        .into());
    }
    println!("OK: package verify-lock {}", lock_path.display());
    Ok(())
}

fn package_check_command(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let report = package_check(&root)?;
    if opts.json {
        println!(
            "{}",
            package_serialize_json(
                &report,
                "package.check",
                "serialize package check report",
                None,
                true,
            )?
        );
    } else {
        println!(
            "OK: package check {} main={}",
            root.join(PACKAGE_MANIFEST).display(),
            report
                .get("main")
                .and_then(|value| value.as_str())
                .unwrap_or("")
        );
    }
    Ok(())
}

fn package_build(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let check_report = package_check(&root)?;
    package_create_workspace_dirs(&root)?;
    let source = package_main_path(&root, &manifest)?;
    let stem = package_name(&manifest);
    let exe_path = opts.out.clone().unwrap_or_else(|| {
        root.join(".ncc")
            .join("bin")
            .join(package_executable_name(&stem))
    });
    let build_report_path = root
        .join(".ncc")
        .join("run")
        .join(format!("{}.package-build.json", stem));

    let mut artifacts = Vec::new();
    let mut pipeline = vec![
        "read ncc.package.json",
        "verify source exists",
        "parse .n",
        "sema",
        "resolve package graph",
    ];
    let lock = package_lock_payload(&root, &manifest)?;
    package_write_json(&root.join(PACKAGE_LOCK), &lock)?;
    artifacts.push(package_artifact_record(&root.join(PACKAGE_LOCK), "lock")?);

    if !opts.no_exe {
        let mut nbuild_args = vec![
            source.display().to_string(),
            exe_path.display().to_string(),
            "--out-dir".to_string(),
            root.join(".ncc").display().to_string(),
            "--profile".to_string(),
            opts.profile
                .clone()
                .unwrap_or_else(|| "release-fast".to_string()),
        ];
        if let Some(cc) = package_build_cc(opts, &manifest) {
            nbuild_args.push("--cc".to_string());
            nbuild_args.push(cc);
        }
        if let Some(cpu) = package_build_cpu(opts, &manifest) {
            nbuild_args.push("--cpu".to_string());
            nbuild_args.push(cpu);
        }
        if let Some(tune_profile) = package_build_tune_profile(opts, &manifest) {
            nbuild_args.push("--tune-profile".to_string());
            nbuild_args.push(tune_profile);
        }
        if let Some(linker) = &opts.linker {
            nbuild_args.push("--linker".to_string());
            nbuild_args.push(linker.clone());
        }
        if let Some(entry) = &opts.entry {
            nbuild_args.push("--entry".to_string());
            nbuild_args.push(entry.clone());
        }
        super::tooling::run_nbuild_command(&nbuild_args)
            .map_err(|error| CliFailure::from_boxed_at(error, "package.build"))?;
        pipeline.extend([
            "lower high-level IR",
            "emit backend object bytes",
            "retain .nm machine bridge",
            "check retained .nm",
            "link executable",
        ]);
        let native_manifest = root
            .join(".ncc")
            .join("run")
            .join(format!("{}.native-build.json", source_stem(&source)));
        let nm_path = root
            .join(".ncc")
            .join("lowering")
            .join(format!("{}.nm", source_stem(&source)));
        let nbo_path = root
            .join(".ncc")
            .join("lowering")
            .join(format!("{}.backend-object.nbo", source_stem(&source)));
        let obj_path = root
            .join(".ncc")
            .join("obj")
            .join(format!("{}.obj", source_stem(&source)));
        for (path, kind) in [
            (native_manifest, "native-build-manifest"),
            (nm_path, "machine-source"),
            (nbo_path, "backend-object"),
            (obj_path, "object"),
            (exe_path.clone(), "executable"),
        ] {
            if path.exists() {
                artifacts.push(package_artifact_record(&path, kind)?);
            }
        }
    } else {
        pipeline.push("skip executable link by --no-exe");
    }

    let payload = serde_json::json!({
        "schema": "ncc.package.build.v1",
        "status": if opts.no_exe { "checked" } else { "built" },
        "root": root.display().to_string(),
        "manifest": root.join(PACKAGE_MANIFEST).display().to_string(),
        "main": source.display().to_string(),
        "check": check_report,
        "lock": root.join(PACKAGE_LOCK).display().to_string(),
        "executable": if opts.no_exe { serde_json::Value::Null } else { serde_json::Value::String(exe_path.display().to_string()) },
        "pipeline": pipeline,
        "artifacts": artifacts,
        "contract": {
            "normal_command": "ncc package build",
            "native_chain": ".n -> .nm -> .obj -> executable",
            "backend_truth": "nbuild emits the retained .nm bridge, native object bytes and executable; package only orchestrates the project graph.",
            "interpreter_fallback": false,
        }
    });
    package_write_json(&build_report_path, &payload)?;
    if opts.json {
        println!(
            "{}",
            package_serialize_json(
                &payload,
                "package.build",
                "serialize package build report",
                Some(&build_report_path),
                true,
            )?
        );
    } else if opts.no_exe {
        println!("OK: package build --no-exe {}", build_report_path.display());
    } else {
        println!(
            "OK: package build {} report={}",
            exe_path.display(),
            build_report_path.display()
        );
    }
    Ok(())
}

fn package_run(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let manifest = package_read_manifest(&root)?;
    let stem = package_name(&manifest);
    let exe_path = opts.out.clone().unwrap_or_else(|| {
        root.join(".ncc")
            .join("bin")
            .join(package_executable_name(&stem))
    });
    if !exe_path.exists() {
        let mut build_opts = PackageArgs {
            path: Some(root.clone()),
            out: Some(exe_path.clone()),
            ..PackageArgs::default()
        };
        build_opts.profile = opts.profile.clone();
        build_opts.cc = opts.cc.clone();
        build_opts.cpu = opts.cpu.clone();
        build_opts.tune_profile = opts.tune_profile.clone();
        build_opts.linker = opts.linker.clone();
        build_opts.entry = opts.entry.clone();
        package_build(&build_opts)?;
    }
    if !exe_path.exists() {
        return Err(actionable_command_error(
            "package",
            format!(
                "package run: executable '{}' was not produced",
                exe_path.display()
            ),
        )
        .into());
    }
    let status = Command::new(&exe_path)
        .current_dir(&root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            CliFailure::subprocess("package.run", "start package executable", &exe_path, error)
        })?;
    if !status.success() {
        return Err(CliFailure::subprocess(
            "package.run",
            "execute package binary",
            &exe_path,
            format!("process exited with {:?}", status.code()),
        )
        .into());
    }
    Ok(())
}

fn package_clean(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    for name in package_workspace_dir_names() {
        let dir = root.join(".ncc").join(name);
        if dir.exists() {
            package_filesystem(
                "package.clean",
                "remove package artifact directory",
                &dir,
                fs::remove_dir_all(&dir),
            )?;
        }
        package_filesystem(
            "package.clean",
            "recreate package artifact directory",
            &dir,
            fs::create_dir_all(&dir),
        )?;
    }
    let lock = root.join(PACKAGE_LOCK);
    if lock.exists() {
        package_filesystem(
            "package.clean",
            "remove package lock",
            &lock,
            fs::remove_file(&lock),
        )?;
    }
    println!("OK: package clean {}", root.display());
    Ok(())
}

fn package_doctor(opts: &PackageArgs) -> Result<(), CliFailure> {
    let root = package_root_for_command(opts)?;
    let report = package_check(&root)?;
    let lock_ok = root.join(PACKAGE_LOCK).exists();
    let payload = serde_json::json!({
        "schema": "ncc.package.doctor.v1",
        "status": if lock_ok { "ready" } else { "ready_without_lock" },
        "root": root.display().to_string(),
        "check": report,
        "lock_present": lock_ok,
        "artifact_dirs": package_workspace_dir_names(),
    });
    println!(
        "{}",
        package_serialize_json(
            &payload,
            "package.doctor",
            "serialize package doctor report",
            None,
            true,
        )?
    );
    Ok(())
}

fn package_check(root: &Path) -> Result<serde_json::Value, CliFailure> {
    let manifest = package_read_manifest(root)?;
    let source = package_main_path(root, &manifest)?;
    if !source.is_file() {
        return Err(actionable_command_error(
            "package",
            format!(
                "package check: main source '{}' does not exist",
                source.display()
            ),
        )
        .into());
    }
    let source_text = package_filesystem(
        "package.check",
        "read package main source",
        &source,
        fs::read_to_string(&source),
    )?;
    validate_import_surface_source_gates(source.to_string_lossy().as_ref(), &source_text)
        .map_err(|error| CliFailure::from_boxed_at(error, "package.check.imports"))?;
    let ast = load_ast_from_path(source.to_string_lossy().as_ref())
        .map_err(|error| CliFailure::from_boxed_at(error, "package.check.parser"))?;
    sema_check_with_warnings(&ast)
        .map_err(|error| CliFailure::from_boxed_at(error, "package.check.sema"))?;
    super::project::materialize_single_file_code_json(&source)
        .map_err(|error| CliFailure::from_boxed_at(error, "package.check.project"))?;
    let resolve = package_resolve_payload(root, &manifest)?;
    Ok(serde_json::json!({
        "schema": "ncc.package.check.v1",
        "status": "ok",
        "manifest": root.join(PACKAGE_MANIFEST).display().to_string(),
        "main": source.display().to_string(),
        "source_sha256": package_sha256_file(&source)?,
        "containers": ast.containers.len(),
        "dependencies": resolve.get("dependencies").cloned().unwrap_or_else(|| serde_json::json!([])),
    }))
}

fn package_default_manifest(name: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": PACKAGE_SCHEMA,
        "package": {
            "name": name,
            "version": "0.1.0",
            "edition": "2026",
            "language": "nC",
        },
        "targets": {
            "main": "src/main.n",
            "artifact_root": ".ncc",
            "bin": ".ncc/bin",
        },
        "dependencies": [],
        "build": {
            "profile": "release-fast",
            "cc": package_default_cc(),
            "cpu": "native",
            "emit": ["nm", "obj", "exe", "run-manifest"],
        }
    })
}

fn package_default_main_source(name: &str) -> String {
    format!(
        "use <io:output, render>;\n\n{} :: runtime {{\n    fn:\n        run() -> int {{\n            println(\"{} listo\");\n            return 0;\n        }}\n\n    ref main ->> run;\n\n    fa:\n        return true;\n}}\n",
        package_container_name(name),
        name
    )
}

fn package_dependency_from_spec(
    spec: &str,
    opts: &PackageArgs,
) -> Result<serde_json::Value, CliFailure> {
    let name = package_dependency_name(spec);
    let kind = if opts.git.is_some() {
        "git"
    } else if opts.dep_path.is_some() || package_looks_like_path(spec) {
        "path"
    } else if spec.starts_with("use::") || spec.contains("::") {
        "import"
    } else {
        "registry"
    };
    let mut dep = serde_json::json!({
        "name": name,
        "kind": kind,
        "require": opts.version.clone().unwrap_or_else(|| "*".to_string()),
        "source": spec,
    });
    if let Some(import) = &opts.import {
        dep["import"] = serde_json::Value::String(import.clone());
    } else if spec.starts_with("use::") {
        dep["import"] = serde_json::Value::String(spec.to_string());
    } else if spec.contains("::") {
        dep["import"] = serde_json::Value::String(format!("use::{};", spec.trim_end_matches(';')));
    }
    if let Some(git) = &opts.git {
        dep["git"] = serde_json::Value::String(git.clone());
    }
    if let Some(path) = &opts.dep_path {
        dep["path"] = serde_json::Value::String(path.clone());
    } else if package_looks_like_path(spec) {
        dep["path"] = serde_json::Value::String(spec.to_string());
    }
    let encoded = package_serialize_json(
        &dep,
        "package.add",
        "serialize dependency for checksum",
        None,
        false,
    )?;
    dep["checksum"] = serde_json::Value::String(package_sha256_text(encoded.as_bytes()));
    Ok(dep)
}

fn package_resolve_payload(
    root: &Path,
    manifest: &serde_json::Value,
) -> Result<serde_json::Value, CliFailure> {
    let deps = manifest
        .get("dependencies")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let resolved = deps
        .into_iter()
        .map(|dep| {
            let checksum = package_sha256_text(serde_json::to_string(&dep).unwrap_or_default().as_bytes());
            serde_json::json!({
                "name": dep.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "kind": dep.get("kind").cloned().unwrap_or(serde_json::Value::String("registry".to_string())),
                "require": dep.get("require").cloned().unwrap_or(serde_json::Value::String("*".to_string())),
                "source": dep.get("source").cloned().unwrap_or(serde_json::Value::Null),
                "import": dep.get("import").cloned().unwrap_or(serde_json::Value::Null),
                "path": dep.get("path").cloned().unwrap_or(serde_json::Value::Null),
                "git": dep.get("git").cloned().unwrap_or(serde_json::Value::Null),
                "checksum": checksum,
                "status": "resolved",
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "schema": PACKAGE_RESOLVE_SCHEMA,
        "status": "resolved",
        "root": root.display().to_string(),
        "package": manifest.get("package").cloned().unwrap_or(serde_json::json!({})),
        "dependencies": resolved,
    }))
}

fn package_lock_payload(
    root: &Path,
    manifest: &serde_json::Value,
) -> Result<serde_json::Value, CliFailure> {
    let resolve = package_resolve_payload(root, manifest)?;
    let locked_resolve = serde_json::json!({
        "package": manifest.get("package").cloned().unwrap_or(serde_json::json!({})),
        "dependencies": resolve.get("dependencies").cloned().unwrap_or_else(|| serde_json::json!([])),
    });
    let manifest_path = root.join(PACKAGE_MANIFEST);
    Ok(serde_json::json!({
        "schema": PACKAGE_LOCK_SCHEMA,
        "version": 1,
        "package": manifest.get("package").cloned().unwrap_or(serde_json::json!({})),
        "manifest": PACKAGE_MANIFEST,
        "manifest_sha256": package_sha256_file(&manifest_path)?,
        "resolve_sha256": package_sha256_text(package_serialize_json(
            &locked_resolve,
            "package.lock",
            "serialize dependency resolution for checksum",
            None,
            false,
        )?.as_bytes()),
        "dependencies": resolve.get("dependencies").cloned().unwrap_or_else(|| serde_json::json!([])),
    }))
}

fn package_root_for_init(opts: &PackageArgs) -> Result<PathBuf, CliFailure> {
    Ok(if let Some(path) = &opts.path {
        path.clone()
    } else if let Some(first) = opts.positionals.first() {
        PathBuf::from(first)
    } else {
        package_filesystem(
            "package.init",
            "resolve current directory",
            Path::new("."),
            env::current_dir(),
        )?
    })
}

fn package_root_for_command(opts: &PackageArgs) -> Result<PathBuf, CliFailure> {
    if let Some(path) = &opts.path {
        return Ok(path.clone());
    }
    let cwd = package_filesystem(
        "package",
        "resolve current directory",
        Path::new("."),
        env::current_dir(),
    )?;
    if let Some(legacy_root) = package_find_reserved_manifest_root(&cwd) {
        return Err(actionable_command_error(
            "package",
            format!(
                "package: found '{}' but .ncp is reserved for primitive[@]/physical hardware sources; rename package metadata to '{}'",
                legacy_root.join(RESERVED_PRIMITIVE_PACKAGE_MANIFEST).display(),
                PACKAGE_MANIFEST
            ),
        )
        .into());
    }
    package_find_root(&cwd).ok_or_else(|| {
        actionable_command_error(
            "package",
            "package: could not find ncc.package.json; pass --path <project> or run `ncc package init`",
        )
        .into()
    })
}

fn package_find_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(PACKAGE_MANIFEST).is_file() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn package_find_reserved_manifest_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(RESERVED_PRIMITIVE_PACKAGE_MANIFEST).is_file() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn package_read_manifest(root: &Path) -> Result<serde_json::Value, CliFailure> {
    let manifest_path = root.join(PACKAGE_MANIFEST);
    if !manifest_path.is_file() {
        let reserved_manifest_path = root.join(RESERVED_PRIMITIVE_PACKAGE_MANIFEST);
        if reserved_manifest_path.is_file() {
            return Err(actionable_command_error(
                "package",
                format!(
                    "package manifest '{}' is invalid here: .ncp is reserved for primitive[@]/physical hardware sources; use '{}'",
                    reserved_manifest_path.display(),
                    manifest_path.display()
                ),
            )
            .into());
        }
        return Err(actionable_command_error(
            "package",
            format!(
                "package: could not read '{}'; run `ncc package init`",
                manifest_path.display()
            ),
        )
        .into());
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        CliFailure::filesystem(
            "package.manifest",
            "read package manifest",
            &manifest_path,
            error,
        )
    })?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|error| {
        CliFailure::json(
            "package.manifest",
            "parse package manifest JSON",
            Some(&manifest_path),
            error,
        )
    })?;
    if manifest.get("schema").and_then(|value| value.as_str()) != Some(PACKAGE_SCHEMA) {
        return Err(actionable_command_error(
            "package",
            format!(
                "package manifest '{}' must declare schema '{}'",
                manifest_path.display(),
                PACKAGE_SCHEMA
            ),
        )
        .into());
    }
    Ok(manifest)
}

fn package_write_json(path: &Path, value: &serde_json::Value) -> Result<(), CliFailure> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CliFailure::filesystem(
                "package.manifest",
                "create package metadata directory",
                parent,
                error,
            )
        })?;
    }
    let rendered = serde_json::to_string_pretty(value).map_err(|error| {
        CliFailure::json(
            "package.manifest",
            "serialize package metadata JSON",
            Some(path),
            error,
        )
    })?;
    fs::write(path, (rendered + "\n").as_bytes()).map_err(|error| {
        CliFailure::filesystem("package.manifest", "write package metadata", path, error)
    })?;
    Ok(())
}

fn package_dependencies_mut(
    manifest: &mut serde_json::Value,
) -> Result<&mut Vec<serde_json::Value>, CliFailure> {
    manifest
        .get_mut("dependencies")
        .and_then(|value| value.as_array_mut())
        .ok_or_else(|| {
            package_contract_error(
                "E_PACKAGE_DEPENDENCIES_001",
                "package.manifest",
                "package manifest dependencies must be an array",
            )
        })
}

fn package_main_path(root: &Path, manifest: &serde_json::Value) -> Result<PathBuf, CliFailure> {
    let raw = manifest
        .get("targets")
        .and_then(|value| value.get("main"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            package_contract_error(
                "E_PACKAGE_MAIN_TARGET_001",
                "package.manifest",
                "package manifest targets.main must be a string",
            )
        })?;
    let path = PathBuf::from(raw);
    Ok(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}

fn package_name(manifest: &serde_json::Value) -> String {
    manifest
        .get("package")
        .and_then(|value| value.get("name"))
        .and_then(|value| value.as_str())
        .map(package_clean_name)
        .unwrap_or_else(|| "app".to_string())
}

fn package_build_cc(opts: &PackageArgs, manifest: &serde_json::Value) -> Option<String> {
    opts.cc.clone().or_else(|| {
        manifest
            .get("build")
            .and_then(|value| value.get("cc"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn package_build_cpu(opts: &PackageArgs, manifest: &serde_json::Value) -> Option<String> {
    opts.cpu.clone().or_else(|| {
        manifest
            .get("build")
            .and_then(|value| value.get("cpu"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn package_build_tune_profile(opts: &PackageArgs, manifest: &serde_json::Value) -> Option<String> {
    opts.tune_profile.clone().or_else(|| {
        manifest
            .get("build")
            .and_then(|value| value.get("tune_profile"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn package_create_workspace_dirs(root: &Path) -> Result<(), CliFailure> {
    for dir in package_workspace_dir_names() {
        let path = root.join(".ncc").join(dir);
        package_filesystem(
            "package.workspace",
            "create package workspace directory",
            &path,
            fs::create_dir_all(&path),
        )?;
    }
    Ok(())
}

fn package_workspace_dir_names() -> Vec<&'static str> {
    vec![
        "bin",
        "code",
        "incremental",
        "lowering",
        "obj",
        "run",
        "cache",
        "ide",
    ]
}

fn package_artifact_record(path: &Path, kind: &str) -> Result<serde_json::Value, CliFailure> {
    let metadata = package_filesystem(
        "package.artifact",
        "read package artifact metadata",
        path,
        fs::metadata(path),
    )?;
    Ok(serde_json::json!({
        "kind": kind,
        "path": path.display().to_string(),
        "size": metadata.len(),
        "sha256": package_sha256_file(path)?,
    }))
}

fn package_sha256_file(path: &Path) -> Result<String, CliFailure> {
    let bytes = package_filesystem(
        "package.hash",
        "read package artifact for hashing",
        path,
        fs::read(path),
    )?;
    Ok(package_sha256_text(&bytes))
}

fn package_sha256_text(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn package_dependency_name(spec: &str) -> String {
    spec.trim()
        .trim_end_matches(';')
        .trim_start_matches("use::")
        .replace("::<", "::")
        .replace(['<', '>', ',', ';', ' ', '\t', '\r', '\n'], "_")
}

fn package_clean_name(name: &str) -> String {
    let clean = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if clean.is_empty() {
        "app".to_string()
    } else {
        clean
    }
}

fn package_container_name(name: &str) -> String {
    let mut clean = package_clean_name(name).replace('-', "_");
    if clean
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(true)
    {
        clean = format!("pkg_{}", clean);
    }
    clean
}

fn package_executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", package_clean_name(name))
    } else {
        package_clean_name(name)
    }
}

fn package_default_cc() -> &'static str {
    if cfg!(windows) {
        "nC.win64"
    } else {
        "nC.sysv64"
    }
}

fn package_looks_like_path(spec: &str) -> bool {
    spec.contains('/') || spec.contains('\\') || spec.starts_with('.') || Path::new(spec).exists()
}

fn source_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .map(package_clean_name)
        .unwrap_or_else(|| "main".to_string())
}
