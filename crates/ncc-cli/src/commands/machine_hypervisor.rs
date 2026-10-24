use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CERTIFICATE_SCHEMA: &str = "ncc.machine.x86_virtualization_hardware_certificate.v1";
const PAYLOAD_DOMAIN: &[u8] = b"ncc.machine.x86_virtualization_hardware_certificate.v1\0";
pub(crate) const KVM_CERTIFICATION_TIMEOUT_MS: u64 = 15_000;

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
        if matches!(arg.as_str(), "--out") {
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
    digest.update(PAYLOAD_DOMAIN);
    digest.update(serde_json::to_vec(&canonical_json(value))?);
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )
))]
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

fn host_probe() -> Value {
    #[cfg(target_arch = "x86_64")]
    let cpuid = {
        use std::arch::x86_64::__cpuid_count;
        let leaf0 = __cpuid_count(0, 0);
        let leaf1 = __cpuid_count(1, 0);
        let extended_max = __cpuid_count(0x8000_0000, 0).eax;
        let extended = (extended_max >= 0x8000_0001).then(|| __cpuid_count(0x8000_0001, 0));
        let brand = if extended_max >= 0x8000_0004 {
            let mut bytes = Vec::with_capacity(48);
            for leaf in 0x8000_0002..=0x8000_0004 {
                let row = __cpuid_count(leaf, 0);
                bytes.extend_from_slice(&row.eax.to_le_bytes());
                bytes.extend_from_slice(&row.ebx.to_le_bytes());
                bytes.extend_from_slice(&row.ecx.to_le_bytes());
                bytes.extend_from_slice(&row.edx.to_le_bytes());
            }
            String::from_utf8_lossy(&bytes)
                .trim_matches(['\0', ' '])
                .to_string()
        } else {
            String::new()
        };
        let hypervisor_vendor = if leaf1.ecx & (1 << 31) != 0 {
            let row = __cpuid_count(0x4000_0000, 0);
            let mut bytes = Vec::with_capacity(12);
            bytes.extend_from_slice(&row.ebx.to_le_bytes());
            bytes.extend_from_slice(&row.ecx.to_le_bytes());
            bytes.extend_from_slice(&row.edx.to_le_bytes());
            Some(
                String::from_utf8_lossy(&bytes)
                    .trim_end_matches('\0')
                    .to_string(),
            )
        } else {
            None
        };
        let base_family = (leaf1.eax >> 8) & 0xf;
        let family = base_family
            + if base_family == 0xf {
                (leaf1.eax >> 20) & 0xff
            } else {
                0
            };
        let base_model = (leaf1.eax >> 4) & 0xf;
        let model = base_model
            | if matches!(base_family, 0x6 | 0xf) {
                ((leaf1.eax >> 16) & 0xf) << 4
            } else {
                0
            };
        let mut vendor = Vec::with_capacity(12);
        vendor.extend_from_slice(&leaf0.ebx.to_le_bytes());
        vendor.extend_from_slice(&leaf0.edx.to_le_bytes());
        vendor.extend_from_slice(&leaf0.ecx.to_le_bytes());
        json!({
            "vendor": String::from_utf8_lossy(&vendor).trim_end_matches('\0'),
            "brand": brand,
            "family": family,
            "model": model,
            "stepping": leaf1.eax & 0xf,
            "vmx": leaf1.ecx & (1 << 5) != 0,
            "svm": extended.is_some_and(|leaf| leaf.ecx & (1 << 2) != 0),
            "hypervisor_present": leaf1.ecx & (1 << 31) != 0,
            "hypervisor_vendor": hypervisor_vendor,
        })
    };
    #[cfg(not(target_arch = "x86_64"))]
    let cpuid = json!({
        "vendor": "not-x86_64",
        "vmx": false,
        "svm": false,
        "hypervisor_present": false,
    });

    let read_trimmed = |path: &str| {
        fs::read_to_string(path)
            .ok()
            .map(|text| text.trim().to_string())
    };
    json!({
        "architecture": std::env::consts::ARCH,
        "operating_system": std::env::consts::OS,
        "cpuid": cpuid,
        "kernel_release": read_trimmed("/proc/sys/kernel/osrelease"),
        "kernel_version": read_trimmed("/proc/sys/kernel/version"),
        "nested_intel": read_trimmed("/sys/module/kvm_intel/parameters/nested"),
        "nested_amd": read_trimmed("/sys/module/kvm_amd/parameters/nested"),
        "dmi_product": read_trimmed("/sys/class/dmi/id/product_name"),
        "dmi_vendor": read_trimmed("/sys/class/dmi/id/sys_vendor"),
        "kvm_device": "/dev/kvm",
        "kvm_exists": Path::new("/dev/kvm").exists(),
        "logical_processors": std::thread::available_parallelism().map(usize::from).ok(),
    })
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux_kvm {
    use super::*;
    use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
    use kvm_ioctls::{Cap, Kvm, VcpuExit, VcpuFd};
    use ncc_machine::{
        run_vm_exit_loop, GuestPhysicalAddress, GuestRegisters, PendingVmEvents, VmEvent,
        VmEventKind, VmExecutionEngine, VmExit, VmExitAction, VmExitError, VmExitHandler,
        VmExitKind, VmLoopStopReason, VmVendor,
    };
    use std::ptr::null_mut;
    use std::slice;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    const GUEST_MEMORY_BYTES: usize = 0x4000;
    const CODE_ADDRESS: usize = 0x1000;
    const INVALID_OPCODE_HANDLER: usize = 0x1100;
    const INTERRUPT_HANDLER: usize = 0x1120;
    const RAM_WRITE_ADDRESS: usize = 0x1300;
    const MMIO_ADDRESS: u64 = 0x8000;
    const DEBUG_PORT: u16 = 0x00e9;
    const RECOVERY_CODE_ADDRESS: usize = 0x1000;
    const RECOVERY_TIMEOUT: Duration = Duration::from_millis(125);
    const KVM_RUN_TIMEOUT_SIGNAL: libc::c_int = libc::SIGUSR1;
    const CERTIFICATION_TIMEOUT: Duration = Duration::from_millis(KVM_CERTIFICATION_TIMEOUT_MS);
    const KVM_RUN_TIMEOUT_MESSAGE: &str = "KVM_RUN physical certification deadline exceeded";

    extern "C" fn kvm_run_timeout_handler(_signal: libc::c_int) {}

    struct KvmRunSignalGuard {
        previous: libc::sigaction,
        previous_mask: libc::sigset_t,
    }

    impl KvmRunSignalGuard {
        fn install() -> Result<Self, Box<dyn Error>> {
            let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
            let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
            let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };
            let mut previous_mask = unsafe { std::mem::zeroed::<libc::sigset_t>() };
            action.sa_sigaction = kvm_run_timeout_handler as *const () as usize;
            action.sa_flags = 0;
            unsafe {
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(KVM_RUN_TIMEOUT_SIGNAL, &action, &mut previous) != 0 {
                    return Err(format!(
                        "[E_MACHINE_KVM_SIGNAL_001] cannot install KVM_RUN timeout handler: {}",
                        std::io::Error::last_os_error()
                    )
                    .into());
                }
                libc::sigemptyset(&mut signal_set);
                libc::sigaddset(&mut signal_set, KVM_RUN_TIMEOUT_SIGNAL);
                if libc::pthread_sigmask(libc::SIG_UNBLOCK, &signal_set, &mut previous_mask) != 0 {
                    libc::sigaction(KVM_RUN_TIMEOUT_SIGNAL, &previous, std::ptr::null_mut());
                    return Err(
                        "[E_MACHINE_KVM_SIGNAL_001] cannot unblock KVM_RUN timeout signal".into(),
                    );
                }
            }
            Ok(Self {
                previous,
                previous_mask,
            })
        }
    }

    impl Drop for KvmRunSignalGuard {
        fn drop(&mut self) {
            unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.previous_mask, std::ptr::null_mut());
                libc::sigaction(KVM_RUN_TIMEOUT_SIGNAL, &self.previous, std::ptr::null_mut());
            }
        }
    }

    const GUEST_CODE: &[u8] = &[
        0xba, 0xe9, 0x00, // mov dx, 0xe9
        0xb0, b'S', // mov al, 'S'
        0xee, // out dx, al
        0x0f, 0x0b, // ud2
        0xb0, b'X', // mov al, 'X'
        0xee, // out dx, al
        0xfb, // sti
        0xf4, // hlt: host injects vector 0x20
        0xc7, 0x06, 0x00, 0x13, 0x34, 0x12, // mov word [0x1300], 0x1234
        0xc6, 0x06, 0x00, 0x80, 0x5a, // mov byte [0x8000], 0x5a
        0xa0, 0x00, 0x80, // mov al, [0x8000]
        0x3c, 0xa5, // cmp al, 0xa5
        0x75, 0x09, // jne failure
        0xb0, b'M', // mov al, 'M'
        0xee, // out dx, al
        0xb0, b'O', // mov al, 'O'
        0xee, // out dx, al
        0xf4, // final hlt
        0xeb, 0xfe, // loop if spuriously resumed
        0xb0, b'F', // failure: mov al, 'F'
        0xee, // out dx, al
        0xf4, // hlt
        0xeb, 0xfe, // loop
    ];

    const INVALID_OPCODE_HANDLER_CODE: &[u8] = &[
        0x55, // push bp
        0x89, 0xe5, // mov bp, sp
        0x83, 0x46, 0x02, 0x02, // add word [bp+2], 2
        0x50, // push ax
        0x52, // push dx
        0xba, 0xe9, 0x00, // mov dx, 0xe9
        0xb0, b'E', // mov al, 'E'
        0xee, // out dx, al
        0x5a, // pop dx
        0x58, // pop ax
        0x5d, // pop bp
        0xcf, // iret
    ];

    const INTERRUPT_HANDLER_CODE: &[u8] = &[
        0x50, // push ax
        0x52, // push dx
        0xba, 0xe9, 0x00, // mov dx, 0xe9
        0xb0, b'I', // mov al, 'I'
        0xee, // out dx, al
        0x5a, // pop dx
        0x58, // pop ax
        0xcf, // iret
    ];

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
                    "[E_MACHINE_KVM_MEMORY_001] mmap failed: {}",
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

    fn write_ivt(memory: &mut [u8], vector: u8, offset: u16) {
        let entry = vector as usize * 4;
        memory[entry..entry + 2].copy_from_slice(&offset.to_le_bytes());
        memory[entry + 2..entry + 4].copy_from_slice(&0_u16.to_le_bytes());
    }

    fn write_guest(memory: &mut [u8]) {
        memory[CODE_ADDRESS..CODE_ADDRESS + GUEST_CODE.len()].copy_from_slice(GUEST_CODE);
        memory[INVALID_OPCODE_HANDLER..INVALID_OPCODE_HANDLER + INVALID_OPCODE_HANDLER_CODE.len()]
            .copy_from_slice(INVALID_OPCODE_HANDLER_CODE);
        memory[INTERRUPT_HANDLER..INTERRUPT_HANDLER + INTERRUPT_HANDLER_CODE.len()]
            .copy_from_slice(INTERRUPT_HANDLER_CODE);
        write_ivt(memory, 6, INVALID_OPCODE_HANDLER as u16);
        write_ivt(memory, 0x20, INTERRUPT_HANDLER as u16);
    }

    fn configure_real_mode_vcpu(
        vcpu: &VcpuFd,
        rip: usize,
        stack: usize,
    ) -> Result<(), Box<dyn Error>> {
        let mut special = vcpu.get_sregs()?;
        special.cs.base = 0;
        special.cs.selector = 0;
        special.ds.base = 0;
        special.ds.selector = 0;
        special.es.base = 0;
        special.es.selector = 0;
        special.ss.base = 0;
        special.ss.selector = 0;
        vcpu.set_sregs(&special)?;
        let mut registers = vcpu.get_regs()?;
        registers.rip = rip as u64;
        registers.rsp = stack as u64;
        registers.rflags = 2;
        vcpu.set_regs(&registers)?;
        Ok(())
    }

    fn certify_blocked_run_recovery(kvm: &Kvm) -> Result<Value, Box<dyn Error>> {
        let vm = kvm.create_vm()?;
        let mut memory = GuestMemory::new(GUEST_MEMORY_BYTES)?;
        memory.bytes_mut()[RECOVERY_CODE_ADDRESS..RECOVERY_CODE_ADDRESS + 2]
            .copy_from_slice(&[0xeb, 0xfe]);
        let region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: 0,
            memory_size: GUEST_MEMORY_BYTES as u64,
            userspace_addr: memory.address(),
        };
        unsafe { vm.set_user_memory_region(region) }?;
        let mut vcpu = vm.create_vcpu(0)?;
        configure_real_mode_vcpu(&vcpu, RECOVERY_CODE_ADDRESS, GUEST_MEMORY_BYTES - 16)?;

        let completed = Arc::new(AtomicBool::new(false));
        let signal_sent = Arc::new(AtomicBool::new(false));
        let target_thread = unsafe { libc::pthread_self() } as usize;
        let watchdog_completed = Arc::clone(&completed);
        let watchdog_signal_sent = Arc::clone(&signal_sent);
        let watchdog = thread::spawn(move || {
            thread::sleep(RECOVERY_TIMEOUT);
            if !watchdog_completed.load(Ordering::Acquire) {
                watchdog_signal_sent.store(true, Ordering::Release);
                unsafe {
                    libc::pthread_kill(target_thread as libc::pthread_t, KVM_RUN_TIMEOUT_SIGNAL);
                }
            }
        });
        let started = Instant::now();
        let blocked_result = vcpu.run();
        let blocked_elapsed = started.elapsed();
        completed.store(true, Ordering::Release);
        watchdog
            .join()
            .map_err(|_| "[E_MACHINE_KVM_RECOVERY_001] recovery watchdog panicked")?;
        let blocked_errno = match blocked_result {
            Err(error) if error.errno() == libc::EINTR => error.errno(),
            Err(error) => {
                return Err(format!(
                    "[E_MACHINE_KVM_RECOVERY_001] blocked KVM_RUN returned errno {}, expected EINTR",
                    error.errno()
                )
                .into())
            }
            Ok(exit) => {
                return Err(format!(
                    "[E_MACHINE_KVM_RECOVERY_001] infinite guest returned before cancellation: {exit:?}"
                )
                .into())
            }
        };
        if !signal_sent.load(Ordering::Acquire) || blocked_elapsed < RECOVERY_TIMEOUT {
            return Err(format!(
                "[E_MACHINE_KVM_RECOVERY_001] cancellation was not observed after the configured deadline: sent={} elapsed_ms={} deadline_ms={}",
                signal_sent.load(Ordering::Acquire),
                blocked_elapsed.as_millis(),
                RECOVERY_TIMEOUT.as_millis()
            )
            .into());
        }

        memory.bytes_mut()[RECOVERY_CODE_ADDRESS] = 0xf4;
        std::sync::atomic::fence(Ordering::SeqCst);
        configure_real_mode_vcpu(&vcpu, RECOVERY_CODE_ADDRESS, GUEST_MEMORY_BYTES - 16)?;
        let reentry_started = Instant::now();
        let reentry = vcpu.run()?;
        let reentry_halted = matches!(reentry, VcpuExit::Hlt);
        if !reentry_halted {
            return Err(format!(
                "[E_MACHINE_KVM_RECOVERY_001] vCPU did not halt after re-entry: {reentry:?}"
            )
            .into());
        }
        Ok(json!({
            "infinite_guest_entered": true,
            "watchdog_signal_sent": true,
            "kvm_run_interrupted": true,
            "interrupted_errno": blocked_errno,
            "expected_errno": libc::EINTR,
            "deadline_milliseconds": RECOVERY_TIMEOUT.as_millis(),
            "blocked_elapsed_milliseconds": blocked_elapsed.as_millis(),
            "vcpu_reentered_after_interrupt": true,
            "reentry_final_halt": true,
            "reentry_elapsed_microseconds": reentry_started.elapsed().as_micros(),
        }))
    }

    #[derive(Debug, Clone)]
    enum KvmExitPayload {
        None,
        IoOut { port: u16, data: Vec<u8> },
        IoIn { port: u16, bytes: usize },
        MmioWrite { address: u64, data: Vec<u8> },
        MmioRead { address: u64, response: Vec<u8> },
        Halt,
        Shutdown,
    }

    enum OwnedKvmExit {
        IoOut { port: u16, data: Vec<u8> },
        IoIn { port: u16, bytes: usize },
        MmioWrite { address: u64, data: Vec<u8> },
        MmioRead { address: u64, response: Vec<u8> },
        Halt,
        Shutdown,
    }

    struct KvmExecutionEngine {
        vcpu: VcpuFd,
        vendor: VmVendor,
        registers: GuestRegisters,
        payload: KvmExitPayload,
        injected_events: Vec<VmEvent>,
        event_window_requests: u64,
        deadline: Instant,
    }

    impl KvmExecutionEngine {
        fn new(vcpu: VcpuFd, vendor: VmVendor, registers: GuestRegisters) -> Self {
            Self {
                vcpu,
                vendor,
                registers,
                payload: KvmExitPayload::None,
                injected_events: Vec::new(),
                event_window_requests: 0,
                deadline: Instant::now() + CERTIFICATION_TIMEOUT,
            }
        }

        fn run_until_deadline(&mut self) -> Result<OwnedKvmExit, VmExitError> {
            let remaining = self
                .deadline
                .checked_duration_since(Instant::now())
                .ok_or(VmExitError::Engine(KVM_RUN_TIMEOUT_MESSAGE))?;
            let target_thread = unsafe { libc::pthread_self() } as usize;
            let cancelled = Arc::new((Mutex::new(false), Condvar::new()));
            let timed_out = Arc::new(AtomicBool::new(false));
            let watchdog_cancelled = Arc::clone(&cancelled);
            let watchdog_timed_out = Arc::clone(&timed_out);
            let watchdog = thread::spawn(move || {
                let (lock, condition) = &*watchdog_cancelled;
                let guard = lock.lock().expect("KVM watchdog mutex poisoned");
                let (guard, timeout) = condition
                    .wait_timeout_while(guard, remaining, |cancelled| !*cancelled)
                    .expect("KVM watchdog condition poisoned");
                if timeout.timed_out() && !*guard {
                    watchdog_timed_out.store(true, Ordering::Release);
                    unsafe {
                        libc::pthread_kill(
                            target_thread as libc::pthread_t,
                            KVM_RUN_TIMEOUT_SIGNAL,
                        );
                    }
                }
            });

            let result = self.vcpu.run();
            {
                let (lock, condition) = &*cancelled;
                *lock
                    .lock()
                    .map_err(|_| VmExitError::Engine("KVM watchdog mutex poisoned"))? = true;
                condition.notify_one();
            }
            watchdog
                .join()
                .map_err(|_| VmExitError::Engine("KVM watchdog thread panicked"))?;
            if timed_out.load(Ordering::Acquire) {
                return Err(VmExitError::Engine(KVM_RUN_TIMEOUT_MESSAGE));
            }
            match result.map_err(|_| VmExitError::Engine("KVM_RUN failed"))? {
                VcpuExit::IoOut(port, data) => Ok(OwnedKvmExit::IoOut {
                    port,
                    data: data.to_vec(),
                }),
                VcpuExit::IoIn(port, data) => {
                    let bytes = data.len();
                    data.fill(0);
                    Ok(OwnedKvmExit::IoIn { port, bytes })
                }
                VcpuExit::MmioWrite(address, data) => Ok(OwnedKvmExit::MmioWrite {
                    address,
                    data: data.to_vec(),
                }),
                VcpuExit::MmioRead(address, data) => {
                    data.fill(0xa5);
                    Ok(OwnedKvmExit::MmioRead {
                        address,
                        response: data.to_vec(),
                    })
                }
                VcpuExit::Hlt => Ok(OwnedKvmExit::Halt),
                VcpuExit::Shutdown => Ok(OwnedKvmExit::Shutdown),
                _ => Err(VmExitError::Engine("unexpected KVM exit")),
            }
        }

        fn guest_registers(registers: &kvm_bindings::kvm_regs) -> GuestRegisters {
            GuestRegisters {
                rax: registers.rax,
                rbx: registers.rbx,
                rcx: registers.rcx,
                rdx: registers.rdx,
                rsi: registers.rsi,
                rdi: registers.rdi,
                rbp: registers.rbp,
                rsp: registers.rsp,
                r8: registers.r8,
                r9: registers.r9,
                r10: registers.r10,
                r11: registers.r11,
                r12: registers.r12,
                r13: registers.r13,
                r14: registers.r14,
                r15: registers.r15,
                rip: registers.rip,
                rflags: registers.rflags,
            }
        }

        fn raw_reason(&self, kind: VmExitKind) -> u64 {
            match (self.vendor, kind) {
                (VmVendor::Intel, VmExitKind::PortIo) => 30,
                (VmVendor::Intel, VmExitKind::Halt) => 12,
                (VmVendor::Intel, VmExitKind::EptViolation) => 48,
                (VmVendor::Intel, VmExitKind::TripleFault) => 2,
                (VmVendor::Amd, VmExitKind::PortIo) => 0x7b,
                (VmVendor::Amd, VmExitKind::Halt) => 0x78,
                (VmVendor::Amd, VmExitKind::NestedPageFault) => 0x800,
                (VmVendor::Amd, VmExitKind::TripleFault) => 0x7f,
                _ => 0,
            }
        }

        fn set_cached_registers(&mut self, registers: GuestRegisters) -> Result<(), VmExitError> {
            let mut native = self
                .vcpu
                .get_regs()
                .map_err(|_| VmExitError::Engine("KVM_GET_REGS failed"))?;
            native.rax = registers.rax;
            native.rbx = registers.rbx;
            native.rcx = registers.rcx;
            native.rdx = registers.rdx;
            native.rsi = registers.rsi;
            native.rdi = registers.rdi;
            native.rbp = registers.rbp;
            native.rsp = registers.rsp;
            native.r8 = registers.r8;
            native.r9 = registers.r9;
            native.r10 = registers.r10;
            native.r11 = registers.r11;
            native.r12 = registers.r12;
            native.r13 = registers.r13;
            native.r14 = registers.r14;
            native.r15 = registers.r15;
            native.rip = registers.rip;
            native.rflags = registers.rflags;
            self.vcpu
                .set_regs(&native)
                .map_err(|_| VmExitError::Engine("KVM_SET_REGS failed"))?;
            self.registers = registers;
            Ok(())
        }
    }

    impl VmExecutionEngine for KvmExecutionEngine {
        fn enter(&mut self) -> Result<VmExit, VmExitError> {
            let (kind, mnemonic, qualification, guest_physical_address, payload) = match self
                .run_until_deadline()?
            {
                OwnedKvmExit::IoOut { port, data } => {
                    let size_code = match data.len() {
                        1 => 0,
                        2 => 1,
                        4 => 3,
                        _ => 0,
                    };
                    (
                        VmExitKind::PortIo,
                        "kvm_io_out",
                        size_code | (1 << 6) | ((port as u64) << 16),
                        None,
                        KvmExitPayload::IoOut { port, data },
                    )
                }
                OwnedKvmExit::IoIn { port, bytes } => {
                    let size_code = match bytes {
                        1 => 0,
                        2 => 1,
                        4 => 3,
                        _ => 0,
                    };
                    (
                        VmExitKind::PortIo,
                        "kvm_io_in",
                        size_code | (1 << 3) | (1 << 6) | ((port as u64) << 16),
                        None,
                        KvmExitPayload::IoIn { port, bytes },
                    )
                }
                OwnedKvmExit::MmioWrite { address, data } => (
                    if self.vendor == VmVendor::Intel {
                        VmExitKind::EptViolation
                    } else {
                        VmExitKind::NestedPageFault
                    },
                    "kvm_second_level_write",
                    if self.vendor == VmVendor::Intel {
                        1 << 1
                    } else {
                        (1 << 1) | (1_u64 << 32)
                    },
                    Some(GuestPhysicalAddress::new(address)),
                    KvmExitPayload::MmioWrite { address, data },
                ),
                OwnedKvmExit::MmioRead { address, response } => (
                    if self.vendor == VmVendor::Intel {
                        VmExitKind::EptViolation
                    } else {
                        VmExitKind::NestedPageFault
                    },
                    "kvm_second_level_read",
                    if self.vendor == VmVendor::Intel {
                        1
                    } else {
                        1_u64 << 32
                    },
                    Some(GuestPhysicalAddress::new(address)),
                    KvmExitPayload::MmioRead { address, response },
                ),
                OwnedKvmExit::Halt => (VmExitKind::Halt, "kvm_hlt", 0, None, KvmExitPayload::Halt),
                OwnedKvmExit::Shutdown => (
                    VmExitKind::TripleFault,
                    "kvm_shutdown",
                    0,
                    None,
                    KvmExitPayload::Shutdown,
                ),
            };
            let native = self
                .vcpu
                .get_regs()
                .map_err(|_| VmExitError::Engine("KVM_GET_REGS failed after exit"))?;
            self.registers = Self::guest_registers(&native);
            self.payload = payload;
            Ok(VmExit {
                vendor: self.vendor,
                kind,
                entry_failure: false,
                raw_reason: self.raw_reason(kind),
                mnemonic,
                qualification,
                guest_physical_address,
                guest_linear_address: None,
                instruction_length: 0,
                interruption: None,
                vectoring: None,
                guest_rip: self.registers.rip,
                guest_rflags: self.registers.rflags,
            })
        }

        fn registers(&self) -> GuestRegisters {
            self.registers
        }

        fn set_registers(&mut self, registers: GuestRegisters) -> Result<(), VmExitError> {
            self.set_cached_registers(registers)
        }

        fn advance_rip(&mut self, instruction_length: u32) -> Result<(), VmExitError> {
            let mut registers = self.registers;
            registers.rip = registers
                .rip
                .checked_add(instruction_length as u64)
                .ok_or(VmExitError::Engine("guest RIP overflow"))?;
            self.set_cached_registers(registers)
        }

        fn inject_event(&mut self, event: VmEvent) -> Result<(), VmExitError> {
            let event = event.validate()?;
            let mut events = self
                .vcpu
                .get_vcpu_events()
                .map_err(|_| VmExitError::Engine("KVM_GET_VCPU_EVENTS failed"))?;
            match event.kind {
                VmEventKind::ExternalInterrupt => {
                    events.interrupt.injected = 1;
                    events.interrupt.nr = event.vector;
                    events.interrupt.soft = 0;
                    events.interrupt.shadow = 0;
                }
                VmEventKind::NonMaskableInterrupt => {
                    events.nmi.injected = 1;
                    events.nmi.pending = 1;
                }
                VmEventKind::HardwareException
                | VmEventKind::PrivilegedSoftwareException
                | VmEventKind::SoftwareException => {
                    events.exception.injected = 1;
                    events.exception.nr = event.vector;
                    events.exception.has_error_code = u8::from(event.error_code.is_some());
                    events.exception.error_code = event.error_code.unwrap_or(0);
                }
                VmEventKind::SoftwareInterrupt => {
                    events.interrupt.injected = 1;
                    events.interrupt.nr = event.vector;
                    events.interrupt.soft = 1;
                    events.interrupt.shadow = 0;
                }
                VmEventKind::OtherEvent => {
                    return Err(VmExitError::Engine(
                        "KVM cannot inject an opaque VM other-event through VCPU_EVENTS",
                    ));
                }
            }
            self.vcpu
                .set_vcpu_events(&events)
                .map_err(|_| VmExitError::Engine("KVM_SET_VCPU_EVENTS failed"))?;
            self.vcpu.get_kvm_run().request_interrupt_window = 0;
            self.injected_events.push(event);
            Ok(())
        }

        fn event_injection_ready(&self, event: VmEvent) -> bool {
            let Ok(events) = self.vcpu.get_vcpu_events() else {
                return false;
            };
            match event.kind {
                VmEventKind::ExternalInterrupt => {
                    self.registers.rflags & (1 << 9) != 0
                        && events.interrupt.injected == 0
                        && events.interrupt.shadow == 0
                }
                VmEventKind::NonMaskableInterrupt => {
                    events.nmi.injected == 0 && events.nmi.masked == 0
                }
                VmEventKind::HardwareException
                | VmEventKind::PrivilegedSoftwareException
                | VmEventKind::SoftwareException => events.exception.injected == 0,
                VmEventKind::SoftwareInterrupt => events.interrupt.injected == 0,
                VmEventKind::OtherEvent => false,
            }
        }

        fn request_event_window(&mut self, _event: VmEvent) -> Result<(), VmExitError> {
            self.vcpu.get_kvm_run().request_interrupt_window = 1;
            self.event_window_requests += 1;
            Ok(())
        }
    }

    #[derive(Default)]
    struct KvmPhysicalHandler {
        markers: Vec<u8>,
        exits: Vec<Value>,
        first_halt_seen: bool,
        final_halt: bool,
        mmio_write: bool,
        mmio_read: bool,
        io_exit_count: u64,
        hlt_exit_count: u64,
        failure: Option<String>,
    }

    impl KvmPhysicalHandler {
        fn reject(&mut self, message: String) -> Result<VmExitAction, VmExitError> {
            self.failure = Some(message);
            Err(VmExitError::Handler("physical KVM evidence mismatch"))
        }
    }

    impl VmExitHandler<KvmExecutionEngine, 16> for KvmPhysicalHandler {
        fn handle(
            &mut self,
            engine: &mut KvmExecutionEngine,
            exit: &VmExit,
            pending: &mut PendingVmEvents<16>,
        ) -> Result<VmExitAction, VmExitError> {
            let ordinal = self.exits.len() as u64;
            match engine.payload.clone() {
                KvmExitPayload::IoOut { port, data } => {
                    self.io_exit_count += 1;
                    self.exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "io_out",
                        "typed_kind": format!("{:?}", exit.kind),
                        "port": port,
                        "bytes": data.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>(),
                    }));
                    if port == DEBUG_PORT {
                        self.markers.extend_from_slice(&data);
                    }
                    Ok(VmExitAction::Resume)
                }
                KvmExitPayload::IoIn { port, bytes } => {
                    self.io_exit_count += 1;
                    self.exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "io_in",
                        "typed_kind": format!("{:?}", exit.kind),
                        "port": port,
                        "bytes": bytes,
                    }));
                    Ok(VmExitAction::Resume)
                }
                KvmExitPayload::MmioWrite { address, data } => {
                    if address != MMIO_ADDRESS || data.as_slice() != [0x5a] {
                        return self.reject(format!(
                            "[E_MACHINE_KVM_MMIO_001] unexpected MMIO write at {address:#x}: {data:02x?}"
                        ));
                    }
                    self.mmio_write = true;
                    self.exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "mmio_write",
                        "typed_kind": format!("{:?}", exit.kind),
                        "guest_physical_address": format!("{address:#x}"),
                        "data_hex": "5a",
                    }));
                    Ok(VmExitAction::Resume)
                }
                KvmExitPayload::MmioRead { address, response } => {
                    if address != MMIO_ADDRESS || response.as_slice() != [0xa5] {
                        return self.reject(format!(
                            "[E_MACHINE_KVM_MMIO_001] unexpected MMIO read at {address:#x}: {response:02x?}"
                        ));
                    }
                    self.mmio_read = true;
                    self.exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "mmio_read",
                        "typed_kind": format!("{:?}", exit.kind),
                        "guest_physical_address": format!("{address:#x}"),
                        "response_hex": "a5",
                    }));
                    Ok(VmExitAction::Resume)
                }
                KvmExitPayload::Halt => {
                    self.hlt_exit_count += 1;
                    self.exits.push(json!({
                        "ordinal": ordinal,
                        "kind": "hlt",
                        "typed_kind": format!("{:?}", exit.kind),
                        "phase": if self.first_halt_seen { "final" } else { "interrupt_window" },
                    }));
                    if self.first_halt_seen {
                        self.final_halt = true;
                        Ok(VmExitAction::Halt)
                    } else {
                        self.first_halt_seen = true;
                        pending.push(VmEvent::external_interrupt(0x20))?;
                        Ok(VmExitAction::Resume)
                    }
                }
                KvmExitPayload::Shutdown => Ok(VmExitAction::Shutdown),
                KvmExitPayload::None => self.reject(
                    "[E_MACHINE_KVM_EXIT_001] handler received no KVM exit payload".to_string(),
                ),
            }
        }
    }

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        let _signal_guard = KvmRunSignalGuard::install()?;
        let kvm = Kvm::new().map_err(|error| {
            format!(
                "[E_MACHINE_KVM_OPEN_001] cannot open /dev/kvm (run the Linux/WSL binary with permission to the device): {error}"
            )
        })?;
        if !kvm.check_extension(Cap::UserMemory) || !kvm.check_extension(Cap::VcpuEvents) {
            return Err("[E_MACHINE_KVM_CAPABILITY_001] KVM lacks UserMemory or VcpuEvents".into());
        }
        let recovery = certify_blocked_run_recovery(&kvm)?;
        let vm = kvm
            .create_vm()
            .map_err(|error| format!("[E_MACHINE_KVM_VM_001] KVM_CREATE_VM failed: {error}"))?;
        let mut memory = GuestMemory::new(GUEST_MEMORY_BYTES)?;
        write_guest(memory.bytes_mut());
        let slot = 0_u32;
        let region = kvm_userspace_memory_region {
            slot,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
            guest_phys_addr: 0,
            memory_size: GUEST_MEMORY_BYTES as u64,
            userspace_addr: memory.address(),
        };
        unsafe { vm.set_user_memory_region(region) }.map_err(|error| {
            format!("[E_MACHINE_KVM_MEMORY_SLOT_001] KVM_SET_USER_MEMORY_REGION failed: {error}")
        })?;

        let vcpu = vm
            .create_vcpu(0)
            .map_err(|error| format!("[E_MACHINE_KVM_VCPU_001] KVM_CREATE_VCPU failed: {error}"))?;
        configure_real_mode_vcpu(&vcpu, CODE_ADDRESS, GUEST_MEMORY_BYTES - 16)?;
        let registers = vcpu.get_regs()?;

        let vendor = if host_probe()
            .pointer("/cpuid/vendor")
            .and_then(Value::as_str)
            == Some("AuthenticAMD")
        {
            VmVendor::Amd
        } else {
            VmVendor::Intel
        };
        let initial_registers = KvmExecutionEngine::guest_registers(&registers);
        let mut engine = KvmExecutionEngine::new(vcpu, vendor, initial_registers);
        let mut handler = KvmPhysicalHandler::default();
        let mut pending = PendingVmEvents::<16>::new();
        let certification_started = Instant::now();
        let outcome = match run_vm_exit_loop(&mut engine, &mut handler, &mut pending, 128) {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(failure) = handler.failure.take() {
                    return Err(failure.into());
                }
                if error.to_string().contains(KVM_RUN_TIMEOUT_MESSAGE) {
                    return Err(format!(
                        "[E_MACHINE_KVM_TIMEOUT_001] KVM_RUN did not return within {} ms",
                        CERTIFICATION_TIMEOUT.as_millis()
                    )
                    .into());
                }
                return Err(format!("[E_MACHINE_KVM_LOOP_001] {error}").into());
            }
        };
        let certification_elapsed = certification_started.elapsed();
        if outcome.stop_reason != VmLoopStopReason::Halted {
            return Err(format!(
                "[E_MACHINE_KVM_LOOP_001] typed VM-exit loop stopped as {:?}",
                outcome.stop_reason
            )
            .into());
        }

        let interrupt_injected = engine
            .injected_events
            .iter()
            .any(|event| event.kind == VmEventKind::ExternalInterrupt && event.vector == 0x20);
        let marker_text = String::from_utf8(handler.markers.clone())?;
        let ram_word = u16::from_le_bytes(
            memory.bytes_mut()[RAM_WRITE_ADDRESS..RAM_WRITE_ADDRESS + 2]
                .try_into()
                .expect("two-byte RAM word"),
        );
        let dirty_bitmap = vm.get_dirty_log(slot, GUEST_MEMORY_BYTES)?;
        let dirty_pages = dirty_bitmap
            .iter()
            .map(|word| word.count_ones() as u64)
            .sum::<u64>();
        let expected_markers = "SEXIMO";
        if marker_text != expected_markers
            || !handler.mmio_write
            || !handler.mmio_read
            || !interrupt_injected
            || !handler.final_halt
            || ram_word != 0x1234
            || dirty_pages == 0
            || !pending.is_empty()
        {
            return Err(format!(
                "[E_MACHINE_KVM_EVIDENCE_001] incomplete hardware evidence: markers={marker_text:?} mmio_write={} mmio_read={} interrupt={interrupt_injected} final_halt={} ram_word={ram_word:#x} dirty_pages={dirty_pages} pending_events={} ",
                handler.mmio_write,
                handler.mmio_read,
                handler.final_halt,
                pending.len(),
            )
            .into());
        }

        Ok(json!({
            "kvm_api_version": kvm.get_api_version(),
            "guest": {
                "mode": "x86_real_mode",
                "memory_bytes": GUEST_MEMORY_BYTES,
                "code_gpa": format!("{CODE_ADDRESS:#x}"),
                "stack_gpa": format!("{:#x}", GUEST_MEMORY_BYTES - 16),
                "mmio_gpa": format!("{MMIO_ADDRESS:#x}"),
                "code_sha256": sha256_bytes(GUEST_CODE),
                "invalid_opcode_handler_sha256": sha256_bytes(INVALID_OPCODE_HANDLER_CODE),
                "interrupt_handler_sha256": sha256_bytes(INTERRUPT_HANDLER_CODE),
            },
            "results": {
                "marker_sequence": marker_text,
                "expected_marker_sequence": expected_markers,
                "invalid_opcode_exception_delivered": handler.markers.contains(&b'E'),
                "external_interrupt_vector_0x20_delivered": handler.markers.contains(&b'I'),
                "mapped_ram_write_value": ram_word,
                "dirty_pages": dirty_pages,
                "unmapped_gpa_mmio_write_exit": handler.mmio_write,
                "unmapped_gpa_mmio_read_exit": handler.mmio_read,
                "interrupt_injected": interrupt_injected,
                "final_halt": handler.final_halt,
                "io_exit_count": handler.io_exit_count,
                "hlt_exit_count": handler.hlt_exit_count,
                "exit_count": handler.exits.len(),
                "typed_vmexit_loop": true,
                "typed_loop_stop_reason": format!("{:?}", outcome.stop_reason),
                "typed_loop_exits": outcome.exits,
                "typed_loop_injected_events": outcome.injected_events,
                "typed_loop_advanced_instructions": outcome.advanced_instructions,
                "pending_events_after_stop": pending.len(),
                "event_window_requests": engine.event_window_requests,
                "deadline_enforced": true,
                "deadline_milliseconds": CERTIFICATION_TIMEOUT.as_millis(),
                "elapsed_milliseconds": certification_elapsed.as_millis(),
                "blocked_kvm_run_interruptible": true,
            },
            "failure_recovery": recovery,
            "injected_events": engine.injected_events.iter().map(|event| json!({
                "vector": event.vector,
                "kind": format!("{:?}", event.kind),
                "error_code": event.error_code,
            })).collect::<Vec<_>>(),
            "exits": handler.exits,
        }))
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "aarch64", target_arch = "riscv64")
))]
mod linux_kvm {
    use super::*;
    use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
    #[cfg(target_arch = "aarch64")]
    use kvm_bindings::{kvm_vcpu_init, KVM_ARM_VCPU_PSCI_0_2, KVM_SYSTEM_EVENT_SHUTDOWN};
    use kvm_ioctls::{Cap, Kvm, VcpuExit};
    use std::ptr::{null_mut, NonNull};
    use std::time::{Duration, Instant};

