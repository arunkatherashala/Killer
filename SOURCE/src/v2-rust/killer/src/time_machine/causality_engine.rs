/// Phase 2: Causality Engine
/// Forward/backward/parallel event tracing with complete paradox prevention
use std::collections::{HashMap, HashSet, VecDeque};

/// Direction for tracing through the causality graph
#[derive(Clone, Debug, PartialEq)]
pub enum TraceDirection {
    Forward,    // What events does this cause?
    Backward,   // What events caused this?
    Both,       // Both forward and backward
}

/// Trace result containing event IDs in causal order
#[derive(Clone, Debug)]
pub struct CausalityTrace {
    pub direction: TraceDirection,
    pub root_event: u64,
    pub events_in_order: Vec<u64>,
    pub event_count: usize,
    pub trace_depth: usize,
    pub branches: usize,
}

/// Timeline branching information
#[derive(Clone, Debug)]
pub struct TimelineBranch {
    pub branch_id: u64,
    pub origin_event: u64,
    pub branch_point: u128,  // timestamp where branch diverges
    pub events: Vec<u64>,
    pub divergence_events: HashSet<u64>,  // Events that differ from original
}

/// Parallel timeline tracker for what-if scenarios
#[derive(Clone, Debug)]
pub struct Timeline {
    pub timeline_id: u64,
    pub description: String,
    pub events: Vec<u64>,
    pub branch_from: Option<u64>,
    pub is_primary: bool,
}

/// Causality Engine for advanced tracing
pub struct CausalityEngine {
    /// Forward edges: event → consequences
    forward_edges: HashMap<u64, Vec<u64>>,
    
    /// Backward edges: event → causes
    backward_edges: HashMap<u64, Vec<u64>>,
    
    /// Timeline branches for parallel exploration
    timelines: HashMap<u64, Timeline>,
    timeline_counter: u64,
    
    /// Traces cache for performance
    trace_cache: HashMap<u64, CausalityTrace>,
    
    /// Paradox detection records
    paradoxes_detected: Vec<(u64, u64, String)>,
}

impl CausalityEngine {
    /// Create a new causality engine
    pub fn new() -> Self {
        CausalityEngine {
            forward_edges: HashMap::new(),
            backward_edges: HashMap::new(),
            timelines: HashMap::new(),
            timeline_counter: 1,
            trace_cache: HashMap::new(),
            paradoxes_detected: Vec::new(),
        }
    }
    
    /// Link events in causality graph
    pub fn link_events(&mut self, cause: u64, consequence: u64) -> Result<(), String> {
        // Check for paradoxes
        if self.would_create_paradox(cause, consequence) {
            let msg = format!("Paradox detected: {} → {} would create cycle", cause, consequence);
            self.paradoxes_detected.push((cause, consequence, msg.clone()));
            return Err(msg);
        }
        
        // Add forward edge
        self.forward_edges
            .entry(cause)
            .or_insert_with(Vec::new)
            .push(consequence);
        
        // Add backward edge
        self.backward_edges
            .entry(consequence)
            .or_insert_with(Vec::new)
            .push(cause);
        
        // Invalidate cache
        self.trace_cache.clear();
        
        Ok(())
    }
    
    /// Check if linking would create a paradox
    fn would_create_paradox(&self, cause: u64, consequence: u64) -> bool {
        // If consequence can already reach cause, linking would create cycle
        self.can_reach(consequence, cause)
    }
    
    /// Check if source can reach target
    fn can_reach(&self, source: u64, target: u64) -> bool {
        if source == target {
            return true;
        }
        
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        
        queue.push_back(source);
        visited.insert(source);
        
        while let Some(current) = queue.pop_front() {
            if let Some(next_nodes) = self.forward_edges.get(&current) {
                for &next in next_nodes {
                    if next == target {
                        return true;
                    }
                    if !visited.contains(&next) {
                        visited.insert(next);
                        queue.push_back(next);
                    }
                }
            }
        }
        
        false
    }
    
    /// Trace forward: what events does this event cause?
    pub fn trace_forward(&mut self, event_id: u64, max_depth: usize) -> CausalityTrace {
        if let Some(cached) = self.trace_cache.get(&event_id) {
            return cached.clone();
        }
        
        let mut trace = CausalityTrace {
            direction: TraceDirection::Forward,
            root_event: event_id,
            events_in_order: Vec::new(),
            event_count: 0,
            trace_depth: 0,
            branches: 0,
        };
        
        self.trace_forward_recursive(event_id, &mut trace.events_in_order, 0, max_depth);
        trace.event_count = trace.events_in_order.len();
        trace.trace_depth = max_depth;
        
        // Count branches including root event
        let mut branch_count = 0;
        if let Some(consequences) = self.forward_edges.get(&event_id) {
            if consequences.len() > 1 {
                branch_count = 1;  // Root event is a branch point
            }
        }
        branch_count += trace.events_in_order.iter()
            .filter(|e| {
                self.forward_edges.get(e).map_or(0, |v| v.len()) > 1
            })
            .count();
        trace.branches = branch_count;
        
        self.trace_cache.insert(event_id, trace.clone());
        trace
    }
    
