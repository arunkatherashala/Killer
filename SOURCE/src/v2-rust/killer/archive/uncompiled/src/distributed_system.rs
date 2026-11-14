// Phase 12: Distributed System Support - cluster coordination, consensus, sharding
// Features: Node management, Raft consensus, state replication, distributed queries

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Node role in cluster
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeRole {
    Leader,
    Follower,
    Candidate,
}

impl NodeRole {
    pub fn as_str(&self) -> &str {
        match self {
            NodeRole::Leader => "leader",
            NodeRole::Follower => "follower",
            NodeRole::Candidate => "candidate",
        }
    }
}

/// Node state
#[derive(Clone, Debug)]
pub struct NodeState {
    pub node_id: String,
    pub role: NodeRole,
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub last_log_index: u64,
    pub last_log_term: u64,
    pub commit_index: u64,
    pub last_applied: u64,
}

impl NodeState {
    pub fn new(node_id: String) -> Self {
        NodeState {
            node_id,
            role: NodeRole::Follower,
            current_term: 0,
            voted_for: None,
            last_log_index: 0,
            last_log_term: 0,
            commit_index: 0,
            last_applied: 0,
        }
    }

    /// Become follower
    pub fn become_follower(mut self, term: u64) -> Self {
        self.role = NodeRole::Follower;
        self.current_term = term;
        self.voted_for = None;
        self
    }

    /// Become candidate
    pub fn become_candidate(mut self) -> Self {
        self.role = NodeRole::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id.clone());
        self
    }

    /// Become leader
    pub fn become_leader(mut self) -> Self {
        self.role = NodeRole::Leader;
        self
    }

    /// Is follower
    pub fn is_follower(&self) -> bool {
        self.role == NodeRole::Follower
    }

    /// Is leader
    pub fn is_leader(&self) -> bool {
        self.role == NodeRole::Leader
    }

    /// Is candidate
    pub fn is_candidate(&self) -> bool {
        self.role == NodeRole::Candidate
    }
}

/// Log entry
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub command: String,
    pub data: Vec<u8>,
}

impl LogEntry {
    pub fn new(term: u64, index: u64, command: String, data: Vec<u8>) -> Self {
        LogEntry {
            term,
            index,
            command,
            data,
        }
    }
}

/// Replication log
#[derive(Clone, Debug)]
pub struct ReplicationLog {
    pub entries: Vec<LogEntry>,
}

impl ReplicationLog {
    pub fn new() -> Self {
        ReplicationLog {
            entries: Vec::new(),
        }
    }

    /// Append entry
    pub fn append(&mut self, entry: LogEntry) -> Result<u64, String> {
        let index = self.entries.len() as u64;
        self.entries.push(entry);
        Ok(index)
    }

    /// Get entry at index
    pub fn get(&self, index: u64) -> Option<LogEntry> {
        self.entries.get(index as usize).cloned()
    }

    /// Get last entry
    pub fn last_entry(&self) -> Option<LogEntry> {
        self.entries.last().cloned()
    }

    /// Get entries from index
    pub fn get_entries_from(&self, index: u64) -> Vec<LogEntry> {
        self.entries.iter()
            .skip(index as usize)
            .cloned()
            .collect()
    }

    /// Truncate from index
    pub fn truncate_from(&mut self, index: u64) {
        self.entries.truncate(index as usize);
    }

    /// Get length
    pub fn len(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ReplicationLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Node info
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub healthy: bool,
    pub last_heartbeat: u64,
}

impl NodeInfo {
    pub fn new(node_id: String, address: String, port: u16) -> Self {
        NodeInfo {
            node_id,
            address,
            port,
            healthy: true,
            last_heartbeat: current_timestamp(),
        }
    }

    /// Get endpoint
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Update heartbeat
    pub fn update_heartbeat(mut self) -> Self {
        self.last_heartbeat = current_timestamp();
        self.healthy = true;
        self
    }

