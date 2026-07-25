# Week 10 Actor Model Reference Guide
## Patterns, Debugging, and Production Strategies

---

# Part 1: Actor Fundamentals Patterns

## Pattern 1: The Actor Model Principles

### Core Principles
```
1. ISOLATION: Actors are isolated units
   - Each has its own state (no shared memory)
   - Only accessible via message passing
   - No data races possible

2. CONCURRENCY: Thousands of actors can run concurrently
   - Event-driven (not thread-per-actor)
   - Mailbox queues incoming messages
   - Processes one message at a time

3. LOCATION TRANSPARENCY: Don't know/care where actor is
   - Could be local (same process)
   - Could be remote (different node)
   - Message passing same either way

4. SUPERVISION: Hierarchical fault tolerance
   - Parent supervises children
   - On failure: restart, resume, or stop
   - Automatic recovery
```

## Pattern 2: Message Passing
```rust
// Types of messages
pub enum ActorMessage {
    Request { data: String },      // Expect response
    Command { action: String },    // Fire and forget
    Event { what_happened: String }, // Event occurred
}

// Actor processes one message at a time
impl MyActor {
    fn receive(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::Request { data } => {
                // Process, prepare response
                // Send back to sender
            }
            ActorMessage::Command { action } => {
                // Just do it
            }
            ActorMessage::Event { what_happened } => {
                // React to external event
            }
        }
    }
}

// Benefits:
// ✓ Simple to reason about (sequential)
// ✓ No locks needed
// ✓ Can handle backpressure (queue fills up)
// ✓ Natural timeouts (actor not responding = dead)
```

---

# Part 2: Supervision Patterns

## Pattern 1: One-for-One Strategy
```
When child fails:
  ✓ Restart only that child
  ✗ Don't affect other children

Use when:
  - Children are independent
  - One child dying shouldn't kill others
  - Example: pool of worker actors

Diagram:
  Supervisor
    ├── Worker1
    ├── Worker2 (crashed, restarted)
    ├── Worker3
    └── Worker4

Worker2 dies → Restart Worker2
Other workers: unaffected
```

## Pattern 2: All-for-One Strategy
```
When ONE child fails:
  ✓ Restart ALL children

Use when:
  - Children are co-dependent
  - Shared state or protocol
  - Must stay in sync

Example: Database with primary + replicas
  - Primary dies → all restart together
  - Ensures consistency on recovery

Supervisor
  ├── PrimaryDB (crashed)
  ├── ReplicaDB1
  └── ReplicaDB2

PrimaryDB dies → Restart All (primary + replicas)
```

## Pattern 3: Escalation Strategy
```
When child restarts too many times:
  ✓ Stop restarting, escalate to parent

Use when:
  - Repeated failures indicate systemic problem
  - Parent supervisor can decide action

Implementation:
  - Count restarts in time window
  - If > threshold: escalate
  - Parent can: retry, stop, or restart itself

Example:
  Worker fails 5 times in 1 minute
  → Escalate to supervisor
  → Supervisor sees: maybe not a flaky issue
  → Full restart of entire service
```

## Pattern 4: Exponential Backoff
```
Don't restart immediately on failure:
  1st failure: restart immediately
  2nd failure: wait 100ms, then restart
  3rd failure: wait 1000ms, then restart
  4th failure: wait 10000ms, then restart
  5th failure: escalate (giving up)

Benefits:
  ✓ Prevents thundering herd
  ✓ Gives transient errors time to recover
  ✓ Cascade doesn't take down system

Code pattern:
  restart_counter.increment();
  backoff = base_delay * (2 ^ restart_counter);
  sleep(backoff);
  restart_actor();
```

---

# Part 3: Actor Patterns in Practice

## Pattern: Request-Response
```rust
// Actor A wants to ask Actor B something
// Includes return address

struct Question {
    query: String,
    reply_to: ActorRef,
}

struct Answer {
    response: String,
}

// Actor B
impl Actor {
    fn receive_question(&mut self, q: Question) {
        let answer = self.compute(&q.query);
        q.reply_to.send(Answer { response: answer });
    }
}

// Benefits:
// ✓ Natural request-response pattern
// ✓ Timeout if actor dies
// ✗ Temporary coupling (waits for response)
```

