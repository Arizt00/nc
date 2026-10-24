#![cfg(all(unix, target_arch = "x86_64"))]

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
        .expect("workspace root must resolve")
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-closure-executable-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn path_with_ncc_first(ncc: &Path) -> String {
    let mut paths = vec![ncc.parent().expect("binary parent").to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .expect("PATH must be joinable")
        .to_string_lossy()
        .to_string()
}

fn run_ncc(args: &[String]) -> Output {
    let ncc = ncc_bin();
    Command::new(&ncc)
        .current_dir(workspace_root())
        .args(args)
        .env("PATH", path_with_ncc_first(&ncc))
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn closures_capture_invoke_and_escape_in_real_elf() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }

    let root = workspace_root();
    let source = root.join("examples/03_types/closureExecutable.n");
    let out_dir = unique_temp_dir();

    let interpreted = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--interpreted".into(),
        "--show-return".into(),
    ]);
    assert!(interpreted.status.success(), "{}", rendered(&interpreted));
    assert!(rendered(&interpreted).contains("[return: true"));

    let build = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(build.status.success(), "{}", rendered(&build));
    let json: serde_json::Value = serde_json::from_slice(&build.stdout)
        .unwrap_or_else(|err| panic!("native build JSON: {err}\n{}", rendered(&build)));
    assert_eq!(json["status"], "native_build_passed");

    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let backend_object = PathBuf::from(
        json["paths"]["machine_backend_object_ir"]
            .as_str()
            .expect("backend object IR path"),
    );
    let backend_text = std::fs::read_to_string(&backend_object).expect("backend object IR");
    assert!(backend_text.contains("scalar-call-indirect"));
    assert!(backend_text.contains(".__invoke"));
    assert!(!backend_text.contains(".__invoke_trap"));

    let execution = Command::new(&executable)
        .current_dir(&root)
        .output()
        .expect("external ELF closure program");
    assert!(execution.status.success(), "{}", rendered(&execution));

    std::fs::remove_dir_all(out_dir).expect("temporary build tree clean");
}

#[test]
fn closures_capture_invoke_and_escape_in_real_i386_elf32() {
    if Command::new("clang").arg("--version").output().is_err()
        || !Path::new("/lib/ld-linux.so.2").exists()
    {
        return;
    }

    let root = workspace_root();
    let source = root.join("examples/03_types/closureExecutable.n");
    let out_dir = unique_temp_dir();
    let build = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "i386-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(build.status.success(), "{}", rendered(&build));
    let json: serde_json::Value = serde_json::from_slice(&build.stdout)
        .unwrap_or_else(|err| panic!("native ELF32 build JSON: {err}\n{}", rendered(&build)));
    assert_eq!(json["status"], "native_build_passed");
    assert_eq!(json["target"]["object_format"], "elf32");

    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("ELF32 executable path"),
    );
    let bytes = std::fs::read(&executable).expect("ELF32 closure executable");
    assert_eq!(bytes.get(0..5), Some(b"\x7fELF\x01".as_slice()));
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 3);

    let execution = Command::new(&executable)
        .current_dir(&root)
        .output()
        .expect("external ELF32 closure program");
    assert!(execution.status.success(), "{}", rendered(&execution));

    std::fs::remove_dir_all(out_dir).expect("temporary ELF32 build tree clean");
}
