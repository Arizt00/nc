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
    let root = workspace_root();
    let direct = root.join(rel);
    let path = if direct.exists() {
        direct
    } else if let Some(rest) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            archived
        } else {
            direct
        }
    } else {
        direct
    };
    path.to_str().expect("utf8 path").to_string()
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
fn high_level_closures_status_exists() {
    let status =
        workspace_root().join("docs/status/STATUS_high_level_closures_lambdas_captures.md");
    assert!(status.exists(), "{} must exist", status.display());
}

#[test]
fn high_level_closures_doctor_gate_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase10_modules_packages_api_surface"], "closed");
    assert_eq!(json["closures_lambdas_captures"], "closed");
    assert_eq!(json["closures"], "closed");
    assert_eq!(json["lambdas"], "closed");
    assert_eq!(json["captures"], "closed");
    assert_eq!(json["capture_analysis"], "closed");
    assert_eq!(json["escape_analysis"], "closed");
    assert_eq!(json["closure_effects"], "closed");
    assert_eq!(json["function_types"], "closed");
    assert_eq!(json["reflection"], "closed_limited");
    assert_eq!(json["compile_time_evaluation"], "closed");
}

#[test]
fn high_level_closures_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "closures", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "lambdas", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "captures", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_closures_phase14_closed_without_phase15() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["closures_plus"], "closures_closed");
    assert_eq!(json["phase12_plus"], "phase12_closed");
    assert_eq!(json["phase13_plus"], "phase13_closed");
    assert_eq!(json["phase14_plus"], "phase14_closed");
    assert_eq!(json["phase15_plus"], "phase15_closed");
    assert_eq!(json["reflection"], "closed_limited");
    assert_eq!(json["compile_time_evaluation"], "closed");
}

ok_example_test!(
    high_level_closures_lambda_basic_ok,
    "examples/high_level/closures/01_lambda_basic.n"
);
ok_example_test!(
    high_level_closures_lambda_no_params_ok,
    "examples/high_level/closures/02_lambda_no_params.n"
);
ok_example_test!(
    high_level_closures_lambda_as_argument_ok,
    "examples/high_level/closures/03_lambda_as_argument.n"
);
ok_example_test!(
    high_level_closures_lambda_as_local_value_ok,
    "examples/high_level/closures/04_lambda_as_local_value.n"
);
ok_example_test!(
    high_level_closures_capture_value_ok,
    "examples/high_level/closures/05_closure_capture_value.n"
);
ok_example_test!(
    high_level_closures_capture_ref_ok,
    "examples/high_level/closures/06_closure_capture_ref.n"
);
ok_example_test!(
    high_level_closures_capture_mut_ok,
    "examples/high_level/closures/07_closure_capture_mut.n"
);
ok_example_test!(
    high_level_closures_move_closure_ok,
    "examples/high_level/closures/08_move_closure_basic.n"
);
ok_example_test!(
    high_level_closures_return_owned_ok,
    "examples/high_level/closures/09_closure_return_owned.n"
);
ok_example_test!(
    high_level_closures_option_result_ok,
    "examples/high_level/closures/10_closure_option_result.n"
);
ok_example_test!(
    high_level_closures_trait_bound_param_ok,
    "examples/high_level/closures/11_trait_bound_closure_param.n"
);
ok_example_test!(
    high_level_closures_effect_closure_ok,
    "examples/high_level/closures/12_effect_closure_basic.n"
);
ok_example_test!(
    high_level_closures_non_escaping_ref_ok,
    "examples/high_level/closures/13_non_escaping_ref_capture.n"
);
ok_example_test!(
    high_level_closures_iterator_map_ok,
    "examples/high_level/closures/14_iterator_map_closure.n"
);
ok_example_test!(
    high_level_closures_iterator_filter_ok,
    "examples/high_level/closures/15_iterator_filter_closure.n"
);
ok_example_test!(
    high_level_closures_iterator_fold_ok,
    "examples/high_level/closures/16_iterator_fold_closure.n"
);

