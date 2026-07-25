# Week 9: Message Passing & Channels - Quick Reference
## Patterns, implementations, and troubleshooting guide

---

## TABLE OF CONTENTS
1. Channel Type Summary
2. Implementation Patterns
3. Common Problems & Solutions
4. Performance Tuning
5. Debugging Guide
6. Integration Patterns
7. Code Templates
8. Glossary

---

## 1. CHANNEL TYPE SUMMARY

### Quick Comparison

| Type | Senders | Receivers | Ordering | Use Case |
|------|---------|-----------|----------|----------|
| **SPSC** | 1 | 1 | Strict FIFO | Pipeline stage |
| **MPSC** | N | 1 | Per-sender FIFO | Fan-in aggregator |
| **SPMC** | 1 | N | Shared | Pub-Sub, Broadcast |
| **MPMC** | N | N | None | Work queue, Pool |

### When to Use Each

**SPSC:**
```rust
// Perfect for: Pipeline stage, producer-consumer pair
// Fastest: Minimal synchronization
// Example:
let (tx, rx) = channel();
thread::spawn(move || {
    for item in input {
        tx.send(item).ok();
    }
});
for item in rx {
    process(item);
}
```

**MPSC:**
```rust
// Perfect for: Aggregating from multiple sources
// Common: Thread pool worker threads
// Example:
let (tx, rx) = channel();
for i in 0..10 {
    let tx = tx.clone();
    thread::spawn(move || {
        tx.send(compute(i)).ok();
    });
}
for result in rx {
    output(result);
}
```

**SPMC:**
```rust
// Perfect for: Broadcasting events to many subscribers
// Challenge: Late subscribers miss old messages
// Example:
let (tx, rx) = broadcast::channel(16);
let user1 = rx.subscribe();
let user2 = rx.subscribe();
tx.send("Hello, everyone!").ok();
```

**MPMC:**
```rust
// Perfect for: Work queue with competing workers
// Challenge: Coordinating shutdown
// Example:
let (tx, rx) = Arc::new(mpsc::channel());
let rx = Arc::new(Mutex::new(rx));
for _ in 0..4 {
    let rx = Arc::clone(&rx);
    thread::spawn(move || {
        while let Ok(job) = rx.lock().unwrap().recv() {
            process(job);
        }
    });
}
```

---

## 2. IMPLEMENTATION PATTERNS

### Pattern 1: Basic Message Queue

```rust
use std::sync::mpsc::channel;

fn basic_queue() {
    let (tx, rx) = channel();
    
    // Send
    tx.send("message").ok();
    
    // Receive
    match rx.recv() {
        Ok(msg) => println!("Got: {}", msg),
        Err(_) => println!("Channel closed"),
    }
}
```

### Pattern 2: Non-Blocking Receive

```rust
use std::sync::mpsc::TryRecvError;

fn non_blocking_recv() {
    let (tx, rx) = channel();
    tx.send("msg").ok();
    
    match rx.try_recv() {
        Ok(msg) => println!("Got: {}", msg),
        Err(TryRecvError::Empty) => println!("No message"),
        Err(TryRecvError::Disconnected) => println!("Sender gone"),
    }
}
```

### Pattern 3: Timeout Receive

```rust
use std::time::Duration;

fn timeout_recv() {
    let (tx, rx) = channel();
    
    match rx.recv_timeout(Duration::from_secs(1)) {
        Ok(msg) => println!("Got: {}", msg),
        Err(e) => println!("Timeout or disconnected"),
    }
}
```

### Pattern 4: Channel Cloning for Multiple Senders

```rust
fn multiple_senders() {
    let (tx, rx) = channel();
    
    // Create multiple producers
    for i in 0..3 {
        let tx = tx.clone(); // Clone for each thread
        std::thread::spawn(move || {
            tx.send(format!("from {}", i)).ok();
        });
    }
    
    drop(tx); // Drop original, keep clones in threads
    
    // Receive all
    for msg in rx {
        println!("{}", msg);
    }
}
```

