// Batch Processing & Pipeline Parallelism
// Improved throughput through request batching and async processing
// Week 3: Throughput optimization

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Batch request configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,           // Max requests per batch
    pub max_wait_time_ms: u64,          // Max wait before processing partial batch
    pub dynamic_batching: bool,          // Adjust size based on latency
    pub pipeline_stages: usize,          // Number of pipeline stages
}

impl Default for BatchConfig {
    fn default() -> Self {
        BatchConfig {
            max_batch_size: 32,             // Process 32 requests at once
            max_wait_time_ms: 10,           // Wait max 10ms for full batch
            dynamic_batching: true,
            pipeline_stages: 4,             // 4-stage pipeline
        }
    }
}

/// Individual request in a batch
#[derive(Debug, Clone)]
pub struct BatchRequest {
    pub id: u64,
    pub operation: String,
    pub input: String,
    pub created_at: Instant,
}

/// Batch of requests ready for processing
#[derive(Debug)]
pub struct RequestBatch {
    pub requests: Vec<BatchRequest>,
    pub batch_id: u64,
    pub created_at: Instant,
}

impl RequestBatch {
    pub fn new(batch_id: u64) -> Self {
        RequestBatch {
            requests: Vec::new(),
            batch_id,
            created_at: Instant::now(),
        }
    }

    pub fn size(&self) -> usize {
        self.requests.len()
    }

    pub fn wait_time_ms(&self) -> u64 {
        self.created_at.elapsed().as_millis() as u64
    }

    pub fn is_full(&self, max_size: usize) -> bool {
        self.requests.len() >= max_size
    }

    pub fn avg_request_latency(&self) -> u64 {
        if self.requests.is_empty() {
            return 0;
        }
        let total: u64 = self.requests
            .iter()
            .map(|r| r.created_at.elapsed().as_millis() as u64)
            .sum();
        total / self.requests.len() as u64
    }
}

/// Batch processor with dynamic sizing
#[allow(dead_code)]
pub struct BatchProcessor {
    config: BatchConfig,
    current_batch: VecDeque<BatchRequest>,
    completed_batches: Vec<RequestBatch>,
    batch_counter: u64,
    request_counter: u64,
    metrics: BatchMetrics,
}

#[derive(Debug, Clone, Default)]
pub struct BatchMetrics {
    pub total_batches_processed: u64,
    pub total_requests_processed: u64,
    pub avg_batch_size: f32,
    pub total_batches_waited_full: u64,     // Batches that hit max size
    pub total_batches_timed_out: u64,       // Batches that timed out
    pub total_batches_urgent: u64,          // Batches processed early due to urgent requests
}

impl BatchProcessor {
    pub fn new(config: BatchConfig) -> Self {
        BatchProcessor {
            config,
            current_batch: VecDeque::new(),
            completed_batches: Vec::new(),
            batch_counter: 0,
            request_counter: 0,
            metrics: BatchMetrics::default(),
        }
    }

    /// Add request to batch, returns ready batch if conditions met
    pub fn enqueue(&mut self, operation: &str, input: &str, urgent: bool) -> Option<RequestBatch> {
        self.request_counter += 1;
        let req = BatchRequest {
            id: self.request_counter,
            operation: operation.to_string(),
            input: input.to_string(),
            created_at: Instant::now(),
        };

        self.current_batch.push_back(req);

        // Check if batch should be processed
        let should_process = 
            // Batch is full
            self.current_batch.len() >= self.config.max_batch_size ||
            // Batch has waited too long
            (!self.current_batch.is_empty() && 
             self.first_request_wait_time() > self.config.max_wait_time_ms) ||
            // Urgent request and batch not empty
            (urgent && !self.current_batch.is_empty());

        if should_process {
            return Some(self.finalize_batch());
        }

        None
    }

    /// Finalize current batch and prepare for processing
    pub fn finalize_batch(&mut self) -> RequestBatch {
        self.batch_counter += 1;
        let mut batch = RequestBatch::new(self.batch_counter);

        while let Some(req) = self.current_batch.pop_front() {
            batch.requests.push(req);
        }

        // Update metrics
        self.metrics.total_batches_processed += 1;
        self.metrics.total_requests_processed += batch.size() as u64;
        self.metrics.avg_batch_size = 
            self.metrics.total_requests_processed as f32 / 
            self.metrics.total_batches_processed as f32;

        batch
    }