    /// Mark unhealthy
    pub fn mark_unhealthy(mut self) -> Self {
        self.healthy = false;
        self
    }
}

/// Cluster configuration
#[derive(Clone, Debug)]
pub struct ClusterConfig {
    pub cluster_id: String,
    pub nodes: HashMap<String, NodeInfo>,
    pub election_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
}

impl ClusterConfig {
    pub fn new(cluster_id: String) -> Self {
        ClusterConfig {
            cluster_id,
            nodes: HashMap::new(),
            election_timeout_ms: 150,
            heartbeat_interval_ms: 50,
        }
    }

    /// Add node
    pub fn add_node(&mut self, node_id: String, address: String, port: u16) -> Result<(), String> {
        if self.nodes.contains_key(&node_id) {
            return Err(format!("Node {} already exists", node_id));
        }

        let node_info = NodeInfo::new(node_id.clone(), address, port);
        self.nodes.insert(node_id, node_info);
        Ok(())
    }

    /// Remove node
    pub fn remove_node(&mut self, node_id: &str) -> Result<(), String> {
        if self.nodes.remove(node_id).is_some() {
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Get node
    pub fn get_node(&self, node_id: &str) -> Option<NodeInfo> {
        self.nodes.get(node_id).cloned()
    }

    /// List all nodes
    pub fn list_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.values().cloned().collect()
    }

    /// Get healthy nodes
    pub fn get_healthy_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.values()
            .filter(|n| n.healthy)
            .cloned()
            .collect()
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Quorum size
    pub fn quorum_size(&self) -> usize {
        (self.nodes.len() / 2) + 1
    }

    /// Has quorum
    pub fn has_quorum(&self) -> bool {
        self.get_healthy_nodes().len() >= self.quorum_size()
    }
}

/// Shard key range
#[derive(Clone, Debug)]
pub struct ShardKeyRange {
    pub shard_id: u32,
    pub start_key: String,
    pub end_key: String,
    pub owner_node: String,
}

impl ShardKeyRange {
    pub fn new(shard_id: u32, start_key: String, end_key: String, owner_node: String) -> Self {
        ShardKeyRange {
            shard_id,
            start_key,
            end_key,
            owner_node,
        }
    }

    /// Check if key in range
    pub fn contains_key(&self, key: &str) -> bool {
        key >= &self.start_key && key < &self.end_key
    }
}

/// Sharding strategy
#[derive(Clone, Debug)]
pub enum ShardingStrategy {
    Range,
    Hash,
    Directory,
}

impl ShardingStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            ShardingStrategy::Range => "range",
            ShardingStrategy::Hash => "hash",
            ShardingStrategy::Directory => "directory",
        }
    }
}

/// Shard map
#[derive(Clone, Debug)]
pub struct ShardMap {
    pub strategy: ShardingStrategy,
    pub shards: HashMap<u32, ShardKeyRange>,
    pub shard_count: u32,
}

impl ShardMap {
    pub fn new(strategy: ShardingStrategy, shard_count: u32) -> Self {
        ShardMap {
            strategy,
            shards: HashMap::new(),
            shard_count,
        }
    }

    /// Add shard
    pub fn add_shard(&mut self, shard: ShardKeyRange) -> Result<(), String> {
        if self.shards.contains_key(&shard.shard_id) {
            return Err(format!("Shard {} already exists", shard.shard_id));
        }
        self.shards.insert(shard.shard_id, shard);
        Ok(())
    }

    /// Get shard for key
    pub fn get_shard_for_key(&self, key: &str) -> Option<ShardKeyRange> {
        self.shards.values()
            .find(|s| s.contains_key(key))
            .cloned()
    }

    /// Get responsible node
    pub fn get_responsible_node(&self, key: &str) -> Option<String> {
        self.get_shard_for_key(key)
            .map(|s| s.owner_node)
    }

    /// List shards
    pub fn list_shards(&self) -> Vec<ShardKeyRange> {
        self.shards.values().cloned().collect()
    }

