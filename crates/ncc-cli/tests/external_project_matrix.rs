use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn ghor_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ghor")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ghor must be set")
}

fn temp_matrix_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc_external_matrix_{name}_{}_{}",
        std::process::id(),
        stamp
    ))
}

fn run_from(bin: PathBuf, cwd: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .current_dir(cwd)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("command must execute")
}

fn run_ncc_from(cwd: &Path, args: &[&str]) -> Output {
    run_from(ncc_bin(), cwd, args)
}

fn run_ghor_from(cwd: &Path, args: &[&str]) -> Output {
    run_from(ghor_bin(), cwd, args)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_success(output: Output, label: &str) -> String {
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert!(
        output.status.success(),
        "{label} failed\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn assert_failure_contains(output: Output, label: &str, needle: &str) {
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert!(
        !output.status.success(),
        "{label} should fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "{label} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn json_success(output: Output, label: &str) -> Value {
    let stdout = assert_success(output, label);
    let values = serde_json::Deserializer::from_str(&stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| {
            panic!("{label} must emit JSON: {err}\nstdout={stdout}");
        });
    values
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("{label} must emit at least one JSON value"))
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn native_exe_name(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

#[test]
fn ncc_package_external_project_builds_runs_and_detects_lock_drift() {
    let root = temp_matrix_dir("ncc_package");
    let driver_cwd = root.join("driver-cwd");
    let app = root.join("app_package");
    fs::create_dir_all(&driver_cwd).expect("driver cwd");

    assert_success(
        run_ncc_from(
            &driver_cwd,
            &["package", "init", "external_pkg", "--path", &path_str(&app)],
        ),
        "ncc package init external",
    );
    assert_success(
        run_ncc_from(
            &driver_cwd,
            &["package", "add", "stoud::io", "--path", &path_str(&app)],
        ),
        "ncc package add external dependency",
    );
    assert_success(
        run_ncc_from(&driver_cwd, &["package", "lock", "--path", &path_str(&app)]),
        "ncc package lock external",
    );
    assert_success(
        run_ncc_from(
            &driver_cwd,
            &["package", "verify-lock", "--path", &path_str(&app)],
        ),
        "ncc package verify-lock external",
    );
    assert_success(
        run_ncc_from(
            &driver_cwd,
            &["package", "check", "--path", &path_str(&app)],
        ),
        "ncc package check external",
    );
    assert_success(
        run_ncc_from(
            &driver_cwd,
            &[
                "package",
                "build",
                "--path",
                &path_str(&app),
                "--profile",
                "release-fast",
                "--cpu",
                "native",
            ],
        ),
        "ncc package build external",
    );
    let run_stdout = assert_success(
        run_ncc_from(&driver_cwd, &["package", "run", "--path", &path_str(&app)]),
        "ncc package run external",
    );
    assert!(
        run_stdout.contains("external_pkg listo"),
        "generated package executable must print expected output: {run_stdout}"
    );

    for path in [
        app.join("ncc.package.json"),
        app.join("ncc.lock"),
        app.join("src/main.n"),
        app.join(".ncc/lowering/main.nm"),
        app.join(".ncc/lowering/main.backend-object.nbo"),
        app.join(".ncc/obj/main.obj"),
        app.join(".ncc/run/main.native-build.json"),
        app.join(".ncc/run/external_pkg.package-build.json"),
        app.join(".ncc/bin").join(native_exe_name("external_pkg")),
    ] {
        assert!(
            path.exists(),
            "expected external artifact {}",
            path.display()
        );
    }

    let manifest_path = app.join("ncc.package.json");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("package manifest bytes"))
            .expect("package manifest json");
    manifest["package"]["version"] = Value::String("0.1.1".to_string());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("manifest json"),
    )
    .expect("manifest drift write");

    assert_failure_contains(
        run_ncc_from(
            &driver_cwd,
            &["package", "verify-lock", "--path", &path_str(&app)],
        ),
        "ncc package verify-lock drift",
        "lock drift",
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ghor_external_path_project_builds_packages_vendors_and_detects_lock_drift() {
    let root = temp_matrix_dir("ghor_path");
    let driver_cwd = root.join("driver-cwd");
    let dep = root.join("external_dep");
    let app = root.join("external_app");
    fs::create_dir_all(&driver_cwd).expect("driver cwd");

    assert_success(
        run_ghor_from(
            &driver_cwd,
            &["new", "matrix_dep", "--out", &path_str(&dep)],
        ),
        "ghor new external dependency",
    );
    assert_success(
        run_ghor_from(
            &driver_cwd,
            &["new", "matrix_app", "--out", &path_str(&app)],
        ),
        "ghor new external app",
    );

    let manifest = app.join("ghor.toml");
    let manifest_arg = path_str(&manifest);
    assert_success(
        run_ghor_from(
            &driver_cwd,
            &[
                "add",
                "matrix_dep",
                "--manifest-path",
                &manifest_arg,
                "--path",
                &path_str(&dep),
                "--version",
                "0.1.0",
            ],
        ),
        "ghor add external path dependency",
    );

    let resolve = json_success(
        run_ghor_from(
            &driver_cwd,
            &["resolve", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor resolve external path dependency",
    );
    assert_eq!(resolve["status"], "ok");
    assert!(
        resolve["resolved"]
            .as_array()
            .expect("resolved")
            .iter()
            .any(|entry| entry["name"] == "matrix_dep" && entry["status"] == "resolved_path"),
        "resolve must include path dependency: {resolve}"
    );

    assert_success(
        run_ghor_from(
            &driver_cwd,
            &["check", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor check external path app",
    );
    let lock = json_success(
        run_ghor_from(
            &driver_cwd,
            &["lock", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor lock external path app",
    );
    assert_eq!(lock["status"], "locked");
    assert!(app.join("ghor.lock").exists(), "ghor.lock must exist");
    assert!(
        app.join("ghor.lock.meta.json").exists(),
        "ghor.lock.meta.json must exist"
    );
    assert_success(
        run_ghor_from(
            &driver_cwd,
            &["verify-lock", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor verify-lock external path app",
    );

    let build = json_success(
        run_ghor_from(
            &driver_cwd,
            &[
                "build",
                "--manifest-path",
                &manifest_arg,
                "--profile",
                "release",
                "--json",
            ],
        ),
        "ghor build external path app",
    );
    assert_eq!(build["status"], "built");
    let build_artifact = build["artifacts"][0]["artifact"]
        .as_str()
        .expect("build artifact path");
    assert!(
        Path::new(build_artifact).exists(),
        "build artifact must exist: {build_artifact}"
    );

    let run = json_success(
        run_ghor_from(
            &driver_cwd,
            &["run", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor run external path app",
    );
    assert_eq!(run["status"], "ok");

    let package_dir = app.join("target/external-package");
    let package = json_success(
        run_ghor_from(
            &driver_cwd,
            &[
                "package",
                "--manifest-path",
                &manifest_arg,
                "--out",
                &path_str(&package_dir),
                "--json",
            ],
        ),
        "ghor package external path app",
    );
    assert_eq!(package["status"], "packaged");
    assert!(
        package_dir.join("manifest.json").exists(),
        "package manifest evidence must exist"
    );
    let archive = package["archive"].as_str().expect("package archive path");
    assert!(Path::new(archive).exists(), "archive must exist: {archive}");

    let list = json_success(
        run_ghor_from(
            &driver_cwd,
            &[
                "package",
                "--list",
                "--manifest-path",
                &manifest_arg,
                "--json",
            ],
        ),
        "ghor package --list external path app",
    );
    assert!(
        list["files"]
            .as_array()
            .expect("package files")
            .iter()
            .any(|entry| entry.as_str().is_some_and(|file| file == "src/main.n")),
        "package list must include src/main.n: {list}"
    );

    let vendor_dir = app.join("third_party/vendor");
    let vendor = json_success(
        run_ghor_from(
            &driver_cwd,
            &[
                "vendor",
                &path_str(&vendor_dir),
                "--manifest-path",
                &manifest_arg,
                "--json",
            ],
        ),
        "ghor vendor external path app",
    );
    assert_eq!(vendor["status"], "ok");
    let path_marker = vendor_dir.join("matrix_dep.path");
    assert!(
        path_marker.exists(),
        "path dependency vendor marker must exist"
    );
    assert!(
        fs::read_to_string(path_marker)
            .expect("path marker")
            .contains(&path_str(&dep)),
        "vendor marker must record dependency path"
    );

    let audit = json_success(
        run_ghor_from(
            &driver_cwd,
            &["audit", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor audit external path app",
    );
    assert_eq!(audit["status"], "ok");
    let sbom = json_success(
        run_ghor_from(
            &driver_cwd,
            &["sbom", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor sbom external path app",
    );
    assert!(
        sbom["dependencies"]
            .as_array()
            .expect("sbom dependencies")
            .iter()
            .any(|entry| entry["name"] == "matrix_dep"),
        "sbom must include matrix_dep: {sbom}"
    );

    let lock_path = app.join("ghor.lock");
    let mut lock_text = fs::read_to_string(&lock_path).expect("ghor.lock text");
    lock_text.push_str("\n# deliberate external matrix drift\n");
    fs::write(&lock_path, lock_text).expect("write drifted lock");
    assert_failure_contains(
        run_ghor_from(
            &driver_cwd,
            &["verify-lock", "--manifest-path", &manifest_arg, "--json"],
        ),
        "ghor verify-lock drift",
        "E_GHOR_LOCK_DRIFT_001",
    );

    let _ = fs::remove_dir_all(&root);
}
