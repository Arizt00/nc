use std::path::PathBuf;
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
        .current_dir(workspace_root())
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

fn assert_failure(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(!out.status.success(), "{label}\n{rendered}");
    rendered
}

fn example(rel: &str) -> String {
    let root = workspace_root();
    let direct = root.join(rel);
    if direct.exists() {
        return direct.to_string_lossy().to_string();
    }
    let mapped = match rel {
        "examples/postIncrementCompact.n" => Some("examples/01_basics/post_increment_compact.n"),
        _ => None,
    };
    if let Some(mapped) = mapped {
        let path = root.join(mapped);
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }
    if let Some(legacy) = rel.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(legacy);
        if archived.exists() {
            return archived.to_string_lossy().to_string();
        }
    }
    direct.to_string_lossy().to_string()
}

#[test]
fn feature_suite_manifest_lists_required_lanes_and_paths() {
    let output = assert_success(
        run_ncc(&["compiler", "feature-suite", "--json", "--strict"]),
        "feature suite contract manifest",
    );
    let json: serde_json::Value =
        serde_json::from_str(&output).expect("feature suite output must be json");

    assert_eq!(
        json["schema"], "nc.compiler.feature_suite_contract.v1",
        "{json:#?}"
    );
    assert_eq!(json["status"], "closed_for_new_features", "{json:#?}");
    assert_eq!(json["coverage"]["all_required_lanes_present"], true);

    let lanes = json["required_lanes"]
        .as_array()
        .expect("required_lanes must be array")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        lanes,
        vec![
            "example_n",
            "parser",
            "sema",
            "ir",
            "mct",
            "run",
            "negative"
        ]
    );

    let suites = json["suites"].as_array().expect("suites must be array");
    let suite = suites
        .iter()
        .find(|suite| suite["id"] == "compact_post_increment")
        .expect("compact post-increment suite must be registered");
    let lane_ids = suite["lanes"]
        .as_array()
        .expect("suite lanes must be array")
        .iter()
        .filter_map(|lane| lane["id"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(lane_ids, lanes, "{suite:#?}");
}

#[test]
fn feature_suite_compact_post_increment_example_n() {
    let path = example("examples/postIncrementCompact.n");
    let output = assert_success(run_ncc(&["check", &path]), "feature suite .n example lane");
    assert!(output.contains("OK"), "{output}");
}

#[test]
fn feature_suite_compact_post_increment_parser() {
    let path = example("examples/postIncrementCompact.n");
    let output = assert_success(
        run_ncc(&["compiler", "ast", &path, "--json"]),
        "feature suite parser lane",
    );
    let json: serde_json::Value = serde_json::from_str(&output).expect("ast lane must be json");
    assert_eq!(
        json["ast"]["containers"][0]["name"], "postIncrementCompact",
        "{json:#?}"
    );
}

#[test]
fn feature_suite_compact_post_increment_sema() {
    let path = example("examples/postIncrementCompact.n");
    let output = assert_success(
        run_ncc(&["compiler", "sema", &path, "--json"]),
        "feature suite sema lane",
    );
    let json: serde_json::Value = serde_json::from_str(&output).expect("sema lane must be json");
    assert_eq!(json["status"], "ok", "{json:#?}");
}

#[test]
fn feature_suite_compact_post_increment_ir() {
    let path = example("examples/postIncrementCompact.n");
    let output = assert_success(run_ncc(&["emit-ir", &path]), "feature suite IR lane");
    assert!(output.contains("postIncrementCompact"), "{output}");
    assert!(output.contains("PostIncrement"), "{output}");
    assert!(output.contains("ScalarProjection"), "{output}");
}

#[test]
fn feature_suite_compact_post_increment_mct() {
    let backend_path = example("examples/backend/mct_llvm_post_increment.n");
    let out = std::env::temp_dir().join(format!(
        "ncc_feature_suite_post_increment_{}.ll",
        std::process::id()
    ));
    let out_path = out.to_string_lossy().to_string();
    let output = assert_success(
        run_ncc(&[
            "mct",
            "emit-llvm",
            &backend_path,
            "--json",
            "--out",
            &out_path,
        ]),
        "feature suite MCT lane",
    );
    assert!(output.contains("expr.post_increment"), "{output}");
    assert!(output.contains("load/add/store"), "{output}");

    let llvm = std::fs::read_to_string(&out).expect("llvm output must be readable");
    assert!(llvm.contains("add i32"), "{llvm}");
    assert!(llvm.contains("store i32"), "{llvm}");
}

#[test]
fn feature_suite_compact_post_increment_run() {
    let path = example("examples/postIncrementCompact.n");
    let output = assert_success(run_ncc(&["run", &path]), "feature suite run lane");
    assert!(output.contains("i=2"), "{output}");
    assert!(output.contains("ok=true"), "{output}");
}

#[test]
fn feature_suite_compact_post_increment_negative() {
    let path = example("examples/feature_suites/negative/compact_post_increment_unbound_fail.n");
    let output = assert_failure(run_ncc(&["check", &path]), "feature suite negative lane");
    assert!(
        output.contains("E_HIGH_POST_INCREMENT_TARGET_001"),
        "{output}"
    );
}
