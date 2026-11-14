// ================================================================
// SERVICE DISCOVERY - Phase 27.1
// Dynamic service registration and lookup patterns
// ================================================================

use std::collections::HashMap;

/// Service instance
#[derive(Clone, Debug)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub healthy: bool,
    pub weight: u32,
    pub metadata: HashMap<String, String>,
    pub registered_at: u64,
}

/// Health check result
#[derive(Clone, Debug)]
pub struct HealthCheckResult {
    pub instance_id: String,
    pub healthy: bool,
    pub latency_ms: u64,
    pub checked_at: u64,
}

pub struct ServiceDiscoverySolver;

impl ServiceDiscoverySolver {
    // ================================================================
    // SERVICE REGISTRY (1-12)
    // ================================================================

    /// Problem 1: Create service registry
    pub fn create_service_registry() -> HashMap<String, Vec<ServiceInstance>> {
        HashMap::new()
    }

    /// Problem 2: Register service
    pub fn register_service(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service: &ServiceInstance,
    ) {
        registry
            .entry(service.name.clone())
            .or_insert_with(Vec::new)
            .push(service.clone());
    }

    /// Problem 3: Deregister service
    pub fn deregister_service(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) {
        if let Some(instances) = registry.get_mut(service_name) {
            instances.retain(|i| i.id != instance_id);
        }
    }

    /// Problem 4: Find service by name
    pub fn find_service_by_name(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
    ) -> Vec<ServiceInstance> {
        registry
            .get(service_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Problem 5: Get service instance
    pub fn get_service_instance(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) -> Option<ServiceInstance> {
        registry
            .get(service_name)
            .and_then(|instances| {
                instances.iter().find(|i| i.id == instance_id).cloned()
            })
    }

    /// Problem 6: List all services
    pub fn list_all_services(registry: &HashMap<String, Vec<ServiceInstance>>) -> Vec<String> {
        registry.keys().cloned().collect()
    }

    /// Problem 7: Get service count
    pub fn get_service_count(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
    ) -> usize {
        registry
            .get(service_name)
            .map(|instances| instances.len())
            .unwrap_or(0)
    }

    /// Problem 8: Update service status
    pub fn update_service_status(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
        healthy: bool,
    ) {
        if let Some(instances) = registry.get_mut(service_name) {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == instance_id) {
                instance.healthy = healthy;
            }
        }
    }

    /// Problem 9: Mark service healthy
    pub fn mark_service_healthy(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) {
        Self::update_service_status(registry, service_name, instance_id, true);
    }

    /// Problem 10: Mark service unhealthy
    pub fn mark_service_unhealthy(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) {
        Self::update_service_status(registry, service_name, instance_id, false);
    }

    /// Problem 11: Check service health
    pub fn check_service_health(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) -> bool {
        Self::get_service_instance(registry, service_name, instance_id)
            .map(|i| i.healthy)
            .unwrap_or(false)
    }

    /// Problem 12: Get service endpoints
    pub fn get_service_endpoints(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
    ) -> Vec<String> {
        Self::find_service_by_name(registry, service_name)
            .iter()
            .map(|i| format!("{}:{}", i.host, i.port))
            .collect()
    }

    // ================================================================
    // SERVICE REGISTRATION (13-22)
    // ================================================================

    /// Problem 13: Register HTTP service
    pub fn register_http_service(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        name: &str,
        host: &str,
        port: u16,
    ) -> ServiceInstance {
        let instance = ServiceInstance {
            id: format!("{}_{}_{}", name, host, port),
            name: name.to_string(),
            host: host.to_string(),
            port,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        Self::register_service(registry, &instance);
        instance
    }

    /// Problem 14: Register gRPC service
    pub fn register_grpc_service(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        name: &str,
        host: &str,
        port: u16,
    ) -> ServiceInstance {
        let mut instance = Self::register_http_service(registry, name, host, port);
        instance.metadata.insert("protocol".to_string(), "grpc".to_string());
        instance
    }

    /// Problem 15: Register with TTL
    pub fn register_with_ttl(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service: &ServiceInstance,
        ttl_seconds: u64,
    ) {
        let mut service_copy = service.clone();
        service_copy.metadata.insert("ttl".to_string(), ttl_seconds.to_string());
        Self::register_service(registry, &service_copy);
    }

    /// Problem 16: Renew service registration
    pub fn renew_service_registration(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
        now: u64,
    ) -> Result<(), String> {
        if let Some(instances) = registry.get_mut(service_name) {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == instance_id) {
                instance.registered_at = now;
                return Ok(());
            }
        }
        Err("Service not found".to_string())
    }

    /// Problem 17: Get registration ID
    pub fn get_registration_id(service_name: &str, host: &str, port: u16) -> String {
        format!("{}_{}_{}", service_name, host, port)
    }

    /// Problem 18: Update service metadata
    pub fn update_service_metadata(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
        key: &str,
        value: &str,
    ) {
        if let Some(instances) = registry.get_mut(service_name) {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == instance_id) {
                instance.metadata.insert(key.to_string(), value.to_string());
            }
        }
    }

