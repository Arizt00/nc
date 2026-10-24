use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CONTENT_CACHE_SCHEMA: &str = "ncc.content_cache.entry.v1";
const CONTENT_CACHE_EPOCH: &str = "2026-07-19.ir-mct-o3.v1";
const DEFAULT_MAX_BYTES: u64 = 1024 * 1024 * 1024;
const DEFAULT_MAX_ENTRIES: usize = 512;
const STALE_LOCK_AGE: Duration = Duration::from_secs(15 * 60);
static CACHE_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) struct ContentCacheArtifact {
    pub(crate) kind: String,
    pub(crate) path: PathBuf,
}

impl ContentCacheArtifact {
    pub(crate) fn new(kind: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ContentCacheHit {
    pub(crate) key: String,
    pub(crate) metadata: serde_json::Value,
    pub(crate) artifact_count: usize,
    pub(crate) bytes: u64,
}

#[derive(Debug)]
struct CacheLock {
    path: PathBuf,
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn sanitize_component(raw: &str) -> String {
    let mut value = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.is_empty() {
        value.push_str("cache");
    }
    value
}

pub(crate) fn content_cache_disabled(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--no-incremental")
        || std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some()
}

pub(crate) fn content_cache_root(explicit: Option<&Path>, artifact_root: &Path) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    if let Some(path) = std::env::var_os("NCC_CONTENT_CACHE_DIR") {
        return PathBuf::from(path);
    }
    artifact_root.join("cache").join("content-v1")
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn sha256_file(path: &Path) -> io::Result<String> {
    fs::read(path).map(|bytes| sha256_bytes(&bytes))
}

pub(crate) fn compiler_image_identity() -> serde_json::Value {
    let path = std::env::current_exe()
        .ok()
        .and_then(|path| fs::canonicalize(&path).ok().or(Some(path)));
    let metadata = path.as_ref().and_then(|path| fs::metadata(path).ok());
    serde_json::json!({
        "package": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "cache_epoch": CONTENT_CACHE_EPOCH,
        "path": path.as_ref().map(|path| path.display().to_string()),
        "bytes": metadata.as_ref().map(|value| value.len()),
        "modified_unix_ns": metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_nanos()),
    })
}

pub(crate) fn tool_identity(path: Option<&str>) -> serde_json::Value {
    let path = path.map(PathBuf::from);
    let canonical = path
        .as_ref()
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
    let metadata = canonical.as_ref().and_then(|path| fs::metadata(path).ok());
    serde_json::json!({
        "path": canonical.as_ref().map(|path| path.display().to_string()),
        "bytes": metadata.as_ref().map(|value| value.len()),
        "modified_unix_ns": metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_nanos()),
    })
}

pub(crate) fn content_cache_key(namespace: &str, material: &serde_json::Value) -> String {
    let envelope = serde_json::json!({
        "schema": "ncc.content_cache.key.v1",
        "cache_epoch": CONTENT_CACHE_EPOCH,
        "namespace": namespace,
        "material": material,
    });
    sha256_bytes(
        &serde_json::to_vec(&envelope).expect("content cache key material must always serialize"),
    )
}

fn cache_entry_dir(root: &Path, namespace: &str, key: &str) -> PathBuf {
    root.join("entries")
        .join(sanitize_component(namespace))
        .join(key)
}

fn cache_lock_dir(root: &Path, namespace: &str, key: &str) -> PathBuf {
    root.join("locks")
        .join(sanitize_component(namespace))
        .join(format!("{key}.lock"))
}

fn lock_is_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= STALE_LOCK_AGE)
}

