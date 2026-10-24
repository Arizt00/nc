use ncc_ast::{
    validate_atomic_order, validate_compare_exchange_orders, AtomicOperation, MemoryOrder,
};
use std::collections::HashMap;
use std::sync::atomic::{self, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

fn ordering(order: MemoryOrder) -> Ordering {
    match order {
        MemoryOrder::Relaxed => Ordering::Relaxed,
        MemoryOrder::Acquire => Ordering::Acquire,
        MemoryOrder::Release => Ordering::Release,
        MemoryOrder::AcqRel => Ordering::AcqRel,
        MemoryOrder::SeqCst => Ordering::SeqCst,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicWaitResult {
    Changed(i64),
    TimedOut(i64),
}

#[derive(Debug)]
pub struct AtomicI64Cell {
    value: AtomicI64,
    notification_generation: Mutex<u64>,
    notification: Condvar,
}

impl AtomicI64Cell {
    pub fn new(value: i64) -> Self {
        Self {
            value: AtomicI64::new(value),
            notification_generation: Mutex::new(0),
            notification: Condvar::new(),
        }
    }

    pub fn load(&self, order: MemoryOrder) -> Result<i64, &'static str> {
        validate_atomic_order(AtomicOperation::Load, order)?;
        Ok(self.value.load(ordering(order)))
    }

    pub fn store(&self, value: i64, order: MemoryOrder) -> Result<(), &'static str> {
        validate_atomic_order(AtomicOperation::Store, order)?;
        self.value.store(value, ordering(order));
        Ok(())
    }

    pub fn exchange(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.swap(value, ordering(order))
    }

    pub fn fetch_add(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.fetch_add(value, ordering(order))
    }

    pub fn fetch_sub(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.fetch_sub(value, ordering(order))
    }

    pub fn fetch_and(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.fetch_and(value, ordering(order))
    }

    pub fn fetch_or(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.fetch_or(value, ordering(order))
    }

    pub fn fetch_xor(&self, value: i64, order: MemoryOrder) -> i64 {
        self.value.fetch_xor(value, ordering(order))
    }

    pub fn volatile_load(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn volatile_store(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn compare_exchange(
        &self,
        current: i64,
        new: i64,
        success: MemoryOrder,
        failure: MemoryOrder,
    ) -> Result<Result<i64, i64>, &'static str> {
        validate_compare_exchange_orders(success, failure)?;
        Ok(self
            .value
            .compare_exchange(current, new, ordering(success), ordering(failure)))
    }

    pub fn compare_exchange_weak(
        &self,
        current: i64,
        new: i64,
        success: MemoryOrder,
        failure: MemoryOrder,
    ) -> Result<Result<i64, i64>, &'static str> {
        validate_compare_exchange_orders(success, failure)?;
        Ok(self
            .value
            .compare_exchange_weak(current, new, ordering(success), ordering(failure)))
    }

    pub fn wait(
        &self,
        expected: i64,
        order: MemoryOrder,
        timeout: Option<Duration>,
    ) -> Result<AtomicWaitResult, &'static str> {
        validate_atomic_order(AtomicOperation::Wait, order)?;
        let deadline = timeout.and_then(|duration| Instant::now().checked_add(duration));
        let mut generation = self
            .notification_generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        loop {
            let current = self.value.load(ordering(order));
            if current != expected {
                return Ok(AtomicWaitResult::Changed(current));
            }
            let observed_generation = *generation;
            match deadline {
                Some(deadline) => {
                    let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                        return Ok(AtomicWaitResult::TimedOut(current));
                    };
                    let (next, result) = self
                        .notification
                        .wait_timeout_while(generation, remaining, |value| {
                            *value == observed_generation
                                && self.value.load(Ordering::Relaxed) == expected
                        })
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    generation = next;
                    if result.timed_out() && self.value.load(ordering(order)) == expected {
                        return Ok(AtomicWaitResult::TimedOut(expected));
                    }
                }
                None => {
                    generation = self
                        .notification
                        .wait_while(generation, |value| {
                            *value == observed_generation
                                && self.value.load(Ordering::Relaxed) == expected
                        })
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            }
        }
    }

    fn publish_notification(&self, all: bool) -> usize {
        let mut generation = self
            .notification_generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *generation = generation.wrapping_add(1);
        drop(generation);
        if all {
            self.notification.notify_all();
        } else {
            self.notification.notify_one();
        }
        if all {
            usize::MAX
        } else {
            1
        }
    }

    pub fn notify_one(&self) -> usize {
        self.publish_notification(false)
    }

    pub fn notify_all(&self) -> usize {
        self.publish_notification(true)
    }
}

