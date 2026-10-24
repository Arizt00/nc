use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join("tmp")
        .join(format!("{}_{}_{}", name, std::process::id(), nanos))
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn assert_success(out: Output, label: &str) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    assert!(
        out.status.success(),
        "{label} failed\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn write_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write fixture file");
}

fn json_stdout(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout).unwrap_or_else(|err| panic!("parse json: {err}\n{stdout}"))
}

#[test]
fn ncc_clean_removes_transient_artifacts_and_preserves_identity_cache_and_bins() {
    let root = temp_root("ncc_clean_default");
    let _ = fs::remove_dir_all(&root);
    write_file(&root.join(".ncc/lowering/unit.lowering.json"), "{}\n");
    write_file(&root.join(".ncc/obj/unit.o"), "object");
    write_file(&root.join(".ncc/run/last.json"), "{}\n");
    write_file(&root.join(".ncc/incremental/status.json"), "{}\n");
    write_file(&root.join(".ncc/tmp/scratch"), "tmp");
    write_file(&root.join(".ncc/cache/ast/unit.ast.json"), "{}\n");
    write_file(&root.join(".ncc/code/main.json"), "{}\n");
    write_file(&root.join(".ncc/bin/ncc.exe"), "binary");

    let stdout = assert_success(
        run_ncc(&[
            "clean".to_string(),
            root.to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "ncc clean default",
    );
    let payload = json_stdout(&stdout);
    assert_eq!(payload["schema"], "ncc.clean.v1");
    assert_eq!(payload["status"], "ok");
    assert!(!root.join(".ncc/lowering").exists());
    assert!(!root.join(".ncc/obj").exists());
    assert!(!root.join(".ncc/run").exists());
    assert!(!root.join(".ncc/incremental").exists());
    assert!(!root.join(".ncc/tmp").exists());
    assert!(root.join(".ncc/cache/ast/unit.ast.json").is_file());
    assert!(root.join(".ncc/code/main.json").is_file());
    assert!(root.join(".ncc/bin/ncc.exe").is_file());
    assert!(!root.join(".ncc/locks/project-cache.lock").exists());
}

#[test]
fn ncc_clean_cache_removes_cache_in_addition_to_transient_artifacts() {
    let root = temp_root("ncc_clean_cache");
    let _ = fs::remove_dir_all(&root);
    write_file(&root.join(".ncc/lowering/unit.lowering.json"), "{}\n");
    write_file(&root.join(".ncc/cache/ast/unit.ast.json"), "{}\n");
    write_file(&root.join(".ncc/code/main.json"), "{}\n");

    let stdout = assert_success(
        run_ncc(&[
            "clean".to_string(),
            root.to_string_lossy().to_string(),
            "--cache".to_string(),
            "--json".to_string(),
        ]),
        "ncc clean cache",
    );
    let payload = json_stdout(&stdout);
    assert_eq!(payload["schema"], "ncc.clean.v1");
    assert_eq!(payload["cache"], true);
    assert!(!root.join(".ncc/lowering").exists());
    assert!(!root.join(".ncc/cache").exists());
    assert!(root.join(".ncc/code/main.json").is_file());
}

#[test]
fn ncc_clean_dry_run_reports_exact_usage_without_mutating_the_project() {
    let root = temp_root("ncc_clean_dry_run");
    let _ = fs::remove_dir_all(&root);
    write_file(&root.join(".ncc/lowering/a.bin"), "1234");
    write_file(&root.join(".ncc/lowering/nested/b.bin"), "56789");

    let stdout = assert_success(
        run_ncc(&[
            "clean".to_string(),
            root.to_string_lossy().to_string(),
            "--dry-run".to_string(),
            "--json".to_string(),
        ]),
        "ncc clean dry run",
    );
    let payload = json_stdout(&stdout);
    assert_eq!(payload["dry_run"], true);
    assert_eq!(payload["reclaimable_bytes"], 9);
    assert_eq!(payload["reclaimable_files"], 2);
    assert_eq!(payload["removed_bytes"], 0);
    assert_eq!(payload["removed"].as_array().map(Vec::len), Some(0));
    assert_eq!(payload["planned"].as_array().map(Vec::len), Some(1));
    assert!(root.join(".ncc/lowering/a.bin").is_file());
    assert!(root.join(".ncc/lowering/nested/b.bin").is_file());
    assert!(!root.join(".ncc/locks").exists());
}

#[test]
fn ncc_clean_build_prunes_only_reproducible_cargo_state() {
    let root = temp_root("ncc_clean_build");
    let _ = fs::remove_dir_all(&root);
    write_file(&root.join("Cargo.toml"), "[workspace]\n");
    write_file(&root.join("target/debug/deps/ncc.rlib"), "debug-cache");
    write_file(
        &root.join("target/ncup/check/release/ncc.exe"),
        "ncup-cache",
    );
    write_file(&root.join("target/release/deps/ncc.rlib"), "release-cache");
    write_file(&root.join("target/release/ncc.exe"), "release-binary");
    write_file(&root.join("target/evidence/report.json"), "{}\n");
    write_file(&root.join("target/custom-artifact/image.bin"), "image");

    let stdout = assert_success(
        run_ncc(&[
            "clean".to_string(),
            root.to_string_lossy().to_string(),
            "--build".to_string(),
            "--json".to_string(),
        ]),
        "ncc clean build",
    );
    let payload = json_stdout(&stdout);
    assert_eq!(payload["build"], true);
    assert!(payload["removed_bytes"].as_u64().unwrap_or(0) > 0);
    assert!(!root.join("target/debug").exists());
    assert!(!root.join("target/ncup").exists());
    assert!(!root.join("target/release/deps").exists());
    assert!(root.join("target/release/ncc.exe").is_file());
    assert!(root.join("target/evidence/report.json").is_file());
    assert!(root.join("target/custom-artifact/image.bin").is_file());
}

#[test]
fn ncc_doctor_run_reports_source_route_lock_and_cleanable_artifacts() {
    let root = temp_root("ncc_doctor_run");
    let _ = fs::remove_dir_all(&root);
    write_file(&root.join("main.n"), "doctorRun { fa: return true; }\n");
    write_file(
        &root.join(".ncc/run/last.json"),
        r#"{"schema":"ncc.run.route.v1","route":"test"}"#,
    );
    write_file(&root.join(".ncc/lowering/unit.lowering.json"), "{}\n");

    let stdout = assert_success(
        run_ncc(&[
            "doctor".to_string(),
            "run".to_string(),
            root.join("main.n").to_string_lossy().to_string(),
            "--json".to_string(),
        ]),
        "ncc doctor run",
    );
    let payload = json_stdout(&stdout);
    assert_eq!(payload["schema"], "ncc.doctor.run.v1");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["source_resolution"]["status"], "ok");
    assert_eq!(payload["last_run"]["status"], "ok");
    assert_eq!(payload["cache_lock"]["held"], false);
    assert_eq!(
        payload["cleanable_artifacts"]["default"][0]["label"],
        "lowering"
    );
}
