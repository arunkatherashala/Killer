# Week 20 Weekly Schedule: Real-Time Systems
## 75 Hours | GC-Aware, Low-Latency Design

---

# WEEKLY OVERVIEW

**Monday**: GC Fundamentals & Measurement (15h)
**Tuesday**: Object Pooling & Memory Patterns (15h)
**Wednesday**: Latency Analysis & Optimization (15h)
**Thursday**: Jitter Elimination & Hard Real-Time (15h)
**Friday**: Capstone - Trading System (<500µs Latency) (15h)

**Time Allocation**
- Concepts & Theory: 25 hours (33%)
- Hands-on Exercises: 35 hours (47%)
- Capstone Project: 15 hours (20%)

---

# MONDAY: GC FUNDAMENTALS & MEASUREMENT (15 hours)

## 09:00-11:00 | Understanding Garbage Collection (2h)

**Concepts**
```
Current Killer GC: Mark-and-Sweep
├─ Mark phase: traverse live objects
├─ Sweep phase: reclaim unmarked memory
├─ Pause time: ~100ms (major problem!)
├─ Triggers: when heap nearly full
└─ Cost: unpredictable latency spikes

GC Impact on Real-Time:
  Request arrives at t=100ms
  GC pauses at t=100ms for 50ms
  Request processed at t=150ms
  Latency = 50ms (unacceptable for trading!)

Real-Time Requirement:
  ✓ All requests processed < 500µs
  ✓ No GC pauses during request
  ✓ Predictable latency (no 99th percentile outliers)
  ✓ No "GC hiccups"
```

**Problems**
```
20.1.1-20.1.10: GC Measurement

20.1.1:  Measure GC pause time
         - Allocate objects
         - Trigger GC
         - Measure pause duration
         - Log pause events

20.1.2:  Track heap statistics
         - Used memory before, after GC
         - Objects reclaimed
         - Efficiency ratio

20.1.3:  Monitor allocation rate
         - Track bytes allocated/sec
         - Predict next GC trigger
         - Warn if unsustainable

20.1.4:  Measure young vs. old gen (if supported)
         - Young generation collections
         - Old generation collections
         - Frequency of each type

20.1.5:  Latency histogram
         - Collect pause times
         - Sort into buckets (0-1ms, 1-10ms, 10-100ms, 100ms+)
         - Identify outliers

20.1.6:  Correlation: allocation vs. GC
         - Heavy allocation triggers GC
         - Measure allocation burst
         - See GC response

20.1.7:  Request latency with GC
         - Send request
         - If GC happens during, measure added latency
         - Compare with/without GC

20.1.8:  Memory fragmentation
         - Track free memory blocks
         - Largest contiguous block
         - Predict allocation failures

20.1.9:  Generational efficiency
         - Track object lifetime
         - Young objects usually short-lived
         - Measure young gen reclaim rate

20.1.10: Fullmark cost
         - Major collection: pause time
         - Compare to minor collection
         - Measure frequency
```

**Hands-on** (Exercise 1-5: GC Measurement)
```rust
pub struct GCMetrics {
    pause_times: Vec<u64>,  // milliseconds
    collections: usize,
    total_pause_time: u64,
}

impl GCMetrics {
    pub fn new() -> Self {
        GCMetrics {
            pause_times: Vec::new(),
            collections: 0,
            total_pause_time: 0,
        }
    }
    
    pub fn record_collection(&mut self, pause_ms: u64) {
        self.pause_times.push(pause_ms);
        self.collections += 1;
        self.total_pause_time += pause_ms;
    }
    
    pub fn avg_pause_ms(&self) -> f64 {
        if self.pause_times.is_empty() {
            return 0.0;
        }
        self.total_pause_time as f64 / self.pause_times.len() as f64
    }
    
    pub fn max_pause_ms(&self) -> u64 {
        *self.pause_times.iter().max().unwrap_or(&0)
    }
    
    pub fn percentile(&self, p: usize) -> u64 {
        if self.pause_times.is_empty() {
            return 0;
        }
        let mut sorted = self.pause_times.clone();
        sorted.sort();
        sorted[(sorted.len() * p) / 100]
    }
}

pub struct AllocationTracker {
    bytes_allocated: usize,
    allocation_count: usize,
    peak_allocation: usize,
}

impl AllocationTracker {
    pub fn record_allocation(&mut self, size: usize) {
        self.bytes_allocated += size;
        self.allocation_count += 1;
        self.peak_allocation = self.peak_allocation.max(self.bytes_allocated);
    }
    
    pub fn allocation_rate_mb_per_sec(&self, duration_sec: f64) -> f64 {
        let mb = self.bytes_allocated as f64 / (1024.0 * 1024.0);
        mb / duration_sec
    }
}

pub struct HeapMetrics {
    used_bytes: usize,
    total_bytes: usize,
    objects_alive: usize,
}

impl HeapMetrics {
    pub fn utilization_percent(&self) -> f64 {
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }
    
    pub fn avg_object_size_bytes(&self) -> f64 {
        if self.objects_alive == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.objects_alive as f64
    }
}

pub struct RequestLatencyWithGC {
    request_id: u64,
    start_time: std::time::Instant,
    gc_happened: bool,
    gc_pause_ms: u64,
    total_latency_ms: u64,
}

impl RequestLatencyWithGC {
    pub fn measure(request_id: u64) -> Self {
        RequestLatencyWithGC {
            request_id,
            start_time: std::time::Instant::now(),
            gc_happened: false,
            gc_pause_ms: 0,
            total_latency_ms: 0,
        }
    }
    
    pub fn finish(&mut self) {
        self.total_latency_ms = self.start_time.elapsed().as_millis() as u64;
    }
    
    pub fn latency_without_gc(&self) -> u64 {
        if self.gc_happened {
            self.total_latency_ms - self.gc_pause_ms
        } else {
            self.total_latency_ms
        }
    }
}
```

