// killer_super/compiler_api.rs - Main compiler API
// High-level interface for using Killer Super

use crate::killer_super::config::KillerSuperConfig;
use crate::killer_super::pipeline::{CompilationPipeline};
use std::time::Instant;

/// Main Killer Super compiler struct
pub struct KillerSuper {
    config: KillerSuperConfig,
    pipeline: CompilationPipeline,
}

/// Compilation result with metrics
#[derive(Debug)]
pub struct CompilationResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    pub output_file: Option<String>,
    pub stats: CompilerStats,
}

/// Compiler statistics and metrics
#[derive(Debug)]
pub struct CompilerStats {
    pub compile_time_ms: u64,
    pub optimization_speedup: f64,
    pub input_size_bytes: usize,
    pub output_size_bytes: usize,
    pub phases_used: u32,
    pub strategies_used: u32,
}

impl KillerSuper {
    /// Create new Killer Super compiler with default (development) config
    pub fn new() -> Self {
        Self::with_config(KillerSuperConfig::development())
    }

    /// Create with custom configuration
    pub fn with_config(config: KillerSuperConfig) -> Self {
        let pipeline = CompilationPipeline::new(config.clone());
        Self { config, pipeline }
    }

    /// Get compiler configuration
    pub fn config(&self) -> &KillerSuperConfig {
        &self.config
    }

    /// Get compilation pipeline
    pub fn pipeline(&self) -> &CompilationPipeline {
        &self.pipeline
    }

    /// Compile source code (simulated, returns success but doesn't write files)
    /// Actual output file writing is handled by the caller
    pub fn compile(&self, source: &str, output_file: &str) -> CompilationResult {
        let start = Instant::now();

        // Validate input
        if source.is_empty() {
            return CompilationResult {
                success: false,
                error_message: Some("Empty source code".to_string()),
                warnings: vec![],
                output_file: None,
                stats: CompilerStats {
                    compile_time_ms: 0,
                    optimization_speedup: 1.0,
                    input_size_bytes: 0,
                    output_size_bytes: 0,
                    phases_used: 0,
                    strategies_used: 0,
                },
            };
        }

        // Execute pipeline
        let pipeline_result = self.pipeline.execute_simulation();

        let elapsed = start.elapsed();
        let compile_time_ms = elapsed.as_millis() as u64;

        let mut phases_used = 0;
        for stage in self.pipeline.enabled_stages() {
            if stage.phase <= 5 {
                phases_used += 1;
            }
        }

        CompilationResult {
            success: pipeline_result.success,
            error_message: None,
            warnings: vec![],
            output_file: Some(output_file.to_string()),
            stats: CompilerStats {
                compile_time_ms,
                optimization_speedup: pipeline_result.final_speedup,
                input_size_bytes: source.len(),
                output_size_bytes: (source.len() as f64 * 0.8) as usize, // Estimate
                phases_used,
                strategies_used: self.config.enabled_strategies.len() as u32,
            },
        }
    }

    /// Get version information
    pub fn version(&self) -> String {
        crate::killer_super::version_info()
    }

    /// Get list of all features
    pub fn features(&self) -> Vec<&'static str> {
        crate::killer_super::compiler_features()
    }

    /// Get compiler info
    pub fn info(&self) -> CompilerInfo {
        CompilerInfo {
            version: crate::killer_super::KILLER_SUPER_VERSION.to_string(),
            mode: format!("{}", self.config.mode),
            optimization_level: format!("{}", self.config.optimization_level),
            target_arch: format!("{}", self.config.target_arch),
            active_phases: self.pipeline.enabled_stages().len() as u32,
            expected_speedup: self.pipeline.total_speedup(),
            features: self.features().iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Default for KillerSuper {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiler information for display
#[derive(Debug)]
pub struct CompilerInfo {
    pub version: String,
    pub mode: String,
    pub optimization_level: String,
    pub target_arch: String,
    pub active_phases: u32,
    pub expected_speedup: f64,
    pub features: Vec<String>,
}

impl std::fmt::Display for CompilationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Compilation Result:\n\
             Status: {}\n\
             Time: {}ms\n\
             Speedup: {:.1}x\n",
            if self.success { "✓ Success" } else { "✗ Failed" },
            self.stats.compile_time_ms,
            self.stats.optimization_speedup,
        )
    }
}

impl std::fmt::Display for CompilerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Killer Super {}\n\
             Mode: {}\n\
             Optimization: {}\n\
             Target: {}\n\
             Active Phases: {}\n\
             Expected Speedup: {:.1}x\n",
            self.version, self.mode, self.optimization_level, self.target_arch, self.active_phases, self.expected_speedup
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_killer_super_new() {
        let compiler = KillerSuper::new();
        assert_eq!(compiler.config().mode, crate::killer_super::CompilerMode::Development);
    }

    #[test]
    fn test_killer_super_with_production_config() {
        let cfg = KillerSuperConfig::production();
        let compiler = KillerSuper::with_config(cfg);
        assert_eq!(compiler.config().mode, crate::killer_super::CompilerMode::Production);
    }

    #[test]
    fn test_compile_empty_source() {
        let compiler = KillerSuper::new();
        let result = compiler.compile("", "output.bin");
        assert!(!result.success);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_compile_valid_source() {
        let compiler = KillerSuper::new();
        let result = compiler.compile("fn main() { print(42); }", "output.bin");
        assert!(result.success);
        assert!(result.stats.phases_used >= 5); // Phases 1-5 plus custom
    }

    #[test]
    fn test_compiler_info() {
        let compiler = KillerSuper::new();
        let info = compiler.info();
        assert!(info.version.contains("4.0"));
        assert!(info.active_phases > 0);
        assert!(info.expected_speedup > 1.0);
    }

    #[test]
    fn test_compilation_result_display() {
        let compiler = KillerSuper::new();
        let result = compiler.compile("fn main() { }", "test.bin");
        let display = format!("{}", result);
        assert!(display.contains("Compilation Result"));
        assert!(display.contains("Speedup"));
    }

    #[test]
    fn test_compiler_info_display() {
        let compiler = KillerSuper::new();
        let info = compiler.info();
        let display = format!("{}", info);
        assert!(display.contains("Killer Super"));
        assert!(display.contains("Mode"));
    }
}
