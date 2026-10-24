use std::fs;
use std::path::{Path, PathBuf};
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
fn high_level_phase9_status_exists() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase8_iterators_ranges_collections"], "closed");
    assert_eq!(json["phase9_async_effects_concurrency"], "closed");
    assert_eq!(json["phase9_plus"], "phase9_closed");
    assert_eq!(json["phase10_plus"], "phase10_closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase9_doctor_gate_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["async_functions"], "closed");
    assert_eq!(json["await"], "closed");
    assert_eq!(json["future"], "closed");
    assert_eq!(json["task"], "closed");
    assert_eq!(json["effects"], "closed");
    assert_eq!(json["effect_checking"], "closed");
    assert_eq!(json["spawn_join"], "closed");
    assert_eq!(json["channels"], "closed_limited");
    assert_eq!(json["backpressure"], "closed");
    assert_eq!(json["race_diagnostics"], "closed");
    assert_eq!(json["async_runtime"], "native_state_machine_closed");
    assert_eq!(json["async_streams"], "closed_typed");
    assert_eq!(json["generators"], "closed_typed");
    assert_eq!(json["closures_lambdas"], "closed_executable");
}

#[test]
fn high_level_phase9_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "async", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "effects", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "concurrency", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "channels", "--json"])["status"],
        "closed_limited"
    );
    assert_eq!(
        command_json(&["highlevel", "async", "--json"])["status"],
        "closed"
    );
}

ok_example_test!(
    high_level_phase9_async_basic_ok,
    "examples/high_level/phase9/01_async_basic.n"
);
ok_example_test!(
    high_level_phase9_await_future_basic_ok,
    "examples/high_level/phase9/02_await_future_basic.n"
);
ok_example_test!(
    high_level_phase9_async_result_match_ok,
    "examples/high_level/phase9/03_async_result_match.n"
);
ok_example_test!(
    high_level_phase9_async_option_match_ok,
    "examples/high_level/phase9/04_async_option_match.n"
);
ok_example_test!(
    high_level_phase9_spawn_join_basic_ok,
    "examples/high_level/phase9/05_spawn_join_basic.n"
);
ok_example_test!(
    high_level_phase9_task_result_basic_ok,
    "examples/high_level/phase9/06_task_result_basic.n"
);
ok_example_test!(
    high_level_phase9_effect_annotation_basic_ok,
    "examples/high_level/phase9/07_effect_annotation_basic.n"
);
ok_example_test!(
    high_level_phase9_effect_propagation_ok,
    "examples/high_level/phase9/08_effect_propagation_ok.n"
);
ok_example_test!(
    high_level_phase9_pure_function_no_effect_ok,
    "examples/high_level/phase9/09_pure_function_no_effect_ok.n"
);
ok_example_test!(
    high_level_phase9_channel_bounded_send_recv_ok,
    "examples/high_level/phase9/10_channel_bounded_send_recv.n"
);
ok_example_test!(
    high_level_phase9_channel_try_send_result_ok,
    "examples/high_level/phase9/11_channel_try_send_result.n"
);
ok_example_test!(
    high_level_phase9_channel_recv_option_ok,
    "examples/high_level/phase9/12_channel_recv_option.n"
);
ok_example_test!(
    high_level_phase9_shared_immutable_task_ok,
    "examples/high_level/phase9/13_shared_immutable_task_ok.n"
);
ok_example_test!(
    high_level_phase9_sync_channel_shared_ok,
    "examples/high_level/phase9/14_sync_channel_shared_ok.n"
);
ok_example_test!(
    high_level_phase9_async_for_range_semantic_ok,
    "examples/high_level/phase9/15_async_for_range_semantic_ok.n"
);
ok_example_test!(
    high_level_phase9_concurrency_graph_report_ok,
    "examples/high_level/phase9/16_concurrency_graph_report.n"
);

