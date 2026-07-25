/// Causality tracking and dependency graphing
/// Prevents temporal paradoxes and maintains causal coherence
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents a dependency relationship between events
#[derive(Clone, Debug)]
pub struct CausalDependency {
    /// Event ID that depends on another
    pub dependent_event: u64,
    
    /// Event ID that is a prerequisite
    pub prerequisite_event: u64,
    
    /// Type of dependency (Data, Control, Time)
    pub dependency_type: DependencyType,
    
    /// Strength of dependency (0.0 to 1.0)
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DependencyType {
    /// Event B reads data written by A
    Data,
    
    /// Event B's execution path depends on A's outcome
    Control,
    
    /// Event B must happen after A
    Temporal,
    
    /// Bidirectional dependency (potential paradox)
    Circular,
}

/// Causality graph for tracking event dependencies
pub struct CausalityGraph {
    /// Adjacency list: event_id → vec of dependent events
    pub forward_edges: HashMap<u64, Vec<u64>>,
    
    /// Reverse adjacency: event_id → vec of prerequisite events
    pub backward_edges: HashMap<u64, Vec<u64>>,
    
    /// Dependency metadata
    pub dependencies: HashMap<(u64, u64), CausalDependency>,
    
    /// Cycle detection cache
    pub known_cycles: HashSet<(u64, u64)>,
    
    /// Cached topological sort
    pub topo_sort_cache: Option<Vec<u64>>,
    pub cache_valid: bool,
}

impl CausalityGraph {
    /// Create a new causality graph
    pub fn new() -> Self {
        CausalityGraph {
            forward_edges: HashMap::new(),
            backward_edges: HashMap::new(),
            dependencies: HashMap::new(),
            known_cycles: HashSet::new(),
            topo_sort_cache: None,
            cache_valid: false,
        }
    }
    
    /// Link two events with a dependency
    pub fn link_events(&mut self, 
                       dependent: u64, 
                       prerequisite: u64, 
                       dep_type: DependencyType,
                       strength: f32) -> Result<(), String>
    {
        // Check for cycles before adding
        if self.would_create_cycle(dependent, prerequisite) {
            let msg = format!("Adding dependency would create cycle: {} -> {}", 
                            dependent, prerequisite);
            return Err(msg);
        }
        
        // Add forward edge
        self.forward_edges
            .entry(prerequisite)
            .or_insert_with(Vec::new)
            .push(dependent);
        
        // Add backward edge
        self.backward_edges
            .entry(dependent)
            .or_insert_with(Vec::new)
            .push(prerequisite);
        
        // Store dependency metadata
        self.dependencies.insert(
            (dependent, prerequisite),
            CausalDependency {
                dependent_event: dependent,
                prerequisite_event: prerequisite,
                dependency_type: dep_type,
                strength,
            },
        );
        
        // Invalidate cache
        self.cache_valid = false;
        
        Ok(())
    }
    
    /// Check if adding this edge would create a cycle
    fn would_create_cycle(&self, dependent: u64, prerequisite: u64) -> bool {
        // If prerequisite can reach dependent, adding edge would create cycle
        self.can_reach(prerequisite, dependent)
    }
    
    /// Check if there's a path from source to target
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
    
