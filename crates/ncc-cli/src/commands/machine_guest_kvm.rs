use super::*;
use kvm_bindings::{
    kvm_pit_config, kvm_segment, kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES,
    KVM_SYSTEM_EVENT_CRASH, KVM_SYSTEM_EVENT_RESET, KVM_SYSTEM_EVENT_SHUTDOWN,
};
use kvm_ioctls::{Cap, Kvm, VcpuExit, VcpuFd};
use ncc_machine::{
    GuestCpuMode, PciVirtioError, Uart16550, VirtioBlockDevice, VirtioGuestMemory,
    VirtioMmioBlockDevice, VirtualBlockDevice, PC_ACPI_PM1_CONTROL_PORT, PC_ACPI_PM1_EVENT_PORT,
    PC_ACPI_PM_TIMER_PORT, PC_VIRTIO_IRQ_BASE, PC_VIRTIO_MMIO_BASE, VIRTIO_F_RING_EVENT_IDX,
    VIRTIO_F_RING_INDIRECT_DESC, VIRTIO_MMIO_BYTES,
};
use std::ptr::{null_mut, NonNull};
use std::slice;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

pub(super) const VIRTIO_MMIO_BASE: u64 = PC_VIRTIO_MMIO_BASE;
pub(super) const VIRTIO_IRQ_BASE: u32 = PC_VIRTIO_IRQ_BASE;
const MAX_DIRECT_MEMORY: u64 = 3 * 1024 * 1024 * 1024;
const MAX_EXITS_DEFAULT: u64 = 10_000_000;
const MAX_REBOOTS: u32 = 16;
static KVM_IMMEDIATE_EXIT: AtomicPtr<u8> = AtomicPtr::new(null_mut());

extern "C" fn timeout_signal_handler(_signal: libc::c_int) {
    let pointer = KVM_IMMEDIATE_EXIT.load(Ordering::Relaxed);
    if !pointer.is_null() {
        unsafe {
            std::ptr::write_volatile(pointer, 1);
        }
    }
}

struct AlarmGuard {
    previous: libc::sigaction,
}

impl AlarmGuard {
    fn install() -> Result<Self, Box<dyn Error>> {
        let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
        let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
        action.sa_sigaction = timeout_signal_handler as *const () as usize;
        unsafe {
            libc::sigemptyset(&mut action.sa_mask);
            if libc::sigaction(libc::SIGALRM, &action, &mut previous) != 0 {
                return Err(format!(
                    "cannot install KVM timeout signal: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
        }
        Ok(Self { previous })
    }
}

impl Drop for AlarmGuard {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(libc::SIGALRM, &self.previous, null_mut());
        }
    }
}

struct ImmediateExitRegistration;

impl ImmediateExitRegistration {
    fn install(pointer: *mut u8) -> Result<Self, Box<dyn Error>> {
        KVM_IMMEDIATE_EXIT
            .compare_exchange(null_mut(), pointer, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "another KVM direct runner is active in this process")?;
        Ok(Self)
    }
}

impl Drop for ImmediateExitRegistration {
    fn drop(&mut self) {
        KVM_IMMEDIATE_EXIT.store(null_mut(), Ordering::SeqCst);
    }
}

struct DeadlineInterrupt {
    cancelled: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl DeadlineInterrupt {
    fn spawn(timeout: Duration) -> Self {
        let cancelled = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_state = Arc::clone(&cancelled);
        let process_id = unsafe { libc::getpid() };
        let thread_id = unsafe { libc::syscall(libc::SYS_gettid) as libc::pid_t };
        let thread = std::thread::spawn(move || {
            let (lock, wake) = &*worker_state;
            let cancelled = lock.lock().expect("deadline mutex poisoned");
            let (cancelled, wait) = wake
                .wait_timeout_while(cancelled, timeout, |cancelled| !*cancelled)
                .expect("deadline mutex poisoned");
            if wait.timed_out() && !*cancelled {
                unsafe {
                    libc::syscall(libc::SYS_tgkill, process_id, thread_id, libc::SIGALRM);
                }
            }
        });
        Self {
            cancelled,
            thread: Some(thread),
        }
    }
}

impl Drop for DeadlineInterrupt {
    fn drop(&mut self) {
        let (lock, wake) = &*self.cancelled;
        *lock.lock().expect("deadline mutex poisoned") = true;
        wake.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct GuestMemory {
    pointer: NonNull<u8>,
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
        let pointer = NonNull::new(pointer.cast::<u8>())
            .filter(|pointer| pointer.as_ptr().cast::<libc::c_void>() != libc::MAP_FAILED)
            .ok_or_else(|| format!("guest mmap failed: {}", std::io::Error::last_os_error()))?;
        Ok(Self { pointer, length })
    }

    fn address(&self) -> u64 {
        self.pointer.as_ptr() as u64
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.pointer.as_ptr(), self.length) }
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.pointer.as_ptr().cast(), self.length);
        }
    }
}

