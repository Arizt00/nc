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
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
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

fn assert_check_ok(rel: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "check must pass for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_run_ok(rel: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "run must pass for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

#[test]
fn expert_release_examples_check_ok() {
    let fixtures = [
        "examples/expert_release/ffi/01_ffi_c_contract_runpath.n",
        "examples/expert_release/ffi/02_ffi_cpp_contract_runpath.n",
        "examples/expert_release/ffi/03_ffi_rust_contract_runpath.n",
        "examples/expert_release/ffi/04_ffi_bundle_contracts.n",
        "examples/expert_release/ffi/05_ffi_asm_bridge_policy.n",
        "examples/expert_release/ffi/06_ffi_release_gate.n",
        "examples/expert_release/bounds/07_bounds_array_static.n",
        "examples/expert_release/bounds/08_bounds_array_dynamic_guard.n",
        "examples/expert_release/bounds/09_bounds_list_dynamic.n",
        "examples/expert_release/bounds/10_bounds_memregion_readwrite.n",
        "examples/expert_release/bounds/11_bounds_matrix_surface.n",
        "examples/expert_release/bounds/12_bounds_tensor_surface.n",
        "examples/expert_release/bounds/13_bounds_slice_span_view.n",
        "examples/expert_release/bounds/14_bounds_offset_contract.n",
        "examples/expert_release/types/15_type_contract_user_profile.n",
        "examples/expert_release/types/16_type_contract_packet_binary.n",
        "examples/expert_release/types/17_type_contract_bounds_fields.n",
        "examples/expert_release/types/18_type_contract_visibility_matrix.n",
        "examples/expert_release/types/19_type_contract_copy_clone_move.n",
        "examples/expert_release/types/20_type_contract_result_option.n",
        "examples/expert_release/types/21_type_contract_ffi_repr_c.n",
        "examples/expert_release/memory/22_memory_array_move_copy_reverse.n",
        "examples/expert_release/memory/23_memory_region_move_copy_fill.n",
        "examples/expert_release/memory/24_memory_memorymove_overlap.n",
        "examples/expert_release/memory/25_memory_ptr_origin_copy.n",
        "examples/expert_release/memory/26_memory_list_move_swap.n",
        "examples/expert_release/memory/27_memory_site_addr_trace.n",
        "examples/expert_release/pipelines/28_pipeline_analytics_batch.n",
        "examples/expert_release/pipelines/29_pipeline_driver_runtime_scaffold.n",
        "examples/expert_release/pipelines/30_enterprise_ultra_300.n",
    ];

    for fixture in fixtures {
        assert_check_ok(fixture);
    }
}

#[test]
fn expert_release_runtime_subset_runs_ok() {
    let fixtures = [
        "examples/expert_release/bounds/08_bounds_array_dynamic_guard.n",
        "examples/expert_release/bounds/09_bounds_list_dynamic.n",
        "examples/expert_release/memory/22_memory_array_move_copy_reverse.n",
        "examples/expert_release/memory/23_memory_region_move_copy_fill.n",
        "examples/expert_release/memory/26_memory_list_move_swap.n",
        "examples/expert_release/pipelines/28_pipeline_analytics_batch.n",
        "examples/expert_release/pipelines/29_pipeline_driver_runtime_scaffold.n",
        "examples/expert_release/pipelines/30_enterprise_ultra_300.n",
    ];

    for fixture in fixtures {
        assert_run_ok(fixture);
    }
}

#[test]
fn expert_release_compiler_ffi_smoke_json_ok() {
    let out = run_ncc(&["compiler", "ffi-smoke", "--json"]);
    assert!(
        out.status.success(),
        "compiler ffi-smoke must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("\"status\": \"ok\"") || stdout.contains("\"status\": \"skipped\""),
        "ffi-smoke must emit status ok|skipped\nstdout={}",
        stdout
    );
}

#[test]
fn expert_release_compiler_bounds_json_ok() {
    let path = fixture_path("examples/expert_release/bounds/08_bounds_array_dynamic_guard.n");
    let out = run_ncc(&[
        "compiler",
        "bounds",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(
        out.status.success(),
        "compiler bounds must succeed\\nstdout={}\\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("\"path\"") && stdout.contains("\"codes\""),
        "compiler bounds payload must include path/codes\\nstdout={}",
        stdout
    );
}
