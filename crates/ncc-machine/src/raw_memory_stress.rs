use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawMemoryStressProfile {
    Smoke,
    Ci,
    Endurance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawMemoryStressConfig {
    pub profile: RawMemoryStressProfile,
    pub seed: u64,
    pub heap_operations: u64,
    pub realloc_operations: u64,
    pub fragmentation_operations: u64,
    pub threads: usize,
    pub operations_per_thread: u64,
}

impl RawMemoryStressProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Ci => "ci",
            Self::Endurance => "endurance",
        }
    }

    pub const fn config(self) -> RawMemoryStressConfig {
        match self {
            Self::Smoke => RawMemoryStressConfig {
                profile: self,
                seed: 0x4e43_5241_574d_454d,
                heap_operations: 10_000,
                realloc_operations: 10_000,
                fragmentation_operations: 10_000,
                threads: 4,
                operations_per_thread: 25_000,
            },
            Self::Ci => RawMemoryStressConfig {
                profile: self,
                seed: 0x4e43_5241_5743_4921,
                heap_operations: 500_000,
                realloc_operations: 500_000,
                fragmentation_operations: 500_000,
                threads: 8,
                operations_per_thread: 1_000_000,
            },
            Self::Endurance => RawMemoryStressConfig {
                profile: self,
                seed: 0x4e43_5241_5745_4e44,
                heap_operations: 10_000_000,
                realloc_operations: 10_000_000,
                fragmentation_operations: 10_000_000,
                threads: 16,
                operations_per_thread: 10_000_000,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMemoryStressMetric {
    pub scenario: &'static str,
    pub operations: u64,
    pub elapsed_ns: u128,
    pub checksum: u64,
    pub peak_live_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMemoryStressReport {
    pub config: RawMemoryStressConfig,
    pub metrics: Vec<RawMemoryStressMetric>,
    pub total_operations: u64,
    pub elapsed_ns: u128,
    pub checksum: u64,
}

#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0.max(1);
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

fn mix(state: u64, value: u64) -> u64 {
    state.rotate_left(11) ^ value.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

fn heap_realloc_stress(config: RawMemoryStressConfig) -> RawMemoryStressMetric {
    let started = Instant::now();
    let mut rng = Rng(config.seed);
    let mut slots = vec![Vec::<u8>::new(); 257];
    let mut checksum = config.seed;
    let mut live = 0usize;
    let mut peak = 0usize;

    for operation in 0..config.heap_operations {
        let slot = rng.next() as usize % slots.len();
        live = live.saturating_sub(slots[slot].len());
        let bytes = (rng.next() as usize % 4096) + 1;
        slots[slot] = vec![(operation as u8).wrapping_mul(17); bytes];
        live = live.saturating_add(bytes);
        peak = peak.max(live);
        checksum = mix(checksum, slots[slot][bytes - 1] as u64 ^ bytes as u64);
    }

    for operation in 0..config.realloc_operations {
        let slot = rng.next() as usize % slots.len();
        let old_len = slots[slot].len();
        let prefix = slots[slot].first().copied();
        let new_len = rng.next() as usize % 8192;
        live = live.saturating_sub(old_len);
        slots[slot].resize(new_len, (operation as u8).wrapping_mul(29));
        live = live.saturating_add(new_len);
        peak = peak.max(live);
        if old_len != 0 && new_len != 0 {
            assert_eq!(
                slots[slot].first().copied(),
                prefix,
                "realloc lost its preserved prefix"
            );
        }
        checksum = mix(checksum, old_len as u64 ^ (new_len as u64).rotate_left(7));
    }

    RawMemoryStressMetric {
        scenario: "heap-realloc",
        operations: config.heap_operations + config.realloc_operations,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        peak_live_bytes: peak,
    }
}

fn fragmentation_stress(config: RawMemoryStressConfig) -> RawMemoryStressMetric {
    let started = Instant::now();
    let mut rng = Rng(config.seed ^ 0x4652_4147_4d45_4e54);
    let mut blocks: Vec<Option<Box<[u8]>>> = (0..1024).map(|_| None).collect();
    let mut checksum = config.seed;
    let mut live = 0usize;
    let mut peak = 0usize;
    for operation in 0..config.fragmentation_operations {
        let slot = rng.next() as usize % blocks.len();
        if let Some(block) = blocks[slot].take() {
            live = live.saturating_sub(block.len());
            checksum = mix(checksum, block.len() as u64);
        } else {
            let bytes = ((rng.next() as usize % 2048) + 1).next_power_of_two();
            let block = vec![(slot as u8).wrapping_add(operation as u8); bytes].into_boxed_slice();
            checksum = mix(checksum, block[bytes - 1] as u64 ^ bytes as u64);
            live = live.saturating_add(bytes);
            peak = peak.max(live);
            blocks[slot] = Some(block);
        }
    }
    RawMemoryStressMetric {
        scenario: "fragmentation",
        operations: config.fragmentation_operations,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
        peak_live_bytes: peak,
    }
}

fn concurrency_stress(config: RawMemoryStressConfig) -> RawMemoryStressMetric {
    let started = Instant::now();
    let cells = Arc::new((0..64).map(|_| AtomicU64::new(0)).collect::<Vec<_>>());
    let barrier = Arc::new(Barrier::new(config.threads));
    let mut workers = Vec::with_capacity(config.threads);
    for worker in 0..config.threads {
        let cells = Arc::clone(&cells);
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            for operation in 0..config.operations_per_thread {
                let cell = (operation as usize + worker * 17) % cells.len();
                cells[cell].fetch_add(1, Ordering::AcqRel);
            }
        }));
    }
    for worker in workers {
        worker.join().expect("raw memory stress worker panicked");
    }
    let total = cells
        .iter()
        .map(|cell| cell.load(Ordering::Acquire))
        .sum::<u64>();
    let expected = config.threads as u64 * config.operations_per_thread;
    assert_eq!(total, expected, "atomic stress lost updates");
    RawMemoryStressMetric {
        scenario: "concurrent-atomics",
        operations: expected,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum: mix(config.seed, total),
        peak_live_bytes: cells.len() * core::mem::size_of::<AtomicU64>(),
    }
}

pub fn run_raw_memory_stress(
    config: RawMemoryStressConfig,
) -> Result<RawMemoryStressReport, String> {
    if config.heap_operations == 0
        || config.realloc_operations == 0
        || config.fragmentation_operations == 0
        || config.operations_per_thread == 0
        || !(2..=64).contains(&config.threads)
    {
        return Err(
            "E_MACHINE_RAW_MEMORY_STRESS_CONFIG_001: invalid zero count or thread count".into(),
        );
    }
    let started = Instant::now();
    let metrics = vec![
        heap_realloc_stress(config),
        fragmentation_stress(config),
        concurrency_stress(config),
    ];
    let total_operations = metrics.iter().map(|metric| metric.operations).sum();
    let checksum = metrics
        .iter()
        .fold(config.seed, |state, metric| mix(state, metric.checksum));
    Ok(RawMemoryStressReport {
        config,
        metrics,
        total_operations,
        elapsed_ns: started.elapsed().as_nanos(),
        checksum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_executes_heap_realloc_fragmentation_and_concurrency() {
        let report = run_raw_memory_stress(RawMemoryStressProfile::Smoke.config()).unwrap();
        assert_eq!(report.metrics.len(), 3);
        assert_eq!(
            report
                .metrics
                .iter()
                .map(|row| row.scenario)
                .collect::<Vec<_>>(),
            ["heap-realloc", "fragmentation", "concurrent-atomics"]
        );
        assert!(report
            .metrics
            .iter()
            .all(|row| row.operations > 0 && row.peak_live_bytes > 0));
        assert_ne!(report.checksum, 0);
    }
}
