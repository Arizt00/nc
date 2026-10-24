use core::fmt;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

use crate::{
    build_test_elf64, run_vm_exit_loop, CoreSyscall, DeviceSnapshot, DmaMapping, DmaPermissions,
    GuestArchitecture, GuestPhysicalAddress, GuestRegisters, IommuArchitecture, IommuDeviceId,
    IommuDomainId, KernelCore, MemorySnapshot, PendingVmEvents, RegisterBank, SnapshotId,
    SnapshotLimits, TlbShootdown, VcpuSnapshot, VirtualDeviceClass, VirtualIommu, VmEvent,
    VmExecutionEngine, VmExit, VmExitAction, VmExitError, VmExitHandler, VmExitKind,
    VmLifecycleError, VmLoopStopReason, VmPowerState, VmSnapshot, VmVendor, X86ExceptionVector,
};

const STRESS_PAGE_SIZE: usize = 4096;
const STRESS_ELF_MACHINE_X86_64: u16 = 62;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelStressProfile {
    Smoke,
    Ci,
    Endurance,
}

impl KernelStressProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Ci => "ci",
            Self::Endurance => "endurance",
        }
    }

    pub const fn config(self) -> KernelStressConfig {
        match self {
            Self::Smoke => KernelStressConfig {
                profile: self,
                seed: 0x4e43_5354_5245_5353,
                vm_exits: 20_000,
                smp_vcpus: 4,
                smp_iterations_per_vcpu: 10_000,
                cow_fault_rounds: 64,
                dma_rounds: 10_000,
                interrupt_events: 1_024,
                snapshot_rounds: 16,
                snapshot_memory_bytes: 256 * 1024,
            },
            Self::Ci => KernelStressConfig {
                profile: self,
                seed: 0x4e43_4349_5354_5253,
                vm_exits: 1_000_000,
                smp_vcpus: 8,
                smp_iterations_per_vcpu: 250_000,
                cow_fault_rounds: 1_024,
                dma_rounds: 500_000,
                interrupt_events: 65_536,
                snapshot_rounds: 256,
                snapshot_memory_bytes: 4 * 1024 * 1024,
            },
            Self::Endurance => KernelStressConfig {
                profile: self,
                seed: 0x4e43_454e_4455_5245,
                vm_exits: 25_000_000,
                smp_vcpus: 16,
                smp_iterations_per_vcpu: 5_000_000,
                cow_fault_rounds: 16_384,
                dma_rounds: 10_000_000,
                interrupt_events: 1_000_000,
                snapshot_rounds: 4_096,
                snapshot_memory_bytes: 64 * 1024 * 1024,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelStressConfig {
    pub profile: KernelStressProfile,
    pub seed: u64,
    pub vm_exits: u64,
    pub smp_vcpus: usize,
    pub smp_iterations_per_vcpu: u64,
    pub cow_fault_rounds: u64,
    pub dma_rounds: u64,
    pub interrupt_events: u64,
    pub snapshot_rounds: u64,
    pub snapshot_memory_bytes: usize,
}

impl KernelStressConfig {
    pub fn validate(self) -> Result<Self, KernelStressError> {
        if self.vm_exits == 0
            || self.smp_iterations_per_vcpu == 0
            || self.cow_fault_rounds == 0
            || self.dma_rounds == 0
            || self.interrupt_events < 3
            || self.snapshot_rounds == 0
        {
            return Err(KernelStressError::InvalidConfig(
                "all stress iteration counts must be non-zero and interrupt_events must be at least 3",
            ));
        }
        if self.smp_vcpus < 2 || self.smp_vcpus > 64 {
            return Err(KernelStressError::InvalidConfig(
                "smp_vcpus must be between 2 and 64",
            ));
        }
        if self.snapshot_memory_bytes < STRESS_PAGE_SIZE
            || self.snapshot_memory_bytes % STRESS_PAGE_SIZE != 0
        {
            return Err(KernelStressError::InvalidConfig(
                "snapshot memory must be a non-zero multiple of 4096 bytes",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelStressMetric {
    pub scenario: &'static str,
    pub operations: u64,
    pub elapsed_ns: u128,
    pub checksum: u64,
    pub rejected_operations: u64,
}

impl KernelStressMetric {
    pub fn operations_per_second(&self) -> u64 {
        if self.elapsed_ns == 0 {
            return self.operations;
        }
        ((self.operations as u128) * 1_000_000_000u128 / self.elapsed_ns).min(u64::MAX as u128)
            as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelStressReport {
    pub config: KernelStressConfig,
    pub metrics: Vec<KernelStressMetric>,
    pub total_operations: u64,
    pub elapsed_ns: u128,
    pub checksum: u64,
    pub assertions: u64,
}

#[derive(Debug)]
pub enum KernelStressError {
    InvalidConfig(&'static str),
    Scenario { name: &'static str, message: String },
}

impl KernelStressError {
    fn scenario(name: &'static str, error: impl fmt::Display) -> Self {
        Self::Scenario {
            name,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for KernelStressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(f, "invalid kernel stress config: {message}"),
            Self::Scenario { name, message } => {
                write!(f, "kernel stress scenario '{name}' failed: {message}")
            }
        }
    }
}

impl std::error::Error for KernelStressError {}

fn mix(checksum: u64, value: u64) -> u64 {
    checksum.rotate_left(17) ^ value.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0xd1b5_4a32_d192_ed03
}

#[derive(Debug, Clone)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

#[derive(Debug)]
struct StormEngine {
    cursor: u64,
    registers: GuestRegisters,
    injected: Vec<u8>,
}

impl StormEngine {
    fn new() -> Self {
        Self {
            cursor: 0,
            registers: GuestRegisters {
                rflags: 2,
                ..GuestRegisters::default()
            },
            injected: Vec::new(),
        }
    }
}

impl VmExecutionEngine for StormEngine {
    fn enter(&mut self) -> Result<VmExit, VmExitError> {
        let index = self.cursor;
        self.cursor = self.cursor.wrapping_add(1);
        let kind = match index % 10 {
            0 => VmExitKind::Cpuid,
            1 => VmExitKind::PortIo,
            2 => VmExitKind::MsrRead,
            3 => VmExitKind::MsrWrite,
            4 => VmExitKind::EptViolation,
            5 => VmExitKind::Hypercall,
            6 => VmExitKind::ExternalInterrupt,
            7 => VmExitKind::ExceptionOrNmi,
            8 => VmExitKind::Pause,
            _ => VmExitKind::PreemptionTimer,
        };
        Ok(VmExit {
            vendor: if index & 1 == 0 {
                VmVendor::Intel
            } else {
                VmVendor::Amd
            },
            kind,
            entry_failure: false,
            raw_reason: index,
            mnemonic: "stress",
            qualification: index.rotate_left(7),
            guest_physical_address: (kind == VmExitKind::EptViolation)
                .then_some(GuestPhysicalAddress::new(0x4000 + ((index & 0xfff) << 12))),
            guest_linear_address: Some(0x7fff_0000 + ((index & 0xffff) << 4)),
            instruction_length: 1 + (index % 3) as u32,
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
        self.registers = registers;
        Ok(())
    }

    fn advance_rip(&mut self, instruction_length: u32) -> Result<(), VmExitError> {
        self.registers.rip = self.registers.rip.wrapping_add(instruction_length as u64);
        Ok(())
    }

    fn inject_event(&mut self, event: VmEvent) -> Result<(), VmExitError> {
        self.injected.push(event.vector);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct StormHandler {
    checksum: u64,
    handled: u64,
}

impl<const EVENTS: usize> VmExitHandler<StormEngine, EVENTS> for StormHandler {
    fn handle(
        &mut self,
        _engine: &mut StormEngine,
        exit: &VmExit,
        _pending: &mut PendingVmEvents<EVENTS>,
    ) -> Result<VmExitAction, VmExitError> {
        self.handled = self.handled.wrapping_add(1);
        self.checksum = mix(
            self.checksum,
            exit.raw_reason ^ ((exit.kind as u64) << 48) ^ exit.qualification,
        );
        Ok(VmExitAction::Advance)
    }
}

fn run_vm_exit_storm(config: KernelStressConfig) -> Result<KernelStressMetric, KernelStressError> {
    let started = Instant::now();
    let mut engine = StormEngine::new();
    let mut handler = StormHandler::default();
    let mut pending = PendingVmEvents::<16>::new();
    pending
        .push(VmEvent::external_interrupt(0x40))
        .map_err(|error| KernelStressError::scenario("vm-exit-storm", error))?;
    pending
        .push(VmEvent::exception(X86ExceptionVector::PageFault, Some(0)))
        .map_err(|error| KernelStressError::scenario("vm-exit-storm", error))?;
    pending
        .push(VmEvent::nmi())
        .map_err(|error| KernelStressError::scenario("vm-exit-storm", error))?;
    let outcome = run_vm_exit_loop(&mut engine, &mut handler, &mut pending, config.vm_exits)
        .map_err(|error| KernelStressError::scenario("vm-exit-storm", error))?;
    if outcome.stop_reason != VmLoopStopReason::ExitBudgetExhausted
        || outcome.exits != config.vm_exits
        || outcome.advanced_instructions != config.vm_exits
        || handler.handled != config.vm_exits
        || engine.injected.len() != 3
        || engine.injected != [2, 14, 64]
    {
        return Err(KernelStressError::scenario(
            "vm-exit-storm",
            "exit, RIP advance, or priority-ordered injection accounting mismatch",
        ));
    }
    let checksum = engine
        .injected
        .iter()
        .fold(handler.checksum, |state, vector| mix(state, *vector as u64));
    Ok(KernelStressMetric {
        scenario: "vm-exit-storm",
        operations: outcome.exits + outcome.injected_events,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        rejected_operations: 0,
    })
}

fn run_smp_contention(config: KernelStressConfig) -> Result<KernelStressMetric, KernelStressError> {
    let started = Instant::now();
    let barrier = Arc::new(Barrier::new(config.smp_vcpus));
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::with_capacity(config.smp_vcpus);
    for cpu in 0..config.smp_vcpus {
        let barrier = Arc::clone(&barrier);
        let counter = Arc::clone(&counter);
        let iterations = config.smp_iterations_per_vcpu;
        let seed = mix(config.seed, cpu as u64);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let mut checksum = seed;
            for iteration in 0..iterations {
                counter.fetch_add(1, Ordering::SeqCst);
                checksum = mix(checksum, iteration ^ (cpu as u64).rotate_left(32));
            }
            (cpu, checksum)
        }));
    }
    let mut per_cpu = Vec::with_capacity(config.smp_vcpus);
    for handle in handles {
        per_cpu.push(
            handle.join().map_err(|_| {
                KernelStressError::scenario("smp-contention", "vCPU thread panicked")
            })?,
        );
    }
    per_cpu.sort_by_key(|(cpu, _)| *cpu);
    let expected = (config.smp_vcpus as u64)
        .checked_mul(config.smp_iterations_per_vcpu)
        .ok_or_else(|| KernelStressError::scenario("smp-contention", "operation overflow"))?;
    if counter.load(Ordering::SeqCst) != expected {
        return Err(KernelStressError::scenario(
            "smp-contention",
            "atomic shared counter lost updates",
        ));
    }
    let mut shootdown = TlbShootdown::new(
        7,
        0xffff_8000_0000_0000,
        STRESS_PAGE_SIZE as u64,
        1,
        config.smp_vcpus,
        0,
    )
    .map_err(|error| KernelStressError::scenario("smp-contention", error))?;
    for cpu in 1..config.smp_vcpus {
        shootdown
            .acknowledge(cpu as u32, 7)
            .map_err(|error| KernelStressError::scenario("smp-contention", error))?;
    }
    if !shootdown.complete() {
        return Err(KernelStressError::scenario(
            "smp-contention",
            "TLB shootdown did not receive every remote acknowledgement",
        ));
    }
    let checksum = per_cpu
        .into_iter()
        .fold(0, |state, (cpu, value)| mix(state, value ^ cpu as u64));
    Ok(KernelStressMetric {
        scenario: "smp-contention",
        operations: expected + config.smp_vcpus.saturating_sub(1) as u64,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        rejected_operations: 0,
    })
}

fn run_cow_fault_storm(
    config: KernelStressConfig,
) -> Result<KernelStressMetric, KernelStressError> {
    let started = Instant::now();
    let frame_count = (config.cow_fault_rounds.min(16_384) as usize).max(256) + 128;
    let mut kernel = KernelCore::new(frame_count, config.smp_vcpus)
        .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
    let elf = build_test_elf64(STRESS_ELF_MACHINE_X86_64, 0x400000, &[0x31, 0xc0, 0xc3]);
    let parent = kernel
        .load_process(
            "stress-init",
            &elf,
            Some(STRESS_ELF_MACHINE_X86_64),
            Some(0),
        )
        .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
    kernel
        .syscall(
            parent,
            CoreSyscall::MapAnonymous as u32,
            [0x600000, 0, 0, 0, 0, 0],
        )
        .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
    let parent_payload = config.seed.to_le_bytes();
    kernel
        .copy_to_user(parent, 0x600000, &parent_payload)
        .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
    let mut checksum = 0;
    let mut rejected = 0;
    for round in 0..config.cow_fault_rounds {
        let child = kernel
            .fork_cow(parent)
            .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
        let child_payload = mix(config.seed, round).to_le_bytes();
        kernel
            .copy_to_user(child, 0x600000, &child_payload)
            .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
        let mut observed_parent = [0u8; 8];
        let mut observed_child = [0u8; 8];
        kernel
            .copy_from_user(parent, 0x600000, &mut observed_parent)
            .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
        kernel
            .copy_from_user(child, 0x600000, &mut observed_child)
            .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
        if observed_parent != parent_payload || observed_child != child_payload {
            return Err(KernelStressError::scenario(
                "cow-page-faults",
                "COW isolation changed parent or lost child data",
            ));
        }
        let mut inaccessible = [0u8; 1];
        if kernel
            .copy_from_user(child, 0xdead_0000, &mut inaccessible)
            .is_err()
        {
            rejected += 1;
        } else {
            return Err(KernelStressError::scenario(
                "cow-page-faults",
                "unmapped user page was unexpectedly readable",
            ));
        }
        kernel
            .exit_process(child, 0)
            .map_err(|error| KernelStressError::scenario("cow-page-faults", error))?;
        checksum = mix(
            checksum,
            u64::from_le_bytes(observed_child) ^ u64::from(child),
        );
    }
    Ok(KernelStressMetric {
        scenario: "cow-page-faults",
        operations: config.cow_fault_rounds.saturating_mul(6),
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        rejected_operations: rejected,
    })
}

fn run_invalid_dma_storm(
    config: KernelStressConfig,
) -> Result<KernelStressMetric, KernelStressError> {
    let started = Instant::now();
    let mut iommu =
        VirtualIommu::<4, 2, 8, 8>::new(IommuArchitecture::SoftwareViommu, IommuDomainId(7));
    let device = IommuDeviceId::pci(0, 0, 3, 0)
        .ok_or_else(|| KernelStressError::scenario("invalid-dma", "invalid PCI identity"))?;
    iommu
        .domain
        .attach_device(device)
        .map_err(|error| KernelStressError::scenario("invalid-dma", error))?;
    iommu
        .domain
        .map(DmaMapping {
            iova: 0x1000,
            host_virtual_address: 0x1000_0000,
            host_physical_address: Some(0x2000_0000),
            length: STRESS_PAGE_SIZE as u64,
            page_size: STRESS_PAGE_SIZE as u64,
            permissions: DmaPermissions::DEVICE_READ,
            pinned: true,
            coherent: true,
        })
        .map_err(|error| KernelStressError::scenario("invalid-dma", error))?;
    let mut checksum = 0;
    let mut rejected = 0;
    for round in 0..config.dma_rounds {
        let offset = round & (STRESS_PAGE_SIZE as u64 - 1);
        let valid = iommu
            .dma(device, 0x1000 + offset, DmaPermissions::DEVICE_READ)
            .map_err(|error| KernelStressError::scenario("invalid-dma", error))?;
        checksum = mix(checksum, valid.host_virtual_address ^ valid.remaining);
        if iommu
            .dma(device, 0x1000 + offset, DmaPermissions::DEVICE_WRITE)
            .is_ok()
        {
            return Err(KernelStressError::scenario(
                "invalid-dma",
                "write through read-only IOVA was accepted",
            ));
        }
        let denied = iommu
            .faults
            .pop()
            .ok_or_else(|| KernelStressError::scenario("invalid-dma", "missing write fault"))?;
        checksum = mix(checksum, denied.iova ^ u64::from(denied.write));
        rejected += 1;
        if iommu
            .dma(device, 0x8000 + offset, DmaPermissions::DEVICE_READ)
            .is_ok()
        {
            return Err(KernelStressError::scenario(
                "invalid-dma",
                "unmapped IOVA was accepted",
            ));
        }
        let unmapped = iommu
            .faults
            .pop()
            .ok_or_else(|| KernelStressError::scenario("invalid-dma", "missing unmapped fault"))?;
        checksum = mix(checksum, unmapped.iova);
        rejected += 1;
    }
    if !iommu.faults.is_empty() {
        return Err(KernelStressError::scenario(
            "invalid-dma",
            "fault queue did not drain",
        ));
    }
    Ok(KernelStressMetric {
        scenario: "invalid-dma",
        operations: config.dma_rounds.saturating_mul(3),
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        rejected_operations: rejected,
    })
}

fn run_interrupt_storm(
    config: KernelStressConfig,
) -> Result<KernelStressMetric, KernelStressError> {
    const CAPACITY: usize = 128;
    let started = Instant::now();
    let mut remaining = config.interrupt_events;
    let mut checksum = 0;
    let mut delivered = 0;
    while remaining != 0 {
        let batch = remaining.min(CAPACITY as u64) as usize;
        let mut pending = PendingVmEvents::<CAPACITY>::new();
        pending
            .push(VmEvent::nmi())
            .map_err(|error| KernelStressError::scenario("simultaneous-interrupts", error))?;
        if batch > 1 {
            pending
                .push(VmEvent::exception(X86ExceptionVector::PageFault, Some(0)))
                .map_err(|error| KernelStressError::scenario("simultaneous-interrupts", error))?;
        }
        for index in 2..batch {
            pending
                .push(VmEvent::external_interrupt(32 + (index % 224) as u8))
                .map_err(|error| KernelStressError::scenario("simultaneous-interrupts", error))?;
        }
        let mut previous_priority = 0;
        let mut first = true;
        while let Some(event) = pending.pop_highest_priority() {
            let priority = match event.kind {
                crate::VmEventKind::NonMaskableInterrupt => 0,
                crate::VmEventKind::HardwareException => 1,
                crate::VmEventKind::ExternalInterrupt => 2,
                _ => 3,
            };
            if !first && priority < previous_priority {
                return Err(KernelStressError::scenario(
                    "simultaneous-interrupts",
                    "event priority order regressed",
                ));
            }
            first = false;
            previous_priority = priority;
            checksum = mix(checksum, ((priority as u64) << 8) | event.vector as u64);
            delivered += 1;
            pending.complete_delivery();
        }
        remaining -= batch as u64;
    }
    if delivered != config.interrupt_events {
        return Err(KernelStressError::scenario(
            "simultaneous-interrupts",
            "not every queued interrupt was delivered",
        ));
    }
    Ok(KernelStressMetric {
        scenario: "simultaneous-interrupts",
        operations: delivered,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        rejected_operations: 0,
    })
}

fn snapshot_vcpus(count: usize, generation: u64) -> Vec<VcpuSnapshot> {
    (0..count)
        .map(|id| {
            let mut registers = RegisterBank::new(GuestArchitecture::X86_64);
            registers.set("rip", 0x1000u128 + generation as u128 * 4 + id as u128);
            registers.set("rsp", 0x8000u128 - id as u128 * 0x100);
            VcpuSnapshot {
                id: id as u32,
                registers,
                halted: false,
                pending_interrupts: vec![32 + id as u32],
                virtual_time_ns: generation * 1_000 + id as u64,
            }
        })
        .collect()
}

fn build_snapshot(
    config: KernelStressConfig,
    id: SnapshotId,
    parent: SnapshotId,
    generation: u64,
    memory: MemorySnapshot,
) -> VmSnapshot {
    let mut metadata = BTreeMap::new();
    metadata.insert("profile".into(), config.profile.name().into());
    metadata.insert("generation".into(), generation.to_string());
    VmSnapshot {
        id,
        parent,
        generation,
        timestamp_ns: generation * 1_000_000,
        state: VmPowerState::Paused,
        architecture: GuestArchitecture::X86_64,
        target: "x86_64-unknown-none".into(),
        vcpus: snapshot_vcpus(config.smp_vcpus, generation),
        memory,
        devices: vec![DeviceSnapshot::new(
            "virtio-block0",
            VirtualDeviceClass::VirtioBlock,
            1,
            true,
            generation.to_le_bytes().to_vec(),
        )],
        metadata,
    }
}

fn run_snapshot_restore_storm(
    config: KernelStressConfig,
) -> Result<KernelStressMetric, KernelStressError> {
    let started = Instant::now();
    let mut rng = DeterministicRng::new(config.seed ^ 0x534e_4150_5348_4f54);
    let mut memory = vec![0u8; config.snapshot_memory_bytes];
    for chunk in memory.chunks_mut(8) {
        let bytes = rng.next().to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    let first_id = SnapshotId::from_sequence(config.seed, 1);
    let full = MemorySnapshot::capture_full(&memory, STRESS_PAGE_SIZE)
        .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
    let full_snapshot = build_snapshot(config, first_id, SnapshotId::NIL, 1, full);
    let encoded = full_snapshot
        .encode()
        .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
    let decoded = VmSnapshot::decode(&encoded, SnapshotLimits::default())
        .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
    let restored = decoded
        .restore_memory(None)
        .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
    if restored != memory {
        return Err(KernelStressError::scenario(
            "snapshot-restore",
            "full snapshot restore mismatch",
        ));
    }
    let mut parent_id = first_id;
    let mut checksum = mix(0, encoded.len() as u64);
    let mut encoded_bytes = encoded.len() as u64;
    for round in 0..config.snapshot_rounds {
        let mut next = memory.clone();
        let page = (rng.next() as usize) % (next.len() / STRESS_PAGE_SIZE);
        let offset = page * STRESS_PAGE_SIZE + (rng.next() as usize % (STRESS_PAGE_SIZE - 64));
        for byte in &mut next[offset..offset + 64] {
            *byte ^= rng.next() as u8;
        }
        let incremental = MemorySnapshot::capture_incremental(&memory, &next, STRESS_PAGE_SIZE)
            .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
        let id = SnapshotId::from_sequence(config.seed, round + 2);
        let snapshot = build_snapshot(config, id, parent_id, round + 2, incremental);
        let bytes = snapshot
            .encode()
            .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
        let decoded = VmSnapshot::decode(&bytes, SnapshotLimits::default())
            .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
        let restored = decoded
            .restore_memory(Some(&memory))
            .map_err(|error| KernelStressError::scenario("snapshot-restore", error))?;
        if restored != next
            || decoded.parent != parent_id
            || decoded.vcpus.len() != config.smp_vcpus
        {
            return Err(KernelStressError::scenario(
                "snapshot-restore",
                "incremental snapshot lost memory, ancestry, or vCPU topology",
            ));
        }
        checksum = mix(checksum, bytes.len() as u64 ^ page as u64 ^ (round + 2));
        encoded_bytes = encoded_bytes.saturating_add(bytes.len() as u64);
        memory = next;
        parent_id = id;
    }
    Ok(KernelStressMetric {
        scenario: "snapshot-restore",
        operations: config.snapshot_rounds + 1,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum: mix(checksum, encoded_bytes),
        rejected_operations: 0,
    })
}

pub fn run_kernel_stress(
    config: KernelStressConfig,
) -> Result<KernelStressReport, KernelStressError> {
    let config = config.validate()?;
    let started = Instant::now();
    let metrics = vec![
        run_vm_exit_storm(config)?,
        run_smp_contention(config)?,
        run_cow_fault_storm(config)?,
        run_invalid_dma_storm(config)?,
        run_interrupt_storm(config)?,
        run_snapshot_restore_storm(config)?,
    ];
    let total_operations = metrics.iter().fold(0u64, |total, metric| {
        total.saturating_add(metric.operations)
    });
    let checksum = metrics.iter().fold(config.seed, |state, metric| {
        mix(state, metric.checksum ^ metric.operations)
    });
    Ok(KernelStressReport {
        config,
        metrics,
        total_operations,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        assertions: 18,
    })
}

impl From<VmLifecycleError> for KernelStressError {
    fn from(error: VmLifecycleError) -> Self {
        Self::scenario("snapshot-restore", error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_profile_executes_every_resistance_family() {
        let report = run_kernel_stress(KernelStressProfile::Smoke.config()).unwrap();
        assert_eq!(report.metrics.len(), 6);
        let expected_operations = report.config.vm_exits
            + 3
            + report.config.smp_vcpus as u64 * report.config.smp_iterations_per_vcpu
            + report.config.smp_vcpus as u64
            - 1
            + report.config.cow_fault_rounds * 6
            + report.config.dma_rounds * 3
            + report.config.interrupt_events
            + report.config.snapshot_rounds
            + 1;
        assert_eq!(report.total_operations, expected_operations);
        assert!(report.metrics.iter().all(|metric| metric.operations != 0));
        assert_eq!(
            report
                .metrics
                .iter()
                .map(|metric| metric.scenario)
                .collect::<Vec<_>>(),
            vec![
                "vm-exit-storm",
                "smp-contention",
                "cow-page-faults",
                "invalid-dma",
                "simultaneous-interrupts",
                "snapshot-restore",
            ]
        );
        assert_eq!(
            report
                .metrics
                .iter()
                .find(|metric| metric.scenario == "cow-page-faults")
                .unwrap()
                .rejected_operations,
            report.config.cow_fault_rounds
        );
        assert_eq!(
            report
                .metrics
                .iter()
                .find(|metric| metric.scenario == "invalid-dma")
                .unwrap()
                .rejected_operations,
            report.config.dma_rounds * 2
        );
    }

    #[test]
    fn invalid_profiles_are_rejected_before_work_starts() {
        let mut config = KernelStressProfile::Smoke.config();
        config.smp_vcpus = 1;
        assert!(matches!(
            run_kernel_stress(config),
            Err(KernelStressError::InvalidConfig(_))
        ));
    }
}
