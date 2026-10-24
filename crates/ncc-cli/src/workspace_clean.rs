use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PathUsage {
    pub(crate) bytes: u64,
    pub(crate) files: u64,
    pub(crate) directories: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanedDirectory {
    pub(crate) label: String,
    pub(crate) path: PathBuf,
    pub(crate) usage: PathUsage,
}

pub(crate) fn path_usage(path: &Path) -> io::Result<PathUsage> {
    let mut usage = PathUsage::default();
    let mut pending = vec![path.to_path_buf()];

    while let Some(current) = pending.pop() {
        let metadata = fs::symlink_metadata(&current)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() || metadata.is_file() {
            usage.files = usage.files.saturating_add(1);
            usage.bytes = usage.bytes.saturating_add(metadata.len());
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }

        usage.directories = usage.directories.saturating_add(1);
        for entry in fs::read_dir(&current)? {
            pending.push(entry?.path());
        }
    }

    Ok(usage)
}

fn canonical_contained_directory(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let root = fs::canonicalize(root)?;
    let metadata = fs::symlink_metadata(candidate)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "refusing to clean symbolic-link directory '{}'",
                candidate.display()
            ),
        ));
    }
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("clean target '{}' is not a directory", candidate.display()),
        ));
    }

    let resolved = fs::canonicalize(candidate)?;
    if resolved == root || !resolved.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to clean '{}' outside root '{}'",
                resolved.display(),
                root.display()
            ),
        ));
    }
    Ok(resolved)
}

pub(crate) fn clean_directory_within(
    root: &Path,
    candidate: &Path,
    label: impl Into<String>,
    dry_run: bool,
) -> io::Result<Option<CleanedDirectory>> {
    if !candidate.exists() {
        return Ok(None);
    }

    let resolved = canonical_contained_directory(root, candidate)?;
    let usage = path_usage(&resolved)?;
    if !dry_run {
        fs::remove_dir_all(&resolved)?;
    }

    Ok(Some(CleanedDirectory {
        label: label.into(),
        path: resolved,
        usage,
    }))
}

fn looks_like_cargo_target_directory(path: &Path) -> bool {
    ["debug/.fingerprint", "release/.fingerprint"]
        .iter()
        .any(|rel| path.join(rel).is_dir())
}

pub(crate) fn cargo_build_directories(root: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    if !root.join("Cargo.toml").is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "build cleanup requires a Cargo workspace root; '{}' has no Cargo.toml",
                root.display()
            ),
        ));
    }

    let target = root.join("target");
    if !target.is_dir() {
        return Ok(Vec::new());
    }

    let mut candidates = vec![
        ("cargo.debug".to_string(), target.join("debug")),
        ("cargo.tmp".to_string(), target.join("tmp")),
        ("cargo.ncup".to_string(), target.join("ncup")),
    ];
    for rel in ["deps", "build", ".fingerprint", "incremental", "examples"] {
        candidates.push((
            format!("cargo.release.{rel}"),
            target.join("release").join(rel),
        ));
    }

    for entry in fs::read_dir(&target)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(name.as_str(), "debug" | "release" | "tmp" | "ncup") {
            continue;
        }
        if entry.file_type()?.is_dir() && looks_like_cargo_target_directory(&path) {
            candidates.push((format!("cargo.target.{name}"), path));
        }
    }

    candidates.sort_by(|left, right| left.1.cmp(&right.1));
    candidates.dedup_by(|left, right| left.1 == right.1);
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time must advance")
            .as_nanos();
        std::env::temp_dir().join(format!("ncc_{label}_{}_{}", std::process::id(), stamp))
    }

    #[test]
    fn usage_counts_files_without_following_symlinks() {
        let root = temp_root("clean_usage");
        fs::create_dir_all(root.join("nested")).expect("fixture directory");
        fs::write(root.join("a.bin"), [1u8, 2, 3]).expect("fixture file");
        fs::write(root.join("nested/b.bin"), [4u8, 5]).expect("fixture file");

        let usage = path_usage(&root).expect("usage must be measurable");
        assert_eq!(usage.bytes, 5);
        assert_eq!(usage.files, 2);
        assert_eq!(usage.directories, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cargo_candidates_preserve_release_binaries_and_evidence() {
        let root = temp_root("clean_candidates");
        fs::create_dir_all(root.join("target/release/deps")).expect("release deps");
        fs::create_dir_all(root.join("target/evidence")).expect("evidence");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("manifest");
        fs::write(root.join("target/release/ncc.exe"), b"exe").expect("release binary");

        let candidates = cargo_build_directories(&root).expect("candidates");
        assert!(candidates
            .iter()
            .any(|(_, path)| path.ends_with("target/release/deps")));
        assert!(!candidates
            .iter()
            .any(|(_, path)| path.ends_with("target/release/ncc.exe")));
        assert!(!candidates
            .iter()
            .any(|(_, path)| path.ends_with("target/evidence")));
        let _ = fs::remove_dir_all(root);
    }
}
