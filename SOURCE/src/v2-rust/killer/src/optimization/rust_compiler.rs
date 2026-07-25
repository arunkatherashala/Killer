/// RustCompiler: Real compilation integration for measuring true optimization effects
/// 
/// This module replaces simulated optimization measurement with actual Rust compilation
/// using rustc and LLVM optimization levels. Enables production-grade performance analysis.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// LLVM optimization levels available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimization (-O0)
    O0,
    /// Basic optimization (-O1)
    O1,
    /// Standard optimization (-O2)
    O2,
    /// Aggressive optimization (-O3)
    O3,
    /// Size optimization (-Oz)
    Oz,
}

impl OptLevel {
    pub fn as_flag(&self) -> &'static str {
        match self {
            OptLevel::O0 => "0",
            OptLevel::O1 => "1",
            OptLevel::O2 => "2",
            OptLevel::O3 => "3",
            OptLevel::Oz => "z",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OptLevel::O0 => "O0",
            OptLevel::O1 => "O1",
            OptLevel::O2 => "O2",
            OptLevel::O3 => "O3",
            OptLevel::Oz => "Oz",
        }
    }
}

/// Configuration for generated Rust code
#[derive(Debug, Clone)]
pub struct GeneratedLoop {
    pub name: String,
    pub code: String,
    pub iterations: u64,
}

/// Compiled binary metadata and measurements
#[derive(Debug, Clone)]
pub struct CompiledBinary {
    pub opt_level: OptLevel,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub compile_time: Duration,
}

/// Compilation result with execution metrics
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub opt_level: OptLevel,
    pub compile_time_ms: f64,
    pub binary_size_kb: f64,
    pub avg_execution_time_ms: f64,
    pub min_execution_time_ms: f64,
    pub max_execution_time_ms: f64,
}

impl CompilationResult {
    pub fn speedup_vs(&self, baseline: &CompilationResult) -> f64 {
        baseline.avg_execution_time_ms / self.avg_execution_time_ms
    }

    pub fn size_ratio_vs(&self, baseline: &CompilationResult) -> f64 {
        self.binary_size_kb / baseline.binary_size_kb
    }
}

/// Main compiler interface for optimization measurement
pub struct RustCompiler {
    work_dir: PathBuf,
    #[allow(dead_code)]
    target_triple: String,
    cleanup_after: bool,
}

impl RustCompiler {
    /// Create a new RustCompiler with temp directory for artifacts
    pub fn new(cleanup: bool) -> Result<Self, String> {
        // Use system temp directory for compilation artifacts
        let work_dir = std::env::temp_dir().join("killer_rust_compiler");
        
        // Create work directory
        fs::create_dir_all(&work_dir)
            .map_err(|e| format!("Failed to create work directory: {}", e))?;

        Ok(RustCompiler {
            work_dir,
            target_triple: Self::detect_target_triple()?,
            cleanup_after: cleanup,
        })
    }

    /// Detect the current compilation target triple
    fn detect_target_triple() -> Result<String, String> {
        let output = Command::new("rustc")
            .args(&["--version", "--verbose"])
            .output()
            .map_err(|e| format!("Failed to run rustc: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("host: ") {
                return Ok(line.strip_prefix("host: ").unwrap().to_string());
            }
        }
        
        Err("Could not detect target triple".to_string())
    }

    /// Compile a code snippet with specified optimization level
    pub fn compile(
        &self,
        loop_def: &GeneratedLoop,
        opt_level: OptLevel,
    ) -> Result<CompiledBinary, String> {
        let output_name = format!(
            "{}_{}_{}",
            loop_def.name,
            opt_level.as_str(),
            std::process::id()
        );
        
        let rs_file = self.work_dir.join(format!("{}.rs", output_name));
        let bin_file = self.work_dir.join(&output_name);

        // Write Rust source file
        self.write_loop_wrapper(&rs_file, loop_def)?;

        // Compile with rustc
        let compile_start = Instant::now();
        self.invoke_rustc(&rs_file, &bin_file, opt_level)?;
        let compile_time = compile_start.elapsed();

        // Get binary size
        let size_bytes = fs::metadata(&bin_file)
            .map_err(|e| format!("Failed to get binary size: {}", e))?
            .len();

        Ok(CompiledBinary {
            opt_level,
            path: bin_file,
            size_bytes,
            compile_time,
        })
    }

