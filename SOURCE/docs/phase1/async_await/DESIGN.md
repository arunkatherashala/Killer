# Killer Async/Await Design
## Phase 1 Weeks 12-15

### Overview
Asynchronous programming enables:
- **Non-blocking I/O**: 1000s of concurrent connections with minimal threads
- **Fine-grained concurrency**: Control over when tasks yield
- **Structured concurrency**: Scope-based task management
- **Sub-microsecond context switching**: <100ns overhead

**Power Example:**
```killer
// Synchronous: blocks entire thread during I/O
fn fetch_sequential_sync() -> i32 uses io {
    let r1 = fetch("url1");  // Blocks ~100ms
    let r2 = fetch("url2");  // Blocks ~100ms
    r1.len() + r2.len()      // Total: ~200ms
}

// Asynchronous: interleaves I/O, runs concurrently
async fn fetch_concurrent() -> i32 uses io {
    let t1 = async { fetch("url1") };  // Start I/O
    let t2 = async { fetch("url2") };  // Start I/O
    let r1 = t1.await;  // Wait for both (parallel)
    let r2 = t2.await;
    r1.len() + r2.len()  // Total: ~100ms
}
```

---

## Core Concepts

### Async Functions

```killer
// Regular function: blocks
fn sync_read(path: String) -> String uses io {
    read_file(path)
}

// Async function: can be interrupted, resumed
async fn async_read(path: String) -> String uses io {
    read_file(path)
}

// Type: async fn returns Future
let future: Future<String> = async_read("file.txt");

// Await: wait for async operation
let result = future.await;
```

### Reading & Writing with Async

```killer
// Async I/O operations
async fn fetch(url: String) -> String uses io {
    http_get(url).await
}

async fn save(path: String, data: String) -> void uses io {
    write_file(path, data).await
}

// Chaining async operations
async fn download_and_save(url: String, path: String) -> void uses io {
    let data = fetch(url).await;
    save(path, data).await;
}
```

### Concurrent Execution with `join`

```killer
// Run multiple async operations concurrently
async fn fetch_multiple(urls: [String]) -> [String] uses io {
    let futures = urls.map(fn(url) { fetch(url) });
    
    // Wait for all to complete (in parallel)
    let results = join_all(futures).await;
    
    results  // All downloads happened concurrently
}
```

### Task Spawning

```killer
// Spawn independent task (fire and forget)
async fn background_job() {
    spawn(async {
        log("Background task running").await;
    });
}

// Spawn and track
async fn main() {
    let handle = spawn(async {
        expensive_computation()
    });
    
    // Do other work while background task runs
    let result = handle.await;
}
```

---

## Implementation Roadmap

### Week 12: Core Infrastructure
**Goal**: Build async runtime and Future type

Tasks:
- [ ] Implement `Future<T>` trait
- [ ] Create async/await keyword recognition in lexer/parser
- [ ] Add `async fn` function variant to AST
- [ ] Implement basic executor (single-threaded)
- [ ] Add `.await` operator to parser

**Deliverable**: Simple async code compiles
```killer
async fn hello() -> String {
    "world"
}
```

### Week 13: Await Support
**Goal**: Enable waiting on futures

Tasks:
- [ ] Implement `.await` expression in parser & type checker
- [ ] Transform `await` into executor calls
- [ ] Handle `.await` type constraints (must be `Future<T>`)
- [ ] Error messages for `.await` outside async context

**Deliverable**: Can call async functions and await results
```killer
async fn main() {
    let result = hello().await;
    println(result);
}
```

### Week 14: Concurrency
**Goal**: Enable parallel async operations

Tasks:
- [ ] Implement `join_all(futures)` for concurrent execution
- [ ] Implement `spawn(task)` for background tasks
- [ ] Add multi-threaded executor
- [ ] Thread pool integration
- [ ] Cancellation token support

**Deliverable**: Can run multiple async tasks concurrently
```killer
async fn fetch_all(urls: [String]) -> [String] uses io {
    join_all(urls.map(fn(u) { fetch(u) })).await
}
```

### Week 15: Integration & Polish
**Goal**: Integrate with other Phase 1 features

Tasks:
- [ ] Async + Effect System integration
- [ ] Async + Dependent Types (e.g., `Vector[n]` over async)
- [ ] Async + Contracts (async preconditions/postconditions)
- [ ] Error handling with async
- [ ] Performance optimization

**Deliverable**: Production-ready async/await
```killer
async fn safe_fetch[n: nat](
    urls: Vector[n]
) -> Vector[n] uses io {
    join_all(urls.map(fetch)).await
}
```

---

## Design Decisions

### Why Async/Await?
1. **Performance**: 1000s of concurrent connections with few threads
2. **Simplicity**: Async/await clearer than callbacks or threads
3. **Composability**: Easily chain async operations
4. **Efficiency**: Sub-microsecond context switch overhead
5. **Type-safe**: Async operations tracked in type system

### Runtime Choice: Custom Implementation
**Selected: Custom single-threaded + multi-threaded executor**

Rationale:
- [ ] Full control over behavior
- [ ] Integrates with Killer's type system
- [ ] Minimal dependencies
- [ ] Can optimize for Killer's use cases
- [ ] Educational value

Alternative considered: Tokio
- Pros: Battle-tested, huge ecosystem
- Cons: Heavy, complex, third-party dependency

### Scheduler Design