fn acquire_lock(root: &Path, namespace: &str, key: &str) -> io::Result<CacheLock> {
    let path = cache_lock_dir(root, namespace, key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let started = SystemTime::now();
    loop {
        match fs::create_dir(&path) {
            Ok(()) => {
                let owner = serde_json::json!({
                    "pid": std::process::id(),
                    "created_unix_ms": unix_millis(),
                });
                let _ = fs::write(
                    path.join("owner.json"),
                    serde_json::to_vec_pretty(&owner).unwrap_or_default(),
                );
                return Ok(CacheLock { path });
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if lock_is_stale(&path) {
                    let _ = fs::remove_dir_all(&path);
                    continue;
                }
                if SystemTime::now()
                    .duration_since(started)
                    .unwrap_or_default()
                    >= Duration::from_secs(30)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "timed out waiting for content cache lock '{}'",
                            path.display()
                        ),
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(err),
        }
    }
}

fn copy_file_atomic(source: &Path, destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let sequence = CACHE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let temporary = destination.with_file_name(format!(
        ".{file_name}.ncc-cache-{}-{sequence}.tmp",
        std::process::id()
    ));
    fs::copy(source, &temporary)?;
    if let Ok(permissions) = fs::metadata(source).map(|metadata| metadata.permissions()) {
        let _ = fs::set_permissions(&temporary, permissions);
    }
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(&temporary, destination).or_else(|err| {
        let _ = fs::remove_file(&temporary);
        Err(err)
    })
}

fn read_valid_manifest(
    root: &Path,
    namespace: &str,
    key: &str,
) -> io::Result<Option<(PathBuf, serde_json::Value)>> {
    let entry = cache_entry_dir(root, namespace, key);
    let manifest_path = entry.join("manifest.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let raw = match fs::read(&manifest_path) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let manifest: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if manifest.get("schema").and_then(serde_json::Value::as_str) != Some(CONTENT_CACHE_SCHEMA)
        || manifest
            .get("namespace")
            .and_then(serde_json::Value::as_str)
            != Some(namespace)
        || manifest.get("key").and_then(serde_json::Value::as_str) != Some(key)
    {
        return Ok(None);
    }
    let Some(artifacts) = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(None);
    };
    let metadata_only = manifest
        .get("metadata_only")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if artifacts.is_empty() && !metadata_only {
        return Ok(None);
    }
    for artifact in artifacts {
        let Some(file) = artifact.get("file").and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Some(expected_size) = artifact.get("size").and_then(serde_json::Value::as_u64) else {
            return Ok(None);
        };
        let Some(expected_hash) = artifact.get("sha256").and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let path = entry.join(file);
        let metadata = match fs::metadata(&path) {
            Ok(value) if value.is_file() && value.len() == expected_size => value,
            _ => return Ok(None),
        };
        if metadata.len() == 0 || sha256_file(&path).ok().as_deref() != Some(expected_hash) {
            return Ok(None);
        }
    }
    Ok(Some((entry, manifest)))
}

pub(crate) fn restore_content_cache_entry(
    root: &Path,
    namespace: &str,
    key: &str,
    destinations: &BTreeMap<String, PathBuf>,
) -> io::Result<Option<ContentCacheHit>> {
    let _lock = acquire_lock(root, namespace, key)?;
    let Some((entry, manifest)) = read_valid_manifest(root, namespace, key)? else {
        return Ok(None);
    };
    let artifacts = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .expect("validated cache manifest has artifacts");
    for artifact in artifacts {
        let kind = artifact
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .expect("validated cache artifact has kind");
        if !destinations.contains_key(kind) {
            return Ok(None);
        }
    }
    let mut bytes = 0u64;
    for artifact in artifacts {
        let kind = artifact
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .expect("validated cache artifact has kind");
        let file = artifact
            .get("file")
            .and_then(serde_json::Value::as_str)
            .expect("validated cache artifact has file");
        let destination = destinations
            .get(kind)
            .expect("cache destination was prevalidated");
        copy_file_atomic(&entry.join(file), destination)?;
        bytes = bytes.saturating_add(
            artifact
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        );
    }
    Ok(Some(ContentCacheHit {
        key: key.to_string(),
        metadata: manifest
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        artifact_count: artifacts.len(),
        bytes,
    }))
}

pub(crate) fn store_content_cache_entry(
    root: &Path,
    namespace: &str,
    key: &str,
    material: &serde_json::Value,
    metadata: serde_json::Value,
    artifacts: &[ContentCacheArtifact],
) -> io::Result<()> {
    let _lock = acquire_lock(root, namespace, key)?;
    if read_valid_manifest(root, namespace, key)?.is_some() {
        return Ok(());
    }

    let namespace_dir = root.join("entries").join(sanitize_component(namespace));
    fs::create_dir_all(&namespace_dir)?;
    let sequence = CACHE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = namespace_dir.join(format!(".tmp-{key}-{}-{sequence}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary)?;
    }
    fs::create_dir_all(&temporary)?;

    let destination = cache_entry_dir(root, namespace, key);
    let write_result = (|| -> io::Result<()> {
        let mut rows = Vec::with_capacity(artifacts.len());
        for (index, artifact) in artifacts.iter().enumerate() {
            let source_metadata = fs::metadata(&artifact.path)?;
            if !source_metadata.is_file() || source_metadata.len() == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "content cache artifact '{}' is absent or empty",
                        artifact.path.display()
                    ),
                ));
            }
            let extension = artifact
                .path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| format!(".{value}"))
                .unwrap_or_default();
            let file = format!(
                "{index:03}-{}{}",
                sanitize_component(&artifact.kind),
                extension
            );
            let artifact_destination = temporary.join(&file);
            fs::copy(&artifact.path, &artifact_destination)?;
            if let Ok(permissions) = fs::metadata(&artifact.path).map(|value| value.permissions()) {
                let _ = fs::set_permissions(&artifact_destination, permissions);
            }
            rows.push(serde_json::json!({
                "kind": artifact.kind,
                "file": file,
                "size": source_metadata.len(),
                "sha256": sha256_file(&artifact.path)?,
            }));
        }

        let manifest = serde_json::json!({
            "schema": CONTENT_CACHE_SCHEMA,
            "cache_epoch": CONTENT_CACHE_EPOCH,
            "namespace": namespace,
            "key": key,
            "created_unix_ms": unix_millis(),
            "material_sha256": sha256_bytes(&serde_json::to_vec(material).unwrap_or_default()),
            "metadata_only": artifacts.is_empty(),
            "metadata": metadata,
            "artifacts": rows,
        });
        fs::write(
            temporary.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
        )?;

        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        fs::rename(&temporary, &destination)
    })();
    if write_result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    write_result?;
    prune_content_cache(root, Some(&destination))
}

