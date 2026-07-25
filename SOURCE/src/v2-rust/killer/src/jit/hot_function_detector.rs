// Phase 2.1: Hot Function Detector
// Identifies frequently-called functions for JIT compilation
// Strategy: Track call counts, compile on threshold (3 calls)

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionStats {
    pub name: String,
    pub call_count: u64,
    pub bytecode_start: usize,
    pub last_compiled: bool,
}

#[derive(Debug, Clone)]
pub struct HotFunctionDetector {
    /// function_name -> call stats
    functions: HashMap<String, FunctionStats>,
    /// Track compilation threshold (default: 3 calls = JIT)
    compile_threshold: u64,
    /// Total functions compiled
    compiled_count: usize,
    /// Total bytecode size compiled (estimate)
    compiled_bytecode_size: usize,
}

impl HotFunctionDetector {
    pub fn new(compile_threshold: u64) -> Self {
        HotFunctionDetector {
            functions: HashMap::new(),
            compile_threshold,
            compiled_count: 0,
            compiled_bytecode_size: 0,
        }
    }

    /// Register a function entry point
    pub fn register_function(&mut self, name: String, bytecode_start: usize) {
        if !self.functions.contains_key(&name) {
            self.functions.insert(
                name.clone(),
                FunctionStats {
                    name,
                    call_count: 0,
                    bytecode_start,
                    last_compiled: false,
                },
            );
        }
    }

    /// Record a function call, returns true if should be compiled
    pub fn record_call(&mut self, name: &str) -> bool {
        if let Some(stats) = self.functions.get_mut(name) {
            stats.call_count += 1;
            
            // Compile on threshold hit (e.g., 3rd call)
            if stats.call_count == self.compile_threshold && !stats.last_compiled {
                stats.last_compiled = true;
                self.compiled_count += 1;
                return true;
            }
        }
        false
    }

    /// Get function statistics
    pub fn get_stats(&self, name: &str) -> Option<FunctionStats> {
        self.functions.get(name).cloned()
    }

    /// Get all functions with call count >= threshold
    pub fn get_hot_functions(&self) -> Vec<FunctionStats> {
        self.functions
            .values()
            .filter(|f| f.call_count >= self.compile_threshold)
            .cloned()
            .collect()
    }

    /// Get total compilation statistics
    pub fn get_total_stats(&self) -> HotFunctionStats {
        let total_functions = self.functions.len();
        let total_hot = self.get_hot_functions().len();
        let total_calls: u64 = self.functions.values().map(|f| f.call_count).sum();

        HotFunctionStats {
            total_functions_registered: total_functions,
            hot_functions_detected: total_hot,
            functions_compiled: self.compiled_count,
            total_function_calls: total_calls,
            estimated_compiled_size: self.compiled_bytecode_size,
            compile_threshold: self.compile_threshold,
            average_calls_per_function: if total_functions > 0 {
                total_calls / total_functions as u64
            } else {
                0
            },
        }
    }

    /// Reset statistics (typically after compilation phase)
    pub fn reset(&mut self) {
        self.functions.clear();
        self.compiled_count = 0;
        self.compiled_bytecode_size = 0;
    }

    /// Set compilation threshold (typically during configuration)
    pub fn set_threshold(&mut self, threshold: u64) {
        self.compile_threshold = threshold;
    }
}

#[derive(Debug, Clone)]
pub struct HotFunctionStats {
    pub total_functions_registered: usize,
    pub hot_functions_detected: usize,
    pub functions_compiled: usize,
    pub total_function_calls: u64,
    pub estimated_compiled_size: usize,
    pub compile_threshold: u64,
    pub average_calls_per_function: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_registration() {
        let mut detector = HotFunctionDetector::new(3);
        detector.register_function("foo".to_string(), 0);
        detector.register_function("bar".to_string(), 100);
        
        assert_eq!(detector.get_total_stats().total_functions_registered, 2);
    }

    #[test]
    fn test_call_counting() {
        let mut detector = HotFunctionDetector::new(3);
        detector.register_function("test_fn".to_string(), 0);
        
        assert!(!detector.record_call("test_fn"));
        assert!(!detector.record_call("test_fn"));
        assert!(detector.record_call("test_fn")); // 3rd call triggers compilation
    }

    #[test]
    fn test_threshold_customization() {
        let mut detector = HotFunctionDetector::new(5);
        detector.register_function("fn1".to_string(), 0);
        
        for _ in 0..4 {
            assert!(!detector.record_call("fn1"));
        }
        assert!(detector.record_call("fn1")); // 5th call
    }

    #[test]
    fn test_hot_function_detection() {
        let mut detector = HotFunctionDetector::new(2);
        detector.register_function("hot".to_string(), 0);
        detector.register_function("cold".to_string(), 100);
        
        detector.record_call("hot");
        detector.record_call("hot"); // Now hot
        
        let hot_funcs = detector.get_hot_functions();
        assert_eq!(hot_funcs.len(), 1);
        assert_eq!(hot_funcs[0].name, "hot");
    }

    #[test]
    fn test_statistics() {
        let mut detector = HotFunctionDetector::new(2);
        detector.register_function("f1".to_string(), 0);
        detector.register_function("f2".to_string(), 50);
        detector.register_function("f3".to_string(), 100);
        
        detector.record_call("f1");
        detector.record_call("f1");
        detector.record_call("f2");
        detector.record_call("f2");
        detector.record_call("f3");
        
        let stats = detector.get_total_stats();
        assert_eq!(stats.total_functions_registered, 3);
        assert_eq!(stats.functions_compiled, 2); // f1 and f2 hit threshold
        assert_eq!(stats.total_function_calls, 5);
    }

    #[test]
    fn test_multiple_threshold_hits() {
        let mut detector = HotFunctionDetector::new(2);
        detector.register_function("busy".to_string(), 0);
        
        // Only compiles once, even with many calls
        for _ in 0..10 {
            detector.record_call("busy");
        }
        
        assert_eq!(detector.get_total_stats().functions_compiled, 1);
    }
}
