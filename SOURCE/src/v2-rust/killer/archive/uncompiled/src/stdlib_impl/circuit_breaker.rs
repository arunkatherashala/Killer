// ================================================================
// CIRCUIT BREAKER - Phase 27.3
// Circuit breaker pattern for resilience
// ================================================================

use std::collections::HashMap;

/// Circuit breaker state
#[derive(Clone, Debug, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker config
#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
    pub half_open_max_requests: u32,
}

/// Circuit breaker status
#[derive(Clone, Debug)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: u64,
    pub total_requests: u32,
    pub total_failures: u32,
}

pub struct CircuitBreakerSolver;

impl CircuitBreakerSolver {
    // ================================================================
    // STATE MANAGEMENT (1-12)
    // ================================================================

    /// Problem 1: Create circuit breaker
    pub fn create_circuit_breaker(config: &CircuitBreakerConfig) -> CircuitBreakerStatus {
        CircuitBreakerStatus {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: 0,
            total_requests: 0,
            total_failures: 0,
        }
    }

    /// Problem 2: Get current state
    pub fn get_current_state(cb: &CircuitBreakerStatus) -> CircuitState {
        cb.state.clone()
    }

    /// Problem 3: Transition to open
    pub fn transition_to_open(cb: &mut CircuitBreakerStatus, now: u64) {
        cb.state = CircuitState::Open;
        cb.failure_count = 0;
        cb.success_count = 0;
        cb.last_failure_time = now;
    }

    /// Problem 4: Transition to half open
    pub fn transition_to_half_open(cb: &mut CircuitBreakerStatus) {
        cb.state = CircuitState::HalfOpen;
        cb.failure_count = 0;
        cb.success_count = 0;
    }

    /// Problem 5: Transition to closed
    pub fn transition_to_closed(cb: &mut CircuitBreakerStatus) {
        cb.state = CircuitState::Closed;
        cb.failure_count = 0;
        cb.success_count = 0;
    }

    /// Problem 6: Is circuit open
    pub fn is_circuit_open(cb: &CircuitBreakerStatus) -> bool {
        cb.state == CircuitState::Open
    }

    /// Problem 7: Is circuit half open
    pub fn is_circuit_half_open(cb: &CircuitBreakerStatus) -> bool {
        cb.state == CircuitState::HalfOpen
    }

    /// Problem 8: Is circuit closed
    pub fn is_circuit_closed(cb: &CircuitBreakerStatus) -> bool {
        cb.state == CircuitState::Closed
    }

    /// Problem 9: Can execute request
    pub fn can_execute_request(cb: &CircuitBreakerStatus) -> bool {
        cb.state == CircuitState::Closed || cb.state == CircuitState::HalfOpen
    }

    /// Problem 10: Should trip circuit
    pub fn should_trip_circuit(
        cb: &CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
    ) -> bool {
        cb.failure_count >= config.failure_threshold
    }

    /// Problem 11: Should allow half open
    pub fn should_allow_half_open(
        cb: &CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
        now: u64,
    ) -> bool {
        if cb.state != CircuitState::Open {
            return false;
        }
        let time_since_failure = now - cb.last_failure_time;
        time_since_failure >= config.timeout_seconds
    }

    /// Problem 12: Get state duration
    pub fn get_state_duration(cb: &CircuitBreakerStatus, now: u64) -> u64 {
        now - cb.last_failure_time
    }

    // ================================================================
    // FAILURE DETECTION (13-22)
    // ================================================================

    /// Problem 13: Record failure
    pub fn record_failure(
        cb: &mut CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
        now: u64,
    ) {
        cb.failure_count += 1;
        cb.total_failures += 1;
        cb.total_requests += 1;
        cb.last_failure_time = now;

        if Self::should_trip_circuit(cb, config) {
            Self::transition_to_open(cb, now);
        }
    }

    /// Problem 14: Record success
    pub fn record_success(
        cb: &mut CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
    ) {
        cb.success_count += 1;
        cb.total_requests += 1;

        if cb.state == CircuitState::HalfOpen {
            if cb.success_count >= config.success_threshold {
                Self::transition_to_closed(cb);
            }
        }
    }

    /// Problem 15: Get failure count
    pub fn get_failure_count(cb: &CircuitBreakerStatus) -> u32 {
        cb.failure_count
    }

    /// Problem 16: Get success count
    pub fn get_success_count(cb: &CircuitBreakerStatus) -> u32 {
        cb.success_count
    }

