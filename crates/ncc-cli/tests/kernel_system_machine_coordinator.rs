use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ncc"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ncc-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(workspace_root())
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc executes")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context}\nstdout={}\nstderr={}",
        text(&output.stdout),
        text(&output.stderr)
    );
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).expect("JSON file bytes")).expect("valid JSON")
}

#[test]
fn machine_coordinates_every_full_kernel_target_through_the_real_cli() {
    for (target, architecture, bits) in [
        ("x86_64-unknown-none", "x86_64", 64),
        ("i386-unknown-none-elf", "i386", 32),
        ("aarch64-unknown-none", "aarch64", 64),
        ("armv7a-unknown-none-eabi", "armv7a-thumb2", 32),
        ("riscv64-unknown-elf", "riscv64", 64),
        ("riscv32imac-unknown-none-elf", "riscv32imac", 32),
    ] {
        let output = run(&["machine", "kernel-system", "coordinate", "--target", target]);
        assert_success(&output, target);
        let payload: Value = serde_json::from_slice(&output.stdout).expect("coordination JSON");
        assert_eq!(payload["status"], "machine_authorized");
        assert_eq!(payload["architecture"], architecture);
        assert_eq!(payload["pointer_width"], bits);
        assert_eq!(
            payload["subsystems"].as_array().map(Vec::len),
            Some(ncc_machine::COMPLETE_KERNEL_SUBSYSTEMS.len()),
            "{target} must coordinate the complete subsystem graph"
        );
        assert!(payload["registers"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()));
        assert!(payload["instructions"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()));
        assert!(payload["relocations"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()));
        assert_eq!(payload["manual_authority"]["source_coordinator"], ".n");
        assert_eq!(payload["manual_authority"]["source_mode"], "manual_raw");
        assert_eq!(payload["manual_authority"]["hidden_heap"], false);
        assert_eq!(payload["manual_authority"]["hidden_scheduler"], false);
    }

    let micro = run(&[
        "machine",
        "kernel-system",
        "coordinate",
        "--target",
        "thumbv7em-none-eabi",
    ]);
    assert_success(&micro, "Cortex-M machine coordination");
    let payload: Value = serde_json::from_slice(&micro.stdout).expect("micro coordination JSON");
    assert_eq!(payload["scope"], "firmware-microcontroller");
    assert_eq!(payload["architecture"], "thumbv7em");
    assert_eq!(payload["subsystems"].as_array().map(Vec::len), Some(3));
}

#[test]
fn host_selftest_certifies_subsystems_and_rejects_tampered_evidence() {
    let parent = unique_temp_dir("kernel-system-evidence");
    let bundle = parent.join("bundle");
    let bundle_text = bundle.to_string_lossy().into_owned();
    let selftest = run(&[
        "machine",
        "kernel-system",
        "selftest",
        "--target",
        "x86_64-unknown-none",
        "--bundle",
        &bundle_text,
    ]);
    assert_success(&selftest, "kernel-system selftest");

    let report = read_json(bundle.join("kernel-system-report.json"));
    assert_eq!(report["status"], "passed");
    assert_eq!(report["processes"]["cow_parent"], "parent");
    assert_eq!(report["processes"]["cow_child"], "child!");
    assert_eq!(report["storage"]["nvme_roundtrip_bytes"], 512);
    assert_eq!(report["network"]["udp_payload"], "kernel-net");
    assert_eq!(report["armv7_thumb"]["return_r0"], 42);
    assert_eq!(report["host_execution"]["qemu"], false);

    let verify = run(&["machine", "kernel-system", "verify", &bundle_text]);
    assert_success(&verify, "untampered certificate verification");

    fs::write(bundle.join("init.elf"), b"tampered").expect("tamper fixture");
    let rejected = run(&["machine", "kernel-system", "verify", &bundle_text]);
    assert!(!rejected.status.success(), "tampered evidence must fail");
    assert!(
        text(&rejected.stderr).contains("artifact hash mismatch: init.elf"),
        "stderr={}",
        text(&rejected.stderr)
    );

    fs::remove_dir_all(parent).expect("remove temporary evidence");
}

