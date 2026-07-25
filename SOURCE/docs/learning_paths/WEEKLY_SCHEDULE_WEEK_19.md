# Week 19 Weekly Schedule: Multi-Threaded Applications
## 75 Hours | Concurrent Programming Without Native Threads

---

# WEEKLY OVERVIEW

**Monday**: Actor-Based Concurrency Foundations (15h)
**Tuesday**: Scaling Actors to High Concurrency (15h)
**Wednesday**: Thread-Safe Data Structures & Patterns (15h)
**Thursday**: Deadlock Prevention & Synchronization (15h)
**Friday**: Capstone - Multi-Node Worker Pool System (15h)

**Time Allocation**
- Concepts & Theory: 25 hours (33%)
- Hands-on Exercises: 35 hours (47%)
- Capstone Project: 15 hours (20%)

---

# MONDAY: ACTOR-BASED CONCURRENCY FOUNDATIONS (15 hours)

## 09:00-11:00 | Actors as Threads Without the Locks (2h)

**Concepts**
```
Traditional Threading Problem:        Actor Model Solution:
├─ spawn(fn) creates threads         ├─ spawn_actor(handler) creates lightweight entity
├─ Shared memory = race conditions   ├─ Isolated state = no races
├─ Mutexes = deadlock risk           ├─ Message queue = natural ordering
├─ Data races hard to debug          ├─ Compile-time safety guarantee
└─ Scale = context switch overhead   └─ Scale = pure concurrency (no OS cost)

Why actors work for "threading":
  ✓ Each actor has own mutable state (no sharing)
  ✓ Messages guarantee ordering within actor
  ✓ No locks needed = no deadlocks
  ✓ Supervision model handles crashes
  ✓ Can simulate threads at high scale
```

**Problems**
```
19.1.1-19.1.10: Actor Concurrency Basics

19.1.1:  Create a simple work distribution actor
         - Accept work requests
         - Process each synchronously
         - Track completion count

19.1.2:  Build echo actor with message logging
         - Log all received messages
         - Return response to sender
         - Maintain receive count

19.1.3:  Implement task counter actor
         - Increment on complete message
         - Decrement on fail message
         - Return current count

19.1.4:  Create accumulator actor
         - Sum all numeric values received
         - Support reset command
         - Return running total

19.1.5:  Build request/response pattern
         - Actor waits for request
         - Processes and sends response
         - Handle multiple outstanding requests

19.1.6:  Implement state machine actor
         - States: Idle, Processing, Complete
         - Transitions on specific messages
         - Validate state transitions

19.1.7:  Create rate-limited actor
         - Accept max 10 messages/second
         - Queue excess messages
         - Process in order

19.1.8:  Build result collector actor
         - Collect responses from N worker actors
         - Wait for all responses (timeout)
         - Aggregate results

19.1.9:  Implement retry-dispatch actor
         - Send message to primary worker
         - Retry on failure to backup
         - Track success/failure count

19.1.10: Create circuit breaker actor
         - Track failures (threshold: 5)
         - Open circuit when exceeded
         - Half-open for recovery testing
```

**Hands-on** (Exercise Group 1: Actor Foundation Patterns)
```rust
// From multithreading_exercises.rs, Exercise 1-5

// 1. Work Distribution Actor
pub struct WorkDistributor {
    pending: Vec<String>,
    completed: usize,
}

impl WorkDistributor {
    pub fn new() -> Self { /* ... */ }
    pub fn handle_work(&mut self, work: String) -> usize {
        self.pending.push(work);
        self.process_one()  // process immediately
    }
    pub fn process_one(&mut self) -> usize {
        if let Some(work) = self.pending.pop() {
            self.completed += 1;
        }
        self.completed
    }
}

// 2. Echo Coordinator (logs all messages)
pub struct EchoCoordinator {
    log: Vec<String>,
    response_count: usize,
}

impl EchoCoordinator {
    pub fn echo(&mut self, msg: String) -> String {
        self.log.push(msg.clone());
        format!("ECHO: {}", msg)
    }
}

// 3. Task Counter
pub struct TaskCounter {
    pending: usize,
    completed: usize,
    failed: usize,
}

impl TaskCounter {
    pub fn complete_task(&mut self) -> usize { self.completed += 1; self.completed }
    pub fn fail_task(&mut self) -> usize { self.failed += 1; self.failed }
    pub fn pending_count(&self) -> usize { self.pending }
}

// 4. Result Aggregator
pub struct ResultAggregator {
    results: Vec<i32>,
    timeout_ms: u64,
}

impl ResultAggregator {
    pub fn aggregate(&mut self) -> i32 {
        self.results.iter().sum()
    }
}

// 5. State Machine Actor
pub enum WorkerState { Idle, Processing, Complete }

pub struct StateMachineWorker {
    state: WorkerState,
    transition_count: usize,
}

impl StateMachineWorker {
    pub fn transition(&mut self, target: WorkerState) -> bool {
        // Validate and apply transition
        self.transition_count += 1;
        true
    }
}
```

### 11:00-13:00 | Message Passing at the Foundation (2h)

**Concepts**
- Actor message types (request, response, event, command)
- Ordered delivery within single actor
- Asynchronous processing mechanics
- Request/response correlation
- Fire-and-forget vs. request-response

**Problems** (19.1.11-19.1.20)
```
19.1.11: Implement typed message enum
         - RequestMessage
         - ResponseMessage
         - EventMessage
         - Support serialization hints

19.1.12: Create message router
         - Route messages to correct handler
         - Handle unknown message types
         - Log misrouted messages

19.1.13: Build request-response correlator
         - Assign IDs to requests
         - Match responses to requests
         - Timeout unmatched requests

19.1.14: Implement fire-and-forget dispatcher
         - Send message, don't wait
         - No response expected
         - Track delivery (best effort)

19.1.15: Create ordered delivery verifier
         - Verify messages arrive in order
         - Send sequence numbers
         - Detect out-of-order reception

19.1.16: Build message deduplicator
         - Track message IDs
         - Discard duplicates
         - Maintain ordering

19.1.17: Implement priority message queue
         - High/medium/low priorities
         - Process high first
         - Prevent starvation

19.1.18: Create message batching actor
         - Collect N messages
         - Process as batch
         - Reduce per-message overhead

19.1.19: Build message compression
         - Compress large messages
         - Decompress on receive
         - Measure overhead

19.1.20: Implement message versioning
         - Support v1, v2 message formats
         - Migrate v1 to v2
         - Track format version
```

**Hands-on** (Exercise 6-10: Message Handling Patterns)
```rust
// 6. Typed Message Handler
pub enum WorkMessage {
    Process(String),
    GetStatus,
    Shutdown,
}

pub struct MessageHandler {
    messages: Vec<String>,
}

impl MessageHandler {
    pub fn handle(&mut self, msg: WorkMessage) -> Option<String> {
        match msg {
            WorkMessage::Process(s) => {
                self.messages.push(s);
                Some("OK".to_string())
            }
            WorkMessage::GetStatus => {
                Some(format!("Processed: {}", self.messages.len()))
            }
            WorkMessage::Shutdown => None,
        }
    }
}

// 7. Router
pub struct MessageRouter {
    handlers: std::collections::HashMap<String, usize>,
}

impl MessageRouter {
    pub fn route(&mut self, target: String, msg: WorkMessage) -> bool {
        self.handlers.contains_key(&target)
    }
}

// 8. Request-Response Correlator
pub struct RequestResponseCorrelator {
    next_id: u32,
    pending: std::collections::HashMap<u32, String>,
}

impl RequestResponseCorrelator {
    pub fn new_request(&mut self, msg: String) -> u32 {
        self.next_id += 1;
        self.pending.insert(self.next_id, msg);
        self.next_id
    }
    pub fn handle_response(&mut self, id: u32, response: String) -> Option<String> {
        self.pending.remove(&id)
    }
}

// 9. Ordered Delivery Verifier
pub struct OrderedDeliveryVerifier {
    next_seq: u32,
    received: Vec<u32>,
}

impl OrderedDeliveryVerifier {
    pub fn verify(&mut self, seq: u32) -> bool {
        if seq == self.next_seq {
            self.received.push(seq);
            self.next_seq += 1;
            true
        } else {
            false
        }
    }
}

// 10. Deduplicator
pub struct MessageDeduplicator {
    seen: std::collections::HashSet<String>,
}

impl MessageDeduplicator {
    pub fn is_duplicate(&mut self, id: String) -> bool {
        !self.seen.insert(id)
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Actor Lifecycle & Supervision (3h)

**Concepts**
- Actor creation (spawn with initial state)
- Actor shutdown (graceful + forceful)
- Supervision: What happens when actor crashes?
- Restart strategies (restart, stop, escalate)
- Monitoring actor health
- Supervisor trees (hierarchical structure)

**Problems** (19.1.21-19.1.30)
```
19.1.21: Create actor lifecycle tracker
         - Track state: Created, Running, Stopped
         - Record timestamps
         - Count state changes

