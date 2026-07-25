/// Week 7: Production Deployment & Real-World Validation
///
/// Validates integrated optimizer on real applications and monitors prediction accuracy

use std::collections::HashMap;
use crate::optimization::{
    LoopType,
};

/// Performance profile from real-world application
#[derive(Debug, Clone)]
pub struct RealWorldProfile {
    /// Application identifier
    pub app_name: String,
    
    /// Loop unique identifier
    pub loop_id: String,
    
    /// Classified loop type
    pub loop_type: LoopType,
    
    /// Predicted speedup (from integrated optimizer)
    pub predicted_speedup: f64,
    
    /// Actual measured speedup (real deployment)
    pub actual_speedup: f64,
    
    /// Prediction accuracy (predicted / actual ratio)
    pub accuracy: f64,
    
    /// Conservative estimate (30-50% of predicted)
    pub conservative_estimate: f64,
    
    /// Whether conservative estimate was accurate
    pub conservative_accurate: bool,
    
    /// Baseline execution time (ms)
    pub baseline_ms: f64,
    
    /// Optimized execution time (ms)
    pub optimized_ms: f64,
}

/// Production deployment tracker
#[derive(Debug)]
pub struct ProductionOptimizer {
    /// Real-world application profiles
    profiles: Vec<RealWorldProfile>,
    
    /// Accuracy metrics by loop type
    accuracy_by_type: HashMap<LoopType, Vec<f64>>,
    
    /// Conservative estimate effectiveness
    conservative_effectiveness: Vec<bool>,
    
    /// Overall statistics
    total_deployments: usize,
    total_speedup_sum: f64,
}

impl ProductionOptimizer {
    /// Create new production optimizer
    pub fn new() -> Self {
        ProductionOptimizer {
            profiles: Vec::new(),
            accuracy_by_type: HashMap::new(),
            conservative_effectiveness: Vec::new(),
            total_deployments: 0,
            total_speedup_sum: 0.0,
        }
    }
    
    /// Record real-world deployment result
    pub fn record_deployment(
        &mut self,
        app_name: String,
        loop_id: String,
        loop_type: LoopType,
        predicted_speedup: f64,
        actual_speedup: f64,
        baseline_ms: f64,
        optimized_ms: f64,
    ) -> RealWorldProfile {
        // Calculate accuracy
        let accuracy = if predicted_speedup > 1.0 && actual_speedup > 1.0 {
            let ratio = predicted_speedup / actual_speedup;
            if ratio > 1.0 { 1.0 / ratio } else { ratio }
        } else {
            0.0
        };
        
        // Conservative estimate (30-50% of predicted)
        let conservative_estimate = predicted_speedup * 0.4;  // 40% = middle of 30-50% range
        let conservative_accurate = 
            actual_speedup >= conservative_estimate;  // Conservative is accurate if actual meets it
        
        let profile = RealWorldProfile {
            app_name,
            loop_id,
            loop_type,
            predicted_speedup,
            actual_speedup,
            accuracy: accuracy.min(1.0).max(0.01),
            conservative_estimate,
            conservative_accurate,
            baseline_ms,
            optimized_ms,
        };
        
        // Track metrics
        self.accuracy_by_type
            .entry(loop_type)
            .or_insert_with(Vec::new)
            .push(accuracy);
        
        self.conservative_effectiveness.push(conservative_accurate);
        self.total_deployments += 1;
        self.total_speedup_sum += actual_speedup;
        
        self.profiles.push(profile.clone());
        profile
    }
    
    /// Get average speedup by loop type
    pub fn avg_speedup_by_type(&self, loop_type: LoopType) -> Option<f64> {
        let type_profiles: Vec<_> = self.profiles
            .iter()
            .filter(|p| p.loop_type == loop_type)
            .collect();
        
        if type_profiles.is_empty() {
            return None;
        }
        
        let sum: f64 = type_profiles.iter().map(|p| p.actual_speedup).sum();
        Some(sum / type_profiles.len() as f64)
    }
    
    /// Get prediction accuracy by loop type
    pub fn accuracy_by_type(&self, loop_type: LoopType) -> Option<f64> {
        self.accuracy_by_type
            .get(&loop_type)
            .map(|accs| accs.iter().sum::<f64>() / accs.len() as f64)
    }
    
    /// Get conservative estimate effectiveness
    pub fn conservative_effectiveness_rate(&self) -> f64 {
        if self.conservative_effectiveness.is_empty() {
            return 0.0;
        }
        
        let accurate_count = self.conservative_effectiveness
            .iter()
            .filter(|&&v| v)
            .count();
        
        accurate_count as f64 / self.conservative_effectiveness.len() as f64
    }
    
    /// Get average actual speedup across all deployments
    pub fn overall_avg_speedup(&self) -> f64 {
        if self.total_deployments == 0 {
            return 0.0;
        }
        
        self.total_speedup_sum / self.total_deployments as f64
    }
    
