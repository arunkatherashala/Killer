// AI Runtime Core with GHOST & ASSASSIN Layers
// src/ai/runtime.rs
//
// Core AIRuntime implementation for managing AI operations
// GHOST: Monitors all processes, predicts errors, optimizes performance
// ASSASSIN: Protects security, prevents attacks, audits all operations

use crate::value::Value;
use std::collections::{HashMap, VecDeque};
use std::time::{Instant, SystemTime};

use super::{AICache, AIConfig, AIStats, ClassifyResult};
use super::providers::ProviderManager;

/// GHOST Layer: Performance Monitoring & Error Prediction
/// Monitors all AI operations to predict and prevent errors
#[derive(Debug, Clone)]
pub struct GhostMonitor {
    total_operations: u64,
    error_predictions: VecDeque<ErrorPrediction>,
    process_metrics: HashMap<String, ProcessMetrics>,
    latency_history: Vec<u64>,
    error_rate_threshold: f64,
    max_predictions: usize,
}

#[derive(Debug, Clone)]
pub struct ErrorPrediction {
    pub operation_id: u64,
    pub risk_level: f64,        // 0.0 - 1.0
    pub error_type: String,
    pub suggested_action: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ProcessMetrics {
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub total_latency_ms: u64,
    pub peak_latency_ms: u64,
    pub min_latency_ms: u64,
}

/// ASSASSIN Layer: Security Protection & Attack Prevention
/// Protects against attacks, validates inputs, audits all operations
#[derive(Debug, Clone)]
pub struct AssassinShield {
    rate_limit_per_second: u64,
    request_times: VecDeque<SystemTime>,
    audit_log: Vec<SecurityEvent>,
    blocked_attacks: u64,
    prompt_blocklist: Vec<String>,
    active_threats: HashMap<String, ThreatLevel>,
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: SystemTime,
    pub operation: String,
    pub status: String,           // "allowed", "blocked", "suspicious"
    pub details: String,
    pub severity: u32,            // 0-10
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    Safe = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Main AI Runtime with GHOST & ASSASSIN integration
/// Manages provider selection, caching, inference, monitoring, and security
pub struct AIRuntime {
    config: AIConfig,
    providers: ProviderManager,
    cache: AICache,
    stats: AIStats,
    ghost: GhostMonitor,
    assassin: AssassinShield,
}

impl GhostMonitor {
    pub fn new() -> Self {
        GhostMonitor {
            total_operations: 0,
            error_predictions: VecDeque::new(),
            process_metrics: HashMap::new(),
            latency_history: Vec::new(),
            error_rate_threshold: 0.1, // 10% error rate threshold
            max_predictions: 1000,
        }
    }

    /// Predict potential errors based on historical patterns
    pub fn predict_errors(&mut self, operation: &str, current_latency: u64) -> Vec<ErrorPrediction> {
        self.total_operations += 1;
        self.latency_history.push(current_latency);

        let metrics = self
            .process_metrics
            .entry(operation.to_string())
            .or_insert_with(|| ProcessMetrics {
                total_calls: 0,
                success_count: 0,
                error_count: 0,
                total_latency_ms: 0,
                peak_latency_ms: 0,
                min_latency_ms: u64::MAX,
            });

        metrics.total_calls += 1;
        metrics.total_latency_ms += current_latency;
        if current_latency > metrics.peak_latency_ms {
            metrics.peak_latency_ms = current_latency;
        }
        if current_latency < metrics.min_latency_ms {
            metrics.min_latency_ms = current_latency;
        }

        let mut predictions = Vec::new();

        // Calculate error rate
        let error_rate = if metrics.total_calls > 0 {
            metrics.error_count as f64 / metrics.total_calls as f64
        } else {
            0.0
        };

        // Predict memory pressure error
        if current_latency > metrics.peak_latency_ms / 2 {
            predictions.push(ErrorPrediction {
                operation_id: self.total_operations,
                risk_level: 0.4,
                error_type: "Memory Pressure".to_string(),
                suggested_action: "Consider clearing cache to free memory".to_string(),
                timestamp: SystemTime::now(),
            });
        }

        // Predict timeout error
        if error_rate > self.error_rate_threshold {
            predictions.push(ErrorPrediction {
                operation_id: self.total_operations,
                risk_level: 0.7,
                error_type: "High Error Rate".to_string(),
                suggested_action: "Review operation parameters and retry with backoff".to_string(),
                timestamp: SystemTime::now(),
            });
        }

        // Predict provider failure
        if metrics.error_count > 5 {
            predictions.push(ErrorPrediction {
                operation_id: self.total_operations,
                risk_level: 0.8,
                error_type: "Provider Failure".to_string(),
                suggested_action: "Switch to backup provider or increase timeout".to_string(),
                timestamp: SystemTime::now(),
            });
        }

        // Store predictions (keep only recent ones)
        for pred in &predictions {
            if self.error_predictions.len() >= self.max_predictions {
                self.error_predictions.pop_front();
            }
            self.error_predictions.push_back(pred.clone());
        }

        predictions
    }

