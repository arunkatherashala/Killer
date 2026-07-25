// killer_rcore/src/benchmark/runner.rs
// Multi-benchmark orchestration and execution
// Week 4 benchmark suite coordinator

use super::harness::BenchmarkHarness;
use super::metrics::{BenchmarkMetrics, PerformanceReport};

/// Orchestrates and runs benchmark suites
pub struct BenchmarkRunner {
    harness: BenchmarkHarness,
    verbose: bool,
}

impl BenchmarkRunner {
    /// Create new benchmark runner
    pub fn new() -> Self {
        BenchmarkRunner {
            harness: BenchmarkHarness::new(),
            verbose: false,
        }
    }
    
    /// Create with caching disabled
    pub fn without_cache() -> Self {
        BenchmarkRunner {
            harness: BenchmarkHarness::without_cache(),
            verbose: false,
        }
    }
    
    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
    
    /// Run comprehensive benchmark suite
    pub fn run_all_benchmarks(&self) -> Result<PerformanceReport, String> {
        if self.verbose {
            println!("🚀 Starting comprehensive benchmark suite...");
        }
        
        let mut metrics = Vec::new();
        
        // Simple loops: Various sizes
        metrics.extend(self.run_simple_loop_benchmarks()?);
        
        // Nested loops
        metrics.extend(self.run_nested_loop_benchmarks()?);
        
        // Conditional loops
        metrics.extend(self.run_conditional_loop_benchmarks()?);
        
        // Array access loops
        metrics.extend(self.run_array_loop_benchmarks()?);
        
        // Function call loops
        metrics.extend(self.run_function_call_loop_benchmarks()?);
        
        if self.verbose {
            println!("✅ Benchmark suite complete");
        }
        
        Ok(PerformanceReport::from_benchmarks(metrics))
    }
    
    /// Run only simple loop benchmarks
    pub fn run_simple_loop_benchmarks(&self) -> Result<Vec<BenchmarkMetrics>, String> {
        if self.verbose {
            println!("\n📊 Running simple loop benchmarks...");
        }
        
        let test_sizes = vec![
            100_000,
            1_000_000,
            10_000_000,
            100_000_000,
        ];
        
        let mut results = Vec::new();
        for size in test_sizes {
            if self.verbose {
                println!("  Testing simple loop: {} iterations", size);
            }
            match self.harness.benchmark_simple_loop(size) {
                Ok(metrics) => {
                    if self.verbose {
                        println!(
                            "    ✅ Speedup: {:.2}x (interpreter: {:.3}s, JIT: {:.3}s)",
                            metrics.speedup(),
                            metrics.interpreter_time.as_secs_f64(),
                            metrics.jit_time.as_secs_f64()
                        );
                    }
                    results.push(metrics);
                }
                Err(e) => {
                    if self.verbose {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run only nested loop benchmarks
    pub fn run_nested_loop_benchmarks(&self) -> Result<Vec<BenchmarkMetrics>, String> {
        if self.verbose {
            println!("\n📊 Running nested loop benchmarks...");
        }
        
        let test_sizes = vec![
            (100, 100),        // 10K iterations
            (1000, 100),       // 100K iterations
            (1000, 1000),      // 1M iterations
        ];
        
        let mut results = Vec::new();
        for (outer, inner) in test_sizes {
            if self.verbose {
                println!("  Testing nested loop: {}x{}", outer, inner);
            }
            match self.harness.benchmark_nested_loop(outer, inner) {
                Ok(metrics) => {
                    if self.verbose {
                        println!(
                            "    ✅ Speedup: {:.2}x",
                            metrics.speedup()
                        );
                    }
                    results.push(metrics);
                }
                Err(e) => {
                    if self.verbose {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run only conditional loop benchmarks
    pub fn run_conditional_loop_benchmarks(&self) -> Result<Vec<BenchmarkMetrics>, String> {
        if self.verbose {
            println!("\n📊 Running conditional loop benchmarks...");
        }
        
        let test_sizes = vec![
            100_000,
            1_000_000,
            10_000_000,
        ];
        
        let mut results = Vec::new();
        for size in test_sizes {
            if self.verbose {
                println!("  Testing conditional loop: {} iterations", size);
            }
            match self.harness.benchmark_conditional_loop(size) {
                Ok(metrics) => {
                    if self.verbose {
                        println!(
                            "    ✅ Speedup: {:.2}x",
                            metrics.speedup()
                        );
                    }
                    results.push(metrics);
                }
                Err(e) => {
                    if self.verbose {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run only array loop benchmarks
    pub fn run_array_loop_benchmarks(&self) -> Result<Vec<BenchmarkMetrics>, String> {
        if self.verbose {
            println!("\n📊 Running array loop benchmarks...");
        }
        
        let test_sizes = vec![
            100_000,
            1_000_000,
            10_000_000,
        ];
        
        let mut results = Vec::new();
        for size in test_sizes {
            if self.verbose {
                println!("  Testing array loop: {} iterations", size);
            }
            match self.harness.benchmark_array_loop(size) {
                Ok(metrics) => {
                    if self.verbose {
                        println!(
                            "    ✅ Speedup: {:.2}x",
                            metrics.speedup()
                        );
                    }
                    results.push(metrics);
                }
                Err(e) => {
                    if self.verbose {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run only function call loop benchmarks
    pub fn run_function_call_loop_benchmarks(&self) -> Result<Vec<BenchmarkMetrics>, String> {
        if self.verbose {
            println!("\n📊 Running function call loop benchmarks...");
        }
        
        let test_sizes = vec![
            100_000,
            1_000_000,
            10_000_000,
        ];
        
        let mut results = Vec::new();
        for size in test_sizes {
            if self.verbose {
                println!("  Testing function call loop: {} iterations", size);
            }
            match self.harness.benchmark_function_call_loop(size) {
                Ok(metrics) => {
                    if self.verbose {
                        println!(
                            "    ✅ Speedup: {:.2}x",
                            metrics.speedup()
                        );
                    }
                    results.push(metrics);
                }
                Err(e) => {
                    if self.verbose {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Run quick benchmark (single representative test)
    pub fn run_quick_benchmark(&self) -> Result<BenchmarkMetrics, String> {
        if self.verbose {
            println!("⚡ Running quick benchmark (1M iterations)...");
        }
        
        self.harness.benchmark_simple_loop(1_000_000)
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_runner_creation() {
        let runner = BenchmarkRunner::new();
        assert!(!runner.verbose);
    }
    
    #[test]
    fn test_runner_with_cache() {
        let runner = BenchmarkRunner::new();
        // Harness should have cache
        let _ = runner;
    }
    
    #[test]
    fn test_runner_without_cache() {
        let runner = BenchmarkRunner::without_cache();
        assert!(!runner.verbose);
    }
    
    #[test]
    fn test_runner_verbose_setting() {
        let runner = BenchmarkRunner::new().with_verbose(true);
        assert!(runner.verbose);
    }
    
    #[test]
    fn test_runner_default() {
        let _runner = BenchmarkRunner::default();
        // Just verify it doesn't panic
    }
    
    // Note: Full integration tests that actually run benchmarks
    // are better as standalone test binaries due to compilation time
}