fail_example_test!(
    high_level_closures_missing_param_type_fail,
    "examples/high_level/closures/negative/01_missing_param_type_fail.n",
    "E_HIGH_CLOSURE_PARAM_001"
);
fail_example_test!(
    high_level_closures_return_type_mismatch_fail,
    "examples/high_level/closures/negative/02_return_type_mismatch_fail.n",
    "E_HIGH_FUNCTION_TYPE_MISMATCH_001"
);
fail_example_test!(
    high_level_closures_call_arg_mismatch_fail,
    "examples/high_level/closures/negative/03_call_arg_mismatch_fail.n",
    "E_HIGH_CLOSURE_ARG_001"
);
fail_example_test!(
    high_level_closures_unknown_capture_fail,
    "examples/high_level/closures/negative/04_unknown_capture_fail.n",
    "E_HIGH_CAPTURE_UNKNOWN_001"
);
fail_example_test!(
    high_level_closures_private_capture_fail,
    "examples/high_level/closures/negative/05_private_capture_fail.n",
    "E_HIGH_CAPTURE_VISIBILITY_001"
);
fail_example_test!(
    high_level_closures_uninitialized_capture_fail,
    "examples/high_level/closures/negative/06_uninitialized_capture_fail.n",
    "E_HIGH_CAPTURE_UNINITIALIZED_001"
);
fail_example_test!(
    high_level_closures_moved_capture_fail,
    "examples/high_level/closures/negative/07_moved_capture_fail.n",
    "E_HIGH_CAPTURE_MOVED_001"
);
fail_example_test!(
    high_level_closures_mut_capture_required_fail,
    "examples/high_level/closures/negative/08_mut_capture_required_fail.n",
    "E_HIGH_CAPTURE_MUT_REQUIRED_001"
);
fail_example_test!(
    high_level_closures_capture_alias_fail,
    "examples/high_level/closures/negative/09_capture_alias_fail.n",
    "E_HIGH_CAPTURE_ALIAS_001"
);
fail_example_test!(
    high_level_closures_return_borrow_fail,
    "examples/high_level/closures/negative/10_return_borrow_fail.n",
    "E_HIGH_CLOSURE_RETURN_BORROW_001"
);
fail_example_test!(
    high_level_closures_effect_missing_fail,
    "examples/high_level/closures/negative/11_effect_missing_fail.n",
    "E_HIGH_CLOSURE_EFFECT_MISSING_001"
);
fail_example_test!(
    high_level_closures_effect_mismatch_fail,
    "examples/high_level/closures/negative/12_effect_mismatch_fail.n",
    "E_HIGH_CLOSURE_EFFECT_MISMATCH_001"
);
fail_example_test!(
    high_level_closures_async_closure_requires_future_callable_fail,
    "examples/high_level/closures/negative/13_async_closure_deferred_fail.n",
    "E_HIGH_FUNCTION_TYPE_MISMATCH_001"
);
fail_example_test!(
    high_level_closures_yield_outside_generator_fail,
    "examples/high_level/closures/negative/14_generator_closure_deferred_fail.n",
    "E_HIGH_YIELD_CONTEXT_001"
);
fail_example_test!(
    high_level_closures_any_param_fail,
    "examples/high_level/closures/negative/15_any_param_fail.n",
    "E_HIGH_CLOSURE_ANY_LEAK_001"
);
fail_example_test!(
    high_level_closures_function_type_mismatch_fail,
    "examples/high_level/closures/negative/16_function_type_mismatch_fail.n",
    "E_HIGH_FUNCTION_TYPE_MISMATCH_001"
);
fail_example_test!(
    high_level_closures_implicit_capture_fail,
    "examples/high_level/closures/negative/17_implicit_capture_fail.n",
    "implicit capture"
);
fail_example_test!(
    high_level_closures_private_type_capture_fail,
    "examples/high_level/closures/negative/18_private_type_capture_fail.n",
    "type section"
);
fail_example_test!(
    high_level_closures_task_capture_unsafe_fail,
    "examples/high_level/closures/negative/19_task_capture_unsafe_fail.n",
    "E_HIGH_EFFECT_UNKNOWN_001"
);
fail_example_test!(
    high_level_closures_iterator_closure_type_fail,
    "examples/high_level/closures/negative/20_iterator_closure_type_fail.n",
    "E_HIGH_ITERATOR_ADAPTER_TYPE_001"
);

#[test]
fn high_level_phase10_examples_still_pass() {
    check_ok("examples/high_level/phase10/01_module_basic.n");
    check_ok("examples/high_level/phase10/16_deprecated_experimental_report.n");
}

#[test]
fn machine_asm_complete_total_still_true() {
    let json = command_json(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert_eq!(json["asm_complete_total"], true);
}