impl VirtioGuestMemory for GuestMemory {
    fn capacity(&self) -> usize {
        self.length
    }

    fn read(&self, address: u64, output: &mut [u8]) -> Result<(), PciVirtioError> {
        let start = usize::try_from(address).map_err(|_| PciVirtioError::GuestMemory {
            address,
            length: output.len(),
        })?;
        let end = start
            .checked_add(output.len())
            .filter(|end| *end <= self.length)
            .ok_or(PciVirtioError::GuestMemory {
                address,
                length: output.len(),
            })?;
        for (index, output) in output.iter_mut().enumerate() {
            *output = unsafe { std::ptr::read_volatile(self.pointer.as_ptr().add(start + index)) };
        }
        debug_assert_eq!(end - start, output.len());
        Ok(())
    }

    fn write(&mut self, address: u64, input: &[u8]) -> Result<(), PciVirtioError> {
        let start = usize::try_from(address).map_err(|_| PciVirtioError::GuestMemory {
            address,
            length: input.len(),
        })?;
        start
            .checked_add(input.len())
            .filter(|end| *end <= self.length)
            .ok_or(PciVirtioError::GuestMemory {
                address,
                length: input.len(),
            })?;
        for (index, byte) in input.iter().enumerate() {
            unsafe {
                std::ptr::write_volatile(self.pointer.as_ptr().add(start + index), *byte);
            }
        }
        Ok(())
    }
}

struct VirtioSlot {
    address: u64,
    irq: u32,
    source_path: PathBuf,
    read_only: bool,
    device: VirtioMmioBlockDevice,
    completed_requests: u64,
}

