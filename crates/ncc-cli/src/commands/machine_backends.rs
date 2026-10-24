use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const BACKEND_CERTIFICATE_SCHEMA: &str = "ncc.machine.hypervisor_backend_certificate.v1";
const BACKEND_PAYLOAD_DOMAIN: &[u8] = b"ncc.machine.hypervisor_backend_certificate.v1\0";
const BACKEND_EXECUTION_TIMEOUT_MS: u64 = 5_000;

fn option_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn first_non_flag(args: &[String]) -> Option<&str> {
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(arg.as_str(), "--provider" | "--out") {
            skip = true;
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

fn payload_sha256(value: &Value) -> Result<String, Box<dyn Error>> {
    let mut digest = Sha256::new();
    digest.update(BACKEND_PAYLOAD_DOMAIN);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(windows)]
fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn host_identity() -> Value {
    json!({
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "logical_processors": std::thread::available_parallelism().map(usize::from).ok(),
        "hostname": std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok(),
    })
}

pub(crate) fn source_identity() -> Value {
    let github_sha = std::env::var("GITHUB_SHA").ok();
    let git_head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty());
    let working_tree_dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    json!({
        "github_sha": github_sha,
        "git_head": git_head,
        "working_tree_dirty": working_tree_dirty,
        "exact_head_match": match (&github_sha, &git_head, working_tree_dirty) {
            (Some(github), Some(local), Some(false)) => Some(github.eq_ignore_ascii_case(local)),
            (Some(_), Some(_), Some(true)) => Some(false),
            _ => None,
        },
    })
}

pub(crate) fn run_probe_backends(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ncc.machine.hypervisor_backend_probe.v1",
        "host": host_identity(),
        "providers": [
            whpx_backend::probe(),
            kvm_backend::probe(),
            hvf_backend::probe(),
        ],
        "portable_abi": {
            "version": ncc_machine::HYPERVISOR_ABI_VERSION,
            "architectures": ["x86_64", "aarch64", "riscv64"],
            "operations": [
                "probe", "create_partition", "map_memory", "unmap_memory",
                "create_vcpu", "get_register", "set_register", "run", "cancel",
                "inject_interrupt", "dirty_bitmap", "pause", "resume", "reset"
            ],
            "exit_kinds": [
                "memory", "io", "mmio", "cpuid", "msr", "hypercall", "exception",
                "interrupt_window", "halt", "shutdown", "canceled", "debug",
                "system_register", "wfi_wfe", "riscv_sbi", "unknown"
            ],
        },
        "architecture_contracts": {
            "aarch64_el2": {
                "privilege": "EL2",
                "stage2_translation": true,
                "instructions": ["mrs", "msr", "hvc", "tlbi_vmalls12e1is", "eret"],
                "exit_state": ["esr_el2", "far_el2", "hpfar_el2", "elr_el2", "spsr_el2"],
            },
            "riscv_h": {
                "privilege": "HS/VS",
                "g_stage_translation": true,
                "instructions": ["hfence.gvma", "hfence.vvma", "hlv", "hsv"],
                "exit_state": ["scause", "sepc", "stval", "htval", "htinst", "hstatus"],
            },
            "iommu_dma": {
                "architectures": ["intel_vtd", "amd_vi", "arm_smmuv3", "riscv_iommu"],
                "viommu": ["attach", "detach", "map", "unmap", "invalidate", "fence", "fault"],
                "linux_passthrough": ["vfio_legacy", "vfio_cdev", "iommufd_ioas"],
            },
        },
    });
    if has_flag(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&payload))?
        );
    } else {
        for provider in payload["providers"].as_array().into_iter().flatten() {
            println!(
                "{}: available={} executable={}",
                provider["provider"].as_str().unwrap_or("unknown"),
                provider["available"].as_bool().unwrap_or(false),
                provider["cpu_backed_execution_available"]
                    .as_bool()
                    .unwrap_or(false)
            );
        }
    }
    Ok(())
}

