# Week 9: Message Passing & Channels - Complete Learning Module
## Foundation for concurrent system communication
**Target: 150+ problems | ~450 hours | Expert Level**

---

## Module Overview

### Learning Objectives
By end of Week 9, you will:
- ✓ Understand channel types (MPSC, SPMC, MPMC)
- ✓ Implement safe message passing patterns
- ✓ Handle backpressure and bounded channels
- ✓ Design protocol-based communication
- ✓ Debug message ordering and delivery issues
- ✓ Optimize for latency and throughput

### Core Topics
1. **Channel Types** (30 problems)
2. **Message Ordering & Guarantees** (25 problems)
3. **Backpressure & Flow Control** (25 problems)
4. **Protocol Design** (30 problems)
5. **Error Handling & Resilience** (25 problems)
6. **Performance Optimization** (15 problems)

---

## CATEGORY 1: CHANNEL TYPES & BASICS (30 problems)

### 1.1 SPSC (Single-Producer, Single-Consumer)

**What it is:**
- One sender, one receiver
- No synchronization overhead
- Highest throughput for point-to-point
- Common in pipeline stages

**Problems:**
```
1.1.1: Basic message queue (string messages)
1.1.2: Bounded queue with capacity tracking
1.1.3: Multiple message types (enum variants)
1.1.4: Sender/receiver cloning (handle passing)
1.1.5: Channel closure detection
1.1.6: Graceful shutdown signaling
1.1.7: Message count statistics
1.1.8: Drain operation (get all pending)
1.1.9: Peek operation (inspect without consuming)
1.1.10: Timeout on receive
```

**Example Problems:**

```rust
// 1.1.1: SPSC Queue
Problem: Create a simple message queue
- Sender can send string "messages"
- Receiver can receive them in order
- Queue.send("msg") -> Result
- Queue.recv() -> Option<String>
- Guarantee: FIFO ordering
- Guarantee: All sent messages received (except dropped)

// 1.1.5: Channel Closure
Problem: Sender can signal "done" to receiver
- Receiver gets None when sender closed
- Multiple recv() after close also returns None
- Can detect "did sender close?" from receiver side
- Standard pattern: drop(sender) to close
```

### 1.2 MPSC (Multi-Producer, Single-Consumer)

**What it is:**
- Multiple senders, one receiver
- Receivers can distinguish senders (if needed)
- Most common pattern
- Used for fan-in aggregation

**Problems:**
```
1.2.1: Two senders, one receiver
1.2.2: Dynamic sender count (add senders at runtime)
1.2.3: Sender ID tracking (know which sender sent)
1.2.4: Round-robin fairness (all senders get turns)
1.2.5: Fair work distribution
1.2.6: Backpressure from one sender
1.2.7: Bounded channel with blocking sends
1.2.8: Dropped message detection
1.2.9: Out-of-order delivery issue (and fix)
1.2.10: Broadcast vs multicast patterns
```

### 1.3 SPMC (Single-Producer, Multi-Consumer)

**What it is:**
- One sender, multiple receivers
- Less common than MPSC
- Requires careful state management
- Used for broadcast/pub-sub patterns

**Problems:**
```
1.3.1: Basic broadcast to multiple receivers
1.3.2: Late subscriber gets queued messages (optional)
1.3.3: Message duplication detection
1.3.4: Selective subscription (filters)
1.3.5: Broadcast order guarantee
1.3.6: Slowest consumer determines throughput
1.3.7: Drop behavior on full queue
1.3.8: Receiver unsubscription
1.3.9: Broadcasting control messages
1.3.10: Broadcast with acknowledgments
```

### 1.4 MPMC (Multi-Producer, Multi-Consumer)

**What it is:**
- Multiple senders and receivers
- Most complex synchronization
- Can have message stealing (consumers race)
- Used in work queues

**Problems:**
```
1.4.1: Work queue with multiple workers
1.4.2: Fan-in + fan-out combined
1.4.3: Load balancing across receivers
1.4.4: Message ordering with MPMC
1.4.5: Ensuring all messages processed once
1.4.6: Graceful shutdown (all senders + receivers)
1.4.7: Deadlock prevention
1.4.8: Resource cleanup
```

