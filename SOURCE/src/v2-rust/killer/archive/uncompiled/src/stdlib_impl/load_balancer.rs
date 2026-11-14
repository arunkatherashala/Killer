// ================================================================
// LOAD BALANCER - Phase 27.2
// Load balancing algorithms and service routing
// ================================================================

use std::collections::HashMap;

/// Load balancer algorithm
#[derive(Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    ConsistentHash,
    LocalityAware,
    Random,
    IPHash,
    SessionAffinity,
    ResponseTime,
    CpuUtilization,
}

/// Server weight
#[derive(Clone, Debug)]
pub struct ServerWeight {
    pub server_id: String,
    pub weight: u32,
}

/// Server state
#[derive(Clone, Debug)]
pub struct ServerState {
    pub id: String,
    pub endpoint: String,
    pub healthy: bool,
    pub connections: u32,
    pub weight: u32,
    pub response_time_ms: u64,
    pub cpu_percent: u8,
}

pub struct LoadBalancerSolver;

impl LoadBalancerSolver {
    // ================================================================
    // ROUND ROBIN (1-10)
    // ================================================================

    /// Problem 1: Initialize round robin
    pub fn init_round_robin(servers: &[String]) -> usize {
        0 // Start at first server
    }

    /// Problem 2: Get next server round robin
    pub fn get_next_server_round_robin(
        servers: &[String],
        current_index: &mut usize,
    ) -> Option<String> {
        if servers.is_empty() {
            return None;
        }
        let server = servers[*current_index].clone();
        *current_index = (*current_index + 1) % servers.len();
        Some(server)
    }

    /// Problem 3: Get all servers
    pub fn get_all_servers(servers: &[String]) -> Vec<String> {
        servers.to_vec()
    }

    /// Problem 4: Add server to pool
    pub fn add_server_to_pool(servers: &mut Vec<String>, server: &str) {
        if !servers.contains(&server.to_string()) {
            servers.push(server.to_string());
        }
    }

    /// Problem 5: Remove server from pool
    pub fn remove_server_from_pool(servers: &mut Vec<String>, server: &str) {
        servers.retain(|s| s != server);
    }

    /// Problem 6: Get pool size
    pub fn get_pool_size(servers: &[String]) -> usize {
        servers.len()
    }

    /// Problem 7: Get current index
    pub fn get_current_index(index: usize) -> usize {
        index
    }

    /// Problem 8: Reset round robin
    pub fn reset_round_robin() -> usize {
        0
    }

    /// Problem 9: Rotate servers
    pub fn rotate_servers(servers: &mut Vec<String>, positions: usize) {
        if servers.is_empty() {
            return;
        }
        let pos = positions % servers.len();
        servers.rotate_left(pos);
    }

    /// Problem 10: Get server at index
    pub fn get_server_at_index(servers: &[String], index: usize) -> Option<String> {
        servers.get(index).cloned()
    }

    // ================================================================
    // LEAST CONNECTIONS (11-20)
    // ================================================================

    /// Problem 11: Initialize least connections
    pub fn init_least_connections(servers: &[ServerState]) -> HashMap<String, u32> {
        let mut connections = HashMap::new();
        for server in servers {
            connections.insert(server.id.clone(), server.connections);
        }
        connections
    }

    /// Problem 12: Find least connected server
    pub fn find_least_connected_server(
        connections: &HashMap<String, u32>,
        servers: &[ServerState],
    ) -> Option<String> {
        servers
            .iter()
            .filter(|s| s.healthy)
            .min_by_key(|s| connections.get(&s.id).copied().unwrap_or(0))
            .map(|s| s.id.clone())
    }

    /// Problem 13: Add connection
    pub fn add_connection(
        connections: &mut HashMap<String, u32>,
        server_id: &str,
    ) {
        *connections.entry(server_id.to_string()).or_insert(0) += 1;
    }

