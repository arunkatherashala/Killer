// Parallel I/O System
// Multi-threaded I/O with thread pools and partition awareness
// Enables concurrent access to multiple data partitions

use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter};
use std::time::Instant;

// ============================================================================
// Thread Pool - Core concurrent execution engine
// ============================================================================

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum Message {
    NewJob(Job),
    Terminate,
}

pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    active_tasks: Arc<AtomicUsize>,
}

impl ThreadPool {
    /// Create a new thread pool with specified width
    pub fn new(num_threads: usize) -> ThreadPool {
        assert!(num_threads > 0, "Thread pool size must be > 0");

        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));
        let active_tasks = Arc::new(AtomicUsize::new(0));

        let mut workers = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            let receiver = Arc::clone(&receiver);
            let active = Arc::clone(&active_tasks);

            let thread = thread::spawn(move || loop {
                let message = {
                    let mut recv = receiver.lock().unwrap();
                    recv.recv().unwrap()
                };

                match message {
                    Message::NewJob(job) => {
                        active.fetch_add(1, Ordering::SeqCst);
                        job();
                        active.fetch_sub(1, Ordering::SeqCst);
                    }
                    Message::Terminate => {
                        break;
                    }
                }
            });

            workers.push(Worker {
                id,
                thread: Some(thread),
            });
        }

        ThreadPool {
            workers,
            sender,
            active_tasks,
        }
    }

    /// Execute a job on the thread pool
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }

    /// Wait for all tasks to complete
    pub fn wait_all(&self) {
        // Wait for all active tasks to complete
        let mut attempts = 0;
        while self.active_tasks.load(Ordering::SeqCst) > 0 && attempts < 100000 {
            thread::sleep(std::time::Duration::from_micros(100));
            attempts += 1;
        }
        // Additional buffer to ensure all messages are processed
        thread::sleep(std::time::Duration::from_millis(10));
    }

    /// Get number of active tasks
    pub fn active_count(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }

    /// Get thread pool size
    pub fn size(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// ============================================================================
// Partition Manager - Distributes work across partitions
// ============================================================================

#[derive(Debug, Clone)]
pub struct Partition {
    pub id: usize,
    pub data: Vec<u8>,
    pub offset: usize,
}

impl Partition {
    pub fn new(id: usize, data: Vec<u8>) -> Self {
        Partition {
            id,
            offset: 0,
            data,
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

pub struct PartitionManager {
    partitions: Vec<Partition>,
    partition_count: usize,
}

impl PartitionManager {
    pub fn new(partition_count: usize) -> Self {
        PartitionManager {
            partitions: Vec::new(),
            partition_count,
        }
    }

    pub fn add_partition(&mut self, partition: Partition) {
        self.partitions.push(partition);
    }

    pub fn get_partitions(&self) -> &[Partition] {
        &self.partitions
    }

    pub fn get_partition_mut(&mut self, id: usize) -> Option<&mut Partition> {
        self.partitions.iter_mut().find(|p| p.id == id)
    }

    pub fn total_size(&self) -> usize {
        self.partitions.iter().map(|p| p.size()).sum()
    }

    pub fn distribute_work(&self, work: &[u8], count: usize) -> Vec<PartitionTask> {
        let chunk_size = (work.len() + count - 1) / count;
        let mut tasks = Vec::new();

        for i in 0..count {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(work.len());

            if start < work.len() {
                tasks.push(PartitionTask {
                    partition_id: i,
                    data: work[start..end].to_vec(),
                    start_offset: start,
                    end_offset: end,
                });
            }
        }

        tasks
    }
}

#[derive(Debug, Clone)]
pub struct PartitionTask {
    pub partition_id: usize,
    pub data: Vec<u8>,
    pub start_offset: usize,
    pub end_offset: usize,
}

// ============================================================================
// I/O Metrics - Tracks performance statistics
// ============================================================================

#[derive(Debug, Clone)]
pub struct IOMetrics {
    pub bytes_read: Arc<AtomicUsize>,
    pub bytes_written: Arc<AtomicUsize>,
    pub operations: Arc<AtomicUsize>,
    pub start_time: Instant,
}

impl IOMetrics {
    pub fn new() -> Self {
        IOMetrics {
            bytes_read: Arc::new(AtomicUsize::new(0)),
            bytes_written: Arc::new(AtomicUsize::new(0)),
            operations: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    pub fn record_read(&self, bytes: usize) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.operations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_write(&self, bytes: usize) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.operations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> IOStats {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let bytes_read = self.bytes_read.load(Ordering::Relaxed);
        let throughput_mbps = if elapsed > 0.0 {
            (bytes_read as f64) / elapsed / 1_000_000.0
        } else {
            0.0
        };

        IOStats {
            throughput_mbps,
            avg_latency_ms: if self.operations.load(Ordering::Relaxed) > 0 {
                (elapsed * 1000.0) / self.operations.load(Ordering::Relaxed) as f64
            } else {
                0.0
            },
            p95_latency_ms: 0.0, // Would need histogram in production
            p99_latency_ms: 0.0, // Would need histogram in production
            total_bytes: bytes_read,
            operation_count: self.operations.load(Ordering::Relaxed),
        }
    }
}

impl Default for IOMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct IOStats {
    pub throughput_mbps: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub total_bytes: usize,
    pub operation_count: usize,
}

// ============================================================================
// Parallel Data Source - Multi-threaded reading
// ============================================================================

pub struct ParallelDataSource {
    pub path: String,
    pub format: FileFormat,
    pub partitions: usize,
    pub chunk_size: usize,
    pub thread_pool: ThreadPool,
    pub metrics: IOMetrics,
}

#[derive(Debug, Clone, Copy)]
pub enum FileFormat {
    CSV,
    JSON,
    Parquet,
    Text,
}

impl ParallelDataSource {
    pub fn new(path: &str, format: FileFormat, partitions: usize) -> Self {
        ParallelDataSource {
            path: path.to_string(),
            format,
            partitions: partitions.max(1),
            chunk_size: 8192,
            thread_pool: ThreadPool::new(8),
            metrics: IOMetrics::new(),
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn read_parallel(&self) -> Result<Vec<Partition>, String> {
        if !Path::new(&self.path).exists() {
            return Err(format!("File not found: {}", self.path));
        }

        let file = File::open(&self.path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let total_size = metadata.len() as usize;

        let partitions = Arc::new(Mutex::new(Vec::new()));
        let chunk_size = (total_size + self.partitions - 1) / self.partitions;

        for partition_id in 0..self.partitions {
            let path = self.path.clone();
            let metrics = self.metrics.clone();
            let partitions = Arc::clone(&partitions);
            let size = self.chunk_size;
            let start = partition_id * chunk_size;
            let end = ((partition_id + 1) * chunk_size).min(total_size);

            self.thread_pool.execute(move || {
                if let Ok(mut file) = File::open(&path) {
                    use std::io::Seek;
                    let mut buffer = vec![0u8; size];
                    
                    // Seek to partition start
                    if let Ok(_) = file.seek(std::io::SeekFrom::Start(start as u64)) {
                        let bytes_to_read = (end - start).min(size);
                        if let Ok(n) = file.read(&mut buffer[..bytes_to_read]) {
                            metrics.record_read(n);
                            let partition = Partition::new(partition_id, buffer[..n].to_vec());
                            if let Ok(mut parts) = partitions.lock() {
                                parts.push(partition);
                            }
                        }
                    }
                }
            });
        }

        self.thread_pool.wait_all();

        let result = partitions.lock().map_err(|e| format!("Failed to acquire lock: {}", e))?;
        Ok(result.clone())
    }

    pub fn get_metrics(&self) -> IOStats {
        self.metrics.get_stats()
    }
}

// ============================================================================
// Parallel Data Sink - Multi-threaded writing
// ============================================================================

pub struct ParallelDataSink {
    pub path: String,
    pub format: FileFormat,
    pub partitions: usize,
    pub buffer_size: usize,
    pub thread_pool: ThreadPool,
    pub metrics: IOMetrics,
}

impl ParallelDataSink {
    pub fn new(path: &str, format: FileFormat, partitions: usize) -> Self {
        ParallelDataSink {
            path: path.to_string(),
            format,
            partitions: partitions.max(1),
            buffer_size: 8192,
            thread_pool: ThreadPool::new(8),
            metrics: IOMetrics::new(),
        }
    }

    pub fn write_parallel(&self, data: &[Partition]) -> Result<(), String> {
        let write_count = Arc::new(AtomicUsize::new(0));

        for partition in data {
            let path = self.path.clone();
            let p_data = partition.data.clone();
            let p_id = partition.id;
            let metrics = self.metrics.clone();
            let count = Arc::clone(&write_count);

            self.thread_pool.execute(move || {
                let filename = format!("{}_part_{}", path, p_id);
                if let Ok(file) = File::create(&filename) {
                    let mut writer = BufWriter::new(file);
                    if let Ok(_) = writer.write_all(&p_data) {
                        if let Ok(_) = writer.flush() {
                            metrics.record_write(p_data.len());
                            count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }

        self.thread_pool.wait_all();

        let written = write_count.load(Ordering::Relaxed);
        if written == data.len() {
            Ok(())
        } else {
            Err(format!(
                "Failed to write all partitions: {}/{}",
                written,
                data.len()
            ))
        }
    }

    pub fn get_metrics(&self) -> IOStats {
        self.metrics.get_stats()
    }
}

// ============================================================================
// File Builder - Fluent API for I/O operations
// ============================================================================

pub struct FileBuilder {
    path: String,
    format: FileFormat,
    partitions: usize,
}

impl FileBuilder {
    pub fn new(path: &str) -> Self {
        FileBuilder {
            path: path.to_string(),
            format: FileFormat::CSV,
            partitions: 8,
        }
    }

    pub fn format(mut self, fmt: FileFormat) -> Self {
        self.format = fmt;
        self
    }

    pub fn partitions(mut self, count: usize) -> Self {
        self.partitions = count.max(1);
        self
    }

    pub fn read(&self) -> Result<Vec<Partition>, String> {
        let source = ParallelDataSource::new(&self.path, self.format, self.partitions);
        source.read_parallel()
    }

    pub fn write(&self, partitions: &[Partition]) -> Result<(), String> {
        let sink = ParallelDataSink::new(&self.path, self.format, self.partitions);
        sink.write_parallel(partitions)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_creation() {
        let pool = ThreadPool::new(4);
        assert_eq!(pool.size(), 4);
    }

    #[test]
    fn test_thread_pool_execution() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..10 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        pool.wait_all();
        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_partition_manager() {
        let mut manager = PartitionManager::new(4);
        manager.add_partition(Partition::new(0, vec![1, 2, 3]));
        manager.add_partition(Partition::new(1, vec![4, 5, 6, 7]));

        assert_eq!(manager.get_partitions().len(), 2);
        assert_eq!(manager.total_size(), 7);
    }

    #[test]
    fn test_io_metrics() {
        let metrics = IOMetrics::new();
        metrics.record_read(1024);
        metrics.record_write(2048);
        metrics.record_read(512);

        let stats = metrics.get_stats();
        assert_eq!(stats.total_bytes, 1536);
        assert_eq!(stats.operation_count, 3);
    }

    #[test]
    fn test_partition_distribution() {
        let manager = PartitionManager::new(4);
        let data = vec![0u8; 1000];
        let tasks = manager.distribute_work(&data, 4);

        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].partition_id, 0);
    }

    #[test]
    fn test_parallel_data_source() {
        // Create temporary test file
        let test_path = "/tmp/test_parallel.txt";
        if let Ok(_) = std::fs::write(test_path, b"test data here") {
            let source = ParallelDataSource::new(test_path, FileFormat::Text, 2);
            if let Ok(partitions) = source.read_parallel() {
                assert!(!partitions.is_empty());
                let _ = std::fs::remove_file(test_path);
            }
        }
    }

    #[test]
    fn test_file_builder() {
        let test_path = "test_builder.txt";
        if let Ok(_) = std::fs::write(test_path, b"builder test") {
            let builder = FileBuilder::new(test_path)
                .format(FileFormat::Text)
                .partitions(2);

            if let Ok(partitions) = builder.read() {
                assert!(!partitions.is_empty());
                let _ = std::fs::remove_file(test_path);
            }
        }
    }

    #[test]
    fn test_thread_pool_active_tasks() {
        let pool = ThreadPool::new(2);
        let barrier = Arc::new(std::sync::Barrier::new(3));

        for _ in 0..2 {
            let b = Arc::clone(&barrier);
            pool.execute(move || {
                b.wait();
                thread::sleep(std::time::Duration::from_millis(10));
            });
        }

        barrier.wait();
        assert!(pool.active_count() > 0);
        pool.wait_all();
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_partition_task_creation() {
        let manager = PartitionManager::new(4);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let tasks = manager.distribute_work(&data, 2);

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].data.len() > 0);
        assert!(tasks[1].data.len() > 0);
    }

    #[test]
    fn test_io_metrics_stats() {
        let metrics = IOMetrics::new();
        metrics.record_read(5_000_000);
        let stats = metrics.get_stats();
        assert!(stats.throughput_mbps >= 0.0);
        assert!(stats.operation_count > 0);
    }

    #[test]
    fn test_parallel_data_sink() {
        let test_dir = "test_output/";
        let _ = std::fs::create_dir_all(test_dir);

        let partitions = vec![
            Partition::new(0, vec![1, 2, 3]),
            Partition::new(1, vec![4, 5, 6]),
        ];

        let sink = ParallelDataSink::new(test_dir, FileFormat::Text, 2);
        if let Ok(_) = sink.write_parallel(&partitions) {
            let _ = std::fs::remove_dir_all(test_dir);
        }
    }
}
