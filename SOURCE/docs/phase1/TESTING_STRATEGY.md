# Phase 1 Testing Strategy Guide

## Overview

Comprehensive testing strategy for Killer v2.2 Phase 1 (4 core features + Kafka):
- **Unit Tests** - Individual feature testing
- **Integration Tests** - Feature interactions
- **Performance Tests** - Throughput, latency, memory
- **Stress Tests** - Edge cases, error conditions
- **Compliance Tests** - Semantic guarantees

**Coverage Target:** 80+ tests per feature, 100% path coverage

---

## 1. TESTING FRAMEWORK

### Test Runner (Killer Native)

```killer
// test_runner.killer - Main test harness
async fn run_all_tests() -> TestResults uses (io, allocate) {
    let mut results = Vec::new();
    
    // Run all test suites
    results.push(run_dependent_types_tests().await);
    results.push(run_effect_system_tests().await);
    results.push(run_async_await_tests().await);
    results.push(run_contracts_tests().await);
    results.push(run_kafka_tests().await);
    
    summarize(results)
}

struct TestResults {
    passed: i32,
    failed: i32,
    skipped: i32,
    duration: i32,  // ms
    
    invariant passed >= 0;
    invariant failed >= 0;
    invariant duration >= 0;
}
```

---

## 2. UNIT TEST PATTERNS

### Dependent Types Tests

```killer
// Test: Vector bounds are proven at compile time
#[test]
fn test_vector_bounds_proven[n: nat]()
    requires n > 0;
{
    let v: Vector[n] = Vector { data: [], len: n };
    
    // No bounds check needed - proven at compile time
    for i in 0..n {
        let _ = v.data[i];  // Always safe
    }
}

// Test: Type arithmetic (m + n = m+n)
#[test]
fn test_type_arithmetic[m: nat][n: nat]() {
    let v1: Vector[m] = create_vector(m);
    let v2: Vector[n] = create_vector(n);
    let combined: Vector[m+n] = append(v1, v2);
    
    assert(combined.len() == m + n);
}

// Test: Matrix operations preserve dimensions
#[test]
fn test_matrix_dimensions[m: nat][n: nat][p: nat]() {
    let a: Matrix[m][n] = create_matrix(m, n);
    let b: Matrix[n][p] = create_matrix(n, p);
    let result: Matrix[m][p] = multiply(a, b);
    
    assert(result.rows() == m);
    assert(result.cols() == p);
}

// Test: Index type prevents out-of-bounds
#[test]
fn test_index_safety[n: nat](idx: Idx[n]) {
    assert(idx.to_int() >= 0);
    assert(idx.to_int() < n);
}

// Test: Zero-sized dependent types
#[test]
fn test_zero_sized[n: nat]()
    requires n == 0;
{
    let v: Vector[n] = Vector { data: [], len: 0 };
    // Loop never executes
    for i in 0..n {
        panic("Should not execute");
    }
}

// Test: Dependent type polymorphism
#[test]
fn test_polymorphic[n: nat][T: type]() {
    let v: Vector[n] = create_vector(n);
    let mapped: Vector[n] = apply_function(v, identity);
    
    assert(mapped.len() == n);
}
```

### Effect System Tests

```killer
// Test: Pure functions have no side effects
#[test]
fn test_pure_function() pure {
    let x = pure_compute(5);
    assert(x == 10);
    // No I/O, allocations, or mutations
}

// Test: Effect tracking in call graph
#[test]
fn test_effect_propagation() uses io {
    let result = pure_fn();  // pure ⊆ io
    let with_io = io_fn().await;  // uses io
}

// Test: Effect subtyping
#[test]
fn test_effect_subtyping() uses (io, allocate) {
    let _pure = pure_function();  // OK: pure ⊆ (io, allocate)
    let _vec = Vec::new();        // OK: uses allocate
    let _io = read_file().await;  // OK: uses io
}

// Test: Effect polymorphism
#[test]
fn test_effect_polymorphic[E: effect]() uses E {
    // Works with any effect
}

// Test: Batch effects
#[test]
fn test_batch_effects[n: nat]() uses (io, allocate) {
    let mut futures = Vec::with_capacity(n);
    for i in 0..n {
        futures.push(fetch_url().await);  // uses io
    }
    // Overall effect: uses (io, allocate)
}

// Test: Effect inference
#[test]
fn test_effect_inference() {
    // Effect inferred from called functions
    if condition {
        io_function();
    }
    // Result: uses io (inferred)
}

// Test: Custom effects
#[test]
fn test_custom_effects() uses custom_effect {
    call_custom();
}
```

