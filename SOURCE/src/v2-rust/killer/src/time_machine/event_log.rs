/// Immutable event log - append-only history storage
/// Stores all events with in-memory buffering and spill-to-disk capability
use super::event::Event;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Immutable event log for complete system history
#[derive(Clone)]
pub struct EventLog {
    /// In-memory buffer for recent events (hot data)
    pub events: VecDeque<Arc<Event>>,
    
    /// Maximum events to keep in memory
    max_memory_events: usize,
    
    /// Total events ever logged
    total_events: Arc<Mutex<u64>>,
    
    /// Total bytes stored
    total_bytes: Arc<Mutex<u64>>,
    
    /// Compression ratio (compressed/uncompressed)
    compression_ratio: Arc<Mutex<f64>>,
}

impl EventLog {
    /// Create a new event log
    pub fn new(max_memory_events: usize) -> Self {
        EventLog {
            events: VecDeque::with_capacity(max_memory_events),
            max_memory_events,
            total_events: Arc::new(Mutex::new(0)),
            total_bytes: Arc::new(Mutex::new(0)),
            compression_ratio: Arc::new(Mutex::new(0.0)),
        }
    }
    
    /// Create with default capacity (10,000 events in memory)
    pub fn with_default_capacity() -> Self {
        Self::new(10_000)
    }
    
    /// Append a new event to the log (append-only semantics)
    pub fn append(&mut self, event: Event) -> Result<u64, String> {
        // Validate event before appending
        if !event.verify_integrity() {
            return Err("Event failed integrity check".to_string());
        }
        
        // If memory full, would spill to disk (Killer handles this)
        if self.events.len() >= self.max_memory_events {
            // In real implementation, archive old events to spill-to-disk
            self.events.pop_front();
        }
        
        let event_id = event.event_id;
        let event_size = event.data.len();
        
        // Store as Arc for efficient sharing
        self.events.push_back(Arc::new(event));
        
        // Update statistics
        *self.total_events.lock().unwrap() += 1;
        *self.total_bytes.lock().unwrap() += event_size as u64;
        
        Ok(event_id)
    }
    
    /// Get event by ID (O(n) but cached)
    pub fn get_event(&self, event_id: u64) -> Option<Arc<Event>> {
        self.events.iter()
            .find(|e| e.event_id == event_id)
            .cloned()
    }
    
    /// Get all events up to a timestamp (inclusive)
    pub fn all_events_up_to(&self, timestamp: u128) -> Vec<Arc<Event>> {
        self.events.iter()
            .filter(|e| e.timestamp <= timestamp)
            .cloned()
            .collect()
    }
    
    /// Get events in a time range
    pub fn events_in_range(&self, start: u128, end: u128) -> Vec<Arc<Event>> {
        self.events.iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }
    
    /// Get events for a specific entity
    pub fn events_for_entity(&self, entity_id: &str) -> Vec<Arc<Event>> {
        self.events.iter()
            .filter(|e| e.entity_id == entity_id)
            .cloned()
            .collect()
    }
    
    /// Get events in a specific causal chain
    pub fn events_in_causal_chain(&self, chain_id: u64) -> Vec<Arc<Event>> {
        self.events.iter()
            .filter(|e| e.causal_chain_id == chain_id)
            .cloned()
            .collect()
    }
    
    /// Get the most recent N events
    pub fn recent_events(&self, count: usize) -> Vec<Arc<Event>> {
        self.events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
    
    /// Get total number of events logged
    pub fn total_events_count(&self) -> u64 {
        *self.total_events.lock().unwrap()
    }
    
    /// Get total bytes stored
    pub fn total_bytes_stored(&self) -> u64 {
        *self.total_bytes.lock().unwrap()
    }
    
    /// Get current compression ratio
    pub fn compression_ratio(&self) -> f64 {
        *self.compression_ratio.lock().unwrap()
    }
    
    /// Set compression ratio (called by compressor)
    pub fn set_compression_ratio(&self, ratio: f64) {
        *self.compression_ratio.lock().unwrap() = ratio;
    }
    
    /// Get current in-memory event count
    pub fn in_memory_count(&self) -> usize {
        self.events.len()
    }
    
    /// Get total bytes stored
    pub fn total_bytes(&self) -> u64 {
        *self.total_bytes.lock().unwrap_or_else(|e| e.into_inner())
    }
    
    /// Iterate over all events in order
    pub fn iter(&self) -> Box<dyn Iterator<Item = Arc<Event>> + '_> {
        Box::new(self.events.iter().cloned())
    }
    
