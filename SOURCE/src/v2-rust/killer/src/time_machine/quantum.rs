/// Phase 5: Quantum Temporal Simulator
/// Superposition states and multiple timeline branches with quantum mechanics
use std::collections::HashMap;

/// Quantum state superposition with probability amplitudes
#[derive(Clone, Debug)]
pub struct QuantumState {
    pub state_id: u64,
    pub amplitudes: HashMap<u64, f64>,  // event_id -> probability amplitude
    pub phase: f64,
    pub coherence: f32,  // Measure of superposition coherence (0.0-1.0)
}

impl QuantumState {
    /// Create new quantum state
    pub fn new(state_id: u64) -> Self {
        QuantumState {
            state_id,
            amplitudes: HashMap::new(),
            phase: 0.0,
            coherence: 1.0,
        }
    }
    
    /// Add amplitude for event
    pub fn add_amplitude(&mut self, event_id: u64, amplitude: f64) {
        self.amplitudes.insert(event_id, amplitude);
    }
    
    /// Get probability of event (amplitude squared)
    pub fn probability(&self, event_id: u64) -> f64 {
        self.amplitudes.get(&event_id)
            .map(|a| a * a)
            .unwrap_or(0.0)
    }
    
    /// Normalize amplitudes
    pub fn normalize(&mut self) {
        let sum_sq: f64 = self.amplitudes.values()
            .map(|a| a * a)
            .sum();
        
        if sum_sq > 0.0 {
            let norm = sum_sq.sqrt();
            for amp in self.amplitudes.values_mut() {
                *amp /= norm;
            }
        }
    }
    
    /// Measure (collapse superposition)
    pub fn measure(&mut self) -> Option<u64> {
        let probabilities: Vec<(u64, f64)> = self.amplitudes.iter()
            .map(|(id, amp)| (*id, amp * amp))
            .collect();
        
        if let Some((event_id, _)) = probabilities.first() {
            // Collapse to first event
            self.amplitudes.clear();
            self.amplitudes.insert(*event_id, 1.0);
            self.coherence = 0.0;  // Lost superposition
            Some(*event_id)
        } else {
            None
        }
    }
    
    /// Decoherence factor (loss of superposition)
    pub fn decoherence_rate(&mut self, rate: f32) {
        self.coherence = (self.coherence * (1.0 - rate)).max(0.0);
        
        if self.coherence < 0.1 {
            // Force collapse if coherence too low
            let _ = self.measure();
        }
    }
}

/// Timeline branch with quantum superposition
#[derive(Clone, Debug)]
pub struct QuantumTimeline {
    pub timeline_id: u64,
    pub quantum_state: QuantumState,
    pub branch_probability: f64,
    pub events: Vec<u64>,
    pub parent_timeline: Option<u64>,
}

impl QuantumTimeline {
    /// Create new quantum timeline
    pub fn new(timeline_id: u64, state_id: u64) -> Self {
        QuantumTimeline {
            timeline_id,
            quantum_state: QuantumState::new(state_id),
            branch_probability: 1.0,
            events: Vec::new(),
            parent_timeline: None,
        }
    }
    
    /// Create branched timeline
    pub fn branch(parent_id: u64, timeline_id: u64, state_id: u64, probability: f64) -> Self {
        let mut tl = QuantumTimeline::new(timeline_id, state_id);
        tl.parent_timeline = Some(parent_id);
        tl.branch_probability = probability;
        tl
    }
    
    /// Add event to timeline
    pub fn add_event(&mut self, event_id: u64) {
        self.events.push(event_id);
    }
    
    /// Get total branch probability
    pub fn total_probability(&self) -> f64 {
        self.branch_probability *
            self.quantum_state.amplitudes.values()
                .map(|a| a * a)
                .sum::<f64>()
    }
}

/// Quantum Temporal Simulator
pub struct QuantumTemporalSimulator {
    /// All timelines in superposition
    timelines: HashMap<u64, QuantumTimeline>,
    
    /// Wave function (global superposition)
    wave_function: HashMap<u64, f64>,  // timeline_id -> amplitude
    
    /// Timeline counter
    timeline_counter: u64,
    
    /// State counter
    state_counter: u64,
    
    /// Total measurement collapses
    collapses: u64,
    
    /// Decoherence rate per step
    decoherence: f32,
}

