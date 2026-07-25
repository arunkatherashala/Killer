// Phase 2.4: JIT Compilation Fallback System
// Graceful degradation when JIT compilation fails
// Silently falls back to interpreter without user visibility

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FallbackReason {
    /// Bytecode too complex for JIT
    ComplexBytecode,
    /// Unsupported instruction in JIT
    UnsupportedInstruction(String),
    /// JIT compilation error
    CompilationError(String),
    /// Out of memory during JIT
    OutOfMemory,
    /// Timeout during compilation
    CompilationTimeout,
}

#[allow(anonymous_parameters)]
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub function_name: String,
    pub reason: FallbackReason,
    pub timestamp: u64,
    pub fallback_count: u32,
}

#[derive(Debug)]
pub struct JITFallbackSystem {
    /// Functions that have fallen back to interpreter
    fallback_functions: HashMap<String, FallbackEvent>,
    /// Total fallback events
    total_fallbacks: u32,
    /// Track consecutive failures per function
    consecutive_failures: HashMap<String, u32>,
    /// Max attempts before giving up on JIT for this function
    max_jit_attempts: u32,
    /// Current timestamp
    timestamp_counter: u64,
}

impl JITFallbackSystem {
    pub fn new() -> Self {
        JITFallbackSystem {
            fallback_functions: HashMap::new(),
            total_fallbacks: 0,
            consecutive_failures: HashMap::new(),
            max_jit_attempts: 5, // Try JIT up to 5 times per function
            timestamp_counter: 0,
        }
    }

    /// Record a JIT compilation fallback
    /// Returns true if function should be retried, false if given up
    pub fn record_fallback(
        &mut self,
        function_name: String,
        reason: FallbackReason,
    ) -> bool {
        self.timestamp_counter += 1;

        // Track consecutive failures
        let failures = self.consecutive_failures.entry(function_name.clone()).or_insert(0);
        *failures += 1;

        // Check if we've exceeded retry limit
        if *failures >= self.max_jit_attempts {
            // Give up on JIT for this function
            self.fallback_functions.insert(
                function_name.clone(),
                FallbackEvent {
                    function_name,
                    reason,
                    timestamp: self.timestamp_counter,
                    fallback_count: *failures,
                },
            );
            self.total_fallbacks += 1;
            return false; // Don't retry
        }

        self.fallback_functions.insert(
            function_name.clone(),
            FallbackEvent {
                function_name,
                reason,
                timestamp: self.timestamp_counter,
                fallback_count: *failures,
            },
        );
        self.total_fallbacks += 1;

        true // Retry allowed
    }

    /// Clear fallback record for a function (successful JIT resets count)
    pub fn clear_fallback_for_function(&mut self, function_name: &str) {
        self.consecutive_failures.remove(function_name);
        self.fallback_functions.remove(function_name);
    }

    /// Check if function should attempt JIT compilation
    pub fn should_attempt_jit(&self, function_name: &str) -> bool {
        let failures = self.consecutive_failures.get(function_name).unwrap_or(&0);
        *failures < self.max_jit_attempts
    }

    /// Get fallback statistics
    pub fn get_statistics(&self) -> FallbackStatistics {
        FallbackStatistics {
            total_fallback_events: self.total_fallbacks,
            fallback_functions: self.fallback_functions.len(),
            functions_given_up_on: self
                .consecutive_failures
                .iter()
                .filter(|(_, &failures)| failures >= self.max_jit_attempts)
                .count(),
            most_problematic_function: self
                .consecutive_failures
                .iter()
                .max_by_key(|(_, &failures)| failures)
                .map(|(name, &failures)| (name.clone(), failures)),
        }
    }

    /// Get detailed fallback report
    pub fn get_fallback_report(&self) -> Vec<FallbackEvent> {
        self.fallback_functions.values().cloned().collect()
    }

    /// Reset all fallback tracking
    pub fn reset(&mut self) {
        self.fallback_functions.clear();
        self.consecutive_failures.clear();
        self.total_fallbacks = 0;
    }
}

#[derive(Debug, Clone)]
pub struct FallbackStatistics {
    pub total_fallback_events: u32,
    pub fallback_functions: usize,
    pub functions_given_up_on: usize,
    pub most_problematic_function: Option<(String, u32)>,
}

