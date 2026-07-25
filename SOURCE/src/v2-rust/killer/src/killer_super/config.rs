// killer_super/config.rs - Configuration system
// Unified configuration for all 6 optimization phases

use std::fmt;

/// Optimization level (maps to LLVM levels)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// -O0: Minimal optimization, fast compile time
    O0,
    /// -O1: Basic optimization, good balance
    O1,
    /// -O2: Aggressive optimization
    O2,
    /// -O3: Maximum optimization, may increase compile time
    O3,
}

/// Target architecture for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X8664,
    Aarch64,
    Wasm32,
    Riscv64,
}

/// Compiler optimization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerMode {
    /// Development mode: balanced, verbose errors
    Development,
    /// Production mode: maximum optimization, minimal errors
    Production,
    /// Debug mode: no optimization, ultra-verbose errors
    Debug,
}

/// Main configuration for Killer Super compiler
#[derive(Debug, Clone)]
pub struct KillerSuperConfig {
    /// Compiler mode (dev/prod/debug)
    pub mode: CompilerMode,
    
    /// Optimization level (-O0 to -O3)
    pub optimization_level: OptimizationLevel,
    
    /// Phase 1: Enable instruction optimizations
    pub enable_phase1_instruction_opt: bool,
    
    /// Phase 2: Enable JIT compilation
    pub enable_phase2_jit: bool,
    
    /// Phase 3: Enable type specialization
    pub enable_phase3_type_specialization: bool,
    
    /// Phase 4: Enable LLVM backend
    pub enable_phase4_llvm_backend: bool,
    
    /// Phase 5: Enable ecosystem components
    pub enable_phase5_ecosystem: bool,
    
    /// Custom: Enable strategy framework
    pub enable_custom_strategies: bool,
    
    /// Target architecture
    pub target_arch: TargetArch,
    
    /// JIT compilation threshold (how many calls before JIT kicks in)
    pub jit_threshold: usize,
    
    /// Enable Link-Time Optimization (LTO)
    pub enable_lto: bool,
    
    /// Enable SIMD vectorization
    pub enable_vectorization: bool,
    
    /// Custom optimization strategies to enable
    pub enabled_strategies: Vec<String>,
}

impl Default for KillerSuperConfig {
    fn default() -> Self {
        Self::development()
    }
}

impl KillerSuperConfig {
    /// Create development mode configuration
    pub fn development() -> Self {
        Self {
            mode: CompilerMode::Development,
            optimization_level: OptimizationLevel::O1,
            enable_phase1_instruction_opt: true,
            enable_phase2_jit: true,
            enable_phase3_type_specialization: true,
            enable_phase4_llvm_backend: true,
            enable_phase5_ecosystem: true,
            enable_custom_strategies: true,
            target_arch: TargetArch::X8664,
            jit_threshold: 3,
            enable_lto: false,
            enable_vectorization: true,
            enabled_strategies: vec![],
        }
    }

    /// Create production mode configuration (maximum optimization)
    pub fn production() -> Self {
        Self {
            mode: CompilerMode::Production,
            optimization_level: OptimizationLevel::O3,
            enable_phase1_instruction_opt: true,
            enable_phase2_jit: true,
            enable_phase3_type_specialization: true,
            enable_phase4_llvm_backend: true,
            enable_phase5_ecosystem: true,
            enable_custom_strategies: true,
            target_arch: TargetArch::X8664,
            jit_threshold: 1, // Aggressive JIT
            enable_lto: true,
            enable_vectorization: true,
            enabled_strategies: vec![
                "memory".to_string(),
                "simd".to_string(),
                "cache".to_string(),
            ],
        }
    }

    /// Create debug mode configuration (no optimization, verbose errors)
    pub fn debug() -> Self {
        Self {
            mode: CompilerMode::Debug,
            optimization_level: OptimizationLevel::O0,
            enable_phase1_instruction_opt: false,
            enable_phase2_jit: false,
            enable_phase3_type_specialization: false,
            enable_phase4_llvm_backend: false,
            enable_phase5_ecosystem: true, // Still need ecosystem
            enable_custom_strategies: false,
            target_arch: TargetArch::X8664,
            jit_threshold: usize::MAX, // Never JIT
            enable_lto: false,
            enable_vectorization: false,
            enabled_strategies: vec![],
        }
    }

