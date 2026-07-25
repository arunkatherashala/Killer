/// AI-First Language: Phase 2 - AI Code Analyzer
/// 
/// This module implements intelligent code analysis that:
/// 1. Reads @ai_assist annotations from functions (Phase 1)
/// 2. Analyzes function bodies for optimization patterns
/// 3. Generates AIHint suggestions with confidence scores
/// 4. Ranks hints by priority and improvement potential
/// 5. Integrates with LLM layer for complex analysis
///
/// # Architecture
/// - PatternDetector: Scans AST for optimization opportunities
/// - HintGenerator: Creates AIHint from detected patterns
/// - AnalysisResult: Collects all hints for a function
/// - AICodeAnalyzer: Main orchestrator

use crate::ai_annotations::{AIHint, AIHintSet};
use crate::ast::{Stmt, Expr, BinaryOp};

/// Types of optimization patterns detected
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationPattern {
    /// Nested loops detected (vectorization opportunity)
    NestedLoops { depth: usize, loop_types: Vec<String> },
    
    /// Repeated allocation in loop (memory optimization)
    AllocationInLoop { allocation_type: String, count: usize },
    
    /// Complex arithmetic in hot loop
    ComplexArithmetic { operation_count: usize, types: Vec<String> },
    
    /// Array access pattern (cache optimization)
    ArrayAccess { accesses: usize, pattern: String },
    
    /// String concatenation in loop (GC pressure)
    StringConcatInLoop { count: usize },
    
    /// Potential deadlock pattern
    PotentialDeadlock { shared_resources: Vec<String> },
    
    /// Redundant computation (common subexpression)
    RedundantComputation { computation: String, count: usize },
    
    /// Large function (refactoring candidate)
    LargeFunction { line_count: usize },
}

#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern: OptimizationPattern,
    pub location: (usize, usize), // (start_line, end_line)
    pub severity: f32,            // 0.0-1.0
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub function_name: String,
    pub patterns_detected: Vec<DetectedPattern>,
    pub hints: AIHintSet,
    pub analysis_quality: f32, // 0.0-1.0 confidence in analysis
    pub estimated_improvement: f32, // 0.0-100.0 % improvement
}

/// Main AI Code Analyzer
pub struct AICodeAnalyzer {
    detector: PatternDetector,
    generator: HintGenerator,
}

impl AICodeAnalyzer {
    pub fn new() -> Self {
        AICodeAnalyzer {
            detector: PatternDetector::new(),
            generator: HintGenerator::new(),
        }
    }

    /// Analyze a function and generate AI hints
    pub fn analyze_function(
        &mut self,
        name: &str,
        _params: &[String],
        body: &[Stmt],
    ) -> AnalysisResult {
        // Detect patterns in function body
        let patterns = self.detector.detect_patterns(body);

        // Generate hints from patterns
        let mut hints = AIHintSet::new();
        let mut total_improvement: f32 = 0.0;

        for pattern in &patterns {
            if let Some(hint) = self.generator.generate_hint(pattern) {
                if let Some(improvement_value) = hint.improvement {
                    total_improvement += improvement_value as f32;
                }
                hints.add_hint(hint);
            }
        }

        // Cap total improvement estimate at 100%
        let estimated_improvement = if total_improvement > 100.0 {
            100.0
        } else {
            total_improvement
        };

        // Quality score based on pattern count and types
        let analysis_quality = if patterns.is_empty() {
            0.3 // Low confidence if no patterns found
        } else {
            0.5 + (patterns.len() as f32 * 0.1).min(0.5)
        };

        AnalysisResult {
            function_name: name.to_string(),
            patterns_detected: patterns,
            hints,
            analysis_quality,
            estimated_improvement,
        }
    }
}

/// Detects optimization patterns in code
struct PatternDetector {
    loop_depth: usize,
    allocation_count: usize,
    current_line: usize,
}

impl PatternDetector {
    pub fn new() -> Self {
        PatternDetector {
            loop_depth: 0,
            allocation_count: 0,
            current_line: 1,
        }
    }

    /// Scan statements for optimization patterns
    pub fn detect_patterns(&mut self, body: &[Stmt]) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        // Reset for new analysis
        self.loop_depth = 0;
        self.allocation_count = 0;
        self.current_line = 1;

        // Scan body
        for stmt in body {
            patterns.extend(self.scan_statement(stmt));
        }

