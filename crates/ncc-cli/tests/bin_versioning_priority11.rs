use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

fn bin_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ncc_bin() -> PathBuf {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_ncc") {
        return PathBuf::from(path);
    }

    let root = workspace_root();
    for rel in ["target/debug/ncc.exe", "target/debug/ncc"] {
        let candidate = root.join(rel);
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "CARGO_BIN_EXE_ncc is not set and target/debug/ncc(.exe) was not found under {}",
        root.display()
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn path_only_dirs(dirs: &[&Path]) -> String {
    std::env::join_paths(dirs.iter().map(|p| p.to_path_buf()))
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn run_ncc_with_path(args: &[&str], path: String) -> Output {
    Command::new(ncc_bin())
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_ncc(args: &[&str]) -> Output {
    let ncc = ncc_bin();
    let bin_dir = ncc.parent().expect("ncc binary has parent");
    run_ncc_with_path(args, path_only_dirs(&[bin_dir]))
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
}

#[test]
fn build_hash_reports_current_executable_hash() {
    let _guard = bin_test_lock();
    let stdout = assert_success(run_ncc(&["--build-hash"]), "build hash");
    let hash = stdout.trim();
    assert!(hash.starts_with("sha256:"), "{stdout}");
    assert_eq!(hash.len(), "sha256:".len() + 64, "{stdout}");

    let stdout = assert_success(run_ncc(&["--build-hash", "--json"]), "build hash json");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "ncc.build_hash.v1");
    assert!(json["current_exe"]
        .as_str()
        .is_some_and(|path| path.contains("ncc")));
    assert!(json["build_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("sha256:")));
}

#[test]
fn doctor_bins_reports_current_binary_and_path_candidates() {
    let _guard = bin_test_lock();
    let stdout = assert_success(run_ncc(&["doctor", "bins", "--json"]), "doctor bins");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "ncc.doctor.bins.v1");
    assert!(json["host_os"].as_str().is_some());
    assert!(json["host_arch"].as_str().is_some());
    assert!(json["host_family"].as_str().is_some());
    assert!(json["path_roots"]
        .as_array()
        .is_some_and(|roots| !roots.is_empty()));
    assert!(json["current_exe"].as_str().is_some());
    assert!(json["current_build_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("sha256:")));
    assert!(json["ncc"]["candidate_names"]
        .as_array()
        .is_some_and(|names| !names.is_empty()));
    assert!(json["ncc"]["candidate_count"].as_u64().unwrap_or(0) >= 1);
    assert_eq!(json["current_in_path_candidates"], true);
    assert_eq!(json["ncc"]["current_in_path_candidates"], true);
    assert_eq!(json["ncc"]["current_not_on_path"], false);
    assert!(json["ncc"]["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .any(|row| row["is_current_exe"] == true
            && row["path_root"].as_str().is_some()
            && row["file_name"].as_str().is_some()));
}

#[test]
fn doctor_bins_detects_duplicate_ncc_on_path() {
    let _guard = bin_test_lock();
    let ncc = ncc_bin();
    let bin_dir = ncc.parent().expect("ncc binary has parent");
    let duplicate_dir = workspace_root().join("target/tmp/priority11-duplicate-bin");
    let _ = fs::remove_dir_all(&duplicate_dir);
    fs::create_dir_all(&duplicate_dir).expect("duplicate dir");
    let duplicate_name = ncc.file_name().expect("ncc filename");
    let duplicate = duplicate_dir.join(duplicate_name);
    fs::copy(&ncc, &duplicate).expect("copy duplicate ncc");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&duplicate).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&duplicate, perms).expect("chmod duplicate");
    }

    let stdout = assert_success(
        run_ncc_with_path(
            &["doctor", "bins", "--json"],
            path_only_dirs(&[&duplicate_dir, bin_dir]),
        ),
        "doctor bins duplicate",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "duplicates_detected");
    assert!(json["ncc"]["duplicate_count"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn doctor_bins_detects_hash_mismatch_ncc_on_path() {
    let _guard = bin_test_lock();
    let ncc = ncc_bin();
    let bin_dir = ncc.parent().expect("ncc binary has parent");
    let mismatch_dir = workspace_root().join("target/tmp/priority11-mismatch-bin");
    let _ = fs::remove_dir_all(&mismatch_dir);
    fs::create_dir_all(&mismatch_dir).expect("mismatch dir");
    let mismatch_name = ncc.file_name().expect("ncc filename");
    let mismatch = mismatch_dir.join(mismatch_name);
    fs::write(&mismatch, b"not the real ncc binary\n").expect("write mismatching ncc");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&mismatch).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mismatch, perms).expect("chmod mismatch");
    }

    let stdout = assert_success(
        run_ncc_with_path(
            &["doctor", "bins", "--json"],
            path_only_dirs(&[&mismatch_dir, bin_dir]),
        ),
        "doctor bins hash mismatch",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "hash_mismatch_detected");
    assert!(json["hash_mismatch_bins"]
        .as_array()
        .is_some_and(|bins| bins.iter().any(|bin| bin.as_str() == Some("ncc"))));
    assert_eq!(json["ncc"]["hash_mismatch"], true);
    assert!(json["ncc"]["distinct_hash_count"].as_u64().unwrap_or(0) >= 2);
}

#[test]
fn doctor_bins_detects_hash_mismatch_when_current_exe_is_not_on_path() {
    let _guard = bin_test_lock();
    let ncc = ncc_bin();
    let mismatch_dir = workspace_root().join("target/tmp/priority11-current-not-path-bin");
    let _ = fs::remove_dir_all(&mismatch_dir);
    fs::create_dir_all(&mismatch_dir).expect("mismatch dir");
    let mismatch_name = ncc.file_name().expect("ncc filename");
    let mismatch = mismatch_dir.join(mismatch_name);
    fs::write(&mismatch, b"stale ncc found through PATH\n").expect("write mismatching ncc");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&mismatch).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mismatch, perms).expect("chmod mismatch");
    }

    let stdout = assert_success(
        run_ncc_with_path(
            &["doctor", "bins", "--json"],
            path_only_dirs(&[&mismatch_dir]),
        ),
        "doctor bins current exe not on PATH",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["status"], "hash_mismatch_detected");
    assert_eq!(json["current_in_path_candidates"], false);
    assert!(json["hash_mismatch_bins"]
        .as_array()
        .is_some_and(|bins| bins.iter().any(|bin| bin.as_str() == Some("ncc"))));
    assert_eq!(json["ncc"]["current_not_on_path"], true);
    assert_eq!(json["ncc"]["hash_mismatch"], true);
    assert!(json["ncc"]["distinct_hash_count"].as_u64().unwrap_or(0) >= 2);
}

#[test]
fn update_all_paths_dry_run_reports_roots_and_weight() {
    let _guard = bin_test_lock();
    let stdout = assert_success(
        run_ncc(&["update", "--all-paths", "--dry-run", "--json"]),
        "update all paths dry run",
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(json["schema"], "ncc.update.all_paths.plan.v1");
    assert_eq!(json["status"], "dry_run");
    assert!(json["root_count"].as_u64().unwrap_or(0) >= 1);
    assert!(json["size_bytes"].as_u64().unwrap_or(0) > 0);
    assert!(!json["path_candidates"]
        .as_array()
        .expect("path candidates")
        .is_empty());
}
