# Week 11 Distributed Systems Reference Guide
## Patterns, Solutions, and Real-World Strategies

---

# Part 1: Distributed Communication Patterns

## Pattern 1: RPC (Remote Procedure Call) Basics
```
Goal: Call functions on remote machines as if local

Problems solved:
  ✓ Network abstraction (send bytes, not objects)
  ✓ Serialization (convert objects to bytes)
  ✓ Deserialization (convert bytes back to objects)
  ✓ Correlation (match request to response)

Design:
  Client                              Server
    |                                  |
    |--- send RPC request -----------→|
    |    (call_id, method, args)       |
    |                                  |--- process ---
    |                                  |
    |←--- send RPC response ----------|
    |     (call_id, result)            |
    |
  Match by call_id: request #123 → response #123
```

## Pattern 2: Idempotence (Handling Duplicates)
```
Network problem: message sent twice to server
  - First try: successful
  - Retry (client doesn't know): duplicate

Solution: Idempotent operations
  PUT /users/123 { name: "Bob" }  → Safe to repeat
  POST /users { name: "Bob" }     → Creates duplicates!

Implementation:
  Track: (client_id, request_id) → (result)
  
  if cache[client_id][request_id]:
      return cached_result  // Don't redo work
  else:
      result = process(request)
      cache[client_id][request_id] = result
      return result

Cost: Extra storage for request cache
Benefit: Can safely retry without corruption
```

## Pattern 3: Serialization Formats
```
Text (JSON):
  {"user_id": 123, "name": "Bob"}
  ✓ Human readable
  ✓ Language agnostic
  ✗ Larger (more bytes)
  ✗ Slower to parse
  Example: REST APIs

Binary (Protocol Buffers):
  [0x0A, 0x03, 0x42, 0x6F, 0x62] = "Bob"
  ✓ Small (efficient)
  ✓ Fast (quick parsing)
  ✓ Schema evolution
  ✗ Not human readable
  Example: gRPC

Choice:
  - Rest/external: JSON
  - Internal/performance: binary
  - Large scale: binary (saves bandwidth)
```

---

# Part 2: Service Discovery Patterns

## Pattern 1: Client-Side Discovery
```
Problem: Where does service X live?

Solution: Client queries registry
  - Client: "Where is payment-service?"
  - Registry: "192.168.1.20:5000"
  - Client: Connects directly

Pros:
  ✓ No intermediary (low latency)
  ✓ Client chooses replica (load balance locally)
  ✗ Client needs discovery library
  ✗ Client must handle failover

Example:
  registry = ServiceRegistry::new();
  payment_service = registry.discover("payment-service");
  payment_service.call(request);
```

## Pattern 2: Server-Side Discovery
```
Problem: Client doesn't know where services are

Solution: Load balancer handles discovery
  - Client connects to load balancer
  - Balancer queries registry
  - Balancer routes to available service

Diagram:
  Client
    ↓
  Load Balancer (queries registry)
    ├→ Service1 (healthy)
    ├→ Service2 (healthy)
    └→ Service3 (sick, rejected)

Pros:
  ✓ Client simple (just connect to LB)
  ✓ Server handles complexity
  ✗ LB is point of failure
  ✗ Extra hop (latency)

Use when:
  - Mobile clients (can't run discovery)
  - Want to hide service topology
```

## Pattern 3: Self-Registration
```
When service starts:
  1. Register itself
  2. Send heartbeat
  3. Deregister on shutdown

pub struct Service {
    registry: Arc<ServiceRegistry>,
    instance: ServiceInstance,
}

impl Service {
    async fn start(&self) {
        self.registry.register(self.instance.clone());
        
        let registry = self.registry.clone();
        let instance = self.instance.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                registry.heartbeat(&instance.id);
            }
        });
    }

    async fn stop(&self) {
        self.registry.deregister(&self.instance.id);
    }
}

Pros:
  ✓ Service is source of truth
  ✓ Always accurate
  ✗ Service needs discovery library
  ✗ Heartbeat overhead
```

---

# Part 3: Consistency Patterns

