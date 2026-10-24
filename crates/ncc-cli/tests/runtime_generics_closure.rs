use std::path::PathBuf;
use std::process::{Command, Output};
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
    let current = root.join(relative);
    if current.exists() {
        return current;
    }
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive/old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    current
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn write_temp_source(name: &str, src: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{}_{}_{}.n", name, std::process::id(), ts));
    std::fs::write(&path, src).expect("write temp source");
    path
}

fn assert_check_ok(rel: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "check must pass for {rel}\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_run_ok_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "run must pass for {rel}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "run for {rel} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn assert_temp_check_ok(name: &str, src: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "temp check must pass for {name}\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_temp_check_fail_contains(name: &str, src: &str, needle: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "temp check must fail for {name}\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "temp check for {name} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn runtime_list_create_generic_returns_list_int_ok() {
    assert_temp_check_ok(
        "runtime_list_create_generic_returns_list_int_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_create_assignment_to_list_int_ok() {
    runtime_list_create_generic_returns_list_int_ok();
}

#[test]
fn runtime_list_get_returns_element_type_ok() {
    assert_temp_check_ok(
        "runtime_list_get_returns_element_type_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
        x: int;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                listAdd(@xs, 7);
                @x >> int : listGet(@xs, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_set_accepts_element_type_ok() {
    assert_temp_check_ok(
        "runtime_list_set_accepts_element_type_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                listAdd(@xs, 1);
                listSet(@xs, 0, 2);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_set_rejects_wrong_element_type() {
    assert_temp_check_fail_contains(
        "runtime_list_set_rejects_wrong_element_type",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                listSet(@xs, 0, true);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_RUNTIME_GENERIC_ARG_MISMATCH_001",
    );
}

#[test]
fn runtime_list_get_assignment_mismatch_fail() {
    assert_temp_check_fail_contains(
        "runtime_list_get_assignment_mismatch_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
        b: bool;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                listAdd(@xs, 7);
                @b >> bool : listGet(@xs, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "projection type mismatch",
    );
}

#[test]
fn runtime_list_create_without_use_fail() {
    assert_temp_check_fail_contains(
        "runtime_list_create_without_use_fail",
        r#"
entry[@runtime] {
    stc:
        xs: list<int>;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<int>();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_RUNTIME_USE_001",
    );
}

#[test]
fn runtime_list_create_generic_mismatch_fail() {
    assert_temp_check_fail_contains(
        "runtime_list_create_generic_mismatch_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        xs: list<int>;
    fn:
        run() -> bool {
            public:
                @xs >> list<int> : listCreate<bool>();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "projection type mismatch",
    );
}

#[test]
fn runtime_list_no_any_leak_ok() {
    assert_check_ok("examples/stress/stress_runtime_list_pack.n");
}

#[test]
fn runtime_region_create_generic_returns_memregion_int_ok() {
    assert_temp_check_ok(
        "runtime_region_create_generic_returns_memregion_int_ok",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
    );
}

#[test]
fn runtime_region_create_assignment_to_memregion_int_ok() {
    runtime_region_create_generic_returns_memregion_int_ok();
}

#[test]
fn runtime_region_read_returns_element_type_ok() {
    assert_temp_check_ok(
        "runtime_region_read_returns_element_type_ok",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
        x: int;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                regionWrite<int>(@r, 0, 9);
                @x >> int : regionRead<int>(@r, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
    );
}

#[test]
fn runtime_region_write_accepts_element_type_ok() {
    assert_temp_check_ok(
        "runtime_region_write_accepts_element_type_ok",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                regionWrite<int>(@r, 0, 9);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
    );
}

#[test]
fn runtime_region_write_rejects_wrong_element_type() {
    assert_temp_check_fail_contains(
        "runtime_region_write_rejects_wrong_element_type",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                regionWrite<int>(@r, 0, true);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
        "E_RUNTIME_GENERIC_ARG_MISMATCH_001",
    );
}

#[test]
fn runtime_region_read_assignment_mismatch_fail() {
    assert_temp_check_fail_contains(
        "runtime_region_read_assignment_mismatch_fail",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
        b: bool;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                regionWrite<int>(@r, 0, 9);
                @b >> bool : regionRead<int>(@r, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
        "projection type mismatch",
    );
}

#[test]
fn runtime_region_create_without_use_fail() {
    assert_temp_check_fail_contains(
        "runtime_region_create_without_use_fail",
        r#"
entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<int>(4);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
        "E_RUNTIME_USE_001",
    );
}

#[test]
fn runtime_region_create_generic_mismatch_fail() {
    assert_temp_check_fail_contains(
        "runtime_region_create_generic_mismatch_fail",
        r#"
use nC.runtime.region;

entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : regionCreate<bool>(4);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        regionInvalidate(@r);
        return true;
}
"#,
        "projection type mismatch",
    );
}

#[test]
fn runtime_region_no_any_leak_ok() {
    assert_check_ok("examples/stress/stress_runtime_region_pack.n");
}

#[test]
fn runtime_generics_no_any_leak_in_stress_examples() {
    for rel in [
        "examples/stress/stress_runtime_list_pack.n",
        "examples/stress/stress_runtime_region_pack.n",
        "examples/stress/stress_runtime_list_region_industrial_pack.n",
    ] {
        assert_check_ok(rel);
    }
}

#[test]
fn stress_runtime_list_region_industrial_pack_runs_ok() {
    assert_run_ok_contains(
        "examples/stress/stress_runtime_list_pack.n",
        "modified[2]=115",
    );
    assert_run_ok_contains(
        "examples/stress/stress_runtime_region_pack.n",
        "region[3]: 40",
    );
    assert_run_ok_contains(
        "examples/stress/stress_runtime_list_region_industrial_pack.n",
        "region checksum: 4486",
    );
}