#[test]
fn resistance_harness_executes_six_families_and_seals_evidence() {
    let parent = unique_temp_dir("kernel-stress-evidence");
    let bundle = parent.join("bundle");
    let bundle_text = bundle.to_string_lossy().into_owned();
    let stress = run(&[
        "machine",
        "kernel-system",
        "stress",
        "--profile",
        "smoke",
        "--bundle",
        &bundle_text,
    ]);
    assert_success(&stress, "kernel resistance harness");

    let report = read_json(bundle.join("kernel-stress-report.json"));
    assert_eq!(report["schema"], "ncc.machine.kernel_stress_report.v1");
    assert_eq!(report["status"], "passed");
    assert_eq!(report["host_execution"]["qemu"], false);
    assert_eq!(report["metrics"].as_array().map(Vec::len), Some(6));
    let scenarios = report["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|metric| metric["scenario"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        scenarios,
        [
            "vm-exit-storm",
            "smp-contention",
            "cow-page-faults",
            "invalid-dma",
            "simultaneous-interrupts",
            "snapshot-restore",
        ]
    );
    assert!(report["summary"]["total_operations"]
        .as_u64()
        .is_some_and(|operations| operations > 90_000));

    let verify = run(&["machine", "kernel-system", "verify-stress", &bundle_text]);
    assert_success(&verify, "untampered stress certificate verification");

    fs::write(
        bundle.join("kernel-stress-summary.txt"),
        b"tampered resistance evidence",
    )
    .expect("tamper stress evidence");
    let rejected = run(&["machine", "kernel-system", "verify-stress", &bundle_text]);
    assert!(
        !rejected.status.success(),
        "tampered stress evidence must fail"
    );
    assert!(
        text(&rejected.stderr).contains("stress artifact hash mismatch"),
        "stderr={}",
        text(&rejected.stderr)
    );

    fs::remove_dir_all(parent).expect("remove temporary stress evidence");
}

#[test]
fn raw_memory_harness_executes_heap_realloc_fragmentation_and_concurrency() {
    let parent = unique_temp_dir("raw-memory-stress-evidence");
    let bundle = parent.join("bundle");
    let bundle_text = bundle.to_string_lossy().into_owned();
    let stress = run(&[
        "machine",
        "kernel-system",
        "raw-memory-stress",
        "--profile",
        "smoke",
        "--bundle",
        &bundle_text,
    ]);
    assert_success(&stress, "raw memory resistance harness");

    let report = read_json(bundle.join("raw-memory-stress-report.json"));
    assert_eq!(report["schema"], "ncc.machine.raw_memory_stress_report.v1");
    assert_eq!(report["status"], "passed");
    assert_eq!(report["host_execution"]["qemu"], false);
    let scenarios = report["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|metric| metric["scenario"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        scenarios,
        ["heap-realloc", "fragmentation", "concurrent-atomics"]
    );
    assert_eq!(report["summary"]["total_operations"], 130_000);
    assert!(report["summary"]["checksum"]
        .as_u64()
        .is_some_and(|checksum| checksum != 0));

    let verify = run(&[
        "machine",
        "kernel-system",
        "verify-raw-memory-stress",
        &bundle_text,
    ]);
    assert_success(&verify, "untampered raw memory certificate verification");

    fs::write(
        bundle.join("raw-memory-stress-summary.txt"),
        b"tampered raw memory evidence",
    )
    .expect("tamper raw memory evidence");
    let rejected = run(&[
        "machine",
        "kernel-system",
        "verify-raw-memory-stress",
        &bundle_text,
    ]);
    assert!(!rejected.status.success(), "tampered evidence must fail");
    assert!(
        text(&rejected.stderr).contains("raw memory stress artifact hash mismatch"),
        "stderr={}",
        text(&rejected.stderr)
    );

    fs::remove_dir_all(parent).expect("remove temporary raw memory evidence");
}

#[test]
fn thumb_nm_emits_executes_and_packages_real_arm_bytes() {
    let root = workspace_root();
    let source = root.join("examples/machine/thumbv7em_return42.nm");
    let temp = unique_temp_dir("thumb-machine-pipeline");
    let raw = temp.join("return42.bin");
    let object = temp.join("return42.o");

    let check = run(&["machine", "check", &source.to_string_lossy()]);
    assert_success(&check, "Thumb .nm check");

    let emit_bytes = run(&[
        "machine",
        "emit-bytes",
        &source.to_string_lossy(),
        "--out",
        &raw.to_string_lossy(),
    ]);
    assert_success(&emit_bytes, "Thumb byte emission");
    assert_eq!(
        fs::read(&raw).expect("Thumb bytes"),
        [0x28, 0x20, 0x02, 0x30, 0x07, 0xdf, 0x70, 0x47]
    );

    let emit_object = run(&[
        "machine",
        "emit-object",
        &source.to_string_lossy(),
        "--target",
        "armv7em-none-eabi",
        "--out",
        &object.to_string_lossy(),
    ]);
    assert_success(&emit_object, "ARM ELF32 object emission");
    let object_bytes = fs::read(&object).expect("ARM ELF32 object");
    assert!(object_bytes.starts_with(b"\x7fELF"));
    assert_eq!(object_bytes[4], 1, "ELFCLASS32");
    assert_eq!(u16::from_le_bytes([object_bytes[18], object_bytes[19]]), 40);

    let verify = run(&["machine", "verify-object", &object.to_string_lossy()]);
    assert_success(&verify, "ARM ELF32 object verification");
    assert!(text(&verify.stdout).contains("\"object_format\": \"elf32\""));

    fs::remove_dir_all(temp).expect("remove temporary machine artifacts");
}

