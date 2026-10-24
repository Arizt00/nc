use serde_json::Value;
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
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| field(left, "code").cmp(field(right, "code")));

    let mut source = String::from(
        "// Generated from diagnostics/registry.json. Do not edit.\n\
         pub const GENERATED_DIAGNOSTIC_CODES: &[(&str, &str)] = &[\n",
    );
    for entry in entries {
        let code = field(entry, "code");
        let phase = field(entry, "phase");
        source.push_str(&format!("    ({code:?}, {phase:?}),\n"));
    }
    source.push_str("];\n");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("diagnostic_catalog_generated.rs"), source)
        .expect("write generated diagnostic catalog");
}

fn field<'a>(entry: &'a Value, name: &str) -> &'a str {
    entry
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("diagnostic registry entry missing string field '{name}'"))
}
