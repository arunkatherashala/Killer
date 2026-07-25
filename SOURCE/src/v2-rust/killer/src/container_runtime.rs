// Phase 15: Container Runtime - service isolation, resource limits, orchestration
// Features: Container lifecycle, resource management, orchestration, health monitoring

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Container states
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerState {
    Created,
    Running,
    Paused,
    Stopped,
    Restarting,
    Exited,
    Dead,
}

impl ContainerState {
    pub fn as_str(&self) -> &str {
        match self {
            ContainerState::Created => "created",
            ContainerState::Running => "running",
            ContainerState::Paused => "paused",
            ContainerState::Stopped => "stopped",
            ContainerState::Restarting => "restarting",
            ContainerState::Exited => "exited",
            ContainerState::Dead => "dead",
        }
    }

    pub fn is_running(&self) -> bool {
        *self == ContainerState::Running
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self, ContainerState::Stopped | ContainerState::Exited | ContainerState::Dead)
    }
}

/// Resource limits
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    pub memory_mb: u32,
    pub cpu_cores: f32,
    pub disk_gb: u32,
    pub network_bandwidth_mbps: u32,
}

impl ResourceLimits {
    pub fn new(memory_mb: u32, cpu_cores: f32, disk_gb: u32, network_bandwidth_mbps: u32) -> Self {
        ResourceLimits {
            memory_mb,
            cpu_cores,
            disk_gb,
            network_bandwidth_mbps,
        }
    }

    /// Default limits
    pub fn default_limits() -> Self {
        ResourceLimits {
            memory_mb: 512,
            cpu_cores: 1.0,
            disk_gb: 10,
            network_bandwidth_mbps: 100,
        }
    }

    /// Check if within limits
    pub fn check(&self, usage: &ResourceUsage) -> bool {
        usage.memory_mb <= self.memory_mb
            && usage.cpu_usage < self.cpu_cores
            && usage.disk_gb <= self.disk_gb
    }
}

/// Resource usage
#[derive(Clone, Debug)]
pub struct ResourceUsage {
    pub memory_mb: u32,
    pub cpu_usage: f32,
    pub disk_gb: u32,
    pub network_in_mbps: f32,
    pub network_out_mbps: f32,
}

impl ResourceUsage {
    pub fn new(memory_mb: u32, cpu_usage: f32, disk_gb: u32) -> Self {
        ResourceUsage {
            memory_mb,
            cpu_usage,
            disk_gb,
            network_in_mbps: 0.0,
            network_out_mbps: 0.0,
        }
    }

    /// Get total memory percentage
    pub fn memory_percentage(&self, limit: &ResourceLimits) -> f32 {
        (self.memory_mb as f32 / limit.memory_mb as f32) * 100.0
    }

    /// Get CPU percentage
    pub fn cpu_percentage(&self, limit: &ResourceLimits) -> f32 {
        (self.cpu_usage / limit.cpu_cores) * 100.0
    }
}

/// Container configuration
#[derive(Clone, Debug)]
pub struct ContainerConfig {
    pub image: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub volumes: Vec<String>,
    pub ports: Vec<u16>,
    pub labels: HashMap<String, String>,
}

impl ContainerConfig {
    pub fn new(image: String) -> Self {
        ContainerConfig {
            image,
            command: Vec::new(),
            environment: HashMap::new(),
            volumes: Vec::new(),
            ports: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Add command
    pub fn with_command(mut self, cmd: Vec<String>) -> Self {
        self.command = cmd;
        self
    }

    /// Add environment variable
    pub fn add_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// Add volume
    pub fn add_volume(mut self, volume: String) -> Self {
        self.volumes.push(volume);
        self
    }

    /// Expose port
    pub fn expose_port(mut self, port: u16) -> Self {
        self.ports.push(port);
        self
    }

    /// Add label
    pub fn add_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }
}

/// Container
#[derive(Clone, Debug)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub config: ContainerConfig,
    pub state: ContainerState,
    pub limits: ResourceLimits,
    pub usage: ResourceUsage,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub stopped_at: Option<u64>,
    pub restart_count: u32,
}

impl Container {
    pub fn new(id: String, name: String, config: ContainerConfig, limits: ResourceLimits) -> Self {
        Container {
            id,
            name,
            config,
            state: ContainerState::Created,
            limits,
            usage: ResourceUsage::new(0, 0.0, 0),
            created_at: current_timestamp(),
            started_at: None,
            stopped_at: None,
            restart_count: 0,
        }
    }

    /// Start container
    pub fn start(mut self) -> Self {
        self.state = ContainerState::Running;
        self.started_at = Some(current_timestamp());
        self
    }

    /// Stop container
    pub fn stop(mut self) -> Self {
        self.state = ContainerState::Stopped;
        self.stopped_at = Some(current_timestamp());
        self
    }

