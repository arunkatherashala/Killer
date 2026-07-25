#![cfg(feature = "legacy-killer-rcore-tests")]
// Week 4 Benchmark Tests
// Comprehensive performance validation suite

#[cfg(test)]
mod week4_benchmark_tests {
    use killer_rcore::BenchmarkRunner;
    
    /// Test that quick benchmark runs successfully
    #[test]
    #[ignore]  // Ignored by default - run with: cargo test week4_quick_benchmark -- --ignored --nocapture
    fn week4_quick_benchmark() {
        println!("\n🚀 Week 4: Quick Performance Benchmark\n");
        
        let runner = BenchmarkRunner::new().with_verbose(true);
        
        match runner.run_quick_benchmark() {
            Ok(metrics) => {
                println!("\n✅ Quick Benchmark Results:");
                println!("  Iterations: {}", metrics.iterations);
                println!("  Interpreter Time: {:.3}s", metrics.interpreter_time.as_secs_f64());
                println!("  JIT Time: {:.3}s", metrics.jit_time.as_secs_f64());
                println!("  Speedup: {:.2}x", metrics.speedup());
                println!("  Target Met: {}", if metrics.meets_target() { "✅" } else { "❌" });
                
                // Assert we hit at least 50x speedup
                assert!(metrics.speedup() >= 50.0, 
                    "Speedup {:.2}x is below 50x target", metrics.speedup());
            }
            Err(e) => {
                eprintln!("❌ Quick benchmark failed: {}", e);
                panic!("Quick benchmark failure: {}", e);
            }
        }
    }
    
    /// Test simple loop benchmarks
    #[test]
    #[ignore]  // Run with: cargo test week4_simple_loops -- --ignored --nocapture
    fn week4_simple_loops() {
        println!("\n📊 Week 4: Simple Loop Benchmarks\n");
        
        let runner = BenchmarkRunner::new().with_verbose(true);
        
        match runner.run_simple_loop_benchmarks() {
            Ok(results) => {
                println!("\n📈 Simple Loop Results:");
                for metric in &results {
                    println!("  {}: {:.2}x speedup", metric.name, metric.speedup());
                }
                
                // Check average
                if !results.is_empty() {
                    let avg = results.iter().map(|m| m.speedup()).sum::<f64>() / results.len() as f64;
                    println!("\n  Average Speedup: {:.2}x", avg);
                    assert!(avg >= 50.0, "Average speedup below 50x target");
                }
            }
            Err(e) => {
                eprintln!("❌ Simple loop benchmarks failed: {}", e);
                panic!("Simple loop test failure: {}", e);
            }
        }
    }
    
    /// Test nested loop benchmarks
    #[test]
    #[ignore]  // Run with: cargo test week4_nested_loops -- --ignored --nocapture
    fn week4_nested_loops() {
        println!("\n📊 Week 4: Nested Loop Benchmarks\n");
        
        let runner = BenchmarkRunner::new().with_verbose(true);
        
        match runner.run_nested_loop_benchmarks() {
            Ok(results) => {
                println!("\n📈 Nested Loop Results:");
                for metric in &results {
                    println!("  {}: {:.2}x speedup", metric.name, metric.speedup());
                }
                
                // Nested loops should see significant speedup
                if !results.is_empty() {
                    let avg = results.iter().map(|m| m.speedup()).sum::<f64>() / results.len() as f64;
                    println!("\n  Average Speedup: {:.2}x", avg);
                    assert!(avg >= 50.0, "Nested loop speedup below 50x target");
                }
            }
            Err(e) => {
                eprintln!("❌ Nested loop benchmarks failed: {}", e);
                panic!("Nested loop test failure: {}", e);
            }
        }
    }
    
    /// Test conditional loop benchmarks
    #[test]
    #[ignore]  // Run with: cargo test week4_conditional_loops -- --ignored --nocapture
    fn week4_conditional_loops() {
        println!("\n📊 Week 4: Conditional Loop Benchmarks\n");
        
        let runner = BenchmarkRunner::new().with_verbose(true);
        
        match runner.run_conditional_loop_benchmarks() {
            Ok(results) => {
                println!("\n📈 Conditional Loop Results:");
                for metric in &results {
                    println!("  {}: {:.2}x speedup", metric.name, metric.speedup());
                }
                
                if !results.is_empty() {
                    let avg = results.iter().map(|m| m.speedup()).sum::<f64>() / results.len() as f64;
                    println!("\n  Average Speedup: {:.2}x", avg);
                    assert!(avg >= 40.0, "Conditional loop speedup below 40x target (conditional adds overhead)");
                }
            }
            Err(e) => {
                eprintln!("❌ Conditional loop benchmarks failed: {}", e);
                panic!("Conditional loop test failure: {}", e);
            }
        }
    }
    
