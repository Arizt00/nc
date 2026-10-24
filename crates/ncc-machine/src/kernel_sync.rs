use core::hint::spin_loop;
use core::sync::atomic::{compiler_fence, fence, AtomicU32, AtomicU64, Ordering};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::thread;

pub const KERNEL_SYNC_SCHEMA: &str = "ncc.machine.kernel_sync.v1";
const WRITER_BIT: u32 = 1 << 31;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelSyncError {
    Contended,
    SpinLimit,
    InvalidTicket { expected: u32, actual: u32 },
    ReaderUnderflow,
    WriterMissing,
    QueueFull,
    QueueEmpty,
    CpuOutOfRange(usize),
    RcuTimeout,
    InvalidOwner { expected: u32, actual: u32 },
    UnsupportedTarget(String),
}

impl fmt::Display for KernelSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contended => f.write_str("kernel synchronization primitive is contended"),
            Self::SpinLimit => f.write_str("kernel synchronization spin limit was reached"),
            Self::InvalidTicket { expected, actual } => {
                write!(
                    f,
                    "invalid spin ticket {actual}, current owner is {expected}"
                )
            }
            Self::ReaderUnderflow => f.write_str("kernel RW lock reader count underflow"),
            Self::WriterMissing => f.write_str("kernel RW lock has no writer"),
            Self::QueueFull => f.write_str("caller-owned wait queue is full"),
            Self::QueueEmpty => f.write_str("caller-owned wait queue is empty"),
            Self::CpuOutOfRange(cpu) => write!(f, "CPU {cpu} is outside the RCU domain"),
            Self::RcuTimeout => f.write_str("RCU grace period did not complete"),
            Self::InvalidOwner { expected, actual } => {
                write!(f, "mutex owner is {actual}, expected {expected}")
            }
            Self::UnsupportedTarget(target) => {
                write!(f, "target '{target}' has no kernel synchronization plan")
            }
        }
    }
}

impl Error for KernelSyncError {}

#[derive(Debug, Default)]
pub struct TicketSpinLock {
    next: AtomicU32,
    owner: AtomicU32,
}

impl TicketSpinLock {
    pub const fn new() -> Self {
        Self {
            next: AtomicU32::new(0),
            owner: AtomicU32::new(0),
        }
    }

    pub fn try_acquire(&self) -> Result<u32, KernelSyncError> {
        let owner = self.owner.load(Ordering::Acquire);
        self.next
            .compare_exchange(
                owner,
                owner.wrapping_add(1),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .map(|_| owner)
            .map_err(|_| KernelSyncError::Contended)
    }

    pub fn acquire_bounded(&self, max_spins: u32) -> Result<u32, KernelSyncError> {
        for _ in 0..max_spins {
            if let Ok(ticket) = self.try_acquire() {
                return Ok(ticket);
            }
            spin_loop();
        }
        Err(KernelSyncError::SpinLimit)
    }

    pub fn acquire(&self) -> u32 {
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);
        while self.owner.load(Ordering::Acquire) != ticket {
            spin_loop();
        }
        ticket
    }

    pub fn release(&self, ticket: u32) -> Result<(), KernelSyncError> {
        let owner = self.owner.load(Ordering::Relaxed);
        if owner != ticket {
            return Err(KernelSyncError::InvalidTicket {
                expected: owner,
                actual: ticket,
            });
        }
        self.owner.store(ticket.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    pub fn queued(&self) -> u32 {
        self.next
            .load(Ordering::Relaxed)
            .wrapping_sub(self.owner.load(Ordering::Relaxed))
    }
}

#[derive(Debug, Default)]
pub struct RawRwLock {
    state: AtomicU32,
}

impl RawRwLock {
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
        }
    }

    pub fn try_read(&self) -> Result<(), KernelSyncError> {
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            if state & WRITER_BIT != 0 || state == WRITER_BIT - 1 {
                return Err(KernelSyncError::Contended);
            }
            match self.state.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(current) => state = current,
            }
        }
    }

    pub fn read_release(&self) -> Result<(), KernelSyncError> {
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            if state == 0 || state & WRITER_BIT != 0 {
                return Err(KernelSyncError::ReaderUnderflow);
            }
            match self.state.compare_exchange_weak(
                state,
                state - 1,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(current) => state = current,
            }
        }
    }

    pub fn try_write(&self) -> Result<(), KernelSyncError> {
        self.state
            .compare_exchange(0, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| KernelSyncError::Contended)
    }

    pub fn write_release(&self) -> Result<(), KernelSyncError> {
        self.state
            .compare_exchange(WRITER_BIT, 0, Ordering::Release, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| KernelSyncError::WriterMissing)
    }

    pub fn readers(&self) -> u32 {
        self.state.load(Ordering::Relaxed) & !WRITER_BIT
    }
}

