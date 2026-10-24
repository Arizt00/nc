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
    assert!(!out.status.success(), "{} unexpectedly passed", rel);
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
fn high_level_phase12_doctor_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["closures_lambdas_captures"], "closed");
    assert_eq!(json["phase12_limited_reflection_introspection"], "closed");
    assert_eq!(json["type_metadata"], "closed");
    assert_eq!(json["symbol_metadata"], "closed");
    assert_eq!(json["visibility_aware"], "closed");
    assert_eq!(json["eval"], "forbidden");
    assert_eq!(json["compile_time_evaluation"], "closed");
    assert_eq!(json["macros"], "closed");
    assert_eq!(json["phase13_compile_time_evaluation_model"], "closed");
    assert_eq!(json["const_eval"], "closed_pure");
    assert_eq!(json["comptime_unsafe"], "closed_explicit_allow_audited");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase12_reflection_command_ok() {
    let json = command_json(&["high-level", "reflection", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["mode"], "read_only_visibility_aware");
    assert_eq!(json["dynamic_eval"], "forbidden");
}

#[test]
fn high_level_phase12_introspection_command_ok() {
    let json = command_json(&["high-level", "introspection", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["api_surface_introspection"], "closed");
}

#[test]
fn high_level_phase12_metadata_command_ok() {
    let json = command_json(&["highlevel", "metadata", "--json"]);
    assert_eq!(json["status"], "closed");
    assert_eq!(json["metadata_model"]["TypeInfo<T>"], "closed_limited");
}

ok_example_test!(
    high_level_phase12_typeinfo_basic_ok,
    "examples/high_level/phase12/01_typeinfo_basic.n"
);
ok_example_test!(
    high_level_phase12_typeinfo_fields_ok,
    "examples/high_level/phase12/02_typeinfo_struct_fields_public.n"
);
ok_example_test!(
    high_level_phase12_enum_variants_ok,
    "examples/high_level/phase12/03_typeinfo_enum_variants.n"
);
ok_example_test!(
    high_level_phase12_symbolinfo_ok,
    "examples/high_level/phase12/06_symbolinfo_function_signature.n"
);
ok_example_test!(
    high_level_phase12_moduleinfo_ok,
    "examples/high_level/phase12/08_moduleinfo_basic.n"
);
ok_example_test!(
    high_level_phase12_api_surface_ok,
    "examples/high_level/phase12/10_api_surface_basic.n"
);
ok_example_test!(
    high_level_phase12_traitinfo_ok,
    "examples/high_level/phase12/11_traitinfo_basic.n"
);
ok_example_test!(
    high_level_phase12_closureinfo_ok,
    "examples/high_level/phase12/14_closureinfo_signature.n"
);
ok_example_test!(
    high_level_phase12_visibility_filter_ok,
    "examples/high_level/phase12/15_visibility_filter_public.n"
);

fail_example_test!(
    high_level_phase12_typeinfo_unknown_fail,
    "examples/high_level/phase12/negative/01_typeinfo_unknown_fail.n",
    "E_HIGH_TYPEINFO_UNKNOWN_001"
);
fail_example_test!(
    high_level_phase12_typeinfo_any_fail,
    "examples/high_level/phase12/negative/02_typeinfo_any_fail.n",
    "E_HIGH_TYPEINFO_ANY_001"
);
fail_example_test!(
    high_level_phase12_unresolved_generic_fail,
    "examples/high_level/phase12/negative/03_typeinfo_unresolved_generic_fail.n",
    "E_HIGH_TYPEINFO_UNRESOLVED_GENERIC_001"
);
fail_example_test!(
    high_level_phase12_symbolinfo_private_fail,
    "examples/high_level/phase12/negative/05_symbolinfo_private_fail.n",
    "E_HIGH_SYMBOLINFO_PRIVATE_001"
);
fail_example_test!(
    high_level_phase12_reflect_private_field_fail,
    "examples/high_level/phase12/negative/06_reflect_private_field_fail.n",
    "E_HIGH_VISIBILITY_PRIVATE_LEAK_001"
);
fail_example_test!(
    high_level_phase12_eval_forbidden_fail,
    "examples/high_level/phase12/negative/10_eval_forbidden_fail.n",
    "E_HIGH_REFLECTION_EVAL_FORBIDDEN_001"
);
fail_example_test!(
    high_level_phase12_reflect_invoke_deferred_fail,
    "examples/high_level/phase12/negative/11_reflect_invoke_deferred_fail.n",
    "E_HIGH_REFLECT_INVOKE_DEFERRED_001"
);
fail_example_test!(
    high_level_phase12_set_field_forbidden_fail,
    "examples/high_level/phase12/negative/12_set_field_forbidden_fail.n",
    "E_HIGH_REFLECTION_MUTATION_001"
);
fail_example_test!(
    high_level_phase12_capture_leak_fail,
    "examples/high_level/phase12/negative/09_closureinfo_capture_leak_fail.n",
    "E_HIGH_CLOSURE_CAPTURE_LEAK_001"
);
fail_example_test!(
    high_level_phase12_metadata_any_leak_fail,
    "examples/high_level/phase12/negative/16_metadata_any_leak_fail.n",
    "E_HIGH_METADATA_ANY_001"
);

#[test]
fn high_level_phase12_sweep_ok() {
    let sweep =
        workspace_root().join("reports/high_level/phase12/run_phase12_limited_reflection_sweep.py");
    assert!(sweep.exists(), "{} must exist", sweep.display());
    let out = Command::new("python3")
        .arg(sweep)
        .current_dir(workspace_root())
        .env("NCC_BIN", ncc_bin())
        .output()
        .expect("phase12 sweep must run");
    assert!(out.status.success(), "{}", combined_output(&out));
}
