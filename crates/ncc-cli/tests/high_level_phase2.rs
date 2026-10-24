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

fn types_json() -> serde_json::Value {
    let out = run_ncc(&["high-level", "types", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("types json")
}

#[test]
fn high_level_phase2_status_exists() {
    let json = doctor_json();
    assert_eq!(json["phase1_syntax_semantic_cleanup"], "closed");
    assert_eq!(json["phase2_type_system_advanced"], "closed");
    assert_eq!(json["phase3_generics_complete"], "closed");
}

#[test]
fn high_level_phase2_types_command_ok() {
    let json = types_json();
    assert_eq!(
        json["status"],
        "high_level_phase2_type_system_advanced_closed"
    );
    assert_eq!(json["primitive_types"]["bool"]["separate_from_int"], true);
    assert_eq!(json["compound_types"]["union"]["status"], "closed");
}

#[test]
fn high_level_phase2_types_alias_command_ok() {
    let out = run_ncc(&["highlevel", "types", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("alias json");
    assert_eq!(json["phase"], "phase2_type_system_advanced");
}

#[test]
fn high_type_exact_assign_ok() {
    check_ok("examples/high_level/phase2/type_identity_assign_ok.n");
}

#[test]
fn high_type_assign_mismatch_fail() {
    check_fail(
        "examples/high_level/phase2/negative/type_assign_mismatch_fail.n",
        "projection type mismatch",
    );
}

#[test]
fn high_type_bool_not_int_fail() {
    check_fail(
        "examples/high_level/phase2/negative/bool_not_int_fail.n",
        "projection type mismatch",
    );
}

#[test]
fn high_type_raw_int_to_addr_is_allowed() {
    check_ok("examples/high_level/phase2/negative/addr_from_int_var_fail.n");
}

#[test]
fn high_type_raw_addr_to_ptr_is_allowed() {
    check_ok("examples/high_level/phase2/negative/ptr_from_addr_fail.n");
}

#[test]
fn high_type_array_len_mismatch_fail() {
    check_fail(
        "examples/high_level/phase2/negative/array_len_mismatch_fail.n",
        "E_HIGH_TYPE_ARRAY_LEN_001",
    );
}

#[test]
fn high_type_null_to_nonnullable_fail() {
    check_fail(
        "examples/high_level/phase2/negative/null_to_nonnullable_fail.n",
        "cannot project null",
    );
}

#[test]
fn high_type_union_assign_ok() {
    check_ok("examples/high_level/phase2/union_assign_ok.n");
}

#[test]
fn high_type_union_invalid_fail() {
    check_fail(
        "examples/high_level/phase2/negative/union_invalid_assign_fail.n",
        "projection type mismatch",
    );
}

#[test]
fn high_numeric_width_assign_ok() {
    check_ok("examples/high_level/phase2/numeric_widths_ok.n");
}

#[test]
fn high_numeric_width_overflow_fail() {
    check_fail(
        "examples/high_level/phase2/negative/numeric_width_narrowing_fail.n",
        "implicit narrowing",
    );
}

#[test]
fn high_numeric_signed_unsigned_mismatch_fail() {
    check_fail(
        "examples/high_level/phase2/negative/numeric_signed_unsigned_mismatch_fail.n",
        "implicit narrowing",
    );
}

#[test]
fn high_numeric_bitwise_ok() {
    check_ok("examples/high_level/phase2/numeric_bitwise_shift_ok.n");
}

#[test]
fn high_numeric_bitwise_bool_fail() {
    check_fail(
        "examples/high_level/phase2/negative/numeric_bitwise_bool_fail.n",
        "bitwise op requires integer-like operands",
    );
}

#[test]
fn high_numeric_shift_ambiguous_fail() {
    check_fail(
        "examples/high_level/phase2/negative/numeric_shift_ambiguous_fail.n",
        "shift op requires integer-like operands",
    );
}

#[test]
fn high_null_union_assign_ok() {
    check_ok("examples/high_level/phase2/null_union_ok.n");
}

#[test]
fn high_raw_null_deref_is_allowed() {
    check_ok("examples/high_level/phase2/negative/null_deref_fail.n");
}

#[test]
fn high_ptr_nullable_ok() {
    check_ok("examples/high_level/phase2/null_union_ok.n");
}

#[test]
fn high_union_int_null_ok() {
    check_ok("examples/high_level/phase2/null_union_ok.n");
}

#[test]
fn high_union_int_str_assign_ok() {
    check_ok("examples/high_level/phase2/union_assign_ok.n");
}

#[test]
fn high_union_operation_without_narrow_fail() {
    check_fail(
        "examples/high_level/phase2/negative/union_operation_without_narrow_fail.n",
        "binary op requires numeric or string-compatible operands",
    );
}

#[test]
fn high_union_pattern_match_deferred() {
    let json = types_json();
    assert_eq!(json["no_scope_phase2"]["pattern_matching"], "not_started");
}

#[test]
fn high_type_alias_policy_ok() {
    check_ok("examples/high_level/phase2/pointer_alias_policy_ok.n");
    let json = types_json();
    assert_eq!(
        json["type_aliases"]["nominal_type_alias_declarations"],
        "not_started"
    );
}

#[test]
fn high_arrays_vectors_ok() {
    check_ok("examples/high_level/phase2/arrays_vectors_ok.n");
}

#[test]
fn high_list_region_ok() {
    check_ok("examples/high_level/phase2/list_region_ok.n");
}

#[test]
fn high_list_element_type_fail() {
    check_fail(
        "examples/high_level/phase2/negative/list_element_type_fail.n",
        "incompatible element types",
    );
}

#[test]
fn high_ptr_addr_ok() {
    check_ok("examples/high_level/phase2/ptr_addr_ok.n");
}

#[test]
fn high_function_signature_ok() {
    check_ok("examples/high_level/phase2/function_signature_ok.n");
}

#[test]
fn high_function_signature_mismatch_fail() {
    check_fail(
        "examples/high_level/phase2/negative/function_signature_mismatch_fail.n",
        "no matching function",
    );
}

#[test]
fn high_casts_strict_check_ok() {
    check_ok("examples/high_level/phase2/strict_check_ok.n");
}

#[test]
fn high_inference_collection_ok() {
    check_ok("examples/high_level/phase2/inference_collection_ok.n");
}

#[test]
fn high_phase2_does_not_start_phase3_plus() {
    let json = types_json();
    assert_eq!(json["no_scope_phase2"]["generics_complete"], "not_started");
    assert_eq!(
        json["no_scope_phase2"]["traits_interfaces_contracts_dispatch"],
        "not_started"
    );
    assert_eq!(json["no_scope_phase2"]["closures"], "not_started");
    assert_eq!(json["no_scope_phase2"]["async_streams"], "not_started");
}