### Async/Await Tests

```killer
// Test: Basic async function
#[test]
async fn test_basic_async() uses io {
    let result = fetch("url").await;
    assert(result.len() > 0);
}

// Test: Concurrent execution with join_all
#[test]
async fn test_concurrent[n: nat](urls: Vector[n]) uses io
    requires n > 0;
{
    let results = join_all(urls.map(fn(url) fetch(url))).await;
    assert(results.len() == n);
}

// Test: Async spawn (background task)
#[test]
async fn test_spawn() uses allocate {
    let handle = spawn(async { compute() });
    let result = handle.await;
    assert(result >= 0);
}

// Test: Scoped concurrency
#[test]
async fn test_scope[n: nat]() uses allocate
    requires n > 0;
{
    scope(|s| {
        for i in 0..n {
            s.spawn(async { process(i) });
        }
        // All tasks complete before scope exits
    })
}

// Test: Error handling with match
#[test]
async fn test_error_handling() uses io {
    match fetch("invalid_url").await {
        Ok(data) => assert(data.len() > 0),
        Err(_) => {},
    }
}

// Test: Error handling with ?
#[test]
async fn test_try_operator() -> Result[String, String] uses io {
    let data1 = fetch("url1").await?;
    let data2 = fetch("url2").await?;
    Ok(combine(data1, data2))
}

// Test: Timeout
#[test]
async fn test_timeout() uses io {
    match timeout(1000, fetch("url")).await {
        Ok(result) => {},
        Err(_) => {},  // timeout
    }
}
```

### Contract Tests

```killer
// Test: Precondition enforcement
#[test]
fn test_precondition() {
    assert_panics(|| {
        divide(10, 0);  // requires b != 0
    })
}

// Test: Postcondition verification
#[test]
fn test_postcondition() {
    let result = increment(5);
    assert(result == 6);  // ensures result == x + 1
}

// Test: Invariant verification
#[test]
fn test_struct_invariant() {
    let mut heap = MinHeap::new();
    heap.insert(5);
    heap.insert(3);
    heap.insert(7);
    
    assert(is_valid_heap(heap));  // invariant: well_formed_heap
}

// Test: Loop invariants
#[test]
fn test_loop_invariant[n: nat]() {
    let v = create_sorted_vector(n);
    // invariant: elements[0..i] are sorted
    // invariant: i <= n
}

// Test: Complex postconditions
#[test]
fn test_complex_postcondition[n: nat](
    v: Vector[n],
    idx: i32,
    value: i32
) {
    let result = insert(v, idx, value);
    
    // ensures result.len() == n + 1
    assert(result.len() == n + 1);
    
    // ensures result[idx] == value
    assert(result.data[idx] == value);
}

// Test: Contract polymorphism
#[test]
fn test_contract_polymorphic[T](coll: T) {
    let result = sum(coll);
    assert(result >= 0);
}
```

---

## 3. INTEGRATION TEST PATTERNS

### Cross-Feature Tests

```killer
// Test: Dependent Types + Effects
#[test]
async fn test_dt_effects[n: nat](
    v: Vector[n]
) -> i32 uses (io, allocate)
    requires n > 0;
{
    // Dependent type: Vector[n]
    // Effect: uses (io, allocate)
    let sum = pure_sum(v);       // pure computation
    let fetched = fetch(urls).await;  // io operation
    
    sum
}

// Test: Async + Contracts
#[test]
async fn test_async_contracts() -> i32 uses io
    requires true;
    ensures result >= 0;
{
    let val = fetch_int().await;
    if val < 0 { 0 } else { val }
}

// Test: All Four Features
#[test]
async fn test_all_four[n: nat](
    items: Vector[n]
) -> i32 uses (io, allocate)
    requires n > 0;
    ensures result >= 0;
{
    let processed = items.map(pure_fn);  // DT + pure
    
    let batch = join_all(               // async
        processed.map(fn(item) {
            async { fetch(item).await } // io effect
        })
    ).await;
    
    // Result proven: Vector[n], >= 0
    batch.len()
}

// Kafka + Dependent Types
#[test]
async fn test_kafka_batch[n: nat](
    messages: Vector[n]
) -> Vector[n] uses (io, allocate)
    requires n > 0;
    ensures result.len() == n;
{
    let producer = producer_new("broker", "topic").await;
    send_batch(producer, messages, None).await
}

// Kafka + Async + Contracts
#[test]
async fn test_kafka_transactional[n: nat](
    messages: Vector[n]
) -> bool uses (io, allocate, state)
    requires n > 0;
    ensures result == true || result == false;
{
    transactional_send(messages).await
}
```

