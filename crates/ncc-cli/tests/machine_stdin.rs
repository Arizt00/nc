use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const NM_RETURN_42: &str = r#"stdinReturn42[@machine] {
    target:
        arch: x86_64;
        abi: sysv64;
        object: elf64;
        endian: little;
        pointerWidth: 64;
        profile: userland;

    symbol:
        entry:
            section: .text;
            binding: global;
            type: routine;

    entry:
        entry;

    proc:
        entry:
            abi: sysv64;

    instr:
        mov rax, 42;
        ret;

    verify:
        target;
        abi;
        symbols;
        encoding;
        object;

    seal:
        artifact: bytes;
        require: verified;
}
"#;

const NASM_RETURN_42: &str = r#"global entry
section .text
entry:
    mov rax, 42
    ret
"#;

const NASM_PHYSICAL_PORT_IO: &str = r#"global transferPortBlocks
section .text
transferPortBlocks:
    insb
    insw
    insd
    outsb
    outsw
    outsd
    ret
"#;

const NASM_KERNEL_SYNC: &str = r#"global waitForKernelEvent
section .text
waitForKernelEvent:
    mfence
    umonitor rax
    umwait eax
    tpause eax
    serialize
    ret
"#;

const WASM_RETURN_42: &str = r#"wasmPipe[@machine] {
    target:
        arch: wasm32;
        abi: wasm;
        object: wasm;
        endian: little;
        pointerWidth: 32;
        profile: userland;
    symbol:
        main:
            section: .text;
            binding: global;
            type: routine;
    entry:
        main;
    proc:
        main:
            abi: wasm;
    instr:
        i32.const 42;
        return;
    verify:
        target;
        abi;
        symbols;
        encoding;
        object;
    seal:
        artifact: object;
        require: verified;
}
"#;

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_temp_file(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-machine-stdin-{}-{nanos}-{name}",
        std::process::id()
    ))
}

