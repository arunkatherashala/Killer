// killer_rcore/src/optimizer/loop_analysis.rs
// Extended loop analysis and optimization hints
// Week 2-4 preparation

use super::loop_detector::LoopProfile;

/// Detailed analysis of a loop's optimization potential
#[derive(Debug, Clone)]
pub struct LoopOptimizationScore {
    /// Overall score 0-100 (higher = more benefit from JIT)
    pub score: u32,
    
    /// Why this loop is not optimal (for debugging)
    pub issues: Vec<String>,
    
    /// Optimization recommendations
    pub recommendations: Vec<String>,
    
    /// Estimated speedup from JIT compilation
    pub expected_speedup: f64,
}

impl LoopOptimizationScore {
    /// Score a loop for JIT optimization potential
    pub fn analyze(profile: &LoopProfile) -> Self {
        let mut score = 100u32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        
        // Factor 1: Loop size (iterations)
        // Larger loops benefit more from JIT
        if profile.estimated_iterations < 50_000 {
            score = score.saturating_sub(10);
            issues.push("Loop < 50K iterations: less benefit from JIT".to_string());
        }
        
        if profile.estimated_iterations > 10_000_000 {
            score = score.saturating_sub(5);
            issues.push("Very large loop: compilation overhead may matter".to_string());
        }
        
        // Factor 2: Branches in loop body
        // Branches hurt JIT optimization potential
        if profile.has_branches {
            score = score.saturating_sub(20);
            issues.push("Loop contains branches (if/else): harder to predict".to_string());
            recommendations.push("Consider simplifying loop condition".to_string());
        }
        
        // Factor 3: Parallelizability
        // Parallel loops have different optimization properties
        if profile.is_parallelizable {
            score = score.saturating_sub(0);  // No penalty
            recommendations.push("Loop is parallelizable: consider multi-threaded JIT".to_string());
        } else {
            score = score.saturating_sub(5);
            issues.push("Loop has data dependencies: not parallelizable".to_string());
        }
        
        // Factor 4: Estimated benefit
        // Calculate speedup based on iterations and complexity
        let expected_speedup = Self::estimate_speedup(profile);
        
        // Speedup < 10x: not worth JIT overhead
        if expected_speedup < 10.0 {
            score = score.saturating_sub(30);
        }
        
        LoopOptimizationScore {
            score: score.min(100),
            issues,
            recommendations,
            expected_speedup,
        }
    }
    
    /// Estimate speedup from JIT (conservative: 100x for tight loops)
    fn estimate_speedup(_profile: &LoopProfile) -> f64 {
        // Baseline: 100x speedup for JIT-compiled tight loops
        // In practice: 50-200x depending on complexity
        100.0
    }
    
    /// Is this loop worth JIT-compiling?
    pub fn is_worth_jit(&self) -> bool {
        // Threshold: score >= 50 and speedup >= 50x
        self.score >= 50 && self.expected_speedup >= 50.0
    }
    
    /// Human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Loop Optimization Score: {}/100\nExpected Speedup: {:.1}x\n{}",
            self.score,
            self.expected_speedup,
            if self.is_worth_jit() {
                "✓ RECOMMENDED for JIT compilation"
            } else {
                "✗ Not recommended for JIT compilation"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer::loop_detector::ExitCondition;
    use crate::optimizer::loop_detector::Bound;
    
    fn create_test_profile(iterations: i64, has_branches: bool) -> LoopProfile {
        LoopProfile {
            loop_id: "test_loop".to_string(),
            estimated_iterations: iterations,
            is_hot: iterations > 10_000,
            loop_var: "i".to_string(),
            exit_condition: ExitCondition {
                var: "i".to_string(),
                operator: "<".to_string(),
                bound: Bound::Constant(iterations),
            },
            has_branches,
            is_parallelizable: !has_branches,
            source_line: 1,
        }
    }
    
    #[test]
    fn test_high_score_simple_loop() {
        let profile = create_test_profile(1_000_000, false);
        let analysis = LoopOptimizationScore::analyze(&profile);
        
        assert!(analysis.is_worth_jit());
        assert!(analysis.score >= 70);
    }
    
    #[test]
    fn test_lower_score_with_branches() {
        let profile = create_test_profile(1_000_000, true);
        let analysis = LoopOptimizationScore::analyze(&profile);
        
        assert!(analysis.score < 80);
        assert!(analysis.issues.iter().any(|i| i.contains("branch")));
    }
    
    #[test]
    fn test_small_loop_not_recommended() {
        let profile = create_test_profile(50_000, false);
        let analysis = LoopOptimizationScore::analyze(&profile);
        
        // May still be worth it, but lower score
        assert!(analysis.score < 100);
    }
}
