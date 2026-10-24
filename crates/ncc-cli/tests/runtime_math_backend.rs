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

#[test]
fn math_backend_real_intrinsics_run() {
    let example = workspace_root().join("examples/03_types/math_backend_real.n");
    let out = run_ncc(&["run", example.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);

    assert!(
        out.status.success(),
        "math backend run must pass\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stdout.contains("sqrtFloat(81) = 9"));
    assert!(stdout.contains("sinFloat(pi/2)= 1"));
    assert!(stdout.contains("cosFloat(0)   = 1"));
    assert!(stdout.contains("logFloat(e)   = 1"));
    assert!(stdout.contains("aureo digits  = 1.6180339887"));
    assert!(stdout.contains("ok = true"));
}