    /// Write a complete Rust program wrapping the loop code
    fn write_loop_wrapper(&self, path: &Path, loop_def: &GeneratedLoop) -> Result<(), String> {
        let wrapper = format!(
            r#"use std::time::Instant;

fn main() {{
    let iterations = {iterations}u64;
    
    // Warm up: single run
    let _ = {name}_loop(iterations);
    
    // Measure: 5 runs, report stats
    let mut times = vec![];
    for _ in 0..5 {{
        let start = Instant::now();
        let _ = {name}_loop(iterations);
        times.push(start.elapsed());
    }}
    
    // Calculate statistics
    let times_ms: Vec<f64> = times.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    let avg = times_ms.iter().sum::<f64>() / times_ms.len() as f64;
    let min = times_ms.iter().fold(f64::MAX, |a: f64, &b| a.min(b));
    let max = times_ms.iter().fold(0.0f64, |a: f64, &b| a.max(b));
    
    println!("avg_ms:{{}},min_ms:{{}},max_ms:{{}}", avg, min, max);
}}

#[inline(never)]
fn {name}_loop(iterations: u64) -> u64 {{
    {code}
}}
"#,
            name = loop_def.name,
            iterations = loop_def.iterations,
            code = loop_def.code
        );

        fs::write(path, wrapper)
            .map_err(|e| format!("Failed to write loop wrapper: {}", e))
    }

    /// Invoke rustc compiler with specified optimization level
    fn invoke_rustc(
        &self,
        rs_file: &Path,
        bin_file: &Path,
        opt_level: OptLevel,
    ) -> Result<(), String> {
        let output = Command::new("rustc")
            .args(&[
                rs_file.to_string_lossy().as_ref(),
                "-o",
                bin_file.to_string_lossy().as_ref(),
                "--edition",
                "2021",
                "-C",
                &format!("opt-level={}", opt_level.as_flag()),
                "-C",
                "target-cpu=native",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to invoke rustc: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("rustc compilation failed: {}", stderr));
        }

        Ok(())
    }

