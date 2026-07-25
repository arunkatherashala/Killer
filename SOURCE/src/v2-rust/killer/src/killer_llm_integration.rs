/// KILLER AI LLM INTEGRATION
/// 
/// Provides pluggable LLM client infrastructure for the Killer AI ecosystem.
/// Supports OpenAI, Claude, Ollama, and local models.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// LLM Configuration - determines which backend to use
#[derive(Debug, Clone)]
pub struct LLMConfig {
    pub backend: LLMBackend,
    pub timeout_secs: u32,
    pub max_retries: u32,
    pub enable_caching: bool,
    pub cache_ttl_secs: u32,
}

#[derive(Debug, Clone)]
pub enum LLMBackend {
    OpenAI {
        model: String,
        api_key_env: String,
        base_url: String,
        max_tokens: u32,
    },
    Claude {
        model: String,
        api_key_env: String,
        base_url: String,
        max_tokens: u32,
    },
    Ollama {
        model: String,
        endpoint: String,
        max_tokens: u32,
    },
    Local {
        model_path: String,
        max_tokens: u32,
    },
}

impl LLMConfig {
    pub fn openai_gpt4() -> Self {
        LLMConfig {
            backend: LLMBackend::OpenAI {
                model: "gpt-4".to_string(),
                api_key_env: "OPENAI_API_KEY".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                max_tokens: 2048,
            },
            timeout_secs: 30,
            max_retries: 3,
            enable_caching: true,
            cache_ttl_secs: 3600,
        }
    }

    pub fn claude_opus() -> Self {
        LLMConfig {
            backend: LLMBackend::Claude {
                model: "claude-3-opus-20240229".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                base_url: "https://api.anthropic.com".to_string(),
                max_tokens: 4096,
            },
            timeout_secs: 30,
            max_retries: 3,
            enable_caching: true,
            cache_ttl_secs: 3600,
        }
    }

    pub fn ollama_local(endpoint: String) -> Self {
        LLMConfig {
            backend: LLMBackend::Ollama {
                model: "llama2".to_string(),
                endpoint,
                max_tokens: 2048,
            },
            timeout_secs: 60,
            max_retries: 2,
            enable_caching: false,
            cache_ttl_secs: 0,
        }
    }
}


/// LLM Request - what we send to the model
#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub temperature: f32,
    pub top_p: f32,
    pub request_id: String,
    pub timestamp_ms: u64,
}

impl LLMRequest {
    pub fn new(system_prompt: impl Into<String>, user_message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        LLMRequest {
            system_prompt: system_prompt.into(),
            user_message: user_message.into(),
            temperature: 0.7,
            top_p: 1.0,
            request_id: format!("req_{}", timestamp_ms),
            timestamp_ms,
        }
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp.clamp(0.0, 2.0);
        self
    }

    pub fn for_optimization() -> Self {
        Self::new(
            "You are a Killer language performance expert. Analyze code and suggest optimizations.",
            ""
        ).with_temperature(0.5)
    }

    pub fn for_security_audit() -> Self {
        Self::new(
            "You are a security expert specializing in the Killer language. \
            Perform rigorous security analysis following the Assassin Layer policies: \
            blocked syscalls, isolated paths, network isolation, resource limits.",
            ""
        ).with_temperature(0.3)
    }

    pub fn for_code_review() -> Self {
        Self::new(
            "You are a senior code reviewer for the Killer language. \
            Perform comprehensive code review covering correctness, performance, readability, and best practices.",
            ""
        ).with_temperature(0.5)
    }
}


/// LLM Response - what the model returns
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub finish_reason: String,
    pub response_time_ms: u32,
    pub cached: bool,
}

impl LLMResponse {
    pub fn new(content: String, model: String) -> Self {
        LLMResponse {
            content,
            model,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            finish_reason: "stop".to_string(),
            response_time_ms: 0,
            cached: false,
        }
    }

