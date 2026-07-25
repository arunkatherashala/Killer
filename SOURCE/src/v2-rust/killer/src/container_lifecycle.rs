// Phase 20: Isolation Architecture - Container Lifecycle Manager
// Manages container execution and isolation

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Created,    // Container created but not started
    Running,    // Container is actively running
    Paused,     // Container is paused
    Exited,     // Container finished execution
    Killed,     // Container was forcefully killed
    Error,      // Container errored out
}

impl ContainerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerState::Created => "created",
            ContainerState::Running => "running",
            ContainerState::Paused => "paused",
            ContainerState::Exited => "exited",
            ContainerState::Killed => "killed",
            ContainerState::Error => "error",
        }
    }
}

/// Container configuration
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub entry_point: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub working_dir: String,
}

/// Container instance
#[derive(Debug, Clone)]
pub struct Container {
    pub config: ContainerConfig,
    pub state: ContainerState,
    pub pid: u32,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub exited_at: Option<u64>,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

impl Container {
    pub fn new(config: ContainerConfig) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Container {
            config,
            state: ContainerState::Created,
            pid: 0,
            created_at: ts,
            started_at: None,
            exited_at: None,
            restart_count: 0,
            last_error: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == ContainerState::Running
    }

    pub fn get_uptime(&self) -> Option<u64> {
        match (self.started_at, self.exited_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                Some(now - start)
            },
            _ => None,
        }
    }
}

/// Container lifecycle manager
pub struct ContainerLifecycleManager {
    containers: HashMap<String, Container>,
    max_containers: usize,
    auto_restart: bool,
}

impl ContainerLifecycleManager {
    pub fn new(max_containers: usize) -> Self {
        ContainerLifecycleManager {
            containers: HashMap::new(),
            max_containers,
            auto_restart: false,
        }
    }

    /// Create a new container from config
    pub fn create_container(&mut self, config: ContainerConfig) -> Result<String, String> {
        if self.containers.len() >= self.max_containers {
            return Err("Maximum container limit reached".to_string());
        }

        let name = config.name.clone();
        if self.containers.contains_key(&name) {
            return Err(format!("Container '{}' already exists", name));
        }

        let container = Container::new(config);
        self.containers.insert(name.clone(), container);
        Ok(name)
    }

