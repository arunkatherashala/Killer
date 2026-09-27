// ============================================================================
// TXN Registry — Global transaction handle store for kore_txn builtins
// ============================================================================
//
// Wraps KoreTxn handles in a global Mutex<HashMap<i64, KoreTxn>> so that
// Killer scripts can call kore_txn_begin() and later kore_txn_commit(id) or
// kore_txn_abort(id) without holding a Rust reference across builtin calls.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicI64, Ordering};
use crate::kore_txn::KoreTxn;

static REGISTRY: OnceLock<Mutex<HashMap<i64, KoreTxn>>> = OnceLock::new();
static NEXT_ID: AtomicI64 = AtomicI64::new(1);

fn registry() -> &'static Mutex<HashMap<i64, KoreTxn>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Begin a new transaction for the given KORE file path.
/// Returns the transaction handle ID (used for commit/abort).
pub fn txn_begin(path: &str) -> i64 {
    let txn = KoreTxn::begin(path);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut map) = registry().lock() {
        map.insert(id, txn);
    }
    id
}

/// Commit the transaction with the given ID and message.
/// Removes the handle from the registry.
pub fn txn_commit(id: i64, message: &str) -> Result<u64, String> {
    let mut map = registry().lock().map_err(|_| "registry poisoned".to_string())?;
    let txn = map.remove(&id).ok_or_else(|| format!("txn {} not found", id))?;
    txn.commit(message)
}

/// Abort the transaction with the given ID.
pub fn txn_abort(id: i64) {
    if let Ok(mut map) = registry().lock() {
        if let Some(txn) = map.remove(&id) {
            txn.abort();
        }
    }
}

/// Return the temp path for the transaction (used for writing data before commit).
pub fn txn_temp_path(id: i64) -> Option<String> {
    if let Ok(map) = registry().lock() {
        return map.get(&id).map(|t| t.temp_path().to_string());
    }
    None
}
