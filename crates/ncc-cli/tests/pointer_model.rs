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
        .expect("ncc invocation must run")
}

fn combined_output(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn assert_check_ok(relative: &str) {
    let path = fixture_path(relative);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "{relative} must pass\n{}",
        combined_output(&out)
    );
}

fn assert_check_fails_with(relative: &str, code: &str) {
    let path = fixture_path(relative);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let text = combined_output(&out);
    assert!(!out.status.success(), "{relative} must fail\n{text}");
    assert!(
        text.contains(code),
        "{relative} must contain {code}\n{text}"
    );
}

fn assert_run_ok(relative: &str) {
    let path = fixture_path(relative);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    assert!(
        out.status.success(),
        "{relative} must run\n{}",
        combined_output(&out)
    );
}

fn assert_run_reports(relative: &str, code: &str) {
    let path = fixture_path(relative);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let text = combined_output(&out);
    assert!(text.contains(code), "{relative} must report {code}\n{text}");
}

fn temp_source_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("ncc_{name}_{}_{}.n", std::process::id(), nanos))
}

fn write_temp_source(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write temp nC source");
}

#[test]
fn pointer_model_manifest_and_doctor_are_closed() {
    let manifest = run_ncc(&["high-level", "pointer-model", "--json"]);
    assert!(manifest.status.success(), "{}", combined_output(&manifest));
    let json: serde_json::Value =
        serde_json::from_slice(&manifest.stdout).expect("pointer-model json");
    assert_eq!(json["status"], "closed");
    assert_eq!(
        json["nullability"]["implicit_null"],
        "accepted_for_p_ptr_addr_as_C_like_undefined_behavior_on_dereference"
    );
    assert_eq!(
        json["ffi_contracts"]["diagnostic"],
        "ffi-declaration:E_PTR_FFI_CONTRACT_001"
    );

    let doctor = run_ncc(&["doctor", "--json"]);
    assert!(doctor.status.success(), "{}", combined_output(&doctor));
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("doctor json");
    assert_eq!(
        doctor_json["included_doctors"]["pointer_model"]["status"],
        "closed"
    );
}

#[test]
fn pointer_model_positive_examples_pass() {
    for relative in [
        "examples/high_level/pointers/pointer_model_safe_ok.n",
        "examples/high_level/pointers/pointer_model_bounds_alignment_ok.n",
        "examples/high_level/pointers/pointer_model_raw_hazard_ok.n",
        "examples/high_level/pointers/pointer_model_silent_pointer_acceptance_ok.n",
        "examples/high_level/pointers/pointer_model_ffi_contract_ok.n",
        "examples/high_level/pointers/pointer_raw_ptrv2_no_risk_ok.n",
        "examples/high_level/pointers/pointer_silent_ptrv2_no_authority_ok.n",
        "examples/pointers/ptr_lowering_raw_backend_ok.n",
        "examples/pointers/ptr_region_view_without_risk_ok.n",
        "examples/pointers/ptr_compact_c_style_ok.n",
        "examples/pointers/ptr_copy_shallow_object_ok.n",
        "examples/pointers/ptr_copy_explicit_shallow_ok.n",
        "examples/pointers/ptr_clone_contract_deep_ok.n",
        "examples/pointers/ptr_move_object_ok.n",
        "examples/pointers/ptr_reinterpret_byte_to_qword_ok.n",
        "examples/pointers/ptr_raw_null_deref_allowed_ok.n",
        "examples/pointers/ptr_raw_use_after_free_allowed_ok.n",
        "examples/pointers/ptr_raw_wrong_pointee_type_allowed_ok.n",
        "examples/06_memory/rawPointerOutOfBounds.n",
        "examples/06_memory/punteros_c_compacto.n",
    ] {
        assert_check_ok(relative);
    }
}

#[test]
fn compact_c_pointer_helpers_keep_heap_region_views_coherent_at_run_time() {
    let path = fixture_path("examples/06_memory/punteros_c_compacto.n");
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let text = combined_output(&out);
    assert!(
        out.status.success(),
        "compact pointer fixture must run\n{text}"
    );
    assert!(text.contains("valor = 29"), "unexpected output\n{text}");
    assert!(text.contains("offset = 2"), "unexpected output\n{text}");
    assert!(
        text.contains("region[2] = 29"),
        "heap write through Write* must be visible through regionRead\n{text}"
    );
}