### 11:00-13:00 | Pause Time & Jitter (2h)

**Concepts**
- GC pause: time when all threads stop
- Jitter: variability in pause times
- P95 pause: 95% of pauses < X
- P99 pause: 99% of pauses < X

**Problems** (20.1.11-20.1.20)
```
20.1.11: Measure pause variability
         - Run 100 collections
         - Calculate std deviation
         - Low = predictable, High = unpredictable

20.1.12: Identify pause outliers
         - Pause times: 50ms, 52ms, 48ms, 200ms, 51ms...
         - 200ms is outlier (full GC?)
         - Separate into categories

20.1.13: Predict next pause
         - Track past pause times
         - Allocate before expected pause
         - Warn if large pause coming

20.1.14: Measure pause impact on throughput
         - 1000 requests, 5 pauses of 100ms each
         - Throughput drop = ?%

20.1.15: Track maximum pause (worst case)
         - Keep running max
         - Report p99, p99.9 pause time

20.1.16: Measure pause frequency
         - GC every 10 seconds? every 100?
         - Too frequent = slow system
         - Predict when next pause

20.1.17: Correlation: heap size vs. pause
         - Larger heap -> longer pauses?
         - Test with different heap sizes

20.1.18: Collection efficiency
         - Pause time / bytes reclaimed
         - Lower = more efficient

20.1.19: Young vs. full collection pause
         - Young pause: 10ms
         - Full pause: 100ms
         - Track separately

20.1.20: Pause budget tracking
         - Total allowed pause: 100ms/sec
         - Track cumulative pause time
         - Alert if exceeds budget
```

