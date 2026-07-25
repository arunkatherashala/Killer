// Killer Language Profiler - Performance analysis tool
// Identifies bottlenecks and provides optimization recommendations

use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::fmt;

#[derive(Clone, Debug)]
pub struct FunctionProfile {
    pub name: String,
    pub total_time_ms: f64,
    pub call_count: usize,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub memory_used_kb: f64,
}

#[derive(Clone, Debug)]
pub struct BlockProfile {
    pub location: String,
    pub duration_ms: f64,
    pub allocations: usize,
    pub memory_freed_kb: f64,
}

pub struct Profiler {
    function_profiles: HashMap<String, Vec<Duration>>,
    block_profiles: Vec<BlockProfile>,
    memory_snapshots: Vec<(Instant, u64)>,
    enabled: bool,
}

impl Profiler {
    pub fn new() -> Self {
        Profiler {
            function_profiles: HashMap::new(),
            block_profiles: Vec::new(),
            memory_snapshots: Vec::new(),
            enabled: true,
        }
    }

    /// Enable or disable profiling
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Record function call timing
    pub fn record_function(&mut self, name: &str, duration: Duration) {
        if !self.enabled {
            return;
        }
        
        self.function_profiles
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Record code block execution
    pub fn record_block(&mut self, location: &str, duration: Duration, allocations: usize) {
        if !self.enabled {
            return;
        }
        
        self.block_profiles.push(BlockProfile {
            location: location.to_string(),
            duration_ms: duration.as_secs_f64() * 1000.0,
            allocations,
            memory_freed_kb: 0.0,
        });
    }

    /// Record memory usage at point in time
    pub fn snapshot_memory(&mut self, bytes: u64) {
        if !self.enabled {
            return;
        }
        
        self.memory_snapshots.push((Instant::now(), bytes));
    }

    /// Get profile for a specific function
    pub fn get_function_profile(&self, name: &str) -> Option<FunctionProfile> {
        self.function_profiles.get(name).map(|durations| {
            let total_time = durations.iter().sum::<Duration>().as_secs_f64() * 1000.0;
            let avg_time = total_time / durations.len() as f64;
            let min_time = durations.iter().map(|d| d.as_secs_f64() * 1000.0).fold(f64::MAX, f64::min);
            let max_time = durations.iter().map(|d| d.as_secs_f64() * 1000.0).fold(0.0, f64::max);

            FunctionProfile {
                name: name.to_string(),
                total_time_ms: total_time,
                call_count: durations.len(),
                avg_time_ms: avg_time,
                min_time_ms: min_time,
                max_time_ms: max_time,
                memory_used_kb: 0.0, // Would be populated from actual memory tracking
            }
        })
    }

    /// Get all function profiles sorted by total time
    pub fn get_all_profiles(&self) -> Vec<FunctionProfile> {
        let mut profiles: Vec<_> = self.function_profiles
            .iter()
            .filter_map(|(name, durations)| {
                if durations.is_empty() {
                    return None;
                }
                
                let total_time = durations.iter().sum::<Duration>().as_secs_f64() * 1000.0;
                let avg_time = total_time / durations.len() as f64;
                let min_time = durations.iter()
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .fold(f64::MAX, f64::min);
                let max_time = durations.iter()
                    .map(|d| d.as_secs_f64() * 1000.0)
                    .fold(0.0, f64::max);

                Some(FunctionProfile {
                    name: name.clone(),
                    total_time_ms: total_time,
                    call_count: durations.len(),
                    avg_time_ms: avg_time,
                    min_time_ms: min_time,
                    max_time_ms: max_time,
                    memory_used_kb: 0.0,
                })
            })
            .collect();

        profiles.sort_by(|a, b| b.total_time_ms.partial_cmp(&a.total_time_ms).unwrap());
        profiles
    }

    /// Get top N hotspots
    pub fn get_hotspots(&self, n: usize) -> Vec<FunctionProfile> {
        self.get_all_profiles().into_iter().take(n).collect()
    }

    /// Analyze memory usage
    pub fn analyze_memory(&self) -> MemoryAnalysis {
        if self.memory_snapshots.is_empty() {
            return MemoryAnalysis::default();
        }

        let start_memory = self.memory_snapshots[0].1;
        let end_memory = self.memory_snapshots[self.memory_snapshots.len() - 1].1;
        let peak_memory = self.memory_snapshots.iter().map(|(_, m)| m).max().copied().unwrap_or(0);

        let growth = end_memory as i64 - start_memory as i64;
        let growth_percent = if start_memory > 0 {
            (growth as f64 / start_memory as f64) * 100.0
        } else {
            0.0
        };

        MemoryAnalysis {
            start_memory_kb: start_memory / 1024,
            end_memory_kb: end_memory / 1024,
            peak_memory_kb: peak_memory / 1024,
            growth_kb: growth / 1024,
            growth_percent,
        }
    }

    /// Get performance report
    pub fn generate_report(&self) -> Report {
        let profiles = self.get_all_profiles();
        let memory = self.analyze_memory();
        let bottlenecks = self.identify_bottlenecks();

        Report {
            profiles,
            memory,
            bottlenecks,
            block_profiles: self.block_profiles.clone(),
        }
    }

    /// Identify performance bottlenecks
    fn identify_bottlenecks(&self) -> Vec<Bottleneck> {
        let profiles = self.get_all_profiles();
        let total_time: f64 = profiles.iter().map(|p| p.total_time_ms).sum();

        profiles
            .iter()
            .filter_map(|profile| {
                let percent = if total_time > 0.0 {
                    (profile.total_time_ms / total_time) * 100.0
                } else {
                    0.0
                };

                // Consider > 10% of total time as a bottleneck
                if percent > 10.0 {
                    Some(Bottleneck {
                        name: profile.name.clone(),
                        time_percent: percent,
                        recommendation: self.recommend_optimization(&profile),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get optimization recommendation for a function
    fn recommend_optimization(&self, profile: &FunctionProfile) -> String {
        if profile.call_count > 10000 {
            format!("High call frequency ({} calls). Consider caching or memoization.", profile.call_count)
        } else if profile.max_time_ms > profile.avg_time_ms * 5.0 {
            "High variance in execution time. Look for early exits or data-dependent paths.".to_string()
        } else if profile.total_time_ms > 1000.0 {
            "High total execution time. Consider algorithmic optimization or parallelization.".to_string()
        } else {
            "Profile shows expected behavior.".to_string()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryAnalysis {
    pub start_memory_kb: u64,
    pub end_memory_kb: u64,
    pub peak_memory_kb: u64,
    pub growth_kb: i64,
    pub growth_percent: f64,
}

#[derive(Clone, Debug)]
pub struct Bottleneck {
    pub name: String,
    pub time_percent: f64,
    pub recommendation: String,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub profiles: Vec<FunctionProfile>,
    pub memory: MemoryAnalysis,
    pub bottlenecks: Vec<Bottleneck>,
    pub block_profiles: Vec<BlockProfile>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "+========================================================+")?;
        writeln!(f, "|         Killer Language Performance Profile            |")?;
        writeln!(f, "+========================================================+")?;

        // Function profiles
        writeln!(f, "\n[Function Profiles] (Top 10)")?;
        writeln!(f, "{:-<60}", "")?;
        writeln!(f, "{:<25} {:>10} {:>10} {:>10}", "Function", "Total (ms)", "Calls", "Avg (ms)")?;
        writeln!(f, "{:-<60}", "")?;

        for profile in self.profiles.iter().take(10) {
            writeln!(f, "{:<25} {:>10.2} {:>10} {:>10.4}",
                profile.name,
                profile.total_time_ms,
                profile.call_count,
                profile.avg_time_ms
            )?;
        }

        // Memory analysis
        writeln!(f, "\n[Memory Analysis]")?;
        writeln!(f, "{:-<60}", "")?;
        writeln!(f, "  Start Memory:     {:>8} KB", self.memory.start_memory_kb)?;
        writeln!(f, "  End Memory:       {:>8} KB", self.memory.end_memory_kb)?;
        writeln!(f, "  Peak Memory:      {:>8} KB", self.memory.peak_memory_kb)?;
        writeln!(f, "  Growth:           {:>8} KB ({:.1}%)", 
            self.memory.growth_kb, self.memory.growth_percent)?;

        // Bottlenecks
        if !self.bottlenecks.is_empty() {
            writeln!(f, "\n[Performance Bottlenecks]")?;
            writeln!(f, "{:-<60}", "")?;
            for bottleneck in &self.bottlenecks {
                writeln!(f, "  {} ({:.1}% of time)",
                    bottleneck.name, bottleneck.time_percent)?;
                writeln!(f, "    → {}", bottleneck.recommendation)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new();
        assert!(profiler.enabled);
    }

    #[test]
    fn test_record_function() {
        let mut profiler = Profiler::new();
        profiler.record_function("test", Duration::from_millis(100));
        
        let profile = profiler.get_function_profile("test");
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().call_count, 1);
    }

    #[test]
    fn test_get_all_profiles() {
        let mut profiler = Profiler::new();
        profiler.record_function("func1", Duration::from_millis(50));
        profiler.record_function("func2", Duration::from_millis(100));
        profiler.record_function("func1", Duration::from_millis(30));
        
        let profiles = profiler.get_all_profiles();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "func2"); // Longest total time first
    }

    #[test]
    fn test_memory_analysis() {
        let mut profiler = Profiler::new();
        profiler.snapshot_memory(1024 * 100);  // 100 KB
        std::thread::sleep(Duration::from_millis(10));
        profiler.snapshot_memory(1024 * 150);  // 150 KB
        
        let analysis = profiler.analyze_memory();
        assert_eq!(analysis.start_memory_kb, 100);
        assert_eq!(analysis.end_memory_kb, 150);
        assert_eq!(analysis.peak_memory_kb, 150);
    }

    #[test]
    fn test_bottleneck_identification() {
        let mut profiler = Profiler::new();
        // Create a function that takes 60% of time
        for _ in 0..100 {
            profiler.record_function("heavy", Duration::from_millis(60));
        }
        for _ in 0..100 {
            profiler.record_function("light", Duration::from_millis(40));
        }
        
        let bottlenecks = profiler.identify_bottlenecks();
        assert!(!bottlenecks.is_empty());
        assert_eq!(bottlenecks[0].name, "heavy");
    }

    #[test]
    fn test_report_generation() {
        let mut profiler = Profiler::new();
        profiler.record_function("test", Duration::from_millis(100));
        profiler.snapshot_memory(1024 * 100);
        
        let report = profiler.generate_report();
        assert!(!report.profiles.is_empty());
        println!("{}", report);
    }
}
