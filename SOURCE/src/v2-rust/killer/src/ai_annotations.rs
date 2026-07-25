/// AI-First Language Extensions for Killer
/// 
/// This module provides AI annotations that can be applied to Killer functions
/// to enable AI-assisted execution, scheduling, and validation at the language level.
///
/// Supported annotations:
/// - @ai_assist("goal") - Enable AI-assisted execution with optimization hints
/// - @ai_schedule("step1|step2|step3", delay=5min, parallel=true) - AI-managed scheduling
/// - @ai_validate("constraints") - AI runtime validation and error correction

use std::collections::HashMap;

/// AI annotation types
#[derive(Debug, Clone, PartialEq)]
pub enum AIAnnotationType {
    /// Assist annotation: @ai_assist("goal description")
    Assist(String),
    
    /// Schedule annotation: @ai_schedule("step1|step2|step3", delay=5min, parallel=true)
    Schedule {
        steps: Vec<String>,
        delay_ms: Option<u64>,
        parallel: bool,
    },
    
    /// Validate annotation: @ai_validate("constraint description")
    Validate(String),
}

/// Complete AI annotation with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct AIAnnotation {
    /// Type of annotation
    pub annotation_type: AIAnnotationType,
    
    /// Function this annotation is applied to
    pub function_name: String,
    
    /// Line number in source code
    pub line: usize,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl AIAnnotation {
    /// Create a new AI annotation
    pub fn new(annotation_type: AIAnnotationType, function_name: String, line: usize) -> Self {
        AIAnnotation {
            annotation_type,
            function_name,
            line,
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to annotation
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Parse annotation string into AIAnnotationType
    /// Examples:
    ///   "assist(numeric optimization)" -> AIAnnotationType::Assist
    ///   "schedule(fetch|analyze|report,delay=5000,parallel=true)" -> AIAnnotationType::Schedule
    ///   "validate(bounds checking)" -> AIAnnotationType::Validate
    pub fn parse_annotation_string(s: &str) -> Result<AIAnnotationType, String> {
        if s.starts_with("assist(") && s.ends_with(")") {
            let goal = s[7..s.len()-1].to_string();
            Ok(AIAnnotationType::Assist(goal))
        } else if s.starts_with("schedule(") && s.ends_with(")") {
            Self::parse_schedule(&s[9..s.len()-1])
        } else if s.starts_with("validate(") && s.ends_with(")") {
            let constraint = s[9..s.len()-1].to_string();
            Ok(AIAnnotationType::Validate(constraint))
        } else {
            Err(format!("Unknown AI annotation: {}", s))
        }
    }
    
    /// Parse schedule annotation parameters
    fn parse_schedule(params: &str) -> Result<AIAnnotationType, String> {
        let parts: Vec<&str> = params.split(',').collect();
        
        if parts.is_empty() {
            return Err("schedule annotation requires at least step names".to_string());
        }
        
        // First part is steps separated by |
        let steps: Vec<String> = parts[0]
            .split('|')
            .map(|s| s.trim().to_string())
            .collect();
        
        let mut delay_ms = None;
        let mut parallel = false;
        
        // Parse additional parameters
        for part in &parts[1..] {
            let kv: Vec<&str> = part.split('=').collect();
            if kv.len() == 2 {
                let key = kv[0].trim();
                let value = kv[1].trim();
                
                if key == "delay" {
                    // Parse delay value (support ms, s, min suffixes)
                    delay_ms = Some(Self::parse_delay(value)?);
                } else if key == "parallel" {
                    parallel = value == "true";
                }
            }
        }
        
        Ok(AIAnnotationType::Schedule {
            steps,
            delay_ms,
            parallel,
        })
    }
    
    /// Parse delay string (e.g., "5000", "5s", "5min")
    fn parse_delay(s: &str) -> Result<u64, String> {
        if s.ends_with("ms") {
            s[..s.len()-2].parse::<u64>()
                .map_err(|_| "invalid delay value".to_string())
        } else if s.ends_with("s") {
            let val = s[..s.len()-1].parse::<u64>()
                .map_err(|_| "invalid delay value".to_string())?;
            Ok(val * 1000)
        } else if s.ends_with("min") {
            let val = s[..s.len()-3].parse::<u64>()
                .map_err(|_| "invalid delay value".to_string())?;
            Ok(val * 60 * 1000)
        } else {
            s.parse::<u64>()
                .map_err(|_| "invalid delay value".to_string())
        }
    }
}

/// AI Hint - Suggestion from AI Optimizer for function execution
#[derive(Debug, Clone)]
pub struct AIHint {
    /// What the suggestion concerns
    pub category: String,
    
    /// The actual hint/suggestion
    pub suggestion: String,
    
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    
    /// Expected improvement (if applicable)
    pub improvement: Option<f64>,
    
    /// Priority (0-10, 10 is highest)
    pub priority: u8,
}

impl AIHint {
    /// Create a new AI hint
    pub fn new(category: String, suggestion: String, confidence: f64) -> Self {
        AIHint {
            category,
            suggestion,
            confidence,
            improvement: None,
            priority: 5,
        }
    }
    
    /// Set expected improvement
    pub fn with_improvement(mut self, improvement: f64) -> Self {
        self.improvement = Some(improvement);
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }
}

/// Collection of AI hints for a function
#[derive(Debug, Clone)]
pub struct AIHintSet {
    pub hints: Vec<AIHint>,
    pub function_name: String,
}

impl AIHintSet {
    /// Create new hint set with default (empty) function name
    pub fn new() -> Self {
        AIHintSet {
            hints: Vec::new(),
            function_name: String::new(),
        }
    }

    /// Create new hint set with specific function name
    pub fn with_function_name(function_name: String) -> Self {
        AIHintSet {
            hints: Vec::new(),
            function_name,
        }
    }
    
    /// Add a hint
    pub fn add_hint(&mut self, hint: AIHint) {
        self.hints.push(hint);
        // Sort by priority (descending)
        self.hints.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    /// Get highest priority hint
    pub fn top_hint(&self) -> Option<&AIHint> {
        self.hints.first()
    }
    
    /// Get all hints with confidence >= threshold
    pub fn high_confidence_hints(&self, threshold: f64) -> Vec<&AIHint> {
        self.hints.iter()
            .filter(|h| h.confidence >= threshold)
            .collect()
    }

    /// Get all hints as slice
    pub fn hints(&self) -> &[AIHint] {
        &self.hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assist_annotation_parse() {
        let annotation = AIAnnotation::parse_annotation_string("assist(numeric optimization)")
            .expect("should parse assist annotation");
        
        match annotation {
            AIAnnotationType::Assist(goal) => {
                assert_eq!(goal, "numeric optimization");
            }
            _ => panic!("expected Assist annotation"),
        }
    }

    #[test]
    fn test_schedule_annotation_parse() {
        let annotation = AIAnnotation::parse_annotation_string(
            "schedule(fetch|analyze|report,delay=5000,parallel=true)"
        )
        .expect("should parse schedule annotation");
        
        match annotation {
            AIAnnotationType::Schedule { steps, delay_ms, parallel } => {
                assert_eq!(steps, vec!["fetch", "analyze", "report"]);
                assert_eq!(delay_ms, Some(5000));
                assert!(parallel);
            }
            _ => panic!("expected Schedule annotation"),
        }
    }

    #[test]
    fn test_validate_annotation_parse() {
        let annotation = AIAnnotation::parse_annotation_string("validate(bounds checking)")
            .expect("should parse validate annotation");
        
        match annotation {
            AIAnnotationType::Validate(constraint) => {
                assert_eq!(constraint, "bounds checking");
            }
            _ => panic!("expected Validate annotation"),
        }
    }

    #[test]
    fn test_ai_hint_creation() {
        let hint = AIHint::new(
            "optimization".to_string(),
            "Use batch processing for large datasets".to_string(),
            0.85,
        )
        .with_improvement(1.25)
        .with_priority(9);
        
        assert_eq!(hint.category, "optimization");
        assert_eq!(hint.confidence, 0.85);
        assert_eq!(hint.improvement, Some(1.25));
        assert_eq!(hint.priority, 9);
    }

    #[test]
    fn test_ai_hint_set() {
        let mut hint_set = AIHintSet::with_function_name("process_data".to_string());
        
        hint_set.add_hint(AIHint::new(
            "performance".to_string(),
            "Cache results".to_string(),
            0.75,
        ).with_priority(7));
        
        hint_set.add_hint(AIHint::new(
            "correctness".to_string(),
            "Add boundary check".to_string(),
            0.95,
        ).with_priority(9));
        
        // Should be sorted by priority
        assert_eq!(hint_set.hints[0].priority, 9);
        assert_eq!(hint_set.hints[1].priority, 7);
    }
}