## Pattern 1: Eventual Consistency (BASE)
```
Trade: Immediate consistency → Eventual consistency

Timeline:
  t=0: Write to service A
       {user.age = 30}
  t=1-100ms: Replication in flight
  t=100ms: All replicas consistent

User reads:
  - t=50ms: Might see old value (age = 29)
  - t=150ms: Always sees new value (age = 30)

Use when:
  - Acceptable to see stale data briefly
  - Social media (slight delay OK)
  - Analytics (hour-old data OK)
  
Don't use when:
  - Financial (correctness critical)
  - Inventory (overbooking risk)
```

## Pattern 2: Strong Consistency (ACID)
```
All replicas agree before returning to client

Timeline:
  Client: "Write age=30"
  System: Coordinate with all replicas
          Reach agreement
          Then: return success

Pros:
  ✓ Always correct
  ✗ Slow (coordinate + network)
  ✗ Can fail if any replica down

Use when:
  - Correctness critical (money involved)
  - Must prevent race conditions
  - Small number of replicas
```

## Pattern 3: Causal Consistency
```
Middle ground: Order must be preserved

Scenario:
  Process A: 1. Write X=1
             2. Read X (sees 1)
             3. Write Y=X+1
  
  Process B: 4. Read Y (sees 2)

Guarantee: If B reads Y=2,
           B can see the chain:
           - X was written
           - Y was written based on X
           - (causal ordering preserved)

Harder to guarantee than eventual, easier than strong
Common in: databases with vector clocks
```

---

# Part 4: Replication Patterns

## Pattern 1: Master-Slave Replication
```
Master (primary):
  - Handles ALL writes
  - Propagates changes to slaves

Slaves (secondaries):
  - Read-only copies
  - Serve read queries

Diagram:
  Write → Master ─→ Update logs
            ├────→ Slave1 (apply logs)
            ├────→ Slave2 (apply logs)
            └────→ Slave3 (apply logs)
  
  Read from Slave (load balance)

Pros:
  ✓ Strong consistency (all from one master)
  ✓ Slaves catch up eventually
  ✓ Scales reads (many slaves)
  ✗ Master is bottleneck for writes
  ✗ Single point of failure (master dies)

Failure recovery:
  Master dies → promote oldest slave to master
  Risk: Lost recent writes (replication lag)
```

## Pattern 2: Multi-Master Replication
```
Every server accepts writes
All servers replicate to all others

Diagram:
  Client1 → Master1 ──┐
                      ├→ Master2 ──┐
  Client2 → Master2 ──┤            ├→ Master3
                      ├→ Master3 ──┘
  Client3 → Master3 ──┘

Pros:
  ✓ No single point of failure
  ✓ Distributed writes
  ✓ Survive multiple failures
  ✗ Conflict resolution (both masters write same key)
  ✗ More complex

Conflict resolution needed:
  - Last-write-wins (timestamp)
  - Merge logic (vector clocks)
  - Human resolution
```

## Pattern 3: Quorum-Based Replication
```
Write to N servers
Read from M servers
Guarantee: N + M > total replicas

Example: 5 replicas total
  Write to 3 (quorum for write)
  Read from 3 (quorum for read)
  N + M = 6 > 5 ✓

Guarantee:
  Write quorum + Read quorum overlap
  → Always see latest write

Consistency:
  - If write quorum = 3, others might be stale
  - But read quorum = 3 will find latest
  - (intersection property)

Fault tolerance:
  - Can lose 2 replicas
  - Still have 3 for write quorum
  - Still have 3 for read quorum
```

---

# Part 5: Consensus Algorithms (Simplified)

## Pattern 1: Raft Consensus
```
Goal: Distributed agreement
Example: 5 servers, who is leader?

Process:
  1. Timeout: if no heartbeat from leader
  2. Candidate: send "vote for me"
  3. Followers vote (once per term)
  4. Winner: becomes leader with majority
  5. Leader: sends heartbeat to maintain authority

Key properties:
  ✓ Safety: at most one leader per term
  ✓ Availability: survives failures
  ✓ Log consistency: all replicate same log

Timeline:
  Term 1: Leader1 (3 followers)
  Leader1 dies
  Followers: timeout, become candidates
  One gets 3 votes → becomes Leader2
  Everyone committed to following Leader2
```

## Pattern 2: Compare-And-Swap (Atomic Operations)
```
Goal: Prevent race conditions

CAS(address, expected, newvalue):
  if memory[address] == expected:
      memory[address] = newvalue
      return true
  else:
      return false

Example: Banking
  balance = 100
  
  Thread A:              Thread B:
  1. CAS(bal, 100, 50)  1. CAS(bal, 100, 75)
     succeeds ✓            fails ✗
  
  Balance = 50 (only one succeeded)

Use: Distributed locks, atomic counters
```