pub(super) fn execute(
    plan: &GuestBootPlan,
    parsed: &ParsedGuest,
    args: &[String],
) -> Result<Value, Box<dyn Error>> {
    if plan.method != GuestSystemBootMethod::LinuxBzImage
        || !matches!(
            plan.architecture,
            GuestSystemArchitecture::X86_64 | GuestSystemArchitecture::I386
        )
        || plan.initial_cpu.mode != GuestCpuMode::Protected32
    {
        return Err("this host executes Linux bzImage directly; EFI needs a firmware platform and OpenSBI needs a RISC-V KVM host".into());
    }
    if plan.memory_base != 0 || plan.memory_size > MAX_DIRECT_MEMORY {
        return Err(
            format!("direct KVM bzImage memory must be in 0..={MAX_DIRECT_MEMORY} bytes").into(),
        );
    }
    if plan.vcpu_count != 1 {
        return Err("direct bzImage execution requires --vcpus 1; use the SMP platform certification for multi-vCPU startup".into());
    }
    let timeout = Duration::from_secs(parse_u64_option(args, "--timeout-seconds", 120)?);
    if timeout.is_zero() || timeout > Duration::from_secs(86_400) {
        return Err("--timeout-seconds must be between 1 and 86400".into());
    }
    let max_exits = parse_u64_option(args, "--max-exits", MAX_EXITS_DEFAULT)?;
    let expected_serial_text = option_value(args, "--expect-serial");
    let expected_serial = expected_serial_text.map(str::as_bytes);
    let halt_is_shutdown = has_flag(args, "--halt-is-shutdown");
    let quiet = has_flag(args, "--quiet");
    let mmio_trace_limit = usize::try_from(parse_u64_option(args, "--mmio-trace-limit", 4096)?)?;
    let mmio_trace_enabled = option_value(args, "--mmio-trace-out").is_some();
    let boot_started = Instant::now();
    let deadline = boot_started + timeout;
    let _alarm = AlarmGuard::install()?;
    let kvm = Kvm::new().map_err(|error| format!("cannot open /dev/kvm: {error}"))?;
    for capability in [Cap::UserMemory, Cap::Irqchip, Cap::Pit2] {
        if !kvm.check_extension(capability) {
            return Err(format!("KVM host lacks required capability {capability:?}").into());
        }
    }
    let vm = kvm.create_vm()?;
    vm.create_irq_chip()?;
    vm.create_pit2(kvm_pit_config::default())?;
    let memory_len = usize::try_from(plan.memory_size)?;
    let mut memory = GuestMemory::new(memory_len)?;
    plan.materialize(memory.bytes_mut())?;
    unsafe {
        vm.set_user_memory_region(kvm_userspace_memory_region {
            slot: 0,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
            guest_phys_addr: 0,
            memory_size: plan.memory_size,
            userspace_addr: memory.address(),
        })?;
    }
    let mut vcpu = vm.create_vcpu(0)?;
    let cpuid = kvm.get_supported_cpuid(256)?;
    vcpu.set_cpuid2(&cpuid)?;
    configure_linux_vcpu(&vcpu, plan)?;
    let immediate_exit = &mut vcpu.get_kvm_run().immediate_exit as *mut u8;
    let _immediate_exit = ImmediateExitRegistration::install(immediate_exit)?;
    let _deadline_interrupt =
        DeadlineInterrupt::spawn(deadline.saturating_duration_since(Instant::now()));
    let mut uart = Uart16550::new(0x3f8);
    let mut serial = Vec::new();
    let mut virtio = build_virtio_slots(plan, parsed)?;
    for slot in &mut virtio {
        if has_flag(args, "--virtio-no-indirect") {
            slot.device.block.transport.offered_features.bits &=
                !(VIRTIO_F_RING_INDIRECT_DESC as u128);
        }
        if has_flag(args, "--virtio-no-event-idx") {
            slot.device.block.transport.offered_features.bits &= !(VIRTIO_F_RING_EVENT_IDX as u128);
        }
    }
    let mut exits = 0_u64;
    let mut io_exits = 0_u64;
    let mut mmio_exits = 0_u64;
    let mut unknown_io = 0_u64;
    let mut reboot_count = 0_u32;
    let mut power_action = GuestPowerAction::None;
    let mut mmio_trace = Vec::new();

    while exits < max_exits {
        let now = Instant::now();
        if now >= deadline {
            persist_guest_diagnostics(args, &serial, &mmio_trace, &virtio, "deadline")?;
            return Err(format!("guest execution exceeded {} seconds", timeout.as_secs()).into());
        }
        let result = vcpu.run();
        let exit = match result {
            Ok(exit) => exit,
            Err(error) if error.errno() == libc::EINTR => {
                if halt_is_shutdown
                    && expected_serial
                        .map(|marker| contains_bytes(&serial, marker))
                        .unwrap_or(false)
                    && contains_bytes(&serial, b"System halted")
                {
                    power_action = GuestPowerAction::Shutdown;
                    break;
                }
                persist_guest_diagnostics(args, &serial, &mmio_trace, &virtio, "signal-timeout")?;
                return Err("guest execution timeout interrupted KVM_RUN".into());
            }
            Err(error) => return Err(format!("KVM_RUN failed: {error}").into()),
        };
        exits += 1;
        match exit {
            VcpuExit::IoOut(port, data) => {
                io_exits += 1;
                if (0x3f8..=0x3ff).contains(&port) {
                    for byte in data {
                        uart.write((port - 0x3f8) as u8, *byte)?;
                    }
                    let output = uart.take_tx();
                    if !quiet && !output.is_empty() {
                        std::io::stdout().write_all(&output)?;
                        std::io::stdout().flush()?;
                    }
                    serial.extend_from_slice(&output);
                } else if port == 0xe9 {
                    if !quiet {
                        std::io::stdout().write_all(data)?;
                        std::io::stdout().flush()?;
                    }
                    serial.extend_from_slice(data);
                } else {
                    let value = io_value(data);
                    power_action = plan.power_contract.decode_port_write(port, value);
                    if power_action != GuestPowerAction::None {
                        if matches!(
                            power_action,
                            GuestPowerAction::WarmReboot | GuestPowerAction::ColdReboot
                        ) {
                            reboot_count += 1;
                            if reboot_count > MAX_REBOOTS || has_flag(args, "--exit-on-reboot") {
                                break;
                            }
                            plan.materialize(memory.bytes_mut())?;
                            configure_linux_vcpu(&vcpu, plan)?;
                            for slot in &mut virtio {
                                slot.device.block.transport.reset();
                                vm.set_irq_line(slot.irq, false)?;
                            }
                            power_action = GuestPowerAction::None;
                            continue;
                        }
                        break;
                    }
                    if !((PC_ACPI_PM1_EVENT_PORT..PC_ACPI_PM1_CONTROL_PORT + 2).contains(&port)
                        || (PC_ACPI_PM_TIMER_PORT..PC_ACPI_PM_TIMER_PORT + 4).contains(&port))
                    {
                        unknown_io += 1;
                    }
                }
            }
            VcpuExit::IoIn(port, data) => {
                io_exits += 1;
                if (0x3f8..=0x3ff).contains(&port) {
                    for byte in data {
                        *byte = uart.read((port - 0x3f8) as u8)?;
                    }
                } else if (PC_ACPI_PM1_EVENT_PORT..PC_ACPI_PM1_CONTROL_PORT).contains(&port) {
                    data.fill(0);
                } else if (PC_ACPI_PM1_CONTROL_PORT..PC_ACPI_PM1_CONTROL_PORT + 2).contains(&port) {
                    let encoded = 1_u16.to_le_bytes();
                    for (index, byte) in data.iter_mut().enumerate() {
                        *byte = encoded.get(index).copied().unwrap_or(0);
                    }
                } else if (PC_ACPI_PM_TIMER_PORT..PC_ACPI_PM_TIMER_PORT + 4).contains(&port) {
                    let ticks =
                        (boot_started.elapsed().as_nanos() * 3_579_545 / 1_000_000_000) as u32;
                    let encoded = ticks.to_le_bytes();
                    for (index, byte) in data.iter_mut().enumerate() {
                        *byte = encoded.get(index).copied().unwrap_or(0);
                    }
                } else {
                    data.fill(0xff);
                    unknown_io += 1;
                }
            }
            VcpuExit::MmioRead(address, data) => {
                mmio_exits += 1;
                if let Some(slot) = virtio.iter_mut().find(|slot| {
                    (slot.address..slot.address + VIRTIO_MMIO_BYTES).contains(&address)
                }) {
                    slot.device.read(address - slot.address, data)?;
                    if mmio_trace_enabled && mmio_trace.len() < mmio_trace_limit {
                        mmio_trace.push(mmio_trace_event(
                            exits, "read", address, data, slot, None, None,
                        ));
                    }
                } else {
                    data.fill(0xff);
                }
            }
            VcpuExit::MmioWrite(address, data) => {
                mmio_exits += 1;
                if let Some(slot) = virtio.iter_mut().find(|slot| {
                    (slot.address..slot.address + VIRTIO_MMIO_BYTES).contains(&address)
                }) {
                    let action = slot
                        .device
                        .write(address - slot.address, data, &mut memory)?;
                    slot.completed_requests = slot
                        .completed_requests
                        .saturating_add(action.completed_requests as u64);
                    std::sync::atomic::fence(Ordering::SeqCst);
                    vm.set_irq_line(slot.irq, slot.device.interrupt_pending())?;
                    if mmio_trace_enabled && mmio_trace.len() < mmio_trace_limit {
                        let ring = (action.completed_requests != 0)
                            .then(|| virtio_ring_bytes(slot, memory.bytes_mut()));
                        mmio_trace.push(mmio_trace_event(
                            exits,
                            "write",
                            address,
                            data,
                            slot,
                            Some(action),
                            ring,
                        ));
                    }
                }
            }
            VcpuExit::SystemEvent(kind, _) => {
                power_action = match kind {
                    KVM_SYSTEM_EVENT_SHUTDOWN => GuestPowerAction::Shutdown,
                    KVM_SYSTEM_EVENT_RESET => GuestPowerAction::ColdReboot,
                    KVM_SYSTEM_EVENT_CRASH => GuestPowerAction::Crash,
                    _ => GuestPowerAction::Crash,
                };
                break;
            }
            VcpuExit::Shutdown => {
                power_action = GuestPowerAction::Shutdown;
                break;
            }
            VcpuExit::Hlt => {
                let marker_observed = expected_serial
                    .map(|marker| !marker.is_empty() && contains_bytes(&serial, marker))
                    .unwrap_or(false);
                if halt_is_shutdown && marker_observed {
                    power_action = GuestPowerAction::Shutdown;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            VcpuExit::Intr | VcpuExit::IrqWindowOpen | VcpuExit::SetTpr | VcpuExit::TprAccess => {}
            VcpuExit::FailEntry(reason, cpu) => {
                return Err(format!("KVM vCPU {cpu} failed entry: {reason:#x}").into())
            }
            VcpuExit::InternalError | VcpuExit::Exception | VcpuExit::Unknown => {
                return Err(format!("fatal KVM guest exit: {exit:?}").into())
            }
            other => return Err(format!("unsupported KVM guest exit: {other:?}").into()),
        }
    }
    if exits >= max_exits && power_action == GuestPowerAction::None {
        persist_guest_diagnostics(args, &serial, &mmio_trace, &virtio, "max-exits")?;
        return Err(format!("guest exceeded --max-exits {max_exits}").into());
    }
    persist_writable_disks(&virtio)?;
    persist_guest_diagnostics(args, &serial, &mmio_trace, &virtio, "complete")?;
    let dirty = vm.get_dirty_log(0, memory_len)?;
    let dirty_pages = dirty
        .iter()
        .map(|word| word.count_ones() as u64)
        .sum::<u64>();
    Ok(json!({
        "backend": "linux-kvm-direct",
        "qemu_used": false,
        "architecture": architecture_name(plan.architecture),
        "boot_method": boot_method_name(plan.method),
        "power_action": format!("{:?}", power_action),
        "clean_stop": matches!(power_action, GuestPowerAction::Shutdown | GuestPowerAction::WarmReboot | GuestPowerAction::ColdReboot),
        "reboots": reboot_count,
        "exits": exits,
        "io_exits": io_exits,
        "mmio_exits": mmio_exits,
        "mmio_trace_events": mmio_trace.len(),
        "unknown_io_exits": unknown_io,
        "dirty_pages": dirty_pages,
        "serial_bytes": serial.len(),
        "serial_sha256": sha256_bytes(&serial),
        "expected_serial": expected_serial_text,
        "expected_serial_observed": expected_serial
            .map(|marker| contains_bytes(&serial, marker))
            .unwrap_or(true),
        "virtio": virtio.iter().map(|slot| json!({
            "address": slot.address,
            "irq": slot.irq,
            "requests": slot.completed_requests,
            "disk_generation": slot.device.block.backend.generation,
            "read_bytes": slot.device.block.backend.read_bytes,
            "written_bytes": slot.device.block.backend.written_bytes,
            "flush_count": slot.device.block.backend.flush_count
        })).collect::<Vec<_>>()
    }))
}

fn mmio_trace_event(
    exit: u64,
    operation: &str,
    address: u64,
    bytes: &[u8],
    slot: &VirtioSlot,
    action: Option<ncc_machine::VirtioMmioAction>,
    ring: Option<Value>,
) -> Value {
    let queue = &slot.device.block.transport.queues[0];
    json!({
        "exit": exit,
        "operation": operation,
        "address": address,
        "offset": address - slot.address,
        "width": bytes.len(),
        "value": bytes.iter().enumerate().fold(0_u64, |value, (index, byte)| {
            value | ((*byte as u64) << (index * 8))
        }),
        "device_status": slot.device.block.transport.device_status,
        "isr_status": slot.device.block.transport.isr_status,
        "irq_asserted": slot.device.interrupt_pending(),
        "queue": {
            "ready": queue.ready,
            "size": queue.size,
            "descriptor": queue.descriptor_address,
            "driver": queue.driver_address,
            "device": queue.device_address,
            "last_available_index": queue.last_available_index,
            "used_index": queue.used_index
        },
        "action": action.map(|action| json!({
            "queue": action.queue,
            "completed_requests": action.completed_requests,
            "raise_interrupt": action.raise_interrupt,
            "reset": action.reset
        })),
        "ring_bytes": ring
    })
}

fn virtio_ring_bytes(slot: &VirtioSlot, memory: &[u8]) -> Value {
    let queue = &slot.device.block.transport.queues[0];
    let descriptors = guest_region(memory, queue.descriptor_address, 64);
    let indirect = decode_indirect_descriptor_evidence(memory, &descriptors);
    json!({
        "descriptors": descriptors,
        "indirect": indirect,
        "available": guest_region(memory, queue.driver_address, 32),
        "used": guest_region(memory, queue.device_address, 32)
    })
}

fn decode_indirect_descriptor_evidence(memory: &[u8], descriptor: &[u8]) -> Value {
    if descriptor.len() < 16 {
        return Value::Null;
    }
    let address = u64::from_le_bytes(descriptor[0..8].try_into().expect("descriptor address"));
    let length = u32::from_le_bytes(descriptor[8..12].try_into().expect("descriptor length"));
    let flags = u16::from_le_bytes(descriptor[12..14].try_into().expect("descriptor flags"));
    if flags & ncc_machine::VIRTQ_DESC_F_INDIRECT == 0 || length == 0 || length % 16 != 0 {
        return Value::Null;
    }
    let table = guest_region(
        memory,
        address,
        usize::try_from(length.min(256)).unwrap_or(256),
    );
    let elements = table
        .chunks_exact(16)
        .map(|element| {
            let address = u64::from_le_bytes(element[0..8].try_into().expect("element address"));
            let length = u32::from_le_bytes(element[8..12].try_into().expect("element length"));
            let flags = u16::from_le_bytes(element[12..14].try_into().expect("element flags"));
            let next = u16::from_le_bytes(element[14..16].try_into().expect("element next"));
            json!({
                "address": address,
                "length": length,
                "flags": flags,
                "next": next,
                "bytes": guest_region(memory, address, usize::try_from(length.min(32)).unwrap_or(32))
            })
        })
        .collect::<Vec<_>>();
    json!({
        "address": address,
        "length": length,
        "table": table,
        "elements": elements
    })
}

fn guest_region(memory: &[u8], address: u64, length: usize) -> Vec<u8> {
    let Ok(start) = usize::try_from(address) else {
        return Vec::new();
    };
    let Some(end) = start.checked_add(length) else {
        return Vec::new();
    };
    memory.get(start..end).map_or_else(Vec::new, <[u8]>::to_vec)
}

fn persist_guest_diagnostics(
    args: &[String],
    serial: &[u8],
    mmio_trace: &[Value],
    virtio: &[VirtioSlot],
    termination: &str,
) -> Result<(), Box<dyn Error>> {
    if let Some(path) = option_value(args, "--serial-out") {
        fs::write(path, serial)?;
    }
    if let Some(path) = option_value(args, "--mmio-trace-out") {
        let report = json!({
            "schema": "ncc.kvm.mmio-trace.v1",
            "termination": termination,
            "events": mmio_trace,
            "virtio": virtio.iter().map(|slot| {
                let queue = &slot.device.block.transport.queues[0];
                json!({
                    "address": slot.address,
                    "irq": slot.irq,
                    "completed_requests": slot.completed_requests,
                    "interrupt_pending": slot.device.interrupt_pending(),
                    "device_status": slot.device.block.transport.device_status,
                    "queue_last_available_index": queue.last_available_index,
                    "queue_used_index": queue.used_index,
                    "read_bytes": slot.device.block.backend.read_bytes,
                    "written_bytes": slot.device.block.backend.written_bytes
                })
            }).collect::<Vec<_>>()
        });
        fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn configure_linux_vcpu(vcpu: &VcpuFd, plan: &GuestBootPlan) -> Result<(), Box<dyn Error>> {
    let linux = plan.linux.as_ref().ok_or("missing Linux boot handoff")?;
    let mut special = vcpu.get_sregs()?;
    special.gdt.base = ncc_machine::LINUX_GDT_ADDRESS;
    special.gdt.limit = 23;
    special.cs = flat_segment(0x10, true);
    special.ds = flat_segment(0x18, false);
    special.es = special.ds;
    special.fs = special.ds;
    special.gs = special.ds;
    special.ss = special.ds;
    special.cr0 |= 1;
    special.cr4 = 0;
    special.efer = 0;
    vcpu.set_sregs(&special)?;
    let mut registers = vcpu.get_regs()?;
    registers.rax = 0;
    registers.rbx = 0;
    registers.rcx = 0;
    registers.rdx = 0;
    registers.rsi = linux.boot_params_address;
    registers.rdi = 0;
    registers.rsp = ncc_machine::LINUX_STACK_TOP;
    registers.rbp = 0;
    registers.rip = linux.code32_start;
    registers.rflags = 2;
    vcpu.set_regs(&registers)?;
    Ok(())
}

fn flat_segment(selector: u16, code: bool) -> kvm_segment {
    kvm_segment {
        base: 0,
        limit: u32::MAX,
        selector,
        type_: if code { 11 } else { 3 },
        present: 1,
        dpl: 0,
        db: 1,
        s: 1,
        l: 0,
        g: 1,
        avl: 0,
        unusable: 0,
        padding: 0,
    }
}

fn build_virtio_slots(
    plan: &GuestBootPlan,
    parsed: &ParsedGuest,
) -> Result<Vec<VirtioSlot>, Box<dyn Error>> {
    plan.disks
        .iter()
        .zip(&parsed.disk_paths)
        .enumerate()
        .map(|(index, (disk, source_path))| {
            let backend = VirtualBlockDevice::from_bytes(
                disk.bytes.clone(),
                disk.block_size,
                disk.read_only,
            )?;
            let block = VirtioBlockDevice::new(backend, disk.id.as_bytes())?;
            Ok(VirtioSlot {
                address: VIRTIO_MMIO_BASE + index as u64 * VIRTIO_MMIO_BYTES,
                irq: VIRTIO_IRQ_BASE + index as u32,
                source_path: source_path.clone(),
                read_only: disk.read_only,
                device: VirtioMmioBlockDevice::new(block)?,
                completed_requests: 0,
            })
        })
        .collect()
}

fn io_value(bytes: &[u8]) -> u32 {
    let mut value = [0u8; 4];
    let count = bytes.len().min(value.len());
    value[..count].copy_from_slice(&bytes[..count]);
    u32::from_le_bytes(value)
}

fn persist_writable_disks(slots: &[VirtioSlot]) -> Result<(), Box<dyn Error>> {
    for slot in slots {
        if slot.read_only || !slot.device.block.backend.dirty {
            continue;
        }
        let temporary = slot
            .source_path
            .with_extension(format!("ncc-{}.tmp", std::process::id()));
        let mut file = fs::File::create(&temporary)?;
        file.write_all(slot.device.block.backend.bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, &slot.source_path)?;
    }
    Ok(())
}
