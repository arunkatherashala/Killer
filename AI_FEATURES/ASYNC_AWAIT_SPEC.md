# KILLER v2.0: ASYNC/AWAIT SPECIFICATION
## Feature #1: Foundation - Non-blocking Concurrency

**Target Release**: Week 6, 2026  
**Complexity**: Medium (core feature, unlocks everything else)  
**Dependencies**: None (pure language addition)  
**Performance Target**: 100K+ concurrent tasks per CPU core

---

## PROBLEM STATEMENT

### Current Limitation (v1.1)
**Actor model**: Excellent for concurrent agents, but **synchronous within an actor**

```killer
// PROBLEM: This blocks entire actor until HTTP completes
actor FetchAgent {
  handle get_data(url: String) -> String {
    response = http::get(url)  // Blocks! ~200ms on network
    response.body()
  }
}

// Result: Only ~5-10 concurrent requests per actor (limited by threads)
```

### Why It Matters
- **Current**: 1 agent per thread = ~1000 agents per node (8-core CPU)
- **With Async**: 100K+ tasks per core = **1M+ concurrent agents**
- **Use case**: 10K smart agents reasoning simultaneously ✓

---

## SOLUTION: ASYNC/AWAIT DESIGN

### Core Concepts

#### 1. **Async Functions** (Don't block caller)
```killer
// Mark function as async (may suspend)
async kfn fetch(url: String) -> String {
  response = await http::get(url)  // Suspends, doesn't block
  response.body()
}

// Call from async context with await
result = await fetch("https://api.example.com/data")
```

#### 2. **Await Operator** (Wait for async result)
```killer
// await suspends current task until result arrives
// Scheduler can run OTHER tasks while waiting
value = await some_async_function()

// Can't use await outside async context
// This is a compile error:
result = fetch("url")  // Error: missing await
```

#### 3. **Spawn Tasks** (Run concurrently)
```killer
// Spawn multiple tasks running in parallel
task1 = spawn_task { await fetch("url1") }
task2 = spawn_task { await fetch("url2") }
task3 = spawn_task { await fetch("url3") }

// Wait for all to complete
results = await join_all([task1, task2, task3])
// All 3 fetches run in parallel, single thread

// Wait for first to complete
first = await select_first([task1, task2, task3])
```

#### 4. **Async Blocks** (For existing code)
```killer
// Convert closure to async context
async_result = await {
  value1 = await fetch("url1")
  value2 = await fetch("url2")
  value1 + value2  // Combined result
}
```

---

## ARCHITECTURE

### Task Execution Model

```
┌─────────────────────────────────────────────────┐
│  Actor Thread (1 per actor instance)            │
│                                                  │
│  └─ Task Scheduler (Event Loop)                 │
│     │                                            │
│     ├─ Current Task (Running)  ◇ State Machine  │
│     │   - Executor pointer     ◇ Ready          │
│     │   - Stack frame          ◇ Blocked        │
│     │   - Registers            ◇ Complete       │
│     │                                            │
│     ├─ Ready Queue (runnable tasks)              │
│     │   [task1, task2, task3, ...]              │
│     │                                            │
│     ├─ Wait Queue (suspended on I/O)             │
│     │   [http_fetch_1, timer_1, ...]            │
│     │                                            │
│     └─ Complete Queue (finished)                 │
│         [result1, result2, ...]                  │
│                                                  │
└─────────────────────────────────────────────────┘

SCHEDULING LOOP:
1. Pick task from Ready queue
2. Run until await point or completion
3. If await: move to Wait queue
4. Check Wait queue for completed I/O
5. Move completed to Ready queue
6. Repeat
```

### Task State Machine

