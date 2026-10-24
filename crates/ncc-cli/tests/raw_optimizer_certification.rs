#![cfg(all(unix, target_arch = "x86_64"))]

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const EXPECTED: &str = "3048\n42\n";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-raw-optimizer-cert-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create raw optimizer temp dir");
    path
}

fn available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_executable(path: &Path) -> String {
    let output = Command::new(path)
        .output()
        .expect("execute certified binary");
    assert!(output.status.success(), "{}", combined(&output));
    String::from_utf8(output.stdout).expect("UTF-8 oracle output")
}

fn compile_c_oracle(cc: &str, level: &str, root: &Path) -> (PathBuf, PathBuf) {
    let source = workspace_root().join("crates/ncc-cli/tests/raw_optimizer_oracle.c");
    let object = root.join(format!("oracle-{cc}-{level}.o"));
    let executable = root.join(format!("oracle-{cc}-{level}"));
    let compile_object = Command::new(cc)
        .args([
            "-std=c11",
            level,
            "-fno-strict-aliasing",
            "-c",
            source.to_str().expect("C source path"),
            "-o",
            object.to_str().expect("C object path"),
        ])
        .output()
        .expect("compile C oracle object");
    assert!(
        compile_object.status.success(),
        "{}",
        combined(&compile_object)
    );
    let link = Command::new(cc)
        .args([
            "-std=c11",
            level,
            "-fno-strict-aliasing",
            source.to_str().expect("C source path"),
            "-o",
            executable.to_str().expect("C executable path"),
        ])
        .output()
        .expect("link C oracle");
    assert!(link.status.success(), "{}", combined(&link));
    (object, executable)
}

#[test]
fn raw_optimizer_matches_gcc_clang_and_retains_memory_observability() {
    if !available("clang") || !available("gcc") || !available("objdump") {
        eprintln!(
            "raw optimizer differential certification skipped: clang, gcc and objdump are required"
        );
        return;
    }

    let temp = temp_dir();
    let nc_out = temp.join("nc");
    let source = workspace_root().join("examples/06_memory/raw_optimizer_memory_intrinsics.n");
    let build = Command::new(env!("CARGO_BIN_EXE_ncc"))
        .args([
            "compiler",
            "build-native",
            source.to_str().expect("nC source path"),
            "--out-dir",
            nc_out.to_str().expect("nC out dir"),
            "--cc",
            "nC.sysv64",
            "-O3",
            "--no-run",
            "--strict",
            "--json",
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc compiler build-native");
    assert!(build.status.success(), "{}", combined(&build));
    let report: Value = serde_json::from_slice(&build.stdout).expect("native build JSON");
    assert_eq!(report["status"], "native_build_passed", "{report:#}");

    let executable = PathBuf::from(
        report["paths"]["executable"]
            .as_str()
            .expect("nC executable"),
    );
    let object = PathBuf::from(report["paths"]["object"].as_str().expect("nC object"));
    let llvm = PathBuf::from(report["paths"]["llvm_ir"].as_str().expect("nC LLVM IR"));
    assert_eq!(run_executable(&executable), EXPECTED);

    let llvm_text = std::fs::read_to_string(&llvm).expect("read nC LLVM IR");
    assert!(llvm_text.contains("call ptr @memcpy("));
    assert!(llvm_text.contains("call ptr @memmove("));
    assert!(llvm_text.contains("call ptr @memset("));
    assert!(llvm_text.contains("load volatile i32"));
    assert!(llvm_text.contains("store volatile i32"));
    assert!(llvm_text.contains("atomicrmw add"));
    assert!(llvm_text.contains("fence seq_cst"));
    assert!(!llvm_text.contains("!tbaa"));
    assert!(!llvm_text.contains("!alias.scope"));
    assert!(!llvm_text.contains("!noalias"));

    let nc_disasm = Command::new("objdump")
        .args(["-d", object.to_str().expect("nC object path")])
        .output()
        .expect("disassemble nC object");
    assert!(nc_disasm.status.success(), "{}", combined(&nc_disasm));
    let nc_asm = String::from_utf8_lossy(&nc_disasm.stdout);
    assert!(
        nc_asm.contains("rawOptimizerMemoryIntrinsics_run"),
        "{nc_asm}"
    );

    for cc in ["clang", "gcc"] {
        for level in ["-O2", "-O3"] {
            let (c_object, c_executable) = compile_c_oracle(cc, level, &temp);
            assert_eq!(run_executable(&c_executable), EXPECTED, "{cc} {level}");
            let disasm = Command::new("objdump")
                .args(["-d", c_object.to_str().expect("C object path")])
                .output()
                .expect("disassemble C object");
            assert!(disasm.status.success(), "{}", combined(&disasm));
            assert!(!disasm.stdout.is_empty(), "{cc} {level} empty disassembly");
        }
    }

    let _ = std::fs::remove_dir_all(temp);
}