    /// Start a container
    pub fn start_container(&mut self, name: &str) -> Result<u32, String> {
        match self.containers.get_mut(name) {
            Some(container) => {
                if container.state != ContainerState::Created && container.state != ContainerState::Exited {
                    return Err(format!("Cannot start container in {:?} state", container.state));
                }

                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                container.pid = std::process::id();
                container.state = ContainerState::Running;
                container.started_at = Some(ts);
                container.restart_count += 1;

                Ok(container.pid)
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Pause a running container
    pub fn pause_container(&mut self, name: &str) -> Result<(), String> {
        match self.containers.get_mut(name) {
            Some(container) => {
                if container.state != ContainerState::Running {
                    return Err("Container is not running".to_string());
                }
                container.state = ContainerState::Paused;
                Ok(())
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Resume a paused container
    pub fn resume_container(&mut self, name: &str) -> Result<(), String> {
        match self.containers.get_mut(name) {
            Some(container) => {
                if container.state != ContainerState::Paused {
                    return Err("Container is not paused".to_string());
                }
                container.state = ContainerState::Running;
                Ok(())
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Stop a container gracefully
    pub fn stop_container(&mut self, name: &str) -> Result<(), String> {
        match self.containers.get_mut(name) {
            Some(container) => {
                if !container.is_running() && container.state != ContainerState::Paused {
                    return Err("Container is not running".to_string());
                }

                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                container.state = ContainerState::Exited;
                container.exited_at = Some(ts);
                Ok(())
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Kill a container forcefully
    pub fn kill_container(&mut self, name: &str) -> Result<(), String> {
        match self.containers.get_mut(name) {
            Some(container) => {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                container.state = ContainerState::Killed;
                container.exited_at = Some(ts);
                Ok(())
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Remove a container
    pub fn remove_container(&mut self, name: &str) -> Result<(), String> {
        match self.containers.get(name) {
            Some(container) => {
                if container.is_running() {
                    return Err("Cannot remove running container".to_string());
                }
                self.containers.remove(name);
                Ok(())
            },
            None => Err(format!("Container '{}' not found", name)),
        }
    }

    /// Get container status
    pub fn get_status(&self, name: &str) -> Option<ContainerStatus> {
        self.containers.get(name).map(|c| ContainerStatus {
            name: c.config.name.clone(),
            state: c.state,
            pid: c.pid,
            uptime: c.get_uptime(),
            restart_count: c.restart_count,
        })
    }

    /// List all containers
    pub fn list_containers(&self) -> Vec<ContainerStatus> {
        self.containers.values()
            .map(|c| ContainerStatus {
                name: c.config.name.clone(),
                state: c.state,
                pid: c.pid,
                uptime: c.get_uptime(),
                restart_count: c.restart_count,
            })
            .collect()
    }

    /// Get runtime statistics
    pub fn get_stats(&self) -> LifecycleStats {
        let total = self.containers.len();
        let running = self.containers.values().filter(|c| c.is_running()).count();
        let paused = self.containers.values().filter(|c| c.state == ContainerState::Paused).count();
        let exited = self.containers.values().filter(|c| c.state == ContainerState::Exited).count();

        LifecycleStats {
            total_containers: total,
            running,
            paused,
            exited,
            killed: total - running - paused - exited,
        }
    }

    /// Print lifecycle report
    pub fn print_report(&self) {
        println!("\n=== Container Lifecycle Report (Phase 20) ===");
        println!("Max Containers: {}", self.max_containers);
        println!("Auto Restart: {}", self.auto_restart);

        let stats = self.get_stats();
        println!("\nContainers:");
        println!("  Total: {}", stats.total_containers);
        println!("  Running: {}", stats.running);
        println!("  Paused: {}", stats.paused);
        println!("  Exited: {}", stats.exited);
        println!("  Killed: {}", stats.killed);

        if !self.containers.is_empty() {
            println!("\nContainer List:");
            for status in self.list_containers() {
                println!("  {}: {} (PID: {}, Restarts: {})", 
                    status.name, status.state.as_str(), status.pid, status.restart_count);
                if let Some(uptime) = status.uptime {
                    println!("    Uptime: {}s", uptime);
                }
            }
        }
    }
}

/// Container status information
#[derive(Debug)]
pub struct ContainerStatus {
    pub name: String,
    pub state: ContainerState,
    pub pid: u32,
    pub uptime: Option<u64>,
    pub restart_count: u32,
}

/// Lifecycle statistics
#[derive(Debug)]
pub struct LifecycleStats {
    pub total_containers: usize,
    pub running: usize,
    pub paused: usize,
    pub exited: usize,
    pub killed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_creation() {
        let config = ContainerConfig {
            name: "test".to_string(),
            image: "killer:latest".to_string(),
            entry_point: "/app".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            working_dir: "/".to_string(),
        };

        let container = Container::new(config);
        assert_eq!(container.state, ContainerState::Created);
    }

    #[test]
    fn test_lifecycle_manager() {
        let mut manager = ContainerLifecycleManager::new(5);

        let config = ContainerConfig {
            name: "app1".to_string(),
            image: "killer:latest".to_string(),
            entry_point: "/app".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            working_dir: "/".to_string(),
        };

        assert!(manager.create_container(config).is_ok());
        assert!(manager.start_container("app1").is_ok());
        assert!(manager.pause_container("app1").is_ok());
        assert!(manager.resume_container("app1").is_ok());
        assert!(manager.stop_container("app1").is_ok());
    }

    #[test]
    fn test_container_limits() {
        let mut manager = ContainerLifecycleManager::new(1);

        let config1 = ContainerConfig {
            name: "app1".to_string(),
            image: "killer:latest".to_string(),
            entry_point: "/app".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            working_dir: "/".to_string(),
        };

        let config2 = ContainerConfig {
            name: "app2".to_string(),
            image: "killer:latest".to_string(),
            entry_point: "/app".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            working_dir: "/".to_string(),
        };

        assert!(manager.create_container(config1).is_ok());
        assert!(manager.create_container(config2).is_err());
    }
}
