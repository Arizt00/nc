use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const X86_DEBUG_ASM: &str = r#"global debugEntry
section .text
debugEntry:
    int3
    int1
    ud2
    mov rax, dr0
    mov dr7, rax
    ret
"#;

const AARCH64_DEBUG_NM: &str = r#"aarch64Debug[@machine] {
    target:
        arch: aarch64;
        abi: aapcs64;
        object: elf64;
        endian: little;
        pointerWidth: 64;
        profile: kernel;
    symbol:
        entry:
            section: .text;
            binding: global;
            type: routine;
    entry:
        entry;
    proc:
        entry:
            abi: aapcs64;
    instr:
        brk 0x1234;
        hlt 0x2345;
        ret;
    verify:
        target;
        abi;
        symbols;
        encoding;
    seal:
        artifact: bytes;
        require: verified;
}
"#;

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_file(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-machine-debug-stdin-{}-{nonce}-{name}",
        std::process::id()
    ))
}

fn pipe(args: &[&str], source: &[u8]) -> Output {
    let mut child = Command::new(ncc_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ncc");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(source)
        .expect("write source");
    child.wait_with_output().expect("wait ncc")
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn debug_isa_is_materialized_from_shell_neutral_stdin_without_external_assembler() {
    let x86_path = unique_file("x86.bin");
    let x86 = pipe(
        &[
            "machine",
            "stdin",
            "emit-bytes",
            "--syntax",
            "nasm",
            "--profile",
            "kernel",
            "--out",
            x86_path.to_str().expect("UTF-8 x86 path"),
        ],
        X86_DEBUG_ASM.as_bytes(),
    );
    assert!(x86.status.success(), "{}", output_text(&x86));
    assert_eq!(
        fs::read(&x86_path).expect("x86 bytes"),
        [0xcc, 0xf1, 0x0f, 0x0b, 0x0f, 0x21, 0xc0, 0x0f, 0x23, 0xf8, 0xc3]
    );

    let aarch64_path = unique_file("aarch64.bin");
    let aarch64 = pipe(
        &[
            "machine",
            "stdin",
            "emit-bytes",
            "--syntax",
            "nm",
            "--out",
            aarch64_path.to_str().expect("UTF-8 aarch64 path"),
        ],
        AARCH64_DEBUG_NM.as_bytes(),
    );
    assert!(aarch64.status.success(), "{}", output_text(&aarch64));
    assert_eq!(
        fs::read(&aarch64_path).expect("aarch64 bytes"),
        [0x80, 0x46, 0x22, 0xd4, 0xa0, 0x68, 0x44, 0xd4, 0xc0, 0x03, 0x5f, 0xd6]
    );

    let _ = fs::remove_file(x86_path);
    let _ = fs::remove_file(aarch64_path);
}
