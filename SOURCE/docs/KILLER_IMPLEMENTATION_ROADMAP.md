# 🔨 Killer Language Implementation Roadmap
## Adding Curriculum Support (v2.2 - v3.0)

**Target**: Make curriculum 100% realistic and teachable  
**Timeline**: 4-12 weeks  
**Impact**: Unlock Weeks 10-22 real-world scenarios  

---

## PHASE 1: v2.2 RELEASE (Weeks 1-2)

### Feature 1.1: System Time API

**What Students Need**:
```killer
let now = system_time_ms()
thread_sleep_ms(100)
let elapsed = system_time_ms() - now
```

**Implementation Location**: `src/v2-rust/killer_vm/src/builtins.rs`

**Code to Add**:
```rust
// In builtins.rs, add:

pub fn builtin_system_time_ms(_args: Vec<Value>, _env: &Rc<RefCell<Environment>>) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    
    Value::Number(duration.as_millis() as f64)
}

pub fn builtin_thread_sleep_ms(args: Vec<Value>, _env: &Rc<RefCell<Environment>>) -> Value {
    if args.is_empty() {
        return Value::Null;
    }
    
    if let Value::Number(ms) = &args[0] {
        std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
        Value::Null
    } else {
        Value::Null
    }
}
```

**Register in Global Scope**:
```rust
// In create_global_environment():

env.define("system_time_ms", Value::Function(BuiltinFunction {
    name: "system_time_ms".to_string(),
    op: builtin_system_time_ms,
}));

env.define("thread_sleep_ms", Value::Function(BuiltinFunction {
    name: "thread_sleep_ms".to_string(),
    op: builtin_thread_sleep_ms,
}));
```

**Tests to Add**:
```rust
#[test]
fn test_system_time_ms() {
    let start = system_time_ms();
    std::thread::sleep(Duration::from_millis(10));
    let end = system_time_ms();
    assert!(end >= start + 10.0);
}

#[test]
fn test_thread_sleep_ms() {
    let start = Instant::now();
    thread_sleep_ms(50);
    assert!(start.elapsed().as_millis() >= 50);
}
```

**Curriculum Impact**:
- ✅ Week 20: Real latency measurement
- ✅ Week 19: Scheduling with actual timing
- ✅ Week 21: Timeout simulation
- ✅ Week 22: Event timestamps

**Updated Examples**:
```killer
// week20_01_latency_measurement.killer (updated)
fn measure_operation(op_fn) {
    let start = system_time_ms()
    op_fn()
    let end = system_time_ms()
    return end - start
}

// Test
fn slow_work() {
    thread_sleep_ms(100)
}

let latency = measure_operation(slow_work)
print("Operation took " + str(latency) + " ms")
```

---

### Feature 1.2: Thread Spawning API

**What Students Need**:
```killer
let handle = spawn_thread(fn() {
    for i in 0..1000 {
        print(i)
    }
})
join_thread(handle)
```

**Implementation Location**: `src/v2-rust/killer_vm/src/thread_runtime.rs` (NEW FILE)

**Code to Add**:
```rust
// NEW FILE: src/v2-rust/killer_vm/src/thread_runtime.rs

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use crate::value::Value;

pub struct ThreadHandle {
    inner: Arc<Mutex<Option<JoinHandle<Value>>>>,
}

impl ThreadHandle {
    pub fn join(&self) -> Value {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(handle) = guard.take() {
                return handle.join().unwrap_or(Value::Null);
            }
        }
        Value::Null
    }
}

pub fn spawn_killer_thread(closure_fn: Value) -> ThreadHandle {
    let handle = thread::spawn(move || {
        // Execute killer closure
        // This requires closure support in Killer
        match closure_fn {
            Value::Function(f) => f.call(vec![]),
            _ => Value::Null,
        }
    });
    
    ThreadHandle {
        inner: Arc::new(Mutex::new(Some(handle))),
    }
}
```

**Register in Global Scope**:
```rust
env.define("spawn_thread", Value::Function(BuiltinFunction {
    name: "spawn_thread".to_string(),
    op: builtin_spawn_thread,
}));

env.define("join_thread", Value::Function(BuiltinFunction {
    name: "join_thread".to_string(),
    op: builtin_join_thread,
}));
```

**Curriculum Impact**:
- ✅ Week 19: True concurrent actors
- ✅ Week 20: Parallel measurement
- ✅ Week 22: Parallel MapReduce

---

## PHASE 2: v2.3 RELEASE (Weeks 3-5)

### Feature 2.1: TCP Socket API