    /// Get expected speedup based on configuration
    pub fn expected_speedup(&self) -> f64 {
        let mut speedup = 1.0;

        if self.enable_phase1_instruction_opt {
            speedup *= 1.1; // 10% improvement
        }

        if self.enable_phase2_jit {
            speedup *= 5.0; // 5x from JIT
        }

        if self.enable_phase3_type_specialization {
            speedup *= 2.5; // 2.5x from type specialization
        }

        if self.enable_phase4_llvm_backend {
            speedup *= match self.optimization_level {
                OptimizationLevel::O0 => 1.5,
                OptimizationLevel::O1 => 3.0,
                OptimizationLevel::O2 => 7.0,
                OptimizationLevel::O3 => 10.0,
            };

            if self.enable_lto {
                speedup *= 1.3; // 30% bonus from LTO
            }

            if self.enable_vectorization {
                speedup *= 1.5; // 50% bonus from vectorization
            }
        }

        speedup
    }

    /// Get description of active optimizations
    pub fn active_optimizations(&self) -> Vec<String> {
        let mut opts = vec![];

        if self.enable_phase1_instruction_opt {
            opts.push("Phase 1: Instruction Optimization".to_string());
        }
        if self.enable_phase2_jit {
            opts.push(format!("Phase 2: JIT (threshold={})", self.jit_threshold));
        }
        if self.enable_phase3_type_specialization {
            opts.push("Phase 3: Type Specialization".to_string());
        }
        if self.enable_phase4_llvm_backend {
            opts.push(format!("Phase 4: LLVM Backend (-O{})", match self.optimization_level {
                OptimizationLevel::O0 => "0",
                OptimizationLevel::O1 => "1",
                OptimizationLevel::O2 => "2",
                OptimizationLevel::O3 => "3",
            }));

            if self.enable_lto {
                opts.push("  + LTO".to_string());
            }
            if self.enable_vectorization {
                opts.push("  + Vectorization".to_string());
            }
        }
        if self.enable_phase5_ecosystem {
            opts.push("Phase 5: Ecosystem (File/JSON/HTTP)".to_string());
        }
        if self.enable_custom_strategies && !self.enabled_strategies.is_empty() {
            opts.push(format!("Custom: {} strategies", self.enabled_strategies.len()));
        }

        opts
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OptimizationLevel::O0 => "-O0",
                OptimizationLevel::O1 => "-O1",
                OptimizationLevel::O2 => "-O2",
                OptimizationLevel::O3 => "-O3",
            }
        )
    }
}

impl fmt::Display for CompilerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CompilerMode::Development => "Development",
                CompilerMode::Production => "Production",
                CompilerMode::Debug => "Debug",
            }
        )
    }
}

impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TargetArch::X8664 => "x86-64",
                TargetArch::Aarch64 => "aarch64",
                TargetArch::Wasm32 => "wasm32",
                TargetArch::Riscv64 => "riscv64",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_development_config() {
        let cfg = KillerSuperConfig::development();
        assert_eq!(cfg.mode, CompilerMode::Development);
        assert_eq!(cfg.optimization_level, OptimizationLevel::O1);
        assert!(cfg.enable_phase1_instruction_opt);
        assert!(cfg.enable_phase3_type_specialization);
    }

    #[test]
    fn test_production_config() {
        let cfg = KillerSuperConfig::production();
        assert_eq!(cfg.mode, CompilerMode::Production);
        assert_eq!(cfg.optimization_level, OptimizationLevel::O3);
        assert_eq!(cfg.jit_threshold, 1); // Aggressive
        assert!(cfg.enable_lto);
    }

    #[test]
    fn test_debug_config() {
        let cfg = KillerSuperConfig::debug();
        assert_eq!(cfg.mode, CompilerMode::Debug);
        assert_eq!(cfg.optimization_level, OptimizationLevel::O0);
        assert!(!cfg.enable_phase2_jit);
        assert!(!cfg.enable_phase3_type_specialization);
    }

    #[test]
    fn test_expected_speedup_production() {
        let cfg = KillerSuperConfig::production();
        let speedup = cfg.expected_speedup();
        // Should be significant with all phases enabled
        assert!(speedup > 50.0, "Production should have high speedup");
    }

    #[test]
    fn test_active_optimizations_displays_phases() {
        let cfg = KillerSuperConfig::production();
        let opts = cfg.active_optimizations();
        assert!(!opts.is_empty());
        assert!(opts[0].contains("Phase"));
    }

    #[test]
    fn test_display_formats() {
        assert_eq!(OptimizationLevel::O3.to_string(), "-O3");
        assert_eq!(CompilerMode::Production.to_string(), "Production");
        assert_eq!(TargetArch::X8664.to_string(), "x86-64");
    }
}
