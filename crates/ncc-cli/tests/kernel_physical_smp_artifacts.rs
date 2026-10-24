use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ncc-kernel-physical-smp-artifacts-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp directory");
    path
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!(
            "cannot read retained artifact '{}': {error}",
            path.display()
        )
    });
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn compact_physical_smp_examples_materialize_without_qemu() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = unique_temp_dir();
    let artifact_root = temp.join(".ncc");
    let cases = [
        (
            "kernel_physical_smp_x86",
            "x86_64-unknown-none",
            [
                "__ncc_kernel_x86_ap_prepare",
                "__ncc_kernel_x86_apic_command",
            ],
        ),
        (
            "kernel_physical_smp_aarch64",
            "aarch64-unknown-none",
            ["smc #0", "icc_sgi1r_el1"],
        ),
        (
            "kernel_physical_smp_riscv",
            "riscv64-unknown-elf",
            ["ecall", "sfence.vma"],
        ),
    ];

    for (stem, target, llvm_needles) in cases {
        let source = root.join(format!("examples/11_machine/{stem}.n"));
        let source_text = fs::read_to_string(&source).expect("physical SMP source");
        assert!(!source_text.contains("use::"));
        assert!(!source_text.contains("[@kernel]"));
        assert!(source_text.contains("main: prepare;"));

        let output = Command::new(ncc_bin())
            .args([
                "build",
                source.to_string_lossy().as_ref(),
                "--profile",
                "kernel",
                "--target",
                target,
                "--out-dir",
                artifact_root.to_string_lossy().as_ref(),
                "--json",
            ])
            .current_dir(&temp)
            .env("NC_NO_COLOR", "1")
            .output()
            .expect("ncc executes");
        assert!(
            output.status.success(),
            "kernel build failed for {target}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let manifest_path = artifact_root
            .join("run")
            .join(format!("{stem}.{target}.kernel-build.json"));
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).expect("kernel manifest"))
                .expect("kernel manifest JSON");
        assert_eq!(manifest["target"]["triple"], target);
        assert_eq!(manifest["status"], "kernel_build_passed");
        assert_eq!(
            manifest["kernel_contract"]["full_high_level_kernel_body_instruction_lowering"],
            true
        );

        let llvm_path = artifact_root
            .join("lowering")
            .join(format!("{stem}.{target}.kernel.body.ll"));
        let llvm = fs::read_to_string(&llvm_path).expect("kernel body LLVM");
        for needle in llvm_needles {
            assert!(llvm.contains(needle), "{target} LLVM lacks '{needle}'");
        }

        for path in [
            artifact_root
                .join("code")
                .join(format!("{stem}.{target}.kernel.nm")),
            artifact_root
                .join("obj")
                .join(format!("{stem}.{target}.kernel.obj")),
            artifact_root
                .join("bin")
                .join(format!("{stem}.{target}.kernel.elf")),
        ] {
            let metadata = fs::metadata(&path)
                .unwrap_or_else(|error| panic!("missing '{}': {error}", path.display()));
            assert!(metadata.is_file() && metadata.len() > 0);
        }

        match target {
            "x86_64-unknown-none" => {
                let contract = &manifest["boot_artifact"]["physical_smp"]["ap_trampoline"];
                assert_eq!(contract["size"], 314);
                assert_eq!(contract["load_address"], "0x7000");
                assert_eq!(contract["mailbox_address"], "0x6000");
                let path = PathBuf::from(contract["path"].as_str().expect("trampoline path"));
                let bytes = fs::read(&path).expect("trampoline bytes");
                assert_eq!(bytes.len(), 314);
                assert_eq!(&bytes[..4], &[0xfa, 0xfc, 0x31, 0xc0]);
                assert_eq!(contract["sha256"], sha256(&path));
                assert!(manifest["boot_artifact"]["uefi"]["application"]["path"]
                    .as_str()
                    .is_some());
                assert!(manifest["boot_artifact"]["multiboot2"]["path"]
                    .as_str()
                    .is_some());
            }
            "aarch64-unknown-none" => {
                assert!(manifest["boot_artifact"]["uefi"]["application"]["path"]
                    .as_str()
                    .is_some());
                assert!(manifest["boot_artifact"]["uefi"]["disk"]["path"]
                    .as_str()
                    .is_some());
            }
            "riscv64-unknown-elf" => {
                assert!(manifest["boot_artifact"]["opensbi"]["flat_payload"]
                    .as_str()
                    .is_some());
                assert_eq!(manifest["boot_artifact"]["opensbi"]["next_mode"], "S-mode");
            }
            _ => unreachable!(),
        }
    }

    fs::remove_dir_all(temp).expect("temp cleanup");
}
