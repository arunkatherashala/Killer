// Circuit Breaker Pattern for Error Recovery
// Purpose: Prevent cascading failures by stopping requests to failing services
// Status: Production-ready

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// State of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - all requests pass through
    Closed,
    /// Service failing - requests are blocked
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Error type for circuit breaker operations
#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
    CircuitOpen,
    TooManyRequests,
    ServiceUnavailable(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::TooManyRequests => write!(f, "Too many requests in half-open state"),
            CircuitBreakerError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

/// Configuration for circuit breaker with exponential backoff
///
/// v4.3 Enhancement: Added exponential backoff for timeout progression
/// Prevents thundering herd when service recovers
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Number of successful requests before closing circuit from half-open
    pub success_threshold: u32,
    /// Initial timeout duration before attempting recovery in half-open state
    pub initial_timeout: Duration,
    /// Maximum timeout (stops exponential backoff from growing indefinitely)
    pub max_timeout: Duration,
    /// Backoff multiplier for exponential progression (e.g., 2.0 for doubling)
    pub backoff_multiplier: f64,
    /// Maximum requests allowed in half-open state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            initial_timeout: Duration::from_secs(30),
            max_timeout: Duration::from_secs(300),  // 5 minutes max
            backoff_multiplier: 2.0,
            half_open_max_requests: 3,
        }
    }
}

/// Circuit breaker for error recovery with exponential backoff (v4.3)
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<SystemTime>>>,
    half_open_requests: Arc<Mutex<u32>>,
    state_changes: Arc<Mutex<Vec<StateChange>>>,
    consecutive_opens: Arc<Mutex<u32>>,  // Track reopenings for backoff calculation
}