    pub fn parse_suggestions(&self) -> Vec<OptimizationSuggestion> {
        // Simple text parsing of suggestions from response
        // In production, would use proper JSON parsing
        let mut suggestions = Vec::new();

        // Look for patterns like "title: ...", "confidence: 0.X", etc.
        let lines: Vec<&str> = self.content.lines().collect();
        for i in 0..lines.len() {
            if lines[i].contains("Suggestion") || lines[i].contains("suggestion") {
                // Try to extract a suggestion from nearby lines
                if i + 5 < lines.len() {
                    suggestions.push(OptimizationSuggestion {
                        title: format!("Suggestion {}", suggestions.len() + 1),
                        description: self.content.clone(),
                        confidence: 0.7,
                        expected_improvement_percent: 15.0,
                        implementation_effort: "medium".to_string(),
                        priority: 5,
                    });
                }
            }
        }

        suggestions
    }
}


/// Optimization suggestion parsed from LLM response
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub expected_improvement_percent: f64,
    pub implementation_effort: String,
    pub priority: u8,
}


/// LLM Client - main interface for interacting with LLMs
pub struct LLMClient {
    config: LLMConfig,
    cache: HashMap<String, CachedResponse>,
    stats: LLMStats,
}

#[derive(Debug, Clone)]
struct CachedResponse {
    response: LLMResponse,
    cached_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct LLMStats {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub cached_hits: u32,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_response_time_ms: u64,
    pub average_response_time_ms: f64,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        LLMClient {
            config,
            cache: HashMap::new(),
            stats: LLMStats::default(),
        }
    }

    /// Process a request through the LLM
    pub fn process(&mut self, request: &LLMRequest) -> Result<LLMResponse, String> {
        // Check cache
        if self.config.enable_caching {
            let cache_key = self.compute_cache_key(request);
            if let Some(cached) = self.cache.get(&cache_key) {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                if now_ms - cached.cached_at_ms < (self.config.cache_ttl_secs as u64 * 1000) {
                    self.stats.cached_hits += 1;
                    let mut response = cached.response.clone();
                    response.cached = true;
                    return Ok(response);
                }
            }
        }

        // Send request (simulated - would actually call API)
        let mut response = self.send_request(request)?;

        // Cache response
        if self.config.enable_caching {
            let cache_key = self.compute_cache_key(request);
            self.cache.insert(cache_key, CachedResponse {
                response: response.clone(),
                cached_at_ms: request.timestamp_ms,
            });
        }

        // Update stats
        self.stats.total_requests += 1;
        self.stats.successful_requests += 1;
        self.stats.total_prompt_tokens += response.prompt_tokens as u64;
        self.stats.total_completion_tokens += response.completion_tokens as u64;
        self.stats.total_response_time_ms += response.response_time_ms as u64;
        self.stats.average_response_time_ms = self.stats.total_response_time_ms as f64
            / self.stats.total_requests as f64;

        Ok(response)
    }

    fn compute_cache_key(&self, request: &LLMRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.system_prompt.hash(&mut hasher);
        request.user_message.hash(&mut hasher);
        format!("llm_cache_{:x}", hasher.finish())
    }

    fn send_request(&self, request: &LLMRequest) -> Result<LLMResponse, String> {
        match &self.config.backend {
            LLMBackend::OpenAI { model, .. } => {
                self.send_openai_request(request, model)
            }
            LLMBackend::Claude { model, .. } => {
                self.send_claude_request(request, model)
            }
            LLMBackend::Ollama { model, endpoint, .. } => {
                self.send_ollama_request(request, model, endpoint)
            }
            LLMBackend::Local { model_path, .. } => {
                self.send_local_request(request, model_path)
            }
        }
    }

    fn send_openai_request(&self, request: &LLMRequest, model: &str) -> Result<LLMResponse, String> {
        // Simulated OpenAI request - in real implementation would use reqwest/hyper
        Ok(LLMResponse {
            content: format!(
                "OpenAI ({}) response to: {}",
                model,
                &request.user_message[..request.user_message.len().min(50)]
            ),
            model: model.to_string(),
            prompt_tokens: 150,
            completion_tokens: 300,
            total_tokens: 450,
            finish_reason: "stop".to_string(),
            response_time_ms: 850,
            cached: false,
        })
    }

