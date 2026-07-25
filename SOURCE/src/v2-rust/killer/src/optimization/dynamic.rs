/// Phase 8B: Dynamic Optimization Module
/// Implements runtime feedback and adaptive optimization parameter adjustment
/// Enables real-time optimization refinement based on production metrics

use std::collections::HashMap;

/// Performance feedback from runtime monitoring
#[derive(Debug, Clone)]
pub struct PerformanceFeedback {
    /// Predicted speedup (from Phase 5)
    pub predicted_speedup: f64,
    /// Actual speedup achieved
    pub actual_speedup: f64,
    /// Prediction error percentage (0.0 = perfect)
    pub prediction_error: f64,
    /// Memory pressure (0.0-1.0, where 1.0 = memory intensive)
    pub memory_pressure: f64,
    /// Cache hit rate (0.0-1.0)
    pub cache_hit_rate: f64,
    /// Thermal throttling detected (0.0-1.0)
    pub thermal_throttling: f64,
    /// CPU utilization (0.0-1.0)
    pub cpu_utilization: f64,
}

impl PerformanceFeedback {
    /// Create feedback from monitoring data
    pub fn new(predicted: f64, actual: f64) -> Self {
        let error = if predicted > 0.0 {
            ((predicted - actual) / predicted).abs()
        } else {
            0.0
        };

        PerformanceFeedback {
            predicted_speedup: predicted,
            actual_speedup: actual,
            prediction_error: error,
            memory_pressure: 0.5,
            cache_hit_rate: 0.75,
            thermal_throttling: 0.0,
            cpu_utilization: 0.8,
        }
    }

    /// Is prediction error acceptable?
    pub fn error_acceptable(&self, threshold: f64) -> bool {
        self.prediction_error < threshold
    }

    /// Is performance acceptable compared to baseline?
    pub fn performance_acceptable(&self, minimum_speedup: f64) -> bool {
        self.actual_speedup >= minimum_speedup
    }
}

/// Adaptation strategy for parameter adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdaptationStrategy {
    /// Try smaller block sizes (reduce memory pressure)
    ReduceBlockSize,
    /// Try larger block sizes (improve cache reuse)
    IncreaseBlockSize,
    /// Reduce vector width (less register pressure)
    ReduceVectorWidth,
    /// Increase vector width (better parallelism)
    IncreaseVectorWidth,
    /// Adjust loop unrolling factor
    AdjustUnrolling,
    /// Disable problematic optimization
    DisableOptimization,
    /// No adaptation needed
    Stable,
}

/// Parameter adjustment result
#[derive(Debug, Clone)]
pub struct ParameterAdjustment {
    /// Recommendation for parameter change
    pub strategy: AdaptationStrategy,
    /// New value to use
    pub new_value: f64,
    /// Confidence in change (0.0-1.0)
    pub confidence: f64,
    /// Reason for adjustment
    pub reason: String,
}

/// Dynamic Optimizer for runtime feedback and adaptation
#[derive(Debug, Clone)]
pub struct DynamicOptimizer {
    /// Current parameters
    pub parameters: HashMap<String, f64>,
    /// Historical feedback (loop_id -> [feedback1, feedback2, ...])
    pub feedback_history: HashMap<String, Vec<PerformanceFeedback>>,
    /// Adaptation recommendations per loop
    pub adaptations: HashMap<String, Vec<ParameterAdjustment>>,
    /// Learning rate for parameter adjustment (0.01-0.5)
    pub learning_rate: f64,
    /// Error threshold for triggering adaptation (default 0.3 = 30%)
    pub error_threshold: f64,
}

impl DynamicOptimizer {
    /// Create a new dynamic optimizer
    pub fn new() -> Self {
        let mut params = HashMap::new();
        params.insert("block_size".to_string(), 1024.0);
        params.insert("vector_width".to_string(), 256.0);
        params.insert("unroll_factor".to_string(), 4.0);

        DynamicOptimizer {
            parameters: params,
            feedback_history: HashMap::new(),
            adaptations: HashMap::new(),
            learning_rate: 0.15,
            error_threshold: 0.30,
        }
    }

    /// Record feedback for a loop
    pub fn record_feedback(&mut self, loop_id: &str, feedback: PerformanceFeedback) {
        self.feedback_history
            .entry(loop_id.to_string())
            .or_insert_with(Vec::new)
            .push(feedback);
    }

