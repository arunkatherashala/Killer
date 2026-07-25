use crate::value::Value;
use crate::error::VmError;
use std::collections::HashMap;

/// Manages generator state and iteration
pub struct GeneratorManager {
    generator_states: HashMap<String, (Vec<Value>, usize)>,
    gen_counter: usize,
    yielded_values: Vec<Value>,  // Accumulate values during function execution
}

impl GeneratorManager {
    /// Create a new generator manager
    pub fn new() -> Self {
        Self {
            generator_states: HashMap::new(),
            gen_counter: 0,
            yielded_values: Vec::new(),
        }
    }

    /// Register a new generator with collected yielded values
    pub fn create_generator(&mut self, values: Vec<Value>) -> String {
        let gen_id = format!("gen_{}", self.gen_counter);
        self.gen_counter += 1;
        self.generator_states.insert(gen_id.clone(), (values, 0));
        gen_id
    }

    /// Push a yielded value during function execution
    pub fn push_yield(&mut self, value: Value) {
        self.yielded_values.push(value);
    }

    /// Get all yielded values and clear the buffer
    pub fn take_yielded_values(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.yielded_values)
    }

    /// Get the next value from a generator
    pub fn get_next(&mut self, gen_id: &str, default: Option<Value>) -> Result<Value, VmError> {
        match self.generator_states.get_mut(gen_id) {
            Some((values, idx)) => {
                let result = if *idx < values.len() {
                    values[*idx].clone()
                } else if let Some(def) = default {
                    def
                } else {
                    Value::Null
                };
                *idx += 1;
                Ok(result)
            }
            None => Err(VmError::runtime_error(format!(
                "Generator {} not found",
                gen_id
            ))),
        }
    }

    /// Check if a generator has more values
    pub fn has_next(&self, gen_id: &str) -> bool {
        match self.generator_states.get(gen_id) {
            Some((values, idx)) => *idx < values.len(),
            None => false,
        }
    }

    /// Get remaining values in a generator
    pub fn remaining(&self, gen_id: &str) -> usize {
        match self.generator_states.get(gen_id) {
            Some((values, idx)) => {
                if *idx < values.len() {
                    values.len() - *idx
                } else {
                    0
                }
            }
            None => 0,
        }
    }

    /// Reset a generator to the beginning
    pub fn reset(&mut self, gen_id: &str) -> Result<(), VmError> {
        match self.generator_states.get_mut(gen_id) {
            Some((_values, idx)) => {
                *idx = 0;
                Ok(())
            }
            None => Err(VmError::runtime_error(format!(
                "Generator {} not found",
                gen_id
            ))),
        }
    }

    /// Get the current index of a generator
    pub fn current_index(&self, gen_id: &str) -> Option<usize> {
        self.generator_states.get(gen_id).map(|(_, idx)| *idx)
    }

    /// Get the total number of values in a generator
    pub fn total_values(&self, gen_id: &str) -> Option<usize> {
        self.generator_states.get(gen_id).map(|(values, _)| values.len())
    }

    /// Remove a generator (cleanup)
    pub fn remove(&mut self, gen_id: &str) -> bool {
        self.generator_states.remove(gen_id).is_some()
    }

    /// Clear all generators
    pub fn clear(&mut self) {
        self.generator_states.clear();
        self.gen_counter = 0;
    }

    /// Get statistics about generators
    pub fn stats(&self) -> GeneratorStats {
        let total_generators = self.generator_states.len();
        let total_values: usize = self.generator_states
            .values()
            .map(|(values, _)| values.len())
            .sum();
        let total_consumed: usize = self.generator_states
            .values()
            .map(|(_, idx)| *idx)
            .sum();

        GeneratorStats {
            total_generators,
            total_values,
            total_consumed,
        }
    }
}

/// Statistics about active generators
#[derive(Debug, Clone)]
pub struct GeneratorStats {
    pub total_generators: usize,
    pub total_values: usize,
    pub total_consumed: usize,
}

