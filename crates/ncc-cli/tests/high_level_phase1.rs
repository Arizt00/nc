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

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let active = root.join(relative);
    if active.exists() {
        return active;
    }

    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }

    active
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

fn high_level_doctor_json() -> serde_json::Value {
    let out = run_ncc(&["high-level", "doctor", "--json"]);
    assert!(
        out.status.success(),
        "high-level doctor failed:\n{}",
        combined_output(&out)
    );
    serde_json::from_slice(&out.stdout).expect("high-level doctor must emit json")
}

#[test]
fn high_level_doctor_exists() {
    let out = run_ncc(&["high-level", "doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let text = combined_output(&out);
    assert!(text.contains("phase1_syntax_semantic_cleanup"));
}

#[test]
fn highlevel_doctor_alias_exists() {
    let out = run_ncc(&["highlevel", "doctor", "--json"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let text = combined_output(&out);
    assert!(text.contains("high_level_phase3_generics_complete_closed"));
    assert!(text.contains("phase1_syntax_semantic_cleanup"));
}

#[test]
fn high_level_doctor_json_ok() {
    let json = high_level_doctor_json();
    assert_eq!(json["component"], "high-level");
    assert_eq!(json["alias"], "highlevel");
    assert_eq!(json["exit_code_policy"], "closed");
    assert_eq!(json["diagnostic_policy"], "closed");
}

#[test]
fn high_level_doctor_phase1_status_ok() {
    let json = high_level_doctor_json();
    assert_eq!(json["phase0_audit"], "closed");
    assert_eq!(json["phase1_syntax_semantic_cleanup"], "closed");
    assert_eq!(
        json["compact_syntax"]["status"],
        "official_normal_n_surface"
    );
    assert_eq!(json["compact_syntax"]["file_family"], ".n");
    assert_eq!(json["advanced_expert"], "not_started");
    assert_eq!(json["phase2_type_system_advanced"], "closed");
    assert_eq!(json["phase3_generics_complete"], "closed");
}

#[test]
fn high_level_exit_policy_error_nonzero_ok() {
    let path = fixture_path("examples/l2d_import_missing_fail.n");
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        !out.status.success(),
        "source error must not exit 0:\n{}",
        combined_output(&out)
    );
}

#[test]
fn high_level_exit_policy_report_only_ok() {
    let json = high_level_doctor_json();
    assert_eq!(json["report_only"], false);
    assert_eq!(json["has_errors"], false);
    assert!(json["exit_policy"].is_object());
}

#[test]
fn high_level_negative_diagnostic_exit_nonzero_or_report_only_ok() {
    let path = fixture_path("examples/l2d_import_missing_fail.n");
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let text = combined_output(&out);
    assert!(
        !out.status.success(),
        "non-report check must exit nonzero on an error diagnostic:\n{text}"
    );
    assert!(text.contains("unresolved reference"));
}

#[test]
fn high_level_no_silent_pass_ok() {
    let path = fixture_path("examples/l2d_import_missing_fail.n");
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let text = combined_output(&out);
    assert!(
        !out.status.success() && !text.contains("OK:"),
        "negative fixture must not silently pass:\n{text}"
    );
}

#[test]
fn high_level_diagnostic_severity_policy_ok() {
    let json = high_level_doctor_json();
    assert_eq!(
        json["diagnostic_policy_summary"]["source_errors_exit_nonzero"],
        true
    );
    assert_eq!(
        json["diagnostic_policy_summary"]["silent_error_success_forbidden"],
        true
    );
}

#[test]
fn high_level_deferred_diagnostic_has_phase_ok() {
    let json = high_level_doctor_json();
    assert_eq!(json["next_phase"], "High Level Fase 16");
    assert_eq!(json["expert_profile_gates"], "documented_only");
}

#[test]
fn high_level_accepted_risk_requires_profile_ok() {
    let json = high_level_doctor_json();
    assert_eq!(json["expert_profile_gates"], "documented_only");
    assert_eq!(json["closed_without_phase8_features"], false);
    assert_eq!(json["closed_without_phase9_features"], false);
}

#[test]
fn high_level_negative_exit0_none_silent_ok() {
    let json = high_level_doctor_json();
    assert_eq!(
        json["negative_examples"]["exit0_silent_after_classification"],
        0
    );
}

#[test]
fn high_level_negative_report_only_marked_ok() {
    let json = high_level_doctor_json();
    assert_eq!(
        json["diagnostic_policy_summary"]["report_only_commands_must_set_has_errors"],
        true
    );
}

#[test]
fn high_level_syntax_baseline_documented_ok() {
    let path = workspace_root().join("docs/spec/high_level/high_level_syntax_baseline.md");
    assert!(path.exists(), "syntax baseline doc must exist");
}

#[test]
fn high_level_legacy_forms_classified_ok() {
    let path = workspace_root().join("docs/spec/high_level/high_level_syntax_baseline.md");
    let text = std::fs::read_to_string(path).expect("syntax baseline doc");
    assert!(text.contains("Official Compact Syntax"));
    assert!(text.contains("stc:"));
    assert!(text.contains("fn:"));
    assert!(text.contains("fa:"));
    assert!(text.contains("name >> type = value"));
    assert!(text.contains("name = expr"));
}

#[test]
fn high_level_official_forms_examples_ok() {
    let path = workspace_root().join("docs/spec/high_level/high_level_syntax_baseline.md");
    let text = std::fs::read_to_string(path).expect("syntax baseline doc");
    assert!(text.contains("entry[@]"));
    assert!(text.contains("ref main ->> @fn"));
}

#[test]
fn high_level_expert_gates_documented_ok() {
    let path = workspace_root().join("docs/spec/high_level/high_level_expert_profile_gates.md");
    let text = std::fs::read_to_string(path).expect("expert gates doc");
    assert!(text.contains("allowRawAccess"));
    assert!(text.contains("documented_only"));
    assert!(text.contains("deferred_to_future_phase"));
}
