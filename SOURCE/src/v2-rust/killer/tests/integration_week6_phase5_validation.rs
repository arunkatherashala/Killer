#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 6 Phase 5: Integrated Optimization Pipeline Tests
///
/// Validates that discovered parameters improve performance
/// Tests cover: classification → recommendation → optimization → validation

#[cfg(test)]
mod tests {
    use killer_rcore::optimization::{
        IntegratedOptimizer, LoopFeatures, LoopType, ParameterRecommender,
    };

    #[test]
    fn test_integrated_optimizer_creation() {
        println!("\n=== Test: Integrated Optimizer Creation ===");
        
        let optimizer = IntegratedOptimizer::new(20, 10);
        assert_eq!(optimizer.get_results().len(), 0);
        println!("✅ Optimizer created successfully");
    }

    #[test]
    fn test_cpu_bound_loop_optimization() {
        println!("\n=== Test: CPU-Bound Loop Optimization ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Define CPU-bound loop features
        let features = LoopFeatures {
            memory_irregularity: 0.1,      // Low irregularity
            arithmetic_intensity: 4.5,      // High arithmetic
            branch_density: 0.02,
            trip_count: 10000,
            vectorizable: true,
        };
        
        assert_eq!(features.classify(), LoopType::CpuBound);
        println!("  Loop classified as: CpuBound");
        
        let result = optimizer.optimize(&features);
        
        println!("  Loop type: {}", result.loop_type);
        println!("  Parameters: {}", result.parameters);
        println!("  Predicted speedup: {:.2}x", result.predicted_speedup);
        println!("  Actual speedup: {:.2}x", result.actual_speedup);
        println!("  Match quality: {:.1}%", result.match_quality * 100.0);
        println!("  Confidence: {:.1}%", result.confidence * 100.0);
        
        // Validate optimization
        assert_eq!(result.loop_type, LoopType::CpuBound);
        assert!(result.predicted_speedup > 1.0, "Should predict speedup");
        assert!(result.actual_speedup > 0.0, "Should measure speedup");
        assert!(result.match_quality > 0.0, "Should have match quality");
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0, "Confidence must be 0-1");
        
        println!("✅ CPU-bound optimization: {:.2}x speedup", result.actual_speedup);
    }

