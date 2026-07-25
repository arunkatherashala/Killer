# Week 10 Weekly Schedule: Actor Model & Supervision
## 75 Hours | Building Fault-Tolerant Systems

---

# WEEKLY OVERVIEW

**Monday**: Actor Fundamentals (15h)
**Tuesday**: Supervision Strategies (15h)
**Wednesday**: Service Architecture (15h)
**Thursday**: Supervision Trees & Integration (15h)
**Friday**: Capstone - Distributed Order System (15h)

**Time Allocation**
- Concepts & Theory: 25 hours (33%)
- Hands-on Exercises: 35 hours (47%)
- Capstone Project: 15 hours (20%)

---

# MONDAY: ACTOR FUNDAMENTALS (15 hours)

## 09:00-11:00 | What is an Actor? (2h)

**Concepts**
```
Actor = Concurrent unit with:
  ✓ Isolation (own state, no sharing)
  ✓ Message-driven (processes one message at a time)
  ✓ Supervision (can be restarted on failure)
  ✓ Location transparency (local or remote)

Why actors?
  - Avoid shared memory bugs (no locks needed)
  - Natural concurrency (think in terms of entities)
  - Fault tolerance (restart on failure)
  - Scalability (thousands running)
```

**Problems**
```
10.1.1-10: Actor basics
  - Define an actor trait
  - Create echo actor (repeats all messages)
  - Send messages to actor
  - Process sequentially
  - Handle state changes
  - Lifecycle (start, receive, stop)
```

**Hands-on** (Exercise 1: Basic Actor Trait from advanced_exercises.rs)
```rust
pub trait Actor: Send {
    fn receive(&mut self, msg: Box<dyn std::any::Any>);
    fn name(&self) -> &str;
    fn shutdown(&mut self);
}

// Implement: EchoActor that counts messages
pub struct EchoActor {
    name: String,
    messages_received: usize,
}
```

### 11:00-13:00 | Message Passing (2h)

**Concepts**
- Message types (request, response, event)
- Mailbox queue (bounded/unbounded)
- Backpressure (queue is full)
- Asynchronous processing

**Problems** (10.1.11-20)
```
- Design message enum for your domain
- Handle multiple message types
- Implement backpressure
- Process ordering guarantees
```

**Hands-on** (Exercise 2: Counter Actor from advanced_exercises.rs)
```rust
enum CounterMessage {
    Increment(i32),
    Decrement(i32),
    Reset,
    GetValue,
}

// Implement: Counter that maintains state
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Actor Lifecycle (3h)

**Concepts**
- Spawning (create, give it a name)
- Running (message loop)
- Recovery (what happens on error?)
- Shutdown (graceful)

**Problems** (10.1.21-30)
```
- Spawn multiple actors
- Send messages between them
- Implement graceful shutdown
- Handle timeouts
```

**Hands-on** (Exercise 3: Actor Pool from advanced_exercises.rs)
```rust
pub struct ActorPool<A: Actor> {
    actors: Vec<Arc<Mutex<A>>>,
    current: usize,
}

// Implement: load balance across multiple actors
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercise 1-3 in advanced_exercises.rs
- Get all tests passing
- Understand foundational patterns

### 18:00-19:00 | Problem Set 10.1 (1h)
- Solve 10-15 basic actor problems
- Prepare for supervision tomorrow

---

# TUESDAY: SUPERVISION STRATEGIES (15 hours)

## 09:00-11:00 | Supervision Concepts (2h)

**Concepts**
```
When actor fails:
  - Application: should it crash too?
  - Solution: Supervisor catches failure
  - Decides: restart, resume, or stop

Hierarchy:
  Supervisor
    ├─ Worker1
    ├─ Worker2
    └─ Worker3

If Worker2 fails:
  Supervisor: "Got error from Worker2"
  Supervisor: Decides action (usually restart)
  Worker2: Restarted with clean state
  Workers 1,3: Unaffected
```

**Problems** (10.2.1-10)
```
- Design supervision strategy
- Decide when to restart vs stop
- Handle supervisor failures
- Nested supervision (supervisor of supervisors)
```

### 11:00-13:00 | Supervision Strategies (2h)

**Three Main Strategies**