    /// Recursive forward tracing
    fn trace_forward_recursive(&self, event_id: u64, result: &mut Vec<u64>, depth: usize, max_depth: usize) {
        if depth >= max_depth {
            return;
        }
        
        if let Some(consequences) = self.forward_edges.get(&event_id) {
            for &consequence in consequences {
                result.push(consequence);
                self.trace_forward_recursive(consequence, result, depth + 1, max_depth);
            }
        }
    }
    
    /// Trace backward: what events caused this event?
    pub fn trace_backward(&mut self, event_id: u64, max_depth: usize) -> CausalityTrace {
        let backward_key = event_id | (1u64 << 63); // Unique key for backward traces
        
        if let Some(cached) = self.trace_cache.get(&backward_key) {
            return cached.clone();
        }
        
        let mut trace = CausalityTrace {
            direction: TraceDirection::Backward,
            root_event: event_id,
            events_in_order: Vec::new(),
            event_count: 0,
            trace_depth: 0,
            branches: 0,
        };
        
        self.trace_backward_recursive(event_id, &mut trace.events_in_order, 0, max_depth);
        trace.event_count = trace.events_in_order.len();
        trace.trace_depth = max_depth;
        
        // Count branches including root event
        let mut branch_count = 0;
        if let Some(causes) = self.backward_edges.get(&event_id) {
            if causes.len() > 1 {
                branch_count = 1;  // Root event has multiple causes (merge point)
            }
        }
        branch_count += trace.events_in_order.iter()
            .filter(|e| {
                self.backward_edges.get(e).map_or(0, |v| v.len()) > 1
            })
            .count();
        trace.branches = branch_count;
        
        self.trace_cache.insert(backward_key, trace.clone());
        trace
    }
    
    /// Recursive backward tracing
    fn trace_backward_recursive(&self, event_id: u64, result: &mut Vec<u64>, depth: usize, max_depth: usize) {
        if depth >= max_depth {
            return;
        }
        
        if let Some(causes) = self.backward_edges.get(&event_id) {
            for &cause in causes {
                result.push(cause);
                self.trace_backward_recursive(cause, result, depth + 1, max_depth);
            }
        }
    }
    
    /// Trace both directions
    pub fn trace_both(&mut self, event_id: u64, max_depth: usize) -> (CausalityTrace, CausalityTrace) {
        let forward = self.trace_forward(event_id, max_depth);
        let backward = self.trace_backward(event_id, max_depth);
        (forward, backward)
    }
    
    /// Create a parallel timeline (what-if scenario)
    pub fn create_timeline(&mut self, description: String, branch_from: Option<u64>) -> u64 {
        let timeline_id = self.timeline_counter;
        self.timeline_counter += 1;
        
        let timeline = Timeline {
            timeline_id,
            description,
            events: Vec::new(),
            branch_from,
            is_primary: self.timelines.is_empty(),
        };
        
        self.timelines.insert(timeline_id, timeline);
        timeline_id
    }
    
    /// Add event to timeline
    pub fn add_to_timeline(&mut self, timeline_id: u64, event_id: u64) -> Result<(), String> {
        if let Some(timeline) = self.timelines.get_mut(&timeline_id) {
            timeline.events.push(event_id);
            Ok(())
        } else {
            Err(format!("Timeline {} not found", timeline_id))
        }
    }
    
    /// Get divergence points between timelines
    pub fn find_divergence(&self, timeline_a: u64, timeline_b: u64) -> Vec<u64> {
        let events_a: HashSet<u64> = self.timelines
            .get(&timeline_a)
            .map(|t| t.events.iter().cloned().collect())
            .unwrap_or_default();
        
        let events_b: HashSet<u64> = self.timelines
            .get(&timeline_b)
            .map(|t| t.events.iter().cloned().collect())
            .unwrap_or_default();
        
        events_a.symmetric_difference(&events_b)
            .cloned()
            .collect()
    }
    
    /// Count branches in trace
    #[allow(dead_code)]
    fn count_branches(&self, events: &[u64]) -> usize {
        events.iter()
            .filter(|e| {
                self.forward_edges.get(e).map_or(0, |v| v.len()) > 1
            })
            .count()
    }
    
    /// Get all paradoxes detected
    pub fn paradoxes(&self) -> &[(u64, u64, String)] {
        &self.paradoxes_detected
    }
    
    /// Check if paradox-free
    pub fn is_paradox_free(&self) -> bool {
        self.paradoxes_detected.is_empty()
    }
    
    /// Get total events in graph
    pub fn event_count(&self) -> usize {
        self.forward_edges.len()
    }
    
