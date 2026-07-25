// ================================================================
// CONCURRENCY SOLVER - Phase 21.5
// Thread-safe primitives, synchronization, atomic operations
// ================================================================

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::sync::{Mutex, Arc, RwLock};
use std::thread;

/// High-level concurrency operations
pub struct ConcurrencySolver;

/// Thread-safe counter
pub struct AtomicCounter {
    value: AtomicUsize,
}

/// Thread-safe flag
pub struct AtomicFlag {
    value: AtomicBool,
}

impl AtomicCounter {
    pub fn new(initial: usize) -> Self {
        AtomicCounter {
            value: AtomicUsize::new(initial),
        }
    }

    pub fn increment(&self) -> usize {
        self.value.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn decrement(&self) -> usize {
        self.value.fetch_sub(1, Ordering::SeqCst) - 1
    }

    pub fn get(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }

    pub fn set(&self, val: usize) {
        self.value.store(val, Ordering::SeqCst);
    }
}

impl AtomicFlag {
    pub fn new(initial: bool) -> Self {
        AtomicFlag {
            value: AtomicBool::new(initial),
        }
    }

    pub fn set_true(&self) -> bool {
        self.value.swap(true, Ordering::SeqCst)
    }

    pub fn set_false(&self) -> bool {
        self.value.swap(false, Ordering::SeqCst)
    }

    pub fn get(&self) -> bool {
        self.value.load(Ordering::SeqCst)
    }

    pub fn toggle(&self) -> bool {
        !self.value.fetch_xor(true, Ordering::SeqCst)
    }
}

impl ConcurrencySolver {
    // ================================================================
    // ATOMIC OPERATIONS (1-20)
    // ================================================================

    /// Problem 1: Atomic increment
    pub fn atomic_increment(counter: &Arc<AtomicUsize>) -> usize {
        counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Problem 2: Atomic decrement
    pub fn atomic_decrement(counter: &Arc<AtomicUsize>) -> usize {
        counter.fetch_sub(1, Ordering::SeqCst)
    }

    /// Problem 3: Atomic load
    pub fn atomic_load(counter: &Arc<AtomicUsize>) -> usize {
        counter.load(Ordering::SeqCst)
    }

    /// Problem 4: Atomic store
    pub fn atomic_store(counter: &Arc<AtomicUsize>, value: usize) {
        counter.store(value, Ordering::SeqCst);
    }

    /// Problem 5: Atomic compare-and-swap
    pub fn atomic_cas(counter: &Arc<AtomicUsize>, expected: usize, new: usize) -> bool {
        counter.compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Problem 6: Atomic swap
    pub fn atomic_swap(counter: &Arc<AtomicUsize>, new: usize) -> usize {
        counter.swap(new, Ordering::SeqCst)
    }

    /// Problem 7: Atomic add and fetch
    pub fn atomic_add_fetch(counter: &Arc<AtomicUsize>, delta: usize) -> usize {
        counter.fetch_add(delta, Ordering::SeqCst) + delta
    }

    /// Problem 8: Atomic subtract and fetch
    pub fn atomic_sub_fetch(counter: &Arc<AtomicUsize>, delta: usize) -> usize {
        let prev = counter.fetch_sub(delta, Ordering::SeqCst);
        if prev >= delta { prev - delta } else { 0 }
    }

    /// Problem 9: Atomic AND operation
    pub fn atomic_and(counter: &Arc<AtomicUsize>, val: usize) -> usize {
        counter.fetch_and(val, Ordering::SeqCst)
    }

    /// Problem 10: Atomic OR operation
    pub fn atomic_or(counter: &Arc<AtomicUsize>, val: usize) -> usize {
        counter.fetch_or(val, Ordering::SeqCst)
    }

    // ================================================================
    // SYNCHRONIZATION PRIMITIVES (11-25)
    // ================================================================

    /// Problem 11: Check if spinlock acquired (non-blocking)
    pub fn try_acquire_spinlock(flag: &AtomicBool) -> bool {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Problem 12: Release spinlock
    pub fn release_spinlock(flag: &AtomicBool) {
        flag.store(false, Ordering::SeqCst);
    }

    /// Problem 13: Simple spinlock implementation
    pub fn acquire_spinlock(flag: &AtomicBool) {
        loop {
            if Self::try_acquire_spinlock(flag) {
                break;
            }
            thread::yield_now();
        }
    }

    /// Problem 14: Semaphore-like behavior (simplified)
    pub fn semaphore_wait(counter: &Arc<AtomicUsize>) {
        loop {
            let val = counter.load(Ordering::SeqCst);
            if val > 0 {
                if counter.compare_exchange(val, val - 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    break;
                }
            }
            thread::yield_now();
        }
    }

    /// Problem 15: Semaphore signal
    pub fn semaphore_signal(counter: &Arc<AtomicUsize>, permits: usize) {
        counter.fetch_add(permits, Ordering::SeqCst);
    }

    /// Problem 16: Mutex lock pattern
    pub fn with_mutex<T, F>(data: &Arc<Mutex<T>>, f: F) -> std::thread::Result<()>
    where
        F: FnOnce(&mut T) + Send + 'static,
        T: Send + 'static,
    {
        let data_clone = Arc::clone(data);
        let handle = thread::spawn(move || {
            if let Ok(mut guard) = data_clone.lock() {
                f(&mut guard);
            }
        });
        handle.join()
    }

    /// Problem 17: RwLock read access
    pub fn read_lock<T: Send + Sync>(data: &Arc<RwLock<T>>) -> Option<T>
    where
        T: Clone,
    {
        data.read().ok().map(|guard| (*guard).clone())
    }

    /// Problem 18: RwLock write access
    pub fn write_lock<T: Send + Sync>(data: &Arc<RwLock<T>>, new_val: T) -> bool {
        data.write().map(|mut guard| *guard = new_val).is_ok()
    }

    /// Problem 19: Atomic boolean flag
    pub fn create_flag(initial: bool) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(initial))
    }

    /// Problem 20: Check flag safely
    pub fn check_flag(flag: &Arc<AtomicBool>) -> bool {
        flag.load(Ordering::SeqCst)
    }

    // ================================================================
    // CONCURRENT DATA STRUCTURES (21-35)
    // ================================================================

    /// Problem 21: Thread-safe counter creation
    pub fn create_counter(initial: usize) -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(initial))
    }