**One-for-One** (Exercise 4)
```
When child fails: restart ONLY that child

  Supervisor
    ├─ Worker1 (fine)
    ├─ Worker2 (crashed → RESTART)
    ├─ Worker3 (fine)
    └─ Worker4 (fine)

Use when: Workers independent
```

**All-for-One** (from ACTOR_REFERENCE.md)
```
When child fails: restart ALL children

  Supervisor
    ├─ PrimaryDB (crashed)
    ├─ ReplicaDB1 (restart)
    ├─ ReplicaDB2 (restart)
    └─ ReplicaDB3 (restart)

Use when: Children must stay in sync
```

**Custom Strategies** (Exercise 4)
```
Different actions based on error type:

  TimeoutError → Restart (transient)
  ConfigError → Stop (permanent)
  DatabaseError → Escalate (to parent supervisor)
```

**Problems** (10.2.11-25)
```
- Implement one-for-one
- Implement all-for-one
- Custom strategy for your domain
- Handle cyclic failures (restart loop)
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Failure Handling (3h)

**Concepts**
- Detecting failure (timeout, exception, slow response)
- Deciding action (pattern matching on error)
- Restart limits (don't restart forever)
- Escalation (when to give up)

**Problems** (10.2.26-35)
```
- Restart counter (limit restarts per minute)
- Exponential backoff (wait longer between restarts)
- Escalation policy (max 3 restarts, then escalate)
- Graceful shutdown (cancel in-flight work)
```

**Hands-on** (Exercise 5: ActorRestartCounter from actor_model_exercises.rs)
```rust
pub struct ActorRestartCounter {
    restart_count: u32,
    max_restarts: u32,
    window: Duration,
}

// Track restarts, prevent infinite loops
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercises 4-5
- Test one-for-one supervisor
- Test restart counter

### 18:00-19:00 | Problem Set 10.2 (1h)
- Solve 15 supervision problems
- Prepare for service architecture

---

# WEDNESDAY: SERVICE ARCHITECTURE (15 hours)

## 09:00-11:00 | Actor Patterns (2h)

**Request-Response Pattern** (Exercise 6)
```
Client sends request with reply address:
  Message { query: "...", reply_to: actor_a }
  
Actor A responds:
  reply_to.send(Response { answer: "..." })

Benefits:
  ✓ Natural for synchronous operations
  ✗ Actor waits (blocks on response)
```

**Fire-and-Forget Pattern**
```
Send command, don't wait:
  actor.send(Command { action: "log", data: "..." })
  
Benefits:
  ✓ Non-blocking
  ✓ High throughput
  ✗ No confirmation (message might be lost)
```

**Event Pattern**
```
Actor publishes event:
  event_bus.publish(OrderCreated { id, items, ... })
  
Other actors listen:
  on(OrderCreated) → process event

Benefits:
  ✓ Decoupled (event originator doesn't know consumers)
  ✓ Scales easily (add consumers)
  ✗ Eventually consistent
```

**Problems** (10.3.1-15)
```
- Use request-response for payment
- Use fire-and-forget for logging
- Use events for order flow
- Handle missing responses (timeout)
```

### 11:00-13:00 | Building Services (2h)

**Hands-on: Design user service with actor pool**
```
UserService (10 actor instances)
  ├─ Instance 1 (handles GetUser requests)
  ├─ Instance 2 (handles CreateUser requests)
  └─ ... (Instance 10)

Router: Load balance across pool
```

**Exercise 6: Actor Pool and Registry**
```rust
pub struct ActorPool<A: Actor> {
    actors: Vec<Arc<Mutex<A>>>,
    current: usize,
}

// Round-robin load balancing
```

**Exercise 7: ActorContext**
```rust
pub struct ActorContext {
    path: ActorPath,
    children: HashMap<String, ActorHandle>,
}

// Track parent-child relationships
```

**Problems** (10.3.16-30)
```
- Build service from actor pool
- Router distributes requests
- Find actor by name/path
- Monitor pool health
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Dead Letters & Broadcast (3h)

**Dead Letter Queue** (Exercise 8)
```
When message can't be delivered:
  1. Service shuts down (actor dies)
  2. Message in mailbox lost
  3. DLQ captures: what, to whom, why

Later:
  Operator can replay DLQ
  Or manual investigation
