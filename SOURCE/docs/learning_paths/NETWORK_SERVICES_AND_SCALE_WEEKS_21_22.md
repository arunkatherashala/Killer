# Week 21: Network Services - Overview
## Building connected systems with HTTP, WebSockets, RPC

**Problem**: Killer has no HTTP library, no async runtime, no socket abstractions

**Solution**: 
1. Implement HTTP parsing (manually, ~500 lines)
2. Use actor pools as fake async (fast enough!)
3. Socket programming via manual recv/send

**Outcome**: 5-service microcluster, 1000 req/sec, inter-service calls

---

# Architecture Pattern: Actor-Based HTTP Server

```
Listener (port 8080)
  ├─ Accept connection
  ├─ Spawn handler actor
  │  ├─ Read HTTP request
  │  ├─ Parse (method, path, headers, body)
  │  ├─ Route to handler
  │  ├─ Execute (blocking call or async pattern)
  │  ├─ Build response
  │  ├─ Send (with keep-alive support)
  │  └─ Close or wait for next request
  └─ Repeat
```

---

# WebSocket Pattern

```
1. HTTP GET /ws with Upgrade: websocket header
2. Server responds HTTP 101 Switching Protocols  
3. Switch to WebSocket frames (binary protocol)
4. Frames: opcode + length + payload
5. Both sides can send anytime (bidirectional)
6. Frame types: text (1), binary (2), close (8), ping (9), pong (10)
```

---

# Microservices: Service-to-Service

```
Client HTTP Request
      │
      ▼
   API Gateway
      │
      ├─→ Order Service (HTTP call)
      │   │
      │   ├─→ Auth Service (verify)
      │   └─→ DB (persist)
      │
      └─→ Response
```

**Key Challenge**: Service discovery (which IP:port?)
- Hardcode for now (easy for 5 services)
- Real: consul, etcd, k8s

---

# Success Metrics

- [ ] HTTP server handling 1000 req/sec
- [ ] WebSocket server supporting 10 concurrent clients
- [ ] 5 services communicating
- [ ] P99 latency < 200ms (include network)
- [ ] Graceful shutdown
- [ ] Error handling (invalid HTTP, closed connections)

---

---

# Week 22: Large-Scale Data Processing - Overview
## MapReduce, partitioning, stream processing

**Problem**: Process 100MB data, 100k events/sec, <100ms latency

**Solution**:
1. Partition data by key (hash sharding)
2. Map phase: process per-partition in parallel (actor pools!)
3. Reduce phase: aggregate results
4. Stream windows: tumbling/sliding windows for aggregation
5. Exactly-once semantics: deduplication + idempotence

**Outcome**: Real-time data pipeline, 100MB/sec, p99 < 100ms

---

# Architecture: Distributed MapReduce

```
Input Stream (log events)
      │
      ▼
   Partitioner (hash(key) % num)
      │
   ┌──┴───┬────┬─────┐
   ▼      ▼    ▼     ▼
 Part0  Part1 Part2 Part3
  │      │    │     │
 Actors Actors Actors Actors
  │      │    │     │
  ├─ Map ├─ Map...
  │      │
  └─ Emit ┬─ Emit
      │
      ▼
   Shuffle (group by result key)
      │
      ▼
   Reduce (aggregate)
      │
      ▼
   Output (metrics)
```

---

# Key Patterns

**Windowing**
```
Tumbling (1s non-overlapping):
[0-1s][1-2s][2-3s]
Emit at 1s, 2s, 3s boundaries

Sliding (1s window, 0.5s step, overlapping):
[0-1s]
   [0.5-1.5s]
      [1-2s]
```

**Exactly-Once**
```
Message ID + idempotent operation
→ Even if replayed, same result

Example: count
  Msg 1: {"id": 100, "count": 5}
  Replay: dedup detects id=100, skips
  Count stays 5 (not 10)
```

**Watermarks**
```
WM = "we've seen all events up to time T"
Events after WM: "late data"
Decide: include in nextwindow or drop?
```

---

# Real-Time Aggregations

```
Example: Count events per minute

Tumbling 60s windows:
  [0-60s]: 1000 events
  [60-120s]: 1100 events
  [120-180s]: 900 events

Output: {timestamp, count}
```

---

# Success Metrics

- [ ] 100MB processed
- [ ] 100k events/sec throughput
- [ ] P99 latency < 100ms
- [ ] Distributed (partitioned)
- [ ] Aggregations correct
- [ ] At-least-once or exactly-once delivered
- [ ] Handles stragglers (late data)

---

# INTEGRATION: WEEKS 19-22 CURRICULUM

```
Week 19: Actor Pools (100s-1000s concurrent)
Week 20: Real-Time (GC-free, <500µs)
Week 21: Network (HTTP, sockets, services)
Week 22: Scale (100MB/sec, 100k events/sec)

Combined: Production System
├─ Concurrent ✓ (actor pools)
├─ Real-time ✓ (no GC pauses)
├─ Networked ✓ (5-service cluster)
├─ Scalable ✓ (distributed processing)
└─ Fault-tolerant ✓ (supervision, recovery)
```

---

# PATTERN COMPARISON

| Pattern | Use Case | Latency | Throughput |
|---------|----------|---------|-----------|
| Single-threaded | Simple server | Low p50 | ~100 req/sec |
| Thread pool | Basic server | Variable | ~1000 req/sec |
| Actor pool | High concurrency | Predictable | ~10k req/sec |
| Async/await | Thousands of connections | Best | 100k req/sec |
| Distributed | Massive scale | Network + compute | 1M+ req/sec |

Killer Week 19-22:
- Actor pools: ✓ (real-time, predictable)
- ~1000 req/sec per service
- ~100k events/sec data processing
- p99 latencies very good (< 200ms)

---

# REMAINING GAPS (Post-Week 22)

→ Proposed in **Killer Enhancement Roadmap**:
- Native async runtime (v2.5, tokio integration) → 100k+ req/sec
- Generational GC (v3.2) → <5ms pauses
- FFI (v4.0) → systems programming
- Distributed framework (v3.5) → built-in MapReduce
- Native threading (v3.0) → true parallelism

---