fn default_process_cache_root() -> PathBuf {
    let artifact_root = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".ncc");
    content_cache_root(None, &artifact_root)
}

fn ast_cache_material(path: &Path, source: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": "ncc.frontend.ast_content_key.v1",
        "source_sha256": sha256_bytes(source.as_bytes()),
        "compiler": compiler_image_identity(),
        "syntax": "normal-nc-compact-and-compatible-verbose",
        "extension": path.extension().and_then(|value| value.to_str()),
    })
}

pub(crate) fn probe_ast(path: &Path, source: &str) -> Option<ncc_ast::Program> {
    if std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some() {
        return None;
    }
    let root = default_process_cache_root();
    let material = ast_cache_material(path, source);
    let key = content_cache_key("frontend-ast", &material);
    let destinations = BTreeMap::new();
    let hit = restore_content_cache_entry(&root, "frontend-ast", &key, &destinations).ok()??;
    serde_json::from_value(hit.metadata.get("ast")?.clone()).ok()
}

pub(crate) fn store_ast(path: &Path, source: &str, ast: &ncc_ast::Program) {
    if std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some() {
        return;
    }
    let root = default_process_cache_root();
    let material = ast_cache_material(path, source);
    let key = content_cache_key("frontend-ast", &material);
    let metadata = serde_json::json!({
        "ast": ast,
        "source_bytes": source.len(),
        "containers": ast.containers.len(),
    });
    let _ = store_content_cache_entry(&root, "frontend-ast", &key, &material, metadata, &[]);
}