---

## CATEGORY 2: MESSAGE ORDERING & GUARANTEES (25 problems)

### 2.1 FIFO Ordering

**Problems:**
```
2.1.1: Verify FIFO order (send 100 messages, verify order)
2.1.2: Multiple senders, order per-sender (not global)
2.1.3: Global ordering with timestamps
2.1.4: Sequence number verification
2.1.5: Out of order detection and recovery
```

### 2.2 Delivery Guarantees

**Problems:**
```
2.2.1: At-most-once delivery (drop duplicates)
2.2.2: At-least-once delivery (retry on failure)
2.2.3: Exactly-once delivery (hard problem!)
2.2.4: Acknowledgment pattern
2.2.5: Nack (negative acknowledgment) handling
2.2.6: Missing message detection
2.2.7: Dead letter queue for failed messages
```

### 2.3 Causal Ordering

**Problems:**
```
2.3.1: Message A must arrive before B (happens-before)
2.3.2: Chain of messages (preserve order)
2.3.3: Dependent message tracking
2.3.4: Causal history vectors (version vectors)
```

---

## CATEGORY 3: BACKPRESSURE & FLOW CONTROL (25 problems)

### 3.1 Bounded Channels

**What it is:**
- Fixed capacity channels
- Prevents unbounded memory growth
- Sender blocks when full
- Can detect "slow consumer"

**Problems:**
```
3.1.1: Basic bounded queue (send blocks if full)
3.1.2: Capacity tracking (how many slots used)
3.1.3: is_full() predicate
3.1.4: Timeout on send (waiting too long)
3.1.5: Try_send() non-blocking variant
3.1.6: Closed channel detection on send
3.1.7: Graceful drain (flush remaining)
3.1.8: Resize channel capacity (problem: need no-copy)
```

### 3.2 Slow Consumer Patterns

**Problems:**
```
3.2.1: Detect slow consumer (throughput metrics)
3.2.2: Multiple producers, one slow consumer
3.2.3: Reject low-priority messages (make room for high)
3.2.4: Drop oldest messages (FIFO->circular)
3.2.5: Callback when backpressure occurs
3.2.6: Adaptive batching (larger batches when slow)
3.2.7: Pipeline stall detection
```

### 3.3 Prioritization Under Backpressure

**Problems:**
```
3.3.1: Priority channel (high-priority goes first)
3.3.2: Mixed priorities FIFO vs priority
3.3.3: Starvation prevention (low-priority gets scheduled)
3.3.4: Dynamic priority adjustment
3.3.5: Priority inversion (high waiting for low)
```

---

## CATEGORY 4: PROTOCOL DESIGN (30 problems)

### 4.1 Request-Response Pattern

**Problems:**
```
4.1.1: Simple request-response
4.1.2: Correlation ID (matching responses to requests)
4.1.3: Timeout on response wait
4.1.4: Multiple in-flight requests
4.1.5: Request cancellation (receiver ignores late cancel)
4.1.6: Response ordering (might arrive out of order)
4.1.7: Pipelined requests (send multiple, collect responses)
4.1.8: Rate limiting requests
4.1.9: Circuit breaker (too many timeouts = fail fast)
4.1.10: Retry logic with exponential backoff
```

### 4.2 Publish-Subscribe Pattern

**Problems:**
```
4.2.1: Basic pub-sub (no message loss if subscribed first)
4.2.2: Late subscriber doesn't get old messages
4.2.3: Replay: late subscriber can catch up
4.2.4: Topic-based filtering (only some messages)
4.2.5: Broadcast vs multicast (all vs subset)
4.2.6: Subscriber timeout detection
4.2.7: Dynamic subscription/unsubscription
4.2.8: Message history maintenance
4.2.9: Backpressure in pub-sub (slow subscriber)
4.2.10: Durable subscriptions (survives disconnect)
```

### 4.3 Pipeline Pattern

**Problems:**
```
4.3.1: Linear pipeline (A -> B -> C)
4.3.2: Throughput bottleneck identification
4.3.3: Scaling bottleneck stage (multiple workers)
4.3.4: Heartbeat between stages
4.3.5: Early termination propagation
4.3.6: Deadlock detection in pipeline
4.3.7: Load balancing across stages
4.3.8: Message ordering through pipeline
4.3.9: Pipeline fork/join patterns
4.3.10: Graceful pipeline shutdown
```