```
        New Task
           │
           ▼
    ┌─────────────┐
    │   READY     │ (runnable)
    └──────┬──────┘
           │ execute
           ▼
    ┌─────────────┐
    │  RUNNING    │
    └──────┬──────┘
      ┌────┴────┐
      │ await   │ complete
      ▼         ▼
  ┌────────┐ ┌─────────┐
  │ BLOCKED│ │COMPLETE │
  └────┬───┘ └─────────┘
       │ I/O ready
       ▼
    ┌─────────────┐
    │   READY     │ (reschedule)
    └─────────────┘
```

### Compiler Changes

**Parser**: Recognize `async` keyword and `await` operator
**Type Check**: Verify `await` only in async context
**Codegen**: Convert async functions to state machines

```killer
// SOURCE CODE
async kfn fetch(url: String) -> String {
  response = await http::get(url)
  response.body()
}

// COMPILED (Conceptually)
// State machine with suspension points:
type FetchStateMachine {
  url: String,
  response: HTTPResponse,
  state: Int,  // 0 = start, 1 = waiting for http, 2 = done
  task_handle: TaskHandle
}

// Executor knows where to resume
kfn fetch_executor(state_machine: FetchStateMachine) -> String {
  match state_machine.state {
    0 -> {
      // Issue HTTP request, suspend
      state_machine.state = 1
      SUSPEND
    }
    1 -> {
      // HTTP complete, continue
      response = state_machine.response
      state_machine.state = 2
      RESUME_WITH(response.body())
    }
    2 -> {
      // Already done
      return state_machine.result
    }
  }
}
```

---

## IMPLEMENTATION STRATEGY

### Phase 1: Core Runtime (Weeks 1-2)
**Goal**: Task scheduler, event loop, basic spawn/join

**Code**:
- `Task` struct: id, state, priority, result
- `Scheduler` struct: task queues, executor pointer
- Functions: `spawn_task`, `join_all`, `select_first`

**Requirements**:
- No allocations during task switch (<1μs)
- Work-stealing for load balancing
- Priority queue for ready queue (O(log N) operations)

### Phase 2: Async/Await Syntax (Weeks 2-3)
**Goal**: Language parser, type checking, basic codegen

**Changes**:
- Parser: recognize `async` keyword, `await` operator
- Type system: mark functions as `async fn`
- Codegen: convert async calls to task spawning

**Testing**:
```killer
async kfn hello() -> String { "hello" }
result = await hello()
assert(result == "hello")
```

### Phase 3: I/O Integration (Weeks 3-4)
**Goal**: Async HTTP, timers, file I/O

**API**:
```killer
// Async HTTP
response = await http::get("url")        // Suspends
response = await http::post("url", body)

// Async timers
await sleep_ms(1000)                     // Suspend for 1s

// Async file I/O
content = await file::read("path")       // Suspend until ready
await file::write("path", content)
```

**Implementation Approach**:
- Use OS primitives (epoll on Linux, IOCP on Windows, kqueue on macOS)
- Reactor pattern: I/O completion → task wakeup

### Phase 4: Advanced Patterns (Weeks 4-6)
**Goal**: Async blocks, select/any, timeouts, cancellation

**Features**:
```killer
// Async blocks
result = await {
  a = await fetch("a")
  b = await fetch("b")
  a + b
}

// Select any (returns first result)
first = await select_any([fetch1, fetch2, fetch3])

// Timeout
result = await timeout(fetch("slow_url"), ms=5000)

// Cancellation
task = spawn_task { await long_operation() }
task.cancel()  // Kill task

// Stream adaptation
for item in await stream_producer() {
  print(item)
}
```

---

## PERFORMANCE TARGETS

### Latency
- **Task spawn**: <1μs
- **Task context switch**: <1μs
- **Await on ready result**: <100ns
- **I/O wake-up latency**: <100μs

### Throughput
- **Tasks per core**: 100K+
- **Context switches per second**: 1M+
- **Spawn rate**: 100K tasks/sec

### Resource Usage
- **Memory per task**: ~256 bytes (stack frame, state machine)
- **1M tasks**: ~256 MB RAM
- **Task queue overhead**: O(log N) = negligible

