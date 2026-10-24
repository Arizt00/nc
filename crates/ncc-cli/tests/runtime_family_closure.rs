use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn runtime_json(args: &[&str]) -> Value {
    let out = run_ncc(args);
    assert!(
        out.status.success(),
        "ncc {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("runtime command must emit JSON")
}

fn families() -> Vec<Value> {
    let payload = runtime_json(&["runtime", "families", "--json"]);
    payload["rows"].as_array().expect("families rows").to_vec()
}

fn family_key(row: &Value) -> &str {
    row["family_key"].as_str().expect("family_key")
}

fn import_path(row: &Value) -> &str {
    row["family"].as_str().expect("family")
}

fn docs_path(key: &str) -> PathBuf {
    workspace_root()
        .join("docs/spec/runtime/imports")
        .join(format!("{key}.md"))
}

fn example_path(key: &str) -> PathBuf {
    let root = workspace_root();
    let current = root
        .join("examples/runtime/families")
        .join(format!("{key}_basic.n"));
    if current.exists() {
        return current;
    }
    let archived = root
        .join("_archive/old_examples/runtime/families")
        .join(format!("{key}_basic.n"));
    if archived.exists() {
        return archived;
    }
    current
}

fn evidence_json_path(key: &str) -> PathBuf {
    workspace_root()
        .join("reports/runtime_closure/evidence")
        .join(format!("{key}.json"))
}

fn evidence_md_path(key: &str) -> PathBuf {
    workspace_root()
        .join("reports/runtime_closure/evidence")
        .join(format!("{key}.md"))
}

fn runtime_family_is_closed(row: &Value) -> bool {
    row["release_status"].as_str() == Some("closed")
        || row["status"].as_str() == Some("closed")
        || row["backend_status"].as_str() == Some("closed")
        || row["nominal_runtime_call_audit"].as_bool() == Some(true)
}

fn callable_name(raw: &str) -> &str {
    raw.split_once('<').map(|(name, _)| name).unwrap_or(raw)
}

fn first_parseable_export(family: &str) -> Value {
    let payload = runtime_json(&["runtime", "exports", family, "--json"]);
    payload["exports"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| {
                    row["name"]
                        .as_str()
                        .map(callable_name)
                        .is_some_and(|name| name != "assert")
                })
                .or_else(|| rows.first())
        })
        .cloned()
        .expect("family must expose at least one export")
}

fn missing_use_source(row: &Value) -> String {
    let key = family_key(row);
    let export = first_parseable_export(import_path(row));
    let raw_name = export["name"].as_str().expect("export name");
    let name = callable_name(raw_name);
    let arity = export["arity_min"].as_u64().expect("arity") as usize;
    let args = (0..arity).map(|_| "0").collect::<Vec<_>>().join(", ");
    format!(
        "runtimeMissingUse{} :: runtime {{\n    fn:\n        run() -> bool {{\n            {}({});\n            return true;\n        }}\n    ref main ->> run;\n    fa:\n        return true;\n}}\n",
        key.replace(|c: char| !c.is_ascii_alphanumeric(), ""),
        name,
        args
    )
    .replace("run() -> bool", "runtimeClosureEntry() -> bool")
    .replace("@fn(run)", "@fn(runtimeClosureEntry)")
}

#[test]
fn runtime_all_90_families_have_docs() {
    let rows = families();
    assert_eq!(
        rows.len(),
        93,
        "runtime family count must include the 90-family base and 3 native extensions"
    );
    for row in &rows {
        let key = family_key(row);
        assert!(docs_path(key).is_file(), "missing docs for {key}");
    }
    assert!(
        docs_path("list").is_file(),
        "missing docs for nC.list alias"
    );
}

#[test]
fn runtime_all_90_families_have_exports() {
    for row in families() {
        let payload = runtime_json(&["runtime", "exports", import_path(&row), "--json"]);
        let exports = payload["exports"].as_array().expect("exports");
        assert!(
            !exports.is_empty(),
            "family {} has no exports",
            family_key(&row)
        );
        for export in exports {
            assert!(export["name"].as_str().is_some(), "export name missing");
            assert!(export["arity_min"].as_u64().is_some(), "arity missing");
            assert!(export["variadic"].as_bool().is_some(), "variadic missing");
            assert!(export["return"].as_str().is_some(), "return missing");
            assert!(
                export["arg_types"].as_array().is_some(),
                "arg_types missing"
            );
        }
    }
}

#[test]
fn runtime_all_90_families_have_capabilities_registered() {
    let catalog = runtime_json(&["runtime", "capabilities", "--json"]);
    let known = catalog["catalog"]
        .as_array()
        .expect("capability catalog")
        .iter()
        .filter_map(Value::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for row in families() {
        for cap in row["required_capabilities"]
            .as_array()
            .expect("capabilities")
        {
            let cap = cap.as_str().expect("capability");
            assert!(known.contains(cap), "unknown capability {cap}");
        }
    }
}

#[test]
fn runtime_all_90_families_have_policies_registered() {
    let catalog = runtime_json(&["runtime", "policies", "--json"]);
    let known = catalog["catalog"]
        .as_array()
        .expect("policy catalog")
        .iter()
        .filter_map(Value::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for row in families() {
        for policy in row["required_policies"].as_array().expect("policies") {
            let policy = policy.as_str().expect("policy");
            assert!(known.contains(policy), "unknown policy {policy}");
        }
    }
}

#[test]
fn runtime_all_90_families_have_no_unknown_errors() {
    let errors = runtime_json(&["runtime", "errors", "--json"]);
    assert_eq!(errors["count"].as_u64(), Some(182));
    for row in errors["errors"].as_array().expect("errors") {
        assert!(row["code"].as_str().is_some(), "error code missing");
        assert!(row["family"].as_str().is_some(), "error family missing");
    }
}

#[test]
fn runtime_all_90_families_have_examples() {
    for row in families() {
        let key = family_key(&row);
        assert!(example_path(key).is_file(), "missing example for {key}");
    }
    assert!(example_path("list").is_file(), "missing list alias example");
}

#[test]
fn runtime_all_90_families_report_json_ok() {
    for row in families() {
        let key = family_key(&row);
        let path = evidence_json_path(key);
        if path.is_file() {
            let text = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("cannot read evidence JSON {}: {err}", path.display())
            });
            let payload: Value = serde_json::from_str(&text).expect("evidence JSON");
            assert_eq!(payload["release_status"].as_str(), Some("closed"));
        } else {
            assert!(
                runtime_family_is_closed(&row),
                "runtime family {key} is not closed"
            );
        }
    }
}

