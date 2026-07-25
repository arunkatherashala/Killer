# Week 9: Message Passing & Channels - Weekly Schedule
## Detailed daily breakdown for 150+ problems mastery

---

## WEEK 9 OVERVIEW

**Total Hours:** 75 hours (450 hours for full mastery across Weeks 9-11)
**Problems:** 150+ (comprehensive message passing patterns)
**Key Focus:** Channel types, message protocols, error handling

### Weekly Allocation
- **Lectures:** 10 hours
- **Coding:** 25 hours
- **Problems:** 20 hours
- **Projects:** 15 hours
- **Review:** 5 hours

---

## MONDAY: CHANNEL FUNDAMENTALS (15 hours)

### Morning (4.5 hours) - Lectures & Concepts

```
09:00-09:30: Lecture - Why Message Passing? (30 min)
  Topics:
  - Alternative to shared memory (sync/mutex)
  - Safety by design
  - Composable concurrency
  - Inter-process communication
  - From CSP (Communicating Sequential Processes)
  
09:30-10:00: Activity - Compare Patterns (30 min)
  Shared Memory vs Message Passing:
  - Race conditions in shared memory
  - No races with message passing
  - Trade-off: synchronization vs complexity
  Submit: 3 examples of each pattern
  
10:00-10:30: Lecture - SPSC Channels (30 min)
  What: Single Producer, Single Consumer
  Properties:
  - Highest throughput (least sync overhead)
  - Strict FIFO ordering
  - No sender identification needed
  - Used: pipeline stages
  
10:30-11:00: Break
11:00-11:30: Lecture - MPSC Channels (30 min)
  What: Multi-Producer, Single Consumer
  Properties:
  - Can identify senders or not
  - Fair scheduling between senders
  - Fan-in pattern (aggregation)
  
11:30-12:00: Lecture - SPMC Channels (30 min)
  What: Single Producer, Multi-Consumer (broadcast)
  Properties:
  - All subscribers get all messages
  - Late subscribers miss old messages
  - Used: pub-sub, notifications
  
12:00-12:30: Lecture - MPMC Channels (30 min)
  What: Multi-Producer, Multi-Consumer (work queue)
  Properties:
  - Most complex synchronization
  - Messages processed once (by one consumer)
  - Load balancing
  - Used: worker pools, job queues
```

### Afternoon (4.5 hours) - Guided Exercises

```
13:00-13:45: Guided Exercise 1 - SPSC Queue (45 min)
  Problem: Implement basic message queue
  Instructor:
  - Show channel creation (channel())
  - Send/recv pattern
  - Closure detection
  - Error handling
  Student:
  - Code along
  - Implement variant (bounded)
  
13:45-14:00: Q&A/Break
14:00-14:45: Guided Exercise 2 - MPSC Fan-In (45 min)
  Problem: Multiple senders to one receiver
  Show:
  - Creating multiple senders
  - Fair scheduling
  - Sender identification (if needed)
  - Detecting which sender is active
  
14:45-15:00: Break
15:00-15:45: Guided Exercise 3 - SPMC Broadcast (45 min)
  Problem: One sender broadcasts to N receivers
  Show:
  - Dynamic subscription
  - Sending to all
  - Late subscriber behavior
  - Handling closed receivers
  
15:45-16:00: Review & Summary
```

### Evening (6 hours) - Independent Work

```
17:00-18:00: Code Review Session (1 hour)
  - Peer review of guided exercises
  - Identify patterns
  - Best practices
  
18:00-19:00: Problem Set 1.1 (1 hour)
  - Solve 5 SPSC problems (problems 1.1.1-1.1.5)
  - Each: complete implementation + tests
  
19:00-20:00: Problem Set 1.2 (1 hour)
  - Solve 3 MPSC problems (1.2.1-1.2.3)
  - Focus: sender tracking
  
20:00-21:00: Practice Exercises (1 hour)
  - SimpleQueue (from exercises)
  - MultiSenderQueue
  - Get all tests passing
  
21:00-21:30: Documentation (30 min)
  - Document channel API
  - Record findings
  
21:30-22:00: Reflection (30 min)
  - Review: Can explain 4 channel types?
  - Prepare: Questions for Tuesday
```

---

## TUESDAY: ADVANCED CHANNELS & ORDERING (15 hours)

### Morning (4.5 hours)

