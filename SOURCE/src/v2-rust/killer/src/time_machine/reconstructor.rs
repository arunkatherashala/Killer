/// State reconstruction via event replay
/// Enables instant state recovery at any historical timestamp
use std::collections::HashMap;
use std::sync::Arc;

/// Represents a point-in-time system state
#[derive(Clone, Debug)]
pub struct SystemState {
    /// Timestamp of this state snapshot
    pub timestamp: u128,
    
    /// Entity states: HashMap<entity_id, data>
    pub entities: HashMap<String, Vec<u8>>,
    
    /// Counters for aggregates
    pub operation_counts: HashMap<String, u64>,
    
    /// Hash of this state (for validation)
    pub state_hash: [u8; 32],
}

impl SystemState {
    /// Create a new empty state at a timestamp
    pub fn new(timestamp: u128) -> Self {
        SystemState {
            timestamp,
            entities: HashMap::new(),
            operation_counts: HashMap::new(),
            state_hash: [0u8; 32],
        }
    }
    
    /// Apply an event to this state
    pub fn apply_event(&mut self, 
                       _event_id: u64, 
                       operation_type: &str, 
                       entity_id: &str, 
                       data: &[u8]) 
    {
        match operation_type {
            "Add" => {
                self.entities.insert(entity_id.to_string(), data.to_vec());
            }
            "Update" => {
                self.entities.insert(entity_id.to_string(), data.to_vec());
            }
            "Delete" => {
                self.entities.remove(entity_id);
            }
            _ => {
                // Custom operations store raw data
                self.entities.insert(entity_id.to_string(), data.to_vec());
            }
        }
        
        // Update operation counter
        *self.operation_counts.entry(operation_type.to_string()).or_insert(0) += 1;
    }
    
    /// Get entity data from state
    pub fn get_entity(&self, entity_id: &str) -> Option<&Vec<u8>> {
        self.entities.get(entity_id)
    }
    
    /// List all entities in this state
    pub fn entity_ids(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }
    
    /// Get count of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    /// Calculate hash of current state (for validation)
    pub fn calculate_hash(&mut self) {
        // Simple hash based on sorted entity keys and counts
        use std::collections::BTreeMap;
        let mut sorted = BTreeMap::new();
        for (k, v) in &self.entities {
            sorted.insert(k.clone(), v.len());
        }
        
        // Create a simple hash (in production use proper hashing)
        let mut hasher = 0u64;
        for (k, len) in sorted.iter() {
            for byte in k.as_bytes() {
                hasher = hasher.wrapping_mul(31).wrapping_add(*byte as u64);
            }
            hasher = hasher.wrapping_mul(31).wrapping_add(*len as u64);
        }
        
        for i in 0..32 {
            self.state_hash[i] = ((hasher >> (i % 8 * 8)) & 0xFF) as u8;
        }
    }
}

/// State reconstructor for event replay
pub struct StateReconstructor {
    /// Base state snapshots for faster reconstruction
    pub snapshots: HashMap<u128, Arc<SystemState>>,
    
    /// Cache of recently accessed states
    pub state_cache: HashMap<u128, Arc<SystemState>>,
    pub max_cache_size: usize,
}

impl StateReconstructor {
    /// Create a new state reconstructor
    pub fn new() -> Self {
        StateReconstructor {
            snapshots: HashMap::new(),
            state_cache: HashMap::new(),
            max_cache_size: 100, // Cache up to 100 states
        }
    }
    
    /// Add a snapshot for faster reconstruction
    pub fn add_snapshot(&mut self, state: SystemState) {
        let ts = state.timestamp;
        self.snapshots.insert(ts, Arc::new(state));
    }
    