impl QuantumTemporalSimulator {
    /// Create new quantum simulator
    pub fn new(decoherence_rate: f32) -> Self {
        QuantumTemporalSimulator {
            timelines: HashMap::new(),
            wave_function: HashMap::new(),
            timeline_counter: 1,
            state_counter: 1,
            collapses: 0,
            decoherence: decoherence_rate,
        }
    }
    
    /// Create new timeline in superposition
    pub fn create_timeline(&mut self, _description: Option<String>) -> u64 {
        let timeline_id = self.timeline_counter;
        let state_id = self.state_counter;
        self.timeline_counter += 1;
        self.state_counter += 1;
        
        let timeline = QuantumTimeline::new(timeline_id, state_id);
        self.timelines.insert(timeline_id, timeline);
        self.wave_function.insert(timeline_id, 1.0);
        
        timeline_id
    }
    
    /// Create branched timeline (superposition branch)
    pub fn branch_timeline(&mut self, parent_id: u64, probability: f64) -> Option<u64> {
        if !self.timelines.contains_key(&parent_id) {
            return None;
        }
        
        let timeline_id = self.timeline_counter;
        let state_id = self.state_counter;
        self.timeline_counter += 1;
        self.state_counter += 1;
        
        let timeline = QuantumTimeline::branch(parent_id, timeline_id, state_id, probability);
        self.timelines.insert(timeline_id, timeline);
        self.wave_function.insert(timeline_id, probability);
        
        Some(timeline_id)
    }
    
    /// Add event to timeline
    pub fn add_event(&mut self, timeline_id: u64, event_id: u64) -> bool {
        if let Some(tl) = self.timelines.get_mut(&timeline_id) {
            tl.add_event(event_id);
            true
        } else {
            false
        }
    }
    
    /// Set amplitude for state in timeline
    pub fn set_amplitude(&mut self, timeline_id: u64, event_id: u64, amplitude: f64) -> bool {
        if let Some(tl) = self.timelines.get_mut(&timeline_id) {
            tl.quantum_state.add_amplitude(event_id, amplitude);
            tl.quantum_state.normalize();
            true
        } else {
            false
        }
    }
    
    /// Measure all timelines (collapse wave function)
    pub fn measure(&mut self) -> Vec<(u64, u64)> {
        let mut results = Vec::new();
        
        for (timeline_id, timeline) in self.timelines.iter_mut() {
            if let Some(event_id) = timeline.quantum_state.measure() {
                results.push((*timeline_id, event_id));
                self.collapses += 1;
            }
        }
        
        results
    }
    
    /// Evolve superposition (apply decoherence)
    pub fn evolve_step(&mut self) {
        for (_, timeline) in self.timelines.iter_mut() {
            timeline.quantum_state.decoherence_rate(self.decoherence);
        }
    }
    
    /// Get total system probability
    pub fn total_probability(&self) -> f64 {
        self.timelines.values()
            .map(|tl| tl.total_probability())
            .sum()
    }
    
    /// Entangle two timelines
    pub fn entangle(&mut self, timeline_a: u64, timeline_b: u64) -> bool {
        if !self.timelines.contains_key(&timeline_a) || !self.timelines.contains_key(&timeline_b) {
            return false;
        }
        
        // Merge amplitudes (simple entanglement)
        if let Some(tl_a) = self.timelines.get(&timeline_a) {
            let amps_a = tl_a.quantum_state.amplitudes.clone();
            if let Some(tl_b) = self.timelines.get_mut(&timeline_b) {
                for (event_id, amp) in amps_a {
                    let current = tl_b.quantum_state.amplitudes.get(&event_id).copied().unwrap_or(0.0);
                    tl_b.quantum_state.add_amplitude(event_id, (current + amp) / 2.0);
                }
                tl_b.quantum_state.normalize();
            }
        }
        
        true
    }
    
    /// Get timeline count
    pub fn timeline_count(&self) -> usize {
        self.timelines.len()
    }
    
    /// Get average coherence across all timelines
    pub fn average_coherence(&self) -> f32 {
        if self.timelines.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = self.timelines.values()
            .map(|tl| tl.quantum_state.coherence)
            .sum();
        
        sum / self.timelines.len() as f32
    }
    
    /// Get collapse count
    pub fn collapse_count(&self) -> u64 {
        self.collapses
    }
}

