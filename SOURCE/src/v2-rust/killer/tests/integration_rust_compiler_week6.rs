#![cfg(feature = "legacy-killer-rcore-tests")]
/// Integration tests for RustCompiler real compilation
/// 
/// Tests real Rust compilation with various optimization levels
/// Validates performance measurement and comparative analysis

#[cfg(test)]
mod integration_rust_compiler {
    use killer_rcore::optimization::{RustCompiler, OptLevel, GeneratedLoop};

    /// Helper to create a simple arithmetic loop
    fn simple_arithmetic_loop(iterations: u64) -> GeneratedLoop {
        GeneratedLoop {
            name: "simple_arithmetic".to_string(),
            code: format!(
                r#"
    let mut result = 0u64;
    for i in 0..{} {{
        result = result.wrapping_add(i);
        result = result.wrapping_mul(2);
    }}
    result
"#,
                iterations
            ),
            iterations,
        }
    }

    /// Helper to create an array access loop
    fn array_access_loop(iterations: u64) -> GeneratedLoop {
        GeneratedLoop {
            name: "array_access".to_string(),
            code: format!(
                r#"
    let mut arr = vec![0u64; 1000];
    let mut result = 0u64;
    for i in 0..{} {{
        let idx = (i as usize) % 1000;
        arr[idx] = arr[idx].wrapping_add(i);
        result = result.wrapping_add(arr[idx]);
    }}
    result
"#,
                iterations
            ),
            iterations,
        }
    }

    /// Test 1: Basic compilation with O0 optimization level
    #[test]
    fn test_compile_with_o0() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let result = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile with O0");
        
