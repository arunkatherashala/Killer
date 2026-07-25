# Real-Time Systems Reference & Overview
## Week 20: GC-Aware, Low-Latency Design

---

# OVERVIEW

**Problem**: Killer's GC pauses (100ms+) make it unsuitable for real-time systems

**Solution**: Eliminate allocation in hot path via pooling, arena allocation, and jitter elimination

**Learning Outcomes**:
- Measure GC impact on latency
- Apply object pooling patterns
- Use arena allocation
- Analyze jitter and eliminate outliers
- Build sub-500µs trading system

---

# QUICK REFERENCE

## Key Metrics

| Metric | Target | Killer Current | Status |
|--------|--------|---|---|
| GC Pause | <5ms | 100ms | ❌ Need pooling |
| Request p99 latency | <500µs | varies | 🟡 Depends on pool |
| Allocation/request | 0 bytes | varies | ✅ Pool achieves |
| Jitter (p99-p50) | <100µs | varies | 🟡 Need tune |

## Patterns

**Object Pool**: Pre-allocate → reuse → reset
**Arena**: Single large allocation, pointer bump
**Latency breakdown**: Measure each stage separately
**Deadline checking**: Verify all requests meet deadline

---

# DETAILED REFERENCE

## 1. Garbage Collection Impact

### Mark-and-Sweep GC
```
Timeline:
  t=0ms: Request arrives
  t=0-50ms: Application runs
  t=50ms: Heap full, GC triggered
  t=50-150ms: GC pause (mark + sweep)
  t=150-250ms: Processing resumes
  ────────────────────────────────
  Latency = 200ms (due to GC!)
```

### Metrics
```
Pause time: max([all GC pauses])
P99 pause: 99th percentile pause
Jitter = P99 - P50
Allocation rate: bytes/sec

Example:
  Pauses: 49.5ms, 49.7ms, 148.2ms, 50.1ms, 49.8ms
  P50 = 49.8ms
  P99 = 148.2ms
  Jitter = 98.4ms (very high!)
```

## 2. Object Pooling

### Pattern

```rust
// Initialize: create pool once
pool = ObjectPool::new(100, WorkItem::new());

// Hot path: get from pool (no allocation!)
item = pool.get();       // O(1), no allocation
do_work(item);
pool.return(item);       // Reset and return

// Benefit: Zero garbage created
```

### When to Pool

| Item | Pool | Why |
|------|------|-----|
| Orders in trading system | ✅ | Many allocated/deallocated |
| HTTP request objects | ✅ | Thousands/sec |
| Fixed-size buffers | ✅ | Exact size known |
| Large objects | ✅ | Expensive to allocate |
| Small structs (< 16 bytes) | ❌ | Stack allocation better |

## 3. Arena Allocation

### Pattern

```
Arena = 1MB block
          
┌─────────────────────────────────────────┐
│  [used]      [used]     [free]          │
│    ↑           ↑                        │
│  item1      item2       ptr → here      │
└─────────────────────────────────────────┘

Allocation: ptr += size (O(1)!)
Deallocation: ptr = start (reset all)
```

### When to Use

- Request-scoped allocation (all freed at once)
- Phase-based processing (multiple phases, free at phase end)
- Known total size upfront
- No individual item deallocation needed

## 4. Latency Measurement

### Breakdown

```
Request latency = receive + validate + process + send
                = 10µs + 50µs + 300µs + 40µs
                = 400µs

Latency histogram:
  0-100µs:   5 requests
  100-200µs: 10 requests
  200-300µs: 200 requests
  300-400µs: 500 requests
  400-500µs: 280 requests
  500+µs:    5 requests (deadline misses!)

Percentiles:
  P50 (median): 350µs
  P95: 490µs
  P99: 505µs (5 over deadline!)
```

### Measurement Best Practices

