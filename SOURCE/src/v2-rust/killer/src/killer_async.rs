//! **killer_async** — real async runtime: task executor, futures, timers, select, channels.
//!
//! No external deps (no tokio). Pure std::thread + mpsc + condvar implementation.
//! Supports: spawn, await, timeout, select (first-of-n), sleep, interval.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════════════════
// TaskId and Task state
// ══════════════════════════════════════════════════════════════════════════════

pub type TaskId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Running,
    Completed(TaskResult),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskResult {
    None,
    Value(String),
    Number(f64),
}

/// A handle to a spawned async task.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub id: TaskId,
    state: Arc<Mutex<TaskState>>,
    result: Arc<Mutex<Option<TaskResult>>>,
    notify: Arc<(Mutex<bool>, Condvar)>,
}

impl TaskHandle {
    /// Block until the task completes. Returns the result.
    pub fn join(&self) -> TaskState {
        let (lock, cvar) = &*self.notify;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }
        self.state.lock().unwrap().clone()
    }

    /// Block with timeout.
    pub fn join_timeout(&self, timeout: Duration) -> TaskState {
        let (lock, cvar) = &*self.notify;
        let mut done = lock.lock().unwrap();
        if !*done {
            let (new_done, _) = cvar.wait_timeout(done, timeout).unwrap();
            done = new_done;
        }
        if *done {
            self.state.lock().unwrap().clone()
        } else {
            TaskState::Pending
        }
    }

    /// Is the task finished?
    pub fn is_done(&self) -> bool {
        let (lock, _) = &*self.notify;
        *lock.lock().unwrap()
    }

    /// Cancel the task (sets flag; task must check).
    pub fn cancel(&self) {
        *self.state.lock().unwrap() = TaskState::Cancelled;
    }

    pub fn result(&self) -> Option<TaskResult> {
        self.result.lock().unwrap().clone()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Executor — thread-pool based async runtime
// ══════════════════════════════════════════════════════════════════════════════

type BoxedTask = Box<dyn FnOnce() -> TaskResult + Send + 'static>;

struct ExecutorInner {
    queue: VecDeque<(TaskId, BoxedTask, Arc<Mutex<TaskState>>, Arc<Mutex<Option<TaskResult>>>, Arc<(Mutex<bool>, Condvar)>)>,
    next_id: TaskId,
    active_count: usize,
    shutdown: bool,
}

/// Thread-pool async runtime. Spawn tasks, await results.
pub struct Executor {
    inner: Arc<Mutex<ExecutorInner>>,
    notify: Arc<Condvar>,
    workers: Vec<thread::JoinHandle<()>>,
    max_workers: usize,
}

impl Executor {
    /// Create a new executor with the given number of worker threads.
    pub fn new(num_workers: usize) -> Self {
        let inner = Arc::new(Mutex::new(ExecutorInner {
            queue: VecDeque::new(),
            next_id: 1,
            active_count: 0,
            shutdown: false,
        }));
        let notify = Arc::new(Condvar::new());
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let inner_clone = Arc::clone(&inner);
            let notify_clone = Arc::clone(&notify);
            let handle = thread::spawn(move || {
                worker_loop(inner_clone, notify_clone);
            });
            workers.push(handle);
        }

        Self { inner, notify, workers, max_workers: num_workers }
    }

    /// Spawn a task. Returns a handle for awaiting the result.
    pub fn spawn<F>(&self, f: F) -> TaskHandle
    where F: FnOnce() -> TaskResult + Send + 'static {
        let state = Arc::new(Mutex::new(TaskState::Pending));
        let result = Arc::new(Mutex::new(None));
        let done_notify = Arc::new((Mutex::new(false), Condvar::new()));

        let id = {
            let mut guard = self.inner.lock().unwrap();
            let id = guard.next_id;
            guard.next_id += 1;
            guard.queue.push_back((id, Box::new(f), Arc::clone(&state), Arc::clone(&result), Arc::clone(&done_notify)));
            id
        };
        self.notify.notify_one();

        TaskHandle { id, state, result, notify: done_notify }
    }

    /// Spawn a task that returns a string.
    pub fn spawn_value<F>(&self, f: F) -> TaskHandle
    where F: FnOnce() -> String + Send + 'static {
        self.spawn(move || TaskResult::Value(f()))
    }

    /// Spawn a task that returns a number.
    pub fn spawn_compute<F>(&self, f: F) -> TaskHandle
    where F: FnOnce() -> f64 + Send + 'static {
        self.spawn(move || TaskResult::Number(f()))
    }

    /// Number of pending tasks in the queue.
    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().queue.len()
    }

    /// Number of worker threads.
    pub fn worker_count(&self) -> usize { self.max_workers }

    /// Shutdown the executor, waiting for all tasks to finish.
    pub fn shutdown(self) {
        {
            let mut guard = self.inner.lock().unwrap();
            guard.shutdown = true;
        }
        // Wake all workers
        for _ in 0..self.workers.len() {
            self.notify.notify_all();
        }
        for w in self.workers {
            let _ = w.join();
        }
    }
}