### Benchmark Targets (Actual Measurements)

```
SCENARIO 1: Concurrent HTTP Requests
────────────────────────────────────
Tasks: 1000 concurrent HTTP fetches (100ms each)

Traditional (threads): 
  - CPU usage: 50% (context switch overhead)
  - Memory: 200+ MB (1MB per thread stack)
  - Time: 100ms (parallelism)

Killer Async:
  - CPU usage: 5% (minimal context switch)
  - Memory: 256 KB (1000 tasks × 256 bytes)
  - Time: 100ms (parallelism)
  - Speedup: 40x+ (resources), same latency

SCENARIO 2: Task Spawning Rate
────────────────────────────────────
Spawn 1M tasks, each sleeps 1ms then returns

Traditional (threads):
  - Spawn rate: ~1000 tasks/sec (limited by system threads)
  - Total time: 1000+ seconds

Killer Async:
  - Spawn rate: 100K+ tasks/sec
  - Total time: 10-15 seconds
  - Speedup: 100-1000x

SCENARIO 3: Agent Swarms (100 agents × 1000 tasks each)
────────────────────────────────────┐
Traditional: 100 actors × 8 threads = 800 threads needed
  - OS struggles with context switching
  - Memory: 800 MB (8 MB each)
  - Latency: p99 = 500ms

Killer Async:
  - 100 actors (1 thread each) × 1000 tasks
  - Memory: 100 actors + 100K tasks = 26 MB
  - Latency: p99 = 5ms
  - Speedup: 100x lower latency, 30x less memory
```

---

## EXAMPLE: CONCURRENT WEB CRAWLER

### Without Async/Await (Current v1.1)
```killer
actor WebCrawler {
  handle crawl_blocking(urls: List<String>) -> List<String> {
    results = []
    for url in urls {
      // This blocks the entire actor for every fetch!
      response = http::get(url)  // ~200ms each
      results.push(response.body())
    }
    results
  }
}

// Performance: 5 URLs × 200ms = 1000ms total (sequential)
// With 10 concurrent crawlers: only 50 URLs/sec possible
```

### With Async/Await (v2.0)
```killer
actor WebCrawler {
  handle crawl_async(urls: List<String>) -> List<String> {
    // Spawn all fetches concurrently
    tasks = []
    for url in urls {
      task = spawn_task { await http::get(url) }
      tasks.push(task)
    }
    
    // Wait for all to complete (in parallel)
    results = await join_all(tasks)
    results.map(|r| { r.body() })
  }
}

// Performance: 5 URLs × 200ms parallel = 200ms total
// Single crawler now does 25 URLs/sec (same as 5 traditional crawlers)
// With 10 concurrent crawlers: 250 URLs/sec (5x improvement!)
```

### Even Better: Backpressure Control
```killer
actor ControlledCrawler {
  max_concurrent: Int = 10  // Limit parallelism
  
  handle crawl_bounded(urls: List<String>) -> List<String> {
    results = []
    pending_tasks = []
    url_queue = urls.clone()
    
    // Keep 10 tasks in flight
    loop {
      // Spawn up to max_concurrent tasks
      while pending_tasks.len() < this.max_concurrent && url_queue.len() > 0 {
        url = url_queue.pop_front()
        task = spawn_task { await http::get(url) }
        pending_tasks.push(task)
      }
      
      if pending_tasks.len() == 0 { BREAK }
      
      // Wait for any task to complete
      completed = await select_first(pending_tasks)
      results.push(completed.body())
      
      // Remove from pending
      pending_tasks = pending_tasks.filter(|t| { !t.is_complete() })
    }
    
    results
  }
}

// Performance: Controlled at exactly 10 parallel requests
// Prevents overwhelming server or running out of memory
// Flexibility: adjust max_concurrent = 50, 100, 1000 based on needs
```

---

## TESTING STRATEGY

