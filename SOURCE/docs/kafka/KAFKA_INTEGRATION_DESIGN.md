# Killer + Kafka Integration Design

## Executive Summary

**Kafka Features for Killer v2.2** adds first-class Kafka producer/consumer support with:
- **Non-blocking async I/O** via Async/Await (no thread per connection)
- **Guaranteed semantics** via Contract Programming (at-least-once/exactly-once)
- **Compile-time batch sizing** via Dependent Types (no overflow)
- **Effect tracking** via Effect System (explicit `uses io, allocate, state`)
- **Type-safe deserialization** with schema validation

**Integration Timeline:**
- **Phase 1.A (Weeks 1-6):** Async Kafka core (producer + simple consumer)
- **Phase 1.B (Weeks 7-11):** Consumer groups + offset management with contracts
- **Phase 1.C (Weeks 12-15):** Transaction support + batch optimizations
- **Production Launch:** August 15, 2026 (alongside other Phase 1 features)

---

## 1. KAFKA PRODUCER API

### Basic Producer

```killer
// Kafka-specific effect
use io, allocate, state;

struct KafkaProducer[n: nat] {
    broker: String,
    topic: String,
    client: Arc<KafkaClient>,
    batch_size: n,
    
    invariant n > 0;
    invariant n <= MAX_BATCH_SIZE;
}

// Create producer with compile-time batch size
async fn producer_new[n: nat](
    brokers: String,
    topic: String
) -> KafkaProducer[n] uses (io, allocate)
    requires brokers.len() > 0;
    requires topic.len() > 0;
    requires n > 0;
    ensures result.batch_size == n;
{
    let client = connect(brokers).await;
    KafkaProducer {
        broker: brokers,
        topic: topic,
        client: Arc::new(client),
        batch_size: n,
    }
}

// Send single message
async fn send[n: nat](
    mut producer: KafkaProducer[n],
    message: String,
    key: Option[String]
) -> i64 uses (io, allocate)
    requires message.len() > 0;
    requires producer.broker.len() > 0;
    ensures result >= 0;  // offset in partition
{
    let record = ProducerRecord {
        topic: producer.topic.clone(),
        partition: None,
        key: key,
        value: message,
        headers: Vec::new(),
        timestamp: None,
    };
    
    let future = producer.client.send(record);
    let metadata = future.await;
    metadata.offset()
}

// Batch send with dependent types
async fn send_batch[n: nat][m: nat](
    producer: KafkaProducer[n],
    messages: Vector[m],
    keys: Option[Vector[m]]
) -> Vector[m] uses (io, allocate)
    requires m > 0;
    requires m <= n;  // Batch size constraint proven at compile time
    requires messages.len() == m;
    ensures result.len() == m;
    ensures forall(i in 0..m, result[i] >= 0);
{
    let mut futures = Vec::with_capacity(m);
    
    // All futures created upfront (no runtime allocation checks needed)
    for i in 0..m {
        let key = if let Some(k) = keys {
            Some(k.data[i].clone())
        } else {
            None
        };
        
        let record = ProducerRecord {
            topic: producer.topic.clone(),
            partition: None,
            key: key,
            value: messages.data[i].clone(),
            headers: Vec::new(),
            timestamp: None,
        };
        
        futures.push(producer.client.send(record));
    }
    
    // Concurrent send: all m messages in flight simultaneously
    let results = join_all(futures).await;
    
    // Extract offsets and return Vector[m]
    let mut offsets = Vector { data: [], len: m };
    for i in 0..m {
        offsets.data[i] = results[i].offset();
    }
    
    offsets
}

// Partitioned batch send
async fn send_to_partitions[n: nat][p: nat](
    producer: KafkaProducer[n],
    partition_messages: Vector[Vector[p]],  // Vector[n] of partitions
    partitions: Vector[i32]
) -> Vector[n] uses (io, allocate)
    requires n > 0;
    requires partition_messages.len() == n;
    requires partitions.len() == n;
    ensures result.len() == n;
    ensures forall(i in 0..n, result[i] >= 0);
{
    let mut futures = Vec::with_capacity(n);
    
    for i in 0..n {
        let msgs = partition_messages.data[i].clone();
        let partition = partitions.data[i];
        
        let record = ProducerRecord {
            topic: producer.topic.clone(),
            partition: Some(partition),
            key: None,
            value: msgs.data[0].clone(),  // Combined message
            headers: Vec::new(),
            timestamp: None,
        };
        
        futures.push(producer.client.send(record));
    }
    
    let results = join_all(futures).await;
    
    let mut offsets = Vector { data: [], len: n };
    for i in 0..n {
        offsets.data[i] = results[i].offset();
    }
    
    offsets
}
```

### Producer with Guarantees

