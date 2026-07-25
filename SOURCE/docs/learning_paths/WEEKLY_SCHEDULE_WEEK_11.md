# Week 11 Weekly Schedule: Distributed Systems Fundamentals
## 75 Hours | Building Systems Across Networks

---

# WEEKLY OVERVIEW

**Monday**: Network Communication & Serialization (15h)
**Tuesday**: RPC & Service Discovery (15h)
**Wednesday**: Replication & Consistency (15h)
**Thursday**: Consensus & Sharding (15h)
**Friday**: Capstone - Multi-Node Distributed System (15h)

**Time Allocation**
- Concepts & Theory: 25 hours (33%)
- Hands-on Exercises: 35 hours (47%)
- Capstone Project: 15 hours (20%)

---

# MONDAY: NETWORK COMMUNICATION & SERIALIZATION (15 hours)

## 09:00-11:00 | Network Challenges (2h)

**Concepts**
```
Distributed systems problem: network is unreliable

Key insights:
  1. Messages can be lost (network partition)
  2. Messages can be delayed (high latency)
  3. Messages can be duplicated (retry on timeout)
  4. Messages can arrive out of order (UDP packets)
  5. Machines have different clocks (clock skew)

Design consequence:
  CAN'T assume reliable, ordered, duplicate-free delivery
  MUST design for failures
```

**Problems** (11.1.1-10)
```
- Identify network failure scenarios
- Design for partial failures
- Handle latency (timeouts)
- Detect dead services
- Acknowledge delivery
```

### 11:00-13:00 | Serialization & Message Formats (2h)

**Text Format (JSON)**
```
{"user_id": 123, "name": "Alice"}
Pros: human readable, language agnostic
Cons: larger size, slower parsing
```

**Binary Format (Protocol Buffers)**
```
[0x08, 0x7B, 0x12, 0x05, 0x41, 0x6C, 0x69, ...]
Pros: smaller, faster, schema evolution
Cons: not human readable
```

**Exercise 1: SerializedValue** (from distributed_systems_exercises.rs)
```rust
pub enum SerializedValue {
    Integer(i64),
    String(String),
    Boolean(bool),
    Bytes(Vec<u8>),
    Array(Vec<SerializedValue>),
    Map(HashMap<String, SerializedValue>),
}

// Implement: serialize() and deserialize()
// Support both types
```

**Problems** (11.1.11-20)
```
- Choose format for your system
- Implement serialization
- Handle schema evolution
- Verify round-trip (serialize then deserialize)
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Message Passing Protocols (3h)

**Request-Response Pattern**
```
Client sends request:
  {call_id: "call-1", method: "GetUser", args: [123]}

Server responds:
  {call_id: "call-1", result: {name: "Alice"}}

Challenge:
  Match response to request using call_id
  What if response never comes? (timeout)
```

**Idempotence (Handling Duplicates)**
```
Network problem: send twice → received twice

Solution: idempotent operations
  PUT /users/123 {...} → safe to repeat
  POST /orders {...}  → creates duplicates!

Implementation:
  track_request(client_id, request_id) → cache result
  if already seen: return cached result
  else: process and cache result
```

**Exercise 2: RpcRequest & RpcResponse** (from distributed_systems_exercises.rs)
```rust
pub struct RpcRequest {
    call_id: String,
    method: String,
    args: Vec<SerializedValue>,
}

pub struct RpcResponse {
    call_id: String,
    result: Result<SerializedValue, String>,
}

// Implement: correlation by call_id
// Test: send 3 requests, responses can arrive out of order
```

**Problems** (11.1.21-30)
```
- Design RPC protocol for your service
- Handle duplicate requests
- Implement timeout detection
- Plan retry strategy (exponential backoff)
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercises 1-2
- Test serialization/deserialization
- Test RPC correlation

### 18:00-19:00 | Problem Set 11.1 (1h)

---

# TUESDAY: RPC & SERVICE DISCOVERY (15 hours)

## 09:00-11:00 | Service Discovery Basics (2h)

**The Problem**
```
Service A wants to call Service B

Bad: hardcoded address
  const B_ADDRESS = "192.168.1.20:5000";
  Problem: service moves, crashes, restarts

Better: dynamic discovery
  address = registry.discover("service-b");
  address = "192.168.1.25:5000";  (new machine)
```

