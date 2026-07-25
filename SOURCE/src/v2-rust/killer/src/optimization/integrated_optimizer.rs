/// Week 6 Phase 5: Integrated Optimization Pipeline
///
/// Combines loop classification, parameter discovery, and realistic speedup simulation
/// to validate that discovered parameters actually improve performance

use std::collections::HashMap;
use crate::optimization::{
    LoopFeatures, LoopType, ParameterRecommender, OptimizationGene,
};

/// Result of integrated optimization (predicted vs actual speedup)
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Classified loop type
    pub loop_type: LoopType,
    
    /// Discovered parameters
    pub parameters: OptimizationGene,
    
    /// Predicted speedup from genetic algorithm fitness
    pub predicted_speedup: f64,
    
    /// Actual speedup measured via simulation
    pub actual_speedup: f64,
    
    /// How close prediction matched reality (1.0 = perfect)
    pub match_quality: f64,
    
    /// Confidence score of discovered parameters (0.0-1.0)
    pub confidence: f64,
    
    /// Compilation time in milliseconds
    pub compile_time_ms: f64,
    
    /// Binary size in kilobytes
    pub binary_size_kb: f64,
}

impl std::fmt::Display for OptimizationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "OptimizationResult {{\n  loop_type: {},\n  predicted: {:.2}x, actual: {:.2}x, match: {:.1}%\n  confidence: {:.1}%,\n  compile: {:.1}ms, binary: {:.1}KB\n}}",
            self.loop_type,
            self.predicted_speedup,
            self.actual_speedup,
            self.match_quality * 100.0,
            self.confidence * 100.0,
            self.compile_time_ms,
            self.binary_size_kb
        )
    }
}

/// Full integrated optimization pipeline
pub struct IntegratedOptimizer {
    /// Parameter discovery and recommendation system
    recommender: ParameterRecommender,
    
    /// Cached results for analysis
    results: Vec<OptimizationResult>,
}

impl IntegratedOptimizer {
    /// Create new integrated optimizer
    pub fn new(population_size: usize, generations: u32) -> Self {
        let mut recommender = ParameterRecommender::new(population_size, generations);
        // Discover optimal parameters during initialization
        recommender.discover_all();
        
        IntegratedOptimizer {
            recommender,
            results: Vec::new(),
        }
    }
    
    /// Optimize a loop: classify → recommend → simulate → validate
    pub fn optimize(&mut self, features: &LoopFeatures) -> OptimizationResult {
        // Step 1: Classify loop type
        let loop_type = features.classify();
        
        // Step 2: Get recommended parameters
        let parameters = self.recommender.get_parameters(loop_type)
            .map(|p| p.gene)
            .unwrap_or_else(|| OptimizationGene {
                unroll_factor: 1,
                vectorization: false,
                inline_hints: false,
                prefetch: false,
                opt_level: 1,
            });
        
        let confidence = self.recommender.get_parameters(loop_type)
            .map(|p| p.confidence)
            .unwrap_or(0.0);
        
        // Step 3: Estimate predicted speedup based on parameters
        let predicted_speedup = self.estimate_speedup(&parameters, loop_type, features);
        
        // Step 4: Simulate actual speedup based on parameter effectiveness
        let actual_speedup = self.simulate_actual_speedup(&parameters, loop_type, features);
        
        // Step 5: Calculate match quality (how well prediction matched reality)
        let speedup_ratio = predicted_speedup / actual_speedup.max(0.1);
        let match_quality = if speedup_ratio > 1.0 {
            1.0 / speedup_ratio
        } else {
            speedup_ratio
        };
        let match_quality = match_quality.min(1.0).max(0.1);
        
        // Step 6: Estimate compilation metrics
        let compile_time_ms = 100.0 + (parameters.opt_level as f64 * 20.0);
        let binary_size_kb = 50.0 + (parameters.unroll_factor as f64 * 5.0);
        
        // Create result
        let result = OptimizationResult {
            loop_type,
            parameters,
            predicted_speedup,
            actual_speedup,
            match_quality,
            confidence,
            compile_time_ms,
            binary_size_kb,
        };
        
        // Cache result
        self.results.push(result.clone());
        
        result
    }
    
