# Killer v2.2 Phase 1 - Complete API Reference

## Overview
This document provides comprehensive API reference for all Phase 1 features:
1. **Dependent Types** - Size-proven and shape-proven types
2. **Effect System** - Explicit side-effect control
3. **Async/Await** - Concurrent I/O programming
4. **Contract Programming** - Formal pre/postconditions and invariants

---

## 1. DEPENDENT TYPES API

### Syntax Overview
```killer
// Type parameter syntax
fn function_name[ParamName: kind](args) -> ReturnType { }

// Parameter kinds:
// - nat: Natural numbers (0, 1, 2, ...)
// - int: Integers (..., -1, 0, 1, ...)
// - type: Type-level types
// - bool: Compile-time booleans
```

### Vector (1D Array)
```killer
// Declaration
fn process[n: nat](v: Vector[n]) -> Vector[n] {
    // v.data: array of n elements
    // v.len: n (provably equal)
}

// Usage
let v5 = Vector { data: [...], len: 5 };  // Vector[5]
let sum = process(v5);                    // Result: Vector[5]
```

**Vector[n] Interface:**
- `len()` → nat (always equals n)
- `data[i]` → Element (safe: i < n guaranteed at compile time)
- `map(f)` → Vector[n] (preserves size)
- `fold(init, f)` → AccumType
- `filter(predicate)` → Vector[≤n]

### Matrix (2D Array)
```killer
// Declaration
fn matrix_ops[m: nat][n: nat](mat: Matrix[m][n]) -> i32 {
    // mat.rows: dimension m
    // mat.cols: dimension n
}

// Usage
let mat3x4 = Matrix { 
    data: [...], 
    rows: 3, 
    cols: 4 
};  // Matrix[3][4]
```

**Matrix[m][n] Interface:**
- `rows()` → m
- `cols()` → n
- `get(i, j)` → Element (safe: i < m, j < n)
- `transpose()` → Matrix[n][m]
- `multiply[p: nat](other: Matrix[n][p])` → Matrix[m][p]

### Index Type
```killer
// Represents valid indices for Vector[n]
fn safe_access[n: nat](v: Vector[n], idx: Idx[n]) -> Element {
    v.data[idx]  // Always safe - idx < n by type
}

// Creating indices
let idx: Idx[10] = safe_index(5);
```

**Idx[n] Interface:**
- Constructor: `Idx::new(value: i32)` → Result[Idx[n]]
- `to_int()` → i32 (value, guaranteed < n)
- `checked_add(k)` → Option[Idx[n]]
- `as_usize()` → usize

### Dependent Type Arithmetic
```killer
// Type-level addition
fn append[m: nat][n: nat](
    v1: Vector[m],
    v2: Vector[n]
) -> Vector[m+n] {
    // Result size proven at compile time
}

// Type-level comparison
fn concat[m: nat][n: nat](
    m1: Matrix[m][n],
    m2: Matrix[m][n]
) -> Matrix[m][n*2] {  // Width doubles
    // Layout guaranteed correct
}
```

### Generic Dependent Functions
```killer
// Polymorphic over dependent parameters
fn apply_to_all[n: nat][T: type](
    v: Vector[n],
    f: fn(T) -> T
) -> Vector[n] {
    // Function preserves vector size
    v
}

// Usage
let v = Vector { data: [1, 2, 3], len: 3 };
let result = apply_to_all(v, |x| x * 2);  // Vector[3]
```

### Struct with Dependent Fields
```killer
// Generic struct with dependent size
struct Table[n: nat] {
    rows: Vector[n],
    index: Vector[n],
    
    invariant rows.len() == n;
    invariant index.len() == n;
}

// Constructor
let table = Table {
    rows: my_vector,
    index: index_vector,
};  // Type-checked: sizes match
```

### Error Handling
```killer
// Dependent type validation
async fn validate[n: nat](
    data: Vector[n]
) -> Result[Vector[n], String] uses io
    requires n > 0;
{
    if data.len() != n {
        Err("Size mismatch")
    } else {
        Ok(data)
    }
}
```

---

## 2. EFFECT SYSTEM API