```

**Actor Broadcast** (Exercise 8)
```
Publisher → many subscribers

pub struct ActorBroadcaster {
    subscribers: Vec<ActorHandle>,
}

impl Broadcaster {
    fn publish(&self, msg: Message) {
        for sub in &self.subscribers {
            sub.send(msg.clone());
        }
    }
}

Use cases:
  - Event bus (all services listen)
  - Notifications (send to all users)
  - Status updates (broadcast to followers)
```

**Problems** (10.3.31-45)
```
- Capture dead letters
- Analyze why lost
- Replay messages
- Set up broadcast bus
- Multiple subscribers
- Handle slow subscribers (backpressure)
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercises 6-8
- Test request-response
- Test fire-and-forget
- Test broadcast

### 18:00-19:00 | Problem Set 10.3 (1h)

---

# THURSDAY: SUPERVISION TREES & INTEGRATION (15 hours)

## 09:00-11:00 | Supervision Trees (2h)

**Hierarchical Organization**
```
RootSupervisor
  ├─ UserServiceSupervisor
  │   ├─ UserActor1
  │   ├─ UserActor2
  │   └─ UserActor3
  ├─ OrderServiceSupervisor
  │   ├─ OrderActor1
  │   ├─ OrderActor2
  │   └─ OrderActor3
  └─ PaymentServiceSupervisor
      ├─ PaymentActor1
      └─ PaymentActor2

Failure propagation:
  UserActor1 fails
  → UserServiceSupervisor restarts it
  → If supervisor can't restart (too many failures)
  → Escalate to RootSupervisor
  → RootSupervisor decides (usually restart UserService)
```

**Problems** (10.4.1-15)
```
- Design supervision hierarchy
- Per-service supervisors
- Escalation rules
- Restart with increasing delays
```

**Hands-on: ActorPath**
```rust
pub struct ActorPath {
    segments: Vec<String>,
}

// Path: "/root/user-service/actor-1"
// Used for: identification, routing, monitoring
```

### 11:00-13:00 | Distributed Actors (2h)

**Location Transparency**
```
Local actor:
  let actor = ActorRef::spawn(MyActor::new());
  actor.send(msg);

Remote actor:
  let actor = ActorRef::remote("node2", "/path/to/actor");
  actor.send(msg);  // Same API!

Behind the scenes:
  Local: Direct call
  Remote: Network RPC + serialization
```

**Remote Actor Refs** (Exercise 9)
```rust
pub struct RemoteActorRef {
    node: String,
    path: String,
}

pub struct ClusterAwareActorRef {
    local_ref: ActorHandle,
    remote_refs: HashMap<String, RemoteActorRef>,
}

// Support replicas across nodes
```

**Problems** (10.4.16-30)
```
- Send to local actor
- Send to remote actor
- Handle network failures
- Manage replicas
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Integration & Testing (3h)

**Integration Patterns**
```
Actor A → Actor B → Database

Test:
  1. Start B (mock database)
  2. Start A
  3. Send message to A
  4. Verify B called database
  5. Verify response came back

Challenge:
  Timing: actors async, need to wait for completion
  Solution: Async/await or message-based assertions
```

**Testing Strategies**
```
Unit: Test actor in isolation
  - Mock dependencies
  - Verify state changes

Integration: Test actor with real dependencies
  - Real database
  - Real network calls

End-to-end: Full system test
  - All actors together
  - From user perspective
```

**Problems** (10.4.31-45)
```
- Test supervisor restart
- Test escalation
- Test remote communication
- Test failure scenarios
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercise 9-10
- Test distributed actors
- Test supervision trees

### 18:00-19:00 | Problem Set 10.4 (1h)

---

# FRIDAY: CAPSTONE - DISTRIBUTED ORDER SYSTEM (15 hours)

## 09:00-11:00 | Design Phase (2h)

**Requirements**
```
Build order system with:
  - 5 service types (User, Product, Order, Payment, Inventory)
  - Each supervises 10 actor instances
  - Handle failures gracefully
  - Process 1000 orders/second
  - 99.9% uptime
```

