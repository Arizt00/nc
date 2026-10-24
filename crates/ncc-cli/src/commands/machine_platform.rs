use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PLATFORM_CERTIFICATE_SCHEMA: &str =
    "ncc.machine.x86_apic_smp_guest_boot_hardware_certificate.v1";
const PLATFORM_PAYLOAD_DOMAIN: &[u8] =
    b"ncc.machine.x86_apic_smp_guest_boot_hardware_certificate.v1\0";
pub(crate) const PLATFORM_CERTIFICATION_TIMEOUT_MS: u64 = 30_000;

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
            "--out" | "--bundle" | "--qemu" | "--bios" | "--uefi"
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

fn payload_sha256(value: &Value) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(PLATFORM_PAYLOAD_DOMAIN);
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
            .unwrap_or("platform-certificate");
        return parent.join(format!("{stem}-bundle"));
    }
    PathBuf::from("target/ncc-machine-platform-certificate")
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
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            output.push(json!({
                "path": relative,
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
        "kernel_release": read_trimmed("/proc/sys/kernel/osrelease"),
        "kernel_version": read_trimmed("/proc/sys/kernel/version"),
        "nested_intel": read_trimmed("/sys/module/kvm_intel/parameters/nested"),
        "nested_amd": read_trimmed("/sys/module/kvm_amd/parameters/nested"),
        "dmi_product": read_trimmed("/sys/class/dmi/id/product_name"),
        "dmi_vendor": read_trimmed("/sys/class/dmi/id/sys_vendor"),
        "logical_processors": std::thread::available_parallelism().map(usize::from).ok(),
        "kvm_device": "/dev/kvm",
        "kvm_exists": Path::new("/dev/kvm").exists(),
    })
}

pub(crate) fn run_certify_platform(args: &[String]) -> Result<(), Box<dyn Error>> {
    let out = option_value(args, "--out");
    let bundle = option_value(args, "--bundle")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_bundle_path(out));
    if bundle.exists() {
        return Err(format!(
            "[E_MACHINE_PLATFORM_BUNDLE_001] bundle already exists: {}",
            bundle.display()
        )
        .into());
    }
    fs::create_dir_all(&bundle)?;

    let execution = platform_host::execute(&bundle, args)?;
    write_json(&bundle.join("execution.json"), &execution)?;
    let artifacts = collect_artifacts(&bundle)?;
    let payload = json!({
        "schema": PLATFORM_CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "cpu_backed_apic_smp_and_firmware_boot_passed",
        "created_epoch_seconds": epoch_seconds(),
        "host": host_probe(),
        "bundle_root": fs::canonicalize(&bundle)?.to_string_lossy(),
        "execution": execution,
        "artifacts": artifacts,
        "truth_boundary": {
            "direct_kvm_api": true,
            "qemu_kvm_acceleration": true,
            "qemu_tcg": false,
            "in_kernel_irqchip": true,
            "cpu_backed_lapic_timer": true,
            "cpu_backed_pit_timer": true,
            "cpu_backed_two_vcpu_delivery": true,
            "cpu_backed_msi_delivery": true,
            "cpu_backed_ioapic_delivery": true,
            "seabios_boot_executed": true,
            "ovmf_uefi_boot_executed": true,
            "bare_metal_board": false,
            "physical_board_certificate": false,
        },
    });
    let mut certificate = payload.clone();
    certificate["attestation"] = json!({
        "algorithm": "sha256",
        "domain": String::from_utf8_lossy(PLATFORM_PAYLOAD_DOMAIN).trim_end_matches('\0'),
        "payload_sha256": payload_sha256(&payload)?,
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
            "OK: APIC/SMP/guest-boot platform certificate written to {}",
            out.unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_platform_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args)
        .map(PathBuf::from)
        .ok_or("virtualization verify-platform-certificate requires a certificate path")?;
    let mut certificate: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(PLATFORM_CERTIFICATE_SCHEMA) {
        return Err("[E_MACHINE_PLATFORM_CERTIFICATE_001] unsupported certificate schema".into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_001] missing payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_001] certificate root must be an object")?
        .remove("attestation");
    let actual = payload_sha256(&certificate)?;
    if expected != actual {
        return Err(format!(
            "[E_MACHINE_PLATFORM_CERTIFICATE_HASH_001] payload hash mismatch: expected {expected}, got {actual}"
        )
        .into());
    }

    for pointer in [
        "/truth_boundary/direct_kvm_api",
        "/truth_boundary/qemu_kvm_acceleration",
        "/truth_boundary/in_kernel_irqchip",
        "/truth_boundary/cpu_backed_lapic_timer",
        "/truth_boundary/cpu_backed_pit_timer",
        "/truth_boundary/cpu_backed_two_vcpu_delivery",
        "/truth_boundary/cpu_backed_msi_delivery",
        "/truth_boundary/cpu_backed_ioapic_delivery",
        "/truth_boundary/seabios_boot_executed",
        "/truth_boundary/ovmf_uefi_boot_executed",
        "/execution/kvm/lapic_timer/passed",
        "/execution/kvm/pit_timer/passed",
        "/execution/kvm/smp/passed",
        "/execution/firmware/bios/passed",
        "/execution/firmware/uefi/passed",
    ] {
        if certificate.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] required evidence '{pointer}' is not true"
            )
            .into());
        }
    }

    let root = certificate
        .get("bundle_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] missing bundle root")?;
    let artifacts = certificate
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] missing artifact manifest")?;
    if artifacts.is_empty() {
        return Err(
            "[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] artifact manifest is empty".into(),
        );
    }
    for artifact in artifacts {
        let relative = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] artifact path is missing")?;
        let expected_hash = artifact
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or("[E_MACHINE_PLATFORM_CERTIFICATE_EVIDENCE_001] artifact hash is missing")?;
        let artifact_path = root.join(relative);
        let bytes = fs::read(&artifact_path).map_err(|error| {
            format!(
                "[E_MACHINE_PLATFORM_CERTIFICATE_ARTIFACT_001] cannot read {}: {error}",
                artifact_path.display()
            )
        })?;
        if sha256_bytes(&bytes) != expected_hash {
            return Err(format!(
                "[E_MACHINE_PLATFORM_CERTIFICATE_ARTIFACT_001] artifact hash mismatch: {}",
                artifact_path.display()
            )
            .into());
        }
    }

    let report = json!({
        "schema": "ncc.machine.x86_apic_smp_guest_boot_hardware_certificate_verification.v1",
        "status": "verified",
        "certificate": path,
        "payload_sha256": actual,
        "verified_artifacts": artifacts.len(),
        "cpu_backed_execution": true,
        "bare_metal_board": false,
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("OK: APIC/SMP/guest-boot platform certificate verified");
    }
    Ok(())
}

