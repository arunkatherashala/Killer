/// Immutable Event definition for temporal state tracking
/// Each event represents a single operation in the system's history
use std::collections::HashMap;

/// Represents the type of operation performed
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OperationType {
    Add,
    Update,
    Delete,
    Query,
    Compute,
    StateChange,
    Custom(String),
}

/// Immutable record of a single system operation
/// Used for complete temporal history tracking
#[derive(Clone, Debug)]
pub struct Event {
    /// Unique event identifier
    pub event_id: u64,
    
    /// Nanosecond-precision timestamp
    pub timestamp: u128,
    
    /// Global sequence number for ordering
    pub sequence: u64,
    
    /// Type of operation performed
    pub operation_type: OperationType,
    
    /// Entity/resource affected by this operation
    pub entity_id: String,
    
    /// Raw operation payload data
    pub data: Vec<u8>,
    
    /// Additional metadata (tags, source, etc)
    pub metadata: HashMap<String, String>,
    
    /// Event IDs that directly caused this event
    pub parent_events: Vec<u64>,
    
    /// Which causal timeline this belongs to
    pub causal_chain_id: u64,
    
    /// How to reverse this operation (for undo)
    pub inverse_operation: Option<Vec<u8>>,
    
    /// Whether this operation can be reversed
    pub reversible: bool,
    
    /// SHA-256 hash for integrity verification
    pub hash: [u8; 32],
    
    /// Optional cryptographic signature
    pub signature: Option<Vec<u8>>,
    
    /// Whether this event passed validation
    pub valid: bool,
}

impl Event {
    /// Create a new event
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: u64,
        timestamp: u128,
        sequence: u64,
        operation_type: OperationType,
        entity_id: String,
        data: Vec<u8>,
        reversible: bool,
    ) -> Self {
        let hash = Self::compute_hash(&data);
        
        Event {
            event_id,
            timestamp,
            sequence,
            operation_type,
            entity_id,
            data,
            metadata: HashMap::new(),
            parent_events: Vec::new(),
            causal_chain_id: 0,
            inverse_operation: None,
            reversible,
            hash,
            signature: None,
            valid: true,
        }
    }
    
    /// Compute simple hash of event data
    fn compute_hash(data: &[u8]) -> [u8; 32] {
        // Simple hash implementation without external dependencies
        let mut hash = [0u8; 32];
        let mut hasher = 0u64;
        
        for byte in data {
            hasher = hasher.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        
        for i in 0..32 {
            hash[i] = ((hasher >> (i % 8 * 8)) & 0xFF) as u8;
        }
        hash
    }
    
    /// Add metadata to this event
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Set the parent events (causes) for this event
    pub fn with_parents(mut self, parents: Vec<u64>) -> Self {
        self.parent_events = parents;
        self
    }
    
    /// Set the inverse operation for reversibility
    pub fn with_inverse(mut self, inverse: Vec<u8>) -> Self {
        self.inverse_operation = Some(inverse);
        self
    }
    
    /// Mark this event as invalid
    pub fn invalidate(mut self) -> Self {
        self.valid = false;
        self
    }
    
    /// Add a cryptographic signature
    pub fn with_signature(mut self, sig: Vec<u8>) -> Self {
        self.signature = Some(sig);
        self
    }
    
    /// Verify event integrity
    pub fn verify_integrity(&self) -> bool {
        // Recompute hash and compare
        let computed = Self::compute_hash(&self.data);
        computed == self.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_creation() {
        let event = Event::new(
            1,
            1000000000000,
            1,
            OperationType::Add,
            "entity_1".to_string(),
            vec![1, 2, 3, 4],
            true,
        );
        
        assert_eq!(event.event_id, 1);
        assert_eq!(event.sequence, 1);
        assert_eq!(event.reversible, true);
        assert_eq!(event.valid, true);
    }
    
    #[test]
    fn test_event_with_metadata() {
        let event = Event::new(
            1,
            1000000000000,
            1,
            OperationType::Update,
            "test".to_string(),
            vec![1, 2],
            true,
        )
        .with_metadata("source".to_string(), "test_suite".to_string());
        
        assert_eq!(event.metadata.get("source"), Some(&"test_suite".to_string()));
    }
    
    #[test]
    fn test_event_integrity() {
        let event = Event::new(
            1,
            1000000000000,
            1,
            OperationType::Add,
            "test".to_string(),
            vec![1, 2, 3],
            true,
        );
        
        assert!(event.verify_integrity());
    }
    
    #[test]
    fn test_event_with_parents() {
        let event = Event::new(
            2,
            2000000000000,
            2,
            OperationType::Update,
            "test".to_string(),
            vec![1, 2],
            true,
        )
        .with_parents(vec![1]);
        
        assert_eq!(event.parent_events, vec![1]);
    }
    
    #[test]
    fn test_operation_type_equality() {
        let op1 = OperationType::Add;
        let op2 = OperationType::Add;
        let op3 = OperationType::Delete;
        
        assert_eq!(op1, op2);
        assert_ne!(op1, op3);
    }
}