**What Students Need**:
```killer
let listener = TcpListener::new("127.0.0.1", 8080)
let connection = listener.accept()
let data = connection.read(1024)
connection.write(data)
connection.close()
```

**Implementation Location**: `src/v2-rust/killer_vm/src/net.rs` (NEW FILE)

**Code to Add**:
```rust
// NEW FILE: src/v2-rust/killer_vm/src/net.rs

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use crate::value::Value;

pub struct KillerTcpListener {
    inner: TcpListener,
}

pub struct KillerTcpStream {
    inner: TcpStream,
}

impl KillerTcpListener {
    pub fn bind(addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(addr)
            .map_err(|e| e.to_string())?;
        Ok(KillerTcpListener { inner: listener })
    }
    
    pub fn accept(&self) -> Result<KillerTcpStream, String> {
        let (stream, _) = self.inner.accept()
            .map_err(|e| e.to_string())?;
        Ok(KillerTcpStream { inner: stream })
    }
}

impl KillerTcpStream {
    pub fn read(&mut self, size: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0; size];
        let n = self.inner.read(&mut buf)
            .map_err(|e| e.to_string())?;
        buf.truncate(n);
        Ok(buf)
    }
    
    pub fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        self.inner.write_all(data)
            .map_err(|e| e.to_string())?;
        Ok(data.len())
    }
    
    pub fn close(&mut self) -> Result<(), String> {
        // TcpStream auto-closes when dropped
        Ok(())
    }
}
```

**Killer API Surface**:
```rust
env.define("TcpListener", Value::Object(map!{
    "new" => builtin_tcp_listener_new,
    "accept" => builtin_tcp_listener_accept,
}));

fn builtin_tcp_listener_new(args: Vec<Value>) -> Value {
    // Create and return listener value
}

fn builtin_tcp_stream_read(stream: Value, size: Value) -> Value {
    // Read from stream
}
```

**Curriculum Impact**:
- ✅ Week 21: Real HTTP servers
- ✅ Week 10-11: Real RPC implementation
- ✅ Week 15-18: Distributed services

**Updated Examples**:
```killer
// week21_http_server.killer (NEW)
let listener = TcpListener::new("127.0.0.1", 8080)
print("Server listening on 127.0.0.1:8080")

for i in 0..10 {
    let connection = listener.accept()
    let request = connection.read(1024)
    
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello"
    connection.write(response)
    connection.close()
}
```

---

### Feature 2.2: HTTP Server Framework

**Killer API**:
```killer
let server = HttpServer::new("127.0.0.1", 8080)

server.get("/", fn(request) {
    return {status: 200, body: "Hello"}
})

server.post("/api/users", fn(request) {
    let user = json_parse(request.body)
    return {status: 201, body: json_stringify(user)}
})

server.start()
```

**Implementation Notes**:
- Build on top of TCP socket API
- Handle HTTP parsing
- Routing table for endpoints
- Request/response structures

---

## PHASE 3: v3.0 RELEASE (Weeks 6-12)

### Feature 3.1: Lightweight Task Scheduler

**What Students Need**:
```killer
let scheduler = Scheduler::new()

let task1 = scheduler.spawn(fn() {
    for i in 0..100 {
        print("Task 1: " + str(i))
    }
})

let task2 = scheduler.spawn(fn() {
    for i in 0..100 {
        print("Task 2: " + str(i))
    }
})

scheduler.run()
```

**Implementation Strategy**:
- Not full async/await, but lightweight tasks
- Cooperative multitasking
- Task queues and scheduling
- Similar to Go's goroutines

---

### Feature 3.2: Async/Await Runtime (Optional)

**Full async/await** using Tokio integration:
```killer
async fn fetch_data(url) {
    // Non-blocking network I/O
}

async fn main() {
    let data = await fetch_data("https://...")
    print(data)
}

await main()
```

---

## IMPLEMENTATION TIMELINE

### Week 1
- [ ] Add `system_time_ms()` - 2 hours
- [ ] Add `thread_sleep_ms()` - 1 hour
- [ ] Add `spawn_thread()` - 3 hours
- [ ] Add `join_thread()` - 1 hour
- [ ] Testing & documentation - 2 hours
- [ ] **Total**: ~9 hours

### Week 2
- [ ] TCP socket API skeleton - 4 hours
- [ ] `TcpListener::bind()` - 2 hours
- [ ] `TcpListener::accept()` - 2 hours
- [ ] `TcpStream::read()` - 2 hours
- [ ] `TcpStream::write()` - 2 hours
- [ ] Testing & documentation - 3 hours
- [ ] **Total**: ~15 hours