    /// Record successful operation
    pub fn record_success(&mut self, operation: &str) {
        if let Some(metrics) = self.process_metrics.get_mut(operation) {
            metrics.success_count += 1;
        }
    }

    /// Record failed operation
    pub fn record_error(&mut self, operation: &str) {
        if let Some(metrics) = self.process_metrics.get_mut(operation) {
            metrics.error_count += 1;
        }
    }

    /// Get performance metrics for operation
    pub fn get_metrics(&self, operation: &str) -> Option<ProcessMetrics> {
        self.process_metrics.get(operation).cloned()
    }

    /// Get all recent predictions
    pub fn get_predictions(&self) -> Vec<ErrorPrediction> {
        self.error_predictions.iter().cloned().collect()
    }
}

impl AssassinShield {
    pub fn new() -> Self {
        AssassinShield {
            rate_limit_per_second: 1000,
            request_times: VecDeque::new(),
            audit_log: Vec::new(),
            blocked_attacks: 0,
            prompt_blocklist: vec![
                "'; DROP TABLE".to_string(),
                "__import__".to_string(),
                "system(".to_string(),
                "exec(".to_string(),
                "eval(".to_string(),
            ],
            active_threats: HashMap::new(),
        }
    }

    /// Check rate limiting
    pub fn check_rate_limit(&mut self) -> Result<bool, String> {
        let now = SystemTime::now();
        let one_sec_ago = now
            .checked_sub(std::time::Duration::from_secs(1))
            .unwrap_or(now);

        // Remove old entries outside the 1-second window
        while let Some(front) = self.request_times.front() {
            if *front < one_sec_ago {
                self.request_times.pop_front();
            } else {
                break;
            }
        }

        // Check if under limit
        if self.request_times.len() < self.rate_limit_per_second as usize {
            self.request_times.push_back(now);
            Ok(true)
        } else {
            self.blocked_attacks += 1;
            self.log_security_event("rate_limit", "blocked", "Request rate limit exceeded", 4);
            Err("Rate limit exceeded: too many requests per second".to_string())
        }
    }

    /// Validate prompt for injection attacks
    pub fn validate_prompt(&mut self, prompt: &str) -> Result<(), String> {
        for blocked in &self.prompt_blocklist {
            if prompt.to_lowercase().contains(&blocked.to_lowercase()) {
                self.blocked_attacks += 1;
                self.log_security_event(
                    "prompt_injection",
                    "blocked",
                    &format!("Blocked pattern: {}", blocked),
                    9,
                );
                return Err(format!("Prompt validation failed: suspicious pattern detected"));
            }
        }

        // Check for unusual length (potential DOS)
        if prompt.len() > 1_000_000 {
            self.blocked_attacks += 1;
            self.log_security_event(
                "dos_attempt",
                "blocked",
                "Prompt exceeds 1MB limit",
                7,
            );
            return Err("Prompt too large (max 1MB)".to_string());
        }

        Ok(())
    }

    /// Log security event
    pub fn log_security_event(
        &mut self,
        operation: &str,
        status: &str,
        details: &str,
        severity: u32,
    ) {
        self.audit_log.push(SecurityEvent {
            timestamp: SystemTime::now(),
            operation: operation.to_string(),
            status: status.to_string(),
            details: details.to_string(),
            severity: severity.min(10),
        });
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> Vec<SecurityEvent> {
        self.audit_log.iter().take(1000).cloned().collect()
    }

    /// Get security summary
    pub fn get_security_summary(&self) -> HashMap<String, Value> {
        let mut summary = HashMap::new();
        summary.insert(
            "total_attacks_blocked".to_string(),
            Value::Number(self.blocked_attacks as f64),
        );
        summary.insert(
            "audit_log_size".to_string(),
            Value::Number(self.audit_log.len() as f64),
        );
        summary.insert(
            "rate_limit".to_string(),
            Value::Number(self.rate_limit_per_second as f64),
        );
        summary.insert(
            "active_threats".to_string(),
            Value::Number(self.active_threats.len() as f64),
        );
        summary
    }
}

impl AIRuntime {
    /// Create new AI runtime with default configuration
    pub fn new() -> Self {
        AIRuntime {
            config: AIConfig::default(),
            providers: ProviderManager::new(),
            cache: AICache::new(1000), // 1000 entry cache
            stats: AIStats::default(),
            ghost: GhostMonitor::new(),
            assassin: AssassinShield::new(),
        }
    }

