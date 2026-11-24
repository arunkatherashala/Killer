/// Streaming - DStream for real-time data processing
/// 
/// Discretized streams for processing streaming data

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::value::Value;

/// Micro-batch of data at a specific time
#[derive(Clone, Debug)]
pub struct RDD {
    id: u64,
    data: Vec<Value>,
    timestamp: u64,
}

impl RDD {
    pub fn new(id: u64, data: Vec<Value>, timestamp: u64) -> Self {
        Self { id, data, timestamp }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn data(&self) -> &[Value] {
        &self.data
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }

    pub fn collect(&self) -> Vec<Value> {
        self.data.clone()
    }
}

/// DStream transformation types
#[derive(Clone, Debug)]
pub enum StreamOp {
    /// Direct stream
    Direct(Vec<RDD>),

    /// Map transformation
    Map {
        source: Box<StreamOp>,
        func: String,
    },

    /// Filter transformation
    Filter {
        source: Box<StreamOp>,
        predicate: String,
    },

    /// Reduce by window
    ReduceWindow {
        source: Box<StreamOp>,
        duration_ms: u64,
    },
}

/// DStream - Discretized Stream for streaming data
#[derive(Clone)]
pub struct DStream {
    id: u64,
    operation: StreamOp,
    batch_duration_ms: u64,
    checkpoint_dir: Option<String>,
}

static DSTREAM_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl DStream {
    /// Create DStream from sequence of RDDs
    pub fn create(rdds: Vec<RDD>, batch_duration_ms: u64) -> Self {
        let id = DSTREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            id: id as u64,
            operation: StreamOp::Direct(rdds),
            batch_duration_ms,
            checkpoint_dir: None,
        }
    }

    /// Create empty DStream
    pub fn empty(batch_duration_ms: u64) -> Self {
        let id = DSTREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            id: id as u64,
            operation: StreamOp::Direct(Vec::new()),
            batch_duration_ms,
            checkpoint_dir: None,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn batch_duration(&self) -> u64 {
        self.batch_duration_ms
    }

    /// Set checkpoint directory for fault tolerance
    pub fn checkpoint(&mut self, dir: &str) -> &mut Self {
        self.checkpoint_dir = Some(dir.to_string());
        self
    }

    /// Map transformation
    pub fn map(&self, func: &str) -> DStream {
        DStream {
            id: DSTREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64,
            operation: StreamOp::Map {
                source: Box::new(self.operation.clone()),
                func: func.to_string(),
            },
            batch_duration_ms: self.batch_duration_ms,
            checkpoint_dir: self.checkpoint_dir.clone(),
        }
    }

    /// Filter transformation
    pub fn filter(&self, predicate: &str) -> DStream {
        DStream {
            id: DSTREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64,
            operation: StreamOp::Filter {
                source: Box::new(self.operation.clone()),
                predicate: predicate.to_string(),
            },
            batch_duration_ms: self.batch_duration_ms,
            checkpoint_dir: self.checkpoint_dir.clone(),
        }
    }

    /// Window operation: reduce over sliding window
    pub fn reduce_by_window(&self, window_duration_ms: u64) -> DStream {
        DStream {
            id: DSTREAM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64,
            operation: StreamOp::ReduceWindow {
                source: Box::new(self.operation.clone()),
                duration_ms: window_duration_ms,
            },
            batch_duration_ms: self.batch_duration_ms,
            checkpoint_dir: self.checkpoint_dir.clone(),
        }
    }

    /// Get RDDs (for testing/action)
    pub fn get_rdds(&self) -> Result<Vec<RDD>, String> {
        match &self.operation {
            StreamOp::Direct(rdds) => Ok(rdds.clone()),
            _ => Err("RDDs only available on direct streams".to_string()),
        }
    }

    /// Print action - output elements
    pub fn print(&self) -> Result<(), String> {
        let rdds = self.get_rdds()?;
        for rdd in rdds {
            println!(
                "[{}] Batch at {}: {:?}",
                self.id, rdd.timestamp, rdd.data
            );
        }
        Ok(())
    }

    /// Count elements in each RDD
    pub fn count(&self) -> Result<Vec<usize>, String> {
        let rdds = self.get_rdds()?;
        Ok(rdds.iter().map(|rdd| rdd.count()).collect())
    }

    /// Save to text files
    pub fn save_as_text_files(&self, prefix: &str) -> Result<(), String> {
        let rdds = self.get_rdds()?;
        for rdd in rdds {
            let filename = format!("{}_{}.txt", prefix, rdd.timestamp);
            println!("Would save batch {} to {}", rdd.id, filename);
        }
        Ok(())
    }

    /// Accumulate results (stateful operation)
    pub fn update_state_by_key<F>(&self, func: F) -> Result<Vec<Value>, String>
    where
        F: Fn(Vec<Value>, Option<Value>) -> Value,
    {
        let rdds = self.get_rdds()?;
        let mut state: Option<Value> = None;

        for rdd in rdds {
            for value in rdd.data() {
                state = Some(func(vec![value.clone()], state));
            }
        }

        Ok(state.map(|s| vec![s]).unwrap_or_default())
    }
}

impl std::fmt::Debug for DStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DStream")
            .field("id", &self.id)
            .field("batch_duration_ms", &self.batch_duration_ms)
            .field("has_checkpoint", &self.checkpoint_dir.is_some())
            .finish()
    }
}