fail_example_test!(
    high_level_phase9_await_non_future_fail,
    "examples/high_level/phase9/negative/01_await_non_future_fail.n",
    "E_HIGH_AWAIT_NON_FUTURE_001"
);
fail_example_test!(
    high_level_phase9_await_wrong_context_fail,
    "examples/high_level/phase9/negative/02_await_wrong_context_fail.n",
    "E_HIGH_AWAIT_CONTEXT_001"
);
fail_example_test!(
    high_level_phase9_async_return_mismatch_fail,
    "examples/high_level/phase9/negative/03_async_return_mismatch_fail.n",
    "E_HIGH_ASYNC_RETURN_001"
);
fail_example_test!(
    high_level_phase9_spawn_non_async_fail,
    "examples/high_level/phase9/negative/04_spawn_non_async_fail.n",
    "E_HIGH_SPAWN_NON_ASYNC_001"
);
fail_example_test!(
    high_level_phase9_join_non_task_fail,
    "examples/high_level/phase9/negative/05_join_non_task_fail.n",
    "E_HIGH_JOIN_NON_TASK_001"
);
fail_example_test!(
    high_level_phase9_missing_effect_fail,
    "examples/high_level/phase9/negative/06_missing_effect_fail.n",
    "E_HIGH_EFFECT_MISSING_001"
);
fail_example_test!(
    high_level_phase9_effect_not_allowed_fail,
    "examples/high_level/phase9/negative/07_effect_not_allowed_fail.n",
    "E_HIGH_EFFECT_NOT_ALLOWED_001"
);
fail_example_test!(
    high_level_phase9_effect_erasure_fail,
    "examples/high_level/phase9/negative/08_effect_erasure_fail.n",
    "E_HIGH_EFFECT_ERASURE_001"
);
fail_example_test!(
    high_level_phase9_pure_calls_effectful_fail,
    "examples/high_level/phase9/negative/09_pure_calls_effectful_fail.n",
    "E_HIGH_EFFECT_PURE_VIOLATION_001"
);
fail_example_test!(
    high_level_phase9_unknown_effect_fail,
    "examples/high_level/phase9/negative/10_unknown_effect_fail.n",
    "E_HIGH_EFFECT_UNKNOWN_001"
);
fail_example_test!(
    high_level_phase9_channel_unbounded_fail,
    "examples/high_level/phase9/negative/11_channel_unbounded_fail.n",
    "E_HIGH_CHANNEL_UNBOUNDED_001"
);
fail_example_test!(
    high_level_phase9_channel_capacity_zero_fail,
    "examples/high_level/phase9/negative/12_channel_capacity_zero_fail.n",
    "E_HIGH_CHANNEL_CAPACITY_001"
);
fail_example_test!(
    high_level_phase9_channel_item_any_fail,
    "examples/high_level/phase9/negative/13_channel_item_any_fail.n",
    "E_HIGH_CHANNEL_ITEM_ANY_001"
);
fail_example_test!(
    high_level_phase9_channel_no_backpressure_fail,
    "examples/high_level/phase9/negative/14_channel_no_backpressure_fail.n",
    "E_HIGH_CHANNEL_BACKPRESSURE_001"
);
fail_example_test!(
    high_level_phase9_shared_mut_race_fail,
    "examples/high_level/phase9/negative/15_shared_mut_race_fail.n",
    "E_HIGH_RACE_SHARED_MUT_001"
);
fail_example_test!(
    high_level_phase9_alias_race_fail,
    "examples/high_level/phase9/negative/16_alias_race_fail.n",
    "E_HIGH_RACE_SHARED_MUT_001"
);
fail_example_test!(
    high_level_phase9_obsolete_async_stream_callable_fail,
    "examples/high_level/phase9/negative/17_async_stream_deferred_fail.n",
    "no matching function 'asyncStream'"
);
fail_example_test!(
    high_level_phase9_yield_outside_generator_fail,
    "examples/high_level/phase9/negative/18_generator_yield_deferred_fail.n",
    "E_HIGH_YIELD_CONTEXT_001"
);
fail_example_test!(
    high_level_phase9_async_closure_deferred_fail,
    "examples/high_level/phase9/negative/19_async_closure_deferred_fail.n",
    "E_HIGH_OBSOLETE_CALLABLE_SPELLING_001"
);
fail_example_test!(
    high_level_phase9_any_leak_async_fail,
    "examples/high_level/phase9/negative/20_any_leak_async_fail.n",
    "E_HIGH_ANY_LEAK_001"
);

#[test]
fn high_level_phase9_no_fake_async_or_concurrency() {
    let roots = [
        workspace_root().join("crates/ncc-parser/src/lib.rs"),
        workspace_root().join("crates/ncc-sema/src/lib.rs"),
        workspace_root().join("crates/ncc-cli/src/lib.rs"),
        workspace_root().join("examples/high_level/phase9"),
    ];
    let fake = "fake ";
    let needles = vec![
        format!("{fake}async"),
        format!("{fake}effect"),
        format!("{fake}concurrency"),
        format!("{fake}channel"),
        format!("{fake}task"),
        format!("{}!", "unimplemented"),
        format!("{}!", "todo"),
    ];
    for root in roots {
        scan_for_forbidden(&root, &needles);
    }
}

#[test]
fn high_level_phase9_does_not_start_closures_plus() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase10_plus"], "phase10_closed");
    assert_eq!(json["closures_plus"], "closures_closed");
    assert_eq!(json["closures_lambdas"], "closed_executable");
    assert_eq!(json["hkt"], "deferred");
}

#[test]
fn high_level_phase8_examples_still_pass() {
    check_ok("examples/high_level/phase8/16_full_phase8_surface.n");
    check_ok("examples/high_level/phase8/01_iterator_next_option.n");
}

#[test]
fn machine_asm_complete_total_still_true() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json output");
    assert_eq!(json["asm_complete_total"], true);
}

fn scan_for_forbidden(path: &Path, needles: &[String]) {
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("scan directory") {
            let entry = entry.expect("directory entry");
            scan_for_forbidden(&entry.path(), needles);
        }
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let lower = text.to_ascii_lowercase();
    for needle in needles {
        assert!(
            !lower.contains(&needle.to_ascii_lowercase()),
            "{} contains forbidden marker '{}'",
            path.display(),
            needle
        );
    }
}