**Service Registry**
```
Central database of services:
  {
    "user-service": [
      {id: "user-1", host: "192.168.1.10", port: 8080, healthy: true},
      {id: "user-2", host: "192.168.1.11", port: 8080, healthy: true},
    ],
    "payment-service": [
      {id: "payment-1", host: "192.168.1.20", port: 5000, healthy: false},
    ]
  }
```

**Exercise 3: ServiceInstance & ServiceRegistry** (from distributed_systems_exercises.rs)
```rust
pub struct ServiceInstance {
    id: String,
    name: String,
    address: String,
    port: u16,
    metadata: HashMap<String, String>,
}

pub struct ServiceRegistry {
    services: HashMap<String, Vec<ServiceInstance>>,
    health_checks: HashMap<String, bool>,
}

// Implement: register, deregister, discover, discover_healthy
```

**Problems** (11.2.1-10)
```
- Register service on startup
- Deregister on shutdown
- Discover by name
- Filter by health status
- Handle missing services
```

### 11:00-13:00 | RPC & Client Library (2h)

**RPC Client**
```
pub struct RpcClient {
    service_address: String,
    pending_calls: HashMap<String, Future>,
    next_call_id: u64,
}

impl RpcClient {
    pub async fn call(&self, method, args) -> Result {
        let call_id = next_call_id();
        let request = RpcRequest { call_id, method, args };
        send_network(request);
        
        let response = wait_for_response(call_id, timeout: 5s)?;
        Ok(response.result)
    }
}
```

**Exercise 4: RpcClient & RpcRegistry** (from distributed_systems_exercises.rs)
```rust
pub struct RpcClient {
    service_address: String,
    pending_calls: Arc<Mutex<HashMap<String, /* Future */>>>,
}

pub struct RpcRegistry {
    methods: HashMap<String, Box<dyn Fn(Vec<SerializedValue>) -> SerializedValue>>,
}

// Implement: call() with correlation
// Implement: registry for registering handlers
```

**Problems** (11.2.11-20)
```
- Discover service from registry
- Call RPC method
- Handle timeout
- Retry on failure
- Connect to different services
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Load Balancing & Failover (3h)

**Load Balancing Strategy: Round-Robin**
```
Clients
  ├→ Service Instance 1
  ├→ Service Instance 2 (next request)
  ├→ Service Instance 3 (next request)
  └→ Service Instance 1 (next request, cycles back)

Simple, fair distribution across available instances
```

**Load Balancing Strategy: Least-Loaded**
```
Track: requests in flight per instance
Route to instance with fewest requests

