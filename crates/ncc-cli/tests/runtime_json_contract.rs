use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn fixture(name: &str) -> PathBuf {
    workspace_root().join("examples/runtime/json").join(name)
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .current_dir(workspace_root())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn runtime_json_family_exports_complete_typed_v1_surface() {
    let output = run_ncc(&[
        "runtime".into(),
        "exports".into(),
        "nC.runtime.json".into(),
        "--json".into(),
    ]);
    let rendered = combined(&output);
    assert!(output.status.success(), "runtime JSON exports\n{rendered}");
    let payload: Value = serde_json::from_str(&rendered).expect("exports must be JSON");
    let functions = payload["functions"]
        .as_array()
        .or_else(|| payload["exports"].as_array())
        .expect("JSON family functions");
    for expected in [
        "jsonParse",
        "jsonStringifyPretty",
        "jsonValidate",
        "jsonObjectGet",
        "jsonArrayGet",
        "jsonPointer",
        "jsonGetString",
        "jsonNewObject",
        "jsonParseFile",
        "jsonSaveFile",
        "jsonErrorName",
    ] {
        assert!(
            functions.iter().any(|row| {
                row["name"].as_str() == Some(expected)
                    || row.as_str() == Some(expected)
                    || row["function"].as_str() == Some(expected)
            }),
            "missing JSON export {expected}: {payload}"
        );
    }
}

#[test]
fn runtime_json_positive_fixture_passes_parser_sema_and_interpreter() {
    let path = fixture("json_complete_ok.n");
    for command in ["check", "run"] {
        let mut args = vec![command.into(), path.to_string_lossy().into_owned()];
        if command == "run" {
            args.push("--interpreted".into());
        }
        let output = run_ncc(&args);
        let rendered = combined(&output);
        assert!(
            output.status.success(),
            "ncc {command} JSON positive fixture\n{rendered}"
        );
    }
}

#[test]
fn runtime_json_wrong_argument_type_is_rejected_by_sema() {
    let output = run_ncc(&[
        "check".into(),
        fixture("json_wrong_argument_type_fail.n")
            .to_string_lossy()
            .into_owned(),
    ]);
    let rendered = combined(&output);
    assert!(!output.status.success(), "wrong JSON arg type must fail");
    assert!(
        rendered.contains("E_SEMA")
            || rendered.contains("E_RUNTIME_GENERIC_ARG_MISMATCH_001")
            || rendered.contains("type")
            || rendered.contains("expects"),
        "missing typed JSON diagnostic: {rendered}"
    );
}