    /// Shard count
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Rebalance shards
    pub fn rebalance(&mut self, nodes: Vec<String>) -> Result<(), String> {
        if nodes.is_empty() {
            return Err("No nodes available".to_string());
        }

        self.shards.clear();
        let shard_size = (u32::MAX as f64 / nodes.len() as f64) as u32;

        for (i, node) in nodes.iter().enumerate() {
            let start = (i as u32) * shard_size;
            let end = if i == nodes.len() - 1 {
                u32::MAX
            } else {
                ((i + 1) as u32) * shard_size
            };

            let shard = ShardKeyRange::new(
                i as u32,
                start.to_string(),
                end.to_string(),
                node.clone(),
            );
            self.add_shard(shard)?;
        }

        Ok(())
    }
}

/// State snapshot
#[derive(Clone, Debug)]
pub struct StateSnapshot {
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub data: Vec<u8>,
}

impl StateSnapshot {
    pub fn new(last_included_index: u64, last_included_term: u64, data: Vec<u8>) -> Self {
        StateSnapshot {
            last_included_index,
            last_included_term,
            data,
        }
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Consensus state machine
#[derive(Clone)]
pub struct ConsensusStateMachine {
    pub node_state: NodeState,
    pub log: ReplicationLog,
    pub state_machine: HashMap<String, Vec<u8>>,
}

impl ConsensusStateMachine {
    pub fn new(node_id: String) -> Self {
        ConsensusStateMachine {
            node_state: NodeState::new(node_id),
            log: ReplicationLog::new(),
            state_machine: HashMap::new(),
        }
    }

    /// Apply log entry to state machine
    pub fn apply_entry(&mut self, entry: &LogEntry) -> Result<(), String> {
        self.state_machine.insert(entry.command.clone(), entry.data.clone());
        self.node_state.last_applied = entry.index;
        Ok(())
    }

    /// Get value
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.state_machine.get(key).cloned()
    }

    /// Set value
    pub fn set(&mut self, key: String, value: Vec<u8>) {
        self.state_machine.insert(key, value);
    }
}

impl Default for ConsensusStateMachine {
    fn default() -> Self {
        Self::new("node".to_string())
    }
}

/// Helper to get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Distributed system coordinator
pub struct DistributedCoordinator {
    pub cluster: ClusterConfig,
    pub shards: ShardMap,
    pub state_machines: HashMap<String, ConsensusStateMachine>,
}

impl DistributedCoordinator {
    pub fn new(cluster: ClusterConfig, shards: ShardMap) -> Self {
        DistributedCoordinator {
            cluster,
            shards,
            state_machines: HashMap::new(),
        }
    }

    /// Register state machine for node
    pub fn register_state_machine(&mut self, node_id: String, sm: ConsensusStateMachine) {
        self.state_machines.insert(node_id, sm);
    }

    /// Get state machine
    pub fn get_state_machine(&self, node_id: &str) -> Option<ConsensusStateMachine> {
        self.state_machines.get(node_id).cloned()
    }

    /// Route request to shard
    pub fn route_request(&self, key: &str) -> Option<String> {
        self.shards.get_responsible_node(key)
    }

    /// Check cluster health
    pub fn is_cluster_healthy(&self) -> bool {
        self.cluster.has_quorum()
    }