#[test]
fn thumb_kernel_primitives_match_the_independent_clang_oracle() {
    let root = workspace_root();
    let source = root.join("examples/machine/thumbv7em_kernel_primitives.nm");
    let temp = unique_temp_dir("thumb-kernel-primitives");
    let raw = temp.join("kernel-primitives.bin");
    let output = run(&[
        "machine",
        "emit-bytes",
        &source.to_string_lossy(),
        "--out",
        &raw.to_string_lossy(),
    ]);
    assert_success(&output, "Thumb kernel primitive encoding");
    assert_eq!(
        fs::read(&raw).expect("Thumb kernel primitive bytes"),
        [
            0xef, 0xf3, 0x10, 0x80, 0x80, 0xf3, 0x10, 0x88, 0xef, 0xf3, 0x14, 0x81, 0x81, 0xf3,
            0x14, 0x88, 0x51, 0xe8, 0x00, 0x0f, 0x41, 0xe8, 0x00, 0x02, 0xbf, 0xf3, 0x2f, 0x8f,
            0x08, 0x40, 0x08, 0x43, 0x48, 0x40, 0x48, 0x43, 0xc0, 0x00, 0x00, 0x09, 0x40, 0x11,
            0x98, 0x47, 0x70, 0x47,
        ]
    );
    fs::remove_dir_all(temp).expect("remove temporary machine artifacts");
}

#[test]
fn normal_compact_n_builds_armv7_elf_and_cortex_m_physical_firmware() {
    let root = workspace_root();
    let source = root.join("examples/11_machine/kernel_subsystems_compact.n");
    let temp = unique_temp_dir("arm-kernel-firmware-e2e");

    let arm_root = temp.join("armv7a");
    let arm = run(&[
        "build",
        &source.to_string_lossy(),
        "--target",
        "armv7a-unknown-none-eabi",
        "--profile",
        "kernel",
        "--out-dir",
        &arm_root.to_string_lossy(),
        "--json",
    ]);
    assert_success(&arm, "normal .n -> ARMv7-A ELF32 kernel");
    let arm_elf =
        arm_root.join("bin/kernel_subsystems_compact.armv7a-unknown-none-eabi.kernel.elf");
    let arm_bytes = fs::read(&arm_elf).expect("ARMv7-A kernel ELF");
    assert!(arm_bytes.starts_with(b"\x7fELF\x01\x01\x01"));
    assert_eq!(u16::from_le_bytes([arm_bytes[18], arm_bytes[19]]), 40);
    let entry = u32::from_le_bytes(arm_bytes[24..28].try_into().unwrap());
    assert_eq!(entry & 1, 1, "ELF entry must retain Thumb state");
    let arm_manifest = read_json(
        arm_root.join("run/kernel_subsystems_compact.armv7a-unknown-none-eabi.kernel-build.json"),
    );
    assert_eq!(
        arm_manifest["kernel_contract"]["kernel_elf32_executable"],
        true
    );
    assert_eq!(
        arm_manifest["truth_boundary"]["kernel_elf32_executable"],
        true
    );
    assert_eq!(
        arm_manifest["truth_boundary"]["kernel_link_artifact"],
        false
    );

    let firmware_root = temp.join("thumbv7em");
    let firmware_build = run(&[
        "build",
        &source.to_string_lossy(),
        "--target",
        "armv7em-none-eabi",
        "--profile",
        "kernel",
        "--out-dir",
        &firmware_root.to_string_lossy(),
        "--json",
    ]);
    assert_success(
        &firmware_build,
        "normal .n -> Cortex-M flat binary and Intel HEX",
    );
    let firmware_path =
        firmware_root.join("bin/kernel_subsystems_compact.armv7em-none-eabi.firmware.bin");
    let firmware = fs::read(&firmware_path).expect("Cortex-M firmware binary");
    assert!(firmware.len() > 128);
    assert_eq!(
        u32::from_le_bytes(firmware[0..4].try_into().unwrap()),
        0x2008_0000
    );
    assert_eq!(
        u32::from_le_bytes(firmware[4..8].try_into().unwrap()),
        0x0800_0041
    );
    let default_handler = u32::from_le_bytes(firmware[8..12].try_into().unwrap());
    assert_eq!(default_handler & 1, 1);
    assert!(firmware[8..64]
        .chunks_exact(4)
        .all(|slot| u32::from_le_bytes(slot.try_into().unwrap()) == default_handler));
    let hex = fs::read_to_string(
        firmware_root.join("bin/kernel_subsystems_compact.armv7em-none-eabi.firmware.hex"),
    )
    .expect("Intel HEX firmware");
    assert!(hex.starts_with(":020000040800F2\n"));
    assert!(hex.ends_with(":00000001FF\n"));
    let report = read_json(format!("{}.link-report.json", firmware_path.display()));
    assert_eq!(report["artifact_format"], "cortex-m-flat-binary");
    assert_eq!(report["firmware_evidence"]["data_copy_loop"], true);
    assert_eq!(report["firmware_evidence"]["bss_zero_loop"], true);
    assert_eq!(report["truth_boundary"]["qemu_required"], false);
    let firmware_manifest = read_json(
        firmware_root.join("run/kernel_subsystems_compact.armv7em-none-eabi.kernel-build.json"),
    );
    assert_eq!(
        firmware_manifest["truth_boundary"]["kernel_elf32_executable"],
        false
    );
    assert_eq!(
        firmware_manifest["truth_boundary"]["kernel_link_artifact"],
        false
    );

    fs::remove_dir_all(temp).expect("remove temporary ARM build artifacts");
}

