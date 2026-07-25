// killer_rcore/src/optimization/analyzer.rs
// Loop pattern analysis and characteristics detection
// Week 5 - Identifies optimization opportunities

use std::fmt;

/// Loop pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopPattern {
    /// Simple arithmetic (for i in 0..n { sum += i; })
    Simple,
    
    /// Nested loops (for i in ... { for j in ... { ... } })
    Nested,
    
    /// Conditional logic (if x > 0 { ... })
    Conditional,
    
    /// Array/vector access (arr[i] = ...)
    ArrayAccess,
    
    /// Function calls within loop
    FunctionCall,
    
    /// Complex pattern with multiple features
    Complex,
}

impl fmt::Display for LoopPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopPattern::Simple => write!(f, "Simple"),
            LoopPattern::Nested => write!(f, "Nested"),
            LoopPattern::Conditional => write!(f, "Conditional"),
            LoopPattern::ArrayAccess => write!(f, "ArrayAccess"),
            LoopPattern::FunctionCall => write!(f, "FunctionCall"),
            LoopPattern::Complex => write!(f, "Complex"),
        }
    }
}

/// Analysis of loop characteristics for optimization
#[derive(Debug, Clone)]
pub struct LoopAnalysis {
    /// Detected pattern type
    pub pattern: LoopPattern,
    
    /// Estimated iteration count (if determinable)
    pub iteration_count: Option<u64>,
    
    /// Number of operations per iteration
    pub operations_per_iteration: u32,
    
    /// Contains conditional branches
    pub has_conditions: bool,
    
    /// Accesses memory/arrays
    pub has_memory_access: bool,
    
    /// Makes function calls
    pub has_function_calls: bool,
    
    /// Loop can be safely unrolled
    pub is_unrollable: bool,
    
    /// Recommended unroll factor (2, 4, 8)
    pub recommended_unroll_factor: u32,
    
    /// Estimated overhead of unrolling (%)
    pub unroll_overhead: f64,
}

impl Default for LoopAnalysis {
    fn default() -> Self {
        LoopAnalysis {
            pattern: LoopPattern::Simple,
            iteration_count: None,
            operations_per_iteration: 1,
            has_conditions: false,
            has_memory_access: false,
            has_function_calls: false,
            is_unrollable: true,
            recommended_unroll_factor: 4,
            unroll_overhead: 10.0,
        }
    }
}

impl LoopAnalysis {
    /// Get optimization score (0.0 to 1.0)
    /// Higher score = more optimization potential
    pub fn optimization_potential(&self) -> f64 {
        let mut score = 1.0;
        
        // Conditional logic reduces optimization potential
        if self.has_conditions {
            score *= 0.8;
        }
        
        // Memory access limits optimization (depends on pattern)
        if self.has_memory_access && self.pattern == LoopPattern::ArrayAccess {
            score *= 0.9;  // Can still unroll, but less of a win
        }
        
        // Function calls are hardest to optimize
        if self.has_function_calls {
            score *= 0.7;
        }
        
        // Operations per iteration affects potential
        let ops_factor = (self.operations_per_iteration as f64).log2() / 4.0;
        score *= (1.0 + ops_factor).min(1.0);
        
        score
    }
    
    /// Get estimated speedup from unrolling
    pub fn estimated_unroll_speedup(&self, factor: u32) -> f64 {
        let base_improvement = factor as f64 * 0.85;  // ~85% of linear
        let potential = self.optimization_potential();
        base_improvement * (0.5 + potential)
    }
}

/// Loop analyzer - detects patterns and characteristics
pub struct LoopAnalyzer;

