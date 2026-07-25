# Multithreading Reference: Complete Guide
## Advanced Concurrent Programming Without Native Threads

---

# TABLE OF CONTENTS

1. [Actor Model Architecture](#1-actor-model-architecture)
2. [Scaling Patterns](#2-scaling-patterns)
3. [Thread-Safe Data Structures](#3-thread-safe-data-structures)
4. [Deadlock Prevention](#4-deadlock-prevention)
5. [Synchronization Primitives](#5-synchronization-primitives)
6. [Performance Optimization](#6-performance-optimization)
7. [Testing Concurrent Code](#7-testing-concurrent-code)
8. [Debugging Race Conditions](#8-debugging-race-conditions)
9. [Production Patterns](#9-production-patterns)
10. [Common Pitfalls](#10-common-pitfalls)

---

# 1. ACTOR MODEL ARCHITECTURE

## 1.1 What is an Actor?

An actor is a computational entity that:
- **Owns state** (no sharing, no races)
- **Processes messages** sequentially
- **Responds asynchronously** (doesn't block sender)
- **Supervises children** (restart on failure)
- **Communicates via messages** (no shared memory)

```
Traditional Threading:        Actor Model:
┌─────────────┐             ┌─────────────┐
│ Thread A    │             │ Actor A     │
│ ┌────────┐  │             │ ┌────────┐  │
│ │Shared  │◄─┼─LOCK───────┤ │Isolated│  │
│ │State   │  │             │ │State   │  │
│ └────────┘  │             │ └────────┘  │
└─────────────┘             └─────────────┘
     ↕ race                       ↕ msgs
 conditions                    ordered
```

## 1.2 Actor Lifecycle

```
CREATE -> RUNNING -> STOPPED
  │
  └─(ERROR)─> RESTARTING -> RUNNING
```

### State Transitions

```rust
pub enum ActorState {
    Created,      // Just instantiated
    Running,      // Processing messages
    Suspended,    // Paused (supervisor decision)
    Stopped,      // Graceful shutdown
    Failed,       // Error occurred
    Restarting,   // Attempting recovery
}
```

### Key Timings

| Event | Duration | Latency Impact |
|-------|----------|----------------|
| Message send | ~100ns | Minimal |
| Message process (simple) | ~1µs | Per-message |
| Context switch (OS thread) | ~1ms | Major bottleneck |
| Actor creation | ~10µs | One-time |
| Supervision check | ~100ns | Every crash |

## 1.3 Actor Communication Patterns

### Request/Response

```rust
// Sender: send request, wait for response
sender.send_request(RequestMsg { id: 1, data: "work" });
let resp = receiver.wait_response(1);  // Blocks until response

// Receiver: read, process, respond
match msg {
    RequestMsg { id, data } => {
        let result = process(data);
        respond(id, result);  // Send back to sender
    }
}
```

### Fire-and-Forget (Event)

```rust
// One-way notification, no response expected
actor.send(EventMsg { data: "update" });

// Receiver just processes, no reply
match msg {
    EventMsg { data } => {
        process(data);  // No respond() call
    }
}
```

### Pub-Sub (Broadcast)

```rust
// Publisher sends to all subscribers
publisher.broadcast(EventMsg { topic: "orders", data: "123" });

// Subscribers receive all events from topic
subscribers.receive_from_topic("orders");  // Subscribed
```

## 1.4 Supervision & Fault Tolerance

### Supervision Strategies

```rust
pub enum SupervisionStrategy {
    /// Restart crashed actor immediately
    Restart,
    
    /// Escalate to parent supervisor
    Escalate,
    
    /// Permanently stop actor
    Stop,
    
    /// Ignore failure, keep running
    Resume,
}
```

### Supervisor Tree

```
         Root
          │
       ┌──┼──┐
       │  │  │
      S1  S2 S3    (Supervisors)
      │   │  │
    ┌─┼─ │  └─┐
    │ │  │    │
   A1 A2 A3  A4   (Actors)

If A1 crashes:
  1. S1 detects failure
  2. Applies restart strategy
  3. Restarts A1
  4. If S1 repeatedly fails, escalate to Root
```

### Restart Exponential Backoff

```rust
pub struct RestartPolicy {
    max_restarts: usize,      // Max 5 restarts
    time_window: Duration,    // Within 60 seconds
    backoff_factor: u32,      // 2^attempt (1s, 2s, 4s, 8s, 16s)
}

// Backoff timing:
// Attempt 1: wait 1s before restart
// Attempt 2: wait 2s before restart
// Attempt 3: wait 4s before restart
// Attempt 4: wait 8s before restart
// Attempt 5: wait 16s before restart
// If 5 attempted and 1 succeeds: reset counter
```

---

# 2. SCALING PATTERNS

## 2.1 Actor Pools

### Fixed Size Pool

```rust
pub struct ActorPool {
    actors: Vec<Arc<Mutex<WorkerActor>>>,
    next: AtomicUsize,
}

impl ActorPool {
    pub fn distribute(&self, work: Work) {
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % self.actors.len();
        let actor = &self.actors[idx];
        // ... send work to actor
    }
}
```

**Trade-offs**
- ✅ No allocation overhead
- ✅ Predictable latency
- ❌ Needs tuning (how many actors?)
- ❌ Can't adapt to load changes

### Dynamic Pool

```rust
pub struct DynamicPool {
    actors: Vec<Arc<Mutex<WorkerActor>>>,
    queue_depth: Arc<AtomicUsize>,
    config: PoolConfig,
}

impl DynamicPool {
    pub fn adjust_size(&mut self) {
        let depth = self.queue_depth.load(Ordering::Relaxed);
        
        // Add actors if queue growing
        if depth > self.config.high_watermark {
            self.add_actor();
        }
        
        // Remove actors if queue shrinking
        if depth < self.config.low_watermark {
            self.remove_actor();
        }
    }
}
```

**Trade-offs**
- ✅ Adapts to load
- ✅ Efficient (only use needed actors)
- ❌ Allocation/deallocation overhead
- ❌ Latency spikes during scaling

### Work Stealing Pool

```rust
pub struct WorkStealingPool {
    queues: Vec<Mutex<VecDeque<Work>>>,  // Per-actor queue
}

impl WorkStealingPool {
    pub fn run_worker(&self, idx: usize) {
        loop {
            // Try own queue first
            if let Some(work) = self.queues[idx].lock().unwrap().pop_front() {
                process(work);
                continue;
            }
            
            // Own queue empty, try stealing
            let next_idx = (idx + 1) % self.queues.len();
            if let Some(work) = self.queues[next_idx].lock().unwrap().pop_back() {
                process(work);
                continue;
            }
            
            // All queues empty, idle wait
            thread::sleep(Duration::from_micros(1));
        }
    }
}
```

**Trade-offs**
- ✅ Better load balancing
- ✅ Reduces idle time
- ❌ Lock contention on neighbour queues
- ❌ Unpredictable latency (stealing overhead)

## 2.2 Load Distribution Strategies

### Round-Robin

```rust
pub struct RoundRobinDistributor {
    counter: AtomicUsize,
    size: usize,
}

impl RoundRobinDistributor {
    pub fn next(&self) -> usize {
        let c = self.counter.fetch_add(1, Ordering::Relaxed);
        c % self.size
    }
}
```

**Best for**: Uniform load, no key affinity

### Least-Busy (Least Queue Depth)

```rust
pub struct LeastBusyDistributor {
    queue_depths: Vec<AtomicUsize>,
}

impl LeastBusyDistributor {
    pub fn next(&self) -> usize {
        let depths: Vec<usize> = self.queue_depths.iter()
            .map(|d| d.load(Ordering::Relaxed))
            .collect();
        depths.iter().position(|&d| d == *depths.iter().min().unwrap()).unwrap()
    }
}
```

**Best for**: Bursty load, varying processing time

### Consistent Hash (by Key)

```rust
pub struct ConsistentHashDistributor {
    size: usize,
}

impl ConsistentHashDistributor {
    pub fn next(&self, key: &str) -> usize {
        let hash = murmur3(key);
        (hash as usize) % self.size
    }
}
```

**Best for**: Ordering per-key guarantees, cache friendly

### Ranged Hash (Slots)

```rust
pub struct RangedDistributor {
    ranges: Vec<(u32, usize)>,  // (max_hash, actor_idx)
}

impl RangedDistributor {
    pub fn next(&self, key: &str) -> usize {
        let hash = murmur3(key);
        for (max, idx) in &self.ranges {
            if hash < *max {
                return *idx;
            }
        }
        self.ranges.last().unwrap().1
    }
}
```

**Best for**: Rebalancing without rehashing all keys

### Sticky (Session Aware)

```rust
pub struct StickyDistributor {
    session_map: Arc<RwLock<HashMap<String, usize>>>,
    rounds: AtomicUsize,
}

impl StickyDistributor {
    pub fn next(&self, session_id: &str) -> usize {
        let mut map = self.session_map.write().unwrap();
        if let Some(&idx) = map.get(session_id) {
            return idx;
        }
        
        // Assign to next in round-robin
        let idx = self.rounds.fetch_add(1, Ordering::Relaxed) % self.size;
        map.insert(session_id.to_string(), idx);
        idx
    }
}
```

**Best for**: Session affinity, stateful processing

## 2.3 Backpressure & Flow Control

### Queue-Based (Implicit)

```rust
pub struct BackpressureQueue {
    items: VecDeque<Work>,
    capacity: usize,
}

impl BackpressureQueue {
    pub fn enqueue(&mut self, work: Work) -> Result<(), Work> {
        if self.items.len() < self.capacity {
            self.items.push_back(work);
            Ok(())
        } else {
            Err(work)  // Sender gets back work, can retry/drop/buffer
        }
    }
}
```

### Explicit Backpressure

```rust
pub struct ExplicitBackpressure {
    permits: AtomicUsize,
    limit: usize,
}

impl ExplicitBackpressure {
    pub fn try_submit(&self, work: Work) -> bool {
        let p = self.permits.load(Ordering::Relaxed);
        if p > 0 && self.permits.compare_exchange(
            p, p - 1,
            Ordering::Relaxed,
            Ordering::Relaxed
        ).is_ok() {
            submit(work);
            return true;
        }
        false
    }
    
    pub fn release(&self) {
        let mut p = self.permits.load(Ordering::Relaxed);
        while p < self.limit {
            if self.permits.compare_exchange(
                p, p + 1,
                Ordering::Relaxed,
                Ordering::Relaxed
            ).is_ok() {
                break;
            }
            p = self.permits.load(Ordering::Relaxed);
        }
    }
}
```

### Rate Limiting

```rust
pub struct RateLimiter {
    tokens: AtomicUsize,
    refill_rate: usize,  // tokens/sec
    capacity: usize,
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    pub fn allow(&self) -> bool {
        let mut lr = self.last_refill.lock().unwrap();
        let elapsed = lr.elapsed();
        let new_tokens = (elapsed.as_millis() as usize * self.refill_rate) / 1000;
        
        if new_tokens > 0 {
            let t = self.tokens.load(Ordering::Relaxed);
            let new_t = (t + new_tokens).min(self.capacity);
            self.tokens.store(new_t, Ordering::Relaxed);
            *lr = Instant::now();
        }
        
        let t = self.tokens.load(Ordering::Relaxed);
        if t > 0 {
            self.tokens.store(t - 1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}
```

---

# 3. THREAD-SAFE DATA STRUCTURES

## 3.1 Ownership and Sharing: Arc<T>

### What is Arc?

Arc = Atomic Reference Count
- Multiple owners of same data
- Last owner deallocates
- Cheap clones (just increment counter)

```
┌────────────────┐
│  Arc<i32>      │  refcount = 1
│  data: 42      │
└────────────────┘

Clone #1:         Clone #2:
┌─────────┐      ┌─────────┐
│Arc<i32> │      │Arc<i32> │
│refcount:2      │refcount:2
└─ ─ ─ ─ ─┘      └─ ─ ─ ─ ─┘
   ↓                ↓
   └─────────────────┘
    Shared data: 42

When both dropped:
  refcount = 0 -> deallocate
```

### Common Pattern: Arc<Mutex<T>>

```rust
// Create shared mutable data
let counter = Arc::new(Mutex::new(0i32));

// Clone for each thread
for i in 0..10 {
    let c = Arc::clone(&counter);
    thread::spawn(move || {
        let mut v = c.lock().unwrap();
        *v += i;
    });
}

// Main can read
let final_val = *counter.lock().unwrap();
```

### Weak References (Prevent Cycles)

```rust
use std::sync::Weak;

pub struct Node {
    value: i32,
    parent: Option<Weak<Mutex<Node>>>,
    children: Vec<Arc<Mutex<Node>>>,
}

// Weak doesn't prevent parent from being deallocated
// Call upgrade() to get Arc (may fail if deallocated)
if let Some(parent) = node.parent.upgrade() {
    let p = parent.lock().unwrap();
    println!("Parent: {}", p.value);
}
```

## 3.2 Mutual Exclusion: Mutex<T>

### Concept

Mutex = "Mutual Exclusion"
- Only one thread can hold lock at a time
- Lock held while accessing data
- Unlock on drop (RAII)

```rust
pub struct Mutex<T> {
    data: T,
    lock: AtomicUsize,  // 0=unlocked, 1=locked
}

impl<T> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<T> {
        // Spin or block until lock available
        while self.lock.compare_exchange(0, 1, ...).is_err() {
            // Wait
        }
        MutexGuard { mutex: self }
    }
}

impl<T> Drop for MutexGuard<T> {
    fn drop(&mut self) {
        self.mutex.lock.store(0, ..);  // Unlock
    }
}
```

### Usage Patterns

```rust
// Simple lock
let data = Arc::new(Mutex::new(vec![1, 2, 3]));
let mut v = data.lock().unwrap();
v.push(4);
drop(v);  // Auto-unlock

// Minimize lock hold time
let data = Arc::new(Mutex::new(large_struct));
{
    let mut d = data.lock().unwrap();
    d.update();  // Held for update only
}  // Lock dropped here
do_other_work();  // No lock held

// Try lock (non-blocking)
match data.try_lock() {
    Ok(mut v) => v.push(5),
    Err(_) => println!("Couldn't acquire lock"),
}
```

## 3.3 Read-Write Lock: RwLock<T>

### Concept

RwLock = Read-Write Lock
- Multiple readers (shared)
- Exclusive writer (single)
- Better for read-heavy workloads

```
Mutex<T>:
  ├─ Basic mutual exclusion
  ├─ One thread at a time
  ├─ No distinction read vs write
  └─ ~100ns overhead

RwLock<T>:
  ├─ Multiple readers concurrent
  ├─ One writer (exclusive)
  ├─ Read-heavy workloads win
  └─ ~50ns per read, ~150ns per write
```

### Usage Example

```rust
pub struct Cache {
    data: RwLock<HashMap<String, String>>,
}

impl Cache {
    pub fn get(&self, key: &str) -> Option<String> {
        // Read lock (shared)
        self.data.read().unwrap().get(key).cloned()
    }
    
    pub fn set(&self, key: String, value: String) {
        // Write lock (exclusive)
        self.data.write().unwrap().insert(key, value);
    }
}

// Multiple threads can read concurrently
// But writes block readers (and vice versa)
```

### Writer Starvation

```rust
// If many readers, writers starve
// Solution: use read-biased or upgrade locks

pub struct UpgradeableLock {
    data: RwLock<Vec<i32>>,
}

impl UpgradeableLock {
    pub fn find_and_update(&self, target: i32, new_val: i32) -> bool {
        // Start with read lock
        let r = self.data.read().unwrap();
        if r.contains(&target) {
            drop(r);  // Release read lock
            
            // Upgrade to write lock
            let mut w = self.data.write().unwrap();
            if let Some(pos) = w.iter().position(|&v| v == target) {
                w[pos] = new_val;
                return true;
            }
        }
        false
    }
}
```

## 3.4 Lock-Free Primitives: Atomic<T>

### AtomicUsize / AtomicBool

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AtomicCounter {
    value: AtomicUsize,
}

impl AtomicCounter {
    pub fn increment(&self) {
        // No lock! Uses CPU atomic instruction
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> usize {
        self.value.load(Ordering::Relaxed)
    }
}
```

### Memory Ordering

```rust
pub enum Ordering {
    /// Relaxed: No synchronization
    /// Fast: ~5ns, no memory barrier
    /// Use: Counters where order doesn't matter
    Relaxed,
    
    /// Release: Synchronize on write
    /// Slower: ~50ns, write barrier
    /// Use: Signaling (e.g., shutdown flag)
    Release,
    
    /// Acquire: Synchronize on read
    /// ~50ns, read barrier
    /// Use: Reading synchronized data
    Acquire,
    
    /// SequentiallyConsistent: Strict ordering
    /// ~150ns, full barrier
    /// Use: When unsure (safe but slow)
    SeqCst,
}
```

### Pattern: Shutdown Flag

```rust
pub struct StopFlag {
    should_stop: Arc<AtomicBool>,
}

impl StopFlag {
    pub fn signal_shutdown(&self) {
        // Write-release: other threads see this
        self.should_stop.store(true, Ordering::Release);
    }
    
    pub fn should_continue(&self) -> bool {
        // Read-acquire: see shutdown signal
        !self.should_stop.load(Ordering::Acquire)
    }
}
```

---

# 4. DEADLOCK PREVENTION

## 4.1 Deadlock Conditions

Deadlock requires **ALL FOUR**:

1. **Mutual Exclusion**: Resource can't be shared
2. **Hold and Wait**: Thread holds resource while waiting for another
3. **No Preemption**: Can't force-take resource
4. **Circular Wait**: Circular dependency of threads on locks

To prevent: **Break any one condition**

## 4.2 Lock Ordering (Prevent Circular Wait)

### Strategy

Assign levels to locks. Always acquire in ascending order.

```
Levels:
  Lock1 < Lock2 < Lock3

Thread A: must acquire Lock1 before Lock2 before Lock3
Thread B: must acquire Lock1 before Lock2 before Lock3
Thread C: must acquire Lock1 before Lock2 before Lock3

Proof no deadlock:
  If Thread A holds Lock2 and waits for Lock3:
    Lock3 > Lock2, so can't happen
  If Thread A holds Lock3 and waits for Lock1:
    Lock1 < Lock3, violates ordering -> ERROR
  Therefore: no circular wait possible
```

### Implementation

```rust
pub struct LeveledLock {
    level: usize,
    lock: Mutex<u64>,
}

pub struct OrderingContext {
    last_level: Option<usize>,
}

impl OrderingContext {
    pub fn on_acquire(&mut self, new_level: usize) {
        match self.last_level {
            None => {},
            Some(last) => assert!(new_level > last, "Lock ordering violation!"),
        }
        self.last_level = Some(new_level);
    }
}

// Usage:
let mut ctx = OrderingContext { last_level: None };

let l1 = LeveledLock { level: 1, lock: Mutex::new(0) };
let l2 = LeveledLock { level: 2, lock: Mutex::new(0) };

ctx.on_acquire(1);
let g1 = l1.lock.lock();  // OK: 1 > None

ctx.on_acquire(2);
let g2 = l2.lock.lock();  // OK: 2 > 1

// (Can't do ctx.on_acquire(1) now - would panic!)
```

### Bank Transfer Example

```rust
pub struct Account {
    id: u32,
    balance: Mutex<u64>,
}

impl Account {
    pub fn transfer(&self, to: &Account, amount: u64) -> bool {
        // Lock ordering: always lower ID first
        let (locker, other) = if self.id < to.id {
            (self, to)
        } else {
            (to, self)
        };
        
        // Lock in order
        let mut b1 = locker.balance.lock().unwrap();
        let mut b2 = other.balance.lock().unwrap();
        
        if *b1 >= amount {
            *b1 -= amount;
            *b2 += amount;
            true
        } else {
            false
        }
    }
}

// Transfers always succeed without deadlock!
// No matter how many threads, no matter order of calls
```

## 4.3 Timeout (Detect & Recover)

### Try Lock with Timeout

```rust
pub fn transfer_with_timeout(a: &Account, b: &Account, amount: u64) -> Result<(), String> {
    let timeout = Duration::from_millis(100);
    
    // Try to acquire first lock
    let g1 = a.balance.try_lock()
        .map_err(|_| "Timeout on first lock".to_string())?;
    
    // Try second lock with timeout
    let start = Instant::now();
    loop {
        if let Ok(g2) = b.balance.try_lock() {
            // Both acquired, do transfer
            return Ok(());
        }
        
        if start.elapsed() > timeout {
            drop(g1);  // Release first lock
            return Err("Timeout acquiring second lock".to_string());
        }
        
        thread::yield_now();  // Spin-wait (bad) - real code should sleep
    }
}
```

### Exponential Backoff

```rust
pub fn transfer_with_backoff(a: &Account, b: &Account, amount: u64) -> Result<(), String> {
    let mut attempt = 0;
    const MAX_ATTEMPTS: u32 = 5;
    
    loop {
        match try_transfer(a, b, amount) {
            Ok(()) => return Ok(()),
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_ATTEMPTS {
                    return Err(format!("Failed after {} attempts: {}", MAX_ATTEMPTS, e));
                }
                
                let backoff_ms = 2_u64.pow(attempt - 1) * 10;  // 10ms, 20ms, 40ms, 80ms, 160ms
                thread::sleep(Duration::from_millis(backoff_ms));
            }
        }
    }
}
```

## 4.4 Deadlock Detection

### Wait-For Graph

```rust
pub struct DeadlockDetector {
    waits_for: RwLock<HashMap<ThreadId, HashSet<ThreadId>>>,
}

impl DeadlockDetector {
    pub fn record_wait(&self, thread: ThreadId, waiting_for: ThreadId) {
        let mut wf = self.waits_for.write().unwrap();
        wf.entry(thread)
            .or_insert_with(HashSet::new)
            .insert(waiting_for);
    }
    
    pub fn detect_cycle(&self) -> Option<Vec<ThreadId>> {
        let wf = self.waits_for.read().unwrap();
        
        for start in wf.keys() {
            let mut path = vec![*start];
            let mut visited = HashSet::new();
            
            if self.dfs_finds_cycle(start, &mut path, &mut visited, &wf) {
                return Some(path);  // Cycle found!
            }
        }
        
        None
    }
    
    fn dfs_finds_cycle(
        &self,
        node: &ThreadId,
        path: &mut Vec<ThreadId>,
        visited: &mut HashSet<ThreadId>,
        graph: &HashMap<ThreadId, HashSet<ThreadId>>,
    ) -> bool {
        visited.insert(*node);
        
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    path.push(*neighbor);
                    if self.dfs_finds_cycle(neighbor, path, visited, graph) {
                        return true;
                    }
                    path.pop();
                } else if path.contains(neighbor) {
                    return true;  // Back edge = cycle
                }
            }
        }
        
        false
    }
}
```

---

# 5. SYNCHRONIZATION PRIMITIVES

## 5.1 Barrier

```rust
use std::sync::Barrier;

pub fn main() {
    let barrier = Arc::new(Barrier::new(3));  // 3 threads
    let mut handles = vec![];
    
    for i in 0..3 {
        let b = Arc::clone(&barrier);
        let h = thread::spawn(move || {
            println!("Thread {} working...", i);
            
            // Do phase 1 work
            thread::sleep(Duration::from_millis((i * 100) as u64));
            
            println!("Thread {} at barrier", i);
            b.wait();  // WAIT FOR ALL
            
            println!("Thread {} continuing phase 2", i);
        });
        handles.push(h);
    }
    
    for h in handles {
        h.join().unwrap();
    }
}

// Output:
// Thread 0/1/2 working...
// Thread 0/1/2 at barrier
// [All three wait here until all arrive]
// Thread 0/1/2 continuing phase 2
```

## 5.2 Condition Variable

```rust
use std::sync::{Arc, Mutex, Condvar};

pub fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    
    // Waiter thread
    let pair2 = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        let mut done = lock.lock().unwrap();
        
        println!("Waiter: waiting for signal...");
        while !*done {
            done = cvar.wait(done).unwrap();  // WAIT & RELEASE LOCK
        }
        println!("Waiter: received signal!");
    });
    
    // Signaler thread
    thread::sleep(Duration::from_millis(100));
    let (lock, cvar) = &*pair;
    {
        let mut done = lock.lock().unwrap();
        *done = true;
        println!("Signaler: setting flag");
    }  // Lock released
    cvar.notify_one();  // WAKE WAITER
    
    waiter.join().unwrap();
}
```

## 5.3 Once Flag

```rust
use std::sync::Once;

static INIT: Once = Once::new();
static mut VALUE: i32 = 0;

pub fn main() {
    // First call initializes
    INIT.call_once(|| {
        unsafe {
            VALUE = 42;
        }
        println!("Initialized once");
    });
    
    // Subsequent calls do nothing
    INIT.call_once(|| {
        unsafe {
            VALUE = 999;  // NOT executed
        }
        println!("Init again?");  // NOT printed
    });
    
    unsafe {
        println!("Value: {}", VALUE);  // 42
    }
}
```

---

# 6. PERFORMANCE OPTIMIZATION

## 6.1 Lock Contention

### Measuring Contention

```rust
pub struct ContentionMetrics {
    successes: AtomicUsize,
    wait_count: AtomicUsize,
    total_wait_time: Mutex<Duration>,
}

impl ContentionMetrics {
    pub fn contention_ratio(&self) -> f64 {
        let succ = self.successes.load(Ordering::Relaxed) as f64;
        let waits = self.wait_count.load(Ordering::Relaxed) as f64;
        
        waits / (succ + waits)  // 0.0 = no contention, 1.0 = always waiting
    }
}
```

### Reducing Contention

```
High Contention Scenario:
  1 Mutex shared by 100 threads
  → All threads fight for lock
  → P99 latency = hundreds of µs

Solution 1: Lock Striping
  100 Mutexes (one per partition)
  → Thread picks own stripe
  → P99 latency = few µs

Solution 2: Lock-Free Data Structures
  AtomicUsize instead of Mutex<usize>
  → No waits at all
  → P99 latency =  < 100ns
```

### Lock Striping Example

```rust
pub struct StripedMap {
    stripes: Vec<Mutex<HashMap<u32, String>>>,
    stripe_count: usize,
}

impl StripedMap {
    pub fn stripe_for_key(&self, key: u32) -> usize {
        (key as usize) % self.stripe_count
    }
    
    pub fn get(&self, key: u32) -> Option<String> {
        let stripe = self.stripe_for_key(key);
        self.stripes[stripe].lock().unwrap().get(&key).cloned()
    }
    
    pub fn set(&self, key: u32, value: String) {
        let stripe = self.stripe_for_key(key);
        self.stripes[stripe].lock().unwrap().insert(key, value);
    }
}

// 100 threads, each on different key:
// Thread 1 (key 1) → stripe 1 → low contention!
// Thread 2 (key 2) → stripe 2 → low contention!
```

## 6.2 False Sharing

### What is False Sharing?

```
CPU Cache Line = 64 bytes typically

If two threads modify:
  let a: u64 = 1;  (bytes 0-7)
  let b: u64 = 2;  (bytes 8-15)

Both in SAME cache line:
  Thread 1 modifies a
  → Invalidates   entire line in Thread 2's cache
  → Thread 2 must reload whole line
  → Performance crater!

Solution: Align to cache line:
  struct Aligned<T> {
      #[repr(align(64))]
      value: T,
  }
```

### Cache-Aligned Atomic

```rust
#[repr(align(64))]
pub struct CacheAligned<T> {
    value: T,
}

// Usage:
pub struct FastCounters {
    counters: [CacheAligned<AtomicUsize>; 8],
}

// Each counter on own cache line
// No false sharing between threads  
// ~8x faster than shared Mutex
```

## 6.3 Scalability Limits

### Amdahl's Law

```
Speedup = 1 / ((1-P) + P/N)

où:
  N = number of processors
  P = fraction of parallelizable code
  (1-P) = serial fraction

Examples:
  P=0.9, N=4:  Speedup = 1 / (0.1 + 0.9/4)     = 2.81x (not 4x!)
  P=0.99, N=4: Speedup = 1 / (0.01 + 0.99/4)   = 3.85x
  P=0.99, N=100: Speedup = 1 / (0.01 + 0.99/100) = 50x (not 100x!)
```

### Implications for Actor Pools

```
If actor processing = 99% parallelizable
  Scheduling, coordination, locking = 1% serial

Max practical speedup with unlimited actors:
  1 / 0.01 = 100x

Beyond 100 actors:
  Returns diminish significantly
  Must reduce serial portions (locks, scheduling)
```

---

# 7. TESTING CONCURRENT CODE

## 7.1 Stress Testing

```rust
#[test]
fn stress_concurrent_writes() {
    let counter = Arc::new(Mutex::new(0usize));
    let mut handles = vec![];
    
    // 100 threads each writing 1000x
    for _ in 0..100 {
        let c = Arc::clone(&counter);
        let h = thread::spawn(move || {
            for _ in 0..1000 {
                let mut v = c.lock().unwrap();
                *v += 1;
            }
        });
        handles.push(h);
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    // Should be exactly 100,000
    assert_eq!(*counter.lock().unwrap(), 100_000);
}
```

## 7.2 Timing Variability

```rust
#[test]
fn measure_latency_percentiles() {
    let data = Arc::new(Mutex::new(vec![]));
    let mut handles = vec![];
    
    for _ in 0..10 {
        let d = Arc::clone(&data);
        let h = thread::spawn(move || {
            for _ in 0..1000 {
                let start = Instant::now();
                let mut v = d.lock().unwrap();
                *v += 1;
                let lat = start.elapsed().as_micros();
                d.lock().unwrap();  // Hack: store latency
            }
        });
        handles.push(h);
    }
    
    // Collects latencies, compute p50/p95/p99
}
```

---

# 8. DEBUGGING RACE CONDITIONS

## 8.1 Tools

### ThreadSanitizer (tsan)

```bash
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test

# Detects:
# - Data races (unsynchronized access)
# - Memory leaks in concurrent code
# - Some deadlocks
```

### Miri (Undefined Behavior Detector)

```bash
cargo +nightly miri test

# Detects:
# - Unsynchronized access in unsafe code
# - Out-of-bounds access
# - Use-after-free
```

## 8.2 Logging Patterns

```rust
pub struct ConcurrentDebugger {
    events: Arc<Mutex<Vec<(ThreadId, String, Instant)>>>,
}

impl ConcurrentDebugger {
    pub fn log_event(&self, msg: &str) {
        let mut events = self.events.lock().unwrap();
        events.push((
            thread::current().id(),
            msg.to_string(),
            Instant::now(),
        ));
    }
    
    pub fn dump_timeline(&self) {
        let events = self.events.lock().unwrap();
        for (tid, msg, time) in events.iter() {
            println!("[{:?}@{:?}] {}", tid, time, msg);
        }
    }
}
```

---

# 9. PRODUCTION PATTERNS

## 9.1 Graceful Shutdown

```rust
pub struct GracefulShutdown {
    shutdown_signal: Arc<AtomicBool>,
    pending_work: Arc<Mutex<usize>>,
}

impl GracefulShutdown {
    pub fn initiate_shutdown(&self) {
        self.shutdown_signal.store(true, Ordering::Release);
    }
    
    pub fn wait_finishing(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            let pending = *self.pending_work.lock().unwrap();
            if pending == 0 {
                return true;
            }
            
            if start.elapsed() > timeout {
                return false;  // Timeout!
            }
            
            thread::sleep(Duration::from_millis(10));
        }
    }
}
```

## 9.2 Health Monitoring

```rust
pub struct PoolHealth {
    last_work_time: Arc<Mutex<Instant>>,
    timeout: Duration,
}

impl PoolHealth {
    pub fn check_health(&self) -> PoolStatus {
        let last = *self.last_work_time.lock().unwrap();
        let elapsed = last.elapsed();
        
        if elapsed > self.timeout {
            PoolStatus::Unhealthy
        } else if elapsed > self.timeout / 2 {
            PoolStatus::Degraded
        } else {
            PoolStatus::Healthy
        }
    }
}
```

## 9.3 Metrics Collection

```rust
pub struct PoolMetrics {
    items_completed: AtomicUsize,
    items_failed: AtomicUsize,
    total_latency_us: AtomicUsize,
}

impl PoolMetrics {
    pub fn record_completion(&self, latency_us: usize) {
        self.items_completed.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
    }
    
    pub fn avg_latency_us(&self) -> usize {
        let completed = self.items_completed.load(Ordering::Relaxed);
        if completed == 0 {
            return 0;
        }
        
        let total = self.total_latency_us.load(Ordering::Relaxed);
        total / completed
    }
    
    pub fn success_rate(&self) -> f64 {
        let completed = self.items_completed.load(Ordering::Relaxed) as f64;
        let failed = self.items_failed.load(Ordering::Relaxed) as f64;
        
        completed / (completed + failed)
    }
}
```

---

# 10. COMMON PITFALLS

## 10.1 Holding Locks Too Long

```rust
// ❌ BAD
let mut data = shared_data.lock().unwrap();
let result = expensive_computation();  // Locks held!
do_io_operation();  // Locks held!
data.update(result);
// Lock released here

// ✅ GOOD
let result = expensive_computation();  // No lock
let io_result = do_io_operation();  // No lock
{
    let mut data = shared_data.lock().unwrap();
    data.update(result);  // Lock held only for actual update
}
```

## 10.2 Wrong Ordering

```rust
// ❌ BAD - Deadlock possible
Thread A:  lock(L1) -> lock(L2)
Thread B:  lock(L2) -> lock(L1)

// ✅ GOOD - Enforce ordering
Thread A:  lock(L1) -> lock(L2)  (1 < 2)
Thread B:  lock(L1) -> lock(L2)  (always same order)
```

## 10.3 Ignoring Errors

```rust
// ❌ BAD - Silently fails
let v = data.try_lock().ok();  // Returns None if locked - lost!

// ✅ GOOD - Handle error
match data.try_lock() {
    Ok(v) => process(v),
    Err(e) => {
        eprintln!("Failed to acquire lock: {}", e);
        retry_or_fail();
    }
}
```

## 10.4 Unbounded Queues

```rust
// ❌ BAD - Can OOM
let queue: Arc<Mutex<Vec<Work>>>;
queue.lock().unwrap().push(work);  // No limit!

// ✅ GOOD - Bounded queue
pub struct BoundedQueue {
    work: VecDeque<Work>,
    capacity: usize,
}

pub fn enqueue(&mut self, work: Work) -> Result<(), Work> {
    if self.work.len() < self.capacity {
        self.work.push_back(work);
        Ok(())
    } else {
        Err(work)  // Apply backpressure
    }
}
```

## 10.5 Panic in Lock Guard

```rust
// ❌ BAD - Panic while holding lock -> DEADLOCK
{
    let mut data = shared_data.lock().unwrap();
    if bad_condition {
        panic!("Error!");  // Lock never released!
    }
}  // Would reach here if no panic

// ✅ GOOD - Use catch_unwind or prevent panic
{
    let mut data = shared_data.lock().unwrap();
    if bad_condition {
        drop(data);  // Explicitly release
        return Err("Error");  // Return instead of panic
    }
}
```

---

# QUICK REFERENCE CHEAT SHEET

| Need | Use | Trade-off |
|------|-----|-----------|
| Simple counts | `AtomicUsize` | Fast (~5ns), simple |
| Mutable data | `Arc<Mutex<T>>` | Safe (~100ns), blocking |
| Read-heavy | `Arc<RwLock<T>>` | Efficient (?50ns read), writer starve |
| Phased execution | `Barrier` | Sync point, all wait |
| Waiting for event | `Condvar` | Efficient wake, needs predicate |
| One-time init | `Once` | Zero cost after first |
| Prevent deadlock | `Lock ordering` | Simple, needs discipline |
| Thread spawning | `Arc<Mutex<>>` | Classic, ~100ns |
| Actor spawning | Actor framework | Lightweight, domain-specific |

---

# SUMMARY

**Week 19 Learning Objectives**
- ✅ Actor model as thread replacement
- ✅ Design scalable pools (100s-1000s of actors)
- ✅ Use Arc, Mutex, RwLock, Atomic safely
- ✅ Prevent deadlocks with lock ordering
- ✅ Measure and optimize latency
- ✅ Build production worker pool system

**Key Insight**: Actors avoid the complexity of shared state and locks by eliminating sharing entirely. Each actor owns its state, communicates via messages, and can be supervised for fault tolerance. This makes concurrent systems dramatically simpler to reason about, test, and deploy.

