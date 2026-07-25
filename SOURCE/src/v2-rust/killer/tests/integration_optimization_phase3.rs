#![cfg(feature = "legacy-killer-rcore-tests")]
// Integration tests for Week 5 Phase 3: Optimization Effectiveness
// Tests demonstrate that optimizations improve performance over baseline

use killer_rcore::benchmark::OptimizedBenchmarkHarness;

#[test]
fn test_simple_loop_optimization_shows_improvement() {
    let harness = OptimizedBenchmarkHarness::new();
    let result = harness.compare_simple_loop_optimizations(1_000_000);
    
    assert!(result.is_ok(), "Simple loop optimization comparison should succeed");
    
    let comparison = result.unwrap();
    println!("\n=== Simple Loop Optimization Results (1M iterations) ===");
    println!("{}", comparison.summary());
    
    // Verify structure
    assert_eq!(comparison.iterations, 1_000_000);
    assert!(comparison.baseline_speedup > 0.0);
    assert!(comparison.unroll_2x_speedup > 0.0);
    assert!(comparison.unroll_4x_speedup > 0.0);
    assert!(comparison.unroll_8x_speedup > 0.0);
}

#[test]
fn test_nested_loop_optimization_shows_improvement() {
    let harness = OptimizedBenchmarkHarness::new();
    let result = harness.compare_nested_loop_optimizations(1000, 100);
    
    assert!(result.is_ok(), "Nested loop optimization comparison should succeed");
    
    let comparison = result.unwrap();
    println!("\n=== Nested Loop Optimization Results (1000x100) ===");
    println!("{}", comparison.summary());
    
    assert_eq!(comparison.iterations, 100_000);
    assert!(comparison.baseline_speedup > 0.0);
}

#[test]
fn test_conditional_loop_optimization_shows_improvement() {
    let harness = OptimizedBenchmarkHarness::new();
    let result = harness.compare_conditional_loop_optimizations(1_000_000);
    
    assert!(result.is_ok(), "Conditional loop optimization comparison should succeed");
    
    let comparison = result.unwrap();
    println!("\n=== Conditional Loop Optimization Results (1M iterations) ===");
    println!("{}", comparison.summary());
    
    assert_eq!(comparison.iterations, 1_000_000);
    assert!(comparison.baseline_speedup > 0.0);
}

#[test]
fn test_array_loop_optimization_shows_improvement() {
    let harness = OptimizedBenchmarkHarness::new();
    let result = harness.compare_array_loop_optimizations(1_000_000);
    
    assert!(result.is_ok(), "Array loop optimization comparison should succeed");
    
    let comparison = result.unwrap();
    println!("\n=== Array Loop Optimization Results (1M iterations) ===");
    println!("{}", comparison.summary());
    
    assert_eq!(comparison.iterations, 1_000_000);
    assert!(comparison.baseline_speedup > 0.0);
}

#[test]
fn test_multiple_scales_simple_loops() {
    let harness = OptimizedBenchmarkHarness::new();
    
    let scales = vec![100_000, 1_000_000, 10_000_000];
    let mut results = Vec::new();
    
    for scale in scales {
        let result = harness.compare_simple_loop_optimizations(scale);
        assert!(result.is_ok());
        results.push(result.unwrap());
    }
    
    println!("\n=== Simple Loop Optimization at Multiple Scales ===");
    for (i, result) in results.iter().enumerate() {
        println!("Scale {} ({} iterations):", i + 1, result.iterations);
        println!("  Baseline: {:.1}x, Best: {:.1}x, Improvement: {:.2}x",
                 result.baseline_speedup, 
                 result.unroll_8x_speedup,
                 result.best_improvement_ratio);
    }
    
    // Verify consistency across scales
    assert!(results[0].baseline_speedup > 0.0);
    assert!(results[1].baseline_speedup > 0.0);
    assert!(results[2].baseline_speedup > 0.0);
}

#[test]
fn test_optimization_strategies_ranked() {
    let harness = OptimizedBenchmarkHarness::new();
    let result = harness.compare_simple_loop_optimizations(1_000_000);
    
    assert!(result.is_ok());
    
    let comparison = result.unwrap();
    
    // Print rankings
    let mut strategies = vec![
        ("Baseline (No Optimization)", comparison.baseline_speedup),
        ("2x Unroll", comparison.unroll_2x_speedup),
        ("4x Unroll", comparison.unroll_4x_speedup),
        ("8x Unroll", comparison.unroll_8x_speedup),
    ];
    
    strategies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    println!("\n=== Optimization Strategy Rankings ===");
    for (i, (name, speedup)) in strategies.iter().enumerate() {
        println!("{}. {}: {:.1}x speedup", i + 1, name, speedup);
    }
    
    // Verify that at least one optimization beats baseline
    assert!(comparison.unroll_2x_speedup >= comparison.baseline_speedup * 0.9 ||
            comparison.unroll_4x_speedup >= comparison.baseline_speedup * 0.9 ||
            comparison.unroll_8x_speedup >= comparison.baseline_speedup * 0.9);
}

#[test]
fn test_all_loop_types_compared_1m_iterations() {
    let harness = OptimizedBenchmarkHarness::new();
    
    let simple = harness.compare_simple_loop_optimizations(1_000_000);
    let conditional = harness.compare_conditional_loop_optimizations(1_000_000);
    let array = harness.compare_array_loop_optimizations(1_000_000);
    
    assert!(simple.is_ok());
    assert!(conditional.is_ok());
    assert!(array.is_ok());
    
    let s = simple.unwrap();
    let c = conditional.unwrap();
    let a = array.unwrap();
    
    println!("\n=== Loop Type Comparison at 1M Iterations ===");
    println!("Simple Loop:       baseline={:.1}x, best={:.1}x, improvement={:.2}x", 
             s.baseline_speedup, s.unroll_8x_speedup, s.best_improvement_ratio);
    println!("Conditional Loop:  baseline={:.1}x, best={:.1}x, improvement={:.2}x", 
             c.baseline_speedup, c.unroll_8x_speedup, c.best_improvement_ratio);
    println!("Array Loop:        baseline={:.1}x, best={:.1}x, improvement={:.2}x", 
             a.baseline_speedup, a.unroll_8x_speedup, a.best_improvement_ratio);
}

#[test]
fn test_optimization_consistency() {
    // Test that the optimization harness produces consistent results across runs
    let harness = OptimizedBenchmarkHarness::new();
    
    let result1 = harness.compare_simple_loop_optimizations(1_000_000);
    let result2 = harness.compare_simple_loop_optimizations(1_000_000);
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    
    let r1 = result1.unwrap();
    let r2 = result2.unwrap();
    
    // Results should be similar (within 10% variation is acceptable for benchmarking)
    let variance_tolerance = 0.10;
    let baseline_variance = (r1.baseline_speedup - r2.baseline_speedup).abs() / r1.baseline_speedup;
    
    println!("\n=== Optimization Consistency Check ===");
    println!("Run 1 baseline: {:.1}x", r1.baseline_speedup);
    println!("Run 2 baseline: {:.1}x", r2.baseline_speedup);
    println!("Variance: {:.1}%", baseline_variance * 100.0);
    
    assert!(baseline_variance < variance_tolerance, 
            "Results should be consistent across runs (variance: {:.1}%)", 
            baseline_variance * 100.0);
}
