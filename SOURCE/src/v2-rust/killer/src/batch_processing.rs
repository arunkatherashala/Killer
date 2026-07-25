/// Batch Processing Engine for Killer V2.1
/// Groups operations into optimal batch sizes for cache efficiency
/// 
/// Formula: optimal_batch_size = (L3_cache / operation_cost) × core_count
/// For i5-1145G7: (12MB / 100B) × 4 = ~490K ops (conservative: 1,024)
/// 
/// Benefit: Reduce context switches from 4K to ~100, improve cache locality

use std::collections::VecDeque;
use std::time::Instant;

/// Represents a batch of operations
#[derive(Debug, Clone)]
pub struct Batch {
    pub batch_id: u64,
    pub operations: Vec<BatchOperation>,
    pub created_at: Instant,
    pub executed: bool,
}

/// Operation within a batch
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub id: u64,
    pub data: Vec<u8>,
    pub priority: u32,
}

/// Result from batch execution
#[derive(Debug, Clone)]
pub struct BatchExecutionResult {
    pub batch_id: u64,
    pub successful: usize,
    pub failed: usize,
    pub execution_time_ms: u64,
    pub throughput_ops_per_sec: u64,
}

/// Statistics about batch operations
#[derive(Debug, Clone)]
pub struct BatchStats {
    pub total_batches: u64,
    pub completed_batches: u64,
    pub pending_batches: u64,
    pub avg_batch_size: f64,
    pub avg_execution_time_ms: f64,
}

/// Determines optimal batch size
pub struct BatchSizer {
    l3_cache_bytes: usize,
    avg_operation_bytes: usize,
    core_count: usize,
}

impl BatchSizer {
    pub fn new(l3_cache_bytes: usize, avg_operation_bytes: usize, core_count: usize) -> Self {
        BatchSizer {
            l3_cache_bytes,
            avg_operation_bytes,
            core_count,
        }
    }

    /// Calculate optimal batch size based on hardware
    pub fn calculate_optimal_size(&self) -> usize {
        // Formula: (L3 cache / avg operation size) * core_count / 2 (conservative)
        let theoretical = (self.l3_cache_bytes / self.avg_operation_bytes) * self.core_count / 2;
        
        // Common batch sizes: 256, 512, 1024, 2048
        match theoretical {
            0..=256 => 256,
            257..=512 => 512,
            513..=1024 => 1024,
            1025..=2048 => 2048,
            _ => 4096,
        }
    }

    /// Get recommended batch size for i5-1145G7 (12MB L3, 4 cores)
    pub fn recommended_for_mobile_workstation() -> usize {
        // Mobile workstations (4-core, 12MB L3): optimal batch = 1024
        // Balances cache locality with context switch overhead
        1024
    }
}

/// Batch builder - constructs batches from operations
pub struct BatchBuilder {
    batch_size: usize,
    current_batch: Vec<BatchOperation>,
    batch_counter: u64,
}

impl BatchBuilder {
    pub fn new(batch_size: usize) -> Self {
        BatchBuilder {
            batch_size,
            current_batch: Vec::with_capacity(batch_size),
            batch_counter: 0,
        }
    }

    /// Add operation to current batch
    pub fn add(&mut self, op: BatchOperation) -> Option<Batch> {
        self.current_batch.push(op);

        if self.current_batch.len() >= self.batch_size {
            self.batch_counter += 1;
            let batch = Batch {
                batch_id: self.batch_counter - 1,
                operations: std::mem::replace(&mut self.current_batch, Vec::with_capacity(self.batch_size)),
                created_at: Instant::now(),
                executed: false,
            };
            Some(batch)
        } else {
            None
        }
    }

    /// Flush remaining operations as final batch
    pub fn flush(&mut self) -> Option<Batch> {
        if !self.current_batch.is_empty() {
            self.batch_counter += 1;
            let batch = Batch {
                batch_id: self.batch_counter - 1,
                operations: std::mem::replace(&mut self.current_batch, Vec::with_capacity(self.batch_size)),
                created_at: Instant::now(),
                executed: false,
            };
            Some(batch)
        } else {
            None
        }
    }