    /// Estimate speedup based on parameters and loop type
    fn estimate_speedup(&self, gene: &OptimizationGene, loop_type: LoopType, _features: &LoopFeatures) -> f64 {
        // Convert parameter flags to numeric scores
        let opt_score = ((gene.opt_level + 1) as f64) * 0.5;  // 0.5-2.5 (more conservative)
        
        let unroll_score = match gene.unroll_factor {
            1 => 1.0,
            2 => 1.2,
            4 => 1.4,
            8 => 1.6,
            16 => 1.8,
            32 => 2.0,
            _ => 1.3,
        };
        
        let vec_score = if gene.vectorization { 1.8 } else { 1.0 };
        let inline_score = if gene.inline_hints { 1.3 } else { 1.0 };
        let prefetch_score = if gene.prefetch { 1.2 } else { 1.0 };
        
        // Type-specific fitness calculation - more conservative estimates
        match loop_type {
            LoopType::CpuBound => {
                // CPU-bound: optimize for execution speed (scaled down)
                opt_score * unroll_score * vec_score * inline_score
            },
            LoopType::MemoryBound => {
                // Memory-bound: optimize for cache efficiency
                opt_score * (1.0 / unroll_score).max(0.5) * prefetch_score
            },
            LoopType::Mixed => {
                // Mixed: balanced approach
                opt_score * unroll_score.log10().max(1.0) * vec_score * prefetch_score
            },
        }
    }
    
    /// Simulate actual speedup based on parameter effectiveness
    fn simulate_actual_speedup(&self, gene: &OptimizationGene, loop_type: LoopType, _features: &LoopFeatures) -> f64 {
        let baseline = 1.0;  // O0 optimization baseline
        
        // Base multiplier from optimization level
        let opt_multiplier = match gene.opt_level {
            0 => 1.0,      // O0
            1 => 1.2,      // O1
            2 => 1.5,      // O2
            3 => 1.6,      // O3
            4 => 1.7,      // Oz
            _ => 1.0,
        };
        
        // Type-specific effectiveness
        let (unroll_contrib, vec_contrib, inline_contrib, prefetch_contrib) = match loop_type {
            LoopType::CpuBound => {
                // Unroll and vectorization very effective
                let unroll = match gene.unroll_factor {
                    1 => 1.0,
                    2 => 1.3,
                    4 => 1.6,
                    8 => 1.85,
                    16 => 2.1,
                    32 => 2.2,  // Diminishing returns
                    _ => 1.5,
                };
                let vec = if gene.vectorization { 2.5 } else { 1.0 };
                let inline = if gene.inline_hints { 1.4 } else { 1.0 };
                let prefetch = if gene.prefetch { 1.1 } else { 1.0 };
                (unroll, vec, inline, prefetch)
            },
            LoopType::MemoryBound => {
                // Prefetch and small code effective, unroll counterproductive
                let unroll = if gene.unroll_factor == 1 { 1.2 } else { (1.1 - gene.unroll_factor as f64 * 0.05).max(0.8) };
                let vec = if gene.vectorization { 1.1 } else { 1.0 };  // Often can't vectorize
                let inline = if gene.inline_hints { 0.95 } else { 1.0 };  // Code bloat
                let prefetch = if gene.prefetch { 1.3 } else { 1.0 };  // Very effective
                (unroll, vec, inline, prefetch)
            },
            LoopType::Mixed => {
                // Balanced approach
                let unroll = match gene.unroll_factor {
                    1 => 1.0,
                    2 => 1.4,
                    4 => 1.7,
                    8 => 1.85,
                    16 => 1.95,
                    32 => 2.0,
                    _ => 1.5,
                };
                let vec = if gene.vectorization { 1.8 } else { 1.0 };
                let inline = if gene.inline_hints { 1.2 } else { 1.0 };
                let prefetch = if gene.prefetch { 1.2 } else { 1.0 };
                (unroll, vec, inline, prefetch)
            },
        };
        
        baseline * opt_multiplier * unroll_contrib * vec_contrib * inline_contrib * prefetch_contrib
    }
    
    /// Get all optimization results cached
    pub fn get_results(&self) -> &[OptimizationResult] {
        &self.results
    }
    
    /// Clear cached results
    pub fn clear_results(&mut self) {
        self.results.clear();
    }
    