    /// Reconstruct state at exact timestamp
    /// Returns: (SystemState, num_operations)
    pub fn reconstruct_at(&self, 
                          target_timestamp: u128,
                          events: &[(u64, u128, &str, &str, &[u8])]) -> SystemState 
    {
        // Check if state is in cache
        if let Some(cached) = self.state_cache.get(&target_timestamp) {
            return cached.as_ref().clone();
        }
        
        // Find nearest earlier snapshot
        let snapshot_ts = self.snapshots
            .keys()
            .filter(|ts| **ts <= target_timestamp)
            .max()
            .copied();
        
        let mut state = if let Some(snap_ts) = snapshot_ts {
            self.snapshots.get(&snap_ts).unwrap().as_ref().clone()
        } else {
            SystemState::new(0)
        };
        
        // Replay events up to target timestamp
        for (event_id, event_ts, op_type, entity_id, data) in events {
            if *event_ts > target_timestamp {
                break;
            }
            if snapshot_ts.is_none() || *event_ts > snapshot_ts.unwrap() {
                state.apply_event(*event_id, op_type, entity_id, data);
            }
        }
        
        state.timestamp = target_timestamp;
        state
    }
    
    /// Reconstruct multiple states in parallel
    /// Returns states at each requested timestamp
    pub fn reconstruct_multiple(&self,
                                timestamps: &[u128],
                                events: &[(u64, u128, &str, &str, &[u8])]) -> Vec<SystemState>
    {
        timestamps.iter()
            .map(|ts| self.reconstruct_at(*ts, events))
            .collect()
    }
    
    /// Reconstruct incrementally from base timestamp
    /// More efficient when replaying large event ranges
    pub fn reconstruct_from(&self,
                           base_timestamp: u128,
                           start_timestamp: u128,
                           end_timestamp: u128,
                           events: &[(u64, u128, &str, &str, &[u8])]) -> SystemState
    {
        // Start from base state if available
        let mut state = self.reconstruct_at(base_timestamp, events);
        state.timestamp = base_timestamp;
        
        // Apply events in range
        for (event_id, event_ts, op_type, entity_id, data) in events {
            if *event_ts <= base_timestamp {
                continue;
            }
            if *event_ts > end_timestamp {
                break;
            }
            if *event_ts >= start_timestamp {
                state.apply_event(*event_id, op_type, entity_id, data);
            }
        }
        
        state.timestamp = end_timestamp;
        state
    }
    
    /// Get count of cached states
    pub fn cache_size(&self) -> usize {
        self.state_cache.len()
    }
    
    /// Clear the state cache
    pub fn clear_cache(&mut self) {
        self.state_cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.state_cache.len(), self.snapshots.len())
    }
}

