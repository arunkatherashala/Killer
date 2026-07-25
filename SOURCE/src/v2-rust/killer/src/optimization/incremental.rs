/// Phase 8C: Incremental Injection Framework
/// Runtime parameter injection for optimized loops
/// Enables per-loop optimization updates without full recompilation

/// Injection schedule for parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InjectionSchedule {
    /// Immediate injection
    Immediate,
    /// Inject at specific timestamp
    Scheduled,
    /// Inject only when loop encountered at runtime
    LazyOnDemand,
    /// Gradually inject (ramp-up)
    Gradual,
}

/// Injection status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InjectionState {
    /// Pending injection
    Pending,
    /// Injection in progress
    InProgress,
    /// Successfully injected
    Complete,
    /// Injection failed
    Failed,
    /// Rolled back
    RolledBack,
}

/// Single parameter injection event
#[derive(Debug, Clone)]
pub struct InjectionEvent {
    /// Loop ID being optimized
    pub loop_id: String,
    /// State of injection
    pub state: InjectionState,
    /// Schedule type
    pub schedule: InjectionSchedule,
    /// Parameter name being injected
    pub parameter_name: String,
    /// Old parameter value
    pub old_value: f64,
    /// New parameter value
    pub new_value: f64,
    /// Timestamp of injection (ms since start)
    pub timestamp_ms: u64,
    /// Speedup after injection
    pub speedup_achieved: f64,
}

impl InjectionEvent {
    /// Create a new injection event
    pub fn new(loop_id: &str, param_name: &str, old_val: f64, new_val: f64) -> Self {
        InjectionEvent {
            loop_id: loop_id.to_string(),
            state: InjectionState::Pending,
            schedule: InjectionSchedule::Immediate,
            parameter_name: param_name.to_string(),
            old_value: old_val,
            new_value: new_val,
            timestamp_ms: 0,
            speedup_achieved: 1.0,
        }
    }

    /// Get parameter change ratio
    pub fn parameter_ratio(&self) -> f64 {
        if self.old_value == 0.0 {
            return 1.0;
        }
        self.new_value / self.old_value
    }
}

/// Incremental Injection Framework
#[derive(Debug, Clone)]
pub struct IncrementalInjectionFramework {
    /// All injection events
    pub events: Vec<InjectionEvent>,
    /// Injections pending
    pub pending_count: usize,
    /// Successful injections
    pub successful_count: usize,
    /// Failed injections
    pub failed_count: usize,
    /// Total injection overhead (ms)
    pub total_overhead_ms: u64,
}

impl IncrementalInjectionFramework {
    /// Create a new incremental injection framework
    pub fn new() -> Self {
        IncrementalInjectionFramework {
            events: Vec::new(),
            pending_count: 0,
            successful_count: 0,
            failed_count: 0,
            total_overhead_ms: 0,
        }
    }

    /// Schedule an injection
    pub fn schedule_injection(
        &mut self,
        loop_id: &str,
        param_name: &str,
        old_val: f64,
        new_val: f64,
        schedule: InjectionSchedule,
    ) -> InjectionEvent {
        let mut event = InjectionEvent::new(loop_id, param_name, old_val, new_val);
        event.schedule = schedule;
        event.state = InjectionState::Pending;
        self.pending_count += 1;
        self.events.push(event.clone());
        event
    }

    /// Execute a pending injection
    pub fn execute_injection(&mut self, loop_id: &str, param_name: &str) -> Result<InjectionEvent, String> {
        // Find the injection event
        let event_index = self
            .events
            .iter()
            .position(|e| e.loop_id == loop_id && e.parameter_name == param_name && e.state == InjectionState::Pending);

        if event_index.is_none() {
            return Err(format!(
                "No pending injection found for loop {} parameter {}",
                loop_id, param_name
            ));
        }

        let index = event_index.unwrap();
        
        // Store length before mutable borrow
        let events_len = self.events.len();
        
        let event = &mut self.events[index];

        // Simulate injection
        event.state = InjectionState::InProgress;

        // Simulate speedup based on parameter change
        // More aggressive parameter changes tend to give more speedup
        let param_ratio = event.parameter_ratio();
        let speedup = if param_ratio > 1.0 {
            // Increasing parameter: speedup proportional to change
            1.0 + (param_ratio - 1.0) * 0.5
        } else {
            // Decreasing parameter: less aggressive speedup
            1.0 + (1.0 - param_ratio) * 0.3
        };

        event.speedup_achieved = speedup.min(4.0); // Cap at 4x
        event.state = InjectionState::Complete;
        event.timestamp_ms = events_len as u64 * 10; // Simulate timing

        // Track overhead
        let overhead = 2 + (events_len / 10) as u64; // 2-5ms overhead
        self.total_overhead_ms += overhead;

        self.pending_count -= 1;
        self.successful_count += 1;

        Ok(self.events[index].clone())
    }

    /// Rollback an injection
    pub fn rollback_injection(&mut self, loop_id: &str, param_name: &str) -> Result<(), String> {
        let event = self
            .events
            .iter_mut()
            .find(|e| e.loop_id == loop_id && e.parameter_name == param_name);

        if let Some(event) = event {
            if event.state == InjectionState::Complete {
                event.state = InjectionState::RolledBack;
                event.speedup_achieved = 1.0;
                self.successful_count = self.successful_count.saturating_sub(1);
                Ok(())
            } else {
                Err("Can only rollback completed injections".to_string())
            }
        } else {
            Err(format!("Injection not found for loop {} parameter {}", loop_id, param_name))
        }
    }

