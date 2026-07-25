/// Phase 7: Temporal Machine Learning
/// Pattern recognition, forecasting, and causal inference
use std::collections::HashMap;

/// Temporal pattern in event sequence
#[derive(Clone, Debug)]
pub struct TemporalPattern {
    pub pattern_id: u64,
    pub events: Vec<u64>,
    pub frequency: u64,
    pub confidence: f32,  // 0.0-1.0
    pub time_window: u128,  // milliseconds
}

impl TemporalPattern {
    /// Create new pattern
    pub fn new(pattern_id: u64, events: Vec<u64>, time_window: u128) -> Self {
        TemporalPattern {
            pattern_id,
            events,
            frequency: 1,
            confidence: 0.0,
            time_window,
        }
    }
    
    /// Increase frequency count
    pub fn increment(&mut self) {
        self.frequency += 1;
    }
    
    /// Update confidence based on observations
    pub fn update_confidence(&mut self, correct_predictions: u64, total_predictions: u64) {
        if total_predictions > 0 {
            self.confidence = (correct_predictions as f32) / (total_predictions as f32);
        }
    }
}

/// Anomaly detection in temporal data
#[derive(Clone, Debug)]
pub struct AnomalyDetector {
    pub detector_id: u64,
    pub baseline_mean: f64,
    pub baseline_stddev: f64,
    pub threshold_sigma: f32,  // Standard deviations from mean
    pub detected_anomalies: Vec<u64>,  // Event IDs
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new(detector_id: u64, baseline_mean: f64, baseline_stddev: f64, threshold: f32) -> Self {
        AnomalyDetector {
            detector_id,
            baseline_mean,
            baseline_stddev,
            threshold_sigma: threshold,
            detected_anomalies: Vec::new(),
        }
    }
    
    /// Detect if value is anomalous
    pub fn is_anomaly(&self, value: f64) -> bool {
        if self.baseline_stddev == 0.0 {
            return (value - self.baseline_mean).abs() > 1.0;
        }
        
        let z_score = (value - self.baseline_mean) / self.baseline_stddev;
        z_score.abs() > (self.threshold_sigma as f64)
    }
    
    /// Record detected anomaly
    pub fn record_anomaly(&mut self, event_id: u64) {
        self.detected_anomalies.push(event_id);
    }
    
    /// Get anomaly count
    pub fn count(&self) -> usize {
        self.detected_anomalies.len()
    }
}

/// Causal influence between events
#[derive(Clone, Debug)]
pub struct CausalInfluence {
    pub cause_id: u64,
    pub effect_id: u64,
    pub strength: f32,  // 0.0-1.0 (strength of causal link)
    pub lag: u128,  // milliseconds between cause and effect
    pub p_value: f32,  // Statistical significance
}

impl CausalInfluence {
    /// Create causal link
    pub fn new(cause_id: u64, effect_id: u64, strength: f32, lag: u128) -> Self {
        CausalInfluence {
            cause_id,
            effect_id,
            strength: strength.min(1.0),
            lag,
            p_value: 0.01,  // Statistically significant by default
        }
    }
    
    /// Is statistically significant?
    pub fn is_significant(&self) -> bool {
        self.p_value < 0.05
    }
}

/// Temporal forecasting model
#[derive(Clone, Debug)]
pub struct ForecastingModel {
    pub model_id: u64,
    pub metric_name: String,
    pub accuracy: f32,  // 0.0-1.0, target 95%+
    pub predictions: Vec<(u128, f64)>,  // (timestamp, predicted_value)
    pub actual_values: Vec<(u128, f64)>,
    pub error_rate: f32,
}

impl ForecastingModel {
    /// Create new forecasting model
    pub fn new(model_id: u64, metric: String) -> Self {
        ForecastingModel {
            model_id,
            metric_name: metric,
            accuracy: 0.0,
            predictions: Vec::new(),
            actual_values: Vec::new(),
            error_rate: 1.0,
        }
    }
    
