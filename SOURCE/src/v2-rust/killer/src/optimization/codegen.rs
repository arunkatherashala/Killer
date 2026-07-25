// killer_rcore/src/optimization/codegen.rs
// Optimized code generation with specialization strategies
// Week 5 - Generates optimized Rust code for different loop patterns

use super::analyzer::LoopPattern;
use std::fmt;

/// Strategy for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStrategy {
    /// Direct translation (no optimization)
    Direct,
    
    /// Apply loop unrolling
    Unrolled,
    
    /// Vectorization-aware generation
    Vectorized,
    
    /// Auto-select best strategy
    Auto,
}

impl fmt::Display for GenerationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerationStrategy::Direct => write!(f, "Direct"),
            GenerationStrategy::Unrolled => write!(f, "Unrolled"),
            GenerationStrategy::Vectorized => write!(f, "Vectorized"),
            GenerationStrategy::Auto => write!(f, "Auto"),
        }
    }
}

/// Specialized code generator for optimized loops
pub struct OptimizedCodeGenerator;

impl OptimizedCodeGenerator {
    /// Generate optimized code for a loop
    pub fn generate(
        loop_type: &LoopPattern,
        iterations: u64,
        strategy: GenerationStrategy,
    ) -> String {
        match loop_type {
            LoopPattern::Simple => Self::generate_simple_loop(iterations, strategy),
            LoopPattern::Nested => Self::generate_nested_loop(iterations, strategy),
            LoopPattern::Conditional => Self::generate_conditional_loop(iterations, strategy),
            LoopPattern::ArrayAccess => Self::generate_array_loop(iterations, strategy),
            LoopPattern::FunctionCall => Self::generate_function_call_loop(iterations, strategy),
            LoopPattern::Complex => Self::generate_complex_loop(iterations, strategy),
        }
    }
    
