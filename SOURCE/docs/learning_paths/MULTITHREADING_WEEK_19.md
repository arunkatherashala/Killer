# Multi-Threaded Applications (Week 19)
## Overview, Learning Outcomes, and Key Concepts

---

# OVERVIEW

**Weeks 1-18** gave you the foundation: language fundamentals, async patterns, actors, and distributed systems.

**Week 19** answers: "How do I build systems that handle massive concurrency?"

**The Challenge**: Single-threaded event loop hitting limits
- 1 CPU core used (others idle)
- Can't scale beyond single machine
- No native thread support in current Killer

**The Solution**: Multi-threaded patterns without native threads
- Actor pools (100s-1000s concurrent)
- Distributed work across logical cores
- Fault tolerance and supervision
- Production-grade system design

---

# LEARNING OUTCOMES

After Week 19, you will:

**Understand**
- [ ] Why actors work as "threads without the complexity"
- [ ] How to design work distribution for high concurrency
- [ ] Thread-safe data structure patterns (Arc, Mutex, RwLock)
- [ ] Deadlock causes and prevention techniques
- [ ] Synchronization primitives (barriers, conditions, semaphores)

**Build**
- [ ] Actor pools with 10-1000 concurrent workers
- [ ] Load distribution (round-robin, least-busy, hash-based)
- [ ] Race condition detection and fixing
- [ ] Deadlock-free systems using lock ordering
- [ ] Production worker pool with metrics

**Measure**
- [ ] Actor throughput (items/sec)
- [ ] Message latency (p50, p95, p99)
- [ ] Lock contention and CPU usage
- [ ] Graceful degradation under load

---

# KEY CONCEPTS

## 1. Actor Model as Thread Replacement

### Problem with Native Threads
```
Thread costs:
  ├─ OS context switch: ~1ms (expensive)
  ├─ Stack allocation: 2MB
  ├─ Shared state: 💣 race conditions
  ├─ Synchronization: 💣 deadlocks
  └─ Debugging: 💔 nearly impossible
```

### Actor Model Advantages
```
Actor benefits:
  ├─ No shared state (isolation)
  ├─ Sequential message processing (ordering)
  ├─ Supervision (automatic restart)
  ├─ Scalability (1000s on single machine)
  └─ Predictable latency
```

## 2. Scaling from 1 to 1000 Actors

```
Scale 1-10:       Simple pool (fixed size)
                  ├─ Create N actors upfront
                  ├─ Round-robin distribute work
                  └─ Low overhead

Scale 10-100:     Dynamic pool (auto-sizing)
                  ├─ Start with 10 actors
                  ├─ Add if queue fills
                  ├─ Remove if idle
                  └─ Adapts to load

Scale 100-1000:   Work stealing (load balance)
                  ├─ Each actor has own queue
                  ├─ Idle actors steal from busy
                  ├─ Reduces queue depth variance
                  └─ Auto-load balancing
```

## 3. Thread-Safe Data Structures

### Arc (Atomic Reference Count)
- **Use**: Multiple owners of same data
- **Cost**: ~10ns per clone (just increment counter)
- **Pattern**: `Arc<Mutex<T>>` for shared mutable state

### Mutex (Mutual Exclusion)
- **Use**: Single mutable accessor at a time
- **Cost**: ~100ns lock/unlock
- **Pattern**: Always release ASAP (minimize hold time)

### RwLock (Read-Write Lock)
- **Use**: Multiple readers, single writer
- **Cost**: ~50ns per read, ~150ns per write
- **Advantage**: Better for read-heavy (cache reads)

### Atomic (Lock-free)
- **Use**: Simple counters, flags
- **Cost**: ~5ns no lock
- **Pattern**: For non-blocking operations

## 4. Deadlock Prevention

### Circular Wait = Deadlock
```
Thread A: holds L1, waits for L2
Thread B: holds L2, waits for L1
Result: DEADLOCK (both wait forever)
```

### Fix: Lock Ordering
```
Assign levels: L1 < L2 < L3

All threads must:
  1. Acquire L1 before L2
  2. Acquire L2 before L3
  3. Never acquire lower after higher

Proof: No circular wait possible!
  → Threads ordered by lock level
  → Can't form cycle
```

## 5. Synchronization Primitives

| Primitive | Use | Latency |
|-----------|-----|---------|
| **Barrier** | Wait for N threads | ~200ns |
| **Condvar** | Wait for event | ~500ns |
| **Semaphore** | Limit concurrent access | ~100ns |
| **Once** | Run code once | ~5ns (cached) |
| **RwLock** | Readers + writer | ~100ns |

---

# CURRICULUM STRUCTURE

## Week Overview

```
Monday (15h):    Foundations
                ├─ Actors as alternatives to threads
                ├─ Message passing patterns
                └─ Lifecycle and supervision

Tuesday (15h):   Scaling
                ├─ Actor pools (fixed, dynamic)
                ├─ Load distribution strategies
                └─ Backpressure handling

Wednesday (15h): Synchronization
                ├─ Arc, Mutex, RwLock patterns
                ├─ Channels and work queues
                └─ Advanced sync primitives

Thursday (15h):  Deadlock Prevention
                ├─ Lock ordering protocols
                ├─ Timeout and recovery
                └─ Detection and debugging

Friday (15h):    Capstone
                ├─ Multi-node worker pool
                ├─ 50 worker actors
                ├─ 10,000 work items
                └─ Production metrics
```

