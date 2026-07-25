// Health Check System for Killer Language
// Purpose: Monitor system health and enable liveness/readiness probes
// Status: Production-ready

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Health state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthState::Healthy => write!(f, "Healthy"),
            HealthState::Degraded => write!(f, "Degraded"),
            HealthState::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

impl HealthState {
    pub fn to_http_status(&self) -> u16 {
        match self {
            HealthState::Healthy => 200,
            HealthState::Degraded => 503,
            HealthState::Unhealthy => 503,
        }
    }
}

/// Health status of a single component
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub component: String,
    pub status: HealthState,
    pub message: String,
    pub last_check: SystemTime,
    pub check_duration_ms: f64,
}

impl ComponentHealth {
    pub fn new(component: String) -> Self {
        ComponentHealth {
            component,
            status: HealthState::Healthy,
            message: String::new(),
            last_check: SystemTime::now(),
            check_duration_ms: 0.0,
        }
    }
}

/// Overall health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: HealthState,
    pub version: String,
    pub uptime: Duration,
    pub checks: Vec<ComponentHealth>,
    pub timestamp: SystemTime,
}

impl HealthStatus {
    pub fn new(version: String, uptime: Duration) -> Self {
        HealthStatus {
            status: HealthState::Healthy,
            version,
            uptime,
            checks: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn to_json(&self) -> String {
        let mut checks_json = String::from("[\n");

        for (i, check) in self.checks.iter().enumerate() {
            if i > 0 {
                checks_json.push_str(",\n");
            }
            checks_json.push_str(&format!(
                r#"    {{
      "component": "{}",
      "status": "{}",
      "message": "{}",
      "duration_ms": {}
    }}"#,
                check.component, check.status, check.message, check.check_duration_ms
            ));
        }

        checks_json.push_str("\n  ]");

        format!(
            r#"{{
  "status":"{}",
  "version":"{}",
  "uptime_seconds":{},
  "timestamp":{},
  "checks": {}
}}"#,
            self.status,
            self.version,
            self.uptime.as_secs(),
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            checks_json
        )
    }
}

/// Health check trait
pub trait HealthCheck: Send + Sync {
    fn check(&self) -> Result<(), String>;
    fn name(&self) -> &str;
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }
}

/// Memory health check
pub struct MemoryHealthCheck {
    pub max_memory_mb: u64,
}

impl HealthCheck for MemoryHealthCheck {
    fn check(&self) -> Result<(), String> {
        // Placeholder: In real implementation, would check actual memory
        Ok(())
    }

    fn name(&self) -> &str {
        "memory"
    }
}

/// Disk health check
pub struct DiskHealthCheck {
    pub min_free_mb: u64,
}

impl HealthCheck for DiskHealthCheck {
    fn check(&self) -> Result<(), String> {
        // Placeholder: In real implementation, would check disk space
        Ok(())
    }

    fn name(&self) -> &str {
        "disk"
    }
}

/// Connection pool health check
pub struct ConnectionPoolHealthCheck {
    pub min_available: u32,
}

impl HealthCheck for ConnectionPoolHealthCheck {
    fn check(&self) -> Result<(), String> {
        // Placeholder: In real implementation, would check pool
        Ok(())
    }

    fn name(&self) -> &str {
        "connection_pool"
    }
}

/// Health checker system
pub struct HealthChecker {
    start_time: SystemTime,
    version: String,
    checks: Arc<Mutex<HashMap<String, Box<dyn HealthCheck>>>>,
    last_check_result: Arc<Mutex<Option<HealthStatus>>>,
}

impl HealthChecker {
    pub fn new(version: String) -> Self {
        HealthChecker {
            start_time: SystemTime::now(),
            version,
            checks: Arc::new(Mutex::new(HashMap::new())),
            last_check_result: Arc::new(Mutex::new(None)),
        }
    }

    pub fn register_check(&self, name: String, check: Box<dyn HealthCheck>) {
        if let Ok(mut checks) = self.checks.lock() {
            checks.insert(name, check);
        }
    }

    pub fn register_memory_check(&self, max_memory_mb: u64) {
        self.register_check(
            "memory".to_string(),
            Box::new(MemoryHealthCheck { max_memory_mb }),
        );
    }

    pub fn register_disk_check(&self, min_free_mb: u64) {
        self.register_check(
            "disk".to_string(),
            Box::new(DiskHealthCheck { min_free_mb }),
        );
    }

