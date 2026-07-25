/// Killer Optimization Engine - Consolidates all VM optimization modules
/// 
/// This module provides a unified interface for all performance optimization layers:
/// - Instruction caching
/// - JIT compilation (tiered)
/// - Hot code detection
/// - Variable caching
/// - Call site caching
/// - Memory pool management
/// - Loop pattern detection
///
/// Architecture:
/// ```
/// VirtualMachine
///   +-- optimization_engine: OptimizationEngine
///       +-- instruction_cache: InstructionCache
///       +-- jit_compiler: JitCompiler
///       +-- hot_detector: HotCodeDetector
///       +-- baseline_jit: BasecodeJITCompiler
///       +-- fast_path: ArithmeticLoopFastPath
///       +-- native_codegen: NativeCodeGenerator
///       +-- variable_cache: LoopOptimization
///       +-- call_site_cache: CallSiteCache
///       +-- value_buffer_pool: ValueBufferPool
///       +-- scope_var_cache: ScopeVariableCache
///       +-- loop_pattern_detector: LoopPatternDetector
/// ```
///
/// This design provides:
/// - Single responsibility: Optimization concerns isolated
/// - Clear dependencies: Pipeline orchestration visible
/// - Testability: Each optimizer tested independently
/// - Maintainability: Easy to enable/disable optimizations
/// - Profiling: Statistics gathered per module

use crate::bytecode::Program;
use crate::instruction_cache::InstructionCache;
use crate::jit_compiler::JitCompiler;
use crate::native_codegen::NativeCodeGenerator;
use crate::variable_caching::LoopOptimization;
use crate::runtime_optimization::{HotCodeDetector, BasecodeJITCompiler, ArithmeticLoopFastPath, OptimizationLevel};
use crate::call_site_cache::CallSiteCache;
use crate::allocation_pool::{ValueBufferPool, ScopeVariableCache};
use crate::loop_pattern_detection::LoopPatternDetector;

/// Central optimization engine managing all performance modules
#[allow(dead_code)]
pub struct OptimizationEngine {
    // Core caching layer
    instruction_cache: Option<InstructionCache>,
    
    // JIT compilation pipeline (tier 0 → tier 3)
    jit_compiler: JitCompiler,           // Tier 0: Simple JIT
    hot_detector: HotCodeDetector,       // Tier 1: Hot path detection
    baseline_jit: BasecodeJITCompiler,   // Tier 2: Baseline JIT
    fast_path: ArithmeticLoopFastPath,   // Tier 3: Fast-path specialization
    native_codegen: NativeCodeGenerator, // Tier 4: Native x86-64 codegen
    
    // Loop optimization
    variable_cache: LoopOptimization,       // O(1) variable access in hot loops
    numeric_fast_mode: bool,                // Skip type checking in arithmetic loops
    loop_pattern_detector: LoopPatternDetector, // Identify optimization opportunities
    
    // Call optimization
    call_site_cache: CallSiteCache,      // Cache method/function call targets
    
    // Memory management
    value_buffer_pool: ValueBufferPool,   // Reuse Value allocations
    scope_var_cache: ScopeVariableCache,  // Cache scope lookups
    
    // Configuration
    level: OptimizationLevel,             // O0-O3 optimization level
    enabled_modules: OptimizationModules, // Which modules are active
}

/// Tracks which optimization modules are enabled
#[derive(Debug, Clone)]
pub struct OptimizationModules {
    pub instruction_cache: bool,
    pub jit_compiler: bool,
    pub hot_detector: bool,
    pub baseline_jit: bool,
    pub fast_path: bool,
    pub native_codegen: bool,
    pub variable_cache: bool,
    pub call_site_cache: bool,
    pub value_buffer_pool: bool,
    pub scope_var_cache: bool,
    pub loop_pattern_detector: bool,
}

impl Default for OptimizationModules {
    fn default() -> Self {
        OptimizationModules {
            instruction_cache: true,
            jit_compiler: false,           // Disabled by default (experimental)
            hot_detector: false,           // Disabled by default (experimental)
            baseline_jit: false,           // Disabled by default (experimental)
            fast_path: false,              // Disabled by default (experimental)
            native_codegen: false,         // Disabled by default (experimental)
            variable_cache: true,
            call_site_cache: true,
            value_buffer_pool: true,
            scope_var_cache: true,
            loop_pattern_detector: true,
        }
    }
}

impl OptimizationEngine {
    /// Create a new optimization engine with default settings
    pub fn new() -> Self {
        Self::with_level(OptimizationLevel::O2)
    }

