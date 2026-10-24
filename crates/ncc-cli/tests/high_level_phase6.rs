use std::fs;
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
fn basal_phase6_name_guard_not_used_here() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
}

#[test]
fn high_level_phase6_doctor_gate_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase1_syntax_semantic_cleanup"], "closed");
    assert_eq!(json["phase2_type_system_advanced"], "closed");
    assert_eq!(json["phase3_generics_complete"], "closed");
    assert_eq!(json["phase4_interfaces_traits_contracts"], "closed");
    assert_eq!(json["phase5_dynamic_dispatch_trait_objects"], "closed");
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["enums"], "closed");
    assert_eq!(json["pattern_matching"], "closed");
    assert_eq!(json["exhaustiveness"], "closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase6_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "enums", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "patterns", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "match", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_phase6_enum_unit_basic_ok() {
    check_ok("examples/high_level/phase6/01_enum_unit_basic.n");
}

#[test]
fn high_level_phase6_enum_payload_basic_ok() {
    check_ok("examples/high_level/phase6/02_enum_payload_basic.n");
}

#[test]
fn high_level_phase6_match_exhaustive_ok() {
    check_ok("examples/high_level/phase6/03_match_exhaustive.n");
}

#[test]
fn high_level_phase6_match_wildcard_ok() {
    check_ok("examples/high_level/phase6/04_match_wildcard.n");
}

#[test]
fn high_level_phase6_match_guard_pure_ok() {
    check_ok("examples/high_level/phase6/05_match_guard_pure.n");
}

#[test]
fn high_level_phase6_generic_enum_basic_ok() {
    check_ok("examples/high_level/phase6/06_generic_enum_basic.n");
}

#[test]
fn high_level_phase6_generic_enum_payload_ok() {
    check_ok("examples/high_level/phase6/07_generic_enum_payload.n");
}

#[test]
fn high_level_phase6_pattern_bindings_scope_ok() {
    check_ok("examples/high_level/phase6/14_pattern_bindings_scope.n");
}

#[test]
fn high_level_phase6_full_surface_ok() {
    check_ok("examples/high_level/phase6/16_full_phase6_surface.n");
}

#[test]
fn high_level_phase6_enum_duplicate_fail() {
    check_fail(
        "examples/high_level/phase6/negative/01_enum_duplicate_fail.n",
        "E_HIGH_ENUM_DUP_001",
    );
}

#[test]
fn high_level_phase6_variant_duplicate_fail() {
    check_fail(
        "examples/high_level/phase6/negative/02_variant_duplicate_fail.n",
        "E_HIGH_ENUM_VARIANT_DUP_001",
    );
}

#[test]
fn high_level_phase6_unknown_variant_fail() {
    check_fail(
        "examples/high_level/phase6/negative/03_unknown_variant_fail.n",
        "E_HIGH_ENUM_VARIANT_UNKNOWN_001",
    );
}

#[test]
fn high_level_phase6_constructor_arity_fail() {
    check_fail(
        "examples/high_level/phase6/negative/04_constructor_arity_fail.n",
        "E_HIGH_ENUM_CONSTRUCTOR_ARITY_001",
    );
}

#[test]
fn high_level_phase6_constructor_type_fail() {
    check_fail(
        "examples/high_level/phase6/negative/05_constructor_type_fail.n",
        "E_HIGH_ENUM_CONSTRUCTOR_TYPE_001",
    );
}

#[test]
fn high_level_phase6_non_exhaustive_fail() {
    check_fail(
        "examples/high_level/phase6/negative/06_non_exhaustive_fail.n",
        "E_HIGH_MATCH_NON_EXHAUSTIVE_001",
    );
}

#[test]
fn high_level_phase6_unreachable_after_wildcard_fail() {
    check_fail(
        "examples/high_level/phase6/negative/07_unreachable_after_wildcard_fail.n",
        "E_HIGH_MATCH_UNREACHABLE_ARM_001",
    );
}