```
09:00-09:30: Review - 4 Channel Types (30 min)
  - Quick recap
  - When to use each
  
09:30-10:15: Lecture - Bounded Channels (45 min)
  What: Fixed capacity channels
  Why: Prevent unbounded memory
  How: Track size, block on full
  Patterns:
  - try_send() non-blocking
  - send_timeout() with deadline
  - Graceful degradation
  
10:15-10:45: Lecture - Backpressure (30 min)
  Definition: Slow consumer slows producer
  Benefits: System stability
  Issues: Deadlock risk
  Patterns: Priority queues, dropping old
  
10:45-11:00: Break
11:00-11:45: Lecture - FIFO Ordering (45 min)
  Guarantee: FIFO within single sender
  Not: Global ordering with MPSC
  How to verify: sequence numbers
  Issues: Detecting out-of-order
  
11:45-12:30: Lecture - Delivery Guarantees (45 min)
  At-most-once: No dups (but may lose)
  At-least-once: No loss (but may dup)
  Exactly-once: Hard!
  How to detect: message IDs, checksums
```

### Afternoon (4.5 hours)

```
13:00-14:00: Guided Exercise - Bounded Channel (1 hour)
  Problem: Implement capacity tracking, backpressure
  Show:
  - is_full() predicate
  - send() blocks or fails
  - Utilization metrics
  - Timeout on send
  
14:00-14:15: Break
14:15-15:15: Guided Exercise - FIFO Verification (1 hour)
  Problem: Verify message ordering
  Show:
  - Sequence number tracking
  - Out-of-order detection
  - Gap detection
  - Recovery strategy
  
15:15-15:45: Guided Exercise - Priority Queue (30 min)
  Problem: Process high-priority before low
  Show:
  - Priority as parameter
  - Sorting in queue
  - Preventing starvation
  
15:45-16:00: Summary
```

### Evening (6 hours)

```
17:00-18:00: Problem Set 1.3 (1 hour)
  - Bounded channel problems (1.2.6-1.2.10)
  
18:00-19:00: Problem Set 2.1 (1 hour)
  - FIFO ordering problems (2.1.1-2.1.3)
  
19:00-20:00: Practice Exercises (1 hour)
  - BoundedChannel full implementation
  - Utilization tracking
  - All tests passing
  
20:00-21:00: Mini-Project (1 hour)
  - Build: 3-stage pipeline with bounded channels
  - Each stage has capacity limit
  - Measure: backpressure propagation
  - Verify: order preserved
  
21:00-22:00: Analysis & Refinement (1 hour)
  - Performance analysis
  - Bottleneck identification
  - Optimization ideas
```

---

## WEDNESDAY: PROTOCOL DESIGN (15 hours)

### Morning (4.5 hours)

```
09:00-10:00: Lecture - Request-Response Pattern (1 hour)
  Problem: How to match response to request?
  Solution 1: Correlation ID
  Solution 2: Separate response channel per request
  Patterns:
  - In-flight request tracking
  - Timeout on response
  - Duplicate detection
  - Cancellation
  
10:00-10:45: Lecture - Pub-Sub Pattern (45 min)
  What: One publisher, many subscribers
  Variants:
  - Late join: no old messages
  - Replay: get history
  - Filtered: topic-based
  - Durable: persist subscriptions
  
10:45-11:00: Break
11:00-11:45: Lecture - Pipeline Pattern (45 min)
  What: Chain of processing stages
  A -> B -> C
  Issues:
  - Ordering through pipeline
  - Bottleneck identification
  - Backpressure propagation
  
11:45-12:45: Case Studies (1 hour)
  Real-world examples:
  - Database: query request-response
  - Messaging: pub-sub systems
  - Data processing: MapReduce pipelines
```

### Afternoon (4.5 hours)

```
13:00-14:00: Guided Exercise - Request-Response (1 hour)
  Problem: Multiple in-flight requests
  Show:
  - Correlation ID generation
  - Response matching
  - Timeout handling
  - Cancellation
  
14:00-15:00: Guided Exercise - Pub-Sub (1 hour)
  Problem: Publisher + dynamic subscribers
  Show:
  - Dynamic subscription
  - Broadcasting
  - Late join handling
  - Unsubscription
  
15:00-16:00: Guided Exercise - Pipeline (1 hour)
  Problem: 3-stage pipeline, measure throughput
  Show:
  - Stage composition
  - Throughput bottleneck
  - Scaling bottleneck
  - Graceful shutdown
```

### Evening (6 hours)

