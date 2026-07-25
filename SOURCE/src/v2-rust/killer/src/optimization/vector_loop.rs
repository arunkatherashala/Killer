/// Phase 8B: Vector Loop Optimization Module
/// Implements SIMD/vectorization optimizations for data-parallel loops
/// Target speedups: 3-8x (AVX-512) or 2.5-4x (AVX2)

use crate::optimization::loop_classifier::{LoopFeatures, LoopType};
use std::collections::HashMap;

/// Vectorization potential assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VectorizationPotential {
    /// High potential: 3-8x speedup (memory regular, branch-free, compute-heavy)
    High,
    /// Medium potential: 1.5-3x speedup (some beneficial properties)
    Medium,
    /// Low potential: <1.5x speedup (unsuitable for vectorization)
    Low,
    /// Cannot vectorize (branches, irregular memory, data dependencies)
    None,
}

/// SIMD instruction set capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimdCapability {
    /// 128-bit vectors (SSE 4.2): ~2x speedup on float32
    SSE42,
    /// 256-bit vectors (AVX2): ~4x speedup on float32
    AVX2,
    /// 256-bit vectors (AVX): ~2-3x speedup on float32
    AVX,
    /// 512-bit vectors (AVX-512): ~8x speedup on float32
    AVX512,
    /// ARM NEON: ~4x speedup on float32
    NEON,
    /// No SIMD available
    None,
}

impl SimdCapability {
    /// Get expected speedup multiplier for this SIMD capability
    pub fn speedup_multiplier(&self) -> f64 {
        match self {
            SimdCapability::SSE42 => 2.0,
            SimdCapability::AVX => 3.0,
            SimdCapability::AVX2 => 4.0,
            SimdCapability::AVX512 => 8.0,
            SimdCapability::NEON => 4.0,
            SimdCapability::None => 1.0,
        }
    }

    /// Get vector width in bytes
    pub fn vector_width_bytes(&self) -> usize {
        match self {
            SimdCapability::SSE42 => 16,
            SimdCapability::AVX => 32,
            SimdCapability::AVX2 => 32,
            SimdCapability::AVX512 => 64,
            SimdCapability::NEON => 16,
            SimdCapability::None => 0,
        }
    }
}

/// Register allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterAllocation {
    /// Linear register assignment
    Linear,
    /// Greedy register assignment (higher utilization)
    Greedy,
    /// Optimal (exhaustive search, slow)
    Optimal,
}

/// Vectorized code representation
#[derive(Debug, Clone)]
pub struct VectorizedCode {
    /// Original loop ID
    pub loop_id: String,
    /// Vectorization method used
    pub method: String,
    /// Expected speedup multiplier
    pub expected_speedup: f64,
    /// SIMD instructions used
    pub simd_instructions: Vec<String>,
    /// Register usage count
    pub register_count: usize,
    /// Generated code (pseudo-code)
    pub code: String,
}

/// Vector Loop Optimizer
#[derive(Debug, Clone)]
pub struct VectorLoopOptimizer {
    /// Target loop type
    pub loop_type: LoopType,
    /// SIMD capability to target
    pub simd_capability: SimdCapability,
    /// Register allocation strategy
    pub register_strategy: RegisterAllocation,
    /// Successfully vectorized loops
    pub vectorized_loops: HashMap<String, VectorizedCode>,
}

impl VectorLoopOptimizer {
    /// Create a new vector loop optimizer
    pub fn new(simd_capability: SimdCapability) -> Self {
        VectorLoopOptimizer {
            loop_type: LoopType::CpuBound,
            simd_capability,
            register_strategy: RegisterAllocation::Greedy,
            vectorized_loops: HashMap::new(),
        }
    }

    /// Analyze loop for vectorization potential
    pub fn analyze_vectorization(&self, features: &LoopFeatures) -> VectorizationPotential {
        // Disqualify if loop has irregular memory access
        if features.memory_irregularity > 0.3 {
            return VectorizationPotential::None;
        }

        // Disqualify if loop has high branch divergence
        if features.branch_density > 0.2 {
            return VectorizationPotential::None;
        }

        // Check for vectorization-friendly properties
        let stride_regular = features.memory_irregularity < 0.15;
        let branch_free = features.branch_density < 0.05;
        let compute_heavy = features.arithmetic_intensity > 0.7;
        let vectorizable = features.vectorizable;

        if stride_regular && branch_free && (compute_heavy || vectorizable) {
            return VectorizationPotential::High;
        }

        if stride_regular && (compute_heavy || vectorizable) {
            return VectorizationPotential::Medium;
        }

        if compute_heavy || vectorizable {
            return VectorizationPotential::Low;
        }

        VectorizationPotential::None
    }