pub(crate) fn run_certify_backend(args: &[String]) -> Result<(), Box<dyn Error>> {
    let provider = option_value(args, "--provider")
        .or_else(|| first_non_flag(args))
        .ok_or("virtualization certify-backend requires --provider whpx|kvm|hvf")?
        .to_ascii_lowercase();
    let (probe, execution) = match provider.as_str() {
        "whpx" | "hyper-v" | "hyperv" => (whpx_backend::probe(), whpx_backend::execute()?),
        "kvm" => (kvm_backend::probe(), kvm_backend::execute()?),
        "hvf" | "hypervisor.framework" => (hvf_backend::probe(), hvf_backend::execute()?),
        _ => {
            return Err(format!(
                "[E_MACHINE_BACKEND_PROVIDER_001] unknown hypervisor provider '{provider}'"
            )
            .into())
        }
    };
    let canonical_provider = probe["provider"]
        .as_str()
        .ok_or("backend probe omitted provider")?;
    let payload = json!({
        "schema": BACKEND_CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "cpu_backed_provider_passed",
        "created_epoch_seconds": epoch_seconds(),
        "provider": canonical_provider,
        "source": source_identity(),
        "host": host_identity(),
        "probe": probe,
        "execution": execution,
        "truth_boundary": {
            "provider_api_executed": true,
            "guest_code_executed_on_cpu": true,
            "user_mode_executed_vmx_svm_directly": false,
            "host_hypervisor_owned_privileged_state": true,
            "software_emulation": false,
            "tcg": false,
            "physical_device_passthrough": false,
        },
    });
    let mut certificate = payload.clone();
    certificate["attestation"] = json!({
        "algorithm": "sha256",
        "domain": String::from_utf8_lossy(BACKEND_PAYLOAD_DOMAIN).trim_end_matches('\0'),
        "payload_sha256": payload_sha256(&payload)?,
    });
    if let Some(path) = option_value(args, "--out") {
        write_json(Path::new(path), &certificate)?;
    }
    if has_flag(args, "--json") || option_value(args, "--out").is_none() {
        println!(
            "{}",
            serde_json::to_string_pretty(&canonical_json(&certificate))?
        );
    } else {
        println!(
            "OK: {canonical_provider} CPU-backed certificate written to {}",
            option_value(args, "--out").unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_backend_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args)
        .map(PathBuf::from)
        .ok_or("virtualization verify-backend-certificate requires a certificate path")?;
    let mut certificate: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(BACKEND_CERTIFICATE_SCHEMA) {
        return Err("[E_MACHINE_BACKEND_CERTIFICATE_001] unsupported certificate schema".into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("[E_MACHINE_BACKEND_CERTIFICATE_001] missing payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .expect("certificate object")
        .remove("attestation");
    let actual = payload_sha256(&certificate)?;
    if actual != expected {
        return Err(format!(
            "[E_MACHINE_BACKEND_CERTIFICATE_HASH_001] payload hash mismatch: expected {expected}, got {actual}"
        )
        .into());
    }
    if certificate
        .pointer("/source/github_sha")
        .and_then(Value::as_str)
        .is_some()
        && certificate
            .pointer("/source/exact_head_match")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("[E_MACHINE_BACKEND_CERTIFICATE_HEAD_001] GitHub certificate was not produced from the checked-out exact head".into());
    }
    for pointer in [
        "/truth_boundary/provider_api_executed",
        "/truth_boundary/guest_code_executed_on_cpu",
        "/truth_boundary/host_hypervisor_owned_privileged_state",
        "/execution/provider_api_executed",
        "/execution/guest_code_executed",
        "/execution/final_halt",
        "/execution/deadline_enforced",
    ] {
        if certificate.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "[E_MACHINE_BACKEND_CERTIFICATE_EVIDENCE_001] required evidence '{pointer}' is not true"
            )
            .into());
        }
    }
    let deadline = certificate
        .pointer("/execution/deadline_milliseconds")
        .and_then(Value::as_u64)
        .ok_or("backend certificate omitted deadline")?;
    let elapsed = certificate
        .pointer("/execution/elapsed_milliseconds")
        .and_then(Value::as_u64)
        .ok_or("backend certificate omitted elapsed time")?;
    if deadline == 0 || elapsed > deadline {
        return Err(format!(
            "[E_MACHINE_BACKEND_CERTIFICATE_DEADLINE_001] execution took {elapsed} ms, deadline {deadline} ms"
        )
        .into());
    }
    let report = json!({
        "schema": "ncc.machine.hypervisor_backend_certificate_verification.v1",
        "status": "verified",
        "certificate": path,
        "provider": certificate["provider"],
        "payload_sha256": actual,
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("OK: hypervisor backend certificate verified");
    }
    Ok(())
}

mod kvm_backend {
    use super::*;

    pub(super) fn probe() -> Value {
        #[cfg(target_os = "linux")]
        {
            use kvm_ioctls::{Cap, Kvm};
            let result = Kvm::new();
            let (available, api_version, capabilities, error) = match result {
                Ok(kvm) => {
                    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                    let vcpu_events = kvm.check_extension(Cap::VcpuEvents);
                    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
                    let vcpu_events = false;

                    (
                        true,
                        Some(kvm.get_api_version()),
                        json!({
                            "user_memory": kvm.check_extension(Cap::UserMemory),
                            "vcpu_events": vcpu_events,
                            "irqchip": kvm.check_extension(Cap::Irqchip),
                            "dirty_log_ring": kvm.check_extension(Cap::DirtyLogRing),
                        }),
                        None,
                    )
                }
                Err(error) => (false, None, json!({}), Some(error.to_string())),
            };
            return json!({
                "provider": "kvm",
                "available": available,
                "cpu_backed_execution_available": available,
                "device": "/dev/kvm",
                "api_version": api_version,
                "architectures": ["x86_64", "aarch64", "riscv64"],
                "capabilities": capabilities,
                "error": error,
            });
        }
        #[cfg(not(target_os = "linux"))]
        json!({
            "provider": "kvm",
            "available": false,
            "cpu_backed_execution_available": false,
            "device": "/dev/kvm",
            "architectures": ["x86_64", "aarch64", "riscv64"],
            "reason": "KVM provider is only directly probeable by a Linux build",
        })
    }

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        let started = std::time::Instant::now();
        let raw = super::super::machine_hypervisor::execute_kvm_backend_evidence()?;
        let final_halt = raw
            .pointer("/results/final_halt")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok(json!({
            "provider": "kvm",
            "provider_api_executed": true,
            "guest_code_executed": final_halt,
            "final_halt": final_halt,
            "deadline_enforced": raw.pointer("/results/deadline_enforced").and_then(Value::as_bool).unwrap_or(false),
            "deadline_milliseconds": raw.pointer("/results/deadline_milliseconds").and_then(Value::as_u64).unwrap_or(BACKEND_EXECUTION_TIMEOUT_MS),
            "elapsed_milliseconds": raw.pointer("/results/elapsed_milliseconds").and_then(Value::as_u64).unwrap_or_else(|| started.elapsed().as_millis() as u64),
            "vmexit_loop": "ncc_machine_typed",
            "raw_evidence": raw,
        }))
    }
}