Pros: Better latency (don't hit congested instance)
Cons: More complexity (track load)
```

**Failover**
```
1. Discover healthy instances
2. Send to instance 1
3. If timeout: mark unhealthy
4. Discover again (excludes unhealthy)
5. Send to instance 2
6. If success: done
7. If all fail: return error

Backoff: don't retry immediately
  1st try: immediate
  2nd try: wait 100ms
  3rd try: wait 1000ms
  4th try: wait 10000ms
  5th try: give up
```

**Exercise 5: Service Discovery Health & Load Balancing** (from distributed_systems_exercises.rs)
```rust
// Already implemented in ServiceRegistry
pub fn discover_healthy(&self, service_name: &str) -> Vec<ServiceInstance> {
    // Filter by health status
}

pub fn mark_healthy(&self, instance_id: &str, healthy: bool) {
    // Update health status
}

// For load balancing, client picks from healthy list
// Round-robin or least-loaded algorithm
```

**Problems** (11.2.21-35)
```
- Implement round-robin load balancing
- Implement least-loaded load balancing
- Detect unhealthy services (slow response)
- Failover to another instance
- Exponential backoff on retry
- Handle all instances down (circuit breaker)
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercises 3-5
- Test service discovery
- Test RPC client
- Test load balancing

### 18:00-19:00 | Problem Set 11.2 (1h)

---

# WEDNESDAY: REPLICATION & CONSISTENCY (15 hours)

## 09:00-11:00 | Replication Basics (2h)

**Why Replicate?**
```
Single server: data loss risk (disk failure)
Replicated: data survives server failure

Trade-off: consistency vs. availability
  Strong consistency: all replicas agree (slower)
  Eventual consistency: replicas catch up (faster)
```

**Master-Slave Replication**
```
Master (primary)
  ├─ accepts writes
  └─ sends update log to slaves

Slaves (secondaries)
  ├─ apply update log
  └─ serve read queries

Pros: strong consistency, scales reads
Cons: master is bottleneck for writes, single point of failure
```

**Problems** (11.3.1-10)
```
- Design replication strategy
- Identify consistency requirements
- Plan failover procedure
- Handle replication lag
```

### 11:00-13:00 | Causality & Vector Clocks (2h)

**The Problem**
```
Messages can arrive out of order by network delays

Timeline:
  Writer: sends "balance=100"
  Network: delayed 1 second
  
  Reader: "What's balance?" (at 0.5s)
  Reader sees: "balance=50" (stale!)

How to detect causality? (what happened first?)
```

**Vector Clocks**
```
Track: (node_A: 5, node_B: 3, node_C: 7)

Increment your clock when you send/receive
Merge clocks to see causal relationship

A happens before B if:
  A_clock < B_clock (element-wise)
  AND at least one element is less
```

**Exercise 6: VectorClock and ReplicatedValue** (from distributed_systems_exercises.rs)
```rust
pub struct VectorClock {
    clock: HashMap<String, u64>,
}

impl VectorClock {
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        // Compare element-wise
        // all_less_or_equal && some_less
    }
    
    pub fn merge(&self, other: &VectorClock) -> VectorClock {
        // Take maximum of each element
    }
}

pub struct ReplicatedValue<T> {
    value: T,
    version: VectorClock,
}

// Implement: update with new clock
// Track which replicas have seen which versions
```

**Problems** (11.3.11-20)
```
- Implement vector clock
- Track causal relationships
- Detect concurrent updates (conflicts)
- Merge conflicting versions
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Consistency Models (3h)

**Consistency Spectrum**

Strongest (Slowest)
```
1. Strong Consistency (ACID)
   All see latest write before any read
   All nodes agree before commit
   Slow: coordinate across replicas
   
2. Read-Your-Writes
   You read your own writes
   Others might see stale
   
3. Session Consistency
   Within session: read-your-writes
   Cross-session: eventual consistency
   
4. Causal Consistency
   If A happens before B, all see A then B
   But unrelated events can be out of order
   
5. Eventual Consistency (BASE)
   Some delay, but eventually all consistent
   Fastest: no coordination needed
```

**Choice Framework**
```
Choose based on:
  ✓ Money involved? → Strong (bank transfers)
  ✓ Correctness critical? → Strong/Causal
  ✓ Can tolerate stale reads? → Eventual

Example:
  User profile: eventual ok (see old name briefly)
  Payment: must be strong (can't double-charge)
  Inventory: causal (don't oversell)
```

**Problems** (11.3.21-35)
```
- Choose consistency for your system
- Design replication with chosen consistency
- Handle read stale scenarios
- Implement conflict resolution
- Test consistency guarantees
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercise 6
- Test vector clocks
- Test replication semantics

### 18:00-19:00 | Problem Set 11.3 (1h)

---

# THURSDAY: CONSENSUS & SHARDING (15 hours)

## 09:00-11:00 | Consensus Basics (2h)

**The Problem**
```
Multiple servers, must agree on decision

Example: Who is leader?
  Server A: "I'm leader"
  Server B: "No, I'm leader"
  → Conflict!

Solution: Consensus algorithm (like Raft)
  All agree on same leader
  Even if some fail
```

**Raft Consensus Simplified**
```
Leaders convince followers:
  1. Candidate: sends "vote for me"
  2. Followers: vote for first candidate in term
     (at most one gets majority votes)
  3. Winner: becomes leader for this term
  4. Leader: sends heartbeat to maintain authority
  
If leader fails:
  Timeout: followers start new election
  New leader elected
```

**Exercise 7: LeaderElection** (from distributed_systems_exercises.rs)
```rust
pub struct LeaderElection {
    state: FollowerState,  // Follower, Candidate, Leader
    votes_received: u32,
    election_term: u64,
}

impl LeaderElection {
    pub fn become_candidate(&mut self) {
        // Vote for self, increment term
    }
    
    pub fn receive_vote(&mut self) {
        // Count votes, maybe become leader
    }
    
    pub fn receive_heartbeat(&mut self, leader_term: u64) {
        // Update if leader_term >= my term
    }
}

// Implement: state machine for leader election
```

**Problems** (11.4.1-10)
```
- Simulate election with multiple servers
- Elect leader by majority
- Detect failure (timeout)
- Re-elect new leader
- Handle split brain (partition)
```

### 11:00-13:00 | Sharding & Partitioning (2h)

**The Problem**
```
Data too large for single server
Solution: divide data across shards

By key hash:
  Key "Alice" → Shard 1
  Key "Bob"   → Shard 2
  Key "Carol" → Shard 3
  
Benefit:
  ✓ Scale data across many servers
  ✓ Queries routed to right shard
  ✗ Cross-shard queries difficult
  ✗ Rebalancing complex (reshard)
```

**Shard Selection**
```
Consistent Hashing:
  hash(key) % num_shards → shard_id
  
Problem: add/remove shard → all keys rehash!
Solution: consistent hashing
  Only nearby keys affected when shard added
```

**Exercise 8: ShardMap** (from distributed_systems_exercises.rs)
```rust
pub struct ShardMap<K: ShardKey, V> {
    shards: Vec<BTreeMap<K, V>>,
    num_shards: u32,
}

pub trait ShardKey {
    fn shard_id(&self, num_shards: u32) -> ShardId;
}

// Implement for String and u64
// insert, get, remove should route to correct shard
```

**Problems** (11.4.11-20)
```
- Shard data by key
- Route queries to correct shard
- Rebalance (add new shard)
- Handle shard failure
- Cross-shard aggregation
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Distributed Transactions (3h)

**Single-Shard Transaction**
```
All data in one shard
Use local ACID transaction
Simple and fast
```

**Multi-Shard Transaction**
```
Data spread across shards

Two-Phase Commit:
  Phase 1 (Prepare):
    Coordinator asks: "Can you commit?"
    Each shard: "Yes/No"
  
  Phase 2 (Commit):
    If all "Yes": commit on all
    If any "No": rollback on all

Problem: slow (2 rounds of messaging)
Problem: blocking (locks held during wait)
```

**Saga Pattern** (from Week 15)
```
For business transactions:
  1. Execute local transaction
  2. Publish event
  3. Next service listens, does its part
  4. On failure: publish compensation event
  5. Other services rollback

Benefit:
  ✓ No blocking
  ✓ Eventual consistency
  ✗ Complex compensation logic
```

**Problems** (11.4.21-30)
```
- Single-shard transaction
- Cross-shard coordination
- Handle timeout mid-transaction
- Rollback on failure
- Deadlock prevention
- Saga pattern for business flows
```

### 17:00-18:00 | Exercise Lab (1h)
- Complete Exercises 7-8
- Test leader election
- Test sharding routing

### 18:00-19:00 | Problem Set 11.4 (1h)

---

# FRIDAY: CAPSTONE - MULTI-NODE DISTRIBUTED SYSTEM (15 hours)

## 09:00-11:00 | Design Phase (2h)

**Requirements**
```
Build 3-server distributed database:

Features:
  ✓ Create, read, update key-value pairs
  ✓ Replicate data across 3 servers
  ✓ Leader-based replication (one primary)
  ✓ Automatic failover on leader crash
  ✓ Service discovery (register/discover)
  ✓ RPC protocol for communication

Load:
  - 1000 requests/sec
  - 100 concurrent clients
  - 99% latency < 100ms
  - Survive 1 server failure

Consistency model:
  - Strong for master
  - Eventual for slaves
  (or choose your model)
```

**Architecture**
```
      Client Application
             |
      Service Registry
             |
      Load Balancer
             |
    ┌────────┼────────┐
    |        |        |
  Server1  Server2  Server3
  (leader)  (replica) (replica)
    |        |        |
    └────────┼────────┘
    Replication log
```

**Design Tasks** (1h)
```
- Draw architecture
- Choose consistency model
- Design replication protocol
- List failure scenarios (10+)
- Design failover procedure
- Plan testing strategy
```

## 11:00-13:00 | Implementation Phase (2h)

**Build Components**
```
1. Service instances (3 servers)
   - Store key-value data
   - Handle RPC requests
   - Send replication logs

2. Leader election
   - Detect leader failure (timeout)
   - Vote for new leader
   - Majority wins

3. Replication
   - Leader sends update log
   - Followers apply updates
   - Acknowledgments

4. Service discovery
   - Register instances
   - Mark unhealthy instances
   - Clients discover leaders
```

**Code Structure** (from distributed_systems_exercises.rs)
```rust
// Already have:
//   ServiceInstance, ServiceRegistry
//   RpcRequest, RpcResponse, RpcClient
//   VectorClock, ReplicatedValue
//   LeaderElection
//   ShardMap

// Need to combine these for capstone:
struct DistributedStore {
    data: ShardMap<String, SerializedValue>,
    registry: ServiceRegistry,
    election: LeaderElection,
}

impl DistributedStore {
    fn handle_request(&mut self, req: RpcRequest) {
        match req.method {
            "put" => self.put(args[0], args[1]),
            "get" => self.get(args[0]),
            "replicate" => self.apply_log_entry(args[0]),
        }
    }
}
```

### 13:00-14:00 | LUNCH (1h)

### 14:00-17:00 | Testing & Verification (3h)

**Integration Tests**
```
Test 1: Basic operations
  put("key1", "val1")
  get("key1") → "val1"

Test 2: Replication
  put on master
  verify appears on replicas within 100ms

Test 3: Leader failure
  Master crashes
  Election happens → new leader elected
  Clients redirect to new leader
  Reads return consistent data

Test 4: Partial failure
  Replica slow (100ms latency)
  Other replica responds fine (< 50ms)
  Master times out → health marked unhealthy
  Next discovery skips unhealthy

Test 5: Network partition
  Master isolated (can't see replicas)
  Election: replicas elect new leader
  Master can't accept writes (minority)
  Partition heals: divergence resolved
```

**Load Test** (1h)
```
Simulate 100 concurrent clients
Each sends 10 requests:
  50% read, 50% write

Measure:
  - Throughput (req/sec)
  - Latency (p50, p95, p99)
  - Error rate
  - Replication lag
  - Recovery time after failure

Target:
  - 1000 req/sec
  - p99 < 100ms
  - Error < 0.1%
  - Replication lag < 10ms
  - Recovery < 5 seconds
```

**Failure Injection** (1h)
```
Chaos testing:
  1. Kill random server
     → Verify other 2 still work
     → Verify automatic failover
  
  2. Slow down network (500ms latency)
     → Verify timeouts work
     → Verify load balancer fails over
  
  3. Partition network (A-B isolated)
     → Verify split-brain prevention
     → Verify minority stops accepting writes
  
  4. Reorder messages
     → Verify vector clocks catch it
     → Verify consistency maintained
```

### 17:00-19:00 | Presentation & Retrospective (2h)

**Present Your System** (30 min)
```
- Architecture diagram
- Replication protocol
- Leader election mechanism
- Failure scenarios handled
- Load test results
- Production readiness assessment
```

**Retrospective** (30 min)
```
What worked well?
What was challenging?
How would you scale further?
What's needed for production?
  - Persistent state (WAL journal)
  - Monitoring (metrics, logs)
  - Observability (tracing, debugging)
  - Security (encryption, auth)
  - Operations (helm chart, runbooks)

Next steps:
  Add persistence (don't lose data)
  Add sharding (partition by key)
  Add consensus (Raft vs Paxos)
  Add conflict resolution (CRDTs)
```

**Key Learnings**
```
✓ Network is fundamentally unreliable
✓ Must handle partial failures
✓ Consensus is essential for multi-server
✓ Replication has latency overhead
✓ Consistency vs availability trade-off
✓ Service discovery is critical
✓ Monitoring is non-negotiable
```

---

# ASSESSMENT

## By end of Week 11, you should:

### Knowledge
- [ ] Understand network challenges and limitations
- [ ] Know RPC patterns and serialization
- [ ] Understand service discovery and load balancing
- [ ] Know replication strategies (master-slave, multi-master)
- [ ] Understand consistency models
- [ ] Know consensus algorithms (Raft)
- [ ] Understand sharding and partitioning

### Skills
- [ ] Can design RPC protocol
- [ ] Can implement service discovery
- [ ] Can build replicated system
- [ ] Can handle network failures
- [ ] Can implement leader election
- [ ] Can shard data by key
- [ ] Can coordinate across shards

### Practice
- [ ] Completed 8 exercises (with tests)
- [ ] Solved 35+ distributed systems problems
- [ ] Built 3-server distributed database
- [ ] Tested failure scenarios
- [ ] Load tested system
- [ ] Verified consistency guarantees

---

# NEXT STEPS

After Week 11:
- You understand foundational distributed systems
- Weeks 12-14: Add formal contracts for correctness
- Weeks 15-18: Scale to production (Kubernetes, monitoring, etc.)

Path to deeper expertise:
- Consensus algorithms (Raft, Paxos, Zookeeper)
- Event sourcing and CQRS
- Streaming systems (Kafka, Flink)
- Data replication (database internals)
- Byzantine fault tolerance
