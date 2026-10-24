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

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "ncc-virtualization-{}-{nonce}-{sequence}",
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
        .output()
        .expect("ncc executes")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn run_ok(args: &[String]) -> Output {
    let output = run(args);
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstdout={}\nstderr={}",
        text(&output.stdout),
        text(&output.stderr)
    );
    output
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("u16"))
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32"))
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("u64"))
}

fn elf64_symbol_type(bytes: &[u8], wanted: &str) -> Option<u8> {
    if !bytes.starts_with(b"\x7fELF") || bytes.get(4) != Some(&2) {
        return None;
    }
    let shoff = u64_at(bytes, 40) as usize;
    let shentsize = u16_at(bytes, 58) as usize;
    let shnum = u16_at(bytes, 60) as usize;
    let symtab_index = (0..shnum).find(|index| {
        let header = shoff + index * shentsize;
        u32_at(bytes, header + 4) == 2
    })?;
    let symtab_header = shoff + symtab_index * shentsize;
    let symtab_offset = u64_at(bytes, symtab_header + 24) as usize;
    let symtab_size = u64_at(bytes, symtab_header + 32) as usize;
    let strtab_index = u32_at(bytes, symtab_header + 40) as usize;
    let entry_size = u64_at(bytes, symtab_header + 56) as usize;
    let strtab_header = shoff + strtab_index * shentsize;
    let strtab_offset = u64_at(bytes, strtab_header + 24) as usize;
    let strtab_size = u64_at(bytes, strtab_header + 32) as usize;
    let strtab = &bytes[strtab_offset..strtab_offset + strtab_size];

    (0..symtab_size / entry_size).find_map(|index| {
        let entry = symtab_offset + index * entry_size;
        let name_offset = u32_at(bytes, entry) as usize;
        let end = strtab[name_offset..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|length| name_offset + length)?;
        (std::str::from_utf8(&strtab[name_offset..end]).ok()? == wanted)
            .then_some(bytes[entry + 4] & 0x0f)
    })
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn vmcs_vmcb_machine_and_kernel_pipeline_is_material() {
    let root = workspace_root();
    let machine_fixture = root.join("examples/machine/x86_virtualization_vmcs_vmcb.nm");
    let kernel_fixture = root.join("examples/11_machine/kernel_x86_virtualization_vmcs_vmcb.n");
    let temp = temp_dir();
    let bin = temp.join("virtualization.bin");
    let object = temp.join("virtualization.o");

    run_ok(&["check".into(), path_arg(&kernel_fixture)]);
    run_ok(&["machine".into(), "check".into(), path_arg(&machine_fixture)]);
    let machine_source = fs::read_to_string(&machine_fixture).expect("machine fixture source");
    let userland_fixture = temp.join("virtualization_userland.nm");
    fs::write(
        &userland_fixture,
        machine_source.replace("profile: kernel;", "profile: userland;"),
    )
    .expect("userland negative fixture");
    let userland = run(&[
        "machine".into(),
        "check".into(),
        path_arg(&userland_fixture),
    ]);
    assert!(!userland.status.success());
    let userland_diagnostic = format!("{}{}", text(&userland.stdout), text(&userland.stderr));
    assert!(
        userland_diagnostic.contains("E_MACHINE_PRIVILEGE_001"),
        "{userland_diagnostic}"
    );

    let feature_fixture = temp.join("virtualization_missing_features.nm");
    fs::write(
        &feature_fixture,
        machine_source.replace("features: vmx,ept,vpid,svm;", "features: vmx;"),
    )
    .expect("feature negative fixture");
    let feature = run(&["machine".into(), "check".into(), path_arg(&feature_fixture)]);
    assert!(!feature.status.success());
    let feature_diagnostic = format!("{}{}", text(&feature.stdout), text(&feature.stderr));
    assert!(
        feature_diagnostic.contains("E_MACHINE_FEATURE_REQUIRED_001")
            && feature_diagnostic.contains("'ept'")
            && feature_diagnostic.contains("'vpid'")
            && feature_diagnostic.contains("'svm'"),
        "{feature_diagnostic}"
    );
    run_ok(&[
        "machine".into(),
        "emit-bytes".into(),
        path_arg(&machine_fixture),
        "--out".into(),
        path_arg(&bin),
    ]);

    let expected = vec![
        0xf3, 0x0f, 0xc7, 0x30, 0x66, 0x0f, 0xc7, 0x30, 0x0f, 0xc7, 0x30, 0x0f, 0xc7, 0x38, 0x0f,
        0x78, 0xd8, 0x0f, 0x78, 0x18, 0x0f, 0x79, 0xc3, 0x0f, 0x79, 0x03, 0x66, 0x0f, 0x38, 0x80,
        0x03, 0x66, 0x0f, 0x38, 0x81, 0x03, 0x0f, 0x01, 0xc1, 0x0f, 0x01, 0xc4, 0x0f, 0x01, 0xc2,
        0x0f, 0x01, 0xc3, 0x0f, 0x01, 0xd8, 0x0f, 0x01, 0xd9, 0x0f, 0x01, 0xda, 0x0f, 0x01, 0xdb,
        0x0f, 0x01, 0xdc, 0x0f, 0x01, 0xdd, 0x0f, 0x01, 0xdf, 0xc3,
    ];
    assert_eq!(fs::read(&bin).expect("machine bytes"), expected);

    let disasm = run_ok(&[
        "machine".into(),
        "disasm-bytes".into(),
        path_arg(&bin),
        "--arch".into(),
        "x86_64".into(),
    ]);
    let disasm: Value = serde_json::from_slice(&disasm.stdout).expect("disasm JSON");
    assert_eq!(disasm["status"], "native_x86_64_bytes_fully_decoded");
    assert_eq!(disasm["covered_bytes"], 70);
    assert_eq!(disasm["total_bytes"], 70);
    assert_eq!(disasm["asm_complete"], true);
    assert_eq!(disasm["instructions"].as_array().map(Vec::len), Some(22));

    run_ok(&[
        "machine".into(),
        "emit-object".into(),
        path_arg(&machine_fixture),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--out".into(),
        path_arg(&object),
    ]);
    let object_bytes = fs::read(&object).expect("ELF object");
    assert_eq!(
        elf64_symbol_type(&object_bytes, "virtualizationEncodingProbe"),
        Some(2),
        "canonical .text entry must be ELF STT_FUNC"
    );

    let roundtrip = run_ok(&[
        "machine".into(),
        "roundtrip".into(),
        path_arg(&machine_fixture),
        "--target".into(),
        "x86_64-unknown-linux-gnu".into(),
        "--json".into(),
    ]);
    let roundtrip: Value = serde_json::from_slice(&roundtrip.stdout).expect("roundtrip JSON");
    assert_eq!(roundtrip["lossless"], true);
    assert_eq!(roundtrip["evidence"]["comparison_status"], "lossless");
    assert_eq!(roundtrip["original_ir"]["instruction_count"], 22);
    assert_eq!(roundtrip["disasm_ir"]["instruction_count"], 22);

    let report = run_ok(&["machine".into(), "virtualization".into(), "--json".into()]);
    let report: Value = serde_json::from_slice(&report.stdout).expect("virtualization JSON");
    assert_eq!(report["status"], "typed_and_encoded");
    assert_eq!(report["vmcs"]["region_bytes"], 4096);
    assert_eq!(report["vmcb"]["region_bytes"], 4096);
    assert_eq!(report["guest_physical_memory"]["table_bytes"], 4096);
    assert_eq!(report["guest_physical_memory"]["table_alignment"], 4096);
    assert_eq!(
        report["guest_physical_memory"]["transactional_pool_exhaustion"],
        true
    );
    assert_eq!(
        report["vmexit_engine"]["architectural_exception_vectors"],
        32
    );
    assert_eq!(report["vmexit_engine"]["entry_failure_requeue"], true);
    assert_eq!(report["vmexit_engine"]["double_fault_escalation"], true);
    assert_eq!(
        report["interrupt_platform"]["ioapic_redirection_entries"],
        24
    );
    assert_eq!(report["interrupt_platform"]["hidden_allocation"], false);
    assert_eq!(report["guest_boot"]["linux_zero_page"], true);
    assert_eq!(report["guest_boot"]["multiboot2_information"], true);
    assert_eq!(report["instructions"].as_array().map(Vec::len), Some(19));
    assert_eq!(
        report["execution_contract"]["cpu_backed_deadline_milliseconds"],
        15_000
    );
    assert_eq!(
        report["execution_contract"]["blocked_kvm_run_interruptible"],
        true
    );

    let rejected = run(&[
        "machine".into(),
        "run-bytes".into(),
        path_arg(&bin),
        "--raw-bin".into(),
    ]);
    assert!(!rejected.status.success());
    let diagnostic = format!("{}{}", text(&rejected.stdout), text(&rejected.stderr));
    assert!(
        diagnostic.contains("E_MACHINE_JIT_PRIVILEGED_001"),
        "{diagnostic}"
    );
    assert!(
        !diagnostic.contains("E_MACHINE_JIT_CHILD_001"),
        "{diagnostic}"
    );

    fs::remove_dir_all(temp).expect("temporary cleanup");
}

#[test]
fn guest_memory_ept_npt_and_vmexit_kernel_body_links_without_stub_fallback() {
    let root = workspace_root();
    let fixture = root.join("examples/11_machine/kernel_guest_memory_vmexit_loop.n");
    let temp = temp_dir();

    run_ok(&["check".into(), path_arg(&fixture)]);
    run_ok(&[
        "build".into(),
        path_arg(&fixture),
        "--target".into(),
        "x86_64-unknown-none".into(),
        "--profile".into(),
        "kernel".into(),
        "--out-dir".into(),
        path_arg(&temp),
    ]);

    let stem = "kernel_guest_memory_vmexit_loop";
    let manifest_path = temp.join(format!("run/{stem}.kernel-build.json"));
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("kernel build manifest"))
            .expect("kernel build JSON");
    assert_eq!(
        manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
        true
    );
    assert_eq!(
        manifest["truth_boundary"]["kmain_calls_high_level_main"],
        true
    );
    assert_eq!(
        manifest["link"]["phase12_report"]["unresolved_symbols"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        manifest["link"]["phase12_report"]["input_objects"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let llvm_path = manifest["paths"]["high_level_body_llvm_ir"]
        .as_str()
        .map(PathBuf::from)
        .expect("retained kernel LLVM path");
    let llvm = fs::read_to_string(llvm_path).expect("retained kernel LLVM");
    for symbol in [
        "@__ncc_kernel_guest_phys_read64",
        "@__ncc_kernel_ept_set_entry",
        "@__ncc_kernel_npt_set_entry",
        "@__ncc_kernel_vmexit_intel_snapshot",
        "@__ncc_kernel_vmexit_amd_snapshot",
        "@__ncc_kernel_vmexit_intel_inject",
        "@__ncc_kernel_vmexit_amd_inject",
        "@__ncc_kernel_vm_call",
        "@__ncc_kernel_svm_call",
        "@__ncc_kernel_svm_invlpga",
    ] {
        assert!(
            llvm.contains(symbol),
            "missing material runtime symbol {symbol}"
        );
    }

    fs::remove_dir_all(temp).expect("temporary cleanup");
}

#[test]
fn apic_smp_timers_and_guest_boot_kernel_body_link_without_imports_or_stubs() {
    let root = workspace_root();
    let fixture = root.join("examples/11_machine/kernel_apic_smp_guest_boot.n");
    let temp = temp_dir();

    run_ok(&["check".into(), path_arg(&fixture)]);
    run_ok(&[
        "build".into(),
        path_arg(&fixture),
        "--target".into(),
        "x86_64-unknown-none".into(),
        "--profile".into(),
        "kernel".into(),
        "--out-dir".into(),
        path_arg(&temp),
    ]);

    let manifest_path = temp.join("run/kernel_apic_smp_guest_boot.kernel-build.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("kernel build manifest"))
            .expect("kernel build JSON");
    assert_eq!(
        manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
        true
    );
    assert_eq!(
        manifest["truth_boundary"]["kmain_calls_high_level_main"],
        true
    );
    assert_eq!(
        manifest["link"]["phase12_report"]["unresolved_symbols"],
        serde_json::json!([])
    );
    let object_path = manifest["paths"]["high_level_body_object"]
        .as_str()
        .map(PathBuf::from)
        .expect("retained kernel body object");
    assert!(object_path.is_file(), "{}", object_path.display());

    let llvm_path = manifest["paths"]["high_level_body_llvm_ir"]
        .as_str()
        .map(PathBuf::from)
        .expect("retained kernel LLVM path");
    let llvm = fs::read_to_string(llvm_path).expect("retained kernel LLVM");
    for symbol in [
        "@__ncc_kernel_apic_eoi",
        "@__ncc_kernel_apic_send_ipi",
        "@__ncc_kernel_ioapic_read",
        "@__ncc_kernel_ioapic_write",
        "@__ncc_kernel_tsc_deadline_write",
        "@__ncc_kernel_cpu_read_msr",
        "@__ncc_kernel_mmio_read64",
    ] {
        assert!(
            llvm.contains(symbol),
            "missing material runtime symbol {symbol}"
        );
    }
    assert!(!llvm.contains("declare i1 @__ncc_kernel_apic_eoi"));
    assert!(!llvm.contains("declare i1 @__ncc_kernel_apic_send_ipi"));

    fs::remove_dir_all(temp).expect("temporary cleanup");
}

#[test]
fn pci_virtio_and_debug_kernel_body_link_without_imports_or_stubs() {
    let root = workspace_root();
    let fixture = root.join("examples/11_machine/kernel_virtual_devices_pci_virtio.n");
    let temp = temp_dir();

    run_ok(&["check".into(), path_arg(&fixture)]);
    run_ok(&[
        "build".into(),
        path_arg(&fixture),
        "--target".into(),
        "x86_64-unknown-none".into(),
        "--profile".into(),
        "kernel".into(),
        "--out-dir".into(),
        path_arg(&temp),
    ]);

    let manifest_path = temp.join("run/kernel_virtual_devices_pci_virtio.kernel-build.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("kernel build manifest"))
            .expect("kernel build JSON");
    assert_eq!(
        manifest["truth_boundary"]["full_high_level_kernel_body_instruction_lowering"],
        true
    );
    assert_eq!(
        manifest["link"]["phase12_report"]["unresolved_symbols"],
        serde_json::json!([])
    );

    let llvm_path = manifest["paths"]["high_level_body_llvm_ir"]
        .as_str()
        .map(PathBuf::from)
        .expect("retained kernel LLVM path");
    let llvm = fs::read_to_string(llvm_path).expect("retained kernel LLVM");
    for symbol in [
        "@__ncc_kernel_pci_config_read32",
        "@__ncc_kernel_pci_config_write32",
        "@__ncc_kernel_pci_ecam_read32",
        "@__ncc_kernel_pci_ecam_write32",
        "@__ncc_kernel_virtio_notify",
        "@__ncc_kernel_debug_break",
    ] {
        assert!(
            llvm.contains(symbol),
            "missing material runtime symbol {symbol}"
        );
    }
    assert!(!llvm.contains("declare i64 @__ncc_kernel_pci_config_read32"));
    assert!(!llvm.contains("declare i1 @__ncc_kernel_virtio_notify"));
    assert!(!llvm.contains("declare i1 @__ncc_kernel_debug_break"));
    assert!(llvm.contains("@__ncc_kernel_debug_idt = internal global [256 x i128]"));
    assert!(llvm.contains("define void @__ncc_kernel_debug_break_handler() naked nounwind"));

    let object_path = manifest["paths"]["high_level_body_object"]
        .as_str()
        .map(PathBuf::from)
        .expect("retained kernel body object");
    let object = fs::read(object_path).expect("retained kernel body object bytes");
    for (name, encoding) in [
        ("sidt [rax]", &[0x0f, 0x01, 0x08][..]),
        ("lidt [rax]", &[0x0f, 0x01, 0x18][..]),
        ("int3", &[0xcc][..]),
        ("iretq", &[0x48, 0xcf][..]),
    ] {
        assert!(
            object
                .windows(encoding.len())
                .any(|window| window == encoding),
            "kernel debug object is missing real {name} bytes"
        );
    }

    fs::remove_dir_all(temp).expect("temporary cleanup");
}