#[test]
fn pointer_model_raw_dangerous_examples_compile_without_implicit_checks() {
    for relative in [
        "examples/high_level/pointers/negative/pointer_implicit_null_fail.n",
        "examples/high_level/pointers/negative/pointer_nullable_deref_fail.n",
        "examples/high_level/pointers/negative/pointer_provenance_uninit_fail.n",
        "examples/high_level/pointers/negative/pointer_alignment_fail.n",
        "examples/high_level/pointers/negative/pointer_stale_site_fail.n",
        "examples/high_level/pointers/negative/pointer_alias_mut_fail.n",
        "examples/pointers/negative/addr_to_ptr_without_policy_fail.n",
        "examples/pointers/negative/alias_mutation_fail.n",
        "examples/pointers/negative/null_deref_fail.n",
        "examples/pointers/negative/null_to_non_nullable_ptr_fail.n",
        "examples/pointers/negative/ptr_alignment_fail.n",
        "examples/pointers/negative/stack_pointer_escape_fail.n",
        "examples/06_memory/rawPointerUndefinedBehavior.n",
    ] {
        assert_check_ok(relative);
    }
}

#[test]
fn bounded_memregion_examples_keep_their_independent_bounds_contract() {
    for relative in [
        "examples/high_level/pointers/negative/pointer_out_of_bounds_fail.n",
        "examples/pointers/negative/ptr_bounds_fail.n",
    ] {
        assert_check_fails_with(relative, "E_PTR_OUT_OF_BOUNDS_001");
    }
}

#[test]
fn pointer_model_explicit_contract_errors_fail_officially() {
    for (relative, code) in [
        (
            "examples/high_level/pointers/negative/pointer_ffi_contract_fail.n",
            "E_PTR_FFI_CONTRACT_001",
        ),
        (
            "examples/pointers/negative/ptr_clone_missing_contract_fail.n",
            "E_TYPE_CLONE_001",
        ),
        (
            "examples/pointers/negative/ptr_compact_read_non_pointer_fail.n",
            "E_PTR_TYPE_002",
        ),
        (
            "examples/pointers/negative/ptr_move_use_after_move_fail.n",
            "E_TYPE_MOVE_AFTER_MOVE_001",
        ),
        (
            "examples/pointers/negative/ptr_copy_shallow_denied_contract_fail.n",
            "E_TYPE_COPY_DENIED_001",
        ),
    ] {
        assert_check_fails_with(relative, code);
    }
}

#[test]
fn pointer_model_raw_nullable_pointer_expression_is_not_checked_implicitly() {
    let cases = [(
        "nullable_index",
        r#"
package ptr_fuzz_nullable { profile: fuzz_lab; }

ptrFuzzNullable[@] {
    stc:
        p: ptr<int> | null;
        x: int;
    fn:
        boot() -> bool {
            public:
                @p >> ptr<int> | null : null;
                @x >> int : @p[0];
                return true;
        }
    ref main ->> @fn(boot);
    fa:
        return true;
}
"#,
    )];

    for (name, src) in cases {
        let path = temp_source_path(name);
        write_temp_source(&path, src);
        let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
        let text = combined_output(&out);
        let _ = std::fs::remove_file(path);
        assert!(out.status.success(), "{name} must pass in raw mode\n{text}");
        assert!(!text.contains("E_PTR_NULL_DEREF_001"), "{name}\n{text}");
    }
}