    const GUEST_ADDRESS: u64 = 0x1_0000;
    const GUEST_MEMORY_BYTES: usize = 0x2_0000;
    const DATA_OFFSET: usize = 0x1_0000;
    const MMIO_ADDRESS: u64 = GUEST_ADDRESS + GUEST_MEMORY_BYTES as u64;
    const MAX_EXITS: usize = 16;
    const CERTIFICATION_TIMEOUT: Duration = Duration::from_millis(KVM_CERTIFICATION_TIMEOUT_MS);

    #[cfg(target_arch = "aarch64")]
    const GUEST_CODE: &[u8] = &[
        0x40, 0x20, 0x80, 0x52, // mov w0, #0x102
        0x00, 0x01, 0x00, 0xb9, // str w0, [x8]
        0x81, 0x60, 0x80, 0x52, // mov w1, #0x304
        0x02, 0x00, 0x80, 0x52, // mov w2, #0
        0x20, 0x01, 0x40, 0xb9, // ldr w0, [x9]
        0x1f, 0x18, 0x14, 0x71, // cmp w0, #0x506
        0x20, 0x00, 0x82, 0x1a, // csel w0, w1, w2, eq
        0x20, 0x01, 0x00, 0xb9, // str w0, [x9]
        0x00, 0x80, 0xb0, 0x52, // mov w0, #0x84000000
        0x00, 0x00, 0x1d, 0x32, // orr w0, w0, #8 (PSCI SYSTEM_OFF)
        0x02, 0x00, 0x00, 0xd4, // hvc #0
        0x00, 0x00, 0x00, 0x14, // b .
    ];