fn sema_cache_material(ast: &ncc_ast::Program) -> Option<serde_json::Value> {
    let ast_bytes = serde_json::to_vec(ast).ok()?;
    Some(serde_json::json!({
        "schema": "ncc.frontend.sema_content_key.v1",
        "linked_ast_sha256": sha256_bytes(&ast_bytes),
        "compiler": compiler_image_identity(),
        "semantic_profile": "normal-high-level-and-linked-project",
    }))
}

pub(crate) fn probe_sema_warnings(ast: &ncc_ast::Program) -> Option<Vec<String>> {
    if std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some() {
        return None;
    }
    let material = sema_cache_material(ast)?;
    let key = content_cache_key("frontend-sema", &material);
    let hit = restore_content_cache_entry(
        &default_process_cache_root(),
        "frontend-sema",
        &key,
        &BTreeMap::new(),
    )
    .ok()??;
    serde_json::from_value(hit.metadata.get("warnings")?.clone()).ok()
}

pub(crate) fn store_sema_warnings(ast: &ncc_ast::Program, warnings: &[String]) {
    if std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some() {
        return;
    }
    let Some(material) = sema_cache_material(ast) else {
        return;
    };
    let key = content_cache_key("frontend-sema", &material);
    let metadata = serde_json::json!({
        "status": "semantic_analysis_passed",
        "warnings": warnings,
        "containers": ast.containers.len(),
    });
    let _ = store_content_cache_entry(
        &default_process_cache_root(),
        "frontend-sema",
        &key,
        &material,
        metadata,
        &[],
    );
}

