/// Phase 6: What-If Analysis Engine
/// Scenario branching and outcome prediction with probability tracking
use std::collections::HashMap;

/// Scenario outcome prediction
#[derive(Clone, Debug)]
pub struct ScenarioOutcome {
    pub scenario_id: u64,
    pub outcome_id: u64,
    pub probability: f64,
    pub impact: f32,  // 0.0 to 1.0 (0=no impact, 1=major)
    pub event_chain: Vec<u64>,
}

impl ScenarioOutcome {
    /// Create new outcome
    pub fn new(scenario_id: u64, outcome_id: u64, probability: f64, impact: f32) -> Self {
        ScenarioOutcome {
            scenario_id,
            outcome_id,
            probability,
            impact,
            event_chain: Vec::new(),
        }
    }
    
    /// Add event to chain
    pub fn add_event(&mut self, event_id: u64) {
        self.event_chain.push(event_id);
    }
    
    /// Get weighted impact (probability * impact)
    pub fn weighted_impact(&self) -> f64 {
        self.probability * (self.impact as f64)
    }
}

/// Scenario branch point
#[derive(Clone, Debug)]
pub struct ScenarioBranch {
    pub branch_id: u64,
    pub decision_point: u64,
    pub alternatives: Vec<(String, f64)>,  // (option, probability)
    pub outcomes: HashMap<u64, ScenarioOutcome>,
}

impl ScenarioBranch {
    /// Create new scenario branch
    pub fn new(branch_id: u64, decision_point: u64) -> Self {
        ScenarioBranch {
            branch_id,
            decision_point,
            alternatives: Vec::new(),
            outcomes: HashMap::new(),
        }
    }
    
    /// Add alternative with probability
    pub fn add_alternative(&mut self, option: String, probability: f64) {
        self.alternatives.push((option, probability.min(1.0)));
    }
    
    /// Add outcome for alternative
    pub fn add_outcome(&mut self, outcome_id: u64, probability: f64, impact: f32) {
        let outcome = ScenarioOutcome::new(self.branch_id, outcome_id, probability, impact);
        self.outcomes.insert(outcome_id, outcome);
    }
    
    /// Get aggregate impact across all outcomes
    pub fn total_impact(&self) -> f64 {
        self.outcomes.values()
            .map(|o| o.weighted_impact())
            .sum()
    }
    
    /// Normalize outcome probabilities
    pub fn normalize_outcomes(&mut self) {
        let total: f64 = self.outcomes.values()
            .map(|o| o.probability)
            .sum();
        
        if total > 0.0 {
            for outcome in self.outcomes.values_mut() {
                outcome.probability /= total;
            }
        }
    }
}

/// Probability tree for outcome prediction
#[derive(Clone, Debug)]
pub struct ProbabilityTree {
    pub tree_id: u64,
    pub root_event: u64,
    pub branches: HashMap<u64, ScenarioBranch>,
    pub leaf_count: u64,
    pub max_depth: u64,
}

impl ProbabilityTree {
    /// Create new probability tree
    pub fn new(tree_id: u64, root_event: u64) -> Self {
        ProbabilityTree {
            tree_id,
            root_event,
            branches: HashMap::new(),
            leaf_count: 0,
            max_depth: 0,
        }
    }
    
    /// Add branch to tree
    pub fn add_branch(&mut self, branch: ScenarioBranch) {
        self.branches.insert(branch.branch_id, branch);
    }
    
    /// Count leaf outcomes
    pub fn count_leaves(&mut self) {
        let mut count = 0u64;
        for branch in self.branches.values() {
            count += branch.outcomes.len() as u64;
        }
        self.leaf_count = count;
    }
    
    /// Get most likely outcome
    pub fn most_likely_outcome(&self) -> Option<(u64, f64)> {
        let mut best = None;
        let mut best_prob = 0.0;
        
        for branch in self.branches.values() {
            for (id, outcome) in &branch.outcomes {
                if outcome.probability > best_prob {
                    best = Some(*id);
                    best_prob = outcome.probability;
                }
            }
        }
        
        best.map(|id| (id, best_prob))
    }
    
    /// Get highest impact outcome
    pub fn max_impact_outcome(&self) -> Option<(u64, f64)> {
        let mut best = None;
        let mut best_impact = 0.0;
        
        for branch in self.branches.values() {
            for (id, outcome) in &branch.outcomes {
                let impact = outcome.weighted_impact();
                if impact > best_impact {
                    best = Some(*id);
                    best_impact = impact;
                }
            }
        }
        
        best.map(|id| (id, best_impact))
    }
}

/// What-If Analysis Engine
pub struct WhatIfAnalysisEngine {
    /// Scenario branches
    branches: HashMap<u64, ScenarioBranch>,
    
    /// Probability trees
    trees: HashMap<u64, ProbabilityTree>,
    
    /// Branch counter
    branch_counter: u64,
    