    /// Initialize with custom configuration
    pub fn with_config(config: AIConfig) -> Self {
        AIRuntime {
            config,
            providers: ProviderManager::new(),
            cache: AICache::new(1000),
            stats: AIStats::default(),
            ghost: GhostMonitor::new(),
            assassin: AssassinShield::new(),
        }
    }


    /// Generate text using configured provider
    /// AI_GENERATE function - Core LLM interface
    /// ASSASSIN: Validates prompt and enforces rate limits
    /// GHOST: Monitors performance and predicts errors
    pub fn ai_generate(
        &mut self,
        prompt: &str,
        options: HashMap<String, Value>,
    ) -> Result<String, String> {
        // ASSASSIN: Check rate limiting
        self.assassin.check_rate_limit()?;

        // ASSASSIN: Validate prompt
        self.assassin.validate_prompt(prompt)?;

        // Validate prompt
        if prompt.is_empty() {
            self.assassin
                .log_security_event("generate", "blocked", "Empty prompt rejected", 2);
            return Err("Prompt cannot be empty".to_string());
        }

        // Extract options
        let model = options
            .get("model")
            .and_then(|v| match v {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.config.default_model.clone());

        let max_tokens = options
            .get("max_tokens")
            .and_then(|v| match v {
                Value::Number(n) => Some(*n as usize),
                _ => None,
            })
            .unwrap_or(256);

        let temperature = options
            .get("temperature")
            .and_then(|v| match v {
                Value::Number(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(0.7);

        // Check cache first
        let cache_key = format!("gen:{}:{}:{}", model, prompt.len(), temperature as u32);
        if let Some(cached) = self.cache.get(&cache_key) {
            self.stats.cache_hits += 1;
            self.ghost.record_success("ai_generate");
            self.assassin
                .log_security_event("generate", "allowed", "Cache hit", 0);
            return Ok(cached);
        }

        self.stats.cache_misses += 1;
        self.stats.total_requests += 1;

        // Get provider based on configuration
        let provider_name = options
            .get("provider")
            .and_then(|v| match v {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.config.default_provider.clone());

        // GHOST: Monitor execution
        let start = Instant::now();
        // Auto-fallback: if the configured provider fails with an auth/config error,
        // transparently retry with the built-in local provider.
        let result = match self.providers.infer(&provider_name, prompt, &model, temperature, max_tokens) {
            Err(e) if e.contains("API key") || e.contains("not configured") || e.contains("auth") => {
                self.providers.infer("local", prompt, &model, temperature, max_tokens)
            }
            other => other,
        };
        let elapsed = start.elapsed().as_millis() as u64;

        // GHOST: Check for error predictions
        let predictions = self.ghost.predict_errors("ai_generate", elapsed);
        if !predictions.is_empty() {
            for pred in &predictions {
                self.assassin.log_security_event(
                    "error_prediction",
                    "warning",
                    &format!("{}: {} - {}", pred.error_type, pred.risk_level, pred.suggested_action),
                    (pred.risk_level * 10.0) as u32,
                );
            }
        }

        match result {
            Ok(response) => {
                self.stats.total_latency_ms += elapsed;
                self.stats.total_tokens += (response.len() as f64 / 4.0) as u64;

                // Cache result
                self.cache.put(cache_key, response.clone(), Some(3600)); // 1 hour TTL

                self.ghost.record_success("ai_generate");
                self.assassin
                    .log_security_event("generate", "allowed", &format!("{}ms latency", elapsed), 0);

                Ok(response)
            }
            Err(e) => {
                self.ghost.record_error("ai_generate");
                self.assassin
                    .log_security_event("generate", "error", &e, 3);
                Err(e)
            }
        }
    }


    /// Generate embeddings
    /// AI_EMBED function - Text to vector
    pub fn ai_embed(&mut self, text: &str, model: &str) -> Result<Vec<f32>, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        // Check cache
        let cache_key = format!("embed:{}:{}", model, text.len());
        if let Some(cached) = self.cache.get(&cache_key) {
            self.stats.cache_hits += 1;
            // Parse cached embedding
            if let Ok(parsed) = parse_embedding_string(&cached) {
                return Ok(parsed);
            }
        }

        self.stats.cache_misses += 1;
        self.stats.total_requests += 1;

        let start = Instant::now();
        let embedding = self.providers.embed(text, model)?;
        let elapsed = start.elapsed().as_millis() as u64;

        self.stats.total_latency_ms += elapsed;

        // Cache embedding as string
        let embedding_str = format!("{:?}", embedding);
        self.cache
            .put(cache_key, embedding_str, Some(86400)); // 24 hour TTL

        Ok(embedding)
    }

    /// Classify text into categories
    /// AI_CLASSIFY function - Zero-shot classification
    pub fn ai_classify(
        &mut self,
        text: &str,
        categories: Vec<String>,
        model: &str,
    ) -> Result<ClassifyResult, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        if categories.is_empty() {
            return Err("At least one category is required".to_string());
        }

        self.stats.total_requests += 1;

        let start = Instant::now();
        let result = self.providers.classify(text, categories, model)?;
        let elapsed = start.elapsed().as_millis() as u64;

        self.stats.total_latency_ms += elapsed;

        Ok(result)
    }

    /// Extract structured data from text
    /// AI_EXTRACT function - Information extraction
    pub fn ai_extract(
        &mut self,
        text: &str,
        schema: HashMap<String, String>,
        model: &str,
    ) -> Result<HashMap<String, Value>, String> {
        if text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        if schema.is_empty() {
            return Err("Schema cannot be empty".to_string());
        }

        self.stats.total_requests += 1;

        let start = Instant::now();
        let result = self.providers.extract(text, schema, model)?;
        let elapsed = start.elapsed().as_millis() as u64;

        self.stats.total_latency_ms += elapsed;

        Ok(result)
    }

    /// Run local ONNX model inference
    /// AI_LOCAL_INFER function - On-device inference
    pub fn ai_local_infer(
        &mut self,
        model_path: &str,
        input: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        if model_path.is_empty() {
            return Err("Model path cannot be empty".to_string());
        }

        self.stats.total_requests += 1;

        let start = Instant::now();
        let result = self.providers.local_infer(model_path, input)?;
        let elapsed = start.elapsed().as_millis() as u64;

        self.stats.total_latency_ms += elapsed;

        Ok(result)
    }

    /// Configure provider settings
    /// AI_PROVIDER_SET function
    pub fn set_provider_config(
        &mut self,
        provider: &str,
        config: HashMap<String, Value>,
    ) -> Result<bool, String> {
        self.providers.configure(provider, config)?;
        Ok(true)
    }

    /// Get provider information
    /// AI_PROVIDER_GET function
    pub fn get_provider_config(&self, provider: &str) -> Result<HashMap<String, Value>, String> {
        self.providers.get_config(provider)
    }

    /// List available providers
    /// AI_PROVIDER_AVAILABLE function
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.list_available()
    }

    /// Enable caching
    /// AI_CACHE_ENABLE function
    pub fn enable_cache(&mut self, cache_type: &str) -> Result<bool, String> {
        match cache_type {
            "memory" => {
                self.cache = AICache::new(1000);
                Ok(true)
            }
            "redis" => {
                // Redis implementation would go here
                Err("Redis caching not yet implemented".to_string())
            }
            "none" => {
                self.cache = AICache::new(0);
                Ok(true)
            }
            _ => Err(format!("Unknown cache type: {}", cache_type)),
        }
    }

    /// Clear cache
    /// AI_CACHE_CLEAR function
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.stats.cache_hits = 0;
        self.stats.cache_misses = 0;
    }

    /// Get cache statistics
    /// AI_CACHE_STATS function
    pub fn get_cache_stats(&self) -> HashMap<String, Value> {
        let mut stats = HashMap::new();
        stats.insert(
            "hits".to_string(),
            Value::Number(self.stats.cache_hits as f64),
        );
        stats.insert(
            "misses".to_string(),
            Value::Number(self.stats.cache_misses as f64),
        );

        let hit_rate = if self.stats.cache_hits + self.stats.cache_misses > 0 {
            self.stats.cache_hits as f64 / (self.stats.cache_hits + self.stats.cache_misses) as f64
        } else {
            0.0
        };
        stats.insert("hit_rate".to_string(), Value::Number(hit_rate));

        stats
    }

    /// Get metrics (enhanced with GHOST & ASSASSIN data)
    /// AI_METRICS function with security and monitoring data
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let mut metrics = HashMap::new();
        metrics.insert(
            "total_requests".to_string(),
            Value::Number(self.stats.total_requests as f64),
        );
        metrics.insert(
            "cache_hits".to_string(),
            Value::Number(self.stats.cache_hits as f64),
        );
        metrics.insert(
            "cache_misses".to_string(),
            Value::Number(self.stats.cache_misses as f64),
        );
        metrics.insert(
            "total_tokens".to_string(),
            Value::Number(self.stats.total_tokens as f64),
        );
        metrics.insert(
            "avg_latency_ms".to_string(),
            Value::Number(if self.stats.total_requests > 0 {
                self.stats.total_latency_ms as f64 / self.stats.total_requests as f64
            } else {
                0.0
            }),
        );

        // GHOST metrics
        metrics.insert(
            "ghost_total_operations".to_string(),
            Value::Number(self.ghost.total_operations as f64),
        );
        metrics.insert(
            "ghost_error_predictions".to_string(),
            Value::Number(self.ghost.error_predictions.len() as f64),
        );

        // ASSASSIN metrics
        metrics.insert(
            "assassin_attacks_blocked".to_string(),
            Value::Number(self.assassin.blocked_attacks as f64),
        );
        metrics.insert(
            "assassin_audit_log_size".to_string(),
            Value::Number(self.assassin.audit_log.len() as f64),
        );

        metrics
    }

