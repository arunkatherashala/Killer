/// Phase 8: Physics Engine
/// Einstein relativity and Planck quantum mechanics in temporal computation

/// Spacetime coordinate
#[derive(Clone, Copy, Debug)]
pub struct SpacetimeCoordinate {
    pub time: f64,      // t
    pub x: f64,         // spatial dimension
    pub y: f64,
    pub z: f64,
}

impl SpacetimeCoordinate {
    /// Create new coordinate
    pub fn new(t: f64, x: f64, y: f64, z: f64) -> Self {
        SpacetimeCoordinate { time: t, x, y, z }
    }
    
    /// Calculate spacetime interval (Minkowski metric)
    pub fn spacetime_interval(&self, other: &SpacetimeCoordinate) -> f64 {
        let dt = self.time - other.time;
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        
        // s^2 = c^2*t^2 - (x^2 + y^2 + z^2)
        // Using c=1 for simplicity
        dt * dt - (dx * dx + dy * dy + dz * dz)
    }
    
    /// Euclidean distance
    pub fn distance(&self, other: &SpacetimeCoordinate) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Relativistic event
#[derive(Clone, Debug)]
pub struct RelativisticEvent {
    pub event_id: u64,
    pub coordinate: SpacetimeCoordinate,
    pub energy: f64,        // Planck energy units
    pub velocity_fraction: f64,  // v/c (0.0 to 1.0)
    pub frequency: f64,     // Quantum frequency
}

impl RelativisticEvent {
    /// Create new relativistic event
    pub fn new(event_id: u64, coord: SpacetimeCoordinate, energy: f64) -> Self {
        RelativisticEvent {
            event_id,
            coordinate: coord,
            energy,
            velocity_fraction: 0.0,
            frequency: 0.0,
        }
    }
    
    /// Lorentz factor (gamma)
    pub fn lorentz_factor(&self) -> f64 {
        if self.velocity_fraction >= 1.0 {
            return f64::INFINITY;
        }
        
        let v_squared = self.velocity_fraction * self.velocity_fraction;
        1.0 / (1.0 - v_squared).sqrt()
    }
    
    /// Relativistic energy
    pub fn relativistic_energy(&self) -> f64 {
        self.energy * self.lorentz_factor()
    }
    
    /// De Broglie wavelength (quantum)
    pub fn de_broglie_wavelength(&self) -> f64 {
        if self.frequency == 0.0 {
            return f64::INFINITY;
        }
        
        // λ = h/p = h/(E/c)
        // Simplified: λ proportional to 1/frequency
        1.0 / self.frequency
    }
}

/// Quantum superposition state in relativistic context
#[derive(Clone, Debug)]
pub struct QuantumRelativistic {
    pub state_id: u64,
    pub events: Vec<RelativisticEvent>,
    pub amplitude: f64,
    pub phase: f64,
}

impl QuantumRelativistic {
    /// Create quantum state
    pub fn new(state_id: u64) -> Self {
        QuantumRelativistic {
            state_id,
            events: Vec::new(),
            amplitude: 1.0,
            phase: 0.0,
        }
    }
    
    /// Add relativistic event
    pub fn add_event(&mut self, event: RelativisticEvent) {
        self.events.push(event);
    }
    
    /// Calculate probability (amplitude squared)
    pub fn probability(&self) -> f64 {
        self.amplitude * self.amplitude
    }
    
    /// Planck action
    pub fn planck_action(&self) -> f64 {
        self.events.iter()
            .map(|e| e.energy * e.frequency)
            .sum()
    }
}

/// Time dilation effect
#[derive(Clone, Debug)]
pub struct TimeDilation {
    pub event_a: u64,
    pub event_b: u64,
    pub proper_time_a: f64,
    pub proper_time_b: f64,
    pub relative_velocity: f64,  // v/c
}

impl TimeDilation {
    /// Create dilation
    pub fn new(event_a: u64, event_b: u64, rel_vel: f64) -> Self {
        TimeDilation {
            event_a,
            event_b,
            proper_time_a: 0.0,
            proper_time_b: 0.0,
            relative_velocity: rel_vel.min(0.9999),
        }
    }
    
    /// Calculate time dilation factor
    pub fn dilation_factor(&self) -> f64 {
        let v_sq = self.relative_velocity * self.relative_velocity;
        (1.0 - v_sq).sqrt()
    }
    
    /// Dilated time for B as observed from A
    pub fn elapsed_from_a(&self, elapsed_b: f64) -> f64 {
        elapsed_b * self.dilation_factor()
    }
}

/// Physics Engine for temporal computing
pub struct PhysicsEngine {
    /// Relativistic events
    events: Vec<RelativisticEvent>,
    
