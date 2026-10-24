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
fn high_level_phase8_doctor_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["phase8_iterators_ranges_collections"], "closed");
    assert_eq!(json["iterator_protocol"], "closed");
    assert_eq!(json["iterable_protocol"], "closed");
    assert_eq!(json["collection_protocol"], "closed");
    assert_eq!(json["ranges"], "closed_finite_integer");
    assert_eq!(json["next_option"], "closed");
    assert_eq!(json["iterator_adapters"], "closed_typed_closures");
    assert_eq!(json["closures_lambdas"], "closed_executable");
    assert_eq!(json["async_effects"], "closed");
    assert_eq!(json["phase9_plus"], "phase9_closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase8_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "iterators", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "ranges", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "collections", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["highlevel", "iterators", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_phase8_positive_examples_ok() {
    for rel in [
        "examples/high_level/phase8/01_iterator_next_option.n",
        "examples/high_level/phase8/02_iterable_iter_protocol.n",
        "examples/high_level/phase8/03_collection_len_is_empty.n",
        "examples/high_level/phase8/04_collection_get.n",
        "examples/high_level/phase8/05_list_for_in.n",
        "examples/high_level/phase8/06_array_for_in.n",
        "examples/high_level/phase8/07_range_exclusive.n",
        "examples/high_level/phase8/08_range_inclusive_step.n",
        "examples/high_level/phase8/09_list_next_option.n",
        "examples/high_level/phase8/10_array_iter_next.n",
        "examples/high_level/phase8/11_mem_region_for_in.n",
        "examples/high_level/phase8/12_mem_region_next_option.n",
        "examples/high_level/phase8/13_generic_iterable_sum.n",
        "examples/high_level/phase8/14_collection_iter.n",
        "examples/high_level/phase8/15_for_in_option_result_payload.n",
        "examples/high_level/phase8/16_full_phase8_surface.n",
    ] {
        check_ok(rel);
    }
}

#[test]
fn high_level_phase8_negative_examples_fail() {
    for (rel, needle) in [
        (
            "examples/high_level/phase8/negative/01_iterator_wrong_arity_fail.n",
            "E_HIGH_ITERATOR_PROTOCOL_ARITY_001",
        ),
        (
            "examples/high_level/phase8/negative/02_iterator_any_fail.n",
            "E_HIGH_ANY_LEAK_001",
        ),
        (
            "examples/high_level/phase8/negative/03_collection_any_fail.n",
            "E_HIGH_ANY_LEAK_001",
        ),
        (
            "examples/high_level/phase8/negative/04_for_in_non_iterable_fail.n",
            "E_HIGH_FOR_IN_ITERABLE_001",
        ),
        (
            "examples/high_level/phase8/negative/05_range_string_bound_fail.n",
            "E_HIGH_RANGE_BOUND_001",
        ),
        (
            "examples/high_level/phase8/negative/06_range_step_string_fail.n",
            "E_HIGH_RANGE_STEP_001",
        ),
        (
            "examples/high_level/phase8/negative/07_range_step_zero_fail.n",
            "E_HIGH_RANGE_STEP_ZERO_001",
        ),
        (
            "examples/high_level/phase8/negative/08_for_in_mutation_fail.n",
            "E_HIGH_FOR_IN_MUTATION_001",
        ),
        (
            "examples/high_level/phase8/negative/09_iterator_adapter_map_deferred_fail.n",
            "E_HIGH_ITERATOR_ADAPTER_DEFERRED_001",
        ),
        (
            "examples/high_level/phase8/negative/10_infinite_stream_deferred_fail.n",
            "E_HIGH_STREAM_UNBOUNDED_DEFERRED_001",
        ),
        (
            "examples/high_level/phase8/negative/11_closure_lambda_deferred_fail.n",
            "E_HIGH_OBSOLETE_CALLABLE_SPELLING_001",
        ),
        (
            "examples/high_level/phase8/negative/12_async_effect_deferred_fail.n",
            "E_HIGH_ASYNC_FN_001",
        ),
        (
            "examples/high_level/phase8/negative/13_next_wrong_args_fail.n",
            "E_HIGH_ITERATOR_NEXT_001",
        ),
        (
            "examples/high_level/phase8/negative/14_collection_get_string_index_fail.n",
            "E_HIGH_COLLECTION_GET_001",
        ),
        (
            "examples/high_level/phase8/negative/15_collection_len_arg_fail.n",
            "E_HIGH_COLLECTION_LEN_001",
        ),
        (
            "examples/high_level/phase8/negative/16_for_in_any_item_fail.n",
            "E_HIGH_ANY_LEAK_001",
        ),
        (
            "examples/high_level/phase8/negative/17_collection_wrong_arity_fail.n",
            "E_HIGH_ITERATOR_PROTOCOL_ARITY_001",
        ),
        (
            "examples/high_level/phase8/negative/18_unbounded_range_deferred_fail.n",
            "E_HIGH_STREAM_UNBOUNDED_DEFERRED_001",
        ),
    ] {
        check_fail(rel, needle);
    }
}

#[test]
fn high_level_phase8_regressions_ok() {
    check_ok("examples/high_level/phase7/16_error_conversion_explicit.n");
    check_ok("examples/high_level/phase6/16_full_phase6_surface.n");
    check_ok("examples/high_level/phase5/05_associated_type_basic.n");
    let machine = command_json(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert_eq!(machine["asm_complete_total"], true);
}

#[test]
fn high_level_phase8_does_not_start_phase9_plus() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase9_plus"], "phase9_closed");
    assert_eq!(json["async_effects"], "closed");
    assert_eq!(json["concurrency"], "closed");
    assert_eq!(json["closures_lambdas"], "closed_executable");
}
