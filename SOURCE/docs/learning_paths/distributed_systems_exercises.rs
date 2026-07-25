// Week 11: Distributed Systems Exercises
// RPC, service discovery, clustering, consensus, replication

use std::sync::{Arc, Mutex};
use std::collections::{HashMap, BTreeMap};

// ============================================================================
// EXERCISE 1: Serialization & Message Format
// ============================================================================

/// Simple serialization format for network messages
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SerializedValue {
    Integer(i64),
    String(String),
    Boolean(bool),
    Bytes(Vec<u8>),
    Array(Vec<SerializedValue>),
    Map(HashMap<String, SerializedValue>),
}

impl SerializedValue {
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            SerializedValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            SerializedValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SerializedValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        format!("{:?}", self).into_bytes()
    }

    pub fn deserialize(data: &[u8]) -> Result<SerializedValue, String> {
        String::from_utf8(data.to_vec())
            .map_err(|_| "Invalid UTF-8".to_string())
            .and_then(|s| {
                if let Ok(i) = s.parse::<i64>() {
                    Ok(SerializedValue::Integer(i))
                } else {
                    Ok(SerializedValue::String(s))
                }
            })
    }
}

// ============================================================================
// EXERCISE 2: RPC (Remote Procedure Call)
// ============================================================================

#[derive(Clone, Debug)]
pub struct RpcRequest {
    pub call_id: String,
    pub method: String,
    pub args: Vec<SerializedValue>,
    pub reply_to: String,
}

#[derive(Clone, Debug)]
pub struct RpcResponse {
    pub call_id: String,
    pub result: Result<SerializedValue, String>,
}

pub struct RpcRegistry {
    methods: Arc<Mutex<HashMap<String, Box<dyn Fn(Vec<SerializedValue>) -> SerializedValue>>>>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        RpcRegistry {
            methods: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_method<F>(&self, name: String, handler: F)
    where
        F: Fn(Vec<SerializedValue>) -> SerializedValue + 'static,
    {
        // Note: In real code, we'd handle this without Box<dyn Fn>
        // For this exercise, we use a simpler approach
    }

    pub fn call(&self, request: RpcRequest) -> RpcResponse {
        let methods = self.methods.lock().unwrap();

        let result = if let Some(_handler) = methods.get(&request.method) {
            // In practice, call the handler
            Ok(SerializedValue::String("success".to_string()))
        } else {
            Err(format!("Method {} not found", request.method))
        };

        RpcResponse {
            call_id: request.call_id,
            result,
        }
    }
}

pub struct RpcClient {
    service_address: String,
    pending_calls: Arc<Mutex<HashMap<String, Box<dyn std::any::Any>>>>,
    next_call_id: Arc<Mutex<u64>>,
}

impl RpcClient {
    pub fn new(service_address: String) -> Self {
        RpcClient {
            service_address,
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            next_call_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn call(&self, method: String, args: Vec<SerializedValue>) -> String {
        let mut id_lock = self.next_call_id.lock().unwrap();
        let call_id = format!("call-{}", *id_lock);
        *id_lock += 1;

        let _request = RpcRequest {
            call_id: call_id.clone(),
            method,
            args,
            reply_to: "client".to_string(),
        };

        // Would send to network here
        // and wait for response

        call_id
    }

    pub fn await_response(&self, call_id: &str, timeout_ms: u64) -> Option<RpcResponse> {
        // Simulate waiting for response
        // In real code: blocking queue or future
        None
    }
}

// ============================================================================
// EXERCISE 3: Service Discovery
// ============================================================================

#[derive(Clone, Debug)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub metadata: HashMap<String, String>,
}

impl ServiceInstance {
    pub fn new(id: String, name: String, address: String, port: u16) -> Self {
        ServiceInstance {
            id,
            name,
            address,
            port,
            metadata: HashMap::new(),
        }
    }