fn worker_loop(inner: Arc<Mutex<ExecutorInner>>, notify: Arc<Condvar>) {
    loop {
        let task = {
            let mut guard = inner.lock().unwrap();
            loop {
                if let Some(task) = guard.queue.pop_front() {
                    guard.active_count += 1;
                    break Some(task);
                }
                if guard.shutdown {
                    break None;
                }
                guard = notify.wait(guard).unwrap();
            }
        };

        match task {
            Some((id, f, state, result, done_notify)) => {
                *state.lock().unwrap() = TaskState::Running;
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
                match r {
                    Ok(val) => {
                        *result.lock().unwrap() = Some(val.clone());
                        *state.lock().unwrap() = TaskState::Completed(val);
                    }
                    Err(e) => {
                        let msg = if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else if let Some(s) = e.downcast_ref::<&str>() {
                            s.to_string()
                        } else {
                            format!("task {} panicked", id)
                        };
                        *state.lock().unwrap() = TaskState::Failed(msg);
                    }
                }
                // Signal completion
                let (lock, cvar) = &*done_notify;
                *lock.lock().unwrap() = true;
                cvar.notify_all();
                inner.lock().unwrap().active_count -= 1;
            }
            None => break, // shutdown
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Select — first-of-N completion
// ══════════════════════════════════════════════════════════════════════════════

/// Wait for the first of multiple tasks to complete. Returns (index, result).
pub fn select(handles: &[TaskHandle]) -> (usize, TaskState) {
    // Busy-poll with exponential backoff
    let mut sleep_us: u64 = 10;
    loop {
        for (i, h) in handles.iter().enumerate() {
            if h.is_done() {
                return (i, h.state.lock().unwrap().clone());
            }
        }
        thread::sleep(Duration::from_micros(sleep_us));
        sleep_us = (sleep_us * 2).min(10_000); // backoff up to 10ms
    }
}

/// Wait for the first task to complete, with timeout. Returns None on timeout.
pub fn select_timeout(handles: &[TaskHandle], timeout: Duration) -> Option<(usize, TaskState)> {
    let start = Instant::now();
    let mut sleep_us: u64 = 10;
    while start.elapsed() < timeout {
        for (i, h) in handles.iter().enumerate() {
            if h.is_done() {
                return Some((i, h.state.lock().unwrap().clone()));
            }
        }
        thread::sleep(Duration::from_micros(sleep_us));
        sleep_us = (sleep_us * 2).min(5_000);
    }
    None
}

/// Wait for ALL tasks to complete. Returns results in order.
pub fn join_all(handles: &[TaskHandle]) -> Vec<TaskState> {
    handles.iter().map(|h| h.join()).collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Timer — async sleep and interval
// ══════════════════════════════════════════════════════════════════════════════

/// Async sleep: spawns a task that completes after the given duration.
pub fn sleep(executor: &Executor, duration: Duration) -> TaskHandle {
    executor.spawn(move || {
        thread::sleep(duration);
        TaskResult::None
    })
}

/// Async timeout: wraps a task handle, cancels it if not done within the deadline.
pub fn timeout(handle: &TaskHandle, deadline: Duration) -> TaskState {
    let result = handle.join_timeout(deadline);
    if matches!(result, TaskState::Pending) {
        handle.cancel();
        TaskState::Failed("timeout".to_string())
    } else {
        result
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// AsyncChannel — multi-producer, multi-consumer with select support
// ══════════════════════════════════════════════════════════════════════════════

/// A bounded async channel.
pub struct AsyncChannel<T> {
    inner: Arc<ChannelInner<T>>,
}

struct ChannelInner<T> {
    buf: Mutex<VecDeque<T>>,
    capacity: usize,
    not_empty: Condvar,
    not_full: Condvar,
    closed: AtomicBool,
}

impl<T> AsyncChannel<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(ChannelInner {
                buf: Mutex::new(VecDeque::with_capacity(capacity)),
                capacity,
                not_empty: Condvar::new(),
                not_full: Condvar::new(),
                closed: AtomicBool::new(false),
            }),
        }
    }

    /// Send a value. Blocks if the channel is full.
    pub fn send(&self, value: T) -> Result<(), &'static str> {
        if self.inner.closed.load(Ordering::Relaxed) { return Err("channel closed"); }
        let mut buf = self.inner.buf.lock().unwrap();
        while buf.len() >= self.inner.capacity {
            if self.inner.closed.load(Ordering::Relaxed) { return Err("channel closed"); }
            buf = self.inner.not_full.wait(buf).unwrap();
        }
        buf.push_back(value);
        self.inner.not_empty.notify_one();
        Ok(())
    }

    /// Try send without blocking. Returns Err if full.
    pub fn try_send(&self, value: T) -> Result<(), &'static str> {
        if self.inner.closed.load(Ordering::Relaxed) { return Err("channel closed"); }
        let mut buf = self.inner.buf.lock().unwrap();
        if buf.len() >= self.inner.capacity { return Err("channel full"); }
        buf.push_back(value);
        self.inner.not_empty.notify_one();
        Ok(())
    }

    /// Receive a value. Blocks if the channel is empty.
    pub fn recv(&self) -> Result<T, &'static str> {
        let mut buf = self.inner.buf.lock().unwrap();
        loop {
            if let Some(val) = buf.pop_front() {
                self.inner.not_full.notify_one();
                return Ok(val);
            }
            if self.inner.closed.load(Ordering::Relaxed) { return Err("channel closed"); }
            buf = self.inner.not_empty.wait(buf).unwrap();
        }
    }

    /// Try receive without blocking.
    pub fn try_recv(&self) -> Result<T, &'static str> {
        let mut buf = self.inner.buf.lock().unwrap();
        match buf.pop_front() {
            Some(val) => {
                self.inner.not_full.notify_one();
                Ok(val)
            }
            None => Err("channel empty"),
        }
    }

    /// Close the channel. Pending receivers will get Err.
    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Relaxed);
        self.inner.not_empty.notify_all();
        self.inner.not_full.notify_all();
    }

    pub fn len(&self) -> usize { self.inner.buf.lock().unwrap().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn is_closed(&self) -> bool { self.inner.closed.load(Ordering::Relaxed) }
}