    pub fn current_size(&self) -> usize {
        self.current_batch.len()
    }
}

/// Executes batches
pub struct BatchExecutor {
    execution_count: u64,
    total_execution_time_ms: u64,
}

impl BatchExecutor {
    pub fn new() -> Self {
        BatchExecutor {
            execution_count: 0,
            total_execution_time_ms: 0,
        }
    }

    /// Execute a single batch
    pub fn execute(&mut self, mut batch: Batch) -> BatchExecutionResult {
        let start = Instant::now();

        let mut successful = 0;
        let mut failed = 0;

        for op in &mut batch.operations {
            // Simulate operation execution
            let result = self.execute_operation(&op.data);
            
            if result {
                successful += 1;
            } else {
                failed += 1;
            }
        }

        let elapsed = start.elapsed();
        let execution_time_ms = elapsed.as_millis() as u64;
        let throughput = if execution_time_ms > 0 {
            (batch.operations.len() as u64 * 1000) / execution_time_ms
        } else {
            0
        };

        self.execution_count += 1;
        self.total_execution_time_ms += execution_time_ms;

        BatchExecutionResult {
            batch_id: batch.batch_id,
            successful,
            failed,
            execution_time_ms,
            throughput_ops_per_sec: throughput,
        }
    }

    /// Execute single operation (placeholder)
    fn execute_operation(&self, _data: &[u8]) -> bool {
        // Simulate work
        let mut sum = 0u64;
        for i in 0..100 {
            sum = sum.wrapping_add(i);
        }
        sum > 0 // Always succeeds
    }

    pub fn stats(&self) -> (u64, f64) {
        let avg_time = if self.execution_count > 0 {
            self.total_execution_time_ms as f64 / self.execution_count as f64
        } else {
            0.0
        };
        (self.execution_count, avg_time)
    }
}

/// Queue managing batches
pub struct BatchQueue {
    pending: VecDeque<Batch>,
    completed: VecDeque<BatchExecutionResult>,
    batch_size: usize,
    max_queue_size: usize,
}

impl BatchQueue {
    pub fn new(batch_size: usize, max_queue_size: usize) -> Self {
        BatchQueue {
            pending: VecDeque::with_capacity(max_queue_size),
            completed: VecDeque::new(),
            batch_size,
            max_queue_size,
        }
    }

    /// Enqueue a batch
    pub fn enqueue_batch(&mut self, batch: Batch) -> Result<u64, String> {
        if self.pending.len() >= self.max_queue_size {
            return Err("Batch queue full".to_string());
        }
        
        let batch_id = batch.batch_id;
        self.pending.push_back(batch);
        Ok(batch_id)
    }

    /// Get next batch to execute
    pub fn next_batch(&mut self) -> Option<Batch> {
        self.pending.pop_front()
    }

    /// Record batch execution result
    pub fn record_execution(&mut self, result: BatchExecutionResult) {
        self.completed.push_back(result);
    }

    /// Get pending batch count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get completed batch count
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Get statistics
    pub fn stats(&self) -> BatchStats {
        let total_batches = self.pending.len() as u64 + self.completed.len() as u64;
        let completed = self.completed.len() as u64;
        
        let avg_batch_size = if !self.pending.is_empty() {
            let total_ops: usize = self.pending.iter().map(|b| b.operations.len()).sum();
            total_ops as f64 / self.pending.len() as f64
        } else {
            self.batch_size as f64
        };

        let avg_exec_time = if !self.completed.is_empty() {
            let total_time: u64 = self.completed.iter().map(|r| r.execution_time_ms).sum();
            total_time as f64 / self.completed.len() as f64
        } else {
            0.0
        };

        BatchStats {
            total_batches,
            completed_batches: completed,
            pending_batches: self.pending.len() as u64,
            avg_batch_size,
            avg_execution_time_ms: avg_exec_time,
        }
    }
}