    fn first_request_wait_time(&self) -> u64 {
        self.current_batch
            .front()
            .map(|r| r.created_at.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn current_batch_size(&self) -> usize {
        self.current_batch.len()
    }

    pub fn get_metrics(&self) -> &BatchMetrics {
        &self.metrics
    }
}

/// Pipeline stage for processing batches
pub struct PipelineStage {
    pub stage_id: usize,
    pub name: String,
    pub input_queue: Arc<Mutex<VecDeque<RequestBatch>>>,
    pub output_queue: Arc<Mutex<VecDeque<RequestBatch>>>,
}

impl PipelineStage {
    pub fn new(stage_id: usize, name: &str) -> Self {
        PipelineStage {
            stage_id,
            name: name.to_string(),
            input_queue: Arc::new(Mutex::new(VecDeque::new())),
            output_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn enqueue_batch(&self, batch: RequestBatch) {
        if let Ok(mut queue) = self.input_queue.lock() {
            queue.push_back(batch);
        }
    }

    pub fn dequeue_batch(&self) -> Option<RequestBatch> {
        if let Ok(mut queue) = self.input_queue.lock() {
            queue.pop_front()
        } else {
            None
        }
    }

    pub fn queue_length(&self) -> usize {
        self.input_queue
            .lock()
            .map(|q| q.len())
            .unwrap_or(0)
    }
}

/// Multi-stage pipeline processor
pub struct Pipeline {
    stages: Vec<PipelineStage>,
    throughput: PipelineThroughput,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineThroughput {
    pub requests_per_second: f32,
    pub avg_pipeline_latency_ms: u64,
    pub stages_utilization: Vec<f32>,  // 0.0-1.0 utilization per stage
}

impl Pipeline {
    pub fn new(num_stages: usize) -> Self {
        let mut stages = Vec::new();
        let stage_names = [
            "Input Normalization",
            "Model Inference",
            "Post Processing",
            "Output Formatting",
        ];

        for i in 0..num_stages {
            let name = stage_names.get(i).unwrap_or(&"Custom Stage");
            stages.push(PipelineStage::new(i, name));
        }

        Pipeline {
            stages,
            throughput: PipelineThroughput::default(),
        }
    }

    pub fn get_stage(&self, idx: usize) -> Option<&PipelineStage> {
        self.stages.get(idx)
    }

    pub fn num_stages(&self) -> usize {
        self.stages.len()
    }

    pub fn get_throughput(&self) -> &PipelineThroughput {
        &self.throughput
    }

    pub fn update_throughput(&mut self, requests_per_sec: f32, latency_ms: u64) {
        self.throughput.requests_per_second = requests_per_sec;
        self.throughput.avg_pipeline_latency_ms = latency_ms;

        // Update utilization based on queue lengths
        self.throughput.stages_utilization = self.stages
            .iter()
            .map(|s| {
                let queue_len = s.queue_length();
                // Rough estimate: queue > 5 means high utilization
                (queue_len as f32 / 10.0).min(1.0)
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_processing() {
        let config = BatchConfig {
            max_batch_size: 4,
            max_wait_time_ms: 100,
            ..Default::default()
        };
        let mut processor = BatchProcessor::new(config);

        // Add 3 requests
        assert!(processor.enqueue("generate", "hello", false).is_none());
        assert!(processor.enqueue("embed", "world", false).is_none());
        assert!(processor.enqueue("classify", "test", false).is_none());

        // 4th request should trigger batch
        let batch = processor.enqueue("extract", "data", false);
        assert!(batch.is_some());
        let batch = batch.unwrap();
        assert_eq!(batch.size(), 4);
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new(4);
        assert_eq!(pipeline.num_stages(), 4);
        assert_eq!(pipeline.get_stage(0).unwrap().stage_id, 0);
    }
}