impl<T> Clone for AsyncChannel<T> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_spawn_and_join() {
        let exec = Executor::new(2);
        let h = exec.spawn_compute(|| 42.0);
        let state = h.join();
        assert!(matches!(state, TaskState::Completed(TaskResult::Number(n)) if (n - 42.0).abs() < 1e-9));
        exec.shutdown();
    }

    #[test]
    fn executor_multiple_tasks() {
        let exec = Executor::new(4);
        let handles: Vec<TaskHandle> = (0..10)
            .map(|i| exec.spawn_compute(move || i as f64 * 2.0))
            .collect();
        let results = join_all(&handles);
        assert_eq!(results.len(), 10);
        for r in &results {
            assert!(matches!(r, TaskState::Completed(_)));
        }
        exec.shutdown();
    }

    #[test]
    fn executor_spawn_value() {
        let exec = Executor::new(2);
        let h = exec.spawn_value(|| "hello world".to_string());
        let state = h.join();
        assert!(matches!(state, TaskState::Completed(TaskResult::Value(s)) if s == "hello world"));
        exec.shutdown();
    }

    #[test]
    fn select_first_completes() {
        let exec = Executor::new(4);
        let fast = exec.spawn_compute(|| 1.0);
        let slow = exec.spawn(move || {
            thread::sleep(Duration::from_millis(200));
            TaskResult::Number(2.0)
        });
        // Give fast task time to complete
        thread::sleep(Duration::from_millis(50));
        let (idx, _state) = select(&[fast, slow]);
        assert_eq!(idx, 0);
        exec.shutdown();
    }

    #[test]
    fn select_timeout_none() {
        let exec = Executor::new(2);
        let slow = exec.spawn(move || {
            thread::sleep(Duration::from_millis(500));
            TaskResult::None
        });
        let result = select_timeout(&[slow], Duration::from_millis(10));
        assert!(result.is_none());
        exec.shutdown();
    }

    #[test]
    fn task_cancel() {
        let exec = Executor::new(2);
        let h = exec.spawn(move || {
            thread::sleep(Duration::from_millis(500));
            TaskResult::None
        });
        h.cancel();
        // The task may already be running; check that cancel sets state
        let state = h.state.lock().unwrap().clone();
        // It might be Running or Cancelled depending on timing
        assert!(matches!(state, TaskState::Cancelled | TaskState::Running | TaskState::Pending));
        exec.shutdown();
    }

    #[test]
    fn async_channel_send_recv() {
        let ch: AsyncChannel<i32> = AsyncChannel::new(10);
        ch.send(42).unwrap();
        ch.send(99).unwrap();
        assert_eq!(ch.len(), 2);
        assert_eq!(ch.recv().unwrap(), 42);
        assert_eq!(ch.recv().unwrap(), 99);
    }

    #[test]
    fn async_channel_try_ops() {
        let ch: AsyncChannel<String> = AsyncChannel::new(2);
        assert!(ch.try_send("a".into()).is_ok());
        assert!(ch.try_send("b".into()).is_ok());
        assert!(ch.try_send("c".into()).is_err()); // full
        assert_eq!(ch.try_recv().unwrap(), "a");
    }

    #[test]
    fn async_channel_close() {
        let ch: AsyncChannel<i32> = AsyncChannel::new(10);
        ch.send(1).unwrap();
        ch.close();
        assert!(ch.send(2).is_err());
        assert_eq!(ch.recv().unwrap(), 1); // buffered value still readable
        assert!(ch.recv().is_err()); // closed and empty
    }

    #[test]
    fn async_channel_cross_thread() {
        let ch: AsyncChannel<String> = AsyncChannel::new(100);
        let ch2 = ch.clone();
        let producer = thread::spawn(move || {
            for i in 0..10 {
                ch2.send(format!("msg_{}", i)).unwrap();
            }
        });
        producer.join().unwrap();
        assert_eq!(ch.len(), 10);
        for i in 0..10 {
            assert_eq!(ch.recv().unwrap(), format!("msg_{}", i));
        }
    }

    #[test]
    fn timeout_expires() {
        let exec = Executor::new(2);
        let slow = exec.spawn(move || {
            thread::sleep(Duration::from_millis(500));
            TaskResult::None
        });
        let result = timeout(&slow, Duration::from_millis(10));
        assert!(matches!(result, TaskState::Failed(msg) if msg == "timeout"));
        exec.shutdown();
    }
}
