#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 6 Phase 4: Loop Classification and Parameter Discovery Tests
/// 
/// Integration tests for discovering optimal parameters per loop type

#[cfg(test)]
mod tests {
    use killer_rcore::optimization::{
        LoopType, LoopFeatures, ParameterRecommender,
    };

    #[test]
    fn test_loop_type_detection() {
        // CPU-bound pattern: high arithmetic, low memory irregularity
        let cpu_loop = LoopFeatures {
            memory_irregularity: 0.15,
            arithmetic_intensity: 2.5,
            branch_density: 0.02,
            trip_count: 10000,
            vectorizable: true,
        };
        assert_eq!(cpu_loop.classify(), LoopType::CpuBound);

        // Memory-bound pattern: low arithmetic, high memory irregularity
        let mem_loop = LoopFeatures {
            memory_irregularity: 0.85,
            arithmetic_intensity: 0.3,
            branch_density: 0.15,
            trip_count: 50000,
            vectorizable: false,
        };
        assert_eq!(mem_loop.classify(), LoopType::MemoryBound);

        // Mixed pattern: moderate values
        let mixed_loop = LoopFeatures {
            memory_irregularity: 0.5,
            arithmetic_intensity: 1.2,
            branch_density: 0.1,
            trip_count: 20000,
            vectorizable: true,
        };
        assert_eq!(mixed_loop.classify(), LoopType::Mixed);

        println!("✅ Loop type detection working");
    }

    #[test]
    fn test_cpu_bound_parameter_discovery() {
        let mut recommender = ParameterRecommender::new(20, 10);

        // Run discovery
        recommender.discover_all();

        // Get CPU-bound parameters
        let params = recommender.get_parameters(LoopType::CpuBound);
        assert!(params.is_some());

        let cpu_params = params.unwrap();
        println!("CPU-Bound Parameters: {}", cpu_params);

        // Should favor certain characteristics
        assert!(
            cpu_params.gene.vectorization,
            "CPU-bound should prefer vectorization"
        );
        assert!(cpu_params.fitness > 50.0, "CPU-bound should have high fitness");

        println!("✅ CPU-bound discovery: fitness={:.2}", cpu_params.fitness);
    }

    #[test]
    fn test_memory_bound_parameter_discovery() {
        let mut recommender = ParameterRecommender::new(20, 10);

        // Run discovery
        recommender.discover_all();

        // Get memory-bound parameters
        let params = recommender.get_parameters(LoopType::MemoryBound);
        assert!(params.is_some());

        let mem_params = params.unwrap();
        println!("Memory-Bound Parameters: {}", mem_params);

        // Should favor prefetch for memory-bound
        assert!(
            mem_params.gene.prefetch,
            "Memory-bound should prefer prefetch"
        );
        
        // Should avoid aggressive unrolling
        assert!(
            mem_params.gene.unroll_factor <= 8,
            "Memory-bound should prefer moderate unroll"
        );

        println!("✅ Memory-bound discovery: fitness={:.2}", mem_params.fitness);
    }

    #[test]
    fn test_mixed_parameter_discovery() {
        let mut recommender = ParameterRecommender::new(20, 10);

        // Run discovery
        recommender.discover_all();

        // Get mixed parameters
        let params = recommender.get_parameters(LoopType::Mixed);
        assert!(params.is_some());

        let mixed_params = params.unwrap();
        println!("Mixed Parameters: {}", mixed_params);

        assert!(mixed_params.fitness > 30.0, "Mixed should have reasonable fitness");
        println!("✅ Mixed discovery: fitness={:.2}", mixed_params.fitness);
    }

    #[test]
    fn test_parameter_recommendation() {
        let mut recommender = ParameterRecommender::new(20, 10);
        recommender.discover_all();

        // Test recommendation for CPU-bound loop
        let cpu_features = LoopFeatures {
            memory_irregularity: 0.2,
            arithmetic_intensity: 3.0,
            branch_density: 0.02,
            trip_count: 10000,
            vectorizable: true,
        };

        let recommendation = recommender.recommend(&cpu_features);
        assert!(recommendation.is_some());

        let params = recommendation.unwrap();
        println!("✅ CPU-bound recommendation: {}", params);

        // Test recommendation for memory-bound loop
        let mem_features = LoopFeatures {
            memory_irregularity: 0.8,
            arithmetic_intensity: 0.4,
            branch_density: 0.12,
            trip_count: 100000,
            vectorizable: false,
        };

        let mem_rec = recommender.recommend(&mem_features);
        assert!(mem_rec.is_some());
        println!("✅ Memory-bound recommendation: {}", mem_rec.unwrap());
    }

    #[test]
    fn test_parameter_discovery_convergence() {
        let mut recommender = ParameterRecommender::new(30, 15);
        recommender.discover_all();

        // All loop types should be discovered
        assert!(recommender.get_parameters(LoopType::CpuBound).is_some());
        assert!(recommender.get_parameters(LoopType::MemoryBound).is_some());
        assert!(recommender.get_parameters(LoopType::Mixed).is_some());

        println!("✅ Parameter discovery converged for all loop types");
    }