    #[cfg(target_arch = "riscv64")]
    const GUEST_CODE: &[u8] = &[
        0x13, 0x05, 0x50, 0x40, // li a0, 0x405
        0x23, 0x20, 0xac, 0x00, // sw a0, 0(s8)
        0x03, 0xa5, 0x0c, 0x00, // lw a0, 0(s9)
        0x93, 0x05, 0x70, 0x60, // li a1, 0x607
        0x23, 0xa0, 0xbc, 0x00, // sw a1, 0(s9)
        0x6f, 0x00, 0x00, 0x00, // j .
    ];

    extern "C" fn deadline_signal_handler(_signal: libc::c_int) {}

    struct DeadlineSignalGuard {
        previous: libc::sigaction,
    }

    impl DeadlineSignalGuard {
        fn install() -> Result<Self, Box<dyn Error>> {
            let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
            let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
            action.sa_sigaction = deadline_signal_handler as *const () as usize;
            action.sa_flags = 0;
            unsafe {
                libc::sigemptyset(&mut action.sa_mask);
                if libc::sigaction(libc::SIGALRM, &action, &mut previous) != 0 {
                    return Err(format!(
                        "[E_MACHINE_KVM_SIGNAL_001] cannot install KVM deadline signal: {}",
                        std::io::Error::last_os_error()
                    )
                    .into());
                }
            }
            Ok(Self { previous })
        }