### Effect Annotations
```killer
// Syntax: uses (effect1, effect2, ...)
// Pure (default, no annotation)
fn pure_function(x: i32) -> i32 {
    x * 2
}

// Single effect
fn io_function() uses io {
    println("Hello");
}

// Multiple effects
fn complex_function() uses (io, allocate, random) {
    // Can use all three
}
```

### Built-in Effects
| Effect | Meaning | Operations |
|--------|---------|-----------|
| `pure` | No side effects | Pure computations only |
| `io` | I/O operations | File/network/console reads/writes |
| `allocate` | Memory allocation | Vec, HashMap, Box creation |
| `random` | Randomness | RNG calls |
| `exception` | Exceptions thrown | Panic, throw |
| `state` | Mutable state | Global refs, mutability |

### Effect Polymorphism
```killer
// Function works with any effect
fn identity[E: effect][T](x: T) -> T uses E {
    x
}

// Usage
let _pure_result = identity::<pure, i32>(5);
let _io_result = identity::<io, String>("hello");
```

### Effect Union
```killer
// Effects compose with union
fn combined() uses (io, allocate) {
    let _vec = Vec::new();      // allocate
    let _bytes = read_file().await;  // io
}

// Subtyping: pure ⊆ any effect
fn call_pure_in_io() uses io {
    let x = pure_function(5);  // pure ⊆ io
}
```

### Effect Inference
```killer
fn inferred_effect() {
    // Effect inferred from call graph
    if condition {
        io_function();  // now uses io
    }
    // Overall effect: uses io
}

// Explicit declaration overrides inference
fn explicit() pure {
    // Error: cannot call uses io function
}
```

### Batch Operations with Effects
```killer
// Process multiple items with effect tracking
fn batch_io[n: nat](
    items: Vector[n]
) -> Vector[n] uses io {
    // All n operations propagate io effect
    items.map(fn(item) {
        fetch(item).await
    })
}

// Conditional effect selection
fn maybe_log[T](
    value: T,
    should_log: bool
) -> T uses (io, allocate) {
    if should_log {
        println("{:?}", value);
    }
    value
}
```

### Effect Declaration
```killer
// Custom effect annotation
fn custom_effect_fn() uses custom {
    // Custom effects tracked but treated as "unknown"
}

// Effect constraints in generics
fn restricted[F: fn() -> i32 |uses io|]() -> i32 {
    F()  // Can call functions that use io
}
```

---

## 3. ASYNC/AWAIT API

### Basic Async
```killer
// Define async function
async fn fetch_url(url: String) -> String uses io {
    // Async body can contain await expressions
    http_get(url).await
}

// Define async closure
let async_closure = async {
    result.await
};
```

### Await Expression
```killer
// Suspend execution until promise resolves
async fn sequential_io() uses io {
    let data1 = fetch("url1").await;
    let data2 = fetch("url2").await;
    process(data1, data2)
}
```

### Concurrent Execution - join_all
```killer
// Execute multiple futures concurrently
async fn concurrent_io[n: nat](
    urls: Vector[n]
) -> Vector[n] uses io
    requires n > 0;
    ensures result.len() == n;
{
    let futures = urls.map(fn(url) {
        fetch(url)
    });
    
    join_all(futures).await
}
```

### Async Spawn
```killer
// Spawn background async task
async fn background_work() uses allocate {
    let handle = spawn(async {
        long_computation()
    });
    
    // Continue without waiting
    other_stuff();
    
    result = handle.await;  // Now wait for result
}
```

### Async Scope
```killer
// Structured concurrency with declared scope
async fn scoped_tasks[n: nat](items: Vector[n]) uses allocate {
    scope(|s| {
        for i in 0..n {
            s.spawn(async {
                process_item(items.data[i]).await
            });
        }
        // All spawned tasks completed before scope exits
    })
}
```

### Error Handling - Match
```killer
// Pattern matching on async results
async fn robust_fetch(url: String) -> String uses io {
    match fetch(url).await {
        Ok(data) => data,
        Err(e) => {
            eprintln("Error: {}", e);
            ""
        }
    }
}
```

