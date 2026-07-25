# Week 11: Distributed Systems & RPC - Complete Learning Module
## Network-based actor systems and inter-node communication
**Target: 140+ problems | ~420 hours | Expert Level**

---

## Module Overview

### Learning Objectives
By end of Week 11, you will:
- ✓ Understand network protocols for actors
- ✓ Implement RPC (Remote Procedure Call)
- ✓ Build multi-node actor systems
- ✓ Handle network partitions
- ✓ Implement consensus algorithms (Raft, Paxos concepts)
- ✓ Design distributed transactions

### Core Topics
1. **Network Communication** (35 problems)
2. **RPC & Remote Actors** (35 problems)
3. **Clustering & Discovery** (30 problems)
4. **Consensus & Coordination** (25 problems)
5. **Distributed Transactions** (15 problems)

---

## CATEGORY 1: NETWORK COMMUNICATION (35 problems)

### 1.1 Serialization & Protocols

**Problems:**
```
1.1.1: Serialize actor message to bytes
1.1.2: Deserialize bytes to message
1.1.3: Binary protocol (fixed format)
1.1.4: Text protocol (JSON-like)
1.1.5: Protocol versioning (forward/backward compat)
1.1.6: Compression (reduce network traffic)
1.1.7: Encryption (secure messages)
1.1.8: Message signing (authentication)
1.1.9: Checksum verification
1.1.10: Handle corrupt messages
```

### 1.2 Transport Layers

**Problems:**
```
1.2.1: TCP connection management
1.2.2: UDP for low-latency
1.2.3: HTTP for web integration
1.2.4: WebSocket for bidirectional
1.2.5: Connection pooling
1.2.6: Backpressure (slow receiver)
1.2.7: Flow control
1.2.8: Timeout on send
1.2.9: Retry on network failure
1.2.10: Circuit breaker (stop trying if network down)
```

### 1.3 Message Delivery Guarantees

**Problems:**
```
1.3.1: At-most-once (may lose)
1.3.2: At-least-once (may duplicate)
1.3.3: Exactly-once (hard!)
1.3.4: Ordered delivery
1.3.5: Delivery confirmation
1.3.6: Acknowledgment protocol
1.3.7: Timeout and retry
1.3.8: Deduplication (detect and drop duplicates)
1.3.9: Sequence numbers
1.3.10: Out-of-order message handling
```

### 1.4 Network Failures

**Problems:**
```
1.4.1: Connection timeout detection
1.4.2: Heartbeat mechanism
1.4.3: Graceful shutdown vs network failure
1.4.4: Network partition detection
1.4.5: Recovery from transient failure
1.4.6: Permanent failure vs temporary
1.4.7: Partial message loss
1.4.8: Message duplication detection
1.4.9: Reordered messages
1.4.10: Cascade failures across network
```

---

## CATEGORY 2: RPC & REMOTE ACTORS (35 problems)

### 2.1 RPC Fundamentals

**Problems:**
```
2.1.1: Call remote actor like local
2.1.2: Request-response over network
2.1.3: Correlation ID matching (responses)
2.1.4: Timeout on remote call
2.1.5: Exception propagation (remote error)
2.1.6: Return value serialization
2.1.7: Parameter serialization
2.1.8: Fire-and-forget call
2.1.9: Streaming responses
2.1.10: Bidirectional RPC
```

### 2.2 Remote Actor References

**Problems:**
```
2.2.1: Address format (host:port/path)
2.2.2: Resolve remote actor address
2.2.3: Connect to remote system
2.2.4: Persistable references
2.2.5: Weak vs strong references
2.2.6: Reference equality
2.2.7: Clone reference for sending
2.2.8: Stale reference detection
2.2.9: Dynamic address discovery
2.2.10: Local vs remote actor abstraction
```

### 2.3 Distribution Transparency

**Problems:**
```
2.3.1: Local actor calls same code as remote
2.3.2: Transparency breaks on network error
2.3.3: Latency exposed (timeouts)
2.3.4: Partial failure (can't tell if processed)
2.3.5: Fallacy: network is reliable
2.3.6: Assume network unreliable
2.3.7: Design for network failure
2.3.8: Metrics: network vs local call latency
2.3.9: Debug network vs local issue
2.3.10: Observability across network
```

### 2.4 GeoDistribution

**Problems:**
```
2.4.1: Actor in different datacenter
2.4.2: Latency across regions (100ms+)
2.4.3: Minimize cross-region calls
2.4.4: Local cache (reduce remote calls)
2.4.5: Eventual consistency (not strong)
2.4.6: Conflict resolution
2.4.7: Multi-master replication
2.4.8: CRDT data structures
2.4.9: Geo-redundancy
2.4.10: Disaster recovery (region failure)
```

---

## CATEGORY 3: CLUSTERING & DISCOVERY (30 problems)

### 3.1 Cluster Formation

**Problems:**
```
3.1.1: Join cluster at startup
3.1.2: Discover cluster nodes
3.1.3: Seed nodes for bootstrap
3.1.4: Dynamic nodes (add/remove)
3.1.5: Cluster membership
3.1.6: Gossip protocol updates
3.1.7: Cluster state awareness
3.1.8: Leave cluster gracefully
3.1.9: Detect node failure
3.1.10: Network split handling
```