## Pattern: Future-Response
```rust
// Ask for something, get back a Future
// Process continues immediately

let future = actor.ask(request);

// Do other things...

let response = future.await; // Block here if needed

// Benefits:
// ✓ Non-blocking
// ✓ Can run multiple queries in parallel
// ✗ More complex code
```

## Pattern: Batching
```
Problem: Many small messages = overhead
Solution: Batch them

// Bad: 1000 individual messages
for i in 0..1000 {
    actor.send(UpdateRequest { data: i });
}

// Good: One batch message
let batch = vec![...1000 items...];
actor.send(BatchUpdateRequest { items: batch });

// Actor processes whole batch:
impl Actor {
    fn handle_batch(&mut self, items: Vec<Item>) {
        for item in items {
            self.process_one(item);
        }
    }
}

// Benefits:
// ✓ 1000x fewer messages
// ✓ Better throughput
// ✓ Natural grouping
```

---

# Part 4: Common Failures & Solutions

## Failure 1: Deadlock (Circular Dependencies)
```
Problem:
  Actor A waits for reply from B
  Actor B waits for reply from A
  → Both blocked forever

Sources:
  - A sends RequestToB (reply_to=A)
  - B tries to send RequestToA (reply_to=B)
  - Both wait for each other

Solutions:
  ✓ Use timeouts (don't wait forever)
  ✓ Avoid circular ask() patterns
  ✓ Use fire-and-forget instead
  ✓ Design: one-way message flows

Detection:
  - Monitor for actors not processing messages
  - Distributed deadlock detector
```

## Failure 2: Memory Leak (Growing Mailbox)
```
Problem:
  Actor can't process messages fast enough
  Mailbox queue grows unbounded
  → Out of memory

Causes:
  - Slow actor (database connection slow)
  - Downstream backpressure (next actor blocked)
  - Message processing getting slower over time

Solutions:
  ✓ Bounded mailbox (reject when full)
  ✓ Backpressure (source stops sending)
  ✓ Optimize actor code
  ✓ Scale horizontally (more actors)

Code:
  // Bounded mailbox
  if mailbox.len() > MAX_SIZE {
      return Err(MailboxFull);
  }
  
  // Backpressure
  if !actor.can_accept_message() {
      wait_and_retry();
  }
```

## Failure 3: Lost Messages
```
Problem:
  Message sent to dead actor
  → Message lost forever

Causes:
  - Actor crashed
  - Network partition
  - Actor restarted (lost in-flight messages)

Solutions:
  ✓ Acknowledgments (confirm receipt)
  ✓ Dead letter queue (capture lost messages)
  ✓ Persistent queues (like Kafka)
  ✓ At-least-once delivery (retry)

Code pattern:
  actor.send(message);
  wait_for_ack();  // Confirm received
  
  or

  // Dead letter queue
  if no_ack_timeout {
      dead_letter_queue.add(message, "no ack");
  }
```

## Failure 4: Stale Data (Concurrent Reads)
```
Problem:
  Actor A reads state
  Actor B modifies state
  Actor A uses stale data

Why this is OK in actors:
  - Actor only accesses own state
  - No shared mutable state
  - Each actor is single-threaded
  - No race conditions!

If you need distributed consistency:
  ✓ Vector clocks (track causality)
  ✓ Event sourcing (all changes versioned)
  ✓ Consensus (multiple agree before commit)
```

---

# Part 5: Actor Patterns at Scale

## Pattern: Actor as Resource Manager
```
Actor owns a database connection:
  - Create connection in init
  - Process queries via messages
  - Close connection in shutdown
  
Benefits:
  ✓ Resource is tied to actor lifecycle
  ✓ Automatic cleanup when actor dies
  ✓ No connection leaks
  ✓ Simple reasoning

Example:
  pub struct DatabaseActor {
      conn: Arc<Connection>,  // Resource
  }

  impl DatabaseActor {
      async fn handle_query(&mut self, sql: String) {
          let result = self.conn.query(&sql).await;
          // Send back result
      }
  }
```

## Pattern: Router Actor
```
Single entry point that routes to workers:

Client
  ↓
Router Actor (receives all requests)
  ├→ Worker1
  ├→ Worker2
  ├→ Worker3
  └→ Worker4

Benefits:
  ✓ Load balance across workers
  ✓ Workers don't contact each other
  ✓ Easy to scale (add more workers)
  ✓ Central monitoring point

Implementation:
  pub struct RouterActor {
      workers: Vec<ActorRef>,
      current: usize,
  }
  
  fn route(&mut self, msg: Message) {
      let worker = self.workers[self.current];
      self.current = (self.current + 1) % self.workers.len();
      worker.send(msg);  // round-robin
  }
```

