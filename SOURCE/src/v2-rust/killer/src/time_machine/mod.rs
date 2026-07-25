/// Time Machine: Temporal Computing Foundation
///
/// Phase 1: Event Sourcing Engine
///
/// In-memory / embeddable temporal history: event log, indexing, compression hooks, replay, causality.
/// Module-level comments elsewhere may state design *targets*; validate on your workload if you rely on latency or compression numbers.

// Phase 1: Event Sourcing Engine
pub mod event;
pub mod event_log;
pub mod compression;
pub mod index;
pub mod reconstructor;
pub mod causality;

// Phase 2: Causality Engine
pub mod causality_engine;

// Phase 3: Reversible Computation
pub mod reversible;

// Phase 4: Time-Series Database
pub mod timeseries;

// Phase 5: Quantum Temporal Simulator
pub mod quantum;

// Phase 6: What-If Analysis Engine
pub mod whatif;

// Phase 7: Temporal Machine Learning
pub mod ml;

// Phase 8: Physics Engine
pub mod physics;

pub use event::{Event, OperationType};
pub use event_log::EventLog;
pub use compression::EventCompressor;
pub use index::TemporalIndex;
pub use reconstructor::{StateReconstructor, SystemState};
pub use causality::{CausalityGraph, CausalDependency, DependencyType};
pub use causality_engine::CausalityEngine;
pub use reversible::ReversibleComputationEngine;
pub use timeseries::TimeSeriesDatabase;
pub use quantum::QuantumTemporalSimulator;
pub use whatif::WhatIfAnalysisEngine;
pub use ml::TemporalMLEngine;
pub use physics::PhysicsEngine;

/// Time Machine statistics and monitoring
#[derive(Clone, Debug)]
pub struct TimeMachineStats {
    pub total_events: u64,
    pub total_bytes: u64,
    pub compression_ratio: f64,
    pub event_rate_per_sec: u64,
    pub query_latency_us: u64,
    pub replay_latency_ms: u64,
    pub causal_chain_length: usize,
}

/// Time Machine engine coordinating all components
pub struct TimeMachine {
    pub event_log: EventLog,
    pub compressor: EventCompressor,
    pub index: TemporalIndex,
    pub reconstructor: StateReconstructor,
    pub causality_graph: CausalityGraph,
    
    stats: TimeMachineStats,
}

impl TimeMachine {
    /// Create a new Time Machine instance
    pub fn new() -> Self {
        TimeMachine {
            event_log: EventLog::new(10_000),
            compressor: EventCompressor::new(),
            index: TemporalIndex::new(),
            reconstructor: StateReconstructor::new(),
            causality_graph: CausalityGraph::new(),
            stats: TimeMachineStats {
                total_events: 0,
                total_bytes: 0,
                compression_ratio: 0.0,
                event_rate_per_sec: 0,
                query_latency_us: 0,
                replay_latency_ms: 0,
                causal_chain_length: 0,
            },
        }
    }
    
    /// Log an event and update all indices
    pub fn log_event(&mut self, 
                    event_id: u64,
                    timestamp: u128,
                    operation_type: OperationType,
                    entity_id: &str,
                    data: &[u8]) -> Result<(), String>
    {
        // Create event with sequence number and reversibility flag
        let sequence = self.stats.total_events;
        let event = Event::new(
            event_id,
            timestamp,
            sequence,
            operation_type.clone(),
            entity_id.to_string(),
            data.to_vec(),
            true,
        );
        
        // Append to log
        self.event_log.append(event)?;
        
        // Compress
        let _compressed = self.compressor.compress(data);
        
        // Update index
        self.index.add_event(
            event_id,
            timestamp,
            entity_id,
            0, // TODO: proper causal chain tracking
            &format!("{:?}", operation_type),
        );
        
        // Update stats
        self.stats.total_events += 1;
        self.stats.total_bytes = self.event_log.total_bytes();
        self.stats.compression_ratio = self.compressor.ratio();
        
        Ok(())
    }
    
    /// Query state at a specific timestamp
    pub fn state_at(&self, timestamp: u128) -> Option<SystemState> {
        // Get events up to timestamp
        let events = self.event_log.all_events_up_to(timestamp);
        
        if events.is_empty() {
            return None;
        }
        
        // Convert events to a format suitable for reconstruction
        // Create event data without temporary references
        let mut state = SystemState::new(0);
        
        for event in &events {
            if event.timestamp <= timestamp {
                let op_str = format!("{:?}", event.operation_type);
                state.apply_event(event.event_id, &op_str, &event.entity_id, &event.data);
            }
        }
        
        state.timestamp = timestamp;
        Some(state)
    }
    
    /// Range query: events between two timestamps
    pub fn events_in_range(&self, start: u128, end: u128) -> Vec<u64> {
        self.index.range(start, end)
    }
    