        assert_eq!(result.opt_level, OptLevel::O0);
        assert!(result.compile_time_ms > 0.0);
        assert!(result.avg_execution_time_ms > 0.0);
        assert!(result.binary_size_kb > 0.0);
    }

    /// Test 2: Basic compilation with O2 optimization level
    #[test]
    fn test_compile_with_o2() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let result = compiler
            .compile_and_measure(&loop_def, OptLevel::O2)
            .expect("Failed to compile with O2");
        
        assert_eq!(result.opt_level, OptLevel::O2);
        assert!(result.compile_time_ms > 0.0);
        assert!(result.avg_execution_time_ms > 0.0);
    }

    /// Test 3: Compilation with O3 (most aggressive) optimization level
    #[test]
    fn test_compile_with_o3() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let result = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile with O3");
        
        assert_eq!(result.opt_level, OptLevel::O3);
        assert!(result.compile_time_ms > 0.0);
        assert!(result.avg_execution_time_ms > 0.0);
    }

    /// Test 4: Compile all standard optimization levels
    #[test]
    fn test_compile_all_levels() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile all levels");
        
        // Should have 5 results: O0, O1, O2, O3, Oz
        assert_eq!(results.len(), 5);
        
        // Verify each level is present
        assert!(results.iter().any(|r| r.opt_level == OptLevel::O0));
        assert!(results.iter().any(|r| r.opt_level == OptLevel::O1));
        assert!(results.iter().any(|r| r.opt_level == OptLevel::O2));
        assert!(results.iter().any(|r| r.opt_level == OptLevel::O3));
        assert!(results.iter().any(|r| r.opt_level == OptLevel::Oz));
    }

    /// Test 5: O2 should be faster than O0
    #[test]
    fn test_o2_faster_than_o0() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let o0 = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile with O0");
        
        let o2 = compiler
            .compile_and_measure(&loop_def, OptLevel::O2)
            .expect("Failed to compile with O2");
        
        // O2 should be at least 10% faster than O0
        let speedup = o0.avg_execution_time_ms / o2.avg_execution_time_ms;
        assert!(
            speedup >= 1.1,
            "O2 should be at least 10% faster than O0, but speedup was {}x",
            speedup
        );
    }

    /// Test 6: O3 should be faster than or equal to O2
    #[test]
    fn test_o3_faster_or_equal_to_o2() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(1_000_000); // Larger iterations for more stable results
        
        let o2 = compiler
            .compile_and_measure(&loop_def, OptLevel::O2)
            .expect("Failed to compile with O2");
        
        let o3 = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile with O3");
        
        // O3 should typically be at least as fast as O2 (allowing 10% variance for system noise)
        let speedup = o2.avg_execution_time_ms / o3.avg_execution_time_ms;
        assert!(
            speedup >= 0.95,
            "O3 should be comparable to O2, but speedup was {}x",
            speedup
        );
    }

    /// Test 7: Array access loop compilation
    #[test]
    fn test_array_access_compilation() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = array_access_loop(50_000);
        
        let o0 = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile array access with O0");
        
        let o3 = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile array access with O3");
        
        // Array access should also benefit from optimization
        let speedup = o0.avg_execution_time_ms / o3.avg_execution_time_ms;
        assert!(
            speedup >= 1.05,
            "Array access should show some optimization benefit, speedup: {}x",
            speedup
        );
    }

    /// Test 8: Speedup calculation
    #[test]
    fn test_speedup_calculation() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let baseline = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile baseline");
        
        let optimized = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile optimized");
        
        let speedup = optimized.speedup_vs(&baseline);
        assert!(speedup > 1.0, "Speedup should be greater than 1.0");
    }

    /// Test 9: Size ratio calculation
    #[test]
    fn test_size_ratio_calculation() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let baseline = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile baseline");
        
        let optimized = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile optimized");
        
        let ratio = optimized.size_ratio_vs(&baseline);
        assert!(ratio > 0.0, "Size ratio should be positive");
        // O3 typically produces larger binaries than O0
        assert!(ratio > 0.5 && ratio < 2.0, "Size ratio should be reasonable");
    }

    /// Test 10: Consistency across multiple runs
    #[test]
    fn test_execution_consistency() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(1_000_000); // Larger iterations reduce variance
        
        // Compile once
        let result1 = compiler
            .compile_and_measure(&loop_def, OptLevel::O2)
            .expect("First compilation failed");
        
        // Compile again
        let result2 = compiler
            .compile_and_measure(&loop_def, OptLevel::O2)
            .expect("Second compilation failed");
        
        // Results should be reasonably consistent (within 35% for system variance)
        // Note: JIT can have significant variance on small iterations due to warmup
        let variance = (result1.avg_execution_time_ms / result2.avg_execution_time_ms - 1.0).abs();
        assert!(
            variance < 0.35,
            "Execution times should be reasonably consistent, variance: {}%",
            variance * 100.0
        );
    }

    /// Test 11: Larger iteration count scaling
    #[test]
    fn test_scaling_with_larger_iterations() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        
        // Test with larger iteration count
        let loop_1m = simple_arithmetic_loop(1_000_000);
        
        let o0 = compiler
            .compile_and_measure(&loop_1m, OptLevel::O0)
            .expect("Failed to compile 1M iterations O0");
        
        let o3 = compiler
            .compile_and_measure(&loop_1m, OptLevel::O3)
            .expect("Failed to compile 1M iterations O3");
        
        // Should still see optimization benefit
        let speedup = o0.avg_execution_time_ms / o3.avg_execution_time_ms;
        assert!(speedup >= 1.1, "Should see optimization benefit at 1M iterations");
    }

    /// Test 12: Compilation time vs execution time tradeoff
    #[test]
    fn test_compilation_optimization_tradeoff() {
        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let loop_def = simple_arithmetic_loop(100_000);
        
        let o0 = compiler
            .compile_and_measure(&loop_def, OptLevel::O0)
            .expect("Failed to compile O0");
        
        let o3 = compiler
            .compile_and_measure(&loop_def, OptLevel::O3)
            .expect("Failed to compile O3");
        
        // O3 will take longer to compile but produce faster code
        assert!(
            o3.compile_time_ms >= o0.compile_time_ms,
            "O3 should take at least as long to compile as O0"
        );
        
        // But execution should be faster
        assert!(
            o3.avg_execution_time_ms < o0.avg_execution_time_ms,
            "O3 should produce faster code than O0"
        );
    }
}