    /// Quantum-relativistic states
    quantum_states: Vec<QuantumRelativistic>,
    
    /// Causality constraints (light cone)
    causality_violations: u64,
    
    /// Speed of light limit enforcement
    ftz_violations: u64,  // Faster-than-light violations
    
    /// Planck scale tracking
    planck_count: u64,
    
    /// Energy-momentum conservation
    total_energy: f64,
    total_momentum: f64,
    
    /// Event counter
    event_counter: u64,
}

impl PhysicsEngine {
    /// Create new physics engine
    pub fn new() -> Self {
        PhysicsEngine {
            events: Vec::new(),
            quantum_states: Vec::new(),
            causality_violations: 0,
            ftz_violations: 0,
            planck_count: 0,
            total_energy: 0.0,
            total_momentum: 0.0,
            event_counter: 1,
        }
    }
    
    /// Add relativistic event
    pub fn add_event(&mut self, coord: SpacetimeCoordinate, energy: f64, velocity_fraction: f64) -> u64 {
        let event_id = self.event_counter;
        self.event_counter += 1;
        
        let mut event = RelativisticEvent::new(event_id, coord, energy);
        event.velocity_fraction = velocity_fraction.min(0.9999);
        
        self.total_energy += event.relativistic_energy();
        self.total_momentum += velocity_fraction * energy;
        
        self.events.push(event);
        event_id
    }
    
    /// Check causality (light cone constraint)
    pub fn check_causality(&mut self, event_a: &RelativisticEvent, event_b: &RelativisticEvent) -> bool {
        let interval = event_a.coordinate.spacetime_interval(&event_b.coordinate);
        
        // Timelike interval: causally connected
        // Spacelike interval: causally disconnected
        if interval < 0.0 {
            // Spacelike separation - causality violated
            self.causality_violations += 1;
            return false;
        }
        
        true
    }
    
    /// Check for faster-than-light violations
    pub fn check_ftz(&mut self, event_a: &RelativisticEvent, event_b: &RelativisticEvent) -> bool {
        let distance = event_a.coordinate.distance(&event_b.coordinate);
        let time_diff = (event_a.coordinate.time - event_b.coordinate.time).abs();
        
        if time_diff == 0.0 {
            return false;
        }
        
        let velocity = distance / time_diff;
        
        // In normalized units, FTL is velocity > 1.0 (c)
        if velocity > 1.0001 {
            self.ftz_violations += 1;
            return false;
        }
        
        true
    }
    
    /// Enforce causality on new event
    pub fn enforce_causality(&mut self, new_event: &RelativisticEvent) -> bool {
        let events_clone: Vec<RelativisticEvent> = self.events.clone();
        
        for existing in events_clone.iter() {
            if !self.check_causality(existing, new_event) {
                return false;
            }
            
            if !self.check_ftz(existing, new_event) {
                return false;
            }
        }
        
        true
    }
    
    /// Create quantum-relativistic state
    pub fn create_quantum_state(&mut self) -> u64 {
        let state_id = self.planck_count;
        self.planck_count += 1;
        
        let state = QuantumRelativistic::new(state_id);
        self.quantum_states.push(state);
        
        state_id as u64
    }
    
    /// Calculate time dilation between events
    pub fn calculate_dilation(&self, event_a_id: u64, event_b_id: u64, relative_velocity: f64) -> Option<TimeDilation> {
        let event_a = self.events.iter().find(|e| e.event_id == event_a_id)?;
        let event_b = self.events.iter().find(|e| e.event_id == event_b_id)?;
        
        let mut dilation = TimeDilation::new(event_a_id, event_b_id, relative_velocity);
        
        dilation.proper_time_a = event_a.coordinate.time;
        dilation.proper_time_b = event_b.coordinate.time;
        
        Some(dilation)
    }
    
    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    
    /// Get quantum state count
    pub fn state_count(&self) -> usize {
        self.quantum_states.len()
    }
    
    /// Get causality violation count
    pub fn causality_violations(&self) -> u64 {
        self.causality_violations
    }
    
    /// Get FTL violation count
    pub fn ftl_violations(&self) -> u64 {
        self.ftz_violations
    }
    
    /// Energy conservation check
    pub fn energy_conserved(&self) -> bool {
        // Simplified: check if total energy is positive
        self.total_energy.is_finite() && self.total_energy >= 0.0
    }
    
    /// Get total system energy
    pub fn total_system_energy(&self) -> f64 {
        self.total_energy
    }
    
