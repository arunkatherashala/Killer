// killer_rcore/src/optimizer/loop_detector.rs
// Hot loop detection module
// Week 1 implementation for Killer Advanced v4.0

use std::collections::HashMap;
use crate::ast::*;

/// Represents a detected loop in the AST
#[derive(Clone, Debug)]
pub struct LoopProfile {
    /// Unique identifier for this loop in source
    pub loop_id: String,
    
    /// Estimated number of iterations (conservative)
    pub estimated_iterations: i64,
    
    /// Is this a "hot" loop (>10K iterations)?
    pub is_hot: bool,
    
    /// Loop variable (iterator): e.g., "i"
    pub loop_var: String,
    
    /// Exit condition: how does the loop end?
    pub exit_condition: ExitCondition,
    
    /// Does this loop have branches (if/else)?
    pub has_branches: bool,
    
    /// Can this loop be safely parallelized?
    pub is_parallelizable: bool,
    
    /// Source location for debugging
    pub source_line: usize,
}

/// Represents the loop exit condition
#[derive(Clone, Debug)]
pub struct ExitCondition {
    /// Loop variable name: "i"
    pub var: String,
    
    /// Comparison operator: "<", "<=", ">", ">=", "!="
    pub operator: String,
    
    /// What we're comparing against
    pub bound: Bound,
}

/// The bound of a loop condition
#[derive(Clone, Debug)]
pub enum Bound {
    /// Constant: while i < 1000000
    Constant(i64),
    
    /// Variable: while i < n
    Variable(String),
    
    /// Expression: while i < n + 100
    Expression(String),
    
    /// Unknown: conservative estimate
    Unknown,
}

/// Main hot loop detector
pub struct LoopDetector {
    loops: HashMap<String, LoopProfile>,
    next_id: u32,
}

impl LoopDetector {
    /// Create a new loop detector
    pub fn new() -> Self {
        LoopDetector {
            loops: HashMap::new(),
            next_id: 0,
        }
    }
    
    /// Scan AST and find all hot loops (>10K iterations)
    pub fn detect_hot_loops(&mut self, ast: &Program) -> Vec<LoopProfile> {
        let mut hot_loops = Vec::new();
        
        // Walk through all statements
        for stmt in &ast.statements {
            self.walk_statement(stmt, 0);
        }
        
        // Filter to only hot loops
        for (_, profile) in self.loops.iter() {
            if profile.is_hot {
                hot_loops.push(profile.clone());
            }
        }
        
        hot_loops
    }
    
    /// Recursively walk through AST statements
    fn walk_statement(&mut self, stmt: &Statement, source_line: usize) {
        match stmt {
            Statement::While { 
                condition, 
                body,
                line_number,
                .. 
            } => {
                self.analyze_while_loop(condition, body, *line_number);
            }
            
            Statement::For { 
                init,
                condition, 
                update, 
                body,
                line_number,
                .. 
            } => {
                self.analyze_for_loop(init, condition, update, body, *line_number);
            }
            
            Statement::If { 
                then_branch, 
                else_branch,
                line_number,
                .. 
            } => {
                if let Some(then_stmt) = then_branch {
                    self.walk_statement(then_stmt, *line_number);
                }
                if let Some(else_stmt) = else_branch {
                    self.walk_statement(else_stmt, *line_number);
                }
            }
            
            Statement::Block { statements, .. } => {
                for s in statements {
                    self.walk_statement(s, source_line);
                }
            }
            
            _ => {}
        }
    }
    