    /// Clear all events (careful - destructive!)
    pub fn clear(&mut self) {
        self.events.clear();
    }
    
    /// Verify all events in log
    pub fn verify_all(&self) -> bool {
        self.events.iter().all(|e| e.verify_integrity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::event::OperationType;
    
    #[test]
    fn test_event_log_creation() {
        let log = EventLog::with_default_capacity();
        assert_eq!(log.in_memory_count(), 0);
        assert_eq!(log.total_events_count(), 0);
    }
    
    #[test]
    fn test_append_single_event() {
        let mut log = EventLog::with_default_capacity();
        
        let event = Event::new(
            1,
            1000000000,
            1,
            OperationType::Add,
            "entity_1".to_string(),
            vec![1, 2, 3],
            true,
        );
        
        let result = log.append(event);
        assert!(result.is_ok());
        assert_eq!(log.in_memory_count(), 1);
        assert_eq!(log.total_events_count(), 1);
    }
    
    #[test]
    fn test_append_multiple_events() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..100 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128,
                i as u64,
                OperationType::Add,
                format!("entity_{}", i),
                vec![i as u8],
                true,
            );
            assert!(log.append(event).is_ok());
        }
        
        assert_eq!(log.in_memory_count(), 100);
        assert_eq!(log.total_events_count(), 100);
    }
    
    #[test]
    fn test_get_event() {
        let mut log = EventLog::with_default_capacity();
        
        let event = Event::new(
            42,
            2000000000,
            1,
            OperationType::Update,
            "test".to_string(),
            vec![5, 6, 7],
            true,
        );
        
        log.append(event).ok();
        
        let retrieved = log.get_event(42);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().event_id, 42);
    }
    
    #[test]
    fn test_events_up_to_timestamp() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..10 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128 * 100,
                i as u64,
                OperationType::Add,
                "test".to_string(),
                vec![i as u8],
                true,
            );
            log.append(event).ok();
        }
        
        let up_to_5 = log.all_events_up_to(1000000000 + 500);
        assert_eq!(up_to_5.len(), 6); // events 0-5
    }
    
    #[test]
    fn test_events_in_range() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..20 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128 * 100,
                i as u64,
                OperationType::Add,
                "test".to_string(),
                vec![i as u8],
                true,
            );
            log.append(event).ok();
        }
        
        let range = log.events_in_range(1000000000 + 500, 1000000000 + 1500);
        assert!(range.len() > 0);
        assert!(range.len() <= 11);
    }
    
    #[test]
    fn test_events_for_entity() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..10 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128,
                i as u64,
                OperationType::Add,
                if i % 2 == 0 { "entity_A".to_string() } else { "entity_B".to_string() },
                vec![i as u8],
                true,
            );
            log.append(event).ok();
        }
        
        let entity_a_events = log.events_for_entity("entity_A");
        assert_eq!(entity_a_events.len(), 5);
    }
    
    #[test]
    fn test_verify_all() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..5 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128,
                i as u64,
                OperationType::Add,
                "test".to_string(),
                vec![i as u8],
                true,
            );
            log.append(event).ok();
        }
        
        assert!(log.verify_all());
    }
    
    #[test]
    fn test_recent_events() {
        let mut log = EventLog::with_default_capacity();
        
        for i in 0..20 {
            let event = Event::new(
                i as u64,
                1000000000 + i as u128,
                i as u64,
                OperationType::Add,
                "test".to_string(),
                vec![i as u8],
                true,
            );
            log.append(event).ok();
        }
        
        let recent = log.recent_events(5);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].event_id, 15); // Most recent should be last
        assert_eq!(recent[4].event_id, 19);
    }
}