### Error Handling - Try Operator
```killer
// ? operator for short-circuit error propagation
async fn chain_fetches() -> String uses io {
    let data1 = fetch("url1").await?;
    let data2 = fetch("url2").await?;
    Ok(combine(data1, data2))
}
```

### Select/Race
```killer
// Race multiple futures
async fn race_requests(
    url1: String,
    url2: String
) -> String uses io {
    select! {
        result1 = fetch(url1) => result1.await,
        result2 = fetch(url2) => result2.await,
    }
}
```

### Timeout
```killer
// Add timeout to async operation
async fn fetch_with_timeout(
    url: String,
    timeout_ms: i32
) -> Result[String, String] uses io {
    timeout(timeout_ms, fetch(url)).await
}
```

### Async Iterator
```killer
// Async stream processing
async fn process_stream[n: nat](
    items: Vector[n]
) -> i32 uses io {
    let mut sum = 0;
    let mut stream = async_iter(items);
    loop {
        match stream.next().await {
            Some(item) => sum = sum + process(item).await,
            None => break,
        }
    }
    sum
}
```

### Future Trait
```killer
// Manually implement Future
impl Future[String] for MyFuture {
    async fn poll() -> Poll[String] {
        // Return Ready(value) or Pending
    }
}
```

---

## 4. CONTRACT PROGRAMMING API

### Preconditions (requires)
```killer
fn safe_divide(a: i32, b: i32) -> i32 pure
    requires b != 0;
{
    a / b
}

// Multiple preconditions
fn binary_search[n: nat](
    v: Vector[n],
    target: i32
) -> Option[i32] pure
    requires n > 0;
    requires is_sorted(v);
    requires v.len() == n;
{
    // Implementation guaranteed: v is sorted, non-empty
}
```

### Postconditions (ensures)
```killer
fn increment(x: i32) -> i32 pure
    ensures result == x + 1;
{
    x + 1
}

// Complex postcondition
fn insert[n: nat](
    v: Vector[n],
    idx: i32,
    value: i32
) -> Vector[n+1] pure
    ensures result.len() == n + 1;
    ensures result.data[idx] == value;
    ensures result.len() > v.len();
{
    // Insert implementation
}
```

### Invariants (invariant)
```killer
// Struct invariant - always true for valid instances
struct MinHeap[n: nat] {
    data: Vector[n],
    
    invariant n > 0;
    invariant well_formed_heap(data);
    invariant data.len() == n;
}

// Loop invariant - true at start of each iteration
fn bubble_sort[n: nat](mut v: Vector[n]) -> Vector[n] pure {
    for i in 0..n {
        // invariant: elements before i are sorted
        // invariant: v[0..i] are in final positions
        
        for j in 0..(n-i-1) {
            if v.data[j] > v.data[j+1] {
                swap(&mut v, j, j+1);
            }
        }
    }
    v
}
```

### Requires Clause Combinations
```killer
fn matrix_multiply[m: nat][n: nat][p: nat](
    a: Matrix[m][n],
    b: Matrix[n][p]
) -> Matrix[m][p] pure
    requires m > 0;
    requires n > 0;
    requires p > 0;
    requires a.cols() == b.rows();
{
    // Size compatibility proven by type system
}
```

### Ensures with Result Pattern
```killer
fn find_max[n: nat](
    v: Vector[n]
) -> i32 pure
    requires n > 0;
    ensures result <= max_element(v);
    ensures exists(i in 0..n, v.data[i] == result);
{
    let mut max = v.data[0];
    for i in 1..n {
        if v.data[i] > max {
            max = v.data[i];
        }
    }
    max
}
```

### Contract Polymorphism
```killer
// Different contracts for different types
fn sum_collection[T](coll: T) -> i32 pure
    requires has_len(coll);
    ensures result >= 0 || true;
{
    0
}
```

### Checked Operations
```killer
// Contracts guard checked operations
fn safe_access[n: nat](
    v: Vector[n],
    idx: i32
) -> Option[i32] pure
    requires v.len() == n;
    ensures match result {
        Some(x) => idx >= 0 && idx < n && v.data[idx] == x,
        None => idx < 0 || idx >= n,
    };
{
    if idx >= 0 && idx < n {
        Some(v.data[idx])
    } else {
        None
    }
}
```