    /// Get total timelines
    pub fn timeline_count(&self) -> usize {
        self.timelines.len()
    }
}

impl Default for CausalityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CausalityEngine {
    fn clone(&self) -> Self {
        CausalityEngine {
            forward_edges: self.forward_edges.clone(),
            backward_edges: self.backward_edges.clone(),
            timelines: self.timelines.clone(),
            timeline_counter: self.timeline_counter,
            trace_cache: self.trace_cache.clone(),
            paradoxes_detected: self.paradoxes_detected.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_causality_engine_creation() {
        let engine = CausalityEngine::new();
        assert_eq!(engine.event_count(), 0);
        assert!(engine.is_paradox_free());
    }
    
    #[test]
    fn test_link_events() {
        let mut engine = CausalityEngine::new();
        let result = engine.link_events(1, 2);
        assert!(result.is_ok());
        assert_eq!(engine.event_count(), 1);
    }
    
    #[test]
    fn test_forward_trace() {
        let mut engine = CausalityEngine::new();
        engine.link_events(1, 2).unwrap();
        engine.link_events(2, 3).unwrap();
        engine.link_events(2, 4).unwrap();
        
        let trace = engine.trace_forward(1, 10);
        assert_eq!(trace.events_in_order.len(), 3);
        assert!(trace.events_in_order.contains(&2));
        assert!(trace.events_in_order.contains(&3));
        assert!(trace.events_in_order.contains(&4));
    }
    
    #[test]
    fn test_backward_trace() {
        let mut engine = CausalityEngine::new();
        engine.link_events(1, 2).unwrap();
        engine.link_events(2, 3).unwrap();
        
        let trace = engine.trace_backward(3, 10);
        assert_eq!(trace.events_in_order.len(), 2);
        assert!(trace.events_in_order.contains(&2));
        assert!(trace.events_in_order.contains(&1));
    }
    
    #[test]
    fn test_paradox_detection() {
        let mut engine = CausalityEngine::new();
        engine.link_events(1, 2).unwrap();
        engine.link_events(2, 3).unwrap();
        
        // Try to create cycle
        let result = engine.link_events(3, 1);
        assert!(result.is_err());
        assert!(!engine.is_paradox_free());
    }
    
    #[test]
    fn test_timeline_creation() {
        let mut engine = CausalityEngine::new();
        let timeline_id = engine.create_timeline("What-if 1".to_string(), None);
        assert!(timeline_id > 0);
        assert_eq!(engine.timeline_count(), 1);
    }
    
    #[test]
    fn test_add_to_timeline() {
        let mut engine = CausalityEngine::new();
        let timeline_id = engine.create_timeline("Test".to_string(), None);
        
        let result = engine.add_to_timeline(timeline_id, 1);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_find_divergence() {
        let mut engine = CausalityEngine::new();
        let t1 = engine.create_timeline("Timeline 1".to_string(), None);
        let t2 = engine.create_timeline("Timeline 2".to_string(), Some(t1));
        
        engine.add_to_timeline(t1, 1).unwrap();
        engine.add_to_timeline(t1, 2).unwrap();
        engine.add_to_timeline(t2, 1).unwrap();
        engine.add_to_timeline(t2, 3).unwrap();
        
        let divergence = engine.find_divergence(t1, t2);
        assert!(divergence.contains(&2));
        assert!(divergence.contains(&3));
    }
    
    #[test]
    fn test_branch_counting() {
        let mut engine = CausalityEngine::new();
        engine.link_events(1, 2).unwrap();
        engine.link_events(1, 3).unwrap();
        engine.link_events(2, 4).unwrap();
        
        let trace = engine.trace_forward(1, 10);
        assert!(trace.branches > 0);
    }
    
    #[test]
    fn test_trace_cache() {
        let mut engine = CausalityEngine::new();
        engine.link_events(1, 2).unwrap();
        
        // First trace populates cache
        let trace1 = engine.trace_forward(1, 10);
        
        // Second trace uses cache
        let trace2 = engine.trace_forward(1, 10);
        
        assert_eq!(trace1.events_in_order, trace2.events_in_order);
    }
    
    #[test]
    fn test_wide_branching() {
        let mut engine = CausalityEngine::new();
        
        // Create wide branching tree
        for i in 2..11 {
            engine.link_events(1, i as u64).unwrap();
        }
        
        let trace = engine.trace_forward(1, 10);
        assert_eq!(trace.events_in_order.len(), 9);
        assert_eq!(trace.branches, 1); // Only 1 branchpoint
    }
    
    #[test]
    fn test_deep_chain() {
        let mut engine = CausalityEngine::new();
        
        // Create deep chain
        for i in 1..100 {
            engine.link_events(i as u64, (i + 1) as u64).unwrap();
        }
        
        let trace = engine.trace_forward(1, 1000);
        assert_eq!(trace.events_in_order.len(), 99);
    }
}