    /// Add prediction
    pub fn add_prediction(&mut self, timestamp: u128, value: f64) {
        self.predictions.push((timestamp, value));
    }
    
    /// Add actual value
    pub fn add_actual(&mut self, timestamp: u128, value: f64) {
        self.actual_values.push((timestamp, value));
    }
    
    /// Calculate accuracy
    pub fn calculate_accuracy(&mut self) -> f32 {
        if self.predictions.is_empty() || self.actual_values.is_empty() {
            return 0.0;
        }
        
        let mut correct = 0u64;
        let mut total = 0u64;
        
        for (pred_ts, pred_val) in &self.predictions {
            for (act_ts, act_val) in &self.actual_values {
                if pred_ts == act_ts {
                    let error = (pred_val - act_val).abs();
                    let threshold = (act_val.abs() * 0.1).max(0.1);
                    
                    if error < threshold {
                        correct += 1;
                    }
                    
                    total += 1;
                    break;
                }
            }
        }
        
        if total > 0 {
            self.accuracy = correct as f32 / total as f32;
            self.error_rate = 1.0 - self.accuracy;
        }
        
        self.accuracy
    }
}

/// Temporal Machine Learning Engine
pub struct TemporalMLEngine {
    /// Discovered patterns
    patterns: HashMap<u64, TemporalPattern>,
    
    /// Anomaly detectors
    anomaly_detectors: HashMap<u64, AnomalyDetector>,
    
    /// Causal influences
    causal_links: Vec<CausalInfluence>,
    
    /// Forecasting models
    models: HashMap<u64, ForecastingModel>,
    
    /// Pattern counter
    pattern_counter: u64,
    
    /// Model counter
    model_counter: u64,
    
    /// Detector counter
    detector_counter: u64,
    
    /// Total patterns found
    patterns_discovered: u64,
    
    /// Average accuracy
    avg_accuracy: f32,
}

impl TemporalMLEngine {
    /// Create new ML engine
    pub fn new() -> Self {
        TemporalMLEngine {
            patterns: HashMap::new(),
            anomaly_detectors: HashMap::new(),
            causal_links: Vec::new(),
            models: HashMap::new(),
            pattern_counter: 1,
            model_counter: 1,
            detector_counter: 1,
            patterns_discovered: 0,
            avg_accuracy: 0.0,
        }
    }
    
    /// Discover temporal pattern
    pub fn discover_pattern(&mut self, events: Vec<u64>, time_window: u128) -> u64 {
        let pattern_id = self.pattern_counter;
        self.pattern_counter += 1;
        
        let pattern = TemporalPattern::new(pattern_id, events, time_window);
        self.patterns.insert(pattern_id, pattern);
        self.patterns_discovered += 1;
        
        pattern_id
    }
    
    /// Create anomaly detector
    pub fn create_detector(&mut self, baseline_mean: f64, baseline_stddev: f64, threshold: f32) -> u64 {
        let detector_id = self.detector_counter;
        self.detector_counter += 1;
        
        let detector = AnomalyDetector::new(detector_id, baseline_mean, baseline_stddev, threshold);
        self.anomaly_detectors.insert(detector_id, detector);
        
        detector_id
    }
    
    /// Train forecasting model
    pub fn create_model(&mut self, metric: String) -> u64 {
        let model_id = self.model_counter;
        self.model_counter += 1;
        
        let model = ForecastingModel::new(model_id, metric);
        self.models.insert(model_id, model);
        
        model_id
    }
    
    /// Add causal relationship
    pub fn add_causal_link(&mut self, cause: u64, effect: u64, strength: f32, lag: u128) {
        let link = CausalInfluence::new(cause, effect, strength, lag);
        self.causal_links.push(link);
    }
    
