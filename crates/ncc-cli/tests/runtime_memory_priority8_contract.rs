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

fn fixture_path(relative: &str) -> String {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return relative.to_string();
    }
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived.to_string_lossy().to_string();
        }
    }
    relative.to_string()
}

fn assert_run_ok(path: &str, marker: &str) {
    let resolved = fixture_path(path);
    let out = run_ncc(&["run", &resolved]);
    let rendered = combined(&out);
    assert!(out.status.success(), "{path}\n{rendered}");
    assert!(rendered.contains(marker), "{path}\n{rendered}");
}

fn assert_ncc_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    let rendered = combined(&out);
    assert!(out.status.success(), "{args:?}\n{rendered}");
    serde_json::from_str(&rendered)
        .unwrap_or_else(|err| panic!("{args:?} must return json: {err}\n{rendered}"))
}

#[test]
fn memory_priority8_examples_run_with_real_runtime_backing() {
    assert_run_ok(
        "examples/runtime/families/memory_buffer_manual_raw.n",
        "manual nC memory buffer backend: true",
    );
    assert_run_ok(
        "examples/runtime/families/memory_views_manual_raw.n",
        "manual nC slice span backend: true",
    );
    assert_run_ok(
        "examples/runtime/families/memory_bytes_manual_raw.n",
        "manual nC memory bytes backend: true",
    );
}

#[test]
fn memory_priority8_imports_are_closed_native_backend_with_mct_symbols() {
    let json = assert_ncc_json(&["imports", "runtime-table", "--json"]);
    let entries = json["entries"].as_array().expect("entries");

    for (surface, symbol, status) in [
        (
            "use::stoud::memory::<buffer>;",
            "nC.runtime.buffer.bufferWrite64",
            "closed_real_backing_bytes",
        ),
        (
            "use::stoud::memory::<alloc>;",
            "nC.runtime.alloc.allocWith",
            "closed_native_backend",
        ),
        (
            "use::stoud::memory::<bytes>;",
            "nC.runtime.bytes.bytesToHex",
            "closed_real_backing_bytes",
        ),
        (
            "use::stoud::memory::<layout>;",
            "nC.runtime.layout.layoutSize",
            "closed_native_backend",
        ),
        (
            "use::stoud::memory::<endian>;",
            "nC.runtime.endian.nativeEndian",
            "closed_native_backend",
        ),
        (
            "use::stoud::memory::<slice>;",
            "nC.runtime.slice.sliceRead8",
            "closed_real_backing_bytes",
        ),
        (
            "use::stoud::memory::<span>;",
            "nC.runtime.span.spanAt",
            "closed_real_backing_bytes",
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing runtime table entry {surface}"));
        assert_eq!(entry["closure_status"], status);
        assert_eq!(entry["can_run_without_runtime_import_failure"], true);
        assert!(entry["runtime_symbols"]
            .as_array()
            .expect("runtime symbols")
            .iter()
            .any(|candidate| candidate == symbol));
    }
}

#[test]
fn memory_priority8_runtime_abi_has_layouts_and_symbols() {
    let json = assert_ncc_json(&["runtime", "abi", "--json"]);
    let layouts = json["layouts"].as_array().expect("layouts");
    for required in [
        "BufferDescriptor",
        "SliceDescriptor",
        "SpanSegment",
        "SpanDescriptor",
        "AllocatorHandle",
        "OwningPtr",
        "MemoryError",
    ] {
        assert!(
            layouts.iter().any(|layout| layout["type"] == required),
            "missing layout {required}"
        );
    }

    let symbols = json["symbols"].as_array().expect("symbols");
    for (symbol, return_ty) in [
        ("nC.runtime.buffer.bufferAt", "bool"),
        ("nC.runtime.buffer.bufferRead64", "u64"),
        ("nC.runtime.slice.sliceRead8", "int"),
        ("nC.runtime.span.spanAt", "bool"),
        (
            "nC.runtime.alloc.allocWith",
            "Result<OwningPtr,MemoryError>",
        ),
        ("nC.runtime.heap.heapAlloc", "Result<ptr<T>,MemoryError>"),
        ("nC.runtime.bytes.byteAt", "int"),
        ("nC.runtime.bytes.hexToBytes", "int"),
    ] {
        assert!(
            symbols
                .iter()
                .any(|row| row["symbol"] == symbol && row["return"]["type"] == return_ty),
            "missing ABI row {symbol} -> {return_ty}"
        );
    }
}
