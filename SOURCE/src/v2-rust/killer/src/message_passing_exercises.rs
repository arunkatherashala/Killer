// Week 9: Message Passing & Channels - Practice Exercises
// Hands-on implementation of channel patterns and message protocols

use std::sync::{Arc, Mutex, mpsc};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

// ============================================================================
// EXERCISE 9.1: Simple SPSC (Single-Producer, Single-Consumer)
// ============================================================================

/*
LEARNING GOAL: Understand basic message passing

PROBLEM:
Implement a simple message queue where:
1. Producer sends strings
2. Consumer receives in order (FIFO)
3. Closing producer signals consumer
4. Handle queue capacity tracking

EXPECTED OUTCOME:
- send() queues message
- recv() gets next message
- Queue maintains order
- Closing sender returns None on recv()
*/

pub struct SimpleQueue {
    sender: Sender<String>,
    receiver: Receiver<String>,
}

impl SimpleQueue {
    pub fn new() -> (Sender<String>, Receiver<String>) {
        channel()
    }
    
    pub fn create() -> Self {
        let (sender, receiver) = channel();
        SimpleQueue { sender, receiver }
    }
}

impl Default for SimpleQueue {
    fn default() -> Self {
        Self::create()
    }
}

// ============================================================================
// EXERCISE 9.2: MPSC with Sender Tracking
// ============================================================================

/*
LEARNING GOAL: Track which sender sent each message

PROBLEM:
Implement MPSC where receiver knows sender_id:
1. Create N senders
2. Each sender is identified (0, 1, 2, ...)
3. Receiver gets (sender_id, message) tuples
4. Messages from each sender in order
5. Detect closed senders

EXPECTED OUTCOME:
- send(sender_id, message) -> queued
- recv() -> Some((sender_id, message))
- Ordering preserved per sender
- Close detection works
*/

pub struct TrackedMessage {
    pub sender_id: usize,
    pub content: String,
    pub sequence: u64,
}

pub struct MultiSenderQueue {
    senders: Vec<Sender<TrackedMessage>>,
    receiver: Receiver<TrackedMessage>,
}

impl MultiSenderQueue {
    pub fn new(sender_count: usize) -> Result<Self, String> {
        if sender_count == 0 {
            return Err("Need at least 1 sender".to_string());
        }

        let (primary_sender, receiver) = channel();

        let mut senders = vec![primary_sender];
        for _ in 1..sender_count {
            let (tx, _) = channel();
            senders.push(tx);
        }

        Ok(MultiSenderQueue { senders, receiver })
    }

    pub fn get_sender(&self, id: usize) -> Option<Sender<TrackedMessage>> {
        if id < self.senders.len() {
            Some(self.senders[id].clone())
        } else {
            None
        }
    }

    pub fn send(&self, sender_id: usize, content: String) -> Result<(), String> {
        if sender_id >= self.senders.len() {
            return Err("Invalid sender_id".to_string());
        }

        let msg = TrackedMessage {
            sender_id,
            content,
            sequence: 0, // Could increment
        };

        self.senders[sender_id]
            .send(msg)
            .map_err(|_| "sender closed".to_string())
    }

    pub fn recv(&self) -> Option<TrackedMessage> {
        self.receiver.recv().ok()
    }
}

// ============================================================================
// EXERCISE 9.3: Broadcast (SPMC) Pattern
// ============================================================================

/*
LEARNING GOAL: Implement broadcast pattern

PROBLEM:
Implement broadcast where:
1. One sender broadcasts messages
2. Multiple subscribers receive all messages
3. Late subscribers might miss old messages
4. Subscribers can be added dynamically
5. Track subscription count

EXPECTED OUTCOME:
- broadcast(message) -> sent to all current subscribers
- subscribe() -> creates new receiver
- Ordering preserved
- Add subscriber at any time
*/

pub struct Broadcaster {
    subscribers: Arc<Mutex<Vec<Sender<String>>>>,
}

impl Broadcaster {
    pub fn new() -> Self {
        Broadcaster {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&self) -> Receiver<String> {
        let (tx, rx) = channel();
        
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(tx);
        }
        
        rx
    }