## Pattern 3: Distributed Locks
```
Problem: Multiple processes need exclusive access

Solution: Centralized lock server
  Process1: "I need lock 'file.txt'"
  Lock server: "Granted, lease expires in 30s"
  
  Process1 does work
  Process1: "Release lock"
  
  Or: Lease expires, lock auto-released

Aha moment:
  What if lock server crashes?
  Process1 thinks it has lock, but server forgot
  Process2 gets same lock
  → Both think they have exclusive access!

Solution: Leases + generation numbers
  Lock = (server_id, generation, expit_time)
  Check lock during operation
  "Is this lock still valid?"
```

---

# Part 6: Network Challenges

## Challenge 1: Partial Failures
```
Real networks: partial failures

Scenario:
  NetworkA ←→ NetworkB
  Connection breaks
  
  A: "Can't reach B"
  B: "Can't reach A"
  
  But A's messages might be arriving (delayed)
  Or B's responses might arrive (out of order)

Solution:
  ✓ Timeouts (assume dead if no response)
  ✓ Retries (but must be idempotent)
  ✓ Heartbeats (detect link broken, not just quiet)
  ✗ Can't reliably detect true failure
```

## Challenge 2: Clock Skew
```
Machines don't have perfectly synchronized clocks

  Machine A: time = 10:00:01
  Machine B: time = 10:00:05 (4 seconds ahead)
  
  A writes X at 10:00:01
  B writes Y at 10:00:02 (B's perspective)
  
  But A thinks B's time was in future!
  
Solution:
  ✓ NTP (Network Time Protocol)
  ✓ Don't rely on timestamps for ordering
  ✓ Use logical clocks (vector clocks)
  ✗ Can't trust "earlier" from timestamps alone
```

## Challenge 3: Message Ordering
```
Network doesn't guarantee order

  A sends:  1, 2, 3
  B receives: 1, 3, 2 (different order!)

Problem:
  - Queue service: dequeue order wrong
  - State machine: apply commands out of order
  - Result: inconsistency

Solution:
  ✓ Use TCP (ordered stream)
  ✓ Sequence numbers on messages
  ✓ Reorder on receiver side
  ✓ Single connection per pair (don't parallel)
```

---

# Part 7: Common Pitfalls & Solutions

## Pitfall 1: Network is Reliable
```
WRONG: Assume message arrives exactly once

RIGHT:
  - Might arrive 0 times (lost)
  - Might arrive 1 time (normal)
  - Might arrive many times (duplicates)

Defence:
  ✓ Retry with idempotence
  ✓ Acknowledgments
  ✓ Timeout detection
  ✓ Checksums (detect corruption)
```

## Pitfall 2: Latency is Zero
```
WRONG: Assume request → response is instant

RIGHT:
  Network latency: 1-100ms (local)
  Internet latency: 100-300ms
  Intercontinental: 100-500ms
  
Design:
  ✓ Expect latency (async code, futures)
  ✓ Batch requests (amortize overhead)
  ✓ Cache results (avoid repeated calls)
  ✓ Timeout aggressively (5-30s max)
```

## Pitfall 3: Bandwidth is Infinite
```
WRONG: Send unlimited data

RIGHT:
  Network bandwidth is limited
  
Example: 1 Gbps connection
  - Can send 125 MB/sec (best case)
  - 1000 requests × 100 KB = 100 MB
  - Takes 800ms to send
  
Design:
  ✓ Compress data
  ✓ Batch messages
  ✓ Paginate large results
  ✓ Use binary format (smaller)
  ✓ Cache frequently accessed data
```

## Pitfall 4: Topology Never Changes
```
WRONG: Hardcode service addresses

RIGHT:
  Services move, scale, fail, restart
  
Solution: Service discovery
  - Automatic registration
  - Dynamic lookup
  - Handle failover

Code:
  ✓ registry.discover("payment-service")
  ✗ hardcoded "192.168.1.20:5000"
```

---

# Part 8: Real-World Distributed Systems