    /// Get average injection overhead (ms)
    pub fn average_injection_overhead_ms(&self) -> f64 {
        if self.successful_count == 0 {
            return 0.0;
        }
        self.total_overhead_ms as f64 / self.successful_count as f64
    }

    /// Get average speedup from successful injections
    pub fn average_speedup(&self) -> f64 {
        let successful: Vec<_> = self.events.iter().filter(|e| e.state == InjectionState::Complete).collect();

        if successful.is_empty() {
            return 1.0;
        }

        let sum: f64 = successful.iter().map(|e| e.speedup_achieved).sum();
        sum / successful.len() as f64
    }

    /// Check if all injections complete with acceptable performance
    pub fn all_injections_successful(&self) -> bool {
        if self.events.is_empty() {
            return false;
        }

        // All injections must be complete or rolled back
        let all_done = self.events.iter().all(|e| {
            e.state == InjectionState::Complete || e.state == InjectionState::RolledBack
        });

        // No failures
        let no_failures = self.failed_count == 0;

        all_done && no_failures
    }

    /// Get injection success rate
    pub fn success_rate(&self) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }
        self.successful_count as f64 / self.events.len() as f64
    }

    /// Get parallel injection speedup (executing multiple injections in parallel)
    pub fn parallel_injection_speedup(&self) -> f64 {
        // With N injections and M execution threads, effective speedup approaches M
        // Overhead reduces this slightly
        let injection_count = self.events.len().max(1);
        let threads = 4; // Assume 4 parallel injection threads
        let overhead_factor = 1.0 + (0.02 * (injection_count as f64 / 10.0));

        (threads as f64 / overhead_factor).min(threads as f64)
    }

    /// Status report
    pub fn status_report(&self) -> String {
        format!(
            "IncrementalInjectionFramework (Pending: {}, Successful: {}, Failed: {}, Avg speedup: {:.2}x)",
            self.pending_count,
            self.successful_count,
            self.failed_count,
            self.average_speedup()
        )
    }
}

impl Default for IncrementalInjectionFramework {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_event_creation() {
        let event = InjectionEvent::new("loop_1", "unroll_factor", 4.0, 8.0);
        assert_eq!(event.loop_id, "loop_1");
        assert_eq!(event.parameter_name, "unroll_factor");
        assert_eq!(event.state, InjectionState::Pending);
    }

    #[test]
    fn test_injection_event_parameter_ratio() {
        let event = InjectionEvent::new("loop_1", "factor", 4.0, 8.0);
        assert_eq!(event.parameter_ratio(), 2.0);

        let event2 = InjectionEvent::new("loop_2", "factor", 8.0, 4.0);
        assert_eq!(event2.parameter_ratio(), 0.5);
    }

    #[test]
    fn test_framework_creation() {
        let framework = IncrementalInjectionFramework::new();
        assert_eq!(framework.pending_count, 0);
        assert_eq!(framework.successful_count, 0);
    }

    #[test]
    fn test_framework_schedule_injection() {
        let mut framework = IncrementalInjectionFramework::new();

        let event = framework.schedule_injection(
            "loop_1",
            "unroll",
            4.0,
            8.0,
            InjectionSchedule::Immediate,
        );

        assert_eq!(framework.pending_count, 1);
        assert_eq!(event.state, InjectionState::Pending);
    }

    #[test]
    fn test_framework_execute_injection() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "unroll", 4.0, 8.0, InjectionSchedule::Immediate);
        let result = framework.execute_injection("loop_1", "unroll");

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.state, InjectionState::Complete);
        assert!(event.speedup_achieved > 1.0);
    }

    #[test]
    fn test_framework_injection_statistics() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "unroll", 4.0, 8.0, InjectionSchedule::Immediate);
        framework.schedule_injection("loop_2", "block", 64.0, 128.0, InjectionSchedule::Immediate);

        let _ = framework.execute_injection("loop_1", "unroll");
        let _ = framework.execute_injection("loop_2", "block");

        assert_eq!(framework.successful_count, 2);
        assert!(framework.average_speedup() > 1.0);
    }

    #[test]
    fn test_framework_rollback_injection() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "factor", 4.0, 8.0, InjectionSchedule::Immediate);
        let _ = framework.execute_injection("loop_1", "factor");

        assert_eq!(framework.successful_count, 1);

        let rollback = framework.rollback_injection("loop_1", "factor");
        assert!(rollback.is_ok());
        assert_eq!(framework.successful_count, 0);
    }

    #[test]
    fn test_framework_success_rate() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "param1", 4.0, 8.0, InjectionSchedule::Immediate);
        framework.schedule_injection("loop_2", "param2", 2.0, 4.0, InjectionSchedule::Immediate);

        let _ = framework.execute_injection("loop_1", "param1");

        let rate = framework.success_rate();
        assert!(rate > 0.4 && rate < 0.6); // 1 out of 2
    }

    #[test]
    fn test_framework_parallel_injection_speedup() {
        let mut framework = IncrementalInjectionFramework::new();

        for i in 0..10 {
            framework.schedule_injection(
                &format!("loop_{}", i),
                "param",
                1.0,
                2.0,
                InjectionSchedule::Immediate,
            );
        }

        let speedup = framework.parallel_injection_speedup();
        assert!(speedup > 2.0 && speedup <= 4.0);
    }

    #[test]
    fn test_framework_status_report() {
        let mut framework = IncrementalInjectionFramework::new();

        framework.schedule_injection("loop_1", "param", 1.0, 2.0, InjectionSchedule::Immediate);
        let _ = framework.execute_injection("loop_1", "param");

        let report = framework.status_report();
        assert!(report.contains("IncrementalInjectionFramework"));
        assert!(report.contains("Successful: 1"));
    }
}
