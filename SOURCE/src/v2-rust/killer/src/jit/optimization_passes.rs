// Phase 2.5: LLVM Optimization Passes Strategy
// Simulated LLVM -O3 compiler optimizations
// Applied to generated IR before compilation to native code

#[derive(Debug, Clone)]
pub enum OptimizationPass {
    /// Inline small and frequently-called functions
    InliningPass,
    /// Eliminate dead/unreachable code
    DeadCodeElimination,
    /// Unroll loops to reduce branching
    LoopUnrolling,
    /// Constant folding and propagation
    ConstantFolding,
    /// Common subexpression elimination
    CommonSubExprElimination,
    /// Escape analysis for stack allocation
    EscapeAnalysis,
    /// Strength reduction (optimize expensive operations)
    StrengthReduction,
}

#[derive(Debug, Clone)]
pub struct OptimizationMetrics {
    pub pass_name: String,
    pub instructions_removed: usize,
    pub instructions_replaced: usize,
    pub estimated_speedup_percent: f32,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct LLVMOptimizationStrategy {
    /// Enabled optimization passes (ordered by typical application)
    passes: Vec<OptimizationPass>,
    /// Metrics for each pass applied
    metrics: Vec<OptimizationMetrics>,
    /// Total optimizations applied
    total_optimizations: usize,
    /// Estimated total speedup from all passes
    estimated_total_speedup: f32,
}

impl LLVMOptimizationStrategy {
    pub fn new_o3() -> Self {
        // Equivalent to LLVM -O3 optimization level
        let passes = vec![
            OptimizationPass::InliningPass,
            OptimizationPass::ConstantFolding,
            OptimizationPass::CommonSubExprElimination,
            OptimizationPass::LoopUnrolling,
            OptimizationPass::StrengthReduction,
            OptimizationPass::DeadCodeElimination,
            OptimizationPass::EscapeAnalysis,
        ];

        LLVMOptimizationStrategy {
            passes,
            metrics: Vec::new(),
            total_optimizations: 0,
            estimated_total_speedup: 1.0,
        }
    }

    pub fn new_o2() -> Self {
        // Equivalent to LLVM -O2 optimization level
        let passes = vec![
            OptimizationPass::InliningPass,
            OptimizationPass::ConstantFolding,
            OptimizationPass::CommonSubExprElimination,
            OptimizationPass::LoopUnrolling,
            OptimizationPass::DeadCodeElimination,
        ];

        LLVMOptimizationStrategy {
            passes,
            metrics: Vec::new(),
            total_optimizations: 0,
            estimated_total_speedup: 1.0,
        }
    }

    pub fn new_o1() -> Self {
        // Equivalent to LLVM -O1 optimization level (minimal)
        let passes = vec![
            OptimizationPass::DeadCodeElimination,
            OptimizationPass::ConstantFolding,
        ];

        LLVMOptimizationStrategy {
            passes,
            metrics: Vec::new(),
            total_optimizations: 0,
            estimated_total_speedup: 1.0,
        }
    }

