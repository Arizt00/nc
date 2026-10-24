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

fn command_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("json output")
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
fn high_level_phase15_status_exists() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase14_macros_codegen"], "closed");
    assert_eq!(json["phase15_risk_authority_expert_policy"], "closed");
    assert_eq!(json["risk_authority"], "closed");
    assert_eq!(json["expert_policy_surface"], "closed_integrated");
    assert_eq!(json["explicit_expert_mode"], "not_created_by_design");
    assert_eq!(json["phase15_plus"], "phase15_closed");
    assert_eq!(json["phase16_plus"], "not_started");
}

#[test]
fn high_level_phase15_risk_command_ok() {
    let json = command_json(&["high-level", "risk", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(
        json["phase"],
        "phase15_risk_authority_expert_policy_surface"
    );
    assert_eq!(json["expert_policy_surface"], "closed_integrated");
    assert_eq!(json["separate_expert_mode"], "not_created");
    assert_eq!(json["phase16_plus"], "not_started");
}

#[test]
fn high_level_phase15_risk_report_command_ok() {
    let path = example("examples/high_level/phase15/risk_policy_basic.n");
    let json = command_json(&["high-level", "risk-report", &path, "--json"]);
    assert_eq!(json["command"], "risk-report");
    assert_eq!(json["has_errors"], false);
    assert_eq!(json["risk_report"]["status"], "closed");
    assert_eq!(
        json["risk_report"]["policy"]["authority"],
        "integrated_high_level_policy_surface"
    );
    assert_eq!(json["risk_report"]["policy"]["accept_count"], 1);
}

#[test]
fn high_level_phase15_authority_report_command_ok() {
    let path = example("examples/high_level/phase15/risk_unsafe_bridge_machine.n");
    let json = command_json(&["high-level", "authority-report", &path, "--json"]);
    assert_eq!(json["command"], "authority-report");
    assert_eq!(json["has_errors"], false);
    assert_eq!(json["risk_report"]["bridges"][0]["target"], "machine");
    assert_eq!(
        json["risk_report"]["authority"]["unsafe_bridge"],
        "explicit_allow_reason_authority_required"
    );
}

#[test]
fn high_level_phase15_unification_prep_command_ok() {
    let json = command_json(&["high-level", "unification-prep", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["runtime"], "prepared_not_merged");
    assert_eq!(json["compiler"], "prepared_not_merged");
    assert_eq!(json["separate_expert_mode"], "not_created");
    assert_eq!(json["phase16_plus"], "not_started");
}

#[test]
fn high_level_phase15_macro_regression_still_ok() {
    let path = example("examples/high_level/phase14/macro_declarative_simple.n");
    let json = command_json(&["high-level", "macro-report", &path, "--json"]);
    assert_eq!(json["has_errors"], false);
    assert_eq!(json["manifest"]["phase15_plus"], "phase15_closed");
    check_ok("examples/high_level/phase14/macro_declarative_simple.n");
}

ok_example_test!(
    high_level_phase15_risk_policy_basic_ok,
    "examples/high_level/phase15/risk_policy_basic.n"
);
ok_example_test!(
    high_level_phase15_risk_allow_race_ok,
    "examples/high_level/phase15/risk_allow_race.n"
);
ok_example_test!(
    high_level_phase15_risk_alias_mutation_ok,
    "examples/high_level/phase15/risk_alias_mutation.n"
);
ok_example_test!(
    high_level_phase15_risk_stream_no_backpressure_ok,
    "examples/high_level/phase15/risk_stream_no_backpressure.n"
);
ok_example_test!(
    high_level_phase15_risk_channel_no_backpressure_ok,
    "examples/high_level/phase15/risk_channel_no_backpressure.n"
);
ok_example_test!(
    high_level_phase15_risk_no_backpressure_both_ok,
    "examples/high_level/phase15/risk_no_backpressure_both.n"
);
ok_example_test!(
    high_level_phase15_risk_no_rollback_ok,
    "examples/high_level/phase15/risk_no_rollback.n"
);
ok_example_test!(
    high_level_phase15_risk_partial_commit_ok,
    "examples/high_level/phase15/risk_partial_commit.n"
);
ok_example_test!(
    high_level_phase15_risk_partial_match_ok,
    "examples/high_level/phase15/risk_partial_match.n"
);
ok_example_test!(
    high_level_phase15_risk_unchecked_unwrap_ok,
    "examples/high_level/phase15/risk_unchecked_unwrap.n"
);
ok_example_test!(
    high_level_phase15_risk_unchecked_cast_ok,
    "examples/high_level/phase15/risk_unchecked_cast.n"
);
ok_example_test!(
    high_level_phase15_risk_effect_erasure_ok,
    "examples/high_level/phase15/risk_effect_erasure.n"
);
ok_example_test!(
    high_level_phase15_risk_private_metadata_ok,
    "examples/high_level/phase15/risk_private_metadata.n"
);
ok_example_test!(
    high_level_phase15_risk_any_leak_ok,
    "examples/high_level/phase15/risk_any_leak.n"
);
ok_example_test!(
    high_level_phase15_risk_any_construction_ok,
    "examples/high_level/phase15/risk_any_construction.n"
);
ok_example_test!(
    high_level_phase15_risk_construction_leak_ok,
    "examples/high_level/phase15/risk_construction_leak.n"
);
ok_example_test!(
    high_level_phase15_risk_raw_ptr_ok,
    "examples/high_level/phase15/risk_raw_ptr.n"
);
ok_example_test!(
    high_level_phase15_risk_raw_addr_ok,
    "examples/high_level/phase15/risk_raw_addr.n"
);
ok_example_test!(
    high_level_phase15_risk_raw_site_ok,
    "examples/high_level/phase15/risk_raw_site.n"
);
ok_example_test!(
    high_level_phase15_raw_ptr_ptrv2_no_allow_ok,
    "examples/high_level/phase15/risk_raw_ptr_ptrv2_no_allow_ok.n"
);
ok_example_test!(
    high_level_phase15_raw_addr_ptrv2_no_allow_ok,
    "examples/high_level/phase15/risk_raw_addr_ptrv2_no_allow_ok.n"
);
ok_example_test!(
    high_level_phase15_raw_site_ptrv2_no_allow_ok,
    "examples/high_level/phase15/risk_raw_site_ptrv2_no_allow_ok.n"
);
ok_example_test!(
    high_level_phase15_risk_unsafe_bridge_machine_ok,
    "examples/high_level/phase15/risk_unsafe_bridge_machine.n"
);
ok_example_test!(
    high_level_phase15_risk_comptime_unsafe_ok,
    "examples/high_level/phase15/risk_comptime_unsafe.n"
);
ok_example_test!(
    high_level_phase15_risk_unsafe_macro_ok,
    "examples/high_level/phase15/risk_unsafe_macro.n"
);
ok_example_test!(
    high_level_phase15_risk_no_limit_loop_ok,
    "examples/high_level/phase15/risk_no_limit_loop.n"
);
ok_example_test!(
    high_level_phase15_risk_no_limit_recursion_ok,
    "examples/high_level/phase15/risk_no_limit_recursion.n"
);
ok_example_test!(
    high_level_phase15_risk_authority_alias_ok,
    "examples/high_level/phase15/risk_authority_alias.n"
);

fail_example_test!(
    high_level_phase15_race_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_race_missing_allow_fail.n",
    "E_HIGH_RISK_RACE_REQUIRES_ALLOW_001"
);
fail_example_test!(
    high_level_phase15_alias_mutation_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_alias_mutation_missing_allow_fail.n",
    "E_HIGH_RISK_ALIAS_MUTATION_001"
);
fail_example_test!(
    high_level_phase15_stream_backpressure_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_stream_backpressure_missing_allow_fail.n",
    "E_HIGH_RISK_STREAM_BACKPRESSURE_001"
);
fail_example_test!(
    high_level_phase15_channel_backpressure_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_channel_backpressure_missing_allow_fail.n",
    "E_HIGH_RISK_CHANNEL_BACKPRESSURE_001"
);
fail_example_test!(
    high_level_phase15_no_backpressure_missing_one_allow_fail,
    "examples/high_level/phase15/negative/risk_no_backpressure_missing_one_allow_fail.n",
    "E_HIGH_RISK_CHANNEL_BACKPRESSURE_001"
);
fail_example_test!(
    high_level_phase15_no_rollback_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_no_rollback_missing_allow_fail.n",
    "E_HIGH_RISK_NO_ROLLBACK_001"
);
fail_example_test!(
    high_level_phase15_partial_commit_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_partial_commit_missing_allow_fail.n",
    "E_HIGH_RISK_PARTIAL_COMMIT_001"
);
fail_example_test!(
    high_level_phase15_partial_match_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_partial_match_missing_allow_fail.n",
    "E_HIGH_RISK_PARTIAL_MATCH_001"
);
fail_example_test!(
    high_level_phase15_unchecked_unwrap_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_unchecked_unwrap_missing_allow_fail.n",
    "E_HIGH_RISK_UNCHECKED_UNWRAP_001"
);
fail_example_test!(
    high_level_phase15_unchecked_cast_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_unchecked_cast_missing_allow_fail.n",
    "E_HIGH_RISK_UNCHECKED_CAST_001"
);
fail_example_test!(
    high_level_phase15_effect_erasure_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_effect_erasure_missing_allow_fail.n",
    "E_HIGH_RISK_EFFECT_ERASURE_001"
);
fail_example_test!(
    high_level_phase15_private_metadata_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_private_metadata_missing_allow_fail.n",
    "E_HIGH_RISK_PRIVATE_METADATA_001"
);
fail_example_test!(
    high_level_phase15_any_leak_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_any_leak_missing_allow_fail.n",
    "E_HIGH_RISK_ANY_LEAK_001"
);
fail_example_test!(
    high_level_phase15_any_construction_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_any_construction_missing_allow_fail.n",
    "E_HIGH_RISK_ANY_LEAK_001"
);
fail_example_test!(
    high_level_phase15_construction_leak_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_construction_leak_missing_allow_fail.n",
    "E_HIGH_RISK_CONSTRUCTION_LEAK_001"
);
fail_example_test!(
    high_level_phase15_bridge_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_bridge_missing_allow_fail.n",
    "E_HIGH_RISK_BRIDGE_AUTHORITY_001"
);
fail_example_test!(
    high_level_phase15_bridge_missing_authority_fail,
    "examples/high_level/phase15/negative/risk_bridge_missing_authority_fail.n",
    "E_HIGH_RISK_BRIDGE_AUTHORITY_001"
);
fail_example_test!(
    high_level_phase15_bridge_unknown_target_fail,
    "examples/high_level/phase15/negative/risk_bridge_unknown_target_fail.n",
    "E_HIGH_RISK_BRIDGE_AUTHORITY_001"
);
fail_example_test!(
    high_level_phase15_comptime_unsafe_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_comptime_unsafe_missing_allow_fail.n",
    "E_HIGH_RISK_COMPTIME_UNSAFE_001"
);
fail_example_test!(
    high_level_phase15_unsafe_macro_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_unsafe_macro_missing_allow_fail.n",
    "E_HIGH_RISK_UNSAFE_MACRO_001"
);
fail_example_test!(
    high_level_phase15_no_limit_missing_allow_fail,
    "examples/high_level/phase15/negative/risk_no_limit_missing_allow_fail.n",
    "E_HIGH_RISK_NOLIMIT_001"
);
fail_example_test!(
    high_level_phase15_unbounded_loop_missing_nolimit_fail,
    "examples/high_level/phase15/negative/risk_unbounded_loop_missing_nolimit_fail.n",
    "E_HIGH_RISK_NOLIMIT_001"
);
fail_example_test!(
    high_level_phase15_unbounded_recursion_missing_specific_allow_fail,
    "examples/high_level/phase15/negative/risk_unbounded_recursion_missing_specific_allow_fail.n",
    "E_HIGH_RISK_UNBOUNDED_RECURSION_001"
);
fail_example_test!(
    high_level_phase15_impossible_lowering_fail,
    "examples/high_level/phase15/negative/risk_impossible_lowering_fail.n",
    "E_HIGH_RISK_BRIDGE_AUTHORITY_001"
);
fail_example_test!(
    high_level_phase15_unresolved_symbol_fail,
    "examples/high_level/phase15/negative/risk_unresolved_symbol_fail.n",
    "E_HIGH_RISK_POLICY_001"
);
fail_example_test!(
    high_level_phase15_invalid_type_fail,
    "examples/high_level/phase15/negative/risk_invalid_type_fail.n",
    "E_HIGH_RISK_POLICY_001"
);
fail_example_test!(
    high_level_phase15_declared_false_safety_fail,
    "examples/high_level/phase15/negative/risk_declared_false_safety_fail.n",
    "E_HIGH_RISK_FAKE_SAFETY_001"
);
fail_example_test!(
    high_level_phase15_silent_fallback_fail,
    "examples/high_level/phase15/negative/risk_silent_fallback_fail.n",
    "E_HIGH_RISK_SILENT_FALLBACK_001"
);
