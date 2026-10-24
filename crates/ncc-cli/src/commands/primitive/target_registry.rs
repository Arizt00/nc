use crate::*;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct PrimitiveTargetSpec {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
    pub(crate) fields: HashMap<String, String>,
    pub(crate) bool_fields: HashMap<String, bool>,
    pub(crate) protocols: BTreeSet<String>,
}

impl PrimitiveTargetSpec {
    pub(crate) fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|v| v.as_str())
    }

    pub(crate) fn has_capability(&self, key: &str) -> bool {
        self.bool_fields.get(key).copied().unwrap_or(false)
    }

    pub(crate) fn has_capability_or(&self, key: &str, fallback: bool) -> bool {
        self.bool_fields.get(key).copied().unwrap_or(fallback)
    }

    pub(crate) fn supports_protocol(&self, protocol: &str) -> bool {
        self.protocols.contains(&protocol.to_ascii_uppercase())
    }
}

fn primitive_parse_nctarget_kv(line: &str) -> Option<(String, String)> {
    let stripped = line
        .split_once('#')
        .map(|(head, _)| head)
        .unwrap_or(line)
        .trim();
    if stripped.is_empty() {
        return None;
    }
    let (key, value) = stripped.split_once('=')?;
    Some((key.trim().to_string(), value.trim().to_string()))
}

fn primitive_unquote(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn primitive_parse_protocol_list(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let trimmed = raw.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return out;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    for item in inner.split(',') {
        let value = primitive_unquote(item).trim().to_string();
        if !value.is_empty() {
            out.insert(value.to_ascii_uppercase());
        }
    }
    out
}

fn primitive_host_path(raw: &Path, windows_host: bool) -> PathBuf {
    if !windows_host {
        return raw.to_path_buf();
    }

    let normalized = raw.to_string_lossy().replace('\\', "/");
    let Some(rest) = normalized.strip_prefix("/mnt/") else {
        return raw.to_path_buf();
    };
    let mut parts = rest.splitn(2, '/');
    let Some(drive) = parts.next() else {
        return raw.to_path_buf();
    };
    let Some(tail) = parts.next() else {
        return raw.to_path_buf();
    };
    if drive.len() != 1 || !drive.as_bytes()[0].is_ascii_alphabetic() {
        return raw.to_path_buf();
    }

    PathBuf::from(format!(
        "{}:\\{}",
        drive.to_ascii_uppercase(),
        tail.replace('/', "\\")
    ))
}

fn primitive_push_ancestor_target_dirs(candidates: &mut Vec<PathBuf>, start: &Path) {
    for ancestor in start.ancestors() {
        candidates.push(ancestor.join("targets"));
    }
}

fn primitive_target_specs_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for name in ["NC_TARGETS_DIR", "NCC_TARGETS_DIR"] {
        if let Ok(raw) = std::env::var(name) {
            candidates.push(primitive_host_path(Path::new(&raw), cfg!(windows)));
        }
    }
    for name in ["NC_SOURCE_ROOT", "NCC_SOURCE_ROOT"] {
        if let Ok(raw) = std::env::var(name) {
            candidates.push(primitive_host_path(Path::new(&raw), cfg!(windows)).join("targets"));
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(bin_dir) = executable.parent() {
            if let Some(prefix) = bin_dir.parent() {
                candidates.push(prefix.join("share").join("nC").join("targets"));
            }
            primitive_push_ancestor_target_dirs(&mut candidates, bin_dir);
        }
    }
    if let Ok(current) = std::env::current_dir() {
        primitive_push_ancestor_target_dirs(&mut candidates, &current);
    }

    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    candidates.push(primitive_host_path(&compiled, cfg!(windows)).join("targets"));

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn primitive_target_specs_dir() -> PathBuf {
    primitive_target_specs_candidates()
        .into_iter()
        .find(|candidate| candidate.is_dir())
        .unwrap_or_else(|| workspace_repo_root().join("targets"))
}

pub(crate) fn primitive_resource_path(raw: &Path) -> PathBuf {
    if raw.is_absolute() {
        return raw.to_path_buf();
    }

    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join(raw));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(bin_dir) = executable.parent() {
            if let Some(prefix) = bin_dir.parent() {
                candidates.push(prefix.join("share").join("nC").join(raw));
            }
            for ancestor in bin_dir.ancestors() {
                candidates.push(ancestor.join(raw));
            }
        }
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    candidates.push(primitive_host_path(&compiled, cfg!(windows)).join(raw));

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| workspace_repo_root().join(raw))
}

pub(crate) fn primitive_target_path(target: &str) -> PathBuf {
    primitive_target_specs_dir().join(format!("{target}.nctarget"))
}

pub(crate) fn load_primitive_target_spec(
    target: &str,
) -> Result<PrimitiveTargetSpec, Box<dyn std::error::Error>> {
    let path = primitive_target_path(target);
    if !path.exists() {
        return Err(format!(
            "E_PRIMITIVE_CLI_TARGET_001: target file not found for '{}': {}",
            target,
            path.to_string_lossy()
        )
        .into());
    }

    let raw = fs::read_to_string(&path)?;
    let mut fields = HashMap::new();
    let mut bool_fields = HashMap::new();
    let mut protocols = BTreeSet::new();

    for line in raw.lines() {
        let Some((key, value)) = primitive_parse_nctarget_kv(line) else {
            continue;
        };
        if key == "protocols" {
            protocols = primitive_parse_protocol_list(&value);
            fields.insert(key, value);
            continue;
        }
        if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
            bool_fields.insert(key.clone(), value.eq_ignore_ascii_case("true"));
            fields.insert(key, value.to_ascii_lowercase());
            continue;
        }
        fields.insert(key, primitive_unquote(&value));
    }

    let id = fields
        .get("name")
        .cloned()
        .unwrap_or_else(|| target.to_string());

    Ok(PrimitiveTargetSpec {
        id,
        path,
        fields,
        bool_fields,
        protocols,
    })
}

pub(crate) fn list_primitive_target_specs(
) -> Result<Vec<PrimitiveTargetSpec>, Box<dyn std::error::Error>> {
    let dir = primitive_target_specs_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("nctarget") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    let mut specs = Vec::new();
    for name in names {
        match load_primitive_target_spec(&name) {
            Ok(spec) => specs.push(spec),
            Err(err) => {
                let rendered = err.to_string();
                if rendered.contains("E_PRIMITIVE_CLI_TARGET_001")
                    || rendered.contains("No such file or directory")
                {
                    continue;
                }
                return Err(err);
            }
        }
    }
    Ok(specs)
}

#[cfg(test)]
mod target_registry_path_tests {
    use super::*;

    #[test]
    fn primitive_target_registry_converts_cross_compiled_wsl_paths_for_windows() {
        let path = primitive_host_path(
            Path::new("/mnt/c/Users/example/nC/crates/ncc-cli/../.."),
            true,
        );
        assert_eq!(
            path.to_string_lossy(),
            "C:\\Users\\example\\nC\\crates\\ncc-cli\\..\\.."
        );
    }

    #[test]
    fn primitive_target_registry_preserves_native_paths() {
        let raw = Path::new("/opt/nc/targets");
        assert_eq!(primitive_host_path(raw, false), raw);
    }

    #[test]
    fn primitive_resource_registry_resolves_the_baremetal_linker() {
        let path = primitive_resource_path(Path::new("link/x86_64_baremetal.ld"));
        assert!(
            path.is_file(),
            "missing linker resource: {}",
            path.display()
        );
    }
}