fn pipe_ncc(args: &[&str], input: &[u8]) -> Output {
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
        .write_all(input)
        .expect("write stdin");
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
fn machine_stdin_accepts_nm_and_nasm_and_materializes_real_bytes() {
    let check = pipe_ncc(
        &["machine", "stdin", "check", "--syntax", "nm"],
        NM_RETURN_42.as_bytes(),
    );
    assert!(check.status.success(), "{}", output_text(&check));

    let bytes_path = unique_temp_file("return42.bin");
    let emit = pipe_ncc(
        &[
            "machine",
            "stdin",
            "emit-bytes",
            "--syntax",
            "nasm",
            "--out",
            bytes_path.to_str().expect("UTF-8 temp path"),
        ],
        NASM_RETURN_42.as_bytes(),
    );
    assert!(emit.status.success(), "{}", output_text(&emit));
    let bytes = fs::read(&bytes_path).expect("emitted bytes");
    assert!(!bytes.is_empty());
    assert_eq!(bytes.last(), Some(&0xc3));

    if cfg!(target_arch = "x86_64") {
        let run = pipe_ncc(
            &[
                "machine",
                "stdin",
                "run-bytes",
                "--syntax",
                "nm",
                "--expect-return",
                "42",
                "--json",
            ],
            NM_RETURN_42.as_bytes(),
        );
        assert!(run.status.success(), "{}", output_text(&run));
        let payload: serde_json::Value =
            serde_json::from_slice(&run.stdout).expect("run JSON payload");
        assert_eq!(payload["return"], 42);
        assert_eq!(payload["permissions"], "rw_to_rx");
        assert_eq!(payload["isolated_child"], true);
    }

    let wasm_path = unique_temp_file("return42.wasm");
    let emit_wasm = pipe_ncc(
        &[
            "machine",
            "stdin",
            "emit-wasm",
            "--syntax",
            "nm",
            "--target",
            "wasm32-unknown-unknown",
            "--out",
            wasm_path.to_str().expect("UTF-8 temp path"),
        ],
        WASM_RETURN_42.as_bytes(),
    );
    assert!(emit_wasm.status.success(), "{}", output_text(&emit_wasm));
    let wasm = fs::read(&wasm_path).expect("WASM module");
    assert!(wasm.starts_with(b"\0asm"));
    let verify_wasm = pipe_ncc(
        &[
            "machine",
            "stdin",
            "verify-object",
            "--syntax",
            "wasm",
            "--json",
        ],
        &wasm,
    );
    assert!(
        verify_wasm.status.success(),
        "{}",
        output_text(&verify_wasm)
    );
    let wasm_report: serde_json::Value =
        serde_json::from_slice(&verify_wasm.stdout).expect("WASM verification JSON");
    assert_eq!(wasm_report["object_format"], "wasm");
    assert_eq!(wasm_report["status"], "object_verified");

    let _ = fs::remove_file(bytes_path);
    let _ = fs::remove_file(wasm_path);
}

#[test]
fn machine_help_exposes_terminal_and_autonomous_kernel_routes() {
    let output = Command::new(ncc_bin())
        .args(["machine", "--help"])
        .output()
        .expect("machine help must execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["stdin", "pipe", "run-bytes", "kernel-system"] {
        assert!(
            stdout.contains(command),
            "machine help must advertise {command}: {stdout}"
        );
    }

    let kernel = Command::new(ncc_bin())
        .args(["machine", "kernel-system", "--help"])
        .output()
        .expect("kernel-system help must execute");
    assert!(kernel.status.success());
    assert!(String::from_utf8_lossy(&kernel.stdout).contains("autonomous"));
}

#[test]
fn machine_stdin_propagates_kernel_profile_before_importing_privileged_asm() {
    let bytes_path = unique_temp_file("physical-port-io.bin");
    let emit = pipe_ncc(
        &[
            "machine",
            "stdin",
            "emit-bytes",
            "--syntax",
            "nasm",
            "--profile",
            "kernel",
            "--out",
            bytes_path.to_str().expect("UTF-8 temp path"),
        ],
        NASM_PHYSICAL_PORT_IO.as_bytes(),
    );
    assert!(emit.status.success(), "{}", output_text(&emit));
    assert_eq!(
        fs::read(&bytes_path).expect("physical port bytes"),
        [0x6c, 0x66, 0x6d, 0x6d, 0x6e, 0x66, 0x6f, 0x6f, 0xc3]
    );

    let rejected = pipe_ncc(
        &["machine", "stdin", "check", "--syntax", "nasm"],
        NASM_PHYSICAL_PORT_IO.as_bytes(),
    );
    assert!(!rejected.status.success());
    assert!(output_text(&rejected).contains("E_MACHINE_PRIVILEGE_001"));
    let _ = fs::remove_file(bytes_path);
}

#[test]
fn machine_stdin_materializes_feature_gated_kernel_synchronization_isa() {
    let bytes = unique_temp_file("kernel-sync.bin");
    let output = pipe_ncc(
        &[
            "machine",
            "stdin",
            "emit-bytes",
            "--syntax",
            "nasm",
            "--profile",
            "kernel",
            "--features",
            "waitpkg,serialize",
            "--out",
            bytes.to_str().unwrap(),
        ],
        NASM_KERNEL_SYNC.as_bytes(),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&bytes).unwrap(),
        [
            0x0f, 0xae, 0xf0, 0xf3, 0x48, 0x0f, 0xae, 0xf0, 0xf2, 0x0f, 0xae, 0xf0, 0x66, 0x0f,
            0xae, 0xf0, 0x0f, 0x01, 0xe8, 0xc3,
        ]
    );

    let rejected = pipe_ncc(
        &[
            "machine",
            "stdin",
            "check",
            "--syntax",
            "nasm",
            "--profile",
            "kernel",
        ],
        NASM_KERNEL_SYNC.as_bytes(),
    );
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("feature 'waitpkg'"));
    let _ = fs::remove_file(bytes);
}