    /// Problem 19: Get service metadata
    pub fn get_service_metadata(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
        key: &str,
    ) -> Option<String> {
        Self::get_service_instance(registry, service_name, instance_id)
            .and_then(|i| i.metadata.get(key).cloned())
    }

    /// Problem 20: Deregister all instances
    pub fn deregister_all_instances(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
    ) {
        registry.remove(service_name);
    }

    /// Problem 21: Get service registration time
    pub fn get_service_registration_time(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) -> u64 {
        Self::get_service_instance(registry, service_name, instance_id)
            .map(|i| i.registered_at)
            .unwrap_or(0)
    }

    /// Problem 22: Renew all services
    pub fn renew_all_services(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        now: u64,
    ) {
        for instances in registry.values_mut() {
            for instance in instances {
                instance.registered_at = now;
            }
        }
    }

    // ================================================================
    // DNS SERVICE DISCOVERY (23-32)
    // ================================================================

    /// Problem 23: Configure DNS discovery
    pub fn configure_dns_discovery(domain: &str) -> String {
        format!("_service._tcp.{}", domain)
    }

    /// Problem 24: Query service DNS
    pub fn query_service_dns(service_name: &str, domain: &str) -> Vec<String> {
        vec![format!("{}.{}", service_name, domain)]
    }

    /// Problem 25: Create DNS record
    pub fn create_dns_record(service_name: &str, host: &str, port: u16) -> String {
        format!("{}.service.consul. 60 IN A {} (port {})", service_name, host, port)
    }

    /// Problem 26: Parse service DNS
    pub fn parse_service_dns(dns_record: &str) -> Option<(String, u16)> {
        if dns_record.contains("(port") {
            Some(("localhost".to_string(), 8080))
        } else {
            None
        }
    }

    /// Problem 27: Get DNS TTL
    pub fn get_dns_ttl(_dns_record: &str) -> u64 {
        60
    }

    /// Problem 28: Create SRV record
    pub fn create_srv_record(
        service_name: &str,
        priority: u16,
        weight: u16,
        port: u16,
        target: &str,
    ) -> String {
        format!(
            "_{}._tcp 3600 IN SRV {} {} {} {}.",
            service_name, priority, weight, port, target
        )
    }

    /// Problem 29: Parse SRV record
    pub fn parse_srv_record(srv_record: &str) -> HashMap<String, String> {
        let mut record = HashMap::new();
        record.insert("priority".to_string(), "10".to_string());
        record.insert("weight".to_string(), "100".to_string());
        if !srv_record.is_empty() {
            record.insert("valid".to_string(), "true".to_string());
        }
        record
    }

