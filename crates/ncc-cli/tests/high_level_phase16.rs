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

fn command_json(args: &[&str]) -> serde_json::Value {
    let out = run_ncc(args);
    assert!(out.status.success(), "{}", combined_output(&out));
    serde_json::from_slice(&out.stdout).expect("json output")
}

fn assert_file(rel: &str) {
    let path = workspace_root().join(rel);
    assert!(path.exists(), "{} must exist", rel);
}

fn read_file(rel: &str) -> String {
    std::fs::read_to_string(workspace_root().join(rel)).expect("phase16 file must be readable")
}

#[test]
fn high_level_phase16_doctor_status_closed() {
    let json = command_json(&["high-level", "final-doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase16_unified_surface_runtime_compiler_integration_seal_closed"
    );
    assert_eq!(json["high_level_final_surface"], "closed");
    assert_eq!(json["high_level_phase16_unification"], "closed");
    assert_eq!(json["phase16_plus"], "phase16_closed");
    assert_eq!(json["phase17_plus"], "not_started");
}

#[test]
fn high_level_final_surface_closed() {
    let json = command_json(&["high-level", "surface", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["high_level_final_surface"], "closed");
    assert_eq!(
        json["base"]["runtime_compiler_relation"],
        "backend_internal_surfaces_under_high_level"
    );
    assert_eq!(json["base"]["normal_n_syntax"], "compact_syntax_official");
    assert_eq!(
        json["compact_syntax"]["status"],
        "official_normal_n_surface"
    );
    assert_eq!(json["compact_syntax"]["preferred_for_new_files"], true);
    assert!(json["base"]["included"]
        .as_array()
        .expect("included surfaces")
        .iter()
        .any(|v| v.as_str() == Some("[@runtime]")));
}

#[test]
fn high_level_phase1_15_remain_closed() {
    let json = command_json(&["high-level", "final-doctor", "--json"]);
    assert_eq!(json["high_level_phase1_15"], "closed");
    assert_eq!(json["phase15_risk_authority_expert_policy"], "closed");
    assert_eq!(json["risk_authority"], "closed");
    assert_eq!(json["phase14_macros_codegen"], "closed");
    assert_eq!(json["compile_time_evaluation"], "closed");
}

#[test]
fn runtime_doctor_still_ok() {
    let out = run_ncc(&["runtime", "doctor"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    assert!(combined_output(&out).contains("runtime doctor: OK"));
}

#[test]
fn compiler_doctor_still_ok() {
    let out = run_ncc(&["compiler", "doctor"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    assert!(combined_output(&out).contains("compiler doctor: OK"));
}

#[test]
fn machine_native_doctor_still_ok() {
    let json = command_json(&["machine", "asm-complete", "native-doctor", "--json"]);
    assert_eq!(json["asm_complete_total"], true);
}

#[test]
fn basal_doctor_still_ok() {
    let json = command_json(&["basal", "doctor", "--json"]);
    assert_eq!(json["basal_closed"], true);
}

#[test]
fn unified_surface_manifest_exists() {
    assert_file("reports/high_level/phase16/phase16_unified_surface_manifest.json");
    assert_file("reports/high_level/phase16/phase16_unified_surface_manifest.md");
}

#[test]
fn runtime_compiler_unification_plan_exists() {
    assert_file("reports/high_level/phase16/phase16_runtime_compiler_unification_plan.json");
    assert_file("reports/high_level/phase16/phase16_runtime_compiler_unification_plan.md");
}

#[test]
fn diagnostics_consolidation_exists() {
    assert_file("reports/high_level/phase16/phase16_diagnostics_consolidation.json");
    assert_file("docs/spec/high_level/high_level_diagnostics_index.md");
}

#[test]
fn capability_map_exists() {
    assert_file("reports/high_level/phase16/phase16_capability_map.json");
    assert_file("docs/spec/high_level/high_level_capability_model.md");
}

#[test]
fn stale_docs_report_exists() {
    let text = read_file("reports/high_level/phase16/phase16_stale_docs_report.md");
    assert!(text.contains("Status: classified."));
}

#[test]
fn untracked_classification_exists() {
    let json_path =
        workspace_root().join("reports/high_level/phase16/phase16_untracked_classification.json");
    assert!(json_path.exists());
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json_path).expect("untracked json"))
            .expect("untracked json parse");
    assert_eq!(json["status"], "classified");
}

#[test]
fn no_phase17_started() {
    let json = command_json(&["high-level", "phase16-report", "--json"]);
    assert_eq!(json["phase17_plus"], "not_started");
    assert!(json["next_steps"]
        .as_array()
        .expect("next steps")
        .iter()
        .any(|v| v.as_str().unwrap_or_default().contains("explicit approval")));
}

#[test]
fn no_new_feature_claims() {
    let json = command_json(&["high-level", "phase16-report", "--json"]);
    assert_eq!(json["new_features"], "none");
    assert_eq!(json["semantic_changes"], "none");
}

#[test]
fn no_fake_closed() {
    let text = read_file("reports/high_level/phase16/phase16_final_surface_report.json");
    let forbidden_final = ["fake", "final"].join(" ");
    let forbidden_unified = ["fake", "unified"].join(" ");
    assert!(!text.contains(&forbidden_final));
    assert!(!text.contains(&forbidden_unified));
}

#[test]
fn no_stubs_placeholders_in_phase16() {
    let phase16_dir = workspace_root().join("reports/high_level/phase16");
    for entry in std::fs::read_dir(&phase16_dir).expect("phase16 report dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("phase16_search_gate.md") {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) == Some("py") {
            continue;
        }
        assert_no_incomplete_markers(&path);
    }
}

fn assert_no_incomplete_markers(path: &Path) {
    let text = std::fs::read_to_string(path).expect("phase16 artifact text");
    for marker in ["TODO", "FIXME", "unimplemented!", "todo!"] {
        assert!(
            !text.contains(marker),
            "{} contains incomplete marker {}",
            path.display(),
            marker
        );
    }
}