```killer
// Exactly-once semantics via contracts
async fn send_idempotent[n: nat](
    producer: KafkaProducer[n],
    message: String,
    key: String,
    sequence_num: i32
) -> i64 uses (io, allocate, state)
    requires message.len() > 0;
    requires key.len() > 0;
    requires sequence_num >= 0;
    ensures result >= 0;
    // Producer guarantees: idempotent_id + sequence prevents duplication
{
    let record = ProducerRecord {
        topic: producer.topic.clone(),
        partition: None,
        key: Some(key),
        value: message,
        headers: vec![
            ("sequence", sequence_num.to_string())
        ],
        timestamp: None,
    };
    
    let future = producer.client.send_idempotent(record);
    future.await.offset()
}

// Transaction support
async fn transactional_send[n: nat](
    producer: KafkaProducer[n],
    messages: Vector[n],
    txn_id: String
) -> bool uses (io, allocate, state)
    requires messages.len() == n;
    requires txn_id.len() > 0;
    requires n > 0;
    ensures result == true || result == false;
{
    // Begin transaction
    producer.client.begin_txn(txn_id);
    
    let mut success = true;
    for i in 0..n {
        let record = ProducerRecord {
            topic: producer.topic.clone(),
            partition: None,
            key: None,
            value: messages.data[i].clone(),
            headers: Vec::new(),
            timestamp: None,
        };
        
        match producer.client.send(record).await {
            Ok(_) => {},
            Err(_) => {
                success = false;
                break;
            }
        }
    }
    
    // Abort or commit atomically
    if success {
        producer.client.commit_txn();
    } else {
        producer.client.abort_txn();
    }
    
    success
}
```

---

## 2. KAFKA CONSUMER API

### Basic Consumer

```killer
struct KafkaConsumer[n: nat] {
    broker: String,
    topics: Vector[n],  // Subscribe to n topics
    group_id: String,
    client: Arc<KafkaClient>,
    max_batch: n,
    
    invariant n > 0;
    invariant topics.len() == n;
}

// Create consumer with topic count
async fn consumer_new[n: nat](
    brokers: String,
    topics: Vector[n],
    group_id: String
) -> KafkaConsumer[n] uses (io, allocate)
    requires brokers.len() > 0;
    requires topics.len() == n;
    requires group_id.len() > 0;
    requires n > 0;
{
    let client = KafkaClient::new(brokers);
    client.subscribe(topics.clone()).await;
    
    KafkaConsumer {
        broker: brokers,
        topics: topics,
        group_id: group_id,
        client: Arc::new(client),
        max_batch: n,
    }
}

// Poll single record
async fn poll[n: nat](
    consumer: KafkaConsumer[n],
    timeout_ms: i32
) -> Option[ConsumerRecord] uses io
    requires timeout_ms >= 0;
    ensures match result {
        Some(record) => record.value.len() > 0,
        None => true,
    };
{
    consumer.client.poll(timeout_ms).await
}

// Poll batch with compile-time size
async fn poll_batch[n: nat][m: nat](
    consumer: KafkaConsumer[n],
    max_records: m,
    timeout_ms: i32
) -> Vector[m] uses (io, allocate)
    requires m > 0;
    requires m <= n;
    requires timeout_ms >= 0;
    ensures result.len() <= m;
{
    let mut records = Vec::with_capacity(m);
    let start = current_time();
    
    loop {
        match consumer.client.poll_batch(m).await {
            Some(batch) => {
                for record in batch {
                    records.push(record);
                    if records.len() >= m {
                        break;
                    }
                }
                break;
            }
            None => {
                if elapsed(start) > timeout_ms {
                    break;
                }
            }
        }
    }
    
    let mut result = Vector { data: [], len: records.len() };
    for i in 0..records.len() {
        result.data[i] = records[i].clone();
    }
    
    result
}
```

### Consumer Group Management

```killer
// Commit offset with contract
async fn commit_offset[n: nat](
    consumer: KafkaConsumer[n],
    partition: i32,
    offset: i64
) -> bool uses (io, allocate, state)
    requires offset >= 0;
    ensures result == true || result == false;
    // Postcondition: if true, consumer can resume from offset
{
    consumer.client.commit_offset(partition, offset).await
}

// Get committed offset
async fn committed_offset[n: nat](
    consumer: KafkaConsumer[n],
    partition: i32
) -> Option[i64] uses io
    ensures match result {
        Some(offset) => offset >= 0,
        None => true,
    };
{
    consumer.client.get_committed(partition).await
}

// Seek to specific offset
async fn seek[n: nat](
    consumer: KafkaConsumer[n],
    partition: i32,
    offset: i64
) -> bool uses (io, allocate, state)
    requires offset >= 0;
    ensures result == true || result == false;
{
    consumer.client.seek(partition, offset).await
}

// Rebalance listener
async fn on_rebalance[n: nat](
    consumer: KafkaConsumer[n],
    handler: fn(Vec[PartitionAssignment]) -> () uses io
) -> () uses (io, allocate, state)
{
    consumer.client.subscribe_rebalance(handler).await
}
```

