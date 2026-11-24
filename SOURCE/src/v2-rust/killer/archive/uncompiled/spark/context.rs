/// SparkSession and SparkContext - Core Spark APIs
///
/// SparkSession: Primary entry point for Spark functionality
/// SparkContext: Low-level distributed computing context

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Configuration for Spark
pub type Config = HashMap<String, String>;

/// SparkSession - Primary entry point for Spark SQL, DataFrames, and Datasets
pub struct SparkSession {
    id: String,
    app_name: String,
    config: Config,
}

impl SparkSession {
    /// Create a new SparkSession
    pub fn new() -> Self {
        let id = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            id: format!("spark-session-{}", id),
            app_name: "Killer-Spark-App".to_string(),
            config: Config::new(),
        }
    }

    /// Builder pattern: set application name
    pub fn app_name(mut self, name: &str) -> Self {
        self.app_name = name.to_string();
        self
    }

    /// Builder pattern: set configuration
    pub fn config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.id
    }

    /// Get application name
    pub fn app_name_val(&self) -> &str {
        &self.app_name
    }

    /// Get session name
    pub fn name(&self) -> &str {
        &self.app_name
    }

    /// Get configuration value
    pub fn config_value(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(|s| s.as_str())
    }

    /// Get all configuration
    pub fn config_all(&self) -> &Config {
        &self.config
    }

    /// Get SparkContext
    pub fn context(&self) -> SparkContext {
        SparkContext::new()
    }

    /// Get parallelism (number of available cores)
    pub fn parallelism(&self) -> usize {
        8 // Default to 8 cores for consistency
    }

    /// Check if running locally
    pub fn is_local(&self) -> bool {
        true
    }

    /// Version string
    pub fn version(&self) -> &'static str {
        "3.4.0-killer"
    }
}

impl Default for SparkSession {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SparkSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SparkSession")
            .field("id", &self.id)
            .field("app_name", &self.app_name)
            .field("config_size", &self.config.len())
            .finish()
    }
}

/// SparkContext - Low-level API for RDD operations and distributed computing
pub struct SparkContext {
    app_name: String,
    parallelism: usize,
}

impl SparkContext {
    /// Create new SparkContext
    pub fn new() -> Self {
        Self {
            app_name: "Killer-Spark".to_string(),
            parallelism: 8,
        }
    }

    /// Create from SparkSession
    pub fn from_session(session: &SparkSession) -> Self {
        Self {
            app_name: session.app_name_val().to_string(),
            parallelism: session.parallelism(),
        }
    }

    /// Get associated SparkSession
    pub fn session(&self) -> SparkSession {
        SparkSession::new().app_name(&self.app_name)
    }

    /// Parallelize: create RDD from collection
    pub fn parallelize<T: Clone + 'static>(&self, data: Vec<T>) -> Result<String, String> {
        let rdd_id = format!("parallelize-rdd-{}", self.parallelism);
        println!(
            "Created RDD {} with {} partitions from {} elements",
            rdd_id,
            self.parallelism,
            data.len()
        );
        Ok(rdd_id)
    }

    /// Read text file
    pub fn text_file(&self, path: &str) -> Result<String, String> {
        if !path.ends_with(".txt") {
            return Err("File must be text format".to_string());
        }
        let rdd_id = format!("text-rdd-{}", self.parallelism);
        println!("Read text file {} into RDD {}", path, rdd_id);
        Ok(rdd_id)
    }

    /// Get parallelism
    pub fn parallelism(&self) -> usize {
        self.parallelism
    }

    /// Set parallelism
    pub fn set_parallelism(&mut self, partitions: usize) {
        self.parallelism = partitions;
    }

    /// Stop the context
    pub fn stop(&self) -> Result<(), String> {
        println!("Stopping SparkContext: {}", self.app_name);
        Ok(())
    }

    /// Get status
    pub fn status(&self) -> String {
        format!(
            "SparkContext(app={}, parallelism={})",
            self.app_name, self.parallelism
        )
    }
}

impl Default for SparkContext {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SparkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SparkContext")
            .field("app_name", &self.app_name)
            .field("parallelism", &self.parallelism)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spark_session_creation() {
        let spark = SparkSession::new();
        assert!(!spark.session_id().is_empty());
    }

    #[test]
    fn test_spark_session_builder() {
        let spark = SparkSession::new().app_name("MyApp");
        assert_eq!(spark.app_name_val(), "MyApp");
    }

    #[test]
    fn test_spark_session_config() {
        let spark = SparkSession::new()
            .app_name("TestApp")
            .config("spark.executor.memory", "4g");

        assert_eq!(spark.config_value("spark.executor.memory"), Some("4g"));
    }

    #[test]
    fn test_spark_context_creation() {
        let ctx = SparkContext::new();
        assert!(ctx.parallelism() > 0);
    }

    #[test]
    fn test_spark_context_from_session() {
        let session = SparkSession::new().app_name("TestApp");
        let ctx = SparkContext::from_session(&session);
        assert_eq!(ctx.app_name, "TestApp");
    }

    #[test]
    fn test_spark_context_parallelism() {
        let mut ctx = SparkContext::new();
        let original = ctx.parallelism();
        ctx.set_parallelism(8);
        assert_eq!(ctx.parallelism(), 8);
    }

    #[test]
    fn test_spark_context_parallelize() {
        let ctx = SparkContext::new();
        let rdd_id = ctx.parallelize(vec![1, 2, 3, 4, 5]).unwrap();
        assert!(rdd_id.starts_with("parallelize-rdd-"));
    }

    #[test]
    fn test_spark_context_text_file() {
        let ctx = SparkContext::new();
        let result = ctx.text_file("data.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_spark_context_text_file_invalid() {
        let ctx = SparkContext::new();
        let result = ctx.text_file("data.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_spark_session_default() {
        let spark = SparkSession::default();
        assert!(!spark.app_name_val().is_empty());
    }

    #[test]
    fn test_spark_context_default() {
        let ctx = SparkContext::default();
        assert!(ctx.parallelism() > 0);
    }
}
