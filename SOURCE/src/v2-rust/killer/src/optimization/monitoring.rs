/// Phase 7D: Performance Monitoring System
///
/// Real-time tracking of prediction accuracy and performance metrics
/// Powers accuracy calibration and deployment health monitoring

use std::collections::HashMap;
use crate::optimization::LoopType;

/// Real-time performance metric snapshot
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
    
    /// Application identifier
    pub app_name: String,
    
    /// Loop identifier
    pub loop_id: String,
    
    /// Loop classification
    pub loop_type: LoopType,
    
    /// Phase 5 prediction
    pub predicted_speedup: f64,
    
    /// Actual measured speedup
    pub actual_speedup: f64,
    
    /// Prediction error (% difference)
    pub prediction_error: f64,
    
    /// Execution time baseline (ms)
    pub baseline_ms: f64,
    
    /// Execution time optimized (ms)
    pub optimized_ms: f64,
}

/// Performance monitor for tracking real-time metrics
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Historical snapshots
    snapshots: Vec<PerformanceSnapshot>,
    
    /// Rolling accuracy by type (last N measurements)
    accuracy_windows: HashMap<LoopType, Vec<f64>>,
    
    /// Alert thresholds
    accuracy_alert_threshold: f64,  // Alert if accuracy < this
    speedup_min_threshold: f64,     // Alert if speedup < this
    
    /// Configuration
    window_size: usize,             // Size of rolling window
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new(window_size: usize) -> Self {
        PerformanceMonitor {
            snapshots: Vec::new(),
            accuracy_windows: HashMap::new(),
            accuracy_alert_threshold: 0.5,  // Alert if <50% accurate
            speedup_min_threshold: 1.5,     // Alert if <1.5x
            window_size,
        }
    }
    
    /// Record a performance measurement
    pub fn record(
        &mut self,
        timestamp: u64,
        app_name: String,
        loop_id: String,
        loop_type: LoopType,
        predicted_speedup: f64,
        actual_speedup: f64,
        baseline_ms: f64,
        optimized_ms: f64,
    ) -> PerformanceSnapshot {
        // Calculate prediction error
        let prediction_error = if predicted_speedup > 1.0 {
            (actual_speedup - predicted_speedup).abs() / predicted_speedup
        } else {
            0.0
        };
        
        let snapshot = PerformanceSnapshot {
            timestamp,
            app_name,
            loop_id,
            loop_type,
            predicted_speedup,
            actual_speedup,
            prediction_error,
            baseline_ms,
            optimized_ms,
        };
        
        // Update rolling accuracy window
        let accuracy = if prediction_error > 0.0 {
            1.0 / (1.0 + prediction_error)  // 0-1 scale where 1=perfect
        } else {
            1.0
        };
        
        self.accuracy_windows
            .entry(loop_type)
            .or_insert_with(Vec::new)
            .push(accuracy);
        
        // Keep window size limited
        if let Some(window) = self.accuracy_windows.get_mut(&loop_type) {
            if window.len() > self.window_size {
                window.remove(0);
            }
        }
        
        self.snapshots.push(snapshot.clone());
        snapshot
    }
    
    /// Get current rolling accuracy for a type
    pub fn current_accuracy(&self, loop_type: LoopType) -> Option<f64> {
        self.accuracy_windows
            .get(&loop_type)
            .map(|accs| accs.iter().sum::<f64>() / accs.len() as f64)
    }
    
    /// Check if accuracy alert should trigger
    pub fn accuracy_alert(&self, loop_type: LoopType) -> bool {
        self.current_accuracy(loop_type)
            .map(|acc| acc < self.accuracy_alert_threshold)
            .unwrap_or(false)
    }
    
    /// Check if speedup alert should trigger
    pub fn speedup_alert(&self) -> bool {
        if let Some(latest) = self.snapshots.last() {
            return latest.actual_speedup < self.speedup_min_threshold;
        }
        false
    }
    
    /// Get average speedup by type (recent)
    pub fn avg_speedup_recent(&self, loop_type: LoopType) -> Option<f64> {
        let recent: Vec<_> = self.snapshots
            .iter()
            .rev()
            .take(self.window_size)
            .filter(|s| s.loop_type == loop_type)
            .collect();
        
        if recent.is_empty() {
            return None;
        }
        
        Some(recent.iter().map(|s| s.actual_speedup).sum::<f64>() / recent.len() as f64)
    }
    
    /// Get prediction error statistics
    pub fn prediction_error_stats(&self) -> (f64, f64, f64) {
        if self.snapshots.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        let errors: Vec<_> = self.snapshots
            .iter()
            .map(|s| s.prediction_error)
            .collect();
        
        let min = errors.iter().cloned().fold(f64::MAX, f64::min);
        let max = errors.iter().cloned().fold(0.0, f64::max);
        let avg = errors.iter().sum::<f64>() / errors.len() as f64;
        
        (min, avg, max)
    }
    
    /// Print monitoring dashboard
    pub fn print_dashboard(&self) {
        println!("\n=== PERFORMANCE MONITORING DASHBOARD ===\n");
        
        if self.snapshots.is_empty() {
            println!("No measurements recorded yet.");
            return;
        }
        
        println!("Total Measurements: {}", self.snapshots.len());
        println!("Window Size: {}\n", self.window_size);
        
        // Current accuracy by type
        println!("Current Accuracy (Recent {}):", self.window_size);
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(acc) = self.current_accuracy(*loop_type) {
                let status = if acc > 0.8 { "✅" } else if acc > 0.6 { "⚠️" } else { "❌" };
                println!("  {} {}: {:.1}%", status, loop_type, acc * 100.0);
                
                if self.accuracy_alert(*loop_type) {
                    println!("    🚨 ACCURACY ALERT: Below {:.0}%", self.accuracy_alert_threshold * 100.0);
                }
            }
        }
        println!();
        
        // Recent speedups
        println!("Recent Speedups (Last {}):", self.window_size);
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(avg) = self.avg_speedup_recent(*loop_type) {
                println!("  {}: {:.2}x", loop_type, avg);
            }
        }
        println!();
        
        // Prediction error stats
        let (min_err, avg_err, max_err) = self.prediction_error_stats();
        println!("Prediction Error Statistics:");
        println!("  Min: {:.1}%", min_err * 100.0);
        println!("  Avg: {:.1}%", avg_err * 100.0);
        println!("  Max: {:.1}%", max_err * 100.0);
        
        if self.speedup_alert() {
            println!("\n🚨 SPEEDUP ALERT: Recent speedup below {:.1}x", self.speedup_min_threshold);
        }
    }
    
    /// Get all snapshots
    pub fn snapshots(&self) -> &[PerformanceSnapshot] {
        &self.snapshots
    }
    
    /// Set accuracy alert threshold
    pub fn set_accuracy_threshold(&mut self, threshold: f64) {
        self.accuracy_alert_threshold = threshold.max(0.0).min(1.0);
    }
    
    /// Set speedup alert threshold
    pub fn set_speedup_threshold(&mut self, threshold: f64) {
        self.speedup_min_threshold = threshold.max(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_monitor_creation() {
        let monitor = PerformanceMonitor::new(10);
        assert_eq!(monitor.snapshots().len(), 0);
    }
    
    #[test]
    fn test_record_snapshot() {
        let mut monitor = PerformanceMonitor::new(10);
        
        monitor.record(
            1000,
            "test_app".to_string(),
            "loop_1".to_string(),
            LoopType::CpuBound,
            10.0,
            9.5,
            10.0,
            1.05,
        );
        
        assert_eq!(monitor.snapshots().len(), 1);
        assert!(monitor.current_accuracy(LoopType::CpuBound).is_some());
    }
    
    #[test]
    fn test_accuracy_calculation() {
        let mut monitor = PerformanceMonitor::new(10);
        
        // Perfect prediction
        monitor.record(1000, "app".to_string(), "loop1".to_string(), LoopType::CpuBound, 10.0, 10.0, 10.0, 1.0);
        
        if let Some(acc) = monitor.current_accuracy(LoopType::CpuBound) {
            assert!(acc > 0.9, "Perfect prediction should have >90% accuracy");
        }
    }
    
    #[test]
    fn test_rolling_window() {
        let mut monitor = PerformanceMonitor::new(3);  // Small window for testing
        
        // Add 5 measurements
        for i in 0..5 {
            monitor.record(
                1000u64 + i,
                "app".to_string(),
                format!("loop_{}", i),
                LoopType::MemoryBound,
                3.0 + i as f64,
                3.0 + i as f64,
                20.0,
                20.0 / (3.0 + i as f64),
            );
        }
        
        // Should only have 5 snapshots total
        assert_eq!(monitor.snapshots().len(), 5);
        
        // But accuracy window should be limited to 3
        if let Some(window) = monitor.accuracy_windows.get(&LoopType::MemoryBound) {
            assert!(window.len() <= 3);
        }
    }
}