#[test]
fn runtime_all_90_families_report_md_ok() {
    for row in families() {
        let key = family_key(&row);
        let path = evidence_md_path(key);
        if path.is_file() {
            let text = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!("cannot read evidence Markdown {}: {err}", path.display())
            });
            assert!(
                text.contains("status: closed"),
                "evidence MD not closed for {key}"
            );
        } else {
            assert!(
                runtime_family_is_closed(&row),
                "runtime family {key} is not closed"
            );
        }
    }
}

#[test]
fn runtime_all_90_families_import_check_ok() {
    for row in families() {
        let key = family_key(&row);
        let example = example_path(key);
        let example = example.to_str().expect("utf8 path");
        let out = run_ncc(&["runtime", "check", example, "--strict", "--json"]);
        assert!(
            out.status.success(),
            "runtime check failed for {key}\nstdout:\n{}\nstderr:\n{}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let payload: Value = serde_json::from_slice(&out.stdout).expect("check JSON");
        assert_eq!(payload["status"].as_str(), Some("ok"));
    }
}

#[test]
fn runtime_no_family_identity_mismatch() {
    for row in families() {
        let key = family_key(&row);
        let payload = runtime_json(&["runtime", "exports", import_path(&row), "--json"]);
        assert_eq!(payload["family_key"].as_str(), Some(key));
        assert_eq!(payload["import_path"].as_str(), Some(import_path(&row)));
    }
}

#[test]
fn runtime_no_export_mixed_between_families() {
    let list = runtime_json(&["runtime", "exports", "nC.runtime.list", "--json"]);
    let queue = runtime_json(&["runtime", "exports", "nC.runtime.queue", "--json"]);
    for export in list["exports"].as_array().expect("list exports") {
        let name = export["name"].as_str().expect("name").to_ascii_lowercase();
        assert!(
            !name.starts_with("queue"),
            "list leaked queue export {name}"
        );
    }
    for export in queue["exports"].as_array().expect("queue exports") {
        let name = export["name"].as_str().expect("name").to_ascii_lowercase();
        assert!(!name.starts_with("list"), "queue leaked list export {name}");
    }
}

#[test]
fn runtime_no_missing_use_silent_fallback() {
    let tmp_dir = std::env::temp_dir();
    for row in families() {
        let key = family_key(&row);
        if matches!(key, "string" | "json") {
            // These two catalog families are intentionally provided by the
            // hosted runtime prelude v1; dedicated tests cover its mode gate.
            continue;
        }
        let path = tmp_dir.join(format!("ncc_runtime_missing_use_{key}.n"));
        fs::write(&path, missing_use_source(&row)).expect("write missing-use fixture");
        let out = run_ncc(&[
            "runtime",
            "check",
            path.to_str().expect("utf8 path"),
            "--strict",
        ]);
        let _ = fs::remove_file(&path);
        assert!(
            !out.status.success(),
            "missing-use fixture unexpectedly passed for {key}"
        );
        let combined = format!("{}{}", normalize(&out.stdout), normalize(&out.stderr));
        let expected_diagnostic = if key == "result" {
            "E_RUNTIME_IMPORT_001"
        } else {
            "E_RUNTIME_USE_001"
        };
        assert!(
            combined.contains(expected_diagnostic),
            "missing-use fixture for {key} did not emit {expected_diagnostic}:\n{combined}"
        );
    }
}

#[test]
fn runtime_family_list_and_queue_identity_ok() {
    let list = runtime_json(&["runtime", "exports", "nC.runtime.list", "--json"]);
    assert_eq!(list["family_key"].as_str(), Some("list"));
    assert_eq!(list["import_path"].as_str(), Some("nc.runtime.list"));
    assert_eq!(list["module_path"].as_str(), Some("runtime.list"));

    let nc_list = runtime_json(&["runtime", "exports", "nC.list", "--json"]);
    assert_eq!(nc_list["family_key"].as_str(), Some("list"));
    assert_eq!(nc_list["import_path"].as_str(), Some("nc.runtime.list"));
    assert_eq!(nc_list["module_path"].as_str(), Some("runtime.list"));

    let queue = runtime_json(&["runtime", "exports", "nC.runtime.queue", "--json"]);
    assert_eq!(queue["family_key"].as_str(), Some("queue"));
    assert_eq!(queue["import_path"].as_str(), Some("nc.runtime.queue"));
    assert_eq!(queue["module_path"].as_str(), Some("runtime.queue"));
}

#[test]
fn runtime_no_disallowed_marker_in_family_examples() {
    let needles = ["<expr>", "String(\"<expr>\")", "runtime stub", "stub route"];
    for row in families() {
        let key = family_key(&row);
        let text = fs::read_to_string(example_path(key)).expect("example text");
        for needle in needles {
            assert!(
                !text.contains(needle),
                "example for {key} contains disallowed marker {needle}"
            );
        }
    }
}