    /// Generate vectorized code for a loop
    pub fn vectorize(&mut self, loop_id: &str, features: &LoopFeatures) -> Result<VectorizedCode, String> {
        let potential = self.analyze_vectorization(features);

        if potential == VectorizationPotential::None {
            return Err(format!("Loop {} not suitable for vectorization", loop_id));
        }

        let base_speedup = match potential {
            VectorizationPotential::High => self.simd_capability.speedup_multiplier() * 0.95,
            VectorizationPotential::Medium => self.simd_capability.speedup_multiplier() * 0.6,
            VectorizationPotential::Low => self.simd_capability.speedup_multiplier() * 0.3,
            VectorizationPotential::None => 1.0,
        };

        // Account for register pressure (fewer registers = more spills)
        let trip_count = features.trip_count as f64;
        let register_usage = self.estimate_register_usage(features);
        let register_penalty = if register_usage > 12 { 0.85 } else { 1.0 };

        let expected_speedup = (base_speedup * register_penalty).min(self.simd_capability.speedup_multiplier());

        let method = format!("SIMD {:?} + {:?} register allocation", self.simd_capability, self.register_strategy);
        let simd_instructions = self.generate_simd_instructions(features);

        let code = VectorizedCode {
            loop_id: loop_id.to_string(),
            method,
            expected_speedup,
            simd_instructions,
            register_count: register_usage,
            code: format!(
                "// Vectorized loop {} using {:?}\n// Expected speedup: {:.2}x\n// Trip count: {}\n// Register usage: {}",
                loop_id, self.simd_capability, expected_speedup, trip_count as usize, register_usage
            ),
        };

        self.vectorized_loops.insert(loop_id.to_string(), code.clone());
        Ok(code)
    }

    /// Estimate register usage for a loop
    fn estimate_register_usage(&self, features: &LoopFeatures) -> usize {
        // Base registers for loop control
        let mut usage = 3;

        // Add registers for arithmetic intensity
        usage += (features.arithmetic_intensity * 4.0) as usize;

        // Add registers for data reuse
        usage = (usage as f64 * (1.0 + features.trip_count as f64 / 1000.0)) as usize;

        // Cap at realistic maximum (typically 8-16 registers available for SIMD loops)
        usage.min(16).max(1)
    }

    /// Generate SIMD instruction set for a loop
    fn generate_simd_instructions(&self, _features: &LoopFeatures) -> Vec<String> {
        let mut instructions = Vec::new();

        match self.simd_capability {
            SimdCapability::AVX512 => {
                instructions.push("vmulpd (AVX-512 packed double multiply)".to_string());
                instructions.push("vaddpd (AVX-512 packed double add)".to_string());
                instructions.push("vmovupd (AVX-512 unaligned load)".to_string());
            }
            SimdCapability::AVX2 => {
                instructions.push("vmulpd (AVX2 packed double multiply)".to_string());
                instructions.push("vaddpd (AVX2 packed double add)".to_string());
                instructions.push("vmovupd (AVX2 unaligned load)".to_string());
            }
            SimdCapability::AVX => {
                instructions.push("vmulps (AVX packed single multiply)".to_string());
                instructions.push("vaddps (AVX packed single add)".to_string());
                instructions.push("vmovups (AVX unaligned load)".to_string());
            }
            SimdCapability::SSE42 => {
                instructions.push("mulpd (SSE4.2 packed double multiply)".to_string());
                instructions.push("addpd (SSE4.2 packed double add)".to_string());
                instructions.push("movupd (SSE4.2 unaligned load)".to_string());
            }
            SimdCapability::NEON => {
                instructions.push("vmul.f64 (NEON vector multiply)".to_string());
                instructions.push("vadd.f64 (NEON vector add)".to_string());
                instructions.push("vld1.64 (NEON vector load)".to_string());
            }
            SimdCapability::None => {
                instructions.push("(no SIMD available)".to_string());
            }
        }

        instructions
    }

