use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
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

fn example(rel: &str) -> String {
    fixture_path(rel).to_str().expect("utf8 path").to_string()
}

fn fixture_path(rel: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(rel);
    if direct.exists() {
        return direct;
    }
    if let Some(rest) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    direct
}

fn check_ok(rel: &str) {
    let path = example(rel);
    let out = run_ncc(&["check", &path]);
    assert!(out.status.success(), "{}\n{}", rel, combined_output(&out));
}

fn check_fail(rel: &str, needle: &str) {
    let path = example(rel);
    let out = run_ncc(&["check", &path]);
    let text = combined_output(&out);
    assert!(!out.status.success(), "{} unexpectedly passed", rel);
    assert!(
        text.contains(needle),
        "{} did not contain '{}':\n{}",
        rel,
        needle,
        text
    );
}

fn command_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("json output")
}

macro_rules! ok_example_test {
    ($name:ident, $rel:literal) => {
        #[test]
        fn $name() {
            check_ok($rel);
        }
    };
}

macro_rules! fail_example_test {
    ($name:ident, $rel:literal, $needle:literal) => {
        #[test]
        fn $name() {
            check_fail($rel, $needle);
        }
    };
}

#[test]
fn high_level_phase13_doctor_gate_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase12_limited_reflection_introspection"], "closed");
    assert_eq!(json["phase13_compile_time_evaluation_model"], "closed");
    assert_eq!(json["compile_time_evaluation"], "closed");
    assert_eq!(json["const_eval"], "closed_pure");
    assert_eq!(json["const_eval_pure"], true);
    assert_eq!(json["const_eval_deterministic"], true);
    assert_eq!(json["const_eval_bounded"], true);
    assert_eq!(json["const_eval_cacheable"], true);
    assert_eq!(json["const_eval_side_effects"], "forbidden");
    assert_eq!(json["const_eval_no_limit"], "forbidden");
    assert_eq!(json["comptime_unsafe"], "closed_explicit_allow_audited");
    assert_eq!(json["comptime_effects"], "closed");
    assert_eq!(json["comptime_no_limit"], "explicit_only");
    assert_eq!(json["phase13_plus"], "phase13_closed");
    assert_eq!(json["phase14_plus"], "phase14_closed");
    assert_eq!(json["phase15_plus"], "phase15_closed");
}

#[test]
fn high_level_phase13_const_eval_command_ok() {
    let path = example("examples/high_level/phase13/const_arithmetic.n");
    let json = command_json(&["high-level", "const-eval", &path, "--json"]);
    assert_eq!(json["status"], "passed");
    assert_eq!(json["const_eval"], "validated");
    assert_eq!(json["manifest"]["surface"], "const_eval");
}

#[test]
fn high_level_phase13_comptime_command_ok() {
    let path = example("examples/high_level/phase13/comptime_filesystem_read.n");
    let json = command_json(&["high-level", "comptime", &path, "--json"]);
    assert_eq!(json["status"], "passed");
    assert_eq!(
        json["comptime_unsafe"],
        "validated_and_audited_when_present"
    );
    assert_eq!(json["manifest"]["surface"], "comptime_unsafe");
}