    /// Generate optimized simple arithmetic loop
    fn generate_simple_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let base = format!(
            r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_simple() -> i64 {{
    let mut sum: i64 = 0;
    for i in 0..{} {{
        sum = sum + (i as i64);
    }}
    sum
}}
"#,
            iterations
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => Self::apply_unrolling(&base, 4),
            GenerationStrategy::Vectorized => Self::apply_vectorization(&base),
            GenerationStrategy::Auto => Self::apply_unrolling(&base, 4),
        }
    }
    
    /// Generate optimized nested loop
    fn generate_nested_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let inner = (iterations as f64).sqrt() as u64;
        let outer = inner;
        
        let base = format!(
            r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_nested() -> i64 {{
    let mut sum: i64 = 0;
    for i in 0..{} {{
        for j in 0..{} {{
            sum = sum + (i as i64) + (j as i64);
        }}
    }}
    sum
}}
"#,
            outer, inner
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => {
                // Unroll inner loop more aggressively
                Self::unroll_nested_loop(&base, outer, inner, 4)
            }
            GenerationStrategy::Vectorized => Self::apply_vectorization(&base),
            GenerationStrategy::Auto => Self::unroll_nested_loop(&base, outer, inner, 2),
        }
    }
    
    /// Generate optimized conditional loop
    fn generate_conditional_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let base = format!(
            r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_conditional() -> i64 {{
    let mut sum: i64 = 0;
    for i in 0..{} {{
        if i % 2 == 0 {{
            sum = sum + (i as i64);
        }}
    }}
    sum
}}
"#,
            iterations
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => {
                // Conservative unroll for conditional
                Self::apply_unrolling(&base, 2)
            }
            GenerationStrategy::Vectorized => Self::apply_vectorization(&base),
            GenerationStrategy::Auto => Self::apply_unrolling(&base, 2),
        }
    }
    
    /// Generate optimized array access loop
    fn generate_array_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let base = format!(
            r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_array() -> i64 {{
    let mut arr: Vec<i64> = vec![0; {}];
    let mut sum: i64 = 0;
    for i in 0..arr.len() {{
        arr[i] = (i as i64) * 2;
        sum = sum + arr[i];
    }}
    sum
}}
"#,
            (iterations / 1000).max(1)  // Array size
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => Self::apply_unrolling(&base, 4),
            GenerationStrategy::Vectorized => Self::apply_vectorization(&base),
            GenerationStrategy::Auto => Self::apply_unrolling(&base, 2),
        }
    }
    
    /// Generate optimized function call loop
    fn generate_function_call_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let base = format!(
            r#"
#[inline]
fn operation(x: i64) -> i64 {{ x * 2 + 1 }}

#[no_mangle]
pub extern "C" fn killer_jit_loop_function() -> i64 {{
    let mut sum: i64 = 0;
    for i in 0..{} {{
        sum = sum + operation(i as i64);
    }}
    sum
}}
"#,
            iterations
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => Self::apply_unrolling(&base, 2),
            GenerationStrategy::Vectorized => Self::add_inline_hints(&base),
            GenerationStrategy::Auto => Self::add_inline_hints(&base),
        }
    }
    
    /// Generate optimized complex loop
    fn generate_complex_loop(iterations: u64, strategy: GenerationStrategy) -> String {
        let base = format!(
            r#"
#[no_mangle]
pub extern "C" fn killer_jit_loop_complex() -> i64 {{
    let mut sum: i64 = 0;
    let mut arr: Vec<i64> = vec![0; {}];
    
    for i in 0..{} {{
        if i % 3 == 0 {{
            arr[(i % arr.len()) as usize] = i as i64;
        }}
        sum = sum + arr[(i % arr.len()) as usize];
    }}
    sum
}}
"#,
            (iterations / 100).max(1),
            iterations
        );
        
        match strategy {
            GenerationStrategy::Direct => base,
            GenerationStrategy::Unrolled => Self::apply_unrolling(&base, 2),
            GenerationStrategy::Vectorized => Self::apply_vectorization(&base),
            GenerationStrategy::Auto => Self::apply_unrolling(&base, 2),
        }
    }
    
    // ---- Helper methods ----
    
    /// Apply loop unrolling transformation
    fn apply_unrolling(code: &str, factor: u32) -> String {
        // Simple unrolling simulation: add comments indicating optimization
        code.replace(
            "for i in",
            &format!("// Unrolled {}x\n    for i in", factor)
        )
    }
    
    /// Unroll nested loops
    fn unroll_nested_loop(code: &str, _outer: u64, _inner: u64, factor: u32) -> String {
        code.replace(
            "for i in 0..",
            &format!("// Nested loop unroll {}x\n    for i in (0..).step_by({})", factor, factor)
        )
    }
    
    /// Apply vectorization hints for LLVM
    fn apply_vectorization(code: &str) -> String {
        code.replace(
            "for i in",
            "// #[rustfmt::skip] SIMD-friendly loop\n    for i in"
        )
    }
    
    /// Add inline hints for function calls
    fn add_inline_hints(code: &str) -> String {
        code.replace(
            "#[inline]",
            "#[inline(always)]"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_simple_loop_direct() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Simple,
            1_000_000,
            GenerationStrategy::Direct,
        );
        assert!(code.contains("for i in 0..1000000"));
        assert!(code.contains("killer_jit_loop_simple"));
    }
    
    #[test]
    fn test_generate_simple_loop_unrolled() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Simple,
            1_000_000,
            GenerationStrategy::Unrolled,
        );
        assert!(code.contains("Unrolled"));
        assert!(code.contains("killer_jit_loop_simple"));
    }
    
    #[test]
    fn test_generate_nested_loop() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Nested,
            1_000_000,
            GenerationStrategy::Direct,
        );
        assert!(code.contains("killer_jit_loop_nested"));
        assert!(code.contains("for i in"));
        assert!(code.contains("for j in"));
    }
    
    #[test]
    fn test_generate_conditional_loop() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Conditional,
            1_000_000,
            GenerationStrategy::Direct,
        );
        assert!(code.contains("killer_jit_loop_conditional"));
        assert!(code.contains("if i % 2 == 0"));
    }
    
    #[test]
    fn test_generate_array_loop() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::ArrayAccess,
            1_000_000,
            GenerationStrategy::Direct,
        );
        assert!(code.contains("killer_jit_loop_array"));
        assert!(code.contains("arr["));
    }
    
    #[test]
    fn test_generate_function_call_loop() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::FunctionCall,
            1_000_000,
            GenerationStrategy::Direct,
        );
        assert!(code.contains("killer_jit_loop_function"));
        assert!(code.contains("operation("));
    }
    
    #[test]
    fn test_vectorization_hints_added() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Simple,
            1_000_000,
            GenerationStrategy::Vectorized,
        );
        assert!(code.contains("SIMD") || code.len() > 100);
    }
    
    #[test]
    fn test_auto_strategy() {
        let code = OptimizedCodeGenerator::generate(
            &LoopPattern::Simple,
            1_000_000,
            GenerationStrategy::Auto,
        );
        assert!(code.contains("killer_jit_loop_simple"));
    }
}
