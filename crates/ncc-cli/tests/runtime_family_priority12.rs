use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
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

fn path_with_ncc_first(ncc: &Path) -> String {
    let bin_dir = ncc.parent().expect("ncc binary must have parent dir");
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn run_ncc(args: &[&str]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_installed_ncc(args: &[&str]) -> Output {
    let ncc = ncc_bin();
    Command::new("ncc")
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("installed/path-resolved ncc command must execute")
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

fn ncc_json(args: &[&str]) -> Value {
    let out = assert_success(run_ncc(args), &format!("ncc {args:?}"));
    serde_json::from_str(&out).unwrap_or_else(|err| panic!("{args:?} must emit JSON: {err}\n{out}"))
}

fn runtime_family_rows() -> Vec<Value> {
    let payload = ncc_json(&["runtime", "families", "--json"]);
    payload["rows"].as_array().expect("family rows").to_vec()
}

fn family_key(row: &Value) -> &str {
    row["family_key"].as_str().expect("family_key")
}

fn import_path(row: &Value) -> &str {
    row["family"].as_str().expect("family import path")
}

fn runtime_family_example_dirs() -> Vec<PathBuf> {
    let root = workspace_root();
    [
        root.join("examples/runtime/families"),
        root.join("_archive/old_examples/runtime/families"),
    ]
    .into_iter()
    .filter(|path| path.is_dir())
    .collect()
}

fn example_path(key: &str) -> PathBuf {
    for dir in runtime_family_example_dirs() {
        let path = dir.join(format!("{key}_basic.n"));
        if path.exists() {
            return path;
        }
    }
    workspace_root()
        .join("examples/runtime/families")
        .join(format!("{key}_basic.n"))
}

fn parse_example(path: &Path) -> ncc_ast::Program {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    let tokens = ncc_lexer::lex(&src)
        .unwrap_or_else(|err| panic!("lex failed for {}: {err}", path.display()));
    ncc_parser::parse(tokens)
        .unwrap_or_else(|err| panic!("parse failed for {}: {err}", path.display()))
}

fn callable_name(raw: &str) -> &str {
    raw.split_once('<').map(|(name, _)| name).unwrap_or(raw)
}

fn first_export_for(row: &Value) -> Value {
    let payload = ncc_json(&["runtime", "exports", import_path(row), "--json"]);
    payload["exports"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|export| {
                    export["arity_min"].as_u64() == Some(0)
                        && export["name"]
                            .as_str()
                            .map(callable_name)
                            .is_some_and(|name| name != "assert")
                })
                .or_else(|| {
                    rows.iter().find(|export| {
                        export["name"]
                            .as_str()
                            .map(callable_name)
                            .is_some_and(|name| name != "assert")
                    })
                })
                .or_else(|| rows.first())
        })
        .cloned()
        .expect("runtime family must expose at least one export")
}

fn arg_literal(ty: &str) -> &'static str {
    match ty.trim() {
        "str" => "\"\"",
        "bool" => "false",
        "ft32" | "ft64" | "float" => "0.0",
        "int" | "byte" | "qword" | "addr" | "any" => "0",
        _ if ty.starts_with("Result<") => "0",
        _ if ty.ends_with("Handle") => "0",
        _ => "0",
    }
}