        fn arm(&self, remaining: Duration) {
            let seconds = remaining.as_secs().saturating_add(1).min(u32::MAX as u64) as u32;
            unsafe {
                libc::alarm(seconds.max(1));
            }
        }

        fn disarm(&self) {
            unsafe {
                libc::alarm(0);
            }
        }
    }

    impl Drop for DeadlineSignalGuard {
        fn drop(&mut self) {
            self.disarm();
            unsafe {
                libc::sigaction(libc::SIGALRM, &self.previous, null_mut());
            }
        }
    }

    struct GuestMemory {
        pointer: NonNull<u8>,
        bytes: usize,
    }

    impl GuestMemory {
        fn new(bytes: usize) -> Result<Self, Box<dyn Error>> {
            let pointer = unsafe {
                libc::mmap(
                    null_mut(),
                    bytes,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                    -1,
                    0,
                )
            };
            let pointer = NonNull::new(pointer.cast::<u8>())
                .filter(|pointer| pointer.as_ptr() as *mut libc::c_void != libc::MAP_FAILED)
                .ok_or_else(|| {
                    format!(
                        "[E_MACHINE_KVM_MEMORY_001] mmap failed: {}",
                        std::io::Error::last_os_error()
                    )
                })?;
            Ok(Self { pointer, bytes })
        }

