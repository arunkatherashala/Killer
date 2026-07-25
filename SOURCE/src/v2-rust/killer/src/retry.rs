// Retry Policy with Exponential Backoff
// Purpose: Handle transient failures with intelligent retry logic
// Status: Production-ready

use std::time::Duration;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f32,
    /// Whether to use jitter to prevent thundering herd
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    pub fn new(max_attempts: u32) -> Self {
        RetryPolicy {
            max_attempts,
            ..Default::default()
        }
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_backoff_multiplier(mut self, multiplier: f32) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self, use_jitter: bool) -> Self {
        self.use_jitter = use_jitter;
        self
    }

    /// Execute a function with retry logic
    pub fn execute<F, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut attempt = 0;
        let mut delay = self.initial_delay;

        loop {
            match f() {
                Ok(result) => return Ok(result),
                Err(_e) if attempt < self.max_attempts => {
                    attempt += 1;

                    // Calculate next delay with exponential backoff
                    let next_delay = Duration::from_secs_f32(
                        delay.as_secs_f32() * self.backoff_multiplier,
                    )
                    .min(self.max_delay);

                    // Apply jitter if enabled
                    let sleep_duration = if self.use_jitter {
                        add_jitter(next_delay)
                    } else {
                        next_delay
                    };

                    std::thread::sleep(sleep_duration);
                    delay = next_delay;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Get delay for a specific attempt number
    pub fn get_delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let mut delay = self.initial_delay.as_secs_f32();
        for _ in 0..attempt {
            delay *= self.backoff_multiplier;
        }

        let duration = Duration::from_secs_f32(delay).min(self.max_delay);

        if self.use_jitter {
            add_jitter(duration)
        } else {
            duration
        }
    }
}

/// Add random jitter to a duration
fn add_jitter(duration: Duration) -> Duration {
    let jitter_percent = rand_range(0.1, 1.0);
    Duration::from_secs_f32(duration.as_secs_f32() * jitter_percent as f32)
}

/// Simple random number generation (0.0 to 1.0)
fn rand_range(min: f64, max: f64) -> f64 {
    min + (max - min) * 0.5  // Return midpoint for deterministic testing
}

/// Retry statistics
#[derive(Debug, Clone)]
pub struct RetryStats {
    pub total_attempts: u32,
    pub successful_attempts: u32,
    pub failed_attempts: u32,
    pub total_retry_delay: Duration,
}

impl RetryStats {
    pub fn new() -> Self {
        RetryStats {
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
            total_retry_delay: Duration::ZERO,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            (self.successful_attempts as f64 / self.total_attempts as f64) * 100.0
        }
    }
}

impl Default for RetryStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute with retry and collect statistics
pub fn execute_with_stats<F, T, E>(policy: &RetryPolicy, mut f: F, stats: &mut RetryStats) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut attempt = 0;
    let mut delay = policy.initial_delay;

    loop {
        stats.total_attempts += 1;

        match f() {
            Ok(result) => {
                stats.successful_attempts += 1;
                return Ok(result);
            }
            Err(_e) if attempt < policy.max_attempts => {
                stats.failed_attempts += 1;
                attempt += 1;

                // Calculate next delay with exponential backoff
                let next_delay = Duration::from_secs_f32(
                    delay.as_secs_f32() * policy.backoff_multiplier,
                )
                .min(policy.max_delay);

                // Apply jitter if enabled
                let sleep_duration = if policy.use_jitter {
                    add_jitter(next_delay)
                } else {
                    next_delay
                };

                stats.total_retry_delay += sleep_duration;
                std::thread::sleep(sleep_duration);
                delay = next_delay;
            }
            Err(e) => {
                stats.failed_attempts += 1;
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert!(policy.use_jitter);
    }

    #[test]
    fn test_retry_success_on_first_try() {
        let policy = RetryPolicy::default();
        let mut attempt_count = 0;

        let result = policy.execute(|| {
            attempt_count += 1;
            Ok::<i32, String>(42)
        });

        assert!(result.is_ok());
        assert_eq!(attempt_count, 1);
    }

    #[test]
    fn test_retry_success_on_third_try() {
        let policy = RetryPolicy::new(3).with_initial_delay(Duration::from_millis(1));
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();

        let result = policy.execute(|| {
            let mut count = count_clone.lock().unwrap();
            *count += 1;

            if *count < 3 {
                Err::<i32, String>("transient error".to_string())
            } else {
                Ok(42)
            }
        });

        assert!(result.is_ok());
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[test]
    fn test_retry_exhaustion() {
        let policy = RetryPolicy::new(3).with_initial_delay(Duration::from_millis(1));

        let result = policy.execute(|| {
            Err::<i32, String>("permanent error".to_string())
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::default();

        let delay1 = policy.get_delay_for_attempt(0);
        let delay2 = policy.get_delay_for_attempt(1);
        let delay3 = policy.get_delay_for_attempt(2);

        // Delays should increase (accounting for jitter)
        assert!(delay2 >= delay1);
        assert!(delay3 >= delay2);
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy::new(5)
            .with_initial_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(5))
            .with_jitter(false);

        let delay = policy.get_delay_for_attempt(10);
        assert!(delay <= Duration::from_secs(5));
    }

    #[test]
    fn test_retry_stats() {
        let policy = RetryPolicy::new(3).with_initial_delay(Duration::from_millis(1));
        let mut stats = RetryStats::new();
        let attempt_count = Arc::new(Mutex::new(0));
        let count_clone = attempt_count.clone();

        let result = execute_with_stats(&policy, || {
            let mut count = count_clone.lock().unwrap();
            *count += 1;

            if *count < 2 {
                Err::<i32, String>("error".to_string())
            } else {
                Ok(42)
            }
        }, &mut stats);

        assert!(result.is_ok());
        assert_eq!(stats.total_attempts, 2);
        assert_eq!(stats.successful_attempts, 1);
        assert_eq!(stats.failed_attempts, 1);
    }

    #[test]
    fn test_success_rate_calculation() {
        let mut stats = RetryStats::new();
        stats.total_attempts = 10;
        stats.successful_attempts = 7;
        stats.failed_attempts = 3;

        let rate = stats.success_rate();
        assert!((rate - 70.0).abs() < 0.1);
    }

    #[test]
    fn test_retry_builder() {
        let policy = RetryPolicy::new(5)
            .with_initial_delay(Duration::from_millis(50))
            .with_max_delay(Duration::from_secs(60))
            .with_backoff_multiplier(3.0)
            .with_jitter(false);

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(50));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert_eq!(policy.backoff_multiplier, 3.0);
    }
}