#[derive(Debug)]
pub struct CallerWaitQueue<'a> {
    slots: &'a mut [u32],
    head: usize,
    len: usize,
}

impl<'a> CallerWaitQueue<'a> {
    pub fn new(slots: &'a mut [u32]) -> Result<Self, KernelSyncError> {
        if slots.is_empty() {
            return Err(KernelSyncError::QueueFull);
        }
        Ok(Self {
            slots,
            head: 0,
            len: 0,
        })
    }

    pub fn push(&mut self, task: u32) -> Result<(), KernelSyncError> {
        if self.len == self.slots.len() {
            return Err(KernelSyncError::QueueFull);
        }
        let tail = (self.head + self.len) % self.slots.len();
        self.slots[tail] = task;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<u32, KernelSyncError> {
        if self.len == 0 {
            return Err(KernelSyncError::QueueEmpty);
        }
        let task = self.slots[self.head];
        self.head = (self.head + 1) % self.slots.len();
        self.len -= 1;
        Ok(task)
    }

    pub const fn len(&self) -> usize {
        self.len
    }
}

#[derive(Debug, Default)]
pub struct KernelMutex {
    owner: AtomicU32,
}

impl KernelMutex {
    pub const UNLOCKED: u32 = u32::MAX;

    pub const fn new() -> Self {
        Self {
            owner: AtomicU32::new(Self::UNLOCKED),
        }
    }

    pub fn try_acquire(&self, task: u32) -> Result<(), KernelSyncError> {
        self.owner
            .compare_exchange(Self::UNLOCKED, task, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| KernelSyncError::Contended)
    }

