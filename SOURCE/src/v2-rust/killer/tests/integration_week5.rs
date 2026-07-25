#![cfg(feature = "legacy-killer-rcore-tests")]
// Integration tests for Week 5: Array and Function Call Loops
// Tests the new loop types added to BenchmarkHarness and BenchmarkRunner

use killer_rcore::benchmark::harness::BenchmarkHarness;
use killer_rcore::benchmark::runner::BenchmarkRunner;
use killer_rcore::benchmark::metrics::LoopType;

#[test]
fn test_array_loop_100k_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_array_loop(100_000);
    assert!(result.is_ok(), "Array loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::ArrayAccess);
    assert_eq!(metrics.iterations, 100_000);
    assert!(metrics.speedup() > 1.0, "Should achieve some speedup over interpreter baseline");
}

#[test]
fn test_array_loop_1m_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_array_loop(1_000_000);
    assert!(result.is_ok(), "Array loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::ArrayAccess);
    assert_eq!(metrics.iterations, 1_000_000);
    
    // Array access pattern overhead is 3x interpreter baseline
    // At 1M iterations: baseline ~37.5ms, JIT should achieve 80-100x speedup
    let speedup = metrics.speedup();
    assert!(speedup >= 70.0, "Array loop should achieve 70x+ speedup, got {}", speedup);
}

#[test]
fn test_array_loop_10m_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_array_loop(10_000_000);
    assert!(result.is_ok(), "Array loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::ArrayAccess);
    assert_eq!(metrics.iterations, 10_000_000);
    
    // At 10M iterations: baseline ~375ms, JIT should maintain 80-100x speedup
    let speedup = metrics.speedup();
    assert!(speedup >= 70.0, "Array loop should maintain 70x+ speedup at scale, got {}", speedup);
}

#[test]
fn test_function_call_loop_100k_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_function_call_loop(100_000);
    assert!(result.is_ok(), "Function call loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::FunctionCall);
    assert_eq!(metrics.iterations, 100_000);
    assert!(metrics.speedup() > 1.0, "Should achieve some speedup over interpreter baseline");
}

#[test]
fn test_function_call_loop_1m_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_function_call_loop(1_000_000);
    assert!(result.is_ok(), "Function call loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::FunctionCall);
    assert_eq!(metrics.iterations, 1_000_000);
    
    // Function call pattern overhead is 5x interpreter baseline
    // At 1M iterations: baseline ~62.5ms, JIT should achieve 50-80x speedup
    let speedup = metrics.speedup();
    assert!(speedup >= 40.0, "Function call loop should achieve 40x+ speedup, got {}", speedup);
}

#[test]
fn test_function_call_loop_10m_iterations() {
    let harness = BenchmarkHarness::new();
    
    let result = harness.benchmark_function_call_loop(10_000_000);
    assert!(result.is_ok(), "Function call loop benchmark should succeed");
    
    let metrics = result.unwrap();
    assert_eq!(metrics.loop_type, LoopType::FunctionCall);
    assert_eq!(metrics.iterations, 10_000_000);
    
    // At 10M iterations: baseline ~625ms, JIT should maintain 50-80x speedup
    let speedup = metrics.speedup();
    assert!(speedup >= 40.0, "Function call loop should maintain 40x+ speedup at scale, got {}", speedup);
}

#[test]
fn test_runner_array_loop_benchmarks() {
    let runner = BenchmarkRunner::new().with_verbose(false);
    
    let results = runner.run_array_loop_benchmarks();
    assert!(results.is_ok(), "Array loop benchmark suite should succeed");
    
    let metrics = results.unwrap();
    assert_eq!(metrics.len(), 3, "Should run 3 array loop benchmarks (100K, 1M, 10M)");
    
    // All should achieve reasonable speedup
    for m in &metrics {
        assert!(m.speedup() >= 50.0, "All array loops should achieve 50x+ speedup");
    }
}

#[test]
fn test_runner_function_call_loop_benchmarks() {
    let runner = BenchmarkRunner::new().with_verbose(false);
    
    let results = runner.run_function_call_loop_benchmarks();
    assert!(results.is_ok(), "Function call loop benchmark suite should succeed");
    
    let metrics = results.unwrap();
    assert_eq!(metrics.len(), 3, "Should run 3 function call loop benchmarks (100K, 1M, 10M)");
    
    // All should achieve reasonable speedup
    for m in &metrics {
        assert!(m.speedup() >= 30.0, "All function call loops should achieve 30x+ speedup");
    }
}