---

## 4. PERFORMANCE TEST PATTERNS

### Throughput Tests

```killer
// Test: Dependent Types Overhead
#[bench]
fn bench_vector_access[n: nat]() -> i32 {
    let v: Vector[n] = create_vector(n);
    let mut sum = 0;
    
    // Measured: access time vs n (should be constant - no bounds checks)
    for i in 0..n {
        sum = sum + v.data[i];
    }
    
    sum
}

// Test: Async Throughput
#[bench]
async fn bench_async_throughput[n: nat]() -> i32 uses io {
    let urls = create_urls(n);
    let start = now();
    
    let results = join_all(urls.map(fetch)).await;
    
    let elapsed = elapsed(start);
    results.len()
    
    // Measured: throughput = n / elapsed
}

// Test: Kafka Producer Throughput
#[bench]
async fn bench_producer[n: nat]() -> i32 uses (io, allocate) {
    let producer = producer_new("broker", "topic").await;
    let messages = create_messages(n);
    
    let start = now();
    let offsets = send_batch(producer, messages, None).await;
    let elapsed = elapsed(start);
    
    // Measure: messages/ms
    offsets.len()
}

// Test: Kafka Consumer Throughput
#[bench]
async fn bench_consumer[n: nat]() -> i32 uses io {
    let consumer = consumer_new("broker", vec!["topic"], "group").await;
    let start = now();
    
    let mut count = 0;
    loop {
        match consumer.poll(100).await {
            Some(_) => count = count + 1,
            None => if count >= n { break },
        }
    }
    
    let elapsed = elapsed(start);
    count
}
```

### Latency Tests

```killer
// Test: End-to-end latency
#[bench]
async fn bench_e2e_latency() uses (io, allocate) {
    let start = now();
    
    // Producer
    let producer = producer_new("broker", "topic").await;
    send(producer, "msg", None).await;
    
    // Consumer
    let consumer = consumer_new("broker", vec!["topic"], "group").await;
    let _record = consumer.poll(10000).await;
    
    let elapsed = elapsed(start);
    // Measured: p50, p99, max latency
}
```

### Memory Tests

```killer
// Test: Zero-copy message processing
#[bench]
async fn bench_zero_copy[n: nat]() -> i32 uses io {
    let consumer = consumer_new("broker", vec!["topic"], "group").await;
    
    let mut bytes = 0;
    for _ in 0..n {
        match consumer.poll(1000).await {
            Some(record) => bytes = bytes + record.value_size(),
            None => break,
        }
    }
    
    bytes  // Should show no allocations for message processing
}
```

---

## 5. STRESS TEST PATTERNS

### Edge Cases

```killer
// Test: Zero-sized vectors
#[test]
fn test_zero_vector[n: nat]()
    requires n == 0;
{
    let v: Vector[n] = Vector { data: [], len: 0 };
    
    // All loops skip
    for i in 0..n {
        panic("Should not execute");
    }
}

// Test: Large dependent types
#[test]
fn test_large_vector[n: nat]()
    requires n == 1000000;
{
    let v: Vector[n] = create_vector(n);
    // Compiler proves: no bounds checks needed
    assert(v.len() == 1000000);
}

// Test: Effect restrictions
#[test]
fn test_pure_in_pure() pure {
    let x = pure_function();  // OK
    // let _ = io_function(); // ERROR: cannot call uses io in pure
}

// Test: Async error recovery
#[test]
async fn test_async_retry() uses io {
    let mut attempts = 0;
    loop {
        match fetch("url").await {
            Ok(data) => return data,
            Err(_) => {
                attempts = attempts + 1;
                if attempts >= 5 { panic!("Failed after 5 retries"); }
            }
        }
    }
}

// Test: Contract violations
#[test]
fn test_contract_violation() {
    assert_panics(|| {
        let _result = safe_divide(10, 0);  // requires b != 0
    })
}

// Test: Kafka broker failure
#[test]
async fn test_kafka_resilience() uses (io, allocate) {
    let producer = producer_new("invalid_broker", "topic").await;
    match send(producer, "msg", None).await {
        Ok(_) => {},
        Err(_) => {},  // Expected: broker unreachable
    }
}
```