    /// Get GHOST monitoring data
    pub fn get_ghost_status(&self) -> HashMap<String, Value> {
        let mut status = HashMap::new();
        status.insert(
            "total_operations".to_string(),
            Value::Number(self.ghost.total_operations as f64),
        );
        status.insert(
            "error_predictions_count".to_string(),
            Value::Number(self.ghost.error_predictions.len() as f64),
        );
        status.insert(
            "latency_history_size".to_string(),
            Value::Number(self.ghost.latency_history.len() as f64),
        );

        if !self.ghost.latency_history.is_empty() {
            let max_latency = *self.ghost.latency_history.iter().max().unwrap_or(&0);
            let min_latency = *self.ghost.latency_history.iter().min().unwrap_or(&0);
            let avg_latency = self.ghost.latency_history.iter().sum::<u64>()
                / self.ghost.latency_history.len() as u64;

            status.insert("max_latency_ms".to_string(), Value::Number(max_latency as f64));
            status.insert("min_latency_ms".to_string(), Value::Number(min_latency as f64));
            status.insert("avg_latency_ms".to_string(), Value::Number(avg_latency as f64));
        }

        status
    }

    /// Get ASSASSIN security data
    pub fn get_assassin_status(&self) -> HashMap<String, Value> {
        self.assassin.get_security_summary()
    }
}

impl Default for AIRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to parse embedding string back to vector
fn parse_embedding_string(s: &str) -> Result<Vec<f32>, String> {
    // Simple parser for debug-formatted vector
    let numbers: Vec<f32> = s
        .trim_matches(|c: char| c == '[' || c == ']')
        .split(',')
        .filter_map(|n| n.trim().parse::<f32>().ok())
        .collect();

    if numbers.is_empty() {
        Err("Failed to parse embedding".to_string())
    } else {
        Ok(numbers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_runtime_creation() {
        let runtime = AIRuntime::new();
        assert_eq!(runtime.stats.total_requests, 0);
        assert_eq!(runtime.stats.cache_hits, 0);
    }

    #[test]
    fn test_cache_stats() {
        let runtime = AIRuntime::new();
        let stats = runtime.get_cache_stats();
        assert!(stats.contains_key("hits"));
        assert!(stats.contains_key("hit_rate"));
    }

    #[test]
    fn test_list_providers() {
        let runtime = AIRuntime::new();
        let providers = runtime.list_providers();
        assert!(!providers.is_empty());
    }

    #[test]
    fn test_metrics() {
        let runtime = AIRuntime::new();
        let metrics = runtime.get_metrics();
        assert!(metrics.contains_key("total_requests"));
        assert!(metrics.contains_key("avg_latency_ms"));
    }

    #[test]
    fn test_parse_embedding() {
        let result = parse_embedding_string("[0.1, 0.2, 0.3]");
        assert!(result.is_ok());
        let vec = result.unwrap();
        assert_eq!(vec.len(), 3);
    }
}