    /// Execute compiled binary and extract timing results
    fn run_binary(&self, binary: &CompiledBinary) -> Result<CompilationResult, String> {
        let output = Command::new(&binary.path)
            .output()
            .map_err(|e| format!("Failed to execute binary: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse output: "avg_ms:X,min_ms:Y,max_ms:Z"
        let (avg_ms, min_ms, max_ms) = Self::parse_timing_output(&stdout)?;

        Ok(CompilationResult {
            opt_level: binary.opt_level,
            compile_time_ms: binary.compile_time.as_secs_f64() * 1000.0,
            binary_size_kb: binary.size_bytes as f64 / 1024.0,
            avg_execution_time_ms: avg_ms,
            min_execution_time_ms: min_ms,
            max_execution_time_ms: max_ms,
        })
    }

    /// Parse timing output from compiled binary
    fn parse_timing_output(output: &str) -> Result<(f64, f64, f64), String> {
        // Expected format: "avg_ms:X,min_ms:Y,max_ms:Z"
        let trimmed = output.trim();
        
        let avg = Self::extract_value(trimmed, "avg_ms:")?;
        let min = Self::extract_value(trimmed, "min_ms:")?;
        let max = Self::extract_value(trimmed, "max_ms:")?;

        Ok((avg, min, max))
    }

    /// Extract a numeric value from output
    fn extract_value(text: &str, prefix: &str) -> Result<f64, String> {
        let start = text
            .find(prefix)
            .ok_or_else(|| format!("Could not find {} in output", prefix))?
            + prefix.len();

        let end = text[start..]
            .find(',')
            .or_else(|| text[start..].find(|c: char| !c.is_numeric() && c != '.'))
            .unwrap_or(text[start..].len());

        let value_str = &text[start..start + end];
        value_str
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse numeric value '{}': {}", value_str, e))
    }

    /// Compile and measure with specified optimization level
    pub fn compile_and_measure(
        &self,
        loop_def: &GeneratedLoop,
        opt_level: OptLevel,
    ) -> Result<CompilationResult, String> {
        let binary = self.compile(loop_def, opt_level)?;
        let result = self.run_binary(&binary)?;
        
        // Clean up binary if requested
        if self.cleanup_after {
            let _ = fs::remove_file(&binary.path);
        }

        Ok(result)
    }

    /// Compile with all standard optimization levels and return comparative results
    pub fn compile_all_levels(
        &self,
        loop_def: &GeneratedLoop,
    ) -> Result<Vec<CompilationResult>, String> {
        let levels = [OptLevel::O0, OptLevel::O1, OptLevel::O2, OptLevel::O3, OptLevel::Oz];
        let mut results = vec![];

        for level in &levels {
            let result = self.compile_and_measure(loop_def, *level)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Clean up all temporary compilation artifacts
    pub fn cleanup_all(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.work_dir)
            .map_err(|e| format!("Failed to cleanup work directory: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_level_flags() {
        assert_eq!(OptLevel::O0.as_flag(), "0");
        assert_eq!(OptLevel::O3.as_flag(), "3");
        assert_eq!(OptLevel::Oz.as_flag(), "z");
    }

    #[test]
    fn test_opt_level_strings() {
        assert_eq!(OptLevel::O0.as_str(), "O0");
        assert_eq!(OptLevel::O3.as_str(), "O3");
    }

    #[test]
    fn test_parse_timing_output() {
        let output = "avg_ms:5.23,min_ms:5.10,max_ms:5.45";
        let (avg, min, max) = RustCompiler::parse_timing_output(output).unwrap();
        
        assert!((avg - 5.23).abs() < 0.01);
        assert!((min - 5.10).abs() < 0.01);
        assert!((max - 5.45).abs() < 0.01);
    }

    #[test]
    fn test_extract_value() {
        let text = "avg_ms:10.5,min_ms:9.8,max_ms:11.2";
        let avg = RustCompiler::extract_value(text, "avg_ms:").unwrap();
        assert!((avg - 10.5).abs() < 0.01);
    }

    #[test]
    fn test_compilation_result_speedup() {
        let baseline = CompilationResult {
            opt_level: OptLevel::O0,
            compile_time_ms: 100.0,
            binary_size_kb: 500.0,
            avg_execution_time_ms: 10.0,
            min_execution_time_ms: 9.5,
            max_execution_time_ms: 10.5,
        };

        let optimized = CompilationResult {
            opt_level: OptLevel::O3,
            compile_time_ms: 150.0,
            binary_size_kb: 520.0,
            avg_execution_time_ms: 7.0,
            min_execution_time_ms: 6.5,
            max_execution_time_ms: 7.5,
        };

        let speedup = optimized.speedup_vs(&baseline);
        assert!((speedup - (10.0 / 7.0)).abs() < 0.01);
    }

    #[test]
    fn test_compilation_result_size_ratio() {
        let baseline = CompilationResult {
            opt_level: OptLevel::O0,
            compile_time_ms: 100.0,
            binary_size_kb: 500.0,
            avg_execution_time_ms: 10.0,
            min_execution_time_ms: 9.5,
            max_execution_time_ms: 10.5,
        };

        let optimized = CompilationResult {
            opt_level: OptLevel::O3,
            compile_time_ms: 150.0,
            binary_size_kb: 600.0,
            avg_execution_time_ms: 7.0,
            min_execution_time_ms: 6.5,
            max_execution_time_ms: 7.5,
        };

        let ratio = optimized.size_ratio_vs(&baseline);
        assert!((ratio - 1.2).abs() < 0.01);
    }

    #[test]
    fn test_detect_target_triple() {
        let triple = RustCompiler::detect_target_triple();
        assert!(triple.is_ok());
        let triple_str = triple.unwrap();
        assert!(!triple_str.is_empty());
        // Should contain platform identifier like x86_64, aarch64, etc
        assert!(triple_str.len() > 5);
    }

    #[test]
    fn test_compiler_creation() {
        let compiler = RustCompiler::new(false);
        assert!(compiler.is_ok());
    }
}