    /// Problem 14: Remove connection
    pub fn remove_connection(
        connections: &mut HashMap<String, u32>,
        server_id: &str,
    ) {
        if let Some(count) = connections.get_mut(server_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }

    /// Problem 15: Get connection count
    pub fn get_connection_count(
        connections: &HashMap<String, u32>,
        server_id: &str,
    ) -> u32 {
        connections.get(server_id).copied().unwrap_or(0)
    }

    /// Problem 16: Get total connections
    pub fn get_total_connections(connections: &HashMap<String, u32>) -> u32 {
        connections.values().sum()
    }

    /// Problem 17: Get average connections
    pub fn get_average_connections(connections: &HashMap<String, u32>) -> u32 {
        if connections.is_empty() {
            0
        } else {
            Self::get_total_connections(connections) / connections.len() as u32
        }
    }

    /// Problem 18: Rebalance connections
    pub fn rebalance_connections(
        connections: &mut HashMap<String, u32>,
        target: u32,
    ) {
        for count in connections.values_mut() {
            if *count > target {
                *count -= 1;
            }
        }
    }

    /// Problem 19: Get max connections
    pub fn get_max_connections(connections: &HashMap<String, u32>) -> u32 {
        connections.values().copied().max().unwrap_or(0)
    }

    /// Problem 20: Check connection threshold
    pub fn check_connection_threshold(
        connections: &HashMap<String, u32>,
        server_id: &str,
        threshold: u32,
    ) -> bool {
        Self::get_connection_count(connections, server_id) >= threshold
    }

    // ================================================================
    // WEIGHTED ROUND ROBIN (21-30)
    // ================================================================

    /// Problem 21: Initialize weighted round robin
    pub fn init_weighted_round_robin(weights: &[ServerWeight]) -> usize {
        0
    }

    /// Problem 22: Get next server weighted
    pub fn get_next_server_weighted(
        weights: &[ServerWeight],
        current_index: &mut usize,
    ) -> Option<String> {
        if weights.is_empty() {
            return None;
        }
        let weight_entry = &weights[*current_index];
        *current_index = (*current_index + 1) % weights.len();
        Some(weight_entry.server_id.clone())
    }

    /// Problem 23: Calculate weight distribution
    pub fn calculate_weight_distribution(weights: &[ServerWeight]) -> HashMap<String, f64> {
        let total_weight: u32 = weights.iter().map(|w| w.weight).sum();
        let mut distribution = HashMap::new();
        
        for weight in weights {
            let percentage = if total_weight > 0 {
                (weight.weight as f64 / total_weight as f64) * 100.0
            } else {
                0.0
            };
            distribution.insert(weight.server_id.clone(), percentage);
        }
        distribution
    }

    /// Problem 24: Adjust weight
    pub fn adjust_weight(weights: &mut [ServerWeight], server_id: &str, new_weight: u32) {
        for weight in weights {
            if weight.server_id == server_id {
                weight.weight = new_weight;
            }
        }
    }

    /// Problem 25: Get total weight
    pub fn get_total_weight(weights: &[ServerWeight]) -> u32 {
        weights.iter().map(|w| w.weight).sum()
    }

    /// Problem 26: Get weight for server
    pub fn get_weight_for_server(weights: &[ServerWeight], server_id: &str) -> u32 {
        weights
            .iter()
            .find(|w| w.server_id == server_id)
            .map(|w| w.weight)
            .unwrap_or(0)
    }

    /// Problem 27: Normalize weights
    pub fn normalize_weights(weights: &mut [ServerWeight]) {
        let total: u32 = weights.iter().map(|w| w.weight).sum();
        if total > 0 {
            for weight in weights {
                weight.weight = (weight.weight * 100) / total;
            }
        }
    }

    /// Problem 28: Scale weights
    pub fn scale_weights(weights: &mut [ServerWeight], factor: u32) {
        for weight in weights {
            weight.weight *= factor;
        }
    }

    /// Problem 29: Get weighted distribution count
    pub fn get_weighted_distribution_count(
        weights: &[ServerWeight],
        total_requests: u32,
    ) -> HashMap<String, u32> {
        let distribution = Self::calculate_weight_distribution(weights);
        let mut counts = HashMap::new();
        for (server_id, percentage) in distribution {
            let count = ((percentage / 100.0) * total_requests as f64) as u32;
            counts.insert(server_id, count);
        }
        counts
    }

    /// Problem 30: Reweight servers
    pub fn reweight_servers(weights: &mut [ServerWeight], new_total_weight: u32) {
        let current_total = Self::get_total_weight(weights);
        if current_total > 0 {
            for weight in weights {
                weight.weight = (weight.weight * new_total_weight) / current_total;
            }
        }
    }

    // ================================================================
    // CONSISTENT HASHING (31-40)
    // ================================================================

    /// Problem 31: Create hash ring
    pub fn create_hash_ring(servers: &[String]) -> HashMap<u64, String> {
        let mut ring = HashMap::new();
        for (i, server) in servers.iter().enumerate() {
            ring.insert(i as u64, server.clone());
        }
        ring
    }

    /// Problem 32: Hash key
    pub fn hash_key(key: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in key.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    /// Problem 33: Find server by key
    pub fn find_server_by_key(
        hash_ring: &HashMap<u64, String>,
        key: &str,
    ) -> Option<String> {
        if hash_ring.is_empty() {
            return None;
        }
        let key_hash = Self::hash_key(key);
        
        // Find next server in ring
        let mut candidates: Vec<u64> = hash_ring.keys().copied().collect();
        candidates.sort();
        
        for ring_hash in &candidates {
            if ring_hash >= &key_hash {
                return hash_ring.get(ring_hash).cloned();
            }
        }
        
        // Wrap around
        hash_ring.get(&candidates[0]).cloned()
    }

    /// Problem 34: Add server to ring
    pub fn add_server_to_ring(
        hash_ring: &mut HashMap<u64, String>,
        server: &str,
    ) -> u64 {
        let server_hash = Self::hash_key(server);
        hash_ring.insert(server_hash, server.to_string());
        server_hash
    }

    /// Problem 35: Remove server from ring
    pub fn remove_server_from_ring(
        hash_ring: &mut HashMap<u64, String>,
        server: &str,
    ) {
        let server_hash = Self::hash_key(server);
        hash_ring.remove(&server_hash);
    }

    /// Problem 36: Get ring nodes
    pub fn get_ring_nodes(hash_ring: &HashMap<u64, String>) -> Vec<String> {
        hash_ring.values().cloned().collect()
    }

    /// Problem 37: Get hash distribution
    pub fn get_hash_distribution(hash_ring: &HashMap<u64, String>) -> HashMap<String, u64> {
        let mut distribution = HashMap::new();
        for (hash, server) in hash_ring {
            distribution.insert(server.clone(), *hash);
        }
        distribution
    }

    /// Problem 38: Rebalance on server addition
    pub fn rebalance_on_server_addition(
        old_distribution: &HashMap<String, u64>,
        new_server: &str,
    ) -> HashMap<String, String> {
        let mut rebalance = HashMap::new();
        rebalance.insert("added".to_string(), new_server.to_string());
        rebalance
    }

    /// Problem 39: Get replicas
    pub fn get_replicas(
        hash_ring: &HashMap<u64, String>,
        key: &str,
        replica_count: usize,
    ) -> Vec<String> {
        if hash_ring.is_empty() {
            return Vec::new();
        }
        let key_hash = Self::hash_key(key);
        let mut candidates: Vec<u64> = hash_ring.keys().copied().collect();
        candidates.sort();
        
        let mut replicas = Vec::new();
        for ring_hash in candidates.iter().cycle().skip_while(|h| **h < key_hash).take(replica_count) {
            if let Some(server) = hash_ring.get(ring_hash) {
                replicas.push(server.clone());
            }
        }
        replicas
    }

    /// Problem 40: Check key ownership
    pub fn check_key_ownership(
        hash_ring: &HashMap<u64, String>,
        key: &str,
        server: &str,
    ) -> bool {
        Self::find_server_by_key(hash_ring, key)
            .map(|s| s == server)
            .unwrap_or(false)
    }

    // ================================================================
    // HEALTH-AWARE ROUTING (41-50)
    // ================================================================

    /// Problem 41: Get healthy servers
    pub fn get_healthy_servers(servers: &[ServerState]) -> Vec<ServerState> {
        servers.iter().filter(|s| s.healthy).cloned().collect()
    }

    /// Problem 42: Select by health
    pub fn select_by_health(servers: &[ServerState]) -> Option<String> {
        Self::get_healthy_servers(servers)
            .first()
            .map(|s| s.id.clone())
    }

    /// Problem 43: Get least utilized server
    pub fn get_least_utilized_server(servers: &[ServerState]) -> Option<String> {
        Self::get_healthy_servers(servers)
            .iter()
            .min_by_key(|s| s.connections)
            .map(|s| s.id.clone())
    }

    /// Problem 44: Get fastest server
    pub fn get_fastest_server(servers: &[ServerState]) -> Option<String> {
        Self::get_healthy_servers(servers)
            .iter()
            .min_by_key(|s| s.response_time_ms)
            .map(|s| s.id.clone())
    }

    /// Problem 45: Get lowest CPU server
    pub fn get_lowest_cpu_server(servers: &[ServerState]) -> Option<String> {
        Self::get_healthy_servers(servers)
            .iter()
            .min_by_key(|s| s.cpu_percent)
            .map(|s| s.id.clone())
    }

    /// Problem 46: Mark server unhealthy
    pub fn mark_server_unhealthy(servers: &mut [ServerState], server_id: &str) {
        if let Some(server) = servers.iter_mut().find(|s| s.id == server_id) {
            server.healthy = false;
        }
    }

    /// Problem 47: Mark server healthy
    pub fn mark_server_healthy(servers: &mut [ServerState], server_id: &str) {
        if let Some(server) = servers.iter_mut().find(|s| s.id == server_id) {
            server.healthy = true;
        }
    }

    /// Problem 48: Update server metrics
    pub fn update_server_metrics(
        servers: &mut [ServerState],
        server_id: &str,
        response_time_ms: u64,
        cpu_percent: u8,
    ) {
        if let Some(server) = servers.iter_mut().find(|s| s.id == server_id) {
            server.response_time_ms = response_time_ms;
            server.cpu_percent = cpu_percent;
        }
    }

    /// Problem 49: Get server load factor
    pub fn get_server_load_factor(server: &ServerState) -> f64 {
        let connection_factor = server.connections as f64 / 1000.0;
        let cpu_factor = server.cpu_percent as f64 / 100.0;
        let response_factor = server.response_time_ms as f64 / 100.0;
        connection_factor + cpu_factor + response_factor
    }

    /// Problem 50: Select optimal server
    pub fn select_optimal_server(servers: &[ServerState]) -> Option<String> {
        Self::get_healthy_servers(servers)
            .iter()
            .min_by(|a, b| {
                let a_factor = Self::get_server_load_factor(a);
                let b_factor = Self::get_server_load_factor(b);
                a_factor.partial_cmp(&b_factor).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let servers = vec!["srv1".to_string(), "srv2".to_string(), "srv3".to_string()];
        let mut index = LoadBalancerSolver::init_round_robin(&servers);
        
        let first = LoadBalancerSolver::get_next_server_round_robin(&servers, &mut index);
        assert_eq!(first, Some("srv1".to_string()));
        
        let second = LoadBalancerSolver::get_next_server_round_robin(&servers, &mut index);
        assert_eq!(second, Some("srv2".to_string()));
    }

    #[test]
    fn test_least_connections() {
        let servers = vec![
            ServerState { id: "s1".to_string(), endpoint: "1.1.1.1".to_string(), healthy: true, connections: 5, weight: 100, response_time_ms: 10, cpu_percent: 20 },
            ServerState { id: "s2".to_string(), endpoint: "2.2.2.2".to_string(), healthy: true, connections: 2, weight: 100, response_time_ms: 10, cpu_percent: 20 },
        ];
        
        let mut conn = LoadBalancerSolver::init_least_connections(&servers);
        let next = LoadBalancerSolver::find_least_connected_server(&conn, &servers);
        assert_eq!(next, Some("s2".to_string()));
        
        LoadBalancerSolver::add_connection(&mut conn, "s1");
        assert_eq!(LoadBalancerSolver::get_total_connections(&conn), 8);
    }

    #[test]
    fn test_weighted_distribution() {
        let weights = vec![
            ServerWeight { server_id: "s1".to_string(), weight: 100 },
            ServerWeight { server_id: "s2".to_string(), weight: 50 },
        ];
        
        let dist = LoadBalancerSolver::calculate_weight_distribution(&weights);
        assert_eq!(dist.len(), 2);
        assert!(dist["s1"] > dist["s2"]);
    }

    #[test]
    fn test_consistent_hash() {
        let servers = vec!["srv1".to_string(), "srv2".to_string(), "srv3".to_string()];
        let ring = LoadBalancerSolver::create_hash_ring(&servers);
        
        let server = LoadBalancerSolver::find_server_by_key(&ring, "key1");
        assert!(server.is_some());
    }

    #[test]
    fn test_hash_key() {
        let hash1 = LoadBalancerSolver::hash_key("key1");
        let hash2 = LoadBalancerSolver::hash_key("key1");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_healthy_servers() {
        let servers = vec![
            ServerState { id: "s1".to_string(), endpoint: "1.1.1.1".to_string(), healthy: true, connections: 5, weight: 100, response_time_ms: 10, cpu_percent: 20 },
            ServerState { id: "s2".to_string(), endpoint: "2.2.2.2".to_string(), healthy: false, connections: 2, weight: 100, response_time_ms: 10, cpu_percent: 20 },
        ];
        
        let healthy = LoadBalancerSolver::get_healthy_servers(&servers);
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_get_fastest_server() {
        let servers = vec![
            ServerState { id: "s1".to_string(), endpoint: "1.1.1.1".to_string(), healthy: true, connections: 5, weight: 100, response_time_ms: 50, cpu_percent: 20 },
            ServerState { id: "s2".to_string(), endpoint: "2.2.2.2".to_string(), healthy: true, connections: 2, weight: 100, response_time_ms: 10, cpu_percent: 20 },
        ];
        
        let fastest = LoadBalancerSolver::get_fastest_server(&servers);
        assert_eq!(fastest, Some("s2".to_string()));
    }

    #[test]
    fn test_select_optimal() {
        let servers = vec![
            ServerState { id: "s1".to_string(), endpoint: "1.1.1.1".to_string(), healthy: true, connections: 20, weight: 100, response_time_ms: 100, cpu_percent: 80 },
            ServerState { id: "s2".to_string(), endpoint: "2.2.2.2".to_string(), healthy: true, connections: 5, weight: 100, response_time_ms: 10, cpu_percent: 20 },
        ];
        
        let optimal = LoadBalancerSolver::select_optimal_server(&servers);
        assert_eq!(optimal, Some("s2".to_string()));
    }

    #[test]
    fn test_mark_unhealthy() {
        let mut servers = vec![
            ServerState { id: "s1".to_string(), endpoint: "1.1.1.1".to_string(), healthy: true, connections: 5, weight: 100, response_time_ms: 10, cpu_percent: 20 },
        ];
        
        LoadBalancerSolver::mark_server_unhealthy(&mut servers, "s1");
        assert!(!servers[0].healthy);
    }

    #[test]
    fn test_replicas() {
        let mut ring = HashMap::new();
        ring.insert(1, "s1".to_string());
        ring.insert(2, "s2".to_string());
        ring.insert(3, "s3".to_string());
        
        let replicas = LoadBalancerSolver::get_replicas(&ring, "key1", 2);
        assert_eq!(replicas.len(), 2);
    }

    #[test]
    fn test_normalize_weights() {
        let mut weights = vec![
            ServerWeight { server_id: "s1".to_string(), weight: 100 },
            ServerWeight { server_id: "s2".to_string(), weight: 200 },
        ];
        
        LoadBalancerSolver::normalize_weights(&mut weights);
        let total: u32 = weights.iter().map(|w| w.weight).sum();
        assert_eq!(total, 100);
    }
}
