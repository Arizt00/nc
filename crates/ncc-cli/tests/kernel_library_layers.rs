#![cfg(unix)]

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

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ncc-{label}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run(args: &[String], cwd: &Path) -> Output {
    Command::new(ncc_bin())
        .current_dir(cwd)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc must run")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .replace("\r\n", "\n")
}

#[test]
fn kernel_alloc_is_explicit_self_contained_and_libc_free() {
    let root = workspace_root();
    let source = root.join("examples/11_machine/kernel_alloc_freestanding.n");
    let temp = temp_dir("kernel-library-layers");

    let common = vec![
        "build".into(),
        source.display().to_string(),
        "--profile".into(),
        "kernel".into(),
        "--target".into(),
        "x86_64-unknown-none".into(),
    ];
    let denied = run(&common, &root);
    assert!(!denied.status.success());
    assert!(text(&denied).contains("E_KERNEL_ALLOC_UNAVAILABLE_001"));

    let out_dir = temp.join(".ncc");
    let mut enabled = common;
    enabled.extend([
        "--kernel-alloc".into(),
        "--out-dir".into(),
        out_dir.display().to_string(),
        "--json".into(),
    ]);
    let built = run(&enabled, &root);
    assert!(
        built.status.success(),
        "freestanding alloc build failed: {}",
        text(&built)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&built.stdout).expect("kernel build JSON");
    assert_eq!(payload["library_layers"]["core"]["requires_os"], false);
    assert_eq!(payload["library_layers"]["alloc"]["enabled"], true);
    assert_eq!(payload["library_layers"]["alloc"]["requires_libc"], false);
    assert_eq!(payload["library_layers"]["stoud"]["enabled"], false);
    assert_eq!(
        payload["kernel_contract"]["full_high_level_kernel_body_instruction_lowering"],
        true
    );
    assert_eq!(
        payload["link"]["phase12_report"]["unresolved_symbols"],
        serde_json::json!([])
    );

    let llvm_path = PathBuf::from(
        payload["paths"]["high_level_body_llvm_ir"]
            .as_str()
            .expect("retained allocator LLVM"),
    );
    let llvm = std::fs::read_to_string(llvm_path).expect("allocator LLVM reads");
    assert!(llvm.contains("define ptr @malloc"));
    assert!(llvm.contains("define void @free"));
    assert!(llvm.contains("define ptr @calloc"));
    assert!(llvm.contains("define ptr @realloc"));
    assert!(!llvm.contains("declare ptr @malloc"));
    assert!(!llvm.contains("declare void @free"));
    assert!(!llvm.contains("declare ptr @calloc"));
    assert!(!llvm.contains("declare ptr @realloc"));

    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn kernel_rejects_hosted_stoud_before_backend_materialization() {
    let root = workspace_root();
    let temp = temp_dir("kernel-hosted-rejection");
    let source = temp.join("hosted_kernel.n");
    std::fs::write(
        &source,
        r#"use::stoud::fs::<file>;

hostedKernel {
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    )
    .expect("hosted kernel fixture");
    let output = run(
        &[
            "build".into(),
            source.display().to_string(),
            "--profile".into(),
            "kernel".into(),
            "--target".into(),
            "x86_64-unknown-none".into(),
            "--out-dir".into(),
            temp.join(".ncc").display().to_string(),
        ],
        &root,
    );
    assert!(!output.status.success());
    assert!(text(&output).contains("E_KERNEL_HOSTED_DEPENDENCY_001"));
    assert!(!temp.join(".ncc/obj").exists());
    let _ = std::fs::remove_dir_all(temp);
}