### Effect + Contract Combination
```killer
async fn validated_fetch(
    url: String,
    max_size: i32
) -> String uses io
    requires url.len() > 0;
    requires max_size > 0;
    ensures result.len() <= max_size;
{
    let data = fetch(url).await;
    if data.len() <= max_size {
        data
    } else {
        data[0..max_size]
    }
}
```

### Contract Verification
```killer
// Compiler proves contracts or requires verification
#[contract_verified]  // Compiler verified this contract
fn proven_function() -> i32 pure
    ensures result == 42;
{
    42
}

#[contract_unverified]  // Requires runtime check
fn unproven_function(x: i32) -> i32 pure
    ensures result >= x;
{
    // Compiler cannot prove - needs verification
    x
}
```

---

## 5. FEATURE INTERACTIONS

### Dependent Types + Effects
```killer
// Pure computation on sized data
fn pure_sum[n: nat](v: Vector[n]) -> i32 pure {
    let mut sum = 0;
    for i in 0..n {
        sum = sum + v.data[i];
    }
    sum
}

// IO with sized batch
async fn batch_fetch[n: nat](
    urls: Vector[n]
) -> Vector[n] uses io
    requires n >= 0;
{
    join_all(urls.map(fn(url) fetch(url))).await
}
```

### Async + Contracts
```killer
async fn guarded_operation() -> i32 uses io
    requires true;
    ensures result >= 0;
{
    let val = fetch_int().await;
    if val < 0 { 0 } else { val }
}
```

### All Four Features
```killer
async fn comprehensive[n: nat](
    items: Vector[n],
    validate_fn: fn(i32) -> bool pure
) -> Vector[n] uses (io, allocate)
    requires n > 0;
    requires validate_fn is pure;
    ensures result.len() == n;
{
    let validated = items.map(fn(item) {
        if validate_fn(item) { item } else { 0 }
    });
    
    let fetched = join_all(validated.map(fn(item) {
        async { fetch_details(item).await }
    })).await;
    
    fetched
}
```

---

## 6. TYPE SAFETY GUARANTEES

| Feature | Guarantee |
|---------|-----------|
| Dependent Types | No runtime bounds checks needed |
| Effect System | Side effects explicitly declared and tracked |
| Async/Await | Data race prevention via ownership |
| Contracts | Preconditions and postconditions verified |
| Combined | Type-safe, concurrency-safe, effect-safe programming |

---

## 7. COMPILER OPTIMIZATIONS ENABLED

### By Dependent Types:
- Elimination of bounds checks
- Compile-time array sizing
- Perfect hash functions (n known)
- SIMD generation (fixed-width vectors)

### By Effect System:
- Parallelization of pure code
- Caching and memoization
- Dead code elimination
- Optimization across effect boundaries

### By Async/Await:
- Zero-allocation async (stackless)
- Work-stealing scheduling
- Custom async runtime optimization
- Future fusion

### By Contracts:
- Constant folding when preconditions proven
- Code path elimination
- Invariant-based optimization
- SMT solver-guided optimization

---

## 8. QUICK REFERENCE

**Dependent Types:**
```killer
Vector[n]        // Size n, proven at compile time
Matrix[m][n]     // Dimensions m×n, proven
Idx[n]           // Index 0..n-1
fn foo[n: nat]   // Generic function over n
```

**Effects:**
```killer
pure             // No side effects
uses io          // File/network I/O
uses allocate    // Memory allocation
uses (io, allocate)  // Multiple effects
```

**Async/Await:**
```killer
async fn foo()   // Async function
x.await          // Await future
join_all(vec)    // Concurrent collect
spawn(async {})  // Background task
scope(|s| {})    // Structured concurrency
```

**Contracts:**
```killer
requires cond    // Precondition
ensures cond     // Postcondition
invariant cond   // Invariant
```

---

## 9. EXAMPLES

See `/tests/phase1/` for comprehensive examples:
- `comprehensive_01_all_features.killer` - All features integrated
- `advanced_01_edge_cases.killer` - Edge cases and complex patterns
- `advanced_02_performance_patterns.killer` - Optimization patterns