    /// Run complete benchmark suite and generate report
    #[test]
    #[ignore]  // Run with: cargo test week4_full_suite -- --ignored --nocapture
    fn week4_full_suite() {
        println!("\n🚀 Week 4: Full Benchmark Suite\n");
        
        let runner = BenchmarkRunner::new().with_verbose(true);
        
        match runner.run_all_benchmarks() {
            Ok(report) => {
                println!("\n✅ BENCHMARK SUITE COMPLETE\n");
                
                // Print markdown report
                let markdown = report.to_markdown();
                println!("{}", markdown);
                
                // Save report to file
                if let Ok(_) = std::fs::write(
                    "KILLER_V4_WEEK_4_BENCHMARKS.md",
                    markdown
                ) {
                    println!("\n📝 Report saved to: KILLER_V4_WEEK_4_BENCHMARKS.md");
                }
                
                // Summary statistics
                println!("\n📊 Summary Statistics:");
                println!("  Total Benchmarks: {}", report.summary.total_benchmarks);
                println!("  Average Speedup: {:.2}x", report.summary.avg_speedup);
                println!("  Min Speedup: {:.2}x", report.summary.min_speedup);
                println!("  Max Speedup: {:.2}x", report.summary.max_speedup);
                println!("  Meeting Target: {}/{}", 
                    report.summary.meeting_target, 
                    report.summary.total_benchmarks);
                println!("  Cache Hit Rate: {:.1}%", report.summary.cache_hit_rate * 100.0);
                
                // Validation
                let success = report.summary.meeting_target == report.summary.total_benchmarks;
                if success {
                    println!("\n✅ ALL BENCHMARKS MEET OR EXCEED 50X TARGET!");
                } else {
                    println!("\n⚠️  {} benchmarks below target", 
                        report.summary.total_benchmarks - report.summary.meeting_target);
                }
                
                assert!(success || report.summary.avg_speedup >= 50.0, 
                    "Performance not meeting expectations");
            }
            Err(e) => {
                eprintln!("❌ Full benchmark suite failed: {}", e);
                panic!("Benchmark suite failure: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod week4_unit_tests {
    use killer_rcore::{BenchmarkMetrics, LoopType};
    use std::time::Duration;
    
    #[test]
    fn test_metrics_speedup_calculation() {
        let metrics = BenchmarkMetrics {
            name: "test".to_string(),
            iterations: 1_000_000,
            interpreter_time: Duration::from_secs_f64(12.5),
            jit_time: Duration::from_secs_f64(0.125),
            compilation_time: Some(Duration::from_millis(500)),
            cache_hit: false,
            peak_memory: 0,
            loop_type: LoopType::Simple,
        };
        
        // 12.5 / 0.125 = 100x
        assert!((metrics.speedup() - 100.0).abs() < 0.1);
    }
    
    #[test]
    fn test_metrics_target_validation() {
        let good = BenchmarkMetrics {
            name: "good".to_string(),
            iterations: 1_000_000,
            interpreter_time: Duration::from_secs_f64(10.0),
            jit_time: Duration::from_secs_f64(0.1),
            compilation_time: None,
            cache_hit: true,
            peak_memory: 0,
            loop_type: LoopType::Simple,
        };
        
        let below_target = BenchmarkMetrics {
            name: "below".to_string(),
            iterations: 1_000,
            interpreter_time: Duration::from_secs_f64(0.1),
            jit_time: Duration::from_secs_f64(0.005),  // 20x speedup - below 50x target
            compilation_time: None,
            cache_hit: false,
            peak_memory: 0,
            loop_type: LoopType::Simple,
        };
        
        assert!(good.meets_target());  // 100x should meet target
        assert!(!below_target.meets_target());  // 20x should NOT meet 50x target
    }
    
    #[test]
    fn test_throughput_calculation() {
        let metrics = BenchmarkMetrics {
            name: "test".to_string(),
            iterations: 1_000_000,
            interpreter_time: Duration::from_secs_f64(10.0),
            jit_time: Duration::from_secs_f64(0.01),  // 10ms
            compilation_time: None,
            cache_hit: true,
            peak_memory: 0,
            loop_type: LoopType::Simple,
        };
        
        let ips = metrics.throughput_ips();
        assert!(ips >= 99_000_000.0);  // Should be ~100M iterations/sec
    }
}