    /// Apply optimization pass with estimated effectiveness
    pub fn apply_optimization_pass(
        &mut self,
        pass: OptimizationPass,
        instruction_count: usize,
    ) -> OptimizationMetrics {
        let (estimated_removed, estimated_replaced, max_speedup) = match pass {
            OptimizationPass::InliningPass => {
                // Typical: 15-25% instruction removal from inlining function calls
                let removed = (instruction_count as f32 * 0.20) as usize;
                (removed, 0, 1.25) // 25% speedup potential
            }
            OptimizationPass::DeadCodeElimination => {
                // Typical: 5-10% unreachable code removal
                let removed = (instruction_count as f32 * 0.075) as usize;
                (removed, 0, 1.10)
            }
            OptimizationPass::LoopUnrolling => {
                // Typical: 10-15% fewer branch instructions
                let removed = (instruction_count as f32 * 0.12) as usize;
                (removed, 0, 1.20)
            }
            OptimizationPass::ConstantFolding => {
                // Typical: 5-15% fewer arithmetic operations
                let removed = (instruction_count as f32 * 0.10) as usize;
                (removed, 0, 1.15)
            }
            OptimizationPass::CommonSubExprElimination => {
                // Typical: 10-20% fewer redundant computations
                let removed = (instruction_count as f32 * 0.15) as usize;
                (removed, 0, 1.20)
            }
            OptimizationPass::EscapeAnalysis => {
                // Typical: 5-10% stack vs heap allocation optimization
                let replaced = (instruction_count as f32 * 0.08) as usize;
                (0, replaced, 1.12)
            }
            OptimizationPass::StrengthReduction => {
                // Typical: 3-8% from replacing expensive ops with cheap ones
                let replaced = (instruction_count as f32 * 0.05) as usize;
                (0, replaced, 1.08)
            }
        };

        let metric = OptimizationMetrics {
            pass_name: format!("{:?}", pass),
            instructions_removed: estimated_removed,
            instructions_replaced: estimated_replaced,
            estimated_speedup_percent: (max_speedup - 1.0) * 100.0,
            enabled: true,
        };

        // Update cumulative speedup (multiplicative)
        self.estimated_total_speedup *= max_speedup;
        self.total_optimizations += 1;
        self.metrics.push(metric.clone());

        metric
    }

    /// Apply all enabled optimization passes to function
    pub fn optimize_function(&mut self, instruction_count: usize) -> Vec<OptimizationMetrics> {
        let mut results = Vec::new();

        for pass in self.passes.clone() {
            let metric = self.apply_optimization_pass(pass, instruction_count);
            results.push(metric);
        }

        results
    }

    /// Get total speedup factor from all applied optimizations
    pub fn get_total_speedup(&self) -> f32 {
        self.estimated_total_speedup
    }

    /// Get optimization metrics
    pub fn get_metrics(&self) -> &[OptimizationMetrics] {
        &self.metrics
    }

    /// Reset metrics for new batch of optimizations
    pub fn reset_metrics(&mut self) {
        self.metrics.clear();
        self.total_optimizations = 0;
        self.estimated_total_speedup = 1.0;
    }

    /// Get passes being applied
    pub fn get_passes(&self) -> &[OptimizationPass] {
        &self.passes
    }

    /// Add custom pass
    pub fn add_pass(&mut self, pass: OptimizationPass) {
        if !self.passes.contains(&pass) {
            self.passes.push(pass);
        }
    }

    /// Remove pass from optimization strategy
    pub fn remove_pass(&mut self, pass: &OptimizationPass) {
        self.passes.retain(|p| p != pass);
    }
}

impl PartialEq for OptimizationPass {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_o3_strategy() {
        let strategy = LLVMOptimizationStrategy::new_o3();
        assert_eq!(strategy.get_passes().len(), 7); // All 7 passes for -O3
    }

    #[test]
    fn test_create_o2_strategy() {
        let strategy = LLVMOptimizationStrategy::new_o2();
        assert_eq!(strategy.get_passes().len(), 5); // Fewer passes for -O2
    }

    #[test]
    fn test_create_o1_strategy() {
        let strategy = LLVMOptimizationStrategy::new_o1();
        assert_eq!(strategy.get_passes().len(), 2); // Minimal passes for -O1
    }

    #[test]
    fn test_apply_inlining_pass() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();
        strategy.reset_metrics();

        let metric = strategy.apply_optimization_pass(OptimizationPass::InliningPass, 1000);

        assert_eq!(metric.pass_name, "InliningPass");
        assert!(metric.instructions_removed > 0);
        assert!(metric.estimated_speedup_percent > 0.0);
        assert!(metric.enabled);
    }

