// Week 10: Actor Model Exercises
// Building isolated, fault-tolerant systems with message passing and supervision

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// EXERCISE 1: Basic Actor Trait
// ============================================================================

/// Message types that actors can receive
pub trait Message: Send {
    fn describe(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct StringMessage(pub String);
impl Message for StringMessage {
    fn describe(&self) -> String {
        format!("StringMessage: {}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct IntMessage(pub i32);
impl Message for IntMessage {
    fn describe(&self) -> String {
        format!("IntMessage: {}", self.0)
    }
}

/// Actor trait - core behavior
pub trait Actor: Send {
    fn receive(&mut self, msg: Box<dyn std::any::Any>);
    fn name(&self) -> &str;
    fn shutdown(&mut self);
}

/// ActorHandle - reference to send messages to an actor
pub struct ActorHandle {
    name: String,
    mailbox: Arc<Mutex<Vec<Box<dyn std::any::Any>>>>,
}

impl ActorHandle {
    pub fn new(name: String) -> Self {
        ActorHandle {
            name,
            mailbox: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn send<M: 'static + std::any::Any>(&self, msg: M) {
        self.mailbox.lock().unwrap().push(Box::new(msg));
    }

    pub fn receive_next(&self) -> Option<Box<dyn std::any::Any>> {
        self.mailbox.lock().unwrap().pop()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// EXERCISE 2: Echo Actor
// ============================================================================

pub struct EchoActor {
    name: String,
    messages_received: usize,
}

impl EchoActor {
    pub fn new(name: String) -> Self {
        EchoActor {
            name,
            messages_received: 0,
        }
    }

    pub fn messages_received(&self) -> usize {
        self.messages_received
    }
}

impl Actor for EchoActor {
    fn receive(&mut self, _msg: Box<dyn std::any::Any>) {
        self.messages_received += 1;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn shutdown(&mut self) {
        println!("{} received {} messages before shutdown", self.name, self.messages_received);
    }
}

// ============================================================================
// EXERCISE 3: Counter Actor with State
// ============================================================================

#[derive(Clone, Debug)]
pub enum CounterMessage {
    Increment(i32),
    Decrement(i32),
    GetValue,
    Reset,
}

pub struct CounterActor {
    name: String,
    value: i32,
    history: Vec<i32>,
}

impl CounterActor {
    pub fn new(name: String) -> Self {
        CounterActor {
            name,
            value: 0,
            history: vec![0],
        }
    }

    pub fn value(&self) -> i32 {
        self.value
    }

    pub fn history(&self) -> &[i32] {
        &self.history
    }

    pub fn handle_counter_message(&mut self, msg: CounterMessage) {
        match msg {
            CounterMessage::Increment(n) => {
                self.value += n;
                self.history.push(self.value);
            }
            CounterMessage::Decrement(n) => {
                self.value -= n;
                self.history.push(self.value);
            }
            CounterMessage::Reset => {
                self.value = 0;
                self.history.push(0);
            }
            CounterMessage::GetValue => {
                // Would normally send response back
            }
        }
    }
}

// ============================================================================
// EXERCISE 4: Supervision Strategies
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorDecision {
    Resume,   // Ignore error, continue
    Restart,  // Create new actor with same ID
    Stop,     // Terminate actor
    Escalate, // Pass to parent supervisor
}

pub struct SupervisorStrategy {
    name: String,
    decisions: HashMap<String, SupervisorDecision>,
)

impl SupervisorStrategy {
    pub fn new(name: String) -> Self {
        let mut decisions = HashMap::new();
        // Default: restart on any error
        decisions.insert("default".to_string(), SupervisorDecision::Restart);

        SupervisorStrategy { name, decisions }
    }

    /// One-for-one: only restart failed child
    pub fn one_for_one() -> Self {
        let mut strategy = SupervisorStrategy::new("one-for-one".to_string());
        strategy.decisions.insert("strategy".to_string(), SupervisorDecision::Restart);
        strategy
    }

    /// All-for-one: restart all children if one fails
    pub fn all_for_one() -> Self {
        SupervisorStrategy::new("all-for-one".to_string())
    }

    pub fn decide_on_error(&self, error_type: &str) -> SupervisorDecision {
        self.decisions
            .get(error_type)
            .copied()
            .unwrap_or(SupervisorDecision::Restart)
    }

    pub fn set_decision(&mut self, error_type: String, decision: SupervisorDecision) {
        self.decisions.insert(error_type, decision);
    }
}

// ============================================================================
// EXERCISE 5: Actor Pool (Multiple Instances)
// ============================================================================

pub struct ActorPool<A: Actor> {
    actors: Vec<Arc<Mutex<A>>>,
    current: usize,
}

impl<A: Actor> ActorPool<A> {
    pub fn new(size: usize, factory: impl Fn(usize) -> A) -> Self {
        let actors = (0..size)
            .map(|i| Arc::new(Mutex::new(factory(i))))
            .collect();

        ActorPool { actors, current: 0 }
    }

    /// Get next actor in round-robin fashion
    pub fn next(&mut self) -> Arc<Mutex<A>> {
        let actor = self.actors[self.current].clone();
        self.current = (self.current + 1) % self.actors.len();
        actor
    }

    pub fn size(&self) -> usize {
        self.actors.len()
    }

    pub fn get(&self, index: usize) -> Option<Arc<Mutex<A>>> {
        self.actors.get(index).cloned()
    }
}

// ============================================================================
// EXERCISE 6: Actor Hierarchy (Parent-Child Relationships)
// ============================================================================

pub struct ActorPath {
    segments: Vec<String>,
}

impl ActorPath {
    pub fn new(segments: Vec<String>) -> Self {
        ActorPath { segments }
    }

    pub fn root() -> Self {
        ActorPath {
            segments: vec!["/".to_string()],
        }
    }

    pub fn child(&self, name: String) -> Self {
        let mut segments = self.segments.clone();
        segments.push(name);
        ActorPath { segments }
    }

    pub fn path_string(&self) -> String {
        if self.segments.len() == 1 {
            "/".to_string()
        } else {
            self.segments.join("/")
        }
    }

    pub fn parent(&self) -> Option<ActorPath> {
        if self.segments.len() <= 1 {
            None
        } else {
            let mut segs = self.segments.clone();
            segs.pop();
            Some(ActorPath { segments: segs })
        }
    }
}

pub struct ActorContext {
    path: ActorPath,
    children: HashMap<String, ActorHandle>,
}

impl ActorContext {
    pub fn new(path: ActorPath) -> Self {
        ActorContext {
            path,
            children: HashMap::new(),
        }
    }

    pub fn spawn_child(&mut self, name: String) -> ActorHandle {
        let child_path = self.path.child(name.clone());
        let handle = ActorHandle::new(child_path.path_string());
        self.children.insert(name, handle.clone());
        handle
    }

    pub fn get_child(&self, name: &str) -> Option<ActorHandle> {
        self.children.get(name).cloned()
    }

    pub fn path(&self) -> &ActorPath {
        &self.path
    }

    pub fn children(&self) -> Vec<String> {
        self.children.keys().cloned().collect()
    }
}

// ============================================================================
// EXERCISE 7: Dead Letter Queue (Failed Messages)
// ============================================================================

#[derive(Clone, Debug)]
pub struct DeadLetter {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub attempts: u32,
}

pub struct DeadLetterQueue {
    letters: Arc<Mutex<Vec<DeadLetter>>>,
    max_size: usize,
}

impl DeadLetterQueue {
    pub fn new(max_size: usize) -> Self {
        DeadLetterQueue {
            letters: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    pub fn add(&self, letter: DeadLetter) {
        let mut letters = self.letters.lock().unwrap();
        if letters.len() < self.max_size {
            letters.push(letter);
        }
    }

    pub fn get_all(&self) -> Vec<DeadLetter> {
        self.letters.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.letters.lock().unwrap().len()
    }

    pub fn clear(&self) {
        self.letters.lock().unwrap().clear();
    }
}

// ============================================================================
// EXERCISE 8: Actor Broadcast (One-to-Many)
// ============================================================================

pub struct ActorBroadcaster {
    subscribers: Arc<Mutex<Vec<ActorHandle>>>,
    name: String,
}

impl ActorBroadcaster {
    pub fn new(name: String) -> Self {
        ActorBroadcaster {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            name,
        }
    }

    pub fn subscribe(&self, actor: ActorHandle) {
        self.subscribers.lock().unwrap().push(actor);
    }

    pub fn publish<M: 'static + std::any::Any + Clone>(&self, msg: M) {
        for subscriber in self.subscribers.lock().unwrap().iter() {
            subscriber.send(msg.clone());
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().unwrap().len()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// EXERCISE 9: Restart Counter & Failure Handling
// ============================================================================

pub struct ActorRestartCounter {
    restart_count: Arc<Mutex<u32>>,
    last_error: Arc<Mutex<Option<String>>>,
    max_restarts: u32,
    window: Duration,
}

impl ActorRestartCounter {
    pub fn new(max_restarts: u32, window: Duration) -> Self {
        ActorRestartCounter {
            restart_count: Arc::new(Mutex::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            max_restarts,
            window,
        }
    }

    pub fn record_restart(&self, error: String) {
        let mut count = self.restart_count.lock().unwrap();
        *count += 1;
        *self.last_error.lock().unwrap() = Some(error);

        if *count > self.max_restarts {
            // Too many restarts in window
            // Would normally escalate to supervisor
        }
    }

    pub fn reset(&self) {
        *self.restart_count.lock().unwrap() = 0;
        *self.last_error.lock().unwrap() = None;
    }

    pub fn restart_count(&self) -> u32 {
        *self.restart_count.lock().unwrap()
    }

    pub fn should_restart(&self) -> bool {
        *self.restart_count.lock().unwrap() <= self.max_restarts
    }
}

// ============================================================================
// EXERCISE 10: Distributed Actor (Cluster Awareness)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RemoteActorRef {
    node: String,
    path: String,
}

impl RemoteActorRef {
    pub fn new(node: String, path: String) -> Self {
        RemoteActorRef { node, path }
    }

    pub fn node(&self) -> &str {
        &self.node
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn is_local(&self) -> bool {
        self.node == "localhost"
    }
}

pub struct ClusterAwareActorRef {
    local_ref: ActorHandle,
    remote_refs: HashMap<String, RemoteActorRef>,
    current_node: String,
}

impl ClusterAwareActorRef {
    pub fn new(local_ref: ActorHandle, node: String) -> Self {
        ClusterAwareActorRef {
            local_ref,
            remote_refs: HashMap::new(),
            current_node: node,
        }
    }

    pub fn add_remote_replica(&mut self, node: String, remote_ref: RemoteActorRef) {
        self.remote_refs.insert(node, remote_ref);
    }

    pub fn get_replica(&self, node: &str) -> Option<&RemoteActorRef> {
        self.remote_refs.get(node)
    }

    pub fn replicas(&self) -> Vec<&RemoteActorRef> {
        self.remote_refs.values().collect()
    }

    pub fn local_ref(&self) -> &ActorHandle {
        &self.local_ref
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_actor() {
        let mut echo = EchoActor::new("echo".to_string());
        assert_eq!(echo.messages_received(), 0);
        assert_eq!(echo.name(), "echo");
    }

    #[test]
    fn test_counter_actor() {
        let mut counter = CounterActor::new("counter".to_string());
        assert_eq!(counter.value(), 0);

        counter.handle_counter_message(CounterMessage::Increment(5));
        assert_eq!(counter.value(), 5);

        counter.handle_counter_message(CounterMessage::Increment(3));
        assert_eq!(counter.value(), 8);

        counter.handle_counter_message(CounterMessage::Decrement(2));
        assert_eq!(counter.value(), 6);

        counter.handle_counter_message(CounterMessage::Reset);
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn test_counter_history() {
        let mut counter = CounterActor::new("counter".to_string());
        counter.handle_counter_message(CounterMessage::Increment(1));
        counter.handle_counter_message(CounterMessage::Increment(2));
        counter.handle_counter_message(CounterMessage::Decrement(1));

        let history = counter.history();
        assert_eq!(history.len(), 4); // initial 0, +1, +2, -1
        assert_eq!(history[0], 0);
        assert_eq!(history[1], 1);
        assert_eq!(history[2], 3);
        assert_eq!(history[3], 2);
    }

    #[test]
    fn test_supervisor_strategy() {
        let strategy = SupervisorStrategy::one_for_one();
        let decision = strategy.decide_on_error("default");
        assert_eq!(decision, SupervisorDecision::Restart);
    }

    #[test]
    fn test_actor_pool() {
        let mut pool = ActorPool::new(3, |i| EchoActor::new(format!("actor-{}", i)));
        assert_eq!(pool.size(), 3);

        let actor1 = pool.next();
        let actor2 = pool.next();
        let actor3 = pool.next();
        let actor1_again = pool.next();

        // Round-robin behavior
        assert!(Arc::ptr_eq(&actor1, &actor1_again));
    }

    #[test]
    fn test_actor_path() {
        let root = ActorPath::root();
        assert_eq!(root.path_string(), "/");

        let child = root.child("supervisor".to_string());
        assert_eq!(child.path_string(), "/supervisor");

        let grandchild = child.child("worker".to_string());
        assert_eq!(grandchild.path_string(), "/supervisor/worker");

        let parent = grandchild.parent();
        assert!(parent.is_some());
        assert_eq!(parent.unwrap().path_string(), "/supervisor");
    }

    #[test]
    fn test_actor_context() {
        let path = ActorPath::root();
        let mut ctx = ActorContext::new(path);

        let child = ctx.spawn_child("worker1".to_string());
        assert_eq!(child.name(), "/worker1");

        assert!(ctx.get_child("worker1").is_some());
        assert!(ctx.get_child("worker2").is_none());

        assert_eq!(ctx.children().len(), 1);
    }

    #[test]
    fn test_dead_letter_queue() {
        let dlq = DeadLetterQueue::new(10);
        assert_eq!(dlq.count(), 0);

        let letter = DeadLetter {
            from: "sender".to_string(),
            to: "receiver".to_string(),
            reason: "actor not found".to_string(),
            attempts: 1,
        };

        dlq.add(letter);
        assert_eq!(dlq.count(), 1);

        let all = dlq.get_all();
        assert_eq!(all[0].from, "sender");
    }

    #[test]
    fn test_actor_broadcaster() {
        let broadcaster = ActorBroadcaster::new("event-bus".to_string());
        assert_eq!(broadcaster.subscriber_count(), 0);

        let handle1 = ActorHandle::new("actor1".to_string());
        let handle2 = ActorHandle::new("actor2".to_string());

        broadcaster.subscribe(handle1);
        broadcaster.subscribe(handle2);

        assert_eq!(broadcaster.subscriber_count(), 2);
    }

    #[test]
    fn test_restart_counter() {
        let counter = ActorRestartCounter::new(3, Duration::from_secs(60));
        assert!(counter.should_restart());
        assert_eq!(counter.restart_count(), 0);

        counter.record_restart("error1".to_string());
        assert_eq!(counter.restart_count(), 1);
        assert!(counter.should_restart());

        counter.reset();
        assert_eq!(counter.restart_count(), 0);
    }

    #[test]
    fn test_remote_actor_ref() {
        let local_ref = RemoteActorRef::new("localhost".to_string(), "/actor1".to_string());
        assert!(local_ref.is_local());

        let remote_ref = RemoteActorRef::new("node2".to_string(), "/actor1".to_string());
        assert!(!remote_ref.is_local());
    }

    #[test]
    fn test_cluster_aware_actor() {
        let handle = ActorHandle::new("local".to_string());
        let mut aware = ClusterAwareActorRef::new(handle, "node1".to_string());

        let remote = RemoteActorRef::new("node2".to_string(), "/actor1".to_string());
        aware.add_remote_replica("node2".to_string(), remote);

        assert!(aware.get_replica("node2").is_some());
        assert_eq!(aware.replicas().len(), 1);
    }
}
