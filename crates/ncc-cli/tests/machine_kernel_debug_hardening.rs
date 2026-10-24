use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_bundle(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-kernel-debug-hardening-{}-{nonce}-{name}",
        std::process::id()
    ))
}

fn run(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .output()
        .expect("run ncc")
}

fn output_text(output: &Output) -> String {
    format!(
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn debug_and_opt_in_hardening_bundle_is_material_verified_and_tamper_evident() {
    let raw = unique_bundle("raw");
    let raw_output = run(&[
        "machine",
        "kernel-system",
        "debug-hardening",
        "--target",
        "x86_64-unknown-none",
        "--bundle",
        raw.to_str().expect("UTF-8 raw bundle"),
    ]);
    assert!(raw_output.status.success(), "{}", output_text(&raw_output));
    let raw_report: Value = serde_json::from_slice(
        &fs::read(raw.join("kernel-debug-hardening-report.json")).expect("raw report"),
    )
    .expect("raw JSON");
    assert_eq!(
        raw_report["manual_contract"]["selected"]["enabled_count"],
        0
    );
    assert_eq!(raw_report["hardening"]["raw_rwx_allowed"], true);
    assert_eq!(raw_report["manual_contract"]["hidden_allocation"], false);

    let hardened = unique_bundle("all");
    let hardened_output = run(&[
        "machine",
        "kernel-system",
        "debug-hardening",
        "--target",
        "x86_64-unknown-none",
        "--enable",
        "all",
        "--bundle",
        hardened.to_str().expect("UTF-8 hardened bundle"),
    ]);
    assert!(
        hardened_output.status.success(),
        "{}",
        output_text(&hardened_output)
    );
    for artifact in [
        "kernel-crash.nccrash",
        "kernel-panic-serial.log",
        "kernel-serial-debug.rsp",
        "kernel-debug-traps.bin",
        "kernel-relocated-image.bin",
        "kernel-debug-hardening-report.json",
        "kernel-debug-hardening-certificate.json",
    ] {
        assert!(hardened.join(artifact).is_file(), "missing {artifact}");
    }
    let verify = run(&[
        "machine",
        "kernel-system",
        "verify-debug-hardening",
        hardened.to_str().expect("UTF-8 hardened bundle"),
    ]);
    assert!(verify.status.success(), "{}", output_text(&verify));
    let verified: Value = serde_json::from_slice(&verify.stdout).expect("verify JSON");
    assert_eq!(verified["status"], "verified");

    let crash_path = hardened.join("kernel-crash.nccrash");
    let mut crash = fs::read(&crash_path).expect("crash bytes");
    assert_eq!(&crash[..8], b"NCCRASH1");
    let last = crash.last_mut().expect("non-empty crash");
    *last ^= 0x80;
    fs::write(&crash_path, crash).expect("tamper crash");
    let rejected = run(&[
        "machine",
        "kernel-system",
        "verify-debug-hardening",
        hardened.to_str().expect("UTF-8 hardened bundle"),
    ]);
    assert!(!rejected.status.success());
    assert!(output_text(&rejected).contains("artifact hash mismatch"));

    let _ = fs::remove_dir_all(raw);
    let _ = fs::remove_dir_all(hardened);
}