fn directory_usage(path: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    let mut pending = vec![path.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

fn configured_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn prune_content_cache(root: &Path, protected: Option<&Path>) -> io::Result<()> {
    let max_bytes = configured_u64("NCC_CONTENT_CACHE_MAX_BYTES", DEFAULT_MAX_BYTES);
    let max_entries =
        configured_u64("NCC_CONTENT_CACHE_MAX_ENTRIES", DEFAULT_MAX_ENTRIES as u64) as usize;
    let entries_root = root.join("entries");
    if !entries_root.is_dir() {
        return Ok(());
    }
    let mut entries = Vec::<(SystemTime, u64, PathBuf)>::new();
    for namespace in fs::read_dir(&entries_root)? {
        let namespace = namespace?.path();
        if !namespace.is_dir() {
            continue;
        }
        for entry in fs::read_dir(namespace)? {
            let path = entry?.path();
            if !path.is_dir()
                || path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with(".tmp-"))
            {
                continue;
            }
            let modified = fs::metadata(path.join("manifest.json"))
                .and_then(|value| value.modified())
                .unwrap_or(UNIX_EPOCH);
            let bytes = directory_usage(&path).unwrap_or(0);
            entries.push((modified, bytes, path));
        }
    }
    entries.sort_by_key(|entry| entry.0);
    let mut bytes = entries.iter().map(|entry| entry.1).sum::<u64>();
    let mut count = entries.len();
    for (_, entry_bytes, path) in entries {
        if bytes <= max_bytes && count <= max_entries {
            break;
        }
        if protected.is_some_and(|protected| protected == path) {
            continue;
        }
        let namespace = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str());
        let key = path.file_name().and_then(|value| value.to_str());
        if namespace.zip(key).is_some_and(|(namespace, key)| {
            root.join("locks")
                .join(namespace)
                .join(format!("{key}.lock"))
                .is_dir()
        }) {
            continue;
        }
        if fs::remove_dir_all(&path).is_ok() {
            bytes = bytes.saturating_sub(entry_bytes);
            count = count.saturating_sub(1);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ncc-content-cache-{label}-{}-{}",
            std::process::id(),
            CACHE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn content_cache_roundtrip_rejects_corrupt_artifacts() {
        let root = temp_root("roundtrip");
        let source = root.join("source.obj");
        let restored = root.join("restored.obj");
        fs::create_dir_all(&root).expect("root");
        fs::write(&source, [1u8, 2, 3, 4]).expect("source");
        let material = serde_json::json!({"source": "abc", "target": "x86_64"});
        let key = content_cache_key("native", &material);
        store_content_cache_entry(
            &root,
            "native",
            &key,
            &material,
            serde_json::json!({"route": "test"}),
            &[ContentCacheArtifact::new("object", &source)],
        )
        .expect("store");
        let destinations = BTreeMap::from([("object".to_string(), restored.clone())]);
        let hit = restore_content_cache_entry(&root, "native", &key, &destinations)
            .expect("restore")
            .expect("hit");
        assert_eq!(hit.artifact_count, 1);
        assert_eq!(fs::read(&restored).expect("restored"), [1, 2, 3, 4]);

        let entry = cache_entry_dir(&root, "native", &key);
        let artifact = fs::read_dir(&entry)
            .expect("entry")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.file_name().and_then(|value| value.to_str()) != Some("manifest.json"))
            .expect("artifact");
        fs::write(artifact, [9u8, 9, 9, 9]).expect("corrupt");
        assert!(
            restore_content_cache_entry(&root, "native", &key, &destinations)
                .expect("probe")
                .is_none()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn content_cache_key_changes_with_material() {
        let a = content_cache_key("mct", &serde_json::json!({"source": "a"}));
        let b = content_cache_key("mct", &serde_json::json!({"source": "b"}));
        assert_ne!(a, b);
    }

    #[test]
    fn content_cache_roundtrips_metadata_only_entries() {
        let root = temp_root("metadata");
        let material = serde_json::json!({"linked_ast": "abc"});
        let key = content_cache_key("frontend-sema", &material);
        store_content_cache_entry(
            &root,
            "frontend-sema",
            &key,
            &material,
            serde_json::json!({"warnings": ["one"]}),
            &[],
        )
        .expect("store metadata");
        let hit = restore_content_cache_entry(&root, "frontend-sema", &key, &BTreeMap::new())
            .expect("restore metadata")
            .expect("metadata hit");
        assert_eq!(hit.artifact_count, 0);
        assert_eq!(hit.metadata["warnings"][0], "one");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_writers_publish_one_complete_entry() {
        let root = temp_root("concurrent");
        fs::create_dir_all(&root).expect("root");
        let source = root.join("source.obj");
        fs::write(&source, [5u8, 8, 13, 21]).expect("source");
        let material = serde_json::json!({"source": "same", "target": "same"});
        let key = content_cache_key("native", &material);
        let mut writers = Vec::new();
        for writer in 0..8 {
            let root = root.clone();
            let source = source.clone();
            let material = material.clone();
            let key = key.clone();
            writers.push(std::thread::spawn(move || {
                store_content_cache_entry(
                    &root,
                    "native",
                    &key,
                    &material,
                    serde_json::json!({"writer": writer}),
                    &[ContentCacheArtifact::new("object", source)],
                )
            }));
        }
        for writer in writers {
            writer.join().expect("writer thread").expect("writer store");
        }

        let restored = root.join("restored.obj");
        let destinations = BTreeMap::from([("object".to_string(), restored.clone())]);
        let hit = restore_content_cache_entry(&root, "native", &key, &destinations)
            .expect("restore")
            .expect("cache hit");
        assert_eq!(hit.artifact_count, 1);
        assert_eq!(fs::read(restored).expect("restored"), [5, 8, 13, 21]);
        let _ = fs::remove_dir_all(root);
    }
}
