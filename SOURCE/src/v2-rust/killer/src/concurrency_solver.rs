//! Concurrency — 40+ functions (atomic ops, synchronization, thread-safe primitives)

use std::sync::Arc;
use std::sync::Mutex;

pub fn atomic_increment(val: Arc<Mutex<i64>>) -> i64 {
    let mut v = val.lock().unwrap();
    *v += 1;
    *v
}