impl Clone for QuantumTemporalSimulator {
    fn clone(&self) -> Self {
        QuantumTemporalSimulator {
            timelines: self.timelines.clone(),
            wave_function: self.wave_function.clone(),
            timeline_counter: self.timeline_counter,
            state_counter: self.state_counter,
            collapses: self.collapses,
            decoherence: self.decoherence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quantum_state_creation() {
        let state = QuantumState::new(1);
        assert_eq!(state.state_id, 1);
        assert_eq!(state.coherence, 1.0);
    }
    
    #[test]
    fn test_amplitude_and_probability() {
        let mut state = QuantumState::new(1);
        state.add_amplitude(1, 0.7);
        state.add_amplitude(2, 0.3);
        
        let prob1 = state.probability(1);
        assert!(prob1 < 0.6);  // ~0.49
    }
    
    #[test]
    fn test_normalization() {
        let mut state = QuantumState::new(1);
        state.add_amplitude(1, 2.0);
        state.add_amplitude(2, 2.0);
        state.normalize();
        
        let sum_sq: f64 = state.amplitudes.values()
            .map(|a| a * a)
            .sum();
        assert!((sum_sq - 1.0).abs() < 0.0001);
    }
    
    #[test]
    fn test_measurement_collapse() {
        let mut state = QuantumState::new(1);
        state.add_amplitude(1, 0.8);
        state.add_amplitude(2, 0.6);
        state.normalize();
        
        let result = state.measure();
        assert!(result.is_some());
        assert_eq!(state.coherence, 0.0);
    }
    
    #[test]
    fn test_decoherence() {
        let mut state = QuantumState::new(1);
        let initial = state.coherence;
        
        state.decoherence_rate(0.2);
        assert!(state.coherence < initial);
    }
    
    #[test]
    fn test_simulator_creation() {
        let sim = QuantumTemporalSimulator::new(0.01);
        assert_eq!(sim.timeline_count(), 0);
    }
    
    #[test]
    fn test_create_timeline() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let id = sim.create_timeline(None);
        
        assert_eq!(sim.timeline_count(), 1);
        assert!(id > 0);
    }
    
    #[test]
    fn test_branch_timeline() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let parent = sim.create_timeline(None);
        let branch = sim.branch_timeline(parent, 0.5);
        
        assert!(branch.is_some());
        assert_eq!(sim.timeline_count(), 2);
    }
    
    #[test]
    fn test_add_event() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let tl_id = sim.create_timeline(None);
        
        assert!(sim.add_event(tl_id, 100));
        assert!(!sim.add_event(999, 100));
    }
    
    #[test]
    fn test_set_amplitude() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let tl_id = sim.create_timeline(None);
        
        assert!(sim.set_amplitude(tl_id, 1, 0.7));
        assert!(!sim.set_amplitude(999, 1, 0.7));
    }
    
    #[test]
    fn test_measure() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let tl_id = sim.create_timeline(None);
        sim.set_amplitude(tl_id, 1, 0.8);
        
        let results = sim.measure();
        assert_eq!(results.len(), 1);
        assert_eq!(sim.collapse_count(), 1);
    }
    
    #[test]
    fn test_evolve_step() {
        let mut sim = QuantumTemporalSimulator::new(0.05);
        let tl_id = sim.create_timeline(None);
        
        if let Some(tl) = sim.timelines.get(&tl_id) {
            let before = tl.quantum_state.coherence;
            let _ = tl;
            
            sim.evolve_step();
            
            if let Some(tl) = sim.timelines.get(&tl_id) {
                assert!(tl.quantum_state.coherence < before);
            }
        }
    }
    
    #[test]
    fn test_entanglement() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let tl_a = sim.create_timeline(None);
        let tl_b = sim.create_timeline(None);
        
        sim.set_amplitude(tl_a, 1, 1.0);
        assert!(sim.entangle(tl_a, tl_b));
        
        if let Some(tl) = sim.timelines.get(&tl_b) {
            assert!(tl.quantum_state.amplitudes.contains_key(&1));
        }
    }
    
    #[test]
    fn test_average_coherence() {
        let mut sim = QuantumTemporalSimulator::new(0.02);
        
        let _tl1 = sim.create_timeline(None);
        let _tl2 = sim.create_timeline(None);
        
        sim.evolve_step();
        
        let avg = sim.average_coherence();
        assert!(avg < 1.0);
        assert!(avg >= 0.0);
    }
    
    #[test]
    fn test_total_probability() {
        let mut sim = QuantumTemporalSimulator::new(0.01);
        let tl = sim.create_timeline(None);
        
        sim.set_amplitude(tl, 1, 0.8);
        sim.set_amplitude(tl, 2, 0.6);
        
        let total = sim.total_probability();
        assert!(total > 0.0);
    }
}