    #[test]
    fn test_full_optimization_pipeline() {
        // Full pipeline: classify loop -> discover parameters -> compile with params
        println!("\n=== Full Optimization Pipeline Test ===\n");

        // Step 1: Create loop classifier and discover parameters
        let mut recommender = ParameterRecommender::new(20, 10);
        recommender.discover_all();

        // Step 2: Analyze loop features
        let loop_features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 2.5,
            branch_density: 0.05,
            trip_count: 10000,
            vectorizable: true,
        };

        // Step 3: Classify loop
        let loop_type = loop_features.classify();
        println!("Loop classified as: {}", loop_type);

        // Step 4: Get recommended parameters
        let params = recommender.recommend(&loop_features);
        assert!(params.is_some());
        
        let recommended = params.unwrap();
        println!("Recommended parameters: {}", recommended);

        // Step 5: Would compile with these parameters
        println!("Unroll factor: {}", recommended.unroll_factor);
        println!("Vectorization: {}", recommended.vectorization);
        println!("Inline hints: {}", recommended.inline_hints);
        println!("Prefetch: {}", recommended.prefetch);
        println!("Opt level: {}", recommended.opt_level);

        println!("\n✅ Full pipeline test passed");
    }

    #[test]
    fn test_parameter_stability() {
        // Run discovery multiple times to check stability
        let mut stability_count = 0;

        for run in 1..=3 {
            let mut recommender = ParameterRecommender::new(15, 8);
            recommender.discover_all();

            if let Some(cpu_params) = recommender.get_parameters(LoopType::CpuBound) {
                println!(
                    "Run {}: CPU-bound fitness={:.2}, opt_level={}",
                    run, cpu_params.fitness, cpu_params.gene.opt_level
                );
                
                // All runs should discover opt_level >= 2 for CPU-bound
                if cpu_params.gene.opt_level >= 2 {
                    stability_count += 1;
                }
            }
        }

        assert!(
            stability_count >= 2,
            "Parameter discovery should be fairly stable"
        );
        println!("✅ Parameter stability: {}/3 runs consistent", stability_count);
    }

    #[test]
    fn test_loop_type_coverage() {
        // Test identifying all three loop types from diverse features
        
        let test_cases = vec![
            (
                LoopType::CpuBound,
                LoopFeatures {
                    memory_irregularity: 0.1,
                    arithmetic_intensity: 4.0,
                    branch_density: 0.01,
                    trip_count: 5000,
                    vectorizable: true,
                },
            ),
            (
                LoopType::MemoryBound,
                LoopFeatures {
                    memory_irregularity: 0.9,
                    arithmetic_intensity: 0.2,
                    branch_density: 0.2,
                    trip_count: 100000,
                    vectorizable: false,
                },
            ),
            (
                LoopType::Mixed,
                LoopFeatures {
                    memory_irregularity: 0.45,
                    arithmetic_intensity: 1.5,
                    branch_density: 0.08,
                    trip_count: 20000,
                    vectorizable: true,
                },
            ),
        ];

        for (expected_type, features) in test_cases {
            let detected = features.classify();
            assert_eq!(
                detected, expected_type,
                "Failed to classify {:?}",
                expected_type
            );
        }

        println!("✅ Loop type coverage: all types detected correctly");
    }

    #[test]
    fn test_confidence_scores() {
        let mut recommender = ParameterRecommender::new(20, 10);
        recommender.discover_all();

        // All discovered parameters should have confidence > 0
        for loop_type in &[LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed] {
            if let Some(params) = recommender.get_parameters(*loop_type) {
                assert!(params.confidence > 0.0, "{} should have > 0 confidence", loop_type);
                assert!(params.confidence <= 1.0, "{} should have <= 1.0 confidence", loop_type);
                println!(
                    "{}: confidence={:.1}%",
                    loop_type,
                    params.confidence * 100.0
                );
            }
        }

        println!("✅ All confidence scores valid");
    }

    #[test]
    fn test_parameter_impact() {
        // Demonstrate parameter impact on speedup
        let mut recommender = ParameterRecommender::new(20, 10);
        recommender.discover_all();

        println!("\n=== Parameter Impact on Speedup ===\n");

        for loop_type in &[LoopType::CpuBound, LoopType::MemoryBound, LoopType::Mixed] {
            if let Some(params) = recommender.get_parameters(*loop_type) {
                // Estimate impact factors
                let unroll_impact = params.gene.unroll_factor as f64 / 2.0;
                let vec_impact = if params.gene.vectorization { 1.3 } else { 1.0 };
                let opt_impact = (params.gene.opt_level as f64 + 1.0) / 2.0;

                let estimated_speedup = unroll_impact * vec_impact * opt_impact;

                println!(
                    "{}: estimated speedup = {:.2}x (unroll={}, vec={}, opt={})",
                    loop_type,
                    estimated_speedup,
                    params.gene.unroll_factor,
                    params.gene.vectorization,
                    params.gene.opt_level
                );
            }
        }

        println!("\n✅ Parameter impact analysis complete");
    }
}