### Pattern 5: Request-Response with Correlation ID

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Debug)]
struct Request {
    id: u64,
    command: String,
}

#[derive(Clone, Debug)]
struct Response {
    id: u64,
    result: String,
}

fn request_response() {
    let (req_tx, req_rx) = channel::<Request>();
    let (resp_tx, resp_rx) = channel::<Response>();
    
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let pending_clone = Arc::clone(&pending);
    
    // Responder thread
    std::thread::spawn(move || {
        while let Ok(req) = req_rx.recv() {
            let result = format!("Result of {}", req.command);
            let resp = Response {
                id: req.id,
                result,
            };
            resp_tx.send(resp).ok();
        }
    });
    
    // Send request
    let req_id = 42;
    req_tx.send(Request {
        id: req_id,
        command: "do_work".to_string(),
    }).ok();
    
    // Track pending
    pending.lock().unwrap().insert(req_id, true);
    
    // Wait for response
    if let Ok(resp) = resp_rx.recv() {
        if resp.id == req_id {
            println!("Response: {}", resp.result);
        }
    }
}
```

### Pattern 6: Broadcast/Subscribe

```rust
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

struct Broadcaster {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<String>>>>,
}

impl Broadcaster {
    fn new() -> Self {
        Broadcaster {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn subscribe(&self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }
    
    fn broadcast(&self, message: String) -> usize {
        let subs = self.subscribers.lock().unwrap();
        let mut sent = 0;
        for sub in subs.iter() {
            if sub.send(message.clone()).is_ok() {
                sent += 1;
            }
        }
        sent
    }
}

fn pub_sub_example() {
    let broadcaster = Broadcaster::new();
    
    let sub1 = broadcaster.subscribe();
    let sub2 = broadcaster.subscribe();
    
    broadcaster.broadcast("Hello!".to_string());
    
    assert_eq!(sub1.recv().unwrap(), "Hello!");
    assert_eq!(sub2.recv().unwrap(), "Hello!");
}
```

### Pattern 7: Pipeline

```rust
fn pipeline() {
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let (tx3, rx3) = mpsc::channel();
    
    // Stage 1: Generate data
    std::thread::spawn(move || {
        for i in 0..10 {
            tx1.send(i).ok();
        }
    });
    
    // Stage 2: Process (double)
    std::thread::spawn(move || {
        while let Ok(val) = rx1.recv() {
            tx2.send(val * 2).ok();
        }
    });
    
    // Stage 3: Process (add 1)
    std::thread::spawn(move || {
        while let Ok(val) = rx2.recv() {
            tx3.send(val + 1).ok();
        }
    });
    
    // Consume results
    for result in rx3 {
        println!("Final: {}", result);
    }
}
```

### Pattern 8: Bounded Channel with Backpressure

```rust
use std::sync::Mutex;
use std::collections::VecDeque;

struct BoundedChannel<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    capacity: usize,
}

impl<T: Clone> BoundedChannel<T> {
    fn new(capacity: usize) -> Self {
        BoundedChannel {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }
    
    fn send(&self, item: T) -> Result<(), String> {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= self.capacity {
            return Err("full".to_string());
        }
        q.push_back(item);
        Ok(())
    }
    
    fn recv(&self) -> Result<T, String> {
        let mut q = self.queue.lock().unwrap();
        q.pop_front().ok_or_else(|| "empty".to_string())
    }
    
    fn is_full(&self) -> bool {
        let q = self.queue.lock().unwrap();
        q.len() >= self.capacity
    }
}

fn bounded_example() {
    let ch = BoundedChannel::new(2);
    
    ch.send("msg1").ok();
    ch.send("msg2").ok();
    assert!(ch.send("msg3").is_err()); // Full!
    
    ch.recv().ok();
    assert!(ch.send("msg3").is_ok()); // Now has space
}
```

### Pattern 9: Dead Letter Queue

```rust
struct DeadLetterQueue<T: Clone + std::fmt::Debug> {
    queue: Arc<Mutex<Vec<(T, String)>>>,
}

impl<T: Clone + std::fmt::Debug> DeadLetterQueue<T> {
    fn new() -> Self {
        DeadLetterQueue {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn add_failed(&self, msg: T, reason: String) {
        let mut q = self.queue.lock().unwrap();
        q.push((msg, reason));
    }
    
    fn drain(&self) -> Vec<(T, String)> {
        let mut q = self.queue.lock().unwrap();
        q.drain(..).collect()
    }
}

fn dlq_example() {
    let dlq = DeadLetterQueue::new();
    
    dlq.add_failed("bad_message", "timeout".to_string());
    
    let drain = dlq.drain();
    assert_eq!(drain.len(), 1);
    assert_eq!(drain[0].1, "timeout");
}
```

### Pattern 10: Message Router

```rust
use std::collections::HashMap;

type MessageHandler = Box<dyn Fn(String) + Send>;

struct Router {
    routes: HashMap<String, Vec<MessageHandler>>,
}

impl Router {
    fn new() -> Self {
        Router {
            routes: HashMap::new(),
        }
    }
    
    fn register<F>(&mut self, msg_type: String, handler: F)
    where
        F: Fn(String) + Send + 'static,
    {
        self.routes
            .entry(msg_type)
            .or_insert_with(Vec::new)
            .push(Box::new(handler));
    }
    
    fn route(&self, msg_type: &str, content: String) -> Result<usize, String> {
        let handlers = self
            .routes
            .get(msg_type)
            .ok_or("no route")?;
        
        for handler in handlers {
            handler(content.clone());
        }
        
        Ok(handlers.len())
    }
}
```

---

## 3. COMMON PROBLEMS & SOLUTIONS

### Problem 1: Receiver Gets None Too Early

**Symptom:** `rx.recv()` returns `None` even though sender is still active

**Cause:** Dropped sender (clones not kept alive)

**Solution:**
```rust
// WRONG
let (tx, rx) = channel();
{
    let tx = tx.clone();
    thread::spawn(move || {
        send_later(tx);
    });
} // tx dropped here if not careful

// RIGHT
let (tx, rx) = channel();
let tx_clone = tx.clone();
thread::spawn(move || {
    send_later(tx_clone);
});
drop(tx); // Explicitly drop original
```

### Problem 2: Deadlock with MPMC

**Symptom:** Program hangs, threads not progressing

**Cause:** Receiver holds lock while trying to send, sender tries to get lock

**Solution:**
```rust
// WRONG
let mut queue = rx.lock().unwrap();
while let Some(item) = queue.pop() {
    send_to_other_channel(item); // Might panic, locks still held
}

// RIGHT
let item = {
    let mut queue = rx.lock().unwrap();
    queue.pop()
};
if let Some(item) = item {
    send_to_other_channel(item);
}
```

### Problem 3: Out of Order Reception

**Symptom:** Messages arrive in different order than sent (MPSC)

**Cause:** Multiple senders, no global ordering

**Solution:**
```rust
// Add timestamps or sequence numbers
#[derive(Clone)]
struct Message {
    sender_id: u32,
    sequence: u64,
    content: String,
}

// Reconstruct per-sender order
let mut per_sender: HashMap<u32, u64> = HashMap::new();
for msg in rx {
    let next_seq = per_sender.entry(msg.sender_id).or_insert(0);
    assert_eq!(msg.sequence, *next_seq);
    *next_seq += 1;
}
```

### Problem 4: Slow Receiver Blocks Producer

**Symptom:** Producer thread blocks waiting to send

**Cause:** Receiver can't keep up, channel is bounded

**Solution - Use Bounded Channel:**
```rust
// Use try_send() for non-blocking
match tx.try_send(item) {
    Ok(()) => {},
    Err(TrySendError::Full(_)) => {
        // Drop low-priority or queue elsewhere
    },
    Err(TrySendError::Disconnected(_)) => {},
}

// Or use timeout
match tx.send_timeout(item, Duration::from_millis(100)) {
    Ok(()) => {},
    Err(_) => println!("Slow receiver!"),
}
```

### Problem 5: Memory Leak with Channel

**Symptom:** Producer unbounded growth of messages

**Cause:** Receiver can't keep up, messages accumulate

**Solution - Use Bounded Queue:**
```rust
// Instead of infinite channel
use crossbeam::queue::SegQueue;

let queue = Arc::new(SegQueue::new());
let max_size = 1000;

if queue.len() < max_size {
    queue.push(item);
} else {
    // Drop or handle backpressure
    println!("Queue full!");
}
```

### Problem 6: Timeout on Response Never Fires

**Symptom:** Waiting for response that won't come

**Cause:** Receiver dropped, but still waiting

**Solution:**
```rust
// Check sender availability first
if tx.is_closed() {
    return Err("Sender gone".into());
}

// Use recv_timeout always
match rx.recv_timeout(Duration::from_secs(5)) {
    Ok(msg) => Ok(msg),
    Err(_) => Err("No response in 5s".into()),
}
```

---

## 4. PERFORMANCE TUNING

### Memory Optimization

**Minimize Allocations:**
```rust
// SLOW: Allocates string each send
for item in items {
    let s = format!("Item: {}", item);
    tx.send(s).ok();
}

// FAST: Pre-allocate and reuse
let mut buf = String::with_capacity(50);
for item in items {
    buf.clear();
    buf.push_str("Item: ");
    buf.push_str(&item.to_string());
    tx.send(buf.clone()).ok();
}
```

**Batch Sending:**
```rust
// SLOW: Send one at a time
for item in items {
    tx.send(item).ok();
}

// FAST: Collect and send batch
let batch: Vec<_> = items.collect();
for item in batch {
    tx.send(item).ok();
}
```

### CPU Optimization

**Reduce Lock Contention:**
```rust
// SLOW: Lock on every check
while rx.lock().unwrap().try_recv().is_ok() {
    // process
}

// FAST: Batch recv operations
let items: Vec<_> = (0..100)
    .filter_map(|_| rx.try_recv().ok())
    .collect();
for item in items {
    // process
}
```

**Busy-Wait vs Sleep:**
```rust
// Busy-wait (high CPU, low latency)
while rx.try_recv().is_ok() {
    // spin
}

// Sleep (low CPU, higher latency)
thread::sleep(Duration::from_millis(10));
let item = rx.recv().ok();

// Hybrid: Adaptive
let mut empty_count = 0;
while empty_count < 100 {
    if rx.try_recv().is_ok() {
        empty_count = 0;
    } else {
        empty_count += 1;
        thread::yield_now();
    }
}
```

---

## 5. DEBUGGING GUIDE

### Tracing Messages

```rust
fn traced_send<T: std::fmt::Debug>(
    tx: &Sender<T>,
    msg: T,
) -> Result<(), String> {
    eprintln!("SEND: {:?}", msg);
    tx.send(msg).map_err(|_| "send failed".into())
}

fn traced_recv<T: std::fmt::Debug>(
    rx: &Receiver<T>,
) -> Option<T> {
    let msg = rx.recv().ok();
    if let Some(ref m) = msg {
        eprintln!("RECV: {:?}", m);
    }
    msg
}
```

### Measuring Latency

```rust
use std::time::Instant;

let start = Instant::now();
tx.send(msg).ok();
let send_latency = start.elapsed();
eprintln!("Send latency: {:?}", send_latency);

let start = Instant::now();
let item = rx.recv();
let recv_latency = start.elapsed();
eprintln!("Recv latency: {:?}", recv_latency);
```

### Queue Depth Monitoring

```rust
// For bounded channels
fn monitor_queue<T>(ch: &BoundedChannel<T>) {
    loop {
        let len = ch.len();
        let util = ch.utilization();
        eprintln!("Queue: {}/{} ({:.1}%)", 
            len, ch.capacity, util * 100.0);
        thread::sleep(Duration::from_secs(1));
    }
}
```

---

## 6. INTEGRATION PATTERNS

### With Async/Await (Week 10)
```rust
// Convert to async
async fn async_recv<T>(rx: &Receiver<T>) -> Option<T> {
    // Use tokio channels in Week 10
    todo!()
}
```

### With Error Handling
```rust
match rx.recv() {
    Ok(msg) => handle(msg),
    Err(e) => {
        eprintln!("Channel error: {}", e);
        return Err(Box::new(e));
    }
}
```

### With Logging
```rust
use log::{info, debug, warn};

fn logged_send(tx: &Sender<String>, msg: String) -> Result<(), String> {
    info!("Sending: {}", msg);
    tx.send(msg).map_err(|e| {
        warn!("Send failed: {}", e);
        format!("{}", e)
    })
}
```

---

## 7. CODE TEMPLATES

### Template 1: Basic Producer-Consumer
```rust
use std::sync::mpsc::channel;
use std::thread;

fn producer_consumer() {
    let (tx, rx) = channel();
    
    // Producer
    thread::spawn(move || {
        for i in 1..=10 {
            println!("Producing: {}", i);
            tx.send(i).unwrap();
        }
    });
    
    // Consumer
    for item in rx {
        println!("Consuming: {}", item);
    }
}
```

### Template 2: Multiple Producers
```rust
fn multiple_producers() {
    let (tx, rx) = channel();
    
    for id in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || {
            for i in 0..5 {
                tx.send((id, i)).unwrap();
            }
        });
    }
    drop(tx);
    
