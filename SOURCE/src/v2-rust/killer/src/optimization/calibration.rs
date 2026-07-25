/// Phase 7C: Accuracy Calibration System
///
/// Refines Phase 5 prediction model based on real-world measurement data
/// Updates confidence factors and speedup multipliers iteratively

use std::collections::HashMap;
use crate::optimization::LoopType;

/// Confidence multiplier for speedup predictions
#[derive(Debug, Clone)]
pub struct ConfidenceFactor {
    /// Original prediction multiplier (1.0 = no adjustment)
    pub base_multiplier: f64,
    
    /// Accumulated adjustment factor
    pub adjustment: f64,
    
    /// Sample count for this type
    pub sample_count: usize,
    
    /// Recent error rate (0-1, 0=perfect)
    pub recent_error: f64,
}

impl ConfidenceFactor {
    /// Create new confidence factor
    pub fn new() -> Self {
        ConfidenceFactor {
            base_multiplier: 1.0,
            adjustment: 0.0,
            sample_count: 0,
            recent_error: 0.0,
        }
    }
    
    /// Get effective multiplier
    pub fn effective(&self) -> f64 {
        self.base_multiplier + self.adjustment
    }
    
    /// Get confidence score (0-1, 1=perfect)
    pub fn confidence(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.5;  // Default uncertainty
        }
        
        // Confidence decays with error
        let error_penalty = self.recent_error * 0.5;  // Up to 50% penalty
        let base = 1.0 - error_penalty;
        
        // Increase with sample count (up to 100 samples)
        let sample_bonus = (self.sample_count as f64 / 100.0).min(0.2);
        
        (base + sample_bonus).max(0.1).min(1.0)
    }
}

/// Calibration engine for refining predictions
#[derive(Debug)]
pub struct AccuracyCalibrator {
    /// Confidence factors per loop type
    factors: HashMap<LoopType, ConfidenceFactor>,
    
    /// Historical prediction-actual pairs
    history: Vec<(f64, f64)>,  // (predicted, actual)
    
    /// Calibration learning rate
    learning_rate: f64,
    
    /// Convergence threshold
    convergence_threshold: f64,
}

impl AccuracyCalibrator {
    /// Create new calibrator
    pub fn new(learning_rate: f64) -> Self {
        AccuracyCalibrator {
            factors: HashMap::new(),
            history: Vec::new(),
            learning_rate: learning_rate.max(0.01).min(0.5),  // Clamp to reasonable range
            convergence_threshold: 0.01,  // Stop when error < 1%
        }
    }
    
    /// Record a prediction-measurement pair
    pub fn record(
        &mut self,
        loop_type: LoopType,
        predicted_speedup: f64,
        actual_speedup: f64,
    ) {
        let factor = self.factors
            .entry(loop_type)
            .or_insert_with(ConfidenceFactor::new);
        
        // Calculate error
        let error = if predicted_speedup > 0.0 {
            (actual_speedup - predicted_speedup).abs() / predicted_speedup
        } else {
            1.0
        };
        
        // Update factor
        factor.sample_count += 1;
        factor.recent_error = factor.recent_error * 0.7 + error * 0.3;  // EMA update
        
        // Adjust multiplier based on error direction
        if actual_speedup > predicted_speedup {
            // Prediction was too conservative, increase multiplier
            factor.adjustment += self.learning_rate * error;
        } else {
            // Prediction was too optimistic, decrease multiplier
            factor.adjustment -= self.learning_rate * error;
        }
        
        // Keep adjustment within reasonable bounds
        factor.adjustment = factor.adjustment.max(-0.5).min(0.5);
        
        self.history.push((predicted_speedup, actual_speedup));
    }
    
    /// Get adjusted prediction for a loop type
    pub fn adjust_prediction(
        &self,
        loop_type: LoopType,
        raw_prediction: f64,
    ) -> f64 {
        let factor = self.factors
            .get(&loop_type)
            .cloned()
            .unwrap_or_else(ConfidenceFactor::new);
        
        raw_prediction * factor.effective()
    }
    
    /// Get confidence score for a loop type
    pub fn confidence_score(&self, loop_type: LoopType) -> f64 {
        self.factors
            .get(&loop_type)
            .map(|f| f.confidence())
            .unwrap_or(0.5)
    }
    
    /// Check if calibrator has converged
    pub fn has_converged(&self) -> bool {
        // All types must have recorded measurements
        if self.factors.is_empty() {
            return false;
        }
        
        // Check if errors are below threshold
        for factor in self.factors.values() {
            if factor.sample_count < 5 {
                return false;  // Not enough samples
            }
            if factor.recent_error > self.convergence_threshold {
                return false;  // Still too much error
            }
        }
        
        true
    }
    
