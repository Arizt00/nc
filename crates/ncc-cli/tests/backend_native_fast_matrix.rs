use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn run_ncc(args: &[String]) -> Output {
    let ncc = ncc_bin();
    let cwd = unique_temp_dir("backend-native-fast-matrix-cwd");
    let output = Command::new(&ncc)
        .current_dir(&cwd)
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute");
    let _ = std::fs::remove_dir_all(cwd);
    output
}

fn run_ncc_lit(args: &[&str]) -> Output {
    let owned = args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    run_ncc(&owned)
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn combined(out: &Output) -> String {
    format!("{}{}", normalize(&out.stdout), normalize(&out.stderr))
}

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
}

fn assert_success_stdout(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    normalize(&out.stdout)
}

fn assert_failure(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(
        !out.status.success(),
        "{label} unexpectedly passed\n{rendered}"
    );
    rendered
}

fn json_stdout(out: Output, label: &str) -> serde_json::Value {
    let stdout = assert_success_stdout(out, label);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|err| panic!("{label} must emit JSON: {err}\n{stdout}"))
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ncc-{label}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir must create");
    dir
}

#[cfg(unix)]
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn assert_fixture_exists(rel: &str) {
    assert!(
        PathBuf::from(example_arg(rel)).exists(),
        "matrix fixture must exist: {rel}"
    );
}

fn materialize_profiled_fixture(temp: &Path, rel: &str, profile: &str) -> PathBuf {
    let source_path = PathBuf::from(example_arg(rel));
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("fixture must be readable {}: {err}", source_path.display()));
    let stem = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("fixture stem must be UTF-8");
    let package = stem
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let destination = temp.join(
        source_path
            .file_name()
            .expect("fixture must have a file name"),
    );
    std::fs::write(
        &destination,
        format!("package {package}_{profile} {{ profile: {profile}; }}\n\n{source}"),
    )
    .unwrap_or_else(|err| panic!("profiled fixture must be writable: {err}"));
    destination
}

fn example_arg(rel: &str) -> String {
    let root = workspace_root();
    if root.join(rel).exists() {
        return root.join(rel).to_string_lossy().to_string();
    }
    for prefix in [
        "examples/backend/",
        "examples/performance/",
        "examples/stress/",
        "examples/high_level/",
    ] {
        if let Some(suffix) = rel.strip_prefix(prefix) {
            let archived = root
                .join("_archive/old_examples")
                .join(prefix.trim_start_matches("examples/"))
                .join(suffix);
            if archived.exists() {
                return archived.to_string_lossy().to_string();
            }
        }
    }
    root.join(rel).to_string_lossy().to_string()
}

#[test]
fn backend_matrix_declares_owned_fixture_rows() {
    for fixture in [
        "examples/backend/mct_llvm_array_executable.n",
        "examples/backend/mct_llvm_struct_layout.n",
        "examples/backend/mct_llvm_vector_struct_access.n",
        "examples/backend/mct_llvm_dynamic_array_slice.n",
        "examples/backend/mct_llvm_print_bytes.n",
        "examples/backend/mct_llvm_dev_trap.n",
        "examples/backend/parity_dynamic_list_push.n",
        "examples/backend/parity_dinarray_struct_push_growth.n",
        "examples/backend/negative/mct_llvm_dynamic_oob_fail.n",
        "examples/backend/negative/mct_llvm_dynamic_list_oob_fail.n",
        "examples/backend/negative/mct_llvm_dinarray_invalid_capacity_fail.n",
        "examples/performance/native_fast/memory_ptr_read_write_widths.n",
        "examples/stress/stress_runtime_list_pack.n",
    ] {
        assert_fixture_exists(fixture);
    }
}