    /// Analyze feedback and suggest adaptations
    pub fn analyze(&mut self, loop_id: &str) -> Option<ParameterAdjustment> {
        let history = self.feedback_history.get(loop_id)?;

        if history.is_empty() {
            return None;
        }

        let latest = &history[history.len() - 1];

        // Check if error is too high
        if latest.prediction_error > self.error_threshold {
            return Some(self.recommend_adaptation(loop_id, latest));
        }

        // Check for memory pressure issues
        if latest.memory_pressure > 0.8 && latest.cache_hit_rate < 0.6 {
            return Some(ParameterAdjustment {
                strategy: AdaptationStrategy::ReduceBlockSize,
                new_value: self.parameters.get("block_size").copied().unwrap_or(1024.0) * 0.8,
                confidence: 0.7,
                reason: "High memory pressure with low cache hits".to_string(),
            });
        }

        // Check for thermal throttling
        if latest.thermal_throttling > 0.5 {
            return Some(ParameterAdjustment {
                strategy: AdaptationStrategy::ReduceVectorWidth,
                new_value: self.parameters.get("vector_width").copied().unwrap_or(256.0) * 0.75,
                confidence: 0.8,
                reason: "Thermal throttling detected".to_string(),
            });
        }

        None
    }

    /// Recommend an adaptation strategy
    fn recommend_adaptation(&self, loop_id: &str, feedback: &PerformanceFeedback) -> ParameterAdjustment {
        if feedback.actual_speedup < feedback.predicted_speedup * 0.5 {
            // Severely underperforming
            ParameterAdjustment {
                strategy: AdaptationStrategy::DisableOptimization,
                new_value: 1.0,
                confidence: 0.9,
                reason: format!(
                    "Loop {} actual {:.2}x severely below predicted {:.2}x",
                    loop_id, feedback.actual_speedup, feedback.predicted_speedup
                ),
            }
        } else if feedback.memory_pressure > 0.7 {
            // Memory-intensive
            ParameterAdjustment {
                strategy: AdaptationStrategy::ReduceBlockSize,
                new_value: self.parameters.get("block_size").copied().unwrap_or(1024.0) * 0.85,
                confidence: 0.7,
                reason: format!("High memory pressure ({:.1}%)", feedback.memory_pressure * 100.0),
            }
        } else if feedback.cache_hit_rate < 0.6 {
            // Poor cache performance
            ParameterAdjustment {
                strategy: AdaptationStrategy::IncreaseBlockSize,
                new_value: self.parameters.get("block_size").copied().unwrap_or(1024.0) * 1.2,
                confidence: 0.6,
                reason: format!("Low cache hit rate ({:.1}%)", feedback.cache_hit_rate * 100.0),
            }
        } else {
            // Default: try smaller blocks to improve cache
            ParameterAdjustment {
                strategy: AdaptationStrategy::ReduceBlockSize,
                new_value: self.parameters.get("block_size").copied().unwrap_or(1024.0) * 0.9,
                confidence: 0.5,
                reason: format!(
                    "Prediction error {:.1}% exceeds threshold {:.1}%",
                    feedback.prediction_error * 100.0,
                    self.error_threshold * 100.0
                ),
            }
        }
    }

    /// Apply an adjustment to parameters
    pub fn apply_adjustment(&mut self, adjustment: &ParameterAdjustment) {
        match adjustment.strategy {
            AdaptationStrategy::ReduceBlockSize | AdaptationStrategy::IncreaseBlockSize => {
                self.parameters.insert("block_size".to_string(), adjustment.new_value);
            }
            AdaptationStrategy::ReduceVectorWidth | AdaptationStrategy::IncreaseVectorWidth => {
                self.parameters.insert("vector_width".to_string(), adjustment.new_value);
            }
            AdaptationStrategy::AdjustUnrolling => {
                self.parameters.insert("unroll_factor".to_string(), adjustment.new_value);
            }
            AdaptationStrategy::DisableOptimization => {
                self.parameters.insert("optimization_enabled".to_string(), 0.0);
            }
            AdaptationStrategy::Stable => {
                // No change
            }
        }
    }

    /// Get average prediction error for a loop
    pub fn average_error(&self, loop_id: &str) -> f64 {
        let history = match self.feedback_history.get(loop_id) {
            Some(h) => h,
            None => return 0.0,
        };

        if history.is_empty() {
            return 0.0;
        }

        let sum: f64 = history.iter().map(|f| f.prediction_error).sum();
        sum / history.len() as f64
    }

    /// Has a loop converged (error stable and acceptable)?
    pub fn has_converged(&self, loop_id: &str) -> bool {
        let history = match self.feedback_history.get(loop_id) {
            Some(h) => h,
            None => return false,
        };

        if history.len() < 3 {
            return false;
        }

        // Check if last 3 errors are within tolerance
        let recent_errors: Vec<f64> = history.iter().rev().take(3).map(|f| f.prediction_error).collect();

        if recent_errors.is_empty() {
            return false;
        }

        let avg_error = recent_errors.iter().sum::<f64>() / recent_errors.len() as f64;
        let variance: f64 = recent_errors
            .iter()
            .map(|e| (e - avg_error).powi(2))
            .sum::<f64>()
            / recent_errors.len() as f64;

        // Converged if error is low and stable
        avg_error < self.error_threshold && variance < 0.01
    }

    /// Get status report
    pub fn status_report(&self) -> String {
        let total_feedback = self.feedback_history.values().map(|v| v.len()).sum::<usize>();
        format!(
            "DynamicOptimizer (Parameters: {}, Feedback samples: {}, Learning rate: {:.3})",
            self.parameters.len(),
            total_feedback,
            self.learning_rate
        )
    }
}

