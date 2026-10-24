#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn example_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    for base in ["examples", "_archive/old_examples"] {
        let candidate = root.join(base).join(relative);
        if candidate.exists() {
            return candidate;
        }
    }
    root.join("examples").join(relative)
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ncc-{label}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir must create");
    dir
}

fn assert_no_materialized_c_sources(root: &std::path::Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("artifact directory") {
            let path = entry.expect("artifact entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                assert!(
                    !path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("c")),
                    "compiler-owned C source leaked into .ncc artifacts: {}",
                    path.display()
                );
            }
        }
    }
}

#[cfg(unix)]
fn fake_linker(dir: &std::path::Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("fake-linker.sh");
    std::fs::write(
        &path,
        r#"#!/usr/bin/env bash
set -eu
out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
    break
  fi
  shift
done
if [ -z "$out" ]; then
  exit 64
fi
cat > "$out" <<'EOS'
#!/usr/bin/env bash
exit 0
EOS
chmod +x "$out"
"#,
    )
    .expect("fake linker must write");
    let mut perms = std::fs::metadata(&path)
        .expect("fake linker metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("fake linker must be executable");
    path
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_ncc_with_stdin_in_dir(args: &[String], stdin: &str, cwd: &std::path::Path) -> Output {
    let mut child = Command::new(ncc_bin())
        .current_dir(cwd)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc command with stdin must execute");
    child
        .stdin
        .take()
        .expect("ncc stdin pipe must exist")
        .write_all(stdin.as_bytes())
        .expect("ncc stdin must accept fixture bytes");
    child.wait_with_output().expect("ncc command must finish")
}

fn run_ncc_in_dir(args: &[String], cwd: &std::path::Path) -> Output {
    Command::new(ncc_bin())
        .current_dir(cwd)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn qemu_system_binary(name: &str) -> Option<PathBuf> {
    if Command::new(name).arg("--version").output().is_ok() {
        return Some(PathBuf::from(name));
    }
    #[cfg(windows)]
    {
        let candidate = PathBuf::from(r"C:\Program Files\qemu").join(format!("{name}.exe"));
        if candidate.exists() && Command::new(&candidate).arg("--version").output().is_ok() {
            return Some(candidate);
        }
    }
    None
}

fn run_command_with_timeout(mut command: Command, timeout: Duration) -> std::io::Result<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return child.wait_with_output();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn decode_hex_bytes(raw: &str) -> Vec<u8> {
    assert_eq!(raw.len() % 2, 0, "hex payload must have even length");
    (0..raw.len())
        .step_by(2)
        .map(|idx| u8::from_str_radix(&raw[idx..idx + 2], 16).expect("hex byte must parse"))
        .collect()
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("u16 slice must fit"),
    )
}

#[cfg(windows)]
#[test]
fn ncc_nbuild_windows_pe_executes_string_concat_without_shadow_space_clobber() {
    let temp = unique_temp_dir("windows-pe-string-concat");
    let source = temp.join("native_string_concat_smoke.n");
    std::fs::write(
        &source,
        r#"nativeStringConcatSmoke {
    stc:
        s >> str = "";

    fn:
        run() -> bool {
            public:
                s = "a" + "b";
                return true;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    )
    .expect("string concat smoke source must write");
    let out_dir = temp.join(".ncc");
    let out = run_ncc(&[
        "nbuild".into(),
        source.to_string_lossy().to_string(),
        "--target".into(),
        "x86_64-pc-windows-msvc".into(),
        "--cc".into(),
        "nC.win64".into(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "nbuild must generate PE64 executable\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("nbuild json must parse");
    assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
    assert_eq!(json["status"], "native_build_passed");
    assert_eq!(
        json["truth_boundary"]["object_is_optimized_backend_output"],
        true
    );
    assert_eq!(json["truth_boundary"]["optimized_ir_report_retained"], true);
    assert_eq!(
        json["truth_boundary"]["machine_backend_object_ir_retained"],
        true
    );
    assert_eq!(json["optimization"]["route"], "standard-mct-machine");
    assert_eq!(
        json["speed_contract"]["complete_artifact_set"],
        serde_json::json!([
            ".nb",
            ".nbb",
            ".optimizer.json",
            ".backend-object.nbo",
            ".standard.mct.ll",
            ".init",
            ".nm",
            ".obj",
            ".entry.obj",
            ".exe"
        ])
    );
    assert!(
        json["pipeline"]
            .as_array()
            .expect("pipeline array")
            .iter()
            .any(|step| step.as_str() == Some("emit optimized MCT LLVM for requested target")),
        "{json:#?}"
    );
    for path in [
        out_dir
            .join("lowering")
            .join("native_string_concat_smoke.optimizer.json"),
        out_dir
            .join("lowering")
            .join("native_string_concat_smoke.backend-object.nbo"),
        out_dir
            .join("lowering")
            .join("native_string_concat_smoke.standard.mct.ll"),
        out_dir
            .join("lowering")
            .join("native_string_concat_smoke.nm"),
        out_dir.join("obj").join("native_string_concat_smoke.obj"),
        out_dir
            .join("obj")
            .join("native_string_concat_smoke.entry.obj"),
        out_dir.join("bin").join("native_string_concat_smoke.exe"),
    ] {
        assert!(path.is_file(), "{} must exist", path.display());
        assert!(
            std::fs::metadata(&path).expect("artifact metadata").len() > 0,
            "{} must be non-empty",
            path.display()
        );
    }
    let optimizer_report = out_dir
        .join("lowering")
        .join("native_string_concat_smoke.optimizer.json");
    let optimizer_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&optimizer_report).expect("optimizer report must read"),
    )
    .expect("optimizer report must be json");
    assert_eq!(
        optimizer_json["schema"],
        "nc.compiler.standard_mct_machine_body.v1"
    );
    assert_eq!(optimizer_json["status"], "materialized");
    let exe = out_dir.join("bin").join("native_string_concat_smoke.exe");
    let run = Command::new(&exe)
        .output()
        .unwrap_or_else(|err| panic!("{} must execute: {err}", exe.display()));
    assert!(
        run.status.success(),
        "PE64 executable must map true to process success without access violation\nstatus={:?}\nstdout={}\nstderr={}",
        run.status.code(),
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(windows)]
#[test]
fn ncc_run_windows_pe_internal_executes_normal_n_and_retains_artifacts() {
    let temp = unique_temp_dir("windows-pe-run-normal");
    let source = temp.join("run_string_concat_smoke.n");
    std::fs::write(
        &source,
        r#"use <io:output>;

runStringConcatSmoke :: runtime {
    stc:
        s >> str <- "";

    fn:
        run() -> bool {
            s <- "a" + "b";
            println(s);
            return true;
        }

    ref main ->> run;

    fa:
        return true;
}
"#,
    )
    .expect("run source must write");
    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--native".into(),
            "--cc".into(),
            "nC.win64".into(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "ncc run must execute the PE64 internal route\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("ab"), "stdout={stdout}");
    assert!(
        stdout.contains("route=native-pe64-internal"),
        "stdout={stdout}"
    );
    for path in [
        temp.join(".ncc")
            .join("lowering")
            .join("run_string_concat_smoke.nm"),
        temp.join(".ncc")
            .join("lowering")
            .join("run_string_concat_smoke.backend-object.nbo"),
        temp.join(".ncc")
            .join("obj")
            .join("run_string_concat_smoke.obj"),
        temp.join(".ncc")
            .join("obj")
            .join("run_string_concat_smoke.entry.obj"),
        temp.join(".ncc")
            .join("bin")
            .join("run_string_concat_smoke.exe"),
        temp.join(".ncc")
            .join("run")
            .join("run_string_concat_smoke.native-run.json"),
    ] {
        assert!(path.is_file(), "{} must exist", path.display());
        assert!(
            std::fs::metadata(&path).expect("artifact metadata").len() > 0,
            "{} must be non-empty",
            path.display()
        );
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_sysv64_without_linker_uses_internal_elf64_linker() {
    let temp = unique_temp_dir("standard-elf64-internal");
    let out_dir = temp.join(".ncc");
    let source = example_path("root_at/root_at_minimal_ok.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc build must use internal ELF64 linker\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
    assert_eq!(json["link"]["route"], "native-elf64-internal", "{json:#?}");
    assert_eq!(json["link"]["internal_os_linker"], true, "{json:#?}");
    assert_eq!(
        json["truth_boundary"]["executable_is_internal_os_loader_direct"], true,
        "{json:#?}"
    );
    assert_eq!(
        json["target"]["triple"], "x86_64-unknown-linux-gnu",
        "{json:#?}"
    );
    assert_eq!(
        json["machine_backend"]["runtime_import_materialization"]["direct_machine_materialized"],
        true,
        "{json:#?}"
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let bytes = std::fs::read(&exe).expect("ELF64 executable bytes");
    assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()));
    let machine_source = std::fs::read_to_string(
        json["paths"]["machine_source"]
            .as_str()
            .expect("machine source path"),
    )
    .expect("machine source must read");
    assert!(
        machine_source.contains("mov rax, 60;") && machine_source.contains("syscall;"),
        "Linux x86_64 entry must terminate through the native exit syscall:\n{machine_source}"
    );
    assert!(
        !machine_source.contains("\n        exit:"),
        "self-contained Linux entry must not retain libc exit:\n{machine_source}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["import_count"], 0,
        "minimal Linux executable must not activate the dynamic loader"
    );
    assert!(
        !bytes
            .windows(b"libc.so.6".len())
            .any(|row| row == b"libc.so.6")
            && !bytes
                .windows(b"ld-linux".len())
                .any(|row| row == b"ld-linux"),
        "minimal Linux executable must be loader-autonomous"
    );
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    assert_eq!(
        Command::new(&exe)
            .status()
            .expect("minimal internal ELF must execute")
            .code(),
        Some(7),
        "raw syscall must preserve the source program's exit status"
    );
    let verify = run_ncc(&[
        "machine".into(),
        "verify-link".into(),
        exe.to_string_lossy().to_string(),
        "--json".into(),
    ]);
    assert!(
        verify.status.success(),
        "verify-link must accept ELF64 executable\nstdout={}\nstderr={}",
        normalize(&verify.stdout),
        normalize(&verify.stderr)
    );
    assert!(
        normalize(&verify.stdout).contains("\"artifact_type\": \"elf64_os_executable\""),
        "stdout={}",
        normalize(&verify.stdout)
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_minimal_linux_entries_use_raw_exit_on_every_supported_isa() {
    let source = example_path("performance/native_fast/minimal_return.n");
    for (target, required) in [
        (
            "x86_64-unknown-linux-gnu",
            &["mov rdi, rax;", "mov rax, 60;", "syscall;"][..],
        ),
        (
            "i386-unknown-linux-gnu",
            &["mov ebx, eax;", "mov eax, 1;", "int 128;"][..],
        ),
        ("aarch64-unknown-linux-gnu", &["mov x8, 93;", "svc 0;"][..]),
        (
            "riscv64-unknown-linux-gnu",
            &["addi a7, zero, 93;", "ecall;"][..],
        ),
    ] {
        let temp = unique_temp_dir(&format!("raw-linux-exit-{}", target.replace('-', "_")));
        let out_dir = temp.join(".ncc");
        let out = run_ncc(&[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--target".into(),
            target.into(),
            "--json".into(),
        ]);
        assert!(
            out.status.success(),
            "minimal build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let json: serde_json::Value =
            serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
        let machine_source = std::fs::read_to_string(
            json["paths"]["machine_source"]
                .as_str()
                .expect("machine source path"),
        )
        .expect("machine source must read");
        for needle in required {
            assert!(
                machine_source.contains(needle),
                "{target} entry must contain {needle:?}:\n{machine_source}"
            );
        }
        assert!(
            !machine_source.contains("\n        exit:")
                && !machine_source.contains("\n        _exit:"),
            "{target} minimal entry must not retain a C exit import:\n{machine_source}"
        );
        assert_eq!(
            json["link"]["phase12_report"]["dynamic_imports"]["import_count"], 0,
            "{target} minimal executable must have no dynamic imports"
        );
        let executable = PathBuf::from(
            json["paths"]["executable"]
                .as_str()
                .expect("executable path"),
        );
        let bytes = std::fs::read(executable).expect("minimal executable bytes");
        assert!(
            !bytes
                .windows(b"libc.so.6".len())
                .any(|row| row == b"libc.so.6"),
            "{target} minimal executable must not depend on libc"
        );
        let _ = std::fs::remove_dir_all(temp);
    }
}

#[test]
fn ncc_build_macho_without_linker_uses_internal_macho_linker() {
    let temp = unique_temp_dir("standard-macho-internal");
    let out_dir = temp.join(".ncc");
    let source = example_path("root_at/root_at_minimal_ok.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "x86_64-apple-darwin".into(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc build must use internal Mach-O linker\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    assert_eq!(json["link"]["route"], "native-macho-internal", "{json:#?}");
    assert_eq!(json["link"]["internal_os_linker"], true, "{json:#?}");
    assert_eq!(json["target"]["object_format"], "macho", "{json:#?}");
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let bytes = std::fs::read(&exe).expect("Mach-O executable bytes");
    assert_eq!(bytes.get(0..4), Some(&[0xcf, 0xfa, 0xed, 0xfe][..]));
    let verify = run_ncc(&[
        "machine".into(),
        "verify-link".into(),
        exe.to_string_lossy().to_string(),
        "--json".into(),
    ]);
    assert!(
        verify.status.success(),
        "verify-link must accept Mach-O executable\nstdout={}\nstderr={}",
        normalize(&verify.stdout),
        normalize(&verify.stderr)
    );
    assert!(
        normalize(&verify.stdout).contains("\"artifact_type\": \"macho64_os_executable\""),
        "stdout={}",
        normalize(&verify.stdout)
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_sysv64_internal_elf64_materializes_output_runtime_import() {
    let temp = unique_temp_dir("standard-elf64-runtime-import");
    let out_dir = temp.join(".ncc");
    let source = example_path("10_mct/print_bytes.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--cc".into(),
        "nC.sysv64".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc build must materialize output through the internal ELF64 linker\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    assert_eq!(json["link"]["route"], "native-elf64-internal", "{json:#?}");
    assert_eq!(
        json["machine_backend"]["runtime_import_materialization"]["status"],
        "closed_by_internal_machine_linker",
        "{json:#?}"
    );
    assert!(
        json["machine_backend"]["runtime_import_materialization"]["runtime_symbols"]
            .as_array()
            .expect("runtime symbols")
            .iter()
            .any(|symbol| matches!(symbol.as_str(), Some("write" | "printf"))),
        "{json:#?}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["status"],
        "closed_dynamic_import_metadata",
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["import_count"]
            .as_u64()
            .is_some_and(|count| count >= 1),
        "{json:#?}"
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    assert_eq!(
        std::fs::read(&exe)
            .expect("ELF64 runtime import executable")
            .get(0..4),
        Some(b"\x7fELF".as_slice())
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_i386_internal_elf32_materializes_runtime_import() {
    let temp = unique_temp_dir("standard-elf32-runtime-import");
    let out_dir = temp.join(".ncc");
    let source = example_path("10_mct/print_bytes.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "i386-unknown-linux-gnu".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc build must materialize i386 runtime import through the internal ELF32 linker\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    assert_eq!(json["link"]["route"], "native-elf32-internal", "{json:#?}");
    assert_eq!(
        json["machine_backend"]["runtime_import_materialization"]["status"],
        "closed_by_internal_machine_linker",
        "{json:#?}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["status"],
        "closed_dynamic_import_metadata",
        "{json:#?}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["format"], "elf32",
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["imports"]
            .as_array()
            .expect("dynamic imports")
            .iter()
            .any(|entry| entry["relocation"] == "R_386_JMP_SLOT"
                && entry["symbol"].as_str().is_some()),
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["sections"]
            .as_array()
            .expect("sections")
            .iter()
            .any(|entry| entry["name"] == ".rel.plt"),
        "{json:#?}"
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let bytes = std::fs::read(&exe).expect("ELF32 runtime import executable");
    assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()));
    assert_eq!(bytes[4], 1, "ELF32 class expected");
    assert_eq!(read_u16_le(&bytes, 18), 3, "i386 e_machine expected");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_riscv64_linux_internal_elf64_materializes_runtime_import() {
    let temp = unique_temp_dir("standard-riscv64-linux-runtime-import");
    let out_dir = temp.join(".ncc");
    let source = example_path("10_mct/print_bytes.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "riscv64-unknown-linux-gnu".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc build must materialize RISC-V64 runtime import through the internal ELF64 linker\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    assert_eq!(json["link"]["route"], "native-elf64-internal", "{json:#?}");
    assert_eq!(
        json["target"]["triple"], "riscv64-unknown-linux-gnu",
        "{json:#?}"
    );
    assert_eq!(
        json["machine_backend"]["runtime_import_materialization"]["status"],
        "closed_by_internal_machine_linker",
        "{json:#?}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["status"],
        "closed_dynamic_import_metadata",
        "{json:#?}"
    );
    assert_eq!(
        json["link"]["phase12_report"]["dynamic_imports"]["format"], "elf64",
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["imports"]
            .as_array()
            .expect("dynamic imports")
            .iter()
            .any(|entry| entry["relocation"] == "R_RISCV_JUMP_SLOT"
                && entry["symbol"].as_str().is_some()),
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["dynamic_imports"]["sections"]
            .as_array()
            .expect("sections")
            .iter()
            .any(|entry| entry["name"] == ".rela.plt"),
        "{json:#?}"
    );
    assert!(
        json["link"]["phase12_report"]["relocations_applied"]
            .as_array()
            .expect("relocations applied")
            .iter()
            .any(|entry| entry["kind"] == "none"
                && entry["status"] == "ignored_relax_or_marker"
                && entry["symbol"] == ""),
        "{json:#?}"
    );
    let exe = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let bytes = std::fs::read(&exe).expect("ELF64 runtime import executable");
    assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()));
    assert_eq!(bytes[4], 2, "RISC-V64 Linux output must be ELF64");
    assert_eq!(read_u16_le(&bytes, 18), 243, "RISC-V e_machine");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_riscv64_linux_internal_elf64_executes_under_qemu_user() {
    let Some(qemu) = qemu_system_binary("qemu-riscv64") else {
        return;
    };
    let temp = unique_temp_dir("standard-riscv64-linux-qemu-loader");
    let out_dir = temp.join(".ncc");
    let source = example_path("performance/native_fast/minimal_return.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        out_dir.to_string_lossy().to_string(),
        "--target".into(),
        "riscv64-unknown-linux-gnu".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "RISC-V64 build must pass before loader execution\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
    let executable = PathBuf::from(
        json["paths"]["executable"]
            .as_str()
            .expect("RISC-V executable path"),
    );
    let machine_source = std::fs::read_to_string(
        json["paths"]["machine_source"]
            .as_str()
            .expect("RISC-V machine bridge path"),
    )
    .expect("RISC-V machine bridge must be retained");
    assert!(
        machine_source.contains("addi a7, zero, 93;") && machine_source.contains("ecall;"),
        "direct Linux entry must terminate through the RISC-V exit syscall:\n{machine_source}"
    );

    let mut command = Command::new(qemu);
    command.arg(&executable);
    let loaded = run_command_with_timeout(command, Duration::from_secs(10))
        .expect("qemu-riscv64 must launch the nC executable");
    assert!(
        loaded.status.success(),
        "QEMU user loader must execute and terminate the RISC-V64 nC executable\nstatus={:?}\nstdout={}\nstderr={}",
        loaded.status.code(),
        normalize(&loaded.stdout),
        normalize(&loaded.stderr)
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_infers_kernel_profile_and_emits_multi_arch_artifacts_from_compact_n() {
    let temp = unique_temp_dir("build-kernel-profile");
    let source = temp.join("kernel_smoke.n");
    std::fs::write(
        &source,
        r#"kernelSmoke {
    boot() -> bool {
        marker at 0x1000;
        *8(marker, 0);
        return true;
    }

    main: boot;
}
"#,
    )
    .expect("kernel source must write");

    for target in [
        "x86_64-unknown-none",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-unknown-none",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
        "i386-unknown-none-elf",
        "i386-unknown-linux-gnu",
        "wasm32-unknown-unknown",
    ] {
        let out_dir = temp.join(format!(".ncc-{}", target.replace('-', "_")));
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "kernel build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let manifest_path = out_dir.join("run").join("kernel_smoke.kernel-build.json");
        let machine_stem = format!(
            "kernel_smoke_{}_kernel",
            target.replace(['/', '\\', ':'], "_")
        );
        let machine_path = out_dir.join("lowering").join(format!("{machine_stem}.nm"));
        let object_path = out_dir.join("obj").join("kernel_smoke.kernel.obj");
        let lowering_path = out_dir
            .join("lowering")
            .join("kernel_smoke.kernel.lowering.json");
        for path in [&manifest_path, &machine_path, &object_path, &lowering_path] {
            let meta = std::fs::metadata(path)
                .unwrap_or_else(|_| panic!("{} must exist for {target}", path.display()));
            assert!(meta.len() > 0, "{} must be non-empty", path.display());
        }
        for dir in [
            "bin",
            "code",
            "incremental",
            "lowering",
            "obj",
            "run",
            "cache",
            "ide",
        ] {
            assert!(
                out_dir.join(dir).is_dir(),
                "{} must exist for {target}",
                out_dir.join(dir).display()
            );
        }
        let manifest_text =
            std::fs::read_to_string(&manifest_path).expect("kernel manifest must read");
        let machine_text =
            std::fs::read_to_string(&machine_path).expect("kernel machine source must read");
        let lowering_text =
            std::fs::read_to_string(&lowering_path).expect("kernel lowering must read");
        let manifest: serde_json::Value =
            serde_json::from_str(&manifest_text).expect("kernel manifest must be json");
        let lowering: serde_json::Value =
            serde_json::from_str(&lowering_text).expect("kernel lowering must be json");
        assert_eq!(manifest["profile"], "kernel", "profile must be retained");
        assert!(
            machine_text.contains("symbol:\n        kmain:")
                && machine_text.contains("entry:\n        kmain;"),
            "retained .nm must materialize the real kernel entry symbol for {target}\n{machine_text}"
        );
        assert_eq!(
            manifest["machine_entry"], "kmain",
            "manifest must retain real machine entry"
        );
        assert_eq!(
            manifest["entry_contract"]["symbol"], "kmain",
            "entry contract must define kmain"
        );
        assert_eq!(
            manifest["loader_contract"]["boot_info"]["layout"], "ncc_kernel_boot_info_v1",
            "loader contract must define the retained boot info layout"
        );
        assert_eq!(
            manifest["loader_contract"]["boot_info"]["minimum_size"], 88,
            "loader contract must retain the concrete boot_info ABI size"
        );
        assert_eq!(
            manifest["loader_contract"]["memory_map"]["entry_size"], 32,
            "loader contract must retain the concrete memory region entry size"
        );
        assert_eq!(
            manifest["loader_contract"]["platform_table"]["own_contract_allowed"], true,
            "loader contract must allow the NCC-owned platform handoff"
        );
        assert_eq!(
            manifest["loader_contract"]["platform_table"]["entry_size"], 32,
            "loader contract must retain the binary platform entry size"
        );
        assert_eq!(
            manifest["loader_contract"]["platform_table"]["kind_catalog"]["4"], "multiboot2_tags",
            "loader contract must retain the Multiboot2 platform adapter kind"
        );
        assert_eq!(
            lowering["lowering"]["kmain_symbol_materialized"], true,
            "lowering manifest must retain kmain materialization"
        );
        if matches!(
            target,
            "x86_64-unknown-none"
                | "x86_64-unknown-linux-gnu"
                | "x86_64-pc-windows-msvc"
                | "x86_64-apple-darwin"
                | "aarch64-unknown-none"
                | "aarch64-unknown-linux-gnu"
                | "riscv64-unknown-elf"
                | "riscv32imac-unknown-elf"
                | "i386-unknown-none-elf"
                | "i386-unknown-linux-gnu"
        ) {
            assert_eq!(
                manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
                true,
                "{target} kernel ref main root body must lower into the linked native body object"
            );
            let body_object = manifest["paths"]["high_level_body_object"]
                .as_str()
                .unwrap_or_else(|| panic!("{target} kernel body object path must be retained"));
            assert!(
                std::path::Path::new(body_object).is_file(),
                "{target} kernel body object must exist at {body_object}"
            );
            assert_eq!(
                manifest["kernel_contract"]["kmain_calls_high_level_main"], true,
                "{target} must record that kmain calls the lowered high-level main"
            );
        }
        assert_eq!(
            lowering["lowering"]["boot_info_layout_materialized"], true,
            "lowering manifest must retain boot info materialization"
        );
        assert_eq!(
            manifest["kernel_contract"]["kernel_attribute_required"], false,
            "normal .n kernel build must not require [@kernel]"
        );
        let expected_boot_arg = match target {
            "x86_64-pc-windows-msvc" => "rcx",
            "x86_64-unknown-none" | "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" => "rdi",
            "aarch64-unknown-none" | "aarch64-unknown-linux-gnu" => "x0",
            "riscv64-unknown-elf" | "riscv32imac-unknown-elf" => "a0",
            "i386-unknown-none-elf" | "i386-unknown-linux-gnu" => "stack[0]",
            "wasm32-unknown-unknown" => "param0",
            other => panic!("missing expected boot argument for {other}"),
        };
        assert_eq!(
            manifest["entry_contract"]["boot_info_argument"], expected_boot_arg,
            "boot info argument must match target ABI"
        );
        assert_eq!(
            manifest["entry_contract"]["stack"]["owner"], "loader",
            "initial stack owner must be the loader"
        );
        assert_eq!(
            manifest["truth_boundary"]["loader_contract_materialized"], true,
            "loader contract must be materialized in retained metadata"
        );
        assert_eq!(
            manifest["target"]["triple"],
            match target {
                "aarch64-unknown-none" => "aarch64-unknown-none",
                other => other,
            },
            "manifest target must match canonical kernel target"
        );
        let image_path = manifest["paths"]["image"]
            .as_str()
            .expect("kernel image path must be present");
        let image_meta = std::fs::metadata(image_path)
            .unwrap_or_else(|_| panic!("kernel image {image_path} must exist for {target}"));
        assert!(image_meta.len() > 0, "kernel image must be non-empty");
        if target == "x86_64-unknown-none" {
            let kernel_elf = std::fs::read(image_path)
                .unwrap_or_else(|_| panic!("kernel ELF {image_path} must read"));
            assert_eq!(
                kernel_elf.get(0..4),
                Some(b"\x7fELF".as_slice()),
                "x86_64 freestanding kernel image must be a real ELF64 executable"
            );
            let boot_image_path = manifest["paths"]["boot_image"]
                .as_str()
                .expect("x86_64 freestanding kernel build must retain a boot smoke image");
            let boot_image = std::fs::read(boot_image_path)
                .unwrap_or_else(|_| panic!("boot image {boot_image_path} must read"));
            assert!(
                boot_image.len() >= 512 + 4096 && boot_image.len() % 512 == 0,
                "boot image must contain stage0, reserved stage1 and kernel ELF payload sectors"
            );
            assert_eq!(
                &boot_image[510..512],
                &[0x55, 0xaa],
                "boot smoke image must carry a BIOS boot signature"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel ELF kmain".len())
                    .any(|window| window == b"nC kernel ELF kmain"),
                "boot image must retain its ELF kmain serial payload"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel E820 ok".len())
                    .any(|window| window == b"nC kernel E820 ok"),
                "boot image must retain the BIOS E820 probe serial payload"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel ACPI RSDP ok".len())
                    .any(|window| window == b"nC kernel ACPI RSDP ok"),
                "boot image must retain the BIOS ACPI RSDP probe serial payload"
            );
            assert!(
                boot_image
                    .windows(b"NCCBOO1".len())
                    .any(|window| window == b"NCCBOO1"),
                "boot image must retain the boot_info magic"
            );
            assert!(
                boot_image
                    .windows(b"NCCSER1".len())
                    .any(|window| window == b"NCCSER1"),
                "boot image must retain a serial descriptor"
            );
            assert!(
                boot_image
                    .windows(b"NCCPLT1".len())
                    .any(|window| window == b"NCCPLT1"),
                "boot image must retain a platform table descriptor"
            );
            assert_eq!(
                manifest["boot_artifact"]["format"], "bios-stage0-stage1-elf64-loader",
                "boot artifact format must be explicit"
            );
            assert_eq!(
                manifest["boot_artifact"]["executes_entry_symbol"], true,
                "boot image must call its retained kmain entry"
            );
            assert_eq!(
                manifest["boot_artifact"]["executes_kernel_elf_entry_symbol"], true,
                "boot image must call the ELF e_entry symbol"
            );
            assert_eq!(
                manifest["boot_artifact"]["stage1_loader"]["kind"], "x86_64-long-mode-elf64-loader",
                "boot artifact must retain the real stage1 loader kind"
            );
            assert_eq!(
                manifest["truth_boundary"]["bootloader_implementation_scope"],
                "x86_64-bios-stage0-stage1-elf64-loader",
                "truth boundary must name the implemented bootloader scope"
            );
            let boot_info = &manifest["boot_artifact"]["boot_info"];
            assert_eq!(
                boot_info["address"], "0x8438",
                "stage1 must retain the actual boot_info pointer passed in rdi"
            );
            assert_eq!(
                boot_info["size"], 88,
                "stage1 boot_info size must match the ABI layout"
            );
            assert_eq!(
                boot_info["flags"]["memory_map_present"], true,
                "boot_info must advertise its memory map"
            );
            assert_eq!(
                boot_info["flags"]["serial_present"], true,
                "boot_info must advertise its serial descriptor"
            );
            assert_eq!(
                boot_info["flags"]["platform_table_present"], true,
                "boot_info must advertise its platform table"
            );
            assert_eq!(
                boot_info["framebuffer"]["present"], false,
                "BIOS stage1 must explicitly report absent framebuffer handoff"
            );
            assert_eq!(
                boot_info["serial"]["base"], "0x3f8",
                "serial descriptor must describe COM1"
            );
            assert_eq!(
                boot_info["serial"]["baud"], 115200,
                "serial descriptor must retain configured baud"
            );
            assert_eq!(
                boot_info["platform_table"]["entries"][0]["kind"], "ncc_boot_v1_own_contract",
                "BIOS stage1 must retain the NCC-owned platform contract"
            );
            let platform_entries = boot_info["platform_table"]["entries"]
                .as_array()
                .expect("platform entries must be an array");
            assert!(
                platform_entries.len() >= 5,
                "platform table must retain own, ACPI, UEFI, Multiboot2 and E820 slots"
            );
            let find_platform = |kind: &str| {
                platform_entries
                    .iter()
                    .find(|entry| entry["kind"] == kind)
                    .unwrap_or_else(|| panic!("missing platform adapter slot {kind}"))
            };
            assert_eq!(
                find_platform("ncc_boot_v1_own_contract")["flags"]["present"],
                true,
                "own boot contract slot must be present"
            );
            assert_eq!(
                find_platform("ncc_boot_v1_own_contract")["ptr"],
                "0x8438",
                "own boot contract slot must point at boot_info"
            );
            for kind in ["uefi_system_table", "multiboot2_tags"] {
                let entry = find_platform(kind);
                assert_eq!(
                    entry["flags"]["external_adapter"], true,
                    "external platform slot {kind} must be marked as adapter-owned"
                );
                assert_eq!(
                    entry["flags"]["present"], false,
                    "external platform slot {kind} must not be faked as present"
                );
                assert_eq!(
                    entry["ptr"], "0x0",
                    "absent external platform slot {kind} must retain a zero pointer"
                );
                assert_eq!(
                    entry["length"], 0,
                    "absent external platform slot {kind} must retain zero length"
                );
            }
            let acpi = find_platform("acpi_rsdp");
            assert_eq!(
                acpi["flags"]["external_adapter"], true,
                "ACPI RSDP slot must be marked as adapter-owned"
            );
            assert_eq!(
                acpi["flags"]["runtime_probe"], true,
                "ACPI RSDP slot must be marked as stage1 runtime-probed"
            );
            assert_eq!(
                acpi["flags"]["dynamic_length"], true,
                "ACPI RSDP slot length must be marked dynamic"
            );
            assert_eq!(
                acpi["flags"]["present"], false,
                "ACPI RSDP slot must not be faked as present before boot"
            );
            assert_eq!(
                acpi["ptr"], "0x0",
                "ACPI RSDP slot must start empty before the runtime probe"
            );
            assert_eq!(
                acpi["length"], 0,
                "ACPI RSDP slot must start with zero length before the runtime probe"
            );
            assert_eq!(
                acpi["status"], "runtime_probe_external",
                "ACPI RSDP slot must state that stage1 fills it from firmware"
            );
            let e820 = find_platform("bios_e820");
            assert_eq!(
                e820["flags"]["external_adapter"], true,
                "BIOS E820 slot must be marked as adapter-owned"
            );
            assert_eq!(
                e820["flags"]["runtime_probe"], true,
                "BIOS E820 slot must be marked as stage1 runtime-probed"
            );
            assert_eq!(
                e820["flags"]["dynamic_length"], true,
                "BIOS E820 slot length must be marked dynamic"
            );
            assert_eq!(
                e820["flags"]["present"], false,
                "BIOS E820 slot must not be faked as present before boot"
            );
            assert_ne!(
                e820["ptr"], "0x0",
                "BIOS E820 slot must retain the real runtime buffer pointer"
            );
            assert_eq!(
                e820["length"], 512,
                "BIOS E820 slot must retain the reserved runtime buffer capacity"
            );
            assert_eq!(
                e820["status"], "runtime_probe_buffer",
                "BIOS E820 slot must state that stage1 fills it at boot"
            );
            assert_eq!(
                boot_info["stack"]["top"], "0x90000",
                "stage1 must retain the real initial stack top"
            );
            assert_eq!(
                boot_info["stack"]["alignment"], 16,
                "stage1 stack must satisfy the x86_64 kernel ABI alignment"
            );
            assert!(
                boot_info["memory_map"]["entries"]
                    .as_array()
                    .is_some_and(|entries| entries.iter().any(|entry| entry["kind"] == "initial_stack")
                        && entries.iter().any(|entry| entry["kind"] == "kernel_loaded_segment")
                        && entries.iter().any(|entry| entry["kind"] == "kernel_elf_file_image")),
                "boot_info memory map must include stack, kernel file image and loaded PT_LOAD segment regions"
            );
            assert_eq!(
                boot_info["memory_map"]["runtime_extensions"]["bios_e820"]["buffer_entry_capacity"],
                16,
                "boot_info must retain the BIOS E820 runtime buffer capacity"
            );
            assert_eq!(
                boot_info["memory_map"]["runtime_extensions"]["bios_e820"]
                    ["updates_memory_map_len"],
                true,
                "stage1 E820 probe must update boot_info.memory_map_len before kmain"
            );
            assert_eq!(
                boot_info["kernel_elf_payload"]["load_address"], "0x9000",
                "boot_info must retain the kernel ELF payload load address"
            );
            let payload_offset = manifest["boot_artifact"]["kernel_elf"]["payload_offset"]
                .as_u64()
                .expect("kernel ELF payload offset must be retained")
                as usize;
            assert_eq!(
                &boot_image[payload_offset..payload_offset + kernel_elf.len()],
                kernel_elf.as_slice(),
                "boot payload must contain exact final kernel ELF bytes"
            );
            assert_eq!(
                manifest["boot_artifact"]["kernel_elf"]["memory_address"], "0x9000",
                "stage1 must retain the kernel ELF memory address"
            );
            assert_eq!(
                manifest["boot_artifact"]["kernel_elf"]["return_contract"], "rax == 0x4f4b",
                "boot image must retain the ELF entry return proof contract"
            );
            assert!(
                manifest["boot_artifact"]["kernel_elf"]["load_segments"]
                    .as_array()
                    .is_some_and(|segments| !segments.is_empty()),
                "boot artifact must retain ELF PT_LOAD segment metadata"
            );
            assert_eq!(
                manifest["truth_boundary"]["kernel_elf64_executable"], true,
                "truth boundary must record the real ELF64 kernel image"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image"], true,
                "x86_64 freestanding must materialize a real boot image"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_long_mode_handoff"], true,
                "boot image must enter long mode before calling kmain"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_executes_kmain"], true,
                "boot image must execute kmain"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_executes_kernel_elf_entry"], true,
                "boot image must execute the final ELF entry"
            );
        } else if target == "i386-unknown-none-elf" {
            let kernel_elf = std::fs::read(image_path)
                .unwrap_or_else(|_| panic!("i386 kernel ELF {image_path} must read"));
            assert_eq!(
                kernel_elf.get(0..4),
                Some(b"\x7fELF".as_slice()),
                "i386 freestanding kernel image must be a real ELF32 executable"
            );
            assert_eq!(
                kernel_elf.get(4).copied(),
                Some(1),
                "i386 freestanding kernel image must use ELFCLASS32"
            );
            let boot_image_path = manifest["paths"]["boot_image"]
                .as_str()
                .expect("i386 freestanding kernel build must retain a boot smoke image");
            let boot_image = std::fs::read(boot_image_path)
                .unwrap_or_else(|_| panic!("i386 boot image {boot_image_path} must read"));
            assert!(
                boot_image.len() >= 512 + 4096 && boot_image.len() % 512 == 0,
                "i386 boot image must contain stage0, reserved stage1 and kernel ELF payload sectors"
            );
            assert_eq!(
                &boot_image[510..512],
                &[0x55, 0xaa],
                "i386 boot image must carry a BIOS boot signature"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel i386 ELF kmain".len())
                    .any(|window| window == b"nC kernel i386 ELF kmain"),
                "i386 boot image must retain its ELF kmain serial payload"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel i386 E820 ok".len())
                    .any(|window| window == b"nC kernel i386 E820 ok"),
                "i386 boot image must retain the BIOS E820 probe serial payload"
            );
            assert!(
                boot_image
                    .windows(b"nC kernel i386 ACPI RSDP ok".len())
                    .any(|window| window == b"nC kernel i386 ACPI RSDP ok"),
                "i386 boot image must retain the BIOS ACPI RSDP probe serial payload"
            );
            assert!(
                boot_image
                    .windows(b"NCCBOO1".len())
                    .any(|window| window == b"NCCBOO1"),
                "i386 boot image must retain the boot_info magic"
            );
            assert!(
                boot_image
                    .windows(b"NCCSER1".len())
                    .any(|window| window == b"NCCSER1"),
                "i386 boot image must retain a serial descriptor"
            );
            assert!(
                boot_image
                    .windows(b"NCCPLT1".len())
                    .any(|window| window == b"NCCPLT1"),
                "i386 boot image must retain a platform table descriptor"
            );
            assert_eq!(
                manifest["boot_artifact"]["format"], "bios-stage0-stage1-elf32-loader",
                "i386 boot artifact format must be explicit"
            );
            assert_eq!(
                manifest["boot_artifact"]["stage1_loader"]["kind"],
                "i386-protected-mode-elf32-loader",
                "i386 boot artifact must retain the real stage1 loader kind"
            );
            assert_eq!(
                manifest["boot_artifact"]["boot_info"]["argument_register"], "stack[0]",
                "i386 boot_info must be passed on the cdecl stack"
            );
            assert_eq!(
                manifest["boot_artifact"]["boot_info"]["stack"]["alignment"], 4,
                "i386 stage1 stack must satisfy the i386 kernel ABI alignment"
            );
            let i386_platform_entries = manifest["boot_artifact"]["boot_info"]["platform_table"]
                ["entries"]
                .as_array()
                .expect("i386 platform entries must be an array");
            let i386_find_platform = |kind: &str| {
                i386_platform_entries
                    .iter()
                    .find(|entry| entry["kind"] == kind)
                    .unwrap_or_else(|| panic!("missing i386 platform adapter slot {kind}"))
            };
            let i386_acpi = i386_find_platform("acpi_rsdp");
            assert_eq!(
                i386_acpi["flags"]["runtime_probe"], true,
                "i386 ACPI RSDP slot must be runtime-probed by stage1"
            );
            assert_eq!(
                i386_acpi["status"], "runtime_probe_external",
                "i386 ACPI RSDP starts empty and is filled from firmware at boot"
            );
            let i386_e820 = i386_find_platform("bios_e820");
            assert_eq!(
                i386_e820["flags"]["runtime_probe"], true,
                "i386 BIOS E820 slot must be runtime-probed by stage1"
            );
            assert_ne!(
                i386_e820["ptr"], "0x0",
                "i386 BIOS E820 slot must retain the real runtime buffer pointer"
            );
            assert_eq!(
                i386_e820["length"], 512,
                "i386 BIOS E820 slot must retain the reserved runtime buffer capacity"
            );
            assert_eq!(
                manifest["boot_artifact"]["boot_info"]["memory_map"]["runtime_extensions"]
                    ["bios_e820"]["updates_memory_map_len"],
                true,
                "i386 E820 probe must update boot_info.memory_map_len before kmain"
            );
            assert_eq!(
                manifest["boot_artifact"]["stage1_loader"]["runtime_probes"][0], "ACPI RSDP search",
                "i386 stage1 must declare its ACPI runtime probe"
            );
            assert_eq!(
                manifest["boot_artifact"]["kernel_elf"]["return_contract"], "eax == 0x4f4b",
                "i386 boot image must retain the ELF entry return proof contract"
            );
            let payload_offset = manifest["boot_artifact"]["kernel_elf"]["payload_offset"]
                .as_u64()
                .expect("i386 kernel ELF payload offset must be retained")
                as usize;
            assert_eq!(
                &boot_image[payload_offset..payload_offset + kernel_elf.len()],
                kernel_elf.as_slice(),
                "i386 boot payload must contain exact final kernel ELF bytes"
            );
            assert_eq!(
                manifest["truth_boundary"]["kernel_elf32_executable"], true,
                "truth boundary must record the real ELF32 kernel image"
            );
            assert_eq!(
                manifest["truth_boundary"]["kernel_link_artifact"], false,
                "i386 freestanding must no longer be an NCCLINK12-only artifact"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image"], true,
                "i386 freestanding must materialize a real boot image"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_protected_mode_handoff"], true,
                "i386 boot image must enter protected mode before calling kmain"
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_long_mode_handoff"], false,
                "i386 boot image must not claim long-mode handoff"
            );
        } else if matches!(
            target,
            "aarch64-unknown-none" | "riscv64-unknown-elf" | "riscv32imac-unknown-elf"
        ) {
            let kernel_elf = std::fs::read(image_path)
                .unwrap_or_else(|_| panic!("QEMU virt kernel ELF {image_path} must read"));
            let expected_class = if target == "riscv32imac-unknown-elf" {
                1
            } else {
                2
            };
            let expected_machine = if target == "aarch64-unknown-none" {
                183_u16
            } else {
                243
            };
            assert_eq!(
                kernel_elf.get(0..4),
                Some(b"\x7fELF".as_slice()),
                "{target}"
            );
            assert_eq!(kernel_elf.get(4).copied(), Some(expected_class), "{target}");
            assert_eq!(
                kernel_elf.get(18..20),
                Some(expected_machine.to_le_bytes().as_slice()),
                "{target} must retain the target ELF machine"
            );
            assert_eq!(
                manifest["paths"]["boot_image"], image_path,
                "QEMU virt must boot the exact final ELF"
            );
            assert_eq!(
                manifest["boot_artifact"]["schema"], "nc.kernel.qemu_virt_elf_boot.v1",
                "{target}"
            );
            assert_eq!(
                manifest["boot_artifact"]["entry_in_executable_segment"],
                true
            );
            assert!(
                manifest["boot_artifact"]["load_segments"]
                    .as_array()
                    .is_some_and(|segments| !segments.is_empty()),
                "{target} must retain PT_LOAD evidence"
            );
            assert_eq!(
                manifest["truth_boundary"]["qemu_virt_elf_loader_adapter"], true,
                "{target}"
            );
            assert_eq!(manifest["truth_boundary"]["kernel_link_artifact"], false);
            assert_eq!(
                manifest["truth_boundary"]["bootloader_implementation"],
                false
            );
            assert_eq!(
                manifest["truth_boundary"]["boot_image_gdt_materialized"], false,
                "non-x86 targets must not claim a GDT"
            );
        } else {
            assert!(
                manifest["paths"]["boot_image"].is_null(),
                "non-bootable kernel target must stay explicit/null for {target}"
            );
        }
    }

    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_arch_primitives_lower_to_each_admitted_architecture() {
    let temp = unique_temp_dir("kernel-arch-primitives");
    let source = example_path("11_machine/kernel_arch_primitives.n");
    for (target, marker, physical) in [
        ("x86_64-unknown-none", "asm sideeffect \"pause\"", true),
        ("x86_64-pc-windows-msvc", "asm sideeffect \"pause\"", true),
        ("x86_64-apple-darwin", "asm sideeffect \"pause\"", true),
        ("aarch64-unknown-none", "asm sideeffect \"yield\"", true),
        (
            "aarch64-unknown-linux-gnu",
            "asm sideeffect \"yield\"",
            true,
        ),
        ("riscv64-unknown-elf", ".word 0x10500073", true),
        ("riscv32imac-unknown-elf", ".word 0x10500073", true),
        ("i386-unknown-none-elf", "asm sideeffect \"pause\"", true),
        (
            "wasm32-unknown-unknown",
            "define i1 @__ncc_kernel_arch_cpu_relax()",
            false,
        ),
    ] {
        let out_dir = temp.join(target.replace('-', "_"));
        let out = run_ncc(&[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--profile".into(),
            "kernel".into(),
            "--target".into(),
            target.into(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--json".into(),
        ]);
        assert!(
            out.status.success(),
            "kernel architecture primitives must build for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let manifest: serde_json::Value =
            serde_json::from_str(&normalize(&out.stdout)).expect("kernel build json");
        let Some(llvm_path) = manifest["paths"]["high_level_body_llvm_ir"].as_str() else {
            assert!(
                !physical,
                "physical target {target} must retain kernel LLVM"
            );
            assert_eq!(
                manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
                false,
                "virtual target {target} must state its non-native body boundary"
            );
            continue;
        };
        let llvm = std::fs::read_to_string(llvm_path).expect("kernel LLVM must be retained");
        assert!(llvm.contains(marker), "missing {marker:?} for {target}");
        assert!(
            llvm.contains("define ptr @memcpy") && llvm.contains("define ptr @memset"),
            "C-compatible memory primitives must be materialized for {target}"
        );
        if physical {
            assert!(
                llvm.contains("@llvm.readcyclecounter"),
                "physical target {target} must lower its cycle counter"
            );
        } else {
            assert!(
                llvm.contains("define i1 @__ncc_kernel_arch_capability(i64 %id)")
                    && llvm.contains("ret i1 false"),
                "virtual target {target} must expose explicit unsupported capabilities"
            );
        }
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_standard_mct_machine_materializes_i386_and_riscv_targets() {
    let temp = unique_temp_dir("standard-mct-machine-targets");
    let source = example_path("root_at/root_at_minimal_ok.n");
    for (target, expected_triple, expected_format, expected_class, expected_machine) in [
        (
            "i386-unknown-linux-gnu",
            "i386-unknown-linux-gnu",
            "elf32",
            1_u8,
            3_u16,
        ),
        (
            "riscv32imac-unknown-elf",
            "riscv32-unknown-elf",
            "elf32",
            1_u8,
            243_u16,
        ),
        (
            "riscv64-unknown-elf",
            "riscv64-unknown-elf",
            "elf64",
            2_u8,
            243_u16,
        ),
    ] {
        let out_dir = temp.join(format!(".ncc-{}", target.replace('-', "_")));
        let out = run_ncc(&[
            "build".into(),
            source.to_string_lossy().to_string(),
            "--out-dir".into(),
            out_dir.to_string_lossy().to_string(),
            "--target".into(),
            target.into(),
            "--json".into(),
        ]);
        assert!(
            out.status.success(),
            "standard MCT-machine build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let json: serde_json::Value =
            serde_json::from_str(&normalize(&out.stdout)).expect("standard build json");
        assert_eq!(json["schema"], "nc.compiler.standard_native_build.v1");
        assert_eq!(json["status"], "native_build_passed", "{json:#?}");
        assert_eq!(
            json["optimization"]["route"], "standard-mct-machine",
            "{json:#?}"
        );
        assert_eq!(
            json["target"]["triple"], expected_triple,
            "target triple must be canonicalized for {target}"
        );
        assert_eq!(
            json["target"]["object_format"], expected_format,
            "{json:#?}"
        );
        assert_eq!(
            json["link"]["route"],
            format!("native-{expected_format}-internal"),
            "{json:#?}"
        );
        assert_eq!(
            json["truth_boundary"]["executable_is_internal_os_loader_direct"], true,
            "{json:#?}"
        );
        assert_eq!(
            json["machine_backend"]["mct_body_object"], true,
            "{json:#?}"
        );
        assert_eq!(
            json["link"]["phase12_report"]["input_objects"][1]["link_unit_origin"],
            "native_object",
            "Phase12 must import the MCT body object as a real third-party/native object for {target}"
        );

        for key in [
            "machine_source",
            "mct_llvm_ir",
            "optimized_ir_report",
            "machine_backend_object_ir",
            "machine_entry_object",
            "object",
            "executable",
        ] {
            let path = PathBuf::from(json["paths"][key].as_str().expect("path string"));
            assert!(
                path.is_file(),
                "{key} artifact must exist for {target}: {}",
                path.display()
            );
            assert!(
                std::fs::metadata(&path).expect("artifact metadata").len() > 0,
                "{key} artifact must be non-empty for {target}: {}",
                path.display()
            );
        }

        let nm = std::fs::read_to_string(
            json["paths"]["machine_source"]
                .as_str()
                .expect("machine source path"),
        )
        .expect("machine source must read");
        assert!(nm.contains("extern:\n        ncc_mct_main:"), "{nm}");
        assert!(nm.contains("entry:\n        main;"), "{nm}");
        assert!(nm.contains("call ncc_mct_main;"), "{nm}");

        let exe = PathBuf::from(
            json["paths"]["executable"]
                .as_str()
                .expect("executable path"),
        );
        assert!(
            exe.extension().is_none(),
            "ELF executable must not be mislabeled as another format: {}",
            exe.display()
        );
        let bytes = std::fs::read(&exe).expect("executable bytes");
        assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()));
        assert_eq!(bytes[4], expected_class, "ELF class mismatch for {target}");
        assert_eq!(
            read_u16_le(&bytes, 18),
            expected_machine,
            "ELF e_machine mismatch for {target}"
        );

        let verify = run_ncc(&[
            "machine".into(),
            "verify-link".into(),
            exe.to_string_lossy().to_string(),
            "--json".into(),
        ]);
        assert!(
            verify.status.success(),
            "verify-link must accept executable for {target}\nstdout={}\nstderr={}",
            normalize(&verify.stdout),
            normalize(&verify.stderr)
        );
        assert!(
            normalize(&verify.stdout).contains(&format!(
                "\"artifact_type\": \"{}_os_executable\"",
                expected_format
            )),
            "stdout={}",
            normalize(&verify.stdout)
        );
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_i386_json_runtime_preserves_target_and_links_real_elf32_object() {
    let temp = unique_temp_dir("standard-mct-json-i386");
    std::fs::create_dir_all(temp.join("lowering")).expect("legacy lowering directory");
    std::fs::write(
        temp.join("lowering/old-main.runtime-json.c"),
        "legacy generated runtime source",
    )
    .expect("legacy JSON runtime source");
    std::fs::write(
        temp.join("lowering/another-unit.runtime-hosted.c"),
        "legacy generated hosted runtime source",
    )
    .expect("legacy hosted runtime source");
    std::fs::create_dir_all(temp.join("lowering/cache/legacy"))
        .expect("nested legacy lowering directory");
    std::fs::write(
        temp.join("lowering/cache/legacy/arbitrary-provider.C"),
        "legacy generated compiler source",
    )
    .expect("nested legacy compiler source");
    let source = workspace_root().join("examples/runtime/json/json_native_pe_status_ok.n");
    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        temp.to_string_lossy().to_string(),
        "--target".into(),
        "i386-unknown-linux-gnu".into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "i386 JSON native build must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("i386 JSON build payload");
    assert_eq!(payload["optimization"]["route"], "standard-mct-machine");
    assert_eq!(payload["target"]["triple"], "i386-unknown-linux-gnu");
    assert_eq!(payload["runtime_json"]["referenced"], true);
    assert_eq!(payload["runtime_json"]["stage"]["status"], "passed");
    assert!(payload["runtime_json"]["source"].is_null());
    assert_eq!(
        payload["runtime_json"]["source_materialization"],
        "compiler-embedded-stdin"
    );
    assert_eq!(
        payload["truth_boundary"]["runtime_json_native_object_is_real_target_object"],
        true
    );
    assert!(payload["link"]["phase12_report"]["unresolved_symbols"]
        .as_array()
        .expect("unresolved symbol list")
        .is_empty());

    for key in ["runtime_json_object", "object", "executable"] {
        let path = PathBuf::from(payload["paths"][key].as_str().expect("artifact path"));
        let bytes = std::fs::read(&path).expect("artifact bytes");
        assert_eq!(bytes.get(0..4), Some(b"\x7fELF".as_slice()), "{key}");
        assert_eq!(bytes[4], 1, "{key} must preserve ELF32 class");
        assert_eq!(read_u16_le(&bytes, 18), 3, "{key} must preserve i386");
    }
    assert_no_materialized_c_sources(&temp);

    #[cfg(target_os = "linux")]
    if std::path::Path::new("/lib/ld-linux.so.2").is_file() {
        let executable = PathBuf::from(
            payload["paths"]["executable"]
                .as_str()
                .expect("executable path"),
        );
        let run = Command::new(executable)
            .current_dir(&temp)
            .output()
            .expect("ELF32 JSON executable must start through the system loader");
        assert!(
            run.status.success(),
            "ELF32 JSON executable failed: stdout={} stderr={}",
            normalize(&run.stdout),
            normalize(&run.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(all(unix, any(target_arch = "x86_64", target_arch = "aarch64")))]
#[test]
fn ncc_build_direct_root_to_main_executes_through_real_system_loader() {
    let temp = unique_temp_dir("standard-mct-direct-root-main");
    let source = temp.join("direct_root_main.n");
    let host_target = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    std::fs::write(
        &source,
        r#"directRootMain[@] {
    stc:
        base >> int = 40;

    to main {
        @answer >> int : @base + 2;
        print("root-mct-ok\n");
    }

    fa:
        return true;
}
"#,
    )
    .expect("direct root to-main source must write");

    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        "--out-dir".into(),
        temp.to_string_lossy().to_string(),
        "--target".into(),
        host_target.into(),
        "--profile".into(),
        "release-fast".into(),
        "--json".into(),
    ]);
    assert!(
        out.status.success(),
        "direct root to-main build must pass\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_str(&normalize(&out.stdout)).expect("direct root build payload");
    assert_eq!(payload["optimization"]["route"], "standard-mct-machine");
    assert_eq!(payload["machine_backend"]["mct_body_object"], true);
    let llvm_path = PathBuf::from(
        payload["paths"]["mct_llvm_ir"]
            .as_str()
            .expect("MCT LLVM path"),
    );
    let llvm = std::fs::read_to_string(llvm_path).expect("MCT LLVM must be retained");
    assert!(llvm.contains("define i32 @ncc_mct_main()"));
    assert!(llvm.contains("!dbg"));

    let executable = PathBuf::from(
        payload["paths"]["executable"]
            .as_str()
            .expect("executable path"),
    );
    let run = Command::new(executable)
        .current_dir(&temp)
        .output()
        .expect("direct root executable must start through the system loader");
    assert!(
        run.status.success(),
        "direct root executable failed: stdout={} stderr={}",
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
    assert_eq!(normalize(&run.stdout), "root-mct-ok\n");
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_profile_lowers_high_level_body_into_linked_kernel_object() {
    let temp = unique_temp_dir("build-kernel-high-level-body");
    let source = temp.join("kernel_body_smoke.n");
    std::fs::write(
        &source,
        r#"kernelBodySmoke[@] {
    type:
        Packet {
            public:
                tag: int;
                score: int;
        }

    stc:
        packet: Packet;
        values: array<int, 4>;
        ok: bool;
        p: ptr<int>;
        label: str;

    local:
        i: int = 0;
        total: int = 0;

    fn:
        bump(x: int) -> int {
            public:
                return x + 1;
        }

        boot() -> bool {
            public:
                @values >> array<int, 4> : [1, 2, 3, 4];
                @packet.tag >> int : bump(@values[1]);
                @packet.score >> int : 0;
                @p >> ptr<int> : addrOf(@values);
                @label >> str : "kernel-body";
                @i >> int : 0;
                @total >> int : 0;
                while (@i < 4) {
                    @total >> int : @total + @values[@i];
                    @i >> int : @i + 1;
                }
                if (@total == 10) {
                    @packet.score >> int : @total + @packet.tag;
                } else {
                    @packet.score >> int : 0;
                }
                @ok >> bool : @packet.score == 13;
                return @ok;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#,
    )
    .expect("kernel body source must write");
    for target in [
        "x86_64-unknown-none",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-unknown-none",
        "aarch64-unknown-linux-gnu",
        "i386-unknown-none-elf",
        "i386-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
    ] {
        let out_dir = temp.join(format!(".ncc-{}", target.replace('-', "_")));
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--profile".into(),
                "kernel".into(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "kernel high-level body build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );

        let manifest_path = out_dir
            .join("run")
            .join("kernel_body_smoke.kernel-build.json");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path).expect("manifest must read"),
        )
        .expect("manifest must be json");
        assert_eq!(
            manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"], true,
            "truth boundary must record real high-level kernel-body lowering for {target}"
        );
        assert_eq!(
            manifest["kernel_contract"]["kmain_calls_high_level_main"], true,
            "kernel contract must record that kmain calls the lowered high-level main for {target}"
        );

        let llvm_path = PathBuf::from(
            manifest["paths"]["high_level_body_llvm_ir"]
                .as_str()
                .expect("body LLVM path"),
        );
        let body_object_path = PathBuf::from(
            manifest["paths"]["high_level_body_object"]
                .as_str()
                .expect("body object path"),
        );
        let machine_source_path = PathBuf::from(
            manifest["paths"]["machine_source"]
                .as_str()
                .expect("machine source path"),
        );
        let lowering_manifest_path = PathBuf::from(
            manifest["paths"]["lowering"]
                .as_str()
                .expect("lowering path"),
        );
        let image_path = PathBuf::from(manifest["paths"]["image"].as_str().expect("image path"));
        for path in [
            &llvm_path,
            &body_object_path,
            &machine_source_path,
            &lowering_manifest_path,
            &image_path,
        ] {
            assert!(
                path.is_file(),
                "expected retained kernel artifact {} for {target}",
                path.display()
            );
            assert!(
                std::fs::metadata(path).expect("artifact metadata").len() > 0,
                "artifact must be non-empty for {target}: {}",
                path.display()
            );
        }

        let llvm = std::fs::read_to_string(&llvm_path).expect("body LLVM must read");
        let lowering_manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&lowering_manifest_path).expect("lowering manifest must read"),
        )
        .expect("lowering manifest must be json");
        let expected_llvm_triple = match target {
            "riscv32imac-unknown-elf" => "riscv32-unknown-elf",
            other => other,
        };
        assert!(
            llvm.contains(&format!("target triple = \"{expected_llvm_triple}\"")),
            "{llvm}"
        );
        assert_eq!(
            lowering_manifest["high_level_body"]["target_triple"], expected_llvm_triple,
            "kernel body lowering must retain the concrete LLVM/backend triple for {target}"
        );
        assert!(llvm.contains("define i1 @kernelBodySmoke_boot()"), "{llvm}");
        assert!(llvm.contains("%struct.kernelBodySmoke_Packet"), "{llvm}");
        assert!(llvm.contains("@__ncc_stc_kernelBodySmoke_values"), "{llvm}");
        assert!(
            llvm.contains("store ptr @__ncc_stc_kernelBodySmoke_values"),
            "{llvm}"
        );
        assert!(llvm.contains("kernel-body\\00"), "{llvm}");
        assert!(llvm.contains("br i1"), "{llvm}");
        assert!(
            llvm.contains("getelementptr inbounds %struct.kernelBodySmoke_Packet"),
            "{llvm}"
        );

        let nm = std::fs::read_to_string(&machine_source_path).expect("machine source must read");
        assert!(nm.contains("extern:"), "{nm}");
        assert!(nm.contains("main:"), "{nm}");
        match target {
            "x86_64-unknown-none" => {
                assert!(nm.contains("call main;"), "{nm}");
                assert!(nm.contains("cmp eax, 0;"), "{nm}");
                assert!(nm.contains("sete al;"), "{nm}");
                assert!(nm.contains("movzx eax, al;"), "{nm}");
                assert!(nm.contains("imul eax, eax, 20299;"), "{nm}");
            }
            "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "x86_64-apple-darwin" => {
                assert!(nm.contains("call main;"), "{nm}");
                assert!(nm.contains("ret;"), "{nm}");
            }
            "aarch64-unknown-none" => {
                assert!(nm.contains("bl main;"), "{nm}");
                assert!(nm.contains("ret;"), "{nm}");
            }
            "aarch64-unknown-linux-gnu" => {
                assert!(nm.contains("bl main;"), "{nm}");
                assert!(nm.contains("ret;"), "{nm}");
            }
            "i386-unknown-none-elf" | "i386-unknown-linux-gnu" => {
                assert!(nm.contains("call main;"), "{nm}");
                assert!(nm.contains("cmp eax, 0;"), "{nm}");
                assert!(nm.contains("sete al;"), "{nm}");
                assert!(nm.contains("movzx eax, al;"), "{nm}");
                assert!(nm.contains("imul eax, eax, 20299;"), "{nm}");
                assert!(nm.contains("type: R_386_PC32;"), "{nm}");
            }
            "riscv64-unknown-elf" | "riscv32imac-unknown-elf" => {
                assert!(nm.contains("call main;"), "{nm}");
                assert!(nm.contains("ret;"), "{nm}");
                assert!(nm.contains("type: R_RISCV_CALL;"), "{nm}");
            }
            other => panic!("missing body wrapper assertions for {other}"),
        }

        let link_report = &manifest["link"]["phase12_report"];
        assert_eq!(
            link_report["input_objects"][1]["link_unit_origin"], "native_object",
            "phase12 must import the high-level body object as a real native object for {target}"
        );
        assert_eq!(
            link_report["input_objects"][1]["path"],
            body_object_path.display().to_string(),
            "phase12 input path must be the retained body object for {target}"
        );
    }

    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_portable_el2_h_iommu_intrinsics_into_real_objects() {
    let temp = unique_temp_dir("build-kernel-portable-hypervisor-iommu");
    let source = example_path("11_machine/kernel_portable_hypervisor_iommu.n");
    for target in [
        "aarch64-unknown-none",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
    ] {
        let out_dir = temp.join(target);
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--profile".into(),
                "kernel".into(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "portable hypervisor kernel build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );

        let manifest_path = out_dir
            .join("run")
            .join("kernel_portable_hypervisor_iommu.kernel-build.json");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path).expect("kernel manifest must read"),
        )
        .expect("kernel manifest must be JSON");
        let llvm_path = PathBuf::from(
            manifest["paths"]["high_level_body_llvm_ir"]
                .as_str()
                .expect("body LLVM path"),
        );
        let object_path = PathBuf::from(
            manifest["paths"]["high_level_body_object"]
                .as_str()
                .expect("body object path"),
        );
        assert!(object_path.is_file(), "{}", object_path.display());
        assert!(
            std::fs::metadata(&object_path).unwrap().len() > 0,
            "{} must contain native object bytes",
            object_path.display()
        );
        let llvm = std::fs::read_to_string(&llvm_path).expect("body LLVM must read");
        assert!(llvm.contains("define i1 @__ncc_kernel_iommu_map_entry"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_descriptor_write"));
        assert!(!llvm.contains("call void @llvm.trap(); unreachable }"));
        assert_eq!(llvm.matches("declare void @llvm.trap()").count(), 1);
        match target {
            "aarch64-unknown-none" => {
                assert!(llvm.contains("mrs $0, hcr_el2"));
                assert!(llvm.contains("tlbi vmalls12e1is"));
                assert!(llvm.contains("hvc #0"));
            }
            "riscv64-unknown-elf" => {
                assert!(llvm.contains(".word 0x62b50073"));
                assert!(llvm.contains(".word 0x6c05c573"));
                assert!(llvm.contains(".word 0x6ea5c073"));
            }
            "riscv32imac-unknown-elf" => {
                assert!(llvm.contains(".word 0x62b50073"));
                assert!(llvm.contains(".word 0x6805c573"));
                assert!(llvm.contains(".word 0x6aa5c073"));
            }
            _ => unreachable!(),
        }
        assert_eq!(
            manifest["link"]["phase12_report"]["unresolved_symbols"]
                .as_array()
                .expect("unresolved symbol list")
                .len(),
            0,
            "{target} must link every portable kernel intrinsic"
        );
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_operational_runtime_across_declared_target_matrix() {
    let temp = unique_temp_dir("build-kernel-operational-matrix");
    let source = example_path("11_machine/kernel_operational_smp.n");
    for target in [
        "x86_64-unknown-none",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-unknown-none",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
        "i386-unknown-none-elf",
        "i386-unknown-linux-gnu",
        "wasm32-unknown-unknown",
    ] {
        let out_dir = temp.join(target);
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--profile".into(),
                "kernel".into(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "operational kernel build must pass for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                out_dir
                    .join("run")
                    .join("kernel_operational_smp.kernel-build.json"),
            )
            .expect("operational kernel manifest must read"),
        )
        .expect("operational kernel manifest must be json");
        assert_eq!(manifest["status"], "kernel_build_passed", "{target}");
        assert_eq!(
            manifest["kernel_contract"]["all_admitted_machine_targets"], true,
            "{target}"
        );
        assert!(
            PathBuf::from(
                manifest["paths"]["image"]
                    .as_str()
                    .expect("kernel image path")
            )
            .is_file(),
            "{target} must retain its final kernel image"
        );
        if target != "wasm32-unknown-unknown" {
            assert_eq!(
                manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
                true,
                "{target} must link the complete operational body"
            );
            assert_eq!(
                manifest["link"]["phase12_report"]["unresolved_symbols"]
                    .as_array()
                    .expect("unresolved symbol list")
                    .len(),
                0,
                "{target} must resolve every kernel runtime symbol"
            );
            assert!(
                manifest["link"]["phase12_report"]["relocations_applied"]
                    .as_array()
                    .expect("applied relocation list")
                    .len()
                    >= 2,
                "{target} must retain relocation evidence for both machine entry and operational body objects"
            );
        }
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_c_style_atomic_mmio_bridge_across_declared_target_matrix() {
    let temp = unique_temp_dir("build-kernel-low-level-c-asm-matrix");
    let source = example_path("11_machine/kernel_low_level_c_asm_bridge.n");
    for target in [
        "x86_64-unknown-none",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-unknown-none",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-elf",
        "riscv32imac-unknown-elf",
        "i386-unknown-none-elf",
        "i386-unknown-linux-gnu",
        "wasm32-unknown-unknown",
    ] {
        let out_dir = temp.join(target);
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--profile".into(),
                "kernel".into(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "low-level kernel bridge must build for {target}\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                out_dir
                    .join("run")
                    .join("kernel_low_level_c_asm_bridge.kernel-build.json"),
            )
            .expect("low-level kernel manifest must read"),
        )
        .expect("low-level kernel manifest must be json");
        assert_eq!(manifest["status"], "kernel_build_passed", "{target}");
        let Some(llvm_path) = manifest["paths"]["high_level_body_llvm_ir"].as_str() else {
            assert_eq!(target, "wasm32-unknown-unknown");
            continue;
        };
        let llvm = std::fs::read_to_string(llvm_path).expect("kernel body LLVM must read");
        for symbol in [
            "@__ncc_kernel_atomic32_load",
            "@__ncc_kernel_atomic32_compare_exchange",
            "@__ncc_kernel_spin_try_lock",
            "@__ncc_kernel_mmio_read64",
            "@__ncc_kernel_mmio_read128",
            "@__ncc_kernel_mmio_read256",
            "@__ncc_kernel_mmio_read512",
            "@__ncc_kernel_mmio_write64",
            "@__ncc_kernel_mmio_write128",
            "@__ncc_kernel_mmio_write256",
            "@__ncc_kernel_mmio_write512",
            "@memmove",
            "@memcmp",
        ] {
            assert!(llvm.contains(symbol), "missing {symbol} for {target}");
        }
        assert!(llvm.contains("atomicrmw xchg"), "{target}");
        assert!(llvm.contains("cmpxchg ptr"), "{target}");
        assert!(llvm.contains("load volatile i64"), "{target}");
        assert!(llvm.contains("store volatile i64"), "{target}");
        assert!(llvm.contains("load volatile i128"), "{target}");
        assert!(llvm.contains("load volatile i256"), "{target}");
        assert!(llvm.contains("load volatile i512"), "{target}");
        assert!(llvm.contains("store volatile i128"), "{target}");
        assert!(llvm.contains("store volatile i256"), "{target}");
        assert!(llvm.contains("store volatile i512"), "{target}");
        assert!(
            manifest["link"]["phase12_report"]["unresolved_symbols"]
                .as_array()
                .expect("unresolved symbol list")
                .is_empty(),
            "{target} must resolve every low-level kernel symbol"
        );
    }
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn ncc_build_kernel_profile_boot_images_execute_machine_object_kmain_in_qemu_when_available() {
    let qemu_x86_64 = qemu_system_binary("qemu-system-x86_64");
    let qemu_i386 =
        qemu_system_binary("qemu-system-i386").or_else(|| qemu_system_binary("qemu-system-x86_64"));
    let qemu_aarch64 = qemu_system_binary("qemu-system-aarch64");
    let qemu_riscv64 = qemu_system_binary("qemu-system-riscv64");
    let qemu_riscv32 = qemu_system_binary("qemu-system-riscv32");
    if qemu_x86_64.is_none()
        && qemu_i386.is_none()
        && qemu_aarch64.is_none()
        && qemu_riscv64.is_none()
        && qemu_riscv32.is_none()
    {
        return;
    }

    for (target, qemu, expected) in [
        (
            "x86_64-unknown-none",
            qemu_x86_64,
            vec![
                "nC kernel ACPI RSDP ok",
                "nC kernel E820 ok",
                "nC kernel ELF kmain",
            ],
        ),
        (
            "i386-unknown-none-elf",
            qemu_i386,
            vec![
                "nC kernel i386 ACPI RSDP ok",
                "nC kernel i386 E820 ok",
                "nC kernel i386 ELF kmain",
            ],
        ),
        (
            "aarch64-unknown-none",
            qemu_aarch64,
            vec!["nC kernel AArch64 kmain"],
        ),
        (
            "riscv64-unknown-elf",
            qemu_riscv64,
            vec!["nC kernel RISC-V64 kmain"],
        ),
        (
            "riscv32imac-unknown-elf",
            qemu_riscv32,
            vec!["nC kernel RISC-V32 kmain"],
        ),
    ] {
        let Some(qemu) = qemu else {
            continue;
        };
        let label = target.replace(['-', '_'], "");
        let temp = unique_temp_dir(&format!("build-kernel-qemu-smoke-{label}"));
        let source = temp.join("kernel_qemu_smoke.n");
        std::fs::write(
            &source,
            r#"kernelQemuSmoke[@] {
    fn:
        boot() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#,
        )
        .expect("kernel source must write");
        let out_dir = temp.join(".ncc");
        let out = run_ncc_in_dir(
            &[
                "build".into(),
                source.to_string_lossy().to_string(),
                "--profile".into(),
                "kernel".into(),
                "--target".into(),
                target.into(),
                "--out-dir".into(),
                out_dir.to_string_lossy().to_string(),
                "--json".into(),
            ],
            &temp,
        );
        assert!(
            out.status.success(),
            "kernel build for {target} must pass before qemu smoke\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );

        let manifest_path = out_dir
            .join("run")
            .join("kernel_qemu_smoke.kernel-build.json");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path).expect("manifest must read"),
        )
        .expect("manifest must be json");
        let boot_image = manifest["paths"]["boot_image"]
            .as_str()
            .expect("boot image path must exist");
        let serial_log = manifest["boot_artifact"]["qemu_smoke"]["serial_log"]
            .as_str()
            .expect("serial log path must exist");

        let mut command = Command::new(qemu);
        match target {
            "x86_64-unknown-none" | "i386-unknown-none-elf" => {
                command
                    .args([
                        "-display",
                        "none",
                        "-machine",
                        "accel=tcg",
                        "-monitor",
                        "none",
                    ])
                    .arg("-serial")
                    .arg(format!("file:{serial_log}"))
                    .arg("-drive")
                    .arg(format!("format=raw,file={boot_image},if=floppy"))
                    .arg("-no-reboot");
            }
            "aarch64-unknown-none" => {
                command
                    .args([
                        "-M",
                        "virt,accel=tcg",
                        "-cpu",
                        "max",
                        "-m",
                        "128M",
                        "-display",
                        "none",
                        "-monitor",
                        "none",
                    ])
                    .arg("-serial")
                    .arg(format!("file:{serial_log}"))
                    .arg("-kernel")
                    .arg(boot_image)
                    .arg("-no-reboot");
            }
            "riscv64-unknown-elf" | "riscv32imac-unknown-elf" => {
                let cpu = if target == "riscv64-unknown-elf" {
                    "rv64"
                } else {
                    "rv32"
                };
                command
                    .args([
                        "-M",
                        "virt,accel=tcg",
                        "-bios",
                        "none",
                        "-cpu",
                        cpu,
                        "-m",
                        "128M",
                        "-display",
                        "none",
                        "-monitor",
                        "none",
                    ])
                    .arg("-serial")
                    .arg(format!("file:{serial_log}"))
                    .arg("-device")
                    .arg(format!("loader,file={boot_image},cpu-num=0"))
                    .arg("-no-reboot");
            }
            _ => unreachable!("kernel QEMU smoke target matrix"),
        }
        let qemu = run_command_with_timeout(command, Duration::from_secs(6))
            .expect("qemu smoke must launch");
        let serial = std::fs::read_to_string(serial_log).unwrap_or_default();
        for marker in expected {
            assert!(
                serial.contains(marker),
                "qemu boot smoke for {target} must emit {marker:?}\nstatus={:?}\nstdout={}\nstderr={}\nserial={}",
                qemu.status.code(),
                normalize(&qemu.stdout),
                normalize(&qemu.stderr),
                serial
            );
        }

        if let Ok(retain_root) = std::env::var("NCC_RETAIN_QEMU_KERNEL_SMOKE") {
            let retain_dir = PathBuf::from(retain_root).join(target);
            std::fs::create_dir_all(&retain_dir).expect("qemu retain dir must create");
            std::fs::copy(&manifest_path, retain_dir.join("kernel-build.json"))
                .expect("qemu manifest evidence must copy");
            std::fs::copy(serial_log, retain_dir.join("serial.log"))
                .expect("qemu serial evidence must copy");
            let retained_boot_name = if target.contains("x86") || target.contains("i386") {
                "boot.img"
            } else {
                "kernel.elf"
            };
            std::fs::copy(boot_image, retain_dir.join(retained_boot_name))
                .expect("qemu boot image evidence must copy");
            std::fs::write(
                retain_dir.join("qemu-runner.txt"),
                format!("target={target}\nstatus={:?}\n", qemu.status.code()),
            )
            .expect("qemu runner evidence must write");
        }

        let _ = std::fs::remove_dir_all(temp);
    }
}

#[cfg(unix)]
fn default_native_fast_host_linker_available() -> bool {
    if std::env::consts::OS != "linux" {
        return false;
    }
    if std::env::consts::ARCH != "x86_64" {
        return false;
    }
    if let Ok(matrix_arch) = std::env::var("NCC_MATRIX_ARCH") {
        if matrix_arch != std::env::consts::ARCH {
            return false;
        }
    }
    Command::new("cc").arg("--version").output().is_ok()
}

#[cfg(unix)]
#[test]
fn ncc_run_release_fast_uses_native_fast_path() {
    let temp = unique_temp_dir("run-release-fast");
    let linker = fake_linker(&temp);
    let source = example_path("main_family_v2_0_0/main_ref_int_ok.n");

    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--profile".into(),
            "release-fast".into(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "ncc run release-fast must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("native-exit=0"), "stdout={stdout}");
    assert!(stdout.contains("route=native-fast-path"), "stdout={stdout}");

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_real_alias_uses_native_fast_path() {
    let temp = unique_temp_dir("run-real-alias");
    let linker = fake_linker(&temp);
    let source = example_path("main_family_v2_0_0/main_ref_int_ok.n");

    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--real".into(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "ncc run --real must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("profile=release-fast"), "stdout={stdout}");
    assert!(stdout.contains("route=native-fast-path"), "stdout={stdout}");

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_real_retains_machine_entry_object_without_c_launcher() {
    let temp = unique_temp_dir("run-real-machine-entry");
    let linker = fake_linker(&temp);
    let source = temp.join("outside_run.n");
    std::fs::write(
        &source,
        r#"outsideRun {
    fn:
        boot() -> int {
            public:
                return 7;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#,
    )
    .expect("source must write");

    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--real".into(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "ncc run --real must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );

    let nm_path = temp.join(".ncc").join("lowering").join("outside_run.nm");
    let nbo_path = temp
        .join(".ncc")
        .join("lowering")
        .join("outside_run.backend-object.nbo");
    let entry_obj_path = temp.join(".ncc").join("obj").join("outside_run.entry.obj");
    let manifest_path = temp
        .join(".ncc")
        .join("run")
        .join("outside_run.native-run.json");
    assert!(nm_path.exists(), "retained machine bridge must exist");
    assert!(nbo_path.exists(), "retained backend object IR must exist");
    assert!(
        entry_obj_path.exists(),
        "retained machine entry object must exist"
    );
    assert!(manifest_path.exists(), "native run manifest must exist");

    let nm = std::fs::read_to_string(&nm_path).expect("retained nm must read");
    assert!(nm.contains("producer: \"ncc hosted .n -> .nm -> .obj -> .exe entry bridge\""));
    assert!(nm.contains("entry:\n        main;"));
    assert!(!nm.contains("#include"));
    assert!(!nm.contains("launcher.c"));

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).expect("manifest must read"))
            .expect("manifest must be json");
    assert_eq!(manifest["truth_boundary"]["c_launcher_generated"], false);
    assert_eq!(
        manifest["truth_boundary"]["host_entry_bridge"],
        "machine_nm_object"
    );
    assert_eq!(
        manifest["paths"]["machine_entry_object"],
        entry_obj_path.to_string_lossy().to_string()
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_real_retains_standard_native_artifacts() {
    let temp = unique_temp_dir("run-real-retained-artifacts");
    let linker = fake_linker(&temp);
    let source = temp.join("runNativeArtifacts.n");
    std::fs::write(
        &source,
        r#"runNativeArtifacts {
    fn:
        boot() -> int {
            public:
                return 7;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#,
    )
    .expect("fixture must write");

    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--real".into(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--cpu".into(),
            "native".into(),
            "--tune-profile".into(),
            "throughput".into(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "ncc run --real must retain artifacts\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let root = temp.join(".ncc");
    let nm = root.join("lowering").join("runNativeArtifacts.nm");
    let nbo = root
        .join("lowering")
        .join("runNativeArtifacts.backend-object.nbo");
    let obj = root.join("obj").join("runNativeArtifacts.obj");
    let entry_obj = root.join("obj").join("runNativeArtifacts.entry.obj");
    let exe = root.join("bin").join("runNativeArtifacts.exe");
    let manifest = root.join("run").join("runNativeArtifacts.native-run.json");
    for path in [&nm, &nbo, &obj, &entry_obj, &exe, &manifest] {
        assert!(
            path.is_file(),
            "missing retained run artifact {}",
            path.display()
        );
        assert!(
            std::fs::metadata(path).expect("metadata").len() > 0,
            "empty retained run artifact {}",
            path.display()
        );
    }
    assert!(std::fs::read_to_string(&nm)
        .expect("machine source")
        .starts_with("runNativeArtifacts :: machine {"));
    let nbo_text = std::fs::read_to_string(&nbo).expect("backend object text");
    assert!(nbo_text.contains("; nc.lowering.root-machine-nbo.v1"));
    assert!(nbo_text.contains("; machine-source=runNativeArtifacts.nm"));
    assert!(!root.join("code").join("runNativeArtifacts.nm").exists());
    let manifest_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).expect("native run manifest"))
            .expect("native run manifest json");
    assert_eq!(
        manifest_json["schema"],
        "nc.compiler.standard_native_run.v1"
    );
    assert_eq!(manifest_json["status"], "native_run_passed");
    assert_eq!(
        manifest_json["speed_contract"]["c_like_artifact_set"],
        serde_json::json!([".nm", ".backend-object.nbo", ".obj", ".entry.obj", ".exe"])
    );
    assert_eq!(
        manifest_json["paths"]["machine_backend_object_ir"],
        nbo.display().to_string()
    );
    assert_eq!(manifest_json["cache"]["status"], "stored");
    assert_eq!(manifest_json["cache"]["hit"], false);
    assert_eq!(
        manifest_json["lowering_lineage"]["order"],
        serde_json::json!([
            "root",
            "machine.nm",
            "backend-object.nbo",
            "native-object",
            "executable"
        ])
    );
    assert_eq!(
        manifest_json["lowering_lineage"]["backend_object"]["source_machine_path_embedded"],
        true
    );
    assert_eq!(
        manifest_json["lowering_lineage"]["backend_object"]["source_machine_sha256_embedded"],
        false
    );
    assert_eq!(
        manifest_json["truth_boundary"]["object_is_real_backend_output"],
        true
    );

    let executable_bytes = std::fs::read(&exe).expect("retained executable bytes");
    let cached = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--real".into(),
            "--linker".into(),
            linker.to_string_lossy().to_string(),
            "--cpu".into(),
            "native".into(),
            "--tune-profile".into(),
            "throughput".into(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        cached.status.success(),
        "second native run must reuse the retained executable\nstdout={}\nstderr={}",
        normalize(&cached.stdout),
        normalize(&cached.stderr)
    );
    let cached_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).expect("cached manifest"))
            .expect("cached manifest json");
    assert_eq!(cached_manifest["route"], "native-run-content-cache");
    assert_eq!(cached_manifest["cache"]["status"], "hit");
    assert_eq!(cached_manifest["cache"]["hit"], true);
    assert_eq!(cached_manifest["cache"]["hit_count"], 1);
    assert_eq!(
        std::fs::read(&exe).expect("cached executable bytes"),
        executable_bytes
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_interpreted_rejects_real_backend_mix() {
    let source = example_path("main_family_v2_0_0/main_ref_int_ok.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--interpreted".into(),
        "--real".into(),
    ]);
    assert!(
        !out.status.success(),
        "ncc run --interpreted --real must fail\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stderr = normalize(&out.stderr);
    assert!(
        stderr.contains("--interpreted") && stderr.contains("--real"),
        "stderr={stderr}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_release_fast_with_real_host_linker_ok() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("main_family_v2_0_0/main_ref_int_ok.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--profile".into(),
        "release-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run release-fast with real host linker must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 7 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("route=native-fast-path"), "stdout={stdout}");
}

#[cfg(unix)]
#[test]
fn ncc_run_plain_auto_selects_native_fast_for_lowerable_single_file() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let temp = unique_temp_dir("run-auto-native-single-file");
    let source = temp.join("plainAutoNative.n");
    std::fs::write(
        &source,
        r#"plainAutoNative[@] {
    fn:
        boot() -> int {
            public:
                return 7;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#,
    )
    .expect("fixture must write");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "plain ncc run must auto-select native-fast for a lowerable single file\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 7 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_destructive_executes_mct_llvm_phase12_binary() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let temp = unique_temp_dir("run-destructive-mct-phase12");
    let fixture = example_path("10_mct/destructive_execute_console.n");
    let source = temp.join("destructive_execute_console.n");
    std::fs::copy(&fixture, &source).expect("destructive run fixture must copy");
    let out = run_ncc_in_dir(
        &[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--native-fast".into(),
            "--profile".into(),
            "release-fast".into(),
            "--cc".into(),
            "nC.destructive".into(),
            "--target".into(),
            "x86_64-unknown-linux-gnu".into(),
            "--show-return".into(),
        ],
        &temp,
    );
    assert!(
        out.status.success(),
        "destructive ncc run must execute its native binary\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(!stdout.contains("omit: this line must disappear"));
    assert_eq!(
        stdout.matches("repeat: this line is emitted twice").count(),
        2,
        "stdout={stdout}"
    );
    assert_eq!(
        stdout
            .matches("split: this line exists here and at the tail")
            .count(),
        2,
        "stdout={stdout}"
    );
    assert_eq!(
        stdout.matches("once: this line is emitted once").count(),
        1,
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("[return: 0 profile=release-fast route=mct-llvm-build-native-fallback]"),
        "stdout={stdout}"
    );

    let manifest = temp.join(".ncc/run/destructive_execute_console.native-run.json");
    let payload: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&manifest).expect("destructive native run manifest must exist"),
    )
    .expect("destructive native run manifest must be valid JSON");
    let link = &payload["build_manifest"]["link"];
    assert_eq!(link["route"], "mct-llvm-phase12-os");
    assert_eq!(link["internal_os_linker"], true);
    assert_eq!(link["internal_os_link_format"], "elf");
    assert_eq!(link["destructive_internal_link"], true);
    assert_eq!(link["missing_symbol_policy"], "materialize_null_address");
    assert_eq!(
        link["phase12_report"]["missing_symbol_policy"],
        "materialize_null_address"
    );
    assert_eq!(payload["speed_contract"]["interpreter_fallback"], false);

    let llvm_path = payload["paths"]["llvm_ir"]
        .as_str()
        .expect("destructive run must retain LLVM IR");
    let llvm = std::fs::read_to_string(llvm_path).expect("retained LLVM IR must read");
    assert!(llvm.contains("ncc-destructive-policy schema=nc.backend.destructive_policy.v2"));
    assert!(llvm.contains("target triple = \"x86_64-unknown-linux-gnu\""));

    let executable_path = payload["paths"]["executable"]
        .as_str()
        .expect("destructive run must retain an executable");
    let executable = std::fs::read(executable_path).expect("retained executable must read");
    assert!(executable.starts_with(b"\x7fELF"));

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_plain_uses_native_fast_for_input_runtime_surface() {
    let temp = unique_temp_dir("run-auto-input-interpreter");
    let source = temp.join("autoInputRuntime.n");
    std::fs::write(
        &source,
        r#"use::stoud::io::<input, output>;

autoInputRuntime {
    to main {
        println("stdin surface stays hosted");
    }

    fa:
        return true;
}
"#,
    )
    .expect("fixture must write");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "plain ncc run with input import must use the native backend\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("stdin surface stays hosted"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("profile=release-fast route=native-fast-path"),
        "stdout={stdout}"
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_plain_materializes_read_int_and_inherits_stdin() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let fixture = example_path("07_runtime/prueba_plugin.n");
    let temp = unique_temp_dir("run-native-read-int");
    let source = temp.join("prueba_plugin.n");
    std::fs::copy(&fixture, &source).expect("readInt fixture must copy");
    let out = run_ncc_with_stdin_in_dir(
        &["run".into(), source.to_string_lossy().to_string()],
        "7\n",
        &temp,
    );
    assert!(
        out.status.success(),
        "plain ncc run with readInt must execute through a native backend\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("Double: 14\n"), "stdout={stdout}");
    let manifest = temp.join(".ncc/run/prueba_plugin.native-run.json");
    assert!(manifest.is_file(), "native run manifest must be retained");
    let payload: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).expect("native run manifest must read"))
            .expect("native run manifest must be json");
    assert_eq!(
        payload.get("route").and_then(|value| value.as_str()),
        Some("mct-llvm-build-native-fallback")
    );
    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_run_real_executes_runtime_list_pack_on_native_fast() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("stress/stress_runtime_list_pack.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--real".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run --real runtime list pack must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("stress_runtime_list_pack"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("sum =130 len = 4 cap = 4"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("checksum = 429"), "stdout={stdout}");
    assert!(stdout.contains("ok = true"), "stdout={stdout}");
    assert!(stdout.contains("route=native-fast-path"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
    let stderr = normalize(&out.stderr);
    assert!(
        !stderr.contains("undefined reference"),
        "native-fast linker stderr must not leak unresolved symbols: {stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "ncc run --real native-fast path should not leak stderr: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_returns_local_scalar_expression_value() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/return_local_expr.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast local expression must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_post_increment_value() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/post_increment.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast post increment must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 2 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_pointer_post_increment_read() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/post_increment_pointer.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast pointer post increment must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 140 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_compact_pointer_c_forms() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    for (fixture, expected) in [
        ("pointer_address_of_amp.n", 33),
        ("pointer_post_increment_step.n", 99),
    ] {
        let source = example_path(&format!("performance/native_fast/{fixture}"));

        let out = run_ncc(&[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--native-fast".into(),
            "--show-return".into(),
        ]);
        assert!(
            out.status.success(),
            "ncc run native-fast pointer fixture '{fixture}' must succeed\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let stdout = normalize(&out.stdout);
        assert!(
            stdout.contains(&format!(
                "[return: {expected} profile=release-fast route=native-fast-path]"
            )),
            "stdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_run_plain_uses_native_fast_for_heap_pointer_compact_program() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("punterosBasicoNormal.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "plain ncc run must use native-fast for output/pointer compact program\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("Distancia pB - pA: 1\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_struct_field_read_write() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/struct_field_read_write.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast struct field read/write must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_struct_field_post_increment() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/struct_field_post_increment.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast struct field post-increment must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 39 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_nested_struct_field_read_write() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/struct_nested_field_read_write.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast nested struct field fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 80 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_struct_array_field_read_write() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/struct_array_field_read_write.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast struct array field fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 22 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_vector_struct_push_and_field_access() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("backend/mct_llvm_vector_struct_access.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast vector<Struct> fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_real_array_storage_fixtures() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    for (fixture, expected) in [
        ("array_zero_fill_100.n", "7"),
        ("array_fill_nonzero.n", "5"),
        ("array_char_fill.n", "true"),
        ("array_multidim_flat.n", "3"),
        ("array_multidim_flat_literal.n", "3"),
        ("array_multidim_write.n", "13"),
    ] {
        let source = example_path(&format!("performance/native_fast/{fixture}"));

        let out = run_ncc(&[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--native-fast".into(),
            "--show-return".into(),
        ]);
        assert!(
            out.status.success(),
            "ncc run native-fast array fixture '{fixture}' must succeed\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let stdout = normalize(&out.stdout);
        assert!(
            stdout.contains(&format!(
                "[return: {expected} profile=release-fast route=native-fast-path]"
            )),
            "stdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_returns_register_param_call_value() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/register_stack_sum.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast register param call must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_returns_stack_local_value() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/register_stack_locals.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast stack locals must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_call_abi_fixtures() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    for fixture in [
        "call_branch_return.n",
        "call_nested_args.n",
        "call_stack_args.n",
        "call_loop_accumulate.n",
        "call_statement_before_return.n",
    ] {
        let source = example_path(&format!("performance/native_fast/{fixture}"));

        let out = run_ncc(&[
            "run".into(),
            source.to_string_lossy().to_string(),
            "--native-fast".into(),
            "--show-return".into(),
        ]);
        assert!(
            out.status.success(),
            "ncc run native-fast call ABI fixture {fixture} must succeed\nstdout={}\nstderr={}",
            normalize(&out.stdout),
            normalize(&out.stderr)
        );
        let stdout = normalize(&out.stdout);
        assert!(
            stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
            "fixture={fixture} stdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_materializes_pointer_byte_qword_backend_ops() {
    let source = example_path("performance/native_fast/memory_ptr_byte_qword_real_bytes.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object pointer byte/qword fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    for needle in [
        "storebyte.",
        "loadbyte.zero",
        "storeqword.",
        "loadqword.zero",
    ] {
        assert!(
            stdout.contains(needle),
            "expected backend op '{needle}' in emit-object output\nstdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_materializes_stc_pointer_globals_as_qword() {
    let source = example_path("performance/native_fast/post_increment_pointer.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object pointer global fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("ty=ptr<int> init=zero"), "stdout={stdout}");
    assert!(
        stdout.contains("storeglobalqword.zero [__ncc_stc_postIncrementPointerNative_p]"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("loadglobalqword.zero r8, [__ncc_stc_postIncrementPointerNative_p]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_materializes_struct_field_backend_ops() {
    let source = example_path("performance/native_fast/struct_field_read_write.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object struct field fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    for needle in ["storedword.", "loaddword."] {
        assert!(
            stdout.contains(needle),
            "expected backend op '{needle}' in emit-object output\nstdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_materializes_nested_struct_field_backend_ops() {
    let source = example_path("performance/native_fast/struct_nested_field_read_write.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object nested struct field fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    for needle in ["storedword.", "loaddword."] {
        assert!(
            stdout.contains(needle),
            "expected backend op '{needle}' in emit-object output\nstdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_materializes_struct_array_field_backend_ops() {
    let source = example_path("performance/native_fast/struct_array_field_read_write.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object struct array field fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    for needle in ["storedword.", "loaddword."] {
        assert!(
            stdout.contains(needle),
            "expected backend op '{needle}' in emit-object output\nstdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_register_allocation_v2_fixture() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let source = example_path("performance/native_fast/register_alloc_v2_liveness.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast register allocation V2 fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("[return: 42 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_runtime_list_print_helpers() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("performance/native_fast/runtime_helpers_list_print.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast runtime helper fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("native-fast-runtime\n"), "stdout={stdout}");
    assert!(stdout.contains("idx=2 cap=4 ok=true\n"), "stdout={stdout}");
    assert!(stdout.contains("tail=115\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_lowers_runtime_list_helpers_without_external_helper_calls() {
    let source = example_path("performance/native_fast/runtime_helpers_list_print.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object native-fast list helper fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("list_clone_copy"), "stdout={stdout}");
    assert!(stdout.contains("list_search"), "stdout={stdout}");
    for unexpected in [
        "call listClone",
        "call listIndexOf",
        "call listContains",
        "call listCapacity",
        "call colorBold",
    ] {
        assert!(
            !stdout.contains(unexpected),
            "runtime helper '{unexpected}' must be lowered inline\nstdout={stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn ncc_run_plain_uses_native_fast_for_list_region_heap_surface() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("stress/stress_runtime_list_region_industrial_pack.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "plain ncc run must use native-fast for list/memRegion/array surface\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("region checksum: 4486\n"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_lays_out_global_dynamic_descriptors_without_overlap() {
    let source = example_path("stress/stress_runtime_list_region_industrial_pack.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object must expose non-overlapping dynamic stc symbols\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    let espejo = native_fast_symbol_offset(
        &stdout,
        "__ncc_stc_stress_runtime_list_region_industrial_pack_espejo",
    );
    let eventos = native_fast_symbol_offset(
        &stdout,
        "__ncc_stc_stress_runtime_list_region_industrial_pack_eventos",
    );
    assert_eq!(
        eventos.saturating_sub(espejo),
        24,
        "list<T>/memRegion<T> stc descriptors must reserve data,len,capacity words\nstdout={stdout}"
    );
}

fn native_fast_symbol_offset(stdout: &str, symbol: &str) -> usize {
    stdout
        .lines()
        .find(|line| line.contains(&format!("{symbol} section=")))
        .and_then(|line| line.split(" offset=").nth(1))
        .and_then(|tail| tail.split_whitespace().next())
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or_else(|| panic!("symbol offset for {symbol} must exist\nstdout={stdout}"))
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_dynamic_heap_dinarray_growth() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("performance/native_fast/dynamic_heap_dinarray_growth.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast dinarray heap fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("dinarray.heap ok=true\n"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_dynamic_heap_vector_struct_growth() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("performance/native_fast/dynamic_heap_vector_struct_growth.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast vector<struct> heap fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("vector.struct.heap tag=100 ok=true\n"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_to_main_high_level_surface() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("calculadoraFacil.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast to-main fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("15\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: 1 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_string_concat_and_state_init_surface() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("helloWorld.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast string/state fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("Hello, world nC!\n"), "stdout={stdout}");
    assert!(stdout.contains("runtime listo\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: 1 profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_lowers_string_literals_to_rodata_without_concat_helper() {
    let source = example_path("helloWorld.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object native-fast string fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains(".rodata.helloWorld.stc"), "stdout={stdout}");
    assert!(
        stdout.contains("__ncc_stc_helloWorld_status.__bytes"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("cstr(\"Hello, world nC!\")"),
        "stdout={stdout}"
    );
    assert!(
        !stdout.contains("__ncc_native_concat2"),
        "string concat must not depend on hosted C concat helper\nstdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_render_assignment_to_string() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("runtime/render/render_assignment_to_str_ok.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast render assignment fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("Checksum: 9038\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_object_lowers_render_assignment_without_render_symbol_call() {
    let source = example_path("runtime/render/render_assignment_to_str_ok.n");

    let out = run_ncc(&[
        "emit-object".into(),
        source.to_string_lossy().to_string(),
        "--cc".into(),
        "nC.sysv64".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc emit-object native-fast render assignment fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("call snprintf"), "stdout={stdout}");
    assert!(stdout.contains("str_len_loop"), "stdout={stdout}");
    assert!(stdout.contains("str_copy_loop"), "stdout={stdout}");
    assert!(
        stdout.contains("storeglobalqword.zero [__ncc_stc_render_assignment_to_str_ok_line]"),
        "stdout={stdout}"
    );
    assert!(
        !stdout.contains("call renderInt"),
        "renderInt must be lowered to native render bytes instead of an external symbol\nstdout={stdout}"
    );
    assert!(
        !stdout.contains("__ncc_native_concat2"),
        "render concat must not depend on hosted C concat helper\nstdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_run_native_fast_executes_global_array_write_with_call_surface() {
    if !default_native_fast_host_linker_available() {
        return;
    }

    let source = example_path("randomArraySearchHidden.n");

    let out = run_ncc(&[
        "run".into(),
        source.to_string_lossy().to_string(),
        "--native-fast".into(),
        "--show-return".into(),
    ]);
    assert!(
        out.status.success(),
        "ncc run native-fast global array/call fixture must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("checksum=4950\n"), "stdout={stdout}");
    assert!(stdout.contains("firstIndex=63\n"), "stdout={stdout}");
    assert!(
        stdout.contains("[return: true profile=release-fast route=native-fast-path]"),
        "stdout={stdout}"
    );
}

#[cfg(unix)]
#[test]
fn ncc_emit_elf_link_and_execute_real_binary_smoke() {
    if !default_native_fast_host_linker_available() {
        return;
    }
    let temp = unique_temp_dir("native-object-executable-smoke");
    let source = example_path("performance/native_fast/register_alloc_v2_liveness.n");
    let object_path = temp.join("register_alloc_v2.o");
    let launcher_path = temp.join("launcher.c");
    let exe_path = temp.join("register_alloc_v2_hosted.bin");

    let emit = run_ncc(&[
        "emit-elf".into(),
        source.to_string_lossy().to_string(),
        object_path.to_string_lossy().to_string(),
    ]);
    assert!(
        emit.status.success(),
        "ncc emit-elf must write a native object\nstdout={}\nstderr={}",
        normalize(&emit.stdout),
        normalize(&emit.stderr)
    );
    let object_bytes = std::fs::read(&object_path).expect("ELF object must be readable");
    assert!(
        object_bytes.starts_with(b"\x7FELF"),
        "emit-elf output must be a real ELF object"
    );

    std::fs::write(
        &launcher_path,
        r#"#include <stdint.h>
#include <stdio.h>
extern int64_t ncc_entry(void) __asm__("nativeRegisterAllocationV2Perf.boot");
int main(void) {
    int64_t value = ncc_entry();
    printf("return=%lld\n", (long long)value);
    return value == 42 ? 0 : 42;
}
"#,
    )
    .expect("launcher source must write");

    let link = Command::new("cc")
        .arg(&launcher_path)
        .arg(&object_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("host linker must execute");
    assert!(
        link.status.success(),
        "host cc must link emitted ELF object\nstdout={}\nstderr={}",
        normalize(&link.stdout),
        normalize(&link.stderr)
    );
    let exe_bytes = std::fs::read(&exe_path).expect("linked executable must be readable");
    assert!(
        exe_bytes.starts_with(b"\x7FELF"),
        "linked output must be a real ELF executable"
    );

    let run = Command::new(&exe_path)
        .output()
        .expect("linked native executable must run");
    assert_eq!(
        run.status.code(),
        Some(0),
        "native executable exit code must be 0\nstdout={}\nstderr={}",
        normalize(&run.stdout),
        normalize(&run.stderr)
    );
    assert_eq!(normalize(&run.stdout), "return=42\n");
    assert_eq!(normalize(&run.stderr), "");

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_build_release_fast_writes_native_artifact() {
    let temp = unique_temp_dir("build-release-fast");
    let linker = fake_linker(&temp);
    let source = example_path("root_at/root_at_minimal_ok.n");
    let out_path = temp.join("root-fast.bin");

    let out = run_ncc(&[
        "build".into(),
        source.to_string_lossy().to_string(),
        out_path.to_string_lossy().to_string(),
        "--profile".into(),
        "release-fast".into(),
        "--linker".into(),
        linker.to_string_lossy().to_string(),
    ]);
    assert!(
        out.status.success(),
        "ncc build release-fast must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    assert!(out_path.exists(), "native build output must exist");
    let metadata = std::fs::metadata(&out_path).expect("native build output metadata");
    assert!(metadata.len() > 0, "native build output must be non-empty");
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains("route=standard-mct-machine"),
        "stdout={stdout}"
    );

    let _ = std::fs::remove_dir_all(temp);
}

#[cfg(unix)]
#[test]
fn ncc_build_real_infers_default_native_artifact_path() {
    let temp = unique_temp_dir("build-real-default");
    let linker = fake_linker(&temp);
    let source = example_path("root_at/root_at_minimal_ok.n");

    let out = Command::new(ncc_bin())
        .arg("build")
        .arg(&source)
        .arg("--real")
        .arg("--linker")
        .arg(&linker)
        .env("NC_NO_COLOR", "1")
        .current_dir(&temp)
        .output()
        .expect("ncc build --real command must execute");

    assert!(
        out.status.success(),
        "ncc build --real must succeed\nstdout={}\nstderr={}",
        normalize(&out.stdout),
        normalize(&out.stderr)
    );

    let out_path = temp
        .join("target")
        .join("nc")
        .join("bin")
        .join("root_at_minimal_ok");
    assert!(out_path.exists(), "default --real output must exist");
    let metadata = std::fs::metadata(&out_path).expect("default build output metadata");
    assert!(metadata.len() > 0, "default build output must be non-empty");
    let stdout = normalize(&out.stdout);
    assert!(stdout.contains("target/nc/bin"), "stdout={stdout}");
    assert!(
        stdout.contains("route=standard-mct-machine"),
        "stdout={stdout}"
    );

    let _ = std::fs::remove_dir_all(temp);
}