#[test]
fn normal_n_retains_manual_raw_machine_authority_in_kernel_artifacts() {
    let root = workspace_root();
    let source = root.join("examples/11_machine/kernel_manual_machine_authority.n");
    let temp = unique_temp_dir("manual-kernel-machine-authority");
    let output = run(&[
        "build",
        &source.to_string_lossy(),
        "--target",
        "x86_64-unknown-none",
        "--profile",
        "kernel",
        "--out-dir",
        &temp.to_string_lossy(),
        "--json",
    ]);
    assert_success(&output, "manual raw .n kernel build");

    let manifest = read_json(
        temp.join("run/kernel_manual_machine_authority.x86_64-unknown-none.kernel-build.json"),
    );
    let authority = &manifest["machine_plan"]["manual_authority"];
    assert_eq!(authority["source_coordinator"], ".n");
    assert_eq!(authority["source_mode"], "manual_raw");
    assert_eq!(authority["raw_memory"], true);
    assert_eq!(authority["unchecked_out_of_range"], true);
    assert_eq!(authority["explicit_register_state"], true);
    assert_eq!(authority["explicit_abi"], true);
    assert_eq!(authority["explicit_relocations"], true);
    assert_eq!(
        authority["compiler_inserts"].as_array().map(Vec::len),
        Some(0)
    );
    assert_eq!(authority["hidden_bounds_checks"], false);
    assert_eq!(authority["hidden_heap"], false);
    assert_eq!(authority["hidden_scheduler"], false);

    let lowering =
        read_json(temp.join(
            "lowering/kernel_manual_machine_authority.x86_64-unknown-none.kernel.lowering.json",
        ));
    assert_eq!(lowering["lowering"]["manual_raw_default"], true);
    assert_eq!(
        lowering["lowering"]["programmer_controls_registers_abi_and_relocations"],
        true
    );
    assert_eq!(lowering["lowering"]["compiler_inserted_heap"], false);
    let llvm_path = lowering["high_level_body_llvm_ir"]
        .as_str()
        .expect("manual kernel body LLVM path");
    let llvm = fs::read_to_string(llvm_path).expect("manual kernel LLVM");
    for symbol in [
        "@__ncc_kernel_cpu_read_cr3",
        "@__ncc_kernel_port_read8",
        "@__ncc_kernel_tlb_invalidate",
    ] {
        assert!(
            llvm.contains(symbol),
            "missing native manual symbol {symbol}"
        );
    }
    let image = temp.join("bin/kernel_manual_machine_authority.x86_64-unknown-none.kernel.elf");
    assert!(fs::read(image).expect("kernel ELF").starts_with(b"\x7fELF"));
    fs::remove_dir_all(temp).expect("remove manual kernel artifacts");
}