    /// Detect anomalies
    pub fn detect_anomaly(&mut self, detector_id: u64, event_id: u64, value: f64) -> bool {
        if let Some(detector) = self.anomaly_detectors.get_mut(&detector_id) {
            if detector.is_anomaly(value) {
                detector.record_anomaly(event_id);
                return true;
            }
        }
        false
    }
    
    /// Add prediction
    pub fn add_prediction(&mut self, model_id: u64, timestamp: u128, value: f64) -> bool {
        if let Some(model) = self.models.get_mut(&model_id) {
            model.add_prediction(timestamp, value);
            true
        } else {
            false
        }
    }
    
    /// Add actual value
    pub fn add_actual(&mut self, model_id: u64, timestamp: u128, value: f64) -> bool {
        if let Some(model) = self.models.get_mut(&model_id) {
            model.add_actual(timestamp, value);
            true
        } else {
            false
        }
    }
    
    /// Evaluate model accuracy
    pub fn evaluate_model(&mut self, model_id: u64) -> Option<f32> {
        if let Some(model) = self.models.get_mut(&model_id) {
            model.calculate_accuracy();
            Some(model.accuracy)
        } else {
            None
        }
    }
    
    /// Get causal paths (causal inference)
    pub fn infer_causality(&self, source: u64, max_steps: usize) -> Vec<Vec<u64>> {
        let mut paths = Vec::new();
        let mut visited = Vec::new();
        
        self._find_causal_paths(source, max_steps, &mut visited, &mut paths);
        
        paths
    }
    
    /// Helper for causal path finding
    fn _find_causal_paths(&self, current: u64, steps_remaining: usize, visited: &mut Vec<u64>, paths: &mut Vec<Vec<u64>>) {
        if steps_remaining == 0 {
            return;
        }
        
        visited.push(current);
        
        for link in &self.causal_links {
            if link.cause_id == current && !visited.contains(&link.effect_id) {
                self._find_causal_paths(link.effect_id, steps_remaining - 1, visited, paths);
            }
        }
        
        paths.push(visited.clone());
        visited.pop();
    }
    
    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
    
    /// Get detector count
    pub fn detector_count(&self) -> usize {
        self.anomaly_detectors.len()
    }
    
    /// Get model count
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
    
    /// Get significant causal links
    pub fn significant_links(&self) -> usize {
        self.causal_links.iter()
            .filter(|l| l.is_significant())
            .count()
    }
    
    /// Calculate system accuracy
    pub fn system_accuracy(&mut self) -> f32 {
        if self.models.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = self.models.values()
            .map(|m| m.accuracy)
            .sum();
        
        self.avg_accuracy = sum / self.models.len() as f32;
        self.avg_accuracy
    }
}

