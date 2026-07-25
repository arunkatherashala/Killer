// Performance Profiling and Optimization Module
// Measures compiler performance and identifies optimization opportunities

use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompilerStage {
    pub name: String,
    pub duration_ms: f64,
    pub input_size_bytes: u64,
    pub output_size_bytes: u64,
    pub allocation_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CompilationProfile {
    pub total_duration: Duration,
    pub stages: Vec<CompilerStage>,
    pub peak_memory_mb: u64,
    pub optimization_speedup: f32,
    pub bottleneck_stage: String,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub stage_timings: HashMap<String, f64>,
    pub memory_usage: HashMap<String, u64>,
    pub allocation_count: HashMap<String, u32>,
    pub instruction_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

pub struct StageProfiler {
    name: String,
    start: Instant,
    input_size: u64,
}

impl StageProfiler {
    pub fn start(name: &str, input_size: u64) -> Self {
        StageProfiler {
            name: name.to_string(),
            start: Instant::now(),
            input_size,
        }
    }

    pub fn finish(self, output_size: u64) -> CompilerStage {
        let duration_ms = self.start.elapsed().as_secs_f64() * 1000.0;

        CompilerStage {
            name: self.name,
            duration_ms,
            input_size_bytes: self.input_size,
            output_size_bytes: output_size,
            allocation_bytes: output_size.saturating_sub(self.input_size),
        }
    }
}

impl CompilationProfile {
    pub fn new() -> Self {
        CompilationProfile {
            total_duration: Duration::ZERO,
            stages: Vec::new(),
            peak_memory_mb: 0,
            optimization_speedup: 1.0,
            bottleneck_stage: String::new(),
        }
    }

    pub fn add_stage(&mut self, stage: CompilerStage) {
        self.stages.push(stage);
    }

    pub fn finalize(&mut self) {
        // Calculate total duration
        let total_ms: f64 = self.stages.iter().map(|s| s.duration_ms).sum();
        self.total_duration = Duration::from_secs_f64(total_ms / 1000.0);

        // Find bottleneck
        if let Some(stage) = self
            .stages
            .iter()
            .max_by(|a, b| a.duration_ms.partial_cmp(&b.duration_ms).unwrap())
        {
            self.bottleneck_stage = stage.name.clone();
        }

        // Estimate optimization speedup
        self.optimization_speedup = self.estimate_speedup();
    }

    fn estimate_speedup(&self) -> f32 {
        let avg_throughput = self
            .stages
            .iter()
            .map(|s| {
                if s.duration_ms > 0.0 {
                    (s.output_size_bytes as f64) / (s.duration_ms / 1000.0)
                } else {
                    1.0
                }
            })
            .sum::<f64>() / (self.stages.len() as f64);

        (avg_throughput / 100_000.0).min(100.0).max(1.0) as f32
    }

    pub fn print_report(&self) {
        println!("+================================================================+");
        println!("|              COMPILATION PERFORMANCE PROFILE                   |");
        println!("+================================================================+");
        println!();
        println!("Total Compilation Time: {:.2}ms", self.total_duration.as_secs_f64() * 1000.0);
        println!("Peak Memory Usage: {}MB", self.peak_memory_mb);
        println!("Optimization Speedup: {:.2}x", self.optimization_speedup);
        println!("Bottleneck Stage: {}", self.bottleneck_stage);
        println!();
        println!("Stage Breakdown:");
        println!("+-------------------------+----------+------------+--------------+");
        println!("| Stage                   | Time(ms) | Input(B)   | Output(B)    |");
        println!("+-------------------------+----------+------------+--------------+");

        for stage in &self.stages {
            println!(
                "| {:<23} | {:>8.2} | {:>10} | {:>12} |",
                stage.name,
                stage.duration_ms,
                stage.input_size_bytes,
                stage.output_size_bytes
            );
        }

        println!("+-------------------------+----------+------------+--------------+");
    }
}

/// Cache efficiency analyzer
pub struct CacheAnalyzer {
    line_size: u32,
    cache_lines: HashMap<u64, u32>,
    hits: u64,
    misses: u64,
}

impl CacheAnalyzer {
    pub fn new(cache_line_size: u32) -> Self {
        CacheAnalyzer {
            line_size: cache_line_size,
            cache_lines: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn access(&mut self, addr: u64) {
        let cache_line = addr / (self.line_size as u64);

        match self.cache_lines.get(&cache_line) {
            Some(count) => {
                self.cache_lines.insert(cache_line, count + 1);
                self.hits += 1;
            }
            None => {
                self.cache_lines.insert(cache_line, 1);
                self.misses += 1;
            }
        }
    }

    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f32) / (total as f32)
        }
    }

    pub fn report(&self) {
        println!("Cache Analysis:");
        println!("  Hits: {}", self.hits);
        println!("  Misses: {}", self.misses);
        println!("  Hit Rate: {:.2}%", self.hit_rate() * 100.0);
        println!("  Unique Lines: {}", self.cache_lines.len());
    }
}

/// Branch prediction analyzer
pub struct BranchAnalyzer {
    branches_taken: u64,
    branches_not_taken: u64,
    mispredictions: u64,
}

impl BranchAnalyzer {
    pub fn new() -> Self {
        BranchAnalyzer {
            branches_taken: 0,
            branches_not_taken: 0,
            mispredictions: 0,
        }
    }

    pub fn record_branch(&mut self, taken: bool) {
        if taken {
            self.branches_taken += 1;
        } else {
            self.branches_not_taken += 1;
        }
    }

    pub fn prediction_accuracy(&self) -> f32 {
        let total = self.branches_taken + self.branches_not_taken;
        if total == 0 {
            1.0
        } else {
            let correct = total - self.mispredictions;
            (correct as f32) / (total as f32)
        }
    }

    pub fn report(&self) {
        println!("Branch Analysis:");
        println!("  Taken: {}", self.branches_taken);
        println!("  Not Taken: {}", self.branches_not_taken);
        println!("  Mispredictions: {}", self.mispredictions);
        println!("  Accuracy: {:.2}%", self.prediction_accuracy() * 100.0);
    }
}

/// Memory access pattern analyzer
pub struct MemoryAccessAnalyzer {
    sequential_accesses: u64,
    random_accesses: u64,
    last_access: Option<u64>,
}

impl MemoryAccessAnalyzer {
    pub fn new() -> Self {
        MemoryAccessAnalyzer {
            sequential_accesses: 0,
            random_accesses: 0,
            last_access: None,
        }
    }

    pub fn record_access(&mut self, addr: u64) {
        if let Some(last) = self.last_access {
            if addr > last && addr <= last + 64 {
                // Sequential access
                self.sequential_accesses += 1;
            } else {
                // Random access
                self.random_accesses += 1;
            }
        }
        self.last_access = Some(addr);
    }

    pub fn locality_score(&self) -> f32 {
        let total = self.sequential_accesses + self.random_accesses;
        if total == 0 {
            1.0
        } else {
            (self.sequential_accesses as f32) / (total as f32)
        }
    }

    pub fn report(&self) {
        println!("Memory Access Pattern:");
        println!("  Sequential: {}", self.sequential_accesses);
        println!("  Random: {}", self.random_accesses);
        println!("  Locality Score: {:.2}", self.locality_score());
    }
}

/// Benchmark result with performance metrics
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub std_dev: f64,
    pub throughput_ops_per_sec: f64,
}