    /// Tree counter
    tree_counter: u64,
    
    /// Total scenarios analyzed
    scenarios_analyzed: u64,
    
    /// Cache of predictions
    prediction_cache: HashMap<u64, Vec<ScenarioOutcome>>,
}

impl WhatIfAnalysisEngine {
    /// Create new what-if engine
    pub fn new() -> Self {
        WhatIfAnalysisEngine {
            branches: HashMap::new(),
            trees: HashMap::new(),
            branch_counter: 1,
            tree_counter: 1,
            scenarios_analyzed: 0,
            prediction_cache: HashMap::new(),
        }
    }
    
    /// Create scenario branch
    pub fn create_branch(&mut self, decision_point: u64) -> u64 {
        let branch_id = self.branch_counter;
        self.branch_counter += 1;
        
        let branch = ScenarioBranch::new(branch_id, decision_point);
        self.branches.insert(branch_id, branch);
        
        branch_id
    }
    
    /// Create probability tree
    pub fn create_tree(&mut self, root_event: u64) -> u64 {
        let tree_id = self.tree_counter;
        self.tree_counter += 1;
        
        let tree = ProbabilityTree::new(tree_id, root_event);
        self.trees.insert(tree_id, tree);
        
        tree_id
    }
    
    /// Add branch to tree
    pub fn add_branch_to_tree(&mut self, tree_id: u64, branch_id: u64) -> bool {
        if let Some(branch) = self.branches.get(&branch_id).cloned() {
            if let Some(tree) = self.trees.get_mut(&tree_id) {
                tree.add_branch(branch);
                return true;
            }
        }
        false
    }
    
    /// Add alternative to branch
    pub fn add_alternative(&mut self, branch_id: u64, option: String, probability: f64) -> bool {
        if let Some(branch) = self.branches.get_mut(&branch_id) {
            branch.add_alternative(option, probability);
            true
        } else {
            false
        }
    }
    
    /// Add outcome to branch
    pub fn add_outcome(&mut self, branch_id: u64, outcome_id: u64, probability: f64, impact: f32) -> bool {
        if let Some(branch) = self.branches.get_mut(&branch_id) {
            branch.add_outcome(outcome_id, probability, impact);
            true
        } else {
            false
        }
    }
    
    /// Predict outcomes for scenario
    pub fn predict_outcomes(&mut self, branch_id: u64) -> Vec<ScenarioOutcome> {
        if let Some(cached) = self.prediction_cache.get(&branch_id) {
            return cached.clone();
        }
        
        let outcomes = if let Some(branch) = self.branches.get(&branch_id) {
            branch.outcomes.values().cloned().collect()
        } else {
            Vec::new()
        };
        
        self.prediction_cache.insert(branch_id, outcomes.clone());
        self.scenarios_analyzed += 1;
        
        outcomes
    }
    
    /// Get most likely path through tree
    pub fn most_likely_path(&self, tree_id: u64) -> Option<Vec<u64>> {
        if let Some(tree) = self.trees.get(&tree_id) {
            let mut path = vec![tree.root_event];
            
            if let Some((outcome_id, _)) = tree.most_likely_outcome() {
                path.push(outcome_id);
            }
            
            Some(path)
        } else {
            None
        }
    }
    
    /// Get high-impact scenario
    pub fn high_impact_scenario(&self, tree_id: u64) -> Option<Vec<u64>> {
        if let Some(tree) = self.trees.get(&tree_id) {
            let mut path = vec![tree.root_event];
            
            if let Some((outcome_id, _)) = tree.max_impact_outcome() {
                path.push(outcome_id);
            }
            
            Some(path)
        } else {
            None
        }
    }
    
    /// Calculate expected value
    pub fn expected_value(&self, branch_id: u64) -> f64 {
        if let Some(branch) = self.branches.get(&branch_id) {
            branch.outcomes.values()
                .map(|o| o.probability * (o.impact as f64))
                .sum()
        } else {
            0.0
        }
    }
    
    /// Risk assessment for branch
    pub fn risk_assessment(&self, branch_id: u64) -> f32 {
        if let Some(branch) = self.branches.get(&branch_id) {
            // Risk = sum of (probability * (1 - impact)) for negative outcomes
            let mut risk = 0.0f32;
            
            for outcome in branch.outcomes.values() {
                if outcome.impact < 0.5 {
                    risk += outcome.probability as f32 * (1.0 - outcome.impact);
                }
            }
            
            risk
        } else {
            0.0
        }
    }
    
    /// Get branch count
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }
    
    /// Get tree count
    pub fn tree_count(&self) -> usize {
        self.trees.len()
    }
    
    /// Get total scenarios analyzed
    pub fn scenarios_count(&self) -> u64 {
        self.scenarios_analyzed
    }
    
    /// Clear prediction cache
    pub fn clear_cache(&mut self) {
        self.prediction_cache.clear();
    }
}