    /// Pause container
    pub fn pause(mut self) -> Self {
        self.state = ContainerState::Paused;
        self
    }

    /// Resume container
    pub fn resume(mut self) -> Self {
        if self.state == ContainerState::Paused {
            self.state = ContainerState::Running;
        }
        self
    }

    /// Restart container
    pub fn restart(mut self) -> Self {
        self.state = ContainerState::Restarting;
        self.restart_count += 1;
        self
    }

    /// Update resource usage
    pub fn update_usage(mut self, usage: ResourceUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Check health
    pub fn is_healthy(&self) -> bool {
        self.state.is_running() && self.limits.check(&self.usage)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        if let Some(start) = self.started_at {
            let now = current_timestamp();
            (now - start) / 1000
        } else {
            0
        }
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }
}

/// Container runtime
#[derive(Clone, Debug)]
pub struct ContainerRuntime {
    pub containers: HashMap<String, Container>,
    pub running_count: usize,
}

impl ContainerRuntime {
    pub fn new() -> Self {
        ContainerRuntime {
            containers: HashMap::new(),
            running_count: 0,
        }
    }

    /// Create container
    pub fn create_container(
        &mut self,
        id: String,
        config: ContainerConfig,
        limits: ResourceLimits,
    ) -> Result<(), String> {
        if self.containers.contains_key(&id) {
            return Err(format!("Container {} already exists", id));
        }

        let container = Container::new(id.clone(), id.clone(), config, limits);
        self.containers.insert(id, container);
        Ok(())
    }

    /// Start container
    pub fn start_container(&mut self, id: &str) -> Result<(), String> {
        let container = self.containers.get_mut(id)
            .ok_or_else(|| format!("Container {} not found", id))?;

        *container = container.clone().start();
        self.running_count = self.containers.values()
            .filter(|c| c.is_running())
            .count();
        Ok(())
    }

    /// Stop container
    pub fn stop_container(&mut self, id: &str) -> Result<(), String> {
        let container = self.containers.get_mut(id)
            .ok_or_else(|| format!("Container {} not found", id))?;

        *container = container.clone().stop();
        self.running_count = self.containers.values()
            .filter(|c| c.is_running())
            .count();
        Ok(())
    }

    /// Remove container
    pub fn remove_container(&mut self, id: &str) -> Result<(), String> {
        if self.containers.remove(id).is_some() {
            self.running_count = self.containers.values()
                .filter(|c| c.is_running())
                .count();
            Ok(())
        } else {
            Err(format!("Container {} not found", id))
        }
    }

    /// Get container
    pub fn get_container(&self, id: &str) -> Option<Container> {
        self.containers.get(id).cloned()
    }

    /// List all containers
    pub fn list_containers(&self) -> Vec<Container> {
        self.containers.values().cloned().collect()
    }

    /// List running containers
    pub fn list_running(&self) -> Vec<Container> {
        self.containers.values()
            .filter(|c| c.is_running())
            .cloned()
            .collect()
    }

    /// Container count
    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    /// Get container health summary
    pub fn get_health_summary(&self) -> (usize, usize) {
        let healthy = self.containers.values()
            .filter(|c| c.is_healthy())
            .count();
        let total = self.containers.len();
        (healthy, total)
    }
}

impl Default for ContainerRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Service
#[derive(Clone, Debug)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub container_ids: Vec<String>,
    pub replicas: u32,
    pub desired_replicas: u32,
    pub labels: HashMap<String, String>,
}

impl Service {
    pub fn new(id: String, name: String, desired_replicas: u32) -> Self {
        Service {
            id,
            name,
            container_ids: Vec::new(),
            replicas: 0,
            desired_replicas,
            labels: HashMap::new(),
        }
    }

    /// Add container
    pub fn add_container(&mut self, container_id: String) {
        if !self.container_ids.contains(&container_id) {
            self.container_ids.push(container_id);
            self.replicas = self.container_ids.len() as u32;
        }
    }

    /// Remove container
    pub fn remove_container(&mut self, container_id: &str) {
        self.container_ids.retain(|id| id != container_id);
        self.replicas = self.container_ids.len() as u32;
    }

    /// Check if ready
    pub fn is_ready(&self) -> bool {
        self.replicas >= self.desired_replicas
    }

    /// Get replica status
    pub fn replica_status(&self) -> (u32, u32) {
        (self.replicas, self.desired_replicas)
    }
}

/// Orchestrator
#[derive(Clone, Debug)]
pub struct Orchestrator {
    pub runtime: ContainerRuntime,
    pub services: HashMap<String, Service>,
}