**Architecture**
```
RootSupervisor
  ├─ UserServiceSupervisor (10 actors)
  ├─ ProductServiceSupervisor (10 actors)
  ├─ OrderServiceSupervisor (20 actors, high traffic)
  ├─ PaymentServiceSupervisor (5 actors)
  └─ InventoryServiceSupervisor (5 actors)

Request flow:
  1. Router receives create order request
  2. Routes to OrderServiceSupervisor (picks 1 of 20)
  3. OrderActor validates using UserActor + ProductActor
  4. PaymentActor charges (with timeout, retry)
  5. InventoryActor reserves items
  6. Return confirmation to client
```

**Design Tasks** (1h)
```
- Draw architecture diagram
- List 15 possible failure scenarios
  - PaymentService down
  - Inventory out of stock
  - Network timeout
  - Actor crash mid-request
  - Restart storm (too many failures)
  - etc.
- Design supervisor strategy for each service
- Define escalation rules
- Plan testing approach
```

## 11:00-13:00 | Implementation Phase (2h)

**Build**
- Implement 5 service supervisors
- Implement actor pools (per service)
- Implement message routing
- Test basic flow (happy path)

**Code Structure**
```rust
// Services (actor instances)
pub struct UserActor { ... }
pub struct OrderActor { ... }
pub struct PaymentActor { ... }

// Supervisors
pub struct UserServiceSupervisor { ... }
pub struct OrderServiceSupervisor { ... }

// Router
pub struct OrderRouter {
    supervisors: HashMap<String, SupervisorRef>,
}

impl OrderRouter {
    fn create_order(&self, req: CreateOrderRequest) {
        let actor = self.supervisors["order"].next_actor();
        actor.send(req);
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Testing & Hardening (3h)

**Integration Tests**
```
Test 1: Happy path
  create_order(user=1, items=[book, pen]) → OK

Test 2: Payment failure (network timeout)
  → Retry (exponential backoff)
  → Order eventually confirmed
  
Test 3: Actor crash mid-request
  → Supervisor restarts
  → Message reprocessed (idempotent)
  → Client eventually gets response

Test 4: Supervisor overloaded
  → Queue fills up
  → New requests rejected (graceful)
  → Existing requests still processed

Test 5: Multiple cascading failures
  → Inventory + Payment both slow
  → System handles (timeouts, retries)
  → Eventually recovers
```

**Load Test** (1h)
```
Send 1000 concurrent order requests
Measure:
  - Throughput (requests/sec)
  - Latency (p50, p95, p99)
  - Error rate
  - Recovery time

Target:
  - 1000 req/sec
  - p99 latency: 500ms
  - Error rate: < 0.1%
```

### 17:00-19:00 | Presentation & Retrospective (2h)

**Present Your System** (30 min)
```
- Architecture diagram
- Data flow (happy path + failures)
- Supervision strategy choices
- Test results
- Lessons learned
```

**Retrospective** (30 min)
```
What worked?
What was hard?
What would you change?
What's missing for production?
  - Monitoring (metrics, logs)
  - Operations (runbooks, alerting)
  - Security (authentication, encryption)
  - Testing (chaos experiments)
```

**Key Takeaways**
```
✓ Actors are natural for distributed systems
✓ Supervision handles failures automatically
✓ Hierarchy is key (scale + recover)
✓ Location transparency (local/remote same API)
✓ Must handle network failures explicitly
✓ Testing is critical (chaos testing, load testing)
```

---

# ASSESSMENT

## By end of Week 10, you should:

### Knowledge
- [ ] Understand actor model fundamentals (isolation, message-passing, supervision)
- [ ] Can explain one-for-one vs all-for-one strategies
- [ ] Know when to use request-response vs fire-and-forget
- [ ] Understand escalation and restart limits
- [ ] Can design supervision hierarchy for service

### Skills
- [ ] Can implement basic actor
- [ ] Can build supervisor with restart logic
- [ ] Can design and test supervision trees
- [ ] Can handle failures and recovery
- [ ] Can build distributed system with actors

### Practice
- [ ] Completed 10 exercises (with tests passing)
- [ ] Solved 45+ actor model problems
- [ ] Built distributed order system
- [ ] Tested failure scenarios
- [ ] Load tested system

---

# NEXT WEEK

Week 11: Distributed Systems
- Add RPC and service discovery
- Add replication and consistency
- Add consensus algorithms
- Scale to multiple nodes
