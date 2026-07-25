/// Killer Concurrency Primitives
/// Channels, mutexes, semaphores, and synchronization primitives
///
/// Features:
/// - MPSC Channels (Multi-Producer, Single-Consumer)
/// - Mutex for shared state
/// - RwLock for read-write synchronization
/// - Barrier for thread synchronization
/// - Atomic operations
/// - Thread spawning

use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use crate::error_handling::Result;

/// MPSC Channel sender
pub struct Sender<T> {
    tx: mpsc::Sender<T>,
}

/// MPSC Channel receiver
pub struct Receiver<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> Sender<T> {
    /// Send value down channel
    pub fn send(&self, value: T) -> Result<()> {
        self.tx.send(value).ok();
        Ok(())
    }
}

impl<T> Receiver<T> {
    /// Receive value from channel (blocking)
    pub fn recv(&self) -> Result<Option<T>> {
        match self.rx.recv_timeout(Duration::from_secs(60)) {
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None),
        }
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Result<Option<T>> {
        match self.rx.try_recv() {
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None),
        }
    }
}

/// Create a new MPSC channel
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel();
    (Sender { tx }, Receiver { rx })
}

/// Shared mutable state protected by mutex
pub struct SharedMutex<T> {
    inner: Arc<std::sync::Mutex<T>>,
}

impl<T> Clone for SharedMutex<T> {
    fn clone(&self) -> Self {
        SharedMutex {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedMutex<T> {
    /// Create new mutex
    pub fn new(value: T) -> SharedMutex<T> {
        SharedMutex {
            inner: Arc::new(std::sync::Mutex::new(value)),
        }
    }

    /// Lock and access value
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>> {
        Ok(self.inner.lock().unwrap())
    }

    /// Try to lock without blocking
    pub fn try_lock(&self) -> Result<Option<std::sync::MutexGuard<'_, T>>> {
        match self.inner.try_lock() {
            Ok(guard) => Ok(Some(guard)),
            Err(_) => Ok(None),
        }
    }
}

/// Read-write lock (multiple readers, exclusive writer)
pub struct SharedRwLock<T> {
    inner: Arc<std::sync::RwLock<T>>,
}

impl<T> Clone for SharedRwLock<T> {
    fn clone(&self) -> Self {
        SharedRwLock {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> SharedRwLock<T> {
    /// Create new RwLock
    pub fn new(value: T) -> SharedRwLock<T> {
        SharedRwLock {
            inner: Arc::new(std::sync::RwLock::new(value)),
        }
    }

    /// Acquire read lock
    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<'_, T>> {
        Ok(self.inner.read().unwrap())
    }

    /// Acquire write lock
    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<'_, T>> {
        Ok(self.inner.write().unwrap())
    }
}

/// Synchronization barrier
pub struct SyncBarrier {
    inner: Arc<std::sync::Barrier>,
}

impl SyncBarrier {
    /// Create barrier for N threads
    pub fn new(n: usize) -> SyncBarrier {
        SyncBarrier {
            inner: Arc::new(std::sync::Barrier::new(n)),
        }
    }

    /// Wait for all threads to reach barrier
    pub fn wait(&self) {
        self.inner.wait();
    }
}

/// Spawned thread handle
pub struct JoinHandle<T> {
    handle: thread::JoinHandle<T>,
}

impl<T: Send + 'static> JoinHandle<T> {
    /// Wait for thread to finish and get result
    pub fn join(self) -> Result<T> {
        Ok(self.handle.join().unwrap())
    }
}

/// Spawn a new thread
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let handle = thread::spawn(f);
    JoinHandle { handle }
}

/// Sleep for duration (in milliseconds)
pub fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// Atomic operations
pub mod atomic {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    pub struct AtomicU64 {
        inner: Arc<AtomicUsize>,
    }

    impl Clone for AtomicU64 {
        fn clone(&self) -> Self {
            AtomicU64 {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl AtomicU64 {
        pub fn new(value: usize) -> AtomicU64 {
            AtomicU64 {
                inner: Arc::new(AtomicUsize::new(value)),
            }
        }

        pub fn load(&self) -> usize {
            self.inner.load(Ordering::Relaxed)
        }

        pub fn store(&self, value: usize) {
            self.inner.store(value, Ordering::Relaxed);
        }

        pub fn fetch_add(&self, delta: usize) -> usize {
            self.inner.fetch_add(delta, Ordering::Relaxed)
        }

        pub fn fetch_sub(&self, delta: usize) -> usize {
            self.inner.fetch_sub(delta, Ordering::Relaxed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel() {
        let (tx, rx) = channel::<i32>();
        let _ = tx.send(42);
        let val = rx.recv().unwrap();
        assert_eq!(val, Some(42));
    }

    #[test]
    fn test_mutex() {
        let m = SharedMutex::new(0);
        {
            let mut guard = m.lock().unwrap();
            *guard = 5;
        }
        let guard = m.lock().unwrap();
        assert_eq!(*guard, 5);
    }

    #[test]
    fn test_rwlock() {
        let lock = SharedRwLock::new(vec![1, 2, 3]);
        {
            let reader = lock.read().unwrap();
            assert_eq!(reader.len(), 3);
        }
        {
            let _writer = lock.write().unwrap();
            // Ideally would mutate here
        }
    }

    #[test]
    fn test_spawn() {
        let handle = spawn(|| {
            42
        });
        let result = handle.join().unwrap();
        assert_eq!(result, 42);
    }
}
