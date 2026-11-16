use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;
use crate::value::Value;

struct ChannelEntry {
    // None once the channel is closed: queued messages can still be received, then `recv` reports
    // "closed" (dropping the only sender is what wakes receivers that are blocked in `recv`)
    tx: Option<mpsc::Sender<Value>>,
    // shared so that `recv` can wait without holding the registry lock (a blocked receiver
    // must not stop other threads from sending, or closing, any channel)
    rx: Arc<Mutex<mpsc::Receiver<Value>>>,
}

static REGISTRY: OnceLock<Mutex<HashMap<i64, ChannelEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<i64, ChannelEntry>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_ID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

pub fn register(tx: mpsc::Sender<Value>, rx: mpsc::Receiver<Value>) -> i64 {
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut map) = registry().lock() {
        map.insert(id, ChannelEntry { tx: Some(tx), rx: Arc::new(Mutex::new(rx)) });
    }
    id
}

fn receiver(id: i64) -> Result<Arc<Mutex<mpsc::Receiver<Value>>>, String> {
    let map = registry().lock().map_err(|_| "channel registry poisoned".to_string())?;
    map.get(&id)
        .map(|entry| Arc::clone(&entry.rx))
        .ok_or_else(|| format!("channel {} not found", id))
}

pub fn send(id: i64, val: Value) -> Result<(), String> {
    let tx = {
        let map = registry().lock().map_err(|_| "channel registry poisoned".to_string())?;
        let entry = map.get(&id).ok_or_else(|| format!("channel {} not found", id))?;
        entry.tx.clone().ok_or_else(|| format!("channel {} is closed", id))?
    };
    // the receiving thread gets its own copy: values must not share reference counts across threads
    tx.send(val.detached()).map_err(|_| "channel closed".to_string())
}

/// How long `recv` waits before giving up (a deadlock guard). 30 s by default;
/// `KILLER_CHAN_RECV_TIMEOUT_SECS` (1 to 3600) changes it, e.g. for test harnesses.
fn recv_guard() -> Duration {
    let secs = std::env::var("KILLER_CHAN_RECV_TIMEOUT_SECS").ok().and_then(|v| v.trim().parse::<u64>().ok()).filter(|n| (1..=3600).contains(n)).unwrap_or(30);
    Duration::from_secs(secs)
}

pub fn recv(id: i64) -> Result<Value, String> {
    let rx = receiver(id)?;
    let rx = rx.lock().map_err(|_| "channel poisoned".to_string())?;
    rx.recv_timeout(recv_guard()).map_err(|e| match e {
        mpsc::RecvTimeoutError::Timeout => format!("channel {} recv timed out", id),
        mpsc::RecvTimeoutError::Disconnected => format!("channel {} is closed", id),
    })
}

pub fn try_recv(id: i64) -> Option<Value> {
    let rx = receiver(id).ok()?;
    let rx = rx.lock().ok()?;
    rx.try_recv().ok()
}

pub fn close(id: i64) {
    if let Ok(mut map) = registry().lock() {
        if let Some(entry) = map.get_mut(&id) {
            entry.tx = None;
        }
    }
}