## Pattern: Ask Pattern with Timeout
```
Distributed request-response with timeout:

// Send request expecting response
let future = actor.ask(request, timeout: 5s);

// Wait for response (with timeout)
match timeout(5s, future).await {
    Ok(Ok(response)) => println!("Got response: {}", response),
    Ok(Err(e)) => println!("Actor returned error: {}", e),
    Err(_) => println!("Timeout waiting for response"),
}

Benefits:
  ✓ Explicit timeout (can't wait forever)
  ✓ Know if actor is dead/slow
  ✓ Can retry with fresh actor
  ✓ Clear failure semantics

When to use:
  - Synchronous operations (prefer false)
  - Most systems: ask rare, fire-and-forget common
```

## Pattern: Supervisor as Registry
```
Supervisor keeps track of children:

pub struct SupervisorActor {
    children: HashMap<String, ActorRef>,
    strategy: SupervisorStrategy,
}

impl Supervisor {
    fn start_child(&mut self, name: String, actor: Actor) {
        let ref = ActorRef::spawn(actor);
        self.children.insert(name, ref);
    }
    
    fn get_child(&self, name: &str) -> Option<ActorRef> {
        self.children.get(name).cloned()
    }
    
    fn restart_child(&mut self, name: &str) {
        // Kill and restart with same name
        // Clients find it via supervisor
    }
}

Benefits:
  ✓ Clients contact supervisor to find workers
  ✓ Workers can restart, clients find new version
  ✓ Central monitoring of all children
  ✓ Can change strategy at runtime
```

---

# Part 6: Debugging Actor Systems

## Debug Technique 1: Structured Logging
```rust
// Log with context
struct ActorLog {
    actor_name: String,
    message_id: String,
    timestamp: Instant,
}

impl Actor {
    fn receive(&mut self, msg: Message) {
        let log = ActorLog {
            actor_name: self.name().to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Instant::now(),
        };
        
        log::info!("{:?} Received message: {:?}", log, msg);
        
        match self.process(msg) {
            Ok(_) => log::info!("{:?} Processed OK", log),
            Err(e) => log::error!("{:?} Error: {}", log, e),
        }
    }
}

// All logs for same message have same ID
// Can trace execution across actors
```

## Debug Technique 2: Message Interception
```
Wrap actor with debugging proxy:

Client → DebugProxy → RealActor
  ↓
  Logs all messages
  Measures latency
  Tracks failures

pub struct DebuggedActor {
    inner: Box<dyn Actor>,
    messages_received: u64,
    total_latency: Duration,
}

impl Actor for DebuggedActor {
    fn receive(&mut self, msg: Message) {
        let start = Instant::now();
        self.inner.receive(msg);
        let latency = start.elapsed();
        
        self.messages_received += 1;
        self.total_latency += latency;
        
        let avg_latency = self.total_latency / self.messages_received;
        println!("Average latency: {:?}", avg_latency);
    }
}
```

## Debug Technique 3: Timeline Visualization
```
Message flow visualization:

Time    Actor1          Actor2          Actor3
----    ------          ------          ------
0ms     |— send MessageA →|
1ms                     |— process —|
2ms                     |— send Response →|
3ms                     |               |— process —|
4ms                     |               |— done

Helps identify:
  ✓ Bottlenecks (slow actors)
  ✓ Cascade failures (cascading delays)
  ✓ Message sequences (did it follow expected flow?)
  ✓ Deadlocks (circular waits)

Tools:
  - Jaeger (distributed tracing)
  - Akka monitoring
  - Custom timeline collector
```

## Debug Technique 4: State Snapshots
```
Periodically dump actor state:

struct StateSnapshot {
    actor_name: String,
    timestamp: Instant,
    internal_state: String,
    mailbox_size: usize,
    messages_processed: u64,
}

impl Actor {
    fn take_snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            actor_name: self.name().to_string(),
            timestamp: Instant::now(),
            internal_state: format!("{:?}", self.state),
            mailbox_size: self.mailbox.len(),
            messages_processed: self.processed_count,
        }
    }
}

// When actor behaves unexpectedly:
// 1. Dump state snapshot
// 2. Reproduce locally with snapshot state
// 3. Debug normal code (not distributed)
```

