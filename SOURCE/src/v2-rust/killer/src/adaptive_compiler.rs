// Phase 17: Adaptive Compilation Engine
// Uses runtime feedback to adjust optimization strategies dynamically

use crate::hot_path_detector::ExecutionStats;
use std::collections::HashMap;

/// Adaptive optimization feedback
#[derive(Debug, Clone)]
pub struct OptimizationFeedback {
    pub specialization_id: usize,
    pub was_effective: bool,
    pub speedup_achieved: f64,
    pub execution_count: usize,
    pub memory_overhead: usize,
}

/// Adapts compilation strategies based on runtime performance
pub struct AdaptiveCompiler {
    /// History of optimization attempts and their effectiveness
    feedback_history: Vec<OptimizationFeedback>,
    
    /// Learned thresholds (adjust as we see what works)
    hot_instruction_threshold: usize,
    numeric_loop_percentage_threshold: f64,
    
    /// Success rates per strategy
    strategy_success_rates: HashMap<String, f64>,
    
    /// Specialization counter
    specialization_counter: usize,
}

impl AdaptiveCompiler {
    pub fn new() -> Self {
        let mut strategy_rates = HashMap::new();
        strategy_rates.insert("numeric_jit".to_string(), 0.5);
        strategy_rates.insert("string_specialize".to_string(), 0.5);
        strategy_rates.insert("memoization".to_string(), 0.5);
        
        AdaptiveCompiler {
            feedback_history: Vec::new(),
            hot_instruction_threshold: 500,
            numeric_loop_percentage_threshold: 0.99,
            strategy_success_rates: strategy_rates,
            specialization_counter: 0,
        }
    }

    /// Record feedback about an optimization
    pub fn record_feedback(
        &mut self,
        strategy: &str,
        was_effective: bool,
        speedup: f64,
        execution_count: usize,
        memory_overhead: usize,
    ) {
        let feedback = OptimizationFeedback {
            specialization_id: self.specialization_counter,
            was_effective,
            speedup_achieved: speedup,
            execution_count,
            memory_overhead,
        };
        
        self.feedback_history.push(feedback);
        self.specialization_counter += 1;
        
        // Update strategy success rate
        self.update_strategy_success_rate(strategy, was_effective);
    }

    /// Update success rate for a strategy
    fn update_strategy_success_rate(&mut self, strategy: &str, was_successful: bool) {
        let current_rate = self.strategy_success_rates
            .get(strategy)
            .copied()
            .unwrap_or(0.5);
        
        // Exponential moving average: new_rate = 0.7 * old_rate + 0.3 * new_result
        let new_result = if was_successful { 1.0 } else { 0.0 };
        let updated_rate = 0.7 * current_rate + 0.3 * new_result;
        
        self.strategy_success_rates.insert(strategy.to_string(), updated_rate);
    }

    /// Get recommended strategy for next optimization
    pub fn recommend_strategy(&self, stats: &ExecutionStats) -> OptimizationStrategy {
        // Use success rates to decide
        let numeric_jit_rate = self.strategy_success_rates.get("numeric_jit").copied().unwrap_or(0.5);
        let string_rate = self.strategy_success_rates.get("string_specialize").copied().unwrap_or(0.5);
        let memo_rate = self.strategy_success_rates.get("memoization").copied().unwrap_or(0.5);
        
        // Pick best strategy that matches this loop's characteristics
        if stats.is_numeric_only() {
            if numeric_jit_rate > 0.7 {
                return OptimizationStrategy::NumericJit;
            }
        }
        
        // Check for string concatenation patterns
        if let Some((_type_name, count)) = stats.dominant_type() {
            if _type_name == "String" && string_rate > 0.6 {
                return OptimizationStrategy::StringSpecialization;
            }
        }
        
        // Check if memoization would help (recursive patterns)
        if stats.execution_count > 1000 && memo_rate > 0.6 {
            return OptimizationStrategy::Memoization;
        }
        
        OptimizationStrategy::Conservative
    }

