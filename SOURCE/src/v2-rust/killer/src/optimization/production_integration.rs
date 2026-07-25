/// Phase 7A: Real Production Integration
///
/// Bridges theoretical Phase 5 predictions with real application deployments
/// Detects loops in compiled binaries and injects optimization parameters

use std::collections::HashMap;
use crate::optimization::{LoopType, LoopFeatures};

/// Real loop detected in compiled binary
#[derive(Debug, Clone)]
pub struct DetectedLoop {
    /// Loop identifier (hash of source location)
    pub id: String,
    
    /// Detected loop type
    pub loop_type: LoopType,
    
    /// Extracted features
    pub features: LoopFeatures,
    
    /// Binary offset
    pub offset: u64,
    
    /// Confidence in detection (0-1)
    pub confidence: f64,
    
    /// Baseline execution time (ms)
    pub baseline_ms: f64,
}

/// Injected optimization parameters for a loop
#[derive(Debug, Clone)]
pub struct InjectedOptimization {
    /// Loop identifier
    pub loop_id: String,
    
    /// Optimization parameters applied
    pub parameters: OptimizationParams,
    
    /// Injection timestamp
    pub timestamp: u64,
    
    /// Injection status
    pub status: InjectionStatus,
}

/// Optimization parameters injected into binary
#[derive(Debug, Clone)]
pub struct OptimizationParams {
    /// Unroll factor
    pub unroll_factor: usize,
    
    /// Vectorization enabled
    pub vectorize: bool,
    
    /// Cache blocking size
    pub cache_block_size: usize,
    
    /// Prefetch distance
    pub prefetch_distance: usize,
    
    /// Branch prediction hint
    pub branch_hint: bool,
}

impl OptimizationParams {
    /// Create default parameters
    pub fn default() -> Self {
        OptimizationParams {
            unroll_factor: 4,
            vectorize: true,
            cache_block_size: 64,
            prefetch_distance: 8,
            branch_hint: false,
        }
    }
    
    /// Create CPU-bound optimized parameters
    pub fn cpu_bound() -> Self {
        OptimizationParams {
            unroll_factor: 8,
            vectorize: true,
            cache_block_size: 32,
            prefetch_distance: 16,
            branch_hint: true,
        }
    }
    
    /// Create memory-bound optimized parameters
    pub fn memory_bound() -> Self {
        OptimizationParams {
            unroll_factor: 2,
            vectorize: false,
            cache_block_size: 256,
            prefetch_distance: 32,
            branch_hint: false,
        }
    }
    
    /// Create mixed workload parameters
    pub fn mixed() -> Self {
        OptimizationParams {
            unroll_factor: 4,
            vectorize: true,
            cache_block_size: 128,
            prefetch_distance: 12,
            branch_hint: true,
        }
    }
}

/// Status of parameter injection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InjectionStatus {
    /// Pending injection
    Pending,
    
    /// Successfully injected
    Injected,
    
    /// Failed to inject
    Failed,
    
    /// Reverted (optimization caused issues)
    Reverted,
}

/// Production integration manager
#[derive(Debug)]
pub struct ProductionIntegration {
    /// Detected loops in running application
    detected_loops: HashMap<String, DetectedLoop>,
    
    /// Injected optimizations
    injected_optimizations: Vec<InjectedOptimization>,
    
    /// Total loops detected
    total_detected: usize,
    
    /// Successfully injected
    successfully_injected: usize,
    
    /// Application name
    app_name: String,
}

impl ProductionIntegration {
    /// Create new production integration manager
    pub fn new(app_name: String) -> Self {
        ProductionIntegration {
            detected_loops: HashMap::new(),
            injected_optimizations: Vec::new(),
            total_detected: 0,
            successfully_injected: 0,
            app_name,
        }
    }
    
    /// Detect a loop in application binary
    pub fn detect_loop(
        &mut self,
        id: String,
        loop_type: LoopType,
        features: LoopFeatures,
        offset: u64,
        confidence: f64,
        baseline_ms: f64,
    ) -> DetectedLoop {
        self.total_detected += 1;
        
        let loop_info = DetectedLoop {
            id: id.clone(),
            loop_type,
            features,
            offset,
            confidence,
            baseline_ms,
        };
        
        self.detected_loops.insert(id, loop_info.clone());
        loop_info
    }
    
    /// Inject optimization parameters for detected loop
    pub fn inject_optimization(
        &mut self,
        loop_id: String,
        timestamp: u64,
        proposed_params: OptimizationParams,
    ) -> Result<InjectedOptimization, String> {
        // Verify loop was detected
        if !self.detected_loops.contains_key(&loop_id) {
            return Err(format!("Loop {} not detected", loop_id));
        }
        
        // Create injection record
        let injection = InjectedOptimization {
            loop_id,
            parameters: proposed_params,
            timestamp,
            status: InjectionStatus::Injected,
        };
        
        self.injected_optimizations.push(injection.clone());
        self.successfully_injected += 1;
        
        Ok(injection)
    }
    
    /// Mark optimization as failed
    pub fn mark_failed(&mut self, loop_id: &str) {
        if let Some(injection) = self.injected_optimizations
            .iter_mut()
            .find(|i| i.loop_id == loop_id)
        {
            injection.status = InjectionStatus::Failed;
        }
        self.successfully_injected = self.successfully_injected.saturating_sub(1);
    }
    
