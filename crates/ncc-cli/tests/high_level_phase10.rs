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
fn high_level_phase10_doctor_ok() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(
        json["status"],
        "high_level_phase15_risk_authority_expert_policy_closed"
    );
    assert_eq!(json["phase9_async_effects_concurrency"], "closed");
    assert_eq!(json["phase10_modules_packages_api_surface"], "closed");
    assert_eq!(json["modules"], "closed");
    assert_eq!(json["packages"], "closed");
    assert_eq!(json["imports"], "closed");
    assert_eq!(json["exports"], "closed");
    assert_eq!(json["visibility"], "closed");
    assert_eq!(json["api_surface"], "closed");
    assert_eq!(json["cycle_detection"], "closed");
    assert_eq!(json["registry"], "not_started");
    assert_eq!(json["ghor"], "separate_tooling");
    assert_eq!(json["closures_plus"], "closures_closed");
    assert_eq!(json["next_phase"], "High Level Fase 16");
}

#[test]
fn high_level_phase10_commands_ok() {
    assert_eq!(
        command_json(&["high-level", "modules", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "packages", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "exports", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["high-level", "api-surface", "--json"])["status"],
        "closed"
    );
    assert_eq!(
        command_json(&["highlevel", "api-surface", "--json"])["status"],
        "closed"
    );
}

ok_example_test!(
    high_level_phase10_module_basic_ok,
    "examples/high_level/phase10/01_module_basic.n"
);
ok_example_test!(
    high_level_phase10_package_basic_ok,
    "examples/high_level/phase10/02_package_basic.n"
);
ok_example_test!(
    high_level_phase10_use_exact_ok,
    "examples/high_level/phase10/03_use_exact_import.n"
);
ok_example_test!(
    high_level_phase10_use_alias_ok,
    "examples/high_level/phase10/04_use_alias_import.n"
);
ok_example_test!(
    high_level_phase10_use_wildcard_ok,
    "examples/high_level/phase10/05_use_wildcard_allowed.n"
);
ok_example_test!(
    high_level_phase10_export_function_ok,
    "examples/high_level/phase10/06_export_function.n"
);
ok_example_test!(
    high_level_phase10_export_type_enum_trait_ok,
    "examples/high_level/phase10/07_export_type_enum_trait.n"
);
ok_example_test!(
    high_level_phase10_reexport_ok,
    "examples/high_level/phase10/08_reexport_basic.n"
);
ok_example_test!(
    high_level_phase10_visibility_ok,
    "examples/high_level/phase10/09_public_private_visibility.n"
);
ok_example_test!(
    high_level_phase10_internal_package_visibility_ok,
    "examples/high_level/phase10/10_internal_package_visibility.n"
);
ok_example_test!(
    high_level_phase10_api_surface_ok,
    "examples/high_level/phase10/11_api_surface_basic.n"
);
ok_example_test!(
    high_level_phase10_api_surface_generics_ok,
    "examples/high_level/phase10/12_api_surface_generics_bounds.n"
);
ok_example_test!(
    high_level_phase10_api_surface_async_effects_ok,
    "examples/high_level/phase10/13_api_surface_async_effects.n"
);
ok_example_test!(
    high_level_phase10_api_surface_result_error_ok,
    "examples/high_level/phase10/14_api_surface_result_error_type.n"
);
ok_example_test!(
    high_level_phase10_module_graph_ok,
    "examples/high_level/phase10/15_module_graph_multi_file.n"
);
ok_example_test!(
    high_level_phase10_deprecated_experimental_report_ok,
    "examples/high_level/phase10/16_deprecated_experimental_report.n"
);

fail_example_test!(
    high_level_phase10_module_duplicate_fail,
    "examples/high_level/phase10/negative/01_module_duplicate_fail.n",
    "E_HIGH_MODULE_DUP_001"
);
fail_example_test!(
    high_level_phase10_module_invalid_path_fail,
    "examples/high_level/phase10/negative/02_module_invalid_path_fail.n",
    "E_HIGH_MODULE_PATH_001"
);
fail_example_test!(
    high_level_phase10_module_cycle_fail,
    "examples/high_level/phase10/negative/03_module_cycle_fail.n",
    "E_HIGH_MODULE_CYCLE_001"
);
fail_example_test!(
    high_level_phase10_package_invalid_version_fail,
    "examples/high_level/phase10/negative/04_package_invalid_version_fail.n",
    "E_HIGH_PACKAGE_VERSION_001"
);
fail_example_test!(
    high_level_phase10_use_unresolved_fail,
    "examples/high_level/phase10/negative/05_use_unresolved_fail.n",
    "E_HIGH_USE_UNRESOLVED_001"
);
fail_example_test!(
    high_level_phase10_use_ambiguous_fail,
    "examples/high_level/phase10/negative/06_use_ambiguous_fail.n",
    "E_HIGH_USE_AMBIGUOUS_001"
);
fail_example_test!(
    high_level_phase10_use_private_fail,
    "examples/high_level/phase10/negative/07_use_private_fail.n",
    "E_HIGH_USE_PRIVATE_001"
);
fail_example_test!(
    high_level_phase10_use_internal_fail,
    "examples/high_level/phase10/negative/08_use_internal_outside_package_fail.n",
    "E_HIGH_USE_INTERNAL_001"
);
fail_example_test!(
    high_level_phase10_use_alias_duplicate_fail,
    "examples/high_level/phase10/negative/09_use_alias_duplicate_fail.n",
    "E_HIGH_USE_ALIAS_DUP_001"
);
fail_example_test!(
    high_level_phase10_use_wildcard_denied_fail,
    "examples/high_level/phase10/negative/10_use_wildcard_denied_fail.n",
    "E_HIGH_USE_WILDCARD_DENIED_001"
);
fail_example_test!(
    high_level_phase10_export_unknown_fail,
    "examples/high_level/phase10/negative/11_export_unknown_fail.n",
    "E_HIGH_EXPORT_UNKNOWN_001"
);
fail_example_test!(
    high_level_phase10_export_private_fail,
    "examples/high_level/phase10/negative/12_export_private_fail.n",
    "E_HIGH_EXPORT_PRIVATE_001"
);
fail_example_test!(
    high_level_phase10_export_duplicate_fail,
    "examples/high_level/phase10/negative/13_export_duplicate_fail.n",
    "E_HIGH_EXPORT_DUP_001"
);
fail_example_test!(
    high_level_phase10_reexport_private_fail,
    "examples/high_level/phase10/negative/14_reexport_private_fail.n",
    "E_HIGH_EXPORT_REEXPORT_001"
);
fail_example_test!(
    high_level_phase10_private_type_leak_fail,
    "examples/high_level/phase10/negative/15_private_type_leak_public_api_fail.n",
    "E_HIGH_API_PRIVATE_TYPE_LEAK_001"
);
fail_example_test!(
    high_level_phase10_any_leak_fail,
    "examples/high_level/phase10/negative/16_any_leak_public_api_fail.n",
    "E_HIGH_API_ANY_LEAK_001"
);
fail_example_test!(
    high_level_phase10_public_api_unresolved_bound_fail,
    "examples/high_level/phase10/negative/17_unresolved_bound_public_api_fail.n",
    "E_HIGH_API_UNRESOLVED_BOUND_001"
);
fail_example_test!(
    high_level_phase10_async_effects_missing_fail,
    "examples/high_level/phase10/negative/18_async_effects_missing_public_api_fail.n",
    "E_HIGH_API_EFFECTS_MISSING_001"
);
fail_example_test!(
    high_level_phase10_result_error_private_type_fail,
    "examples/high_level/phase10/negative/19_result_error_private_type_fail.n",
    "E_HIGH_API_PRIVATE_TYPE_LEAK_001"
);
fail_example_test!(
    high_level_phase10_duplicate_alias_fail,
    "examples/high_level/phase10/negative/20_api_surface_unstable_duplicate_alias_fail.n",
    "E_HIGH_USE_AMBIGUOUS_001"
);

#[test]
fn high_level_phase10_sweep_ok() {
    let phase10_dir = fixture_path("examples/high_level/phase10");
    let positives = fs::read_dir(&phase10_dir)
        .expect("phase10 positive dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("n"))
        .collect::<Vec<_>>();
    assert_eq!(positives.len(), 16);
    for path in positives {
        let rel = path
            .strip_prefix(workspace_root())
            .or_else(|_| path.strip_prefix(workspace_root().join("_archive").join("old_examples")))
            .expect("workspace-relative")
            .to_str()
            .expect("utf8 path")
            .replace('\\', "/");
        check_ok(&rel);
    }

    let negatives = fs::read_dir(fixture_path("examples/high_level/phase10/negative"))
        .expect("phase10 negative dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("n"))
        .collect::<Vec<_>>();
    assert_eq!(negatives.len(), 20);
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
fn high_level_phase10_no_fake_modules_or_api() {
    let roots = [
        workspace_root().join("crates/ncc-parser/src/lib.rs"),
        workspace_root().join("crates/ncc-sema/src/lib.rs"),
        workspace_root().join("crates/ncc-cli/src/lib.rs"),
        fixture_path("examples/high_level/phase10"),
    ];
    let fake = "fake ";
    let unimplemented_marker = ["un", "implemented"].concat() + "!";
    let todo_marker = ['t', 'o', 'd', 'o'].iter().collect::<String>() + "!";
    let needles = vec![
        format!("{fake}module"),
        format!("{fake}package"),
        format!("{fake}export"),
        format!("{fake}api"),
        format!("{fake}visibility"),
        unimplemented_marker,
        todo_marker,
    ];
    for root in roots {
        scan_for_forbidden(&root, &needles);
    }
}

#[test]
fn high_level_phase10_does_not_start_closures_plus() {
    let json = command_json(&["high-level", "doctor", "--json"]);
    assert_eq!(json["phase10_plus"], "phase10_closed");
    assert_eq!(json["closures_plus"], "closures_closed");
    assert_eq!(json["closures_lambdas"], "closed_executable");
    assert_eq!(json["reflection"], "closed_limited");
}

#[test]
fn high_level_phase9_examples_still_pass() {
    check_ok("examples/high_level/phase9/16_concurrency_graph_report.n");
    check_ok("examples/high_level/phase9/01_async_basic.n");
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