### Stream Processing

```killer
// Consume and process with dependent type
async fn process_stream[n: nat](
    consumer: KafkaConsumer[n],
    handler: fn(ConsumerRecord) -> () pure,
    count: i32
) -> () uses (io, allocate)
    requires count >= 0;
    requires n > 0;
{
    let mut processed = 0;
    
    loop {
        match consumer.poll(1000).await {
            Some(record) => {
                handler(record);  // pure - can parallelize
                processed = processed + 1;
                
                if processed >= count {
                    break;
                }
            }
            None => {}
        }
    }
}

// Parallel process with work stealing
async fn parallel_process[n: nat][p: nat](
    consumer: KafkaConsumer[n],
    handler: fn(ConsumerRecord) -> () pure,
    parallelism: p
) -> () uses (io, allocate)
    requires p > 0;
    requires n > 0;
    requires p <= n;
{
    scope(|s| {
        for _ in 0..p {
            s.spawn(async {
                loop {
                    match consumer.poll(1000).await {
                        Some(record) => handler(record),
                        None => break,
                    }
                }
            });
        }
    })
}
```

---

## 3. SCHEMA REGISTRY INTEGRATION

```killer
// Schema-aware producer
async fn send_with_schema[n: nat](
    producer: KafkaProducer[n],
    message: String,
    schema_id: i32
) -> i64 uses (io, allocate)
    requires message.len() > 0;
    requires schema_id > 0;
    ensures result >= 0;
{
    // Lookup schema from registry
    let schema = fetch_schema(schema_id).await;
    
    // Validate message against schema
    if validate_schema(message, schema) {
        send(producer, message, None).await
    } else {
        panic("Message validation failed");
        0
    }
}

// Schema-aware consumer
async fn poll_with_schema[n: nat](
    consumer: KafkaConsumer[n],
    handler: fn(String, i32) -> () pure
) -> () uses (io, allocate)
{
    loop {
        match consumer.poll(1000).await {
            Some(record) => {
                let schema_id = parse_schema_id(&record.headers);
                let value = record.value.clone();
                handler(value, schema_id);  // pure handler
            }
            None => {}
        }
    }
}
```

---

## 4. EXACTLY-ONCE SEMANTICS

```killer
// Transactional processing
async fn process_exactly_once[n: nat](
    consumer: KafkaConsumer[n],
    producer: KafkaProducer[n],
    handler: fn(ConsumerRecord) -> String pure
) -> bool uses (io, allocate, state)
    requires n > 0;
    ensures result == true || result == false;
    // Postcondition: Either all processed and produced, or none
{
    consumer.client.begin_txn();
    producer.client.begin_txn();
    
    match consumer.poll(1000).await {
        Some(record) => {
            let partition = record.partition;
            let offset = record.offset;
            
            // Process message (pure)
            let output = handler(record);
            
            // Send result atomically
            match send(producer, output, None).await {
                result_offset => {
                    // Commit both offsets atomically
                    consumer.client.commit_offset(partition, offset);
                    consumer.client.commit_txn();
                    producer.client.commit_txn();
                    true
                }
            }
        }
        None => {
            consumer.client.abort_txn();
            producer.client.abort_txn();
            false
        }
    }
}
```

---

## 5. PERFORMANCE PATTERNS

### Batch Processing with Dependent Types

```killer
// Process in batches of exactly n items
async fn batch_process[n: nat](
    consumer: KafkaConsumer[n],
    handler: fn(Vector[n]) -> () pure
) -> () uses (io, allocate)
    requires n > 0;
{
    let mut batch = Vec::with_capacity(n);
    
    loop {
        match consumer.poll(100).await {
            Some(record) => {
                batch.push(record);
                
                if batch.len() == n {
                    // Create Vector[n] - compile-time proven size
                    let mut batch_vec = Vector { data: [], len: n };
                    for i in 0..n {
                        batch_vec.data[i] = batch[i].clone();
                    }
                    
                    handler(batch_vec);  // Handler sees Vector[n]
                    batch.clear();
                }
            }
            None => {}
        }
    }
}

// SIMD-friendly batch operations
async fn simd_process[n: nat](
    consumer: KafkaConsumer[n]
) -> () uses (io, allocate)
    requires n == 16;  // SIMD width
{
    // Compiler generates single instruction for n=16 operations
    batch_process(consumer, |batch: Vector[16]| {
        // Process 16 items with single SIMD instruction
        ()
    }).await
}
```

### Zero-Copy Design