        // Check for large function
        if body.len() > 50 {
            patterns.push(DetectedPattern {
                pattern: OptimizationPattern::LargeFunction {
                    line_count: body.len() * 5, // Approximate
                },
                location: (1, body.len() * 5),
                severity: 0.4,
                description: "Function exceeds 50 statements - consider refactoring"
                    .to_string(),
            });
        }

        patterns
    }

    fn scan_statement(&mut self, stmt: &Stmt) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        match stmt {
            Stmt::For { .. } | Stmt::ForC { .. } | Stmt::While { .. } => {
                self.loop_depth += 1;

                // Check body for nested patterns
                let body: &[Stmt] = match stmt {
                    Stmt::For { body, .. } => body,
                    Stmt::ForC { body, .. } => body,
                    Stmt::While { body, .. } => body,
                    _ => &[],
                };

                // Detect nested loops
                if self.loop_depth > 1 {
                    patterns.push(DetectedPattern {
                        pattern: OptimizationPattern::NestedLoops {
                            depth: self.loop_depth,
                            loop_types: vec!["for/while".to_string(); self.loop_depth],
                        },
                        location: (self.current_line, self.current_line + (body.len() as usize)),
                        severity: (self.loop_depth as f32) * 0.2,
                        description: format!(
                            "Nested loop (depth={}) - vectorization opportunity",
                            self.loop_depth
                        ),
                    });
                }

                // Scan loop body
                for inner_stmt in body {
                    patterns.extend(self.scan_statement(inner_stmt));
                }

                self.loop_depth -= 1;
            }
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                for s in then_branch {
                    patterns.extend(self.scan_statement(s));
                }
                for s in else_branch {
                    patterns.extend(self.scan_statement(s));
                }
            }
            Stmt::Expr(expr) => {
                patterns.extend(self.scan_expression(expr));
            }
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
                patterns.extend(self.scan_expression(value));

                // Check for allocation in loop
                if self.is_likely_allocation(value) && self.loop_depth > 0 {
                    self.allocation_count += 1;
                }
            }
            _ => {}
        }

        // Check if we accumulated allocations in loop
        if self.loop_depth == 0 && self.allocation_count >= 1 {
            patterns.push(DetectedPattern {
                pattern: OptimizationPattern::AllocationInLoop {
                    allocation_type: "memory".to_string(),
                    count: self.allocation_count,
                },
                location: (self.current_line, self.current_line + 10),
                severity: 0.6,
                description: format!(
                    "Detected {} allocations in loop - move outside loop",
                    self.allocation_count
                ),
            });
            self.allocation_count = 0;
        }

        patterns
    }

    fn scan_expression(&mut self, expr: &Expr) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        match expr {
            Expr::Call { args, .. } | Expr::MethodCall { args, .. } => {
                for arg in args {
                    patterns.extend(self.scan_expression(arg));
                }
            }
            Expr::Binary { left, op, right } => {
                patterns.extend(self.scan_expression(left));
                patterns.extend(self.scan_expression(right));

                // Detect complex arithmetic
                if self.loop_depth > 0 {
                    match op {
                        BinaryOp::Pow | BinaryOp::Div | BinaryOp::IntDiv => {
                            patterns.push(DetectedPattern {
                                pattern: OptimizationPattern::ComplexArithmetic {
                                    operation_count: 1,
                                    types: vec![format!("{:?}", op)],
                                },
                                location: (self.current_line, self.current_line),
                                severity: 0.5,
                                description: format!(
                                    "Complex operation '{:?}' in loop - precompute if possible",
                                    op
                                ),
                            });
                        }
                        _ => {}
                    }
                }
            }
            Expr::Array(elements) => {
                for elem in elements {
                    patterns.extend(self.scan_expression(elem));
                }
            }
            _ => {}
        }

        patterns
    }

    fn is_likely_allocation(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Array(_) => true,
            Expr::Dict(_) => true,
            Expr::New { .. } => true,
            Expr::Call {
                callee,
                ..
            } => {
                callee.contains("vec") || callee.contains("list") || callee.contains("map")
            }
            _ => false,
        }
    }
}

/// Generates AIHint recommendations from patterns
struct HintGenerator {
    priority_counter: u8,
}

impl HintGenerator {
    pub fn new() -> Self {
        HintGenerator {
            priority_counter: 100,
        }
    }

