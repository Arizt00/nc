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
    assert!(
        !out.status.success(),
        "{} unexpectedly passed:\n{}",
        rel,
        text
    );
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

#[test]
fn high_level_phase7_doctor_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["option"], "closed");
    assert_eq!(json["result"], "closed");
    assert_eq!(json["error_algebra"], "closed");
    assert_eq!(json["try_operator"], "closed_postfix_and_prefix");
    assert_eq!(json["unwrap"], "legacy_unqualified_option_deferred");
    assert_eq!(json["typed_unwrap_helpers"], "closed");
    assert_eq!(json["iterators_ranges"], "closed");
    assert_eq!(json["phase8_iterators_ranges_collections"], "closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase7_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "option", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "result", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "errors", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_phase7_positive_examples_ok() {
    for rel in [
        "examples/high_level/phase7/01_option_some_basic.n",
        "examples/high_level/phase7/02_option_none_basic.n",
        "examples/high_level/phase7/03_option_match_exhaustive.n",
        "examples/high_level/phase7/04_option_generic_container.n",
        "examples/high_level/phase7/05_result_ok_basic.n",
        "examples/high_level/phase7/06_result_err_basic.n",
        "examples/high_level/phase7/07_result_match_exhaustive.n",
        "examples/high_level/phase7/08_result_error_enum_basic.n",
        "examples/high_level/phase7/09_result_nested_basic.n",
        "examples/high_level/phase7/10_result_with_contract.n",
        "examples/high_level/phase7/11_result_with_trait_bound.n",
        "examples/high_level/phase7/12_result_propagation_match_explicit.n",
        "examples/high_level/phase7/13_result_try_operator_deferred_surface.n",
        "examples/high_level/phase7/14_option_helper_status.n",
        "examples/high_level/phase7/15_result_error_transport.n",
        "examples/high_level/phase7/16_error_conversion_explicit.n",
    ] {
        check_ok(rel);
    }
}

#[test]
fn high_level_phase7_negative_examples_fail() {
    for (rel, needle) in [
        (
            "examples/high_level/phase7/negative/01_option_wrong_arity_fail.n",
            "E_HIGH_OPTION_ARITY_001",
        ),
        (
            "examples/high_level/phase7/negative/02_option_none_no_context_fail.n",
            "E_HIGH_OPTION_NONE_CONTEXT_001",
        ),
        (
            "examples/high_level/phase7/negative/03_option_some_type_mismatch_fail.n",
            "E_HIGH_OPTION_SOME_TYPE_001",
        ),
        (
            "examples/high_level/phase7/negative/04_result_wrong_arity_fail.n",
            "E_HIGH_RESULT_ARITY_001",
        ),
        (
            "examples/high_level/phase7/negative/05_result_ok_type_mismatch_fail.n",
            "E_HIGH_RESULT_OK_TYPE_001",
        ),
        (
            "examples/high_level/phase7/negative/06_result_err_type_mismatch_fail.n",
            "E_HIGH_RESULT_ERR_TYPE_001",
        ),
        (
            "examples/high_level/phase7/negative/07_result_error_any_fail.n",
            "E_HIGH_RESULT_ERROR_ANY_001",
        ),
        (
            "examples/high_level/phase7/negative/08_result_match_non_exhaustive_fail.n",
            "E_HIGH_MATCH_NON_EXHAUSTIVE_001",
        ),
        (
            "examples/high_level/phase7/negative/09_result_unreachable_arm_fail.n",
            "E_HIGH_MATCH_UNREACHABLE_ARM_001",
        ),
        (
            "examples/high_level/phase7/negative/10_try_on_non_result_fail.n",
            "E_HIGH_TRY_OPERAND_001",
        ),
        (
            "examples/high_level/phase7/negative/11_try_in_non_result_function_fail.n",
            "E_HIGH_TRY_CONTEXT_001",
        ),
        (
            "examples/high_level/phase7/negative/12_try_error_mismatch_fail.n",
            "E_HIGH_TRY_CONTEXT_001",
        ),
        (
            "examples/high_level/phase7/negative/13_error_conversion_missing_fail.n",
            "E_HIGH_RESULT_ERR_TYPE_001",
        ),
        (
            "examples/high_level/phase7/negative/14_error_erasure_fail.n",
            "E_HIGH_ERROR_ERASURE_001",
        ),
        (
            "examples/high_level/phase7/negative/15_unwrap_unsafe_fail.n",
            "E_HIGH_UNWRAP_DEFERRED_001",
        ),
        (
            "examples/high_level/phase7/negative/16_exception_syntax_not_supported_fail.n",
            "E_HIGH_EXCEPTION_UNSUPPORTED_001",
        ),
        (
            "examples/high_level/phase7/negative/17_any_leak_option_fail.n",
            "E_HIGH_ANY_LEAK_001",
        ),
        (
            "examples/high_level/phase7/negative/18_any_leak_result_fail.n",
            "E_HIGH_ANY_LEAK_001",
        ),
    ] {
        check_fail(rel, needle);
    }
}

#[test]
fn high_level_phase7_regressions_ok() {
    check_ok("examples/high_level/phase6/16_full_phase6_surface.n");
    let machine = command_json(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert_eq!(machine["asm_complete_total"], true);
}