```
17:00-18:00: Problem Set 4.1 (1 hour)
  - Request-response problems (4.1.1-4.1.5)
  
18:00-19:00: Problem Set 4.2 (1 hour)
  - Pub-Sub problems (4.2.1-4.2.3)
  
19:00-20:00: Practice Exercises (1 hour)
  - RequestResponder (from exercises)
  - Broadcaster (from exercises)
  - Complete implementations
  
20:00-21:30: Major Project (1.5 hours)
  - Build: Chat system
  - Users can broadcast messages
  - Subscribe to channels
  - Message history (last N)
  
21:30-22:00: Reflection & Documentation
```

---

## THURSDAY: ERROR HANDLING & RESILIENCE (15 hours)

### Morning (4.5 hours)

```
09:00-09:45: Lecture - Failure Modes (45 min)
  What can go wrong:
  - Sender drops unexpectedly
  - Receiver disconnects
  - Message corruption
  - Timeout (no response)
  - Deadlock
  - Panic in handler
  
09:45-10:30: Lecture - Detection Patterns (45 min)
  How to detect:
  - try_recv() returns error
  - recv() returns Err when sender gone
  - Watchdog timer for timeout
  - Checksum for corruption
  - Thread panic detection
  
10:30-11:00: Break
11:00-11:45: Lecture - Recovery Patterns (45 min)
  Techniques:
  - Restart component
  - Replay from log
  - Circuit breaker
  - Graceful degradation
  - Bulkhead isolation
  
11:45-12:30: Lecture - Observability (45 min)
  Debugging:
  - Message tracing
  - Latency measurement
  - Throughput metrics
  - Queue depth monitoring
  - Deadlock detection
```

### Afternoon (4.5 hours)

```
13:00-14:00: Guided Exercise - Timeout Handling (1 hour)
  Problem: Detect missing responses
  Show:
  - recv_timeout() pattern
  - Exponential backoff retry
  - Circuit breaker activation
  
14:00-15:00: Guided Exercise - Dead Letter Queue (1 hour)
  Problem: Capture failed messages
  Show:
  - DLQ pattern
  - Reason tracking
  - Replay mechanism
  
15:00-16:00: Guided Exercise - Metrics & Observability (1 hour)
  Problem: Track channel health
  Show:
  - Message counters
  - Latency tracking
  - Queue depth
  - Error rates
```

### Evening (6 hours)

```
17:00-18:00: Problem Set 5.1 (1 hour)
  - Error handling problems (5.1.1-5.1.3)
  
18:00-19:00: Problem Set 5.2 (1 hour)
  - Recovery patterns (5.2.1-5.2.3)
  
19:00-20:00: Practice Exercises (1 hour)
  - Add error handling to previous
  - Timeout support
  - Dead letter queue
  
20:00-21:30: Capstone Project (1.5 hours)
  - Build: Reliable message processor
  - Handles timeouts
  - Retries with backoff
  - Dead letter queue
  - Metrics/monitoring
  
21:30-22:00: Code Review & Refinement
```

---

## FRIDAY: PERFORMANCE & CAPSTONE (15 hours)

### Morning (4.5 hours)

```
09:00-09:45: Lecture - Throughput Optimization (45 min)
  Techniques:
  - Batching messages
  - Lock-free data structures
  - Producer-consumer separation
  - Zero-copy message transfer
  - Buffer pooling
  
09:45-10:30: Lecture - Latency Optimization (45 min)
  Techniques:
  - Minimize lock contention
  - Dedicated thread priorities
  - Memory locality
  - Busy-wait vs sleep
  - Profiling/measurement
  
10:30-11:00: Break
11:00-12:00: Performance Analysis (1 hour)
  Case study: Measure real system
  - Profile bottleneck
  - Identify contention
  - Apply optimization
  - Measure improvement
  
12:00-12:30: Lecture - Advanced Topics (30 min)
  Preview:
  - Lock-free queues
  - RCU (Read-Copy-Update)
  - Message compression
  - Serialization strategies
```

### Afternoon (4.5 hours)

```
13:00-14:30: Capstone Project Setup (1.5 hours)
  Project: Multi-stage Data Processing Pipeline
  Requirements:
  1. Input stage: reads numbers
  2. Processing: complex computation
  3. Output: writes results
  4. Error handling: timeouts, retries
  5. Monitoring: throughput, latency, errors
  
  Architecture:
  - Stage 1 (producer): bounded queue to Stage 2
  - Stage 2 (worker): bounded queue to Stage 3
  - Stage 3 (consumer): output queue
  - Backpressure handling
  - Graceful shutdown
  
14:30-15:00: Break
15:00-16:00: Capstone Implementation (1 hour)
  - Implement 3 stages
  - Connect with channels
  - Add error handling
  - Add metrics
  
16:00-16:30: Testing & Validation (30 min)
  - Verify ordering
  - Check error handling
  - Measure performance
```

