use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "ncc-guest-{label}-{}-{nonce}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn run(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .env("SOURCE_DATE_EPOCH", "1784318400")
        .output()
        .expect("ncc executes")
}

fn run_ok(args: &[String]) -> Output {
    let output = run(args);
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn bzimage_fixture() -> Vec<u8> {
    let mut image = vec![0x90; 32 * 1024];
    image[0x1f1] = 4;
    image[0x202..0x206].copy_from_slice(b"HdrS");
    image[0x206..0x208].copy_from_slice(&0x020c_u16.to_le_bytes());
    image[0x211] = 1;
    image[0x214..0x218].copy_from_slice(&0x10_0000_u32.to_le_bytes());
    image[0x22c..0x230].copy_from_slice(&0x37ff_ffff_u32.to_le_bytes());
    image[0x230..0x234].copy_from_slice(&0x20_0000_u32.to_le_bytes());
    image[0x234] = 1;
    image[0x238..0x23c].copy_from_slice(&2048_u32.to_le_bytes());
    let image_len = image.len() as u32;
    image[0x260..0x264].copy_from_slice(&image_len.to_le_bytes());
    image
}

fn riscv_elf64_fixture(load: u64) -> Vec<u8> {
    let mut elf = vec![0u8; 0x140];
    elf[0..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[16..18].copy_from_slice(&2_u16.to_le_bytes());
    elf[18..20].copy_from_slice(&243_u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[24..32].copy_from_slice(&load.to_le_bytes());
    elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
    elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
    elf[68..72].copy_from_slice(&5_u32.to_le_bytes());
    elf[72..80].copy_from_slice(&0x100_u64.to_le_bytes());
    elf[80..88].copy_from_slice(&load.to_le_bytes());
    elf[88..96].copy_from_slice(&load.to_le_bytes());
    elf[96..104].copy_from_slice(&64_u64.to_le_bytes());
    elf[104..112].copy_from_slice(&64_u64.to_le_bytes());
    elf[112..120].copy_from_slice(&0x1000_u64.to_le_bytes());
    elf[0x100..0x140].fill(0x13);
    elf
}

#[test]
fn guest_plans_for_bzimage_efi_and_opensbi_are_certified() {
    let temp = temp_dir("loaders");
    let initrd = temp.join("initrd.img");
    let firmware = temp.join("firmware.bin");
    let bzimage = temp.join("bzImage");
    let efi = temp.join("BOOTX64.EFI");
    let riscv = temp.join("Image.elf");
    fs::write(&initrd, b"ncc-initrd-material").unwrap();
    fs::write(&firmware, vec![0xff; 4096]).unwrap();
    fs::write(&bzimage, bzimage_fixture()).unwrap();
    fs::write(
        &efi,
        ncc_machine::build_x86_64_uefi_application(&[0x31, 0xc0, 0xc3]).unwrap(),
    )
    .unwrap();
    fs::write(&riscv, riscv_elf64_fixture(0x8020_0000)).unwrap();

    let mut relocated_certificates = Vec::new();
    for (name, boot, arch, kernel, needs_firmware) in [
        ("bzimage", "bzimage", "x86_64", &bzimage, false),
        ("efi", "efi", "x86_64", &efi, true),
        ("opensbi", "opensbi", "riscv64", &riscv, true),
    ] {
        let bundle = temp.join(format!("{name}-bundle"));
        let certificate = temp.join(format!("{name}-certificate.json"));
        let mut args = vec![
            "machine".into(),
            "virtualization".into(),
            "prepare-guest".into(),
            "--boot".into(),
            boot.into(),
            "--arch".into(),
            arch.into(),
            "--kernel".into(),
            path(kernel),
            "--initrd".into(),
            path(&initrd),
            "--cmdline".into(),
            "console=ttyS0 panic=-1".into(),
            "--memory-mib".into(),
            "128".into(),
            "--bundle".into(),
            path(&bundle),
            "--out".into(),
            path(&certificate),
        ];
        if needs_firmware {
            args.push("--firmware".into());
            args.push(path(&firmware));
        }
        run_ok(&args);
        run_ok(&[
            "machine".into(),
            "virtualization".into(),
            "verify-guest-certificate".into(),
            path(&certificate),
        ]);
        let value: Value = serde_json::from_slice(&fs::read(&certificate).unwrap()).unwrap();
        assert_eq!(
            value.pointer("/status").and_then(Value::as_str),
            Some("guest_boot_plan_prepared")
        );
        assert!(value
            .get("source_files")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .all(|source| source
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.starts_with("sources/"))));
        fs::copy(&certificate, bundle.join("guest-certificate.json")).unwrap();
        let relocated = temp.join(format!("{name}-bundle-relocated"));
        fs::rename(&bundle, &relocated).unwrap();
        relocated_certificates.push(relocated.join("guest-certificate.json"));
    }
    for source in [&initrd, &firmware, &bzimage, &efi, &riscv] {
        fs::remove_file(source).unwrap();
    }
    for certificate in relocated_certificates {
        run_ok(&[
            "machine".into(),
            "virtualization".into(),
            "verify-guest-certificate".into(),
            path(&certificate),
        ]);
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn live_migration_tcp_snapshot_roundtrip_is_certified() {
    let temp = temp_dir("migration");
    let bundle = temp.join("bundle");
    let certificate = temp.join("migration-certificate.json");
    run_ok(&[
        "machine".into(),
        "virtualization".into(),
        "certify-migration".into(),
        "--memory-mib".into(),
        "4".into(),
        "--vcpus".into(),
        "8".into(),
        "--bundle".into(),
        path(&bundle),
        "--out".into(),
        path(&certificate),
    ]);
    run_ok(&[
        "machine".into(),
        "virtualization".into(),
        "verify-migration-certificate".into(),
        path(&certificate),
    ]);
    let value: Value = serde_json::from_slice(&fs::read(&certificate).unwrap()).unwrap();
    assert_eq!(
        value
            .pointer("/truth_boundary/transactional_two_phase_commit")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        value.pointer("/execution/vcpus").and_then(Value::as_u64),
        Some(8)
    );
    assert!(bundle.join("restored.ncsnap").is_file());
    fs::copy(&certificate, bundle.join("migration-certificate.json")).unwrap();
    let relocated = temp.join("bundle-relocated");
    fs::rename(&bundle, &relocated).unwrap();
    run_ok(&[
        "machine".into(),
        "virtualization".into(),
        "verify-migration-certificate".into(),
        path(&relocated.join("migration-certificate.json")),
    ]);
    fs::remove_dir_all(temp).unwrap();
}
