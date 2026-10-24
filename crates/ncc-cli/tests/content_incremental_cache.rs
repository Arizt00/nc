#![cfg(unix)]

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-content-incremental-e2e-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn run_build(source: &Path, out: &Path, cache: &Path) -> Output {
    Command::new(ncc_bin())
        .args([
            "compiler",
            "build-native",
            source.to_str().expect("source path"),
            "--out-dir",
            out.to_str().expect("out path"),
            "--content-cache-dir",
            cache.to_str().expect("cache path"),
            "--profile",
            "release-fast",
            "--lto",
            "off",
            "--pgo",
            "off",
            "--no-run",
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc build-native")
}

fn parse_success(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("native build JSON")
}

fn sha256(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("artifact bytes");
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn native_content_cache_reuses_identical_o3_and_recovers_from_corruption() {
    if std::env::consts::OS != "linux"
        || std::env::consts::ARCH != "x86_64"
        || !tool_available("clang")
    {
        return;
    }

    let root = temp_dir();
    let source = root.join("incremental_case.n");
    std::fs::copy(
        workspace_root().join("_archive/old_examples/performance/native_fast/minimal_return.n"),
        &source,
    )
    .expect("copy source");
    let cache = root.join("content-cache");

    let first = parse_success(&run_build(&source, &root.join("first"), &cache));
    assert_eq!(first["status"], "native_build_passed", "{first:#?}");
    assert_eq!(first["incremental"]["cache_hit"], false, "{first:#?}");
    assert_eq!(first["incremental"]["stored"], true, "{first:#?}");

    let second = parse_success(&run_build(&source, &root.join("second"), &cache));
    assert_eq!(second["incremental"]["cache_hit"], true, "{second:#?}");
    assert!(
        second["incremental"]["restored_artifacts"]
            .as_u64()
            .is_some_and(|count| count >= 3),
        "{second:#?}"
    );
    for kind in ["object", "executable", "llvm_ir"] {
        let first_path = PathBuf::from(first["paths"][kind].as_str().expect("first path"));
        let second_path = PathBuf::from(second["paths"][kind].as_str().expect("second path"));
        assert_eq!(sha256(&first_path), sha256(&second_path), "{kind}");
    }

    let key = second["incremental"]["key"].as_str().expect("cache key");
    let entry = cache.join("entries").join("compiler-native-o3").join(key);
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(entry.join("manifest.json")).expect("cache manifest"),
    )
    .expect("cache manifest JSON");
    let object_file = manifest["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .find(|artifact| artifact["kind"] == "object")
        .and_then(|artifact| artifact["file"].as_str())
        .expect("cached object");
    std::fs::write(entry.join(object_file), b"corrupt").expect("corrupt object");

    let recovered = parse_success(&run_build(&source, &root.join("recovered"), &cache));
    assert_eq!(
        recovered["incremental"]["cache_hit"], false,
        "a corrupt entry must never be reused: {recovered:#?}"
    );
    assert_eq!(recovered["incremental"]["stored"], true, "{recovered:#?}");

    let mut changed = std::fs::read_to_string(&source).expect("source");
    changed.push_str("\n// content-key mutation\n");
    std::fs::write(&source, changed).expect("mutate source");
    let mutated = parse_success(&run_build(&source, &root.join("mutated"), &cache));
    assert_eq!(mutated["incremental"]["cache_hit"], false, "{mutated:#?}");
    assert_ne!(
        recovered["incremental"]["key"],
        mutated["incremental"]["key"]
    );

    let _ = std::fs::remove_dir_all(root);
}