## Problem Categories

**19.1: Actor Foundations** (30 problems)
- Basic actor patterns (echo, counter accumulator)
- Message passing (typed messages, routing)
- Lifecycle and supervision (restart, health checks)

**19.2: Scaling** (30 problems)
- Fixed/dynamic pool creation
- Load distribution strategies
- Backpressure and flow control

**19.3: Synchronization** (20 problems)
- Arc<Mutex<>> patterns
- RwLock for cache
- Channels and work queues
- Advanced primitives (barrier, condvar)

**19.4: Deadlock Prevention** (20 problems)
- Lock ordering enforcement
- Timeout-based recovery
- Detection algorithms

---

# TECHNICAL DEPTH

## Architecture Pattern: Worker Pool

```
Request Handler     Work Queue         Worker Pool
    │
    ├─ Submit(Work1) ─→ [Work1] ─→ Worker1 (processing...)
    │
    ├─ Submit(Work2) ─→ [Work2] ─→ Worker2 (processing...)
    │                  [Work3]
    └─ Wait Results  ← [Work4] ←─ Worker3 (idle)
                      [Work5]     Worker4
                      [Work6]     ...Worker50
                         ↓
                    Backpressure:
                    Queue full? Reject/Wait
```

## Performance Targets

```
Actor overhead:        ~1µs per message (vs. 1ms threads)
Throughput goal:       1000 items/sec per worker
P95 latency target:    <10ms (including queue wait)
P99 latency target:    <100ms (no outliers)
Scale target:          10,000 items across 50 actors
GC pause impact:       <100ms total job time (1% overhead)
```

## Failure Modes & Recovery

```
Transient Failure:     Random packet loss, temporary overload
Recovery:              Automatic retry (same actor)
Max attempts:          3 retries with exponential backoff

Permanent Failure:     Actor consistently errors
Recovery:              Mark failed, escalate to supervisor
Restart strategy:      1s, 2s, 4s, 8s, 16s backoff

Timeout:              Actor unresponsive > 5s
Recovery:             Kill actor, restart fresh
Monitor:              Heartbeat every 1s
```

---

# INTEGRATION POINTS

## Builds On (Weeks Before)

- **Week 1-9**: Core language + async foundations
- **Week 10**: Actor model basics (message passing, supervision)
- **Week 11**: Distributed systems (consensus, routing)

## Enables (Weeks After)

- **Week 20**: Real-time systems (GC pause optimization)
- **Week 21**: Network services (HTTP handling via pools)
- **Week 22**: Data processing (MapReduce over pools)

---

# RESOURCE REQUIREMENTS

## Hardware

- **Minimum**: 4-core CPU (for visible parallelism)
- **Recommended**: 8+ cores (better load distribution)
- **Memory**: 1GB+ (50 actors × small stack each)

## Time Budget

- **Theory**: 25 hours (concepts, patterns, tradeoffs)
- **Hands-on**: 35 hours (implementation, testing)
- **Capstone**: 15 hours (worker pool system)

## Tools

- Killer compiler (v2.1+)
- Rust stdlib (for sync primitives reference)
- Performance profiler (measure latencies)

---

# ASSESSMENT CRITERIA

## Knowledge (End of Week)

- [ ] Explain actor model vs. shared-memory concurrency
- [ ] Design pool for given load (throughput requirements)
- [ ] Identify race conditions and fix with synchronization
- [ ] Prevent deadlocks using lock ordering
- [ ] Measure latency percentiles and optimize

## Skills (Capstone)

- [ ] Implement 50-actor pool from scratch
- [ ] Process 10,000 work items reliably
- [ ] Handle failures (transient, permanent, timeout)
- [ ] Measure p50/p95/p99 latencies
- [ ] Achieve target throughput (1000 items/sec)

## Production Readiness

- [ ] Graceful shutdown (drain queue)
- [ ] Health monitoring (actor heartbeat)
- [ ] Metrics collection (throughput, latency, errors)
- [ ] Observability (logs, traces)
- [ ] Load testing (under sustained load)

---

# QUICK START

## Hello Actors

```rust
// 1. Define an actor
pub struct EchoActor;

impl EchoActor {
    pub fn handle(&mut self, msg: String) -> String {
        format!("Echo: {}", msg)
    }
}

// 2. Create pool
let mut pool = Vec::new();
for _ in 0..10 {
    pool.push(EchoActor);
}

// 3. Send work
let result = pool[0].handle("hello".to_string());

// 4. Scale
for i in 0..10000 {
    let actor_idx = i % pool.len();
    pool[actor_idx].handle(format!("work-{}", i));
}
```

## Common Patterns

**Pattern 1: Round-Robin Distribution**
```
worker_idx = item_count % num_workers
→ Balanced across workers
```

**Pattern 2: Least-Busy Scheduling**
```
worker_idx = queue_depths.min()
→ Always pick least loaded
```

**Pattern 3: Hash-Based Affinity**
```
worker_idx = hash(key) % num_workers
→ Same key always to same worker
```

---

# SUMMARY

**Problem**: How to build concurrent systems without deadlocks/races?
**Solution**: Actor model + work pools + careful synchronization

**Key Insight**: State isolation (each actor owns its state) makes reasoning about concurrency tractable. Add message passing for communication, add pools for scale, add supervision for reliability. You get thread-like parallelism without thread nightmares.

**Week 19 is the bridge** from single-threaded to truly concurrent systems.