#[test]
fn high_level_phase13_compile_time_command_ok() {
    let json = command_json(&["high-level", "compile-time", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["const_eval"]["status"], "closed");
    assert_eq!(json["comptime_unsafe"]["status"], "closed");
    assert_eq!(json["phase14_plus"], "not_started");
}

ok_example_test!(
    high_level_phase13_const_arithmetic_ok,
    "examples/high_level/phase13/const_arithmetic.n"
);
ok_example_test!(
    high_level_phase13_const_boolean_logic_ok,
    "examples/high_level/phase13/const_boolean_logic.n"
);
ok_example_test!(
    high_level_phase13_const_string_concat_ok,
    "examples/high_level/phase13/const_string_concat.n"
);
ok_example_test!(
    high_level_phase13_const_shift_bitwise_ok,
    "examples/high_level/phase13/const_shift_bitwise.n"
);
ok_example_test!(
    high_level_phase13_const_array_len_ok,
    "examples/high_level/phase13/const_array_len.n"
);
ok_example_test!(
    high_level_phase13_const_index_lookup_ok,
    "examples/high_level/phase13/const_index_lookup.n"
);
ok_example_test!(
    high_level_phase13_const_fn_add_ok,
    "examples/high_level/phase13/const_fn_add.n"
);
ok_example_test!(
    high_level_phase13_const_match_exhaustive_ok,
    "examples/high_level/phase13/const_match_exhaustive.n"
);
ok_example_test!(
    high_level_phase13_const_enum_variant_count_ok,
    "examples/high_level/phase13/const_enum_variant_count.n"
);
ok_example_test!(
    high_level_phase13_const_enum_discriminant_ok,
    "examples/high_level/phase13/const_enum_discriminant.n"
);
ok_example_test!(
    high_level_phase13_const_typeinfo_name_ok,
    "examples/high_level/phase13/const_typeinfo_name.n"
);
ok_example_test!(
    high_level_phase13_const_typekind_ok,
    "examples/high_level/phase13/const_typekind.n"
);
ok_example_test!(
    high_level_phase13_const_api_surface_count_ok,
    "examples/high_level/phase13/const_api_surface_count.n"
);
ok_example_test!(
    high_level_phase13_const_local_projection_ok,
    "examples/high_level/phase13/const_local_projection.n"
);
ok_example_test!(
    high_level_phase13_const_hex_binary_literals_ok,
    "examples/high_level/phase13/const_hex_binary_literals.n"
);
ok_example_test!(
    high_level_phase13_const_cacheable_reuse_ok,
    "examples/high_level/phase13/const_cacheable_reuse.n"
);
ok_example_test!(
    high_level_phase13_comptime_const_bridge_ok,
    "examples/high_level/phase13/comptime_const_bridge.n"
);
ok_example_test!(
    high_level_phase13_comptime_filesystem_read_ok,
    "examples/high_level/phase13/comptime_filesystem_read.n"
);
ok_example_test!(
    high_level_phase13_comptime_filesystem_write_bundle_ok,
    "examples/high_level/phase13/comptime_filesystem_write_bundle.n"
);
ok_example_test!(
    high_level_phase13_comptime_process_system_ok,
    "examples/high_level/phase13/comptime_process_system.n"
);
ok_example_test!(
    high_level_phase13_comptime_heap_reflection_private_ok,
    "examples/high_level/phase13/comptime_heap_reflection_private.n"
);
ok_example_test!(
    high_level_phase13_comptime_dynamic_construction_sideeffect_ok,
    "examples/high_level/phase13/comptime_dynamic_construction_sideeffect.n"
);
ok_example_test!(
    high_level_phase13_comptime_no_limit_loop_recursion_ok,
    "examples/high_level/phase13/comptime_no_limit_loop_recursion.n"
);
ok_example_test!(
    high_level_phase13_const_if_expression_ok,
    "examples/high_level/phase13/const_if_expression.n"
);

fail_example_test!(
    high_level_phase13_const_runtime_print_fail,
    "examples/high_level/phase13/negative/const_runtime_print_fail.n",
    "E_HIGH_CONST_EVAL_SIDE_EFFECT_001"
);
fail_example_test!(
    high_level_phase13_const_filesystem_forbidden_fail,
    "examples/high_level/phase13/negative/const_filesystem_forbidden_fail.n",
    "E_HIGH_CONST_EVAL_FORBIDDEN_EFFECT_001"
);
fail_example_test!(
    high_level_phase13_const_process_forbidden_fail,
    "examples/high_level/phase13/negative/const_process_forbidden_fail.n",
    "E_HIGH_CONST_EVAL_SYSTEM_FORBIDDEN_001"
);
fail_example_test!(
    high_level_phase13_const_ffi_forbidden_fail,
    "examples/high_level/phase13/negative/const_ffi_forbidden_fail.n",
    "E_HIGH_CONST_EVAL_FFI_FORBIDDEN_001"
);
fail_example_test!(
    high_level_phase13_const_machine_forbidden_fail,
    "examples/high_level/phase13/negative/const_machine_forbidden_fail.n",
    "E_HIGH_CONST_EVAL_MACHINE_FORBIDDEN_001"
);
fail_example_test!(
    high_level_phase13_const_heap_mutation_fail,
    "examples/high_level/phase13/negative/const_heap_mutation_fail.n",
    "E_HIGH_CONST_EVAL_FORBIDDEN_EFFECT_001"
);
fail_example_test!(
    high_level_phase13_const_reflection_mutation_fail,
    "examples/high_level/phase13/negative/const_reflection_mutation_fail.n",
    "E_HIGH_CONST_EVAL_REFLECTION_MUTATION_001"
);
fail_example_test!(
    high_level_phase13_const_dynamic_eval_fail,
    "examples/high_level/phase13/negative/const_dynamic_eval_fail.n",
    "E_HIGH_CONST_EVAL_DYNAMIC_EVAL_001"
);
fail_example_test!(
    high_level_phase13_const_private_metadata_fail,
    "examples/high_level/phase13/negative/const_private_metadata_fail.n",
    "E_HIGH_CONST_EVAL_PRIVATE_METADATA_001"
);
fail_example_test!(
    high_level_phase13_const_any_leak_fail,
    "examples/high_level/phase13/negative/const_any_leak_fail.n",
    "E_HIGH_CONST_EVAL_ANY_LEAK_001"
);
fail_example_test!(
    high_level_phase13_const_construction_leak_fail,
    "examples/high_level/phase13/negative/const_construction_leak_fail.n",
    "E_HIGH_CONST_EVAL_CONSTRUCTION_LEAK_001"
);
fail_example_test!(
    high_level_phase13_const_side_effect_fail,
    "examples/high_level/phase13/negative/const_side_effect_fail.n",
    "E_HIGH_CONST_EVAL_SIDE_EFFECT_001"
);
fail_example_test!(
    high_level_phase13_const_no_limit_fail,
    "examples/high_level/phase13/negative/const_no_limit_fail.n",
    "E_HIGH_CONST_EVAL_STEP_LIMIT_001"
);
fail_example_test!(
    high_level_phase13_const_unbounded_loop_fail,
    "examples/high_level/phase13/negative/const_unbounded_loop_fail.n",
    "E_HIGH_CONST_EVAL_STEP_LIMIT_001"
);
fail_example_test!(
    high_level_phase13_const_recursion_fail,
    "examples/high_level/phase13/negative/const_recursion_fail.n",
    "E_HIGH_CONST_EVAL_RECURSION_LIMIT_001"
);
fail_example_test!(
    high_level_phase13_const_overflow_fail,
    "examples/high_level/phase13/negative/const_overflow_fail.n",
    "E_HIGH_CONST_EVAL_OVERFLOW_001"
);
fail_example_test!(
    high_level_phase13_const_div_zero_fail,
    "examples/high_level/phase13/negative/const_div_zero_fail.n",
    "E_HIGH_CONST_EVAL_DIV_ZERO_001"
);
fail_example_test!(
    high_level_phase13_const_size_limit_fail,
    "examples/high_level/phase13/negative/const_size_limit_fail.n",
    "E_HIGH_CONST_EVAL_SIZE_LIMIT_001"
);
fail_example_test!(
    high_level_phase13_const_visibility_fail,
    "examples/high_level/phase13/negative/const_visibility_private_type_fail.n",
    "E_HIGH_CONST_EVAL_VISIBILITY_001"
);
fail_example_test!(
    high_level_phase13_const_match_non_exhaustive_fail,
    "examples/high_level/phase13/negative/const_match_non_exhaustive_fail.n",
    "E_HIGH_CONST_EVAL_FORBIDDEN_EFFECT_001"
);
fail_example_test!(
    high_level_phase13_comptime_missing_allow_fs_fail,
    "examples/high_level/phase13/negative/comptime_missing_allow_fs_fail.n",
    "E_HIGH_COMPTIME_FS_001"
);
fail_example_test!(
    high_level_phase13_comptime_process_no_allow_fail,
    "examples/high_level/phase13/negative/comptime_process_no_allow_fail.n",
    "E_HIGH_COMPTIME_PROCESS_001"
);
fail_example_test!(
    high_level_phase13_comptime_system_no_allow_fail,
    "examples/high_level/phase13/negative/comptime_system_no_allow_fail.n",
    "E_HIGH_COMPTIME_SYSTEM_001"
);
fail_example_test!(
    high_level_phase13_comptime_heap_no_allow_fail,
    "examples/high_level/phase13/negative/comptime_heap_no_allow_fail.n",
    "E_HIGH_COMPTIME_HEAP_MUTATION_001"
);
fail_example_test!(
    high_level_phase13_comptime_dynamic_no_allow_fail,
    "examples/high_level/phase13/negative/comptime_dynamic_no_allow_fail.n",
    "E_HIGH_COMPTIME_DYNAMIC_EVAL_001"
);
fail_example_test!(
    high_level_phase13_comptime_private_no_allow_fail,
    "examples/high_level/phase13/negative/comptime_private_no_allow_fail.n",
    "E_HIGH_COMPTIME_PRIVATE_METADATA_001"
);
fail_example_test!(
    high_level_phase13_comptime_no_limit_missing_fail,
    "examples/high_level/phase13/negative/comptime_no_limit_missing_fail.n",
    "E_HIGH_COMPTIME_NOLIMIT_001"
);
fail_example_test!(
    high_level_phase13_comptime_no_effect_fail,
    "examples/high_level/phase13/negative/comptime_no_effect_fail.n",
    "E_HIGH_COMPTIME_UNDECLARED_EFFECT_001"
);

#[test]
fn high_level_phase13_reports_exist() {
    for rel in [
        "reports/high_level/phase13/run_phase13_compile_time_sweep.py",
        "reports/high_level/phase13/phase13_compile_time_evaluation_sweep.json",
        "reports/high_level/phase13/phase13_const_eval_manifest.json",
        "reports/high_level/phase13/phase13_comptime_unsafe_manifest.json",
        "reports/high_level/phase13/phase13_positive_manifest.json",
        "reports/high_level/phase13/phase13_negative_manifest.json",
        "reports/high_level/phase13/phase13_comptime_effects_report.json",
        "reports/high_level/phase13/phase13_reproducibility_report.json",
        "reports/high_level/phase13/phase13_doctor_after.json",
        "reports/high_level/phase13/phase13_next_steps.md",
    ] {
        let path = workspace_root().join(rel);
        assert!(path.exists(), "{} must exist", path.display());
    }
}
