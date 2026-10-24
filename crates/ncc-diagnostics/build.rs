use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let registry_path = manifest.join("../../diagnostics/registry.json");
    println!("cargo:rerun-if-changed={}", registry_path.display());

    let bytes = fs::read(&registry_path).expect("read canonical diagnostic registry");
    let root: Value = serde_json::from_slice(&bytes).expect("parse canonical diagnostic registry");
    assert_eq!(
        root.get("schema").and_then(Value::as_str),
        Some("nc.diagnostic.registry.v1"),
        "unsupported canonical diagnostic registry schema"
    );
    let entries = root
        .get("entries")
        .and_then(Value::as_array)
        .expect("diagnostic registry entries");
    assert!(!entries.is_empty(), "diagnostic registry cannot be empty");
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| field(left, "code").cmp(field(right, "code")));

    let mut seen = BTreeSet::new();
    let mut generated = String::from(
        "// Generated from diagnostics/registry.json. Do not edit.\n\
         pub const DIAGNOSTIC_REGISTRY: &[DiagnosticDefinition] = &[\n",
    );
    for entry in entries {
        let code = field(entry, "code");
        let phase = field(entry, "phase");
        let summary = field(entry, "summary");
        let help = field(entry, "help");
        assert!(canonical_code(code), "invalid diagnostic code '{code}'");
        assert!(seen.insert(code), "duplicate diagnostic code '{code}'");
        generated.push_str(&format!(
            "    DiagnosticDefinition {{ code: {code:?}, phase: {phase:?}, summary: {summary:?}, help: {help:?} }},\n"
        ));
    }
    generated.push_str("];\n\npub mod codes {\n");
    for code in seen {
        let identifier = code.replace('-', "_");
        generated.push_str(&format!("    pub const {identifier}: &str = {code:?};\n"));
    }
    generated.push_str("}\n");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("diagnostic_registry_generated.rs"), generated)
        .expect("write generated diagnostic registry");
}

fn field<'a>(entry: &'a Value, name: &str) -> &'a str {
    entry
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("diagnostic registry entry missing string field '{name}'"))
}

fn canonical_code(code: &str) -> bool {
    let Some((stem, suffix)) = code.rsplit_once('_') else {
        return code.starts_with("NCC-");
    };
    matches!(code.as_bytes().first(), Some(b'E' | b'W' | b'I' | b'N'))
        && stem.len() > 2
        && suffix.len() == 3
        && suffix.bytes().all(|byte| byte.is_ascii_digit())
        && stem
            .bytes()
            .skip(2)
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}