impl Default for DynamicOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_feedback_creation() {
        let feedback = PerformanceFeedback::new(5.0, 4.8);
        assert!(feedback.prediction_error < 0.1);
        assert_eq!(feedback.predicted_speedup, 5.0);
        assert_eq!(feedback.actual_speedup, 4.8);
    }

    #[test]
    fn test_error_acceptable() {
        let good_feedback = PerformanceFeedback::new(5.0, 4.8);  // 4% error
        let bad_feedback = PerformanceFeedback::new(5.0, 2.0);   // 60% error

        assert!(good_feedback.error_acceptable(0.1));
        assert!(!bad_feedback.error_acceptable(0.1));
    }

    #[test]
    fn test_record_and_analyze_feedback() {
        let mut optimizer = DynamicOptimizer::new();

        let feedback1 = PerformanceFeedback::new(5.0, 4.2);
        let feedback2 = PerformanceFeedback::new(5.0, 4.5);

        optimizer.record_feedback("loop_1", feedback1.clone());
        optimizer.record_feedback("loop_1", feedback2.clone());

        assert_eq!(optimizer.feedback_history.get("loop_1").unwrap().len(), 2);
    }

    #[test]
    fn test_analyze_high_error_triggers_adaptation() {
        let mut optimizer = DynamicOptimizer::new();
        optimizer.error_threshold = 0.3;

        let bad_feedback = PerformanceFeedback::new(5.0, 2.0);  // 60% error
        optimizer.record_feedback("loop_bad", bad_feedback);

        let adaptation = optimizer.analyze("loop_bad");
        assert!(adaptation.is_some());
    }

    #[test]
    fn test_memory_pressure_triggers_block_size_reduction() {
        let mut optimizer = DynamicOptimizer::new();

        let mut feedback = PerformanceFeedback::new(4.0, 3.8);
        feedback.memory_pressure = 0.85;
        feedback.cache_hit_rate = 0.55;

        optimizer.record_feedback("loop_memory", feedback);

        let adaptation = optimizer.analyze("loop_memory");
        assert!(adaptation.is_some());

        let adj = adaptation.unwrap();
        assert_eq!(adj.strategy, AdaptationStrategy::ReduceBlockSize);
    }

    #[test]
    fn test_thermal_throttling_detected() {
        let mut optimizer = DynamicOptimizer::new();

        let mut feedback = PerformanceFeedback::new(4.0, 3.5);
        feedback.thermal_throttling = 0.6;

        optimizer.record_feedback("loop_thermal", feedback);

        let adaptation = optimizer.analyze("loop_thermal");
        assert!(adaptation.is_some());

        let adj = adaptation.unwrap();
        assert_eq!(adj.strategy, AdaptationStrategy::ReduceVectorWidth);
    }

    #[test]
    fn test_apply_adjustment_updates_parameters() {
        let mut optimizer = DynamicOptimizer::new();

        let original_block = optimizer.parameters.get("block_size").copied().unwrap_or(1024.0);

        let adjustment = ParameterAdjustment {
            strategy: AdaptationStrategy::ReduceBlockSize,
            new_value: original_block * 0.8,
            confidence: 0.7,
            reason: "Test".to_string(),
        };

        optimizer.apply_adjustment(&adjustment);

        let new_block = optimizer.parameters.get("block_size").copied().unwrap_or(1024.0);
        assert!((new_block - original_block * 0.8).abs() < 0.01);
    }

    #[test]
    fn test_convergence_detection() {
        let mut optimizer = DynamicOptimizer::new();

        // Add 3 samples with similar, low error
        for _ in 0..3 {
            let feedback = PerformanceFeedback::new(4.0, 3.95);
            optimizer.record_feedback("loop_stable", feedback);
        }

        assert!(optimizer.has_converged("loop_stable"));
    }

    #[test]
    fn test_average_error_calculation() {
        let mut optimizer = DynamicOptimizer::new();

        let feed1 = PerformanceFeedback::new(5.0, 4.5);  // 10% error
        let feed2 = PerformanceFeedback::new(4.0, 3.6);  // 10% error
        let feed3 = PerformanceFeedback::new(3.0, 2.7);  // 10% error

        optimizer.record_feedback("loop_avg", feed1);
        optimizer.record_feedback("loop_avg", feed2);
        optimizer.record_feedback("loop_avg", feed3);

        let avg = optimizer.average_error("loop_avg");
        assert!((avg - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_status_report() {
        let mut optimizer = DynamicOptimizer::new();

        let feedback = PerformanceFeedback::new(4.0, 3.8);
        optimizer.record_feedback("loop_status", feedback);

        let report = optimizer.status_report();
        assert!(report.contains("DynamicOptimizer"));
        assert!(report.contains("1"));  // 1 feedback sample
    }
}