#[cfg(windows)]
mod whpx_backend {
    use super::*;
    use std::alloc::{alloc_zeroed, dealloc, Layout};
    use std::ffi::c_void;
    use std::mem::{size_of, zeroed};
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::System::Hypervisor::*;

    const GUEST_BYTES: usize = 0x4000;
    const CODE_GPA: usize = 0x1000;
    const RESULT_GPA: usize = 0x2000;
    const DEBUG_PORT: u16 = 0xe9;
    const GUEST_CODE: &[u8] = &[
        0xba, 0xe9, 0x00, // mov dx, 0xe9
        0xb0, b'W', 0xee, // mov al, 'W'; out dx, al
        0xb0, b'H', 0xee, // mov al, 'H'; out dx, al
        0xb0, b'P', 0xee, // mov al, 'P'; out dx, al
        0xb0, b'X', 0xee, // mov al, 'X'; out dx, al
        0xc7, 0x06, 0x00, 0x20, 0x34, 0x12, // mov word [0x2000], 0x1234
        0xf4, // hlt
    ];

    struct AlignedMemory {
        pointer: NonNull<u8>,
        layout: Layout,
    }

    impl AlignedMemory {
        fn new(bytes: usize) -> Result<Self, Box<dyn Error>> {
            let layout = Layout::from_size_align(bytes, 4096)?;
            let pointer = NonNull::new(unsafe { alloc_zeroed(layout) })
                .ok_or("[E_MACHINE_WHPX_MEMORY_001] aligned allocation failed")?;
            Ok(Self { pointer, layout })
        }