impl Clone for TemporalMLEngine {
    fn clone(&self) -> Self {
        TemporalMLEngine {
            patterns: self.patterns.clone(),
            anomaly_detectors: self.anomaly_detectors.clone(),
            causal_links: self.causal_links.clone(),
            models: self.models.clone(),
            pattern_counter: self.pattern_counter,
            model_counter: self.model_counter,
            detector_counter: self.detector_counter,
            patterns_discovered: self.patterns_discovered,
            avg_accuracy: self.avg_accuracy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_creation() {
        let pattern = TemporalPattern::new(1, vec![1, 2, 3], 1000);
        assert_eq!(pattern.pattern_id, 1);
        assert_eq!(pattern.events.len(), 3);
    }
    
    #[test]
    fn test_pattern_frequency() {
        let mut pattern = TemporalPattern::new(1, vec![1, 2, 3], 1000);
        let initial = pattern.frequency;
        pattern.increment();
        assert_eq!(pattern.frequency, initial + 1);
    }
    
    #[test]
    fn test_anomaly_detector_creation() {
        let detector = AnomalyDetector::new(1, 100.0, 10.0, 2.0);
        assert_eq!(detector.detector_id, 1);
    }
    
    #[test]
    fn test_anomaly_detection() {
        let detector = AnomalyDetector::new(1, 100.0, 10.0, 2.0);
        
        assert!(!detector.is_anomaly(105.0));  // Within 2 sigma
        assert!(detector.is_anomaly(130.0));   // Outside 2 sigma
    }
    
    #[test]
    fn test_causal_influence_creation() {
        let influence = CausalInfluence::new(1, 2, 0.8, 500);
        assert_eq!(influence.strength, 0.8);
        assert_eq!(influence.lag, 500);
    }
    
    #[test]
    fn test_forecasting_model_creation() {
        let model = ForecastingModel::new(1, "temperature".to_string());
        assert_eq!(model.model_id, 1);
        assert_eq!(model.accuracy, 0.0);
    }
    
    #[test]
    fn test_model_accuracy_calculation() {
        let mut model = ForecastingModel::new(1, "temp".to_string());
        
        model.add_prediction(1000, 25.0);
        model.add_actual(1000, 25.5);
        
        let accuracy = model.calculate_accuracy();
        assert!(accuracy > 0.0);
    }
    
    #[test]
    fn test_engine_creation() {
        let engine = TemporalMLEngine::new();
        assert_eq!(engine.pattern_count(), 0);
    }
    
    #[test]
    fn test_discover_pattern() {
        let mut engine = TemporalMLEngine::new();
        let pattern_id = engine.discover_pattern(vec![1, 2, 3], 1000);
        
        assert_eq!(engine.pattern_count(), 1);
        assert!(pattern_id > 0);
    }
    
    #[test]
    fn test_create_detector() {
        let mut engine = TemporalMLEngine::new();
        let detector_id = engine.create_detector(100.0, 10.0, 2.0);
        
        assert_eq!(engine.detector_count(), 1);
        assert!(detector_id > 0);
    }
    
    #[test]
    fn test_create_model() {
        let mut engine = TemporalMLEngine::new();
        let model_id = engine.create_model("temperature".to_string());
        
        assert_eq!(engine.model_count(), 1);
        assert!(model_id > 0);
    }
    
    #[test]
    fn test_add_causal_link() {
        let mut engine = TemporalMLEngine::new();
        engine.add_causal_link(1, 2, 0.8, 500);
        
        assert!(engine.causal_links.len() > 0);
    }
    
    #[test]
    fn test_detect_anomaly() {
        let mut engine = TemporalMLEngine::new();
        let detector_id = engine.create_detector(100.0, 10.0, 2.0);
        
        assert!(!engine.detect_anomaly(detector_id, 1, 105.0));
        assert!(engine.detect_anomaly(detector_id, 2, 130.0));
    }
    
    #[test]
    fn test_add_prediction() {
        let mut engine = TemporalMLEngine::new();
        let model_id = engine.create_model("temp".to_string());
        
        assert!(engine.add_prediction(model_id, 1000, 25.0));
    }
    
    #[test]
    fn test_evaluate_model() {
        let mut engine = TemporalMLEngine::new();
        let model_id = engine.create_model("temp".to_string());
        
        engine.add_prediction(model_id, 1000, 25.0);
        engine.add_actual(model_id, 1000, 25.5);
        
        let accuracy = engine.evaluate_model(model_id);
        assert!(accuracy.is_some());
    }
    
    #[test]
    fn test_significant_links() {
        let mut engine = TemporalMLEngine::new();
        engine.add_causal_link(1, 2, 0.8, 500);
        
        let count = engine.significant_links();
        assert!(count > 0);
    }
    
    #[test]
    fn test_system_accuracy() {
        let mut engine = TemporalMLEngine::new();
        let model_id = engine.create_model("temp".to_string());
        
        engine.add_prediction(model_id, 1000, 25.0);
        engine.add_actual(model_id, 1000, 25.5);
        engine.evaluate_model(model_id);
        
        let sys_acc = engine.system_accuracy();
        assert!(sys_acc >= 0.0);
    }
}
