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

fn doctor_json() -> serde_json::Value {
    let out = run_ncc(&["high-level", "doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("doctor json")
}

fn generics_json() -> serde_json::Value {
    let out = run_ncc(&["high-level", "generics", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("generics json")
}

#[test]
fn high_level_phase3_status_exists() {
    let json = doctor_json();
    assert_eq!(json["phase3_generics_complete"], "closed");
    assert_eq!(json["phase4_interfaces_traits_contracts"], "closed");
    assert_eq!(json["phase5_dynamic_dispatch_trait_objects"], "closed");
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
}

#[test]
fn high_level_phase3_generics_manifest_ok() {
    let json = generics_json();
    assert_eq!(json["phase"], "phase3_generics_complete");
    assert_eq!(json["generic_functions"]["status"], "closed");
    assert_eq!(json["generic_types"]["status"], "closed");
    assert_eq!(json["runtime_generic_families"]["status"], "closed");
}

#[test]
fn high_level_phase3_generics_alias_ok() {
    let out = run_ncc(&["highlevel", "generics", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("alias json");
    assert_eq!(json["status"], "high_level_phase3_generics_complete_closed");
}

#[test]
fn high_level_phase3_generic_identity_ok() {
    check_ok("examples/high_level/phase3/generic_identity_ok.n");
}

#[test]
fn high_level_phase3_generic_function_explicit_ok() {
    check_ok("examples/high_level/phase3/generic_function_explicit_ok.n");
}

#[test]
fn high_level_phase3_generic_function_infer_ok() {
    check_ok("examples/high_level/phase3/generic_function_infer_ok.n");
}

#[test]
fn high_level_phase3_generic_list_ok() {
    check_ok("examples/high_level/phase3/generic_list_ok.n");
}

#[test]
fn high_level_phase3_generic_nested_list_ok() {
    check_ok("examples/high_level/phase3/generic_nested_list_ok.n");
}

#[test]
fn high_level_phase3_generic_ptr_ok() {
    check_ok("examples/high_level/phase3/generic_ptr_ok.n");
}

#[test]
fn high_level_phase3_generic_memregion_ok() {
    check_ok("examples/high_level/phase3/generic_memregion_ok.n");
}

#[test]
fn high_level_phase3_generic_array_ok() {
    check_ok("examples/high_level/phase3/generic_array_ok.n");
}

#[test]
fn high_level_phase3_generic_runtime_list_ok() {
    check_ok("examples/high_level/phase3/generic_runtime_list_ok.n");
}

#[test]
fn high_level_phase3_generic_runtime_region_ok() {
    check_ok("examples/high_level/phase3/generic_runtime_region_ok.n");
}

#[test]
fn high_level_phase3_generic_runtime_ptr_ok() {
    check_ok("examples/high_level/phase3/generic_runtime_ptr_ok.n");
}

#[test]
fn high_level_phase3_generic_lowering_ok() {
    check_ok("examples/high_level/phase3/generic_lowering_ok.n");
}

#[test]
fn high_level_phase3_duplicate_param_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_duplicate_param_fail.n",
        "E_HIGH_GENERIC_DUP_PARAM_001",
    );
}

#[test]
fn high_level_phase3_unknown_param_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_unknown_param_fail.n",
        "E_HIGH_GENERIC_UNKNOWN_PARAM_001",
    );
}

#[test]
fn high_level_phase3_wrong_arity_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_wrong_arity_fail.n",
        "E_HIGH_GENERIC_ARITY_001",
    );
}

#[test]
fn high_level_phase3_unknown_base_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_unknown_base_fail.n",
        "E_HIGH_GENERIC_UNKNOWN_BASE_001",
    );
}

#[test]
fn high_level_phase3_list_type_mismatch_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_list_type_mismatch_fail.n",
        "incompatible element types",
    );
}

#[test]
fn high_level_phase3_ptr_type_mismatch_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_ptr_type_mismatch_fail.n",
        "E_HIGH_GENERIC_TYPE_MISMATCH_001",
    );
}

#[test]
fn high_level_phase3_runtime_type_mismatch_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_runtime_type_mismatch_fail.n",
        "E_RUNTIME_GENERIC_ARG_MISMATCH_001",
    );
}

#[test]
fn high_level_phase3_infer_ambiguous_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_infer_ambiguous_fail.n",
        "E_HIGH_GENERIC_INFER_AMBIGUOUS_001",
    );
}

#[test]
fn high_level_phase3_return_only_infer_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_return_only_infer_fail.n",
        "E_HIGH_GENERIC_INFER_RETURN_ONLY_001",
    );
}

#[test]
fn high_level_phase3_unresolved_lowering_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_unresolved_lowering_fail.n",
        "E_HIGH_GENERIC_INFER_RETURN_ONLY_001",
    );
}

#[test]
fn high_level_phase3_any_leak_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_any_leak_fail.n",
        "E_HIGH_GENERIC_ANY_LEAK_001",
    );
}

#[test]
fn high_level_phase3_hkt_deferred_fail() {
    check_fail(
        "examples/high_level/phase3/negative/generic_hkt_deferred_fail.n",
        "E_HIGH_GENERIC_HKT_DEFERRED_001",
    );
}

#[test]
fn high_level_phase3_does_not_start_phase4_plus() {
    let json = doctor_json();
    assert_eq!(json["phase4_interfaces_traits_contracts"], "closed");
    assert_eq!(json["phase5_dynamic_dispatch_trait_objects"], "closed");
    assert_eq!(
        json["generics"]["no_scope_phase3"]["higher_kinded_types"],
        "deferred"
    );
}