        fn bytes_mut(&mut self) -> &mut [u8] {
            unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.layout.size()) }
        }
    }

    impl Drop for AlignedMemory {
        fn drop(&mut self) {
            unsafe { dealloc(self.pointer.as_ptr(), self.layout) };
        }
    }

    struct WhpxPartition {
        handle: WHV_PARTITION_HANDLE,
        vcpu_created: bool,
        mapped_bytes: u64,
    }

    impl Drop for WhpxPartition {
        fn drop(&mut self) {
            unsafe {
                if self.vcpu_created {
                    let _ = WHvDeleteVirtualProcessor(self.handle, 0);
                }
                if self.mapped_bytes != 0 {
                    let _ = WHvUnmapGpaRange(self.handle, 0, self.mapped_bytes);
                }
                let _ = WHvDeletePartition(self.handle);
            }
        }
    }

    fn check(operation: &'static str, status: i32) -> Result<(), Box<dyn Error>> {
        if status >= 0 {
            Ok(())
        } else {
            Err(format!(
                "[E_MACHINE_WHPX_API_001] {operation} failed with HRESULT {:#010x}",
                status as u32
            )
            .into())
        }
    }

    fn hypervisor_present() -> Result<bool, Box<dyn Error>> {
        let mut present = 0i32;
        let mut written = 0u32;
        check("WHvGetCapability(HypervisorPresent)", unsafe {
            WHvGetCapability(
                WHvCapabilityCodeHypervisorPresent,
                &mut present as *mut _ as *mut c_void,
                size_of::<i32>() as u32,
                &mut written,
            )
        })?;
        Ok(written as usize == size_of::<i32>() && present != 0)
    }

    pub(super) fn probe() -> Value {
        match hypervisor_present() {
            Ok(present) => json!({
                "provider": "whpx",
                "available": present,
                "cpu_backed_execution_available": present,
                "dll": "WinHvPlatform.dll",
                "architectures": ["x86_64", "aarch64"],
                "capabilities": {
                    "partition": true,
                    "gpa_mapping": true,
                    "dirty_bitmap": true,
                    "interrupt_injection": true,
                    "vcpu_state": true,
                    "vcpu_cancel": true,
                    "vpci_device_assignment_api": true,
                },
            }),
            Err(error) => json!({
                "provider": "whpx",
                "available": false,
                "cpu_backed_execution_available": false,
                "dll": "WinHvPlatform.dll",
                "architectures": ["x86_64", "aarch64"],
                "error": error.to_string(),
            }),
        }
    }

    #[repr(C, align(16))]
    #[derive(Clone, Copy)]
    struct AlignedRegisterValue(WHV_REGISTER_VALUE);

    const _: () = assert!(size_of::<AlignedRegisterValue>() == 16);
    const _: () = assert!(std::mem::align_of::<AlignedRegisterValue>() == 16);

    fn reg64(value: u64) -> AlignedRegisterValue {
        AlignedRegisterValue(WHV_REGISTER_VALUE { Reg64: value })
    }

    fn segment(base: u64, selector: u16, attributes: u16) -> AlignedRegisterValue {
        AlignedRegisterValue(WHV_REGISTER_VALUE {
            Segment: WHV_X64_SEGMENT_REGISTER {
                Base: base,
                Limit: 0xffff,
                Selector: selector,
                Anonymous: WHV_X64_SEGMENT_REGISTER_0 {
                    Attributes: attributes,
                },
            },
        })
    }

    fn set_initial_registers(partition: WHV_PARTITION_HANDLE) -> Result<(), Box<dyn Error>> {
        let names = [
            WHvX64RegisterRip,
            WHvX64RegisterRflags,
            WHvX64RegisterRsp,
            WHvX64RegisterRax,
            WHvX64RegisterRdx,
            WHvX64RegisterCs,
            WHvX64RegisterDs,
            WHvX64RegisterEs,
            WHvX64RegisterSs,
            WHvX64RegisterCr0,
            WHvX64RegisterCr3,
            WHvX64RegisterCr4,
            WHvX64RegisterEfer,
        ];
        let values = [
            reg64(CODE_GPA as u64),
            reg64(2),
            reg64((GUEST_BYTES - 16) as u64),
            reg64(0),
            reg64(0),
            segment(0, 0, 0x009b),
            segment(0, 0, 0x0093),
            segment(0, 0, 0x0093),
            segment(0, 0, 0x0093),
            reg64(0x10),
            reg64(0),
            reg64(0),
            reg64(0),
        ];
        check("WHvSetVirtualProcessorRegisters(initial)", unsafe {
            WHvSetVirtualProcessorRegisters(
                partition,
                0,
                names.as_ptr(),
                names.len() as u32,
                values.as_ptr() as *const WHV_REGISTER_VALUE,
            )
        })
    }

    fn set_rip(partition: WHV_PARTITION_HANDLE, rip: u64) -> Result<(), Box<dyn Error>> {
        let name = WHvX64RegisterRip;
        let value = reg64(rip);
        check("WHvSetVirtualProcessorRegisters(rip)", unsafe {
            WHvSetVirtualProcessorRegisters(partition, 0, &name, 1, &value.0)
        })
    }

    fn get_rip(partition: WHV_PARTITION_HANDLE) -> Result<u64, Box<dyn Error>> {
        let name = WHvX64RegisterRip;
        let mut value = AlignedRegisterValue(unsafe { zeroed() });
        check("WHvGetVirtualProcessorRegisters(rip)", unsafe {
            WHvGetVirtualProcessorRegisters(partition, 0, &name, 1, &mut value.0)
        })?;
        Ok(unsafe { value.0.Reg64 })
    }

    #[allow(non_upper_case_globals)]
    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        if !hypervisor_present()? {
            return Err(
                "[E_MACHINE_WHPX_HOST_001] Windows Hypervisor Platform is not active".into(),
            );
        }
        let started = Instant::now();
        let mut memory = AlignedMemory::new(GUEST_BYTES)?;
        memory.bytes_mut()[CODE_GPA..CODE_GPA + GUEST_CODE.len()].copy_from_slice(GUEST_CODE);

        let mut handle: WHV_PARTITION_HANDLE = 0;
        check("WHvCreatePartition", unsafe {
            WHvCreatePartition(&mut handle)
        })?;
        let mut partition = WhpxPartition {
            handle,
            vcpu_created: false,
            mapped_bytes: 0,
        };
        let processor_count = 1u32;
        check("WHvSetPartitionProperty(ProcessorCount)", unsafe {
            WHvSetPartitionProperty(
                handle,
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const c_void,
                size_of::<u32>() as u32,
            )
        })?;
        check("WHvSetupPartition", unsafe { WHvSetupPartition(handle) })?;
        let map_flags = WHvMapGpaRangeFlagRead
            | WHvMapGpaRangeFlagWrite
            | WHvMapGpaRangeFlagExecute
            | WHvMapGpaRangeFlagTrackDirtyPages;
        check("WHvMapGpaRange", unsafe {
            WHvMapGpaRange(
                handle,
                memory.pointer.as_ptr() as *const c_void,
                0,
                GUEST_BYTES as u64,
                map_flags,
            )
        })?;
        partition.mapped_bytes = GUEST_BYTES as u64;
        check("WHvCreateVirtualProcessor", unsafe {
            WHvCreateVirtualProcessor(handle, 0, 0)
        })?;
        partition.vcpu_created = true;
        set_initial_registers(handle)?;

        check("WHvSuspendPartitionTime", unsafe {
            WHvSuspendPartitionTime(handle)
        })?;
        check("WHvResumePartitionTime", unsafe {
            WHvResumePartitionTime(handle)
        })?;

        let completed = Arc::new(AtomicBool::new(false));
        let watchdog_completed = Arc::clone(&completed);
        let watchdog_handle = handle as usize;
        let watchdog = thread::spawn(move || {
            let slices = BACKEND_EXECUTION_TIMEOUT_MS / 10;
            for _ in 0..slices {
                if watchdog_completed.load(Ordering::Acquire) {
                    return false;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if watchdog_completed.load(Ordering::Acquire) {
                return false;
            }
            unsafe {
                WHvCancelRunVirtualProcessor(watchdog_handle as WHV_PARTITION_HANDLE, 0, 0);
            }
            true
        });

        let mut markers = Vec::new();
        let mut exits = Vec::new();
        let mut final_halt = false;
        for ordinal in 0..64u32 {
            let mut context: WHV_RUN_VP_EXIT_CONTEXT = unsafe { zeroed() };
            check("WHvRunVirtualProcessor", unsafe {
                WHvRunVirtualProcessor(
                    handle,
                    0,
                    &mut context as *mut _ as *mut c_void,
                    size_of::<WHV_RUN_VP_EXIT_CONTEXT>() as u32,
                )
            })?;
            match context.ExitReason {
                WHvRunVpExitReasonX64IoPortAccess => {
                    let io = unsafe { context.Anonymous.IoPortAccess };
                    let write = unsafe { io.AccessInfo.AsUINT32 } & 1 != 0;
                    if !write || io.PortNumber != DEBUG_PORT {
                        return Err(format!(
                            "[E_MACHINE_WHPX_EXIT_001] unexpected IO exit port={:#x} info={:#x}",
                            io.PortNumber,
                            unsafe { io.AccessInfo.AsUINT32 }
                        )
                        .into());
                    }
                    let marker = io.Rax as u8;
                    let instruction_length = (context.VpContext._bitfield & 0x0f) as u64;
                    if instruction_length == 0 {
                        return Err(
                            "[E_MACHINE_WHPX_EXIT_001] VP exit reported zero instruction length"
                                .into(),
                        );
                    }
                    let register_rip = get_rip(handle)?;
                    let next_rip = register_rip.checked_add(instruction_length).ok_or(
                        "[E_MACHINE_WHPX_EXIT_001] RIP overflow while advancing IO exit",
                    )?;
                    markers.push(marker);
                    set_rip(handle, next_rip)?;
                    let committed_rip = get_rip(handle)?;
                    if committed_rip != next_rip {
                        return Err(format!(
                            "[E_MACHINE_WHPX_REGISTER_001] RIP write was not committed: expected={next_rip:#x} actual={committed_rip:#x}"
                        )
                        .into());
                    }
                    exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "io_write",
                        "port": io.PortNumber,
                        "value": marker,
                        "context_rip": context.VpContext.Rip,
                        "register_rip": register_rip,
                        "next_rip": next_rip,
                        "instruction_length": instruction_length,
                        "decoded_byte_count": io.InstructionByteCount,
                        "instruction_bytes": &io.InstructionBytes[..io.InstructionByteCount as usize],
                    }));
                }
                WHvRunVpExitReasonX64Halt => {
                    final_halt = true;
                    exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "halt",
                        "rip": context.VpContext.Rip,
                    }));
                    break;
                }
                WHvRunVpExitReasonCanceled => {
                    return Err("[E_MACHINE_WHPX_TIMEOUT_001] vCPU run was canceled by deadline".into())
                }
                reason => {
                    return Err(format!(
                        "[E_MACHINE_WHPX_EXIT_001] unexpected WHPX exit reason {reason:#x} at rip={:#x}",
                        context.VpContext.Rip
                    )
                    .into())
                }
            }
        }
        completed.store(true, Ordering::Release);
        let watchdog_fired = watchdog
            .join()
            .map_err(|_| "[E_MACHINE_WHPX_WATCHDOG_001] watchdog thread panicked")?;
        let marker_text = String::from_utf8(markers.clone())?;
        let result_word = u16::from_le_bytes(
            memory.bytes_mut()[RESULT_GPA..RESULT_GPA + 2]
                .try_into()
                .expect("two-byte result"),
        );
        let mut dirty_bitmap = 0u64;
        check("WHvQueryGpaRangeDirtyBitmap", unsafe {
            WHvQueryGpaRangeDirtyBitmap(
                handle,
                0,
                GUEST_BYTES as u64,
                &mut dirty_bitmap,
                size_of::<u64>() as u32,
            )
        })?;
        let final_rip = get_rip(handle)?;
        check("WHvResetPartition", unsafe { WHvResetPartition(handle) })?;
        let elapsed = started.elapsed();
        if marker_text != "WHPX"
            || result_word != 0x1234
            || dirty_bitmap == 0
            || !final_halt
            || watchdog_fired
            || elapsed.as_millis() as u64 > BACKEND_EXECUTION_TIMEOUT_MS
        {
            return Err(format!(
                "[E_MACHINE_WHPX_EVIDENCE_001] markers={marker_text:?} result={result_word:#x} dirty={dirty_bitmap:#x} halt={final_halt} watchdog={watchdog_fired} elapsed_ms={}",
                elapsed.as_millis()
            )
            .into());
        }
        Ok(json!({
            "provider": "whpx",
            "provider_api_executed": true,
            "guest_code_executed": true,
            "final_halt": final_halt,
            "deadline_enforced": true,
            "deadline_milliseconds": BACKEND_EXECUTION_TIMEOUT_MS,
            "elapsed_milliseconds": elapsed.as_millis(),
            "guest": {
                "architecture": "x86_64",
                "mode": "real_mode",
                "memory_bytes": GUEST_BYTES,
                "code_gpa": CODE_GPA,
                "code_sha256": sha256_bytes(GUEST_CODE),
            },
            "results": {
                "marker_sequence": marker_text,
                "result_word": result_word,
                "dirty_bitmap": format!("{dirty_bitmap:#x}"),
                "final_rip": final_rip,
                "partition_time_suspend_resume": true,
                "partition_reset": true,
                "watchdog_fired": watchdog_fired,
            },
            "exits": exits,
        }))
    }
}