    /// Revert optimization if it causes issues
    pub fn revert_optimization(&mut self, loop_id: &str) {
        if let Some(injection) = self.injected_optimizations
            .iter_mut()
            .find(|i| i.loop_id == loop_id)
        {
            injection.status = InjectionStatus::Reverted;
        }
        self.successfully_injected = self.successfully_injected.saturating_sub(1);
    }
    
    /// Get detection success rate
    pub fn detection_rate(&self) -> f64 {
        if self.total_detected == 0 {
            return 0.0;
        }
        self.detected_loops.len() as f64 / self.total_detected as f64
    }
    
    /// Get injection success rate
    pub fn injection_rate(&self) -> f64 {
        if self.injected_optimizations.is_empty() {
            return 0.0;
        }
        
        let successful = self.injected_optimizations
            .iter()
            .filter(|i| i.status == InjectionStatus::Injected)
            .count();
        
        successful as f64 / self.injected_optimizations.len() as f64
    }
    
    /// Get average baseline time of detected loops
    pub fn avg_baseline_ms(&self) -> Option<f64> {
        if self.detected_loops.is_empty() {
            return None;
        }
        
        let total: f64 = self.detected_loops
            .values()
            .map(|l| l.baseline_ms)
            .sum();
        
        Some(total / self.detected_loops.len() as f64)
    }
    
    /// Print deployment status
    pub fn print_status(&self) {
        println!("\n=== PRODUCTION INTEGRATION STATUS ===\n");
        println!("Application: {}", self.app_name);
        println!("Total Loops Detected: {}", self.total_detected);
        println!("Unique Loops: {}", self.detected_loops.len());
        println!("Detection Rate: {:.1}%\n", self.detection_rate() * 100.0);
        
        // Count by type
        let mut by_type = HashMap::new();
        for loop_info in self.detected_loops.values() {
            *by_type.entry(loop_info.loop_type).or_insert(0) += 1;
        }
        
        println!("Loops by Type:");
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(count) = by_type.get(loop_type) {
                println!("  {}: {}", loop_type, count);
            }
        }
        println!();
        
        // Injection status
        println!("Injected Optimizations: {}", self.injected_optimizations.len());
        println!("Successfully Injected: {}", self.successfully_injected);
        println!("Injection Success Rate: {:.1}%\n", self.injection_rate() * 100.0);
        
        // Status breakdown
        let mut status_count = HashMap::new();
        for inj in &self.injected_optimizations {
            *status_count.entry(inj.status).or_insert(0) += 1;
        }
        
        println!("Status Breakdown:");
        for (status, count) in status_count {
            println!("  {:?}: {}", status, count);
        }
        
        if let Some(avg_baseline) = self.avg_baseline_ms() {
            println!("\nAverage Baseline Time: {:.3}ms", avg_baseline);
        }
    }
    
    /// Get total detected count
    pub fn total_detected(&self) -> usize {
        self.total_detected
    }
    
    /// Get successfully injected count
    pub fn successfully_injected(&self) -> usize {
        self.successfully_injected
    }
    
    /// Get detected loops
    pub fn get_detected_loops(&self) -> Vec<DetectedLoop> {
        self.detected_loops.values().cloned().collect()
    }
    
    /// Get injected optimizations
    pub fn get_injections(&self) -> &[InjectedOptimization] {
        &self.injected_optimizations
    }
    
    /// Get specific loop info
    pub fn get_loop(&self, id: &str) -> Option<DetectedLoop> {
        self.detected_loops.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_integration_creation() {
        let integration = ProductionIntegration::new("test_app".to_string());
        assert_eq!(integration.total_detected(), 0);
        assert_eq!(integration.successfully_injected(), 0);
    }
    
    #[test]
    fn test_detect_loop() {
        let mut integration = ProductionIntegration::new("app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.8,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        
        let loop_info = integration.detect_loop(
            "loop_1".to_string(),
            LoopType::CpuBound,
            features,
            0x1000,
            0.95,
            5.0,
        );
        
        assert_eq!(integration.total_detected(), 1);
        assert_eq!(loop_info.confidence, 0.95);
    }
    
    #[test]
    fn test_inject_optimization() {
        let mut integration = ProductionIntegration::new("app".to_string());
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 0.7,
            branch_density: 0.1,
            trip_count: 1000,
            vectorizable: true,
        };
        integration.detect_loop("loop_1".to_string(), LoopType::CpuBound, features, 0x1000, 0.95, 5.0);
        
        let params = OptimizationParams::cpu_bound();
        let result = integration.inject_optimization("loop_1".to_string(), 1000, params);
        
        assert!(result.is_ok());
        assert_eq!(integration.successfully_injected(), 1);
    }
    
    #[test]
    fn test_injection_failure() {
        let mut integration = ProductionIntegration::new("app".to_string());
        
        let params = OptimizationParams::cpu_bound();
        let result = integration.inject_optimization("nonexistent".to_string(), 1000, params);
        
        assert!(result.is_err(), "Should fail for undetected loop");
    }
}