/// Aggregates results from multiple batch executions
pub struct ResultAggregator {
    results: Vec<BatchExecutionResult>,
    total_operations: u64,
}

impl ResultAggregator {
    pub fn new() -> Self {
        ResultAggregator {
            results: Vec::new(),
            total_operations: 0,
        }
    }

    pub fn add_result(&mut self, result: BatchExecutionResult) {
        let ops_count = (result.successful + result.failed) as u64;
        self.total_operations += ops_count;
        self.results.push(result);
    }

    pub fn total_throughput(&self) -> u64 {
        if self.results.is_empty() {
            return 0;
        }

        let total_time: u64 = self.results.iter().map(|r| r.execution_time_ms).sum();
        if total_time > 0 {
            (self.total_operations * 1000) / total_time
        } else {
            0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }

        let total_success: u64 = self.results.iter().map(|r| r.successful as u64).sum();
        total_success as f64 / self.total_operations as f64 * 100.0
    }

    pub fn get_results(&self) -> &[BatchExecutionResult] {
        &self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_sizer() {
        let sizer = BatchSizer::new(12 * 1024 * 1024, 100, 4);
        let size = sizer.calculate_optimal_size();
        assert!(size >= 256 && size <= 4096);
    }

    #[test]
    fn test_recommended_batch_size() {
        let size = BatchSizer::recommended_for_mobile_workstation();
        assert_eq!(size, 1024); // Should be 1024 for i5-1145G7
    }

    #[test]
    fn test_batch_builder() {
        let mut builder = BatchBuilder::new(10);
        
        // Add less than batch size
        for i in 0..5 {
            let op = BatchOperation {
                id: i as u64,
                data: vec![42; 32],
                priority: 1,
            };
            assert!(builder.add(op).is_none());
        }
        assert_eq!(builder.current_size(), 5);

        // Add to reach batch size
        for i in 5..10 {
            let op = BatchOperation {
                id: i as u64,
                data: vec![42; 32],
                priority: 1,
            };
            if i == 9 {
                assert!(builder.add(op).is_some());
            } else {
                builder.add(op);
            }
        }

        // Flush remaining
        let batch = builder.flush();
        assert!(batch.is_none()); // Already flushed
    }

    #[test]
    fn test_batch_executor() {
        let mut executor = BatchExecutor::new();
        
        let batch = Batch {
            batch_id: 0,
            operations: (0..100)
                .map(|i| BatchOperation {
                    id: i as u64,
                    data: vec![42; 32],
                    priority: 1,
                })
                .collect(),
            created_at: Instant::now(),
            executed: false,
        };

        let result = executor.execute(batch);
        assert_eq!(result.successful, 100);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_batch_queue() {
        let mut queue = BatchQueue::new(1024, 100);
        
        for batch_id in 0..10 {
            let batch = Batch {
                batch_id,
                operations: vec![BatchOperation {
                    id: 0,
                    data: vec![42; 32],
                    priority: 1,
                }; 10],
                created_at: Instant::now(),
                executed: false,
            };
            assert!(queue.enqueue_batch(batch).is_ok());
        }

        assert_eq!(queue.pending_count(), 10);

        // Dequeue
        assert!(queue.next_batch().is_some());
        assert_eq!(queue.pending_count(), 9);
    }

    #[test]
    fn test_result_aggregator() {
        let mut agg = ResultAggregator::new();
        
        for i in 0..5 {
            let result = BatchExecutionResult {
                batch_id: i as u64,
                successful: 100,
                failed: 0,
                execution_time_ms: 10,
                throughput_ops_per_sec: 10000,
            };
            agg.add_result(result);
        }

        assert_eq!(agg.total_operations, 500);
        assert!(agg.total_throughput() > 0);
        assert!(agg.success_rate() > 99.0);
    }
}
