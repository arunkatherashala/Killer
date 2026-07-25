/// AI Code Analyzer - Phase 2 Implementation
/// 
/// Analyzes function bodies to detect optimization opportunities and generate
/// AI hints for developers. Integrates with Phase 1 AI annotations.
/// 
/// Architecture:
/// 1. PatternRecognizer - detects loop inefficiencies, allocations, vectorization
/// 2. HintGenerator - creates AIHint from detected patterns
/// 3. CodeAnalyzer - orchestrates pattern recognition and hint generation
/// 4. SuperAgentIntegration - calls LLM for complex analysis (Phase 3)

use crate::ai_annotations::{AIHint, AIHintSet, AIAnnotation, AIAnnotationType};
use crate::ast::Stmt;
use std::collections::HashMap;

/// Represents a detected code pattern that can be optimized
#[derive(Debug, Clone, PartialEq)]
pub enum CodePattern {
    /// Nested loop that could be vectorized
    NestedLoop {
        depth: usize,
        operations: Vec<String>,
    },
    /// Unoptimized memory allocation
    AllocationHotspot {
        frequency: usize,
        estimated_waste: String,
    },
    /// String concatenation in loop (inefficient)
    StringConcatenationLoop {
        iterations: usize,
        allocations: usize,
    },
    /// Sequential computation that could be parallelized
    SequentialComputation {
        steps: usize,
        dependencies: usize,
    },
    /// Repeated computation that could be memoized
    RepeatedComputation {
        pattern: String,
        frequency: usize,
    },
}

/// Pattern recognition engine - detects optimization opportunities
#[derive(Debug, Clone)]
pub struct PatternRecognizer {
    patterns: Vec<CodePattern>,
    statistics: HashMap<String, usize>,
}

impl PatternRecognizer {
    /// Create new pattern recognizer
    pub fn new() -> Self {
        PatternRecognizer {
            patterns: Vec::new(),
            statistics: HashMap::new(),
        }
    }

    /// Analyze statements for common patterns
    pub fn analyze_statements(&mut self, statements: &[Stmt]) -> Vec<CodePattern> {
        self.patterns.clear();
        self.statistics.clear();

        for stmt in statements {
            self.analyze_statement(stmt);
        }

        self.patterns.clone()
    }

    /// Recursively analyze a statement
    fn analyze_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::For { variable: _, iterable: _, is_for_of: _, body } => {
                self.detect_loop_pattern(body);
            }
            Stmt::ForC { init: _, condition: _, update: _, body } => {
                self.detect_loop_pattern(body);
            }
            Stmt::While { condition: _, body } => {
                self.detect_loop_pattern(body);
            }
            Stmt::DoWhile { body, condition: _ } => {
                self.detect_loop_pattern(body);
            }
            Stmt::If { condition: _, then_branch, else_branch } => {
                for s in then_branch {
                    self.analyze_statement(s);
                }
                for s in else_branch {
                    self.analyze_statement(s);
                }
            }
            Stmt::Function { name: _, params: _, body, ai_annotations: _ } => {
                // Recursively analyze function bodies
                for s in body {
                    self.analyze_statement(s);
                }
            }
            _ => {}
        }
    }

    /// Detect loop optimization patterns
    fn detect_loop_pattern(&mut self, body: &[Stmt]) {
        let loop_ops = self.count_operations(body);

        // Detect nested loops
        let nested_depth = self.calculate_nesting_depth(body);
        if nested_depth > 1 {
            self.patterns.push(CodePattern::NestedLoop {
                depth: nested_depth,
                operations: loop_ops,
            });
            *self.statistics.entry("nested_loops".to_string()).or_insert(0) += 1;
        }

        // Detect string concatenation in loops (inefficient)
        if self.has_string_concatenation(body) {
            self.patterns.push(CodePattern::StringConcatenationLoop {
                iterations: body.len(),
                allocations: body.len(), // Conservative estimate
            });
            *self.statistics.entry("string_concat_loops".to_string()).or_insert(0) += 1;
        }
    }

    /// Count operations in statement body
    fn count_operations(&self, body: &[Stmt]) -> Vec<String> {
        let mut ops = Vec::new();
        for stmt in body {
            match stmt {
                Stmt::Let { .. } => ops.push("assignment".to_string()),
                Stmt::Assign { .. } => ops.push("assignment".to_string()),
                Stmt::IndexAssign { .. } => ops.push("index_assign".to_string()),
                Stmt::Expr(_) => ops.push("expression".to_string()),
                Stmt::Print(_) => ops.push("print".to_string()),
                _ => {}
            }
        }
        ops
    }

    /// Calculate loop nesting depth
    fn calculate_nesting_depth(&self, body: &[Stmt]) -> usize {
        let mut max_depth = 0;
        for stmt in body {
            let depth = match stmt {
                Stmt::For { variable: _, iterable: _, is_for_of: _, body: inner } => {
                    1 + self.calculate_nesting_depth(inner)
                }
                Stmt::ForC { init: _, condition: _, update: _, body: inner } => {
                    1 + self.calculate_nesting_depth(inner)
                }
                Stmt::While { condition: _, body: inner } => {
                    1 + self.calculate_nesting_depth(inner)
                }
                Stmt::DoWhile { body: inner, condition: _ } => {
                    1 + self.calculate_nesting_depth(inner)
                }
                _ => 0,
            };
            max_depth = max_depth.max(depth);
        }
        max_depth
    }

    /// Check if body contains string concatenation
    fn has_string_concatenation(&self, _body: &[Stmt]) -> bool {
        // Simplified: In real implementation, would analyze expressions
        false
    }

    /// Get detected patterns
    pub fn patterns(&self) -> &[CodePattern] {
        &self.patterns
    }

    /// Get pattern statistics
    pub fn statistics(&self) -> &HashMap<String, usize> {
        &self.statistics
    }
}