    /// Get cluster status
    pub fn get_cluster_status(&self) -> (usize, usize, bool) {
        let total_nodes = self.cluster.node_count();
        let healthy_nodes = self.cluster.get_healthy_nodes().len();
        let has_quorum = self.cluster.has_quorum();
        (total_nodes, healthy_nodes, has_quorum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_state_creation() {
        let state = NodeState::new("node1".to_string());
        assert_eq!(state.node_id, "node1");
        assert_eq!(state.role, NodeRole::Follower);
    }

    #[test]
    fn test_node_state_become_candidate() {
        let state = NodeState::new("node1".to_string());
        let candidate = state.become_candidate();
        assert_eq!(candidate.role, NodeRole::Candidate);
        assert_eq!(candidate.current_term, 1);
    }

    #[test]
    fn test_node_state_become_leader() {
        let state = NodeState::new("node1".to_string());
        let leader = state.become_candidate().become_leader();
        assert_eq!(leader.role, NodeRole::Leader);
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(1, 0, "SET".to_string(), vec![1, 2, 3]);
        assert_eq!(entry.term, 1);
        assert_eq!(entry.index, 0);
    }

    #[test]
    fn test_replication_log_append() {
        let mut log = ReplicationLog::new();
        let entry = LogEntry::new(1, 0, "SET".to_string(), vec![]);
        assert!(log.append(entry).is_ok());
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_replication_log_get() {
        let mut log = ReplicationLog::new();
        let entry = LogEntry::new(1, 0, "SET".to_string(), vec![]);
        log.append(entry.clone()).unwrap();
        
        let retrieved = log.get(0);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_replication_log_truncate() {
        let mut log = ReplicationLog::new();
        for i in 0..5 {
            log.append(LogEntry::new(1, i, "SET".to_string(), vec![])).unwrap();
        }
        
        log.truncate_from(3);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_node_info_creation() {
        let node = NodeInfo::new("node1".to_string(), "localhost".to_string(), 8080);
        assert_eq!(node.node_id, "node1");
        assert_eq!(node.endpoint(), "localhost:8080");
    }

    #[test]
    fn test_cluster_config_add_node() {
        let mut config = ClusterConfig::new("cluster1".to_string());
        assert!(config.add_node("node1".to_string(), "localhost".to_string(), 8080).is_ok());
        assert_eq!(config.node_count(), 1);
    }

    #[test]
    fn test_cluster_config_quorum() {
        let mut config = ClusterConfig::new("cluster1".to_string());
        config.add_node("node1".to_string(), "localhost".to_string(), 8080).unwrap();
        config.add_node("node2".to_string(), "localhost".to_string(), 8081).unwrap();
        config.add_node("node3".to_string(), "localhost".to_string(), 8082).unwrap();
        
        assert_eq!(config.quorum_size(), 2);
    }

    #[test]
    fn test_shard_key_range() {
        let shard = ShardKeyRange::new(0, "a".to_string(), "m".to_string(), "node1".to_string());
        assert!(shard.contains_key("hello"));
        assert!(!shard.contains_key("zombie"));
    }

    #[test]
    fn test_shard_map_add() {
        let mut map = ShardMap::new(ShardingStrategy::Range, 2);
        let shard = ShardKeyRange::new(0, "a".to_string(), "m".to_string(), "node1".to_string());
        assert!(map.add_shard(shard).is_ok());
    }

    #[test]
    fn test_shard_map_get_shard() {
        let mut map = ShardMap::new(ShardingStrategy::Range, 2);
        let shard = ShardKeyRange::new(0, "a".to_string(), "m".to_string(), "node1".to_string());
        map.add_shard(shard).unwrap();
        
        let result = map.get_shard_for_key("hello");
        assert!(result.is_some());
    }

    #[test]
    fn test_state_snapshot() {
        let snap = StateSnapshot::new(10, 2, vec![1, 2, 3, 4, 5]);
        assert_eq!(snap.size(), 5);
    }

    #[test]
    fn test_consensus_state_machine_creation() {
        let sm = ConsensusStateMachine::new("node1".to_string());
        assert_eq!(sm.node_state.node_id, "node1");
    }

    #[test]
    fn test_consensus_state_machine_set_get() {
        let mut sm = ConsensusStateMachine::new("node1".to_string());
        sm.set("key".to_string(), vec![1, 2, 3]);
        
        let value = sm.get("key");
        assert_eq!(value, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_distributed_coordinator_creation() {
        let cluster = ClusterConfig::new("cluster1".to_string());
        let shards = ShardMap::new(ShardingStrategy::Range, 2);
        let coordinator = DistributedCoordinator::new(cluster, shards);
        
        assert_eq!(coordinator.cluster.cluster_id, "cluster1");
    }

    #[test]
    fn test_node_role_as_str() {
        assert_eq!(NodeRole::Leader.as_str(), "leader");
        assert_eq!(NodeRole::Follower.as_str(), "follower");
        assert_eq!(NodeRole::Candidate.as_str(), "candidate");
    }
}