    fn send_claude_request(&self, request: &LLMRequest, model: &str) -> Result<LLMResponse, String> {
        // Simulated Claude request
        Ok(LLMResponse {
            content: format!(
                "Claude ({}) response to: {}",
                model,
                &request.user_message[..request.user_message.len().min(50)]
            ),
            model: model.to_string(),
            prompt_tokens: 160,
            completion_tokens: 320,
            total_tokens: 480,
            finish_reason: "stop".to_string(),
            response_time_ms: 1200,
            cached: false,
        })
    }

    fn send_ollama_request(&self, _request: &LLMRequest, model: &str, endpoint: &str) -> Result<LLMResponse, String> {
        // Simulated Ollama request
        Ok(LLMResponse {
            content: format!(
                "Ollama ({}) response from {}",
                model, endpoint
            ),
            model: model.to_string(),
            prompt_tokens: 100,
            completion_tokens: 150,
            total_tokens: 250,
            finish_reason: "stop".to_string(),
            response_time_ms: 2500,
            cached: false,
        })
    }

    fn send_local_request(&self, _request: &LLMRequest, model_path: &str) -> Result<LLMResponse, String> {
        // Simulated local model request
        Ok(LLMResponse {
            content: format!("Local model response from {}", model_path),
            model: "local".to_string(),
            prompt_tokens: 80,
            completion_tokens: 120,
            total_tokens: 200,
            finish_reason: "stop".to_string(),
            response_time_ms: 5000,
            cached: false,
        })
    }