    /// Create with specific optimization level
    pub fn with_level(level: OptimizationLevel) -> Self {
        let mut modules = OptimizationModules::default();
        
        // Enable modules based on optimization level
        match level {
            OptimizationLevel::O0 => {
                // Minimal optimization: only essential caches
                modules.instruction_cache = true;
                modules.variable_cache = false;
                modules.call_site_cache = false;
                modules.value_buffer_pool = false;
            }
            OptimizationLevel::O1 => {
                // Basic optimization
                modules.instruction_cache = true;
                modules.variable_cache = true;
                modules.call_site_cache = true;
                modules.value_buffer_pool = true;
                modules.loop_pattern_detector = true;
            }
            OptimizationLevel::O2 | OptimizationLevel::Os => {
                // Standard optimization (default)
                modules.instruction_cache = true;
                modules.jit_compiler = true;
                modules.hot_detector = true;
                modules.baseline_jit = true;
                modules.variable_cache = true;
                modules.call_site_cache = true;
                modules.value_buffer_pool = true;
                modules.scope_var_cache = true;
                modules.loop_pattern_detector = true;
            }
            OptimizationLevel::O3 | OptimizationLevel::Oz => {
                // Maximum optimization: all modules enabled
                modules.instruction_cache = true;
                modules.jit_compiler = true;
                modules.hot_detector = true;
                modules.baseline_jit = true;
                modules.fast_path = true;
                modules.native_codegen = true;
                modules.variable_cache = true;
                modules.call_site_cache = true;
                modules.value_buffer_pool = true;
                modules.scope_var_cache = true;
                modules.loop_pattern_detector = true;
            }
        }

        OptimizationEngine {
            instruction_cache: None,
            jit_compiler: JitCompiler::new(),
            hot_detector: HotCodeDetector::new(1000),  // 1K warm-up threshold
            baseline_jit: BasecodeJITCompiler::new(),
            fast_path: ArithmeticLoopFastPath::new(),
            native_codegen: NativeCodeGenerator::new(),
            variable_cache: LoopOptimization::new(),
            numeric_fast_mode: false,
            loop_pattern_detector: LoopPatternDetector::new(),
            call_site_cache: CallSiteCache::new(),
            value_buffer_pool: ValueBufferPool::default(),
            scope_var_cache: ScopeVariableCache::new(),
            level,
            enabled_modules: modules,
        }
    }

    /// Initialize cache structures at program start
    pub fn initialize(&mut self, program: &Program) {
        if self.enabled_modules.instruction_cache {
            self.instruction_cache = Some(InstructionCache::new(program));
        }
    }

    /// Record a variable access for cache statistics
    pub fn record_variable_access(&mut self, var_name: &str, scope_depth: usize) {
        if self.enabled_modules.scope_var_cache {
            self.scope_var_cache.access(var_name, scope_depth);
        }
    }

    /// Check if numeric fast mode should be enabled for a loop
    pub fn should_use_numeric_fast_mode(&self) -> bool {
        self.enabled_modules.variable_cache && self.numeric_fast_mode
    }

    // ========== Module Accessors ==========

    pub fn jit_compiler_mut(&mut self) -> &mut JitCompiler {
        &mut self.jit_compiler
    }

    pub fn hot_detector_mut(&mut self) -> &mut HotCodeDetector {
        &mut self.hot_detector
    }

    pub fn call_site_cache_mut(&mut self) -> &mut CallSiteCache {
        &mut self.call_site_cache
    }

    pub fn value_buffer_pool_mut(&mut self) -> &mut ValueBufferPool {
        &mut self.value_buffer_pool
    }

    pub fn scope_var_cache_mut(&mut self) -> &mut ScopeVariableCache {
        &mut self.scope_var_cache
    }

    pub fn loop_pattern_detector_mut(&mut self) -> &mut LoopPatternDetector {
        &mut self.loop_pattern_detector
    }

    pub fn variable_cache_mut(&mut self) -> &mut LoopOptimization {
        &mut self.variable_cache
    }

    pub fn fast_path_mut(&mut self) -> &mut ArithmeticLoopFastPath {
        &mut self.fast_path
    }

    pub fn native_codegen_mut(&mut self) -> &mut NativeCodeGenerator {
        &mut self.native_codegen
    }

    // ========== Statistics ==========

    pub fn get_statistics(&self) -> OptimizationStatistics {
        OptimizationStatistics {
            instruction_cache_enabled: self.enabled_modules.instruction_cache,
            jit_enabled: self.enabled_modules.jit_compiler,
            hot_detector_enabled: self.enabled_modules.hot_detector,
            call_site_cache_stats: if self.enabled_modules.call_site_cache {
                Some(self.call_site_cache.statistics())
            } else {
                None
            },
            scope_var_cache_hit_rate: if self.enabled_modules.scope_var_cache {
                self.scope_var_cache.hit_rate()
            } else {
                0.0
            },
            optimization_level: self.level.clone(),
        }
    }
}

/// Statistics from the optimization engine
#[derive(Debug)]
pub struct OptimizationStatistics {
    pub instruction_cache_enabled: bool,
    pub jit_enabled: bool,
    pub hot_detector_enabled: bool,
    pub call_site_cache_stats: Option<crate::call_site_cache::CallSiteCacheStats>,
    pub scope_var_cache_hit_rate: f64,
    pub optimization_level: OptimizationLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_engine_creation() {
        let engine = OptimizationEngine::new();
        assert_eq!(engine.level, OptimizationLevel::O2);
    }

    #[test]
    fn test_o0_disables_most_optimizations() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O0);
        assert!(engine.enabled_modules.instruction_cache);
        assert!(!engine.enabled_modules.jit_compiler);
    }

    #[test]
    fn test_o3_enables_all_optimizations() {
        let engine = OptimizationEngine::with_level(OptimizationLevel::O3);
        assert!(engine.enabled_modules.instruction_cache);
        assert!(engine.enabled_modules.jit_compiler);
        assert!(engine.enabled_modules.native_codegen);
    }
}