        fn address(&self) -> u64 {
            self.pointer.as_ptr() as u64
        }

        fn bytes_mut(&mut self) -> &mut [u8] {
            unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.bytes) }
        }
    }

    impl Drop for GuestMemory {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.pointer.as_ptr().cast(), self.bytes);
            }
        }
    }

    fn set_registers(
        vm: &kvm_ioctls::VmFd,
        vcpu: &kvm_ioctls::VcpuFd,
    ) -> Result<u64, Box<dyn Error>> {
        #[cfg(target_arch = "aarch64")]
        {
            let mut init = kvm_vcpu_init::default();
            vm.get_preferred_target(&mut init)?;
            init.features[0] |= 1 << KVM_ARM_VCPU_PSCI_0_2;
            vcpu.vcpu_init(&init)?;
            let core = UINT64_C_ARM_CORE_REG_BASE;
            vcpu.set_one_reg(core + 2 * 32, &(GUEST_ADDRESS as u128).to_le_bytes())?;
            vcpu.set_one_reg(
                core + 2 * 8,
                &((GUEST_ADDRESS + DATA_OFFSET as u64) as u128).to_le_bytes(),
            )?;
            vcpu.set_one_reg(core + 2 * 9, &(MMIO_ADDRESS as u128).to_le_bytes())?;
            return Ok(core + 2 * 32);
        }
        #[cfg(target_arch = "riscv64")]
        {
            let _ = vm;
            let core = UINT64_C_RISCV_CORE_REG_BASE;
            vcpu.set_one_reg(core, &(GUEST_ADDRESS as u128).to_le_bytes())?;
            vcpu.set_one_reg(
                core + 24,
                &((GUEST_ADDRESS + DATA_OFFSET as u64) as u128).to_le_bytes(),
            )?;
            vcpu.set_one_reg(core + 25, &(MMIO_ADDRESS as u128).to_le_bytes())?;
            return Ok(core);
        }
    }

    #[cfg(target_arch = "aarch64")]
    const UINT64_C_ARM_CORE_REG_BASE: u64 = 0x6030_0000_0010_0000;
    #[cfg(target_arch = "riscv64")]
    const UINT64_C_RISCV_CORE_REG_BASE: u64 = 0x8030_0000_0200_0000;

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        let signal = DeadlineSignalGuard::install()?;
        let kvm = Kvm::new()
            .map_err(|error| format!("[E_MACHINE_KVM_OPEN_001] cannot open /dev/kvm: {error}"))?;
        if !kvm.check_extension(Cap::UserMemory) {
            return Err("[E_MACHINE_KVM_CAPABILITY_001] KVM lacks UserMemory".into());
        }
        let vm = kvm.create_vm()?;
        let mut memory = GuestMemory::new(GUEST_MEMORY_BYTES)?;
        memory.bytes_mut()[..GUEST_CODE.len()].copy_from_slice(GUEST_CODE);
        let slot = 0_u32;
        let region = kvm_userspace_memory_region {
            slot,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
            guest_phys_addr: GUEST_ADDRESS,
            memory_size: GUEST_MEMORY_BYTES as u64,
            userspace_addr: memory.address(),
        };
        unsafe { vm.set_user_memory_region(region) }?;
        let mut vcpu = vm.create_vcpu(0)?;
        let pc_register = set_registers(&vm, &vcpu)?;
        let started = Instant::now();
        let deadline = started + CERTIFICATION_TIMEOUT;
        let mut exits = Vec::new();
        let mut mmio_read = false;
        let mut mmio_write = false;
        let mut final_stop = false;

        for ordinal in 0..MAX_EXITS {
            let now = Instant::now();
            if now >= deadline {
                return Err("[E_MACHINE_KVM_TIMEOUT_001] portable KVM deadline expired".into());
            }
            signal.arm(deadline.saturating_duration_since(now));
            let exit = vcpu.run();
            signal.disarm();
            let exit = match exit {
                Ok(exit) => exit,
                Err(error) if Instant::now() >= deadline => {
                    return Err(format!(
                        "[E_MACHINE_KVM_TIMEOUT_001] KVM_RUN exceeded {} ms: {error}",
                        KVM_CERTIFICATION_TIMEOUT_MS
                    )
                    .into())
                }
                Err(error) => return Err(format!("[E_MACHINE_KVM_RUN_001] {error}").into()),
            };
            match exit {
                VcpuExit::MmioRead(address, data) => {
                    if address != MMIO_ADDRESS || data.len() != 4 {
                        return Err(format!(
                            "[E_MACHINE_KVM_MMIO_001] unexpected read {address:#x}/{}",
                            data.len()
                        )
                        .into());
                    }
                    data.copy_from_slice(&[0x06, 0x05, 0x00, 0x00]);
                    mmio_read = true;
                    exits.push(json!({"ordinal": ordinal, "kind": "mmio_read", "address": address, "response": "06050000"}));
                }
                VcpuExit::MmioWrite(address, data) => {
                    #[cfg(target_arch = "aarch64")]
                    let expected = [0x04, 0x03, 0x00, 0x00];
                    #[cfg(target_arch = "riscv64")]
                    let expected = [0x07, 0x06, 0x00, 0x00];
                    if address != MMIO_ADDRESS || data != expected {
                        return Err(format!(
                            "[E_MACHINE_KVM_MMIO_001] unexpected write {address:#x}/{data:02x?}"
                        )
                        .into());
                    }
                    mmio_write = true;
                    exits.push(json!({"ordinal": ordinal, "kind": "mmio_write", "address": address, "data": format!("{data:02x?}")}));
                    #[cfg(target_arch = "riscv64")]
                    {
                        final_stop = true;
                        break;
                    }
                }
                VcpuExit::SystemEvent(kind, data) => {
                    #[cfg(target_arch = "aarch64")]
                    if kind == KVM_SYSTEM_EVENT_SHUTDOWN && data.first().copied().unwrap_or(0) == 0
                    {
                        final_stop = true;
                    }
                    exits.push(json!({"ordinal": ordinal, "kind": "system_event", "event": kind, "data": data}));
                    if final_stop {
                        break;
                    }
                }
                other => {
                    return Err(format!(
                        "[E_MACHINE_KVM_EXIT_001] unexpected portable KVM exit {other:?}"
                    )
                    .into())
                }
            }
        }

        let mut pc = [0_u8; 16];
        vcpu.get_one_reg(pc_register, &mut pc)?;
        let final_pc = u128::from_le_bytes(pc) as u64;
        let material_word = u32::from_le_bytes(
            memory.bytes_mut()[DATA_OFFSET..DATA_OFFSET + 4]
                .try_into()
                .expect("four-byte material word"),
        );
        #[cfg(target_arch = "aarch64")]
        let expected_word = 0x102;
        #[cfg(target_arch = "riscv64")]
        let expected_word = 0x405;
        let dirty_bitmap = vm.get_dirty_log(slot, GUEST_MEMORY_BYTES)?;
        let dirty_pages = dirty_bitmap
            .iter()
            .map(|word| word.count_ones() as u64)
            .sum::<u64>();
        if !mmio_read
            || !mmio_write
            || !final_stop
            || material_word != expected_word
            || dirty_pages == 0
        {
            return Err(format!(
                "[E_MACHINE_KVM_EVIDENCE_001] read={mmio_read} write={mmio_write} stop={final_stop} material={material_word:#x} dirty={dirty_pages}"
            )
            .into());
        }

        #[cfg(target_arch = "aarch64")]
        let (architecture, extension, terminal_condition) = ("aarch64", "EL2", "psci_system_off");
        #[cfg(target_arch = "riscv64")]
        let (architecture, extension, terminal_condition) =
            ("riscv64", "H", "last_known_mmio_write");
        Ok(json!({
            "kvm_api_version": kvm.get_api_version(),
            "guest": {
                "architecture": architecture,
                "hardware_virtualization_extension": extension,
                "mode": "native_kvm_guest",
                "memory_bytes": GUEST_MEMORY_BYTES,
                "code_gpa": format!("{GUEST_ADDRESS:#x}"),
                "mmio_gpa": format!("{MMIO_ADDRESS:#x}"),
                "code_sha256": sha256_bytes(GUEST_CODE),
            },
            "results": {
                "mapped_ram_write_value": material_word,
                "dirty_pages": dirty_pages,
                "unmapped_gpa_mmio_read_exit": mmio_read,
                "unmapped_gpa_mmio_write_exit": mmio_write,
                "final_halt": final_stop,
                "terminal_condition": terminal_condition,
                "final_pc": format!("{final_pc:#x}"),
                "exit_count": exits.len(),
                "typed_vmexit_loop": true,
                "deadline_enforced": true,
                "deadline_milliseconds": KVM_CERTIFICATION_TIMEOUT_MS,
                "elapsed_milliseconds": started.elapsed().as_millis(),
            },
            "exits": exits,
        }))
    }
}

