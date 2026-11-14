// ================================================================
// DISTRIBUTED LOCKS - Phase 28.4
// Mutual exclusion across cluster
// ================================================================

use std::collections::HashMap;
use std::time::SystemTime;

/// Lock state
#[derive(Clone, Debug)]
pub enum LockState {
    Available,
    Locked,
    Expired,
}

/// Lock information
#[derive(Clone, Debug)]
pub struct LockInfo {
    pub lock_id: String,
    pub owner: String,
    pub acquired_at: u64,
    pub ttl: u64,
    pub state: LockState,
}

/// Reader/Writer lock permission
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LockPermission {
    Read,
    Write,
    None,
}

pub struct DistributedLocksSolver;

impl DistributedLocksSolver {
    // ================================================================
    // BASIC LOCKING (1-12)
    // ================================================================

    /// Problem 1: Acquire lock
    pub fn acquire_lock(
        lock_id: &str,
        owner: &str,
        now: u64,
    ) -> Result<LockInfo, String> {
        Ok(LockInfo {
            lock_id: lock_id.to_string(),
            owner: owner.to_string(),
            acquired_at: now,
            ttl: 30000,
            state: LockState::Locked,
        })
    }

    /// Problem 2: Release lock
    pub fn release_lock(lock: &mut LockInfo) {
        lock.state = LockState::Available;
    }

    /// Problem 3: Check lock status
    pub fn check_lock_status(lock: &LockInfo) -> LockState {
        lock.state.clone()
    }

    /// Problem 4: Get lock owner
    pub fn get_lock_owner(lock: &LockInfo) -> String {
        lock.owner.clone()
    }

    /// Problem 5: Validate lock ownership
    pub fn validate_lock_ownership(lock: &LockInfo, owner: &str) -> bool {
        lock.owner == owner
    }

    /// Problem 6: Create lock ID
    pub fn create_lock_id(resource: &str, sequence: u64) -> String {
        format!("{}_lock_{}", resource, sequence)
    }

    /// Problem 7: Store lock
    pub fn store_lock(
        locks: &mut HashMap<String, LockInfo>,
        lock: LockInfo,
    ) {
        locks.insert(lock.lock_id.clone(), lock);
    }

    /// Problem 8: Retrieve lock
    pub fn retrieve_lock(
        locks: &HashMap<String, LockInfo>,
        lock_id: &str,
    ) -> Option<LockInfo> {
        locks.get(lock_id).cloned()
    }

    /// Problem 9: Delete lock
    pub fn delete_lock(locks: &mut HashMap<String, LockInfo>, lock_id: &str) {
        locks.remove(lock_id);
    }

    /// Problem 10: List active locks
    pub fn list_active_locks(locks: &HashMap<String, LockInfo>) -> Vec<String> {
        locks
            .values()
            .filter(|l| matches!(l.state, LockState::Locked))
            .map(|l| l.lock_id.clone())
            .collect()
    }

    /// Problem 11: Count locks by owner
    pub fn count_locks_by_owner(
        locks: &HashMap<String, LockInfo>,
        owner: &str,
    ) -> usize {
        locks
            .values()
            .filter(|l| l.owner == owner && matches!(l.state, LockState::Locked))
            .count()
    }

    /// Problem 12: Find lock conflicts
    pub fn find_lock_conflicts(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
    ) -> Vec<String> {
        locks
            .values()
            .filter(|l| l.lock_id.starts_with(resource) && matches!(l.state, LockState::Locked))
            .map(|l| l.owner.clone())
            .collect()
    }

    // ================================================================
    // LOCK EXPIRATION (13-24)
    // ================================================================

    /// Problem 13: Check expiration
    pub fn check_expiration(lock: &LockInfo, now: u64) -> bool {
        now > lock.acquired_at + lock.ttl
    }

    /// Problem 14: Set TTL
    pub fn set_ttl(lock: &mut LockInfo, ttl: u64) {
        lock.ttl = ttl;
    }