```killer
// String view without allocation
struct MessageView[n: nat] {
    data: &[u8; n],
    len: n,
}

async fn zero_copy_process[n: nat](
    consumer: KafkaConsumer[n],
    handler: fn(MessageView) -> () pure
) -> () uses io
    // No allocate effect - zero-copy mode
{
    loop {
        match consumer.poll(1000).await {
            Some(record) => {
                let view = MessageView {
                    data: record.value_ref(),
                    len: record.value.len(),
                };
                handler(view);  // Handler gets reference, not copy
            }
            None => {}
        }
    }
}
```

---

## 6. ERROR HANDLING & RESILIENCE

```killer
// Exponential backoff with contracts
async fn send_with_retry[n: nat](
    producer: KafkaProducer[n],
    message: String,
    max_retries: i32
) -> Result[i64, String] uses (io, allocate)
    requires message.len() > 0;
    requires max_retries >= 0;
    ensures match result {
        Ok(offset) => offset >= 0,
        Err(_) => true,
    };
{
    let mut retries = 0;
    let mut backoff = 100;  // ms
    
    loop {
        match send(producer, message.clone(), None).await {
            Ok(offset) => return Ok(offset),
            Err(e) => {
                retries = retries + 1;
                if retries > max_retries {
                    return Err(format!("Failed after {} retries", retries));
                }
                
                // Exponential backoff
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                backoff = backoff * 2;
                
                if backoff > 30000 {  // 30 second max
                    backoff = 30000;
                }
            }
        }
    }
}

// Circuit breaker pattern
struct CircuitBreaker {
    failures: i32,
    threshold: i32,
    timeout: i32,
    
    invariant threshold > 0;
    invariant timeout > 0;
}

async fn call_with_circuit_breaker[n: nat](
    breaker: mut CircuitBreaker,
    producer: KafkaProducer[n],
    message: String
) -> Result[i64, String] uses (io, allocate, state)
    requires breaker.threshold > 0;
{
    if breaker.failures >= breaker.threshold {
        return Err("Circuit breaker open".to_string());
    }
    
    match send(producer, message, None).await {
        Ok(offset) => {
            breaker.failures = 0;
            Ok(offset)
        }
        Err(e) => {
            breaker.failures = breaker.failures + 1;
            Err(e)
        }
    }
}
```

---

## 7. TYPE SAFETY GUARANTEES WITH KAFKA

| Guarantee | How Killer Achieves It |
|-----------|------------------------|
| **No buffer overflows** | `Vector[n]` with compile-time bounds checking |
| **No message loss** | `Contracts` with at-least-once/exactly-once postconditions |
| **No race conditions** | Async/await with Rust ownership model |
| **Batch size safety** | Dependent types: `Vector[n]` prevents overflow |
| **Resource cleanup** | RAII + effect tracking (`uses allocate`) |
| **Effect tracking** | `uses io` + `uses state` shows all side effects |
| **Concurrency safety** | Effect system prevents unsafe parallelization |
| **Schema validation** | Pure functions for validation before send |

---

## 8. EXAMPLES & PATTERNS

See `/tests/phase1/` for examples:
- `kafka_01_basic_producer.killer` - Single/batch sends
- `kafka_02_consumer.killer` - Polling and consumption
- `kafka_03_exactly_once.killer` - Transactional processing
- `kafka_04_performance.killer` - Batch/SIMD patterns
- `kafka_05_error_handling.killer` - Resilience patterns

---

## 9. ECOSYSTEM INTEGRATION

### Works with Existing Killer Features

**Dependent Types:**
- `KafkaProducer[n]` with compile-time batch size n
- `Vector[n]` for exactly n messages
- No runtime overflow checks

**Effect System:**
- `uses io` - network I/O
- `uses allocate` - buffer creation
- `uses state` - offset tracking

**Async/Await:**
- `async fn` for non-blocking operations
- `await` for futures
- `join_all()` for concurrent sends
- Custom async runtime optimization

**Contracts:**
- `requires` - preconditions (brokers valid, batch size ok)
- `ensures` - postconditions (offset >= 0, committed)
- At-least-once/exactly-once guarantees

---

## 10. IMPLEMENTATION ROADMAP

| Phase | Timeline | Features |
|-------|----------|----------|
| **1.A** | Weeks 1-6 | Async producer, basic consumer, Rust bindings |
| **1.B** | Weeks 7-11 | Consumer groups, offset mgmt, contracts |
| **1.C** | Weeks 12-15 | Transactions, Schema Registry, error handling |
| **Production** | August 15 | Full integration, benchmarks, documentation |

---

## 11. PERFORMANCE TARGETS

- **Throughput:** 100K+ messages/second (Killer producer)
- **Latency:** <10ms p99 (async non-blocking)
- **Memory:** Zero-copy for large messages
- **CPU:** Single core → 8+ cores with work-stealing
- **Allocations:** Minimized via dependent types

---

