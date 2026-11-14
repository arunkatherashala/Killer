// Phase 3.1: Type Inference Engine
// Analyzes bytecode to determine likely types at compile time
// Enables type-specialized code generation

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InferredType {
    I64,
    F64,
    Bool,
    String,
    Array(Box<InferredType>),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TypeAssumption {
    pub var_name: String,
    pub inferred_type: InferredType,
    pub confidence: f32, // 0.0-1.0
    pub observation_count: u32,
}

#[derive(Debug)]
pub struct TypeInferenceEngine {
    /// Type assumptions per variable
    assumptions: HashMap<String, TypeAssumption>,
    /// Type history for each variable
    type_history: HashMap<String, Vec<InferredType>>,
    /// Observed operation types for specialization
    operation_types: HashMap<String, Vec<InferredType>>, // (var_name_op -> types tried)
    /// Total type observations
    total_observations: u32,
    /// Confidence threshold (default 0.8 = 80%)
    confidence_threshold: f32,
}

impl TypeInferenceEngine {
    pub fn new(confidence_threshold: f32) -> Self {
        TypeInferenceEngine {
            assumptions: HashMap::new(),
            type_history: HashMap::new(),
            operation_types: HashMap::new(),
            total_observations: 0,
            confidence_threshold,
        }
    }

    /// Record observed type for variable
    pub fn observe_type(&mut self, var_name: String, inferred_type: InferredType) {
        self.total_observations += 1;

        // Update type history
        self.type_history
            .entry(var_name.clone())
            .or_insert_with(Vec::new)
            .push(inferred_type.clone());

        // Update assumption
        let assumption = self
            .assumptions
            .entry(var_name.clone())
            .or_insert_with(|| TypeAssumption {
                var_name,
                inferred_type: InferredType::Unknown,
                confidence: 0.0,
                observation_count: 0,
            });

        assumption.inferred_type = inferred_type;
        assumption.observation_count += 1;

        // Calculate confidence: observation_count / total_observations
        let empty_vec = Vec::new();
        let history = &self.type_history.get(&assumption.var_name).unwrap_or(&empty_vec);
        let same_type_count = history.iter().filter(|t| *t == &assumption.inferred_type).count() as u32;
        assumption.confidence = same_type_count as f32 / assumption.observation_count.max(1) as f32;
    }

    /// Get inferred type for variable
    pub fn get_inferred_type(&self, var_name: &str) -> Option<TypeAssumption> {
        self.assumptions.get(var_name).cloned()
    }

    /// Check if type is confident enough for specialization
    pub fn can_specialize(&self, var_name: &str) -> bool {
        if let Some(assumption) = self.assumptions.get(var_name) {
            assumption.confidence >= self.confidence_threshold
                && assumption.inferred_type != InferredType::Unknown
        } else {
            false
        }
    }

    /// Get all variables suitable for specialization
    pub fn get_specializable_vars(&self) -> Vec<TypeAssumption> {
        self.assumptions
            .values()
            .filter(|a| self.can_specialize(&a.var_name))
            .cloned()
            .collect()
    }

    /// Get type distribution for variable
    pub fn get_type_distribution(&self, var_name: &str) -> HashMap<InferredType, usize> {
        let mut distribution = HashMap::new();

        if let Some(history) = self.type_history.get(var_name) {
            for type_observed in history {
                *distribution.entry(type_observed.clone()).or_insert(0) += 1;
            }
        }

        distribution
    }

    /// Reset inference for new specialization round
    pub fn reset(&mut self) {
        self.assumptions.clear();
        self.type_history.clear();
        self.operation_types.clear();
        self.total_observations = 0;
    }

    /// Get inference statistics
    pub fn get_statistics(&self) -> TypeInferenceStats {
        let specializable = self.get_specializable_vars();
        let polymorphic: Vec<_> = self
            .assumptions
            .values()
            .filter(|a| !self.can_specialize(&a.var_name))
            .collect();

        TypeInferenceStats {
            total_variables: self.assumptions.len(),
            specializable_variables: specializable.len(),
            polymorphic_variables: polymorphic.len(),
            total_observations: self.total_observations,
            average_observations_per_var: if self.assumptions.is_empty() {
                0
            } else {
                self.total_observations / self.assumptions.len() as u32
            },
            confidence_threshold: self.confidence_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeInferenceStats {
    pub total_variables: usize,
    pub specializable_variables: usize,
    pub polymorphic_variables: usize,
    pub total_observations: u32,
    pub average_observations_per_var: u32,
    pub confidence_threshold: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = TypeInferenceEngine::new(0.8);
        assert_eq!(engine.total_observations, 0);
    }

    #[test]
    fn test_observe_type() {
        let mut engine = TypeInferenceEngine::new(0.8);

        engine.observe_type("x".to_string(), InferredType::I64);
        assert_eq!(engine.total_observations, 1);
    }

    #[test]
    fn test_confidence_calculation() {
        let mut engine = TypeInferenceEngine::new(0.8);

        // Observe I64 5 times
        for _ in 0..5 {
            engine.observe_type("x".to_string(), InferredType::I64);
        }

        // Observe F64 once
        engine.observe_type("x".to_string(), InferredType::F64);

        let assumption = engine.get_inferred_type("x").unwrap();
        // Should be mostly I64 with some confidence
        assert!(assumption.observation_count >= 5);
    }

    #[test]
    fn test_specialization_threshold() {
        let mut engine = TypeInferenceEngine::new(0.9); // High threshold

        // Observe with 80% confidence
        for _ in 0..4 {
            engine.observe_type("x".to_string(), InferredType::I64);
        }
        engine.observe_type("x".to_string(), InferredType::F64);

        // Should not specialize (80% < 90% threshold)
        assert!(!engine.can_specialize("x"));
    }

    #[test]
    fn test_get_specializable_vars() {
        let mut engine = TypeInferenceEngine::new(0.8);

        // x: I64 with high confidence
        for _ in 0..8 {
            engine.observe_type("x".to_string(), InferredType::I64);
        }
        engine.observe_type("x".to_string(), InferredType::F64);

        // y: Unknown (polymorphic)
        engine.observe_type("y".to_string(), InferredType::I64);
        engine.observe_type("y".to_string(), InferredType::F64);

        let specializable = engine.get_specializable_vars();
        assert!(specializable.iter().any(|a| a.var_name == "x"));
        assert!(!specializable.iter().any(|a| a.var_name == "y"));
    }

    #[test]
    fn test_type_distribution() {
        let mut engine = TypeInferenceEngine::new(0.8);

        for _ in 0..6 {
            engine.observe_type("x".to_string(), InferredType::I64);
        }
        for _ in 0..3 {
            engine.observe_type("x".to_string(), InferredType::F64);
        }

        let dist = engine.get_type_distribution("x");
        assert_eq!(dist.get(&InferredType::I64), Some(&6));
        assert_eq!(dist.get(&InferredType::F64), Some(&3));
    }

    #[test]
    fn test_reset() {
        let mut engine = TypeInferenceEngine::new(0.8);
        engine.observe_type("x".to_string(), InferredType::I64);

        assert!(engine.total_observations > 0);

        engine.reset();
        assert_eq!(engine.total_observations, 0);
        assert_eq!(engine.assumptions.len(), 0);
    }

    #[test]
    fn test_statistics() {
        let mut engine = TypeInferenceEngine::new(0.8);

        for _ in 0..5 {
            engine.observe_type("x".to_string(), InferredType::I64);
        }
        for _ in 0..5 {
            engine.observe_type("y".to_string(), InferredType::I64);
        }

        let stats = engine.get_statistics();
        assert_eq!(stats.total_variables, 2);
        assert_eq!(stats.total_observations, 10);
    }
}