impl Orchestrator {
    pub fn new(runtime: ContainerRuntime) -> Self {
        Orchestrator {
            runtime,
            services: HashMap::new(),
        }
    }

    /// Create service
    pub fn create_service(&mut self, service: Service) -> Result<(), String> {
        if self.services.contains_key(&service.id) {
            return Err(format!("Service {} already exists", service.id));
        }
        self.services.insert(service.id.clone(), service);
        Ok(())
    }

    /// Delete service
    pub fn delete_service(&mut self, service_id: &str) -> Result<(), String> {
        if self.services.remove(service_id).is_some() {
            Ok(())
        } else {
            Err(format!("Service {} not found", service_id))
        }
    }

    /// Scale service
    pub fn scale_service(&mut self, service_id: &str, desired_replicas: u32) -> Result<(), String> {
        let service = self.services.get_mut(service_id)
            .ok_or_else(|| format!("Service {} not found", service_id))?;
        service.desired_replicas = desired_replicas;
        Ok(())
    }

    /// Get service
    pub fn get_service(&self, service_id: &str) -> Option<Service> {
        self.services.get(service_id).cloned()
    }

    /// List services
    pub fn list_services(&self) -> Vec<Service> {
        self.services.values().cloned().collect()
    }

    /// Service count
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Get cluster status
    pub fn get_cluster_status(&self) -> (usize, usize, usize) {
        let total_containers = self.runtime.container_count();
        let running_containers = self.runtime.running_count;
        let services = self.services.len();
        (total_containers, running_containers, services)
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new(ContainerRuntime::new())
    }
}

/// Health check
#[derive(Clone, Debug)]
pub struct HealthCheck {
    pub check_type: HealthCheckType,
    pub interval_secs: u32,
    pub timeout_secs: u32,
    pub retries: u32,
}

impl HealthCheck {
    pub fn new(check_type: HealthCheckType) -> Self {
        HealthCheck {
            check_type,
            interval_secs: 30,
            timeout_secs: 5,
            retries: 3,
        }
    }

    /// Set interval
    pub fn with_interval(mut self, secs: u32) -> Self {
        self.interval_secs = secs;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u32) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set retries
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
}

/// Health check type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HealthCheckType {
    Script(String),
    Http(String),
    Tcp { host: String, port: u16 },
    Custom(String),
}