    /// Problem 15: Get remaining TTL
    pub fn get_remaining_ttl(lock: &LockInfo, now: u64) -> u64 {
        let elapsed = now - lock.acquired_at;
        if elapsed >= lock.ttl {
            0
        } else {
            lock.ttl - elapsed
        }
    }

    /// Problem 16: Expire lock
    pub fn expire_lock(lock: &mut LockInfo) {
        lock.state = LockState::Expired;
    }

    /// Problem 17: Extend lock TTL
    pub fn extend_lock_ttl(
        lock: &mut LockInfo,
        additional_ttl: u64,
        owner: &str,
    ) -> Result<(), String> {
        if lock.owner == owner {
            lock.ttl += additional_ttl;
            Ok(())
        } else {
            Err("Not lock owner".to_string())
        }
    }

    /// Problem 18: Cleanup expired locks
    pub fn cleanup_expired_locks(
        locks: &mut HashMap<String, LockInfo>,
        now: u64,
    ) -> usize {
        let expired: Vec<String> = locks
            .iter()
            .filter(|(_, lock)| Self::check_expiration(lock, now))
            .map(|(id, _)| id.clone())
            .collect();
        let count = expired.len();
        for lock_id in expired {
            locks.remove(&lock_id);
        }
        count
    }

    /// Problem 19: Schedule expiration callback
    pub fn schedule_expiration_callback(ttl: u64) -> u64 {
        ttl * 1000
    }

