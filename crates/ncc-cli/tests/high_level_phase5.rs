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

#[test]
fn high_level_phase5_doctor_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase4_interfaces_traits_contracts"], "closed");
    assert_eq!(json["phase5_dynamic_dispatch_trait_objects"], "closed");
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["dynamic_dispatch"], "closed");
    assert_eq!(json["trait_objects"], "closed");
    assert_eq!(json["associated_types"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase5_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "dyn", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "trait-objects", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "associated-types", "--json"])["status"],
        "closed"
    );
}

#[test]
fn high_level_phase5_dyn_trait_basic_ok() {
    check_ok("examples/high_level/phase5/01_dyn_trait_basic.n");
}

#[test]
fn high_level_phase5_trait_object_method_call_ok() {
    check_ok("examples/high_level/phase5/02_trait_object_method_call.n");
}

#[test]
fn high_level_phase5_static_dynamic_coexist_ok() {
    check_ok("examples/high_level/phase5/04_static_and_dynamic_dispatch_coexist.n");
}

#[test]
fn high_level_phase5_associated_type_basic_ok() {
    check_ok("examples/high_level/phase5/05_associated_type_basic.n");
}

#[test]
fn high_level_phase5_associated_type_generic_ok() {
    check_ok("examples/high_level/phase5/06_associated_type_generic_function.n");
}

#[test]
fn high_level_phase5_object_safety_ok() {
    check_ok("examples/high_level/phase5/08_object_safety_basic.n");
}

#[test]
fn high_level_phase5_vtable_layout_ok() {
    check_ok("examples/high_level/phase5/11_vtable_layout_report.n");
}

#[test]
fn high_level_phase5_no_any_leak_ok() {
    check_ok("examples/high_level/phase5/14_dyn_no_any_leak.n");
}

#[test]
fn high_level_phase5_dyn_unknown_trait_fail() {
    check_fail(
        "examples/high_level/phase5/negative/01_dyn_unknown_trait_fail.n",
        "E_HIGH_DYN_TRAIT_UNKNOWN_001",
    );
}

#[test]
fn high_level_phase5_dyn_non_trait_fail() {
    check_fail(
        "examples/high_level/phase5/negative/02_dyn_non_trait_fail.n",
        "E_HIGH_DYN_NON_TRAIT_001",
    );
}

#[test]
fn high_level_phase5_dyn_non_object_safe_fail() {
    check_fail(
        "examples/high_level/phase5/negative/03_dyn_non_object_safe_fail.n",
        "E_HIGH_DYN_OBJECT_SAFETY_001",
    );
}

#[test]
fn high_level_phase5_dyn_generic_method_fail() {
    check_fail(
        "examples/high_level/phase5/negative/04_dyn_generic_method_fail.n",
        "E_HIGH_DYN_GENERIC_METHOD_001",
    );
}

#[test]
fn high_level_phase5_associated_type_missing_fail() {
    check_fail(
        "examples/high_level/phase5/negative/07_associated_type_missing_fail.n",
        "E_HIGH_ASSOC_TYPE_MISSING_001",
    );
}

#[test]
fn high_level_phase5_associated_type_ambiguous_fail() {
    check_fail(
        "examples/high_level/phase5/negative/10_associated_type_ambiguous_fail.n",
        "E_HIGH_ASSOC_TYPE_AMBIGUOUS_001",
    );
}

#[test]
fn high_level_phase5_implicit_dyn_fail() {
    check_fail(
        "examples/high_level/phase5/negative/12_implicit_dyn_fail.n",
        "E_HIGH_DYN_IMPLICIT_001",
    );
}

#[test]
fn high_level_phase5_hkt_deferred_fail() {
    check_fail(
        "examples/high_level/phase5/negative/14_hkt_still_deferred_fail.n",
        "E_HIGH_GENERIC_HKT_DEFERRED_001",
    );
}

#[test]
fn high_level_phase5_specialization_deferred_fail() {
    check_fail(
        "examples/high_level/phase5/negative/15_specialization_still_deferred_fail.n",
        "E_HIGH_SPECIALIZATION_DEFERRED_001",
    );
}

#[test]
fn high_level_phase5_any_leak_fail() {
    check_fail(
        "examples/high_level/phase5/negative/13_any_leak_dyn_fail.n",
        "E_HIGH_ANY_LEAK_001",
    );
}

#[test]
fn high_level_phase5_sweep_ok() {
    let root = fixture_path("examples/high_level/phase5");
    let mut positives = fs::read_dir(&root)
        .expect("phase5 examples dir")
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
        .expect("phase5 negative examples dir")
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
fn high_level_phase5_does_not_start_phase6_plus() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase6_adt_pattern_matching"], "closed");
    assert_eq!(json["phase7_result_option_error_algebra"], "closed");
    assert_eq!(json["phase8_iterators_ranges_collections"], "closed");
    assert_eq!(json["phase9_plus"], "phase9_closed");
    assert_eq!(json["specialization"], "deferred");
    assert_eq!(json["hkt"], "deferred");
}

#[test]
fn machine_asm_complete_total_still_true() {
    let out = run_ncc(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json output");
    assert_eq!(json["asm_complete_total"], true);
}

#[test]
fn phase4_sectioned_base_still_passes() {
    check_ok("examples/high_level/phase4/multiple_bounds_ok.n");
    let text = fs::read_to_string(Path::new(&example(
        "examples/high_level/phase4/multiple_bounds_ok.n",
    )))
    .expect("phase4 fixture");
    assert!(text.contains("interface:"));
    assert!(text.contains("trait:"));
    assert!(text.contains("impl:"));
}
