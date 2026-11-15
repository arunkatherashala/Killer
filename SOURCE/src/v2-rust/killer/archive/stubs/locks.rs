//! Distributed Locks — Coordinate access across processes

use std::sync::Mutex;

pub struct DistributedLock {
    _owner: Mutex<Option<String>>,
}

impl DistributedLock {
    pub fn new() -> Self { Self { _owner: Mutex::new(None) } }
}
