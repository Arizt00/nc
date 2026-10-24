use ncc_machine::{build_test_elf64, create_ext2_writable_image, UnifiedKernelImage};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn temporary_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-machine-kernel-autonomous-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn autonomous_cli_builds_boots_verifies_and_rejects_tampering() {
    let root = temporary_root("e2e");
    let bundle = root.join("evidence");
    let output = root.join("kernel.nki");
    fs::create_dir_all(&root).unwrap();
    let kernel = root.join("kernel.elf");
    let init = root.join("init.elf");
    let rootfs = root.join("root.ext2");
    fs::write(&kernel, build_test_elf64(62, 0x20_0000, &[0xfa, 0xf4])).unwrap();
    fs::write(&init, build_test_elf64(62, 0x40_0000, &[0x31, 0xc0, 0xc3])).unwrap();
    fs::write(&rootfs, create_ext2_writable_image()).unwrap();

    let built = Command::new(ncc())
        .args(["machine", "kernel-system", "autonomous", "--kernel"])
        .arg(&kernel)
        .arg("--rootfs")
        .arg(&rootfs)
        .args(["--root-format", "ext2", "--init"])
        .arg(&init)
        .args([
            "--target",
            "x86_64-unknown-none",
            "--frames",
            "256",
            "--cpus",
            "4",
            "--ticks",
            "3",
            "--bundle",
        ])
        .arg(&bundle)
        .arg("--out")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(output.is_file());
    let image = fs::read(bundle.join("kernel-autonomous.nki")).unwrap();
    assert!(UnifiedKernelImage::decode(&image).is_ok());
    let report: Value =
        serde_json::from_slice(&fs::read(bundle.join("kernel-autonomous-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["image"]["components"], 6);
    assert_eq!(report["boot"]["init_pid"], 1);
    assert_eq!(report["boot"]["cpus"], 4);
    assert_eq!(report["boot"]["network_online"], true);
    assert_eq!(report["execution_boundary"]["single_image"], true);
    assert_eq!(
        report["execution_boundary"]["firmware_or_board_entry_executed"],
        false
    );

    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-autonomous"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );

    let image_path = bundle.join("kernel-autonomous.nki");
    let mut damaged = fs::read(&image_path).unwrap();
    let last = damaged.len() - 1;
    damaged[last] ^= 0x5a;
    fs::write(&image_path, damaged).unwrap();
    let rejected = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-autonomous"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("artifact hash mismatch"));
    let _ = fs::remove_dir_all(root);
}