pub fn atomic_fence(order: MemoryOrder) {
    atomic::fence(ordering(order));
}

pub fn compiler_fence(order: MemoryOrder) {
    atomic::compiler_fence(ordering(order));
}

/// Performs an observable volatile load. The caller owns pointer validity and alignment.
pub unsafe fn volatile_read<T: Copy>(address: *const T) -> T {
    std::ptr::read_volatile(address)
}

/// Performs an observable volatile store. The caller owns pointer validity and alignment.
pub unsafe fn volatile_write<T>(address: *mut T, value: T) {
    std::ptr::write_volatile(address, value);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomicRegistryError {
    InvalidHandle(u64),
    InvalidOrder(&'static str),
}

#[derive(Debug, Default)]
pub struct AtomicRegistry {
    next_handle: AtomicU64,
    cells: RwLock<HashMap<u64, Arc<AtomicI64Cell>>>,
}

impl AtomicRegistry {
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU64::new(1),
            cells: RwLock::new(HashMap::new()),
        }
    }

    pub fn create(&self, initial: i64) -> u64 {
        let handle = self.next_handle.fetch_add(1, Ordering::Relaxed);
        self.cells
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(handle, Arc::new(AtomicI64Cell::new(initial)));
        handle
    }

    pub fn remove(&self, handle: u64) -> Result<(), AtomicRegistryError> {
        self.cells
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&handle)
            .map(|_| ())
            .ok_or(AtomicRegistryError::InvalidHandle(handle))
    }

    pub fn get(&self, handle: u64) -> Result<Arc<AtomicI64Cell>, AtomicRegistryError> {
        self.cells
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&handle)
            .cloned()
            .ok_or(AtomicRegistryError::InvalidHandle(handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn hosted_atomic_operations_preserve_c11_orders() {
        let cell = AtomicI64Cell::new(4);
        assert_eq!(cell.fetch_add(3, MemoryOrder::AcqRel), 4);
        assert_eq!(cell.fetch_xor(2, MemoryOrder::SeqCst), 7);
        assert_eq!(cell.load(MemoryOrder::Acquire), Ok(5));
        assert_eq!(
            cell.compare_exchange(5, 9, MemoryOrder::AcqRel, MemoryOrder::Acquire),
            Ok(Ok(5))
        );
        assert_eq!(cell.load(MemoryOrder::Relaxed), Ok(9));
    }

    #[test]
    fn wait_notify_has_no_polling_or_lost_notification_contract() {
        let cell = Arc::new(AtomicI64Cell::new(0));
        let waiter = Arc::clone(&cell);
        let join = thread::spawn(move || {
            waiter
                .wait(0, MemoryOrder::Acquire, Some(Duration::from_secs(2)))
                .expect("valid wait")
        });
        thread::sleep(Duration::from_millis(20));
        cell.store(1, MemoryOrder::Release).expect("valid store");
        cell.notify_all();
        assert_eq!(
            join.join().expect("waiter joins"),
            AtomicWaitResult::Changed(1)
        );
    }

    #[test]
    fn volatile_access_is_observable_and_exact() {
        let mut value = 3u64;
        unsafe {
            volatile_write(&mut value, 17);
            assert_eq!(volatile_read(&value), 17);
        }
    }
}