impl BenchmarkResult {
    pub fn print_report(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Total Time: {:.2}ms", self.total_time_ms);
        println!("  Average: {:.4}ms", self.avg_time_ms);
        println!("  Min: {:.4}ms", self.min_time_ms);
        println!("  Max: {:.4}ms", self.max_time_ms);
        println!("  Std Dev: {:.4}ms", self.std_dev);
        println!("  Throughput: {:.0} ops/sec", self.throughput_ops_per_sec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_profiler() {
        let profiler = StageProfiler::start("test", 1000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let stage = profiler.finish(2000);

        assert_eq!(stage.name, "test");
        assert_eq!(stage.input_size_bytes, 1000);
        assert_eq!(stage.output_size_bytes, 2000);
        assert!(stage.duration_ms >= 10.0);
    }

    #[test]
    fn test_compilation_profile() {
        let mut profile = CompilationProfile::new();
        
        let stage1 = CompilerStage {
            name: "lexer".to_string(),
            duration_ms: 5.0,
            input_size_bytes: 1000,
            output_size_bytes: 1500,
            allocation_bytes: 500,
        };

        profile.add_stage(stage1);
        profile.finalize();

        assert_eq!(profile.stages.len(), 1);
        assert_eq!(profile.bottleneck_stage, "lexer");
    }

    #[test]
    fn test_cache_analyzer() {
        let mut analyzer = CacheAnalyzer::new(64);
        analyzer.access(0);       // Cache line 0, miss
        analyzer.access(32);      // Cache line 0, hit (same line)
        analyzer.access(64);      // Cache line 1, miss
        analyzer.access(96);      // Cache line 1, hit (same line)

        assert_eq!(analyzer.hits, 2);
        assert_eq!(analyzer.misses, 2);
        assert_eq!(analyzer.hit_rate(), 0.5);
    }

    #[test]
    fn test_memory_access_analyzer() {
        let mut analyzer = MemoryAccessAnalyzer::new();
        analyzer.record_access(0);
        analyzer.record_access(16); // Sequential
        analyzer.record_access(32); // Sequential
        analyzer.record_access(1000); // Random jump

        assert!(analyzer.locality_score() > 0.4);
    }
}