pub(crate) fn run_probe_platform(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ncc.machine.x86_apic_smp_guest_boot_host_probe.v1",
        "host": host_probe(),
        "certification_available": platform_host::available(),
        "qemu": platform_host::probe_qemu(option_value(args, "--qemu")),
        "bios": platform_host::probe_bios(option_value(args, "--bios")),
        "uefi": platform_host::probe_uefi(option_value(args, "--uefi")),
        "required_command": "ncc machine virtualization certify-platform --out certificate.json --bundle certificate-bundle",
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("machine platform host probe: {}", payload);
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn execute_direct_kvm_platform_evidence() -> Result<Value, Box<dyn Error>> {
    platform_host::execute_direct()
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod platform_host {
    use super::*;
    use kvm_bindings::{
        kvm_irqchip, kvm_mp_state, kvm_msi, kvm_msr_entry, kvm_pit_config,
        kvm_userspace_memory_region, Msrs, KVM_IRQCHIP_IOAPIC, KVM_MP_STATE_RUNNABLE,
    };
    use kvm_ioctls::{Cap, Kvm, VcpuExit, VcpuFd, VmFd};
    use ncc_machine::{
        build_bios_boot_sector, build_bios_disk_image, build_uefi_fat12_disk,
        build_x86_64_uefi_application, detect_guest_image, GuestFirmwareDescriptor,
    };
    use std::process::{Command, Stdio};
    use std::ptr::null_mut;
    use std::slice;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    const GUEST_MEMORY_BYTES: usize = 0x20_000;
    const DEBUG_PORT: u16 = 0x00e9;
    const DONE_PORT: u16 = 0x00f1;
    const BSP_CODE: usize = 0x1000;
    const AP_CODE: usize = 0x1200;
    const TIMER_CODE: usize = 0x1400;
    const LAPIC_HANDLER: usize = 0x1800;
    const MSI_HANDLER: usize = 0x1840;
    const IOAPIC_HANDLER: usize = 0x1880;
    const PIT_HANDLER: usize = 0x18c0;
    const GP_HANDLER: usize = 0x1900;
    const PF_HANDLER: usize = 0x1940;
    const UD_HANDLER: usize = 0x1980;
    const DF_HANDLER: usize = 0x19c0;
    const RUN_TIMEOUT_SIGNAL: libc::c_int = libc::SIGUSR2;
    const RUN_TIMEOUT: Duration = Duration::from_millis(PLATFORM_CERTIFICATION_TIMEOUT_MS);

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
                        "[E_MACHINE_PLATFORM_SIGNAL_001] cannot install timeout signal: {}",
                        std::io::Error::last_os_error()
                    )
                    .into());
                }
                libc::sigemptyset(&mut signal_set);
                libc::sigaddset(&mut signal_set, RUN_TIMEOUT_SIGNAL);
                if libc::pthread_sigmask(libc::SIG_UNBLOCK, &signal_set, &mut previous_mask) != 0 {
                    libc::sigaction(RUN_TIMEOUT_SIGNAL, &previous, null_mut());
                    return Err(
                        "[E_MACHINE_PLATFORM_SIGNAL_001] cannot unblock timeout signal".into(),
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
                    "[E_MACHINE_PLATFORM_MEMORY_001] mmap failed: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
            Ok(Self {
                pointer: pointer.cast(),
                length,
            })
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

    fn create_vm_memory(kvm: &Kvm) -> Result<(VmFd, GuestMemory), Box<dyn Error>> {
        let vm = kvm.create_vm()?;
        vm.create_irq_chip()?;
        let memory = GuestMemory::new(GUEST_MEMORY_BYTES)?;
        unsafe {
            vm.set_user_memory_region(kvm_userspace_memory_region {
                slot: 0,
                flags: 0,
                guest_phys_addr: 0,
                memory_size: GUEST_MEMORY_BYTES as u64,
                userspace_addr: memory.address(),
            })?
        };
        Ok((vm, memory))
    }

    fn write_ivt(memory: &mut [u8], vector: u8, offset: usize) {
        let entry = vector as usize * 4;
        memory[entry..entry + 2].copy_from_slice(&(offset as u16).to_le_bytes());
        memory[entry + 2..entry + 4].copy_from_slice(&0_u16.to_le_bytes());
    }

    fn marker_handler(marker: u8, pic_eoi: bool) -> Vec<u8> {
        let mut code = vec![0x50, 0x52]; // push ax; push dx
        if pic_eoi {
            code.extend_from_slice(&[0xb0, 0x20, 0xe6, 0x20]);
        }
        code.extend_from_slice(&[
            0xba, 0xe9, 0x00, // mov dx, 0xe9
            0xb0, marker, 0xee, // mov al, marker; out dx, al
            0x5a, 0x58, 0xcf, // pop dx; pop ax; iret
        ]);
        code
    }

    fn fatal_handler(marker: u8) -> Vec<u8> {
        vec![
            0xfa, // cli
            0xba, 0xe9, 0x00, // mov dx, 0xe9
            0xb0, marker, 0xee, // diagnostic marker
            0xba, 0xf1, 0x00, 0xee, // explicit failed completion
            0xf4, 0xeb, 0xfe,
        ]
    }

    fn install_fault_handlers(memory: &mut [u8]) {
        for (vector, address, marker) in [
            (13, GP_HANDLER, b'G'),
            (14, PF_HANDLER, b'F'),
            (6, UD_HANDLER, b'U'),
            (8, DF_HANDLER, b'D'),
        ] {
            write_program(memory, address, &fatal_handler(marker));
            write_ivt(memory, vector, address);
        }
    }

    fn wait_program(first: u8, after: &[u8]) -> Vec<u8> {
        let mut code = vec![
            0xba, 0xe9, 0x00, // mov dx, 0xe9
            0xb0, first, 0xee, // marker
        ];
        for marker in after {
            code.extend_from_slice(&[
                0xfb, 0xf4, // sti; hlt
                0xfa, // leave the host synchronization point with IF clear
                0xb0, *marker, 0xee, // continuation marker
            ]);
        }
        code.extend_from_slice(&[
            0xfa, 0xba, 0xf1, 0x00, 0xb0, 0x00, 0xee, // explicit completion
            0xf4, 0xeb, 0xfe,
        ]);
        code
    }

    fn pit_program() -> Vec<u8> {
        vec![
            0xfa, // cli
            0xb0, 0x11, 0xe6, 0x20, 0xe6, 0xa0, // ICW1
            0xb0, 0x20, 0xe6, 0x21, // master vector 0x20
            0xb0, 0x28, 0xe6, 0xa1, // slave vector 0x28
            0xb0, 0x04, 0xe6, 0x21, // cascade
            0xb0, 0x02, 0xe6, 0xa1, // cascade identity
            0xb0, 0x01, 0xe6, 0x21, 0xe6, 0xa1, // 8086 mode
            0xb0, 0xfe, 0xe6, 0x21, // unmask IRQ0 only
            0xb0, 0xff, 0xe6, 0xa1, // mask slave
            0xb0, 0x30, 0xe6, 0x43, // PIT channel 0, lo/hi, mode 0
            0xb0, 0xff, 0xe6, 0x40, // count low
            0xb0, 0x0f, 0xe6, 0x40, // count high
            0xba, 0xe9, 0x00, 0xb0, b'P', 0xee, // armed marker
            0xfb, 0xf4, // sti; hlt
            0xb0, b'p', 0xee, // resumed marker
            0xfa, 0xba, 0xf1, 0x00, 0xb0, 0x00, 0xee, // explicit completion
            0xf4, 0xeb, 0xfe,
        ]
    }

    fn write_program(memory: &mut [u8], address: usize, code: &[u8]) {
        memory[address..address + code.len()].copy_from_slice(code);
    }

    fn configure_real_mode(vcpu: &VcpuFd, rip: usize, stack: usize) -> Result<(), Box<dyn Error>> {
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
        registers.rsp = stack as u64;
        registers.rflags = 2;
        vcpu.set_regs(&registers)?;
        Ok(())
    }

    fn enable_xapic(vcpu: &VcpuFd, bsp: bool) -> Result<u64, Box<dyn Error>> {
        let mut current = Msrs::from_entries(&[kvm_msr_entry {
            index: ncc_machine::IA32_APIC_BASE_MSR,
            ..Default::default()
        }])?;
        if vcpu.get_msrs(&mut current)? != 1 {
            return Err("[E_MACHINE_PLATFORM_APIC_MSR_001] cannot read IA32_APIC_BASE".into());
        }
        let preserved = current.as_slice()[0].data;
        let mut value = (preserved & 0xfff) | ncc_machine::X86_LOCAL_APIC_BASE | (1 << 11);
        value &= !(1 << 10);
        if bsp {
            value |= 1 << 8;
        } else {
            value &= !(1 << 8);
        }
        let replacement = Msrs::from_entries(&[kvm_msr_entry {
            index: ncc_machine::IA32_APIC_BASE_MSR,
            data: value,
            ..Default::default()
        }])?;
        if vcpu.set_msrs(&replacement)? != 1 {
            return Err("[E_MACHINE_PLATFORM_APIC_MSR_001] cannot write IA32_APIC_BASE".into());
        }
        Ok(value)
    }

    fn lapic_write(state: &mut kvm_bindings::kvm_lapic_state, offset: usize, value: u32) {
        for (index, byte) in value.to_le_bytes().iter().enumerate() {
            state.regs[offset + index] = *byte as i8;
        }
    }

    fn lapic_read(state: &kvm_bindings::kvm_lapic_state, offset: usize) -> u32 {
        u32::from_le_bytes([
            state.regs[offset] as u8,
            state.regs[offset + 1] as u8,
            state.regs[offset + 2] as u8,
            state.regs[offset + 3] as u8,
        ])
    }

    enum RunExit {
        Halt,
        Shutdown,
        Io(Vec<u8>),
        Done,
    }

    fn run_once(vcpu: &mut VcpuFd) -> Result<RunExit, Box<dyn Error>> {
        let target_thread = unsafe { libc::pthread_self() } as usize;
        let cancelled = Arc::new((Mutex::new(false), Condvar::new()));
        let timed_out = Arc::new(AtomicBool::new(false));
        let watchdog_cancelled = Arc::clone(&cancelled);
        let watchdog_timed_out = Arc::clone(&timed_out);
        let watchdog = thread::spawn(move || {
            let (lock, condition) = &*watchdog_cancelled;
            let guard = lock.lock().expect("platform watchdog mutex poisoned");
            let (guard, timeout) = condition
                .wait_timeout_while(guard, RUN_TIMEOUT, |cancelled| !*cancelled)
                .expect("platform watchdog condition poisoned");
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
                .map_err(|_| "[E_MACHINE_PLATFORM_KVM_001] watchdog mutex poisoned")? = true;
            condition.notify_one();
        }
        watchdog
            .join()
            .map_err(|_| "[E_MACHINE_PLATFORM_KVM_001] watchdog thread panicked")?;
        if timed_out.load(Ordering::Acquire) {
            return Err(format!(
                "[E_MACHINE_PLATFORM_TIMEOUT_001] KVM_RUN exceeded {} ms",
                RUN_TIMEOUT.as_millis()
            )
            .into());
        }
        match result? {
            VcpuExit::IoOut(port, data) if port == DEBUG_PORT => Ok(RunExit::Io(data.to_vec())),
            VcpuExit::IoOut(port, _) if port == DONE_PORT => Ok(RunExit::Done),
            VcpuExit::Hlt => Ok(RunExit::Halt),
            VcpuExit::Shutdown => Ok(RunExit::Shutdown),
            other => {
                let description = format!("{other:?}");
                let registers = vcpu.get_regs()?;
                Err(format!(
                    "[E_MACHINE_PLATFORM_KVM_EXIT_001] unexpected KVM exit: {description}; rip={:#x} rax={:#x} rdx={:#x} rflags={:#x}",
                    registers.rip, registers.rax, registers.rdx, registers.rflags
                )
                .into())
            }
        }
    }

    fn run_to_done(vcpu: &mut VcpuFd) -> Result<(String, u64), Box<dyn Error>> {
        let mut markers = Vec::new();
        let mut exits = 0_u64;
        loop {
            exits += 1;
            if exits > 64 {
                return Err("[E_MACHINE_PLATFORM_KVM_EXIT_001] guest exceeded 64 exits".into());
            }
            let next = run_once(vcpu).map_err(|error| {
                format!(
                    "{error}; markers_before_failure={:?}; exits={exits}",
                    String::from_utf8_lossy(&markers)
                )
            })?;
            match next {
                RunExit::Io(bytes) => markers.extend_from_slice(&bytes),
                RunExit::Done => return Ok((String::from_utf8(markers)?, exits)),
                RunExit::Halt => {
                    return Err(
                        "[E_MACHINE_PLATFORM_KVM_EXIT_001] guest halted before explicit completion"
                            .into(),
                    )
                }
                RunExit::Shutdown => {
                    return Err("[E_MACHINE_PLATFORM_KVM_EXIT_001] guest shut down".into())
                }
            }
        }
    }

    fn run_expected_marker(vcpu: &mut VcpuFd, expected: u8) -> Result<(), Box<dyn Error>> {
        match run_once(vcpu)? {
            RunExit::Io(bytes) if bytes.as_slice() == [expected] => Ok(()),
            RunExit::Io(bytes) => Err(format!(
                "[E_MACHINE_PLATFORM_MARKER_001] expected marker {:?}, got {:?}",
                expected as char,
                String::from_utf8_lossy(&bytes)
            )
            .into()),
            RunExit::Done => {
                Err("[E_MACHINE_PLATFORM_MARKER_001] guest completed before marker".into())
            }
            RunExit::Halt => {
                Err("[E_MACHINE_PLATFORM_MARKER_001] guest halted before marker".into())
            }
            RunExit::Shutdown => {
                Err("[E_MACHINE_PLATFORM_MARKER_001] guest shut down before marker".into())
            }
        }
    }

    fn certify_lapic_timer(kvm: &Kvm) -> Result<Value, Box<dyn Error>> {
        let (vm, mut memory) = create_vm_memory(kvm)?;
        install_fault_handlers(memory.bytes_mut());
        write_program(memory.bytes_mut(), TIMER_CODE, &wait_program(b'L', b"l"));
        write_program(
            memory.bytes_mut(),
            LAPIC_HANDLER,
            &marker_handler(b'T', false),
        );
        write_ivt(memory.bytes_mut(), 0x61, LAPIC_HANDLER);
        let mut vcpu = vm.create_vcpu(0)?;
        configure_real_mode(&vcpu, TIMER_CODE, GUEST_MEMORY_BYTES - 16)?;
        let apic_base_msr = enable_xapic(&vcpu, true)?;
        let mut lapic = vcpu.get_lapic()?;
        lapic_write(&mut lapic, 0x0f0, 0x1ff);
        lapic_write(&mut lapic, 0x080, 0);
        lapic_write(&mut lapic, 0x3e0, 0x0b);
        lapic_write(&mut lapic, 0x320, 0x61);
        lapic_write(&mut lapic, 0x380, 10_000_000);
        vcpu.set_lapic(&lapic)?;
        let started = Instant::now();
        run_expected_marker(&mut vcpu, b'L')?;
        let (tail, exits) = run_to_done(&mut vcpu)?;
        let markers = format!("L{tail}");
        let final_lapic = vcpu.get_lapic()?;
        let current_count = lapic_read(&final_lapic, 0x390);
        let passed = markers == "LTl";
        if !passed {
            return Err(format!(
                "[E_MACHINE_PLATFORM_LAPIC_001] expected marker LTl, got {markers:?}"
            )
            .into());
        }
        Ok(json!({
            "passed": true,
            "markers": markers,
            "vector": 0x61,
            "mode": "one_shot",
            "initial_count": 10_000_000_u32,
            "current_count_after_interrupt": current_count,
            "kvm_exits": exits,
            "elapsed_milliseconds": started.elapsed().as_millis(),
            "in_kernel_lapic": true,
            "ia32_apic_base_msr": format!("{apic_base_msr:#x}"),
        }))
    }

    fn certify_pit_timer(kvm: &Kvm) -> Result<Value, Box<dyn Error>> {
        let (vm, mut memory) = create_vm_memory(kvm)?;
        install_fault_handlers(memory.bytes_mut());
        vm.create_pit2(kvm_pit_config::default())?;
        write_program(memory.bytes_mut(), TIMER_CODE, &pit_program());
        write_program(memory.bytes_mut(), PIT_HANDLER, &marker_handler(b'T', true));
        write_ivt(memory.bytes_mut(), 0x20, PIT_HANDLER);
        let mut vcpu = vm.create_vcpu(0)?;
        configure_real_mode(&vcpu, TIMER_CODE, GUEST_MEMORY_BYTES - 16)?;
        let apic_base_msr = enable_xapic(&vcpu, true)?;
        let started = Instant::now();
        run_expected_marker(&mut vcpu, b'P')?;
        let (tail, exits) = run_to_done(&mut vcpu)?;
        let markers = format!("P{tail}");
        let pit = vm.get_pit2()?;
        let passed = markers == "PTp";
        if !passed {
            return Err(format!(
                "[E_MACHINE_PLATFORM_PIT_001] expected marker PTp, got {markers:?}"
            )
            .into());
        }
        Ok(json!({
            "passed": true,
            "markers": markers,
            "irq": 0,
            "vector": 0x20,
            "mode": "8254_one_shot",
            "reload_count": 4095,
            "pit_flags": pit.flags,
            "kvm_exits": exits,
            "elapsed_milliseconds": started.elapsed().as_millis(),
            "in_kernel_pit2": true,
            "ia32_apic_base_msr": format!("{apic_base_msr:#x}"),
        }))
    }

    fn configure_ioapic_route(
        vm: &VmFd,
        pin: usize,
        vector: u8,
        destination: u8,
    ) -> Result<u64, Box<dyn Error>> {
        let mut chip = kvm_irqchip::default();
        chip.chip_id = KVM_IRQCHIP_IOAPIC;
        vm.get_irqchip(&mut chip)?;
        let bits = u64::from(vector) | (u64::from(destination) << 56);
        unsafe {
            let mut ioapic = chip.chip.ioapic;
            ioapic.redirtbl[pin].bits = bits;
            chip.chip.ioapic = ioapic;
        }
        vm.set_irqchip(&chip)?;
        Ok(bits)
    }

    fn certify_smp(kvm: &Kvm) -> Result<Value, Box<dyn Error>> {
        let (vm, mut memory) = create_vm_memory(kvm)?;
        install_fault_handlers(memory.bytes_mut());
        write_program(memory.bytes_mut(), BSP_CODE, &wait_program(b'B', b"b"));
        write_program(memory.bytes_mut(), AP_CODE, &wait_program(b'A', b"io"));
        write_program(
            memory.bytes_mut(),
            MSI_HANDLER,
            &marker_handler(b'I', false),
        );
        write_program(
            memory.bytes_mut(),
            IOAPIC_HANDLER,
            &marker_handler(b'O', false),
        );
        write_ivt(memory.bytes_mut(), 0x42, MSI_HANDLER);
        write_ivt(memory.bytes_mut(), 0x53, IOAPIC_HANDLER);

        let mut bsp = vm.create_vcpu(0)?;
        let mut ap = vm.create_vcpu(1)?;
        configure_real_mode(&bsp, BSP_CODE, GUEST_MEMORY_BYTES - 16)?;
        configure_real_mode(&ap, AP_CODE, GUEST_MEMORY_BYTES - 0x100)?;
        let bsp_apic_base_msr = enable_xapic(&bsp, true)?;
        let ap_apic_base_msr = enable_xapic(&ap, false)?;
        ap.set_mp_state(kvm_mp_state {
            mp_state: KVM_MP_STATE_RUNNABLE,
        })?;
        for vcpu in [&bsp, &ap] {
            let mut lapic = vcpu.get_lapic()?;
            lapic_write(&mut lapic, 0x0f0, 0x1ff);
            vcpu.set_lapic(&lapic)?;
        }

        run_expected_marker(&mut bsp, b'B')
            .map_err(|error| format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] bsp_start: {error}"))?;
        run_expected_marker(&mut ap, b'A')
            .map_err(|error| format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] ap_start: {error}"))?;
        let bsp_markers = "B";
        let ap_start_markers = "A";
        let bsp_exits = 1_u64;
        let ap_start_exits = 1_u64;

        let msi = kvm_msi {
            address_lo: 0xfee0_0000 | (1 << 12),
            address_hi: 0,
            data: 0x42,
            flags: 0,
            devid: 0,
            pad: [0; 12],
        };
        let msi_delivered = vm.signal_msi(msi)?;
        run_expected_marker(&mut ap, b'I')
            .map_err(|error| format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] msi_handler: {error}"))?;
        run_expected_marker(&mut ap, b'i')
            .map_err(|error| format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] msi_resume: {error}"))?;
        let msi_markers = "Ii";
        let msi_exits = 2_u64;

        let redirection_bits = configure_ioapic_route(&vm, 16, 0x53, 1)?;
        vm.set_irq_line(16, true)?;
        vm.set_irq_line(16, false)?;
        run_expected_marker(&mut ap, b'O').map_err(|error| {
            format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] ioapic_handler: {error}")
        })?;
        run_expected_marker(&mut ap, b'o').map_err(|error| {
            format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] ioapic_resume: {error}")
        })?;
        let (completion_markers, completion_exits) = run_to_done(&mut ap)
            .map_err(|error| format!("[E_MACHINE_PLATFORM_SMP_PHASE_001] completion: {error}"))?;
        let ioapic_markers = "Oo";
        let ioapic_exits = 2_u64 + completion_exits;

        let bsp_id = lapic_read(&bsp.get_lapic()?, 0x020) >> 24;
        let ap_id = lapic_read(&ap.get_lapic()?, 0x020) >> 24;
        let bsp_state = bsp.get_mp_state()?.mp_state;
        let ap_state = ap.get_mp_state()?.mp_state;
        let passed = msi_delivered > 0
            && msi_markers == "Ii"
            && ioapic_markers == "Oo"
            && completion_markers.is_empty();
        if !passed {
            return Err(format!(
                "[E_MACHINE_PLATFORM_SMP_001] delivery mismatch: msi_result={msi_delivered}, msi={msi_markers:?}, ioapic={ioapic_markers:?}"
            )
            .into());
        }
        Ok(json!({
            "passed": true,
            "vcpu_count": 2,
            "bsp": {
                "id": bsp_id,
                "startup_markers": bsp_markers,
                "mp_state_after_halt": bsp_state,
                "kvm_exits": bsp_exits,
            },
            "application_processor": {
                "id": ap_id,
                "startup_markers": ap_start_markers,
                "mp_state_after_interrupts": ap_state,
                "startup_kvm_exits": ap_start_exits,
            },
            "msi": {
                "destination_apic_id": 1,
                "vector": 0x42,
                "kernel_delivery_result": msi_delivered,
                "markers": msi_markers,
                "kvm_exits": msi_exits,
            },
            "ioapic": {
                "pin": 16,
                "gsi": 16,
                "destination_apic_id": 1,
                "vector": 0x53,
                "redirection_entry": format!("{redirection_bits:#018x}"),
                "markers": ioapic_markers,
                "kvm_exits": ioapic_exits,
            },
            "in_kernel_irqchip": true,
            "directed_delivery": true,
            "bsp_ia32_apic_base_msr": format!("{bsp_apic_base_msr:#x}"),
            "ap_ia32_apic_base_msr": format!("{ap_apic_base_msr:#x}"),
        }))
    }

    fn resolve_existing(explicit: Option<&str>, candidates: &[&str]) -> Option<PathBuf> {
        explicit
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| {
                candidates
                    .iter()
                    .map(PathBuf::from)
                    .find(|path| path.is_file())
            })
    }

    pub(super) fn probe_qemu(explicit: Option<&str>) -> Value {
        let path = explicit
            .map(PathBuf::from)
            .or_else(|| which("qemu-system-x86_64"));
        json!({ "path": path, "available": path.as_ref().is_some_and(|path| path.is_file()) })
    }

    pub(super) fn probe_bios(explicit: Option<&str>) -> Value {
        let path = resolve_existing(
            explicit,
            &[
                "/usr/share/seabios/bios.bin",
                "/usr/share/seabios/bios-256k.bin",
            ],
        );
        json!({ "path": path, "available": path.is_some() })
    }

    pub(super) fn probe_uefi(explicit: Option<&str>) -> Value {
        let path = resolve_existing(
            explicit,
            &[
                "/usr/share/ovmf/OVMF.fd",
                "/usr/share/OVMF/OVMF_CODE_4M.fd",
                "/usr/share/edk2/x64/OVMF_CODE.fd",
            ],
        );
        json!({ "path": path, "available": path.is_some() })
    }

    fn which(name: &str) -> Option<PathBuf> {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|path| path.join(name))
                .find(|path| path.is_file())
        })
    }

    fn qemu_version(qemu: &Path) -> Result<String, Box<dyn Error>> {
        let output = Command::new(qemu).arg("--version").output()?;
        if !output.status.success() {
            return Err("[E_MACHINE_PLATFORM_QEMU_001] qemu --version failed".into());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string())
    }

    struct ProcessEvidence {
        status: i32,
        elapsed: Duration,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    }

    fn run_qemu(
        qemu: &Path,
        firmware: &Path,
        disk: &Path,
        boot_order: &str,
        debug_log: &Path,
    ) -> Result<ProcessEvidence, Box<dyn Error>> {
        let mut child = Command::new(qemu)
            .args([
                "-accel", "kvm", "-machine", "pc", "-cpu", "host", "-smp", "2", "-m", "64M",
                "-bios",
            ])
            .arg(firmware)
            .args(["-drive"])
            .arg(format!("file={},format=raw,if=ide", disk.display()))
            .args(["-boot"])
            .arg(format!("order={boot_order},strict=on"))
            .args([
                "-debugcon",
                &format!("file:{}", debug_log.display()),
                "-global",
                "isa-debugcon.iobase=0xe9",
                "-device",
                "isa-debug-exit,iobase=0xf4,iosize=0x04",
                "-display",
                "none",
                "-serial",
                "none",
                "-monitor",
                "none",
                "-nic",
                "none",
                "-no-reboot",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let started = Instant::now();
        loop {
            if child.try_wait()?.is_some() {
                let output = child.wait_with_output()?;
                return Ok(ProcessEvidence {
                    status: output.status.code().unwrap_or(-1),
                    elapsed: started.elapsed(),
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            if started.elapsed() > RUN_TIMEOUT {
                child.kill()?;
                let output = child.wait_with_output()?;
                return Err(format!(
                    "[E_MACHINE_PLATFORM_QEMU_TIMEOUT_001] QEMU exceeded {} ms: {}",
                    RUN_TIMEOUT.as_millis(),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn output_code(marker: &[u8], exit_value: u32, mode64: bool) -> Vec<u8> {
        let mut code = Vec::new();
        if mode64 {
            code.extend_from_slice(&[0x66, 0xba, 0xe9, 0x00]);
        } else {
            code.extend_from_slice(&[0xba, 0xe9, 0x00]);
        }
        for byte in marker {
            code.extend_from_slice(&[0xb0, *byte, 0xee]);
        }
        if mode64 {
            code.extend_from_slice(&[0x66, 0xba, 0xf4, 0x00]);
            code.push(0xb8);
            code.extend_from_slice(&exit_value.to_le_bytes());
            code.extend_from_slice(&[0xef, 0x31, 0xc0, 0xc3]);
        } else {
            code.extend_from_slice(&[0xba, 0xf4, 0x00, 0xb8]);
            code.extend_from_slice(&(exit_value as u16).to_le_bytes());
            code.extend_from_slice(&[0xef, 0xfa, 0xf4, 0xeb, 0xfe]);
        }
        code
    }

    fn certify_firmware(bundle: &Path, args: &[String]) -> Result<Value, Box<dyn Error>> {
        let qemu = option_value(args, "--qemu")
            .map(PathBuf::from)
            .or_else(|| which("qemu-system-x86_64"))
            .ok_or("[E_MACHINE_PLATFORM_QEMU_001] qemu-system-x86_64 not found")?;
        let bios = resolve_existing(
            option_value(args, "--bios"),
            &[
                "/usr/share/seabios/bios.bin",
                "/usr/share/seabios/bios-256k.bin",
            ],
        )
        .ok_or("[E_MACHINE_PLATFORM_BIOS_001] SeaBIOS firmware not found")?;
        let uefi = resolve_existing(
            option_value(args, "--uefi"),
            &[
                "/usr/share/ovmf/OVMF.fd",
                "/usr/share/OVMF/OVMF_CODE_4M.fd",
                "/usr/share/edk2/x64/OVMF_CODE.fd",
            ],
        )
        .ok_or("[E_MACHINE_PLATFORM_UEFI_001] OVMF firmware not found")?;

        let bios_code = output_code(b"NCCBIOS", 0x10, false);
        let boot_sector = build_bios_boot_sector(&bios_code)?;
        let bios_disk = build_bios_disk_image(&boot_sector)?;
        let bios_disk_path = bundle.join("bios-guest.img");
        let bios_log_path = bundle.join("bios-debugcon.log");
        fs::write(&bios_disk_path, &bios_disk)?;
        let bios_plan = detect_guest_image(&boot_sector)?;
        let bios_descriptor = GuestFirmwareDescriptor::parse_bios(&fs::read(&bios)?)?;
        let bios_run = run_qemu(&qemu, &bios, &bios_disk_path, "c", &bios_log_path)?;
        fs::write(bundle.join("bios-qemu.stdout.log"), &bios_run.stdout)?;
        fs::write(bundle.join("bios-qemu.stderr.log"), &bios_run.stderr)?;
        let bios_markers = fs::read_to_string(&bios_log_path).unwrap_or_default();
        if bios_run.status != 33 || !bios_markers.contains("NCCBIOS") {
            return Err(format!(
                "[E_MACHINE_PLATFORM_BIOS_001] SeaBIOS boot failed: status={} markers={bios_markers:?} stderr={}",
                bios_run.status,
                String::from_utf8_lossy(&bios_run.stderr)
            )
            .into());
        }

        let efi_code = output_code(b"NCCUEFI", 0x20, true);
        let efi_app = build_x86_64_uefi_application(&efi_code)?;
        let uefi_disk = build_uefi_fat12_disk(&efi_app)?;
        let efi_path = bundle.join("BOOTX64.EFI");
        let uefi_disk_path = bundle.join("uefi-guest-fat12.img");
        let uefi_log_path = bundle.join("uefi-debugcon.log");
        fs::write(&efi_path, &efi_app)?;
        fs::write(&uefi_disk_path, &uefi_disk)?;
        let efi_plan = detect_guest_image(&efi_app)?;
        let uefi_descriptor = GuestFirmwareDescriptor::parse_uefi(&fs::read(&uefi)?)?;
        let uefi_run = run_qemu(&qemu, &uefi, &uefi_disk_path, "c", &uefi_log_path)?;
        fs::write(bundle.join("uefi-qemu.stdout.log"), &uefi_run.stdout)?;
        fs::write(bundle.join("uefi-qemu.stderr.log"), &uefi_run.stderr)?;
        let uefi_markers = fs::read_to_string(&uefi_log_path).unwrap_or_default();
        if uefi_run.status != 65 || !uefi_markers.contains("NCCUEFI") {
            return Err(format!(
                "[E_MACHINE_PLATFORM_UEFI_001] OVMF boot failed: status={} markers={uefi_markers:?} stderr={}",
                uefi_run.status,
                String::from_utf8_lossy(&uefi_run.stderr)
            )
            .into());
        }

        Ok(json!({
            "qemu": {
                "path": qemu,
                "version": qemu_version(&qemu)?,
                "accelerator": "kvm",
                "cpu": "host",
                "vcpus": 2,
                "tcg": false,
            },
            "bios": {
                "passed": true,
                "firmware": bios,
                "firmware_kind": format!("{:?}", bios_descriptor.kind),
                "firmware_bytes": bios_descriptor.image_bytes,
                "reset_vector": format!("{:#x}", bios_descriptor.reset_vector),
                "image_kind": format!("{bios_plan:?}"),
                "markers": bios_markers,
                "exit_status": bios_run.status,
                "expected_exit_status": 33,
                "elapsed_milliseconds": bios_run.elapsed.as_millis(),
            },
            "uefi": {
                "passed": true,
                "firmware": uefi,
                "firmware_kind": format!("{:?}", uefi_descriptor.kind),
                "firmware_bytes": uefi_descriptor.image_bytes,
                "firmware_volume_offset": uefi_descriptor.firmware_volume_offset,
                "image_kind": format!("{efi_plan:?}"),
                "removable_boot_path": "/EFI/BOOT/BOOTX64.EFI",
                "markers": uefi_markers,
                "exit_status": uefi_run.status,
                "expected_exit_status": 65,
                "elapsed_milliseconds": uefi_run.elapsed.as_millis(),
            },
        }))
    }

    pub(super) fn available() -> bool {
        Path::new("/dev/kvm").exists()
            && which("qemu-system-x86_64").is_some()
            && resolve_existing(None, &["/usr/share/seabios/bios.bin"]).is_some()
            && resolve_existing(
                None,
                &["/usr/share/ovmf/OVMF.fd", "/usr/share/OVMF/OVMF_CODE_4M.fd"],
            )
            .is_some()
    }

    pub(super) fn execute_direct() -> Result<Value, Box<dyn Error>> {
        let _signal_guard = SignalGuard::install()?;
        let kvm = Kvm::new().map_err(|error| {
            format!("[E_MACHINE_PLATFORM_KVM_001] cannot open /dev/kvm: {error}")
        })?;
        for capability in [
            Cap::UserMemory,
            Cap::Irqchip,
            Cap::Pit2,
            Cap::MpState,
            Cap::SignalMsi,
        ] {
            if !kvm.check_extension(capability) {
                return Err(format!(
                    "[E_MACHINE_PLATFORM_KVM_CAPABILITY_001] missing KVM capability {capability:?}"
                )
                .into());
            }
        }
        let lapic_timer = certify_lapic_timer(&kvm)
            .map_err(|error| format!("[E_MACHINE_PLATFORM_PHASE_001] lapic_timer: {error}"))?;
        let pit_timer = certify_pit_timer(&kvm)
            .map_err(|error| format!("[E_MACHINE_PLATFORM_PHASE_001] pit_timer: {error}"))?;
        let smp = certify_smp(&kvm)
            .map_err(|error| format!("[E_MACHINE_PLATFORM_PHASE_001] smp: {error}"))?;
        let kvm_evidence = json!({
            "api_version": kvm.get_api_version(),
            "lapic_timer": lapic_timer,
            "pit_timer": pit_timer,
            "smp": smp,
        });
        Ok(kvm_evidence)
    }

    pub(super) fn execute(bundle: &Path, args: &[String]) -> Result<Value, Box<dyn Error>> {
        let kvm_evidence = execute_direct()?;
        let firmware = certify_firmware(bundle, args)
            .map_err(|error| format!("[E_MACHINE_PLATFORM_PHASE_001] firmware: {error}"))?;
        Ok(json!({
            "kvm": kvm_evidence,
            "firmware": firmware,
            "deadline_milliseconds": RUN_TIMEOUT.as_millis(),
        }))
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod platform_host {
    use super::*;

    pub(super) fn available() -> bool {
        false
    }

    pub(super) fn probe_qemu(_explicit: Option<&str>) -> Value {
        json!({ "path": null, "available": false })
    }

    pub(super) fn probe_bios(_explicit: Option<&str>) -> Value {
        json!({ "path": null, "available": false })
    }

    pub(super) fn probe_uefi(_explicit: Option<&str>) -> Value {
        json!({ "path": null, "available": false })
    }

    pub(super) fn execute(_bundle: &Path, _args: &[String]) -> Result<Value, Box<dyn Error>> {
        Err("[E_MACHINE_PLATFORM_HOST_001] platform certification requires the x86_64 Linux ncc binary, /dev/kvm, QEMU, SeaBIOS and OVMF".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_attestation_changes_when_evidence_changes() {
        let mut payload = json!({
            "schema": PLATFORM_CERTIFICATE_SCHEMA,
            "truth_boundary": { "cpu_backed_two_vcpu_delivery": true },
        });
        let before = payload_sha256(&payload).unwrap();
        payload["truth_boundary"]["cpu_backed_two_vcpu_delivery"] = json!(false);
        assert_ne!(before, payload_sha256(&payload).unwrap());
    }

    #[test]
    fn default_bundle_is_derived_from_certificate_name() {
        assert!(
            default_bundle_path(Some("reports/platform.json")).ends_with("reports/platform-bundle")
        );
    }
}