    /// Get total speedup from all vectorized loops
    pub fn total_speedup(&self) -> f64 {
        if self.vectorized_loops.is_empty() {
            return 1.0;
        }

        let sum: f64 = self.vectorized_loops.values().map(|v| v.expected_speedup).sum();
        sum / self.vectorized_loops.len() as f64
    }

    /// Get status report
    pub fn status_report(&self) -> String {
        format!(
            "VectorLoopOptimizer (SIMD: {:?}, Vectorized: {}, Avg Speedup: {:.2}x)",
            self.simd_capability,
            self.vectorized_loops.len(),
            self.total_speedup()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectorization_potential_assessment() {
        let optimizer = VectorLoopOptimizer::new(SimdCapability::AVX2);

        // High potential: regular memory, no branches, compute-heavy
        let high_features = LoopFeatures {
            memory_irregularity: 0.05,
            arithmetic_intensity: 0.85,
            branch_density: 0.02,
            trip_count: 1000,
            vectorizable: true,
        };
        assert_eq!(optimizer.analyze_vectorization(&high_features), VectorizationPotential::High);

        // Low potential: some issues but still viable
        let medium_features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.6,
            branch_density: 0.08,
            trip_count: 500,
            vectorizable: true,
        };
        assert_eq!(optimizer.analyze_vectorization(&medium_features), VectorizationPotential::Medium);

        // None: irregular memory access
        let none_features = LoopFeatures {
            memory_irregularity: 0.5,
            arithmetic_intensity: 0.8,
            branch_density: 0.02,
            trip_count: 1000,
            vectorizable: true,
        };
        assert_eq!(optimizer.analyze_vectorization(&none_features), VectorizationPotential::None);
    }

    #[test]
    fn test_simd_capability_speedups() {
        assert_eq!(SimdCapability::SSE42.speedup_multiplier(), 2.0);
        assert_eq!(SimdCapability::AVX.speedup_multiplier(), 3.0);
        assert_eq!(SimdCapability::AVX2.speedup_multiplier(), 4.0);
        assert_eq!(SimdCapability::AVX512.speedup_multiplier(), 8.0);
    }

    #[test]
    fn test_vectorize_high_potential_loop() {
        let mut optimizer = VectorLoopOptimizer::new(SimdCapability::AVX2);

        let features = LoopFeatures {
            memory_irregularity: 0.08,
            arithmetic_intensity: 0.9,
            branch_density: 0.01,
            trip_count: 2000,
            vectorizable: true,
        };

        let result = optimizer.vectorize("loop_matrix_multiply", &features);
        assert!(result.is_ok());

        let vectorized = result.unwrap();
        assert_eq!(vectorized.loop_id, "loop_matrix_multiply");
        assert!(vectorized.expected_speedup > 3.0);
        assert!(vectorized.expected_speedup <= 4.0);
        assert!(!vectorized.simd_instructions.is_empty());
    }

    #[test]
    fn test_vectorize_unsuitable_loop() {
        let mut optimizer = VectorLoopOptimizer::new(SimdCapability::AVX2);

        let features = LoopFeatures {
            memory_irregularity: 0.6,
            arithmetic_intensity: 0.3,
            branch_density: 0.4,
            trip_count: 100,
            vectorizable: false,
        };

        let result = optimizer.vectorize("loop_irregular", &features);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_usage_estimation() {
        let optimizer = VectorLoopOptimizer::new(SimdCapability::AVX2);

        let light_features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.3,
            branch_density: 0.05,
            trip_count: 100,
            vectorizable: true,
        };

        let heavy_features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.9,
            branch_density: 0.01,
            trip_count: 5000,
            vectorizable: true,
        };

        let light_regs = optimizer.estimate_register_usage(&light_features);
        let heavy_regs = optimizer.estimate_register_usage(&heavy_features);

        // Heavy workload should use more registers
        assert!(heavy_regs >= light_regs);
        assert!(light_regs >= 1);
        assert!(heavy_regs <= 16);
    }
}