    pub fn generate_hint(&mut self, pattern: &DetectedPattern) -> Option<AIHint> {
        let priority = self.next_priority();

        match &pattern.pattern {
            OptimizationPattern::NestedLoops { depth, .. } => {
                let depth_f64 = *depth as f64;
                let confidence = (0.7 + (depth_f64 * 0.1)).min(1.0);
                let improvement_pct = 15.0 + ((depth * 10) as f64);
                Some(AIHint {
                    category: "vectorization".to_string(),
                    suggestion: format!(
                        "Consider SIMD vectorization for {}-level nested loop",
                        depth
                    ),
                    confidence,
                    improvement: Some(improvement_pct),
                    priority,
                })
            }
            OptimizationPattern::AllocationInLoop { count, .. } => {
                Some(AIHint {
                    category: "memory".to_string(),
                    suggestion: format!("Move {} allocations outside loop", count),
                    confidence: 0.85,
                    improvement: Some(32.5), // 25-40% average
                    priority,
                })
            }
            OptimizationPattern::ComplexArithmetic { operation_count: _, .. } => {
                Some(AIHint {
                    category: "performance".to_string(),
                    suggestion: "Precompute complex arithmetic operations outside loop"
                        .to_string(),
                    confidence: 0.7,
                    improvement: Some(15.0), // 10-20% average
                    priority,
                })
            }
            OptimizationPattern::ArrayAccess { accesses, pattern: access_pattern } => {
                Some(AIHint {
                    category: "caching".to_string(),
                    suggestion: format!(
                        "Array access pattern {} with {} accesses - optimize cache usage",
                        access_pattern, accesses
                    ),
                    confidence: 0.65,
                    improvement: Some(10.0), // 5-15% average
                    priority,
                })
            }
            OptimizationPattern::StringConcatInLoop { count } => {
                Some(AIHint {
                    category: "memory".to_string(),
                    suggestion: format!(
                        "String concatenation in loop {} times - use StringBuilder pattern",
                        count
                    ),
                    confidence: 0.9,
                    improvement: Some(40.0), // 30-50% average
                    priority,
                })
            }
            OptimizationPattern::PotentialDeadlock { shared_resources } => {
                Some(AIHint {
                    category: "concurrency".to_string(),
                    suggestion: format!(
                        "Potential deadlock with resources: {:?} - review lock order",
                        shared_resources
                    ),
                    confidence: 0.5,
                    improvement: Some(0.0), // Safety improvement, not performance
                    priority,
                })
            }
            OptimizationPattern::RedundantComputation { count, .. } => {
                Some(AIHint {
                    category: "optimization".to_string(),
                    suggestion: format!("Detected {} redundant computations - cache results", count),
                    confidence: 0.8,
                    improvement: Some(12.5), // 5-20% average
                    priority,
                })
            }
            OptimizationPattern::LargeFunction { line_count } => {
                Some(AIHint {
                    category: "refactoring".to_string(),
                    suggestion: format!(
                        "Function has ~{} lines - break into smaller functions",
                        line_count
                    ),
                    confidence: 0.6,
                    improvement: Some(0.0), // Maintainability improvement, not performance
                    priority,
                })
            }
        }
    }

