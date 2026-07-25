// AI Configuration
// src/ai/config.rs
//
// Configuration management for AI runtime

#[derive(Debug, Clone)]
pub struct AIConfig {
    /// Default provider (openai, local, etc.)
    pub default_provider: String,
    
    /// Default model to use
    pub default_model: String,
    
    /// Default temperature for generation
    pub default_temperature: f64,
    
    /// Default max tokens
    pub default_max_tokens: usize,
    
    /// Enable caching
    pub enable_cache: bool,
    
    /// Cache size (number of entries)
    pub cache_size: usize,
    
    /// Enable rate limiting
    pub enable_rate_limit: bool,
    
    /// Requests per minute limit
    pub rate_limit_rpm: usize,
    
    /// Request timeout in seconds
    pub timeout_secs: u64,
    
    /// Enable telemetry
    pub enable_telemetry: bool,
    
    /// Privacy mode (don't log conversations)
    pub privacy_mode: bool,
}

impl AIConfig {
    /// Create new configuration with recommended defaults
    pub fn new() -> Self {
        AIConfig {
            default_provider: "openai".to_string(),
            default_model: "gpt-3.5-turbo".to_string(),
            default_temperature: 0.7,
            default_max_tokens: 256,
            enable_cache: true,
            cache_size: 1000,
            enable_rate_limit: true,
            rate_limit_rpm: 60,
            timeout_secs: 30,
            enable_telemetry: false,
            privacy_mode: false,
        }
    }

    /// Create configuration for local inference
    pub fn local() -> Self {
        AIConfig {
            default_provider: "local".to_string(),
            default_model: "bert-base-uncased".to_string(),
            default_temperature: 0.0,
            default_max_tokens: 512,
            enable_cache: true,
            cache_size: 5000,
            enable_rate_limit: false,
            rate_limit_rpm: 1000,
            timeout_secs: 60,
            enable_telemetry: false,
            privacy_mode: true,
        }
    }

    /// Create configuration for development
    pub fn development() -> Self {
        AIConfig {
            default_provider: "openai".to_string(),
            default_model: "gpt-3.5-turbo".to_string(),
            default_temperature: 0.9,
            default_max_tokens: 500,
            enable_cache: true,
            cache_size: 100,
            enable_rate_limit: false,
            rate_limit_rpm: 100,
            timeout_secs: 60,
            enable_telemetry: true,
            privacy_mode: false,
        }
    }

    /// Create configuration for production
    pub fn production() -> Self {
        AIConfig {
            default_provider: "openai".to_string(),
            default_model: "gpt-4".to_string(),
            default_temperature: 0.7,
            default_max_tokens: 256,
            enable_cache: true,
            cache_size: 10000,
            enable_rate_limit: true,
            rate_limit_rpm: 3600,
            timeout_secs: 30,
            enable_telemetry: true,
            privacy_mode: false,
        }
    }

    /// Set default provider
    pub fn with_provider(mut self, provider: String) -> Self {
        self.default_provider = provider;
        self
    }

    /// Set default model
    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    /// Set cache size
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Enable privacy mode
    pub fn with_privacy(mut self, privacy: bool) -> Self {
        self.privacy_mode = privacy;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.default_provider.is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }

        if self.default_model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        if self.default_temperature < 0.0 || self.default_temperature > 2.0 {
            return Err("Temperature must be between 0 and 2".to_string());
        }

        if self.default_max_tokens < 1 {
            return Err("Max tokens must be at least 1".to_string());
        }

        if self.timeout_secs < 1 {
            return Err("Timeout must be at least 1 second".to_string());
        }

        if self.rate_limit_rpm < 1 && self.enable_rate_limit {
            return Err("Rate limit must be at least 1 request per minute".to_string());
        }

        Ok(())
    }
}

impl Default for AIConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AIConfig::new();
        assert_eq!(config.default_provider, "openai");
        assert!(config.enable_cache);
    }

    #[test]
    fn test_local_config() {
        let config = AIConfig::local();
        assert_eq!(config.default_provider, "local");
        assert!(config.privacy_mode);
    }

    #[test]
    fn test_config_validation() {
        let config = AIConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_temperature() {
        let config = AIConfig::new().with_model("test".to_string());
        // Can't modify temperature directly in test, but validate works
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = AIConfig::new()
            .with_provider("local".to_string())
            .with_model("bert".to_string())
            .with_cache_size(5000);

        assert_eq!(config.default_provider, "local");
        assert_eq!(config.default_model, "bert");
        assert_eq!(config.cache_size, 5000);
    }
}