    /// Entity query: all events for an entity
    pub fn events_for_entity(&self, entity_id: &str) -> Vec<u64> {
        self.index.for_entity(entity_id)
    }
    
    /// Check for temporal paradoxes
    pub fn has_paradox(&mut self) -> bool {
        self.causality_graph.has_cycle()
    }
    
    /// Get statistics
    pub fn stats(&self) -> &TimeMachineStats {
        &self.stats
    }
    
    /// Verify integrity of all components
    pub fn verify(&self) -> Result<(), String> {
        // Verify event log
        if !self.event_log.verify_all() {
            return Err("Event log verification failed".to_string());
        }
        
        // Verify no paradoxes
        let mut graph = self.causality_graph.clone();
        if graph.has_cycle() {
            return Err("Temporal paradox detected!".to_string());
        }
        
        Ok(())
    }
    
    /// Get memory usage estimate
    pub fn memory_usage_estimate(&self) -> usize {
        let mut total = 0;
        total += self.event_log.total_bytes() as usize;
        total += self.index.memory_usage_estimate();
        // Storage for compressor, reconstructor, etc
        total += 100_000;
        total
    }
}

impl Default for TimeMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TimeMachine {
    fn clone(&self) -> Self {
        TimeMachine {
            event_log: self.event_log.clone(),
            compressor: self.compressor.clone(),
            index: self.index.clone(),
            reconstructor: self.reconstructor.clone(),
            causality_graph: self.causality_graph.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl Clone for CausalityGraph {
    fn clone(&self) -> Self {
        let mut new_graph = CausalityGraph::new();
        new_graph.forward_edges = self.forward_edges.clone();
        new_graph.backward_edges = self.backward_edges.clone();
        new_graph.dependencies = self.dependencies.clone();
        new_graph.known_cycles = self.known_cycles.clone();
        new_graph.topo_sort_cache = self.topo_sort_cache.clone();
        new_graph.cache_valid = self.cache_valid;
        new_graph
    }
}

impl Clone for TemporalIndex {
    fn clone(&self) -> Self {
        TemporalIndex {
            timestamp_index: self.timestamp_index.clone(),
            entity_index: self.entity_index.clone(),
            causal_index: self.causal_index.clone(),
            operation_index: self.operation_index.clone(),
        }
    }
}

impl Clone for StateReconstructor {
    fn clone(&self) -> Self {
        StateReconstructor {
            snapshots: self.snapshots.clone(),
            state_cache: self.state_cache.clone(),
            max_cache_size: self.max_cache_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_machine_creation() {
        let tm = TimeMachine::new();
        assert_eq!(tm.stats.total_events, 0);
    }
    
    #[test]
    fn test_log_event() {
        let mut tm = TimeMachine::new();
        let result = tm.log_event(
            1,
            1000,
            OperationType::Add,
            "entity_1",
            b"test_data",
        );
        assert!(result.is_ok());
        assert_eq!(tm.stats.total_events, 1);
    }
    
    #[test]
    fn test_multiple_events() {
        let mut tm = TimeMachine::new();
        
        for i in 0..10 {
            tm.log_event(
                i,
                1000u128 + (i as u128 * 100),
                OperationType::Add,
                &format!("entity_{}", i),
                b"data",
            ).unwrap();
        }
        
        assert_eq!(tm.stats.total_events, 10);
    }
    
    #[test]
    fn test_range_query() {
        let mut tm = TimeMachine::new();
        
        for i in 0..10 {
            tm.log_event(
                i,
                1000u128 + (i as u128 * 100),
                OperationType::Add,
                &format!("entity_{}", i),
                b"data",
            ).unwrap();
        }
        
        let range = tm.events_in_range(1000, 1500);
        assert!(range.len() > 0);
    }
    
    #[test]
    fn test_entity_query() {
        let mut tm = TimeMachine::new();
        
        tm.log_event(1, 1000u128, OperationType::Add, "entity_A", b"v1").unwrap();
        tm.log_event(2, 1100u128, OperationType::Update, "entity_A", b"v2").unwrap();
        tm.log_event(3, 1200u128, OperationType::Add, "entity_B", b"v1").unwrap();
        
        let events_a = tm.events_for_entity("entity_A");
        assert_eq!(events_a.len(), 2);
    }
    
    #[test]
    fn test_verify_integrity() {
        let tm = TimeMachine::new();
        let result = tm.verify();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_memory_usage() {
        let mut tm = TimeMachine::new();
        
        for i in 0..100 {
            tm.log_event(i, 1000u128 + (i as u128 * 10), OperationType::Add, 
                        &format!("e{}", i), b"data").unwrap();
        }
        
        let usage = tm.memory_usage_estimate();
        assert!(usage > 0);
    }
}