fn export_args(export: &Value) -> String {
    let arity = export["arity_min"].as_u64().expect("arity") as usize;
    let arg_types = export["arg_types"].as_array().expect("arg_types");
    (0..arity)
        .map(|idx| {
            arg_types
                .get(idx)
                .and_then(Value::as_str)
                .map(arg_literal)
                .unwrap_or("0")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn negative_missing_use_source(row: &Value) -> String {
    let key = family_key(row);
    let export = first_export_for(row);
    let name = callable_name(export["name"].as_str().expect("export name"));
    let args = export_args(&export);
    let unrelated_import = if key == "input" {
        "use nC.runtime.output;"
    } else {
        "use nC.runtime.input;"
    };
    format!(
        "{unrelated_import}\n\nruntimePriority12MissingUse{}[@runtime] {{\n    fn:\n        entry() -> bool {{\n            public:\n                {}({});\n                return true;\n        }}\n    ref main ->> @fn(entry);\n    fa:\n        return true;\n}}\n",
        key.replace(|c: char| !c.is_ascii_alphanumeric(), ""),
        name,
        args
    )
}

#[test]
fn priority12_every_runtime_family_has_example_n() {
    let rows = runtime_family_rows();
    assert_eq!(rows.len(), 93, "runtime family count must stay explicit");
    for row in rows {
        let key = family_key(&row);
        assert!(example_path(key).is_file(), "missing .n example for {key}");
    }
}

#[test]
fn priority12_every_runtime_family_example_parses() {
    for row in runtime_family_rows() {
        let key = family_key(&row);
        let ast = parse_example(&example_path(key));
        assert!(
            ast.containers.iter().any(|container| {
                container
                    .name
                    .to_ascii_lowercase()
                    .contains(&key.to_ascii_lowercase())
            }),
            "parser lane did not preserve family identity for {key}"
        );
    }
}

#[test]
fn priority12_every_runtime_family_example_passes_sema() {
    for row in runtime_family_rows() {
        let key = family_key(&row);
        let ast = parse_example(&example_path(key));
        ncc_sema::analyze(&ast).unwrap_or_else(|err| panic!("sema failed for {key}: {err}"));
    }
}

#[test]
fn priority12_every_runtime_family_has_mct_activation_bytes() {
    let manifest = ncc_json(&["mct", "imports", "--json"]);
    assert_eq!(manifest["schema"], "nc.imports.backend_lowering.v1");
    let entries = manifest["entries"].as_array().expect("mct entries");

    for row in runtime_family_rows() {
        let key = family_key(&row);
        let import = import_path(&row);
        let entry = entries
            .iter()
            .find(|entry| {
                entry["surface_import"]
                    .as_str()
                    .is_some_and(|surface| surface.starts_with("use nC.runtime."))
                    && entry["expands_to"].as_array().is_some_and(|expanded| {
                        expanded.iter().any(|path| {
                            path.as_str()
                                .is_some_and(|path| path.eq_ignore_ascii_case(import))
                        })
                    })
            })
            .unwrap_or_else(|| panic!("missing MCT import activation row for {import}"));

        assert_eq!(
            entry["backing_status"], "closed_real_backing_bytes",
            "{import}"
        );
        assert_eq!(
            entry["runtime_call_lowering"], "direct_runtime_symbol_call_without_real_flag",
            "{import}"
        );
        assert_eq!(
            entry["mct_profiles"]
                .as_array()
                .expect("mct_profiles")
                .len(),
            4,
            "{import}"
        );
        assert!(
            entry["emitted_bytes"]
                .as_array()
                .expect("emitted_bytes")
                .iter()
                .all(|row| row["byte_count"].as_u64().unwrap_or_default() > 0
                    && row["bytes_hex"].as_str().is_some_and(|hex| !hex.is_empty())),
            "{import} must carry real activation bytes"
        );
        assert!(
            entry["runtime_symbols"]
                .as_array()
                .expect("runtime_symbols")
                .iter()
                .any(|symbol| {
                    symbol.as_str().is_some_and(|symbol| {
                        symbol
                            .to_ascii_lowercase()
                            .starts_with(&import.to_ascii_lowercase())
                    })
                }),
            "{import} must expose at least one runtime symbol for {key}"
        );
    }
}

#[test]
fn priority12_every_runtime_family_example_runs() {
    for row in runtime_family_rows() {
        let key = family_key(&row);
        let path = example_path(key);
        let rendered = assert_success(
            run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]),
            &format!("run family {key}"),
        );
        assert!(
            !rendered.contains("E_RUNTIME_USE_001"),
            "run lane regressed to missing-use diagnostic for {key}\n{rendered}"
        );
    }
}

#[test]
fn priority12_every_runtime_family_has_negative_missing_use_test() {
    let tmp = workspace_root().join("target/tmp/runtime-priority12-negative");
    fs::create_dir_all(&tmp).expect("tmp dir");
    for row in runtime_family_rows() {
        let key = family_key(&row);
        let path = tmp.join(format!("{key}_missing_use_fail.n"));
        fs::write(&path, negative_missing_use_source(&row)).expect("write negative fixture");
        let rendered = assert_failure(
            run_ncc(&[
                "runtime",
                "check",
                path.to_str().expect("utf8 path"),
                "--strict",
            ]),
            &format!("negative missing-use family {key}"),
        );
        assert!(
            rendered.contains("E_RUNTIME_USE_001"),
            "negative lane for {key} must report official missing-use diagnostic\n{rendered}"
        );
    }
}

#[test]
fn priority12_installed_binary_resolves_and_runs_runtime_family_contract() {
    let current = ncc_bin().canonicalize().expect("current ncc official path");
    let version = assert_success(
        run_installed_ncc(&["--version"]),
        "path-resolved installed ncc --version",
    );
    assert!(
        version.contains(&current.to_string_lossy().to_string()),
        "installed/path-resolved ncc must report the same executable path\n{version}"
    );

    let hash = assert_success(
        run_installed_ncc(&["--build-hash"]),
        "path-resolved installed ncc --build-hash",
    );
    assert!(hash.trim().starts_with("sha256:"), "{hash}");

    let doctor = assert_success(
        run_installed_ncc(&["doctor", "bins", "--json"]),
        "path-resolved installed ncc doctor bins",
    );
    let payload: Value = serde_json::from_str(&doctor)
        .unwrap_or_else(|err| panic!("doctor bins JSON: {err}\n{doctor}"));
    assert_eq!(payload["schema"], "ncc.doctor.bins.v1");
    assert_eq!(
        payload["current_exe"],
        current.to_string_lossy().to_string(),
        "{payload:#?}"
    );

    let check = assert_success(
        run_installed_ncc(&[
            "runtime",
            "check",
            example_path("output").to_str().expect("utf8 path"),
            "--strict",
            "--json",
        ]),
        "path-resolved installed ncc runtime check",
    );
    let payload: Value = serde_json::from_str(&check)
        .unwrap_or_else(|err| panic!("runtime check JSON: {err}\n{check}"));
    assert_eq!(payload["status"], "ok");
}

#[test]
fn priority12_runtime_family_manifest_matches_docs_examples_and_mct_sets() {
    let family_keys = runtime_family_rows()
        .iter()
        .map(|row| family_key(row).to_string())
        .collect::<BTreeSet<_>>();
    let docs = fs::read_dir(workspace_root().join("docs/spec/runtime/imports"))
        .expect("runtime import docs dir")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let examples = runtime_family_example_dirs()
        .into_iter()
        .flat_map(|dir| {
            fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
        })
        .filter_map(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix("_basic.n"))
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();
    let manifest = ncc_json(&["mct", "imports", "--json"]);
    let mct = manifest["entries"]
        .as_array()
        .expect("mct entries")
        .iter()
        .filter(|entry| {
            entry["surface_import"]
                .as_str()
                .is_some_and(|surface| surface.starts_with("use nC.runtime."))
        })
        .filter_map(|entry| entry["expands_to"].as_array())
        .flat_map(|expanded| expanded.iter())
        .filter_map(|path| path.as_str())
        .filter_map(|path| path.rsplit('.').next())
        .map(|key| key.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    assert!(
        family_keys.is_subset(&docs),
        "runtime docs missing: {:?}",
        family_keys.difference(&docs).collect::<Vec<_>>()
    );
    assert!(
        family_keys.is_subset(&examples),
        "runtime examples missing: {:?}",
        family_keys.difference(&examples).collect::<Vec<_>>()
    );
    assert_eq!(
        family_keys, mct,
        "MCT activation rows must match family set"
    );
}