19.1.22: Implement graceful shutdown handler
         - Drain pending messages
         - Flush any state
         - Report final stats

19.1.23: Build actor crash recovery
         - Detect crash on message send failure
         - Restart from initial state
         - Preserve some history

19.1.24: Create restart strategy evaluator
         - Exponential backoff (1s, 2s, 4s, 8s)
         - Max restarts (e.g., 5 in 60s)
         - Circuit break on too many restarts

19.1.25: Implement supervision tree
         - Parent supervises N children
         - One child crashes -> restart logic
         - Multiple crashes -> escalate to parent

19.1.26: Build health check system
         - Ping actor, expect pong
         - Timeout = unhealthy
         - Mark failed, trigger restart

19.1.27: Create crash reporter
         - Log all crashes with timestamp
         - Include last message received
         - Stack trace (if available)

19.1.28: Implement escalation policy
         - Child crash -> parent restart
         - Parent crash -> stop all children
         - Top-level crash -> system shutdown

19.1.29: Build recovery metrics
         - Track crash count per actor
         - Time to recovery (crash -> healthy)
         - Success rate after recovery

19.1.30: Create actor census
         - List all running actors
         - Show their state
         - Count by type
```

**Hands-on** (Exercise 11-15: Lifecycle & Supervision)
```rust
// 11. Lifecycle Tracker
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActorLifecycleState { Created, Running, Stopped }

pub struct LifecycleTracker {
    state: ActorLifecycleState,
    state_changes: usize,
    created_at: std::time::Instant,
    stopped_at: Option<std::time::Instant>,
}

impl LifecycleTracker {
    pub fn new() -> Self {
        LifecycleTracker {
            state: ActorLifecycleState::Created,
            state_changes: 0,
            created_at: std::time::Instant::now(),
            stopped_at: None,
        }
    }
    pub fn transition_to(&mut self, new_state: ActorLifecycleState) -> bool {
        self.state = new_state;
        self.state_changes += 1;
        if new_state == ActorLifecycleState::Stopped {
            self.stopped_at = Some(std::time::Instant::now());
        }
        true
    }
}

// 12. Graceful Shutdown
pub struct GracefulShutdown {
    pending_messages: Vec<String>,
    flushed: bool,
}

impl GracefulShutdown {
    pub fn shutdown(&mut self) -> (usize, bool) {
        let count = self.pending_messages.len();
        self.pending_messages.clear();
        self.flushed = true;
        (count, self.flushed)
    }
}

// 13. Crash Recovery
pub struct CrashRecovery {
    crash_count: usize,
    last_crash_time: Option<std::time::Instant>,
    recovery_attempts: usize,
}

impl CrashRecovery {
    pub fn record_crash(&mut self) {
        self.crash_count += 1;
        self.last_crash_time = Some(std::time::Instant::now());
    }
    pub fn attempt_recovery(&mut self) -> bool {
        self.recovery_attempts += 1;
        self.crash_count > 0
    }
}

// 14. Restart Strategy (Exponential Backoff)
pub struct RestartStrategy {
    restart_count: usize,
    max_restarts: usize,
    time_window_secs: u64,
}

impl RestartStrategy {
    pub fn next_backoff_ms(&self) -> u64 {
        let backoff: u64 = 1000 * 2_u64.pow(self.restart_count as u32);
        backoff.min(8000)  // Cap at 8s
    }
    pub fn can_restart(&self) -> bool {
        self.restart_count < self.max_restarts
    }
}

// 15. Supervision Tree
pub struct SupervisionTree {
    children: std::collections::HashMap<String, ActorLifecycleState>,
}

impl SupervisionTree {
    pub fn new() -> Self {
        SupervisionTree {
            children: std::collections::HashMap::new(),
        }
    }
    pub fn add_child(&mut self, name: String) {
        self.children.insert(name, ActorLifecycleState::Running);
    }
    pub fn child_crashed(&mut self, name: &str) -> usize {
        self.children.insert(name.to_string(), ActorLifecycleState::Created);
        self.children.len()
    }
}
```

---

# TUESDAY: SCALING ACTORS TO HIGH CONCURRENCY (15 hours)

## 09:00-11:00 | Actor Pools & Work Distribution (2h)

**Concepts**
```
Single Actor System:        Actor Pool System:
├─ 1 actor processes all   ├─ N actors (10-1000)
├─ Sequential processing   ├─ Parallel work distribution
├─ Throughput = rate/sec   ├─ Throughput = N × (rate/sec)
├─ Bottleneck = single CPU ├─ Scale across CPUs
└─ Simple but slow          └─ Complex but fast

Pool Architecture:
┌─────────────────────────────┐
│ Work Queue (unbounded)      │
└────┬──────┬──────┬──────────┘
     │      │      │
  ┌──▼──┐ ┌─▼──┐ ┌─▼──┐
  │ W1  │ │ W2 │ │ W3 │  ... N workers
  └──┬──┘ └────┘ └────┘
     └─────┬─────┘
      Result Queue (ordered)

Key Challenge: Ordering
- Work arrives in order [A,B,C]
- Workers process in parallel -> order scrambled
- Solution: Shard by key or accept out-of-order
```

**Problems** (19.2.1-19.2.15)
```
19.2.1:  Create fixed-size actor pool
         - 10 worker actors
         - Distribute work round-robin
         - Track completion

19.2.2:  Implement dynamic pool sizing
         - Start with 5 workers
         - Add workers if queue > threshold
         - Remove if idle too long

19.2.3:  Build work distribution scheduler
         - Accept work items
         - Assign to least-busy worker
         - Track queue depth per worker

19.2.4:  Create worker pool with backpressure
         - Queue capacity = 1000 items
         - Reject new work if full
         - Signal back to sender

19.2.5:  Implement round-robin distributor
         - Cycle through workers 1,2,3,...N,1,2...
         - Load doesn't matter, just order
         - Measure variance in load

19.2.6:  Build hash-based distributor
         - Hash work item key to worker
         - Same key always goes to same worker
         - Maintains ordering per key

19.2.7:  Create work batching pool
         - Collect N items per batch
         - Send batch to worker
         - Reduce per-item overhead

19.2.8:  Implement priority work queue
         - High priority jobs first
         - Medium, low follow
         - Prevent starvation (low always gets some)

19.2.9:  Build work stealing pool
         - Workers can steal from neighbor's queue
         - Reduces idle time
         - Improves load balancing

19.2.10: Create pool with work timeouts
         - Each work item has deadline
         - Cancel if not started before deadline
         - Track expired/cancelled items

19.2.11: Implement result collection
         - Send results back to submitter
         - Maintain order if needed
         - Handle failures

19.2.12: Build pool statistics
         - Throughput (items/sec)
         - Latency (min, max, avg)
         - Queue depth over time

19.2.13: Create pool drain functionality
         - Stop accepting new work
         - Process all pending work
         - Then shutdown

19.2.14: Implement worker recycling
         - Restart worker after N items
         - Prevents memory leaks
         - Transparent to pool

19.2.15: Build adaptive pool
         - Monitor latency
         - If p95 > threshold, add workers
         - If idle, remove workers
```

**Hands-on** (Exercise 16-20: Pool Management)
```rust
// 16. Fixed-Size Actor Pool
pub struct ActorPool<T> {
    workers: Vec<T>,
    next_worker: usize,
    work_count: usize,
}

impl<T> ActorPool<T> {
    pub fn new(size: usize) -> Self {
        ActorPool {
            workers: Vec::with_capacity(size),
            next_worker: 0,
            work_count: 0,
        }
    }
    
    pub fn distribute(&mut self, work: String) {
        let idx = self.next_worker % self.workers.len();
        self.next_worker = idx + 1;
        self.work_count += 1;
    }
    
    pub fn work_distributed(&self) -> usize {
        self.work_count
    }
}