    for (producer, value) in rx {
        println!("From {}: {}", producer, value);
    }
}
```

### Template 3: Request-Response Server
```rust
fn request_response_server() {
    let (req_tx, req_rx) = channel();
    let (resp_tx, resp_rx) = channel();
    
    // Server
    thread::spawn(move || {
        while let Ok((id, cmd)) = req_rx.recv() {
            let result = format!("Response to {}: {}", id, cmd);
            resp_tx.send((id, result)).ok();
        }
    });
    
    // Client
    thread::spawn(move || {
        req_tx.send((1, "get_status")).ok();
        if let Ok((_id, result)) = resp_rx.recv() {
            println!("{}", result);
        }
    });
}
```

---

## 8. GLOSSARY

- **Channel:** Primitive for sending messages between threads
- **FIFO:** First-In-First-Out ordering within single sender
- **MPSC:** Multiple-Producer, Single-Consumer channel
- **SPMC:** Single-Producer, Multiple-Consumer (broadcast)
- **MPMC:** Multiple-Producer, Multiple-Consumer (work stealing)
- **Backpressure:** Receiver too slow, producer must wait
- **Correlation ID:** Unique identifier to match request/response
- **Dead Letter Queue:** Storage for failed messages
- **Bounded Channel:** Fixed capacity with blocking/error on full
- **Graceful Shutdown:** Close channels, await remaining messages

---

## INDEX

- Async integration → Section 6
- Backpressure handling → Problem 4, Section 4
- Batching → Section 4
- Bounded channels → Pattern 8, Problem 5
- Broadcast → Pattern 6
- Channel types → Section 1
- Correlation ID → Pattern 5
- CPU optimization → Section 4
- Dead letter queue → Pattern 9
- Debugging → Section 5
- Deadlock → Problem 2
- Error handling → Problem 6, Section 6
- FIFO ordering → Problem 7 (Week 9 learning)
- Memory optimization → Section 4
- Message ordering → Problem 1
- Message router → Pattern 10
- Memory leak → Problem 5
- MPSC → Section 1, Pattern 4
- MPMC → Pattern, Problem 2
- Non-blocking → Pattern 2
- Out of order → Problem 3
- Performance → Section 4
- Pipeline → Pattern 7
- pub/sub → Pattern 6
- Request-response → Pattern 5
- SPMC → Section 1, Pattern 6
- SPSC → Section 1, Pattern 1
- Templates → Section 7
- Timeout → Pattern 3, Problem 6
- Tracing → Section 5