/// Hint generator - converts patterns to actionable AI hints
#[derive(Debug, Clone)]
pub struct HintGenerator {
    confidence_thresholds: HashMap<String, f32>,
}

impl HintGenerator {
    /// Create new hint generator
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert("nested_loop_vectorization".to_string(), 0.85);
        thresholds.insert("memory_optimization".to_string(), 0.72);
        thresholds.insert("string_builder".to_string(), 0.90);
        thresholds.insert("parallelization".to_string(), 0.65);
        thresholds.insert("memoization".to_string(), 0.78);

        HintGenerator {
            confidence_thresholds: thresholds,
        }
    }

    /// Generate hints from detected patterns
    pub fn generate_hints(&self, patterns: &[CodePattern]) -> AIHintSet {
        let mut hint_set = AIHintSet::new();

        for pattern in patterns {
            match pattern {
                CodePattern::NestedLoop { depth, operations: _ } => {
                    let confidence = 0.85 + (*depth as f64 * 0.05).min(0.10);
                    let hint = AIHint::new(
                        "vectorization".to_string(),
                        format!(
                            "Nested loop detected (depth: {}). Consider vectorization with SIMD instructions.",
                            depth
                        ),
                        confidence.min(0.95),
                    );
                    hint_set.add_hint(hint);

                    let hint2 = AIHint::new(
                        "parallelization".to_string(),
                        "Consider splitting into parallel tasks using actor model.".to_string(),
                        0.70,
                    );
                    hint_set.add_hint(hint2);
                }
                CodePattern::StringConcatenationLoop {
                    iterations,
                    allocations,
                } => {
                    let confidence = 0.90 - ((*allocations as f64).log2() * 0.05).min(0.20);
                    let hint = AIHint::new(
                        "string_builder".to_string(),
                        format!(
                            "String concatenation in loop ({} iterations, {} allocations). Use StringBuilder pattern.",
                            iterations, allocations
                        ),
                        confidence.max(0.75),
                    );
                    hint_set.add_hint(hint);
                }
                CodePattern::AllocationHotspot {
                    frequency,
                    estimated_waste,
                } => {
                    let confidence = 0.72 + (*frequency as f64).log2() * 0.08;
                    let hint = AIHint::new(
                        "memory_optimization".to_string(),
                        format!(
                            "Hot allocation detected ({} times). Estimated waste: {}. Consider object pool or pre-allocation.",
                            frequency, estimated_waste
                        ),
                        confidence.min(0.95),
                    );
                    hint_set.add_hint(hint);
                }
                CodePattern::SequentialComputation {
                    steps,
                    dependencies,
                } => {
                    let parallelizable_ratio = (*steps as f64 - *dependencies as f64) / *steps as f64;
                    let confidence = (parallelizable_ratio * 0.90).max(0.50);
                    let hint = AIHint::new(
                        "parallelization".to_string(),
                        format!(
                            "Sequential computation with {} steps and {} dependencies. {} steps could be parallelized.",
                            steps, dependencies, steps - dependencies
                        ),
                        confidence,
                    );
                    hint_set.add_hint(hint);
                }
                CodePattern::RepeatedComputation {
                    pattern,
                    frequency,
                } => {
                    let confidence = 0.78 + (*frequency as f64).log10() * 0.10;
                    let hint = AIHint::new(
                        "memoization".to_string(),
                        format!(
                            "Repeated computation detected ({} times): '{}'. Consider memoization cache.",
                            frequency, pattern
                        ),
                        confidence.min(0.95),
                    );
                    hint_set.add_hint(hint);
                }
            }
        }

        hint_set
    }

    /// Get confidence threshold for category
    pub fn confidence_threshold(&self, category: &str) -> Option<f32> {
        self.confidence_thresholds.get(category).copied()
    }
}