// 17. Least-Busy Scheduler
pub struct LeastBusyScheduler {
    queues: Vec<usize>,  // queue depth per worker
}

impl LeastBusyScheduler {
    pub fn schedule(&mut self) -> usize {
        self.queues.iter().position(|&q| q == *self.queues.iter().min().unwrap()).unwrap()
    }
}

// 18. Hash-Based Distributor
pub struct HashDistributor {
    worker_count: usize,
    distribution_count: usize,
}

impl HashDistributor {
    pub fn select_worker(&mut self, key: &str) -> usize {
        let hash = key.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
        self.distribution_count += 1;
        (hash as usize) % self.worker_count
    }
}

// 19. Work Queue with Backpressure
pub struct BackpressureQueue {
    items: Vec<String>,
    capacity: usize,
    rejected: usize,
}

impl BackpressureQueue {
    pub fn new(cap: usize) -> Self {
        BackpressureQueue {
            items: Vec::new(),
            capacity: cap,
            rejected: 0,
        }
    }
    
    pub fn try_enqueue(&mut self, item: String) -> bool {
        if self.items.len() < self.capacity {
            self.items.push(item);
            true
        } else {
            self.rejected += 1;
            false
        }
    }
}

// 20. Dynamic Pool Sizing
pub struct DynamicPool {
    current_size: usize,
    min_size: usize,
    max_size: usize,
    queue_depth: usize,
}

impl DynamicPool {
    pub fn evaluate_sizing(&mut self) -> Option<usize> {
        if self.queue_depth > 100 && self.current_size < self.max_size {
            self.current_size += 1;
            Some(self.current_size)
        } else {
            None
        }
    }
}
```

### 11:00-13:00 | Thread-Safe Coordination (2h)

**Concepts**
- Shared state across actor system (Arc, Mutex, RwLock)
- When actors need to see same data
- Trade-off: Safety vs. Performance
- Lock contention and bottlenecks

**Problems** (19.2.16-19.2.30)
```
19.2.16: Create shared counter with Arc<Mutex<>>
         - Multiple actors increment same counter
         - Atomic increments
         - Thread-safe reads

19.2.17: Implement shared cache
         - Multiple actors read/write cache
         - Use Arc<RwLock<HashMap>>
         - Measure lock contention

19.2.18: Build shared metrics collector
         - All actors update same metrics
         - Latency histogram (100 buckets)
         - Lock-free counter if possible

19.2.19: Create shared configuration
         - All actors share config
         - Config can be updated
         - Actors see new config instantly

19.2.20: Implement shared state synchronization
         - Actor A changes state
         - Actor B reads changed state
         - Verify consistency

19.2.21: Build atomic flag
         - Shutdown signal to all actors
         - Set once, read N times
         - No need for lock

19.2.22: Create barrier synchronization
         - 3 actors must wait for each other
         - Barrier releases all at once
         - Measure overhead

19.2.23: Implement rendezvous point
         - Actors wait at rendezvous
         - When all N arrive, proceed
         - Good for phased execution

19.2.24: Build fairness monitor
         - Track which actors get locks
         - Detect starvation
         - Ensure fair scheduling

19.2.25: Implement adaptive lock strategy
         - Spin-lock (fast) vs. sleep (slow)
         - Switch based on contention
         - Benchmark both

19.2.26: Create lock-free counter
         - Use compare-and-swap
         - Avoid lockwait entirely
         - Measure performance

19.2.27: Build seqlock (sequence number lock)
         - Readers don't block writers
         - Writers don't block readers
         - Measure latency improvement

19.2.28: Implement read-write lock patterns
         - Heavy readers, light writers
         - Read lock (shared), write lock (exclusive)
         - Benchmark vs. Mutex

19.2.29: Create transactional update
         - Read state, make changes, commit
         - If changed, retry
         - Optimistic concurrency

19.2.30: Build consensus protocol
         - N actors vote on decision
         - Majority wins
         - Measure convergence time
```

**Hands-on** (Exercise 21-25: Shared State)
```rust
// 21. Shared Counter
use std::sync::{Arc, Mutex};

pub struct SharedCounter {
    value: Arc<Mutex<u64>>,
}