    /// Analyze a while loop: while i < 1000000 { ... }
    fn analyze_while_loop(
        &mut self,
        condition: &Expr,
        body: &[Statement],
        line_number: usize,
    ) {
        if let Some(exit_cond) = self.extract_exit_condition(condition) {
            let loop_id = format!("loop_{:08x}", self.next_id);
            self.next_id += 1;
            
            // Estimate iterations from bound
            let iterations = self.estimate_iterations(&exit_cond);
            let is_hot = iterations > 10_000;
            
            // Check for branches in body
            let has_branches = self.check_has_branches(body);
            let is_parallelizable = !has_branches;
            
            let profile = LoopProfile {
                loop_id: loop_id.clone(),
                estimated_iterations: iterations,
                is_hot,
                loop_var: exit_cond.var.clone(),
                exit_condition: exit_cond,
                has_branches,
                is_parallelizable,
                source_line: line_number,
            };
            
            self.loops.insert(loop_id, profile);
        }
    }
    
    /// Analyze a for loop: for i = 0; i < N; i += 1 { ... }
    fn analyze_for_loop(
        &mut self,
        _init: &Option<Box<Statement>>,
        condition: &Option<Box<Expr>>,
        _update: &Option<Box<Statement>>,
        body: &[Statement],
        line_number: usize,
    ) {
        // For loops are similar to while loops
        if let Some(cond_expr) = condition {
            if let Some(exit_cond) = self.extract_exit_condition(cond_expr) {
                let loop_id = format!("loop_{:08x}", self.next_id);
                self.next_id += 1;
                
                let iterations = self.estimate_iterations(&exit_cond);
                let is_hot = iterations > 10_000;
                let has_branches = self.check_has_branches(body);
                let is_parallelizable = !has_branches;
                
                let profile = LoopProfile {
                    loop_id: loop_id.clone(),
                    estimated_iterations: iterations,
                    is_hot,
                    loop_var: exit_cond.var.clone(),
                    exit_condition: exit_cond,
                    has_branches,
                    is_parallelizable,
                    source_line: line_number,
                };
                
                self.loops.insert(loop_id, profile);
            }
        }
    }
    
    /// Extract exit condition from a boolean expression
    /// Looks for patterns like: i < 1000000, i <= N, i != max, etc.
    fn extract_exit_condition(&self, condition: &Expr) -> Option<ExitCondition> {
        match condition {
            Expr::BinaryOp { left, op, right } => {
                // Check if left side is a variable (loop iterator)
                if let Expr::Var(var_name) = left.as_ref() {
                    // Extract bound from right side
                    if let Some(bound) = self.extract_bound(right) {
                        return Some(ExitCondition {
                            var: var_name.clone(),
                            operator: op.clone(),
                            bound,
                        });
                    }
                }
                
                // Try reversed: bound op i (less common but valid)
                if let Expr::Var(var_name) = right.as_ref() {
                    if let Some(bound) = self.extract_bound(left) {
                        // Reverse the operator
                        let reversed_op = match op.as_str() {
                            "<" => ">",
                            "<=" => ">=",
                            ">" => "<",
                            ">=" => "<=",
                            "==" => "==",
                            "!=" => "!=",
                            _ => return None,
                        };
                        
                        return Some(ExitCondition {
                            var: var_name.clone(),
                            operator: reversed_op.to_string(),
                            bound,
                        });
                    }
                }
            }
            _ => {}
        }
        
        None
    }
    
    /// Extract the bound from an expression
    fn extract_bound(&self, expr: &Expr) -> Option<Bound> {
        match expr {
            // Constant literal: 1000000
            Expr::Literal(Literal::Integer(n)) => {
                Some(Bound::Constant(*n as i64))
            }
            
            // Variable reference: n
            Expr::Var(v) => {
                Some(Bound::Variable(v.clone()))
            }
            
            // Complex expression: n + 100, len - 1, etc.
            Expr::BinaryOp { .. } => {
                // For now, treat as unknown
                // In production, we'd recursively analyze
                Some(Bound::Unknown)
            }
            
            _ => None,
        }
    }
    
