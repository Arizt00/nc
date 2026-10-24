use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
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

#[test]
fn nc_unified_doctor_root_scope_is_normal_high_level_only() {
    let json = command_json(&["doctor", "--json"]);
    assert_eq!(json["schema"], "nc.doctor.unified.normal_high_level.v1");
    assert_eq!(json["scope"], "normal_nc_high_level_only");
    assert_eq!(json["status"], "ok");
    assert_eq!(
        json["semantic_changes"],
        "pointer_model_officialized_normal_high_level"
    );
    assert_eq!(
        json["features_added"],
        "pointer_model_cli_diagnostics_examples_docs"
    );

    for key in [
        "normal_nc",
        "local",
        "high_level",
        "runtime",
        "compiler",
        "pointer_model",
    ] {
        assert!(
            json["included_doctors"].get(key).is_some(),
            "included doctor {key} missing"
        );
    }
}

#[test]
fn nc_unified_doctor_excludes_low_level_release_authorities() {
    let json = command_json(&["doctor", "--json"]);
    for key in ["basal", "machine", "primitive", "kernel"] {
        let item = &json["excluded_doctors"][key];
        assert_eq!(item["grouped"], false, "{key} must not be grouped");
    }

    assert!(json.get("basal_doctor_status").is_none());
    assert!(json.get("machine_native_doctor_status").is_none());
    assert_eq!(json["guarantees"]["does_not_run_basal_doctor"], true);
    assert_eq!(json["guarantees"]["does_not_run_machine_doctor"], true);
    assert_eq!(json["guarantees"]["does_not_run_primitive_doctor"], true);
    assert_eq!(json["guarantees"]["does_not_claim_kernel_doctor"], true);
}

#[test]
fn high_level_unified_doctor_uses_same_exclusion_policy() {
    let json = command_json(&["high-level", "unified-doctor", "--json"]);
    assert_eq!(
        json["doctor_unification_scope"],
        "normal_nc_high_level_only"
    );
    assert_eq!(
        json["machine_dependency"],
        "external_foundation_not_grouped"
    );
    assert_eq!(json["basal_dependency"], "external_foundation_not_grouped");
    assert!(json["normal_nc_doctor_status"].is_object());
    assert!(json["local_doctor_status"].is_object());
    assert!(json.get("machine_native_doctor_status").is_none());
    assert!(json.get("basal_doctor_status").is_none());
}

#[test]
fn nc_unified_doctor_text_and_help_are_available() {
    let out = run_ncc(&["doctor"]);
    assert!(out.status.success(), "{}", combined_output(&out));
    let text = combined_output(&out);
    assert!(text.contains("ncc doctor: OK"));
    assert!(text.contains("scope: normal_nc_high_level_only"));
    assert!(text.contains("excluded_by_design: basal, machine, primitive, kernel"));

    let help = run_ncc(&["doctor", "--help"]);
    assert!(help.status.success(), "{}", combined_output(&help));
    let help_text = combined_output(&help);
    assert!(help_text.contains("Usage: doctor [--json]"));
    assert!(help_text.contains("Excluded by design"));
}
