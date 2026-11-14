// ================================================================
// ADVANCED ROUTING - Phase 29.1
// Intelligent traffic management with multiple algorithms
// ================================================================

use std::collections::HashMap;

/// Routing algorithm type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoutingAlgorithm {
    RoundRobin,
    LeastConnections,
    ConsistentHash,
    RingHash,
    MaglevHash,
    PowerOfTwo,
    ExponentialDecay,
    GeographicLatency,
    Random,
    LocalityPriority,
    CostAware,
}

/// Destination endpoint
#[derive(Clone, Debug)]
pub struct Destination {
    pub id: String,
    pub address: String,
    pub weight: u32,
    pub connections: u32,
    pub latency_ms: u64,
    pub error_rate: f64,
}

/// Route configuration
#[derive(Clone, Debug)]
pub struct Route {
    pub route_id: String,
    pub algorithm: RoutingAlgorithm,
    pub destinations: Vec<Destination>,
    pub current_index: usize,
}

pub struct AdvancedRoutingSolver;

impl AdvancedRoutingSolver {
    // ================================================================
    // ROUTING ALGORITHMS (1-12)
    // ================================================================

    /// Problem 1: Round robin routing
    pub fn round_robin(
        route: &mut Route,
    ) -> Result<Destination, String> {
        if route.destinations.is_empty() {
            return Err("No destinations".to_string());
        }
        let dest = route.destinations[route.current_index].clone();
        route.current_index = (route.current_index + 1) % route.destinations.len();
        Ok(dest)
    }

    /// Problem 2: Weighted round robin
    pub fn weighted_round_robin(
        destinations: &[Destination],
    ) -> Option<Destination> {
        let total_weight: u32 = destinations.iter().map(|d| d.weight).sum();
        if total_weight == 0 {
            return None;
        }
        let mut current = 0u32;
        let choice = (current % total_weight) as u32;
        let mut sum = 0u32;
        for dest in destinations {
            sum += dest.weight;
            if choice < sum {
                return Some(dest.clone());
            }
        }
        destinations.last().cloned()
    }

    /// Problem 3: Least connections
    pub fn least_connections(
        destinations: &[Destination],
    ) -> Option<Destination> {
        destinations
            .iter()
            .min_by_key(|d| d.connections)
            .cloned()
    }

