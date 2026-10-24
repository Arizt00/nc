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

fn fixture_path(rel: &str) -> PathBuf {
    let root = workspace_root();
    let primary = root.join(rel);
    if primary.exists() {
        return primary;
    }
    if let Some(rest) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    primary
}

fn fixture_arg(rel: &str) -> String {
    fixture_path(rel).to_string_lossy().to_string()
}

fn assert_ncc_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    let rendered = combined(&out);
    assert!(out.status.success(), "{args:?}\n{rendered}");
    serde_json::from_str(&rendered)
        .unwrap_or_else(|err| panic!("{args:?} must return json: {err}\n{rendered}"))
}

#[test]
fn priority9_errors_example_runs_and_prints_real_payloads() {
    let fixture = fixture_arg("examples/runtime/runtime_real_errors_priority9.n");
    let out = run_ncc(&["run", &fixture, "--interpreted"]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{rendered}");
    assert!(rendered.contains("priority9-errors-ok=true"), "{rendered}");
    assert!(rendered.contains("fs kind=FsError"), "{rendered}");
    assert!(rendered.contains("tcp kind=TcpError"), "{rendered}");
    assert!(rendered.contains("memory kind=MemoryError"), "{rendered}");
    assert!(rendered.contains("osCause="), "{rendered}");
}

#[test]
fn priority9_error_layouts_are_stable_in_runtime_abi() {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    let layouts = json["layouts"].as_array().expect("layouts");

    for ty in [
        "FsError",
        "TcpError",
        "ProcessError",
        "ThreadError",
        "MemoryError",
    ] {
        let layout = layouts
            .iter()
            .find(|layout| layout["type"] == ty)
            .unwrap_or_else(|| panic!("missing ABI layout for {ty}"));
        assert_eq!(layout["native_lowering"]["size_bytes"], 32);
        assert_eq!(layout["runtime_layout"]["value"]["code"], "int");
        assert_eq!(layout["runtime_layout"]["value"]["message"], "str");
        assert_eq!(layout["runtime_layout"]["value"]["osCause"], "str");
        assert_eq!(layout["runtime_layout"]["value"]["kind"], "str");
    }
}

#[test]
fn priority9_error_symbols_and_typed_results_are_in_abi() {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    let symbols = json["symbols"].as_array().expect("symbols");

    for symbol in [
        "nC.runtime.error.errorCode",
        "nC.runtime.error.errorMessage",
        "nC.runtime.error.errorOsCause",
        "nC.runtime.error.errorKind",
    ] {
        assert!(
            symbols.iter().any(|row| row["symbol"] == symbol),
            "missing error ABI symbol {symbol}"
        );
    }

    for (symbol, return_ty) in [
        ("nC.runtime.fs.fsread", "Result<str,FsError>"),
        ("nC.runtime.fs.listDir", "Result<list<str>,FsError>"),
        ("nC.runtime.fs.readBytesInto", "Result<int,FsError>"),
        ("nC.runtime.fs.writeBytes", "Result<int,FsError>"),
        ("nC.runtime.tcp.tcpConnect", "Result<TcpHandle,TcpError>"),
        ("nC.runtime.tcp.tcpRecv", "Result<str,TcpError>"),
    ] {
        assert!(
            symbols
                .iter()
                .any(|row| row["symbol"] == symbol && row["return"]["type"] == return_ty),
            "missing ABI row {symbol} -> {return_ty}"
        );
    }
}

#[test]
fn priority9_error_import_is_closed_native_backend() {
    let json = assert_ncc_json(&["imports", "runtime-table", "--json"]);
    let entries = json["entries"].as_array().expect("entries");
    let entry = entries
        .iter()
        .find(|entry| entry["surface_import"] == "use::stoud::collections::<error>;")
        .expect("missing collections error runtime table entry");

    assert_eq!(entry["closure_status"], "closed_native_backend");
    assert_eq!(entry["can_run_without_runtime_import_failure"], true);
    assert!(entry["runtime_symbols"]
        .as_array()
        .expect("runtime symbols")
        .iter()
        .any(|symbol| symbol == "nC.runtime.error.errorOsCause"));
}