impl Clone for WhatIfAnalysisEngine {
    fn clone(&self) -> Self {
        WhatIfAnalysisEngine {
            branches: self.branches.clone(),
            trees: self.trees.clone(),
            branch_counter: self.branch_counter,
            tree_counter: self.tree_counter,
            scenarios_analyzed: self.scenarios_analyzed,
            prediction_cache: self.prediction_cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_outcome_creation() {
        let outcome = ScenarioOutcome::new(1, 1, 0.8, 0.7);
        assert_eq!(outcome.probability, 0.8);
        assert_eq!(outcome.impact, 0.7);
    }
    
    #[test]
    fn test_weighted_impact() {
        let outcome = ScenarioOutcome::new(1, 1, 0.8, 0.5);
        let weighted = outcome.weighted_impact();
        assert!((weighted - 0.4).abs() < 0.001);
    }
    
    #[test]
    fn test_branch_creation() {
        let branch = ScenarioBranch::new(1, 100);
        assert_eq!(branch.branch_id, 1);
        assert_eq!(branch.decision_point, 100);
    }
    
    #[test]
    fn test_add_alternative() {
        let mut branch = ScenarioBranch::new(1, 100);
        branch.add_alternative("Option A".to_string(), 0.6);
        branch.add_alternative("Option B".to_string(), 0.4);
        
        assert_eq!(branch.alternatives.len(), 2);
    }
    
    #[test]
    fn test_branch_total_impact() {
        let mut branch = ScenarioBranch::new(1, 100);
        branch.add_outcome(1, 0.5, 0.8);
        branch.add_outcome(2, 0.5, 0.6);
        
        let impact = branch.total_impact();
        assert!(impact > 0.0);
    }
    
    #[test]
    fn test_probability_tree_creation() {
        let tree = ProbabilityTree::new(1, 1000);
        assert_eq!(tree.tree_id, 1);
        assert_eq!(tree.root_event, 1000);
    }
    
    #[test]
    fn test_most_likely_outcome() {
        let mut tree = ProbabilityTree::new(1, 1000);
        let mut branch = ScenarioBranch::new(1, 1000);
        
        branch.add_outcome(1, 0.8, 0.5);
        branch.add_outcome(2, 0.2, 0.7);
        
        tree.add_branch(branch);
        
        let outcome = tree.most_likely_outcome();
        assert!(outcome.is_some());
    }
    
    #[test]
    fn test_engine_creation() {
        let engine = WhatIfAnalysisEngine::new();
        assert_eq!(engine.branch_count(), 0);
    }
    
    #[test]
    fn test_create_branch() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        
        assert_eq!(engine.branch_count(), 1);
        assert!(branch_id > 0);
    }
    
    #[test]
    fn test_create_tree() {
        let mut engine = WhatIfAnalysisEngine::new();
        let tree_id = engine.create_tree(1000);
        
        assert_eq!(engine.tree_count(), 1);
        assert!(tree_id > 0);
    }
    
    #[test]
    fn test_engine_add_alternative() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        
        assert!(engine.add_alternative(branch_id, "Option A".to_string(), 0.6));
    }
    
    #[test]
    fn test_add_outcome() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        
        assert!(engine.add_outcome(branch_id, 1, 0.8, 0.7));
    }
    
    #[test]
    fn test_predict_outcomes() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        engine.add_outcome(branch_id, 1, 0.8, 0.7);
        
        let outcomes = engine.predict_outcomes(branch_id);
        assert_eq!(outcomes.len(), 1);
    }
    
    #[test]
    fn test_expected_value() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        engine.add_outcome(branch_id, 1, 0.8, 0.5);
        
        let ev = engine.expected_value(branch_id);
        assert!(ev > 0.0);
    }
    
    #[test]
    fn test_risk_assessment() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        engine.add_outcome(branch_id, 1, 0.5, 0.3);  // Low impact
        
        let risk = engine.risk_assessment(branch_id);
        assert!(risk >= 0.0);
    }
    
    #[test]
    fn test_most_likely_path() {
        let mut engine = WhatIfAnalysisEngine::new();
        let tree_id = engine.create_tree(1000);
        let branch_id = engine.create_branch(1000);
        
        engine.add_branch_to_tree(tree_id, branch_id);
        engine.add_outcome(branch_id, 2000, 0.9, 0.8);
        
        let path = engine.most_likely_path(tree_id);
        assert!(path.is_some());
    }
    
    #[test]
    fn test_cache_predictions() {
        let mut engine = WhatIfAnalysisEngine::new();
        let branch_id = engine.create_branch(100);
        engine.add_outcome(branch_id, 1, 0.8, 0.7);
        
        let _results1 = engine.predict_outcomes(branch_id);
        let count1 = engine.scenarios_count();
        
        let _results2 = engine.predict_outcomes(branch_id);
        let count2 = engine.scenarios_count();
        
        // Cached, so count should increment only once
        assert_eq!(count1, count2);
    }
}
