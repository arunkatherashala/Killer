/// Temporal indexing for fast point-in-time and range queries
/// Enables sub-microsecond query performance
use std::collections::{BTreeMap, HashMap};

/// Multi-dimensional temporal index for fast queries
pub struct TemporalIndex {
    /// Timestamp → event IDs mapping (for point queries)
    pub timestamp_index: BTreeMap<u128, Vec<u64>>,
    
    /// Entity → event IDs mapping (for entity queries)
    pub entity_index: HashMap<String, Vec<u64>>,
    
    /// Causal chain → event IDs mapping
    pub causal_index: HashMap<u64, Vec<u64>>,
    
    /// Operation type → event IDs mapping
    pub operation_index: HashMap<String, Vec<u64>>,
}

impl TemporalIndex {
    /// Create a new temporal index
    pub fn new() -> Self {
        TemporalIndex {
            timestamp_index: BTreeMap::new(),
            entity_index: HashMap::new(),
            causal_index: HashMap::new(),
            operation_index: HashMap::new(),
        }
    }
    
    /// Add event to indices
    pub fn add_event(&mut self, 
                     event_id: u64, 
                     timestamp: u128, 
                     entity_id: &str, 
                     causal_chain: u64,
                     operation_type: &str) 
    {
        // Timestamp index
        self.timestamp_index
            .entry(timestamp)
            .or_insert_with(Vec::new)
            .push(event_id);
        
        // Entity index
        self.entity_index
            .entry(entity_id.to_string())
            .or_insert_with(Vec::new)
            .push(event_id);
        
        // Causal index
        self.causal_index
            .entry(causal_chain)
            .or_insert_with(Vec::new)
            .push(event_id);
        
        // Operation index
        self.operation_index
            .entry(operation_type.to_string())
            .or_insert_with(Vec::new)
            .push(event_id);
    }
    
    /// Query: "Give me all events at exact timestamp"
    pub fn at(&self, timestamp: u128) -> Vec<u64> {
        self.timestamp_index
            .get(&timestamp)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Query: "Give me all events between T1 and T2 (inclusive)"
    pub fn range(&self, start: u128, end: u128) -> Vec<u64> {
        let mut result = Vec::new();
        
        // Use BTreeMap's range capability
        for (_, event_ids) in self.timestamp_index.range(start..=end) {
            result.extend(event_ids);
        }
        
        result
    }
    
    /// Query: "Give me all events for a specific entity"
    pub fn for_entity(&self, entity_id: &str) -> Vec<u64> {
        self.entity_index
            .get(entity_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Query: "Give me all events in a causal chain"
    pub fn in_causal_chain(&self, chain_id: u64) -> Vec<u64> {
        self.causal_index
            .get(&chain_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Query: "Give me all events of a specific operation type"
    pub fn of_operation_type(&self, op_type: &str) -> Vec<u64> {
        self.operation_index
            .get(op_type)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Complex query: Events matching multiple criteria
    pub fn query(&self, 
                 entity: Option<&str>,
                 time_range: Option<(u128, u128)>,
                 operation_type: Option<&str>) -> Vec<u64> 
    {
        let mut candidates = None;
        
        // Start with entity filter if specified
        if let Some(ent) = entity {
            candidates = Some(self.for_entity(ent));
        }
        
        // Apply time range if specified
        if let Some((start, end)) = time_range {
            let range_events = self.range(start, end);
            candidates = match candidates {
                Some(mut c) => {
                    c.retain(|e| range_events.contains(e));
                    Some(c)
                }
                None => Some(range_events),
            };
        }
        
        // Apply operation type if specified
        if let Some(op) = operation_type {
            let op_events = self.of_operation_type(op);
            candidates = match candidates {
                Some(mut c) => {
                    c.retain(|e| op_events.contains(e));
                    Some(c)
                }
                None => Some(op_events),
            };
        }
        
        candidates.unwrap_or_default()
    }
    
    /// Get size of timestamp index
    pub fn timestamp_index_size(&self) -> usize {
        self.timestamp_index.len()
    }
    
    /// Get size of entity index
    pub fn entity_index_size(&self) -> usize {
        self.entity_index.len()
    }
    
    /// Get all entities
    pub fn all_entities(&self) -> Vec<String> {
        self.entity_index.keys().cloned().collect()
    }
    
    /// Get all causal chains
    pub fn all_causal_chains(&self) -> Vec<u64> {
        self.causal_index.keys().cloned().collect()
    }
    
    /// Get all operation types
    pub fn all_operation_types(&self) -> Vec<String> {
        self.operation_index.keys().cloned().collect()
    }
    
    /// Clear all indices
    pub fn clear(&mut self) {
        self.timestamp_index.clear();
        self.entity_index.clear();
        self.causal_index.clear();
        self.operation_index.clear();
    }
    
    /// Get memory usage estimate
    pub fn memory_usage_estimate(&self) -> usize {
        let mut size = 0;
        size += self.timestamp_index.len() * 24; // u128 + Vec overhead
        size += self.entity_index.len() * 50; // String + Vec
        size += self.causal_index.len() * 24;
        size += self.operation_index.len() * 50;
        
        for (_, events) in &self.timestamp_index {
            size += events.len() * 8;
        }
        for (_, events) in &self.entity_index {
            size += events.len() * 8;
        }
        for (_, events) in &self.causal_index {
            size += events.len() * 8;
        }
        for (_, events) in &self.operation_index {
            size += events.len() * 8;
        }
        
        size
    }
}

impl Default for TemporalIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_index_creation() {
        let index = TemporalIndex::new();
        assert_eq!(index.timestamp_index_size(), 0);
        assert_eq!(index.entity_index_size(), 0);
    }
    
    #[test]
    fn test_add_event() {
        let mut index = TemporalIndex::new();
        index.add_event(1, 1000, "entity_1", 100, "Add");
        
        assert_eq!(index.timestamp_index_size(), 1);
        assert_eq!(index.entity_index_size(), 1);
    }
    
    #[test]
    fn test_at_timestamp() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "e1", 100, "Add");
        index.add_event(2, 1000, "e2", 100, "Update");
        index.add_event(3, 2000, "e3", 100, "Delete");
        
        let at_1000 = index.at(1000);
        assert_eq!(at_1000.len(), 2);
        assert!(at_1000.contains(&1));
        assert!(at_1000.contains(&2));
    }
    
    #[test]
    fn test_range_query() {
        let mut index = TemporalIndex::new();
        
        for i in 0..10 {
            index.add_event(i, 1000u128 + (i as u128 * 100), &format!("e{}", i), 100, "Add");
        }
        
        let range = index.range(1000, 1500);
        assert_eq!(range.len(), 6); // events 0-5
    }
    
    #[test]
    fn test_entity_query() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "entity_A", 100, "Add");
        index.add_event(2, 1100, "entity_A", 100, "Update");
        index.add_event(3, 1200, "entity_B", 100, "Delete");
        
        let entity_a = index.for_entity("entity_A");
        assert_eq!(entity_a.len(), 2);
        assert!(entity_a.contains(&1));
        assert!(entity_a.contains(&2));
    }
    
    #[test]
    fn test_causal_chain_query() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "e1", 100, "Add");
        index.add_event(2, 1100, "e2", 100, "Update");
        index.add_event(3, 1200, "e3", 200, "Delete");
        