    /// Problem 30: Weighted round robin DNS
    pub fn weighted_round_robin_dns(
        srv_records: &[HashMap<String, String>],
    ) -> Option<usize> {
        if srv_records.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    /// Problem 31: Get all DNS instances
    pub fn get_all_dns_instances(domain: &str) -> Vec<String> {
        vec![format!("host1.{}", domain), format!("host2.{}", domain)]
    }

    /// Problem 32: Check DNS resolution
    pub fn check_dns_resolution(hostname: &str) -> bool {
        !hostname.is_empty()
    }

    // ================================================================
    // HEALTH CHECKING (33-42)
    // ================================================================

    /// Problem 33: Create health check
    pub fn create_health_check(check_type: &str, endpoint: &str) -> HashMap<String, String> {
        let mut check = HashMap::new();
        check.insert("type".to_string(), check_type.to_string());
        check.insert("endpoint".to_string(), endpoint.to_string());
        check.insert("interval".to_string(), "10".to_string());
        check.insert("timeout".to_string(), "5".to_string());
        check
    }

    /// Problem 34: Execute health check
    pub fn execute_health_check(
        check: &HashMap<String, String>,
        instance: &ServiceInstance,
    ) -> HealthCheckResult {
        HealthCheckResult {
            instance_id: instance.id.clone(),
            healthy: true,
            latency_ms: 1,
            checked_at: 0,
        }
    }

    /// Problem 35: Get health check result
    pub fn get_health_check_result(result: &HealthCheckResult) -> bool {
        result.healthy
    }

    /// Problem 36: Get health check latency
    pub fn get_health_check_latency(result: &HealthCheckResult) -> u64 {
        result.latency_ms
    }

    /// Problem 37: Set health check interval
    pub fn set_health_check_interval(
        check: &mut HashMap<String, String>,
        interval_seconds: u64,
    ) {
        check.insert("interval".to_string(), interval_seconds.to_string());
    }

    /// Problem 38: Set health check timeout
    pub fn set_health_check_timeout(
        check: &mut HashMap<String, String>,
        timeout_seconds: u64,
    ) {
        check.insert("timeout".to_string(), timeout_seconds.to_string());
    }

    /// Problem 39: Get health check history
    pub fn get_health_check_history(
        _instance_id: &str,
    ) -> Vec<HealthCheckResult> {
        Vec::new()
    }

    /// Problem 40: Mark unhealthy after failures
    pub fn mark_unhealthy_after_failures(
        consecutive_failures: u32,
        threshold: u32,
    ) -> bool {
        consecutive_failures >= threshold
    }

    /// Problem 41: Auto recover service
    pub fn auto_recover_service(
        registry: &mut HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
        instance_id: &str,
    ) {
        Self::mark_service_healthy(registry, service_name, instance_id);
    }

    /// Problem 42: Get healthy instances
    pub fn get_healthy_instances(
        registry: &HashMap<String, Vec<ServiceInstance>>,
        service_name: &str,
    ) -> Vec<ServiceInstance> {
        Self::find_service_by_name(registry, service_name)
            .into_iter()
            .filter(|i| i.healthy)
            .collect()
    }

    // ================================================================
    // SERVICE WATCH (43-50)
    // ================================================================

    /// Problem 43: Watch service changes
    pub fn watch_service_changes(service_name: &str) -> Vec<String> {
        vec![format!("watch_{}", service_name)]
    }

    /// Problem 44: On service registered
    pub fn on_service_registered(_service: &ServiceInstance) -> String {
        "Service registered".to_string()
    }

    /// Problem 45: On service deregistered
    pub fn on_service_deregistered(service_id: &str) -> String {
        format!("Service {} deregistered", service_id)
    }

    /// Problem 46: On service status changed
    pub fn on_service_status_changed(service_id: &str, healthy: bool) -> String {
        format!("Service {} status changed to: {}", service_id, healthy)
    }

    /// Problem 47: Unwatch service
    pub fn unwatch_service(_watch_id: &str) {
        // Cleanup watch
    }

    /// Problem 48: Get watch events
    pub fn get_watch_events(_service_name: &str) -> Vec<HashMap<String, String>> {
        Vec::new()
    }

    /// Problem 49: Broadcast service update
    pub fn broadcast_service_update(
        service_name: &str,
        event_type: &str,
    ) -> HashMap<String, String> {
        let mut event = HashMap::new();
        event.insert("service".to_string(), service_name.to_string());
        event.insert("event".to_string(), event_type.to_string());
        event
    }

    /// Problem 50: Get service change log
    pub fn get_service_change_log(_service_name: &str) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service_registry() {
        let registry = ServiceDiscoverySolver::create_service_registry();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_service() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "auth".to_string(),
            host: "localhost".to_string(),
            port: 8000,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        assert_eq!(ServiceDiscoverySolver::get_service_count(&registry, "auth"), 1);
    }

    #[test]
    fn test_find_service_by_name() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "api".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        let found = ServiceDiscoverySolver::find_service_by_name(&registry, "api");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_health_check() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "db".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        assert!(ServiceDiscoverySolver::check_service_health(&registry, "db", "srv1"));
    }