### 3.2 Service Discovery

**Problems:**
```
3.2.1: Register service at startup
3.2.2: Locate service by name
3.2.3: Load balance across instances
3.2.4: Health checking
3.2.5: Deregister on failure
3.2.6: DNS integration
3.2.7: Dynamic configuration
3.2.8: Service versioning
3.2.9: Canary deployment
3.2.10: Blue-green deployment
```

### 3.3 Replication & Sharding

**Problems:**
```
3.3.1: Replicate actor across nodes
3.3.2: Primary-backup replication
3.3.3: Read from replicas
3.3.4: Failover to replica
3.3.5: Shard by key (partition data)
3.3.6: Consistent hashing
3.3.7: Rebalance shards
3.3.8: Shard migration
3.3.9: Multi-shard transaction
3.3.10: Shard failure recovery
```

---

## CATEGORY 4: CONSENSUS & COORDINATION (25 problems)

### 4.1 Distributed Agreement

**Problems:**
```
4.1.1: Consensus on state change
4.1.2: Majority quorum
4.1.3: Byzantine tolerant
4.1.4: Raft consensus basics
4.1.5: Paxos algorithm basics
4.1.6: Leader election
4.1.7: Heartbeat mechanism (leader liveness)
4.1.8: Log replication
4.1.9: Commit safety (majority acked)
4.1.10: State machine
```

### 4.2 Distributed Locks

**Problems:**
```
4.2.1: Lock across multiple nodes
4.2.2: Mutual exclusion
4.2.3: Prevent deadlock
4.2.4: Timeout on lock acquisition
4.2.5: Fair scheduling
4.2.6: Reader-writer locks
4.2.7: Optimistic locking
4.2.8: Lock poisoning
4.2.9: Distributed transaction lock
4.2.10: Lease-based locks
```

### 4.3 Single Point of Coordination

**Problems:**
```
4.3.1: Central coordinator
4.3.2: Coordinator failure detection
4.3.3: Coordinator election
4.3.4: State propagation from coordinator
4.3.5: Two-phase commit (coordinator)
4.3.6: Compensating transactions
4.3.7: Timeout and recovery
4.3.8: Coordination state persistence
4.3.9: Coordinator unavailability handling
4.3.10: Decentralization alternatives
```

---

## CATEGORY 5: DISTRIBUTED TRANSACTIONS (15 problems)

### 5.1 Transaction Coordination

**Problems:**
```
5.1.1: ACID properties across network
5.1.2: Two-phase commit protocol
5.1.3: Prepare phase (can we commit?)
5.1.4: Commit/abort phase
5.1.5: Failure recovery
5.1.6: Timeout handling
5.1.7: Cascading aborts
5.1.8: Deadlock detection
5.1.9: Rollback to known state
5.1.10: Sagas (long-running transactions)
```

### 5.2 Eventual Consistency

**Problems:**
```
5.2.1: Accept temporary inconsistency
5.2.2: Conflict detection
5.2.3: Conflict resolution strategy
5.2.4: CRDT data types
5.2.5: Last-write-wins ordering
```

---

## Weekly Schedule

**Monday:** Network Communication (30 hours)
- Serialization, protocols, transport
- Message delivery guarantees
- Problems 1.1-1.4

**Tuesday:** RPC & Remote Actors (30 hours)
- RPC fundamentals
- Remote references
- Distribution transparency
- Problems 2.1-2.4

**Wednesday:** Clustering & Discovery (30 hours)
- Cluster formation
- Service discovery
- Replication & sharding
- Problems 3.1-3.3

**Thursday:** Consensus & Coordination (30 hours)
- Distributed agreement
- Consensus algorithms overview
- Distributed locks
- Problems 4.1-4.3

**Friday:** Transactions & Capstone (30 hours)
- Distributed transactions
- Build multi-node system
- Problems 5.1-5.2

---

## Key Concepts

### CAP Theorem
- **Consistency:** All nodes see same state
- **Availability:** System always responsive
- **Partition Tolerance:** Survive network split

You can only guarantee 2 of 3.

### Challenges
1. **Partial Failure:** Some nodes fail, others work
2. **Asynchrony:** Messages delayed unpredictably
3. **Network Partition:** Nodes can't communicate
4. **Clock Skew:** Nodes have different times
5. **Byzantine Failures:** Nodes lie/behave unexpectedly

### Real Examples
**Google Spanner:** Strong consistency across datacenters (sacrifices availability in partitions)
**Amazon DynamoDB:** Eventual consistency, high availability
**Kafka:** Partitioned clusters with replication
**Consul:** Service discovery with consensus
**etcd:** Distributed key-value with Raft

---

## Integration with Week 12

Week 12 adds contracts/verification:
- Verify distributed invariants
- Contract programming for RPC
- Preconditions on remote calls
- Postconditions with eventual consistency

---

## Success Criteria

By end of Week 11, you should:
- [ ] Build multi-node actor system
- [ ] Implement RPC layer
- [ ] Handle network failures gracefully
- [ ] Implement service discovery
- [ ] Understand consensus algorithms
- [ ] Build distributed transaction system
- [ ] Debug distributed systems
- [ ] Achieve fault tolerance

**Mastery = Design and build resilient, scalable distributed systems**
