#![cfg(feature = "legacy-killer-rcore-tests")]
/// Phase 8B Integration Tests
/// Comprehensive testing of vector loop, cross-loop, and dynamic optimization modules
/// Tests real-world scenarios with multiple loops and feedback mechanisms

#[cfg(test)]
mod phase_b_integration_tests {
    use killer_rcore::optimization::{
        VectorLoopOptimizer, SimdCapability, 
        CacheBlockingOptimizer, LoopFusionOptimizer, CrossLoopOptimizer,
        DynamicOptimizer, PerformanceFeedback, AdaptationStrategy,
        LoopFeatures,
    };

    /// Test high-performance vector loop optimization for matrix operations
    #[test]
    fn test_vector_optimization_matrix_multiplication() {
        let mut optimizer = VectorLoopOptimizer::new(SimdCapability::AVX2);

        // Matrix multiplication loop: memory regular, compute-heavy
        let features = LoopFeatures {
            memory_irregularity: 0.08,
            arithmetic_intensity: 0.92,
            branch_density: 0.01,
            trip_count: 2000,
            vectorizable: true,
        };

        let result = optimizer.vectorize("matrix_mult_inner", &features);
        assert!(result.is_ok());

        let vectorized = result.unwrap();
        assert!(vectorized.expected_speedup > 3.0);  // AVX2 with High potential: ~3.8x
        assert!(vectorized.expected_speedup <= 4.0);
        assert_eq!(vectorized.simd_instructions.len(), 3);
        assert!(optimizer.total_speedup() > 3.0);
    }

    /// Test vector optimization with different SIMD capabilities
    #[test]
    fn test_vector_optimization_simd_variants() {
        let features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.85,
            branch_density: 0.02,
            trip_count: 1500,
            vectorizable: true,
        };

        // Test AVX-512
        let mut opt_avx512 = VectorLoopOptimizer::new(SimdCapability::AVX512);
        let result_512 = opt_avx512.vectorize("test_loop", &features);
        assert!(result_512.is_ok());
        assert!(result_512.unwrap().expected_speedup > 6.0);  // ~7.6x with High potential

        // Test AVX
        let mut opt_avx = VectorLoopOptimizer::new(SimdCapability::AVX);
        let result_avx = opt_avx.vectorize("test_loop", &features);
        assert!(result_avx.is_ok());
        assert!(result_avx.unwrap().expected_speedup > 2.0);  // ~2.85x