#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "riscv64")
)))]
mod linux_kvm {
    use super::*;

    pub(super) fn execute() -> Result<Value, Box<dyn Error>> {
        Err(
            "[E_MACHINE_KVM_HOST_001] CPU-backed certification requires a Linux x86_64, AArch64 or RISC-V64 ncc binary with /dev/kvm access"
                .into(),
        )
    }
}

pub(crate) fn execute_kvm_backend_evidence() -> Result<Value, Box<dyn Error>> {
    linux_kvm::execute()
}

pub(crate) fn run_certify_host(args: &[String]) -> Result<(), Box<dyn Error>> {
    let execution = linux_kvm::execute()?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let execution = {
        let mut execution = execution;
        execution["direct_platform"] =
            super::machine_platform::execute_direct_kvm_platform_evidence()?;
        execution
    };
    let host = host_probe();
    let hypervisor_present = host
        .pointer("/cpuid/hypervisor_present")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let bare_metal = !hypervisor_present;
    let nested_extension_available = host
        .get("nested_intel")
        .and_then(Value::as_str)
        .or_else(|| host.get("nested_amd").and_then(Value::as_str))
        .is_some_and(|value| matches!(value, "Y" | "1"));
    let payload = json!({
        "schema": CERTIFICATE_SCHEMA,
        "certificate_version": 1,
        "status": "cpu_backed_hardware_passed",
        "created_epoch_seconds": epoch_seconds(),
        "source": super::machine_backends::source_identity(),
        "host": host,
        "execution": execution,
        "truth_boundary": {
            "cpu_backed_execution": true,
            "kvm_acceleration": true,
            "nested_virtualization_detected": hypervisor_present,
            "nested_extension_available": nested_extension_available,
            "bare_metal_board": bare_metal,
            "physical_board_certificate": bare_metal,
            "qemu_tcg": false,
            "direct_kvm_api": true,
            "host_vmcs_vmcb_owned_by_kvm": true,
            "ncc_vmcs_vmcb_layout_and_opcode_evidence_is_separate": true,
            "guest_memory_and_vmexit_behavior_executed_on_cpu": true,
            "blocked_run_recovery_executed": execution.pointer("/failure_recovery/vcpu_reentered_after_interrupt").and_then(Value::as_bool) == Some(true),
            "multi_vcpu_delivery_executed": execution.pointer("/direct_platform/smp/passed").and_then(Value::as_bool) == Some(true),
        },
    });
    let mut certificate = payload.clone();
    certificate["attestation"] = json!({
        "algorithm": "sha256",
        "domain": String::from_utf8_lossy(PAYLOAD_DOMAIN).trim_end_matches('\0'),
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
            "OK: CPU-backed KVM virtualization certificate written to {}",
            option_value(args, "--out").unwrap_or_default()
        );
    }
    Ok(())
}

