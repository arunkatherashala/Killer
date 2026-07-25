#![cfg(feature = "legacy-killer-rcore-tests")]
/// Week 6 Phase 2: Real-World Loop Testing
/// 
/// Tests 9 production-grade loop patterns across CPU-bound, memory-bound, and mixed workloads
/// Validates optimization effectiveness on realistic code scenarios

#[cfg(test)]
mod week6_phase2_realworld {
    use killer_rcore::optimization::{RustCompiler, OptLevel, GeneratedLoop};

    // ============================================================================
    // SCENARIO 1: CPU-BOUND KERNELS (Mathematical computation)
    // ============================================================================

    /// Matrix multiplication (16×16)
    /// Expected: 2.5x-4x speedup with O3 (nested FLOP-heavy loops)
    #[test]
    fn test_cpu_bound_matrix_multiplication() {
        let loop_def = GeneratedLoop {
            name: "matrix_mult_16x16".to_string(),
            code: r#"
    let mut a = vec![vec![1.0; 16]; 16];
    let mut b = vec![vec![2.0; 16]; 16];
    let mut c = vec![vec![0.0; 16]; 16];
    
    for i in 0..16 {
        for j in 0..16 {
            for k in 0..16 {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    (c[0][0] as u64)
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile matrix multiplication");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        // Matrix mult should show strong optimization benefit (nested loops + math)
        eprintln!("Matrix Multiplication (16×16): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.5,
            "Matrix mult should show at least 1.5x speedup, got {:.2}x",
            speedup
        );
    }

    /// Dot product (1M elements)
    /// Expected: 1.8x-3x speedup with O3 (SIMD-friendly vector operation)
    #[test]
    fn test_cpu_bound_dot_product() {
        let loop_def = GeneratedLoop {
            name: "dot_product_1m".to_string(),
            code: r#"
    let a: Vec<f64> = (0..1_000_000)
        .map(|i| i as f64)
        .collect();
    let b: Vec<f64> = (0..1_000_000)
        .map(|i| (i * 2) as f64)
        .collect();
    
    let mut result = 0.0;
    for i in 0..a.len() {
        result += a[i] * b[i];
    }
    (result as u64)
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile dot product");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Dot Product (1M elements): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.3,
            "Dot product should show at least 1.3x speedup, got {:.2}x",
            speedup
        );
    }

    /// Complex number arithmetic (1M iterations)
    /// Expected: 1.6x-2.5x speedup with O3 (multi-operation FLOP kernels)
    #[test]
    fn test_cpu_bound_complex_arithmetic() {
        let loop_def = GeneratedLoop {
            name: "complex_ops_1m".to_string(),
            code: r#"
    let mut result = 0.0;
    for i in 0..1_000_000u64 {
        let real = i as f64;
        let imag = (i * 2) as f64;
        
        // (a+bi) * (c+di) = (ac-bd) + (ad+bc)i
        let a = real;
        let b = imag;
        let c = real * 0.5;
        let d = imag * 0.5;
        
        let real_part = a * c - b * d;
        let imag_part = a * d + b * c;
        
        result += real_part * real_part + imag_part * imag_part;
    }
    (result as u64)
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile complex arithmetic");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Complex Arithmetic (1M iterations): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.3,
            "Complex arithmetic should show at least 1.3x speedup, got {:.2}x",
            speedup
        );
    }

    // ============================================================================
    // SCENARIO 2: MEMORY-BOUND PATTERNS (Array/vector operations)
    // ============================================================================

    /// Simplified image convolution (32×32 image)
    /// Expected: 1.5x-2.2x speedup with O3 (memory prefetch optimization)
    #[test]
    fn test_memory_bound_image_convolution() {
        let loop_def = GeneratedLoop {
            name: "image_conv_32x32".to_string(),
            code: r#"
    let size = 32u64;
    let mut img = vec![vec![1u64; 32]; 32];
    let mut result = vec![vec![0u64; 32]; 32];
    
    let kernel = [1, 2, 1, 2, 4, 2, 1, 2, 1];
    
    for y in 1..31usize {
        for x in 1..31usize {
            let mut sum = 0u64;
            for ky in 0..3usize {
                for kx in 0..3usize {
                    sum += img[y + ky - 1][x + kx - 1] * kernel[ky * 3 + kx];
                }
            }
            result[y][x] = sum / 16;
        }
    }
    result[1][1]
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile convolution");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Image Convolution (32×32): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.2,
            "Convolution should show at least 1.2x speedup, got {:.2}x",
            speedup
        );
    }

    /// Sparse matrix operations (10K elements with ~20% fill)
    /// Expected: 1.4x-1.9x speedup with O3 (sparse indexing optimization)
    #[test]
    fn test_memory_bound_sparse_matrix() {
        let loop_def = GeneratedLoop {
            name: "sparse_matrix_10k".to_string(),
            code: r#"
    let elements = 10_000u64;
    let mut matrix = vec![0u64; 10_000];
    let mut indices = vec![];
    
    // Simulate sparse matrix with ~20% fill
    for i in 0..10_000 {
        if (i * 7) % 5 == 0 {
            indices.push(i);
            matrix[i] = (i % 1000) as u64;
        }
    }
    
    let mut result = 0u64;
    for &idx in &indices {
        result = result.wrapping_add(matrix[idx] * 2);
    }
    result
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile sparse matrix");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Sparse Matrix (10K elements): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.1,
            "Sparse matrix should show at least 1.1x speedup, got {:.2}x",
            speedup
        );
    }

    /// Struct iteration (1K struct instances with 3 fields each)
    /// Expected: 1.3x-1.8x speedup with O3 (cache-aware layout optimization)
    #[test]
    fn test_memory_bound_struct_iteration() {
        let loop_def = GeneratedLoop {
            name: "struct_iter_1k".to_string(),
            code: r#"
    #[derive(Clone)]
    #[repr(C)]
    struct Point {
        x: u64,
        y: u64,
        z: u64,
    }
    
    let mut points = vec![Point { x: 0, y: 0, z: 0 }; 1000];
    
    for i in 0..points.len() {
        points[i].x = i as u64;
        points[i].y = (i % 100) as u64;
        points[i].z = (i / 100) as u64;
    }
    
    let mut sum = 0u64;
    for p in &points {
        sum = sum.wrapping_add(p.x + p.y + p.z);
    }
    sum
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile struct iteration");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Struct Iteration (1K instances): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.1,
            "Struct iteration should show at least 1.1x speedup, got {:.2}x",
            speedup
        );
    }

    // ============================================================================
    // SCENARIO 3: MIXED WORKLOADS (Computation + conditional + memory)
    // ============================================================================

    /// Streaming aggregation (1M iterations with multiple accumulators)
    /// Expected: 1.4x-2.0x speedup with O3 (register pressure optimization)
    #[test]
    fn test_mixed_streaming_aggregation() {
        let loop_def = GeneratedLoop {
            name: "stream_agg_1m".to_string(),
            code: r#"
    let mut sum = 0u64;
    let mut count_pos = 0u64;
    let mut max_val = 0u64;
    
    for i in 0..1_000_000u64 {
        let val = (i.wrapping_mul(17).wrapping_add(42)) % 10_000;
        
        sum = sum.wrapping_add(val);
        if val > 5000 {
            count_pos += 1;
        }
        if val > max_val {
            max_val = val;
        }
    }
    sum ^ count_pos ^ max_val
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile streaming aggregation");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Streaming Aggregation (1M items): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.2,
            "Streaming agg should show at least 1.2x speedup, got {:.2}x",
            speedup
        );
    }

    /// Filter and transform (1M items with conditional processing)
    /// Expected: 1.3x-1.9x speedup with O3 (if-statement optimization + loop unrolling)
    #[test]
    fn test_mixed_filter_and_transform() {
        let loop_def = GeneratedLoop {
            name: "filter_xform_1m".to_string(),
            code: r#"
    let data: Vec<u64> = (0..1_000_000u64)
        .map(|i: u64| (i.wrapping_mul(19).wrapping_add(7)) % 1000)
        .collect();
    
    let mut result = 0u64;
    for &val in &data {
        // Filter: keep values > 300
        if val > 300 {
            // Transform: apply function
            let transformed = (val.wrapping_mul(2).wrapping_add(5)) % 1000;
            result = result.wrapping_add(transformed);
        }
    }
    result
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile filter/transform");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Filter & Transform (1M items): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.1,
            "Filter/xform should show at least 1.1x speedup, got {:.2}x",
            speedup
        );
    }

    /// Branching-intensive code (1M iterations with match statements)
    /// Expected: 1.1x-1.4x speedup with O3 (branch prediction optimization)
    #[test]
    fn test_mixed_branching_intensive() {
        let loop_def = GeneratedLoop {
            name: "branching_1m".to_string(),
            code: r#"
    let mut result = 0u64;
    
    for i in 0..1_000_000u64 {
        let classification = match (i / 100) % 5 {
            0 => { result = result.wrapping_add(i); i },
            1 => { result = result.wrapping_mul((i % 10) + 1); i / 2 },
            2 => { result = result ^ i; i * 3 },
            3 => { if i % 2 == 0 { i } else { result.wrapping_sub(i) } },
            _ => { result = result.wrapping_add(1); 0 },
        };
        result = result.wrapping_add(classification);
    }
    result
"#.to_string(),
            iterations: 1,
        };

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");
        let results = compiler
            .compile_all_levels(&loop_def)
            .expect("Failed to compile branching");

        let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
        let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

        let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
        
        eprintln!("Branching-Intensive (1M iterations): {:.2}x speedup", speedup);
        assert!(
            speedup >= 1.05,
            "Branching-heavy should show at least 1.05x speedup, got {:.2}x",
            speedup
        );
    }

    // ============================================================================
    // COMPARATIVE ANALYSIS: All patterns together
    // ============================================================================

    /// Test all 9 patterns and compare optimization effectiveness
    #[test]
    fn test_scenario_comparative_speedup_analysis() {
        let patterns = vec![
            ("CPU: Matrix Mult", GeneratedLoop {
                name: "matrix_mult_16x16".to_string(),
                code: r#"
    let mut a = vec![vec![1.0; 16]; 16];
    let mut b = vec![vec![2.0; 16]; 16];
    let mut c = vec![vec![0.0; 16]; 16];
    
    for i in 0..16 {
        for j in 0..16 {
            for k in 0..16 {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    (c[0][0] as u64)
"#.to_string(),
                iterations: 1,
            }),
            ("CPU: Dot Product", GeneratedLoop {
                name: "dot_product_1m".to_string(),
                code: r#"
    let mut result = 0.0;
    for i in 0..1_000_000u64 {
        result += i as f64 * (i * 2) as f64;
    }
    (result as u64)
"#.to_string(),
                iterations: 1,
            }),
            ("CPU: Complex Ops", GeneratedLoop {
                name: "complex_ops_1m".to_string(),
                code: r#"
    let mut result = 0.0;
    for i in 0..1_000_000u64 {
        let a = i as f64;
        let b = (i * 2) as f64;
        result += (a * a) + (b * b);
    }
    (result as u64)
"#.to_string(),
                iterations: 1,
            }),
            ("MEM: Simple Array Sum", GeneratedLoop {
                name: "array_sum_1k".to_string(),
                code: r#"
    let arr = vec![1u64; 1000];
    let mut sum = 0u64;
    for &val in &arr {
        sum = sum.wrapping_add(val);
    }
    sum
"#.to_string(),
                iterations: 1,
            }),
            ("MEM: Vector Iter", GeneratedLoop {
                name: "vec_iter_100k".to_string(),
                code: r#"
    let vec = (0..100_000u64).collect::<Vec<_>>();
    let mut result = 0u64;
    for &v in &vec {
        result = result.wrapping_add(v % 100);
    }
    result
"#.to_string(),
                iterations: 1,
            }),
            ("MIX: Conditional Sum", GeneratedLoop {
                name: "cond_sum_1m".to_string(),
                code: r#"
    let mut result = 0u64;
    for i in 0..1_000_000u64 {
        if i % 2 == 0 {
            result = result.wrapping_add(i);
        }
    }
    result
"#.to_string(),
                iterations: 1,
            }),
            ("MIX: Mod Operation", GeneratedLoop {
                name: "mod_op_1m".to_string(),
                code: r#"
    let mut result = 0u64;
    for i in 1..1_000_001u64 {
        result = result.wrapping_add(i % 7);
    }
    result
"#.to_string(),
                iterations: 1,
            }),
            ("MIX: Multi-Op", GeneratedLoop {
                name: "multiop_1m".to_string(),
                code: r#"
    let mut sum = 0u64;
    let mut product = 1u64;
    for i in 0..1_000_000u64 {
        sum = sum.wrapping_add(i);
        product = product.wrapping_mul((i % 100) + 1);
    }
    sum ^ product
"#.to_string(),
                iterations: 1,
            }),
            ("BRANCH: Match Stmt", GeneratedLoop {
                name: "match_1m".to_string(),
                code: r#"
    let mut result = 0u64;
    for i in 0..1_000_000u64 {
        result = result.wrapping_add(match i % 3 {
            0 => 1,
            1 => 2,
            _ => 3,
        });
    }
    result
"#.to_string(),
                iterations: 1,
            }),
        ];

        let compiler = RustCompiler::new(true).expect("Failed to create compiler");

        eprintln!("\n+===============================================================+");
        eprintln!("|         WEEK 6 PHASE 2: REAL-WORLD LOOP PERFORMANCE         |");
        eprintln!("+===============================================================+\n");
        eprintln!("{:<30} | {:>10} | {:>10} | {:>10}", "Pattern", "O0 (ms)", "O3 (ms)", "Speedup");
        eprintln!("{:-<30}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");

        let mut speedups = vec![];

        for (name, loop_def) in patterns {
            let results = compiler
                .compile_all_levels(&loop_def)
                .expect(&format!("Failed to compile {}", name));

            let baseline = results.iter().find(|r| r.opt_level == OptLevel::O0).unwrap();
            let optimized = results.iter().find(|r| r.opt_level == OptLevel::O3).unwrap();

            let speedup = baseline.avg_execution_time_ms / optimized.avg_execution_time_ms;
            speedups.push(speedup);

            eprintln!(
                "{:<30} | {:>10.3} | {:>10.3} | {:>10.2}x",
                name,
                baseline.avg_execution_time_ms,
                optimized.avg_execution_time_ms,
                speedup
            );
        }

        let avg_speedup = speedups.iter().sum::<f64>() / speedups.len() as f64;
        eprintln!("{:-<30}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");
        eprintln!("{:<30} | {:>10} | {:>10} | {:>10.2}x", "AVERAGE SPEEDUP", "", "", avg_speedup);
        eprintln!("\n");

        // All patterns should show at least 1.05x speedup (5% improvement minimum)
        assert!(
            speedups.iter().all(|&s| s >= 1.05),
            "All patterns should show at least 1.05x speedup"
        );

        eprintln!("✅ All 9 patterns show optimization benefit\n");
    }
}