#[cfg(unix)]
#[test]
fn llvm_mct_executes_backend_surface_matrix() {
    if !tool_available("clang") {
        return;
    }
    let temp = unique_temp_dir("llvm-mct-backend-matrix");

    for (id, fixture, expected_stdout) in [
        (
            "main_globals_array",
            "examples/backend/mct_llvm_array_executable.n",
            "",
        ),
        ("structs", "examples/backend/mct_llvm_struct_layout.n", ""),
        (
            "vector_struct",
            "examples/backend/mct_llvm_vector_struct_access.n",
            "",
        ),
        (
            "pointers_regions",
            "examples/backend/mct_llvm_dynamic_array_slice.n",
            "",
        ),
        (
            "print_strings",
            "examples/backend/mct_llvm_print_bytes.n",
            "A WORD\nvalue=42\n\n",
        ),
    ] {
        let out_dir = temp.join(id);
        let out_dir_arg = out_dir.to_string_lossy().to_string();
        let fixture_arg = example_arg(fixture);
        let json = json_stdout(
            run_ncc(&[
                "mct".into(),
                "llvm-exe-smoke".into(),
                fixture_arg,
                "--out-dir".into(),
                out_dir_arg,
                "--json".into(),
                "--cc".into(),
                "nC.sysv64".into(),
            ]),
            &format!("llvm-exe-smoke {id}"),
        );

        assert_eq!(json["schema"], "nc.mct.llvm.exe_smoke.v1", "{json:#?}");
        assert_eq!(json["status"], "llvm_exe_smoke_passed", "{json:#?}");
        assert_eq!(json["run"]["status_code"], 0, "{json:#?}");
        assert_eq!(json["run"]["stderr"], "", "{json:#?}");
        assert_eq!(json["run"]["stdout"], expected_stdout, "{json:#?}");
        assert_eq!(
            json["module"]["status"], "llvm_adapter_emitted",
            "{json:#?}"
        );
        assert_eq!(json["module"]["target_triple"], "x86_64-unknown-linux-gnu");
        assert!(
            json["module"]["emitted_function_count"]
                .as_u64()
                .is_some_and(|count| count > 0),
            "{json:#?}"
        );

        let rendered = serde_json::to_string(&json).expect("json must render");
        let llvm_ir = std::fs::read_to_string(
            json["ll_path"]
                .as_str()
                .expect("llvm smoke must report ll_path"),
        )
        .expect("llvm smoke must write ll_path");
        if id == "main_globals_array" {
            assert!(llvm_ir.contains("define i32 @main()"), "{llvm_ir}");
            assert!(llvm_ir.contains("@__ncc_stc_"), "{llvm_ir}");
            assert!(
                rendered.contains("\"construct\":\"stc.symbol\""),
                "{rendered}"
            );
        }
        if id == "structs" || id == "vector_struct" {
            assert!(rendered.contains("Packet"), "{rendered}");
            assert!(llvm_ir.contains("getelementptr"), "{llvm_ir}");
        }
    }

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn native_fast_matches_llvm_mct_for_dynamic_container_parity() {
    if !tool_available("clang") || !tool_available("cc") {
        return;
    }
    let temp = unique_temp_dir("native-fast-dynamic-container-parity");

    for (id, fixture, expected_stdout) in [
        (
            "list_push_growth",
            "examples/backend/parity_dynamic_list_push.n",
            "parity.list idx=28 get=63 ok=true",
        ),
        (
            "dinarray_struct_push_growth",
            "examples/backend/parity_dinarray_struct_push_growth.n",
            "parity.dinstruct idx=60 get=110 total=1010 ok=true",
        ),
    ] {
        let out_dir_arg = temp.join(id).to_string_lossy().to_string();
        let fixture_arg = example_arg(fixture);
        let llvm = json_stdout(
            run_ncc(&[
                "mct".into(),
                "llvm-exe-smoke".into(),
                fixture_arg.clone(),
                "--out-dir".into(),
                out_dir_arg,
                "--json".into(),
                "--cc".into(),
                "nC.sysv64".into(),
            ]),
            &format!("llvm dynamic parity {id}"),
        );
        assert_eq!(llvm["status"], "llvm_exe_smoke_passed", "{llvm:#?}");
        assert_eq!(llvm["run"]["status_code"], 0, "{llvm:#?}");
        assert!(
            llvm["run"]["stdout"]
                .as_str()
                .is_some_and(|stdout| stdout.contains(expected_stdout)),
            "{llvm:#?}"
        );

        let native = assert_success(
            run_ncc(&[
                "run".into(),
                fixture_arg,
                "--native-fast".into(),
                "--show-return".into(),
            ]),
            &format!("native-fast dynamic parity {id}"),
        );
        assert!(native.contains(expected_stdout), "{native}");
        assert!(
            native.contains("[return: true profile=release-fast route=native-fast-path]"),
            "{native}"
        );
    }

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn native_fast_matches_llvm_mct_for_static_backend_subset() {
    if !tool_available("clang") || !tool_available("cc") {
        return;
    }
    let temp = unique_temp_dir("native-fast-llvm-parity");

    for (id, fixture) in [
        ("array", "examples/backend/mct_llvm_array_executable.n"),
        ("struct", "examples/backend/mct_llvm_struct_layout.n"),
        (
            "vector_struct",
            "examples/backend/mct_llvm_vector_struct_access.n",
        ),
    ] {
        let out_dir_arg = temp.join(id).to_string_lossy().to_string();
        let fixture_arg = example_arg(fixture);
        let llvm = json_stdout(
            run_ncc(&[
                "mct".into(),
                "llvm-exe-smoke".into(),
                fixture_arg.clone(),
                "--out-dir".into(),
                out_dir_arg,
                "--json".into(),
                "--cc".into(),
                "nC.sysv64".into(),
            ]),
            &format!("llvm parity {id}"),
        );
        assert_eq!(llvm["status"], "llvm_exe_smoke_passed", "{llvm:#?}");
        assert_eq!(llvm["run"]["status_code"], 0, "{llvm:#?}");

        let native = assert_success(
            run_ncc(&[
                "run".into(),
                fixture_arg,
                "--native-fast".into(),
                "--show-return".into(),
            ]),
            &format!("native-fast parity {id}"),
        );
        assert!(
            native.contains("[return: true profile=release-fast route=native-fast-path]"),
            "{native}"
        );
    }

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn native_fast_executes_global_dynamic_stc_list_pack() {
    if !tool_available("cc") {
        return;
    }

    let list = assert_success(
        run_ncc(&[
            "run".into(),
            example_arg("examples/stress/stress_runtime_list_pack.n"),
            "--real".into(),
            "--show-return".into(),
        ]),
        "list<T> global dynamic stc native-fast boundary",
    );
    assert!(list.contains("stress_runtime_list_pack"), "{list}");
    assert!(list.contains("sum =130 len = 4 cap = 4"), "{list}");
    assert!(list.contains("checksum = 429"), "{list}");
    assert!(list.contains("ok = true"), "{list}");
    assert!(list.contains("route=native-fast-path"), "{list}");
}

#[test]
fn backend_trap_lowers_to_llvm_and_native_trap_ops() {
    assert_success(
        run_ncc(&[
            "check".into(),
            example_arg("examples/backend/mct_llvm_dev_trap.n"),
        ]),
        "dev trap fixture check",
    );

    let llvm = assert_success_stdout(
        run_ncc(&[
            "mct".into(),
            "emit-llvm".into(),
            example_arg("examples/backend/mct_llvm_dev_trap.n"),
            "--json".into(),
            "--cc".into(),
            "nC.sysv64".into(),
        ]),
        "dev trap mct emit-llvm",
    );
    assert!(llvm.contains("\"construct\": \"expr.dev_trap\""), "{llvm}");
    assert!(llvm.contains("declare void @llvm.trap()"), "{llvm}");
    assert!(llvm.contains("call void @llvm.trap()"), "{llvm}");

    let backend = assert_success(
        run_ncc(&[
            "emit-object".into(),
            example_arg("examples/backend/mct_llvm_dev_trap.n"),
            "--cc".into(),
            "nC.sysv64".into(),
        ]),
        "dev trap native backend text",
    );
    assert!(backend.contains("ud2 ; scalar-trap"), "{backend}");
}

#[test]
fn backend_raw_unsafe_dynamic_containers_emit_without_assistance() {
    for fixture in [
        "examples/backend/negative/mct_llvm_dynamic_oob_fail.n",
        "examples/backend/negative/mct_llvm_dynamic_list_oob_fail.n",
        "examples/backend/negative/mct_llvm_dinarray_invalid_capacity_fail.n",
    ] {
        let rendered = assert_success_stdout(
            run_ncc(&[
                "mct".into(),
                "emit-llvm".into(),
                example_arg(fixture),
                "--json".into(),
                "--cc".into(),
                "nC.sysv64".into(),
            ]),
            &format!("raw unsafe mct emit-llvm {fixture}"),
        );
        assert!(
            !rendered.contains("W_MCT_LLVM_DYNAMIC_BOUNDS_UNSAFE_001"),
            "{rendered}"
        );
        assert!(
            !rendered.contains("W_MCT_LLVM_DINARRAY_CAPACITY_UNSAFE_001"),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"status\": \"llvm_adapter_emitted\""),
            "{rendered}"
        );
    }
}

#[test]
fn backend_raw_dynamic_container_access_has_no_implicit_checks() {
    let temp = unique_temp_dir("raw-dynamic-container-policy");

    for fixture in [
        "examples/backend/negative/mct_llvm_dynamic_oob_fail.n",
        "examples/backend/negative/mct_llvm_dynamic_list_oob_fail.n",
    ] {
        let source = materialize_profiled_fixture(&temp, fixture, "raw");
        let rendered = assert_success_stdout(
            run_ncc(&[
                "mct".into(),
                "emit-llvm".into(),
                source.to_string_lossy().to_string(),
                "--json".into(),
                "--cc".into(),
                "nC.sysv64".into(),
            ]),
            &format!("raw unchecked mct emit-llvm {fixture}"),
        );
        assert!(
            !rendered.contains("E_MCT_LLVM_DYNAMIC_BOUNDS_001"),
            "{rendered}"
        );
        assert!(rendered.contains("llvm_adapter_emitted"), "{rendered}");
    }

    let capacity_fixture = "examples/backend/negative/mct_llvm_dinarray_invalid_capacity_fail.n";
    let capacity_source = materialize_profiled_fixture(&temp, capacity_fixture, "raw");
    let rendered = assert_success_stdout(
        run_ncc(&[
            "mct".into(),
            "emit-llvm".into(),
            capacity_source.to_string_lossy().to_string(),
            "--json".into(),
            "--cc".into(),
            "nC.sysv64".into(),
        ]),
        "raw unchecked dinArray capacity",
    );
    assert!(
        !rendered.contains("W_MCT_LLVM_DINARRAY_CAPACITY_UNSAFE_001"),
        "{rendered}"
    );
    assert!(
        rendered.contains("\"status\": \"llvm_adapter_emitted\""),
        "{rendered}"
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn runtime_imports_keep_mct_native_fast_llvm_routes_declared() {
    let manifest = json_stdout(
        run_ncc_lit(&["imports", "backend", "--json"]),
        "imports backend matrix",
    );
    assert_eq!(manifest["schema"], "nc.imports.backend_lowering.v1");
    assert_eq!(
        manifest["mct_connection"],
        "connected_to_mct_import_activation_and_runtime_symbols"
    );
    let entries = manifest["entries"].as_array().expect("entries");

    for (surface, symbol) in [
        ("use::stoud::io::<fsread>;", "nC.runtime.fs.fsread"),
        ("use::stoud::io::<fswrite>;", "nC.runtime.fs.fswrite"),
        ("use::stoud::net::<tcp>;", "nC.runtime.tcp.tcpRecv"),
        ("use::auth::system::<process>;", "nC.runtime.process.spawn"),
        (
            "use::auth::system::<thread>;",
            "nC.runtime.thread.threadJoin",
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["surface_import"] == surface)
            .unwrap_or_else(|| panic!("missing backend import row {surface}"));
        assert_eq!(
            entry["backend_parity_status"],
            "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge"
        );
        let routes = entry["backend_routes"].as_array().expect("backend routes");
        for route in ["mct", "native_fast", "llvm", "machine", "basal"] {
            assert!(
                routes.iter().any(|candidate| candidate["route"] == route),
                "missing route {route}: {entry}"
            );
        }
        assert!(
            entry["runtime_symbols"]
                .as_array()
                .expect("runtime symbols")
                .iter()
                .any(|candidate| candidate == symbol),
            "missing runtime symbol {symbol}: {entry}"
        );
    }
}

#[test]
fn closure_boundary_is_explicit_for_backend_matrix() {
    assert_success(
        run_ncc(&[
            "check".into(),
            example_arg("examples/high_level/closures/05_closure_capture_value.n"),
        ]),
        "closures check value check",
    );
    let rejected = assert_failure(
        run_ncc(&[
            "check".into(),
            example_arg("examples/high_level/phase8/negative/11_closure_lambda_deferred_fail.n"),
        ]),
        "phase8 closure deferred negative",
    );
    assert!(
        rejected.contains("E_HIGH_OBSOLETE_CALLABLE_SPELLING_001"),
        "{rejected}"
    );
}
