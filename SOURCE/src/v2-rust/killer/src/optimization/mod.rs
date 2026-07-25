// killer_rcore/src/optimization/mod.rs
// Advanced loop optimization transformations
// Week 5: Loop unrolling, vectorization, and specialized code generation

pub mod analyzer;
pub mod unroller;
pub mod codegen;
pub mod rust_compiler;
pub mod genetic_algorithm;
pub mod loop_classifier;
pub mod integrated_optimizer;
pub mod real_integration;
pub mod production;
pub mod monitoring;
pub mod calibration;
pub mod production_integration;
pub mod deployment;
pub mod vector_loop;
pub mod cross_loop;
pub mod dynamic;
pub mod scaling;
pub mod batch;
pub mod incremental;

pub use analyzer::{LoopAnalysis, LoopPattern, LoopAnalyzer};
pub use unroller::{LoopUnroller, UnrollConfiguration};
pub use codegen::{OptimizedCodeGenerator, GenerationStrategy};
pub use rust_compiler::{RustCompiler, OptLevel, GeneratedLoop, CompiledBinary, CompilationResult};
pub use genetic_algorithm::{OptimizationGene, Individual, GeneticOptimizer, PerformanceMetrics, GenerationStats};
pub use loop_classifier::{LoopType, LoopFeatures, OptimalParameters, ParameterDiscovery, ParameterRecommender};
pub use integrated_optimizer::{IntegratedOptimizer, OptimizationResult};
pub use real_integration::{RealCompiler, RealCompilationResult};
pub use production::{ProductionOptimizer, RealWorldProfile};
pub use monitoring::{PerformanceMonitor, PerformanceSnapshot};
pub use calibration::{AccuracyCalibrator, ConfidenceFactor};
pub use production_integration::{ProductionIntegration, DetectedLoop, InjectedOptimization, OptimizationParams};
pub use deployment::{PilotDeployment, DeploymentTarget, DeploymentStage, DeploymentEvent, DeploymentEventType};
pub use vector_loop::{VectorLoopOptimizer, VectorizationPotential, SimdCapability, VectorizedCode};
pub use cross_loop::{CacheBlockingOptimizer, LoopFusionOptimizer, CrossLoopOptimizer, BlockedLoop, FusedLoops};
pub use dynamic::{DynamicOptimizer, PerformanceFeedback, AdaptationStrategy, ParameterAdjustment};
pub use scaling::{ScalingStudyOrchestrator, ScalingStudyResult, BinarySize, LoopCategory};
pub use batch::{BatchLoopOptimizer, BatchLoopResult, OptimizationStatus};
pub use incremental::{IncrementalInjectionFramework, InjectionEvent, InjectionSchedule, InjectionState};

/// Configuration for optimization pipeline
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Enable loop unrolling
    pub enable_unrolling: bool,
    
    /// Unroll factor (2, 4, 8, 16)
    pub unroll_factor: u32,
    
    /// Enable vectorization hints for LLVM
    pub enable_vectorization: bool,
    
    /// Maximum code size increase (%)
    pub max_code_growth: f64,
    
    /// Aggressive optimizations (may increase compilation time)
    pub aggressive: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        OptimizationConfig {
            enable_unrolling: true,
            unroll_factor: 4,
            enable_vectorization: true,
            max_code_growth: 150.0,  // Allow up to 50% size increase
            aggressive: false,
        }
    }
}

impl OptimizationConfig {
    /// Conservative settings for fast compilation
    pub fn conservative() -> Self {
        OptimizationConfig {
            enable_unrolling: true,
            unroll_factor: 2,
            enable_vectorization: false,
            max_code_growth: 125.0,
            aggressive: false,
        }
    }
    
    /// Aggressive settings for maximum performance
    pub fn aggressive() -> Self {
        OptimizationConfig {
            enable_unrolling: true,
            unroll_factor: 8,
            enable_vectorization: true,
            max_code_growth: 200.0,
            aggressive: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = OptimizationConfig::default();
        assert!(config.enable_unrolling);
        assert_eq!(config.unroll_factor, 4);
    }
    
    #[test]
    fn test_conservative_config() {
        let config = OptimizationConfig::conservative();
        assert_eq!(config.unroll_factor, 2);
        assert!(!config.enable_vectorization);
    }
    
    #[test]
    fn test_aggressive_config() {
        let config = OptimizationConfig::aggressive();
        assert_eq!(config.unroll_factor, 8);
        assert!(config.enable_vectorization);
        assert!(config.aggressive);
    }
}