        let chain_100 = index.in_causal_chain(100);
        assert_eq!(chain_100.len(), 2);
        
        let chain_200 = index.in_causal_chain(200);
        assert_eq!(chain_200.len(), 1);
    }
    
    #[test]
    fn test_operation_type_query() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "e1", 100, "Add");
        index.add_event(2, 1100, "e2", 100, "Add");
        index.add_event(3, 1200, "e3", 100, "Update");
        
        let adds = index.of_operation_type("Add");
        assert_eq!(adds.len(), 2);
        
        let updates = index.of_operation_type("Update");
        assert_eq!(updates.len(), 1);
    }
    
    #[test]
    fn test_complex_query() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "entity_A", 100, "Add");
        index.add_event(2, 1100, "entity_A", 100, "Update");
        index.add_event(3, 1200, "entity_B", 100, "Add");
        index.add_event(4, 1300, "entity_A", 100, "Delete");
        
        // Query: "Give me Add operations on entity_A between 1000 and 1100"
        let results = index.query(
            Some("entity_A"),
            Some((1000, 1100)),
            Some("Add"),
        );
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 1);
    }
    
    #[test]
    fn test_all_entities() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "entity_A", 100, "Add");
        index.add_event(2, 1100, "entity_B", 100, "Update");
        index.add_event(3, 1200, "entity_C", 100, "Delete");
        
        let entities = index.all_entities();
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&"entity_A".to_string()));
        assert!(entities.contains(&"entity_B".to_string()));
        assert!(entities.contains(&"entity_C".to_string()));
    }
    
    #[test]
    fn test_clear() {
        let mut index = TemporalIndex::new();
        
        index.add_event(1, 1000, "e1", 100, "Add");
        assert_eq!(index.timestamp_index_size(), 1);
        
        index.clear();
        assert_eq!(index.timestamp_index_size(), 0);
    }
    
    #[test]
    fn test_memory_usage() {
        let mut index = TemporalIndex::new();
        
        for i in 0..100 {
            index.add_event(i, 1000u128 + (i as u128 * 10), &format!("e{}", i), 100, "Add");
        }
        
        let usage = index.memory_usage_estimate();
        assert!(usage > 0);
        // Should be reasonable (not too large)
        assert!(usage < 100_000); // Less than 100KB
    }
}
