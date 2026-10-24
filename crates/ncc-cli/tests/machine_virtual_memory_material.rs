use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_temp_dir(target: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-kernel-vm-{}-{nanos}-{}",
        std::process::id(),
        target.replace('-', "_")
    ));
    fs::create_dir_all(&path).expect("temp directory");
    path
}

fn run(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .output()
        .expect("ncc executes")
}

fn text(output: &Output) -> String {
    format!(
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn certify(target: &str, bundle: &Path) -> Value {
    let output = run(&[
        "machine".into(),
        "kernel-system".into(),
        "virtual-memory".into(),
        "--target".into(),
        target.into(),
        "--bundle".into(),
        bundle.to_string_lossy().into_owned(),
    ]);
    assert!(output.status.success(), "{}", text(&output));
    serde_json::from_slice(&output.stdout).expect("certificate JSON")
}

#[test]
fn material_page_tables_cow_faults_huge_pages_and_shootdowns_cover_all_architectures() {
    for (target, profile, activation) in [
        ("x86_64-unknown-none", "x86_64-pml4-4k-v1", "cr3"),
        ("aarch64-unknown-none", "aarch64-stage1-4k-v1", "ttbr0_el1"),
        ("riscv64gc-unknown-none-elf", "riscv64-sv39-v1", "satp"),
    ] {
        let bundle = unique_temp_dir(target);
        let certificate = certify(target, &bundle);
        assert_eq!(certificate["payload"]["profile"], profile);
        let report: Value = serde_json::from_slice(
            &fs::read(bundle.join("kernel-virtual-memory-report.json")).expect("report"),
        )
        .expect("report JSON");
        assert_eq!(report["activation"]["register"], activation);
        assert_eq!(report["faults"]["guard"], "guard_page");
        assert_eq!(report["faults"]["cow"], "copy_on_write_required");
        assert_eq!(report["shootdown"]["complete"], true);
        let mappings = report["mappings"].as_array().expect("mappings");
        assert!(mappings
            .iter()
            .any(|row| row["page_size"] == 1_073_741_824_u64));
        assert!(mappings.iter().any(|row| {
            row["virtual_address"] == 0x40_0000_u64
                && row["flags"].as_u64().is_some_and(|flags| flags & 8 != 0)
        }));
        assert!(mappings.iter().any(|row| {
            row["virtual_address"] == 0x4000_0000_u64
                && row["flags"].as_u64().is_some_and(|flags| flags & 8 == 0)
        }));
        assert!(
            fs::metadata(bundle.join("page-tables.bin"))
                .expect("table image")
                .len()
                > 4096
        );

        let verify = run(&[
            "machine".into(),
            "kernel-system".into(),
            "verify-virtual-memory".into(),
            bundle.to_string_lossy().into_owned(),
        ]);
        assert!(verify.status.success(), "{}", text(&verify));
        let verified: Value = serde_json::from_slice(&verify.stdout).expect("verification JSON");
        assert_eq!(verified["status"], "verified");
        let _ = fs::remove_dir_all(bundle);
    }
}
