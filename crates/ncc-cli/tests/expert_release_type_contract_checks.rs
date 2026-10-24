use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    direct
}

fn assert_check_ok(rel: &str) {
    let path = fixture_path(rel);
    let out = Command::new(ncc_bin())
        .arg("check")
        .arg(path.to_str().expect("utf8 path"))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc check must execute");
    assert!(
        out.status.success(),
        "check must pass for {}\nstatus={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_run_ok(rel: &str) {
    let path = fixture_path(rel);
    let out = Command::new(ncc_bin())
        .arg("run")
        .arg(path.to_str().expect("utf8 path"))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc run must execute");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "run must pass for {}\nstatus={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        stdout,
        stderr
    );
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        !combined.contains("section not allowed outside [@primitive] context"),
        "run output for {} must not regress to primitive-section parser diagnostic\n{}",
        rel,
        combined
    );
}

fn assert_temp_run_fail_contains(prefix: &str, src: &str, needle: &str) {
    let path = unique_temp_nc_path(prefix);
    std::fs::write(&path, src).expect("temp source should write");

    let out = Command::new(ncc_bin())
        .arg("run")
        .arg(path.to_str().expect("utf8 path"))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc run must execute");
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        !out.status.success(),
        "run must fail for {}\n{}",
        prefix,
        combined
    );
    assert!(
        combined.contains(needle),
        "run output must contain {} for {}\n{}",
        needle,
        prefix,
        combined
    );
}

fn unique_temp_nc_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    path.push(format!("{}_{}.n", prefix, nanos));
    path
}

#[test]
fn expert_release_type_01_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/01_type_contract_release.n");
}

#[test]
fn expert_release_type_02_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/02_type_contract_release.n");
}

#[test]
fn expert_release_type_03_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/03_type_contract_release.n");
}

#[test]
fn expert_release_type_04_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/04_type_contract_release.n");
}

#[test]
fn expert_release_type_05_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/05_type_contract_release.n");
}

#[test]
fn expert_release_type_06_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/06_type_contract_release.n");
}

#[test]
fn expert_release_type_07_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/07_type_contract_release.n");
}

#[test]
fn expert_release_type_08_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/08_type_contract_release.n");
}

#[test]
fn expert_release_type_09_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/09_type_contract_release.n");
}

#[test]
fn expert_release_type_10_check_ok() {
    assert_check_ok("examples/expert_release_v2/types/10_type_contract_release.n");
}

#[test]
fn expert_release_type_01_run_ok() {
    assert_run_ok("examples/expert_release_v2/types/01_type_contract_release.n");
}

#[test]
fn expert_release_type_04_run_ok() {
    assert_run_ok("examples/expert_release_v2/types/04_type_contract_release.n");
}

#[test]
fn expert_release_type_all_runs_ok() {
    let fixtures = [
        "examples/expert_release_v2/types/01_type_contract_release.n",
        "examples/expert_release_v2/types/02_type_contract_release.n",
        "examples/expert_release_v2/types/03_type_contract_release.n",
        "examples/expert_release_v2/types/04_type_contract_release.n",
        "examples/expert_release_v2/types/05_type_contract_release.n",
        "examples/expert_release_v2/types/06_type_contract_release.n",
        "examples/expert_release_v2/types/07_type_contract_release.n",
        "examples/expert_release_v2/types/08_type_contract_release.n",
        "examples/expert_release_v2/types/09_type_contract_release.n",
        "examples/expert_release_v2/types/10_type_contract_release.n",
    ];
    for fixture in fixtures {
        assert_run_ok(fixture);
    }
}

#[test]
fn expert_release_type_runtime_executes_nominal_stmt() {
    let src = r#"
typeRuntimeDeferred[@] {
    fn:
        run() -> bool {
            public:
                destructure.static([1, 2, 3]);
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
    let path = unique_temp_nc_path("nc_type_runtime_deferred");
    std::fs::write(&path, src).expect("temp source should write");

    let out = Command::new(ncc_bin())
        .arg("run")
        .arg(path.to_str().expect("utf8 path"))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc run must execute");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        out.status.success(),
        "run must execute nominal type-contract statement paths without deferred fallback\n{}",
        combined
    );
    assert!(
        !combined.contains("E_TYPE_CONTRACT_DEFERRED_001"),
        "nominal statement execution should not emit deferred runtime diagnostic:\n{}",
        combined
    );
    assert!(
        !combined.contains("section not allowed outside [@primitive] context"),
        "must not regress to primitive parser-section error:\n{}",
        combined
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn expert_release_type_runtime_bounds_guard_fails() {
    let src = r#"
typeBoundsGuardFail[@] {
    type:
        Tx {
            public:
                amount: int;
            bounds:
                amount: range(1..10);
        }

    stc:
        t: Tx;

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
    assert_temp_run_fail_contains("nc_type_bounds_guard_fail", src, "E_TYPE_BOUNDS_RANGE_001");
}

#[test]
fn expert_release_type_runtime_invariant_guard_fails() {
    let src = r#"
typeInvariantGuardFail[@] {
    type:
        Acc {
            public:
                balance: int;
            invariant:
                @balance >= 1;
        }

    stc:
        a: Acc;

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
    assert_temp_run_fail_contains(
        "nc_type_invariant_guard_fail",
        src,
        "E_TYPE_INVARIANT_RUNTIME_FAIL_001",
    );
}

#[test]
fn expert_release_type_runtime_nullability_guard_fails() {
    let src = r#"
typeNullabilityGuardFail[@] {
    type:
        Node {
            public:
                next: ptr<int> | null;
            nullability:
                next: nonnull;
        }

    stc:
        n: Node;

    fn:
        run() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
    assert_temp_run_fail_contains(
        "nc_type_nullability_guard_fail",
        src,
        "E_TYPE_NULLABILITY_001",
    );
}