#[test]
fn pointer_industrial_closure_payloads_are_closed() {
    let high_level = run_ncc(&["high-level", "pointer-industrial", "--json"]);
    assert!(
        high_level.status.success(),
        "{}",
        combined_output(&high_level)
    );
    let high_json: serde_json::Value =
        serde_json::from_slice(&high_level.stdout).expect("high-level pointer json");
    assert_eq!(high_json["status"], "closed");
    assert_eq!(
        high_json["doctrine"]["programmer_power"],
        "c_cpp_grade_raw_pointer_power_by_default"
    );
    assert_eq!(
        high_json["doctrine"]["silent_implicit_ambiguity"],
        "accepted_as_raw_pointer_hazard_with_backend_evidence"
    );

    let compiler = run_ncc(&["compiler", "pointer-industrial", "--json"]);
    assert!(compiler.status.success(), "{}", combined_output(&compiler));
    let compiler_json: serde_json::Value =
        serde_json::from_slice(&compiler.stdout).expect("compiler pointer json");
    assert_eq!(compiler_json["status"], "closed");
    assert_eq!(
        compiler_json["c_cpp_parity"]["ub_policy"],
        "raw_pointer_out_of_range_is_programmer_control_not_bounded_safety"
    );
    assert_eq!(
        compiler_json["runtime_heap"]["alloc_size_unit"],
        "raw_bytes_never_scaled_by_pointee_type"
    );
    assert_eq!(
        compiler_json["runtime_heap"]["raw_range_policy"],
        "ptr_arithmetic_read_write_may_leave_allocation"
    );

    let lowering = run_ncc(&["compiler", "pointer-lowering", "--json"]);
    assert!(lowering.status.success(), "{}", combined_output(&lowering));
    let lowering_json: serde_json::Value =
        serde_json::from_slice(&lowering.stdout).expect("compiler pointer lowering json");
    assert_eq!(lowering_json["status"], "closed");
    assert_eq!(
        lowering_json["evidence"]["basal_pointer_surface_records"],
        "closed"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["ptrRead<int>(p)"],
        "load i32 [addr]"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["ptrWrite<int>(p, v)"],
        "store i32 [addr], v"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["ptrOffset<int>(p, 1)"],
        "addr + sizeof(int)"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["ptrDiff<T>(pB, pA)"],
        "(addrB - addrA) / sizeof(T)"
    );
    assert_eq!(
        lowering_json["memory_model"]["raw_pointer_bounds"],
        "unchecked allocation range; arithmetic wraps at target pointer width"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["regionFromPtr<int>(p, n)"],
        "view { base=addr(p), len=n, elem_width=sizeof(int), owner_generation=heap.generation }"
    );
    assert_eq!(
        lowering_json["physical_lowering_equations"]["regionPtrAt<int>(r, i)"],
        "addr = region.base + i * sizeof(int); bounds <= region.end; generation == heap.generation"
    );
    assert_eq!(lowering_json["abi_ffi_bridge"]["c_abi"]["status"], "closed");
    assert_eq!(
        lowering_json["abi_ffi_bridge"]["cpp_abi"]["status"],
        "closed_real_toolchain_smoke"
    );
    assert_eq!(
        lowering_json["abi_ffi_bridge"]["asm_bridge"]["status"],
        "closed"
    );
    assert_eq!(
        lowering_json["performance_measurement"]["status"],
        "measurable"
    );
    assert!(lowering_json["operations"]
        .as_array()
        .is_some_and(|items| items
            .iter()
            .any(|row| row["op"] == "ptrRead<T>"
                && row["physical_formula"] == "load i{width(T)} [addr]")));
    assert!(lowering_json["operations"]
        .as_array()
        .is_some_and(|items| items
            .iter()
            .any(|row| row["op"] == "regionFromPtr<T>/regionPtrAt<T>"
                && row["runtime_e2e_fixture"]
                    == "examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n")));

    let compiler_doctor = run_ncc(&["compiler", "doctor", "--json"]);
    assert!(
        compiler_doctor.status.success(),
        "{}",
        combined_output(&compiler_doctor)
    );
    let compiler_doctor_json: serde_json::Value =
        serde_json::from_slice(&compiler_doctor.stdout).expect("compiler doctor json");
    assert_eq!(compiler_doctor_json["pointer_surface"]["status"], "closed");

    let runtime_doctor = run_ncc(&["runtime", "doctor", "--json"]);
    assert!(
        runtime_doctor.status.success(),
        "{}",
        combined_output(&runtime_doctor)
    );
    let runtime_doctor_json: serde_json::Value =
        serde_json::from_slice(&runtime_doctor.stdout).expect("runtime doctor json");
    assert_eq!(
        runtime_doctor_json["pointer_runtime_heap_contract"]["status"],
        "closed"
    );
    assert_eq!(
        runtime_doctor_json["pointer_runtime_heap_contract"]["silent_pointer_ambiguity"],
        "accepted_as_raw_pointer_hazard_with_backend_evidence"
    );

    let doctor = run_ncc(&["doctor", "--json"]);
    assert!(doctor.status.success(), "{}", combined_output(&doctor));
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("unified doctor json");
    assert_eq!(
        doctor_json["included_doctors"]["pointer_industrial_closure"]["status"],
        "closed"
    );
}