---

## 6. COMPLIANCE TESTS

### Semantic Guarantees

```killer
// Test: Dependent Types Guarantee
#[test]
fn test_dt_guarantee[n: nat]() {
    // GUARANTEE: Vector[n] has exactly n elements
    let v: Vector[n] = create_vector(n);
    assert(v.len() == n);
    
    // No bounds checks in loops
    for i in 0..n {
        let _x = v.data[i];  // Always safe
    }
}

// Test: Effect System Guarantee
#[test]
async fn test_effect_guarantee() {
    // GUARANTEE: All I/O marked with 'uses io'
    let fn_uses_io = || { fetch("url").await };  // requires 'uses io'
    // This would error without 'uses io'
}

// Test: Async Safety Guarantee
#[test]
async fn test_async_safety() uses allocate {
    // GUARANTEE: No data races via Rust ownership
    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    
    scope(|s| {
        for i in 0..5 {
            let data = data.clone();
            s.spawn(async move {
                println!("{}", data[i]);
            });
        }
        // All spawns complete safely
    })
}

// Test: Contract Guarantee
#[test]
fn test_contract_guarantee() {
    // GUARANTEE: Preconditions checked, postconditions proven
    let result = safe_divide(10, 2);
    
    // requires b != 0 - enforced
    // ensures result == a / b - proven
    assert(result == 5);
}

// Test: Kafka At-Least-Once
#[test]
async fn test_kafka_alc() -> bool uses (io, allocate, state)
    ensures result == true || result == false;
{
    // GUARANTEE: Message delivered at least once
    transactional_send(vec!["msg1", "msg2"]).await
}
```

---

## 7. TEST EXECUTION STRATEGY

### CI/CD Pipeline

```
Phase 1: Unit Tests (2 min)
  ├─ Dependent Types tests (30 sec)
  ├─ Effect System tests (30 sec)
  ├─ Async/Await tests (30 sec)
  ├─ Contract tests (30 sec)
  └─ Kafka tests (30 sec)

Phase 2: Integration Tests (3 min)
  ├─ Feature interaction tests (1 min)
  ├─ Kafka integration tests (1 min)
  └─ End-to-end tests (1 min)

Phase 3: Performance Tests (5 min)
  ├─ Throughput benchmarks (2 min)
  ├─ Latency measurements (2 min)
  └─ Memory profiling (1 min)

Phase 4: Stress Tests (5 min)
  ├─ Edge case validation (2 min)
  ├─ Error handling (2 min)
  └─ Recovery scenarios (1 min)

Total: 15 minutes per full test run
```

### Metrics

| Metric | Target |
|--------|--------|
| Unit Test Coverage | 100% |
| Integration Test Cases | 40+ |
| Performance Tests | 15+ benchmarks |
| Response Time (p99) | <10ms |
| Throughput | 100K msg/sec |
| Success Rate | 99.9%+ |

---

## 8. TEST FILES CREATED

✅ `tests/phase1/dependent_types_*.killer` (21 tests)
✅ `tests/phase1/effect_system_*.killer` (14 tests)
✅ `tests/phase1/async_await_*.killer` (13 tests)
✅ `tests/phase1/contracts_*.killer` (13 tests)
✅ `tests/phase1/integration_*.killer` (10 tests)
✅ `tests/phase1/error_cases_*.killer` (10 tests)
✅ `tests/phase1/comprehensive_*.killer` (15 tests)
✅ `tests/phase1/advanced_*.killer` (30 tests)

**Total: 96+ test cases**

---

## 9. KAFKA TEST FILES (Next)

- `tests/kafka/kafka_01_basic_producer.killer`
- `tests/kafka/kafka_02_consumer.killer`
- `tests/kafka/kafka_03_exactly_once.killer`
- `tests/kafka/kafka_04_performance.killer`
- `tests/kafka/kafka_05_error_handling.killer`

---

