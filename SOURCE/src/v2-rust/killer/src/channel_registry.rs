use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;
use crate::value::Value;

struct ChannelEntry {
    tx: mpsc::Sender<Value>,
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
        map.insert(id, ChannelEntry { tx, rx: Arc::new(Mutex::new(rx)) });
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
        map.get(&id).ok_or_else(|| format!("channel {} not found", id))?.tx.clone()
    };
    // the receiving thread gets its own copy: values must not share reference counts across threads
    tx.send(val.detached()).map_err(|_| "channel closed".to_string())
}

pub fn recv(id: i64) -> Result<Value, String> {
    let rx = receiver(id)?;
    let rx = rx.lock().map_err(|_| "channel poisoned".to_string())?;
    rx.recv_timeout(Duration::from_secs(30))
        .map_err(|_| format!("channel {} recv timed out or closed", id))
}

pub fn try_recv(id: i64) -> Option<Value> {
    let rx = receiver(id).ok()?;
    let rx = rx.lock().ok()?;
    rx.try_recv().ok()
}

pub fn close(id: i64) {
    if let Ok(mut map) = registry().lock() {
        map.remove(&id);
    }
}