### 4.4 Fan-Out Patterns

**Problems:**
```
4.4.1: One input broadcasts to many outputs
4.4.2: Reorder messages after distribution
4.4.3: Partial failures (one output fails)
4.4.4: Aggregation (collect responses)
4.4.5: Quorum-based completion
```

---

## CATEGORY 5: ERROR HANDLING & RESILIENCE (25 problems)

### 5.1 Failure Modes

**Problems:**
```
5.1.1: Sender dropped/crashed (receiver detects)
5.1.2: Receiver dropped/crashed (sender's option to fail)
5.1.3: Channel poison (panic in message handler)
5.1.4: Message corruption (detect with checksum)
5.1.5: Timeout-based failure detection
5.1.6: Watchdog timer for unresponsive receiver
5.1.7: Graceful degradation under partial failure
```

### 5.2 Recovery Patterns

**Problems:**
```
5.2.1: Restart failed component (resend lost messages)
5.2.2: Replay from log
5.2.3: Checkpoint recovery (last known good state)
5.2.4: Alternative routing (backup path)
5.2.5: Circuit breaker (stop sending to failed service)
5.2.6: Bulkhead isolation (failure doesn't cascade)
5.2.7: Graceful shutdown (drain queue before close)
```

### 5.3 Debugging & Observability

**Problems:**
```
5.3.1: Message tracing (log every message)
5.3.2: Latency measurement (how long in channel?)
5.3.3: Throughput metrics (msgs/sec)
5.3.4: Queue depth monitoring
5.3.5: Message loss detection
5.3.6: Deadlock detection (all stuck)
5.3.7: Performance bottleneck identification
```

---

## CATEGORY 6: PERFORMANCE OPTIMIZATION (15 problems)

### 6.1 Throughput Optimization

**Problems:**
```
6.1.1: Batching messages (send 10 at once)
6.1.2: Lock-free queue (if possible)
6.1.3: Producer-consumer separation
6.1.4: Zero-copy message passing
6.1.5: Pre-allocated buffers
```

### 6.2 Latency Optimization

**Problems:**
```
6.2.1: Minimize lock contention
6.2.2: Dedicated receiver thread priority
6.2.3: Busy-wait vs sleep tradeoff
6.2.4: Memory locality (cache optimization)
6.2.5: Buffer reuse (reduce allocations)
```

### 6.3 Resource Efficiency

**Problems:**
```
6.3.1: Memory usage with large messages
6.3.2: Channel growth unbounded vs bounded
6.3.3: CPU usage (spinning vs sleeping)
6.3.4: File descriptor usage (sockets)
6.3.5: Connection pooling
```

---

## Learning Progression

### Daily Structure (Week 9)

**Monday: Channel Fundamentals** (30 hours)
- SPSC channels: send/receive, closure
- MPSC channels: multiple senders
- SPMC pattern: broadcast basics
- Problems 1.1.1-1.1.10, 1.2.1-1.2.5

**Tuesday: Advanced Channels** (30 hours)
- MPMC work queues
- Bounded channels and backpressure
- Channel statistics and monitoring
- Problems 1.2.6-1.2.10, 1.3.1-1.3.5, 1.4.1-1.4.4

**Wednesday: Guarantees & Ordering** (30 hours)
- FIFO ordering verification
- Delivery guarantees patterns
- Sequence numbers and tracking
- Problems 2.1.1-2.1.5, 2.2.1-2.2.4

**Thursday: Protocols & Patterns** (30 hours)
- Request-response with correlation IDs
- Publish-subscribe mechanics
- Pipeline patterns
- Problems 4.1.1-4.1.5, 4.2.1-4.2.5

**Friday: Error Handling & Performance** (30 hours)
- Failure detection and recovery
- Message tracing and observability
- Throughput and latency optimization
- Problems 5.1.1-5.1.4, 6.1.1-6.1.3
- Capstone: Multi-stage pipeline with error handling

---

## Example Problems with Solutions

### Problem 1.2.3: Sender ID Tracking