    #[test]
    fn test_memory_bound_loop_optimization() {
        println!("\n=== Test: Memory-Bound Loop Optimization ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Define memory-bound loop features
        let features = LoopFeatures {
            memory_irregularity: 0.8,       // High irregularity
            arithmetic_intensity: 0.5,      // Low arithmetic
            branch_density: 0.05,
            trip_count: 5000,
            vectorizable: false,
        };
        
        assert_eq!(features.classify(), LoopType::MemoryBound);
        println!("  Loop classified as: MemoryBound");
        
        let result = optimizer.optimize(&features);
        
        println!("  Loop type: {}", result.loop_type);
        println!("  Parameters: {}", result.parameters);
        println!("  Predicted speedup: {:.2}x", result.predicted_speedup);
        println!("  Actual speedup: {:.2}x", result.actual_speedup);
        println!("  Match quality: {:.1}%", result.match_quality * 100.0);
        
        // Validate optimization (memory-bound typically has lower speedup)
        assert_eq!(result.loop_type, LoopType::MemoryBound);
        assert!(result.predicted_speedup > 0.0, "Should predict speedup");
        assert!(result.actual_speedup > 0.0, "Should measure speedup");
        assert!(result.match_quality > 0.0, "Should have match quality");
        
        println!("✅ Memory-bound optimization: {:.2}x speedup", result.actual_speedup);
    }

    #[test]
    fn test_mixed_loop_optimization() {
        println!("\n=== Test: Mixed Loop Optimization ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Define mixed loop features (balanced)
        let features = LoopFeatures {
            memory_irregularity: 0.4,       // Medium irregularity
            arithmetic_intensity: 2.0,      // Medium arithmetic
            branch_density: 0.08,
            trip_count: 8000,
            vectorizable: true,
        };
        
        assert_eq!(features.classify(), LoopType::Mixed);
        println!("  Loop classified as: Mixed");
        
        let result = optimizer.optimize(&features);
        
        println!("  Loop type: {}", result.loop_type);
        println!("  Parameters: {}", result.parameters);
        println!("  Predicted speedup: {:.2}x", result.predicted_speedup);
        println!("  Actual speedup: {:.2}x", result.actual_speedup);
        println!("  Match quality: {:.1}%", result.match_quality * 100.0);
        
        assert_eq!(result.loop_type, LoopType::Mixed);
        assert!(result.predicted_speedup > 1.0, "Should predict speedup");
        assert!(result.actual_speedup > 0.0, "Should measure speedup");
        
        println!("✅ Mixed optimization: {:.2}x speedup", result.actual_speedup);
    }

    #[test]
    fn test_full_end_to_end_pipeline() {
        println!("\n=== Test: Full End-to-End Pipeline ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Test all three loop types
        let test_loops = vec![
            (
                "CPU-Bound",
                LoopFeatures {
                    memory_irregularity: 0.1,
                    arithmetic_intensity: 4.0,
                    branch_density: 0.02,
                    trip_count: 10000,
                    vectorizable: true,
                }
            ),
            (
                "Memory-Bound",
                LoopFeatures {
                    memory_irregularity: 0.8,
                    arithmetic_intensity: 0.4,
                    branch_density: 0.05,
                    trip_count: 5000,
                    vectorizable: false,
                }
            ),
            (
                "Mixed",
                LoopFeatures {
                    memory_irregularity: 0.45,
                    arithmetic_intensity: 1.8,
                    branch_density: 0.08,
                    trip_count: 8000,
                    vectorizable: true,
                }
            ),
        ];
        
        for (name, features) in test_loops {
            let result = optimizer.optimize(&features);
            
            println!("\n  {}: {} → {}", name, features.classify(), result.loop_type);
            println!("    Parameters: unroll={}, vec={}, inline={}, prefetch={}, opt={}",
                     result.parameters.unroll_factor,
                     result.parameters.vectorization,
                     result.parameters.inline_hints,
                     result.parameters.prefetch,
                     result.parameters.opt_level);
            println!("    Predicted: {:.2}x, Actual: {:.2}x, Match: {:.1}%",
                     result.predicted_speedup, result.actual_speedup, result.match_quality * 100.0);
            
            assert!(result.actual_speedup > 0.0, "Should measure positive speedup");
            assert!(result.loop_type == features.classify(), "Should classify correctly");
        }
        
        println!("\n✅ Full E2E pipeline: all 3 loop types optimized successfully");
    }

    #[test]
    fn test_accuracy_cpu_bound() {
        println!("\n=== Test: Parameter Accuracy - CPU-Bound ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Run multiple CPU-bound optimizations
        let test_cases = vec![
            (0.05, 3.5),  // Low memory, high arithmetic
            (0.15, 5.0),  // Low memory, very high arithmetic
            (0.08, 3.0),  // Very low memory, high arithmetic
        ];
        let test_cases_len = test_cases.len();
        
        let mut total_match = 0.0;
        
        for (memory_irr, arith_int) in &test_cases {
            let features = LoopFeatures {
                memory_irregularity: *memory_irr,
                arithmetic_intensity: *arith_int,
                branch_density: 0.01,
                trip_count: 10000,
                vectorizable: true,
            };
            
            let result = optimizer.optimize(&features);
            total_match += result.match_quality;
            
            println!("  Memory={:.2}, Arithmetic={:.2}: {:.2}x (match: {:.1}%)",
                     memory_irr, arith_int, result.actual_speedup, result.match_quality * 100.0);
        }
        
        let avg_match = total_match / test_cases_len as f64;
        println!("  Average match quality: {:.1}%", avg_match * 100.0);
        
        // Should be reasonably accurate for CPU-bound (at least 15%)
        assert!(avg_match > 0.15, "Should have decent accuracy for CPU-bound");
        println!("✅ CPU-bound parameters have adequate accuracy");
    }

    #[test]
    fn test_accuracy_memory_bound() {
        println!("\n=== Test: Parameter Accuracy - Memory-Bound ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Run multiple memory-bound optimizations
        let test_cases = vec![
            (0.75, 0.4),  // High memory, low arithmetic
            (0.85, 0.3),  // Very high memory, very low arithmetic
            (0.70, 0.6),  // High memory, low-medium arithmetic
        ];
        let test_cases_len_2 = test_cases.len();
        
        let mut total_match = 0.0;
        
        for (memory_irr, arith_int) in &test_cases {
            let features = LoopFeatures {
                memory_irregularity: *memory_irr,
                arithmetic_intensity: *arith_int,
                branch_density: 0.03,
                trip_count: 5000,
                vectorizable: false,
            };
            
            let result = optimizer.optimize(&features);
            total_match += result.match_quality;
            
            println!("  Memory={:.2}, Arithmetic={:.2}: {:.2}x (match: {:.1}%)",
                     memory_irr, arith_int, result.actual_speedup, result.match_quality * 100.0);
        }
        
        let avg_match = total_match / test_cases_len_2 as f64;
        println!("  Average match quality: {:.1}%", avg_match * 100.0);
        
        assert!(avg_match > 0.2, "Should have basic accuracy for memory-bound");
        println!("✅ Memory-bound parameters have baseline accuracy");
    }

    #[test]
    fn test_confidence_score_tracking() {
        println!("\n=== Test: Confidence Score Tracking ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Run same optimization
        let features = LoopFeatures {
            memory_irregularity: 0.2,
            arithmetic_intensity: 3.0,
            branch_density: 0.03,
            trip_count: 10000,
            vectorizable: true,
        };
        
        let result1 = optimizer.optimize(&features);
        println!("  CPU-bound confidence={:.1}%", result1.confidence * 100.0);
        assert!(result1.confidence > 0.0, "Should have confidence");
        
        println!("✅ Confidence scores tracked correctly: {:.1}%", result1.confidence * 100.0);
    }

    #[test]
    fn test_optimization_result_display() {
        println!("\n=== Test: OptimizationResult Display ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        let features = LoopFeatures {
            memory_irregularity: 0.3,
            arithmetic_intensity: 2.5,
            branch_density: 0.04,
            trip_count: 10000,
            vectorizable: true,
        };
        
        let result = optimizer.optimize(&features);
        
        // Test that Display trait works
        let display_string = format!("{}", result);
        
        println!("  Result display:\n{}", display_string);
        
        assert!(display_string.contains("loop_type"));
        assert!(display_string.contains("predicted"));
        assert!(display_string.contains("actual"));
        assert!(display_string.contains("match"));
        
        println!("✅ OptimizationResult Display trait working correctly");
    }

    #[test]
    fn test_results_caching_and_summary() {
        println!("\n=== Test: Results Caching and Summary ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Run multiple optimizations
        let test_features = vec![
            LoopFeatures {
                memory_irregularity: 0.1,
                arithmetic_intensity: 4.0,
                branch_density: 0.02,
                trip_count: 10000,
                vectorizable: true,
            },
            LoopFeatures {
                memory_irregularity: 0.8,
                arithmetic_intensity: 0.5,
                branch_density: 0.05,
                trip_count: 5000,
                vectorizable: false,
            },
            LoopFeatures {
                memory_irregularity: 0.4,
                arithmetic_intensity: 2.0,
                branch_density: 0.08,
                trip_count: 8000,
                vectorizable: true,
            },
        ];
        
        for features in test_features {
            optimizer.optimize(&features);
        }
        
        assert_eq!(optimizer.get_results().len(), 3);
        println!("  Cached 3 results");
        
        // Print summary (should not panic)
        optimizer.print_summary();
        
        // Clear and verify
        optimizer.clear_results();
        assert_eq!(optimizer.get_results().len(), 0);
        println!("  Results cleared successfully");
        
        println!("✅ Results caching and summary working correctly");
    }

    #[test]
    fn test_real_world_pattern_cpu_intensive() {
        println!("\n=== Test: Real-World Pattern - CPU Intensive ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Pattern: matrix multiplication (high arithmetic intensity)
        let features = LoopFeatures {
            memory_irregularity: 0.05,
            arithmetic_intensity: 3.5,
            branch_density: 0.00,
            trip_count: 100000,
            vectorizable: true,
        };
        
        let result = optimizer.optimize(&features);
        
        println!("  Pattern: Matrix multiplication");
        println!("  Classified as: {}", result.loop_type);
        println!("  Actual speedup: {:.2}x", result.actual_speedup);
        
        assert_eq!(result.loop_type, LoopType::CpuBound);
        assert!(result.actual_speedup > 1.0);
        
        println!("✅ Real-world CPU pattern optimized: {:.2}x speedup", result.actual_speedup);
    }

    #[test]
    fn test_real_world_pattern_memory_intensive() {
        println!("\n=== Test: Real-World Pattern - Memory Intensive ===");
        
        let mut optimizer = IntegratedOptimizer::new(20, 10);
        
        // Pattern: pointer chasing (low arithmetic, irregular access)
        let features = LoopFeatures {
            memory_irregularity: 0.85,
            arithmetic_intensity: 0.2,
            branch_density: 0.10,
            trip_count: 50000,
            vectorizable: false,
        };
        
        let result = optimizer.optimize(&features);
        
        println!("  Pattern: Pointer chasing");
        println!("  Classified as: {}", result.loop_type);
        println!("  Actual speedup: {:.2}x", result.actual_speedup);
        
        assert_eq!(result.loop_type, LoopType::MemoryBound);
        
        println!("✅ Real-world memory pattern optimized: {:.2}x speedup", result.actual_speedup);
    }

    #[test]
    fn test_integration_with_recommender() {
        println!("\n=== Test: Integration with ParameterRecommender ===");
        
        let mut recommender = ParameterRecommender::new(20, 10);
        recommender.discover_all();
        
        // Get parameters for each loop type
        let cpu_params = recommender.get_parameters(LoopType::CpuBound);
        let mem_params = recommender.get_parameters(LoopType::MemoryBound);
        let mixed_params = recommender.get_parameters(LoopType::Mixed);
        
        println!("  CPU-Bound params: {:?}", cpu_params.map(|p| &p.gene));
        println!("  Memory-Bound params: {:?}", mem_params.map(|p| &p.gene));
        println!("  Mixed params: {:?}", mixed_params.map(|p| &p.gene));
        
        assert!(cpu_params.is_some(), "Should discover CPU-bound parameters");
        assert!(mem_params.is_some(), "Should discover memory-bound parameters");
        assert!(mixed_params.is_some(), "Should discover mixed parameters");
        
        println!("✅ ParameterRecommender integration verified");
    }
}