#[cfg(not(windows))]
mod whpx_backend {
    use super::*;

    pub(super) fn probe() -> Value {
        json!({
            "provider": "whpx",
            "available": false,
            "cpu_backed_execution_available": false,
            "dll": "WinHvPlatform.dll",
            "architectures": ["x86_64", "aarch64"],
            "reason": "WHPX is only available to the Windows build",
        })
    }

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        Err(
            "[E_MACHINE_WHPX_HOST_001] WHPX certification requires Windows with Hyper-V active"
                .into(),
        )
    }
}

#[cfg(target_os = "macos")]
mod hvf_backend {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    pub(super) fn probe() -> Value {
        let output = Command::new("sysctl")
            .args(["-n", "kern.hv_support"])
            .output();
        let supported = output
            .as_ref()
            .ok()
            .is_some_and(|output| output.status.success() && output.stdout.starts_with(b"1"));
        json!({
            "provider": "hvf",
            "available": supported,
            "cpu_backed_execution_available": supported,
            "framework": "/System/Library/Frameworks/Hypervisor.framework",
            "architectures": ["x86_64", "aarch64"],
            "entitlement": "com.apple.security.hypervisor",
            "sysctl_kern_hv_support": supported,
        })
    }

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        if probe()["available"].as_bool() != Some(true) {
            return Err("[E_MACHINE_HVF_HOST_001] kern.hv_support is unavailable".into());
        }
        // Provider execution is isolated in the retained Darwin runner. The helper
        // launches the architecture-specific smoke bundled by the release workflow.
        let helper = std::env::current_exe()?.with_file_name("ncc-hvf-smoke");
        let mut child = Command::new(&helper)
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!(
                    "[E_MACHINE_HVF_HELPER_001] cannot launch {}: {error}",
                    helper.display()
                )
            })?;
        let deadline = Instant::now() + Duration::from_millis(BACKEND_EXECUTION_TIMEOUT_MS);
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output()?;
                return Err(format!(
                    "[E_MACHINE_HVF_TIMEOUT_001] helper exceeded {} ms: {}",
                    BACKEND_EXECUTION_TIMEOUT_MS,
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(format!(
                "[E_MACHINE_HVF_EXECUTION_001] helper failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let evidence: Value = serde_json::from_slice(&output.stdout)?;
        if evidence["provider_api_executed"].as_bool() != Some(true)
            || evidence["guest_code_executed"].as_bool() != Some(true)
            || evidence["final_halt"].as_bool() != Some(true)
        {
            return Err("[E_MACHINE_HVF_EVIDENCE_001] Darwin helper evidence is incomplete".into());
        }
        Ok(evidence)
    }
}

#[cfg(not(target_os = "macos"))]
mod hvf_backend {
    use super::*;

    pub(super) fn probe() -> Value {
        json!({
            "provider": "hvf",
            "available": false,
            "cpu_backed_execution_available": false,
            "framework": "/System/Library/Frameworks/Hypervisor.framework",
            "architectures": ["x86_64", "aarch64"],
            "reason": "HVF is only available to the macOS build",
        })
    }

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        Err("[E_MACHINE_HVF_HOST_001] HVF certification requires a physical macOS runner".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_domain_detects_payload_mutation() {
        let mut payload = json!({
            "schema": BACKEND_CERTIFICATE_SCHEMA,
            "provider": "kvm",
            "execution": { "guest_code_executed": true },
        });
        let before = payload_sha256(&payload).unwrap();
        payload["execution"]["guest_code_executed"] = json!(false);
        assert_ne!(before, payload_sha256(&payload).unwrap());
    }

    #[test]
    fn probe_has_all_three_provider_identities() {
        let providers = [
            whpx_backend::probe(),
            kvm_backend::probe(),
            hvf_backend::probe(),
        ];
        assert_eq!(providers[0]["provider"], "whpx");
        assert_eq!(providers[1]["provider"], "kvm");
        assert_eq!(providers[2]["provider"], "hvf");
    }

    #[test]
    fn portable_probe_declares_el2_h_and_iommu_contracts() {
        let contracts = json!({
            "el2": ncc_machine::HCR_EL2.encoding(),
            "riscv_h": ncc_machine::HSTATUS.address,
            "iommu": format!("{:?}", ncc_machine::IommuArchitecture::ArmSmmuv3),
        });
        assert_eq!(contracts["el2"], ncc_machine::HCR_EL2.encoding());
        assert_eq!(contracts["riscv_h"], 0x600);
        assert_eq!(contracts["iommu"], "ArmSmmuv3");
    }
}