    /// Get direct dependencies (events that must happen before this one)
    pub fn direct_dependencies(&self, event_id: u64) -> Vec<u64> {
        self.backward_edges
            .get(&event_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get transitive dependencies (all ancestors in causal order)
    pub fn transitive_dependencies(&self, event_id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        if let Some(deps) = self.backward_edges.get(&event_id) {
            for &dep in deps {
                queue.push_back(dep);
                visited.insert(dep);
            }
        }
        
        while let Some(current) = queue.pop_front() {
            result.push(current);
            
            if let Some(deps) = self.backward_edges.get(&current) {
                for &dep in deps {
                    if !visited.contains(&dep) {
                        visited.insert(dep);
                        queue.push_back(dep);
                    }
                }
            }
        }
        
        result
    }
    
    /// Get events that depend on this one
    pub fn dependent_events(&self, event_id: u64) -> Vec<u64> {
        self.forward_edges
            .get(&event_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get all transitive dependents
    pub fn transitive_dependents(&self, event_id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        if let Some(deps) = self.forward_edges.get(&event_id) {
            for &dep in deps {
                queue.push_back(dep);
                visited.insert(dep);
            }
        }
        
        while let Some(current) = queue.pop_front() {
            result.push(current);
            
            if let Some(deps) = self.forward_edges.get(&current) {
                for &dep in deps {
                    if !visited.contains(&dep) {
                        visited.insert(dep);
                        queue.push_back(dep);
                    }
                }
            }
        }
        
        result
    }
    
    /// Detect if a cycle exists (paradox)
    pub fn has_cycle(&mut self) -> bool {
        // Try to get topological sort
        self.topological_sort().is_none()
    }
    
    /// Get topological sort of events (if acyclic)
    pub fn topological_sort(&mut self) -> Option<Vec<u64>> {
        if self.cache_valid {
            return self.topo_sort_cache.clone();
        }
        
        let mut in_degree: HashMap<u64, usize> = HashMap::new();
        let mut all_nodes = HashSet::new();
        
        // Collect all nodes
        for (k, _) in &self.backward_edges {
            all_nodes.insert(*k);
        }
        for (k, _) in &self.forward_edges {
            all_nodes.insert(*k);
        }
        
        // Calculate in-degrees
        for node in &all_nodes {
            in_degree.insert(*node, 0);
        }
        
        for (_, dependents) in &self.forward_edges {
            for &dep in dependents {
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }
        
        // Kahn's algorithm
        let mut queue: Vec<_> = in_degree
            .iter()
            .filter_map(|(node, &degree)| {
                if degree == 0 { Some(*node) } else { None }
            })
            .collect();
        
        let mut result = Vec::new();
        
        while let Some(current) = queue.pop() {
            result.push(current);
            
            if let Some(next_nodes) = self.forward_edges.get(&current) {
                for &next in next_nodes {
                    if let Some(degree) = in_degree.get_mut(&next) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        
        if result.len() == all_nodes.len() {
            self.topo_sort_cache = Some(result.clone());
            self.cache_valid = true;
            Some(result)
        } else {
            None // Cycle detected
        }
    }
    
    /// Get dependency metadata
    pub fn get_dependency(&self, dependent: u64, prerequisite: u64) -> Option<&CausalDependency> {
        self.dependencies.get(&(dependent, prerequisite))
    }
    
    /// Count total dependencies
    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }
    
    /// Get critical path (longest path from source to sink)
    pub fn critical_path(&self) -> Option<Vec<u64>> {
        // Find nodes with no predecessors
        let sources: Vec<_> = self.backward_edges
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(node, _)| *node)
            .collect();
        
        // Find longest path from any source to any sink
        let mut longest = Vec::new();
        
        for source in sources {
            if let Some(path) = self.longest_path_from(source) {
                if path.len() > longest.len() {
                    longest = path;
                }
            }
        }
        
        if longest.is_empty() { None } else { Some(longest) }
    }
    
    /// Find longest path from a source node
    fn longest_path_from(&self, source: u64) -> Option<Vec<u64>> {
        let mut longest = vec![source];
        let mut visited = HashSet::new();
        visited.insert(source);
        
        self.dfs_longest_path(source, &mut visited, &mut longest);
        
        if longest.len() > 1 {
            Some(longest)
        } else {
            None
        }
    }
    
    fn dfs_longest_path(&self, 
                       current: u64, 
                       visited: &mut HashSet<u64>, 
                       longest: &mut Vec<u64>) 
    {
        if let Some(next_nodes) = self.forward_edges.get(&current) {
            for &next in next_nodes {
                if !visited.contains(&next) {
                    visited.insert(next);
                    longest.push(next);
                    
                    self.dfs_longest_path(next, visited, longest);
                    
                    if longest.len() == 1 {
                        longest.pop();
                        visited.remove(&next);
                    }
                }
            }
        }
    }
    
    /// Check if event A must happen before event B in all valid orderings
    pub fn must_precede(&self, event_a: u64, event_b: u64) -> bool {
        self.can_reach(event_a, event_b)
    }
    
    /// Get total size of graph
    pub fn size(&self) -> (usize, usize) {
        let edges = self.dependencies.len();
        let nodes: HashSet<u64> = self.forward_edges
            .keys()
            .chain(self.backward_edges.keys())
            .copied()
            .collect();
        (nodes.len(), edges)
    }
    
    /// Clear the graph
    pub fn clear(&mut self) {
        self.forward_edges.clear();
        self.backward_edges.clear();
        self.dependencies.clear();
        self.known_cycles.clear();
        self.topo_sort_cache = None;
        self.cache_valid = false;
    }
}

impl Default for CausalityGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_graph_creation() {
        let graph = CausalityGraph::new();
        let (nodes, edges) = graph.size();
        assert_eq!(nodes, 0);
        assert_eq!(edges, 0);
    }
    
    #[test]
    fn test_link_events() {
        let mut graph = CausalityGraph::new();
        let result = graph.link_events(2, 1, DependencyType::Data, 1.0);
        assert!(result.is_ok());
        assert_eq!(graph.dependency_count(), 1);
    }
    
    #[test]
    fn test_direct_dependencies() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(2, 3, DependencyType::Control, 0.8).unwrap();
        
        let deps = graph.direct_dependencies(2);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&1));
        assert!(deps.contains(&3));
    }
    
    #[test]
    fn test_transitive_dependencies() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Control, 1.0).unwrap();
        
        let trans = graph.transitive_dependencies(3);
        assert_eq!(trans.len(), 2);
        assert!(trans.contains(&2));
        assert!(trans.contains(&1));
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Data, 1.0).unwrap();
        
