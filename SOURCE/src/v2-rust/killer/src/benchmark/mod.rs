// killer_rcore/src/benchmark/mod.rs
// Benchmarking infrastructure for JIT performance validation
// Week 4: Performance measurement and validation

pub mod harness;
pub mod metrics;
pub mod runner;
pub mod optimization_harness;

pub use harness::BenchmarkHarness;
pub use metrics::{BenchmarkMetrics, PerformanceReport, LoopType};
pub use runner::BenchmarkRunner;
pub use optimization_harness::{OptimizedBenchmarkHarness, OptimizationComparisonResult};