    /// Print production deployment summary
    pub fn print_summary(&self) {
        println!("\n=== PRODUCTION DEPLOYMENT SUMMARY ===\n");
        
        if self.profiles.is_empty() {
            println!("No deployments recorded yet.");
            return;
        }
        
        println!("Total Deployments: {}", self.total_deployments);
        println!("Overall Average Speedup: {:.2}x\n", self.overall_avg_speedup());
        
        // By loop type
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(avg_speedup) = self.avg_speedup_by_type(*loop_type) {
                if let Some(accuracy) = self.accuracy_by_type(*loop_type) {
                    println!("{}:", loop_type);
                    println!("  Average Speedup: {:.2}x", avg_speedup);
                    println!("  Prediction Accuracy: {:.1}%", accuracy * 100.0);
                    
                    let type_profiles: Vec<_> = self.profiles
                        .iter()
                        .filter(|p| p.loop_type == *loop_type)
                        .collect();
                    println!("  Count: {}\n", type_profiles.len());
                }
            }
        }
        
        // Conservative estimate effectiveness
        let conservative_rate = self.conservative_effectiveness_rate();
        println!("Conservative Estimate Effectiveness: {:.1}%", conservative_rate * 100.0);
        
        if conservative_rate > 0.8 {
            println!("✅ Conservative estimates VERY EFFECTIVE");
        } else if conservative_rate > 0.6 {
            println!("✅ Conservative estimates EFFECTIVE");
        } else if conservative_rate > 0.4 {
            println!("⚠️  Conservative estimates REASONABLE");
        } else {
            println!("❌ Conservative estimates need REFINEMENT");
        }
        
        // Speedup distribution
        println!("\n=== Speedup Distribution ===");
        
        let mut speedups: Vec<_> = self.profiles.iter().map(|p| p.actual_speedup).collect();
        speedups.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        println!("Min: {:.2}x", speedups.first().copied().unwrap_or(0.0));
        println!("Max: {:.2}x", speedups.last().copied().unwrap_or(0.0));
        
        if speedups.len() > 1 {
            let mid = speedups.len() / 2;
            println!("Median: {:.2}x", speedups[mid]);
        }
    }
    
    /// Get all profiles
    pub fn profiles(&self) -> &[RealWorldProfile] {
        &self.profiles
    }
    
    /// Get total deployments
    pub fn total_deployments(&self) -> usize {
        self.total_deployments
    }
    
    /// Export results for analysis
    pub fn export_csv(&self) -> String {
        let mut csv = String::from("app_name,loop_id,loop_type,predicted,actual,accuracy,conservative,conservative_accurate\n");
        
        for profile in &self.profiles {
            csv.push_str(&format!(
                "{},{},{},{:.2},{:.2},{:.3},{:.2},{}\n",
                profile.app_name,
                profile.loop_id,
                profile.loop_type,
                profile.predicted_speedup,
                profile.actual_speedup,
                profile.accuracy,
                profile.conservative_estimate,
                profile.conservative_accurate
            ));
        }
        
        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_production_optimizer_creation() {
        let optimizer = ProductionOptimizer::new();
        assert_eq!(optimizer.total_deployments, 0);
        assert_eq!(optimizer.overall_avg_speedup(), 0.0);
    }
    
    #[test]
    fn test_record_deployment() {
        let mut optimizer = ProductionOptimizer::new();
        
        let profile = optimizer.record_deployment(
            "matrix_multiply".to_string(),
            "loop_1".to_string(),
            LoopType::CpuBound,
            10.0,    // predicted
            12.5,    // actual
            10.0,    // baseline_ms
            0.8,     // optimized_ms
        );
        
        assert_eq!(profile.loop_type, LoopType::CpuBound);
        assert_eq!(profile.predicted_speedup, 10.0);
        assert_eq!(profile.actual_speedup, 12.5);
        assert!(profile.accuracy > 0.7);  // Should be reasonably close
        assert_eq!(optimizer.total_deployments, 1);
    }
    
    #[test]
    fn test_conservative_estimate_tracking() {
        let mut optimizer = ProductionOptimizer::new();
        
        // Deployment where actual is within 80-120% of conservative estimate
        optimizer.record_deployment(
            "test_app".to_string(),
            "loop_1".to_string(),
            LoopType::MemoryBound,
            10.0,    // predicted
            4.5,     // actual (conservative is 4.0, actual is 112% of conservative)
            5.0,
            1.1,
        );
        
        let effectiveness = optimizer.conservative_effectiveness_rate();
        assert!(effectiveness > 0.0);  // Should record effectiveness
    }
    
    #[test]
    fn test_accuracy_by_type() {
        let mut optimizer = ProductionOptimizer::new();
        
        optimizer.record_deployment(
            "app1".to_string(),
            "loop_1".to_string(),
            LoopType::CpuBound,
            10.0,
            10.5,
            10.0,
            0.95,
        );
        
        optimizer.record_deployment(
            "app2".to_string(),
            "loop_2".to_string(),
            LoopType::MemoryBound,
            5.0,
            5.2,
            5.0,
            0.96,
        );
        
        let cpu_accuracy = optimizer.accuracy_by_type(LoopType::CpuBound);
        assert!(cpu_accuracy.is_some());
        assert!(cpu_accuracy.unwrap() > 0.9);
    }
}
