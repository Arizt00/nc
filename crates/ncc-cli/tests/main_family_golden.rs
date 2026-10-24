use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn fixture_path(name: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join("examples").join("main_family_v2_0_0").join(name);
    if direct.exists() {
        return direct;
    }
    let archived = root
        .join("_archive")
        .join("old_examples")
        .join("main_family_v2_0_0")
        .join(name);
    if archived.exists() {
        return archived;
    }
    direct
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn main_family_ref_int_ok_runs_and_shows_return() {
    let fixture = fixture_path("main_ref_int_ok.n");
    let out = run_ncc(&[
        "run",
        fixture.to_str().expect("utf8 path"),
        "--interpreted",
        "--show-return",
    ]);
    assert!(
        out.status.success(),
        "run must succeed, status={:?}",
        out.status
    );

    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        stdout.contains("[return: 7 profile=") && stdout.contains("route="),
        "unexpected stdout: {:?}",
        stdout
    );
    assert_eq!(stderr, "");
}

#[test]
fn main_family_skk_alias_executes_successfully() {
    let fixture = fixture_path("main_skk_alias_ok.n");
    let out = run_ncc(&["run", fixture.to_str().expect("utf8 path"), "--interpreted"]);
    assert!(
        out.status.success(),
        "run must succeed, status={:?}",
        out.status
    );
    assert_eq!(normalize(&out.stderr), "");
}

#[test]
fn main_family_ambiguous_profile_without_selection_fails_officially() {
    let fixture = fixture_path("main_profile_ambiguous.n");
    let out = run_ncc(&["run", fixture.to_str().expect("utf8 path")]);
    assert!(
        !out.status.success(),
        "run must fail for ambiguous profile selection"
    );

    let stderr = normalize(&out.stderr);
    assert!(
        stderr.contains("multiple profiled main entries found; select a profile explicitly"),
        "unexpected stderr: {:?}",
        stderr
    );
}

#[test]
fn main_family_requested_profile_missing_fails_officially() {
    let fixture = fixture_path("main_profile_missing_selected.n");
    let out = run_ncc(&[
        "run",
        fixture.to_str().expect("utf8 path"),
        "--main-profile",
        "fast",
    ]);
    assert!(
        !out.status.success(),
        "run must fail when requested profile is missing"
    );

    let stderr = normalize(&out.stderr);
    assert!(
        stderr.contains("requested profile 'fast' has no valid main entry"),
        "unexpected stderr: {:?}",
        stderr
    );
}

#[test]
fn main_family_invalid_main_return_contract_fails_officially() {
    let fixture = fixture_path("main_return_invalid_string.n");
    let out = run_ncc(&["run", fixture.to_str().expect("utf8 path")]);
    assert!(
        !out.status.success(),
        "run must fail for invalid main return contract"
    );

    let stderr = normalize(&out.stderr);
    assert!(
        stderr.contains("entry target has invalid return contract for to main"),
        "unexpected stderr: {:?}",
        stderr
    );
}
