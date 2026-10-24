use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn temporary_bundle(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ncc-machine-kernel-io-storage-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn physical_driver_and_storage_certificate_is_material_and_independently_verified() {
    let bundle = temporary_bundle("valid");
    let output = Command::new(ncc())
        .args(["machine", "kernel-system", "io-storage", "--bundle"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(bundle.join("kernel-io-storage-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["drivers"]["keyboard_scancode"], 0x1c);
    assert_eq!(report["drivers"]["nvme_submission_bytes"], 64);
    assert_eq!(report["drivers"]["ahci_command_list_bytes"], 1024);
    assert_eq!(report["drivers"]["xhci_trb_bytes"], 16);
    assert_eq!(report["drivers"]["recovered_faults"], 1);
    assert_eq!(report["storage"]["files_materialized"], 3);
    assert_eq!(report["storage"]["permission_rejections"], 1);
    assert_eq!(report["storage"]["busy_unmount_rejections"], 1);
    assert_eq!(report["storage"]["cache_evictions"], 1);
    assert_eq!(
        report["storage"]["writable_filesystems"],
        serde_json::json!(["Fat12", "Ext2"])
    );
    assert!(report["storage"]["journal_commits"].as_u64().unwrap() >= 3);
    assert_eq!(report["storage"]["journal_replays"], 1);
    assert!(report["storage"]["indirect_blocks"].as_u64().unwrap() > 0);
    assert!(report["storage"]["acl_blocks"].as_u64().unwrap() > 0);
    assert!(report["storage"]["long_names"].as_u64().unwrap() > 0);
    assert_eq!(report["physical_execution"]["performed"], false);

    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-io-storage"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let _ = fs::remove_dir_all(bundle);
}

#[test]
fn io_storage_certificate_rejects_a_modified_report() {
    let bundle = temporary_bundle("tamper");
    assert!(Command::new(ncc())
        .args(["machine", "kernel-system", "io-storage", "--bundle"])
        .arg(&bundle)
        .status()
        .unwrap()
        .success());
    let report = bundle.join("kernel-io-storage-report.json");
    let mut bytes = fs::read(&report).unwrap();
    let index = bytes.iter().position(|byte| *byte == b'1').unwrap();
    bytes[index] = b'2';
    fs::write(&report, bytes).unwrap();
    let verified = Command::new(ncc())
        .args(["machine", "kernel-system", "verify-io-storage"])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(!verified.status.success());
    assert!(String::from_utf8_lossy(&verified.stderr).contains("report hash mismatch"));
    let _ = fs::remove_dir_all(bundle);
}