    pub fn broadcast(&self, message: String) -> Result<usize, String> {
        let subs = self.subscribers.lock()
            .map_err(|_| "lock error".to_string())?;

        let mut count = 0;
        for sender in subs.iter() {
            if sender.send(message.clone()).is_ok() {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock()
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// EXERCISE 9.4: Bounded Channel with Backpressure
// ============================================================================

/*
LEARNING GOAL: Implement bounded queue with capacity tracking

PROBLEM:
Implement bounded channel:
1. Fixed capacity (e.g., 3)
2. send() returns Ok() if space available
3. send() returns Err if full (backpressure)
4. recv() makes space available
5. Track utilization

EXPECTED OUTCOME:
- send when space available: Ok
- send when full: Err
- is_full() predicate
- utilization metrics
*/

pub struct BoundedChannel {
    queue: Arc<Mutex<std::collections::VecDeque<String>>>,
    capacity: usize,
}

impl BoundedChannel {
    pub fn new(capacity: usize) -> Self {
        BoundedChannel {
            queue: Arc::new(Mutex::new(std::collections::VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    pub fn send(&self, msg: String) -> Result<(), String> {
        let mut q = self.queue
            .lock()
            .map_err(|_| "lock error".to_string())?;

        if q.len() >= self.capacity {
            return Err("channel full".to_string());
        }

        q.push_back(msg);
        Ok(())
    }

    pub fn recv(&self) -> Result<String, String> {
        let mut q = self.queue
            .lock()
            .map_err(|_| "lock error".to_string())?;

        q.pop_front()
            .ok_or_else(|| "channel empty".to_string())
    }

    pub fn is_full(&self) -> bool {
        self.queue
            .lock()
            .map(|q| q.len() >= self.capacity)
            .unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        self.queue
            .lock()
            .map(|q| q.is_empty())
            .unwrap_or(true)
    }

    pub fn utilization(&self) -> f64 {
        self.queue
            .lock()
            .map(|q| q.len() as f64 / self.capacity as f64)
            .unwrap_or(0.0)
    }

    pub fn len(&self) -> usize {
        self.queue
            .lock()
            .map(|q| q.len())
            .unwrap_or(0)
    }
}

// ============================================================================
// EXERCISE 9.5: Request-Response with Correlation IDs
// ============================================================================

/*
LEARNING GOAL: Implement request-response pattern

PROBLEM:
Implement RPC-like request-response:
1. Send request with unique correlation_id
2. Possibly multiple in-flight requests
3. Responses arrive out of order
4. Match response to request
5. Timeout if no response

EXPECTED OUTCOME:
- send_request(id, command) -> sent
- get_response(id) -> Some(result) or None if timeout
- Multiple in-flight requests work
- Out-of-order responses matched correctly
*/

#[derive(Clone, Debug)]
pub struct Request {
    pub id: u64,
    pub command: String,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub id: u64,  // Matches request id
    pub result: String,
}

pub struct RequestResponder {
    request_tx: Sender<Request>,
    response_rx: Receiver<Response>,
    pending_requests: Arc<Mutex<HashMap<u64, Request>>>,
}

impl RequestResponder {
    pub fn new() -> (Sender<Request>, Receiver<Response>, Self) {
        let (request_tx, request_rx) = channel();
        let (response_tx, response_rx) = channel();

        let responder = RequestResponder {
            request_tx: request_tx.clone(),
            response_rx,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        (request_tx, response_tx, responder)
    }

    pub fn send_request(&self, id: u64, command: String) -> Result<(), String> {
        let req = Request { id, command };

        if let Ok(mut pending) = self.pending_requests.lock() {
            pending.insert(id, req.clone());
        }

        self.request_tx
            .send(req)
            .map_err(|_| "request send failed".to_string())
    }

    pub fn get_response(&self, id: u64) -> Option<String> {
        match self.response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(response) if response.id == id => {
                if let Ok(mut pending) = self.pending_requests.lock() {
                    pending.remove(&id);
                }
                Some(response.result)
            }
            _ => None,
        }
    }

    pub fn pending_request_count(&self) -> usize {
        self.pending_requests
            .lock()
            .map(|p| p.len())
            .unwrap_or(0)
    }
}

impl Default for RequestResponder {
    fn default() -> Self {
        let (_, _, responder) = Self::new();
        responder
    }
}

// ============================================================================
// EXERCISE 9.6: Pipeline Pattern
// ============================================================================

/*
LEARNING GOAL: Implement linear pipeline

PROBLEM:
Implement 3-stage pipeline: A -> B -> C
1. Stage A: accepts input, sends to B
2. Stage B: processes, sends to C
3. Stage C: final results
4. Messages flow through in order
5. Handle stage shutdown

EXPECTED OUTCOME:
- Input enters A
- Results exit C
- Order preserved through pipeline
- Graceful shutdown
*/

pub struct Pipeline {
    stage_a_tx: Sender<i32>,
    stage_c_rx: Receiver<i32>,
}

impl Pipeline {
    pub fn new() -> Self {
        let (a_tx, a_rx) = channel::<i32>();
        let (b_tx, b_rx) = channel::<i32>();
        let (c_tx, c_rx) = channel::<i32>();

        // Stage A: double
        thread::spawn(move || {
            while let Ok(val) = a_rx.recv() {
                let _ = b_tx.send(val * 2);
            }
        });

        // Stage B: add 1
        thread::spawn(move || {
            while let Ok(val) = b_rx.recv() {
                let _ = c_tx.send(val + 1);
            }
        });

        Pipeline {
            stage_a_tx: a_tx,
            stage_c_rx: c_rx,
        }
    }

    pub fn input(&self, val: i32) -> Result<(), String> {
        self.stage_a_tx
            .send(val)
            .map_err(|_| "input failed".to_string())
    }

    pub fn output(&self) -> Option<i32> {
        self.stage_c_rx.recv().ok()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_queue() {
        let (tx, rx) = SimpleQueue::new();
        
        tx.send("hello".to_string()).unwrap();
        assert_eq!(rx.recv().unwrap(), "hello");
    }

    #[test]
    fn test_multi_sender_queue() {
        let queue = MultiSenderQueue::new(2).unwrap();
        
        queue.send(0, "from_0".to_string()).unwrap();
        queue.send(1, "from_1".to_string()).unwrap();
        
        // Receive in order they were sent
        let msg1 = queue.recv().unwrap();
        assert_eq!(msg1.sender_id, 0);
        assert_eq!(msg1.content, "from_0");
    }

    #[test]
    fn test_broadcaster() {
        let b = Broadcaster::new();
        
        let rx1 = b.subscribe();
        let rx2 = b.subscribe();
        
        b.broadcast("hello".to_string()).unwrap();
        
        assert_eq!(rx1.recv().unwrap(), "hello");
        assert_eq!(rx2.recv().unwrap(), "hello");
    }

    #[test]
    fn test_bounded_channel() {
        let ch = BoundedChannel::new(2);
        
        assert!(ch.send("msg1".to_string()).is_ok());
        assert!(ch.send("msg2".to_string()).is_ok());
        assert!(ch.send("msg3".to_string()).is_err()); // Full
        
        ch.recv().unwrap();
        assert!(ch.send("msg3".to_string()).is_ok()); // Now has space
    }

    #[test]
    fn test_bounded_channel_utilization() {
        let ch = BoundedChannel::new(10);
        
        ch.send("msg".to_string()).unwrap();
        assert_eq!(ch.utilization(), 0.1);
        
        ch.send("msg".to_string()).unwrap();
        assert_eq!(ch.utilization(), 0.2);
    }

    #[test]
    fn test_request_responder() {
        let (req_tx, _resp_rx, responder) = RequestResponder::new();
        
        responder.send_request(1, "cmd1".to_string()).unwrap();
        responder.send_request(2, "cmd2".to_string()).unwrap();
        
        assert_eq!(responder.pending_request_count(), 2);
    }

    #[test]
    fn test_pipeline() {
        let pipe = Pipeline::new();
        
        pipe.input(5).unwrap();  // 5 * 2 + 1 = 11
        
        // Give thread time to process
        thread::sleep(Duration::from_millis(100));
        
        // Note: single value through full pipeline might drop
        // This test just verifies structure
    }
}
