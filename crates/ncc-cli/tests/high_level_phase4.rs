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
fn high_level_phase4_status_exists() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase4_interfaces_traits_contracts"], "closed");
    assert_eq!(json["interfaces"], "closed");
    assert_eq!(json["traits"], "closed");
    assert_eq!(json["contracts"], "closed");
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase4_doctor_gate_ok() {
    let out = run_ncc(&["high-level", "doctor"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let text = combined_output(&out);
    assert!(text.contains("phase4_interfaces_traits_contracts: closed"));
}

#[test]
fn high_level_phase4_interface_trait_contract_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "interfaces", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "traits", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "contracts", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_phase4_remains_closed_after_phase5() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase5_dynamic_dispatch_trait_objects"], "closed");
    assert_eq!(json["dynamic_dispatch"], "closed");
    assert_eq!(json["higher_kinded_types"], "deferred");
}

#[test]
fn high_level_phase4_interface_basic_ok() {
    check_ok("examples/high_level/phase4/interface_basic_ok.n");
}

#[test]
fn high_level_phase4_interface_generic_bound_ok() {
    check_ok("examples/high_level/phase4/interface_generic_bound_ok.n");
}

#[test]
fn high_level_phase4_trait_required_ok() {
    check_ok("examples/high_level/phase4/trait_required_ok.n");
}

#[test]
fn high_level_phase4_trait_default_ok() {
    check_ok("examples/high_level/phase4/trait_default_ok.n");
}

#[test]
fn high_level_phase4_impl_signature_ok() {
    check_ok("examples/high_level/phase4/impl_signature_ok.n");
}

#[test]
fn high_level_phase4_generic_bound_inline_ok() {
    check_ok("examples/high_level/phase4/generic_bound_inline_ok.n");
}

#[test]
fn high_level_phase4_generic_bound_where_ok() {
    check_ok("examples/high_level/phase4/generic_bound_where_ok.n");
}

#[test]
fn high_level_phase4_multiple_bounds_ok() {
    check_ok("examples/high_level/phase4/multiple_bounds_ok.n");
}

#[test]
fn high_level_phase4_contract_require_ok() {
    check_ok("examples/high_level/phase4/contract_require_ok.n");
}

#[test]
fn high_level_phase4_contract_generic_bound_ok() {
    check_ok("examples/high_level/phase4/contract_generic_bound_ok.n");
}

#[test]
fn high_level_phase4_static_dispatch_concrete_ok() {
    check_ok("examples/high_level/phase4/static_dispatch_concrete_ok.n");
}

#[test]
fn high_level_phase4_static_dispatch_generic_ok() {
    check_ok("examples/high_level/phase4/static_dispatch_generic_ok.n");
}

#[test]
fn high_level_phase4_bound_monomorphization_ok() {
    check_ok("examples/high_level/phase4/bound_monomorphization_ok.n");
}

#[test]
fn high_level_phase4_interface_trait_same_file_ok() {
    check_ok("examples/high_level/phase4/interface_trait_same_file_ok.n");
}

#[test]
fn high_level_phase4_interface_two_methods_ok() {
    check_ok("examples/high_level/phase4/interface_two_methods_ok.n");
}

#[test]
fn high_level_phase4_full_surface_ok() {
    check_ok("examples/high_level/phase4/full_phase4_surface_ok.n");
}

#[test]
fn high_level_phase4_interface_missing_method_fail() {
    check_fail(
        "examples/high_level/phase4/negative/interface_missing_method_fail.n",
        "E_HIGH_INTERFACE_METHOD_MISSING_001",
    );
}

#[test]
fn high_level_phase4_interface_signature_mismatch_fail() {
    check_fail(
        "examples/high_level/phase4/negative/interface_signature_mismatch_fail.n",
        "E_HIGH_INTERFACE_METHOD_SIGNATURE_001",
    );
}

#[test]
fn high_level_phase4_trait_missing_required_fail() {
    check_fail(
        "examples/high_level/phase4/negative/trait_missing_required_fail.n",
        "E_HIGH_TRAIT_METHOD_MISSING_001",
    );
}

#[test]
fn high_level_phase4_trait_signature_mismatch_fail() {
    check_fail(
        "examples/high_level/phase4/negative/trait_signature_mismatch_fail.n",
        "E_HIGH_TRAIT_METHOD_SIGNATURE_001",
    );
}

#[test]
fn high_level_phase4_impl_unknown_trait_fail() {
    check_fail(
        "examples/high_level/phase4/negative/impl_unknown_trait_fail.n",
        "E_HIGH_IMPL_UNKNOWN_TRAIT_001",
    );
}

#[test]
fn high_level_phase4_duplicate_interface_fail() {
    check_fail(
        "examples/high_level/phase4/negative/duplicate_interface_fail.n",
        "E_HIGH_INTERFACE_DUP_001",
    );
}

#[test]
fn high_level_phase4_duplicate_trait_fail() {
    check_fail(
        "examples/high_level/phase4/negative/duplicate_trait_fail.n",
        "E_HIGH_TRAIT_DUP_001",
    );
}

#[test]
fn high_level_phase4_duplicate_impl_fail() {
    check_fail(
        "examples/high_level/phase4/negative/duplicate_impl_fail.n",
        "E_HIGH_IMPL_DUP_001",
    );
}

#[test]
fn high_level_phase4_bound_unknown_trait_fail() {
    check_fail(
        "examples/high_level/phase4/negative/bound_unknown_trait_fail.n",
        "E_HIGH_BOUND_UNKNOWN_TRAIT_001",
    );
}

#[test]
fn high_level_phase4_bound_unsatisfied_fail() {
    check_fail(
        "examples/high_level/phase4/negative/bound_unsatisfied_fail.n",
        "E_HIGH_BOUND_UNSATISFIED_001",
    );
}

#[test]
fn high_level_phase4_bound_unknown_param_fail() {
    check_fail(
        "examples/high_level/phase4/negative/bound_unknown_param_fail.n",
        "E_HIGH_BOUND_UNKNOWN_PARAM_001",
    );
}

#[test]
fn high_level_phase4_bound_dynamic_dispatch_deferred_fail() {
    check_fail(
        "examples/high_level/phase4/negative/bound_dynamic_dispatch_deferred_fail.n",
        "E_HIGH_BOUND_DYNAMIC_DISPATCH_DEFERRED_001",
    );
}

#[test]
fn high_level_phase4_bound_hkt_deferred_fail() {
    check_fail(
        "examples/high_level/phase4/negative/bound_hkt_deferred_fail.n",
        "E_HIGH_BOUND_HKT_DEFERRED_001",
    );
}

#[test]
fn high_level_phase4_contract_false_fail() {
    check_fail(
        "examples/high_level/phase4/negative/contract_false_fail.n",
        "E_HIGH_CONTRACT_REQUIRE_FAIL_001",
    );
}

#[test]
fn high_level_phase4_contract_ensure_deferred_fail() {
    check_fail(
        "examples/high_level/phase4/negative/contract_ensure_deferred_fail.n",
        "E_HIGH_CONTRACT_ENSURE_DEFERRED_001",
    );
}

#[test]
fn high_level_phase4_contract_invariant_deferred_fail() {
    check_fail(
        "examples/high_level/phase4/negative/contract_invariant_deferred_fail.n",
        "E_HIGH_CONTRACT_INVARIANT_DEFERRED_001",
    );
}

#[test]
fn high_level_phase4_contract_unverifiable_fail() {
    check_fail(
        "examples/high_level/phase4/negative/contract_unverifiable_fail.n",
        "E_HIGH_CONTRACT_REQUIRE_UNVERIFIABLE_001",
    );
}

#[test]
fn high_level_phase4_static_dispatch_ambiguous_fail() {
    check_fail(
        "examples/high_level/phase4/negative/static_dispatch_ambiguous_fail.n",
        "E_HIGH_TRAIT_AMBIGUOUS_IMPL_001",
    );
}

#[test]
fn high_level_phase4_regressions_phase3_still_pass() {
    check_ok("examples/high_level/phase3/generic_identity_ok.n");
    check_ok("examples/high_level/phase3/generic_function_infer_ok.n");
}
