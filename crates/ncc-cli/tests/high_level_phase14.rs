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
fn high_level_phase14_status_exists() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase13_compile_time_evaluation_model"], "closed");
    assert_eq!(json["phase14_macros_codegen"], "closed");
    assert_eq!(json["macros"], "closed");
    assert_eq!(json["macro_declarative"], "closed");
    assert_eq!(json["macro_template"], "closed");
    assert_eq!(json["macro_attribute"], "closed");
    assert_eq!(json["macro_derive"], "closed");
    assert_eq!(json["unsafe_macros"], "closed_explicit_allow_audited");
    assert_eq!(json["hygiene"], "closed");
    assert_eq!(json["source_maps"], "closed");
    assert_eq!(json["expansion_reports"], "closed");
    assert_eq!(json["phase14_plus"], "phase14_closed");
    assert_eq!(json["phase15_plus"], "phase15_closed");
}

#[test]
fn high_level_phase14_macros_command_ok() {
    let json = command_json(&["high-level", "macros", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["phase"], "phase14_macros_compile_time_code_generation");
    assert_eq!(json["macro_families"]["declarative"], "closed");
    assert_eq!(json["macro_families"]["template"], "closed");
    assert_eq!(json["macro_families"]["attribute"], "closed");
    assert_eq!(json["macro_families"]["derive"], "closed");
    assert_eq!(
        json["macro_families"]["unsafe"],
        "closed_explicit_allow_audited"
    );
    assert_eq!(json["private_leak"], "forbidden");
    assert_eq!(json["any_leak"], "forbidden");
    assert_eq!(json["phase15_plus"], "phase15_closed");
}

#[test]
fn high_level_phase14_macro_expand_command_ok() {
    let path = example("examples/high_level/phase14/macro_declarative_simple.n");
    let json = command_json(&["high-level", "macro-expand", &path, "--json"]);
    assert_eq!(json["command"], "macro-expand");
    assert_eq!(json["has_errors"], false);
    assert_eq!(json["macro_report"]["expansion_report"]["status"], "closed");
    assert_eq!(json["macro_report"]["hygiene"]["status"], "closed");
}

#[test]
fn high_level_phase14_macro_report_command_ok() {
    let path = example("examples/high_level/phase14/macro_unsafe_no_limit_audited.n");
    let json = command_json(&["high-level", "macro-report", &path, "--json"]);
    assert_eq!(json["command"], "macro-report");
    assert_eq!(json["has_errors"], false);
    assert_eq!(
        json["macro_report"]["unsafe_effects"][0]["capability"],
        "sideEffects.report"
    );
    assert_eq!(json["macro_report"]["declarations"][0]["no_limit"], true);
}

#[test]
fn high_level_phase14_macro_hygiene_command_ok() {
    let path = example("examples/high_level/phase14/macro_hygiene_prevents_capture.n");
    let json = command_json(&["high-level", "macro-hygiene", &path, "--json"]);
    assert_eq!(json["command"], "macro-hygiene");
    assert_eq!(json["has_errors"], false);
    assert_eq!(json["macro_report"]["hygiene"]["status"], "closed");
    assert_eq!(json["macro_report"]["hygiene"]["private_leak"], "forbidden");
    assert_eq!(json["macro_report"]["hygiene"]["any_leak"], "forbidden");
}

ok_example_test!(
    high_level_phase14_macro_declarative_simple_ok,
    "examples/high_level/phase14/macro_declarative_simple.n"
);
ok_example_test!(
    high_level_phase14_macro_declarative_expression_template_ok,
    "examples/high_level/phase14/macro_declarative_expression_template.n"
);
ok_example_test!(
    high_level_phase14_macro_typed_parameter_ok,
    "examples/high_level/phase14/macro_typed_parameter.n"
);
ok_example_test!(
    high_level_phase14_macro_generating_function_ok,
    "examples/high_level/phase14/macro_generating_function.n"
);
ok_example_test!(
    high_level_phase14_macro_generating_type_ok,
    "examples/high_level/phase14/macro_generating_type.n"
);
ok_example_test!(
    high_level_phase14_macro_const_eval_pure_ok,
    "examples/high_level/phase14/macro_const_eval_pure.n"
);
ok_example_test!(
    high_level_phase14_macro_enum_metadata_public_ok,
    "examples/high_level/phase14/macro_enum_metadata_public.n"
);
ok_example_test!(
    high_level_phase14_macro_typeinfo_public_safe_ok,
    "examples/high_level/phase14/macro_typeinfo_public_safe.n"
);
ok_example_test!(
    high_level_phase14_macro_hygiene_prevents_capture_ok,
    "examples/high_level/phase14/macro_hygiene_prevents_capture.n"
);
ok_example_test!(
    high_level_phase14_macro_explicit_capture_allowed_ok,
    "examples/high_level/phase14/macro_explicit_capture_allowed.n"
);
ok_example_test!(
    high_level_phase14_macro_local_generated_symbols_ok,
    "examples/high_level/phase14/macro_local_generated_symbols.n"
);
ok_example_test!(
    high_level_phase14_macro_source_mapping_ok,
    "examples/high_level/phase14/macro_source_mapping.n"
);
ok_example_test!(
    high_level_phase14_macro_template_generates_ast_ok,
    "examples/high_level/phase14/macro_template_generates_ast.n"
);
ok_example_test!(
    high_level_phase14_macro_attribute_on_function_ok,
    "examples/high_level/phase14/macro_attribute_on_function.n"
);
ok_example_test!(
    high_level_phase14_macro_derive_trait_ok,
    "examples/high_level/phase14/macro_derive_trait.n"
);
ok_example_test!(
    high_level_phase14_macro_nested_within_limits_ok,
    "examples/high_level/phase14/macro_nested_within_limits.n"
);
ok_example_test!(
    high_level_phase14_macro_recursion_within_limit_ok,
    "examples/high_level/phase14/macro_recursion_within_limit.n"
);
ok_example_test!(
    high_level_phase14_macro_expansion_report_ok,
    "examples/high_level/phase14/macro_expansion_report.n"
);
ok_example_test!(
    high_level_phase14_macro_unsafe_filesystem_allowed_ok,
    "examples/high_level/phase14/macro_unsafe_filesystem_allowed.n"
);
ok_example_test!(
    high_level_phase14_macro_unsafe_dynamic_eval_allowed_ok,
    "examples/high_level/phase14/macro_unsafe_dynamic_eval_allowed.n"
);
ok_example_test!(
    high_level_phase14_macro_unsafe_reflection_mutation_allowed_ok,
    "examples/high_level/phase14/macro_unsafe_reflection_mutation_allowed.n"
);
ok_example_test!(
    high_level_phase14_macro_unsafe_private_metadata_allowed_ok,
    "examples/high_level/phase14/macro_unsafe_private_metadata_allowed.n"
);
ok_example_test!(
    high_level_phase14_macro_unsafe_no_limit_audited_ok,
    "examples/high_level/phase14/macro_unsafe_no_limit_audited.n"
);
ok_example_test!(
    high_level_phase14_macro_generated_code_passes_sema_ok,
    "examples/high_level/phase14/macro_generated_code_passes_sema.n"
);
ok_example_test!(
    high_level_phase14_macro_comptime_bridge_audited_ok,
    "examples/high_level/phase14/macro_comptime_bridge_audited.n"
);

fail_example_test!(
    high_level_phase14_macro_parse_invalid_fail,
    "examples/high_level/phase14/negative/macro_parse_invalid_fail.n",
    "E_HIGH_MACRO_PARSE_001"
);
fail_example_test!(
    high_level_phase14_macro_pattern_mismatch_fail,
    "examples/high_level/phase14/negative/macro_pattern_mismatch_fail.n",
    "E_HIGH_MACRO_PATTERN_001"
);
fail_example_test!(
    high_level_phase14_macro_invalid_generated_ast_fail,
    "examples/high_level/phase14/negative/macro_invalid_generated_ast_fail.n",
    "E_HIGH_MACRO_INVALID_GENERATED_AST_001"
);
fail_example_test!(
    high_level_phase14_macro_hygiene_capture_fail,
    "examples/high_level/phase14/negative/macro_hygiene_capture_fail.n",
    "E_HIGH_MACRO_CAPTURE_001"
);
fail_example_test!(
    high_level_phase14_macro_symbol_collision_fail,
    "examples/high_level/phase14/negative/macro_symbol_collision_fail.n",
    "E_HIGH_MACRO_SYMBOL_COLLISION_001"
);
fail_example_test!(
    high_level_phase14_macro_private_leak_fail,
    "examples/high_level/phase14/negative/macro_private_leak_fail.n",
    "E_HIGH_MACRO_PRIVATE_LEAK_001"
);
fail_example_test!(
    high_level_phase14_macro_any_leak_fail,
    "examples/high_level/phase14/negative/macro_any_leak_fail.n",
    "E_HIGH_MACRO_ANY_LEAK_001"
);
fail_example_test!(
    high_level_phase14_macro_visibility_violation_fail,
    "examples/high_level/phase14/negative/macro_visibility_violation_fail.n",
    "E_HIGH_MACRO_PRIVATE_METADATA_001"
);
fail_example_test!(
    high_level_phase14_macro_recursion_limit_fail,
    "examples/high_level/phase14/negative/macro_recursion_limit_fail.n",
    "E_HIGH_MACRO_RECURSION_LIMIT_001"
);
fail_example_test!(
    high_level_phase14_macro_expansion_step_limit_fail,
    "examples/high_level/phase14/negative/macro_expansion_step_limit_fail.n",
    "E_HIGH_MACRO_EXPANSION_LIMIT_001"
);
fail_example_test!(
    high_level_phase14_macro_generated_size_limit_fail,
    "examples/high_level/phase14/negative/macro_generated_size_limit_fail.n",
    "E_HIGH_MACRO_GENERATED_SIZE_LIMIT_001"
);
fail_example_test!(
    high_level_phase14_macro_nested_expansion_limit_fail,
    "examples/high_level/phase14/negative/macro_nested_expansion_limit_fail.n",
    "E_HIGH_MACRO_NESTING_LIMIT_001"
);
fail_example_test!(
    high_level_phase14_macro_const_eval_side_effect_fail,
    "examples/high_level/phase14/negative/macro_const_eval_side_effect_fail.n",
    "E_HIGH_MACRO_CONST_EVAL_FAILED_001"
);
fail_example_test!(
    high_level_phase14_macro_const_eval_private_metadata_fail,
    "examples/high_level/phase14/negative/macro_const_eval_private_metadata_fail.n",
    "E_HIGH_MACRO_CONST_EVAL_VISIBILITY_001"
);
fail_example_test!(
    high_level_phase14_macro_filesystem_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_filesystem_without_unsafe_fail.n",
    "E_HIGH_MACRO_UNSAFE_REQUIRED_001"
);
fail_example_test!(
    high_level_phase14_macro_process_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_process_without_unsafe_fail.n",
    "E_HIGH_MACRO_UNSAFE_REQUIRED_001"
);
fail_example_test!(
    high_level_phase14_macro_system_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_system_without_unsafe_fail.n",
    "E_HIGH_MACRO_UNSAFE_REQUIRED_001"
);
fail_example_test!(
    high_level_phase14_macro_dynamic_eval_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_dynamic_eval_without_unsafe_fail.n",
    "E_HIGH_MACRO_DYNAMIC_EVAL_001"
);
fail_example_test!(
    high_level_phase14_macro_reflection_mutation_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_reflection_mutation_without_unsafe_fail.n",
    "E_HIGH_MACRO_REFLECTION_MUTATION_001"
);
fail_example_test!(
    high_level_phase14_macro_private_metadata_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_private_metadata_without_unsafe_fail.n",
    "E_HIGH_MACRO_PRIVATE_METADATA_001"
);
fail_example_test!(
    high_level_phase14_macro_construction_leak_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_construction_leak_without_unsafe_fail.n",
    "E_HIGH_MACRO_CONSTRUCTION_LEAK_001"
);
fail_example_test!(
    high_level_phase14_macro_side_effect_without_unsafe_fail,
    "examples/high_level/phase14/negative/macro_side_effect_without_unsafe_fail.n",
    "E_HIGH_MACRO_UNSAFE_REQUIRED_001"
);
fail_example_test!(
    high_level_phase14_macro_no_limit_without_explicit_fail,
    "examples/high_level/phase14/negative/macro_no_limit_without_explicit_fail.n",
    "E_HIGH_MACRO_NOLIMIT_001"
);
fail_example_test!(
    high_level_phase14_unsafe_macro_undeclared_filesystem_effect_fail,
    "examples/high_level/phase14/negative/unsafe_macro_undeclared_filesystem_effect_fail.n",
    "E_HIGH_MACRO_UNDECLARED_EFFECT_001"
);
fail_example_test!(
    high_level_phase14_unsafe_macro_undeclared_dynamic_eval_fail,
    "examples/high_level/phase14/negative/unsafe_macro_undeclared_dynamic_eval_fail.n",
    "E_HIGH_MACRO_UNDECLARED_EFFECT_001"
);
fail_example_test!(
    high_level_phase14_unsafe_macro_undeclared_reflection_mutation_fail,
    "examples/high_level/phase14/negative/unsafe_macro_undeclared_reflection_mutation_fail.n",
    "E_HIGH_MACRO_UNDECLARED_EFFECT_001"
);
fail_example_test!(
    high_level_phase14_macro_generated_code_sema_fail,
    "examples/high_level/phase14/negative/macro_generated_code_sema_fail.n",
    "E_HIGH_MACRO_INVALID_GENERATED_AST_001"
);
fail_example_test!(
    high_level_phase14_macro_source_map_missing_fail,
    "examples/high_level/phase14/negative/macro_source_map_missing_fail.n",
    "E_HIGH_MACRO_SOURCE_MAP_001"
);

#[test]
fn high_level_phase14_reports_exist() {
    for rel in [
        "reports/high_level/phase14/run_phase14_macro_sweep.py",
        "reports/high_level/phase14/phase14_macros_sweep.json",
        "reports/high_level/phase14/phase14_macro_manifest.json",
        "reports/high_level/phase14/phase14_macro_positive_manifest.json",
        "reports/high_level/phase14/phase14_macro_negative_manifest.json",
        "reports/high_level/phase14/phase14_macro_expansion_report.json",
        "reports/high_level/phase14/phase14_macro_hygiene_report.json",
        "reports/high_level/phase14/phase14_macro_unsafe_effects_report.json",
        "reports/high_level/phase14/phase14_doctor_after.json",
        "reports/high_level/phase14/phase14_next_steps.md",
    ] {
        let path = workspace_root().join(rel);
        assert!(path.exists(), "{} must exist", path.display());
    }
}
