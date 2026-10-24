use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let bin_dir = ncc.parent().expect("ncc binary must have parent dir");
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn run_ncc(args: &[&str]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
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

fn assert_route_set(row: &serde_json::Value) {
    let routes = row["backend_routes"].as_array().expect("backend routes");
    for required in ["ncc_run", "mct", "native_fast", "llvm", "machine", "basal"] {
        assert!(
            routes.iter().any(|route| route["route"] == required),
            "missing route {required}: {row}"
        );
    }
}

#[test]
fn runtime_abi_declares_backend_routes_for_every_real_symbol() {
    let stdout = assert_success(run_ncc(&["runtime", "abi", "--json"]), "runtime abi");
    let manifest: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(manifest["schema"], "nc.runtime.symbol_abi.v1");
    assert_eq!(
        manifest["backend_parity_schema"],
        "nc.runtime.backend_parity.v1"
    );

    let symbols = manifest["symbols"].as_array().expect("symbols");
    assert!(!symbols.is_empty(), "runtime ABI must expose symbols");
    for symbol in symbols {
        assert_eq!(
            symbol["backend_parity_status"],
            "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge"
        );
        assert_route_set(symbol);
    }

    let spawn = symbols
        .iter()
        .find(|entry| entry["symbol"] == "nC.runtime.process.spawn")
        .expect("process spawn ABI");
    assert!(spawn["backend_routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["route"] == "llvm"
            && route["status"] == "closed_llvm_runtime_declaration_call"));
}

#[test]
fn import_backend_declares_same_priority10_routes() {
    let stdout = assert_success(
        run_ncc(&["imports", "backend", "--json"]),
        "imports backend",
    );
    let manifest: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let entries = manifest["entries"].as_array().expect("entries");

    for surface in [
        "use::stoud::io::<fsread>;",
        "use::stoud::io::<fswrite>;",
        "use::stoud::net::<tcp>;",
        "use::auth::system::<process>;",
        "use::auth::system::<thread>;",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing import backend entry {surface}"));
        assert_eq!(
            entry["backend_parity_status"],
            "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge"
        );
        assert_route_set(entry);
        assert!(entry["backend_routes"]
            .as_array()
            .expect("routes")
            .iter()
            .any(|route| route["route"] == "mct"
                && route["status"] == "closed_runtime_symbol_activation_bytes"));
    }
}

#[test]
fn entropy_import_has_closed_backend_routes() {
    let stdout = assert_success(
        run_ncc(&["imports", "backend", "--json"]),
        "imports backend",
    );
    let manifest: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let entries = manifest["entries"].as_array().expect("entries");
    let entropy = entries
        .iter()
        .find(|entry| entry["surface_import"] == "use::stoud::io::<entropy>;")
        .expect("entropy entry");

    assert_eq!(
        entropy["backend_parity_status"],
        "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge"
    );
    assert_route_set(entropy);
    assert!(entropy["backend_routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["route"] == "llvm"
            && route["status"] == "closed_llvm_runtime_declaration_call"));
}

#[test]
fn priority10_example_runs_without_real_flag() {
    let stdout = assert_success(
        run_ncc(&["run", "examples/07_runtime/backend_mct_real_bytes.n"]),
        "priority10 example",
    );
    assert!(
        stdout.contains("Backend/MCT closed_real_backing_bytes"),
        "{stdout}"
    );
    assert!(stdout.contains("process-exit=0"), "{stdout}");
    assert!(stdout.contains("ok=true"), "{stdout}");
}