impl SharedCounter {
    pub fn new() -> Self {
        SharedCounter {
            value: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn increment(&self) -> u64 {
        let mut v = self.value.lock().unwrap();
        *v += 1;
        *v
    }
}

// 22. Shared Cache
use std::collections::HashMap;
use std::sync::RwLock;

pub struct SharedCache {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl SharedCache {
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.read().unwrap().get(key).cloned()
    }
    
    pub fn set(&self, key: String, value: String) {
        self.data.write().unwrap().insert(key, value);
    }
}

// 23. Metrics Collector
pub struct MetricsCollector {
    counters: Arc<Mutex<HashMap<String, u64>>>,
}

impl MetricsCollector {
    pub fn increment(&self, name: &str) {
        let mut c = self.counters.lock().unwrap();
        *c.entry(name.to_string()).or_insert(0) += 1;
    }
}

// 24. Shared Configuration
pub struct SharedConfig {
    config: Arc<Mutex<String>>,
}

impl SharedConfig {
    pub fn update(&self, new_config: String) {
        *self.config.lock().unwrap() = new_config;
    }
    
    pub fn read(&self) -> String {
        self.config.lock().unwrap().clone()
    }
}

// 25. Atomic Shutdown Flag
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ShutdownSignal {
    should_shutdown: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn shutdown(&self) {
        self.should_shutdown.store(true, Ordering::SeqCst);
    }
    
    pub fn is_shutdown(&self) -> bool {
        self.should_shutdown.load(Ordering::SeqCst)
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Race Conditions & Data Races (3h)

**Concepts**
- Race condition vs. data race (different things!)
- Race condition: timing-dependent behavior
- Data race: unsynchronized memory access (undefined behavior)
- Compiler checks (Rust helps a lot here)
- Testing for races (hardest part)
- Tools: ThreadSanitizer, Miri

**Problems** (19.2.31-19.2.35)
```
19.2.31: Identify race condition in code
         - Two threads increment counter
         - No synchronization, no lock
         - Demonstrate counter loss

19.2.32: Fix with Mutex
         - Lock before increment
         - Verify counter value correct
         - Measure overhead

19.2.33: Create intentional data race (in unsafe block)
         - Write to mutable ref from two threads
         - Show UB behavior
         - Fix with proper synchronization

19.2.34: Build race condition detector
         - Run same operation N times
         - Look for inconsistent results
         - Report if any race found

19.2.35: Create test harness for data races
         - Spawn 100 threads
         - Each modifies shared state
         - Verify final state correct
```

**Hands-on** (Exercise 26-30: Concurrency Safety)
```rust
// 26. Race Condition Demonstrator
pub fn demonstrate_race() -> u64 {
    let mut counter = 0u64;
    // Thread 1: increment 1000x
    // Thread 2: increment 1000x
    // Expected: 2000, Actual: varies (RACE!)
    counter
}

// 27. Fixed with Mutex
use std::sync::{Arc, Mutex};
use std::thread;

pub fn safe_increment() -> u64 {
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = vec![];
    
    for _ in 0..2 {
        let c = Arc::clone(&counter);
        let h = thread::spawn(move || {
            for _ in 0..1000 {
                *c.lock().unwrap() += 1;
            }
        });
        handles.push(h);
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    *counter.lock().unwrap()  // Always 2000
}

// 28. Race Condition Detector
pub struct RaceDetector {
    results: Vec<u64>,
    iterations: usize,
}

impl RaceDetector {
    pub fn run(&mut self) -> bool {
        // Run same test N times
        // If results vary -> race detected
        let mut seen = std::collections::HashSet::new();
        for result in &self.results {
            seen.insert(*result);
        }
        seen.len() == 1  // All same = no race
    }
}

// 29. Multi-Thread Test Harness
pub fn stress_test_state() {
    let state = Arc::new(Mutex::new(0usize));
    let mut handles = vec![];
    
    for _ in 0..50 {
        let s = Arc::clone(&state);
        let h = thread::spawn(move || {
            for _ in 0..100 {
                let mut v = s.lock().unwrap();
                *v += 1;
            }
        });
        handles.push(h);
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    assert_eq!(*state.lock().unwrap(), 5000);
}

// 30. Atomic Operations (Lock-free)
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AtomicCounter {
    value: Arc<AtomicUsize>,
}

impl AtomicCounter {
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }
    
    pub fn get(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }
}
```

---

# WEDNESDAY: THREAD-SAFE DATA STRUCTURES & PATTERNS (15 hours)

## 09:00-11:00 | Arc, Mutex, RwLock Patterns (2h)

**Concepts**
```
Ownership Transfer:
  Arc = Atomic Reference Count
  - Shared ownership (multiple owners)
  - Cheap clones (increment refcount)
  - When all drop, deallocate

Mutability Control:
  Mutex = Mutual Exclusion
  - Interior mutability (change through &)
  - Lock required to access
  - Blocks competing threads

  RwLock = Read-Write Lock
  - Multiple readers OR one writer
  - More efficient for read-heavy
  - Readers don't block each other

Combined: Arc<Mutex<T>>
  ✓ Multiple threads own T
  ✓ Only one can mutate at a time
  ✓ Common pattern in concurrent code
```

**Problems** (19.3.1-19.3.10)
```
19.3.1:  Create Arc clone and verify refcount
         - Create Arc<i32>
         - Clone it 5 times
         - Verify refcount = 6

19.3.2:  Implement Arc<Mutex<>> for shared state
         - Shared counter across threads
         - Increment 100 times from 3 threads
         - Verify final = 300

19.3.3:  Build Arc<RwLock<>> for cache
         - Reader threads: 10 concurrent
         - Writer threads: 1
         - Measure throughput

19.3.4:  Create Arc<AtomicUsize> for lock-free counter
         - No locks, pure atomics
         - Increment from 4 threads
         - Compare perf to Mutex

19.3.5:  Implement Arc pool (reuse Arc instances)
         - Create 100 Arcs
         - Distribute, then collect
         - Measure allocation savings

19.3.6:  Build Arc chains (nested structures)
         - Arc<Mutex<Vec<Arc<Mutex<i32>>>>>
         - Inner mutability pattern
         - Handle nested locks carefully

19.3.7:  Create deadlock scenario (Arc<Mutex<T>>)
         - Thread A: lock L1 then L2
         - Thread B: lock L2 then L1
         - DEADLOCK! (demonstrate)

19.3.8:  Implement Arc with custom Drop
         - Track when dropped
         - Cleanup on final drop
         - Useful for resources

19.3.9:  Build weak references (Arc -> Weak<T>)
         - Prevent circular references
         - Thread-safe weak refs
         - Upgrade to Arc safely

19.3.10: Create Arc wrapper struct
         - Hide synchronization details
         - Safe, simple interface
         - Encapsulate pattern
```

**Hands-on** (Exercise 31-35: Shared Ownership)
```rust
// 31. Arc Refcount Tracker
use std::sync::Arc;

pub struct ArcRefCountTracker {
    value: Arc<i32>,
}

impl ArcRefCountTracker {
    pub fn new(val: i32) -> Self {
        ArcRefCountTracker {
            value: Arc::new(val),
        }
    }
    
    pub fn clone_ref(&self) -> Arc<i32> {
        Arc::clone(&self.value)
    }
    
    pub fn refcount(&self) -> usize {
        Arc::strong_count(&self.value)
    }
}

// 32. Shared State with Arc<Mutex<>>
pub struct SharedState {
    counter: Arc<Mutex<u64>>,
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            counter: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn increment(&self) -> u64 {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        *c
    }
    
    pub fn clone_for_thread(&self) -> Arc<Mutex<u64>> {
        Arc::clone(&self.counter)
    }
}

// 33. Cache with Arc<RwLock<>>
use std::collections::HashMap;
use std::sync::RwLock;

pub struct SharedCache {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl SharedCache {
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.read().unwrap().get(key).cloned()
    }
    
    pub fn set(&self, key: String, value: String) {
        self.data.write().unwrap().insert(key, value);
    }
    
    pub fn size(&self) -> usize {
        self.data.read().unwrap().len()
    }
}

// 34. Lock-Free Counter with Atomic
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LockFreeCounter {
    value: Arc<AtomicUsize>,
}

impl LockFreeCounter {
    pub fn new() -> Self {
        LockFreeCounter {
            value: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> usize {
        self.value.load(Ordering::Relaxed)
    }
}

// 35. Arc Wrapper Pattern
pub struct SafeSharedValue<T: Send + Sync> {
    value: Arc<Mutex<T>>,
}

impl<T: Send + Sync> SafeSharedValue<T> {
    pub fn new(val: T) -> Self {
        SafeSharedValue {
            value: Arc::new(Mutex::new(val)),
        }
    }
    
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut v = self.value.lock().unwrap();
        f(&mut *v)
    }
}
```

### 11:00-13:00 | Channel Patterns for Actor Communication (2h)

**Concepts**
- Channels: Producer-Consumer queues
- Sender + Receiver (mpsc = multi-producer, single-consumer)
- Send message, get response
- Bounded vs. unbounded channels
- Channel errors (sender dropped, receiver dropped)

**Problems** (19.3.11-19.3.20)
```
19.3.11: Create simple channel (Sender, Receiver)
         - Send "hello"
         - Receive and verify
         - Error if receiver dropped

19.3.12: Implement multi-producer channel
         - 3 threads send to 1 receiver
         - Receiver collects all
         - Measure throughput

19.3.13: Build request-response with channels
         - Sender: send request, wait for response
         - Receiver: read request, send response
         - Correlation with IDs

19.3.14: Create bounded channel
         - Capacity = 100 messages
         - Sender blocks if full
         - Receiver drains, sender unblocks

19.3.15: Implement channel select (2 channels)
         - Listen on channel A or B
         - Process whichever has message
         - Similar to erlang receive

19.3.16: Build pipeline with channels
         - Stage 1 sends to stage 2
         - Stage 2 sends to stage 3
         - Chain multiple stages

19.3.17: Create channel timeout
         - Send with timeout
         - Receive with timeout
         - Return error if timeout

19.3.18: Implement broadcast channel
         - 1 sender, N receivers
         - All receivers see message
         - Useful for events

19.3.19: Build work distribution with channel
         - Worker thread reads from channel
         - Main sends work
         - Worker sends back results

19.3.20: Create channel statistics
         - Track messages sent
         - Track messages received
         - Measure queue depth over time
```

**Hands-on** (Exercise 36-40: Channels)
```rust
// 36. Simple Channel
use std::sync::mpsc;
use std::thread;

pub struct SimpleChannel {
    sent: usize,
}

impl SimpleChannel {
    pub fn demo() -> usize {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            tx.send("hello".to_string()).unwrap();
        });
        
        let _ = rx.recv().unwrap();
        1
    }
}

// 37. Multi-Producer Channel
pub struct MultiProducer {
    messages_count: usize,
}

impl MultiProducer {
    pub fn demo() -> usize {
        let (tx, rx) = mpsc::channel();
        let mut handles = vec![];
        
        for _ in 0..3 {
            let t = tx.clone();
            let h = thread::spawn(move || {
                t.send("msg".to_string()).unwrap();
            });
            handles.push(h);
        }
        drop(tx);
        
        let mut count = 0;
        for _ in rx.iter() {
            count += 1;
        }
        
        for h in handles {
            h.join().unwrap();
        }
        
        count
    }
}

// 38. Request-Response Channel
pub struct RequestResponse {
    request_ids: usize,
}

impl RequestResponse {
    pub fn demo() -> (String, String) {
        let (req_tx, req_rx) = mpsc::channel();
        let (resp_tx, resp_rx) = mpsc::channel();
        
        thread::spawn(move || {
            let id = req_rx.recv().unwrap();
            resp_tx.send(format!("response to {}", id)).unwrap();
        });
        
        req_tx.send("request-1".to_string()).unwrap();
        let response = resp_rx.recv().unwrap();
        
        ("request-1".to_string(), response)
    }
}

// 39. Bounded Channel (Backpressure)
pub struct BoundedChannelDemo {
    max_capacity: usize,
}

impl BoundedChannelDemo {
    pub fn demo() -> usize {
        let (tx, rx) = mpsc::channel::<String>();
        let mut count = 0;
        
        // Sender will block if channel full
        for i in 0..100 {
            if tx.send(format!("msg-{}", i)).is_ok() {
                count += 1;
            }
        }
        
        // Receiver drains
        for _ in rx.iter() {
            // Process messages
        }
        
        count
    }
}

// 40. Channel Statistics
pub struct ChannelStats {
    sent: usize,
    received: usize,
    failed: usize,
}

impl ChannelStats {
    pub fn track_send(&mut self, result: bool) {
        if result {
            self.sent += 1;
        } else {
            self.failed += 1;
        }
    }
    
    pub fn track_receive(&mut self) {
        self.received += 1;
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Advanced Sync Primitives (3h)

**Concepts**
- Barrier: synchronize N threads at a point
- Condvar: wake threads when condition met
- Once: run code exactly once
- Semaphore: limit concurrent access to resource

**Problems** (19.3.21-19.3.35)
```
19.3.21: Create barrier for 3 threads
         - All 3 reach barrier
         - All resume together
         - Measure synchronization overhead

19.3.22: Implement condition variable
         - Thread A: wait on condition
         - Thread B: set condition, notify
         - A wakes up and continues

19.3.23: Build bounded resource pool
         - Max 5 concurrent requests
         - Wait if limit reached
         - Release to free slot

19.3.24: Create Once flag for initialization
         - Init code runs exactly once
         - Subsequent calls skipped
         - Thread-safe without overhead

19.3.25: Implement reader-writer lock fairness
         - Multiple readers
         - One writer
         - Ensure writer eventually gets access

19.3.26: Build parking lot pattern
         - Threads park on wait
         - Woken with unpark
         - Lower overhead than Condvar

19.3.27: Create semaphore (N permits)
         - Acquire reduces count
         - Release increases count
         - Wait if count = 0

19.3.28: Implement spin lock
         - Busy-wait instead of sleep
         - Better for very short holds
         - Measure CPU usage

19.3.29: Build futex (fast userspace mutex)
         - Kernel-assisted synchronization
         - Fast path, slow path
         - More efficient than spin lock

19.3.30: Create hierarchical mutex
         - Level-based locking order
         - Deadlock prevention
         - Verify no cycles possible

19.3.31: Implement transactional lock
         - Read state, make changes
         - If changed, conflict and retry
         - Optimistic synchronization

19.3.32: Build stm (Software Transactional Memory)
         - Atomic blocks
         - Automatic retry on conflict
         - High-level concurrency

19.3.33: Create monitor pattern
         - Lock + condition combined
         - Wait/notify built-in
         - Safer than raw primitives

19.3.34: Implement epoch-based reclamation
         - Track which threads using old version
         - Safe to free when no thread in epoch
         - For lock-free data structures

19.3.35: Build generation counter
         - Track version of data
         - Writers bump generation
         - Readers verify generation didn't change
```

**Hands-on** (Exercise 41-45: Advanced Sync)
```rust
// 41. Barrier Pattern
use std::sync::{Arc, Barrier};
use std::thread;

pub struct BarrierDemo {
    thread_count: usize,
}

impl BarrierDemo {
    pub fn demo() -> usize {
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = vec![];
        
        for _ in 0..3 {
            let b = Arc::clone(&barrier);
            let h = thread::spawn(move || {
                // Do work...
                b.wait();  // All wait here
                // Continue...
            });
            handles.push(h);
        }
        
        for h in handles {
            h.join().unwrap();
        }
        
        3
    }
}

// 42. Condition Variable
use std::sync::{Arc, Mutex, Condvar};

pub struct CondvarDemo {
    signaled: bool,
}

impl CondvarDemo {
    pub fn demo() -> bool {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = Arc::clone(&pair);
        
        let h = thread::spawn(move || {
            let (lock, cvar) = &*pair2;
            let mut flag = lock.lock().unwrap();
            while !*flag {
                flag = cvar.wait(flag).unwrap();
            }
        });
        
        thread::sleep(std::time::Duration::from_millis(10));
        let (lock, cvar) = &*pair;
        *lock.lock().unwrap() = true;
        cvar.notify_one();
        
        h.join().unwrap();
        true
    }
}

// 43. Bounded Resource Pool
pub struct ResourcePool {
    available: usize,
    capacity: usize,
}

impl ResourcePool {
    pub fn new(capacity: usize) -> Self {
        ResourcePool {
            available: capacity,
            capacity,
        }
    }
    
    pub fn acquire(&mut self) -> bool {
        if self.available > 0 {
            self.available -= 1;
            true
        } else {
            false
        }
    }
    
    pub fn release(&mut self) {
        self.available = (self.available + 1).min(self.capacity);
    }
}

// 44. Once Initialization
use std::sync::Once;

pub struct OnceDemo {
    init_count: usize,
}

impl OnceDemo {
    pub fn demo() -> usize {
        static INIT: Once = Once::new();
        static mut VALUE: usize = 0;
        
        INIT.call_once(|| {
            unsafe { VALUE = 42; }
        });
        
        unsafe { VALUE }
    }
}

// 45. Generation Counter Pattern
pub struct GenerationCounter {
    generation: Arc<Mutex<u64>>,
}

impl GenerationCounter {
    pub fn new() -> Self {
        GenerationCounter {
            generation: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn bump(&self) -> u64 {
        let mut g = self.generation.lock().unwrap();
        *g += 1;
        *g
    }
    
    pub fn current(&self) -> u64 {
        *self.generation.lock().unwrap()
    }
}
```

---

# THURSDAY: DEADLOCK PREVENTION & SYNCHRONIZATION (15 hours)

## 09:00-11:00 | Deadlock Classification & Detection (2h)

**Concepts**
```
Deadlock = Circular Wait for Resources

Example:
  Thread A: holds L1, waiting for L2
  Thread B: holds L2, waiting for L1
  → Circular dependency → DEADLOCK

Conditions (all must be true):
  1. Mutual Exclusion (resource can't be shared)
  2. Hold and Wait (hold resources while waiting)
  3. No Preemption (can't take resource back)
  4. Circular Wait (cycle in dependency graph)

Detection:
  - Build wait graph
  - Check for cycles
  - If cycle found -> deadlock
```

**Problems** (19.4.1-19.4.10)
```
19.4.1:  Create deadlock scenario
         - Thread A: lock L1 then L2
         - Thread B: lock L2 then L1
         - Demonstrate hang

19.4.2:  Detect deadlock (wait graph)
         - Track who's waiting for what
         - Build dependency graph
         - Find cycles

19.4.3:  Create dining philosophers problem
         - 5 philosophers, 5 forks
         - Odd order: take left then right
         - Even order: take right then left
         - Should avoid deadlock

19.4.4:  Implement lock timeout
         - Try to acquire with 1s timeout
         - Fail gracefully if timeout
         - Retry with backoff

19.4.5:  Build deadlock recovery
         - Detect cycle
         - Force release one lock
         - Restart that thread

19.4.6:  Create lock ordering constraint
         - Level-based: L1 < L2 < L3
         - Always acquire in order
         - Prove no deadlock possible

19.4.7:  Implement bankers algorithm
         - Request resources
         - Check if safe state
         - Grant only if safe

19.4.8:  Build lock graph visualizer
         - Show which lock depends on which
         - Highlight cycles
         - Output as string

19.4.9:  Create cyclic dependency detector
         - Maintain dependency graph
         - On each lock, check for cycles
         - Report immediately if found

19.4.10: Implement wait-for matrix
          - N threads × N resources
          - Mark who's waiting for what
          - Detect cycle
```

**Hands-on** (Exercise 46-50: Deadlock)
```rust
// 46. Basic Deadlock Scenario (WILL HANG - for demo only)
pub struct DeadlockDemo {
    hung: bool,
}

impl DeadlockDemo {
    pub fn demo() -> bool {
        // This would hang:
        // let l1 = Arc::new(Mutex::new(0));
        // let l2 = Arc::new(Mutex::new(0));
        // thread A: lock(l1) -> lock(l2)
        // thread B: lock(l2) -> lock(l1)
        // DEADLOCK!
        
        // For testing, just return false (indicating potential deadlock)
        false
    }
}

// 47. Deadlock Detector (Wait Graph)
use std::collections::{HashMap, HashSet};

pub struct DeadlockDetector {
    wait_graph: HashMap<String, HashSet<String>>,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        DeadlockDetector {
            wait_graph: HashMap::new(),
        }
    }
    
    pub fn add_wait(&mut self, thread: String, waiting_for: String) {
        self.wait_graph.entry(thread).or_insert_with(HashSet::new).insert(waiting_for);
    }
    
    pub fn has_cycle(&self) -> bool {
        // DFS to detect cycle
        for start in self.wait_graph.keys() {
            let mut visited = HashSet::new();
            if self.dfs_has_cycle(start, &mut visited, &mut HashSet::new()) {
                return true;
            }
        }
        false
    }
    
    fn dfs_has_cycle(&self, node: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        
        if let Some(neighbors) = self.wait_graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_has_cycle(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(node);
        false
    }
}

// 48. Lock Ordering (Safe)
pub struct SafeLocking {
    value: Arc<Mutex<(u64, u64)>>,
}

impl SafeLocking {
    pub fn safe_update(&self) -> (u64, u64) {
        let mut v = self.value.lock().unwrap();
        v.0 += 1;
        v.1 += 1;
        *v
    }
}

// 49. Try Lock with Timeout
pub struct TimeoutLock {
    lock: Arc<Mutex<u64>>,
}

impl TimeoutLock {
    pub fn try_update(&self) -> Option<u64> {
        // Rust's Mutex doesn't have built-in timeout
        // Simulate with try_lock
        if let Ok(mut v) = self.lock.try_lock() {
            *v += 1;
            Some(*v)
        } else {
            None  // Would be timeout
        }
    }
}

// 50. Lock Graph Analyzer
pub struct LockGraphAnalyzer {
    graph: HashMap<String, Vec<String>>,
}

impl LockGraphAnalyzer {
    pub fn new() -> Self {
        LockGraphAnalyzer { graph: HashMap::new() }
    }
    
    pub fn add_edge(&mut self, from: String, to: String) {
        self.graph.entry(from).or_insert_with(Vec::new).push(to);
    }
    
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for (from, tos) in &self.graph {
            for to in tos {
                result.push_str(&format!("{} -> {}\n", from, to));
            }
        }
        result
    }
}
```

### 11:00-13:00 | Lock Ordering Protocols (2h)

**Concepts**
- Assign levels to locks: L1, L2, L3...
- Always acquire L1 before L2 before L3
- Proof: no cycle possible (acyclic ordering)
- Trade-off: sometimes lock more than needed

**Problems** (19.4.11-19.4.20)
```
19.4.11: Implement level-based lock ordering
         - Asset lock (level 1)
         - Transaction lock (level 2)
         - Account lock (level 3)
         - Always acquire in order

19.4.12: Create bank transfer system
         - Transfer from account A to B
         - Use lock ordering (lower ID first)
         - No deadlock possible

19.4.13: Build hierarchical lock
         - Parent lock before child
         - Natural tree structure
         - Guarantees acyclic

19.4.14: Implement strict ordering enforcement
         - Check lock levels on acquire
         - Panic if acquire lower after higher
         - Prevents deadlock

19.4.15: Create lock ordering validator
         - Read code, extract lock operations
         - Verify strictly ascending levels
         - Report violations

19.4.16: Build multi-table transaction
         - 3 tables with assigned levels
         - Transfer data between tables
         - Use strict ordering

19.4.17: Implement resource allocation graph
         - Show lock dependencies
         - Verify acyclic
         - Safe allocation proven

19.4.18: Create safe transfer protocol
         - Lock accounts in sorted order
         - Transfer money
         - Unlock in reverse order

19.4.19: Build deadlock-free scheduler
         - Tasks declare required locks upfront
         - Scheduler orders based on lock levels
         - Execute deadlock-free

19.4.20: Implement phase-based locking
         - Phase 1: acquire all locks needed
         - Phase 2: execute work
         - Phase 3: release locks
```

**Hands-on** (Exercise 51-55: Lock Ordering)
```rust
// 51. Level-Based Locking
use std::sync::Mutex;
use std::cmp::Ordering;

pub struct LeveledLock {
    level: usize,
    value: Mutex<u64>,
}

impl LeveledLock {
    pub fn new(level: usize, init: u64) -> Self {
        LeveledLock {
            level,
            value: Mutex::new(init),
        }
    }
}

pub struct LockOrderingPolicy {
    last_acquired_level: Option<usize>,
}

impl LockOrderingPolicy {
    pub fn new() -> Self {
        LockOrderingPolicy {
            last_acquired_level: None,
        }
    }
    
    pub fn can_acquire(&self, lock: &LeveledLock) -> bool {
        match self.last_acquired_level {
            None => true,
            Some(last) => lock.level > last,
        }
    }
}

// 52. Bank Transfer (Safe Ordering)
pub struct Account {
    id: u32,
    balance: Mutex<u64>,
}

impl Account {
    pub fn transfer_to(&self, other: &Account, amount: u64) -> bool {
        // Always lock lower ID first
        let (first, second) = if self.id < other.id {
            (self, other)
        } else {
            (other, self)
        };
        
        let mut b1 = first.balance.lock().unwrap();
        let mut b2 = second.balance.lock().unwrap();
        
        if *b1 >= amount {
            *b1 -= amount;
            *b2 += amount;
            true
        } else {
            false
        }
    }
}

// 53. Hierarchical Locks
pub struct HierarchicalLock {
    level: usize,
    parent: Option<Box<HierarchicalLock>>,
    value: Mutex<u64>,
}

impl HierarchicalLock {
    pub fn depth(&self) -> usize {
        match &self.parent {
            None => self.level,
            Some(p) => p.depth() + 1,
        }
    }
}

// 54. Strict Ordering Enforcer
pub struct StrictOrderEnforcer {
    current_level: Option<usize>,
}

impl StrictOrderEnforcer {
    pub fn acquire(&mut self, level: usize) -> bool {
        match self.current_level {
            None => {
                self.current_level = Some(level);
                true
            }
            Some(current) => {
                if level > current {
                    self.current_level = Some(level);
                    true
                } else {
                    false  // Violation!
                }
            }
        }
    }
}

// 55. Safe Transaction
pub struct SafeTransaction {
    locks_acquired: Vec<usize>,
}

impl SafeTransaction {
    pub fn acquire_locks(&mut self, mut levels: Vec<usize>) {
        // Sort before acquiring (deadlock-safe)
        levels.sort();
        self.locks_acquired = levels;
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Synchronization Primitives & Patterns (3h)

**Concepts**
- Barriers, condition variables, semaphores
- When to use which
- Custom synchronization structures
- Performance vs. safety trade-offs

**Problems** (19.4.21-19.4.35)
```
19.4.21: Implement barrier for 5 threads
         - All 5 wait at barrier
         - Last arrival unblocks all
         - For phased execution

19.4.22: Create condition variable signaling
         - Thread waits on predicate
         - Another thread changes state, notifies
         - Waiter wakes up

19.4.23: Build semaphore (limit N concurrent)
         - Max 3 threads access resource
         - Others wait in queue
         - Release frees one waiter

19.4.24: Implement read-write lock
         - Multiple readers
         - Exclusive writer
         - Reader priority optimal?

19.4.25: Create not-empty condition
         - Queue starts empty
         - Producers add, signal
         - Consumers wait until not empty

19.4.26: Build monitor pattern
         - Lock + condition variable combined
         - Single interface for synchronization
         - Cleaner code

19.4.27: Implement rendezvous (2 threads)
         - Both must reach point
         - Both wait for other
         - Both proceed together

19.4.28: Create barrier with timeout
         - Wait up to 1s for all threads
         - Timeout if not all arrive
         - Handle partial completion

19.4.29: Build staged pipeline
         - Stage 1 -> Stage 2 -> Stage 3
         - Each stage has barrier
         - Syncpoint between stages

19.4.30: Implement latch (countdown)
         - Initial count N
         - Each threadmarks done (N-1)
         - Last thread unblocks all

19.4.31: Create exclusive access lock
         - Limited resource (1 copy)
         - One thread accesses
         - Others queue
         - FIFO order

19.4.32: Build producer-consumer pattern
         - Bounded buffer (capacity 10)
         - Producers add, consumers take
         - Fullness/emptiness conditions

19.4.33: Implement two-phase commit
         - Prepare phase: all locks
         - Commit phase: all apply
         - Rollback if any prepare fails

19.4.34: Create thread pool with queue
         - Fixed N workers
         - Work queue (bounded)
         - Wait for completion

19.4.35: Build starvation detector
         - Track avg wait time
         - If some threads never progress -> starvation
         - Alert & restart
```

**Hands-on** (Exercise 56-60: Synchronization)
```rust
// 56. Barrier
use std::sync::{Arc, Barrier};
use std::thread;

pub struct BarrierSync {
    thread_count: usize,
}

impl BarrierSync {
    pub fn demo(n: usize) -> usize {
        let barrier = Arc::new(Barrier::new(n));
        let mut handles = vec![];
        
        for i in 0..n {
            let b = Arc::clone(&barrier);
            let h = thread::spawn(move || {
                // Phase 1
                
                // Wait for all
                b.wait();
                
                // Phase 2
                i
            });
            handles.push(h);
        }
        
        for h in handles {
            h.join().unwrap();
        }
        n
    }
}

// 57. Condition Variable
use std::sync::{Arc, Mutex, Condvar};

pub struct NotEmptyQueue {
    items: Arc<(Mutex<Vec<String>>, Condvar)>,
}

impl NotEmptyQueue {
    pub fn new() -> Self {
        NotEmptyQueue {
            items: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
        }
    }
    
    pub fn add(&self, item: String) {
        let (lock, cvar) = &*self.items;
        let mut items = lock.lock().unwrap();
        items.push(item);
        cvar.notify_one();
    }
    
    pub fn take(&self) -> String {
        let (lock, cvar) = &*self.items;
        let mut items = lock.lock().unwrap();
        while items.is_empty() {
            items = cvar.wait(items).unwrap();
        }
        items.pop().unwrap()
    }
}

// 58. Semaphore (Limited Access)
pub struct Semaphore {
    count: Arc<Mutex<usize>>,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Semaphore {
            count: Arc::new(Mutex::new(permits)),
        }
    }
    
    pub fn acquire(&self) -> bool {
        let mut c = self.count.lock().unwrap();
        if *c > 0 {
            *c -= 1;
            true
        } else {
            false
        }
    }
    
    pub fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
    }
}

// 59. Producer-Consumer Buffer
pub struct BoundedBuffer {
    buffer: Arc<(Mutex<Vec<u64>>, Condvar, Condvar)>,
    capacity: usize,
}

impl BoundedBuffer {
    pub fn new(cap: usize) -> Self {
        BoundedBuffer {
            buffer: Arc::new((Mutex::new(Vec::new()), Condvar::new(), Condvar::new())),
            capacity: cap,
        }
    }
    
    pub fn produce(&self, item: u64) {
        let (lock, not_full, not_empty) = &*self.buffer;
        let mut buf = lock.lock().unwrap();
        while buf.len() >= self.capacity {
            buf = not_full.wait(buf).unwrap();
        }
        buf.push(item);
        not_empty.notify_one();
    }
    
    pub fn consume(&self) -> Option<u64> {
        let (lock, not_full, not_empty) = &*self.buffer;
        let mut buf = lock.lock().unwrap();
        while buf.is_empty() {
            buf = not_empty.wait(buf).unwrap();
        }
        let item = buf.pop();
        not_full.notify_one();
        item
    }
}

// 60. Latch (Countdown)
pub struct Latch {
    count: Arc<(Mutex<usize>, Condvar)>,
}

impl Latch {
    pub fn new(initial: usize) -> Self {
        Latch {
            count: Arc::new((Mutex::new(initial), Condvar::new())),
        }
    }
    
    pub fn done(&self) {
        let (lock, cvar) = &*self.count;
        let mut c = lock.lock().unwrap();
        if *c > 0 {
            *c -= 1;
            if *c == 0 {
                cvar.notify_all();
            }
        }
    }
    
    pub fn wait(&self) {
        let (lock, cvar) = &*self.count;
        let mut c = lock.lock().unwrap();
        while *c > 0 {
            c = cvar.wait(c).unwrap();
        }
    }
}
```

---

# FRIDAY: CAPSTONE - MULTI-NODE WORKER POOL SYSTEM (15 hours)

## 09:00-12:00 | Design & Architecture (3h)

**Project Brief**
```
Build a production-grade, multi-node worker pool that:
  ✓ Accepts 10,000 work items
  ✓ Distributes to 50 worker actors
  ✓ Handles failures gracefully (retry logic)
  ✓ Tracks completion (success/failure metrics)
  ✓ Measures throughput (items/sec)
  ✓ Supports graceful shutdown
  ✓ Real-world: order processing, image transformation, data ETL
```

**Architecture**
```
┌─────────────────────────────────────────────┐
│ Main Coordinator                            │
│  - Accept work items                        │
│  - Track in-flight requests                 │
│  - Collect results                          │
└────────────┬─────────────────────────────────┘
             │
      ┌──────┴─────────────────┐
      ▼                         ▼
 Work Queue              Result Queue
      │                         ▲
      └──────────────┬──────────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
     Worker1      Worker2  ...  Worker50
     (Process)  (Process)    (Process)
```

**Key Requirements**
- 200+ lines of production code
- All 50 workers active
- Track 3 failure scenarios:
  1. Transient failure (retry succeeds)
  2. Permanent failure (max retries, discard)
  3. Timeout (worker hung, restart)
- Measure p50, p95, p99 latencies

**Hands-on** (Capstone Exercises)

```rust
// CAPSTONE: Multi-Node Worker Pool

use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;

// ===== Work Item Definition =====

#[derive(Clone, Debug)]
pub struct WorkItem {
    id: u64,
    payload: String,
    submitted_at: Instant,
}

#[derive(Clone, Debug)]
pub struct WorkResult {
    item_id: u64,
    status: ResultStatus,
    latency_ms: u64,
}

#[derive(Clone, Debug)]
pub enum ResultStatus {
    Success,
    Retried(u32),         // succeeded after N retries
    Failed(String),       // permanent failure reason
    TimedOut,
}

// ===== Worker Actor =====

pub struct WorkerActor {
    id: u32,
    items_processed: u64,
    last_heartbeat: Instant,
}

impl WorkerActor {
    pub fn new(id: u32) -> Self {
        WorkerActor {
            id,
            items_processed: 0,
            last_heartbeat: Instant::now(),
        }
    }
    
    pub fn process(&mut self, item: WorkItem) -> Result<String, String> {
        // Simulate work: hash the payload
        let hash = item.payload.chars()
            .fold(0u32, |acc, c| acc.wrapping_add(c as u32));
        
        // 5% chance of transient failure
        if hash % 20 == 0 {
            return Err("transient".to_string());
        }
        
        // 1% chance of timeout (long work)
        if hash % 100 == 0 {
            thread::sleep(Duration::from_millis(5000));
        }
        
        self.items_processed += 1;
        self.last_heartbeat = Instant::now();
        
        Ok(format!("processed-{}", hash))
    }
}

// ===== Worker Pool Coordinator =====

pub struct WorkerPool {
    workers: Vec<Arc<Mutex<WorkerActor>>>,
    work_queue: Arc<Mutex<Vec<WorkItem>>>,
    result_queue: Arc<Mutex<Vec<WorkResult>>>,
    queue_condvar: Arc<Condvar>,
    result_condvar: Arc<Condvar>,
    next_worker: Arc<AtomicUsize>,
    total_submitted: Arc<AtomicUsize>,
    total_completed: Arc<AtomicUsize>,
    is_shutdown: Arc<Mutex<bool>>,
}

impl WorkerPool {
    pub fn new(num_workers: usize) -> Self {
        let mut workers = Vec::new();
        for i in 0..num_workers {
            workers.push(Arc::new(Mutex::new(WorkerActor::new(i as u32))));
        }
        
        WorkerPool {
            workers,
            work_queue: Arc::new(Mutex::new(Vec::new())),
            result_queue: Arc::new(Mutex::new(Vec::new())),
            queue_condvar: Arc::new(Condvar::new()),
            result_condvar: Arc::new(Condvar::new()),
            next_worker: Arc::new(AtomicUsize::new(0)),
            total_submitted: Arc::new(AtomicUsize::new(0)),
            total_completed: Arc::new(AtomicUsize::new(0)),
            is_shutdown: Arc::new(Mutex::new(false)),
        }
    }
    
    pub fn submit_work(&self, item: WorkItem) -> bool {
        let mut queue = self.work_queue.lock().unwrap();
        queue.push(item);
        self.total_submitted.fetch_add(1, Ordering::Relaxed);
        self.queue_condvar.notify_one();
        true
    }
    
    pub fn run_workers(&self) {
        let mut handles = vec![];
        
        for worker_idx in 0..self.workers.len() {
            let worker = Arc::clone(&self.workers[worker_idx]);
            let queue = Arc::clone(&self.work_queue);
            let results = Arc::clone(&self.result_queue);
            let queue_cv = Arc::clone(&self.queue_condvar);
            let result_cv = Arc::clone(&self.result_condvar);
            let shutdown = Arc::clone(&self.is_shutdown);
            let completed = Arc::clone(&self.total_completed);
            
            let h = thread::spawn(move || {
                loop {
                    // Check shutdown
                    if *shutdown.lock().unwrap() {
                        let mut q = queue.lock().unwrap();
                        if q.is_empty() {
                            break;
                        }
                        drop(q);
                    }
                    
                    // Get work item
                    let mut q = queue.lock().unwrap();
                    if q.is_empty() {
                        if *shutdown.lock().unwrap() {
                            break;
                        }
                        q = queue_cv.wait(q).unwrap();
                        continue;
                    }
                    
                    let item = q.remove(0);
                    drop(q);
                    
                    // Process with retries
                    let mut retries = 0;
                    let mut success = false;
                    let mut error = String::new();
                    
                    while retries < 3 {
                        let mut w = worker.lock().unwrap();
                        match w.process(item.clone()) {
                            Ok(_) => {
                                success = true;
                                break;
                            }
                            Err(e) => {
                                error = e;
                                retries += 1;
                            }
                        }
                        drop(w);
                        
                        if retries < 3 {
                            thread::sleep(Duration::from_millis(10 * (2 ^ retries)));
                        }
                    }
                    
                    // Record result
                    let latency_ms = item.submitted_at.elapsed().as_millis() as u64;
                    let status = if success {
                        if retries > 0 {
                            ResultStatus::Retried(retries)
                        } else {
                            ResultStatus::Success
                        }
                    } else {
                        ResultStatus::Failed(error)
                    };
                    
                    let result = WorkResult {
                        item_id: item.id,
                        status,
                        latency_ms,
                    };
                    
                    let mut res_queue = results.lock().unwrap();
                    res_queue.push(result);
                    completed.fetch_add(1, Ordering::Relaxed);
                    result_cv.notify_one();
                }
            });
            
            handles.push(h);
        }
        
        // Wait for all workers
        for h in handles {
            h.join().unwrap();
        }
    }
    
    pub fn shutdown(&self) {
        *self.is_shutdown.lock().unwrap() = true;
        self.queue_condvar.notify_all();
    }
    
    pub fn collect_results(&self) -> Vec<WorkResult> {
        let mut results = self.result_queue.lock().unwrap();
        results.drain(..).collect()
    }
    
    pub fn stats(&self) -> PoolStats {
        let results = self.result_queue.lock().unwrap();
        
        let mut latencies: Vec<u64> = results.iter()
            .map(|r| r.latency_ms)
            .collect();
        latencies.sort();
        
        let len = latencies.len() as u64;
        let p50 = latencies.get(len / 2).copied().unwrap_or(0);
        let p95 = latencies.get((len * 95) / 100).copied().unwrap_or(0);
        let p99 = latencies.get((len * 99) / 100).copied().unwrap_or(0);
        
        let success_count = results.iter()
            .filter(|r| matches!(r.status, ResultStatus::Success | ResultStatus::Retried(_)))
            .count();
        
        let failed_count = results.iter()
            .filter(|r| matches!(r.status, ResultStatus::Failed(_)))
            .count();
        
        let total_latency: u64 = results.iter().map(|r| r.latency_ms).sum();
        let avg_latency = if results.len() > 0 {
            total_latency / results.len() as u64
        } else {
            0
        };
        
        PoolStats {
            total_items: results.len(),
            successful: success_count,
            failed: failed_count,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
        }
    }
}

pub struct PoolStats {
    pub total_items: usize,
    pub successful: usize,
    pub failed: usize,
    pub avg_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
}

// ===== Test Harness =====

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_pool() {
        let pool = WorkerPool::new(10);
        
        // Submit 100 items
        for i in 0..100 {
            pool.submit_work(WorkItem {
                id: i,
                payload: format!("work-{}", i),
                submitted_at: Instant::now(),
            });
        }
        
        // Run pool (workers process from queue)
        let start = Instant::now();
        pool.run_workers();
        let elapsed = start.elapsed();
        
        // Collect results
        let stats = pool.stats();
        
        println!("Pool Stats:");
        println!("  Items: {}", stats.total_items);
        println!("  Success: {}", stats.successful);
        println!("  Failed: {}", stats.failed);
        println!("  Avg latency: {}ms", stats.avg_latency_ms);
        println!("  P50 latency: {}ms", stats.p50_latency_ms);
        println!("  P95 latency: {}ms", stats.p95_latency_ms);
        println!("  P99 latency: {}ms", stats.p99_latency_ms);
        println!("  Elapsed: {:?}", elapsed);
        
        assert_eq!(stats.total_items, 100);
        assert!(stats.successful > 90);  // Most should succeed
    }
    
    #[test]
    fn test_high_concurrency() {
        let pool = WorkerPool::new(50);
        
        // Submit 10,000 items
        for i in 0..10000 {
            pool.submit_work(WorkItem {
                id: i,
                payload: format!("work-{}", i),
                submitted_at: Instant::now(),
            });
        }
        
        // Run with graceful shutdown
        let start = Instant::now();
        pool.run_workers();
        let elapsed = start.elapsed();
        
        let stats = pool.stats();
        let throughput = stats.total_items as f64 / elapsed.as_secs_f64();
        
        println!("High Concurrency Test:");
        println!("  Total items: {}", stats.total_items);
        println!("  Successful: {}", stats.successful);
        println!("  Failed: {}", stats.failed);
        println!("  Throughput: {:.0} items/sec", throughput);
        println!("  Latencies: p50={}ms, p95={}ms, p99={}ms",
                 stats.p50_latency_ms, stats.p95_latency_ms, stats.p99_latency_ms);
        
        assert_eq!(stats.total_items, 10000);
    }
}
```

### 12:00-13:00 | LUNCH (1h)

### 13:00-17:00 | Implementation & Testing (4h)

**Milestones**
- 13:00-13:45: Core pool implementation (creation, work distribution)
- 13:45-14:30: Worker task processing (with retries)
- 14:30-15:15: Result collection & metrics
- 15:15-16:00: Testing (10k items, 50 workers, measure p95)
- 16:00-16:45: Performance tuning & edge cases
- 16:45-17:00: Documentation & code review

**Testing Checklist**
- [ ] Submit 100 items, verify all processed
- [ ] Submit 10,000 items, measure throughput
- [ ] Verify p50, p95, p99 latencies
- [ ] Test failure handling (some items fail, retry)
- [ ] Test timeout handling (skip items that timeout)
- [ ] Verify graceful shutdown
- [ ] Verify result ordering (optional)
- [ ] Measure CPU usage (should use all cores)

---

# FRIDAY AFTERNOON STRETCH GOALS (if time permits)

**Extend the pool with:**
1. **Result ordering**: Maintain order of results (queue order)
2. **Priority work**: High/medium/low priority items
3. **Work stealing**: Idle workers steal from busy workers
4. **Metrics dashboard**: Real-time throughput, latency, queue depth
5. **Health checks**: Worker heartbeat, restart dead workers
6. **Distributed pool**: Pool spans multiple machines (via channels)

---

# WEEKLY METRICS & SUCCESS CRITERIA

**Knowledge Goals**
- ✅ Understand actor model as thread replacement
- ✅ Design for 1000+ concurrent actors
- ✅ Identify and prevent deadlocks
- ✅ Use Arc, Mutex, RwLock patterns
- ✅ Implement thread-safe data structures
- ✅ Build production worker pool

**Performance Goals**
- ✅ Actor overhead < 1µs (vs. thread: 1ms)
- ✅ Message passing latency < 100ns
- ✅ Pool throughput: 1000 items/sec per worker
- ✅ Deadlock-free by design (lock ordering)
- ✅ No memory leaks (Arc guarantees)

**Capstone Assessment**
- ✅ 200+ lines of production code
- ✅ 10,000 items processed reliably
- ✅ 50 workers in parallel
- ✅ Latency tracking (p50, p95, p99)
- ✅ Failure handling & recovery
- ✅ Graceful shutdown

---

# INTEGRATION WITH PREVIOUS WEEKS

**Builds On**
- Week 1-9: Language foundations and async basics
- Week 10-11: Actor model and distributed systems

**Feeds Forward To**
- Week 20: Real-time systems (reduce GC pauses)
- Week 21: Network services (use pool for HTTP handling)
- Week 22: Data processing (use pool for MapReduce)

---