    #[test]
    fn test_apply_dead_code_elimination() {
        let mut strategy = LLVMOptimizationStrategy::new_o1();
        strategy.reset_metrics();

        let metric = strategy.apply_optimization_pass(OptimizationPass::DeadCodeElimination, 1000);

        assert_eq!(metric.pass_name, "DeadCodeElimination");
        assert!(metric.instructions_removed > 0);
    }

    #[test]
    fn test_cumulative_speedup() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();
        strategy.reset_metrics();

        // Apply all passes to 1000-instruction function
        strategy.optimize_function(1000);

        let total_speedup = strategy.get_total_speedup();
        // With all 7 passes: 1.25 * 1.10 * 1.20 * 1.15 * 1.20 * 1.12 * 1.08 ≈ 3.2-3.5x
        assert!(total_speedup > 2.0, "Should achieve at least 2x speedup from -O3");
        assert!(total_speedup < 5.0, "Speedup estimate should be realistic");
    }

    #[test]
    fn test_o3_better_than_o2() {
        let mut o2 = LLVMOptimizationStrategy::new_o2();
        let mut o3 = LLVMOptimizationStrategy::new_o3();

        o2.reset_metrics();
        o3.reset_metrics();

        o2.optimize_function(1000);
        o3.optimize_function(1000);

        assert!(
            o3.get_total_speedup() > o2.get_total_speedup(),
            "-O3 should have better speedup than -O2"
        );
    }

    #[test]
    fn test_metrics_collection() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();
        strategy.reset_metrics();

        strategy.optimize_function(1000);

        let metrics = strategy.get_metrics();
        assert!(metrics.len() > 0);

        // Verify all metrics have reasonable values
        for metric in metrics {
            assert!(metric.estimated_speedup_percent > 0.0);
            assert!(!metric.pass_name.is_empty());
        }
    }

    #[test]
    fn test_add_custom_pass() {
        let mut strategy = LLVMOptimizationStrategy::new_o1();
        let initial_count = strategy.get_passes().len();

        strategy.add_pass(OptimizationPass::InliningPass);

        assert_eq!(strategy.get_passes().len(), initial_count + 1);
    }

    #[test]
    fn test_remove_pass() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();
        let initial_count = strategy.get_passes().len();

        strategy.remove_pass(&OptimizationPass::InliningPass);

        assert_eq!(strategy.get_passes().len(), initial_count - 1);
    }

    #[test]
    fn test_reset_metrics() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();

        strategy.optimize_function(1000);
        assert!(strategy.get_metrics().len() > 0);

        strategy.reset_metrics();
        assert_eq!(strategy.get_metrics().len(), 0);
        assert_eq!(strategy.get_total_speedup(), 1.0);
    }

    #[test]
    fn test_multiple_function_optimizations() {
        let mut strategy = LLVMOptimizationStrategy::new_o3();

        // Optimize first function
        strategy.reset_metrics();
        let speedup1 = {
            strategy.optimize_function(500);
            strategy.get_total_speedup()
        };

        // Optimize second function (different size)
        strategy.reset_metrics();
        let speedup2 = {
            strategy.optimize_function(5000);
            strategy.get_total_speedup()
        };

        // Both should benefit from same passes, speedup should be similar
        assert!(
            (speedup1 - speedup2).abs() < 0.5,
            "Speedup should be consistent across function sizes"
        );
    }

    #[test]
    fn test_optimization_order_matters() {
        let strategy = LLVMOptimizationStrategy::new_o3();

        // Verify passes are in typical optimization order
        let passes = strategy.get_passes();
        let inlining_idx = passes
            .iter()
            .position(|p| p == &OptimizationPass::InliningPass);
        let dce_idx = passes
            .iter()
            .position(|p| p == &OptimizationPass::DeadCodeElimination);

        // Inlining should come before DCE for best results
        if let (Some(inl), Some(dce)) = (inlining_idx, dce_idx) {
            assert!(inl < dce, "Inlining should precede dead code elimination");
        }
    }
}