#[test]
fn high_level_phase6_duplicate_arm_fail() {
    check_fail(
        "examples/high_level/phase6/negative/08_duplicate_arm_fail.n",
        "E_HIGH_MATCH_UNREACHABLE_ARM_001",
    );
}

#[test]
fn high_level_phase6_pattern_arity_fail() {
    check_fail(
        "examples/high_level/phase6/negative/09_pattern_arity_fail.n",
        "E_HIGH_PATTERN_ARITY_001",
    );
}

#[test]
fn high_level_phase6_pattern_binding_dup_fail() {
    check_fail(
        "examples/high_level/phase6/negative/10_pattern_binding_dup_fail.n",
        "E_HIGH_PATTERN_BINDING_DUP_001",
    );
}

#[test]
fn high_level_phase6_match_non_enum_scrutinee_ok() {
    check_ok("examples/high_level/phase6/11_match_non_enum_scrutinee_ok.n");
}

#[test]
fn high_level_phase6_guard_non_bool_fail() {
    check_fail(
        "examples/high_level/phase6/negative/12_guard_non_bool_fail.n",
        "E_HIGH_GUARD_TYPE_001",
    );
}

#[test]
fn high_level_phase6_guard_bool_call_ok() {
    check_ok("examples/high_level/phase6/17_guard_bool_call_ok.n");
}

#[test]
fn high_level_phase6_match_arm_type_fail() {
    check_fail(
        "examples/high_level/phase6/negative/14_match_arm_type_fail.n",
        "E_HIGH_MATCH_ARM_TYPE_001",
    );
}

#[test]
fn high_level_phase6_any_leak_match_fail() {
    check_fail(
        "examples/high_level/phase6/negative/15_any_leak_match_fail.n",
        "E_HIGH_ANY_LEAK_001",
    );
}

#[test]
fn high_level_phase6_unknown_enum_pattern_fail() {
    check_fail(
        "examples/high_level/phase6/negative/16_unknown_enum_pattern_fail.n",
        "E_HIGH_ENUM_UNKNOWN_001",
    );
}

#[test]
fn high_level_phase6_enum_bounds_deferred_fail() {
    check_fail(
        "examples/high_level/phase6/negative/17_enum_bounds_deferred_fail.n",
        "E_HIGH_ENUM_PARSE_001",
    );
}

#[test]
fn high_level_phase6_empty_match_fail() {
    check_fail(
        "examples/high_level/phase6/negative/18_empty_match_fail.n",
        "E_HIGH_MATCH_EMPTY_001",
    );
}

#[test]
fn high_level_phase6_sweep_ok() {
    let root = fixture_path("examples/high_level/phase6");
    let mut positives = fs::read_dir(&root)
        .expect("phase6 examples dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "n"))
        .collect::<Vec<_>>();
    positives.sort();
    for path in positives {
        let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
        assert!(
            out.status.success(),
            "{}\n{}",
            path.display(),
            combined_output(&out)
        );
    }

    let negative_root = root.join("negative");
    let mut negatives = fs::read_dir(&negative_root)
        .expect("phase6 negative examples dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "n"))
        .collect::<Vec<_>>();
    negatives.sort();
    for path in negatives {
        let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
        assert!(
            !out.status.success(),
            "{} unexpectedly passed:\n{}",
            path.display(),
            combined_output(&out)
        );
    }
}

#[test]
fn high_level_phase6_does_not_start_phase7_plus() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["result"], "closed");
    assert_eq!(json["option"], "closed");
    assert_eq!(json["async_effects"], "closed");
    assert_eq!(json["phase8_iterators_ranges_collections"], "closed");
    assert_eq!(json["phase9_plus"], "phase9_closed");
    assert_eq!(json["closed_without_phase8_features"], false);
    assert_eq!(json["closed_without_phase9_features"], false);
}

#[test]
fn high_level_phase5_regression_still_passes() {
    check_ok("examples/high_level/phase5/01_dyn_trait_basic.n");
    check_ok("examples/high_level/phase5/05_associated_type_basic.n");
}

#[test]
fn machine_asm_complete_total_still_true() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json output");
    assert_eq!(json["asm_complete_total"], true);
}
