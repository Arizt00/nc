use std::fs;
use std::path::{Path, PathBuf};
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
    let primary = root.join(relative);
    if primary.exists() {
        return primary;
    }

    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive/old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }

    primary
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

fn assert_fixture_exists(path: &Path) {
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
}

fn normalized_unit_name(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn official_run_fixture(fixture: &Path, container: &str) -> PathBuf {
    let stem = fixture
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if normalized_unit_name(stem) == normalized_unit_name(container) {
        return fixture.to_path_buf();
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ncc_program_creation_suite_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).expect("official fixture temp dir must be created");
    let official = dir.join(format!("{container}.n"));
    fs::copy(fixture, &official).expect("archived fixture must copy to official temp file");
    official
}

#[test]
fn program_creation_run_examples_execute_cleanly() {
    let runnable_examples = [
        (
            "examples/program_suite/run_business_profit_report.n",
            "program:run_business_profit_report",
            "businessProfitReport",
        ),
        (
            "examples/program_suite/run_business_inventory_gate.n",
            "program:run_business_inventory_gate",
            "businessInventoryGate",
        ),
        (
            "examples/program_suite/run_business_service_health.n",
            "program:run_business_service_health",
            "businessServiceHealth",
        ),
        (
            "examples/program_suite/run_business_kpi_rollup.n",
            "program:run_business_kpi_rollup",
            "businessKpiRollup",
        ),
        (
            "examples/program_suite/run_business_order_routing.n",
            "program:run_business_order_routing",
            "businessOrderRouting",
        ),
    ];

    for (rel, marker, container) in runnable_examples {
        let fixture = fixture_path(rel);
        assert_fixture_exists(&fixture);
        let run_fixture = official_run_fixture(&fixture, container);

        let out = run_ncc(&["run", run_fixture.to_str().expect("utf8 path")]);
        assert!(
            out.status.success(),
            "run must succeed for {}: status={:?} stderr={}",
            rel,
            out.status,
            normalize(&out.stderr)
        );

        let stdout = normalize(&out.stdout);
        let stderr = normalize(&out.stderr);
        assert!(
            stdout.contains(marker),
            "stdout for {} must contain marker {:?}, got: {:?}",
            rel,
            marker,
            stdout
        );
        assert_eq!(stderr, "", "stderr for {} must stay empty", rel);
    }
}

#[test]
fn program_creation_check_examples_pass() {
    let check_examples = [
        "examples/program_suite/check_spatial_arrays_ok.n",
        "examples/program_suite/check_matrix_tensor_ok.n",
        "examples/program_suite/check_dinarray_binary_ok.n",
        "examples/program_suite/check_dinarray_unbinary_ok.n",
        "examples/program_suite/check_destructure_constructure_ok.n",
        "examples/program_suite/check_visibility_void_null_ok.n",
        "examples/program_suite/check_conditionals_match_guard_unless_ok.n",
    ];

    for rel in check_examples {
        let fixture = fixture_path(rel);
        assert_fixture_exists(&fixture);

        let out = run_ncc(&["check", fixture.to_str().expect("utf8 path")]);
        assert!(
            out.status.success(),
            "check must succeed for {}: status={:?} stderr={} stdout={}",
            rel,
            out.status,
            normalize(&out.stderr),
            normalize(&out.stdout)
        );
    }
}
