use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
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

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), ts));
    std::fs::create_dir_all(&dir).expect("temp dir must be creatable");
    dir
}

#[test]
fn driver_new_generates_runtime_safe_scaffold_and_executes() {
    let out_dir = unique_temp_dir("ncc_driver_scaffold");

    let create = run_ncc(&[
        "driver",
        "new",
        "telemetry_driver",
        "--out",
        out_dir.to_str().expect("utf8 path"),
    ]);
    assert!(
        create.status.success(),
        "driver new must succeed\nstdout={}\nstderr={}",
        normalize(&create.stdout),
        normalize(&create.stderr)
    );

    let source = out_dir.join("telemetry_driver").join("telemetry_driver.n");
    assert!(source.exists(), "generated source must exist");

    let source_text = std::fs::read_to_string(&source).expect("must read generated source");
    assert!(
        source_text.contains("use nC.runtime.output;")
            && source_text.contains("use nC.runtime.list;")
            && source_text.contains("use nC.runtime.region;")
            && source_text.contains("use nC.runtime.addr;")
            && source_text.contains("use nC.runtime.ptr;"),
        "generated scaffold must contain runtime-safe imports only"
    );
    assert!(
        !source_text.contains("use nC.runtime.inspect;"),
        "generated scaffold must avoid unresolved runtime.inspect import"
    );

    let check = run_ncc(&["check", source.to_str().expect("utf8 path")]);
    assert!(
        check.status.success(),
        "generated scaffold must pass check\nstdout={}\nstderr={}",
        normalize(&check.stdout),
        normalize(&check.stderr)
    );

    let run = run_ncc(&["run", source.to_str().expect("utf8 path")]);
    assert!(
        run.status.success(),
        "generated scaffold must run\nstdout={}\nstderr={}",
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
}

#[test]
fn driver_new_requires_force_to_overwrite_existing_scaffold() {
    let out_dir = unique_temp_dir("ncc_driver_overwrite");

    let first = run_ncc(&[
        "driver",
        "new",
        "overwrite_driver",
        "--out",
        out_dir.to_str().expect("utf8 path"),
    ]);
    assert!(first.status.success(), "initial scaffold must succeed");

    let second = run_ncc(&[
        "driver",
        "new",
        "overwrite_driver",
        "--out",
        out_dir.to_str().expect("utf8 path"),
    ]);
    assert!(
        !second.status.success(),
        "second scaffold without --force must fail"
    );

    let stderr = normalize(&second.stderr);
    assert!(
        stderr.contains("rerun with --force"),
        "expected overwrite diagnostic, got: {}",
        stderr
    );

    let forced = run_ncc(&[
        "driver",
        "new",
        "overwrite_driver",
        "--out",
        out_dir.to_str().expect("utf8 path"),
        "--force",
    ]);
    assert!(
        forced.status.success(),
        "forced overwrite must succeed\nstdout={}\nstderr={}",
        normalize(&forced.stdout),
        normalize(&forced.stderr)
    );
}