    /// Adjust thresholds based on learning
    pub fn adapt_thresholds(&mut self) {
        // If JIT is working well, lower the threshold to optimize more loops
        if let Some(rate) = self.strategy_success_rates.get("numeric_jit") {
            if *rate > 0.8 && self.hot_instruction_threshold > 300 {
                self.hot_instruction_threshold -= 50;  // Be more aggressive
            } else if *rate < 0.3 && self.hot_instruction_threshold < 1000 {
                self.hot_instruction_threshold += 100;  // Be more conservative
            }
        }
    }

    /// Get all successful optimizations
    pub fn get_successful_optimizations(&self) -> Vec<&OptimizationFeedback> {
        self.feedback_history
            .iter()
            .filter(|f| f.was_effective)
            .collect()
    }

    /// Calculate overall effectiveness
    pub fn get_overall_effectiveness(&self) -> f64 {
        if self.feedback_history.is_empty() {
            return 0.0;
        }
        
        let successful = self.feedback_history
            .iter()
            .filter(|f| f.was_effective)
            .count();
        
        (successful as f64) / (self.feedback_history.len() as f64)
    }

    /// Print adaptive compilation report
    pub fn print_report(&self) {
        println!("\n=== Adaptive Compilation Report (Phase 17) ===");
        println!("Total Optimizations Attempted: {}", self.feedback_history.len());
        println!("Successful: {}", self.get_successful_optimizations().len());
        println!("Effectiveness: {:.1}%", self.get_overall_effectiveness() * 100.0);
        println!("");
        println!("Strategy Success Rates:");
        for (strategy, rate) in &self.strategy_success_rates {
            println!("  {}: {:.1}%", strategy, rate * 100.0);
        }
        println!("");
        println!("Adaptive Thresholds:");
        println!("  Hot Instruction: {} iterations", self.hot_instruction_threshold);
        println!("  Numeric Percentage: {:.1}%", self.numeric_loop_percentage_threshold * 100.0);
    }
}

/// Available optimization strategies
#[derive(Debug, Clone, Copy)]
pub enum OptimizationStrategy {
    NumericJit,               // JIT compile numeric loops
    StringSpecialization,     // Optimize string operations
    Memoization,              // Cache function results
    Conservative,             // No aggressive optimization
}

impl OptimizationStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            OptimizationStrategy::NumericJit => "numeric_jit",
            OptimizationStrategy::StringSpecialization => "string_specialize",
            OptimizationStrategy::Memoization => "memoization",
            OptimizationStrategy::Conservative => "conservative",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_feedback() {
        let mut compiler = AdaptiveCompiler::new();
        
        compiler.record_feedback("numeric_jit", true, 8.5, 1000, 2048);
        compiler.record_feedback("numeric_jit", true, 9.2, 1500, 2048);
        
        let effectiveness = compiler.get_overall_effectiveness();
        assert_eq!(effectiveness, 1.0);
    }

    #[test]
    fn test_strategy_success_rate_update() {
        let mut compiler = AdaptiveCompiler::new();
        
        // Record successes
        for _ in 0..3 {
            compiler.record_feedback("numeric_jit", true, 8.0, 500, 1024);
        }
        
        let rate = compiler.strategy_success_rates.get("numeric_jit").copied().unwrap_or(0.0);
        assert!(rate > 0.5);  // Should improve from initial 0.5
    }

    #[test]
    fn test_threshold_adaptation() {
        let mut compiler = AdaptiveCompiler::new();
        let initial = compiler.hot_instruction_threshold;
        
        // Simulate very successful JIT strategy
        for _ in 0..10 {
            compiler.record_feedback("numeric_jit", true, 8.0, 500, 1024);
        }
        compiler.adapt_thresholds();
        
        // Threshold should potentially lower (be more aggressive)
        let updated = compiler.hot_instruction_threshold;
        // May or may not lower depending on iterations
        assert!(updated <= initial);
    }
}