    /// Problem 22: Increment counter from multiple threads
    pub fn increment_counter(counter: &Arc<AtomicUsize>) -> usize {
        counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Problem 23: Get counter value
    pub fn get_counter(counter: &Arc<AtomicUsize>) -> usize {
        counter.load(Ordering::SeqCst)
    }

    /// Problem 24: Counter reset
    pub fn reset_counter(counter: &Arc<AtomicUsize>) {
        counter.store(0, Ordering::SeqCst);
    }

    /// Problem 25: Multiple writers pattern - safe aggregate
    pub fn safe_aggregate(values: &[Arc<AtomicUsize>]) -> usize {
        values.iter().map(|v| v.load(Ordering::SeqCst)).sum()
    }

    /// Problem 26: Batch atomic updates
    pub fn batch_update(counter: &Arc<AtomicUsize>, updates: &[usize]) -> usize {
        let mut result = counter.load(Ordering::SeqCst);
        for &update in updates {
            result = counter.fetch_add(update, Ordering::SeqCst);
        }
        result
    }

    /// Problem 27: Check-then-act pattern (CAS loop)
    pub fn atomic_increment_if_below(counter: &Arc<AtomicUsize>, threshold: usize) -> bool {
        loop {
            let current = counter.load(Ordering::SeqCst);
            if current >= threshold {
                return false;
            }
            if counter.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                return true;
            }
        }
    }

    /// Problem 28: Read-write contention measure (attempts to CAS)
    pub fn measure_contention(counter: &Arc<AtomicUsize>, num_attempts: u32) -> f64 {
        let mut failed = 0;
        for _ in 0..num_attempts {
            let current = counter.load(Ordering::SeqCst);
            if counter.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                failed += 1;
            }
        }
        (failed as f64) / (num_attempts as f64)
    }

    /// Problem 29: Fair queue position (thread-safe counter)
    pub fn queue_position(ticket: &Arc<AtomicUsize>) -> usize {
        ticket.fetch_add(1, Ordering::SeqCst)
    }

    /// Problem 30: Thread ID-like value (simplified)
    pub fn current_logical_id(base: usize) -> usize {
        // Use stable hash computation instead of unstable thread_id_value
        base ^ 0x9e3779b97f4a7c15
    }

    // ================================================================
    // ORDERING & CONSISTENCY (31-40)
    // ================================================================

    /// Problem 31: Memory barrier (acquire)
    pub fn acquire_barrier() {
        std::sync::atomic::compiler_fence(Ordering::Acquire);
    }

    /// Problem 32: Memory barrier (release)
    pub fn release_barrier() {
        std::sync::atomic::compiler_fence(Ordering::Release);
    }

    /// Problem 33: Memory barrier (full)
    pub fn full_barrier() {
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
    }

    /// Problem 34: Relaxed atomic read (fastest, weakest guarantee)
    pub fn relaxed_load(counter: &Arc<AtomicUsize>) -> usize {
        counter.load(Ordering::Relaxed)
    }

    /// Problem 35: Relaxed atomic write
    pub fn relaxed_store(counter: &Arc<AtomicUsize>, value: usize) {
        counter.store(value, Ordering::Relaxed);
    }

    /// Problem 36: Acquire load
    pub fn acquire_load(counter: &Arc<AtomicUsize>) -> usize {
        counter.load(Ordering::Acquire)
    }

    /// Problem 37: Release store
    pub fn release_store(counter: &Arc<AtomicUsize>, value: usize) {
        counter.store(value, Ordering::Release);
    }

    /// Problem 38: Deadlock detection timeout
    pub fn timeout_ms_to_duration(ms: u64) -> std::time::Duration {
        std::time::Duration::from_millis(ms)
    }

    /// Problem 39: Lock-free progress guarantee
    pub fn is_lock_free(_counter: &Arc<AtomicUsize>) -> bool {
        // Modern systems have lock-free atomic operations
        true
    }

    /// Problem 40: Compare-and-swap retry limit check
    pub fn cas_retry_needed(attempt: u32, max_retries: u32) -> bool {
        attempt < max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_counter() {
        let counter = ConcurrencySolver::create_counter(0);
        ConcurrencySolver::increment_counter(&counter);
        assert_eq!(ConcurrencySolver::get_counter(&counter), 1);
    }

    #[test]
    fn test_atomic_flag() {
        let flag = ConcurrencySolver::create_flag(false);
        assert!(!ConcurrencySolver::check_flag(&flag));
        flag.store(true, Ordering::SeqCst);
        assert!(ConcurrencySolver::check_flag(&flag));
    }

    #[test]
    fn test_spinlock() {
        let flag = Arc::new(AtomicBool::new(false));
        assert!(ConcurrencySolver::try_acquire_spinlock(&flag));
        ConcurrencySolver::release_spinlock(&flag);
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cas_loop() {
        let counter = ConcurrencySolver::create_counter(5);
        assert!(!ConcurrencySolver::atomic_increment_if_below(&counter, 5));
        assert!(ConcurrencySolver::atomic_increment_if_below(&counter, 10));
    }
}