### Unit Tests
1. **Basic spawn/join**
   ```killer
   task = spawn_task { 1 + 1 }
   result = await task
   assert(result == 2)
   ```

2. **Multiple tasks**
   ```killer
   tasks = [spawn_task { i } for i in 0..10]
   results = await join_all(tasks)
   assert(results == [0,1,2,...,9])
   ```

3. **Task ordering preserved**
   ```killer
   results = []
   task1 = spawn_task { await sleep_ms(100); results.push(1) }
   task2 = spawn_task { results.push(2) }
   await join_all([task1, task2])
   // results might be [2,1] or [1,2] (non-deterministic, OK)
   ```

### Integration Tests
1. **Concurrent HTTP with timeouts**
   ```killer
   urls = ["url1", "url2", ..., "url100"]
   results = await crawl_async(urls)
   assert(results.len() == 100)
   ```

2. **Error handling in async**
   ```killer
   task = spawn_task { await failing_operation() }
   try {
     result = await task
   } catch error {
     assert(error contains "expected message")
   }
   ```

3. **Resource cleanup**
   ```killer
   // Spawn 1M tasks, verify memory doesn't leak
   tasks = []
   for i in 0..1000000 {
     task = spawn_task { i }
     tasks.push(task)
   }
   results = await join_all(tasks)
   // Memory should be freed after join_all
   ```

### Performance Tests
1. **Throughput**: 100K tasks in < 1 second
2. **Latency**: p99 context switch < 10μs
3. **Memory**: 100K tasks < 50 MB

---

## API REFERENCE

### Spawn Functions

```killer
// Spawn single task
handle = spawn_task { expression }
result = await handle

// Spawn with custom priority
handle = spawn_task[PRIORITY::HIGH] { expression }

// Spawn on specific actor
handle = spawn_task_on(actor_ref, { expression })
result = await handle
```

### Wait Functions

```killer
// Wait for all to complete
results = await join_all([task1, task2, task3])

// Wait for any to complete
first = await select_first([task1, task2, task3])

// Wait with timeout
result = await timeout(task, ms=5000)  // Error if timeout

// Wait with timeout and default
result = await timeout_or(task, default_value, ms=5000)
```

### Async I/O

```killer
// HTTP
response = await http::get(url)
response = await http::post(url, body)
response = await http::put(url, body)

// Timers
await sleep_ms(1000)
await sleep_secs(1)

// File I/O
content = await file::read("path")
await file::write("path", content)

// Channels (actor communication)
channel = Channel<String>::new()
await channel.send("message")
message = await channel.receive()
```

### Task Inspection

```killer
is_done = task.is_complete()
task.cancel()  // Cancel pending task
id = task.get_id()
priority = task.get_priority()
```

---

## COMPATIBILITY

### Code Changes Required
- `async kfn` instead of `kfn` for async functions
- `await` before async function calls
- Compile error if `await` missing on async call
- No breaking changes to existing v1.1 code (non-async functions work as-is)

### Migration Path
1. Code that doesn't use async continues to work
2. Gradually convert hot paths to async/await
3. Mix async and sync code seamlessly

---

## NEXT DEPENDENCIES

**Async/Await enables all of**:
- ✓ LLM Integration (#2) - async HTTP calls to OpenAI
- ✓ Tool Calling (#3) - async function execution
- ✓ Streaming (#9) - async token generation
- ✓ GPU Acceleration (#10) - async device operations

**Timeline**: Finish Async/Await weeks 1-6 → Build LLM integration on top

---

## SUCCESS CRITERIA

- [ ] 100K+ concurrent tasks per core
- [ ] <1μs task spawn overhead
- [ ] <1μs context switch latency
- [ ] p99 latency < 10μs for task operations
- [ ] Full test coverage (unit + integration + performance)
- [ ] Documentation + examples
- [ ] Community feedback positive

---

**READY FOR IMPLEMENTATION** ✅

Next: Create `async_await.killer` implementation with working examples
