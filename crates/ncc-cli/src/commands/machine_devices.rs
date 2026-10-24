use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEVICE_CERTIFICATE_SCHEMA: &str =
    "ncc.machine.virtual_devices_lifecycle_debug_hardware_certificate.v1";
const DEVICE_PAYLOAD_DOMAIN: &[u8] =
    b"ncc.machine.virtual_devices_lifecycle_debug_hardware_certificate.v1\0";
const VMX_CERTIFICATE_SCHEMA: &str = "ncc.machine.x86_virtualization_hardware_certificate.v1";
const PLATFORM_CERTIFICATE_SCHEMA: &str =
    "ncc.machine.x86_apic_smp_guest_boot_hardware_certificate.v1";
pub(crate) const DEVICE_CERTIFICATION_TIMEOUT_MS: u64 = 45_000;

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn first_non_flag(args: &[String]) -> Option<&str> {
    let mut skip_value = false;
    for arg in args {
        if skip_value {
            skip_value = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--out"
                | "--bundle"
                | "--qemu"
                | "--qemu-img"
                | "--vmx-certificate"
                | "--platform-certificate"
        ) {
            skip_value = true;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg);
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let mut result = Map::new();
            for key in keys {
                result.insert(key.clone(), canonical_json(&object[key]));
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn payload_sha256(domain: &[u8], value: &Value) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&canonical_json(value))?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn default_bundle_path(out: Option<&str>) -> PathBuf {
    if let Some(out) = out {
        let path = Path::new(out);
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("device-certificate");
        return parent.join(format!("{stem}-bundle"));
    }
    PathBuf::from("target/ncc-machine-device-certificate")
}

fn collect_artifacts(root: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    fn visit(root: &Path, directory: &Path, output: &mut Vec<Value>) -> Result<(), Box<dyn Error>> {
        let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output)?;
                continue;
            }
            let bytes = fs::read(&path)?;
            output.push(json!({
                "path": path.strip_prefix(root)?.to_string_lossy().replace('\\', "/"),
                "bytes": bytes.len(),
                "sha256": sha256_bytes(&bytes),
            }));
        }
        Ok(())
    }
    let mut artifacts = Vec::new();
    visit(root, root, &mut artifacts)?;
    Ok(artifacts)
}

fn host_probe() -> Value {
    #[cfg(target_arch = "x86_64")]
    let cpuid = {
        use std::arch::x86_64::__cpuid_count;
        let leaf0 = __cpuid_count(0, 0);
        let leaf1 = __cpuid_count(1, 0);
        let mut vendor = Vec::with_capacity(12);
        vendor.extend_from_slice(&leaf0.ebx.to_le_bytes());
        vendor.extend_from_slice(&leaf0.edx.to_le_bytes());
        vendor.extend_from_slice(&leaf0.ecx.to_le_bytes());
        json!({
            "vendor": String::from_utf8_lossy(&vendor).trim_end_matches('\0'),
            "family_model_stepping": format!("{:08x}", leaf1.eax),
            "vmx": leaf1.ecx & (1 << 5) != 0,
            "hypervisor_present": leaf1.ecx & (1 << 31) != 0,
        })
    };
    #[cfg(not(target_arch = "x86_64"))]
    let cpuid = json!({ "vendor": "not-x86_64", "vmx": false });
    let read_trimmed = |path: &str| {
        fs::read_to_string(path)
            .ok()
            .map(|text| text.trim().to_string())
    };
    json!({
        "cpuid": cpuid,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "kernel_release": read_trimmed("/proc/sys/kernel/osrelease"),
        "kernel_version": read_trimmed("/proc/sys/kernel/version"),
        "nested_intel": read_trimmed("/sys/module/kvm_intel/parameters/nested"),
        "nested_amd": read_trimmed("/sys/module/kvm_amd/parameters/nested"),
        "dmi_product": read_trimmed("/sys/class/dmi/id/product_name"),
        "dmi_vendor": read_trimmed("/sys/class/dmi/id/sys_vendor"),
        "logical_processors": std::thread::available_parallelism().map(usize::from).ok(),
        "kvm_exists": Path::new("/dev/kvm").exists(),
    })
}

fn verify_prior_certificate(path: &Path, expected_schema: &str) -> Result<Value, Box<dyn Error>> {
    let certificate_bytes = fs::read(path)?;
    let mut certificate: Value = serde_json::from_slice(&certificate_bytes)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(expected_schema) {
        return Err(format!(
            "[E_MACHINE_DEVICE_CHAIN_001] {} has the wrong schema",
            path.display()
        )
        .into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("[E_MACHINE_DEVICE_CHAIN_001] prior certificate has no payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .ok_or("[E_MACHINE_DEVICE_CHAIN_001] prior certificate root is not an object")?
        .remove("attestation");
    let mut domain = expected_schema.as_bytes().to_vec();
    domain.push(0);
    let actual = payload_sha256(&domain, &certificate)?;
    if expected != actual {
        return Err(format!(
            "[E_MACHINE_DEVICE_CHAIN_HASH_001] prior certificate payload mismatch: {}",
            path.display()
        )
        .into());
    }
    let mut verified_artifacts = 0usize;
    if let (Some(root), Some(artifacts)) = (
        certificate.get("bundle_root").and_then(Value::as_str),
        certificate.get("artifacts").and_then(Value::as_array),
    ) {
        let root = Path::new(root);
        for artifact in artifacts {
            let relative = artifact
                .get("path")
                .and_then(Value::as_str)
                .ok_or("[E_MACHINE_DEVICE_CHAIN_001] prior artifact has no path")?;
            let expected_hash = artifact
                .get("sha256")
                .and_then(Value::as_str)
                .ok_or("[E_MACHINE_DEVICE_CHAIN_001] prior artifact has no hash")?;
            let bytes = fs::read(root.join(relative))?;
            if sha256_bytes(&bytes) != expected_hash {
                return Err(format!(
                    "[E_MACHINE_DEVICE_CHAIN_ARTIFACT_001] prior artifact mismatch: {relative}"
                )
                .into());
            }
            verified_artifacts += 1;
        }
    }
    Ok(json!({
        "path": fs::canonicalize(path)?.to_string_lossy(),
        "schema": expected_schema,
        "certificate_sha256": sha256_bytes(&certificate_bytes),
        "payload_sha256": actual,
        "verified_artifacts": verified_artifacts,
        "verified": true,
    }))
}

fn source_manifest() -> Result<Vec<Value>, Box<dyn Error>> {
    let paths = [
        "crates/ncc-machine/src/virtual_devices.rs",
        "crates/ncc-machine/src/pci_virtio.rs",
        "crates/ncc-machine/src/vm_lifecycle.rs",
        "crates/ncc-machine/src/x86_virtualization.rs",
        "crates/ncc-machine/src/x86_guest_memory.rs",
        "crates/ncc-machine/src/x86_vmexit.rs",
        "crates/ncc-machine/src/x86_apic.rs",
        "crates/ncc-machine/src/x86_guest_boot.rs",
        "crates/ncc-ir/src/kernel_core_llvm.rs",
        "crates/ncc-sema/src/lib.rs",
        "crates/ncc-cli/src/commands/machine_devices.rs",
    ];
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path)?;
            Ok(json!({
                "path": path,
                "bytes": bytes.len(),
                "sha256": sha256_bytes(&bytes),
            }))
        })
        .collect()
}