    /// Get consistency ratio
    pub fn consistency_ratio(&self) -> f32 {
        if self.event_count() == 0 {
            return 1.0;
        }
        
        let violations = (self.causality_violations + self.ftz_violations) as f32;
        let total = self.event_count() as f32;
        
        ((total - violations) / total).max(0.0)
    }
}

impl Clone for PhysicsEngine {
    fn clone(&self) -> Self {
        PhysicsEngine {
            events: self.events.clone(),
            quantum_states: self.quantum_states.clone(),
            causality_violations: self.causality_violations,
            ftz_violations: self.ftz_violations,
            planck_count: self.planck_count,
            total_energy: self.total_energy,
            total_momentum: self.total_momentum,
            event_counter: self.event_counter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coordinate_creation() {
        let coord = SpacetimeCoordinate::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(coord.time, 1.0);
    }
    
    #[test]
    fn test_spacetime_interval() {
        let coord1 = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let coord2 = SpacetimeCoordinate::new(1.0, 0.0, 0.0, 0.0);
        
        let interval = coord1.spacetime_interval(&coord2);
        assert_eq!(interval, 1.0);
    }
    
    #[test]
    fn test_distance() {
        let coord1 = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let coord2 = SpacetimeCoordinate::new(0.0, 3.0, 4.0, 0.0);
        
        let dist = coord1.distance(&coord2);
        assert!((dist - 5.0).abs() < 0.001);
    }
    
    #[test]
    fn test_relativistic_event_creation() {
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let event = RelativisticEvent::new(1, coord, 100.0);
        
        assert_eq!(event.energy, 100.0);
    }
    
    #[test]
    fn test_lorentz_factor() {
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let mut event = RelativisticEvent::new(1, coord, 100.0);
        
        event.velocity_fraction = 0.0;
        assert!((event.lorentz_factor() - 1.0).abs() < 0.001);
        
        event.velocity_fraction = 0.9;
        let gamma = event.lorentz_factor();
        assert!(gamma > 1.0);
    }
    
    #[test]
    fn test_relativistic_energy() {
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let mut event = RelativisticEvent::new(1, coord, 100.0);
        
        event.velocity_fraction = 0.0;
        let energy = event.relativistic_energy();
        assert_eq!(energy, 100.0);
    }
    
    #[test]
    fn test_time_dilation() {
        let dilation = TimeDilation::new(1, 2, 0.8);
        let factor = dilation.dilation_factor();
        
        assert!(factor < 1.0);
        assert!(factor > 0.0);
    }
    
    #[test]
    fn test_physics_engine_creation() {
        let engine = PhysicsEngine::new();
        assert_eq!(engine.event_count(), 0);
    }
    
    #[test]
    fn test_add_event() {
        let mut engine = PhysicsEngine::new();
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        
        let event_id = engine.add_event(coord, 100.0, 0.5);
        assert_eq!(engine.event_count(), 1);
        assert!(event_id > 0);
    }
    
    #[test]
    fn test_causality_check_timelike() {
        let _engine = PhysicsEngine::new();
        
        // Timelike separation (causally connected)
        let coord1 = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let coord2 = SpacetimeCoordinate::new(1.0, 0.5, 0.0, 0.0);
        
        let _event1 = RelativisticEvent::new(1, coord1, 100.0);
        let _event2 = RelativisticEvent::new(2, coord2, 100.0);
        
        let interval = coord1.spacetime_interval(&coord2);
        assert!(interval > 0.0);  // Timelike
    }
    
    #[test]
    fn test_create_quantum_state() {
        let mut engine = PhysicsEngine::new();
        let state_id = engine.create_quantum_state();
        
        assert_eq!(engine.state_count(), 1);
        assert_eq!(state_id, 0);
    }
    
    #[test]
    fn test_energy_conservation() {
        let engine = PhysicsEngine::new();
        assert!(engine.energy_conserved());
    }
    
    #[test]
    fn test_consistency_ratio() {
        let mut engine = PhysicsEngine::new();
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        
        engine.add_event(coord, 100.0, 0.1);
        let ratio = engine.consistency_ratio();
        
        assert!(ratio >= 0.0 && ratio <= 1.0);
    }
    
    #[test]
    fn test_quantum_relativistic_state() {
        let state = QuantumRelativistic::new(1);
        assert_eq!(state.state_id, 1);
        assert_eq!(state.events.len(), 0);
    }
    
    #[test]
    fn test_de_broglie_wavelength() {
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        let mut event = RelativisticEvent::new(1, coord, 100.0);
        
        event.frequency = 2.0;
        let wavelength = event.de_broglie_wavelength();
        assert!(wavelength > 0.0);
    }
    
    #[test]
    fn test_total_system_energy() {
        let mut engine = PhysicsEngine::new();
        let coord = SpacetimeCoordinate::new(0.0, 0.0, 0.0, 0.0);
        
        engine.add_event(coord, 100.0, 0.1);
        let energy = engine.total_system_energy();
        assert!(energy > 0.0);
    }
}