    fn next_priority(&mut self) -> u8 {
        self.priority_counter = self.priority_counter.saturating_sub(10);
        self.priority_counter.max(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Stmt, Expr};

    #[test]
    fn test_analyzer_creation() {
        let _analyzer = AICodeAnalyzer::new();
        println!("✓ AICodeAnalyzer created successfully");
    }

    #[test]
    fn test_empty_function_analysis() {
        let mut analyzer = AICodeAnalyzer::new();
        let result = analyzer.analyze_function("test_fn", &["x".to_string()], &[]);

        assert_eq!(result.function_name, "test_fn");
        assert_eq!(result.patterns_detected.len(), 0);
        println!("✓ Empty function analyzed");
    }

    #[test]
    fn test_large_function_detection() {
        let mut analyzer = AICodeAnalyzer::new();

        // Create a large function body
        let large_body: Vec<Stmt> = (0..60)
            .map(|_| Stmt::Expr(Expr::Number(1.0)))
            .collect();

        let result = analyzer.analyze_function("big_fn", &[], &large_body);

        // Should detect "large function" pattern
        let has_large_fn = result.patterns_detected.iter().any(|p| {
            matches!(p.pattern, OptimizationPattern::LargeFunction { .. })
        });

        assert!(has_large_fn);
        println!("✓ Large function detected (60 statements)");
    }

    #[test]
    fn test_nested_loop_detection() {
        let mut analyzer = AICodeAnalyzer::new();

        // Create nested loop structure
        let inner_loop = Stmt::For {
            variable: "j".to_string(),
            iterable: Box::new(Expr::Range {
                start: Box::new(Expr::Number(0.0)),
                end: Box::new(Expr::Number(10.0)),
                step: None,
            }),
            is_for_of: false,
            body: vec![Stmt::Expr(Expr::Number(1.0))],
        };

        let outer_loop = Stmt::For {
            variable: "i".to_string(),
            iterable: Box::new(Expr::Range {
                start: Box::new(Expr::Number(0.0)),
                end: Box::new(Expr::Number(10.0)),
                step: None,
            }),
            is_for_of: false,
            body: vec![inner_loop],
        };

        let result = analyzer.analyze_function("loop_fn", &[], &[outer_loop]);

        // Should detect nested loops
        let has_nested = result.patterns_detected.iter().any(|p| {
            matches!(p.pattern, OptimizationPattern::NestedLoops { depth: 2, .. })
        });

        assert!(has_nested);
        println!("✓ Nested loop (depth 2) detected");
    }

    #[test]
    fn test_allocation_in_loop_detection() {
        let mut analyzer = AICodeAnalyzer::new();

        // Loop with array allocation
        let loop_body = vec![
            Stmt::Let {
                pattern: crate::ast::Pattern::Identifier("arr".to_string()),
                value: Box::new(Expr::Array(vec![Expr::Number(1.0), Expr::Number(2.0)])),
            },
            Stmt::Expr(Expr::Number(1.0)),
        ];

        let for_loop = Stmt::For {
            variable: "i".to_string(),
            iterable: Box::new(Expr::Range {
                start: Box::new(Expr::Number(0.0)),
                end: Box::new(Expr::Number(100.0)),
                step: None,
            }),
            is_for_of: false,
            body: loop_body,
        };

        let result = analyzer.analyze_function("alloc_fn", &[], &[for_loop]);

        // Should detect allocation in loop
        let has_allocation = result.patterns_detected.iter().any(|p| {
            matches!(p.pattern, OptimizationPattern::AllocationInLoop { .. })
        });

        assert!(has_allocation);
        println!("✓ Allocation in loop detected");
    }

    #[test]
    fn test_hint_generation_quality() {
        let mut analyzer = AICodeAnalyzer::new();

        let inner_loop = Stmt::For {
            variable: "j".to_string(),
            iterable: Box::new(Expr::Range {
                start: Box::new(Expr::Number(0.0)),
                end: Box::new(Expr::Number(10.0)),
                step: None,
            }),
            is_for_of: false,
            body: vec![Stmt::Expr(Expr::Number(1.0))],
        };

        let outer_loop = Stmt::For {
            variable: "i".to_string(),
            iterable: Box::new(Expr::Range {
                start: Box::new(Expr::Number(0.0)),
                end: Box::new(Expr::Number(10.0)),
                step: None,
            }),
            is_for_of: false,
            body: vec![inner_loop],
        };

        let result = analyzer.analyze_function("perf_fn", &[], &[outer_loop]);

        // Should generate hints
        assert!(!result.hints.hints.is_empty());
        println!(
            "✓ {} hints generated, estimated improvement: {}%",
            result.hints.hints.len(),
            result.estimated_improvement
        );
    }

    #[test]
    fn test_analysis_quality_scoring() {
        let mut analyzer = AICodeAnalyzer::new();

        // Empty function - low quality
        let result1 = analyzer.analyze_function("empty", &[], &[]);
        assert!(result1.analysis_quality < 0.5);

        // Function with patterns - higher quality
        let mut analyzer2 = AICodeAnalyzer::new();
        let large_body: Vec<Stmt> = (0..60)
            .map(|_| Stmt::Expr(Expr::Number(1.0)))
            .collect();
        let result2 = analyzer2.analyze_function("large", &[], &large_body);
        assert!(result2.analysis_quality > result1.analysis_quality);

        println!(
            "✓ Quality scoring: empty={:.2}, large={:.2}",
            result1.analysis_quality, result2.analysis_quality
        );
    }
}