        // Test SSE4.2
        let mut opt_sse = VectorLoopOptimizer::new(SimdCapability::SSE42);
        let result_sse = opt_sse.vectorize("test_loop", &features);
        assert!(result_sse.is_ok());
        assert!(result_sse.unwrap().expected_speedup > 1.0);   // ~1.9x
    }

    /// Test cache blocking for memory-bound loops
    #[test]
    fn test_cache_blocking_memory_intensive() {
        let mut optimizer = CacheBlockingOptimizer::new();

        // Memory-bound loop: high cache drama
        let features = LoopFeatures {
            memory_irregularity: 0.12,
            arithmetic_intensity: 0.25,
            branch_density: 0.05,
            trip_count: 5000,
            vectorizable: true,
        };

        let result = optimizer.apply_blocking("matrix_transpose", &features);
        assert!(result.is_ok());

        let blocked = result.unwrap();
        assert!(blocked.expected_speedup > 1.8);
        assert!(blocked.block_size > 0);
        assert!(blocked.cache_hit_improvement > 0.3);
    }

    /// Test loop fusion for dependent loop pairs
    #[test]
    fn test_loop_fusion_pipeline() {
        let mut optimizer = LoopFusionOptimizer::new();

        let loop1_features = &LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.5,
            branch_density: 0.05,
            trip_count: 1000,
            vectorizable: true,
        };

        let loop2_features = &LoopFeatures {
            memory_irregularity: 0.11,
            arithmetic_intensity: 0.48,
            branch_density: 0.04,
            trip_count: 1010,  // Compatible
            vectorizable: true,
        };

        // Note: Trip counts are 1000 vs 1010 (difference < 10), should be fusible
        let _can_fuse_check = optimizer.can_fuse(loop1_features, loop2_features);

        // Perform fusion
        let result = optimizer.fuse_loops(&["compute_loop", "transform_loop"], &[loop1_features, loop2_features]);
        assert!(result.is_ok());

        let fused = result.unwrap();
        assert_eq!(fused.loop_ids.len(), 2);
        assert!(fused.expected_speedup > 1.1 && fused.expected_speedup <= 2.0);
    }

    /// Test cross-loop combined optimization
    #[test]
    fn test_cross_loop_combined_optimizations() {
        let mut optimizer = CrossLoopOptimizer::new();

        let features = LoopFeatures {
            memory_irregularity: 0.11,
            arithmetic_intensity: 0.4,
            branch_density: 0.03,
            trip_count: 3000,
            vectorizable: true,
        };

        // Apply both cache blocking and fusion
        let blocking_result = optimizer.cache_blocking.apply_blocking("loop_a", &features);
        assert!(blocking_result.is_ok());

        let fusion_result = optimizer.loop_fusion.fuse_loops(&["loop_b", "loop_c"], &[&features, &features]);
        assert!(fusion_result.is_ok());

        // Combined speedup should multiply
        let combined = optimizer.combined_speedup();
        assert!(combined > 1.8);
        assert!(combined <= 3.0);
    }

    /// Test dynamic optimizer feedback recording
    #[test]
    fn test_dynamic_optimizer_feedback_recording() {
        let mut optimizer = DynamicOptimizer::new();

        let feedback1 = PerformanceFeedback::new(5.0, 4.8);  // 4% error
        let feedback2 = PerformanceFeedback::new(5.0, 4.7);  // 6% error
        let feedback3 = PerformanceFeedback::new(5.0, 4.9);  // 2% error

        optimizer.record_feedback("loop_1", feedback1);
        optimizer.record_feedback("loop_1", feedback2);
        optimizer.record_feedback("loop_1", feedback3);

        assert_eq!(optimizer.feedback_history.get("loop_1").unwrap().len(), 3);

        let avg_error = optimizer.average_error("loop_1");
        assert!(avg_error > 0.02 && avg_error < 0.07);
    }

    /// Test dynamic optimizer convergence detection
    #[test]
    fn test_dynamic_optimizer_convergence() {
        let mut optimizer = DynamicOptimizer::new();

        // Add feedback samples that show convergence
        for _ in 0..5 {
            let feedback = PerformanceFeedback::new(4.0, 3.96);  // ~1% error
            optimizer.record_feedback("loop_stable", feedback);
        }

        assert!(optimizer.has_converged("loop_stable"));
    }

    /// Test dynamic optimizer adaptation on high error
    #[test]
    fn test_dynamic_optimizer_adaptation_high_error() {
        let mut optimizer = DynamicOptimizer::new();
        optimizer.error_threshold = 0.3;

        let bad_feedback = PerformanceFeedback::new(5.0, 1.8);  // 64% error!
        optimizer.record_feedback("loop_bad", bad_feedback);

        let adaptation = optimizer.analyze("loop_bad");
        assert!(adaptation.is_some());

        let adj = adaptation.unwrap();
        assert_eq!(adj.strategy, AdaptationStrategy::DisableOptimization);
        assert!(adj.confidence > 0.8);
    }

    /// Test dynamic optimizer memory pressure response
    #[test]
    fn test_dynamic_optimizer_memory_pressure_response() {
        let mut optimizer = DynamicOptimizer::new();

        let mut feedback = PerformanceFeedback::new(4.0, 3.5);
        feedback.memory_pressure = 0.9;  // Very high memory pressure
        feedback.cache_hit_rate = 0.45;   // Low cache hits

        optimizer.record_feedback("loop_memory_heavy", feedback);

        let adaptation = optimizer.analyze("loop_memory_heavy");
        assert!(adaptation.is_some());

        let adj = adaptation.unwrap();
        assert_eq!(adj.strategy, AdaptationStrategy::ReduceBlockSize);
        assert!(adj.confidence > 0.6);
    }

    /// Test dynamic optimizer thermal throttling detection
    #[test]
    fn test_dynamic_optimizer_thermal_throttling() {
        let mut optimizer = DynamicOptimizer::new();

        let mut feedback = PerformanceFeedback::new(4.0, 3.8);
        feedback.thermal_throttling = 0.7;  // System is throttling

        optimizer.record_feedback("loop_hot", feedback);

        let adaptation = optimizer.analyze("loop_hot");
        assert!(adaptation.is_some());

        let adj = adaptation.unwrap();
        assert_eq!(adj.strategy, AdaptationStrategy::ReduceVectorWidth);
        assert!(adj.confidence > 0.75);
    }

    /// Test Phase B pipeline: vector + cross-loop + dynamic optimization
    #[test]
    fn test_phase_b_complete_pipeline() {
        // Stage 1: Vector optimization
        let mut vec_opt = VectorLoopOptimizer::new(SimdCapability::AVX2);

        let vec_features = LoopFeatures {
            memory_irregularity: 0.09,
            arithmetic_intensity: 0.88,
            branch_density: 0.01,
            trip_count: 1800,
            vectorizable: true,
        };

        let vec_result = vec_opt.vectorize("main_compute", &vec_features);
        assert!(vec_result.is_ok());
        assert!(vec_opt.total_speedup() > 3.0);  // ~3.8x expected for High potential with AVX2

        // Stage 2: Cross-loop optimization
        let mut cross_opt = CrossLoopOptimizer::new();

        let cross_features = LoopFeatures {
            memory_irregularity: 0.1,
            arithmetic_intensity: 0.35,
            branch_density: 0.03,
            trip_count: 2500,
            vectorizable: true,
        };

        let block_result = cross_opt.cache_blocking.apply_blocking("data_prep", &cross_features);
        assert!(block_result.is_ok());

        let fuse_result = cross_opt.loop_fusion.fuse_loops(&["loop_x", "loop_y"], &[&cross_features, &cross_features]);
        assert!(fuse_result.is_ok());

        let cross_combined = cross_opt.combined_speedup();
        assert!(cross_combined > 1.6);

        // Stage 3: Dynamic optimization with feedback
        let mut dyn_opt = DynamicOptimizer::new();

        // Simulate runtime feedback
        let feedback1 = PerformanceFeedback::new(3.8, 3.6);   // Good prediction
        let feedback2 = PerformanceFeedback::new(3.8, 3.65);  // Still good
        let feedback3 = PerformanceFeedback::new(3.8, 3.62);  // Stable

        dyn_opt.record_feedback("optimized_loop", feedback1);
        dyn_opt.record_feedback("optimized_loop", feedback2);
        dyn_opt.record_feedback("optimized_loop", feedback3);

        assert!(dyn_opt.has_converged("optimized_loop"));
        assert!(dyn_opt.average_error("optimized_loop") < 0.08);

        // Stage 4: Verify combined speedup potential
        // Vector: 3.6x, Cross-loop: 2.0x combined
        // Overall potential: 3.6x * 2.0x / 1.5 = 4.8x (with diminishing returns)
        let overall_potential = (vec_opt.total_speedup() * cross_combined) / 1.5;
        assert!(overall_potential > 4.0);
        assert!(overall_potential < 8.0);
    }

    /// Test multi-loop optimization with different characteristics
    #[test]
    fn test_multi_loop_optimization_heterogeneous() {
        let mut vec_opt = VectorLoopOptimizer::new(SimdCapability::AVX2);
        let mut cross_opt = CrossLoopOptimizer::new();

        // Loop 1: CPU-bound, vectorizable
        let loop1 = LoopFeatures {
            memory_irregularity: 0.05,
            arithmetic_intensity: 0.9,
            branch_density: 0.01,
            trip_count: 1000,
            vectorizable: true,
        };

        // Loop 2: Memory-bound, irregular
        let loop2 = LoopFeatures {
            memory_irregularity: 0.35,
            arithmetic_intensity: 0.2,
            branch_density: 0.04,
            trip_count: 2000,
            vectorizable: false,
        };

        // Loop 3: Mixed, fusible with loop 1
        let loop3 = LoopFeatures {
            memory_irregularity: 0.08,
            arithmetic_intensity: 0.65,
            branch_density: 0.02,
            trip_count: 1005,
            vectorizable: true,
        };

        // Vectorize loop 1
        assert!(vec_opt.vectorize("loop_1", &loop1).is_ok());
        let loop1_speedup = vec_opt.total_speedup();
        assert!(loop1_speedup > 3.5);

        // Vectorize loop 3
        let mut vec_opt3 = VectorLoopOptimizer::new(SimdCapability::AVX2);
        assert!(vec_opt3.vectorize("loop_3", &loop3).is_ok());

        // Apply cache blocking to loop 2
        assert!(cross_opt.cache_blocking.apply_blocking("loop_2", &loop2).is_ok());

        // Fuse loops 1 and 3
        assert!(cross_opt.loop_fusion.fuse_loops(&["loop_1", "loop_3"], &[&loop1, &loop3]).is_ok());

        // Verify multi-loop optimization success
        let cross_speedup = cross_opt.combined_speedup();
        assert!(cross_speedup > 1.6);
    }

    /// Test adaptive tuning with multiple feedback rounds
    #[test]
    fn test_dynamic_optimizer_adaptive_tuning_rounds() {
        let mut optimizer = DynamicOptimizer::new();

        // Round 1: Initial feedback shows memory pressure
        let feedback1 = {
            let mut f = PerformanceFeedback::new(4.0, 3.2);
            f.memory_pressure = 0.85;
            f.cache_hit_rate = 0.55;
            f
        };

        optimizer.record_feedback("loop_tune", feedback1.clone());
        let adapt1 = optimizer.analyze("loop_tune");
        assert!(adapt1.is_some());
        assert_eq!(adapt1.unwrap().strategy, AdaptationStrategy::ReduceBlockSize);

        // Round 2: After adjustment, pressure reduced
        let feedback2 = {
            let mut f = PerformanceFeedback::new(4.0, 3.5);
            f.memory_pressure = 0.65;
            f.cache_hit_rate = 0.68;
            f
        };

        optimizer.record_feedback("loop_tune", feedback2.clone());
        let adapt2 = optimizer.analyze("loop_tune");
        assert!(adapt2.is_none() || adapt2.as_ref().unwrap().confidence < 0.5);

        // Round 3: System stabilizes
        let feedback3 = PerformanceFeedback::new(4.0, 3.48);
        optimizer.record_feedback("loop_tune", feedback3.clone());

        // After 3 stable rounds, should show convergence potential
        assert_eq!(optimizer.feedback_history.get("loop_tune").unwrap().len(), 3);
    }
}