#[test]
fn pointer_runtime_calls_execute_end_to_end() {
    for relative in [
        "examples/pointers/ptr_runtime_read_write_offset_diff_e2e_ok.n",
        "examples/pointers/ptr_generic_short_full_runtime_ok.n",
        "examples/pointers/ptr_runtime_region_roundtrip_e2e_ok.n",
        "examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n",
        "examples/pointers/ptr_runtime_offset_one_past_ok.n",
        "examples/pointers/negative/ptr_runtime_offset_oob_fail.n",
        "examples/06_memory/rawPointerOutOfBounds.n",
        "examples/pointers/ptr_copy_explicit_shallow_ok.n",
        "examples/pointers/ptr_clone_contract_deep_ok.n",
        "examples/pointers/ptr_move_object_ok.n",
        "examples/pointers/ptr_compact_c_style_ok.n",
        "examples/punterosBasicoNormal.n",
    ] {
        assert_check_ok(relative);
        assert_run_ok(relative);
    }
}

#[test]
fn pointer_runtime_region_imports_expand_to_stoud_runtime_paths() {
    let path = fixture_path("examples/pointers/ptr_runtime_region_roundtrip_e2e_future.n");
    let out = run_ncc(&[
        "imports",
        "expand",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("imports expand json");
    let paths = json["imports"]
        .as_array()
        .expect("imports must be array")
        .iter()
        .filter_map(|item| item["path"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "nC.runtime.output",
        "nC.runtime.render",
        "nC.runtime.ptr",
        "nC.runtime.region",
        "nC.runtime.heap",
    ] {
        assert!(paths.contains(&expected), "missing {expected} in {paths:?}");
    }
}

#[test]
fn pointer_runtime_stale_read_reports_official_diagnostic() {
    assert_run_reports(
        "examples/pointers/negative/ptr_runtime_stale_read_fail.n",
        "E_PTR_STALE_SITE_001",
    );
    assert_check_ok("examples/pointers/ptr_region_view_without_risk_ok.n");
    assert_run_reports(
        "examples/pointers/negative/ptr_region_oob_fail.n",
        "E_REGION_BOUNDS_001",
    );
    assert_run_reports(
        "examples/pointers/negative/ptr_region_stale_after_free_fail.n",
        "E_PTR_STALE_SITE_001",
    );
    assert_run_reports(
        "examples/pointers/negative/ptr_region_type_mismatch_fail.n",
        "E_PTR_TYPE_002",
    );
}

#[test]
fn basal_pointer_surface_is_recorded_in_ir() {
    let path = fixture_path("examples/basal/phase_pointer/basal_pointer_surface_ok.nb");
    let check = run_ncc(&["basal", "check", path.to_str().expect("utf8 path")]);
    assert!(check.status.success(), "{}", combined_output(&check));

    let ir = run_ncc(&["basal", "ir", path.to_str().expect("utf8 path"), "--json"]);
    assert!(ir.status.success(), "{}", combined_output(&ir));
    let json: serde_json::Value = serde_json::from_slice(&ir.stdout).expect("basal ir json");
    assert_eq!(json["executable_surface"]["pointer_surface"], "closed");
    assert!(
        json["executable_surface"]["pointer_accesses"]
            .as_array()
            .is_some_and(|items| items.len() >= 4),
        "basal pointer accesses must be recorded"
    );
}

#[test]
fn pointer_industrial_positive_examples_pass() {
    for relative in [
        "examples/pointers/ffi_c_pointer_boundary_ok.n",
        "examples/pointers/heap_alloc_free_ok.n",
        "examples/pointers/interprocedural_free_propagates_ok.n",
        "examples/pointers/machine_lowering_ptr_read_write_ok.n",
        "examples/pointers/ptr_generic_short_full_runtime_ok.n",
        "examples/pointers/ptr_compact_c_style_ok.n",
        "examples/pointers/ptr_basic_read_write_ok.n",
        "examples/pointers/ptr_diff_ok.n",
        "examples/pointers/ptr_from_addr_expert_ok.n",
        "examples/pointers/ptr_nullable_refine_ok.n",
        "examples/pointers/ptr_offset_ok.n",
        "examples/pointers/realloc_new_pointer_ok.n",
        "examples/pointers/region_from_ptr_ok.n",
        "examples/high_level/phase15/risk_raw_ptr_addr_site_closed_ok.n",
    ] {
        assert_check_ok(relative);
    }
}

#[test]
fn pointer_industrial_ffi_contract_errors_fail_officially() {
    for (relative, code) in [(
        "examples/pointers/negative/ffi_pointer_missing_abi_fail.n",
        "E_FFI_ABI_MISSING_001",
    )] {
        assert_check_fails_with(relative, code);
    }
}