    /// Estimate iteration count from exit condition
    fn estimate_iterations(&self, cond: &ExitCondition) -> i64 {
        match &cond.bound {
            // Constant bound: use directly
            Bound::Constant(n) => {
                match cond.operator.as_str() {
                    "<" | "<=" => *n,
                    ">" | ">=" => *n,
                    "!=" => *n / 2, // Conservative estimate
                    _ => 100_000,
                }
            }
            
            // Variable bound: conservative estimate (assume large)
            Bound::Variable(_) => 100_000,
            
            // Expression bound: conservative estimate
            Bound::Expression(_) => 100_000,
            
            // Unknown: conservative estimate
            Bound::Unknown => 100_000,
        }
    }
    
    /// Check if loop body contains branches (if/else/switch)
    fn check_has_branches(&self, body: &[Statement]) -> bool {
        for stmt in body {
            if self.statement_has_branches(stmt) {
                return true;
            }
        }
        false
    }
    
    fn statement_has_branches(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::If { .. } => true,
            Statement::Switch { .. } => true,
            Statement::Block { statements, .. } => {
                self.check_has_branches(statements)
            }
            _ => false,
        }
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper to create a simple test loop
    fn create_simple_while_loop(iterations: i64) -> Statement {
        Statement::While {
            condition: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Var("i".to_string())),
                op: "<".to_string(),
                right: Box::new(Expr::Literal(Literal::Integer(iterations as i32))),
            }),
            body: vec![
                Statement::Assignment {
                    target: "i".to_string(),
                    value: Box::new(Expr::BinaryOp {
                        left: Box::new(Expr::Var("i".to_string())),
                        op: "+".to_string(),
                        right: Box::new(Expr::Literal(Literal::Integer(1))),
                    }),
                    line_number: 1,
                }
            ],
            line_number: 1,
        }
    }
    
    #[test]
    fn test_simple_hot_loop_detection() {
        let mut detector = LoopDetector::new();
        let program = Program {
            statements: vec![create_simple_while_loop(1_000_000)],
        };
        
        let hot_loops = detector.detect_hot_loops(&program);
        
        assert_eq!(hot_loops.len(), 1);
        assert!(hot_loops[0].is_hot);
        assert_eq!(hot_loops[0].estimated_iterations, 1_000_000);
        assert_eq!(hot_loops[0].loop_var, "i");
    }
    
    #[test]
    fn test_cold_loop_ignored() {
        let mut detector = LoopDetector::new();
        let program = Program {
            statements: vec![create_simple_while_loop(100)],
        };
        
        let hot_loops = detector.detect_hot_loops(&program);
        
        assert_eq!(hot_loops.len(), 0); // 100 < 10K = cold loop
    }
    
    #[test]
    fn test_boundary_10k() {
        let mut detector = LoopDetector::new();
        
        // Exactly 10,000 iterations - boundary
        let program = Program {
            statements: vec![create_simple_while_loop(10_000)],
        };
        
        let hot_loops = detector.detect_hot_loops(&program);
        
        assert_eq!(hot_loops.len(), 0); // 10K = still cold (need > 10K)
    }
    
    #[test]
    fn test_just_above_threshold() {
        let mut detector = LoopDetector::new();
        
        // 10,001 iterations - should be hot
        let program = Program {
            statements: vec![create_simple_while_loop(10_001)],
        };
        
        let hot_loops = detector.detect_hot_loops(&program);
        
        assert_eq!(hot_loops.len(), 1);
        assert!(hot_loops[0].is_hot);
    }
    
    #[test]
    fn test_loop_var_extraction() {
        let mut detector = LoopDetector::new();
        
        let program = Program {
            statements: vec![
                Statement::While {
                    condition: Box::new(Expr::BinaryOp {
                        left: Box::new(Expr::Var("counter".to_string())),
                        op: "<".to_string(),
                        right: Box::new(Expr::Literal(Literal::Integer(1_000_000))),
                    }),
                    body: vec![],
                    line_number: 1,
                }
            ],
        };
        
        let hot_loops = detector.detect_hot_loops(&program);
        
        assert_eq!(hot_loops.len(), 1);
        assert_eq!(hot_loops[0].loop_var, "counter");
    }
}