## Example 1: Google Spanner
```
Goal: Global database with strong consistency

Trick: TrueTime API (atomic clocks)
  - Knows exact time ± small error
  - Can order events globally
  - Commits by timestamp

Architecture:
  - Multiple datacenters
  - Strong consistency (across world)
  - Survive datacenter failures
  
Trade-offs:
  ✓ Consistent reads (correct data)
  ✗ Slow writes (global coordination)
  ✗ Expensive (atomic clocks)
```

## Example 2: DynamoDB (Eventual Consistency)
```
Design: Distributed hash table

Key insight:
  - Partition data by key
  - Each partition: multi-replica (eventual consistency)
  - Millions of partitions
  - Scales to any size

Trick: Eventual consistency
  - Write returns quickly (async replication)
  - Reads might see stale data
  - Acceptable for most use cases

Performance:
  - <10ms latency (single-digit)
  - Millions requests/sec
  - Survives datacenter failures

Trade-off:
  ✓ High performance
  ✗ Can read stale data
  ✗ Complex conflict resolution
```

## Example 3: Kafka (Distributed Queue)
```
Design: Distributed event log

How it works:
  1. Producer: append to log
  2. Log: ordered, replicated
  3. Consumer: read from offset

Scale:
  - One log → millions of partitions
  - Each partition: replicated
  - Thousands of consumers

Guarantee:
  ✓ Order within partition
  ✓ Survival (replication)
  ✗ Global order (across partitions "no)

Use cases:
  - Event streaming
  - Log aggregation
  - Microservice communication
```

---

# Part 9: Debugging Distributed Systems

## Technique 1: Request IDs (Tracing)
```
Assign unique ID to request:
  
  Client: req-id = UUID()
  Send HTTP request with X-Request-ID header
  
  Service A: receives req-id
    Logs: [req-id] "started processing"
    Calls Service B: passes X-Request-ID
    
  Service B: receives same req-id
    Logs: [req-id] "called from A"
    Returns response
    
  Service A: completes
    Logs: [req-id] "done"

Debugging:
  $ grep "req-id-abc123" /var/log/* | sort by time
  Ready: full request flow across all services!
```

## Technique 2: Message Capture
```
Capture all network messages:

  wireshark (packet analyzer)
  curl -v (show headers)
  tcpdump (network sniffer)

After failure:
  1. Capture messages during incident
  2. Replay messages locally
  3. Debug with full visibility

Example:
  tcpdump -i eth0 -w capture.pcap
  (run test case)
  wireshark capture.pcap
  (inspect messages, ordering, timeouts)
```

## Technique 3: Chaos Engineering
```
Intentionally break systems to find problems:

Common experiments:
  - Kill random service (can system recover?)
  - Introduce 500ms latency (how breaks?)
  - Fill disk 90% (graceful degradation?)
  - Network partition (who notices?)
  - Slow database (cascades to others?)

Benefits:
  ✓ Find issues before production
  ✓ Train team for real failures
  ✓ Build confidence
  ✓ Break fast (controlled environment)

Tools: Gremlin, AWS FIS, Kubernetes chaos monkey
```

---

# Part 10: Production Checklist

Before shipping distributed system:

### Network Resilience
- [ ] Timeout on all RPC calls (5-30 seconds)
- [ ] Retry logic with exponential backoff
- [ ] Idempotent operations (safe to retry)
- [ ] Circuit breakers (fail fast, not cascade)
- [ ] Bulkheads (isolate failures)

### Monitoring
- [ ] Request IDs for tracing
- [ ] Latency percentiles (p50, p95, p99)
- [ ] Error rates and types
- [ ] Service dependencies (call graph)
- [ ] Alerts on SLO violations

### Data Consistency
- [ ] Chose consistency model (eventual/strong/causal)
- [ ] Replication strategy tested
- [ ] Conflict resolution documented
- [ ] Backup/restore tested

### Operations
- [ ] Multi-region failover tested
- [ ] Service discovery working
- [ ] Graceful shutdown (1 minute timeout)
- [ ] Capacity planning (traffic growth)
- [ ] Load testing (2x expected traffic)

### Security
- [ ] Encryption in transit (HTTPS/TLS)
- [ ] Authentication (who are you?)
- [ ] Authorization (what can you do?)
- [ ] Rate limiting (prevent abuse)
- [ ] Audit logging (who did what when?)

---

**Key Insight**: Distributed systems are hard because network is unreliable. Design for failure, not for the happy path. Every assumption is a liability. Test mercilessly.