**Hands-on** (Exercise 6-10)
```rust
pub struct PauseAnalyzer {
    pauses: Vec<u64>,
}

impl PauseAnalyzer {
    pub fn std_deviation(&self) -> f64 {
        if self.pauses.len() < 2 {
            return 0.0;
        }
        
        let mean = self.pauses.iter().sum::<u64>() as f64 / self.pauses.len() as f64;
        let variance = self.pauses.iter()
            .map(|&p| {
                let diff = p as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / self.pauses.len() as f64;
        
        variance.sqrt()
    }
    
    pub fn outliers(&self, threshold_std_dev: f64) -> Vec<u64> {
        let mean = self.pauses.iter().sum::<u64>() as f64 / self.pauses.len() as f64;
        let std_dev = self.std_deviation();
        
        self.pauses.iter()
            .filter(|&&p| (p as f64 - mean).abs() > threshold_std_dev * std_dev)
            .copied()
            .collect()
    }
}

pub struct PauseBudget {
    total_allowed_ms_per_sec: u64,
    pause_times_this_sec: Vec<u64>,
    window_start: std::time::Instant,
}

impl PauseBudget {
    pub fn new(budget_ms: u64) -> Self {
        PauseBudget {
            total_allowed_ms_per_sec: budget_ms,
            pause_times_this_sec: Vec::new(),
            window_start: std::time::Instant::now(),
        }
    }
    
    pub fn record_pause(&mut self, pause_ms: u64) -> bool {
        // Reset window if > 1 second
        if self.window_start.elapsed().as_secs() > 0 {
            self.pause_times_this_sec.clear();
            self.window_start = std::time::Instant::now();
        }
        
        let total = self.pause_times_this_sec.iter().sum::<u64>() + pause_ms;
        let remaining = self.total_allowed_ms_per_sec;
        
        if total <= remaining {
            self.pause_times_this_sec.push(pause_ms);
            true
        } else {
            false  // Over budget!
        }
    }
    
    pub fn utilization_percent(&self) -> f64 {
        let total: u64 = self.pause_times_this_sec.iter().sum();
        (total as f64 / self.total_allowed_ms_per_sec as f64) * 100.0
    }
}

pub struct ThroughputMonitor {
    requests_before_gc: usize,
    gc_pauses: Vec<u64>,
    total_requests: usize,
}

impl ThroughputMonitor {
    pub fn throughput_drop_percent(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        
        let total_pause: u64 = self.gc_pauses.iter().sum();
        let total_time_ms = 1000;  // Assume 1 second test
        
        (total_pause as f64 / total_time_ms as f64) * 100.0
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Real-Time Constraints & Solutions (3h)

**Concepts**
- Hard real-time: miss deadline = failure (trading, aerospace)
- Soft real-time: miss some, but minimize (gaming)
- Firm real-time: late is useless (live video)
- GC incompatible with hard real-time

**Solutions**
- Avoid dynamic allocation (pre-allocate)
- Avoid GC pauses (use pools)
- Predictable timing (no surprise pauses)

**Problems** (20.1.21-20.1.30)
```
20.1.21: Define real-time constraints
         - Specify deadline (500µs for order)
         - Acceptable latency (p99 < deadline)

20.1.22: Measure if deadline met
         - Issue 1000 requests
         - Count missing deadline
         - Calculate success rate

20.1.23: Model GC impact
         - Pause every 10s? = missed every 10s
         - Not suitable for hard real-time

20.1.24: Worst-case latency
         - Identify slowest possible request
         - Full GC + processing + scheduling
         - Ensure still < deadline

20.1.25: Jitter budget
         - Allowed variance: p99 vs. p50
         - If p50=100µs, p99=500µs
         - Jitter = 400µs (high!)

20.1.26: Deadline miss analysis
         - Which requests miss deadline?
         - Correlated with GC?
         - Correlated with load?

20.1.27: Safety margin
         - Deadline = 500µs
         - Process in 400µs
         - Safety margin = 100µs (20%)

20.1.28: Admission control
         - Can we process this request in time?
         - If no, reject early
         - Prevents missed deadlines

20.1.29: Graceful degradation
         - Under load, drop low-priority requests
         - Ensure high-priority succeed

20.1.30: End-to-end latency breakdown
         - Receive: 10µs
         - Validate: 50µs
         - Process: 300µs
         - Send: 20µs
         - Total: 380µs < 500µs deadline ✓
```

**Hands-on** (Exercise 11-15)
```rust
pub struct RealTimeConstraint {
    deadline_us: u64,
}

impl RealTimeConstraint {
    pub fn check_met(&self, latency_us: u64) -> bool {
        latency_us < self.deadline_us
    }
}

pub struct PercentileLatency {
    latencies: Vec<u64>,  // microseconds
}

impl PercentileLatency {
    pub fn p50_us(&self) -> u64 {
        self.percentile(50)
    }
    
    pub fn p99_us(&self) -> u64 {
        self.percentile(99)
    }
    
    pub fn jitter_us(&self) -> u64 {
        self.p99_us() - self.p50_us()
    }
    
    fn percentile(&self, p: usize) -> u64 {
        let mut sorted = self.latencies.clone();
        sorted.sort();
        sorted[(sorted.len() * p) / 100]
    }
}

pub struct DeadlineChecker {
    deadline_us: u64,
    results: Vec<bool>,
}

impl DeadlineChecker {
    pub fn check(&mut self, latency_us: u64) {
        let met = latency_us < self.deadline_us;
        self.results.push(met);
    }
    
    pub fn success_rate(&self) -> f64 {
        let successes = self.results.iter().filter(|&&r| r).count();
        (successes as f64 / self.results.len() as f64) * 100.0
    }
}

pub struct LatencyBreakdown {
    receive_us: u64,
    validate_us: u64,
    process_us: u64,
    send_us: u64,
}

impl LatencyBreakdown {
    pub fn total_us(&self) -> u64 {
        self.receive_us + self.validate_us + self.process_us + self.send_us
    }
    