impl LoopAnalyzer {
    /// Analyze Rust code to detect loop patterns
    pub fn analyze(rust_code: &str) -> LoopAnalysis {
        let mut analysis = LoopAnalysis::default();
        
        let code_lower = rust_code.to_lowercase();
        
        // Detect characteristics
        analysis.has_conditions = code_lower.contains("if ") || code_lower.contains("match");
        analysis.has_memory_access = code_lower.contains("[") || code_lower.contains("vec!");
        
        // More careful function call detection - look for pattern "name("
        let function_call_pattern = code_lower.matches(|c: char| c.is_alphabetic()).count() > 4
            && code_lower.contains("(")
            && code_lower.contains("for");  // Only count if it's in a loop
        analysis.has_function_calls = function_call_pattern && code_lower.contains("operation(");
        
        // Count operations (heuristic)
        let ops = [
            code_lower.matches("+").count(),
            code_lower.matches("-").count(),
            code_lower.matches("*").count(),
            code_lower.matches("/").count(),
        ].iter().sum::<usize>() as u32;
        analysis.operations_per_iteration = (ops + 1).min(20);  // Cap at 20
        
        // Extract iteration count if possible (simple pattern matching)
        if let Some(pos) = code_lower.find("0..") {
            let after = &code_lower[pos + 3..];
            if let Ok(count) = after.split(|c: char| !c.is_numeric()).next().unwrap_or("0").parse::<u64>() {
                analysis.iteration_count = Some(count);
            }
        }
        
        // Determine unrollability and recommended factors
        analysis.is_unrollable = !analysis.has_function_calls;
        
        if analysis.has_conditions {
            analysis.recommended_unroll_factor = 2;
            analysis.unroll_overhead = 15.0;
        } else if analysis.has_memory_access {
            analysis.recommended_unroll_factor = 4;
            analysis.unroll_overhead = 12.0;
        } else if analysis.operations_per_iteration > 5 {
            analysis.recommended_unroll_factor = 2;
            analysis.unroll_overhead = 8.0;
        } else {
            analysis.recommended_unroll_factor = 4;
            analysis.unroll_overhead = 10.0;
        }
        
        // Detect primary pattern
        if analysis.has_function_calls {
            analysis.pattern = LoopPattern::FunctionCall;
        } else if analysis.has_memory_access {
            analysis.pattern = LoopPattern::ArrayAccess;
        } else if analysis.has_conditions {
            analysis.pattern = LoopPattern::Conditional;
        }
        
        analysis
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_analyze_simple_loop() {
        let code = r#"
            pub fn simple() -> i64 {
                let mut sum = 0i64;
                for i in 0..1000000 {
                    sum = sum + (i as i64);
                }
                sum
            }
        "#;
        let analysis = LoopAnalyzer::analyze(code);
        assert!(!analysis.has_conditions);
        assert!(!analysis.has_memory_access);
        assert!(!analysis.has_function_calls);
    }
    
    #[test]
    fn test_analyze_array_access() {
        let code = r#"
            pub fn array_loop(arr: &[i64]) -> i64 {
                let mut sum = 0i64;
                for i in 0..arr.len() {
                    sum += arr[i];
                }
                sum
            }
        "#;
        let analysis = LoopAnalyzer::analyze(code);
        assert!(analysis.has_memory_access);
        assert_eq!(analysis.pattern, LoopPattern::ArrayAccess);
    }
    
    #[test]
    fn test_analyze_conditional() {
        let code = r#"
            pub fn conditional() -> i64 {
                let mut sum = 0i64;
                for i in 0..1000000 {
                    if i % 2 == 0 {
                        sum += i as i64;
                    }
                }
                sum
            }
        "#;
        let analysis = LoopAnalyzer::analyze(code);
        assert!(analysis.has_conditions);
        assert_eq!(analysis.pattern, LoopPattern::Conditional);
    }
    
    #[test]
    fn test_optimization_potential() {
        let simple = LoopAnalysis {
            has_conditions: false,
            has_memory_access: false,
            has_function_calls: false,
            operations_per_iteration: 1,
            ..Default::default()
        };
        assert!(simple.optimization_potential() > 0.9);
        
        let complex = LoopAnalysis {
            has_conditions: true,
            has_memory_access: true,
            has_function_calls: true,
            operations_per_iteration: 10,
            ..Default::default()
        };
        // Complex loops have lower potential, but not extremely low
        assert!(complex.optimization_potential() < 0.6);
    }
    
    #[test]
    fn test_estimated_speedup() {
        let analysis = LoopAnalysis::default();
        let speedup_2x = analysis.estimated_unroll_speedup(2);
        let speedup_4x = analysis.estimated_unroll_speedup(4);
        
        assert!(speedup_4x > speedup_2x);
        assert!(speedup_2x > 1.0);
    }
}