    pub fn acquire_or_queue(
        &self,
        task: u32,
        queue: &mut CallerWaitQueue<'_>,
    ) -> Result<bool, KernelSyncError> {
        match self.try_acquire(task) {
            Ok(()) => Ok(true),
            Err(KernelSyncError::Contended) => {
                queue.push(task)?;
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub fn release_to_next(
        &self,
        task: u32,
        queue: &mut CallerWaitQueue<'_>,
    ) -> Result<Option<u32>, KernelSyncError> {
        let owner = self.owner.load(Ordering::Relaxed);
        if owner != task {
            return Err(KernelSyncError::InvalidOwner {
                expected: task,
                actual: owner,
            });
        }
        let next = if queue.len() == 0 {
            None
        } else {
            Some(queue.pop()?)
        };
        self.owner
            .store(next.unwrap_or(Self::UNLOCKED), Ordering::Release);
        Ok(next)
    }

    pub fn owner(&self) -> Option<u32> {
        let owner = self.owner.load(Ordering::Relaxed);
        (owner != Self::UNLOCKED).then_some(owner)
    }
}

pub trait InterruptMask {
    type State: Copy;

    fn save_and_disable(&mut self) -> Self::State;
    fn restore(&mut self, state: Self::State);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrqLockToken<S: Copy> {
    pub ticket: u32,
    pub previous_interrupt_state: S,
}

#[derive(Debug, Default)]
pub struct InterruptSafeSpinLock {
    lock: TicketSpinLock,
}

impl InterruptSafeSpinLock {
    pub const fn new() -> Self {
        Self {
            lock: TicketSpinLock::new(),
        }
    }

    pub fn acquire_bounded<M: InterruptMask>(
        &self,
        mask: &mut M,
        max_spins: u32,
    ) -> Result<IrqLockToken<M::State>, KernelSyncError> {
        let previous_interrupt_state = mask.save_and_disable();
        match self.lock.acquire_bounded(max_spins) {
            Ok(ticket) => Ok(IrqLockToken {
                ticket,
                previous_interrupt_state,
            }),
            Err(error) => {
                mask.restore(previous_interrupt_state);
                Err(error)
            }
        }
    }

    pub fn release<M: InterruptMask>(
        &self,
        mask: &mut M,
        token: IrqLockToken<M::State>,
    ) -> Result<(), KernelSyncError> {
        self.lock.release(token.ticket)?;
        mask.restore(token.previous_interrupt_state);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SequenceLock {
    sequence: AtomicU64,
    writer: TicketSpinLock,
}

impl SequenceLock {
    pub const fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            writer: TicketSpinLock::new(),
        }
    }

    pub fn read_begin(&self) -> u64 {
        loop {
            let sequence = self.sequence.load(Ordering::Acquire);
            if sequence & 1 == 0 {
                return sequence;
            }
            spin_loop();
        }
    }

    pub fn read_retry(&self, sequence: u64) -> bool {
        fence(Ordering::Acquire);
        self.sequence.load(Ordering::Relaxed) != sequence
    }

    pub fn write_begin(&self, max_spins: u32) -> Result<u32, KernelSyncError> {
        let ticket = self.writer.acquire_bounded(max_spins)?;
        self.sequence.fetch_add(1, Ordering::Release);
        Ok(ticket)
    }

    pub fn write_end(&self, ticket: u32) -> Result<(), KernelSyncError> {
        self.sequence.fetch_add(1, Ordering::Release);
        self.writer.release(ticket)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RcuReadToken {
    pub cpu: usize,
    pub epoch: u64,
}

#[derive(Debug)]
pub struct RcuDomain<const CPUS: usize> {
    epoch: AtomicU64,
    readers: [AtomicU64; CPUS],
}

impl<const CPUS: usize> RcuDomain<CPUS> {
    pub fn new() -> Self {
        Self {
            epoch: AtomicU64::new(2),
            readers: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    pub fn read_lock(&self, cpu: usize) -> Result<RcuReadToken, KernelSyncError> {
        let slot = self
            .readers
            .get(cpu)
            .ok_or(KernelSyncError::CpuOutOfRange(cpu))?;
        let epoch = self.epoch.load(Ordering::Acquire);
        slot.store(epoch, Ordering::Release);
        Ok(RcuReadToken { cpu, epoch })
    }

    pub fn read_unlock(&self, token: RcuReadToken) -> Result<(), KernelSyncError> {
        let slot = self
            .readers
            .get(token.cpu)
            .ok_or(KernelSyncError::CpuOutOfRange(token.cpu))?;
        slot.store(0, Ordering::Release);
        Ok(())
    }

    pub fn publish_epoch(&self) -> u64 {
        self.epoch.fetch_add(2, Ordering::AcqRel) + 2
    }

    pub fn synchronize_bounded(
        &self,
        new_epoch: u64,
        max_spins: u32,
    ) -> Result<(), KernelSyncError> {
        for _ in 0..max_spins {
            if self.readers.iter().all(|reader| {
                let observed = reader.load(Ordering::Acquire);
                observed == 0 || observed >= new_epoch
            }) {
                return Ok(());
            }
            spin_loop();
        }
        Err(KernelSyncError::RcuTimeout)
    }

    pub fn active_readers(&self) -> usize {
        self.readers
            .iter()
            .filter(|reader| reader.load(Ordering::Relaxed) != 0)
            .count()
    }
}

impl<const CPUS: usize> Default for RcuDomain<CPUS> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierKind {
    Compiler,
    Acquire,
    Release,
    Full,
    Io,
    Instruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarrierEncoding {
    pub kind: BarrierKind,
    pub mnemonic: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSyncPlan {
    pub schema: &'static str,
    pub target: &'static str,
    pub atomics: &'static [&'static str],
    pub wait_hints: &'static [&'static str],
    pub barriers: &'static [BarrierEncoding],
    pub hidden_scheduler: bool,
    pub hidden_allocation: bool,
}

const X86_BARRIERS: &[BarrierEncoding] = &[
    BarrierEncoding {
        kind: BarrierKind::Compiler,
        mnemonic: "compiler_fence",
        bytes: &[],
    },
    BarrierEncoding {
        kind: BarrierKind::Acquire,
        mnemonic: "lfence",
        bytes: &[0x0f, 0xae, 0xe8],
    },
    BarrierEncoding {
        kind: BarrierKind::Release,
        mnemonic: "sfence",
        bytes: &[0x0f, 0xae, 0xf8],
    },
    BarrierEncoding {
        kind: BarrierKind::Full,
        mnemonic: "mfence",
        bytes: &[0x0f, 0xae, 0xf0],
    },
    BarrierEncoding {
        kind: BarrierKind::Io,
        mnemonic: "sfence",
        bytes: &[0x0f, 0xae, 0xf8],
    },
    BarrierEncoding {
        kind: BarrierKind::Instruction,
        mnemonic: "serialize",
        bytes: &[0x0f, 0x01, 0xe8],
    },
];

const AARCH64_BARRIERS: &[BarrierEncoding] = &[
    BarrierEncoding {
        kind: BarrierKind::Compiler,
        mnemonic: "compiler_fence",
        bytes: &[],
    },
    BarrierEncoding {
        kind: BarrierKind::Acquire,
        mnemonic: "dmb ishld",
        bytes: &[0xbf, 0x39, 0x03, 0xd5],
    },
    BarrierEncoding {
        kind: BarrierKind::Release,
        mnemonic: "dmb ishst",
        bytes: &[0xbf, 0x3a, 0x03, 0xd5],
    },
    BarrierEncoding {
        kind: BarrierKind::Full,
        mnemonic: "dmb ish",
        bytes: &[0xbf, 0x3b, 0x03, 0xd5],
    },
    BarrierEncoding {
        kind: BarrierKind::Io,
        mnemonic: "dmb osh",
        bytes: &[0xbf, 0x33, 0x03, 0xd5],
    },
    BarrierEncoding {
        kind: BarrierKind::Instruction,
        mnemonic: "isb",
        bytes: &[0xdf, 0x3f, 0x03, 0xd5],
    },
];

const ARMV7_BARRIERS: &[BarrierEncoding] = &[
    BarrierEncoding {
        kind: BarrierKind::Compiler,
        mnemonic: "compiler_fence",
        bytes: &[],
    },
    BarrierEncoding {
        kind: BarrierKind::Acquire,
        mnemonic: "dmb sy",
        bytes: &[0xbf, 0xf3, 0x5f, 0x8f],
    },
    BarrierEncoding {
        kind: BarrierKind::Release,
        mnemonic: "dmb sy",
        bytes: &[0xbf, 0xf3, 0x5f, 0x8f],
    },
    BarrierEncoding {
        kind: BarrierKind::Full,
        mnemonic: "dmb sy",
        bytes: &[0xbf, 0xf3, 0x5f, 0x8f],
    },
    BarrierEncoding {
        kind: BarrierKind::Io,
        mnemonic: "dsb sy",
        bytes: &[0xbf, 0xf3, 0x4f, 0x8f],
    },
    BarrierEncoding {
        kind: BarrierKind::Instruction,
        mnemonic: "isb sy",
        bytes: &[0xbf, 0xf3, 0x6f, 0x8f],
    },
];

const RISCV_BARRIERS: &[BarrierEncoding] = &[
    BarrierEncoding {
        kind: BarrierKind::Compiler,
        mnemonic: "compiler_fence",
        bytes: &[],
    },
    BarrierEncoding {
        kind: BarrierKind::Acquire,
        mnemonic: "fence r,rw",
        bytes: &[0x0f, 0x00, 0x30, 0x02],
    },
    BarrierEncoding {
        kind: BarrierKind::Release,
        mnemonic: "fence rw,w",
        bytes: &[0x0f, 0x00, 0x10, 0x03],
    },
    BarrierEncoding {
        kind: BarrierKind::Full,
        mnemonic: "fence rw,rw",
        bytes: &[0x0f, 0x00, 0x30, 0x03],
    },
    BarrierEncoding {
        kind: BarrierKind::Io,
        mnemonic: "fence iorw,iorw",
        bytes: &[0x0f, 0x00, 0xf0, 0x0f],
    },
    BarrierEncoding {
        kind: BarrierKind::Instruction,
        mnemonic: "fence.i",
        bytes: &[0x0f, 0x10, 0x00, 0x00],
    },
];

pub fn kernel_sync_plan(target: &str) -> Result<KernelSyncPlan, KernelSyncError> {
    let plan = match target {
        "x86_64-unknown-none" | "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" => {
            KernelSyncPlan {
                schema: KERNEL_SYNC_SCHEMA,
                target: "x86_64",
                atomics: &["lock cmpxchg", "lock xadd", "cmpxchg16b"],
                wait_hints: &["pause", "monitor", "mwait", "umonitor", "umwait", "tpause"],
                barriers: X86_BARRIERS,
                hidden_scheduler: false,
                hidden_allocation: false,
            }
        }
        "i386-unknown-none" | "i386-unknown-linux-gnu" => KernelSyncPlan {
            schema: KERNEL_SYNC_SCHEMA,
            target: "i386",
            atomics: &["lock cmpxchg", "lock xadd", "cmpxchg8b"],
            wait_hints: &["pause", "monitor", "mwait"],
            barriers: X86_BARRIERS,
            hidden_scheduler: false,
            hidden_allocation: false,
        },
        "aarch64-unknown-none" | "aarch64-unknown-linux-gnu" => KernelSyncPlan {
            schema: KERNEL_SYNC_SCHEMA,
            target: "aarch64",
            atomics: &["ldaxr/stlxr", "casal", "swpal", "ldaddal"],
            wait_hints: &["yield", "wfe", "sev", "sevl"],
            barriers: AARCH64_BARRIERS,
            hidden_scheduler: false,
            hidden_allocation: false,
        },
        "armv7a-unknown-none-eabi" | "armv7-unknown-none-eabi" | "thumbv7em-none-eabi" => {
            KernelSyncPlan {
                schema: KERNEL_SYNC_SCHEMA,
                target: "armv7-thumb2",
                atomics: &["ldrex/strex", "clrex"],
                wait_hints: &["yield", "wfe", "sev"],
                barriers: ARMV7_BARRIERS,
                hidden_scheduler: false,
                hidden_allocation: false,
            }
        }
        "riscv32imac-unknown-elf" | "riscv64-unknown-none" | "riscv64-unknown-elf" => {
            KernelSyncPlan {
                schema: KERNEL_SYNC_SCHEMA,
                target: "riscv",
                atomics: &["lr/sc", "amoadd", "amoswap", "amoor", "amoand", "amoxor"],
                wait_hints: &["pause", "wfi"],
                barriers: RISCV_BARRIERS,
                hidden_scheduler: false,
                hidden_allocation: false,
            }
        }
        _ => return Err(KernelSyncError::UnsupportedTarget(target.to_string())),
    };
    Ok(plan)
}

pub fn execute_barrier(kind: BarrierKind) {
    match kind {
        BarrierKind::Compiler => compiler_fence(Ordering::SeqCst),
        BarrierKind::Acquire => fence(Ordering::Acquire),
        BarrierKind::Release => fence(Ordering::Release),
        BarrierKind::Full | BarrierKind::Io | BarrierKind::Instruction => fence(Ordering::SeqCst),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSyncReport {
    pub targets: usize,
    pub barrier_encodings: usize,
    pub contended_increments: u64,
    pub queued_mutex_handoff: u32,
    pub irq_restores: u64,
    pub rcu_epochs: u64,
    pub seqlock_retries: u64,
}

#[derive(Debug, Default)]
struct TracedInterruptMask {
    enabled: bool,
    restores: u64,
}

impl InterruptMask for TracedInterruptMask {
    type State = bool;

    fn save_and_disable(&mut self) -> Self::State {
        let previous = self.enabled;
        self.enabled = false;
        previous
    }

    fn restore(&mut self, state: Self::State) {
        self.enabled = state;
        self.restores += 1;
    }
}

pub fn certify_kernel_sync_contracts() -> Result<KernelSyncReport, KernelSyncError> {
    let targets = [
        "x86_64-unknown-none",
        "i386-unknown-none",
        "aarch64-unknown-none",
        "armv7a-unknown-none-eabi",
        "riscv32imac-unknown-elf",
        "riscv64-unknown-elf",
    ];
    let mut barrier_encodings = 0;
    for target in targets {
        let plan = kernel_sync_plan(target)?;
        barrier_encodings += plan.barriers.len();
        if plan.hidden_allocation || plan.hidden_scheduler {
            return Err(KernelSyncError::UnsupportedTarget(target.to_string()));
        }
    }

    let lock = Arc::new(TicketSpinLock::new());
    let counter = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::new();
    for _ in 0..4 {
        let lock = Arc::clone(&lock);
        let counter = Arc::clone(&counter);
        workers.push(thread::spawn(move || -> Result<(), KernelSyncError> {
            for _ in 0..2_000 {
                let ticket = lock.acquire();
                let value = counter.load(Ordering::Relaxed);
                counter.store(value + 1, Ordering::Relaxed);
                lock.release(ticket)?;
            }
            Ok(())
        }));
    }
    for worker in workers {
        worker.join().map_err(|_| KernelSyncError::SpinLimit)??;
    }

    let mut queue_slots = [0; 4];
    let mut queue = CallerWaitQueue::new(&mut queue_slots)?;
    let mutex = KernelMutex::new();
    mutex.try_acquire(7)?;
    if mutex.acquire_or_queue(9, &mut queue)? {
        return Err(KernelSyncError::Contended);
    }
    let queued_mutex_handoff = mutex
        .release_to_next(7, &mut queue)?
        .ok_or(KernelSyncError::QueueEmpty)?;
    mutex.release_to_next(queued_mutex_handoff, &mut queue)?;

    let irq_lock = InterruptSafeSpinLock::new();
    let mut mask = TracedInterruptMask {
        enabled: true,
        restores: 0,
    };
    let irq = irq_lock.acquire_bounded(&mut mask, 100)?;
    irq_lock.release(&mut mask, irq)?;

    let rcu = RcuDomain::<4>::new();
    let read = rcu.read_lock(0)?;
    let new_epoch = rcu.publish_epoch();
    if !matches!(
        rcu.synchronize_bounded(new_epoch, 1),
        Err(KernelSyncError::RcuTimeout)
    ) {
        return Err(KernelSyncError::RcuTimeout);
    }
    rcu.read_unlock(read)?;
    rcu.synchronize_bounded(new_epoch, 100)?;

    let sequence = SequenceLock::new();
    let before = sequence.read_begin();
    let writer = sequence.write_begin(100)?;
    sequence.write_end(writer)?;
    let seqlock_retries = u64::from(sequence.read_retry(before));

    Ok(KernelSyncReport {
        targets: targets.len(),
        barrier_encodings,
        contended_increments: counter.load(Ordering::Relaxed),
        queued_mutex_handoff,
        irq_restores: mask.restores,
        rcu_epochs: new_epoch,
        seqlock_retries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_queue_mutex_rwlock_and_irq_lock_remain_explicit() {
        let rw = RawRwLock::new();
        rw.try_read().unwrap();
        assert_eq!(rw.readers(), 1);
        assert_eq!(rw.try_write(), Err(KernelSyncError::Contended));
        rw.read_release().unwrap();
        rw.try_write().unwrap();
        rw.write_release().unwrap();

        let mut slots = [0; 2];
        let mut queue = CallerWaitQueue::new(&mut slots).unwrap();
        let mutex = KernelMutex::new();
        mutex.try_acquire(1).unwrap();
        assert!(!mutex.acquire_or_queue(2, &mut queue).unwrap());
        assert_eq!(mutex.release_to_next(1, &mut queue).unwrap(), Some(2));
        assert_eq!(mutex.owner(), Some(2));
    }

    #[test]
    fn bounded_spin_timeout_never_leaves_an_abandoned_ticket() {
        let lock = TicketSpinLock::new();
        let held = lock.acquire();
        assert_eq!(lock.acquire_bounded(4), Err(KernelSyncError::SpinLimit));
        lock.release(held).unwrap();
        let next = lock.try_acquire().unwrap();
        lock.release(next).unwrap();
        assert_eq!(lock.queued(), 0);
    }

    #[test]
    fn rcu_requires_explicit_quiescence_before_reclamation() {
        let rcu = RcuDomain::<2>::new();
        let reader = rcu.read_lock(0).unwrap();
        let epoch = rcu.publish_epoch();
        assert_eq!(
            rcu.synchronize_bounded(epoch, 2),
            Err(KernelSyncError::RcuTimeout)
        );
        rcu.read_unlock(reader).unwrap();
        rcu.synchronize_bounded(epoch, 2).unwrap();
    }

    #[test]
    fn synchronization_certification_covers_all_kernel_targets() {
        let report = certify_kernel_sync_contracts().unwrap();
        assert_eq!(report.targets, 6);
        assert_eq!(report.barrier_encodings, 36);
        assert_eq!(report.contended_increments, 8_000);
        assert_eq!(report.queued_mutex_handoff, 9);
        assert_eq!(report.irq_restores, 1);
        assert_eq!(report.seqlock_retries, 1);
    }
}