    /// Problem 17: Reset failure count
    pub fn reset_failure_count(cb: &mut CircuitBreakerStatus) {
        cb.failure_count = 0;
    }

    /// Problem 18: Get failure rate
    pub fn get_failure_rate(cb: &CircuitBreakerStatus) -> f64 {
        if cb.total_requests == 0 {
            0.0
        } else {
            (cb.total_failures as f64 / cb.total_requests as f64) * 100.0
        }
    }

    /// Problem 19: Check failure threshold exceeded
    pub fn check_failure_threshold_exceeded(
        cb: &CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
    ) -> bool {
        cb.failure_count >= config.failure_threshold
    }

    /// Problem 20: Get last failure time
    pub fn get_last_failure_time(cb: &CircuitBreakerStatus) -> u64 {
        cb.last_failure_time
    }

    /// Problem 21: Is consecutive failures
    pub fn is_consecutive_failures(cb: &CircuitBreakerStatus, consecutive: u32) -> bool {
        cb.failure_count >= consecutive
    }

    /// Problem 22: Get failure percentage
    pub fn get_failure_percentage(cb: &CircuitBreakerStatus) -> u32 {
        if cb.total_requests == 0 {
            0
        } else {
            ((cb.total_failures as f64 / cb.total_requests as f64) * 100.0) as u32
        }
    }

    // ================================================================
    // RECOVERY STRATEGIES (23-32)
    // ================================================================

    /// Problem 23: Try reset on timeout
    pub fn try_reset_on_timeout(
        cb: &mut CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
        now: u64,
    ) -> bool {
        if Self::should_allow_half_open(cb, config, now) {
            Self::transition_to_half_open(cb);
            true
        } else {
            false
        }
    }

    /// Problem 24: Execute with circuit breaker
    pub fn execute_with_circuit_breaker(
        cb: &CircuitBreakerStatus,
    ) -> Result<(), String> {
        if Self::is_circuit_open(cb) {
            Err("Circuit breaker is open".to_string())
        } else {
            Ok(())
        }
    }

    /// Problem 25: Automatic recovery
    pub fn automatic_recovery(
        cb: &mut CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
        now: u64,
    ) {
        if Self::should_allow_half_open(cb, config, now) {
            Self::transition_to_half_open(cb);
        }
    }

    /// Problem 26: Gradual recovery
    pub fn gradual_recovery(
        cb: &mut CircuitBreakerStatus,
        config: &CircuitBreakerConfig,
        successes_so_far: u32,
    ) -> bool {
        if cb.state == CircuitState::HalfOpen {
            successes_so_far >= (config.success_threshold / 2)
        } else {
            false
        }
    }

    /// Problem 27: Fast fail on open
    pub fn fast_fail_on_open(cb: &CircuitBreakerStatus) -> bool {
        Self::is_circuit_open(cb)
    }

    /// Problem 28: Bulkhead isolation
    pub fn bulkhead_isolation(
        circuit_breakers: &HashMap<String, CircuitBreakerStatus>,
        service_name: &str,
    ) -> bool {
        circuit_breakers
            .get(service_name)
            .map(|cb| !Self::is_circuit_open(cb))
            .unwrap_or(true)
    }

    /// Problem 29: Set recovery timeout
    pub fn set_recovery_timeout(config: &mut CircuitBreakerConfig, timeout_seconds: u64) {
        config.timeout_seconds = timeout_seconds;
    }

    /// Problem 30: Exponential backoff recovery
    pub fn exponential_backoff_recovery(
        cb: &CircuitBreakerStatus,
        attempt: u32,
        base_timeout: u64,
    ) -> u64 {
        if attempt == 0 {
            base_timeout
        } else {
            base_timeout * 2_u64.pow(attempt)
        }
    }

    /// Problem 31: Get recovery time estimate
    pub fn get_recovery_time_estimate(
        cb: &CircuitBreakerStatus,
        timeouts_triggered: u32,
    ) -> u64 {
        Self::exponential_backoff_recovery(cb, timeouts_triggered, 1)
    }

    /// Problem 32: Cool down period
    pub fn cool_down_period(
        cb: &CircuitBreakerStatus,
        now: u64,
        cool_down_seconds: u64,
    ) -> bool {
        let time_since_failure = now - cb.last_failure_time;
        time_since_failure >= cool_down_seconds
    }

