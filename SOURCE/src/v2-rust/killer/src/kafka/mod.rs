// Kafka integration for Killer v2.2
// Producer, Consumer, and transactional support

pub mod producer;
pub mod consumer;
pub mod client;
pub mod errors;

pub use producer::KafkaProducer;
pub use consumer::KafkaConsumer;
pub use client::KafkaClient;
pub use errors::{KafkaError, KafkaResult};

/// Kafka configuration for producers and consumers
#[derive(Clone, Debug)]
pub struct KafkaConfig {
    /// Broker addresses: "localhost:9092,localhost:9093"
    pub brokers: String,
    
    /// Client timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Batch size for producer
    pub batch_size: usize,
    
    /// Number of retries on failure
    pub retries: i32,
    
    /// Retry backoff in milliseconds
    pub retry_backoff_ms: u64,
    
    /// Compression: "none", "gzip", "snappy", "lz4", "zstd"
    pub compression: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        KafkaConfig {
            brokers: "localhost:9092".to_string(),
            timeout_ms: 30000,
            batch_size: 100,
            retries: 3,
            retry_backoff_ms: 100,
            compression: "none".to_string(),
        }
    }
}

/// Message sent or received via Kafka
#[derive(Clone, Debug)]
pub struct KafkaRecord {
    pub topic: String,
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub timestamp: Option<i64>,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

impl KafkaRecord {
    pub fn new(topic: String, value: Vec<u8>) -> Self {
        KafkaRecord {
            topic,
            partition: None,
            offset: None,
            timestamp: None,
            key: None,
            value,
            headers: Vec::new(),
        }
    }
    
    pub fn with_key(mut self, key: Vec<u8>) -> Self {
        self.key = Some(key);
        self
    }
    
    pub fn with_partition(mut self, partition: i32) -> Self {
        self.partition = Some(partition);
        self
    }
    
    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.push((name, value));
        self
    }
}

/// Consumer group and offset management
#[derive(Clone, Debug)]
pub struct ConsumerGroupConfig {
    pub group_id: String,
    pub auto_commit: bool,
    pub auto_commit_interval_ms: u64,
    pub max_poll_records: usize,
    pub session_timeout_ms: u64,
}

impl Default for ConsumerGroupConfig {
    fn default() -> Self {
        ConsumerGroupConfig {
            group_id: "default-group".to_string(),
            auto_commit: true,
            auto_commit_interval_ms: 5000,
            max_poll_records: 500,
            session_timeout_ms: 30000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kafka_config_default() {
        let config = KafkaConfig::default();
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.batch_size, 100);
    }
    
    #[test]
    fn test_kafka_record_creation() {
        let record = KafkaRecord::new("topic".to_string(), vec![1, 2, 3]);
        assert_eq!(record.topic, "topic");
        assert_eq!(record.value, vec![1, 2, 3]);
        assert_eq!(record.partition, None);
    }
    
    #[test]
    fn test_kafka_record_builder() {
        let record = KafkaRecord::new("topic".to_string(), vec![1, 2, 3])
            .with_key(vec![4, 5, 6])
            .with_partition(0)
            .with_header("header1".to_string(), "value1".to_string());
        
        assert_eq!(record.key, Some(vec![4, 5, 6]));
        assert_eq!(record.partition, Some(0));
        assert_eq!(record.headers.len(), 1);
    }
}
