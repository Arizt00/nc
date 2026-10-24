use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub const MANUAL_POOL_EMPTY: usize = usize::MAX;
pub const MANUAL_ATOMIC_POOL_EMPTY: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualBlock {
    pub address: usize,
    pub bytes: usize,
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return None;
    }
    value
        .checked_add(alignment - 1)
        .map(|v| v & !(alignment - 1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualArenaState {
    pub base: usize,
    pub capacity: usize,
    pub cursor: usize,
    pub high_water: usize,
}

impl ManualArenaState {
    pub const fn new(base: usize, capacity: usize) -> Self {
        Self {
            base,
            capacity,
            cursor: 0,
            high_water: 0,
        }
    }

    pub fn allocate(&mut self, bytes: usize, alignment: usize) -> Option<ManualBlock> {
        let start = align_up(self.cursor, alignment)?;
        let end = start.checked_add(bytes)?;
        if end > self.capacity {
            return None;
        }
        self.cursor = end;
        self.high_water = self.high_water.max(end);
        Some(ManualBlock {
            address: self.base.checked_add(start)?,
            bytes,
        })
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn remaining(&self) -> usize {
        self.capacity.saturating_sub(self.cursor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualPoolState {
    pub base: usize,
    pub block_bytes: usize,
    pub block_count: usize,
    pub free_head: usize,
    pub free_count: usize,
    pub high_water_blocks: usize,
}

impl ManualPoolState {
    pub fn initialize(
        base: usize,
        block_bytes: usize,
        block_count: usize,
        caller_next: &mut [usize],
    ) -> Option<Self> {
        if block_bytes == 0 || caller_next.len() < block_count {
            return None;
        }
        for (index, next) in caller_next.iter_mut().take(block_count).enumerate() {
            *next = if index + 1 < block_count {
                index + 1
            } else {
                MANUAL_POOL_EMPTY
            };
        }
        Some(Self {
            base,
            block_bytes,
            block_count,
            free_head: if block_count == 0 {
                MANUAL_POOL_EMPTY
            } else {
                0
            },
            free_count: block_count,
            high_water_blocks: 0,
        })
    }

    pub fn allocate(&mut self, caller_next: &[usize]) -> Option<ManualBlock> {
        let index = self.free_head;
        if index == MANUAL_POOL_EMPTY || index >= self.block_count || index >= caller_next.len() {
            return None;
        }
        self.free_head = caller_next[index];
        self.free_count = self.free_count.saturating_sub(1);
        self.high_water_blocks = self
            .high_water_blocks
            .max(self.block_count.saturating_sub(self.free_count));
        Some(ManualBlock {
            address: self
                .base
                .checked_add(index.checked_mul(self.block_bytes)?)?,
            bytes: self.block_bytes,
        })
    }

    pub fn release(&mut self, block: ManualBlock, caller_next: &mut [usize]) -> bool {
        if block.address < self.base || self.block_bytes == 0 {
            return false;
        }
        let delta = block.address - self.base;
        if delta % self.block_bytes != 0 {
            return false;
        }
        let index = delta / self.block_bytes;
        if index >= self.block_count || index >= caller_next.len() {
            return false;
        }
        caller_next[index] = self.free_head;
        self.free_head = index;
        self.free_count = self.free_count.wrapping_add(1);
        true
    }
}

pub struct ManualSlabClass<'a> {
    pub maximum_bytes: usize,
    pub pool: &'a mut ManualPoolState,
    pub caller_next: &'a mut [usize],
}

pub fn manual_slab_allocate(
    classes: &mut [ManualSlabClass<'_>],
    bytes: usize,
    alignment: usize,
) -> Option<(usize, ManualBlock)> {
    for (index, class) in classes.iter_mut().enumerate() {
        if bytes <= class.maximum_bytes
            && alignment <= class.pool.block_bytes
            && class.pool.block_bytes % alignment.max(1) == 0
        {
            if let Some(block) = class.pool.allocate(class.caller_next) {
                return Some((index, block));
            }
        }
    }
    None
}

pub fn manual_slab_release(
    classes: &mut [ManualSlabClass<'_>],
    class_index: usize,
    block: ManualBlock,
) -> bool {
    classes
        .get_mut(class_index)
        .is_some_and(|class| class.pool.release(block, class.caller_next))
}

const BUDDY_FREE: u8 = 0;
const BUDDY_SPLIT: u8 = 1;
const BUDDY_FULL: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualBuddyState {
    pub base: usize,
    pub minimum_order: u8,
    pub maximum_order: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualFragmentation {
    pub free_bytes: usize,
    pub largest_free_block: usize,
    pub external_parts_per_million: u32,
}

impl ManualBuddyState {
    pub fn metadata_bytes(minimum_order: u8, maximum_order: u8) -> Option<usize> {
        let levels = maximum_order.checked_sub(minimum_order)? as u32;
        1usize.checked_shl(levels + 1)?.checked_sub(1)
    }

    pub fn initialize(
        base: usize,
        minimum_order: u8,
        maximum_order: u8,
        caller_tree: &mut [u8],
    ) -> Option<Self> {
        let required = Self::metadata_bytes(minimum_order, maximum_order)?;
        if caller_tree.len() < required || maximum_order as u32 >= usize::BITS {
            return None;
        }
        caller_tree[..required].fill(BUDDY_FREE);
        Some(Self {
            base,
            minimum_order,
            maximum_order,
        })
    }

    pub fn allocate(
        &self,
        caller_tree: &mut [u8],
        bytes: usize,
        alignment: usize,
    ) -> Option<ManualBlock> {
        if bytes == 0 || alignment == 0 || !alignment.is_power_of_two() {
            return None;
        }
        let needed = bytes.max(alignment).max(1usize << self.minimum_order);
        let target_order = (usize::BITS - (needed - 1).leading_zeros()) as u8;
        if target_order > self.maximum_order {
            return None;
        }
        let offset = buddy_allocate_node(caller_tree, 0, self.maximum_order, target_order, 0)?;
        Some(ManualBlock {
            address: self.base.checked_add(offset)?,
            bytes: 1usize.checked_shl(target_order as u32)?,
        })
    }

    pub fn release(&self, caller_tree: &mut [u8], block: ManualBlock) -> bool {
        if block.address < self.base || !block.bytes.is_power_of_two() {
            return false;
        }
        let order = block.bytes.trailing_zeros() as u8;
        if order < self.minimum_order || order > self.maximum_order {
            return false;
        }
        let offset = block.address - self.base;
        buddy_release_node(caller_tree, 0, self.maximum_order, order, 0, offset)
    }

    pub fn fragmentation(&self, caller_tree: &[u8]) -> ManualFragmentation {
        let (free_bytes, largest_free_block) =
            buddy_fragmentation_node(caller_tree, 0, self.maximum_order);
        let external_parts_per_million = if free_bytes == 0 {
            0
        } else {
            (((free_bytes - largest_free_block) as u128 * 1_000_000) / free_bytes as u128) as u32
        };
        ManualFragmentation {
            free_bytes,
            largest_free_block,
            external_parts_per_million,
        }
    }
}

fn buddy_allocate_node(
    tree: &mut [u8],
    index: usize,
    order: u8,
    target_order: u8,
    offset: usize,
) -> Option<usize> {
    let state = *tree.get(index)?;
    if state == BUDDY_FULL || order < target_order {
        return None;
    }
    if order == target_order {
        if state != BUDDY_FREE {
            return None;
        }
        tree[index] = BUDDY_FULL;
        return Some(offset);
    }
    if state == BUDDY_FREE {
        tree[index] = BUDDY_SPLIT;
    }
    let left = index.checked_mul(2)?.checked_add(1)?;
    let right = left.checked_add(1)?;
    let half = 1usize.checked_shl((order - 1) as u32)?;
    let result = buddy_allocate_node(tree, left, order - 1, target_order, offset)
        .or_else(|| buddy_allocate_node(tree, right, order - 1, target_order, offset + half));
    let left_state = *tree.get(left)?;
    let right_state = *tree.get(right)?;
    tree[index] = if left_state == BUDDY_FULL && right_state == BUDDY_FULL {
        BUDDY_FULL
    } else {
        BUDDY_SPLIT
    };
    result
}

fn buddy_release_node(
    tree: &mut [u8],
    index: usize,
    order: u8,
    target_order: u8,
    node_offset: usize,
    target_offset: usize,
) -> bool {
    if index >= tree.len() || order < target_order {
        return false;
    }
    if order == target_order {
        if node_offset != target_offset || tree[index] == BUDDY_FREE {
            return false;
        }
        tree[index] = BUDDY_FREE;
        return true;
    }
    if tree[index] != BUDDY_SPLIT && tree[index] != BUDDY_FULL {
        return false;
    }
    let left = index * 2 + 1;
    let right = left + 1;
    let half = 1usize << (order - 1);
    let released = if target_offset < node_offset + half {
        buddy_release_node(
            tree,
            left,
            order - 1,
            target_order,
            node_offset,
            target_offset,
        )
    } else {
        buddy_release_node(
            tree,
            right,
            order - 1,
            target_order,
            node_offset + half,
            target_offset,
        )
    };
    if released {
        tree[index] = match (tree[left], tree[right]) {
            (BUDDY_FREE, BUDDY_FREE) => BUDDY_FREE,
            (BUDDY_FULL, BUDDY_FULL) => BUDDY_FULL,
            _ => BUDDY_SPLIT,
        };
    }
    released
}

fn buddy_fragmentation_node(tree: &[u8], index: usize, order: u8) -> (usize, usize) {
    let Some(state) = tree.get(index).copied() else {
        return (0, 0);
    };
    if state == BUDDY_FREE {
        let bytes = 1usize.checked_shl(order as u32).unwrap_or(0);
        return (bytes, bytes);
    }
    if state == BUDDY_FULL || order == 0 {
        return (0, 0);
    }
    let left = buddy_fragmentation_node(tree, index * 2 + 1, order - 1);
    let right = buddy_fragmentation_node(tree, index * 2 + 2, order - 1);
    (left.0 + right.0, left.1.max(right.1))
}

#[derive(Debug)]
pub struct ManualAtomicArena {
    pub base: usize,
    pub capacity: usize,
    pub cursor: AtomicUsize,
    pub high_water: AtomicUsize,
}

impl ManualAtomicArena {
    pub const fn new(base: usize, capacity: usize) -> Self {
        Self {
            base,
            capacity,
            cursor: AtomicUsize::new(0),
            high_water: AtomicUsize::new(0),
        }
    }

    pub fn allocate(
        &self,
        bytes: usize,
        alignment: usize,
        success: Ordering,
        failure: Ordering,
    ) -> Option<ManualBlock> {
        let mut observed = self.cursor.load(failure);
        loop {
            let start = align_up(observed, alignment)?;
            let end = start.checked_add(bytes)?;
            if end > self.capacity {
                return None;
            }
            match self
                .cursor
                .compare_exchange_weak(observed, end, success, failure)
            {
                Ok(_) => {
                    self.high_water.fetch_max(end, Ordering::Relaxed);
                    return Some(ManualBlock {
                        address: self.base.checked_add(start)?,
                        bytes,
                    });
                }
                Err(current) => observed = current,
            }
        }
    }
}

#[derive(Debug)]
pub struct ManualAtomicPool {
    pub base: usize,
    pub block_bytes: usize,
    pub block_count: u32,
    pub tagged_head: AtomicU64,
}

fn pack_atomic_head(index: u32, tag: u32) -> u64 {
    (u64::from(tag) << 32) | u64::from(index)
}

fn unpack_atomic_head(value: u64) -> (u32, u32) {
    (value as u32, (value >> 32) as u32)
}

impl ManualAtomicPool {
    pub fn initialize(
        base: usize,
        block_bytes: usize,
        block_count: u32,
        caller_next: &[AtomicU32],
    ) -> Option<Self> {
        if block_bytes == 0 || caller_next.len() < block_count as usize {
            return None;
        }
        for index in 0..block_count {
            let next = if index + 1 < block_count {
                index + 1
            } else {
                MANUAL_ATOMIC_POOL_EMPTY
            };
            caller_next[index as usize].store(next, Ordering::Relaxed);
        }
        let head = if block_count == 0 {
            MANUAL_ATOMIC_POOL_EMPTY
        } else {
            0
        };
        Some(Self {
            base,
            block_bytes,
            block_count,
            tagged_head: AtomicU64::new(pack_atomic_head(head, 0)),
        })
    }

    pub fn allocate(&self, caller_next: &[AtomicU32]) -> Option<ManualBlock> {
        let mut observed = self.tagged_head.load(Ordering::Acquire);
        loop {
            let (index, tag) = unpack_atomic_head(observed);
            if index == MANUAL_ATOMIC_POOL_EMPTY || index >= self.block_count {
                return None;
            }
            let next = caller_next.get(index as usize)?.load(Ordering::Relaxed);
            let updated = pack_atomic_head(next, tag.wrapping_add(1));
            match self.tagged_head.compare_exchange_weak(
                observed,
                updated,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(ManualBlock {
                        address: self.base.checked_add(index as usize * self.block_bytes)?,
                        bytes: self.block_bytes,
                    })
                }
                Err(current) => observed = current,
            }
        }
    }

    pub fn release(&self, caller_next: &[AtomicU32], block: ManualBlock) -> bool {
        if block.address < self.base || self.block_bytes == 0 {
            return false;
        }
        let delta = block.address - self.base;
        if delta % self.block_bytes != 0 {
            return false;
        }
        let Ok(index) = u32::try_from(delta / self.block_bytes) else {
            return false;
        };
        if index >= self.block_count || index as usize >= caller_next.len() {
            return false;
        }
        let mut observed = self.tagged_head.load(Ordering::Acquire);
        loop {
            let (head, tag) = unpack_atomic_head(observed);
            caller_next[index as usize].store(head, Ordering::Relaxed);
            let updated = pack_atomic_head(index, tag.wrapping_add(1));
            match self.tagged_head.compare_exchange_weak(
                observed,
                updated,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(current) => observed = current,
            }
        }
    }
}

#[derive(Debug)]
pub struct ManualSpinLock {
    held: AtomicBool,
}

impl ManualSpinLock {
    pub const fn new() -> Self {
        Self {
            held: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        while self
            .held
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
    }

    pub fn unlock(&self) {
        self.held.store(false, Ordering::Release);
    }
}

impl Default for ManualSpinLock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualPerCpuHeapTable {
    pub table_base: usize,
    pub heap_stride: usize,
    pub cpu_count: usize,
}

impl ManualPerCpuHeapTable {
    pub fn heap_address_raw(&self, cpu_index: usize) -> usize {
        self.table_base
            .wrapping_add(cpu_index.wrapping_mul(self.heap_stride))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn arena_pool_slab_and_per_cpu_are_caller_backed() {
        let mut arena = ManualArenaState::new(0x1000, 256);
        assert_eq!(arena.allocate(17, 64).unwrap().address, 0x1000);
        assert_eq!(arena.allocate(1, 64).unwrap().address, 0x1040);
        assert_eq!(arena.high_water, 65);

        let mut next16 = [0usize; 4];
        let mut next64 = [0usize; 2];
        let mut pool16 = ManualPoolState::initialize(0x2000, 16, 4, &mut next16).unwrap();
        let mut pool64 = ManualPoolState::initialize(0x3000, 64, 2, &mut next64).unwrap();
        let mut classes = [
            ManualSlabClass {
                maximum_bytes: 16,
                pool: &mut pool16,
                caller_next: &mut next16,
            },
            ManualSlabClass {
                maximum_bytes: 64,
                pool: &mut pool64,
                caller_next: &mut next64,
            },
        ];
        let (class, block) = manual_slab_allocate(&mut classes, 33, 8).unwrap();
        assert_eq!(class, 1);
        assert_eq!(block.address, 0x3000);
        assert!(manual_slab_release(&mut classes, class, block));

        let table = ManualPerCpuHeapTable {
            table_base: 0x8000,
            heap_stride: 0x1000,
            cpu_count: 4,
        };
        assert_eq!(table.heap_address_raw(3), 0xb000);
        assert_eq!(table.heap_address_raw(9), 0x11000);
    }

    #[test]
    fn buddy_splits_coalesces_and_reports_fragmentation() {
        let mut tree = [0u8; 31];
        let buddy = ManualBuddyState::initialize(0x10000, 8, 12, &mut tree).unwrap();
        let a = buddy.allocate(&mut tree, 300, 256).unwrap();
        let b = buddy.allocate(&mut tree, 900, 512).unwrap();
        assert_eq!(a.bytes, 512);
        assert_eq!(b.bytes, 1024);
        let fragmented = buddy.fragmentation(&tree);
        assert_eq!(fragmented.free_bytes, 2560);
        assert!(fragmented.external_parts_per_million > 0);
        assert!(buddy.release(&mut tree, a));
        assert!(buddy.release(&mut tree, b));
        assert_eq!(
            buddy.fragmentation(&tree),
            ManualFragmentation {
                free_bytes: 4096,
                largest_free_block: 4096,
                external_parts_per_million: 0,
            }
        );
    }

    #[test]
    fn atomic_arena_has_unique_ranges_under_contention() {
        let arena = Arc::new(ManualAtomicArena::new(0x100000, 8 * 8_000));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let arena = Arc::clone(&arena);
            workers.push(thread::spawn(move || {
                let mut addresses = Vec::with_capacity(1_000);
                for _ in 0..1_000 {
                    addresses.push(
                        arena
                            .allocate(8, 8, Ordering::AcqRel, Ordering::Acquire)
                            .unwrap()
                            .address,
                    );
                }
                addresses
            }));
        }
        let addresses = workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(addresses.len(), 8_000);
        assert_eq!(
            addresses.iter().copied().collect::<HashSet<_>>().len(),
            8_000
        );
        assert!(arena
            .allocate(1, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_none());
    }

    #[test]
    fn tagged_atomic_pool_survives_parallel_pop_push_cycles() {
        let next = Arc::new((0..64).map(|_| AtomicU32::new(0)).collect::<Vec<_>>());
        let pool = Arc::new(ManualAtomicPool::initialize(0x400000, 64, 64, &next).unwrap());
        let mut workers = Vec::new();
        for _ in 0..8 {
            let next = Arc::clone(&next);
            let pool = Arc::clone(&pool);
            workers.push(thread::spawn(move || {
                for _ in 0..20_000 {
                    if let Some(block) = pool.allocate(&next) {
                        assert!(pool.release(&next, block));
                    }
                }
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let mut blocks = Vec::new();
        while let Some(block) = pool.allocate(&next) {
            blocks.push(block);
        }
        assert_eq!(blocks.len(), 64);
        assert_eq!(
            blocks
                .iter()
                .map(|b| b.address)
                .collect::<HashSet<_>>()
                .len(),
            64
        );
    }
}