    pub fn url(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

pub struct ServiceRegistry {
    services: Arc<Mutex<HashMap<String, Vec<ServiceInstance>>>>,
    health_checks: Arc<Mutex<HashMap<String, bool>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: Arc::new(Mutex::new(HashMap::new())),
            health_checks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(&self, instance: ServiceInstance) {
        let mut services = self.services.lock().unwrap();
        services
            .entry(instance.name.clone())
            .or_insert_with(Vec::new)
            .push(instance.clone());

        self.health_checks
            .lock()
            .unwrap()
            .insert(instance.id, true);
    }

    pub fn deregister(&self, instance_id: &str) {
        self.health_checks.lock().unwrap().remove(instance_id);
    }

    pub fn discover(&self, service_name: &str) -> Vec<ServiceInstance> {
        self.services
            .lock()
            .unwrap()
            .get(service_name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn discover_healthy(&self, service_name: &str) -> Vec<ServiceInstance> {
        let health = self.health_checks.lock().unwrap();
        self.services
            .lock()
            .unwrap()
            .get(service_name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|inst| health.get(&inst.id).copied().unwrap_or(false))
            .collect()
    }

    pub fn mark_healthy(&self, instance_id: &str, healthy: bool) {
        self.health_checks.lock().unwrap().insert(instance_id.to_string(), healthy);
    }
}

// ============================================================================
// EXERCISE 4: Replication & Consistency
// ============================================================================

/// Version vector for tracking causality
#[derive(Clone, Debug)]
pub struct VectorClock {
    clock: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new(nodes: Vec<String>) -> Self {
        let mut clock = HashMap::new();
        for node in nodes {
            clock.insert(node, 0);
        }
        VectorClock { clock }
    }

    pub fn increment(&mut self, node: &str) {
        *self.clock.entry(node.to_string()).or_insert(0) += 1;
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut all_less_or_equal = true;
        let mut some_less = false;

        for (node, time) in &self.clock {
            let other_time = other.clock.get(node).copied().unwrap_or(0);
            if time > &other_time {
                all_less_or_equal = false;
                break;
            }
            if time < &other_time {
                some_less = true;
            }
        }

        all_less_or_equal && some_less
    }

    pub fn merge(&self, other: &VectorClock) -> VectorClock {
        let mut merged = self.clone();
        for (node, time) in &other.clock {
            let current = merged.clock.entry(node.clone()).or_insert(0);
            *current = (*current).max(*time);
        }
        merged
    }
}

pub struct ReplicatedValue<T: Clone> {
    value: T,
    version: VectorClock,
    replicas: HashMap<String, VectorClock>,
}

impl<T: Clone> ReplicatedValue<T> {
    pub fn new(value: T, nodes: Vec<String>) -> Self {
        ReplicatedValue {
            value,
            version: VectorClock::new(nodes.clone()),
            replicas: {
                let mut m = HashMap::new();
                for node in nodes {
                    m.insert(node, VectorClock::new(vec![]));
                }
                m
            },
        }
    }

    pub fn update(&mut self, new_value: T, node: &str) {
        self.version.increment(node);
        self.value = new_value;
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn version(&self) -> &VectorClock {
        &self.version
    }
}

// ============================================================================
// EXERCISE 5: Consensus (Leader Election)
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FollowerState {
    Follower,
    Candidate,
    Leader,
}

pub struct LeaderElection {
    node_id: String,
    state: FollowerState,
    current_leader: Option<String>,
    votes_received: u32,
    election_term: u64,
    nodes: Vec<String>,
}

impl LeaderElection {
    pub fn new(node_id: String, nodes: Vec<String>) -> Self {
        LeaderElection {
            node_id,
            state: FollowerState::Follower,
            current_leader: None,
            votes_received: 0,
            election_term: 0,
            nodes,
        }
    }

    pub fn become_candidate(&mut self) {
        self.state = FollowerState::Candidate;
        self.election_term += 1;
        self.votes_received = 1; // vote for self
    }

    pub fn receive_vote(&mut self) {
        if self.state == FollowerState::Candidate {
            self.votes_received += 1;
            let quorum = (self.nodes.len() / 2) + 1;
            if self.votes_received >= quorum as u32 {
                self.become_leader();
            }
        }
    }

    pub fn become_leader(&mut self) {
        self.state = FollowerState::Leader;
        self.current_leader = Some(self.node_id.clone());
    }

    pub fn receive_heartbeat(&mut self, leader_id: String, leader_term: u64) {
        if leader_term >= self.election_term {
            self.state = FollowerState::Follower;
            self.election_term = leader_term;
            self.current_leader = Some(leader_id);
        }
    }

    pub fn state(&self) -> FollowerState {
        self.state.clone()
    }

    pub fn current_leader(&self) -> Option<&str> {
        self.current_leader.as_deref()
    }

    pub fn is_leader(&self) -> bool {
        self.state == FollowerState::Leader
    }
}

// ============================================================================
// EXERCISE 6: Sharding (Data Partitioning)
// ============================================================================

pub struct ShardId(pub u32);

pub trait ShardKey {
    fn shard_id(&self, num_shards: u32) -> ShardId;
}

impl ShardKey for String {
    fn shard_id(&self, num_shards: u32) -> ShardId {
        let hash = self.len() as u32; // simple hash
        ShardId(hash % num_shards)
    }
}

impl ShardKey for u64 {
    fn shard_id(&self, num_shards: u32) -> ShardId {
        ShardId((*self % num_shards as u64) as u32)
    }
}

pub struct ShardMap<K: ShardKey + Clone, V: Clone> {
    shards: Vec<Arc<Mutex<BTreeMap<K, V>>>>,
    num_shards: u32,
}

impl<K: ShardKey + Clone, V: Clone> ShardMap<K, V> {
    pub fn new(num_shards: u32) -> Self {
        let mut shards = Vec::new();
        for _ in 0..num_shards {
            shards.push(Arc::new(Mutex::new(BTreeMap::new())));
        }

        ShardMap { shards, num_shards }
    }

    pub fn insert(&self, key: K, value: V) {
        let shard_id = key.shard_id(self.num_shards).0 as usize;
        self.shards[shard_id].lock().unwrap().insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let shard_id = key.shard_id(self.num_shards).0 as usize;
        self.shards[shard_id].lock().unwrap().get(key).cloned()
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let shard_id = key.shard_id(self.num_shards).0 as usize;
        self.shards[shard_id].lock().unwrap().remove(key)
    }

    pub fn shard_count(&self) -> u32 {
        self.num_shards
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let val = SerializedValue::Integer(42);
        assert_eq!(val.as_integer(), Some(42));
        assert_eq!(val.as_string(), None);

        let str_val = SerializedValue::String("hello".to_string());
        assert_eq!(str_val.as_string(), Some("hello"));
    }

    #[test]
    fn test_rpc_request() {
        let req = RpcRequest {
            call_id: "call-1".to_string(),
            method: "get_user".to_string(),
            args: vec![SerializedValue::Integer(123)],
            reply_to: "client".to_string(),
        };
        assert_eq!(req.method, "get_user");
    }

    #[test]
    fn test_service_instance() {
        let instance = ServiceInstance::new(
            "service-1".to_string(),
            "user-service".to_string(),
            "192.168.1.10".to_string(),
            8080,
        );
        assert_eq!(instance.url(), "192.168.1.10:8080");
    }

    #[test]
    fn test_service_registry() {
        let registry = ServiceRegistry::new();

        let instance1 = ServiceInstance::new(
            "inst-1".to_string(),
            "user-service".to_string(),
            "192.168.1.10".to_string(),
            8080,
        );

        registry.register(instance1);
        let discovered = registry.discover("user-service");
        assert_eq!(discovered.len(), 1);
    }

    #[test]
    fn test_service_discovery_health() {
        let registry = ServiceRegistry::new();
        let instance = ServiceInstance::new(
            "inst-1".to_string(),
            "data-service".to_string(),
            "192.168.1.20".to_string(),
            5000,
        );

        registry.register(instance);
        let healthy = registry.discover_healthy("data-service");
        assert_eq!(healthy.len(), 1);

        registry.mark_healthy("inst-1", false);
        let healthy = registry.discover_healthy("data-service");
        assert_eq!(healthy.len(), 0);
    }

    #[test]
    fn test_vector_clock() {
        let nodes = vec!["A".to_string(), "B".to_string()];
        let mut vc1 = VectorClock::new(nodes.clone());
        vc1.increment("A");

        let mut vc2 = VectorClock::new(nodes);
        vc2.increment("A");
        vc2.increment("B");

        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));
    }

    #[test]
    fn test_leader_election() {
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];
        let mut election = LeaderElection::new("node1".to_string(), nodes);

        assert_eq!(election.state(), FollowerState::Follower);

        election.become_candidate();
        assert_eq!(election.state(), FollowerState::Candidate);

        election.receive_vote();
        election.receive_vote(); // quorum = 2
        assert_eq!(election.state(), FollowerState::Leader);
    }

    #[test]
    fn test_shard_map() {
        let map: ShardMap<String, String> = ShardMap::new(4);

        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());

        assert_eq!(map.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(map.get(&"key2".to_string()), Some("value2".to_string()));

        map.remove(&"key1".to_string());
        assert_eq!(map.get(&"key1".to_string()), None);
    }
}