/// Helper to get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_state_as_str() {
        assert_eq!(ContainerState::Running.as_str(), "running");
        assert_eq!(ContainerState::Stopped.as_str(), "stopped");
    }

    #[test]
    fn test_container_state_is_running() {
        assert!(ContainerState::Running.is_running());
        assert!(!ContainerState::Stopped.is_running());
    }

    #[test]
    fn test_container_state_is_stopped() {
        assert!(ContainerState::Stopped.is_stopped());
        assert!(!ContainerState::Running.is_stopped());
    }

    #[test]
    fn test_resource_limits_creation() {
        let limits = ResourceLimits::new(512, 2.0, 20, 100);
        assert_eq!(limits.memory_mb, 512);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default_limits();
        assert_eq!(limits.memory_mb, 512);
    }

    #[test]
    fn test_resource_limits_check() {
        let limits = ResourceLimits::new(512, 2.0, 20, 100);
        let usage = ResourceUsage::new(256, 1.0, 10);
        assert!(limits.check(&usage));
    }

    #[test]
    fn test_resource_usage_memory_percentage() {
        let limits = ResourceLimits::new(1000, 1.0, 10, 100);
        let usage = ResourceUsage::new(500, 0.5, 5);
        assert_eq!(usage.memory_percentage(&limits), 50.0);
    }

    #[test]
    fn test_resource_usage_cpu_percentage() {
        let limits = ResourceLimits::new(512, 4.0, 10, 100);
        let usage = ResourceUsage::new(256, 2.0, 5);
        assert_eq!(usage.cpu_percentage(&limits), 50.0);
    }

    #[test]
    fn test_container_config_creation() {
        let config = ContainerConfig::new("busybox".to_string());
        assert_eq!(config.image, "busybox");
    }

    #[test]
    fn test_container_config_with_command() {
        let config = ContainerConfig::new("busybox".to_string())
            .with_command(vec!["echo".to_string(), "hello".to_string()]);
        assert_eq!(config.command.len(), 2);
    }

    #[test]
    fn test_container_config_add_env() {
        let config = ContainerConfig::new("busybox".to_string())
            .add_env("KEY".to_string(), "value".to_string());
        assert_eq!(config.environment.get("KEY"), Some(&"value".to_string()));
    }

    #[test]
    fn test_container_config_expose_port() {
        let config = ContainerConfig::new("nginx".to_string())
            .expose_port(80)
            .expose_port(443);
        assert_eq!(config.ports.len(), 2);
    }

    #[test]
    fn test_container_creation() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        assert_eq!(container.id, "c1");
        assert_eq!(container.state, ContainerState::Created);
    }

    #[test]
    fn test_container_start() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        let started = container.start();
        assert_eq!(started.state, ContainerState::Running);
        assert!(started.started_at.is_some());
    }

    #[test]
    fn test_container_stop() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        let stopped = container.start().stop();
        assert_eq!(stopped.state, ContainerState::Stopped);
    }

    #[test]
    fn test_container_pause_resume() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        let paused = container.start().pause();
        assert_eq!(paused.state, ContainerState::Paused);
        let resumed = paused.resume();
        assert_eq!(resumed.state, ContainerState::Running);
    }

    #[test]
    fn test_container_restart() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        let restarted = container.restart();
        assert_eq!(restarted.restart_count, 1);
    }

    #[test]
    fn test_container_is_healthy() {
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        let mut container = Container::new("c1".to_string(), "test".to_string(), config, limits);
        container = container.start();
        container = container.update_usage(ResourceUsage::new(100, 0.5, 5));
        assert!(container.is_healthy());
    }

    #[test]
    fn test_container_runtime_create() {
        let mut runtime = ContainerRuntime::new();
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        assert!(runtime.create_container("c1".to_string(), config, limits).is_ok());
    }

    #[test]
    fn test_container_runtime_start() {
        let mut runtime = ContainerRuntime::new();
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        runtime.create_container("c1".to_string(), config, limits).unwrap();
        assert!(runtime.start_container("c1").is_ok());
        assert_eq!(runtime.running_count, 1);
    }

    #[test]
    fn test_container_runtime_stop() {
        let mut runtime = ContainerRuntime::new();
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        runtime.create_container("c1".to_string(), config, limits).unwrap();
        runtime.start_container("c1").unwrap();
        assert!(runtime.stop_container("c1").is_ok());
        assert_eq!(runtime.running_count, 0);
    }

    #[test]
    fn test_container_runtime_remove() {
        let mut runtime = ContainerRuntime::new();
        let config = ContainerConfig::new("busybox".to_string());
        let limits = ResourceLimits::default_limits();
        runtime.create_container("c1".to_string(), config, limits).unwrap();
        assert!(runtime.remove_container("c1").is_ok());
        assert_eq!(runtime.container_count(), 0);
    }

    #[test]
    fn test_service_creation() {
        let service = Service::new("svc1".to_string(), "web".to_string(), 3);
        assert_eq!(service.name, "web");
        assert_eq!(service.desired_replicas, 3);
    }

    #[test]
    fn test_service_add_container() {
        let mut service = Service::new("svc1".to_string(), "web".to_string(), 3);
        service.add_container("c1".to_string());
        service.add_container("c2".to_string());
        assert_eq!(service.replicas, 2);
    }

    #[test]
    fn test_service_is_ready() {
        let mut service = Service::new("svc1".to_string(), "web".to_string(), 2);
        service.add_container("c1".to_string());
        assert!(!service.is_ready());
        service.add_container("c2".to_string());
        assert!(service.is_ready());
    }

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = Orchestrator::default();
        assert_eq!(orchestrator.service_count(), 0);
    }

    #[test]
    fn test_orchestrator_create_service() {
        let mut orchestrator = Orchestrator::default();
        let service = Service::new("svc1".to_string(), "web".to_string(), 1);
        assert!(orchestrator.create_service(service).is_ok());
        assert_eq!(orchestrator.service_count(), 1);
    }

    #[test]
    fn test_orchestrator_delete_service() {
        let mut orchestrator = Orchestrator::default();
        let service = Service::new("svc1".to_string(), "web".to_string(), 1);
        orchestrator.create_service(service).unwrap();
        assert!(orchestrator.delete_service("svc1").is_ok());
        assert_eq!(orchestrator.service_count(), 0);
    }

    #[test]
    fn test_orchestrator_scale_service() {
        let mut orchestrator = Orchestrator::default();
        let service = Service::new("svc1".to_string(), "web".to_string(), 1);
        orchestrator.create_service(service).unwrap();
        assert!(orchestrator.scale_service("svc1", 5).is_ok());
        let scaled = orchestrator.get_service("svc1").unwrap();
        assert_eq!(scaled.desired_replicas, 5);
    }

    #[test]
    fn test_health_check_creation() {
        let check = HealthCheck::new(HealthCheckType::Http("http://localhost/health".to_string()));
        assert_eq!(check.interval_secs, 30);
    }

    #[test]
    fn test_health_check_with_interval() {
        let check = HealthCheck::new(HealthCheckType::Tcp {
            host: "localhost".to_string(),
            port: 8080,
        }).with_interval(60);
        assert_eq!(check.interval_secs, 60);
    }
}