#[derive(Debug, Clone)]
pub struct StateChange {
    pub timestamp: SystemTime,
    pub from_state: CircuitState,
    pub to_state: CircuitState,
    pub reason: String,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        CircuitBreaker {
            config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            half_open_requests: Arc::new(Mutex::new(0)),
            state_changes: Arc::new(Mutex::new(Vec::new())),
            consecutive_opens: Arc::new(Mutex::new(0)),
        }
    }

    /// Calculate exponential backoff timeout
    /// Progression: 30s → 60s → 120s → 240s → 300s (capped at max_timeout)
    /// consecutive_opens=1 (first trip) → initial_timeout; =2 → *2; =3 → *4
    fn calculate_timeout(&self, consecutive_opens: u32) -> Duration {
        let base_secs = self.config.initial_timeout.as_secs_f64();
        let backoff_count = consecutive_opens.saturating_sub(1); // first open uses base
        let multiplier = self.config.backoff_multiplier.powi(backoff_count as i32);
        let timeout_secs = base_secs * multiplier;
        let max_secs = self.config.max_timeout.as_secs_f64();
        
        Duration::from_secs_f64(timeout_secs.min(max_secs))
    }

    /// Check if recovery should be attempted (respects exponential backoff)
    #[allow(dead_code)]
    fn should_attempt_recovery(&self) -> bool {
        if let Ok(last_failure) = self.last_failure_time.lock() {
            if let Some(failure_time) = *last_failure {
                if let Ok(opens) = self.consecutive_opens.lock() {
                    let timeout = self.calculate_timeout(*opens);
                    
                    if let Ok(elapsed) = failure_time.elapsed() {
                        return elapsed >= timeout;
                    }
                }
            }
        }
        false
    }

    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    pub fn get_state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failure_count = self.failure_count.lock().unwrap();
        let mut success_count = self.success_count.lock().unwrap();
        let mut half_open_requests = self.half_open_requests.lock().unwrap();

        *failure_count = 0;  // Reset failure count on success

        match *state {
            CircuitState::Closed => {
                // Continue normal operation
            }
            CircuitState::HalfOpen => {
                *success_count += 1;
                if *success_count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    *success_count = 0;
                    *half_open_requests = 0;
                    // Reset exponential backoff counter on full recovery
                    if let Ok(mut opens) = self.consecutive_opens.lock() { *opens = 0; }

                    let old_state = CircuitState::HalfOpen;
                    self.record_state_change(old_state, *state, "Recovery succeeded");
                }
            }
            CircuitState::Open => {
                // Ignore success while open
            }
        }
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failure_count = self.failure_count.lock().unwrap();
        let mut last_failure_time = self.last_failure_time.lock().unwrap();

        *failure_count += 1;
        *last_failure_time = Some(SystemTime::now());

        match *state {
            CircuitState::Closed => {
                if *failure_count >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    if let Ok(mut opens) = self.consecutive_opens.lock() { *opens += 1; }
                    let old_state = CircuitState::Closed;
                    self.record_state_change(old_state, *state, "Failure threshold exceeded");
                }
            }
            CircuitState::HalfOpen => {
                // Single failure returns to open; increment consecutive opens for longer backoff
                *state = CircuitState::Open;
                if let Ok(mut opens) = self.consecutive_opens.lock() { *opens += 1; }
                let old_state = CircuitState::HalfOpen;
                self.record_state_change(old_state, *state, "Failure in half-open state");
            }
            CircuitState::Open => {
                // Continue counting failures while open
            }
        }
    }

    pub fn call<F, T>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let mut state = self.state.lock().unwrap();
        let mut last_failure_time = self.last_failure_time.lock().unwrap();

        match *state {
            CircuitState::Closed => {
                drop(state);  // Release lock before calling function
                drop(last_failure_time);  // Also release — record_failure() re-acquires it
                match f() {
                    Ok(result) => {
                        self.record_success();
                        Ok(result)
                    }
                    Err(e) => {
                        self.record_failure();
                        Err(CircuitBreakerError::ServiceUnavailable(e))
                    }
                }
            }
            CircuitState::Open => {
                // Check if exponential backoff timeout has passed
                if let Some(last_failure) = *last_failure_time {
                    let consecutive = self.consecutive_opens.lock().map(|g| *g).unwrap_or(0);
                    let timeout = self.calculate_timeout(consecutive);
                    if last_failure.elapsed().unwrap_or(Duration::ZERO) >= timeout {
                        *state = CircuitState::HalfOpen;
                        *last_failure_time = None;
                        let old_state = CircuitState::Open;
                        self.record_state_change(old_state, CircuitState::HalfOpen, "Timeout elapsed, attempting recovery");
                        drop(state);
                        drop(last_failure_time);
                        // Retry call in half-open state
                        return self.call(f);
                    }
                }
                Err(CircuitBreakerError::CircuitOpen)
            }
            CircuitState::HalfOpen => {
                let mut half_open_requests = self.half_open_requests.lock().unwrap();

                if *half_open_requests >= self.config.half_open_max_requests {
                    return Err(CircuitBreakerError::TooManyRequests);
                }

                *half_open_requests += 1;
                drop(state);
                drop(last_failure_time);
                drop(half_open_requests);

                match f() {
                    Ok(result) => {
                        self.record_success();
                        Ok(result)
                    }
                    Err(e) => {
                        self.record_failure();
                        Err(CircuitBreakerError::ServiceUnavailable(e))
                    }
                }
            }
        }
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failure_count = self.failure_count.lock().unwrap();
        let mut success_count = self.success_count.lock().unwrap();
        let mut half_open_requests = self.half_open_requests.lock().unwrap();

        *state = CircuitState::Closed;
        *failure_count = 0;
        *success_count = 0;
        *half_open_requests = 0;
    }

    fn record_state_change(&self, from: CircuitState, to: CircuitState, reason: &str) {
        if let Ok(mut changes) = self.state_changes.lock() {
            changes.push(StateChange {
                timestamp: SystemTime::now(),
                from_state: from,
                to_state: to,
                reason: reason.to_string(),
            });
        }
    }

    pub fn get_state_history(&self) -> Vec<StateChange> {
        self.state_changes.lock().ok().map(|c| c.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_closed_success() {
        let cb = CircuitBreaker::with_defaults();

        let result = cb.call(|| Ok::<_, String>(42));
        assert!(result.is_ok());
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        for _ in 0..3 {
            let _ = cb.call(|| Err::<i32, String>("error".to_string()));
        }

        assert_eq!(cb.get_state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_blocks_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        let _ = cb.call(|| Err::<i32, String>("error".to_string()));
        assert_eq!(cb.get_state(), CircuitState::Open);

        let result = cb.call(|| Ok::<_, String>(42));
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
    }

    #[test]
    fn test_circuit_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            initial_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        // Trip the circuit
        let _ = cb.call(|| Err::<i32, String>("error".to_string()));
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Should transition to half-open and try recovery
        let result = cb.call(|| Ok::<_, String>(42));
        assert!(result.is_ok());

        // One success in half-open
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);

        // Second success should close the circuit
        let result = cb.call(|| Ok::<_, String>(42));
        assert!(result.is_ok());
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[test]
    fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        let _ = cb.call(|| Err::<i32, String>("error".to_string()));
        assert_eq!(cb.get_state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.get_state(), CircuitState::Closed);

        let result = cb.call(|| Ok::<_, String>(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_history() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            initial_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        let _ = cb.call(|| Err::<i32, String>("error".to_string()));
        std::thread::sleep(Duration::from_millis(100));
        let _ = cb.call(|| Ok::<_, String>(42));

        let history = cb.get_state_history();
        assert!(history.len() >= 2);
    }
}