impl Default for JITFallbackSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_system_creation() {
        let system = JITFallbackSystem::new();
        assert_eq!(system.total_fallbacks, 0);
    }

    #[test]
    fn test_record_single_fallback() {
        let mut system = JITFallbackSystem::new();

        let should_retry = system.record_fallback(
            "test_fn".to_string(),
            FallbackReason::ComplexBytecode,
        );

        assert!(should_retry, "Should allow retry after first failure");
        assert_eq!(system.total_fallbacks, 1);
        assert_eq!(system.consecutive_failures.get("test_fn"), Some(&1));
    }

    #[test]
    fn test_max_jit_attempts() {
        let mut system = JITFallbackSystem::new();
        system.max_jit_attempts = 3;

        let mut should_retry = true;
        let mut attempts = 0;

        while should_retry && attempts < 5 {
            should_retry = system.record_fallback(
                "problematic_fn".to_string(),
                FallbackReason::CompilationTimeout,
            );
            attempts += 1;
        }

        assert_eq!(attempts, 3, "Should attempt JIT 3 times before giving up");
        assert!(!should_retry, "Should return false after max attempts");
    }

    #[test]
    fn test_clear_fallback_resets_count() {
        let mut system = JITFallbackSystem::new();

        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);
        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);

        assert_eq!(system.consecutive_failures.get("fn1"), Some(&2));

        system.clear_fallback_for_function("fn1");

        assert_eq!(system.consecutive_failures.get("fn1"), None);
    }

    #[test]
    fn test_should_attempt_jit() {
        let mut system = JITFallbackSystem::new();
        system.max_jit_attempts = 3;

        // First few attempts should allow JIT
        assert!(system.should_attempt_jit("new_fn"));

        // Record failures
        for _ in 0..3 {
            system.record_fallback("new_fn".to_string(), FallbackReason::CompilationError("test".to_string()));
        }

        // After max attempts, should not retry
        assert!(!system.should_attempt_jit("new_fn"));
    }

    #[test]
    fn test_multiple_functions() {
        let mut system = JITFallbackSystem::new();
        system.max_jit_attempts = 2;

        // Function 1 fails once
        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);

        // Function 2 fails 3 times (exceeds limit)
        system.record_fallback("fn2".to_string(), FallbackReason::UnsupportedInstruction("add".to_string()));
        system.record_fallback("fn2".to_string(), FallbackReason::UnsupportedInstruction("add".to_string()));
        system.record_fallback("fn2".to_string(), FallbackReason::UnsupportedInstruction("add".to_string()));

        let stats = system.get_statistics();
        assert_eq!(stats.fallback_functions, 2);
        assert_eq!(stats.functions_given_up_on, 1); // Only fn2
        assert_eq!(stats.total_fallback_events, 3);
    }

    #[test]
    fn test_fallback_statistics() {
        let mut system = JITFallbackSystem::new();

        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);
        system.record_fallback("fn2".to_string(), FallbackReason::OutOfMemory);
        system.record_fallback("fn2".to_string(), FallbackReason::OutOfMemory);

        let stats = system.get_statistics();
        assert_eq!(stats.total_fallback_events, 3);
        assert_eq!(stats.fallback_functions, 2);
        
        if let Some((name, failures)) = stats.most_problematic_function {
            assert_eq!(name, "fn2");
            assert_eq!(failures, 2);
        }
    }

    #[test]
    fn test_fallback_report() {
        let mut system = JITFallbackSystem::new();

        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);
        system.record_fallback("fn2".to_string(), FallbackReason::CompilationError("test".to_string()));

        let report = system.get_fallback_report();
        assert_eq!(report.len(), 2);
        assert!(report.iter().any(|e| e.function_name == "fn1"));
        assert!(report.iter().any(|e| e.function_name == "fn2"));
    }

    #[test]
    fn test_reset_clears_all() {
        let mut system = JITFallbackSystem::new();

        system.record_fallback("fn1".to_string(), FallbackReason::ComplexBytecode);
        system.record_fallback("fn2".to_string(), FallbackReason::OutOfMemory);

        assert_eq!(system.total_fallbacks, 2);

        system.reset();

        assert_eq!(system.total_fallbacks, 0);
        assert_eq!(system.fallback_functions.len(), 0);
        assert_eq!(system.consecutive_failures.len(), 0);
    }
}