pub(crate) fn run_certify_devices(args: &[String]) -> Result<(), Box<dyn Error>> {
    let out = option_value(args, "--out");
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bundle_path(out));
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_DEVICE_BUNDLE_001] bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    let vmx_path = option_value(args, "--vmx-certificate")
        .map(PathBuf::from)
        .ok_or("certify-devices requires --vmx-certificate <certificate.json>")?;
    let platform_path = option_value(args, "--platform-certificate")
        .map(PathBuf::from)
        .ok_or("certify-devices requires --platform-certificate <certificate.json>")?;
    let vmx = verify_prior_certificate(&vmx_path, VMX_CERTIFICATE_SCHEMA)?;
    let platform = verify_prior_certificate(&platform_path, PLATFORM_CERTIFICATE_SCHEMA)?;
    fs::create_dir_all(&bundle)?;
    let execution = device_host::execute(&bundle, args)?;
    write_json(&bundle.join("execution.json"), &execution)?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": DEVICE_CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "cpu_backed_virtual_devices_lifecycle_persistence_debug_passed",
        "created_epoch_seconds": epoch_seconds(),
        "host": host_probe(),
        "bundle_root": fs::canonicalize(&bundle)?.to_string_lossy(),
        "source_manifest": source_manifest()?,
        "prior_evidence": {
            "vmcs_vmcb_ept_npt_vmexit": vmx,
            "apic_smp_bios_uefi": platform,
        },
        "execution": execution,
        "artifacts": artifacts,
        "truth_boundary": {
            "direct_kvm_cpu_backed": true,
            "guest_pio_uart": true,
            "guest_pio_ps2_keyboard": true,
            "guest_pio_pci_configuration": true,
            "guest_virtio_block": true,
            "guest_virtio_network": true,
            "guest_virtio_console": true,
            "guest_virtio_input": true,
            "guest_mmio_framebuffer": true,
            "kvm_hardware_breakpoint_exit": true,
            "pause_resume_and_vcpu_reset": true,
            "full_and_incremental_snapshot_roundtrip": true,
            "atomic_snapshot_persistence": true,
            "qemu_kvm_acceleration": true,
            "qmp_pause_resume_reset": true,
            "qmp_snapshot_save_load": true,
            "qemu_pci_virtio_topology": true,
            "prior_certificates_verified": true,
            "emulated_cpu_only": false,
            "bare_metal_board": false,
            "physical_board_certificate": false,
        },
    });
    let mut certificate = payload.clone();
    certificate["attestation"] = json!({
        "algorithm": "sha256",
        "domain": String::from_utf8_lossy(DEVICE_PAYLOAD_DOMAIN).trim_end_matches('\0'),
        "payload_sha256": payload_sha256(DEVICE_PAYLOAD_DOMAIN, &payload)?,
    });
    if let Some(path) = out {
        write_json(Path::new(path), &certificate)?;
    }
    if has_flag(args, "--json") || out.is_none() {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&certificate))?
        );
    } else {
        println!(
            "OK: virtual-device/lifecycle/debug certificate written to {}",
            out.unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_device_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args)
        .map(PathBuf::from)
        .ok_or("verify-devices-certificate requires a certificate path")?;
    let certificate_bytes = fs::read(&path)?;
    let mut certificate: Value = serde_json::from_slice(&certificate_bytes)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(DEVICE_CERTIFICATE_SCHEMA) {
        return Err("[E_MACHINE_DEVICE_CERTIFICATE_001] unsupported certificate schema".into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_001] missing payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_001] certificate root is not an object")?
        .remove("attestation");
    let actual = payload_sha256(DEVICE_PAYLOAD_DOMAIN, &certificate)?;
    if expected != actual {
        return Err(format!(
            "[E_MACHINE_DEVICE_CERTIFICATE_HASH_001] payload mismatch: expected {expected}, got {actual}"
        )
        .into());
    }
    for pointer in [
        "/truth_boundary/direct_kvm_cpu_backed",
        "/truth_boundary/guest_pio_uart",
        "/truth_boundary/guest_pio_ps2_keyboard",
        "/truth_boundary/guest_pio_pci_configuration",
        "/truth_boundary/guest_virtio_block",
        "/truth_boundary/guest_virtio_network",
        "/truth_boundary/guest_virtio_console",
        "/truth_boundary/guest_virtio_input",
        "/truth_boundary/guest_mmio_framebuffer",
        "/truth_boundary/kvm_hardware_breakpoint_exit",
        "/truth_boundary/pause_resume_and_vcpu_reset",
        "/truth_boundary/full_and_incremental_snapshot_roundtrip",
        "/truth_boundary/atomic_snapshot_persistence",
        "/truth_boundary/qemu_kvm_acceleration",
        "/truth_boundary/qmp_pause_resume_reset",
        "/truth_boundary/qmp_snapshot_save_load",
        "/truth_boundary/qemu_pci_virtio_topology",
        "/truth_boundary/prior_certificates_verified",
        "/prior_evidence/vmcs_vmcb_ept_npt_vmexit/verified",
        "/prior_evidence/apic_smp_bios_uefi/verified",
        "/execution/kvm/passed",
        "/execution/qemu_qmp/passed",
    ] {
        if certificate.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] required evidence '{pointer}' is not true"
            )
            .into());
        }
    }
    let root = certificate
        .get("bundle_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] missing bundle root")?;
    let artifacts = certificate
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] missing artifacts")?;
    if artifacts.is_empty() {
        return Err("[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] no artifacts".into());
    }
    for artifact in artifacts {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] artifact path missing")?;
        let expected_hash = artifact
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or("[E_MACHINE_DEVICE_CERTIFICATE_EVIDENCE_001] artifact hash missing")?;
        let bytes = fs::read(root.join(relative))?;
        if sha256_bytes(&bytes) != expected_hash {
            return Err(format!(
                "[E_MACHINE_DEVICE_CERTIFICATE_ARTIFACT_001] hash mismatch: {relative}"
            )
            .into());
        }
    }
    let report = json!({
        "schema": "ncc.machine.virtual_devices_lifecycle_debug_hardware_certificate_verification.v1",
        "status": "verified",
        "certificate": path,
        "certificate_sha256": sha256_bytes(&certificate_bytes),
        "payload_sha256": actual,
        "verified_artifacts": artifacts.len(),
        "cpu_backed": true,
        "bare_metal_board": false,
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("OK: virtual-device/lifecycle/debug certificate verified");
    }
    Ok(())
}