**Problem Statement:**
```
Implement MPSC channel where receiver can identify which sender sent a message

Requirements:
1. At least 3 senders
2. Each sender sends labeled messages
3. Receiver receives (sender_id, message) tuples
4. Verify all messages from sender A arrive in order
5. Messages from different senders can interleave
6. Detect message loss per-sender
```

**Solution Approach:**
```rust
// Wrapper type
struct TrackedMessage {
    sender_id: usize,
    content: String,
    sequence: u64,  // Per-sender sequence number
}

// Implementation:
// - Each sender tracks its own sequence number
// - Receiver unpacks (sender_id, message)
// - Verification: re-sequence and check order
```

### Problem 2.1.2: Multiple Senders, Per-Sender FIFO

**Problem Statement:**
```
With N senders -> 1 receiver, ensure each sender's messages 
arrive in order (but not necessarily globally ordered)

Implementation: timestamps or sequence numbers per sender
Verification: reconstruct per-sender streams
```

### Problem 3.1.3: is_full() Predicate

**Problem Statement:**
```
Implement is_full() that detects when bounded channel is at capacity

Pattern: Check if next send would block
Implementation: size >= capacity
Use: makes decision "should I wait or reject?"
```

### Problem 4.1.2: Correlation ID Matching

**Problem Statement:**
```
Request-response where multiple requests in-flight:
1. Send request with correlation_id
2. Receive response with matching correlation_id
3. Match them despite out-of-order arrival
4. Timeout if response never arrives
5. Cancel request (receiver ignores)
```

**Implementation:**
```rust
// Request
struct Request {
    id: u64,  // Correlation ID
    command: String,
}

// Response
struct Response {
    id: u64,  // Matches request id
    result: String,
}

// Pattern:
// - Sender stores request with id in map
// - Receives response with id
// - Look up request from map
// - Match them
```

### Problem 5.1.1: Sender Dropped Detection

**Problem Statement:**
```
Main thread spawns sender thread, sender exits unexpectedly
Receiver should detect this (getting None on recv)

Patterns:
- try_recv() returns error vs success
- Iteration ends when sender drops
- Graceful shutdown vs unexpected exit
```

---

## Key Concepts Summary

### Channel Types
| Type | Senders | Receivers | Use Case |
|------|---------|-----------|----------|
| SPSC | 1 | 1 | Pipeline stage |
| MPSC | N | 1 | Fan-in aggregator |
| SPMC | 1 | N | Broadcast |
| MPMC | N | N | Work queue |

### Guarantees
- **FIFO:** Order preserved for single sender
- **At-most-once:** No duplicates
- **At-least-once:** May have duplicates (requires dedup)
- **Exactly-once:** Gold standard, hard to achieve

### Patterns
1. **Request-Response:** Correlation ID matching
2. **Pub-Sub:** Topic filtering + late join
3. **Pipeline:** Stage composition
4. **Fan-out:** Broadcast + aggregation
5. **Work Distribution:** MPMC with fair scheduling

---

## Integration with Other Concepts

### With Week 8 (Async/Await)
- Async senders/receivers
- `select!` on multiple channels
- Channel integration with `tokio::mpsc`

### With Week 10 (Actor Model)
- Actors as channel endpoints
- Message handlers
- Supervised failure recovery

### With Week 11 (Distributed Systems)
- Network channels (TCP, UDP)
- RPC over channels
- Message serialization

---

## Assessment Criteria

### By End of Week 9
- [ ] Implement all 4 channel types
- [ ] Solve 150+ problems
- [ ] Design custom protocols
- [ ] Debug message ordering
- [ ] Optimize for latency/throughput
- [ ] Handle failures gracefully

**Mastery = Deep understanding of message-based concurrency**

---

## Further Reading

### Papers
- "Communicating Sequential Processes" - Hoare
- "The Go Memory Model" - concurrency with channels
- "Actor Systems" - message delivery semantics

### Documentation
- Rust `std::sync::mpsc` guide
- Tokio channels documentation
- Protocol buffer message design

### Tools
- `crossbeam` channels (better performance)
- `flume` channels (alternative MPMC)
- Message queue systems (RabbitMQ, Kafka)
