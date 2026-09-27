use std::collections::HashMap;
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;
use crate::value::Value;

struct ChannelEntry {
    tx: mpsc::Sender<Value>,
    rx: mpsc::Receiver<Value>,
}

static REGISTRY: OnceLock<Mutex<HashMap<i64, ChannelEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<i64, ChannelEntry>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_ID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

pub fn register(tx: mpsc::Sender<Value>, rx: mpsc::Receiver<Value>) -> i64 {
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut map) = registry().lock() {
        map.insert(id, ChannelEntry { tx, rx });
    }
    id
}

pub fn send(id: i64, val: Value) -> Result<(), String> {
    let map = registry().lock().map_err(|_| "channel registry poisoned".to_string())?;
    let entry = map.get(&id).ok_or_else(|| format!("channel {} not found", id))?;
    entry.tx.send(val).map_err(|_| "channel closed".to_string())
}

pub fn recv(id: i64) -> Result<Value, String> {
    let map = registry().lock().map_err(|_| "channel registry poisoned".to_string())?;
    let entry = map.get(&id).ok_or_else(|| format!("channel {} not found", id))?;
    entry.rx.recv_timeout(Duration::from_secs(30))
        .map_err(|_| format!("channel {} recv timed out or closed", id))
}

pub fn try_recv(id: i64) -> Option<Value> {
    let map = registry().lock().ok()?;
    let entry = map.get(&id)?;
    entry.rx.try_recv().ok()
}

pub fn close(id: i64) {
    if let Ok(mut map) = registry().lock() {
        map.remove(&id);
    }
}
