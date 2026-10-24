use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const DIRECT_MESSAGE_MARKERS: &[&str] = &[
    "Diagnostic::new(",
    "CanonicalDiagnostic::new(",
    "MctDiagnosticIR::new(",
    "MachineDiagnostic::new(",
    "stage0_registered_diagnostic(",
    "stage0_registered(",
    "Err(format!(",
    "return Err(format!(",
    "message: format!(",
    "message: \"",
];

#[derive(Debug, Clone, Copy)]
struct Allowance {
    code_literals: usize,
    direct_message_sites: usize,
}

#[test]
fn rust_cannot_grow_a_parallel_diagnostic_authority() {
    let workspace = workspace();
    let registry = load_registry(&workspace);
    let allowances = load_allowances(&workspace);
    let code_pattern =
        Regex::new(r"\b(?:[EWNI]_[A-Z][A-Z0-9_]*_[0-9]{3}|NCC-(?:CLI|DEPRECATION|DX)-[0-9]{4})\b")
            .expect("canonical diagnostic regex");

    for (crate_name, allowance) in allowances {
        let source_root = workspace.join("crates").join(&crate_name).join("src");
        assert!(
            source_root.is_dir(),
            "missing governed source root {source_root:?}"
        );
        let mut files = Vec::new();
        collect_rust(&source_root, &mut files);
        let mut code_literals = 0usize;
        let mut direct_message_sites = 0usize;
        for path in files {
            let source = fs::read_to_string(&path).expect("read governed Rust source");
            for matched in code_pattern.find_iter(&source) {
                let prefix = &source[..matched.start()];
                if prefix.ends_with("codes::") {
                    continue;
                }
                code_literals += 1;
                assert!(
                    registry.contains(matched.as_str()),
                    "{} defines unknown diagnostic '{}' in {}; add it to diagnostics/registry.json and use ncc_diagnostics::codes",
                    crate_name,
                    matched.as_str(),
                    path.display()
                );
            }
            direct_message_sites += DIRECT_MESSAGE_MARKERS
                .iter()
                .map(|marker| source.matches(marker).count())
                .sum::<usize>();
        }
        assert!(
            code_literals <= allowance.code_literals,
            "{crate_name} grew direct diagnostic codes from {} to {code_literals}; use a generated ncc_diagnostics::codes constant",
            allowance.code_literals
        );
        assert!(
            direct_message_sites <= allowance.direct_message_sites,
            "{crate_name} grew direct diagnostic messages from {} to {direct_message_sites}; put the summary/help in diagnostics/registry.json",
            allowance.direct_message_sites
        );
    }
}

#[test]
fn registry_declares_the_nc_material_implementation() {
    let workspace = workspace();
    let root: Value = serde_json::from_slice(
        &fs::read(workspace.join("diagnostics/registry.json")).expect("read registry"),
    )
    .expect("parse registry");
    assert_eq!(root["schema"], "nc.diagnostic.registry.v1");
    assert_eq!(
        root["material_implementation"],
        "runtime/nc/diagnostic_registry.n"
    );
    assert!(workspace
        .join(root["material_implementation"].as_str().unwrap())
        .is_file());
    assert_eq!(
        root["entries"].as_array().map(Vec::len),
        Some(ncc_diagnostics::DIAGNOSTIC_REGISTRY.len())
    );
}

#[test]
fn rust_is_declared_as_stage0_until_nc_rebuilds_the_toolchain() {
    let workspace = workspace();
    let stages: Value = serde_json::from_slice(
        &fs::read(workspace.join("bootstrap/stages.json")).expect("read bootstrap stages"),
    )
    .expect("parse bootstrap stages");
    assert_eq!(stages["schema"], "nc.bootstrap.stages.v1");
    assert_eq!(stages["active_stage"], "stage0");
    assert_eq!(stages["stage0"]["language"], "Rust");
    assert_eq!(stages["stage1"]["language"], "nC");
    assert_eq!(stages["claims"]["self_hosted"], false);
    assert_eq!(stages["claims"]["rust_is_stage0"], true);
    assert!(stages["retirement_gates"]
        .as_array()
        .is_some_and(|gates| gates.len() >= 5));
    for material in stages["stage1"]["material_roots"]
        .as_array()
        .expect("stage1 material roots")
    {
        assert!(
            workspace
                .join(material.as_str().expect("material path"))
                .is_file(),
            "missing stage1 material root {material}"
        );
    }
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_registry(workspace: &Path) -> BTreeSet<String> {
    let root: Value = serde_json::from_slice(
        &fs::read(workspace.join("diagnostics/registry.json")).expect("read registry"),
    )
    .expect("parse registry");
    root["entries"]
        .as_array()
        .expect("registry entries")
        .iter()
        .map(|entry| entry["code"].as_str().expect("entry code").to_string())
        .collect()
}

fn load_allowances(workspace: &Path) -> BTreeMap<String, Allowance> {
    let root: Value = serde_json::from_slice(
        &fs::read(workspace.join("diagnostics/rust-stage0-allowlist.json"))
            .expect("read stage0 allowlist"),
    )
    .expect("parse stage0 allowlist");
    assert_eq!(root["schema"], "nc.diagnostic.rust-stage0-allowlist.v1");
    root["crates"]
        .as_object()
        .expect("allowlisted crates")
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                Allowance {
                    code_literals: value["code_literals"].as_u64().unwrap() as usize,
                    direct_message_sites: value["direct_message_sites"].as_u64().unwrap() as usize,
                },
            )
        })
        .collect()
}

fn collect_rust(path: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read governed source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            collect_rust(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}
