//! KILLER V2.2 PHASE 1 - FEATURE MARKERS
//! This module contains markers and structures for Phase 1 features
//! 
//! Features:
//! 1. Dependent Types
//! 2. Effect System
//! 3. Async/Await
//! 4. Contract Programming

/// Dependent type parameter: Vector[n: nat]
#[derive(Debug, Clone, PartialEq)]
pub struct DependentTypeParam {
    pub name: String,           // "n", "m"
    pub constraint: String,     // "nat", "positive"
}

/// Effect in a function signature: uses io, random
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub name: String,           // "io", "random", "allocate"
}

/// Contract clause: requires, ensures, invariant
#[derive(Debug, Clone, PartialEq)]
pub enum ContractClause {
    Requires(String),           // Precondition
    Ensures(String),            // Postcondition
    Invariant(String),          // Loop/struct invariant
}

/// Phase 1 metadata for functions
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Phase1Metadata {
    pub is_pure: bool,
    pub is_async: bool,
    pub effects: Vec<Effect>,
    pub contracts: Vec<ContractClause>,
    pub dependent_params: Vec<DependentTypeParam>,
}

impl Phase1Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pure(mut self) -> Self {
        self.is_pure = true;
        self
    }

    pub fn with_async(mut self) -> Self {
        self.is_async = true;
        self
    }

    pub fn add_effect(mut self, name: &str) -> Self {
        self.effects.push(Effect {
            name: name.to_string(),
        });
        self
    }

    pub fn add_dependent_param(mut self, name: &str, constraint: &str) -> Self {
        self.dependent_params.push(DependentTypeParam {
            name: name.to_string(),
            constraint: constraint.to_string(),
        });
        self
    }

    pub fn add_contract(mut self, clause: ContractClause) -> Self {
        self.contracts.push(clause);
        self
    }
}

/// Phase 1 Feature Flags
#[derive(Debug, Clone)]
pub struct Phase1Features {
    pub dependent_types_enabled: bool,
    pub effects_enabled: bool,
    pub async_await_enabled: bool,
    pub contracts_enabled: bool,
}

impl Default for Phase1Features {
    fn default() -> Self {
        Self {
            dependent_types_enabled: true,
            effects_enabled: true,
            async_await_enabled: true,
            contracts_enabled: true,
        }
    }
}

impl Phase1Features {
    /// Check if all Phase 1 features are enabled
    pub fn all_enabled(&self) -> bool {
        self.dependent_types_enabled
            && self.effects_enabled
            && self.async_await_enabled
            && self.contracts_enabled
    }

    /// Get count of enabled features
    pub fn enabled_count(&self) -> usize {
        [
            self.dependent_types_enabled,
            self.effects_enabled,
            self.async_await_enabled,
            self.contracts_enabled,
        ]
        .iter()
        .filter(|&&x| x)
        .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase1_metadata() {
        let mut metadata = Phase1Metadata::new();
        assert!(!metadata.is_pure);
        assert!(!metadata.is_async);

        metadata = metadata.with_pure().with_async();
        assert!(metadata.is_pure);
        assert!(metadata.is_async);
    }

    #[test]
    fn test_phase1_features() {
        let features = Phase1Features::default();
        assert!(features.all_enabled());
        assert_eq!(features.enabled_count(), 4);
    }

    #[test]
    fn test_dependent_type_param() {
        let param = DependentTypeParam {
            name: "n".to_string(),
            constraint: "nat".to_string(),
        };
        assert_eq!(param.name, "n");
        assert_eq!(param.constraint, "nat");
    }
}