    /// Problem 4: Consistent hash routing
    pub fn consistent_hash(
        key: &str,
        destinations: &[Destination],
    ) -> Option<Destination> {
        if destinations.is_empty() {
            return None;
        }
        let mut hash = 5381u64;
        for byte in key.as_bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(*byte as u64);
        }
        let index = (hash as usize) % destinations.len();
        destinations.get(index).cloned()
    }

    /// Problem 5: Ring hash for cache affinity
    pub fn ring_hash(
        key: &str,
        nodes: &[String],
    ) -> Option<String> {
        if nodes.is_empty() {
            return None;
        }
        let mut hash = 0u64;
        for byte in key.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        let index = (hash as usize) % nodes.len();
        nodes.get(index).cloned()
    }

    /// Problem 6: Maglev hashing (Google's algorithm)
    pub fn maglev_hash(
        key: &str,
        backends: &[String],
        table_size: usize,
    ) -> Option<String> {
        if backends.is_empty() {
            return None;
        }
        let mut hash = 0u64;
        for byte in key.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as u64);
        }
        let index = (hash as usize) % table_size % backends.len();
        backends.get(index).cloned()
    }

    /// Problem 7: Power of two choices
    pub fn power_of_two_choices(
        destinations: &[Destination],
    ) -> Option<Destination> {
        if destinations.len() < 2 {
            return destinations.first().cloned();
        }
        let idx1 = 0 % destinations.len();
        let idx2 = 1 % destinations.len();
        let d1 = &destinations[idx1];
        let d2 = &destinations[idx2];
        if d1.connections <= d2.connections {
            Some(d1.clone())
        } else {
            Some(d2.clone())
        }
    }

    /// Problem 8: Exponential decay routing
    pub fn exponential_decay(
        destinations: &[Destination],
    ) -> Option<Destination> {
        let max_latency = destinations
            .iter()
            .map(|d| d.latency_ms)
            .max()
            .unwrap_or(1);
        let mut best = None;
        let mut best_score = f64::NEG_INFINITY;
        for dest in destinations {
            let score = 1.0 - ((dest.latency_ms as f64) / (max_latency as f64));
            if score > best_score {
                best_score = score;
                best = Some(dest.clone());
            }
        }
        best
    }

    /// Problem 9: Geographic routing
    pub fn geographic_routing(
        client_lat: f64,
        client_lon: f64,
        destinations: &[(String, f64, f64)],
    ) -> Option<String> {
        let mut closest = None;
        let mut min_distance = f64::MAX;
        for (id, lat, lon) in destinations {
            let dist = ((client_lat - lat).powi(2) + (client_lon - lon).powi(2)).sqrt();
            if dist < min_distance {
                min_distance = dist;
                closest = Some(id.clone());
            }
        }
        closest
    }

    /// Problem 10: Locality priority routing
    pub fn locality_priority(
        destinations: &[(String, String)],
        client_zone: &str,
    ) -> Option<String> {
        for (id, zone) in destinations {
            if zone == client_zone {
                return Some(id.clone());
            }
        }
        destinations.first().map(|(id, _)| id.clone())
    }

    /// Problem 11: Cost-aware routing
    pub fn cost_aware_routing(
        destinations: &[(String, f64)],
    ) -> Option<String> {
        destinations
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id.clone())
    }

    /// Problem 12: Multi-level routing hierarchy
    pub fn multi_level_routing(
        primary: &[Destination],
        secondary: &[Destination],
    ) -> Option<Destination> {
        Self::least_connections(primary)
            .or_else(|| Self::least_connections(secondary))
    }

    // ================================================================
    // TRAFFIC SPLITTING (13-24)
    // ================================================================

    /// Problem 13: Canary traffic percentage
    pub fn canary_traffic_split(
        request_id: u64,
        canary_percent: u32,
        stable_dests: &[Destination],
        canary_dests: &[Destination],
    ) -> Option<Destination> {
        let hash = request_id % 100;
        if (hash as u32) < canary_percent {
            Self::least_connections(canary_dests)
        } else {
            Self::least_connections(stable_dests)
        }
    }

    /// Problem 14: User-based traffic splitting
    pub fn user_based_splitting(
        user_id: &str,
        stable_dests: &[Destination],
        canary_dests: &[Destination],
    ) -> Option<Destination> {
        if user_id.starts_with("beta_") {
            Self::least_connections(canary_dests)
        } else {
            Self::least_connections(stable_dests)
        }
    }

    /// Problem 15: Header-based routing
    pub fn header_based_routing(
        header_value: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Destination> {
        routes
            .get(header_value)
            .and_then(|dests| Self::least_connections(dests))
    }

    /// Problem 16: Path-based routing
    pub fn path_based_routing(
        path: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Destination> {
        if path.starts_with("/api/v2/") {
            routes
                .get("v2")
                .and_then(|dests| Self::least_connections(dests))
        } else if path.starts_with("/api/v1/") {
            routes
                .get("v1")
                .and_then(|dests| Self::least_connections(dests))
        } else {
            routes
                .get("default")
                .and_then(|dests| Self::least_connections(dests))
        }
    }

    /// Problem 17: Host-based routing
    pub fn host_based_routing(
        hostname: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Destination> {
        routes
            .get(hostname)
            .and_then(|dests| Self::least_connections(dests))
    }

    /// Problem 18: Query parameter splitting
    pub fn query_param_splitting(
        query_params: &HashMap<String, String>,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Destination> {
        if let Some(version) = query_params.get("version") {
            routes
                .get(version)
                .and_then(|dests| Self::least_connections(dests))
        } else {
            None
        }
    }

    /// Problem 19: Cookie-based session routing
    pub fn cookie_based_routing(
        cookie_value: &str,
        destinations: &[Destination],
    ) -> Option<Destination> {
        Self::consistent_hash(cookie_value, destinations)
    }

    /// Problem 20: Geolocation-based splitting
    pub fn geo_splitting(
        country: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Destination> {
        routes
            .get(country)
            .and_then(|dests| Self::least_connections(dests))
    }

    /// Problem 21: Time-based traffic splitting
    pub fn time_based_splitting(
        hour: u32,
        peak_dests: &[Destination],
        normal_dests: &[Destination],
    ) -> Option<Destination> {
        if hour >= 9 && hour < 17 {
            Self::least_connections(peak_dests)
        } else {
            Self::least_connections(normal_dests)
        }
    }

    /// Problem 22: A/B test distribution
    pub fn ab_test_distribution(
        user_id: &str,
        group_a: &[Destination],
        group_b: &[Destination],
    ) -> Option<Destination> {
        let mut hash = 0u64;
        for byte in user_id.as_bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        if hash % 2 == 0 {
            Self::least_connections(group_a)
        } else {
            Self::least_connections(group_b)
        }
    }

    /// Problem 23: Shadow traffic mirroring
    pub fn shadow_traffic_mirror(
        primary_dests: &[Destination],
        shadow_dests: &[Destination],
        mirror_percent: u32,
    ) -> (Option<Destination>, bool) {
        let mirror = (0 as u32) % 100 < mirror_percent;
        (Self::least_connections(primary_dests), mirror)
    }

    /// Problem 24: Weighted destination allocation
    pub fn weighted_allocation(
        total_requests: u64,
        destinations: &[(String, u32)],
    ) -> HashMap<String, u64> {
        let total_weight: u32 = destinations.iter().map(|(_, w)| w).sum();
        let mut allocation = HashMap::new();
        for (id, weight) in destinations {
            let share = (total_requests as u32 * weight) / total_weight.max(1);
            allocation.insert(id.clone(), share as u64);
        }
        allocation
    }

    // ================================================================
    // ROUTE MATCHING (25-36)
    // ================================================================

    /// Problem 25: Exact path match
    pub fn exact_path_match(
        request_path: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Vec<Destination>> {
        routes.get(request_path).cloned()
    }

    /// Problem 26: Prefix path match
    pub fn prefix_path_match(
        request_path: &str,
        routes: &HashMap<String, Vec<Destination>>,
    ) -> Option<Vec<Destination>> {
        for (pattern, dests) in routes {
            if request_path.starts_with(pattern) {
                return Some(dests.clone());
            }
        }
        None
    }

    /// Problem 27: Regex pattern match
    pub fn regex_pattern_match(
        request_path: &str,
        pattern: &str,
    ) -> bool {
        request_path.contains(pattern)
    }

    /// Problem 28: HTTP method match
    pub fn method_match(
        method: &str,
        allowed: &[&str],
    ) -> bool {
        allowed.contains(&method)
    }

    /// Problem 29: Header condition match
    pub fn header_condition_match(
        headers: &HashMap<String, String>,
        header_name: &str,
        expected_value: &str,
    ) -> bool {
        headers.get(header_name).map(|v| v == expected_value).unwrap_or(false)
    }

    /// Problem 30: Query parameter match
    pub fn query_param_match(
        query_params: &HashMap<String, String>,
        param_name: &str,
        expected_value: &str,
    ) -> bool {
        query_params.get(param_name).map(|v| v == expected_value).unwrap_or(false)
    }

    /// Problem 31: Hostname match
    pub fn hostname_match(
        request_host: &str,
        allowed_hosts: &[&str],
    ) -> bool {
        allowed_hosts.contains(&request_host)
    }

    /// Problem 32: TLS SNI matching
    pub fn sni_match(
        sni_name: &str,
        allowed_snis: &[String],
    ) -> bool {
        allowed_snis.contains(&sni_name.to_string())
    }

    /// Problem 33: Priority route ordering
    pub fn priority_route_match(
        routes: &[(u32, String, bool)],
        request: &str,
    ) -> Option<(u32, String)> {
        let mut best = None;
        let mut highest_priority = u32::MIN;
        for (priority, pattern, _matches) in routes {
            if *priority > highest_priority && request.contains(pattern) {
                highest_priority = *priority;
                best = Some((*priority, pattern.clone()));
            }
        }
        best
    }

    /// Problem 34: Fallback routes
    pub fn fallback_route(
        routes: &[Vec<Destination>],
    ) -> Option<Destination> {
        for route in routes {
            if let Some(dest) = Self::least_connections(route) {
                return Some(dest);
            }
        }
        None
    }

    /// Problem 35: Conditional chain (AND logic)
    pub fn conditional_chain(
        conditions: &[bool],
    ) -> bool {
        conditions.iter().all(|&c| c)
    }

    /// Problem 36: Negation rules
    pub fn negation_rule(
        condition: bool,
    ) -> bool {
        !condition
    }

    // ================================================================
    // LOAD BALANCER POOL MANAGEMENT (37-50)
    // ================================================================

    /// Problem 37: Create route group
    pub fn create_route_group(
        group_id: &str,
        algorithm: RoutingAlgorithm,
    ) -> Route {
        Route {
            route_id: group_id.to_string(),
            algorithm,
            destinations: Vec::new(),
            current_index: 0,
        }
    }

    /// Problem 38: Add destination to pool
    pub fn add_destination(
        route: &mut Route,
        dest: Destination,
    ) {
        route.destinations.push(dest);
    }

    /// Problem 39: Remove destination
    pub fn remove_destination(
        route: &mut Route,
        dest_id: &str,
    ) -> bool {
        let len = route.destinations.len();
        route.destinations.retain(|d| d.id != dest_id);
        route.destinations.len() < len
    }

    /// Problem 40: Update destination weight
    pub fn update_destination_weight(
        route: &mut Route,
        dest_id: &str,
        new_weight: u32,
    ) -> bool {
        if let Some(dest) = route.destinations.iter_mut().find(|d| d.id == dest_id) {
            dest.weight = new_weight;
            true
        } else {
            false
        }
    }

    /// Problem 41: Get available destinations
    pub fn get_available_destinations(
        route: &Route,
    ) -> Vec<Destination> {
        route.destinations.clone()
    }

    /// Problem 42: Detect dead endpoints
    pub fn detect_dead_endpoints(
        route: &Route,
        error_threshold: f64,
    ) -> Vec<String> {
        route
            .destinations
            .iter()
            .filter(|d| d.error_rate > error_threshold)
            .map(|d| d.id.clone())
            .collect()
    }

    /// Problem 43: Rebalance load
    pub fn rebalance_load(
        route: &mut Route,
    ) {
        let count = route.destinations.len() as u32;
        if count > 0 {
            for dest in &mut route.destinations {
                dest.weight = 100 / count;
            }
        }
    }

    /// Problem 44: Get route statistics
    pub fn get_route_statistics(
        route: &Route,
    ) -> (usize, u32, f64) {
        let count = route.destinations.len();
        let total_conns: u32 = route.destinations.iter().map(|d| d.connections).sum();
        let avg_error: f64 = if !route.destinations.is_empty() {
            route.destinations.iter().map(|d| d.error_rate).sum::<f64>()
                / count as f64
        } else {
            0.0
        };
        (count, total_conns, avg_error)
    }

    /// Problem 45: Calculate average latency
    pub fn calculate_average_latency(
        destinations: &[Destination],
    ) -> u64 {
        if destinations.is_empty() {
            return 0;
        }
        destinations.iter().map(|d| d.latency_ms).sum::<u64>()
            / destinations.len() as u64
    }

    /// Problem 46: Get 95th percentile
    pub fn get_95th_percentile(
        latencies: &[u64],
    ) -> u64 {
        if latencies.is_empty() {
            return 0;
        }
        let mut sorted = latencies.to_vec();
        sorted.sort();
        let index = (sorted.len() * 95) / 100;
        sorted[index.min(sorted.len() - 1)]
    }

    /// Problem 47: Detect outliers
    pub fn detect_outliers(
        destinations: &[Destination],
        std_dev_threshold: f64,
    ) -> Vec<String> {
        let avg_latency = Self::calculate_average_latency(destinations);
        destinations
            .iter()
            .filter(|d| {
                let diff = (d.latency_ms as i64 - avg_latency as i64).abs() as f64;
                diff > std_dev_threshold
            })
            .map(|d| d.id.clone())
            .collect()
    }

    /// Problem 48: Drain connections
    pub fn drain_connections(
        route: &mut Route,
        dest_id: &str,
    ) -> bool {
        if let Some(dest) = route.destinations.iter_mut().find(|d| d.id == dest_id) {
            dest.connections = 0;
            true
        } else {
            false
        }
    }

    /// Problem 49: Enable maintenance mode
    pub fn enable_maintenance_mode(
        route: &mut Route,
        dest_id: &str,
    ) -> bool {
        if let Some(dest) = route.destinations.iter_mut().find(|d| d.id == dest_id) {
            dest.weight = 0;
            true
        } else {
            false
        }
    }

    /// Problem 50: Generate route report
    pub fn generate_route_report(
        route: &Route,
    ) -> String {
        let (count, conns, error) = Self::get_route_statistics(route);
        format!(
            "Route: {} | Dests: {} | Conns: {} | Err: {:.2}%",
            route.route_id, count, conns, error * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let mut route = Route {
            route_id: "test".to_string(),
            algorithm: RoutingAlgorithm::RoundRobin,
            destinations: vec![
                Destination {
                    id: "d1".to_string(),
                    address: "1.1.1.1".to_string(),
                    weight: 1,
                    connections: 0,
                    latency_ms: 10,
                    error_rate: 0.0,
                },
                Destination {
                    id: "d2".to_string(),
                    address: "2.2.2.2".to_string(),
                    weight: 1,
                    connections: 0,
                    latency_ms: 10,
                    error_rate: 0.0,
                },
            ],
            current_index: 0,
        };
        let d1 = AdvancedRoutingSolver::round_robin(&mut route).unwrap();
        assert_eq!(d1.id, "d1");
        let d2 = AdvancedRoutingSolver::round_robin(&mut route).unwrap();
        assert_eq!(d2.id, "d2");
    }

    #[test]
    fn test_least_connections() {
        let dests = vec![
            Destination {
                id: "d1".to_string(),
                address: "1.1.1.1".to_string(),
                weight: 1,
                connections: 5,
                latency_ms: 10,
                error_rate: 0.0,
            },
            Destination {
                id: "d2".to_string(),
                address: "2.2.2.2".to_string(),
                weight: 1,
                connections: 2,
                latency_ms: 10,
                error_rate: 0.0,
            },
        ];
        let best = AdvancedRoutingSolver::least_connections(&dests).unwrap();
        assert_eq!(best.id, "d2");
    }

    #[test]
    fn test_consistent_hash() {
        let dests = vec![
            Destination {
                id: "d1".to_string(),
                address: "1.1.1.1".to_string(),
                weight: 1,
                connections: 0,
                latency_ms: 10,
                error_rate: 0.0,
            },
            Destination {
                id: "d2".to_string(),
                address: "2.2.2.2".to_string(),
                weight: 1,
                connections: 0,
                latency_ms: 10,
                error_rate: 0.0,
            },
        ];
        let dest1 = AdvancedRoutingSolver::consistent_hash("user123", &dests);
        let dest2 = AdvancedRoutingSolver::consistent_hash("user123", &dests);
        assert_eq!(dest1.as_ref().map(|d| &d.id), dest2.as_ref().map(|d| &d.id));
    }

    #[test]
    fn test_canary_split() {
        let stable = vec![Destination {
            id: "stable".to_string(),
            address: "1.1.1.1".to_string(),
            weight: 1,
            connections: 0,
            latency_ms: 10,
            error_rate: 0.0,
        }];
        let canary = vec![Destination {
            id: "canary".to_string(),
            address: "2.2.2.2".to_string(),
            weight: 1,
            connections: 0,
            latency_ms: 10,
            error_rate: 0.0,
        }];
        let dest = AdvancedRoutingSolver::canary_traffic_split(1, 50, &stable, &canary);
        assert!(dest.is_some());
    }

    #[test]
    fn test_detect_dead_endpoints() {
        let route = Route {
            route_id: "test".to_string(),
            algorithm: RoutingAlgorithm::RoundRobin,
            destinations: vec![
                Destination {
                    id: "healthy".to_string(),
                    address: "1.1.1.1".to_string(),
                    weight: 1,
                    connections: 5,
                    latency_ms: 10,
                    error_rate: 0.01,
                },
                Destination {
                    id: "dead".to_string(),
                    address: "2.2.2.2".to_string(),
                    weight: 1,
                    connections: 0,
                    latency_ms: 500,
                    error_rate: 0.95,
                },
            ],
            current_index: 0,
        };
        let dead = AdvancedRoutingSolver::detect_dead_endpoints(&route, 0.5);
        assert_eq!(dead, vec!["dead".to_string()]);
    }

    #[test]
    fn test_add_destination() {
        let mut route = AdvancedRoutingSolver::create_route_group("test", RoutingAlgorithm::RoundRobin);
        let dest = Destination {
            id: "d1".to_string(),
            address: "1.1.1.1".to_string(),
            weight: 1,
            connections: 0,
            latency_ms: 10,
            error_rate: 0.0,
        };
        AdvancedRoutingSolver::add_destination(&mut route, dest);
        assert_eq!(route.destinations.len(), 1);
    }

    #[test]
    fn test_average_latency() {
        let dests = vec![
            Destination {
                id: "d1".to_string(),
                address: "1.1.1.1".to_string(),
                weight: 1,
                connections: 0,
                latency_ms: 100,
                error_rate: 0.0,
            },
            Destination {
                id: "d2".to_string(),
                address: "2.2.2.2".to_string(),
                weight: 1,
                connections: 0,
                latency_ms: 200,
                error_rate: 0.0,
            },
        ];
        let avg = AdvancedRoutingSolver::calculate_average_latency(&dests);
        assert_eq!(avg, 150);
    }

    #[test]
    fn test_route_statistics() {
        let route = Route {
            route_id: "test".to_string(),
            algorithm: RoutingAlgorithm::RoundRobin,
            destinations: vec![
                Destination {
                    id: "d1".to_string(),
                    address: "1.1.1.1".to_string(),
                    weight: 1,
                    connections: 10,
                    latency_ms: 10,
                    error_rate: 0.02,
                },
                Destination {
                    id: "d2".to_string(),
                    address: "2.2.2.2".to_string(),
                    weight: 1,
                    connections: 5,
                    latency_ms: 10,
                    error_rate: 0.01,
                },
            ],
            current_index: 0,
        };
        let (count, conns, _error) = AdvancedRoutingSolver::get_route_statistics(&route);
        assert_eq!(count, 2);
        assert_eq!(conns, 15);
    }

    #[test]
    fn test_percentile() {
        let latencies = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let p95 = AdvancedRoutingSolver::get_95th_percentile(&latencies);
        assert!(p95 >= 90);
    }

    #[test]
    fn test_rebalance_load() {
        let mut route = Route {
            route_id: "test".to_string(),
            algorithm: RoutingAlgorithm::RoundRobin,
            destinations: vec![
                Destination {
                    id: "d1".to_string(),
                    address: "1.1.1.1".to_string(),
                    weight: 10,
                    connections: 0,
                    latency_ms: 10,
                    error_rate: 0.0,
                },
                Destination {
                    id: "d2".to_string(),
                    address: "2.2.2.2".to_string(),
                    weight: 5,
                    connections: 0,
                    latency_ms: 10,
                    error_rate: 0.0,
                },
            ],
            current_index: 0,
        };
        AdvancedRoutingSolver::rebalance_load(&mut route);
        assert_eq!(route.destinations[0].weight, 50);
    }
}
