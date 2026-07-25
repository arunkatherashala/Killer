# Week 22 Weekly Schedule: Large-Scale Data Processing
## 75 Hours | Distributed Patterns, MapReduce, Stream Processing, Scale

---

# OVERVIEW
Final week combines everything: actor pools (Week 19), real-time patterns (Week 20), network services (Week 21) into large-scale distributed data system.

**Challenge**: Process 100MB of data in real-time with < 100ms latency p99.

**Solution**: MapReduce-style processing across actor pool, streaming pipelines, distributed aggregation.

---

# WEEKLY STRUCTURE

**Monday (15h)**: Data Partitioning & Distribution
- Hash-based sharding, consistent hashing, range partitioning
- Problems 22.1.1-30: Partition creation, routing, rebalancing, hot-spot detection

**Tuesday (15h)**: MapReduce & Aggregation
- Map phase (per-partition), reduce phase (aggregate), combine
- Problems 22.2.1-30: Word count pipeline, aggregation, distributed sort, custom reducers

**Wednesday (15h)**: Stream Processing & Windowing
- Tumbling windows, sliding windows, session windows
- Problems 22.3.1-30: Real-time aggregation, watermarks, late data handling, joins

**Thursday (15h)**: Distributed Consensus & Fault Tolerance
- Replicated state machines, consensus, recovery
- Problems 22.4.1-30: Replication, failover, consistency checking

**Friday (15h)**: Capstone - Real-Time Data Pipeline
- Ingest 100MB/sec, process, aggregate, output results
- All in < 100ms latency p99, 100k events/sec throughput

---

# ARCHITECTURE

```
Data Source (log stream, events)
      │
      ▼
   Buffer (bounded queue, batching)
      │
      ▼
┌─────────────────────────────────┐
│ Distributed Processor           │
├─────────────────────────────────┤
│ Partition 1      Partition 2    │ (hash(key) % num_partitions)
│ Actor Pool 1     Actor Pool 2   │ (10 actors each)
│ ├─ Map           ├─ Map         │
│ ├─ Filter        ├─ Filter      │
│ └─ Emit events   └─ Emit events │
└─────────────────────────────────┘
      │
      ▼
┌─────────────────────────────────┐
│ Aggregation                     │
├─────────────────────────────────┤
│ Count aggregator                │
│ Sum aggregator                  │
│ Top-N aggregator                │
└─────────────────────────────────┘
      │
      ▼
   Output (metrics, alerts)
```

---

# KEY PATTERNS

**MapReduce Pipeline**
```
Map:  input → emit(key, value)
Shuffle: group by key
Reduce: combine all values for each key
```

**Windowing**
```
Tumbling (1 second windows):
  [0-1s]  [1-2s]  [2-3s]
   emit   emit    emit

Sliding (1 second window, 0.5s slide):
  [0-1s]
  [0.5-1.5s]
  [1-2s]
```

**Exactly-Once Semantics**
```
Idempotent: can replay events without double-counting
Via deduplication ID + timestamp
```

---

# SUCCESS METRICS

- 100MB data processed
- 100k events/sec throughput
- P99 latency < 100ms
- At-least-once delivery
- Distributed processing across partitions
- Results aggregated correctly

---

# INTEGRATION SUMMARY

**Weeks 1-18**: Language fundamentals
**Week 19**: Multi-threading via actor pools
**Week 20**: Real-time systems (GC-free, low-latency)
**Week 21**: Network services (HTTP, WebSockets, RPC)
**Week 22**: Distributed data processing (scale, fault tolerance)

**Cumulative**: Build production systems with Killer
- Concurrent (1000s actors)
- Real-time (< 500µs trading system)
- Networked (5-service cluster)
- Scalable (100MB/sec data)