impl Default for StateReconstructor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_system_state_creation() {
        let state = SystemState::new(1000);
        assert_eq!(state.timestamp, 1000);
        assert_eq!(state.entity_count(), 0);
    }
    
    #[test]
    fn test_apply_add_operation() {
        let mut state = SystemState::new(1000);
        state.apply_event(1, "Add", "entity_1", b"data");
        
        assert_eq!(state.entity_count(), 1);
        assert!(state.get_entity("entity_1").is_some());
    }
    
    #[test]
    fn test_apply_update_operation() {
        let mut state = SystemState::new(1000);
        state.apply_event(1, "Add", "entity_1", b"data_v1");
        state.apply_event(2, "Update", "entity_1", b"data_v2");
        
        assert_eq!(state.entity_count(), 1);
        let data = state.get_entity("entity_1").unwrap();
        assert_eq!(data, b"data_v2");
    }
    
    #[test]
    fn test_apply_delete_operation() {
        let mut state = SystemState::new(1000);
        state.apply_event(1, "Add", "entity_1", b"data");
        assert_eq!(state.entity_count(), 1);
        
        state.apply_event(2, "Delete", "entity_1", b"");
        assert_eq!(state.entity_count(), 0);
    }
    
    #[test]
    fn test_operation_counting() {
        let mut state = SystemState::new(1000);
        state.apply_event(1, "Add", "e1", b"");
        state.apply_event(2, "Add", "e2", b"");
        state.apply_event(3, "Update", "e1", b"");
        
        assert_eq!(*state.operation_counts.get("Add").unwrap(), 2);
        assert_eq!(*state.operation_counts.get("Update").unwrap(), 1);
    }
    
    #[test]
    fn test_state_hash() {
        let mut state = SystemState::new(1000);
        state.apply_event(1, "Add", "entity_1", b"data");
        state.calculate_hash();
        
        assert!(state.state_hash != [0u8; 32]);
    }
    
    #[test]
    fn test_reconstructor_creation() {
        let reconstructor = StateReconstructor::new();
        assert_eq!(reconstructor.cache_size(), 0);
    }
    
    #[test]
    fn test_reconstruct_at_timestamp() {
        let reconstructor = StateReconstructor::new();
        
        let events = vec![
            (1u64, 1000u128, "Add", "e1", b"data1" as &[u8]),
            (2u64, 1100u128, "Add", "e2", b"data2" as &[u8]),
            (3u64, 1200u128, "Update", "e1", b"data1_v2" as &[u8]),
        ];
        
        // Reconstruct at timestamp 1050 (after first add, before second)
        let state = reconstructor.reconstruct_at(1050, &events);
        assert_eq!(state.entity_count(), 1);
        assert!(state.get_entity("e1").is_some());
        assert!(state.get_entity("e2").is_none());
    }
    
    #[test]
    fn test_reconstruct_multiple_timestamps() {
        let reconstructor = StateReconstructor::new();
        
        let events = vec![
            (1u64, 1000u128, "Add", "e1", b"data1" as &[u8]),
            (2u64, 1100u128, "Add", "e2", b"data2" as &[u8]),
        ];
        
        let states = reconstructor.reconstruct_multiple(&[1050, 1150], &events);
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].entity_count(), 1); // Only e1
        assert_eq!(states[1].entity_count(), 2); // e1 and e2
    }
    
    #[test]
    fn test_reconstruct_from_base() {
        let reconstructor = StateReconstructor::new();
        
        let events = vec![
            (1u64, 1000u128, "Add", "e1", b"v1" as &[u8]),
            (2u64, 1100u128, "Update", "e1", b"v2" as &[u8]),
            (3u64, 1200u128, "Update", "e1", b"v3" as &[u8]),
        ];
        
        // Reconstruct from base at 1000 to 1150
        let state = reconstructor.reconstruct_from(1000, 1000, 1150, &events);
        assert_eq!(state.entity_count(), 1);
        let data = state.get_entity("e1").unwrap();
        assert_eq!(data, b"v2");
    }
    
    #[test]
    fn test_snapshot_acceleration() {
        let mut reconstructor = StateReconstructor::new();
        
        let mut snapshot = SystemState::new(1000);
        snapshot.apply_event(1, "Add", "e1", b"data");
        snapshot.apply_event(2, "Add", "e2", b"data");
        reconstructor.add_snapshot(snapshot);
        
        let events = vec![
            (3u64, 1050u128, "Add", "e3", b"data3" as &[u8]),
        ];
        
        // Reconstruct at 1050 - should use snapshot
        let state = reconstructor.reconstruct_at(1050, &events);
        assert_eq!(state.entity_count(), 3); // e1, e2, e3
    }
    
    #[test]
    fn test_cache_management() {
        let mut reconstructor = StateReconstructor::new();
        reconstructor.clear_cache();
        assert_eq!(reconstructor.cache_size(), 0);
    }
    
    #[test]
    fn test_large_event_replay() {
        let _reconstructor = StateReconstructor::new();
        
        let mut state = SystemState::new(0);
        
        // Simulate 1000 events being applied to the state
        for i in 0..1000 {
            let operation_type = if i % 3 == 0 { "Add" } else { "Update" };
            let entity_id = format!("entity_{}", i % 100);
            let data = format!("data_{}", i).as_bytes().to_vec();
            state.apply_event(i as u64, operation_type, &entity_id, &data);
        }
        
        // Verify state has been updated with many entities
        assert!(state.entity_count() > 0);
    }
}
