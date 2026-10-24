#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn ncc_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
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

fn run_ncc(args: &[String], cwd: &Path) -> Output {
    Command::new(ncc_bin())
        .current_dir(cwd)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc must run")
}

fn rendered(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn clang_supports(target: &str) -> bool {
    Command::new("clang")
        .args(["--print-targets"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).contains(target))
}

fn build_target(root: &Path, source: &Path, target: &str, out_dir: &Path) -> Output {
    run_ncc(
        &[
            "build".into(),
            source.display().to_string(),
            "--target".into(),
            target.into(),
            "--profile".into(),
            "release-fast".into(),
            "--out-dir".into(),
            out_dir.display().to_string(),
            "--json".into(),
        ],
        root,
    )
}

#[test]
fn direct_atomic_type_emits_valid_llvm_and_real_x86_object() {
    if Command::new("clang").arg("--version").output().is_err() {
        return;
    }
    let root = workspace_root();
    let source = root.join("examples/06_memory/c11_atomic_native.n");
    let temp = temp_dir("c11-atomic-llvm");
    let llvm = temp.join("atomic.ll");
    let emitted = run_ncc(
        &[
            "mct".into(),
            "emit-llvm".into(),
            source.display().to_string(),
            "--out".into(),
            llvm.display().to_string(),
        ],
        &root,
    );
    assert!(emitted.status.success(), "{}", rendered(&emitted));
    let body = std::fs::read_to_string(&llvm).expect("LLVM output");
    for needle in [
        "store atomic i32 7",
        "atomicrmw add",
        "load atomic i32",
        "cmpxchg ptr",
        "fence seq_cst",
        "asm sideeffect \"pause\"",
        "store volatile i32",
        "load volatile i32",
    ] {
        assert!(body.contains(needle), "missing {needle} in:\n{body}");
    }
    assert!(!body.contains("@nC.runtime.atomic"));
    assert!(!body.contains("@atomicLoadOrder"));

    let object = temp.join("atomic.o");
    let clang = Command::new("clang")
        .args(["-c", "-x", "ir"])
        .arg(&llvm)
        .arg("-o")
        .arg(&object)
        .output()
        .expect("clang");
    assert!(clang.status.success(), "{}", rendered(&clang));
    assert!(std::fs::metadata(&object).expect("object").len() > 0);
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn direct_atomic_program_builds_and_executes_on_host() {
    if std::env::consts::ARCH != "x86_64"
        || Command::new("clang").arg("--version").output().is_err()
    {
        return;
    }
    let root = workspace_root();
    let source = root.join("examples/06_memory/c11_atomic_native.n");
    let temp = temp_dir("c11-atomic-host");
    let output = build_target(&root, &source, "x86_64-unknown-linux-gnu", &temp);
    assert!(output.status.success(), "{}", rendered(&output));
    let executable = temp.join("bin/c11_atomic_native.exe");
    let run = Command::new(&executable)
        .output()
        .expect("native executable");
    assert!(run.status.success(), "{}", rendered(&run));
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn aarch64_and_riscv64_objects_retain_target_atomic_instructions() {
    let root = workspace_root();
    let source = root.join("examples/06_memory/c11_atomic_native.n");

    if clang_supports("aarch64") {
        let temp = temp_dir("c11-atomic-aarch64");
        let output = build_target(&root, &source, "aarch64-unknown-linux-gnu", &temp);
        assert!(output.status.success(), "{}", rendered(&output));
        let llvm = std::fs::read_to_string(temp.join("lowering/c11_atomic_native.standard.mct.ll"))
            .expect("AArch64 retained LLVM");
        assert!(llvm.contains("asm sideeffect \"wfe\""), "{llvm}");
        assert!(llvm.contains("asm sideeffect \"sev\""), "{llvm}");
        assert!(
            std::fs::metadata(temp.join("obj/c11_atomic_native.obj"))
                .expect("AArch64 object")
                .len()
                > 0
        );
        let _ = std::fs::remove_dir_all(temp);
    }

    if clang_supports("riscv64") {
        let temp = temp_dir("c11-atomic-riscv64");
        let output = build_target(&root, &source, "riscv64-unknown-linux-gnu", &temp);
        assert!(output.status.success(), "{}", rendered(&output));
        let llvm = std::fs::read_to_string(temp.join("lowering/c11_atomic_native.standard.mct.ll"))
            .expect("RISC-V retained LLVM");
        assert!(llvm.contains(".word 0x0100000f"), "{llvm}");
        assert!(llvm.contains("fence rw, rw"), "{llvm}");
        let executable = temp.join("bin/c11_atomic_native.exe");
        assert!(std::fs::metadata(&executable).expect("RISC-V ELF").len() > 0);
        if Command::new("qemu-riscv64")
            .arg("--version")
            .output()
            .is_ok()
        {
            let run = Command::new("qemu-riscv64")
                .arg(&executable)
                .output()
                .expect("qemu-riscv64");
            assert!(run.status.success(), "{}", rendered(&run));
        }
        let _ = std::fs::remove_dir_all(temp);
    }
}
