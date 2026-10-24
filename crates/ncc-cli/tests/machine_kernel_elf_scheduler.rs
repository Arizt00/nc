use ncc_machine::{build_test_elf32_dynamic_tls, build_test_elf64_dynamic_tls};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_bundle(target: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-kernel-user-runtime-{}-{nanos}-{}",
        std::process::id(),
        target.replace('-', "_")
    ))
}

fn output_text(output: &Output) -> String {
    format!(
        "status={}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn load_from_stdin(target: &str, bundle: &Path, image: &[u8]) -> Output {
    let mut child = Command::new(ncc_bin())
        .args([
            "machine",
            "kernel-system",
            "load-elf",
            "--image",
            "-",
            "--target",
            target,
            "--bundle",
            &bundle.to_string_lossy(),
            "--arg",
            "demo",
            "--arg",
            "--raw",
            "--env",
            "MODE=test",
            "--symbol",
            "host_write=0x60000000",
            "--cpus",
            "4",
            "--frames",
            "512",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ncc starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(image)
        .expect("ELF stdin");
    child.wait_with_output().expect("ncc completes")
}

fn verify(bundle: &Path) -> Output {
    Command::new(ncc_bin())
        .args([
            "machine",
            "kernel-system",
            "verify-user-runtime",
            &bundle.to_string_lossy(),
        ])
        .output()
        .expect("verification executes")
}

#[test]
fn elf_loader_and_multicore_scheduler_are_material_for_every_user_elf_abi() {
    for (target, image) in [
        ("x86_64-unknown-none", build_test_elf64_dynamic_tls(62)),
        ("i386-unknown-none-elf", build_test_elf32_dynamic_tls()),
        ("aarch64-unknown-none", build_test_elf64_dynamic_tls(183)),
        (
            "riscv64gc-unknown-none-elf",
            build_test_elf64_dynamic_tls(243),
        ),
    ] {
        let bundle = unique_bundle(target);
        let output = load_from_stdin(target, &bundle, &image);
        assert!(output.status.success(), "{}", output_text(&output));
        let certificate: Value = serde_json::from_slice(&output.stdout).expect("certificate JSON");
        assert_eq!(
            certificate["payload"]["status"],
            "verified_material_execution"
        );
        let report: Value = serde_json::from_slice(
            &fs::read(bundle.join("kernel-user-runtime-report.json")).expect("report"),
        )
        .expect("report JSON");
        assert_eq!(report["image"]["relocations_applied"], 2);
        assert_eq!(report["image"]["needed_libraries"][0], "libdemo.so");
        assert_eq!(report["initial_stack"]["alignment"], 0);
        assert_eq!(report["initial_stack"]["argv"].as_array().unwrap().len(), 2);
        assert_eq!(report["initial_stack"]["envp"].as_array().unwrap().len(), 1);
        assert_eq!(report["tls"]["bytes"], 16);
        assert_eq!(
            report["scheduler"]["first"],
            report["scheduler"]["pids"]["high"]
        );
        assert_eq!(report["scheduler"]["inherited_priority"], 2);
        assert_eq!(report["scheduler"]["high_blocked_on_lock"], true);
        assert_eq!(report["scheduler"]["high_state"], "Running");
        assert_eq!(report["state_checks"]["high_running"], true);
        assert_eq!(report["state_checks"]["main_priority_restored"], true);
        assert_eq!(report["contracts"]["stdin_binary_safe"], true);
        let verified = verify(&bundle);
        assert!(verified.status.success(), "{}", output_text(&verified));
        let verification: Value =
            serde_json::from_slice(&verified.stdout).expect("verification JSON");
        assert_eq!(verification["status"], "verified");
        fs::remove_dir_all(bundle).expect("cleanup bundle");
    }
}

#[test]
fn user_runtime_certificate_rejects_a_tampered_retained_elf() {
    let target = "x86_64-unknown-none";
    let bundle = unique_bundle(target);
    let image = build_test_elf64_dynamic_tls(62);
    let output = load_from_stdin(target, &bundle, &image);
    assert!(output.status.success(), "{}", output_text(&output));
    let path = bundle.join("user-image.elf");
    let mut retained = fs::read(&path).expect("retained ELF");
    retained[0x180] ^= 0xff;
    fs::write(path, retained).expect("tamper retained ELF");
    let verification = verify(&bundle);
    assert!(!verification.status.success(), "tamper must be rejected");
    assert!(
        String::from_utf8_lossy(&verification.stderr).contains("artifact hash mismatch"),
        "{}",
        output_text(&verification)
    );
    fs::remove_dir_all(bundle).expect("cleanup bundle");
}