    // ================================================================
    // MULTI-CIRCUIT MANAGEMENT (33-42)
    // ================================================================

    /// Problem 33: Create circuits for services
    pub fn create_circuits_for_services(
        services: &[String],
        config: &CircuitBreakerConfig,
    ) -> HashMap<String, CircuitBreakerStatus> {
        let mut circuits = HashMap::new();
        for service in services {
            circuits.insert(
                service.clone(),
                Self::create_circuit_breaker(config),
            );
        }
        circuits
    }

    /// Problem 34: Get circuit by service
    pub fn get_circuit_by_service(
        circuits: &HashMap<String, CircuitBreakerStatus>,
        service_name: &str,
    ) -> Option<CircuitBreakerStatus> {
        circuits.get(service_name).cloned()
    }

    /// Problem 35: Update circuit status
    pub fn update_circuit_status(
        circuits: &mut HashMap<String, CircuitBreakerStatus>,
        service_name: &str,
        status: CircuitBreakerStatus,
    ) {
        circuits.insert(service_name.to_string(), status);
    }

    /// Problem 36: Get open circuits count
    pub fn get_open_circuits_count(circuits: &HashMap<String, CircuitBreakerStatus>) -> usize {
        circuits
            .values()
            .filter(|cb| Self::is_circuit_open(cb))
            .count()
    }

    /// Problem 37: Get circuit states
    pub fn get_circuit_states(
        circuits: &HashMap<String, CircuitBreakerStatus>,
    ) -> HashMap<String, String> {
        let mut states = HashMap::new();
        for (service, cb) in circuits {
            let state_str = match cb.state {
                CircuitState::Closed => "Closed".to_string(),
                CircuitState::Open => "Open".to_string(),
                CircuitState::HalfOpen => "HalfOpen".to_string(),
            };
            states.insert(service.clone(), state_str);
        }
        states
    }

    /// Problem 38: Health check all circuits
    pub fn health_check_all_circuits(
        circuits: &HashMap<String, CircuitBreakerStatus>,
    ) -> HashMap<String, bool> {
        let mut health = HashMap::new();
        for (service, cb) in circuits {
            health.insert(service.clone(), !Self::is_circuit_open(cb));
        }
        health
    }

    /// Problem 39: Cascade failure detection
    pub fn cascade_failure_detection(
        circuits: &HashMap<String, CircuitBreakerStatus>,
        threshold_percent: u32,
    ) -> bool {
        let total = circuits.len() as u32;
        let open_count = Self::get_open_circuits_count(circuits) as u32;
        let open_percent = (open_count * 100) / total;
        open_percent >= threshold_percent
    }