### Week 3-4
- [ ] HTTP server framework - 10 hours
- [ ] Request parsing - 4 hours
- [ ] Response building - 2 hours
- [ ] Routing table - 3 hours
- [ ] Testing & examples - 5 hours
- [ ] **Total**: ~24 hours

### Week 5-6
- [ ] Lightweight task scheduler - 15 hours
- [ ] Task queues - 3 hours
- [ ] Scheduling algorithm - 3 hours
- [ ] Testing & documentation - 3 hours
- [ ] **Total**: ~24 hours

### Week 7-12 (Optional)
- [ ] Async/await integration with Tokio - 20+ hours

---

## FEATURE COMPLETION CHECKLIST

### v2.2 Features (Timing & Threading)
- [ ] `system_time_ms()` implemented
- [ ] `thread_sleep_ms()` implemented
- [ ] `spawn_thread()` implemented
- [ ] `join_thread()` implemented
- [ ] Examples updated (week19_*, week20_*)
- [ ] Tests passing
- [ ] Documentation written
- [ ] Release notes updated

### v2.3 Features (Networking)
- [ ] `TcpListener` implemented
- [ ] `TcpStream` implemented
- [ ] HTTP parsing library added
- [ ] HTTP server framework
- [ ] Examples: `week21_http_server.killer`
- [ ] Tests passing
- [ ] Real HTTP server works
- [ ] Release notes updated

### v3.0 Features (Advanced)
- [ ] Lightweight scheduler
- [ ] Async task support
- [ ] Better error handling
- [ ] Performance optimization
- [ ] Full curriculum support

---

## TESTING STRATEGY

### For Each Feature:
1. **Unit Tests**: Test function in isolation
2. **Integration Tests**: Test with curriculum examples
3. **Curriculum Validation**: Run all 400+ problem solutions
4. **Performance Tests**: Verify no major slowdown

### Curriculum Validation
```bash
# After each feature, run:
for week in 19 20 21 22; do
    for example in examples/week${week}_*.killer; do
        killer $example
        if [ $? -ne 0 ]; then
            echo "FAILED: $example"
        fi
    done
done
```

---

## DOCUMENTATION UPDATES

### For Each Feature:
1. **API Documentation**: Function signature, parameters, return type
2. **Examples**: Show usage in context
3. **Curriculum Integration**: Map to curriculum weeks
4. **Limitations**: What it doesn't do (yet)

### Files to Update:
- `DOCUMENTATION_INDEX.md` - Add new features
- `README.md` - Update features list
- `examples/WEEKS_19_22_EXAMPLES_README.md` - Update example status
- Release notes for each version

---

## RESOURCE REQUIREMENTS

### Developer Time
- Lead: 60-80 hours (4-6 weeks full-time)
- Reviewer: 20-30 hours (design reviews, testing)
- Total: ~100 hours across phases

### Testing Infrastructure
- Continuous integration (already have)
- Curriculum test suite (to create)
- Performance benchmarks (to establish baselines)

---

## SUCCESS METRICS

### Feature Success
✅ Feature is implemented  
✅ Tests pass  
✅ Examples run  
✅ No performance regression  

### Curriculum Success
✅ All 400+ problems have example solutions  
✅ Students can run examples  
✅ Latency targets achievable  
✅ Network I/O works end-to-end  

---

## RISK MITIGATION

### Risk: Threading Unsafe
- Mitigation: Add Mutex wrappers, document safety
- Test with TSAN (Thread Sanitizer)

### Risk: Socket Errors
- Mitigation: Comprehensive error handling
- Document common errors
- Provide error recovery examples

### Risk: Performance Regression
- Mitigation: Benchmark before/after each change
- Compare to baseline
- Optimize hot paths

---

## APPROVAL PROCESS

1. **Design Review**: Team reviews architecture
2. **Implementation**: Developer implements feature
3. **Testing**: QA validates completeness
4. **Curriculum Validation**: Run against all examples
5. **Documentation**: Update all relevant docs
6. **Release**: Tag version, update roadmap

---

## Next Steps

1. **Today**: Approve this roadmap
2. **Tomorrow**: Start v2.2 Phase 1 (timing & threading)
3. **Week 2**: Complete Phase 1, start Phase 2 (sockets)
4. **Week 4**: Complete Phase 2, prepare Phase 3
5. **Week 12**: All features in place, curriculum 100% realistic

---

**Recommendation**: Start with v2.2 Phase 1 immediately. Timing & threading are foundational for everything else.

**Expected Outcome**: By Week 4, curriculum will be 85-90% realistic. By Week 12, fully production-ready.
