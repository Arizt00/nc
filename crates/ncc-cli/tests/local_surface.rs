use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label}\n{rendered}");
    rendered
}

fn assert_failure_contains(out: Output, label: &str, needle: &str) {
    let rendered = combined(&out);
    assert!(
        !out.status.success(),
        "{label} should fail but passed\n{rendered}"
    );
    assert!(
        rendered.contains(needle),
        "{label} must contain {needle:?}\n{rendered}"
    );
}

fn example(rel: &str) -> String {
    let root = workspace_root();
    let direct = root.join(rel);
    if direct.exists() {
        return direct.to_string_lossy().to_string();
    }
    let archived = root.join("_archive").join("old_examples").join(
        rel.strip_prefix("examples/")
            .or_else(|| rel.strip_prefix("examples\\"))
            .unwrap_or(rel),
    );
    if archived.exists() {
        return archived.to_string_lossy().to_string();
    }
    direct.to_string_lossy().to_string()
}

fn assert_exists(rel: &str) {
    assert!(
        Path::new(&example(rel)).exists(),
        "fixture must exist: {rel}"
    );
}

#[test]
fn local_surface_doctor_closed() {
    let out = run_ncc(&["local", "doctor", "--json"]);
    let stdout = assert_success(out, "local doctor");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("doctor json");
    assert_eq!(json["status"], "closed");
    assert_eq!(json["local_section"], "official");
    assert_eq!(json["stc_legacy"], "preserved");
    assert_eq!(json["mode_gates"]["machine"], "forbidden");
    assert_eq!(json["mode_gates"]["basal_nb"], "not_touched");
}

#[test]
fn local_manifest_documents_fixtures() {
    let out = run_ncc(&["local", "manifest", "--json"]);
    let stdout = assert_success(out, "local manifest");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("manifest json");
    assert_eq!(json["official_section"], "local:");
    assert_eq!(json["legacy_section"], "stc:");
    assert!(json["decl_kinds"]
        .as_array()
        .expect("decl kinds")
        .iter()
        .any(|item| item.as_str() == Some("type_alias")));
}

#[test]
fn local_positive_examples_pass_check() {
    let positives = [
        "examples/local/local_basic_state.n",
        "examples/local/local_const_values.n",
        "examples/local/local_type_alias.n",
        "examples/local/local_type_record.n",
        "examples/local/local_array_with_const_len.n",
        "examples/local/local_runtime_region_ptr.n",
        "examples/local/local_visibility.n",
        "examples/local/local_system_state_policy.n",
        "examples/local/local_macro_const_type_integration.n",
        "examples/local/stc_legacy_still_works.n",
    ];
    for rel in positives {
        assert_exists(rel);
        let path = example(rel);
        assert_success(run_ncc(&["check", &path]), rel);
    }
}

#[test]
fn local_ast_and_ir_include_local_surface() {
    let path = example("examples/local/local_type_alias.n");
    let ast = assert_success(run_ncc(&["emit-ast", &path]), "emit ast");
    assert!(ast.contains("\"Local\""), "{ast}");
    let ir = assert_success(run_ncc(&["emit-ir", &path]), "emit ir");
    assert!(ir.contains("localTypeAlias"), "{ir}");
}

#[test]
fn local_negative_examples_fail_with_official_diagnostics() {
    let negatives = [
        (
            "examples/local/negative/local_duplicate_symbol_fail.n",
            "E_LOCAL_DUP_SYMBOL_001",
        ),
        (
            "examples/local/negative/local_duplicate_type_fail.n",
            "E_LOCAL_TYPE_DUP_001",
        ),
        (
            "examples/local/negative/local_const_mutation_fail.n",
            "E_LOCAL_CONST_MUTATION_001",
        ),
        (
            "examples/local/negative/local_unknown_type_fail.n",
            "E_LOCAL_TYPE_UNKNOWN_001",
        ),
        (
            "examples/local/negative/local_any_leak_fail.n",
            "E_LOCAL_TYPE_ANY_LEAK_001",
        ),
        (
            "examples/local/negative/local_type_alias_cycle_fail.n",
            "E_LOCAL_TYPE_RECURSIVE_001",
        ),
        (
            "examples/local/negative/local_public_private_type_fail.n",
            "E_LOCAL_PUBLIC_PRIVATE_TYPE_001",
        ),
    ];
    for (rel, diag) in negatives {
        assert_exists(rel);
        let path = example(rel);
        assert_failure_contains(run_ncc(&["check", &path]), rel, diag);
    }
}

#[test]
fn local_forbidden_modes_fail_with_local_gate() {
    let negatives = [
        "examples/local/negative/local_forbidden_in_machine_fail.nm",
        "examples/local/negative/local_forbidden_in_primitive_fail.ncp",
        "examples/local/negative/local_forbidden_in_kernel_fail.n",
    ];
    for rel in negatives {
        assert_exists(rel);
        let path = example(rel);
        assert_failure_contains(
            run_ncc(&["check", &path]),
            rel,
            "E_LOCAL_MODE_FORBIDDEN_001",
        );
    }
}
