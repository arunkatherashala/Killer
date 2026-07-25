// AI Module Declaration
// src/ai/mod.rs
//
// Main AI subsystem module that brings everything together

pub mod cache;
pub mod config;
pub mod error;
pub mod providers;
pub mod runtime;
pub mod utils;
pub mod quantization;
pub mod batching;
pub mod optimizer;

pub use cache::AICache;
pub use config::AIConfig;
pub use error::{AIError, AIResult};
pub use runtime::AIRuntime;
pub use providers::{Provider, ProviderManager};
pub use quantization::{QuantizationConfig, QuantizedModel, QuantizationPrecision, QuantizationCache};
pub use batching::{BatchConfig, BatchProcessor, Pipeline, BatchMetrics};
pub use optimizer::{AIOptimizer, OptimizerConfig, OptimizationResults, OptimizationImpact, AdaptiveOptimizer};

use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIProvider {
    OpenAI,
    Local,
    Anthropic,
}

impl AIProvider {
    pub fn as_str(&self) -> &str {
        match self {
            AIProvider::OpenAI => "openai",
            AIProvider::Local => "local",
            AIProvider::Anthropic => "anthropic",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, AIError> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(AIProvider::OpenAI),
            "local" => Ok(AIProvider::Local),
            "anthropic" => Ok(AIProvider::Anthropic),
            _ => Err(AIError::provider_not_found(s)),
        }
    }
}

/// Statistics for AI operations
#[derive(Debug, Clone, Default)]
pub struct AIStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_latency_ms: u64,
}

impl AIStats {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / total as f64) * 100.0
        }
    }

    pub fn average_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.total_requests as f64
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
}

/// Result of a classification operation
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    pub category: String,
    pub confidence: f64,
    pub all_scores: HashMap<String, f64>,
}

impl ClassifyResult {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(
            "category".to_string(),
            Value::Str(self.category.clone()),
        );
        map.insert("confidence".to_string(), Value::Number(self.confidence));

        let scores: HashMap<String, Value> = self
            .all_scores
            .iter()
            .map(|(k, v)| (k.clone(), Value::Number(*v)))
            .collect();
        map.insert("all_scores".to_string(), Value::Dict(Box::new(scores)));

        Value::Dict(Box::new(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_provider_from_str() {
        assert_eq!(AIProvider::from_str("openai").unwrap(), AIProvider::OpenAI);
        assert_eq!(AIProvider::from_str("local").unwrap(), AIProvider::Local);
        assert!(AIProvider::from_str("invalid").is_err());
    }

    #[test]
    fn test_ai_stats() {
        let mut stats = AIStats::default();
        stats.total_requests = 100;
        stats.successful_requests = 95;
        stats.cache_hits = 50;
        stats.cache_misses = 50;

        assert_eq!(stats.cache_hit_rate(), 50.0);
        assert!(stats.success_rate() > 94.0 && stats.success_rate() < 96.0);
    }

    #[test]
    fn test_classify_result_to_value() {
        let mut scores = HashMap::new();
        scores.insert("positive".to_string(), 0.8);
        scores.insert("negative".to_string(), 0.2);

        let result = ClassifyResult {
            category: "positive".to_string(),
            confidence: 0.8,
            all_scores: scores,
        };

        let value = result.to_value();
        match value {
            Value::Dict(map) => {
                assert!(map.contains_key("category"));
                assert!(map.contains_key("confidence"));
            }
            _ => panic!("Expected dict"),
        }
    }
}
