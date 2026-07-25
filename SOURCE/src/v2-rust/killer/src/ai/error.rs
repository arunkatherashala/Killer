// AI Error Handling
// src/ai/error.rs
//
// Error types for the AI subsystem

use std::fmt;

#[derive(Debug, Clone)]
pub enum AIError {
    /// Provider not found or not configured
    ProviderNotFound(String),

    /// Provider configuration error
    ConfigError(String),

    /// Network/API error
    APIError {
        provider: String,
        message: String,
        code: Option<String>,
    },

    /// Model not found or unsupported
    ModelNotFound(String),

    /// Invalid parameters
    InvalidParams(String),

    /// Inference failed
    InferenceFailed(String),

    /// Cache error
    CacheError(String),

    /// Rate limit exceeded
    RateLimitExceeded {
        retry_after_secs: u64,
    },

    /// Request timeout
    Timeout(String),

    /// Invalid configuration
    InvalidConfig(String),

    /// Model file error
    ModelFileError(String),

    /// Encoding/Decoding error
    EncodingError(String),

    /// Unknown error
    Unknown(String),
}

impl AIError {
    /// Create a provider not found error
    pub fn provider_not_found(name: impl Into<String>) -> Self {
        AIError::ProviderNotFound(name.into())
    }

    /// Create an API error
    pub fn api_error(provider: impl Into<String>, message: impl Into<String>) -> Self {
        AIError::APIError {
            provider: provider.into(),
            message: message.into(),
            code: None,
        }
    }

    /// Create an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        AIError::InvalidParams(message.into())
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            AIError::ProviderNotFound(name) => format!("Provider '{}' not found", name),
            AIError::ConfigError(msg) => format!("Configuration error: {}", msg),
            AIError::APIError {
                provider,
                message,
                code,
            } => {
                if let Some(c) = code {
                    format!("API error from {}: {} ({})", provider, message, c)
                } else {
                    format!("API error from {}: {}", provider, message)
                }
            }
            AIError::ModelNotFound(name) => format!("Model '{}' not found", name),
            AIError::InvalidParams(msg) => format!("Invalid parameters: {}", msg),
            AIError::InferenceFailed(msg) => format!("Inference failed: {}", msg),
            AIError::CacheError(msg) => format!("Cache error: {}", msg),
            AIError::RateLimitExceeded {
                retry_after_secs,
            } => {
                format!(
                    "Rate limit exceeded, retry after {} seconds",
                    retry_after_secs
                )
            }
            AIError::Timeout(msg) => format!("Timeout: {}", msg),
            AIError::InvalidConfig(msg) => format!("Invalid configuration: {}", msg),
            AIError::ModelFileError(msg) => format!("Model file error: {}", msg),
            AIError::EncodingError(msg) => format!("Encoding error: {}", msg),
            AIError::Unknown(msg) => format!("Unknown error: {}", msg),
        }
    }

    /// Get error code for API responses
    pub fn code(&self) -> &str {
        match self {
            AIError::ProviderNotFound(_) => "PROVIDER_NOT_FOUND",
            AIError::ConfigError(_) => "CONFIG_ERROR",
            AIError::APIError { .. } => "API_ERROR",
            AIError::ModelNotFound(_) => "MODEL_NOT_FOUND",
            AIError::InvalidParams(_) => "INVALID_PARAMS",
            AIError::InferenceFailed(_) => "INFERENCE_FAILED",
            AIError::CacheError(_) => "CACHE_ERROR",
            AIError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            AIError::Timeout(_) => "TIMEOUT",
            AIError::InvalidConfig(_) => "INVALID_CONFIG",
            AIError::ModelFileError(_) => "MODEL_FILE_ERROR",
            AIError::EncodingError(_) => "ENCODING_ERROR",
            AIError::Unknown(_) => "UNKNOWN_ERROR",
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AIError::APIError { .. }
                | AIError::Timeout(_)
                | AIError::RateLimitExceeded { .. }
        )
    }
}

impl fmt::Display for AIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::error::Error for AIError {}

/// Result type for AI operations
pub type AIResult<T> = Result<T, AIError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AIError::provider_not_found("openai");
        assert_eq!(err.code(), "PROVIDER_NOT_FOUND");
    }

    #[test]
    fn test_error_description() {
        let err = AIError::invalid_params("bad value");
        assert!(err.description().contains("bad value"));
    }

    #[test]
    fn test_retryable_errors() {
        let timeout_err = AIError::Timeout("connection timeout".to_string());
        assert!(timeout_err.is_retryable());

        let config_err = AIError::ConfigError("bad config".to_string());
        assert!(!config_err.is_retryable());
    }

    #[test]
    fn test_rate_limit_error() {
        let err = AIError::RateLimitExceeded {
            retry_after_secs: 60,
        };
        assert!(err.is_retryable());
        assert!(err.description().contains("60"));
    }
}