/// StreamingContext - manages streaming applications
pub struct StreamingContext {
    batch_duration_ms: u64,
    checkpoint_dir: Option<String>,
}

impl StreamingContext {
    pub fn new(batch_duration_ms: u64) -> Self {
        Self {
            batch_duration_ms,
            checkpoint_dir: None,
        }
    }

    pub fn batch_duration(&self) -> u64 {
        self.batch_duration_ms
    }

    /// Set checkpoint directory
    pub fn checkpoint(&mut self, dir: &str) -> &mut Self {
        self.checkpoint_dir = Some(dir.to_string());
        self
    }

    /// Create streaming context with checkpoint
    pub fn with_checkpoint(batch_duration_ms: u64, checkpoint_dir: &str) -> Self {
        Self {
            batch_duration_ms,
            checkpoint_dir: Some(checkpoint_dir.to_string()),
        }
    }

    /// Create DStream from RDDs
    pub fn create_dstream(&self, rdds: Vec<RDD>) -> DStream {
        let mut dstream = DStream::create(rdds, self.batch_duration_ms);
        if let Some(dir) = &self.checkpoint_dir {
            dstream.checkpoint(dir);
        }
        dstream
    }

    /// Start the context (would begin receiving data)
    pub fn start(&self) -> Result<(), String> {
        println!("StreamingContext started with {}ms batches", self.batch_duration_ms);
        if let Some(dir) = &self.checkpoint_dir {
            println!("Checkpointing to: {}", dir);
        }
        Ok(())
    }

    /// Stop the context
    pub fn stop(&self) -> Result<(), String> {
        println!("StreamingContext stopped");
        Ok(())
    }

    /// Await termination
    pub fn await_termination(&self, timeout_ms: u64) -> Result<(), String> {
        println!("Awaiting termination for {}ms", timeout_ms);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdd_creation() {
        let rdd = RDD::new(1, vec![Value::Number(1.0), Value::Number(2.0)], 0);
        assert_eq!(rdd.count(), 2);
    }

    #[test]
    fn test_dstream_creation() {
        let rdd = RDD::new(1, vec![Value::Number(1.0)], 0);
        let dstream = DStream::create(vec![rdd], 1000);
        assert_eq!(dstream.batch_duration(), 1000);
    }

    #[test]
    fn test_dstream_map() {
        let rdd = RDD::new(1, vec![Value::Number(1.0)], 0);
        let dstream = DStream::create(vec![rdd], 1000);
        let mapped = dstream.map("value * 2");
        assert_eq!(mapped.id, dstream.id + 1);
    }

    #[test]
    fn test_dstream_filter() {
        let rdd = RDD::new(1, vec![Value::Number(1.0)], 0);
        let dstream = DStream::create(vec![rdd], 1000);
        let filtered = dstream.filter("value > 0");
        assert_eq!(filtered.id, dstream.id + 1);
    }

    #[test]
    fn test_dstream_reduce_window() {
        let rdd = RDD::new(1, vec![Value::Number(1.0)], 0);
        let dstream = DStream::create(vec![rdd], 1000);
        let windowed = dstream.reduce_by_window(5000);
        assert_eq!(windowed.batch_duration(), 1000);
    }

    #[test]
    fn test_streaming_context() {
        let ctx = StreamingContext::new(1000);
        assert_eq!(ctx.batch_duration(), 1000);
    }

    #[test]
    fn test_streaming_context_checkpoint() {
        let ctx = StreamingContext::with_checkpoint(1000, "/tmp/spark-checkpoint");
        assert!(ctx.checkpoint_dir.is_some());
    }

    #[test]
    fn test_dstream_count() {
        let rdd1 = RDD::new(1, vec![Value::Number(1.0), Value::Number(2.0)], 0);
        let rdd2 = RDD::new(2, vec![Value::Number(3.0)], 1000);
        let dstream = DStream::create(vec![rdd1, rdd2], 1000);
        let counts = dstream.count().unwrap();
        assert_eq!(counts, vec![2, 1]);
    }
}