    /// Get overall prediction accuracy
    pub fn overall_accuracy(&self) -> f64 {
        if self.history.is_empty() {
            return 0.5;
        }
        
        let mut total_error = 0.0;
        for (predicted, actual) in &self.history {
            if *predicted > 0.0 {
                let error = (actual - predicted).abs() / predicted;
                total_error += error;
            }
        }
        
        let avg_error = total_error / self.history.len() as f64;
        1.0 / (1.0 + avg_error)  // Convert to 0-1 accuracy
    }
    
    /// Get prediction accuracy by type
    pub fn accuracy_by_type(&self, loop_type: LoopType) -> Option<f64> {
        self.factors
            .get(&loop_type)
            .map(|f| 1.0 - f.recent_error)
    }
    
    /// Print calibration status
    pub fn print_status(&self) {
        println!("\n=== ACCURACY CALIBRATION STATUS ===\n");
        
        println!("Overall Accuracy: {:.1}%", self.overall_accuracy() * 100.0);
        println!("Learning Rate: {:.3}", self.learning_rate);
        println!("Convergence Threshold: {:.1}%\n", self.convergence_threshold * 100.0);
        
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(factor) = self.factors.get(loop_type) {
                println!("{}:", loop_type);
                println!("  Samples: {}", factor.sample_count);
                println!("  Base Multiplier: {:.3}", factor.base_multiplier);
                println!("  Adjustment: {:.3}", factor.adjustment);
                println!("  Effective: {:.3}x", factor.effective());
                println!("  Confidence: {:.1}%", factor.confidence() * 100.0);
                println!("  Recent Error: {:.1}%", factor.recent_error * 100.0);
            }
        }
        
        if self.has_converged() {
            println!("\n✅ Calibration CONVERGED");
        } else {
            println!("\n⏳ Still calibrating...");
        }
    }
    
    /// Get factor for a type
    pub fn get_factor(&self, loop_type: LoopType) -> Option<ConfidenceFactor> {
        self.factors.get(&loop_type).cloned()
    }
    
    /// Reset all factors
    pub fn reset(&mut self) {
        self.factors.clear();
        self.history.clear();
    }
    
    /// Get sample count for type
    pub fn sample_count(&self, loop_type: LoopType) -> usize {
        self.factors
            .get(&loop_type)
            .map(|f| f.sample_count)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calibrator_creation() {
        let cal = AccuracyCalibrator::new(0.1);
        assert!(!cal.has_converged());
        assert_eq!(cal.overall_accuracy(), 0.5);
    }
    
    #[test]
    fn test_record_and_adjust() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record prediction that's too conservative
        cal.record(LoopType::CpuBound, 10.0, 12.0);  // Actual > Predicted
        
        // Multiplier should increase
        let factor = cal.get_factor(LoopType::CpuBound).unwrap();
        assert!(factor.adjustment > 0.0, "Should increase multiplier when actual > predicted");
    }
    
    #[test]
    fn test_record_too_optimistic() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record prediction that's too optimistic
        cal.record(LoopType::MemoryBound, 10.0, 8.0);  // Actual < Predicted
        
        // Multiplier should decrease
        let factor = cal.get_factor(LoopType::MemoryBound).unwrap();
        assert!(factor.adjustment < 0.0, "Should decrease multiplier when actual < predicted");
    }
    
    #[test]
    fn test_convergence() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // Record many perfect predictions
        for _ in 0..10 {
            cal.record(LoopType::CpuBound, 5.0, 5.0);
            cal.record(LoopType::MemoryBound, 3.0, 3.0);
            cal.record(LoopType::Mixed, 4.0, 4.0);
        }
        
        // Should converge with perfect predictions
        assert!(cal.has_converged(), "Should converge with perfect predictions");
    }
    
    #[test]
    fn test_confidence_scores() {
        let mut cal = AccuracyCalibrator::new(0.1);
        
        // No measurements yet
        assert_eq!(cal.confidence_score(LoopType::CpuBound), 0.5);
        
        // Add measurements
        for _ in 0..5 {
            cal.record(LoopType::CpuBound, 10.0, 10.5);
        }
        
        // Should improve with samples
        let conf = cal.confidence_score(LoopType::CpuBound);
        assert!(conf > 0.5, "Confidence should improve with samples");
    }
    
    #[test]
    fn test_adjustment_bounds() {
        let mut cal = AccuracyCalibrator::new(0.5);  // High learning rate
        
        // Record extreme discrepancies
        for _ in 0..20 {
            cal.record(LoopType::Mixed, 1.0, 100.0);
        }
        
        let factor = cal.get_factor(LoopType::Mixed).unwrap();
        assert!(factor.adjustment <= 0.5, "Adjustment should be bounded at +0.5");
        assert!(factor.adjustment >= -0.5, "Adjustment should be bounded at -0.5");
    }
}