    pub fn bottleneck(&self) -> &str {
        let max = self.receive_us
            .max(self.validate_us)
            .max(self.process_us)
            .max(self.send_us);
        
        if max == self.process_us {
            "process"
        } else if max == self.validate_us {
            "validate"
        } else if max == self.receive_us {
            "receive"
        } else {
            "send"
        }
    }
}

pub struct AdmissionControl {
    deadline_us: u64,
    processing_time_us: u64,
}

impl AdmissionControl {
    pub fn can_process(&self) -> bool {
        self.processing_time_us < self.deadline_us
    }
    
    pub fn safety_margin_us(&self) -> u64 {
        if self.processing_time_us < self.deadline_us {
            self.deadline_us - self.processing_time_us
        } else {
            0
        }
    }
}
```

---

# TUESDAY: OBJECT POOLING & MEMORY PATTERNS (15 hours)

## 09:00-11:00 | Object Pooling Strategy (2h)

**Concepts**
```
Problem: GC happens when allocating new objects

Solution: Pre-allocate everything, reuse

Object Pool Pattern:
┌───────────────┐
│  Initializer  │
│  - Create 100 │
│    WorkItems  │
│  - Reset on   │
│    return     │
└───────────────┘
       ↓
    /   \
   V     V
[Idle]  [Idle] ... [Idle] (all pre-allocated)
   │     │           │
   └─────┼───────────┘
       Request pool.get()
         ↓
      [InUse]
       ↓
     Use and return
       ↓
     pool.return(item)
       ↓
     [Idle] (reset, ready for reuse)

Benefit: Zero allocation in hot path!
```

**Problems** (20.2.1-20.2.15)
```
20.2.1:  Create object pool
         - Pre-allocate 100 WorkItems
         - get() returns one item
         - return(item) puts back

20.2.2:  Pool with reset
         - On return, reset to initial state
         - Clear any transient data
         - Ready for reuse

20.2.3:  Pool statistics
         - Items in use
         - Items available
         - Total pool size
         - Reuse count

20.2.4:  Dynamic pool
         - Start with 10
         - Auto-grow if depleted
         - Max size = 100
         - Never deallocate

20.2.5:  Bounded pool (fixed size)
         - Max 50 items
         - Returns None if exhausted
         - Caller must wait or drop

20.2.6:  Exception handling in pool
         - Item used, error thrown
         - Still return to pool
         - Verify reset works

20.2.7:  Pool with factory
         - Custom creation function
         - Custom reset function
         - Different item types

20.2.8:  Nested pool
         - Pool of vectors
         - Vectors pre-allocated with capacity
         - Clear vector on return

20.2.9:  Pool reuse metrics
         - Track reuse count per item
         - Measure allocation savings
         - Compare to malloc/free

20.2.10: Aging pool items
         - Too many reuses -> allocate new
         - Prevents long-lived items
         - Refresh pool over time
         
... (20.2.11-20.2.15 continued)
```

**Hands-on** (Exercise 16-20: Object Pools)
```rust
pub struct WorkItem {
    id: u64,
    data: Vec<u8>,
    done: bool,
}

impl WorkItem {
    pub fn new(id: u64) -> Self {
        WorkItem {
            id,
            data: Vec::with_capacity(1024),
            done: false,
        }
    }
    
    pub fn reset(&mut self) {
        self.data.clear();
        self.done = false;
    }
}

pub struct ObjectPool<T> {
    available: Vec<T>,
    in_use: usize,
    reuse_count: usize,
}

impl<T: Clone> ObjectPool<T> {
    pub fn new(size: usize, template: T) -> Self {
        let mut available = Vec::with_capacity(size);
        for _ in 0..size {
            available.push(template.clone());
        }
        
        ObjectPool {
            available,
            in_use: 0,
            reuse_count: 0,
        }
    }
    
    pub fn get(&mut self) -> Option<T> {
        if let Some(item) = self.available.pop() {
            self.in_use += 1;
            self.reuse_count += 1;
            Some(item)
        } else {
            None
        }
    }
    
    pub fn return_item(&mut self, mut item: T, reset: impl Fn(&mut T)) {
        reset(&mut item);
        self.available.push(item);
        self.in_use = self.in_use.saturating_sub(1);
    }
    
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.available.len(), self.in_use, self.reuse_count)
    }
}

pub struct DynamicPool<T> {
    available: Vec<T>,
    in_use: usize,
    factory: fn() -> T,
    max_size: usize,
}

