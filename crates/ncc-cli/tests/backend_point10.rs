use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
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
fn mct_parity_closes_backend_point10_matrix() {
    let stdout = assert_success(run_ncc(&["mct", "parity", "--json"]), "mct parity");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("json");

    assert_eq!(json["schema"], "nc.backend.point10.parity.v1");
    assert_eq!(json["status"], "backend_point10_closed_for_current_surface");

    let routes = json["routes"].as_array().expect("routes");
    for id in [
        "mct_bytes_contract",
        "llvm_executables",
        "native_fast_critical_paths",
        "machine_low_authority",
        "cross_route_parity_tests",
    ] {
        assert!(
            routes.iter().any(|route| route["id"] == id),
            "missing route {id}: {stdout}"
        );
    }

    let diagnostics = json["diagnostics"].as_array().expect("diagnostics");
    for code in [
        "E_BACKEND_PARITY_MCT_BYTES_001",
        "E_BACKEND_PARITY_LLVM_EXEC_001",
        "E_BACKEND_PARITY_NATIVE_FAST_001",
        "E_BACKEND_PARITY_MACHINE_AUTHORITY_001",
        "E_BACKEND_PARITY_DIVERGENCE_001",
        "E_BACKEND_TOOLCHAIN_MISSING_001",
    ] {
        assert!(
            diagnostics.iter().any(|diag| diag["code"] == code),
            "missing diagnostic {code}: {stdout}"
        );
    }

    assert_eq!(json["truth_boundary"]["mct_bytes_contract"], true);
    assert_eq!(json["truth_boundary"]["llvm_executable_route"], true);
    assert_eq!(json["truth_boundary"]["native_fast_critical_route"], true);
    assert_eq!(json["truth_boundary"]["machine_low_authority_route"], true);
    assert_eq!(json["truth_boundary"]["full_llvm_universe_parity"], false);
    assert_eq!(
        json["truth_boundary"]["missing_tools_are_reported_not_faked"],
        true
    );
}

#[test]
fn mct_parity_list_exposes_ordered_routes() {
    let stdout = assert_success(run_ncc(&["mct", "parity", "--list"]), "mct parity list");
    assert!(stdout.contains("1\tmct_bytes_contract"), "{stdout}");
    assert!(stdout.contains("2\tllvm_executables"), "{stdout}");
    assert!(stdout.contains("3\tnative_fast_critical_paths"), "{stdout}");
    assert!(stdout.contains("4\tmachine_low_authority"), "{stdout}");
    assert!(stdout.contains("5\tcross_route_parity_tests"), "{stdout}");
}