#[test]
fn test_runner_all_benchmarks_includes_new_loops() {
    let runner = BenchmarkRunner::new().with_verbose(false);
    
    let report = runner.run_all_benchmarks();
    assert!(report.is_ok(), "Comprehensive benchmark suite should succeed");
    
    let perf_report = report.unwrap();
    
    // Verify report includes benchmarks from all types
    let report_str = perf_report.to_markdown();
    assert!(report_str.contains("Speedup") || report_str.contains("speedup"), 
        "Report should contain speedup metrics");
}

#[test]
fn test_array_loop_vs_function_call_performance() {
    let harness = BenchmarkHarness::new();
    
    let array_result = harness.benchmark_array_loop(1_000_000);
    let function_result = harness.benchmark_function_call_loop(1_000_000);
    
    assert!(array_result.is_ok());
    assert!(function_result.is_ok());
    
    let array_metrics = array_result.unwrap();
    let function_metrics = function_result.unwrap();
    
    // Array access (3x overhead) should be faster than function calls (5x overhead)
    let array_speedup = array_metrics.speedup();
    let function_speedup = function_metrics.speedup();
    
    println!("Array speedup: {:.2}x, Function speedup: {:.2}x", array_speedup, function_speedup);
    
    // Function calls generally have lower speedup due to higher interpreter overhead
    // But due to compilation characteristics, this may vary
    assert!(array_speedup > 1.0 && function_speedup > 1.0, 
        "Both should achieve some speedup");
}

#[test]
fn test_array_loop_scaling() {
    let harness = BenchmarkHarness::new();
    
    let small = harness.benchmark_array_loop(100_000).unwrap();
    let medium = harness.benchmark_array_loop(1_000_000).unwrap();
    let large = harness.benchmark_array_loop(10_000_000).unwrap();
    
    println!("Array Loop Scaling:");
    println!("  100K: {:.2}x speedup, {:.3}ms JIT", small.speedup(), small.jit_time.as_secs_f64() * 1000.0);
    println!("  1M:   {:.2}x speedup, {:.3}ms JIT", medium.speedup(), medium.jit_time.as_secs_f64() * 1000.0);
    println!("  10M:  {:.2}x speedup, {:.3}ms JIT", large.speedup(), large.jit_time.as_secs_f64() * 1000.0);
    
    // Speedup should remain relatively consistent as iterations increase
    let speeds = vec![small.speedup(), medium.speedup(), large.speedup()];
    let min_speed = speeds.iter().copied().fold(f64::INFINITY, f64::min);
    let max_speed = speeds.iter().copied().fold(0.0, f64::max);
    let speedup_ratio = min_speed / max_speed;
    
    assert!(speedup_ratio > 0.80, "Speedup should remain consistent across scales (ratio: {:.1}%)", speedup_ratio * 100.0);
}

#[test]
fn test_function_call_loop_scaling() {
    let harness = BenchmarkHarness::new();
    
    let small = harness.benchmark_function_call_loop(100_000).unwrap();
    let medium = harness.benchmark_function_call_loop(1_000_000).unwrap();
    let large = harness.benchmark_function_call_loop(10_000_000).unwrap();
    
    println!("Function Call Loop Scaling:");
    println!("  100K: {:.2}x speedup, {:.3}ms JIT", small.speedup(), small.jit_time.as_secs_f64() * 1000.0);
    println!("  1M:   {:.2}x speedup, {:.3}ms JIT", medium.speedup(), medium.jit_time.as_secs_f64() * 1000.0);
    println!("  10M:  {:.2}x speedup, {:.3}ms JIT", large.speedup(), large.jit_time.as_secs_f64() * 1000.0);
    
    // Speedup should remain relatively consistent as iterations increase
    let speeds = vec![small.speedup(), medium.speedup(), large.speedup()];
    let min_speed = speeds.iter().copied().fold(f64::INFINITY, f64::min);
    let max_speed = speeds.iter().copied().fold(0.0, f64::max);
    let speedup_ratio = min_speed / max_speed;
    
    assert!(speedup_ratio > 0.80, "Speedup should remain consistent across scales (ratio: {:.1}%)", speedup_ratio * 100.0);
}