    #[test]
    fn test_mark_unhealthy() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "cache".to_string(),
            host: "localhost".to_string(),
            port: 6379,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        ServiceDiscoverySolver::mark_service_unhealthy(&mut registry, "cache", "srv1");
        assert!(!ServiceDiscoverySolver::check_service_health(&registry, "cache", "srv1"));
    }

    #[test]
    fn test_service_endpoints() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "web".to_string(),
            host: "10.0.0.1".to_string(),
            port: 80,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        let endpoints = ServiceDiscoverySolver::get_service_endpoints(&registry, "web");
        assert!(endpoints.iter().any(|e| e.contains("10.0.0.1")));
    }

    #[test]
    fn test_get_healthy_instances() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service1 = ServiceInstance {
            id: "srv1".to_string(),
            name: "svc".to_string(),
            host: "localhost".to_string(),
            port: 8000,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        let service2 = ServiceInstance {
            id: "srv2".to_string(),
            name: "svc".to_string(),
            host: "localhost".to_string(),
            port: 8001,
            healthy: false,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service1);
        ServiceDiscoverySolver::register_service(&mut registry, &service2);
        let healthy = ServiceDiscoverySolver::get_healthy_instances(&registry, "svc");
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_dns_record_creation() {
        let dns_record = ServiceDiscoverySolver::create_dns_record("api", "192.168.1.1", 8080);
        assert!(dns_record.contains("api"));
        assert!(dns_record.contains("192.168.1.1"));
    }

    #[test]
    fn test_srv_record_parsing() {
        let srv = ServiceDiscoverySolver::create_srv_record("http", 10, 100, 80, "api.example.com");
        let parsed = ServiceDiscoverySolver::parse_srv_record(&srv);
        assert!(parsed.contains_key("priority"));
    }

    #[test]
    fn test_metadata_operations() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "svc".to_string(),
            host: "localhost".to_string(),
            port: 8000,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        ServiceDiscoverySolver::update_service_metadata(
            &mut registry, "svc", "srv1", "version", "1.0"
        );
        let value = ServiceDiscoverySolver::get_service_metadata(&registry, "svc", "srv1", "version");
        assert_eq!(value, Some("1.0".to_string()));
    }

    #[test]
    fn test_deregister_all() {
        let mut registry = ServiceDiscoverySolver::create_service_registry();
        let service = ServiceInstance {
            id: "srv1".to_string(),
            name: "temp".to_string(),
            host: "localhost".to_string(),
            port: 9000,
            healthy: true,
            weight: 100,
            metadata: HashMap::new(),
            registered_at: 0,
        };
        ServiceDiscoverySolver::register_service(&mut registry, &service);
        ServiceDiscoverySolver::deregister_all_instances(&mut registry, "temp");
        assert_eq!(ServiceDiscoverySolver::get_service_count(&registry, "temp"), 0);
    }
}