    /// Problem 40: Service dependency graph
    pub fn service_dependency_graph() -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        graph.insert("api".to_string(), vec!["db".to_string(), "cache".to_string()]);
        graph.insert("db".to_string(), vec![]);
        graph.insert("cache".to_string(), vec![]);
        graph
    }

    /// Problem 41: Detect broken dependencies
    pub fn detect_broken_dependencies(
        circuits: &HashMap<String, CircuitBreakerStatus>,
        dependencies: &HashMap<String, Vec<String>>,
        service: &str,
    ) -> Vec<String> {
        dependencies
            .get(service)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|dep| {
                circuits
                    .get(dep)
                    .map(|cb| Self::is_circuit_open(cb))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Problem 42: Mitigate cascade failures
    pub fn mitigate_cascade_failures(
        circuits: &mut HashMap<String, CircuitBreakerStatus>,
        config: &CircuitBreakerConfig,
    ) {
        for (_, cb) in circuits.iter_mut() {
            if Self::is_circuit_open(cb) {
                cb.success_count = 0;
                cb.failure_count = config.failure_threshold;
            }
        }
    }

    // ================================================================
    // METRICS & MONITORING (43-50)
    // ================================================================

    /// Problem 43: Get circuit metrics
    pub fn get_circuit_metrics(cb: &CircuitBreakerStatus) -> HashMap<String, u32> {
        let mut metrics = HashMap::new();
        metrics.insert("total_requests".to_string(), cb.total_requests);
        metrics.insert("total_failures".to_string(), cb.total_failures);
        metrics.insert("current_failures".to_string(), cb.failure_count);
        metrics.insert("current_successes".to_string(), cb.success_count);
        metrics
    }

    /// Problem 44: Get average failure rate
    pub fn get_average_failure_rate(circuits: &HashMap<String, CircuitBreakerStatus>) -> f64 {
        if circuits.is_empty() {
            return 0.0;
        }
        let total_failure_rate: f64 = circuits
            .values()
            .map(Self::get_failure_rate)
            .sum();
        total_failure_rate / circuits.len() as f64
    }

    /// Problem 45: Get circuit health score
    pub fn get_circuit_health_score(cb: &CircuitBreakerStatus) -> u32 {
        match cb.state {
            CircuitState::Closed => 100,
            CircuitState::HalfOpen => 50,
            CircuitState::Open => 0,
        }
    }

    /// Problem 46: Get system health score
    pub fn get_system_health_score(circuits: &HashMap<String, CircuitBreakerStatus>) -> u32 {
        if circuits.is_empty() {
            return 100;
        }
        let total_score: u32 = circuits
            .values()
            .map(Self::get_circuit_health_score)
            .sum();
        total_score / circuits.len() as u32
    }

    /// Problem 47: Alert on threshold breach
    pub fn alert_on_threshold_breach(
        cb: &CircuitBreakerStatus,
        threshold_percent: u32,
    ) -> Option<String> {
        let failure_percent = Self::get_failure_percentage(cb);
        if failure_percent > threshold_percent {
            Some(format!("Failure rate: {}%", failure_percent))
        } else {
            None
        }
    }

    /// Problem 48: Generate circuit breaker report
    pub fn generate_circuit_breaker_report(
        circuits: &HashMap<String, CircuitBreakerStatus>,
    ) -> String {
        format!(
            "Circuit Breaker Report: {} circuits, {} open",
            circuits.len(),
            Self::get_open_circuits_count(circuits)
        )
    }

    /// Problem 49: Get state transition history
    pub fn get_state_transition_history(
        _service_name: &str,
    ) -> Vec<(CircuitState, u64)> {
        Vec::new()
    }

    /// Problem 50: Export metrics for monitoring
    pub fn export_metrics_for_monitoring(
        circuits: &HashMap<String, CircuitBreakerStatus>,
    ) -> String {
        let health_score = Self::get_system_health_score(circuits);
        format!("health_score={}", health_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_circuit_breaker() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        assert_eq!(cb.state, CircuitState::Closed);
    }

    #[test]
    fn test_transition_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let mut cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        CircuitBreakerSolver::transition_to_open(&mut cb, 1000);
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn test_record_failure_triggers_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let mut cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        CircuitBreakerSolver::record_failure(&mut cb, &config, 1000);
        CircuitBreakerSolver::record_failure(&mut cb, &config, 1001);
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn test_half_open_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let mut cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        CircuitBreakerSolver::transition_to_open(&mut cb, 1000);
        assert!(CircuitBreakerSolver::should_allow_half_open(&cb, &config, 1061));
    }

    #[test]
    fn test_can_execute_request() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        assert!(CircuitBreakerSolver::can_execute_request(&cb));
    }

    #[test]
    fn test_failure_rate() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let mut cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        CircuitBreakerSolver::record_failure(&mut cb, &config, 1000);
        CircuitBreakerSolver::record_success(&mut cb, &config);
        assert_eq!(CircuitBreakerSolver::get_failure_percentage(&cb), 50);
    }

    #[test]
    fn test_multiple_circuits() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let services = vec!["api".to_string(), "db".to_string(), "cache".to_string()];
        let circuits = CircuitBreakerSolver::create_circuits_for_services(&services, &config);
        assert_eq!(circuits.len(), 3);
    }

    #[test]
    fn test_cascade_detection() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let mut circuits = HashMap::new();
        let mut cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        CircuitBreakerSolver::transition_to_open(&mut cb, 1000);
        circuits.insert("api".to_string(), cb);

        let is_cascade = CircuitBreakerSolver::cascade_failure_detection(&circuits, 50);
        assert!(is_cascade);
    }

    #[test]
    fn test_health_score() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        assert_eq!(CircuitBreakerSolver::get_circuit_health_score(&cb), 100);
    }

    #[test]
    fn test_execute_with_circuit_breaker() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_seconds: 60,
            half_open_max_requests: 5,
        };
        let cb = CircuitBreakerSolver::create_circuit_breaker(&config);
        assert!(CircuitBreakerSolver::execute_with_circuit_breaker(&cb).is_ok());
    }
}