```rust
// ✅ GOOD: timestamp at start/end
let start = Instant::now();
do_work();
let latency = start.elapsed();

// ❌ BAD: measuring measurement overhead
let before = now();
let after = now();
let overhead = after - before;  // includes measurement!

// ✅ GOOD: batch measurement at end (reduce overhead)
let mut latencies = Vec::new();
for i in 0..1000 {
    let start = Instant::now();
    process_order(orders[i]);
    latencies.push(start.elapsed());
}
// Analyze latencies (not in hot path)
```

## 5. Jitter Elimination

### Sources of Jitter

| Source | Cause | Solution |
|--------|-------|----------|
| GC pauses | Dynamic allocation | Pool everything |
| Cache misses | Poor memory layout | Align data ` |
| Context switches | Too many threads | Actor pool (not OS threads) |
| Lock contention | Shared state | Minimize locking |
| Page faults | Large allocations | Pre-fault pages |

### Example: Reducing Jitter

```
Before pooling:
  Latencies: 100µs, 110µs, 150µs (GC hiccup), 105µs, 120µs
  Jitter (p99-p50) = 40µs

After pooling:
  Latencies: 100µs, 105µs, 102µs, 104µs, 101µs
  Jitter (p99-p50) = 5µs (8x improvement!)
```

## 6. Real-Time Deadline Checking

### Hard Deadline

```
Definition: Request must complete within deadline
Checking: Verify all requests < deadline

Example:
  Deadline = 500µs
  Results: [450µs, 480µs, 520µs, 490µs]
  Failures = 1 (520µs exceeds deadline)
  Success rate = 75% ❌ (not good enough!)
```

### Soft Deadline with SLA

```
Definition: 99% of requests must meet deadline

Example:
  Deadline = 500µs, SLA = 99%
  1000 requests, p99 = 499µs ✅ (passes SLA)
  1000 requests, p99 = 510µs ❌ (fails SLA)
```

## 7. Lock-Free & Atomic Operations

```rust
// ✅ Lock-free (no pause on contention)
atomic.fetch_add(1, Ordering::Relaxed);

// ❌ Blocking (can pause)
let mut v = mutex.lock().unwrap();
*v += 1;
```

## 8. Trading System Example

### Architecture

```
Order Input (0-5µs)
    ↓
Price Lookup (< 1µs, pre-loaded)
    ↓
Risk Check (< 10µs, thresholds in cache)
    ↓
Order Matching (< 100µs, in-memory matching)
    ↓  
Execution (< 50µs, send to socket)
    ↓
Output (Total: ~160-200µs)

Remaining budget: 500µs - 200µs = 300µs safety margin
```

---

# PROBLEM BANK OVERVIEW (100 Problems)

**Category 20.1: GC Metrics & Measurement (30 problems)**
- Pause time measurement (20.1.1-10)
- Real-time constraints (20.1.11-20)
- Deadline checking (20.1.21-30)

**Category 20.2: Pooling & Allocation (30 problems)**
- Object pooling (20.2.1-15)
- Arena allocation (20.2.16-25)
- Memory optimization (20.2.26-30)

**Category 20.3: Latency Analysis (25 problems)**
- Breakdown analysis (20.3.1-10)
- Percentile tracking (20.3.11-20)
- Jitter measurement (20.3.21-25)

**Category 20.4: Jitter Elimination (15 problems)**
- Cache optimization (20.4.1-5)
- CPU affinity (20.4.6-10)
- Predictable latency (20.4.11-15)

---

# WEEK 20 PROBLEM BANK (100 Problems)

[Problems 20.1.1 through 20.4.15 follow similar format to Week 19]

Key problem areas:
- GC pause measurement
- Object pooling implementation
- Latency tracking and analysis
- Deadline verification
- Jitter elimination techniques
- Real-time system design
- Trading system implementation

---

# SUCCESS CRITERIA

**Knowledge**: Understand GC impact, pooling patterns, deadline design
**Skills**: Measure latencies, implement pools, analyze jitter
**Capstone**: Trading system with p99 < 500µs, no GC pauses