impl<T> DynamicPool<T> {
    pub fn new(initial: usize, max: usize, factory: fn() -> T) -> Self {
        let mut available = Vec::with_capacity(max);
        for _ in 0..initial {
            available.push(factory());
        }
        
        DynamicPool {
            available,
            in_use: 0,
            factory,
            max_size: max,
        }
    }
    
    pub fn get(&mut self) -> T {
        if let Some(item) = self.available.pop() {
            self.in_use += 1;
            item
        } else if self.available.len() + self.in_use < self.max_size {
            self.in_use += 1;
            (self.factory)()
        } else {
            // Pool full, still create but don't track (will delete)
            (self.factory)()
        }
    }
}

pub struct VectorPool {
    pool: ObjectPool<Vec<u8>>,
}

impl VectorPool {
    pub fn new(size: usize, capacity: usize) -> Self {
        let template = Vec::with_capacity(capacity);
        VectorPool {
            pool: ObjectPool::new(size, template),
        }
    }
    
    pub fn get(&mut self) -> Option<Vec<u8>> {
        self.pool.get()
    }
    
    pub fn return_vec(&mut self, vec: Vec<u8>) {
        self.pool.return_item(vec, |v| v.clear());
    }
}
```

### 11:00-13:00 | Arena Allocation (2h)

**Concepts**
```
Arena = Pre-allocated large block

Allocation:
  ptr += size  (just move pointer!)

Deallocation:
  Reset arena = free all at once

Benefit: O(1) allocation, O(1) deallocation
```

**Problems** (20.2.16-20.2.30)
```
20.2.16: Create arena allocator
         - Fixed size (1MB)
         - All allocations from arena
         - Track usage

20.2.17: Request-scoped arena
         - Create arena for request
         - Allocate during processing
         - Free arena at end
         - All autos freed

20.2.18: Arena statistics
         - Total capacity
         - Used bytes
         - Fragmentation
         - Peak usage

... (20.2.19-20.2.30 continued)
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Memory Layout & Optimization (3h)

**Concepts**
- Data structures impact GC pause time
- Cache locality improves performance
- Alignment and padding
- Struct vs. array layout

**Hands-on**: Structure optimization exercises

---

# WEDNESDAY & THURSDAY: LATENCY ANALYSIS & JITTER ELIMINATION (30 hours)

[Detailed schedule with exercises for measuring, profiling, analyzing, and eliminating latency sources]

---

# FRIDAY: CAPSTONE - TRADING SYSTEM (<500µs LATENCY) (15 hours)

## 09:00-12:00 | Design & Architecture (3h)

**Project Brief**
```
Build trading system that:
  ✓ Receives market data (price updates)
  ✓ Processes orders (<500µs)
  ✓ Executes trades
  ✓ Latency p99 < 500µs
  ✓ No GC pauses during hot path
  ✓ Real-time price quotes
  ✓ Measure full latency breakdown
```

**Architecture**
```
MarketData    Order Queue    Trading Engine    Execution
   Feed    →  (lock-free)   →  (pre-allocated) → (socket)
                                ├─ Price lookup (<1µs)
                                ├─ Risk check (<10µs)
                                ├─ Match order (<100µs)
                                └─ Send exec (<50µs)
                                Total: ~200µs
```

**Implementation** (200+ lines production code)