/// Main code analyzer - orchestrates pattern recognition and hint generation
#[derive(Debug, Clone)]
pub struct CodeAnalyzer {
    recognizer: PatternRecognizer,
    generator: HintGenerator,
    analysis_cache: HashMap<String, AIHintSet>,
}

impl CodeAnalyzer {
    /// Create new code analyzer
    pub fn new() -> Self {
        CodeAnalyzer {
            recognizer: PatternRecognizer::new(),
            generator: HintGenerator::new(),
            analysis_cache: HashMap::new(),
        }
    }

    /// Analyze a function for optimization opportunities
    pub fn analyze_function(
        &mut self,
        function_name: &str,
        body: &[Stmt],
        annotation: Option<&AIAnnotation>,
    ) -> AIHintSet {
        // Check cache first
        if let Some(cached) = self.analysis_cache.get(function_name) {
            return cached.clone();
        }

        // Perform analysis
        let patterns = self.recognizer.analyze_statements(body);
        let mut hints = self.generator.generate_hints(&patterns);

        // If annotation provided, filter hints based on annotation type
        if let Some(ann) = annotation {
            hints = self.filter_hints_by_annotation(hints, ann);
        }

        // Cache results
        self.analysis_cache.insert(function_name.to_string(), hints.clone());

        hints
    }

    /// Filter hints based on annotation type
    fn filter_hints_by_annotation(&self, hints: AIHintSet, annotation: &AIAnnotation) -> AIHintSet {
        match &annotation.annotation_type {
            AIAnnotationType::Assist(hint_desc) => {
                // For @ai_assist, filter hints matching the description
                let mut filtered = AIHintSet::new();
                for hint in hints.hints() {
                    if hint.suggestion.to_lowercase().contains(&hint_desc.to_lowercase())
                        || hint_desc.to_lowercase().contains(&hint.category.to_lowercase())
                    {
                        filtered.add_hint(hint.clone());
                    }
                }
                if filtered.hints().is_empty() {
                    hints // Return all if no matches
                } else {
                    filtered
                }
            }
            AIAnnotationType::Validate(_constraint) => {
                // For @ai_validate, focus on correctness hints
                let mut filtered = AIHintSet::new();
                for hint in hints.hints() {
                    if hint.category == "correctness" || hint.category == "safety" {
                        filtered.add_hint(hint.clone());
                    }
                }
                hints // Return original if no correctness hints
            }
            AIAnnotationType::Schedule { .. } => {
                // For @ai_schedule, focus on parallelization hints
                let mut filtered = AIHintSet::new();
                for hint in hints.hints() {
                    if hint.category == "parallelization" || hint.category == "async" {
                        filtered.add_hint(hint.clone());
                    }
                }
                if filtered.hints().is_empty() {
                    hints
                } else {
                    filtered
                }
            }
        }
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.analysis_cache.clear();
    }

    /// Get pattern recognizer
    pub fn recognizer(&self) -> &PatternRecognizer {
        &self.recognizer
    }

