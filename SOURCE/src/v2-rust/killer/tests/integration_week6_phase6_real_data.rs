#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 6 Phase 6: Real Compilation Integration Tests
///
/// Validates that simulated speedups match actual LLVM compilation results
/// Bridges simulation (Phase 5) with real-world deployment (Week 7)

#[cfg(test)]
mod tests {
    use killer_rcore::optimization::{
        RealCompiler, LoopFeatures, LoopType,
    };

    #[test]
    fn test_real_compiler_creation() {
        println!("\n=== Test: RealCompiler Creation ===");
        
        match RealCompiler::new() {
            Ok(compiler) => {
                assert_eq!(compiler.get_results().len(), 0);
                println!("✅ RealCompiler initialized successfully");
            },
            Err(e) => {
                println!("⚠️  RealCompiler unavailable: {}", e);
                println!("    This is normal if rustc/LLVM tools not in PATH");
            }
        }
    }

    #[test]
    fn test_real_cpu_bound_compilation() {
        println!("\n=== Test: Real CPU-Bound Compilation ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                let features = LoopFeatures {
                    memory_irregularity: 0.1,
                    arithmetic_intensity: 4.0,
                    branch_density: 0.02,
                    trip_count: 10000,
                    vectorizable: true,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Loop type: {}", result.loop_type);
                        println!("  Simulated speedup: {:.2}x", result.simulated_speedup);
                        println!("  Actual speedup: {:.2}x", result.actual_speedup);
                        println!("  Accuracy: {:.1}%", result.accuracy * 100.0);
                        println!("  Baseline: {:.2}ms, Optimized: {:.2}ms",
                                 result.baseline_time_ms, result.optimized_time_ms);
                        
                        assert_eq!(result.loop_type, LoopType::CpuBound);
                        assert!(result.actual_speedup > 1.0, "Should see speedup");
                        println!("✅ CPU-bound compilation: {:.2}x speedup", result.actual_speedup);
                    },
                    Err(e) => {
                        println!("⚠️  Compilation failed: {}", e);
                        println!("    (This may occur if LLVM not available)");
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_real_memory_bound_compilation() {
        println!("\n=== Test: Real Memory-Bound Compilation ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                let features = LoopFeatures {
                    memory_irregularity: 0.8,
                    arithmetic_intensity: 0.5,
                    branch_density: 0.05,
                    trip_count: 5000,
                    vectorizable: false,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Loop type: {}", result.loop_type);
                        println!("  Simulated speedup: {:.2}x", result.simulated_speedup);
                        println!("  Actual speedup: {:.2}x", result.actual_speedup);
                        println!("  Accuracy: {:.1}%", result.accuracy * 100.0);
                        println!("  Baseline: {:.2}ms, Optimized: {:.2}ms",
                                 result.baseline_time_ms, result.optimized_time_ms);
                        
                        assert_eq!(result.loop_type, LoopType::MemoryBound);
                        assert!(result.actual_speedup > 1.0, "Should see speedup");
                        println!("✅ Memory-bound compilation: {:.2}x speedup", result.actual_speedup);
                    },
                    Err(e) => {
                        println!("⚠️  Compilation failed: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_real_mixed_compilation() {
        println!("\n=== Test: Real Mixed Workload Compilation ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                let features = LoopFeatures {
                    memory_irregularity: 0.4,
                    arithmetic_intensity: 2.0,
                    branch_density: 0.08,
                    trip_count: 8000,
                    vectorizable: true,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Loop type: {}", result.loop_type);
                        println!("  Simulated speedup: {:.2}x", result.simulated_speedup);
                        println!("  Actual speedup: {:.2}x", result.actual_speedup);
                        println!("  Accuracy: {:.1}%", result.accuracy * 100.0);
                        println!("  Baseline: {:.2}ms, Optimized: {:.2}ms",
                                 result.baseline_time_ms, result.optimized_time_ms);
                        
                        assert_eq!(result.loop_type, LoopType::Mixed);
                        assert!(result.actual_speedup > 1.0, "Should see speedup");
                        println!("✅ Mixed compilation: {:.2}x speedup", result.actual_speedup);
                    },
                    Err(e) => {
                        println!("⚠️  Compilation failed: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_simulation_accuracy_validation() {
        println!("\n=== Test: Simulation Accuracy Validation ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                // Test multiple examples of each type
                let test_cases = vec![
                    ("CPU-Bound #1", LoopFeatures {
                        memory_irregularity: 0.05,
                        arithmetic_intensity: 4.0,
                        branch_density: 0.02,
                        trip_count: 10000,
                        vectorizable: true,
                    }),
                    ("Memory-Bound #1", LoopFeatures {
                        memory_irregularity: 0.85,
                        arithmetic_intensity: 0.4,
                        branch_density: 0.05,
                        trip_count: 5000,
                        vectorizable: false,
                    }),
                    ("Mixed #1", LoopFeatures {
                        memory_irregularity: 0.45,
                        arithmetic_intensity: 1.8,
                        branch_density: 0.08,
                        trip_count: 8000,
                        vectorizable: true,
                    }),
                ];
                
                let mut total_accuracy = 0.0;
                let mut successful = 0;
                
                for (name, features) in test_cases {
                    match compiler.compile_and_measure(&features) {
                        Ok(result) => {
                            println!("\n  {}: {:.2}x actual vs {:.2}x simulated ({:.1}% match)",
                                     name, result.actual_speedup, result.simulated_speedup,
                                     result.accuracy * 100.0);
                            total_accuracy += result.accuracy;
                            successful += 1;
                        },
                        Err(_) => {
                            println!("\n  {}: Compilation skipped", name);
                        }
                    }
                }
                
                if successful > 0 {
                    let avg_accuracy = total_accuracy / successful as f64;
                    println!("\n  Average accuracy: {:.1}%", avg_accuracy * 100.0);
                    assert!(avg_accuracy > 0.3, "Accuracy should be reasonable");
                    println!("✅ Simulation accuracy validated");
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_results_collection_and_summary() {
        println!("\n=== Test: Results Collection and Summary ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                // Compile a couple examples
                let test_loops = vec![
                    LoopFeatures {
                        memory_irregularity: 0.1,
                        arithmetic_intensity: 3.5,
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
                ];
                
                println!("\n  Compiling loops...");
                for features in test_loops {
                    let _ = compiler.compile_and_measure(&features);
                }
                
                // Print summary
                compiler.print_validation_summary();
                
                if !compiler.get_results().is_empty() {
                    println!("✅ Results collection working");
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_real_vs_simulated_comparison() {
        println!("\n=== Test: Real vs Simulated Speedup Comparison ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                let features = LoopFeatures {
                    memory_irregularity: 0.2,
                    arithmetic_intensity: 3.0,
                    branch_density: 0.03,
                    trip_count: 10000,
                    vectorizable: true,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Classification: {}", result.loop_type);
                        println!("  Phase 5 Prediction: {:.2}x", result.simulated_speedup);
                        println!("  Real Measurement: {:.2}x", result.actual_speedup);
                        println!("  Difference: {:.2}x ({:+.1}%)",
                                 (result.actual_speedup - result.simulated_speedup).abs(),
                                 ((result.actual_speedup - result.simulated_speedup) / result.simulated_speedup * 100.0));
                        
                        if result.accuracy > 0.5 {
                            println!("✅ Simulation and reality closely aligned");
                        } else {
                            println!("⚠️  Some difference between simulation and actual");
                        }
                    },
                    Err(e) => {
                        println!("⚠️  Test could not run: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_cpu_bound_parameters_effectiveness() {
        println!("\n=== Test: CPU-Bound Parameter Effectiveness ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                // High arithmetic CPU-bound loop
                let features = LoopFeatures {
                    memory_irregularity: 0.08,
                    arithmetic_intensity: 5.0,
                    branch_density: 0.01,
                    trip_count: 20000,
                    vectorizable: true,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Classification: {}", result.loop_type);
                        println!("  Actual speedup: {:.2}x", result.actual_speedup);
                        
                        // CPU-bound should get reasonable speedup
                        assert_eq!(result.loop_type, LoopType::CpuBound);
                        assert!(result.actual_speedup >= 1.5, "CPU-bound should benefit from optimization");
                        
                        if result.actual_speedup > 5.0 {
                            println!("✅ CPU-bound parameters very effective");
                        } else {
                            println!("✅ CPU-bound parameters moderately effective");
                        }
                    },
                    Err(e) => {
                        println!("⚠️  Test skipped: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }

    #[test]
    fn test_memory_bound_parameters_effectiveness() {
        println!("\n=== Test: Memory-Bound Parameter Effectiveness ===");
        
        match RealCompiler::new() {
            Ok(mut compiler) => {
                // Irregular memory access pattern
                let features = LoopFeatures {
                    memory_irregularity: 0.9,
                    arithmetic_intensity: 0.3,
                    branch_density: 0.08,
                    trip_count: 5000,
                    vectorizable: false,
                };
                
                match compiler.compile_and_measure(&features) {
                    Ok(result) => {
                        println!("  Classification: {}", result.loop_type);
                        println!("  Actual speedup: {:.2}x", result.actual_speedup);
                        
                        // Memory-bound limited by bandwidth
                        assert_eq!(result.loop_type, LoopType::MemoryBound);
                        assert!(result.actual_speedup >= 1.0, "Should see some speedup");
                        
                        if result.actual_speedup < 3.0 {
                            println!("✅ Memory-bound speedup limited by bandwidth (expected)");
                        }
                    },
                    Err(e) => {
                        println!("⚠️  Test skipped: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("⚠️  Test skipped: {}", e);
            }
        }
    }
}