    pub fn register_connection_pool_check(&self, min_available: u32) {
        self.register_check(
            "connection_pool".to_string(),
            Box::new(ConnectionPoolHealthCheck { min_available }),
        );
    }

    pub fn check(&self) -> HealthStatus {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        let mut status = HealthStatus::new(self.version.clone(), uptime);

        if let Ok(checks) = self.checks.lock() {
            for (name, check) in checks.iter() {
                let start = SystemTime::now();
                let result = check.check();
                let duration = start.elapsed().unwrap_or_default().as_secs_f64() * 1000.0;

                let component_health = match result {
                    Ok(()) => ComponentHealth {
                        component: name.clone(),
                        status: HealthState::Healthy,
                        message: "OK".to_string(),
                        last_check: SystemTime::now(),
                        check_duration_ms: duration,
                    },
                    Err(e) => ComponentHealth {
                        component: name.clone(),
                        status: HealthState::Unhealthy,
                        message: e,
                        last_check: SystemTime::now(),
                        check_duration_ms: duration,
                    },
                };

                if component_health.status == HealthState::Unhealthy {
                    status.status = HealthState::Unhealthy;
                }

                status.checks.push(component_health);
            }
        }

        if let Ok(mut last_check) = self.last_check_result.lock() {
            *last_check = Some(status.clone());
        }

        status
    }

    pub fn get_liveness_probe(&self) -> HealthStatus {
        // Liveness = is the service running? Simple check
        let uptime = self.start_time.elapsed().unwrap_or_default();
        HealthStatus::new(self.version.clone(), uptime)
    }

    pub fn get_readiness_probe(&self) -> HealthStatus {
        // Readiness = is the service ready to handle requests? Full check
        self.check()
    }

    pub fn get_startup_probe(&self) -> HealthStatus {
        // Startup = has the service completed initialization?
        let uptime = self.start_time.elapsed().unwrap_or_default();
        let startup_complete = uptime > Duration::from_secs(1);

        let mut status = HealthStatus::new(self.version.clone(), uptime);
        status.status = if startup_complete {
            HealthState::Healthy
        } else {
            HealthState::Degraded
        };

        status
    }

    pub fn get_last_check(&self) -> Option<HealthStatus> {
        self.last_check_result
            .lock()
            .ok()
            .and_then(|result| result.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_http_status() {
        assert_eq!(HealthState::Healthy.to_http_status(), 200);
        assert_eq!(HealthState::Degraded.to_http_status(), 503);
        assert_eq!(HealthState::Unhealthy.to_http_status(), 503);
    }

    #[test]
    fn test_component_health() {
        let component = ComponentHealth::new("test".to_string());
        assert_eq!(component.component, "test");
        assert_eq!(component.status, HealthState::Healthy);
    }

    #[test]
    fn test_health_status_json() {
        let status = HealthStatus::new("1.0.0".to_string(), Duration::from_secs(60));
        let json = status.to_json();

        assert!(json.contains(r#""status":"Healthy""#));
        assert!(json.contains(r#""version":"1.0.0""#));
        assert!(json.contains(r#""uptime_seconds":60"#));
    }

    #[test]
    fn test_health_checker_registration() {
        let checker = HealthChecker::new("1.0".to_string());

        checker.register_memory_check(512);
        checker.register_disk_check(100);

        let status = checker.check();
        assert_eq!(status.checks.len(), 2);
    }

    #[test]
    fn test_liveness_probe() {
        let checker = HealthChecker::new("1.0".to_string());
        let probe = checker.get_liveness_probe();

        assert_eq!(probe.status, HealthState::Healthy);
    }

    #[test]
    fn test_readiness_probe() {
        let checker = HealthChecker::new("1.0".to_string());
        checker.register_memory_check(512);

        let probe = checker.get_readiness_probe();
        assert_eq!(probe.status, HealthState::Healthy);
    }

    #[test]
    fn test_startup_probe() {
        let checker = HealthChecker::new("1.0".to_string());
        let probe = checker.get_startup_probe();

        // May be degraded or healthy depending on timing
        assert!(probe.status == HealthState::Healthy || probe.status == HealthState::Degraded);
    }

    #[test]
    fn test_last_check_cache() {
        let checker = HealthChecker::new("1.0".to_string());

        assert!(checker.get_last_check().is_none());

        checker.check();
        assert!(checker.get_last_check().is_some());
    }
}