---

# Part 7: Production Checklist

## Before deploying actor system:

### Supervision & Resilience
- [ ] Supervisor strategy chosen (one-for-one, all-for-one, custom)
- [ ] Child restart limits configured (max 3 restarts per minute)
- [ ] Escalation rules defined (when to give up)
- [ ] Dead letter queue configured
- [ ] Timeout values set for all ask() operations

### Monitoring & Observability
- [ ] Structured logging with actor context
- [ ] Metrics: message rate, error rate, latency
- [ ] Alerts: high error rate, slow actors, dead letter queue growing
- [ ] Distributed tracing enabled
- [ ] Dashboards created (actor throughput, failures, messaging)

### Testing & Verification
- [ ] Unit tests for each actor type
- [ ] Happy path integration tests
- [ ] Failure scenarios (actor crashes while processing)
- [ ] Backpressure tests (queue fills up)
- [ ] Lifecycle tests (start, shutdown gracefully)
- [ ] Load tests (1000+ concurrent messages)

### Operations
- [ ] Graceful shutdown implemented (finish in-flight, stop accepting)
- [ ] Health checks (responds to ping)
- [ ] Runbook for common failures
- [ ] Rolling update strategy (new actors replace old)
- [ ] Monitoring of supervision decisions (restarts/escalations)

### Security
- [ ] Message validation (untrusted sources)
- [ ] Authentication between actors (if distributed)
- [ ] Encryption for sensitive data in transit
- [ ] Rate limiting on actor mailboxes

---

# Part 8: Real-World Examples

## Example 1: Chat Application Actors
```
Architecture:
  Connection Actor (per WebSocket)
    ├→ handles network IO
    ├→ encodes/decodes messages
    └→ routes to ChatRoom

  ChatRoom Actor
    ├→ maintains members list
    ├→ broadcasts messages
    └→ handles member join/leave

  Database Actor
    ├→ handles persistence
    └→ shared across all rooms

Flow:
  User types message
  → Connection Actor receives
  → Sends to ChatRoom
  → ChatRoom broadcasts to all connections
  → Connections send to users
```

## Example 2: Game Server Actors
```
Architecture:
  GameSession Actor (per game)
    ├→ manages players
    ├→ game state
    └→ sends updates

  Player Actor (per player)
    ├→ network connection
    ├→ input handling
    └→ action processing

  PhysicsActor
    ├→ collision detection
    ├→ world state
    └→ periodic updates

Scale: Million players
  - 1 million player actors
  - 100K game sessions
  - 1 physics actor per cluster
  - ~1000 message/sec per player
```

## Example 3: Microservices as Actors
```
Each microservice = pool of actor instances

UserService
  → 20 actor instances (load balanced)
  → Each handles requests
  → State: in-memory cache + persistence

OrderService
  → 50 actor instances (high volume)
  → Each handles order processing
  → State: order data + customer info

InventoryService
  → 10 actor instances
  → Each manages product info
  → Uses Ask pattern for consistency

Communication: Message passing between services
Scalability: Add more actor instances as needed
Resilience: Automatic restart on failure
```

---

# Part 9: Advanced Topics

## Topic: Stashing Pattern
```
Sometimes actor can't handle message yet:
  - Waiting for prerequisite data
  - State machine not in right state
  - Downstream service overloaded

Solution: Stash message, handle later

impl MyActor {
    fn receive(&mut self, msg: Message) {
        match self.state {
            State::Initializing => {
                // Can't handle yet
                self.stash(msg);
            }
            State::Ready => {
                // Process immediately
                self.handle(msg);
            }
        }
    }
    
    fn become_ready(&mut self) {
        self.state = State::Ready;
        // Process stashed messages
        while let Some(msg) = self.unstash():
            self.handle(msg);
        }
    }
}
```

## Topic: Hot-Swap (Code Updates)
```
Change actor behavior without stopping:

impl MyActor {
    fn become(&mut self, behavior: Behavior) {
        self.behavior = behavior;
        // No message lost
        // Processed as new behavior going forward
    }
}

Example: A/B testing
  - Actor can swap between behaviors
  - Route 10% to new behavior, 90% to old
  - Gradually increase percentage
  - Rollback if issues
```

---

**Summary**: Actor model provides powerful patterns for building concurrent, distributed systems. Key to success: simple supervision, clear messaging, explicit timeouts, comprehensive monitoring.