pub(crate) fn run_verify_host_certificate(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_non_flag(args)
        .map(PathBuf::from)
        .ok_or("virtualization verify-certificate requires a certificate path")?;
    let mut certificate: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if certificate.get("schema").and_then(Value::as_str) != Some(CERTIFICATE_SCHEMA) {
        return Err("[E_MACHINE_KVM_CERTIFICATE_001] unsupported certificate schema".into());
    }
    let expected = certificate
        .pointer("/attestation/payload_sha256")
        .and_then(Value::as_str)
        .ok_or("[E_MACHINE_KVM_CERTIFICATE_001] missing payload hash")?
        .to_string();
    certificate
        .as_object_mut()
        .expect("certificate object")
        .remove("attestation");
    let actual = payload_sha256(&certificate)?;
    if actual != expected {
        return Err(format!(
            "[E_MACHINE_KVM_CERTIFICATE_HASH_001] payload hash mismatch: expected {expected}, got {actual}"
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
        return Err("[E_MACHINE_KVM_CERTIFICATE_HEAD_001] GitHub certificate was not produced from the checked-out exact head".into());
    }
    if has_flag(args, "--require-bare-metal")
        && (certificate
            .pointer("/truth_boundary/bare_metal_board")
            .and_then(Value::as_bool)
            != Some(true)
            || certificate
                .pointer("/truth_boundary/physical_board_certificate")
                .and_then(Value::as_bool)
                != Some(true))
    {
        return Err("[E_MACHINE_KVM_CERTIFICATE_PHYSICAL_001] certificate is CPU-backed but does not come from a bare-metal host".into());
    }
    let mut required = vec![
        "/truth_boundary/cpu_backed_execution",
        "/truth_boundary/kvm_acceleration",
        "/execution/results/unmapped_gpa_mmio_write_exit",
        "/execution/results/unmapped_gpa_mmio_read_exit",
        "/execution/results/final_halt",
        "/execution/results/deadline_enforced",
    ];
    if certificate
        .pointer("/host/architecture")
        .and_then(Value::as_str)
        == Some("x86_64")
    {
        required.extend([
            "/execution/results/invalid_opcode_exception_delivered",
            "/execution/results/external_interrupt_vector_0x20_delivered",
            "/execution/results/blocked_kvm_run_interruptible",
            "/execution/failure_recovery/infinite_guest_entered",
            "/execution/failure_recovery/watchdog_signal_sent",
            "/execution/failure_recovery/kvm_run_interrupted",
            "/execution/failure_recovery/vcpu_reentered_after_interrupt",
            "/execution/failure_recovery/reentry_final_halt",
            "/execution/direct_platform/lapic_timer/passed",
            "/execution/direct_platform/pit_timer/passed",
            "/execution/direct_platform/smp/passed",
            "/truth_boundary/blocked_run_recovery_executed",
            "/truth_boundary/multi_vcpu_delivery_executed",
        ]);
    }
    for pointer in required {
        if certificate.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!(
                "[E_MACHINE_KVM_CERTIFICATE_EVIDENCE_001] required evidence '{pointer}' is not true"
            )
            .into());
        }
    }
    let deadline_ms = certificate
        .pointer("/execution/results/deadline_milliseconds")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_KVM_CERTIFICATE_EVIDENCE_001] missing execution deadline")?;
    let elapsed_ms = certificate
        .pointer("/execution/results/elapsed_milliseconds")
        .and_then(Value::as_u64)
        .ok_or("[E_MACHINE_KVM_CERTIFICATE_EVIDENCE_001] missing execution duration")?;
    if deadline_ms == 0 || elapsed_ms > deadline_ms {
        return Err(format!(
            "[E_MACHINE_KVM_CERTIFICATE_EVIDENCE_001] execution duration {elapsed_ms} ms exceeds deadline {deadline_ms} ms"
        )
        .into());
    }
    let report = json!({
        "schema": "ncc.machine.x86_virtualization_hardware_certificate_verification.v1",
        "status": "verified",
        "certificate": path,
        "payload_sha256": actual,
        "bare_metal_board": certificate.pointer("/truth_boundary/bare_metal_board").and_then(Value::as_bool).unwrap_or(false),
        "cpu_backed_execution": true,
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("OK: CPU-backed virtualization certificate verified");
    }
    Ok(())
}

pub(crate) fn run_probe_host(args: &[String]) -> Result<(), Box<dyn Error>> {
    let payload = json!({
        "schema": "ncc.machine.x86_virtualization_host_probe.v1",
        "host": host_probe(),
        "certification_available": cfg!(all(target_os = "linux", target_arch = "x86_64"))
            && Path::new("/dev/kvm").exists(),
        "required_command": "ncc machine virtualization certify-host --out certificate.json",
    });
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("machine virtualization host probe: {}", payload["host"]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_attestation_rejects_any_payload_mutation() {
        let mut payload = json!({
            "schema": CERTIFICATE_SCHEMA,
            "status": "cpu_backed_hardware_passed",
            "truth_boundary": { "cpu_backed_execution": true },
        });
        let before = payload_sha256(&payload).unwrap();
        payload["truth_boundary"]["cpu_backed_execution"] = json!(false);
        assert_ne!(before, payload_sha256(&payload).unwrap());
    }
}