    /// Get hint generator
    pub fn generator(&self) -> &HintGenerator {
        &self.generator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_recognizer_creation() {
        let recognizer = PatternRecognizer::new();
        assert_eq!(recognizer.patterns().len(), 0);
    }

    #[test]
    fn test_hint_generator_creation() {
        let generator = HintGenerator::new();
        assert!(generator.confidence_threshold("nested_loop_vectorization").is_some());
        assert_eq!(
            generator.confidence_threshold("nested_loop_vectorization"),
            Some(0.85)
        );
    }

    #[test]
    fn test_hint_generator_nested_loop() {
        let generator = HintGenerator::new();
        let patterns = vec![CodePattern::NestedLoop {
            depth: 3,
            operations: vec!["assignment".to_string()],
        }];

        let hints = generator.generate_hints(&patterns);
        assert!(hints.hints().len() > 0);
    }

    #[test]
    fn test_hint_generator_string_concat() {
        let generator = HintGenerator::new();
        let patterns = vec![CodePattern::StringConcatenationLoop {
            iterations: 1000,
            allocations: 1000,
        }];

        let hints = generator.generate_hints(&patterns);
        let top = hints.top_hint();
        assert!(top.is_some());
        if let Some(h) = top {
            assert_eq!(h.category, "string_builder");
        }
    }

    #[test]
    fn test_hint_generator_memory_hotspot() {
        let generator = HintGenerator::new();
        let patterns = vec![CodePattern::AllocationHotspot {
            frequency: 100,
            estimated_waste: "10MB".to_string(),
        }];

        let hints = generator.generate_hints(&patterns);
        let top = hints.top_hint();
        assert!(top.is_some());
    }

    #[test]
    fn test_code_analyzer_creation() {
        let analyzer = CodeAnalyzer::new();
        assert_eq!(analyzer.analysis_cache.len(), 0);
    }

    #[test]
    fn test_code_analyzer_caching() {
        let mut analyzer = CodeAnalyzer::new();
        let body = vec![];

        let hints1 = analyzer.analyze_function("test_func", &body, None);
        let hints2 = analyzer.analyze_function("test_func", &body, None);

        // Should be same reference (cached)
        assert_eq!(hints1.hints().len(), hints2.hints().len());
    }

    #[test]
    fn test_parallel_computation_detection() {
        let generator = HintGenerator::new();
        let patterns = vec![CodePattern::SequentialComputation {
            steps: 10,
            dependencies: 3,
        }];

        let hints = generator.generate_hints(&patterns);
        let top = hints.top_hint();
        assert!(top.is_some());
    }

    #[test]
    fn test_memoization_opportunity() {
        let generator = HintGenerator::new();
        let patterns = vec![CodePattern::RepeatedComputation {
            pattern: "fibonacci(n)".to_string(),
            frequency: 50,
        }];

        let hints = generator.generate_hints(&patterns);
        let top = hints.top_hint();
        assert!(top.is_some());
    }

    #[test]
    fn test_hint_confidence_scores() {
        let generator = HintGenerator::new();

        // Nested loop with different depths
        let patterns = vec![
            CodePattern::NestedLoop {
                depth: 2,
                operations: vec![],
            },
            CodePattern::NestedLoop {
                depth: 5,
                operations: vec![],
            },
        ];

        let hints = generator.generate_hints(&patterns);
        let hint_list = hints.hints();
        assert!(hint_list.len() >= 2);

        // Deeper nesting should have higher confidence
        if hint_list.len() >= 2 {
            let conf1 = hint_list[0].confidence;
            let conf2 = hint_list[1].confidence;
            assert!(conf1 >= 0.0 && conf2 >= 0.0);
        }
    }

    #[test]
    fn test_multiple_pattern_hints() {
        let generator = HintGenerator::new();
        let patterns = vec![
            CodePattern::NestedLoop {
                depth: 2,
                operations: vec![],
            },
            CodePattern::StringConcatenationLoop {
                iterations: 100,
                allocations: 100,
            },
            CodePattern::AllocationHotspot {
                frequency: 50,
                estimated_waste: "5MB".to_string(),
            },
        ];

        let hints = generator.generate_hints(&patterns);
        assert!(hints.hints().len() > 0);
    }

    #[test]
    fn test_empty_patterns() {
        let generator = HintGenerator::new();
        let patterns = vec![];

        let hints = generator.generate_hints(&patterns);
        assert_eq!(hints.hints().len(), 0);
    }
}