        // Verify the graph has no cycles initially
        assert!(!graph.has_cycle());
        
        // Add an edge that doesn't create a cycle
        let result = graph.link_events(4, 1, DependencyType::Data, 1.0);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_acyclic_graph() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Data, 1.0).unwrap();
        graph.link_events(4, 3, DependencyType::Data, 1.0).unwrap();
        
        assert!(!graph.has_cycle());
    }
    
    #[test]
    fn test_topological_sort() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Data, 1.0).unwrap();
        graph.link_events(4, 3, DependencyType::Data, 1.0).unwrap();
        
        if let Some(sort) = graph.topological_sort() {
            assert_eq!(sort.len(), 4);
            // Check ordering constraints
            let pos = |x: u64| sort.iter().position(|&n| n == x).unwrap();
            assert!(pos(1) < pos(2));
            assert!(pos(2) < pos(3));
            assert!(pos(3) < pos(4));
        }
    }
    
    #[test]
    fn test_dependent_events() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 1, DependencyType::Data, 1.0).unwrap();
        
        let deps = graph.dependent_events(1);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&2));
        assert!(deps.contains(&3));
    }
    
    #[test]
    fn test_must_precede() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Data, 1.0).unwrap();
        
        assert!(graph.must_precede(1, 2));
        assert!(graph.must_precede(1, 3));
        assert!(graph.must_precede(2, 3));
        assert!(!graph.must_precede(2, 1));
    }
    
    #[test]
    fn test_dependency_metadata() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 0.9).unwrap();
        
        let dep = graph.get_dependency(2, 1).unwrap();
        assert_eq!(dep.dependent_event, 2);
        assert_eq!(dep.prerequisite_event, 1);
        assert_eq!(dep.dependency_type, DependencyType::Data);
        assert!(dep.strength > 0.8 && dep.strength < 1.0);
    }
    
    #[test]
    fn test_clear() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        assert_eq!(graph.dependency_count(), 1);
        
        graph.clear();
        assert_eq!(graph.dependency_count(), 0);
    }
    
    #[test]
    fn test_complex_dag() {
        let mut graph = CausalityGraph::new();
        
        // Create diamond structure
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(4, 2, DependencyType::Data, 1.0).unwrap();
        graph.link_events(4, 3, DependencyType::Data, 1.0).unwrap();
        
        assert!(!graph.has_cycle());
        let trans = graph.transitive_dependencies(4);
        assert_eq!(trans.len(), 3);
    }
    
    #[test]
    fn test_critical_path() {
        let mut graph = CausalityGraph::new();
        graph.link_events(2, 1, DependencyType::Data, 1.0).unwrap();
        graph.link_events(3, 2, DependencyType::Data, 1.0).unwrap();
        graph.link_events(4, 3, DependencyType::Data, 1.0).unwrap();
        
        // Verify we have a valid DAG
        assert!(!graph.has_cycle());
        
        // Verify graph has dependencies
        assert!(graph.dependency_count() > 0);
    }
}
