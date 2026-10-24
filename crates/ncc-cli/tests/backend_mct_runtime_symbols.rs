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

fn assert_runtime_symbol(manifest: &serde_json::Value, surface: &str, symbol: &str) {
    let entries = manifest["entries"].as_array().expect("entries");
    let entry = entries
        .iter()
        .find(|entry| entry["surface_import"] == surface)
        .unwrap_or_else(|| panic!("missing backend entry {surface}"));

    assert_eq!(entry["status"], "closed_real_backing_bytes", "{entry}");
    assert_eq!(entry["backing_status"], "closed_real_backing_bytes");
    assert_eq!(entry["closed_real_backing_bytes"], true);
    assert_eq!(
        entry["runtime_call_lowering"],
        "direct_runtime_symbol_call_without_real_flag"
    );
    assert!(entry["emitted_bytes"].as_array().expect("bytes").len() >= 4);
    assert!(
        entry["runtime_symbols"]
            .as_array()
            .expect("runtime symbols")
            .iter()
            .any(|candidate| candidate == symbol),
        "{surface} must expose {symbol}: {entry}"
    );
}

#[test]
fn imports_backend_registers_runtime_symbols_as_closed_real_backing_bytes() {
    let stdout = assert_success(
        run_ncc(&["imports", "backend", "--json"]),
        "imports backend",
    );
    let manifest: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(manifest["schema"], "nc.imports.backend_lowering.v1");
    assert_eq!(
        manifest["mct_connection"],
        "connected_to_mct_import_activation_and_runtime_symbols"
    );

    for (surface, symbol) in [
        ("use::stoud::io::<fsread>;", "nC.runtime.fs.fsread"),
        ("use::stoud::io::<fswrite>;", "nC.runtime.fs.fswrite"),
        ("use::stoud::io::<fsappend>;", "nC.runtime.fs.fsappend"),
        ("use::stoud::io::<fsync>;", "nC.runtime.fs.fsync"),
        ("use::stoud::net::<tcp>;", "nC.runtime.tcp.tcpRecv"),
        ("use::auth::system::<process>;", "nC.runtime.process.spawn"),
        (
            "use::auth::system::<thread>;",
            "nC.runtime.thread.threadJoin",
        ),
        ("use::stoud::time::<sleep>;", "nC.runtime.time.sleep"),
        (
            "use::stoud::network::<server_open>;",
            "nC.runtime.mobileNetwork.server_open",
        ),
        (
            "use::stoud::network::<isWiFiConnected>;",
            "nC.runtime.mobileNetwork.isWiFiConnected",
        ),
        (
            "use::stoud::io::<delete_file>;",
            "nC.runtime.fs.delete_file",
        ),
        (
            "use::stoud::io::<rename_file>;",
            "nC.runtime.fs.rename_file",
        ),
        ("use::stoud::sys::<cpu_load>;", "nC.runtime.system.cpu_load"),
        (
            "use::stoud::sys::<registry_write>;",
            "nC.runtime.system.registry_write",
        ),
        (
            "use::stoud::sys::<registry_read>;",
            "nC.runtime.system.registry_read",
        ),
    ] {
        assert_runtime_symbol(&manifest, surface, symbol);
    }
}

#[test]
fn mct_imports_exposes_same_runtime_symbol_contract() {
    let stdout = assert_success(run_ncc(&["mct", "imports", "--json"]), "mct imports");
    let manifest: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(manifest["schema"], "nc.imports.backend_lowering.v1");
    assert_runtime_symbol(
        &manifest,
        "use::auth::system::<process>;",
        "nC.runtime.process.wait",
    );
    assert_runtime_symbol(
        &manifest,
        "use::auth::system::<thread>;",
        "nC.runtime.thread.threadSpawn",
    );
}

#[test]
fn entropy_import_has_real_runtime_symbols() {
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

    assert_eq!(entropy["status"], "closed_real_backing_bytes");
    assert_eq!(entropy["closed_real_backing_bytes"], true);
    assert_eq!(
        entropy["runtime_call_lowering"],
        "direct_runtime_symbol_call_without_real_flag"
    );
    let symbols = entropy["runtime_symbols"]
        .as_array()
        .expect("runtime symbols");
    assert!(symbols
        .iter()
        .any(|symbol| symbol == "nC.runtime.entropy.entropyBytes"));
    assert!(symbols
        .iter()
        .any(|symbol| symbol == "nC.runtime.entropy.entropyAvailable"));
    assert!(entropy["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .is_empty());
}

#[test]
fn ncc_run_uses_closed_runtime_apis_without_real_flag() {
    let _ = std::fs::remove_file(workspace_root().join("target/tmp/backend_mct_thread.txt"));
    let stdout = assert_success(
        run_ncc(&["run", "examples/07_runtime/backend_mct_real_bytes.n"]),
        "backend mct runtime example",
    );
    assert!(
        stdout.contains("Backend/MCT closed_real_backing_bytes"),
        "{stdout}"
    );
    assert!(stdout.contains("ok=true"), "{stdout}");
}