    /// Problem 20: Detect stale locks
    pub fn detect_stale_locks(
        locks: &HashMap<String, LockInfo>,
        now: u64,
        stale_threshold: u64,
    ) -> Vec<String> {
        locks
            .iter()
            .filter(|(_, lock)| {
                now > lock.acquired_at + lock.ttl + stale_threshold
                    && matches!(lock.state, LockState::Expired)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Problem 21: Renew lock before expiration
    pub fn renew_lock_before_expiration(
        lock: &mut LockInfo,
        now: u64,
        owner: &str,
        renewal_threshold: u64,
    ) -> bool {
        if lock.owner == owner {
            let remaining_ttl = Self::get_remaining_ttl(lock, now);
            if remaining_ttl < renewal_threshold {
                lock.ttl += renewal_threshold;
                return true;
            }
        }
        false
    }

    /// Problem 22: Handle expiration backoff
    pub fn handle_expiration_backoff(attempt: u32) -> u64 {
        if attempt > 0 {
            (1000 * (2u64).pow(attempt - 1)).min(30000)
        } else {
            1000
        }
    }

    /// Problem 23: Update lock timestamp
    pub fn update_lock_timestamp(lock: &mut LockInfo, now: u64) {
        lock.acquired_at = now;
    }

    /// Problem 24: Validate lock freshness
    pub fn validate_lock_freshness(
        lock: &LockInfo,
        now: u64,
        max_age: u64,
    ) -> bool {
        now <= lock.acquired_at + max_age
    }

    // ================================================================
    // READER-WRITER LOCKS (25-36)
    // ================================================================

    /// Problem 25: Grant read permission
    pub fn grant_read_permission(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
        reader: &str,
    ) -> LockPermission {
        let write_locks: Vec<_> = locks
            .values()
            .filter(|l| {
                l.lock_id.starts_with(resource)
                    && l.owner != reader
                    && matches!(l.state, LockState::Locked)
            })
            .collect();

        if write_locks.is_empty() {
            LockPermission::Read
        } else {
            LockPermission::None
        }
    }

    /// Problem 26: Grant write permission
    pub fn grant_write_permission(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
        writer: &str,
    ) -> LockPermission {
        let existing: Vec<_> = locks
            .values()
            .filter(|l| {
                l.lock_id.starts_with(resource)
                    && l.owner != writer
                    && matches!(l.state, LockState::Locked)
            })
            .collect();

        if existing.is_empty() {
            LockPermission::Write
        } else {
            LockPermission::None
        }
    }

    /// Problem 27: Count active readers
    pub fn count_active_readers(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
    ) -> usize {
        locks
            .values()
            .filter(|l| {
                l.lock_id.starts_with(&format!("{}_read", resource))
                    && matches!(l.state, LockState::Locked)
            })
            .count()
    }

    /// Problem 28: Count active writers
    pub fn count_active_writers(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
    ) -> usize {
        locks
            .values()
            .filter(|l| {
                l.lock_id.starts_with(&format!("{}_write", resource))
                    && matches!(l.state, LockState::Locked)
            })
            .count()
    }

    /// Problem 29: Upgrade read to write
    pub fn upgrade_read_to_write(
        locks: &mut HashMap<String, LockInfo>,
        read_lock_id: &str,
        owner: &str,
    ) -> Result<String, String> {
        if let Some(lock) = locks.get_mut(read_lock_id) {
            if lock.owner == owner {
                let resource = lock.lock_id.replace("_read", "");
                let write_lock = LockInfo {
                    lock_id: format!("{}_write", resource),
                    owner: owner.to_string(),
                    acquired_at: lock.acquired_at,
                    ttl: lock.ttl,
                    state: LockState::Locked,
                };
                let write_id = write_lock.lock_id.clone();
                locks.remove(read_lock_id);
                locks.insert(write_lock.lock_id.clone(), write_lock);
                Ok(write_id)
            } else {
                Err("Not owner".to_string())
            }
        } else {
            Err("Lock not found".to_string())
        }
    }

    /// Problem 30: Downgrade write to read
    pub fn downgrade_write_to_read(
        locks: &mut HashMap<String, LockInfo>,
        write_lock_id: &str,
        owner: &str,
    ) -> Result<String, String> {
        if let Some(lock) = locks.get_mut(write_lock_id) {
            if lock.owner == owner {
                let resource = lock.lock_id.replace("_write", "");
                let read_lock = LockInfo {
                    lock_id: format!("{}_read", resource),
                    owner: owner.to_string(),
                    acquired_at: lock.acquired_at,
                    ttl: lock.ttl,
                    state: LockState::Locked,
                };
                let read_id = read_lock.lock_id.clone();
                locks.remove(write_lock_id);
                locks.insert(read_lock.lock_id.clone(), read_lock);
                Ok(read_id)
            } else {
                Err("Not owner".to_string())
            }
        } else {
            Err("Lock not found".to_string())
        }
    }

    /// Problem 31: Detect reader starvation
    pub fn detect_reader_starvation(
        locks: &HashMap<String, LockInfo>,
        resource: &str,
        wait_time: u64,
    ) -> bool {
        let writers = Self::count_active_writers(locks, resource);
        writers > 3 && wait_time > 10000
    }

    /// Problem 32: Prevent writer starvation
    pub fn prevent_writer_starvation(
        reader_count: usize,
        pending_writers: usize,
    ) -> bool {
        pending_writers > 0 && reader_count < 5
    }

    /// Problem 33: Acquire shared read lock
    pub fn acquire_shared_read_lock(
        locks: &mut HashMap<String, LockInfo>,
        resource: &str,
        reader_id: &str,
        now: u64,
    ) -> Result<String, String> {
        let perm = Self::grant_read_permission(locks, resource, reader_id);
        if perm == LockPermission::Read {
            let lock_id = format!("{}_read_{}", resource, reader_id);
            let lock = LockInfo {
                lock_id: lock_id.clone(),
                owner: reader_id.to_string(),
                acquired_at: now,
                ttl: 30000,
                state: LockState::Locked,
            };
            locks.insert(lock.lock_id.clone(), lock);
            Ok(lock_id)
        } else {
            Err("Write lock in progress".to_string())
        }
    }

    /// Problem 34: Release reader lock
    pub fn release_reader_lock(
        locks: &mut HashMap<String, LockInfo>,
        lock_id: &str,
        owner: &str,
    ) -> Result<(), String> {
        if let Some(lock) = locks.get_mut(lock_id) {
            if lock.owner == owner {
                lock.state = LockState::Available;
                locks.remove(lock_id);
                Ok(())
            } else {
                Err("Not owner".to_string())
            }
        } else {
            Err("Lock not found".to_string())
        }
    }

    /// Problem 35: Acquire exclusive write lock
    pub fn acquire_exclusive_write_lock(
        locks: &mut HashMap<String, LockInfo>,
        resource: &str,
        writer_id: &str,
        now: u64,
    ) -> Result<String, String> {
        let perm = Self::grant_write_permission(locks, resource, writer_id);
        if perm == LockPermission::Write {
            let lock_id = format!("{}_write_{}", resource, writer_id);
            let lock = LockInfo {
                lock_id: lock_id.clone(),
                owner: writer_id.to_string(),
                acquired_at: now,
                ttl: 30000,
                state: LockState::Locked,
            };
            locks.insert(lock.lock_id.clone(), lock);
            Ok(lock_id)
        } else {
            Err("Other locks in progress".to_string())
        }
    }

    /// Problem 36: Release writer lock
    pub fn release_writer_lock(
        locks: &mut HashMap<String, LockInfo>,
        lock_id: &str,
        owner: &str,
    ) -> Result<(), String> {
        if let Some(lock) = locks.get_mut(lock_id) {
            if lock.owner == owner {
                lock.state = LockState::Available;
                locks.remove(lock_id);
                Ok(())
            } else {
                Err("Not owner".to_string())
            }
        } else {
            Err("Lock not found".to_string())
        }
    }

    // ================================================================
    // LOCK MANAGER (37-50)
    // ================================================================

    /// Problem 37: Create lock manager
    pub fn create_lock_manager() -> HashMap<String, LockInfo> {
        HashMap::new()
    }

    /// Problem 38: Register lock
    pub fn register_lock(
        manager: &mut HashMap<String, LockInfo>,
        lock: LockInfo,
    ) {
        manager.insert(lock.lock_id.clone(), lock);
    }

    /// Problem 39: Unregister lock
    pub fn unregister_lock(
        manager: &mut HashMap<String, LockInfo>,
        lock_id: &str,
    ) -> bool {
        manager.remove(lock_id).is_some()
    }

    /// Problem 40: Query lock status
    pub fn query_lock_status(
        manager: &HashMap<String, LockInfo>,
        lock_id: &str,
    ) -> Option<LockState> {
        manager.get(lock_id).map(|l| l.state.clone())
    }

    /// Problem 41: List all locks
    pub fn list_all_locks(manager: &HashMap<String, LockInfo>) -> Vec<String> {
        manager.keys().cloned().collect()
    }

    /// Problem 42: Filter locks by owner
    pub fn filter_locks_by_owner(
        manager: &HashMap<String, LockInfo>,
        owner: &str,
    ) -> Vec<String> {
        manager
            .iter()
            .filter(|(_, l)| l.owner == owner)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Problem 43: Get lock statistics
    pub fn get_lock_statistics(
        manager: &HashMap<String, LockInfo>,
    ) -> (usize, usize, usize) {
        let total = manager.len();
        let active = manager
            .values()
            .filter(|l| matches!(l.state, LockState::Locked))
            .count();
        let expired = manager
            .values()
            .filter(|l| matches!(l.state, LockState::Expired))
            .count();
        (total, active, expired)
    }

    /// Problem 44: Maintain lock invariants
    pub fn maintain_lock_invariants(manager: &HashMap<String, LockInfo>) -> bool {
        for (id, lock) in manager {
            if id != &lock.lock_id {
                return false;
            }
        }
        true
    }

    /// Problem 45: Rollback lock state
    pub fn rollback_lock_state(
        manager: &mut HashMap<String, LockInfo>,
        lock_id: &str,
        previous_state: LockState,
    ) -> bool {
        if let Some(lock) = manager.get_mut(lock_id) {
            lock.state = previous_state;
            true
        } else {
            false
        }
    }

    /// Problem 46: Serialize lock state
    pub fn serialize_lock_state(lock: &LockInfo) -> String {
        format!(
            "{}|{}|{}|{}",
            lock.lock_id, lock.owner, lock.acquired_at, lock.ttl
        )
    }

    /// Problem 47: Deserialize lock state
    pub fn deserialize_lock_state(s: &str) -> Option<LockInfo> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() == 4 {
            Some(LockInfo {
                lock_id: parts[0].to_string(),
                owner: parts[1].to_string(),
                acquired_at: parts[2].parse().ok()?,
                ttl: parts[3].parse().ok()?,
                state: LockState::Locked,
            })
        } else {
            None
        }
    }

    /// Problem 48: Migrate lock to peer
    pub fn migrate_lock_to_peer(
        manager: &mut HashMap<String, LockInfo>,
        lock_id: &str,
        peer_id: &str,
    ) -> Result<(), String> {
        if let Some(mut lock) = manager.remove(lock_id) {
            lock.owner = peer_id.to_string();
            manager.insert(lock_id.to_string(), lock);
            Ok(())
        } else {
            Err("Lock not found".to_string())
        }
    }

    /// Problem 49: Detect deadlock
    pub fn detect_deadlock(
        locks: &HashMap<String, LockInfo>,
        wait_graph: &HashMap<String, Vec<String>>,
    ) -> bool {
        for (_waiter, waitees) in wait_graph {
            let mut visited = std::collections::HashSet::new();
            let mut to_visit = vec![_waiter.clone()];
            while let Some(current) = to_visit.pop() {
                if visited.contains(&current) {
                    return true;
                }
                visited.insert(current.clone());
                if let Some(next_waitees) = wait_graph.get(&current) {
                    to_visit.extend(next_waitees.clone());
                }
            }
        }
        false
    }

    /// Problem 50: Break deadlock
    pub fn break_deadlock(
        manager: &mut HashMap<String, LockInfo>,
        victim_lock_id: &str,
    ) -> bool {
        manager.remove(victim_lock_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_lock() {
        let lock = DistributedLocksSolver::acquire_lock("resource1", "client1", 1000).unwrap();
        assert_eq!(lock.owner, "client1");
    }

    #[test]
    fn test_check_expiration() {
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        assert!(!DistributedLocksSolver::check_expiration(&lock, 5000));
        assert!(DistributedLocksSolver::check_expiration(&lock, 35000));
    }

    #[test]
    fn test_rw_locks() {
        let mut locks = HashMap::new();
        let perm = DistributedLocksSolver::grant_read_permission(&locks, "db", "reader1");
        assert_eq!(perm, LockPermission::Read);
    }

    #[test]
    fn test_cleanup_expired_locks() {
        let mut locks = HashMap::new();
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 100,
            state: LockState::Locked,
        };
        locks.insert("test".to_string(), lock);
        let removed = DistributedLocksSolver::cleanup_expired_locks(&mut locks, 5000);
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_create_lock_id() {
        let id = DistributedLocksSolver::create_lock_id("resource1", 5);
        assert!(id.contains("resource1"));
    }

    #[test]
    fn test_extend_ttl() {
        let mut lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        let result = DistributedLocksSolver::extend_lock_ttl(&mut lock, 10000, "client1");
        assert!(result.is_ok());
        assert_eq!(lock.ttl, 40000);
    }

    #[test]
    fn test_list_active_locks() {
        let mut locks = HashMap::new();
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        locks.insert("test".to_string(), lock);
        let active = DistributedLocksSolver::list_active_locks(&locks);
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_lock_statistics() {
        let mut locks = HashMap::new();
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        locks.insert("test".to_string(), lock);
        let (total, active, expired) = DistributedLocksSolver::get_lock_statistics(&locks);
        assert_eq!(total, 1);
        assert_eq!(active, 1);
    }

    #[test]
    fn test_serialize_deserialize() {
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        let serialized = DistributedLocksSolver::serialize_lock_state(&lock);
        let deserialized = DistributedLocksSolver::deserialize_lock_state(&serialized).unwrap();
        assert_eq!(deserialized.lock_id, "test");
    }

    #[test]
    fn test_remaining_ttl() {
        let lock = LockInfo {
            lock_id: "test".to_string(),
            owner: "client1".to_string(),
            acquired_at: 1000,
            ttl: 30000,
            state: LockState::Locked,
        };
        let remaining = DistributedLocksSolver::get_remaining_ttl(&lock, 11000);
        assert_eq!(remaining, 20000);
    }
}