    pub fn get_stats(&self) -> &LLMStats {
        &self.stats
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}


/// Killer-specific LLM features
pub struct KillerLLMFeatures {
    pub code_optimization: bool,
    pub security_audit: bool,
    pub performance_profiling: bool,
    pub documentation_generation: bool,
    pub test_generation: bool,
    pub bug_detection: bool,
}

impl Default for KillerLLMFeatures {
    fn default() -> Self {
        KillerLLMFeatures {
            code_optimization: true,
            security_audit: true,
            performance_profiling: true,
            documentation_generation: true,
            test_generation: true,
            bug_detection: true,
        }
    }
}

/// Use Killer LLM features to automatically optimize code
pub fn auto_optimize_with_llm(
    code: &str,
    client: &mut LLMClient,
) -> Result<String, String> {
    let mut request = LLMRequest::for_optimization();
    request.user_message = format!(
        "Analyze and suggest optimizations for this Killer code:\n```killer\n{}\n```\n\
        Return suggestions as JSON with: title, description, confidence (0-1), improvement_percent, effort, priority",
        code
    );

    let response = client.process(&request)?;
    Ok(response.content)
}

/// Use Killer LLM for security auditing
pub fn security_audit_with_llm(
    code: &str,
    client: &mut LLMClient,
) -> Result<String, String> {
    let mut request = LLMRequest::for_security_audit();
    request.user_message = format!(
        "Perform security audit on this Killer code:\n```killer\n{}\n```\n\
        Check against Assassin Layer policies. Return: issues found, severity levels, recommended fixes",
        code
    );

    let response = client.process(&request)?;
    Ok(response.content)
}

/// Use Killer LLM for code review
pub fn code_review_with_llm(
    code: &str,
    client: &mut LLMClient,
) -> Result<String, String> {
    let mut request = LLMRequest::for_code_review();
    request.user_message = format!(
        "Perform comprehensive code review of this Killer code:\n```killer\n{}\n```\n\
        Cover: correctness, performance, readability, best practices, security",
        code
    );

    let response = client.process(&request)?;
    Ok(response.content)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_presets() {
        let gpt4 = LLMConfig::openai_gpt4();
        assert_eq!(gpt4.timeout_secs, 30);
        assert_eq!(gpt4.max_retries, 3);

        let claude = LLMConfig::claude_opus();
        assert_eq!(claude.timeout_secs, 30);

        let ollama = LLMConfig::ollama_local("http://localhost:11434".to_string());
        assert_eq!(ollama.timeout_secs, 60);
    }

    #[test]
    fn test_llm_request_creation() {
        let req = LLMRequest::new("system", "user message");
        assert_eq!(req.system_prompt, "system");
        assert_eq!(req.user_message, "user message");
        assert_eq!(req.temperature, 0.7);
        assert!(req.timestamp_ms > 0);
        assert!(!req.request_id.is_empty());
    }

    #[test]
    fn test_llm_request_temperature_clamping() {
        let req = LLMRequest::new("", "").with_temperature(3.5);
        assert_eq!(req.temperature, 2.0);  // Clamped to max

        let req = LLMRequest::new("", "").with_temperature(-0.5);
        assert_eq!(req.temperature, 0.0);  // Clamped to min
    }

    #[test]
    fn test_llm_client_processing() {
        let config = LLMConfig::openai_gpt4();
        let mut client = LLMClient::new(config);
        let request = LLMRequest::new("system", "test message");

        let response = client.process(&request);
        assert!(response.is_ok());

        let resp = response.unwrap();
        assert_eq!(resp.model, "gpt-4");
        assert!(resp.response_time_ms > 0);

        // Check stats updated
        assert_eq!(client.stats.total_requests, 1);
        assert_eq!(client.stats.successful_requests, 1);
    }

    #[test]
    fn test_llm_caching() {
        let mut config = LLMConfig::openai_gpt4();
        config.enable_caching = true;
        config.cache_ttl_secs = 3600;

        let mut client = LLMClient::new(config);
        let request = LLMRequest::new("system", "test");

        // First request
        let resp1 = client.process(&request).unwrap();
        assert!(!resp1.cached);

        // Second identical request (should be cached)
        let resp2 = client.process(&request).unwrap();
        assert!(resp2.cached);

        // Cache should have 1 entry
        assert_eq!(client.cache_size(), 1);
        assert_eq!(client.stats.cached_hits, 1);
    }

    #[test]
    fn test_killer_llm_features() {
        let features = KillerLLMFeatures::default();
        assert!(features.code_optimization);
        assert!(features.security_audit);
        assert!(features.performance_profiling);
        assert!(features.documentation_generation);
        assert!(features.test_generation);
        assert!(features.bug_detection);
    }

    #[test]
    fn test_optimization_request_preset() {
        let req = LLMRequest::for_optimization();
        assert_eq!(req.temperature, 0.5);  // Lower temp for consistency
        assert!(req.system_prompt.contains("performance"));
    }

    #[test]
    fn test_security_audit_request_preset() {
        let req = LLMRequest::for_security_audit();
        assert_eq!(req.temperature, 0.3);  // Very low temp for rigor
        assert!(req.system_prompt.contains("security"));
        assert!(req.system_prompt.contains("Assassin Layer"));
    }

    #[test]
    fn test_code_review_request_preset() {
        let req = LLMRequest::for_code_review();
        assert_eq!(req.temperature, 0.5);
        assert!(req.system_prompt.contains("code review"));
    }

    #[test]
    fn test_llm_response_stats() {
        let response = LLMResponse {
            content: "test".to_string(),
            model: "gpt-4".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            finish_reason: "stop".to_string(),
            response_time_ms: 500,
            cached: false,
        };

        assert_eq!(response.total_tokens, 150);
        assert_eq!(response.response_time_ms, 500);
        assert!(!response.cached);
    }

    #[test]
    fn test_optimization_suggestion() {
        let suggestion = OptimizationSuggestion {
            title: "Test Optimization".to_string(),
            description: "A test suggestion".to_string(),
            confidence: 0.85,
            expected_improvement_percent: 20.0,
            implementation_effort: "easy".to_string(),
            priority: 8,
        };

        assert_eq!(suggestion.confidence, 0.85);
        assert_eq!(suggestion.priority, 8);
        assert!(suggestion.confidence > 0.5);
    }
}
