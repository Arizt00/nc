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

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    let archived = root.join("_archive").join("old_examples").join(
        relative
            .strip_prefix("examples/")
            .or_else(|| relative.strip_prefix("examples\\"))
            .unwrap_or(relative),
    );
    if archived.exists() {
        return archived;
    }
    direct
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
fn industrial_line_band_examples_check_and_run() {
    let fixtures = [
        "examples/industrial/line_bands/max50/driver_mem_compact_max50.n",
        "examples/industrial/line_bands/gt100/driver_mem_extended_gt100.n",
        "examples/industrial/line_bands/gt300/driver_mem_extreme_gt300.n",
    ];

    for rel in fixtures {
        let path = fixture_path(rel);
        assert!(
            path.exists(),
            "fixture must exist: {}",
            path.to_string_lossy()
        );

        let check = run_ncc(&["check", path.to_str().expect("utf8 path")]);
        assert!(
            check.status.success(),
            "check must pass for {}\nstdout={}\nstderr={}",
            rel,
            normalize(&check.stdout),
            normalize(&check.stderr)
        );

        let run = run_ncc(&["run", path.to_str().expect("utf8 path")]);
        assert!(
            run.status.success(),
            "run must pass for {}\nstdout={}\nstderr={}",
            rel,
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
    }
}