pub(crate) fn run_probe_devices(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ncc.machine.virtual_devices_lifecycle_debug_host_probe.v1",
        "host": host_probe(),
        "certification_available": device_host::available(),
        "qemu": device_host::probe_qemu(option_value(args, "--qemu")),
        "qemu_img": device_host::probe_qemu_img(option_value(args, "--qemu-img")),
        "required_command": "ncc machine virtualization certify-devices --vmx-certificate <vmx.json> --platform-certificate <platform.json> --out <certificate.json> --bundle <bundle>",
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("machine virtual-device host probe: {payload}");
    }
    Ok(())
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod device_host {
    use super::*;

    pub(super) fn available() -> bool {
        false
    }

    pub(super) fn probe_qemu(_override_path: Option<&str>) -> Value {
        json!({ "available": false, "reason": "requires x86_64 Linux KVM host" })
    }

    pub(super) fn probe_qemu_img(_override_path: Option<&str>) -> Value {
        json!({ "available": false, "reason": "requires x86_64 Linux KVM host" })
    }

    pub(super) fn execute(_bundle: &Path, _args: &[String]) -> Result<Value, Box<dyn Error>> {
        Err("[E_MACHINE_DEVICE_HOST_001] certification requires the x86_64 Linux ncc binary, /dev/kvm, qemu-system-x86_64 and qemu-img; run it on Linux or WSL with KVM access".into())
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod device_host {
    use super::*;
    use kvm_bindings::{
        kvm_guest_debug, kvm_guest_debug_arch, kvm_userspace_memory_region, KVM_GUESTDBG_ENABLE,
        KVM_GUESTDBG_USE_HW_BP, KVM_MEM_LOG_DIRTY_PAGES,
    };
    use kvm_ioctls::{Cap, Kvm, VcpuExit, VcpuFd};
    use ncc_machine::{
        load_snapshot, persist_snapshot_atomic, BreakpointArchitecture, DebugStopReason,
        DeviceSnapshot, GuestArchitecture, MemorySnapshot, PciAddress, PciBar, PciBarKind, PciBus,
        PixelFormat, Ps2Controller, Ps2Key, Ps2KeyEvent, RegisterBank, SnapshotId, SnapshotLimits,
        Uart16550, VcpuSnapshot, VirtQueue, VirtioBlockDevice, VirtioConsoleDevice,
        VirtioDescriptor, VirtioGpuDevice, VirtioInputDevice, VirtioNetworkDevice,
        VirtioPciTransport, VirtualBlockDevice, VirtualDeviceClass, VirtualFramebuffer,
        VirtualNetworkDevice, VmDebugger, VmLifecycle, VmPowerState, VmResetKind, VmSnapshot,
        VIRTIO_STATUS_ACKNOWLEDGE, VIRTIO_STATUS_DRIVER, VIRTIO_STATUS_DRIVER_OK,
        VIRTIO_STATUS_FEATURES_OK, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE,
    };
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::process::{Child, Command, Stdio};
    use std::ptr::null_mut;
    use std::slice;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    const GUEST_MEMORY_BYTES: usize = 2 * 1024 * 1024;
    const KVM_RAM_BYTES: usize = 512 * 1024;
    const KVM_MMIO_HOLE_BYTES: usize = 4096;
    const KVM_HIGH_RAM_START: usize = 0x3000;
    const CODE_ADDRESS: usize = 0x1000;
    const RESET_CODE_ADDRESS: usize = 0x1800;
    const STACK_ADDRESS: usize = 0x1f000;
    const PS2_STATUS_RESULT: usize = 0x200;
    const PS2_DATA_RESULT: usize = 0x201;
    const PCI_RESULT: usize = 0x204;
    const SNAPSHOT_MUTATION: usize = 0x300;
    const COM1_PORT: u16 = 0x3f8;
    const PS2_DATA_PORT: u16 = 0x60;
    const PS2_STATUS_PORT: u16 = 0x64;
    const PCI_CONFIG_ADDRESS_PORT: u16 = 0xcf8;
    const PCI_CONFIG_DATA_PORT: u16 = 0xcfc;
    const DEBUG_PORT: u16 = 0xe9;
    const DONE_PORT: u16 = 0xf1;
    const BLOCK_NOTIFY_PORT: u16 = 0xc100;
    const NETWORK_NOTIFY_PORT: u16 = 0xc120;
    const CONSOLE_NOTIFY_PORT: u16 = 0xc140;
    const INPUT_NOTIFY_PORT: u16 = 0xc160;
    const GPU_NOTIFY_PORT: u16 = 0xc180;
    const FRAMEBUFFER_MMIO: u64 = 0x0000_2000;
    const RUN_TIMEOUT_SIGNAL: libc::c_int = libc::SIGUSR1;
    const RUN_TIMEOUT: Duration = Duration::from_millis(DEVICE_CERTIFICATION_TIMEOUT_MS);

    const BLOCK_DESC: u64 = 0x4000;
    const BLOCK_AVAIL: u64 = 0x5000;
    const BLOCK_USED: u64 = 0x6000;
    const BLOCK_HEADER: usize = 0x7000;
    const BLOCK_DATA: usize = 0x7200;
    const BLOCK_STATUS: usize = 0x7400;
    const NETWORK_DESC: u64 = 0x8000;
    const NETWORK_AVAIL: u64 = 0x9000;
    const NETWORK_USED: u64 = 0xa000;
    const NETWORK_PACKET: usize = 0xb000;
    const CONSOLE_DESC: u64 = 0xc000;
    const CONSOLE_AVAIL: u64 = 0xd000;
    const CONSOLE_USED: u64 = 0xe000;
    const CONSOLE_DATA: usize = 0xf000;
    const INPUT_DESC: u64 = 0x10000;
    const INPUT_AVAIL: u64 = 0x11000;
    const INPUT_USED: u64 = 0x12000;
    const INPUT_DATA: usize = 0x13000;
    const GPU_DESC: u64 = 0x14000;
    const GPU_AVAIL: u64 = 0x15000;
    const GPU_USED: u64 = 0x16000;
    const GPU_PIXELS: usize = 0x17000;

    extern "C" fn timeout_signal_handler(_signal: libc::c_int) {}

    struct SignalGuard {
        previous: libc::sigaction,
        previous_mask: libc::sigset_t,
    }

    impl SignalGuard {
        fn install() -> Result<Self, Box<dyn Error>> {
            let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
            let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
            let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };
            let mut previous_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
            action.sa_sigaction = timeout_signal_handler as *const () as usize;
            unsafe {
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(RUN_TIMEOUT_SIGNAL, &action, &mut previous) != 0 {
                    return Err(format!(
                        "[E_MACHINE_DEVICE_SIGNAL_001] cannot install timeout signal: {}",
                        std::io::Error::last_os_error()
                    )
                    .into());
                }
                libc::sigemptyset(&mut signal_set);
                libc::sigaddset(&mut signal_set, RUN_TIMEOUT_SIGNAL);
                if libc::pthread_sigmask(libc::SIG_UNBLOCK, &signal_set, &mut previous_mask) != 0 {
                    libc::sigaction(RUN_TIMEOUT_SIGNAL, &previous, null_mut());
                    return Err(
                        "[E_MACHINE_DEVICE_SIGNAL_001] cannot unblock timeout signal".into(),
                    );
                }
            }
            Ok(Self {
                previous,
                previous_mask,
            })
        }
    }

    impl Drop for SignalGuard {
        fn drop(&mut self) {
            unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.previous_mask, null_mut());
                libc::sigaction(RUN_TIMEOUT_SIGNAL, &self.previous, null_mut());
            }
        }
    }

    struct GuestMemory {
        pointer: *mut u8,
        length: usize,
    }

    impl GuestMemory {
        fn new(length: usize) -> Result<Self, Box<dyn Error>> {
            let pointer = unsafe {
                libc::mmap(
                    null_mut(),
                    length,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                    -1,
                    0,
                )
            };
            if pointer == libc::MAP_FAILED {
                return Err(format!(
                    "[E_MACHINE_DEVICE_MEMORY_001] mmap failed: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
            Ok(Self {
                pointer: pointer.cast(),
                length,
            })
        }

        fn bytes(&self) -> &[u8] {
            unsafe { slice::from_raw_parts(self.pointer, self.length) }
        }

        fn bytes_mut(&mut self) -> &mut [u8] {
            unsafe { slice::from_raw_parts_mut(self.pointer, self.length) }
        }

        fn address(&self) -> u64 {
            self.pointer as u64
        }
    }

    impl Drop for GuestMemory {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.pointer.cast(), self.length);
            }
        }
    }

    #[derive(Clone, PartialEq, Eq)]
    struct DeviceRig {
        uart: Uart16550,
        ps2: Ps2Controller,
        pci: PciBus,
        block: VirtioBlockDevice,
        network: VirtioNetworkDevice,
        console: VirtioConsoleDevice,
        input: VirtioInputDevice,
        gpu: VirtioGpuDevice,
    }

    fn ready_transport(transport: &mut VirtioPciTransport) -> Result<(), Box<dyn Error>> {
        transport.set_status(VIRTIO_STATUS_ACKNOWLEDGE)?;
        transport.set_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER)?;
        transport.driver_features = transport.offered_features;
        transport.set_status(
            VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK,
        )?;
        transport.set_status(
            VIRTIO_STATUS_ACKNOWLEDGE
                | VIRTIO_STATUS_DRIVER
                | VIRTIO_STATUS_FEATURES_OK
                | VIRTIO_STATUS_DRIVER_OK,
        )?;
        Ok(())
    }

    fn write_descriptor(memory: &mut [u8], table: u64, index: u16, descriptor: VirtioDescriptor) {
        let offset = table as usize + index as usize * 16;
        memory[offset..offset + 8].copy_from_slice(&descriptor.address.to_le_bytes());
        memory[offset + 8..offset + 12].copy_from_slice(&descriptor.length.to_le_bytes());
        memory[offset + 12..offset + 14].copy_from_slice(&descriptor.flags.to_le_bytes());
        memory[offset + 14..offset + 16].copy_from_slice(&descriptor.next.to_le_bytes());
    }

    fn configure_queue(
        queue: &mut VirtQueue,
        memory: &mut [u8],
        descriptor: u64,
        available: u64,
        used: u64,
        heads: &[u16],
    ) -> Result<(), Box<dyn Error>> {
        queue.configure(8, descriptor, available, used)?;
        memory[available as usize..available as usize + 6 + heads.len() * 2].fill(0);
        memory[available as usize + 2..available as usize + 4]
            .copy_from_slice(&(heads.len() as u16).to_le_bytes());
        for (index, head) in heads.iter().enumerate() {
            let offset = available as usize + 4 + index * 2;
            memory[offset..offset + 2].copy_from_slice(&head.to_le_bytes());
        }
        memory[used as usize..used as usize + 4 + heads.len() * 8].fill(0);
        Ok(())
    }

    fn add_pci_function(
        bus: &mut PciBus,
        transport: &VirtioPciTransport,
        device: u8,
    ) -> Result<(), Box<dyn Error>> {
        let base = 0xd000_0000u64 + device as u64 * 0x10_000;
        let address = PciAddress::new(0, 0, device, 0)?;
        let function = transport.pci_function(
            address,
            transport.device_type as u16,
            PciBar::new(PciBarKind::Memory32, base, 0x1000, false)?,
            PciBar::new(PciBarKind::Memory32, base + 0x2000, 0x1000, false)?,
            PciBar::new(PciBarKind::Memory32, base + 0x4000, 0x1000, false)?,
        )?;
        bus.attach(function)?;
        Ok(())
    }

    fn build_device_rig(memory: &mut [u8]) -> Result<DeviceRig, Box<dyn Error>> {
        let mut block = VirtioBlockDevice::new(
            VirtualBlockDevice::new(64 * 1024, 512, false)?,
            b"ncc-kvm-block-0001",
        )?;
        ready_transport(&mut block.transport)?;
        configure_queue(
            &mut block.transport.queues[0],
            memory,
            BLOCK_DESC,
            BLOCK_AVAIL,
            BLOCK_USED,
            &[0],
        )?;
        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&1u32.to_le_bytes());
        header[8..16].copy_from_slice(&2u64.to_le_bytes());
        memory[BLOCK_HEADER..BLOCK_HEADER + 16].copy_from_slice(&header);
        memory[BLOCK_DATA..BLOCK_DATA + 512].fill(0x5a);
        memory[BLOCK_STATUS] = 0xff;
        write_descriptor(
            memory,
            BLOCK_DESC,
            0,
            VirtioDescriptor {
                address: BLOCK_HEADER as u64,
                length: 16,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            },
        );
        write_descriptor(
            memory,
            BLOCK_DESC,
            1,
            VirtioDescriptor {
                address: BLOCK_DATA as u64,
                length: 512,
                flags: VIRTQ_DESC_F_NEXT,
                next: 2,
            },
        );
        write_descriptor(
            memory,
            BLOCK_DESC,
            2,
            VirtioDescriptor {
                address: BLOCK_STATUS as u64,
                length: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            },
        );

        let network_backend = VirtualNetworkDevice::new([0x02, 0, 0, 0, 0, 1], 1500, 16)?;
        let mut network = VirtioNetworkDevice::new(network_backend)?;
        ready_transport(&mut network.transport)?;
        configure_queue(
            &mut network.transport.queues[1],
            memory,
            NETWORK_DESC,
            NETWORK_AVAIL,
            NETWORK_USED,
            &[0],
        )?;
        let mut packet = vec![0u8; 12 + 64];
        packet[12..18].copy_from_slice(&[0xff; 6]);
        packet[18..24].copy_from_slice(&[0x02, 0, 0, 0, 0, 1]);
        packet[24..].fill(0xa5);
        memory[NETWORK_PACKET..NETWORK_PACKET + packet.len()].copy_from_slice(&packet);
        write_descriptor(
            memory,
            NETWORK_DESC,
            0,
            VirtioDescriptor {
                address: NETWORK_PACKET as u64,
                length: packet.len() as u32,
                flags: 0,
                next: 0,
            },
        );

        let mut console = VirtioConsoleDevice::new(Uart16550::new(0x2f8))?;
        ready_transport(&mut console.transport)?;
        configure_queue(
            &mut console.transport.queues[1],
            memory,
            CONSOLE_DESC,
            CONSOLE_AVAIL,
            CONSOLE_USED,
            &[0],
        )?;
        memory[CONSOLE_DATA..CONSOLE_DATA + 5].copy_from_slice(b"nC VM");
        write_descriptor(
            memory,
            CONSOLE_DESC,
            0,
            VirtioDescriptor {
                address: CONSOLE_DATA as u64,
                length: 5,
                flags: 0,
                next: 0,
            },
        );

        let mut input = VirtioInputDevice::new()?;
        ready_transport(&mut input.transport)?;
        configure_queue(
            &mut input.transport.queues[0],
            memory,
            INPUT_DESC,
            INPUT_AVAIL,
            INPUT_USED,
            &[0, 1],
        )?;
        for index in 0..2u16 {
            write_descriptor(
                memory,
                INPUT_DESC,
                index,
                VirtioDescriptor {
                    address: (INPUT_DATA + index as usize * 8) as u64,
                    length: 8,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0,
                },
            );
        }
        input.inject_key(
            Ps2KeyEvent {
                key: Ps2Key::Letter(b'a'),
                pressed: true,
            },
            30,
        );

        let framebuffer = VirtualFramebuffer::new(2, 2, PixelFormat::Rgba8888)?;
        let mut gpu = VirtioGpuDevice::new(framebuffer)?;
        ready_transport(&mut gpu.transport)?;
        configure_queue(
            &mut gpu.transport.queues[0],
            memory,
            GPU_DESC,
            GPU_AVAIL,
            GPU_USED,
            &[],
        )?;
        gpu.create_resource_2d(1, 2, 2, PixelFormat::Rgba8888)?;
        memory[GPU_PIXELS..GPU_PIXELS + 16].copy_from_slice(&[
            0xff, 0, 0, 0xff, 0, 0xff, 0, 0xff, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ]);
        gpu.attach_backing(1, &[(GPU_PIXELS as u64, 16)])?;
        gpu.set_scanout(1)?;

        let mut pci = PciBus::default();
        add_pci_function(&mut pci, &block.transport, 1)?;
        add_pci_function(&mut pci, &network.transport, 2)?;
        add_pci_function(&mut pci, &console.transport, 3)?;
        add_pci_function(&mut pci, &input.transport, 4)?;
        add_pci_function(&mut pci, &gpu.transport, 5)?;

        let mut ps2 = Ps2Controller::new();
        ps2.inject_key(Ps2KeyEvent {
            key: Ps2Key::Letter(b'a'),
            pressed: true,
        })?;
        Ok(DeviceRig {
            uart: Uart16550::new(COM1_PORT),
            ps2,
            pci,
            block,
            network,
            console,
            input,
            gpu,
        })
    }

    fn guest_program() -> (Vec<u8>, u64) {
        let mut code = vec![0xfa];
        code.extend_from_slice(&[0xba, 0xf8, 0x03, 0xb0, b'U', 0xee]);
        code.extend_from_slice(&[0xba, 0x64, 0x00, 0xec, 0xa2, 0x00, 0x02]);
        code.extend_from_slice(&[0xba, 0x60, 0x00, 0xec, 0xa2, 0x01, 0x02]);
        code.extend_from_slice(&[0x66, 0xb8, 0x00, 0x08, 0x00, 0x80]);
        code.extend_from_slice(&[0xba, 0xf8, 0x0c, 0x66, 0xef]);
        code.extend_from_slice(&[0xba, 0xfc, 0x0c, 0x66, 0xed, 0x66, 0xa3, 0x04, 0x02]);
        for (port, queue) in [
            (BLOCK_NOTIFY_PORT, 0u16),
            (NETWORK_NOTIFY_PORT, 1),
            (CONSOLE_NOTIFY_PORT, 1),
            (INPUT_NOTIFY_PORT, 0),
            (GPU_NOTIFY_PORT, 0),
        ] {
            code.extend_from_slice(&[0xba, port as u8, (port >> 8) as u8]);
            code.extend_from_slice(&[0xb8, queue as u8, (queue >> 8) as u8, 0xef]);
        }
        let breakpoint_address = (CODE_ADDRESS + code.len()) as u64;
        code.push(0x90);
        code.extend_from_slice(&[0x66, 0xc7, 0x06]);
        code.extend_from_slice(&(FRAMEBUFFER_MMIO as u16).to_le_bytes());
        code.extend_from_slice(&0x1122_3344u32.to_le_bytes());
        code.extend_from_slice(&[0xba, 0xe9, 0x00, 0xb0, b'D', 0xee]);
        code.extend_from_slice(&[0xba, 0xf1, 0x00, 0x30, 0xc0, 0xee, 0xf4]);
        (code, breakpoint_address)
    }

    fn reset_program() -> Vec<u8> {
        vec![
            0xfa, 0xba, 0xe9, 0x00, 0xb0, b'R', 0xee, 0xba, 0xf1, 0x00, 0x30, 0xc0, 0xee, 0xf4,
        ]
    }

    fn configure_real_mode(vcpu: &VcpuFd, rip: usize) -> Result<(), Box<dyn Error>> {
        let mut special = vcpu.get_sregs()?;
        for segment in [
            &mut special.cs,
            &mut special.ds,
            &mut special.es,
            &mut special.fs,
            &mut special.gs,
            &mut special.ss,
        ] {
            segment.base = 0;
            segment.selector = 0;
        }
        vcpu.set_sregs(&special)?;
        let mut registers = vcpu.get_regs()?;
        registers.rip = rip as u64;
        registers.rsp = STACK_ADDRESS as u64;
        registers.rflags = 2;
        registers.rax = 0;
        registers.rbx = 0;
        registers.rcx = 0;
        registers.rdx = 0;
        vcpu.set_regs(&registers)?;
        Ok(())
    }

    fn run_with_timeout<'a>(vcpu: &'a mut VcpuFd) -> Result<VcpuExit<'a>, Box<dyn Error>> {
        let target_thread = unsafe { libc::pthread_self() } as usize;
        let cancelled = Arc::new((Mutex::new(false), Condvar::new()));
        let timed_out = Arc::new(AtomicBool::new(false));
        let watchdog_cancelled = Arc::clone(&cancelled);
        let watchdog_timed_out = Arc::clone(&timed_out);
        let watchdog = thread::spawn(move || {
            let (lock, condition) = &*watchdog_cancelled;
            let guard = lock.lock().expect("device watchdog mutex poisoned");
            let (guard, timeout) = condition
                .wait_timeout_while(guard, RUN_TIMEOUT, |cancelled| !*cancelled)
                .expect("device watchdog condition poisoned");
            if timeout.timed_out() && !*guard {
                watchdog_timed_out.store(true, Ordering::Release);
                unsafe {
                    libc::pthread_kill(target_thread as libc::pthread_t, RUN_TIMEOUT_SIGNAL);
                }
            }
        });
        let result = vcpu.run();
        {
            let (lock, condition) = &*cancelled;
            *lock
                .lock()
                .map_err(|_| "[E_MACHINE_DEVICE_KVM_001] watchdog mutex poisoned")? = true;
            condition.notify_one();
        }
        watchdog
            .join()
            .map_err(|_| "[E_MACHINE_DEVICE_KVM_001] watchdog thread panicked")?;
        if timed_out.load(Ordering::Acquire) {
            return Err(format!(
                "[E_MACHINE_DEVICE_TIMEOUT_001] KVM_RUN exceeded {} ms",
                RUN_TIMEOUT.as_millis()
            )
            .into());
        }
        Ok(result?)
    }

    fn register_snapshot(vcpu: &VcpuFd) -> Result<VcpuSnapshot, Box<dyn Error>> {
        let registers = vcpu.get_regs()?;
        let mut bank = RegisterBank::new(GuestArchitecture::X86_64);
        for (name, value) in [
            ("rax", registers.rax),
            ("rbx", registers.rbx),
            ("rcx", registers.rcx),
            ("rdx", registers.rdx),
            ("rsi", registers.rsi),
            ("rdi", registers.rdi),
            ("rbp", registers.rbp),
            ("rsp", registers.rsp),
            ("r8", registers.r8),
            ("r9", registers.r9),
            ("r10", registers.r10),
            ("r11", registers.r11),
            ("r12", registers.r12),
            ("r13", registers.r13),
            ("r14", registers.r14),
            ("r15", registers.r15),
            ("rip", registers.rip),
            ("rflags", registers.rflags),
        ] {
            bank.set(name, value as u128);
        }
        Ok(VcpuSnapshot {
            id: 0,
            registers: bank,
            halted: false,
            pending_interrupts: Vec::new(),
            virtual_time_ns: 0,
        })
    }

    fn device_snapshots(rig: &DeviceRig) -> Result<Vec<DeviceSnapshot>, Box<dyn Error>> {
        let snapshots = vec![
            DeviceSnapshot::new(
                "uart0",
                VirtualDeviceClass::Uart,
                1,
                true,
                serde_json::to_vec(&json!({
                    "base_port": rig.uart.base_port,
                    "interrupt_enable": rig.uart.interrupt_enable,
                    "line_control": rig.uart.line_control,
                    "divisor": rig.uart.divisor,
                    "pending_rx": rig.uart.pending_rx(),
                    "pending_tx": rig.uart.pending_tx(),
                }))?,
            ),
            DeviceSnapshot::new(
                "ps2",
                VirtualDeviceClass::Ps2,
                1,
                true,
                serde_json::to_vec(&json!({
                    "command_byte": rig.ps2.command_byte,
                    "output_port": rig.ps2.output_port,
                    "keyboard_enabled": rig.ps2.keyboard_enabled,
                    "scanning_enabled": rig.ps2.scanning_enabled,
                    "scancode_set": rig.ps2.scancode_set,
                    "pending_bytes": rig.ps2.pending_bytes(),
                }))?,
            ),
            DeviceSnapshot::new(
                "virtio-blk0",
                VirtualDeviceClass::VirtioBlock,
                1,
                true,
                serde_json::to_vec(&json!({
                    "capacity": rig.block.backend.capacity(),
                    "sector_size": rig.block.backend.sector_size,
                    "generation": rig.block.backend.generation,
                    "flush_count": rig.block.backend.flush_count,
                    "disk_sha256": sha256_bytes(rig.block.backend.bytes()),
                    "transport_status": rig.block.transport.device_status,
                    "queue_used": rig.block.transport.queues[0].used_index,
                }))?,
            ),
            DeviceSnapshot::new(
                "virtio-net0",
                VirtualDeviceClass::VirtioNetwork,
                1,
                true,
                serde_json::to_vec(&json!({
                    "mac": rig.network.backend.mac,
                    "mtu": rig.network.backend.mtu,
                    "link_up": rig.network.backend.link_up,
                    "counters": {
                        "tx_packets": rig.network.backend.counters.tx_packets,
                        "tx_bytes": rig.network.backend.counters.tx_bytes,
                        "rx_packets": rig.network.backend.counters.rx_packets,
                        "rx_bytes": rig.network.backend.counters.rx_bytes,
                    },
                    "queue_used": rig.network.transport.queues[1].used_index,
                }))?,
            ),
            DeviceSnapshot::new(
                "virtio-console0",
                VirtualDeviceClass::VirtioConsole,
                1,
                true,
                serde_json::to_vec(&json!({
                    "pending_tx": rig.console.backend.pending_tx(),
                    "pending_rx": rig.console.backend.pending_rx(),
                    "queue_used": rig.console.transport.queues[1].used_index,
                }))?,
            ),
            DeviceSnapshot::new(
                "virtio-input0",
                VirtualDeviceClass::VirtioInput,
                1,
                true,
                serde_json::to_vec(&json!({
                    "queue_used": rig.input.transport.queues[0].used_index,
                    "interrupt_status": rig.input.transport.isr_status,
                }))?,
            ),
            DeviceSnapshot::new(
                "virtio-gpu0",
                VirtualDeviceClass::VirtioGpu,
                1,
                true,
                serde_json::to_vec(&json!({
                    "width": rig.gpu.framebuffer.width,
                    "height": rig.gpu.framebuffer.height,
                    "scanout": rig.gpu.scanout,
                    "flush_count": rig.gpu.flush_count,
                    "framebuffer_crc32": rig.gpu.framebuffer.checksum(),
                    "framebuffer_sha256": sha256_bytes(rig.gpu.framebuffer.pixels()),
                }))?,
            ),
            DeviceSnapshot::new(
                "pci0",
                VirtualDeviceClass::PciBus,
                1,
                true,
                serde_json::to_vec(&json!({
                    "config_address": rig.pci.config_address,
                    "functions": rig.pci.functions.keys().map(|address| json!({
                        "segment": address.segment,
                        "bus": address.bus,
                        "device": address.device,
                        "function": address.function,
                    })).collect::<Vec<_>>(),
                }))?,
            ),
        ];
        Ok(snapshots)
    }

    fn make_snapshot(
        id: SnapshotId,
        parent: SnapshotId,
        generation: u64,
        lifecycle: &VmLifecycle,
        vcpu: &VcpuFd,
        memory: MemorySnapshot,
        rig: &DeviceRig,
    ) -> Result<VmSnapshot, Box<dyn Error>> {
        Ok(VmSnapshot {
            id,
            parent,
            generation,
            timestamp_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0),
            state: lifecycle.state(),
            architecture: GuestArchitecture::X86_64,
            target: "x86_64-unknown-none".into(),
            vcpus: vec![register_snapshot(vcpu)?],
            memory,
            devices: device_snapshots(rig)?,
            metadata: BTreeMap::from([
                ("backend".into(), "linux-kvm".into()),
                ("snapshot_state".into(), format!("{:?}", lifecycle.state())),
            ]),
        })
    }

    fn service_io_out(
        port: u16,
        data: &[u8],
        memory: &mut [u8],
        rig: &mut DeviceRig,
        markers: &mut Vec<u8>,
    ) -> Result<bool, Box<dyn Error>> {
        match port {
            COM1_PORT => {
                for byte in data {
                    rig.uart.write(0, *byte)?;
                }
            }
            PCI_CONFIG_ADDRESS_PORT => {
                if data.len() != 4 {
                    return Err("[E_MACHINE_DEVICE_PCI_001] CF8 write must be 32-bit".into());
                }
                rig.pci.mechanism_one_write(
                    port,
                    4,
                    u32::from_le_bytes(data.try_into().unwrap()),
                )?;
            }
            PCI_CONFIG_DATA_PORT..=0xcff => {
                let mut bytes = [0u8; 4];
                bytes[..data.len()].copy_from_slice(data);
                rig.pci
                    .mechanism_one_write(port, data.len() as u8, u32::from_le_bytes(bytes))?;
            }
            BLOCK_NOTIFY_PORT => {
                let queue =
                    u16::from_le_bytes(data.try_into().map_err(|_| {
                        "[E_MACHINE_DEVICE_VIRTIO_001] block notify must be 16-bit"
                    })?);
                rig.block.transport.notify(queue)?;
                let completed = rig.block.process_queue(memory)?;
                if completed != 1 {
                    let state = &rig.block.transport.queues[0];
                    let available_index = u16::from_le_bytes(
                        memory[BLOCK_AVAIL as usize + 2..BLOCK_AVAIL as usize + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let material_used_index = u16::from_le_bytes(
                        memory[BLOCK_USED as usize + 2..BLOCK_USED as usize + 4]
                            .try_into()
                            .unwrap(),
                    );
                    return Err(format!(
                        "[E_MACHINE_DEVICE_VIRTIO_001] block queue completed {completed} chains: notify={queue} ready={} size={} avail.idx={available_index} last_avail={} used.idx={material_used_index} model_used={}",
                        state.ready, state.size, state.last_available_index, state.used_index
                    )
                    .into());
                }
            }
            NETWORK_NOTIFY_PORT => {
                let queue =
                    u16::from_le_bytes(data.try_into().map_err(|_| {
                        "[E_MACHINE_DEVICE_VIRTIO_001] network notify must be 16-bit"
                    })?);
                rig.network.transport.notify(queue)?;
                if rig.network.process_tx(memory)? != 1 {
                    return Err("[E_MACHINE_DEVICE_VIRTIO_001] network TX did not complete".into());
                }
            }
            CONSOLE_NOTIFY_PORT => {
                let queue =
                    u16::from_le_bytes(data.try_into().map_err(|_| {
                        "[E_MACHINE_DEVICE_VIRTIO_001] console notify must be 16-bit"
                    })?);
                rig.console.transport.notify(queue)?;
                if rig.console.process_tx(memory)? != 1 {
                    return Err("[E_MACHINE_DEVICE_VIRTIO_001] console TX did not complete".into());
                }
            }
            INPUT_NOTIFY_PORT => {
                let queue =
                    u16::from_le_bytes(data.try_into().map_err(|_| {
                        "[E_MACHINE_DEVICE_VIRTIO_001] input notify must be 16-bit"
                    })?);
                rig.input.transport.notify(queue)?;
                if rig.input.process_events(memory)? != 2 {
                    return Err(
                        "[E_MACHINE_DEVICE_VIRTIO_001] input events did not complete".into(),
                    );
                }
            }
            GPU_NOTIFY_PORT => {
                let queue = u16::from_le_bytes(
                    data.try_into()
                        .map_err(|_| "[E_MACHINE_DEVICE_VIRTIO_001] GPU notify must be 16-bit")?,
                );
                rig.gpu.transport.notify(queue)?;
                rig.gpu.transfer_to_host_2d(memory, 1)?;
                rig.gpu.resource_flush(1)?;
                rig.gpu.transport.interrupt(false);
            }
            DEBUG_PORT => markers.extend_from_slice(data),
            DONE_PORT => return Ok(true),
            _ => {
                return Err(format!(
                    "[E_MACHINE_DEVICE_IO_001] unexpected PIO write {port:#x}: {data:02x?}"
                )
                .into())
            }
        }
        Ok(false)
    }

    fn service_io_in(
        port: u16,
        data: &mut [u8],
        rig: &mut DeviceRig,
    ) -> Result<(), Box<dyn Error>> {
        let value = match port {
            PS2_STATUS_PORT if data.len() == 1 => rig.ps2.status() as u32,
            PS2_DATA_PORT if data.len() == 1 => rig.ps2.read_data() as u32,
            PCI_CONFIG_ADDRESS_PORT | PCI_CONFIG_DATA_PORT..=0xcff => {
                rig.pci.mechanism_one_read(port, data.len() as u8)?
            }
            _ => {
                return Err(format!(
                    "[E_MACHINE_DEVICE_IO_001] unexpected PIO read {port:#x}/{}",
                    data.len()
                )
                .into())
            }
        };
        data.copy_from_slice(&value.to_le_bytes()[..data.len()]);
        Ok(())
    }

    fn execute_kvm(bundle: &Path) -> Result<Value, Box<dyn Error>> {
        let _signal_guard = SignalGuard::install()?;
        let kvm = Kvm::new()
            .map_err(|error| format!("[E_MACHINE_DEVICE_KVM_001] cannot open /dev/kvm: {error}"))?;
        for capability in [Cap::UserMemory, Cap::SetGuestDebug, Cap::VcpuEvents] {
            if !kvm.check_extension(capability) {
                return Err(format!(
                    "[E_MACHINE_DEVICE_KVM_001] missing KVM capability {capability:?}"
                )
                .into());
            }
        }
        let vm = kvm.create_vm()?;
        let mut memory = GuestMemory::new(GUEST_MEMORY_BYTES)?;
        unsafe {
            vm.set_user_memory_region(kvm_userspace_memory_region {
                slot: 0,
                flags: KVM_MEM_LOG_DIRTY_PAGES,
                guest_phys_addr: 0,
                memory_size: FRAMEBUFFER_MMIO,
                userspace_addr: memory.address(),
            })?;
            vm.set_user_memory_region(kvm_userspace_memory_region {
                slot: 1,
                flags: KVM_MEM_LOG_DIRTY_PAGES,
                guest_phys_addr: KVM_HIGH_RAM_START as u64,
                memory_size: (KVM_RAM_BYTES - KVM_HIGH_RAM_START) as u64,
                userspace_addr: memory.address() + KVM_HIGH_RAM_START as u64,
            })?;
        };
        let (program, breakpoint_address) = guest_program();
        let reset = reset_program();
        memory.bytes_mut()[CODE_ADDRESS..CODE_ADDRESS + program.len()].copy_from_slice(&program);
        memory.bytes_mut()[RESET_CODE_ADDRESS..RESET_CODE_ADDRESS + reset.len()]
            .copy_from_slice(&reset);
        let mut rig = build_device_rig(memory.bytes_mut())?;
        let rig_initial = rig.clone();
        let mut vcpu = vm.create_vcpu(0)?;
        configure_real_mode(&vcpu, CODE_ADDRESS)?;
        let mut debug_arch = kvm_guest_debug_arch::default();
        debug_arch.debugreg[0] = breakpoint_address;
        debug_arch.debugreg[7] = 1;
        vcpu.set_guest_debug(&kvm_guest_debug {
            control: KVM_GUESTDBG_ENABLE | KVM_GUESTDBG_USE_HW_BP,
            pad: 0,
            arch: debug_arch,
        })?;

        let mut lifecycle = VmLifecycle::new(128);
        lifecycle.start()?;
        let mut debugger = VmDebugger::new(BreakpointArchitecture::X86, 128);
        let mut markers = Vec::new();
        let mut exits = Vec::new();
        let mut completed = false;
        let mut debug_evidence = None;
        let started = Instant::now();
        for sequence in 1..=128u64 {
            let exit = run_with_timeout(&mut vcpu)?;
            match exit {
                VcpuExit::IoOut(port, data) => {
                    exits.push(json!({ "sequence": sequence, "kind": "pio_write", "port": port, "bytes": data.len(), "data": data }));
                    match service_io_out(port, data, memory.bytes_mut(), &mut rig, &mut markers) {
                        Ok(true) => {
                            completed = true;
                            break;
                        }
                        Ok(false) => {}
                        Err(error) => {
                            let failure_trace = json!({
                                "schema": "ncc.machine.virtual_devices_kvm_failure_trace.v1",
                                "error": error.to_string(),
                                "exits": exits,
                            });
                            write_json(&bundle.join("kvm-failure-trace.json"), &failure_trace)?;
                            return Err(format!(
                                "{error}; partial trace: {}",
                                bundle.join("kvm-failure-trace.json").display()
                            )
                            .into());
                        }
                    }
                }
                VcpuExit::IoIn(port, data) => {
                    service_io_in(port, data, &mut rig)?;
                    exits.push(json!({ "sequence": sequence, "kind": "pio_read", "port": port, "bytes": data.len(), "response": data }));
                }
                VcpuExit::MmioWrite(address, data) => {
                    if address != FRAMEBUFFER_MMIO || data.len() != 4 {
                        return Err(format!(
                            "[E_MACHINE_DEVICE_MMIO_001] unexpected MMIO write {address:#x}: {data:02x?}"
                        )
                        .into());
                    }
                    let rgba = u32::from_le_bytes(data.try_into().unwrap());
                    rig.gpu.framebuffer.write_pixel(0, 0, rgba)?;
                    exits.push(json!({ "sequence": sequence, "kind": "mmio_write", "address": address, "data": data }));
                }
                VcpuExit::MmioRead(address, data) => {
                    data.fill(0);
                    exits.push(json!({ "sequence": sequence, "kind": "mmio_read", "address": address, "bytes": data.len() }));
                }
                VcpuExit::Debug(debug) => {
                    let registers = vcpu.get_regs()?;
                    lifecycle.pause("KVM hardware execution breakpoint")?;
                    debugger.record_stop(
                        0,
                        DebugStopReason::HardwareBreakpoint {
                            id: 0,
                            address: debug.pc,
                        },
                    );
                    vcpu.set_guest_debug(&kvm_guest_debug::default())?;
                    let mut pending_events = vcpu.get_vcpu_events()?;
                    let pending_debug_exception = json!({
                        "injected": pending_events.exception.injected,
                        "pending": pending_events.exception.pending,
                        "vector": pending_events.exception.nr,
                        "has_error_code": pending_events.exception.has_error_code,
                    });
                    pending_events.exception.injected = 0;
                    pending_events.exception.pending = 0;
                    pending_events.exception.nr = 0;
                    pending_events.exception.has_error_code = 0;
                    pending_events.exception.error_code = 0;
                    pending_events.exception_has_payload = 0;
                    pending_events.exception_payload = 0;
                    vcpu.set_vcpu_events(&pending_events)?;
                    let mut resume_registers = vcpu.get_regs()?;
                    resume_registers.rip = debug.pc + 1;
                    resume_registers.rflags |= 1 << 16;
                    vcpu.set_regs(&resume_registers)?;
                    let base = memory.bytes().to_vec();
                    let full_id = SnapshotId::from_sequence(0x4e_43, 1);
                    let full = make_snapshot(
                        full_id,
                        SnapshotId::NIL,
                        1,
                        &lifecycle,
                        &vcpu,
                        MemorySnapshot::capture_full(&base, 4096)?,
                        &rig,
                    )?;
                    let full_path = bundle.join("vm-full.ncsnap");
                    persist_snapshot_atomic(&full_path, &full)?;
                    let rig_checkpoint = rig.clone();
                    memory.bytes_mut()[SNAPSHOT_MUTATION] ^= 0xa5;
                    rig.uart.scratch ^= 0x5a;
                    let changed = memory.bytes().to_vec();
                    let delta = make_snapshot(
                        SnapshotId::from_sequence(0x4e_43, 2),
                        full_id,
                        2,
                        &lifecycle,
                        &vcpu,
                        MemorySnapshot::capture_incremental(&base, &changed, 4096)?,
                        &rig,
                    )?;
                    let delta_path = bundle.join("vm-delta.ncsnap");
                    persist_snapshot_atomic(&delta_path, &delta)?;
                    let loaded_full = load_snapshot(&full_path, SnapshotLimits::default())?;
                    let loaded_delta = load_snapshot(&delta_path, SnapshotLimits::default())?;
                    let restored_base = loaded_full.restore_memory(None)?;
                    let restored = loaded_delta.restore_memory(Some(&restored_base))?;
                    if restored != changed {
                        return Err(
                            "[E_MACHINE_DEVICE_SNAPSHOT_001] incremental restore mismatch".into(),
                        );
                    }
                    memory.bytes_mut().copy_from_slice(&restored);
                    rig = rig_checkpoint.clone();
                    lifecycle.resume()?;
                    exits.push(json!({
                        "sequence": sequence,
                        "kind": "debug",
                        "exception": debug.exception,
                        "pc": debug.pc,
                        "dr6": debug.dr6,
                        "dr7": debug.dr7,
                        "rip": registers.rip,
                        "resume_rip": resume_registers.rip,
                        "resume_flag_set": true,
                        "pending_event_before_resume": pending_debug_exception,
                        "pending_event_cleared": true,
                    }));
                    debug_evidence = Some(json!({
                        "exception": debug.exception,
                        "pc": debug.pc,
                        "rip": registers.rip,
                        "full_snapshot": "vm-full.ncsnap",
                        "incremental_snapshot": "vm-delta.ncsnap",
                        "incremental_pages": loaded_delta.memory.pages.len(),
                        "memory_roundtrip": true,
                        "device_checkpoint_restored": rig == rig_checkpoint,
                    }));
                }
                VcpuExit::Hlt => {
                    return Err("[E_MACHINE_DEVICE_KVM_001] guest halted before completion".into())
                }
                VcpuExit::Shutdown => {
                    return Err("[E_MACHINE_DEVICE_KVM_001] guest shut down".into())
                }
                other => {
                    return Err(format!(
                        "[E_MACHINE_DEVICE_KVM_001] unexpected KVM exit: {other:?}"
                    )
                    .into())
                }
            }
        }
        if !completed {
            return Err("[E_MACHINE_DEVICE_KVM_001] guest did not signal completion".into());
        }
        if started.elapsed() > RUN_TIMEOUT {
            return Err("[E_MACHINE_DEVICE_TIMEOUT_001] device run exceeded deadline".into());
        }

        let uart_bytes = rig.uart.take_tx();
        let console_bytes = rig.console.backend.take_tx();
        let network_frame = rig
            .network
            .backend
            .take_transmitted()
            .ok_or("[E_MACHINE_DEVICE_VERIFY_001] no transmitted frame")?;
        let mut block_sector = [0u8; 512];
        rig.block.backend.read_sectors(2, &mut block_sector)?;
        let pci_value = u32::from_le_bytes(memory.bytes()[PCI_RESULT..PCI_RESULT + 4].try_into()?);
        let input_press = &memory.bytes()[INPUT_DATA..INPUT_DATA + 8];
        let input_sync = &memory.bytes()[INPUT_DATA + 8..INPUT_DATA + 16];
        let framebuffer_crc32 = rig.gpu.framebuffer.checksum();
        if uart_bytes != b"U"
            || memory.bytes()[PS2_STATUS_RESULT] & 1 == 0
            || memory.bytes()[PS2_DATA_RESULT] != 0x1c
            || pci_value != 0x1042_1af4
            || memory.bytes()[BLOCK_STATUS] != 0
            || !block_sector.iter().all(|byte| *byte == 0x5a)
            || network_frame.len() != 64
            || console_bytes != b"nC VM"
            || input_press != [1, 0, 30, 0, 1, 0, 0, 0]
            || input_sync != [0; 8]
            || framebuffer_crc32 == 0
            || debug_evidence.is_none()
        {
            return Err(format!(
                "[E_MACHINE_DEVICE_VERIFY_001] material device mismatch: uart={uart_bytes:02x?} ps2={:02x}/{:02x} pci={pci_value:#x} block_status={} block_ok={} net={} console={console_bytes:02x?} input={input_press:02x?}/{input_sync:02x?} framebuffer={framebuffer_crc32:#x} debug={} markers={:?}",
                memory.bytes()[PS2_STATUS_RESULT],
                memory.bytes()[PS2_DATA_RESULT],
                memory.bytes()[BLOCK_STATUS],
                block_sector.iter().all(|byte| *byte == 0x5a),
                network_frame.len(),
                debug_evidence.is_some(),
                String::from_utf8_lossy(&markers),
            )
            .into());
        }
        fs::write(bundle.join("virtio-block-sector.bin"), block_sector)?;
        fs::write(bundle.join("virtio-network-frame.bin"), &network_frame)?;
        fs::write(bundle.join("virtio-console.bin"), &console_bytes)?;
        fs::write(
            bundle.join("framebuffer.rgba"),
            rig.gpu.framebuffer.pixels(),
        )?;

        lifecycle.begin_reset(VmResetKind::Warm)?;
        rig = rig_initial;
        rig.uart.reset();
        rig.ps2.reset();
        rig.network.backend.reset();
        rig.gpu.framebuffer.reset();
        vcpu.set_guest_debug(&kvm_guest_debug::default())?;
        configure_real_mode(&vcpu, RESET_CODE_ADDRESS)?;
        lifecycle.complete_reset(false)?;
        let mut reset_markers = Vec::new();
        let mut reset_completed = false;
        for _ in 0..16 {
            match run_with_timeout(&mut vcpu)? {
                VcpuExit::IoOut(port, data) if port == DEBUG_PORT => {
                    reset_markers.extend_from_slice(data)
                }
                VcpuExit::IoOut(port, _) if port == DONE_PORT => {
                    reset_completed = true;
                    break;
                }
                VcpuExit::Hlt => {
                    return Err("[E_MACHINE_DEVICE_RESET_001] reset guest halted early".into())
                }
                other => {
                    return Err(format!(
                        "[E_MACHINE_DEVICE_RESET_001] unexpected reset exit: {other:?}"
                    )
                    .into())
                }
            }
        }
        if !reset_completed || reset_markers != b"R" {
            return Err("[E_MACHINE_DEVICE_RESET_001] vCPU reset marker mismatch".into());
        }
        lifecycle.stop("certification complete")?;
        let low_dirty_bitmap = vm.get_dirty_log(0, FRAMEBUFFER_MMIO as usize)?;
        let high_dirty_bitmap = vm.get_dirty_log(1, KVM_RAM_BYTES - KVM_HIGH_RAM_START)?;
        let dirty_pages = low_dirty_bitmap
            .iter()
            .chain(high_dirty_bitmap.iter())
            .map(|word| word.count_ones() as u64)
            .sum::<u64>();
        if dirty_pages == 0
            || lifecycle.reset_count() != 1
            || lifecycle.state() != VmPowerState::Stopped
        {
            return Err("[E_MACHINE_DEVICE_RESET_001] reset/lifecycle evidence incomplete".into());
        }
        let trace = json!({
            "schema": "ncc.machine.virtual_devices_kvm_trace.v1",
            "exits": exits,
            "markers": String::from_utf8_lossy(&markers),
            "reset_markers": String::from_utf8_lossy(&reset_markers),
            "lifecycle_events": lifecycle.events().map(|event| json!({
                "sequence": event.sequence,
                "from": format!("{:?}", event.from),
                "to": format!("{:?}", event.to),
                "reason": event.reason.clone(),
            })).collect::<Vec<_>>(),
        });
        write_json(&bundle.join("kvm-trace.json"), &trace)?;
        Ok(json!({
            "passed": true,
            "cpu_backed": true,
            "direct_kvm_api": true,
            "elapsed_milliseconds": started.elapsed().as_millis(),
            "exit_count": trace["exits"].as_array().map(Vec::len).unwrap_or(0),
            "uart": { "bytes": uart_bytes, "passed": true },
            "ps2": { "status": memory.bytes()[PS2_STATUS_RESULT], "scancode": memory.bytes()[PS2_DATA_RESULT], "passed": true },
            "pci": { "vendor_device": format!("{pci_value:08x}"), "functions": rig.pci.functions.len(), "passed": true },
            "virtio_block": { "status": memory.bytes()[BLOCK_STATUS], "sector_sha256": sha256_bytes(&block_sector), "passed": true },
            "virtio_network": { "frame_bytes": network_frame.len(), "frame_sha256": sha256_bytes(&network_frame), "passed": true },
            "virtio_console": { "bytes": console_bytes, "passed": true },
            "virtio_input": { "press": input_press, "sync": input_sync, "passed": true },
            "virtio_gpu": { "framebuffer_crc32": format!("{framebuffer_crc32:08x}"), "passed": true },
            "debug": debug_evidence,
            "lifecycle": {
                "pause_resume": true,
                "warm_reset": true,
                "stopped_cleanly": true,
                "reset_count": lifecycle.reset_count(),
                "event_count": lifecycle.events().count(),
            },
            "persistence": {
                "full_snapshot": "vm-full.ncsnap",
                "incremental_snapshot": "vm-delta.ncsnap",
                "atomic_files": true,
                "roundtrip": true,
            },
            "dirty_pages": dirty_pages,
            "guest_memory_map": {
                "low_ram_bytes": FRAMEBUFFER_MMIO,
                "mmio_hole_address": FRAMEBUFFER_MMIO,
                "mmio_hole_bytes": KVM_MMIO_HOLE_BYTES,
                "high_ram_start": KVM_HIGH_RAM_START,
                "high_ram_end": KVM_RAM_BYTES,
            },
        }))
    }

    fn resolve_program(override_path: Option<&str>, fallback: &str) -> String {
        override_path.unwrap_or(fallback).to_string()
    }

    fn program_probe(program: &str) -> Value {
        match Command::new(program).arg("--version").output() {
            Ok(output) => json!({
                "program": program,
                "available": output.status.success(),
                "status": output.status.code(),
                "version": String::from_utf8_lossy(&output.stdout).lines().next(),
                "stderr": String::from_utf8_lossy(&output.stderr).lines().next(),
            }),
            Err(error) => json!({
                "program": program,
                "available": false,
                "error": error.to_string(),
            }),
        }
    }

    pub(super) fn probe_qemu(override_path: Option<&str>) -> Value {
        program_probe(&resolve_program(override_path, "qemu-system-x86_64"))
    }

    pub(super) fn probe_qemu_img(override_path: Option<&str>) -> Value {
        program_probe(&resolve_program(override_path, "qemu-img"))
    }

    pub(super) fn available() -> bool {
        Path::new("/dev/kvm").exists()
            && program_probe("qemu-system-x86_64")["available"] == true
            && program_probe("qemu-img")["available"] == true
    }

    struct QmpClient {
        reader: BufReader<UnixStream>,
        writer: UnixStream,
        next_id: u64,
        transcript: Vec<Value>,
    }

    impl QmpClient {
        fn connect(path: &Path) -> Result<Self, Box<dyn Error>> {
            let stream = UnixStream::connect(path)?;
            stream.set_read_timeout(Some(Duration::from_secs(10)))?;
            stream.set_write_timeout(Some(Duration::from_secs(10)))?;
            let writer = stream.try_clone()?;
            let mut client = Self {
                reader: BufReader::new(stream),
                writer,
                next_id: 1,
                transcript: Vec::new(),
            };
            let greeting = client.read_message()?;
            if greeting.get("QMP").is_none() {
                return Err("[E_MACHINE_DEVICE_QMP_001] missing QMP greeting".into());
            }
            client.transcript.push(json!({ "receive": greeting }));
            client.call("qmp_capabilities", None)?;
            Ok(client)
        }

        fn read_message(&mut self) -> Result<Value, Box<dyn Error>> {
            loop {
                let mut line = String::new();
                let count = self.reader.read_line(&mut line)?;
                if count == 0 {
                    return Err("[E_MACHINE_DEVICE_QMP_001] QMP socket closed".into());
                }
                if !line.trim().is_empty() {
                    return Ok(serde_json::from_str(line.trim())?);
                }
            }
        }

        fn call(
            &mut self,
            command: &str,
            arguments: Option<Value>,
        ) -> Result<Value, Box<dyn Error>> {
            let id = self.next_id;
            self.next_id += 1;
            let mut request = json!({ "execute": command, "id": id });
            if let Some(arguments) = arguments {
                request["arguments"] = arguments;
            }
            self.transcript.push(json!({ "send": request.clone() }));
            serde_json::to_writer(&mut self.writer, &request)?;
            self.writer.write_all(b"\r\n")?;
            self.writer.flush()?;
            loop {
                let response = self.read_message()?;
                self.transcript.push(json!({ "receive": response.clone() }));
                if response.get("id").and_then(Value::as_u64) != Some(id) {
                    continue;
                }
                if let Some(error) = response.get("error") {
                    return Err(format!(
                        "[E_MACHINE_DEVICE_QMP_001] QMP command '{command}' failed: {error}"
                    )
                    .into());
                }
                return response.get("return").cloned().ok_or_else(|| {
                    format!("[E_MACHINE_DEVICE_QMP_001] QMP command '{command}' has no result")
                        .into()
                });
            }
        }

        fn send_no_wait(
            &mut self,
            command: &str,
            arguments: Option<Value>,
        ) -> Result<(), Box<dyn Error>> {
            let id = self.next_id;
            self.next_id += 1;
            let mut request = json!({ "execute": command, "id": id });
            if let Some(arguments) = arguments {
                request["arguments"] = arguments;
            }
            self.transcript.push(json!({ "send": request.clone() }));
            serde_json::to_writer(&mut self.writer, &request)?;
            self.writer.write_all(b"\r\n")?;
            self.writer.flush()?;
            Ok(())
        }
    }

    struct QemuGuard {
        child: Child,
        socket: PathBuf,
    }

    impl QemuGuard {
        fn finish(mut self) -> Result<std::process::ExitStatus, Box<dyn Error>> {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if let Some(status) = self.child.try_wait()? {
                    let _ = fs::remove_file(&self.socket);
                    return Ok(status);
                }
                if Instant::now() >= deadline {
                    self.child.kill()?;
                    let status = self.child.wait()?;
                    let _ = fs::remove_file(&self.socket);
                    return Err(format!(
                        "[E_MACHINE_DEVICE_QEMU_001] QEMU did not exit after quit ({status})"
                    )
                    .into());
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    impl Drop for QemuGuard {
        fn drop(&mut self) {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
            let _ = fs::remove_file(&self.socket);
        }
    }

    fn wait_for_qmp(child: &mut Child, socket: &Path) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if socket.exists() {
                return Ok(());
            }
            if let Some(status) = child.try_wait()? {
                return Err(format!(
                    "[E_MACHINE_DEVICE_QEMU_001] QEMU exited before QMP was ready: {status}"
                )
                .into());
            }
            if Instant::now() >= deadline {
                return Err("[E_MACHINE_DEVICE_QEMU_001] QMP socket startup timeout".into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    fn hmp(client: &mut QmpClient, command: &str) -> Result<String, Box<dyn Error>> {
        let result = client.call(
            "human-monitor-command",
            Some(json!({ "command-line": command })),
        )?;
        let output = result.as_str().unwrap_or_default().to_string();
        let lowered = output.to_ascii_lowercase();
        if lowered.contains("error") || lowered.contains("failed") {
            return Err(
                format!("[E_MACHINE_DEVICE_QMP_001] HMP '{command}' failed: {output}").into(),
            );
        }
        Ok(output)
    }

    fn execute_qemu(bundle: &Path, args: &[String]) -> Result<Value, Box<dyn Error>> {
        let qemu = resolve_program(option_value(args, "--qemu"), "qemu-system-x86_64");
        let qemu_img = resolve_program(option_value(args, "--qemu-img"), "qemu-img");
        if program_probe(&qemu)["available"] != true
            || program_probe(&qemu_img)["available"] != true
        {
            return Err(
                "[E_MACHINE_DEVICE_QEMU_001] qemu-system-x86_64 and qemu-img are required".into(),
            );
        }
        let disk = bundle.join("qemu-device-disk.qcow2");
        let image_output = Command::new(&qemu_img)
            .args(["create", "-f", "qcow2"])
            .arg(&disk)
            .arg("16M")
            .output()?;
        fs::write(bundle.join("qemu-img-create.stdout"), &image_output.stdout)?;
        fs::write(bundle.join("qemu-img-create.stderr"), &image_output.stderr)?;
        if !image_output.status.success() {
            return Err(format!(
                "[E_MACHINE_DEVICE_QEMU_IMG_001] qemu-img create failed: {}",
                String::from_utf8_lossy(&image_output.stderr)
            )
            .into());
        }
        let qmp_socket = PathBuf::from(format!(
            "/tmp/ncc-device-qmp-{}-{}.sock",
            std::process::id(),
            epoch_seconds()
        ));
        if qmp_socket.exists() {
            fs::remove_file(&qmp_socket)?;
        }
        let log_path = bundle.join("qemu.log");
        let console_path = bundle.join("qemu-virtconsole.log");
        let log = fs::File::create(&log_path)?;
        let log_error = log.try_clone()?;
        let mut child = Command::new(&qemu)
            .args([
                "-machine",
                "q35,accel=kvm",
                "-cpu",
                "host",
                "-m",
                "128M",
                "-smp",
                "2",
                "-S",
                "-nodefaults",
                "-display",
                "none",
                "-serial",
                "none",
                "-monitor",
                "none",
                "-qmp",
            ])
            .arg(format!("unix:{},server=on,wait=off", qmp_socket.display()))
            .args(["-drive"])
            .arg(format!(
                "file={},if=none,id=drive0,format=qcow2,cache=unsafe",
                disk.display()
            ))
            .args([
                "-device",
                "virtio-blk-pci,drive=drive0,id=block0",
                "-netdev",
                "user,id=net0,restrict=on",
                "-device",
                "virtio-net-pci,netdev=net0,id=network0",
                "-device",
                "virtio-serial-pci,id=serial0",
                "-chardev",
            ])
            .arg(format!("file,id=consolech,path={}", console_path.display()))
            .args([
                "-device",
                "virtconsole,chardev=consolech,id=console0",
                "-device",
                "virtio-keyboard-pci,id=input0",
                "-device",
                "virtio-gpu-pci,id=gpu0",
            ])
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_error))
            .spawn()?;
        wait_for_qmp(&mut child, &qmp_socket)?;
        let guard = QemuGuard {
            child,
            socket: qmp_socket.clone(),
        };
        let mut client = QmpClient::connect(&qmp_socket)?;
        let kvm = client.call("query-kvm", None)?;
        if kvm.get("enabled").and_then(Value::as_bool) != Some(true)
            || kvm.get("present").and_then(Value::as_bool) != Some(true)
        {
            return Err(format!(
                "[E_MACHINE_DEVICE_QEMU_KVM_001] QEMU is not KVM accelerated: {kvm}"
            )
            .into());
        }
        let initial = client.call("query-status", None)?;
        if !matches!(
            initial.get("status").and_then(Value::as_str),
            Some("paused" | "prelaunch")
        ) {
            return Err(format!(
                "[E_MACHINE_DEVICE_QMP_STATE_001] expected initial paused/prelaunch state: {initial}"
            )
            .into());
        }
        let topology = client.call("query-pci", None)?;
        let topology_text = serde_json::to_string(&topology)?;
        for id in ["block0", "network0", "serial0", "input0", "gpu0"] {
            if !topology_text.contains(id) {
                return Err(format!(
                    "[E_MACHINE_DEVICE_QEMU_TOPOLOGY_001] missing QEMU PCI device '{id}'"
                )
                .into());
            }
        }
        let save_output = hmp(&mut client, "savevm ncc-device-snapshot")?;
        let snapshot_list = hmp(&mut client, "info snapshots")?;
        if !snapshot_list.contains("ncc-device-snapshot") {
            return Err("[E_MACHINE_DEVICE_QMP_SNAPSHOT_001] savevm snapshot is absent".into());
        }
        client.call("cont", None)?;
        let running = client.call("query-status", None)?;
        if running.get("status").and_then(Value::as_str) != Some("running") {
            return Err(
                format!("[E_MACHINE_DEVICE_QMP_STATE_001] cont did not run VM: {running}").into(),
            );
        }
        client.call("stop", None)?;
        let paused = client.call("query-status", None)?;
        if paused.get("status").and_then(Value::as_str) != Some("paused") {
            return Err(format!(
                "[E_MACHINE_DEVICE_QMP_STATE_001] stop did not pause VM: {paused}"
            )
            .into());
        }
        let load_output = hmp(&mut client, "loadvm ncc-device-snapshot")?;
        client.call("system_reset", None)?;
        let reset = client.call("query-status", None)?;
        if reset.get("running").and_then(Value::as_bool) != Some(false)
            || !matches!(
                reset.get("status").and_then(Value::as_str),
                Some("paused" | "prelaunch")
            )
        {
            return Err(format!(
                "[E_MACHINE_DEVICE_QMP_STATE_001] reset did not preserve a stopped prelaunch/paused state: {reset}"
            )
            .into());
        }
        client.send_no_wait("quit", None)?;
        let transcript = json!({
            "schema": "ncc.machine.virtual_devices_qmp_trace.v1",
            "messages": client.transcript,
        });
        write_json(&bundle.join("qmp-transcript.json"), &transcript)?;
        drop(client);
        let status = guard.finish()?;
        if !status.success() {
            return Err(format!("[E_MACHINE_DEVICE_QEMU_001] QEMU exited with {status}").into());
        }
        let snapshot_info = Command::new(&qemu_img)
            .args(["snapshot", "-l"])
            .arg(&disk)
            .output()?;
        fs::write(
            bundle.join("qemu-img-snapshots.stdout"),
            &snapshot_info.stdout,
        )?;
        fs::write(
            bundle.join("qemu-img-snapshots.stderr"),
            &snapshot_info.stderr,
        )?;
        if !snapshot_info.status.success()
            || !String::from_utf8_lossy(&snapshot_info.stdout).contains("ncc-device-snapshot")
        {
            return Err(
                "[E_MACHINE_DEVICE_QMP_SNAPSHOT_001] qemu-img did not retain snapshot".into(),
            );
        }
        Ok(json!({
            "passed": true,
            "qemu": qemu,
            "qemu_img": qemu_img,
            "kvm": kvm,
            "initial_status": initial,
            "running_status": running,
            "paused_status": paused,
            "reset_status": reset,
            "pci_topology": topology,
            "required_device_ids": ["block0", "network0", "serial0", "input0", "gpu0"],
            "savevm": { "name": "ncc-device-snapshot", "output": save_output, "listed": true },
            "loadvm": { "name": "ncc-device-snapshot", "output": load_output, "passed": true },
            "transcript": "qmp-transcript.json",
            "disk": "qemu-device-disk.qcow2",
        }))
    }

    pub(super) fn execute(bundle: &Path, args: &[String]) -> Result<Value, Box<dyn Error>> {
        if !available() {
            return Err("[E_MACHINE_DEVICE_HOST_001] /dev/kvm, qemu-system-x86_64 and qemu-img are required".into());
        }
        let started = Instant::now();
        let kvm = execute_kvm(bundle)?;
        let qemu_qmp = execute_qemu(bundle, args)?;
        let elapsed = started.elapsed();
        if elapsed > RUN_TIMEOUT {
            return Err(format!(
                "[E_MACHINE_DEVICE_TIMEOUT_001] combined certification exceeded {} ms",
                RUN_TIMEOUT.as_millis()
            )
            .into());
        }
        Ok(json!({
            "passed": true,
            "elapsed_milliseconds": elapsed.as_millis(),
            "deadline_milliseconds": RUN_TIMEOUT.as_millis(),
            "kvm": kvm,
            "qemu_qmp": qemu_qmp,
        }))
    }
}
