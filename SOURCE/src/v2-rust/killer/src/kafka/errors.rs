// Kafka Error Types and Handling

use std::fmt;

/// Kafka error types
#[derive(Debug, Clone)]
pub enum KafkaError {
    /// Connection failed to brokers
    ConnectionError(String),
    
    /// Request timed out
    TimeoutError(String),
    
    /// Broker returned error
    BrokerError(String),
    
    /// Message serialization failed
    SerializationError(String),
    
    /// Message deserialization failed
    DeserializationError(String),
    
    /// Topic not found
    UnknownTopicError(String),
    
    /// Partition not available
    PartitionError(String),
    
    /// Consumer group error
    GroupError(String),
    
    /// Offset out of range
    OffsetOutOfRange(i64),
    
    /// Message compression failed
    CompressionError(String),
    
    /// Authentication failed
    AuthenticationError(String),
    
    /// Authorization failed
    AuthorizationError(String),
    
    /// Invalid configuration
    ConfigError(String),
    
    /// Generic error
    Other(String),
}

impl fmt::Display for KafkaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KafkaError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            KafkaError::TimeoutError(msg) => write!(f, "Timeout: {}", msg),
            KafkaError::BrokerError(msg) => write!(f, "Broker error: {}", msg),
            KafkaError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            KafkaError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            KafkaError::UnknownTopicError(msg) => write!(f, "Unknown topic: {}", msg),
            KafkaError::PartitionError(msg) => write!(f, "Partition error: {}", msg),
            KafkaError::GroupError(msg) => write!(f, "Group error: {}", msg),
            KafkaError::OffsetOutOfRange(offset) => write!(f, "Offset out of range: {}", offset),
            KafkaError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            KafkaError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            KafkaError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            KafkaError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            KafkaError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for KafkaError {}

/// Result type for Kafka operations
pub type KafkaResult<T> = Result<T, KafkaError>;

/// Retry policy for failed operations
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_retries: i32,
    pub backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_retries: 3,
            backoff_ms: 100,
            max_backoff_ms: 30000,
        }
    }
}

impl RetryPolicy {
    /// Calculate backoff for retry attempt
    pub fn calculate_backoff(&self, attempt: i32) -> u64 {
        let exponential = self.backoff_ms * (2_u64.pow(attempt as u32));
        exponential.min(self.max_backoff_ms)
    }
}

/// Circuit breaker for prevention of cascading failures
#[derive(Clone, Debug)]
pub struct CircuitBreaker {
    pub failure_threshold: i32,
    pub success_threshold: i32,
    pub timeout_sec: u64,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        CircuitBreaker {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_sec: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let err = KafkaError::ConnectionError("Failed to connect".to_string());
        assert_eq!(
            err.to_string(),
            "Connection error: Failed to connect"
        );
    }
    
    #[test]
    fn test_retry_policy_backoff() {
        let policy = RetryPolicy::default();
        
        assert_eq!(policy.calculate_backoff(0), 100);   // 100ms
        assert_eq!(policy.calculate_backoff(1), 200);   // 200ms
        assert_eq!(policy.calculate_backoff(2), 400);   // 400ms
        assert_eq!(policy.calculate_backoff(10), 30000); // capped at 30s
    }
    
    #[test]
    fn test_circuit_breaker_creation() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.failure_threshold, 5);
        assert_eq!(cb.success_threshold, 2);
    }
}