### Evening (4 hours)

```
17:00-18:00: Problem Solving (1 hour)
  - Final problem set (6.1-6.3)
  - Optimization challenges
  
18:00-19:00: Capstone Completion (1 hour)
  - Polish implementation
  - Add comprehensive tests
  - Document code
  
19:00-19:30: Code Review (30 min)
  - Self-review
  - Peer feedback
  
19:30-20:00: Testing & Metrics (30 min)
  - Benchmark capstone
  - Record metrics
  - Document results
```

### Final Review (1 hour)

```
20:00-20:30: Week 9 Reflection
  Self-assessment:
  - [ ] Understand all 4 channel types
  - [ ] Solve 150+ problems
  - [ ] Design custom protocols
  - [ ] Handle errors gracefully
  - [ ] Optimize for performance
  - [ ] Build complex systems
  
20:30-21:00: Prepare Week 10
  - Read actor model introduction
  - Review message handling patterns
  - Preview supervised failure
```

---

## WEEKEND REVIEW

### Saturday (6 hours)
- Review all Week 9 code
- Solve additional 20+ problems
- Refactor capstone for clarity
- Document patterns discovered
- Create personal reference guide

### Sunday (4 hours)
- Polish all code
- Complete documentation
- Prepare code samples
- Review for Week 10 transition

---

## DAILY CHECKLIST

✓ **Morning (Pre-work)**
- [ ] Review yesterday's learning
- [ ] Read today's topics
- [ ] Set 3 concrete goals
- [ ] Prepare workspace

✓ **Lecture/Concept Time** (30 min tasks)
- [ ] Watch/attend lecture
- [ ] Take notes on key points
- [ ] Ask clarifying questions
- [ ] Identify examples

✓ **Guided Practice** (45 min exercises)
- [ ] Follow along with instructor
- [ ] Implement variant
- [ ] Test thoroughly
- [ ] Explain to peer

✓ **Independent Work** (2+ hours)
- [ ] Solve assigned problems
- [ ] Write comprehensive tests
- [ ] Handle edge cases
- [ ] Document learning

✓ **Evening Reflection** (30 min)
- [ ] Review what learned
- [ ] Identify gaps
- [ ] Plan tomorrow
- [ ] Record progress

---

## PROGRESS TRACKING

### Daily Target: 3-5 problems solved
- Monday: 8 problems
- Tuesday: 8 problems
- Wednesday: 8 problems
- Thursday: 8 problems
- Friday: 8 problems
- **Weekend: 25-30 problems (catch-up + bonus)**
- **Total: 75+ problems minimum**

### Self-Assessment Points

| Topic | Monday | Tuesday | Wednesday | Thursday | Friday |
|-------|--------|---------|-----------|----------|--------|
| SPSC Channels | [ ] | [ ] | [ ] | [ ] | ✓|
| MPSC Channels | [ ] | ✓ | [ ] | [ ] | ✓|
| SPMC (Broadcast) | [ ] | ✓ | [ ] | [ ] | ✓|
| MPMC (Work Queue) | [ ] | [ ] | [ ] | [ ] | ✓|
| Bounded Channels | [ ] | ✓ | ✓ | [ ] | ✓|
| Request-Response | [ ] | [ ] | ✓ | ✓ | ✓|
| Pub-Sub Pattern | [ ] | [ ] | ✓ | [ ] | ✓|
| Pipeline Pattern | [ ] | [ ] | ✓ | ✓ | ✓|
| Error Handling | [ ] | [ ] | [ ] | ✓ | ✓|
| Performance | [ ] | [ ] | [ ] | [ ] | ✓|

All should be ✓ by Friday EOD

---

## Resources This Week

### Documentation
- `MESSAGE_PASSING_WEEK_9.md` - Comprehensive guide
- `message_passing_exercises.rs` - Worked examples
- `PROBLEM_BANK_WEEKS_9_11.killer` - Full problem set
- `MESSAGE_PASSING_REFERENCE.md` - Quick patterns

### Tools
- Rust std::sync::mpsc
- crossbeam channels (if time)
- tokio::sync channels (Week 10 transition)

### Reading
- Rust Book: "Fearless Concurrency"
- "Communicating Sequential Processes" - seminal paper
- Channel implementation internals