```rust
use std::time::Instant;

pub struct Order {
    id: u64,
    symbol: u32,
    quantity: u64,
    price_units: u64,
    buy: bool,
    timestamp_us: u64,
}

pub struct Trade {
    order_id: u64,
    executed_units: u64,
    executed_price_units: u64,
    latency_us: u64,
}

pub struct FixedPriceBook {
    bids: [u64; 1000],  // Pre-allocated
    asks: [u64; 1000],
    bid_count: usize,
    ask_count: usize,
}

impl FixedPriceBook {
    pub fn new() -> Self {
        FixedPriceBook {
            bids: [0; 1000],
            asks: [0; 1000],
            bid_count: 0,
            ask_count: 0,
        }
    }
    
    pub fn best_bid(&self) -> Option<u64> {
        if self.bid_count > 0 {
            Some(self.bids[self.bid_count - 1])
        } else {
            None
        }
    }
    
    pub fn best_ask(&self) -> Option<u64> {
        if self.ask_count > 0 {
            Some(self.asks[0])
        } else {
            None
        }
    }
}

pub struct TradingEngine {
    order_pool: Vec<Order>,
    trade_results: Vec<Trade>,
    price_book: FixedPriceBook,
    latency_tracker: Vec<u64>,
}

impl TradingEngine {
    pub fn new(pool_size: usize) -> Self {
        TradingEngine {
            order_pool: vec![
                Order {
                    id: 0,
                    symbol: 0,
                    quantity: 0,
                    price_units: 0,
                    buy: false,
                    timestamp_us: 0,
                };
                pool_size
            ],
            trade_results: Vec::with_capacity(10000),
            price_book: FixedPriceBook::new(),
            latency_tracker: Vec::new(),
        }
    }
    
    pub fn process_order(&mut self, order: Order) -> Trade {
        let start = Instant::now();
        
        // Price lookup (<1µs)
        let bid = self.price_book.best_bid();
        let ask = self.price_book.best_ask();
        
        // Risk check (<10µs)
        let allowed = true;  // Simplified
        
        // Match order (<100µs)
        let executed_units = order.quantity;
        let executed_price = if order.buy {
            ask.unwrap_or(0)
        } else {
            bid.unwrap_or(0)
        };
        
        // Create result
        let trade = Trade {
            order_id: order.id,
            executed_units,
            executed_price_units: executed_price,
            latency_us: start.elapsed().as_micros() as u64,
        };
        
        self.trade_results.push(trade.clone());
        self.latency_tracker.push(trade.latency_us);
        
        trade
    }
    
    pub fn latency_stats(&self) -> (u64, u64, u64) {
        if self.latency_tracker.is_empty() {
            return (0, 0, 0);
        }
        
        let mut sorted = self.latency_tracker.clone();
        sorted.sort();
        
        let p50 = sorted[sorted.len() / 2];
        let p95 = sorted[(sorted.len() * 95) / 100];
        let p99 = sorted[(sorted.len() * 99) / 100];
        
        (p50, p95, p99)
    }
    
    pub fn check_deadlines(&self, deadline_us: u64) -> f64 {
        let met = self.latency_tracker.iter()
            .filter(|&&lat| lat < deadline_us)
            .count();
        
        (met as f64 / self.latency_tracker.len() as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trading_latency() {
        let mut engine = TradingEngine::new(10000);
        
        // Process 1000 orders
        for i in 0..1000 {
            let order = Order {
                id: i as u64,
                symbol: 1,
                quantity: 100,
                price_units: 10000,
                buy: i % 2 == 0,
                timestamp_us: i as u64 * 1000,
            };
            
            engine.process_order(order);
        }
        
        let (p50, p95, p99) = engine.latency_stats();
        println!("Trading System Latency:");
        println!("  P50: {}µs", p50);
        println!("  P95: {}µs", p95);
        println!("  P99: {}µs", p99);
        
        let deadline_met = engine.check_deadlines(500);
        println!("  Deadline <500µs: {:.1}%", deadline_met);
        
        assert!(p99 < 500, "P99 latency must be < 500µs");
        assert!(deadline_met > 95.0, "95% of orders must meet deadline");
    }
}
```

### 12:00-13:00 | LUNCH (1h)

### 13:00-17:00 | Testing & Optimization (4h)

**Milestones**
- 13:00-13:45: Core trading engine (matching, execution)
- 13:45-14:30: Latency measurement (timestamp tracking)
- 14:30-15:15: Load testing (1000 orders)
- 15:15-16:00: Deadline verification (p99 < 500µs)
- 16:00-16:45: GC testing (ensure no pauses)
- 16:45-17:00: Documentation

**Testing**
- [ ] 1000 orders processed
- [ ] P99 latency < 500µs
- [ ] No GC pauses observed
- [ ] Price updates < 1µs
- [ ] Risk checks < 10µs
- [ ] All deadlines met

---

# WEEKLY METRICS & SUCCESS CRITERIA

**Knowledge Objectives**
- Understand GC impact on real-time
- Measure and analyze latency
- Apply object pooling / arena allocation
- Eliminate jitter and GC pauses
- Design hard real-time systems

**Performance Targets**
- Object pool: O(1) allocation
- Request processing: < 500µs
- P99 latency: predictable (low jitter)
- No GC pauses in hot path
- 1000+ orders/sec with deadline met

**Capstone Success**
- ✅ Trading system built
- ✅ All 1000 orders < 500µs p99
- ✅ No GC pauses during trading
- ✅ Real-time price quotes working
- ✅ Full latency breakdown documented

---

# INTEGRATION

**Builds On**: Week 19 (actor pools, concurrency)
**Feeds Forward To**: Week 21 (network services, handling request storms)

