// Phase 16: Hot Path Detection & Type Profiling Infrastructure
// This module tracks execution statistics to identify optimization opportunities

use std::collections::HashMap;

/// Statistics for a bytecode instruction or loop
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub execution_count: usize,      // How many times executed
    pub total_cycles: u64,           // Total CPU cycles spent
    pub type_profile: HashMap<String, usize>,  // Type distribution (e.g., "Number" -> 850, "String" -> 2)
    pub specialization_attempts: usize,  // How many times we tried to specialize this
    pub specialization_success_count: usize,  // How many times specialization helped
}

impl ExecutionStats {
    pub fn new() -> Self {
        ExecutionStats {
            execution_count: 0,
            total_cycles: 0,
            type_profile: HashMap::new(),
            specialization_attempts: 0,
            specialization_success_count: 0,
        }
    }

    /// Record a value being used in a hot path
    pub fn record_type(&mut self, type_name: &str) {
        *self.type_profile.entry(type_name.to_string()).or_insert(0) += 1;
    }

    /// Check if this path is "hot" (called frequently enough to justify optimization)
    pub fn is_hot(&self) -> bool {
        self.execution_count >= 500  // Threshold for hot path detection
    }

    /// Get the dominant type in this hot path (if any)
    pub fn dominant_type(&self) -> Option<(String, usize)> {
        self.type_profile
            .iter()
            .max_by_key(|(_k, v)| *v)
            .map(|(k, v)| (k.clone(), *v))
    }

    /// Check if this is a "numeric-only" hot path (99%+ numeric operations)
    pub fn is_numeric_only(&self) -> bool {
        let total: usize = self.type_profile.values().sum();
        if total == 0 {
            return false;
        }
        
        let numeric_count = self.type_profile.get("Number").copied().unwrap_or(0);
        (numeric_count as f64) / (total as f64) >= 0.99
    }

    /// Specialization ratio: how often specialization helps
    pub fn specialization_effectiveness(&self) -> f64 {
        if self.specialization_attempts == 0 {
            return 0.0;
        }
        (self.specialization_success_count as f64) / (self.specialization_attempts as f64)
    }
}

/// Tracks hot paths and type profiling across the VM execution
pub struct HotPathDetector {
    /// Map from instruction address to execution stats
    instruction_stats: HashMap<usize, ExecutionStats>,
    
    /// Map from loop start address to execution stats
    loop_stats: HashMap<usize, ExecutionStats>,
    
    /// Cycles per instruction for this execution
    cycles_per_instruction: f64,
    
    /// Threshold for identifying hot instructions
    hot_instruction_threshold: usize,
}

impl HotPathDetector {
    pub fn new() -> Self {
        HotPathDetector {
            instruction_stats: HashMap::new(),
            loop_stats: HashMap::new(),
            cycles_per_instruction: 1.0,  // Will be calibrated
            hot_instruction_threshold: 500,
        }
    }

    /// Record execution of an instruction
    pub fn record_instruction(&mut self, address: usize, type_name: Option<&str>) {
        let stats = self.instruction_stats.entry(address).or_insert_with(ExecutionStats::new);
        stats.execution_count += 1;
        if let Some(typ) = type_name {
            stats.record_type(typ);
        }
    }

    /// Record a loop execution
    pub fn record_loop(&mut self, loop_start: usize) {
        let stats = self.loop_stats.entry(loop_start).or_insert_with(ExecutionStats::new);
        stats.execution_count += 1;
    }

    /// Record a value type being used in a loop
    pub fn record_loop_type(&mut self, loop_start: usize, type_name: &str) {
        let stats = self.loop_stats.entry(loop_start).or_insert_with(ExecutionStats::new);
        stats.record_type(type_name);
    }

    /// Get all hot instructions (candidates for specialization)
    pub fn get_hot_instructions(&self) -> Vec<(usize, &ExecutionStats)> {
        self.instruction_stats
            .iter()
            .filter(|(_addr, stats)| stats.is_hot())
            .map(|(addr, stats)| (*addr, stats))
            .collect()
    }

    /// Get all hot loops (candidates for JIT compilation)
    pub fn get_hot_loops(&self) -> Vec<(usize, &ExecutionStats)> {
        self.loop_stats
            .iter()
            .filter(|(_addr, stats)| stats.is_hot())
            .map(|(addr, stats)| (*addr, stats))
            .collect()
    }

    /// Get numeric-only hot loops (best candidates for specialization)
    pub fn get_numeric_loops(&self) -> Vec<(usize, &ExecutionStats)> {
        self.loop_stats
            .iter()
            .filter(|(_addr, stats)| stats.is_hot() && stats.is_numeric_only())
            .map(|(addr, stats)| (*addr, stats))
            .collect()
    }

    /// Mark a specialization attempt for an instruction/loop
    pub fn mark_specialization_attempt(&mut self, address: usize, was_successful: bool) {
        if let Some(stats) = self.instruction_stats.get_mut(&address) {
            stats.specialization_attempts += 1;
            if was_successful {
                stats.specialization_success_count += 1;
            }
        }
    }

    /// Print profiling report
    pub fn print_report(&self) {
        println!("\n=== Hot Path Detection Report ===");
        println!("Hot Instructions: {}", self.get_hot_instructions().len());
        println!("Hot Loops: {}", self.get_hot_loops().len());
        println!("Numeric-only Loops (optimization targets): {}", self.get_numeric_loops().len());
        
        if !self.get_numeric_loops().is_empty() {
            println!("\nTop optimization targets:");
            for (i, (addr, stats)) in self.get_numeric_loops().iter().take(5).enumerate() {
                println!("  {}. Loop @ 0x{:x}: {} executions, {:.1}% numeric",
                    i + 1,
                    addr,
                    stats.execution_count,
                    (stats.type_profile.get("Number").copied().unwrap_or(0) as f64
                        / stats.type_profile.values().sum::<usize>() as f64) * 100.0
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_path_detection() {
        let mut detector = HotPathDetector::new();
        
        // Simulate 600 numeric operations at address 100
        for _ in 0..600 {
            detector.record_instruction(100, Some("Number"));
        }
        
        let hot = detector.get_hot_instructions();
        assert!(!hot.is_empty());
        assert!(hot[0].1.is_hot());
    }

    #[test]
    fn test_numeric_loop_detection() {
        let mut detector = HotPathDetector::new();
        
        // Simulate numeric-only loop
        for _ in 0..500 {
            detector.record_loop(200);
            detector.record_loop_type(200, "Number");
        }
        
        let numeric_loops = detector.get_numeric_loops();
        assert!(!numeric_loops.is_empty());
        assert!(numeric_loops[0].1.is_numeric_only());
    }
}