impl Default for GeneratorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_generator() {
        let mut gm = GeneratorManager::new();
        let values = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let gen_id = gm.create_generator(values);
        
        assert!(gen_id.starts_with("gen_"));
        assert!(gm.has_next(&gen_id));
    }

    #[test]
    fn test_generator_next() {
        let mut gm = GeneratorManager::new();
        let values = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let gen_id = gm.create_generator(values);
        
        let v1 = gm.get_next(&gen_id, None).unwrap();
        assert_eq!(v1, Value::Number(1.0));
        
        let v2 = gm.get_next(&gen_id, None).unwrap();
        assert_eq!(v2, Value::Number(2.0));
        
        let v3 = gm.get_next(&gen_id, None).unwrap();
        assert_eq!(v3, Value::Number(3.0));
        
        // Should return null when exhausted
        let v4 = gm.get_next(&gen_id, None).unwrap();
        assert_eq!(v4, Value::Null);
    }

    #[test]
    fn test_generator_exhaustion() {
        let mut gm = GeneratorManager::new();
        let values = vec![Value::Number(1.0)];
        let gen_id = gm.create_generator(values);
        
        assert!(gm.has_next(&gen_id));
        gm.get_next(&gen_id, None).ok();
        assert!(!gm.has_next(&gen_id));
    }

    #[test]
    fn test_generator_remaining() {
        let mut gm = GeneratorManager::new();
        let values = vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ];
        let gen_id = gm.create_generator(values);
        
        assert_eq!(gm.remaining(&gen_id), 4);
        gm.get_next(&gen_id, None).ok();
        assert_eq!(gm.remaining(&gen_id), 3);
        gm.get_next(&gen_id, None).ok();
        assert_eq!(gm.remaining(&gen_id), 2);
    }

    #[test]
    fn test_generator_reset() {
        let mut gm = GeneratorManager::new();
        let values = vec![Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)];
        let gen_id = gm.create_generator(values);
        
        gm.get_next(&gen_id, None).ok();
        gm.get_next(&gen_id, None).ok();
        assert_eq!(gm.current_index(&gen_id), Some(2));
        
        gm.reset(&gen_id).ok();
        assert_eq!(gm.current_index(&gen_id), Some(0));
    }

    #[test]
    fn test_generator_default_value() {
        let mut gm = GeneratorManager::new();
        let values = vec![Value::Number(1.0)];
        let gen_id = gm.create_generator(values);
        
        gm.get_next(&gen_id, None).ok();
        let v = gm.get_next(&gen_id, Some(Value::Str("default".to_string()))).unwrap();
        assert_eq!(v, Value::Str("default".to_string()));
    }

    #[test]
    fn test_generator_stats() {
        let mut gm = GeneratorManager::new();
        
        let gen1 = gm.create_generator(vec![
            Value::Number(1.0),
            Value::Number(2.0),
        ]);
        let gen2 = gm.create_generator(vec![
            Value::Number(3.0),
            Value::Number(4.0),
            Value::Number(5.0),
        ]);
        
        gm.get_next(&gen1, None).ok();
        gm.get_next(&gen2, None).ok();
        gm.get_next(&gen2, None).ok();
        
        let stats = gm.stats();
        assert_eq!(stats.total_generators, 2);
        assert_eq!(stats.total_values, 5);
        assert_eq!(stats.total_consumed, 3);
    }

    #[test]
    fn test_generator_remove() {
        let mut gm = GeneratorManager::new();
        let gen_id = gm.create_generator(vec![Value::Number(1.0)]);
        
        assert!(gm.has_next(&gen_id));
        gm.remove(&gen_id);
        assert!(!gm.has_next(&gen_id));
    }

    #[test]
    fn test_generator_clear() {
        let mut gm = GeneratorManager::new();
        gm.create_generator(vec![Value::Number(1.0)]);
        gm.create_generator(vec![Value::Number(2.0)]);
        
        let stats = gm.stats();
        assert_eq!(stats.total_generators, 2);
        
        gm.clear();
        let stats = gm.stats();
        assert_eq!(stats.total_generators, 0);
    }
}