```
┌─────────────────────┐
│   Killer Executor   │
├─────────────────────┤
│ Work Queue          │
│ Ready Tasks         │
│ Waiting Futures     │
├─────────────────────┤
│ User Threads (N)    │
│ (Task Stealers)     │
└─────────────────────┘
```

Single-threaded Phase 1, multi-threaded Phase 2+

---

## Examples

### Example 1: Sequential vs Concurrent I/O
```killer
// Sequential: takes ~100ms
async fn sequential() -> i32 uses io {
    let a = fetch("url1").await;  // ~50ms
    let b = fetch("url2").await;  // ~50ms
    a.len() + b.len()
}

// Concurrent: takes ~50ms (parallel)
async fn concurrent() -> i32 uses io {
    let f1 = fetch("url1");
    let f2 = fetch("url2");
    let a = f1.await;
    let b = f2.await;
    a.len() + b.len()
}

// Using join_all: ~50ms (parallel)
async fn joined() -> i32 uses io {
    let results = join_all([
        fetch("url1"),
        fetch("url2"),
    ]).await;
    
    results[0].len() + results[1].len()
}
```

### Example 2: Background Tasks
```killer
async fn download_in_background(urls: [String]) {
    // Spawn without waiting
    for url in urls {
        spawn(async {
            let data = fetch(url).await;
            save_to_cache(url, data).await;
        });
    }
    println("Downloads queued");
    // Downloads happen in background
}

// Main task can do other work
async fn main() {
    download_in_background(["url1", "url2", "url3"]);
    
    let local_work = heavy_computation();  // Runs while downloads happen
    println("Done");
}
```

### Example 3: Error Handling
```killer
// Async with error handling
async fn fetch_safe(url: String) -> Result<String> uses io {
    try {
        fetch(url).await
    } catch (e) {
        Err("Failed to fetch: " + e)
    }
}

// Propagate errors
async fn fetch_all_safe(urls: [String]) -> Result<[String]> uses io {
    let results = join_all(urls.map(fetch_safe)).await;
    
    // Check all succeeded
    for result in results {
        if result is Err {
            return Err("One fetch failed");
        }
    }
    
    Ok(results.map(fn(r) { r.value } ))
}
```

### Example 4: Async with Dependent Types
```killer
// Type-safe: always returns Vector of size n
async fn fetch_n[n: nat](
    urls: Vector[n]
) -> Vector[n] uses io {
    join_all(urls.map(fetch)).await
}

// Guaranteed to get exactly n results
async fn process() {
    let urls = Vector[5] { "u1", "u2", "u3", "u4", "u5" };
    let results = fetch_n(urls).await;
    // results.len() == 5 (proven at compile time)
}
```

### Example 5: Structured Concurrency
```killer
// Scope ensures all tasks complete
async fn with_scope() {
    scope(async {
        // Spawn tasks in scope
        spawn_in_scope(async {
            heavy_work_1().await;
        });
        
        spawn_in_scope(async {
            heavy_work_2().await;
        });
        
        // Implicit: wait for all tasks before exiting scope
    });
    
    // All background work complete here
    println("All tasks done");
}
```

---

## Testing Strategy

### Compilation Tests
```
✓ async fn syntax recognized
✓ .await only in async context
✗ .await in synchronous function (error)
✓ Future<T> type inference
✗ Type mismatch in async (error)
```

### Runtime Tests (20+ cases)
```
✓ Basic async function execution
✓ Multiple concurrent tasks
✓ Join_all waits for all tasks
✓ Spawn background tasks
✓ Cancellation
✓ Error propagation in async
✓ Nested async/await
✓ Timeout support
```

### Performance Tests
```
✓ Context switch < 100ns
✓ 10,000 concurrent tasks
✓ No memory leaks under load
✓ CPU usage within expected bounds
```

---

## Files to Create

```
docs/phase1/async_await/
  ├── DESIGN.md (this file)
  ├── EXECUTION_PLAN.md
  ├── RUNTIME_INTERNALS.md
  └── examples/
      ├── basic_async.killer
      ├── concurrent_io.killer
      ├── spawn_tasks.killer
      ├── error_handling.killer
      └── with_other_features.killer

src/v2-rust/killer_vm/src/
  ├── async_runtime.rs
  ├── future.rs
  ├── executor.rs
  └── scheduler.rs
```

---

## Success Criteria

✓ `async fn` syntax parsing
✓ `.await` operator implemented
✓ Basic executor running tasks
✓ `join_all` for concurrent execution
✓ `spawn` for background tasks
✓ Multi-threaded executor
✓ <100ns context switch overhead
✓ Tests pass: 20+ async examples
✓ Parallel I/O works (2-10x faster than sequential)

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Context switch | <100ns |
| Task spawn | <1µs |
| Concurrent tasks | 10,000+ |
| I/O throughput | 100,000+ ops/sec |

---

## Integration with Other Phase 1 Features

### Async + Effects
```killer
async fn download() -> String uses io {
    fetch("url").await
}
```

### Async + Dependent Types
```killer
async fn fetch_batch[n: nat](urls: Vector[n]) -> Vector[n] uses io {
    join_all(urls.map(fetch)).await
}
```

### Async + Contracts
```killer
async fn safe_fetch(url: String) -> String uses io
    requires len(url) > 0
    ensures result.len() > 0
{
    fetch(url).await
}
```

---

## Next Steps After Week 15

Contract Programming (Week 16-18)
- Runtime assertion for contract violations
- Integration with async error handling