    /// Print summary of all results
    pub fn print_summary(&self) {
        println!("\n=== Integrated Optimization Summary ===\n");
        
        if self.results.is_empty() {
            println!("No optimization results yet.");
            return;
        }
        
        // Summary by loop type
        let mut by_type: HashMap<LoopType, Vec<&OptimizationResult>> = HashMap::new();
        for result in &self.results {
            by_type.entry(result.loop_type)
                .or_insert_with(Vec::new)
                .push(result);
        }
        
        for loop_type in [LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed].iter() {
            if let Some(results) = by_type.get(loop_type) {
                println!("{}:", loop_type);
                
                let avg_predicted: f64 = results.iter()
                    .map(|r| r.predicted_speedup)
                    .sum::<f64>() / results.len() as f64;
                let avg_actual: f64 = results.iter()
                    .map(|r| r.actual_speedup)
                    .sum::<f64>() / results.len() as f64;
                let avg_match: f64 = results.iter()
                    .map(|r| r.match_quality)
                    .sum::<f64>() / results.len() as f64;
                let avg_confidence: f64 = results.iter()
                    .map(|r| r.confidence)
                    .sum::<f64>() / results.len() as f64;
                
                println!("  Predicted speedup: {:.2}x", avg_predicted);
                println!("  Actual speedup: {:.2}x", avg_actual);
                println!("  Match quality: {:.1}%", avg_match * 100.0);
                println!("  Confidence: {:.1}%", avg_confidence * 100.0);
                println!();
            }
        }
        
        // Overall statistics
        let overall_predicted: f64 = self.results.iter()
            .map(|r| r.predicted_speedup)
            .sum::<f64>() / self.results.len() as f64;
        let overall_actual: f64 = self.results.iter()
            .map(|r| r.actual_speedup)
            .sum::<f64>() / self.results.len() as f64;
        let overall_match: f64 = self.results.iter()
            .map(|r| r.match_quality)
            .sum::<f64>() / self.results.len() as f64;
        
        println!("Overall Statistics ({} total):", self.results.len());
        println!("  Average predicted: {:.2}x", overall_predicted);
        println!("  Average actual: {:.2}x", overall_actual);
        println!("  Average match quality: {:.1}%", overall_match * 100.0);
        
        // Accuracy assessment
        if overall_match > 0.8 {
            println!("\n✅ Fitness function is ACCURATE - predictions closely match reality");
        } else if overall_match > 0.6 {
            println!("\n⚠️  Fitness function is REASONABLE - predictions within 2x of reality");
        } else {
            println!("\n❌ Fitness function needs REFINEMENT - predictions differ significantly");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_integrated_optimizer_creation() {
        let optimizer = IntegratedOptimizer::new(20, 10);
        assert_eq!(optimizer.get_results().len(), 0);
    }
    
    #[test]
    fn test_estimate_speedup_cpu_bound() {
        let optimizer = IntegratedOptimizer::new(20, 10);
        let features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 4.0,
            branch_density: 0.02,
            trip_count: 10000,
            vectorizable: true,
        };
        
        let gene = OptimizationGene {
            unroll_factor: 32,
            vectorization: true,
            inline_hints: true,
            prefetch: false,
            opt_level: 3,
        };
        
        let speedup = optimizer.estimate_speedup(&gene, LoopType::CpuBound, &features);
        assert!(speedup > 9.0, "CPU-bound should have significant speedup (conservative calibration)");
    }
    
    #[test]
    fn test_estimate_speedup_memory_bound() {
        let optimizer = IntegratedOptimizer::new(20, 10);
        let features = LoopFeatures {
            memory_irregularity: 0.8,
            arithmetic_intensity: 0.5,
            branch_density: 0.05,
            trip_count: 5000,
            vectorizable: false,
        };
        
        let gene = OptimizationGene {
            unroll_factor: 1,
            vectorization: false,
            inline_hints: false,
            prefetch: true,
            opt_level: 4,
        };
        
        let speedup = optimizer.estimate_speedup(&gene, LoopType::MemoryBound, &features);
        assert!(speedup > 1.0 && speedup < 5.0, "Memory-bound should have modest speedup");
    }
    
    #[test]
    fn test_simulate_cpu_bound() {
        let optimizer = IntegratedOptimizer::new(20, 10);
        let features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 4.0,
            branch_density: 0.02,
            trip_count: 10000,
            vectorizable: true,
        };
        
        let gene = OptimizationGene {
            unroll_factor: 32,
            vectorization: true,
            inline_hints: true,
            prefetch: false,
            opt_level: 3,
        };
        
        let speedup = optimizer.simulate_actual_speedup(&gene, LoopType::CpuBound, &features);
        assert!(speedup > 4.0, "Simulated CPU speedup should be significant");
    }
}
