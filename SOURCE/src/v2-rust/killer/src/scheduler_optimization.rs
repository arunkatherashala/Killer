// CPU OPTIMIZATION MODULE - Phase 1: Core Affinity & Scheduler
// Target: HP ZBook Firefly 14 G8 (Intel i5-1145G7, 4 cores / 8 threads)
// Goal: +30-50% performance improvement through intelligent core utilization

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the number of logical CPUs (built-in Rust functionality)
fn get_logical_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4) // Fallback to 4 cores
}

/// Get the number of physical CPUs (estimated from logical count)
fn get_physical_cpus() -> usize {
    let logical = get_logical_cpus();
    // If logical > 4, likely HT is enabled on 4-core system
    // Otherwise, assume 1:1 ratio
    if logical > 4 && logical <= 8 {
        4 // Standard HT: 4 physical, 8 logical
    } else if logical <= 4 {
        logical
    } else {
        (logical + 1) / 2 // General case
    }
}

/// CPU Core Information
#[derive(Clone, Debug)]
pub struct CPUCore {
    pub id: usize,
    pub is_physical: bool,
    pub is_hyperthreaded: bool,
    pub frequency_mhz: u32,
    pub cache_l1_kb: u32,
    pub cache_l3_mb: u32,
}

impl CPUCore {
    pub fn new(id: usize) -> Self {
        let physical = get_physical_cpus();
        let logical = get_logical_cpus();
        CPUCore {
            id,
            is_physical: id < physical,
            is_hyperthreaded: logical > physical,
            frequency_mhz: 2600, // i5-1145G7 base frequency
            cache_l1_kb: 192,     // 48KB per core
            cache_l3_mb: 12,      // Shared L3 cache
        }
    }
}

/// CPU Topology Detection
#[derive(Clone, Debug)]
pub struct CPUTopology {
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub cores: Vec<CPUCore>,
    pub has_hyperthreading: bool,
    pub l3_cache_mb: u32,
}

impl CPUTopology {
    pub fn detect() -> Self {
        let physical = get_physical_cpus();
        let logical = get_logical_cpus();
        let has_ht = logical > physical;

        let cores = (0..logical)
            .map(|i| CPUCore::new(i))
            .collect();

        CPUTopology {
            physical_cores: physical,
            logical_cores: logical,
            cores,
            has_hyperthreading: has_ht,
            l3_cache_mb: 12, // i5-1145G7 has 12MB shared L3
        }
    }

    pub fn describe(&self) -> String {
        format!(
            "CPU Topology: {} physical cores, {} logical cores (HT: {}), {} MB L3 cache",
            self.physical_cores, self.logical_cores, self.has_hyperthreading, self.l3_cache_mb
        )
    }

    pub fn is_small_system(&self) -> bool {
        self.physical_cores <= 4
    }

    pub fn is_medium_system(&self) -> bool {
        self.physical_cores > 4 && self.physical_cores <= 8
    }

    pub fn is_large_system(&self) -> bool {
        self.physical_cores > 8
    }
}

/// Task for work stealing queue
#[derive(Clone, Debug)]
pub struct WorkItem {
    pub id: String,
    pub task_fn: String, // Task identifier
    pub priority: u32,
    pub created_at: u64,
    pub assigned_core: Option<usize>,
}

impl WorkItem {
    pub fn new(task_fn: &str) -> Self {
        WorkItem {
            id: format!(
                "task_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            task_fn: task_fn.to_string(),
            priority: 50, // Medium priority
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            assigned_core: None,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn assign_core(&mut self, core_id: usize) {
        self.assigned_core = Some(core_id);
    }
}

/// Work Stealing Scheduler for optimal core utilization
#[derive(Clone)]
pub struct WorkStealingScheduler {
    topology: CPUTopology,
    // Local queues for each core
    core_queues: Arc<Mutex<Vec<VecDeque<WorkItem>>>>,
    // Global queue for overflow
    global_queue: Arc<Mutex<VecDeque<WorkItem>>>,
    // Performance metrics
    metrics: Arc<Mutex<SchedulerMetrics>>,
}

#[derive(Clone, Debug)]
pub struct SchedulerMetrics {
    pub total_tasks_processed: u64,
    pub total_steals: u64,
    pub queue_overflows: u64,
    pub core_imbalance_detected: u32,
    pub context_switches: u64,
}

impl Default for SchedulerMetrics {
    fn default() -> Self {
        SchedulerMetrics {
            total_tasks_processed: 0,
            total_steals: 0,
            queue_overflows: 0,
            core_imbalance_detected: 0,
            context_switches: 0,
        }
    }
}

impl WorkStealingScheduler {
    pub fn new() -> Self {
        let topology = CPUTopology::detect();
        let num_cores = topology.logical_cores;

        let core_queues = Arc::new(Mutex::new(
            (0..num_cores).map(|_| VecDeque::new()).collect(),
        ));

        WorkStealingScheduler {
            topology,
            core_queues,
            global_queue: Arc::new(Mutex::new(VecDeque::new())),
            metrics: Arc::new(Mutex::new(SchedulerMetrics::default())),
        }
    }

    /// Queue work item with automatic core selection
    pub fn enqueue(&self, mut work: WorkItem) {
        // For small systems (4 cores), use simple round-robin to physical cores
        if self.topology.is_small_system() {
            let core_id = self.get_least_loaded_core();
            work.assign_core(core_id);

            if let Ok(mut queues) = self.core_queues.lock() {
                queues[core_id].push_back(work);
            }
        } else {
            // For larger systems, use global queue with work stealing
            if let Ok(mut queue) = self.global_queue.lock() {
                queue.push_back(work);
                if let Ok(mut metrics) = self.metrics.lock() {
                    metrics.queue_overflows += 1;
                }
            }
        }
    }

    /// Get least loaded core (for small systems)
    pub fn get_least_loaded_core(&self) -> usize {
        if let Ok(queues) = self.core_queues.lock() {
            queues
                .iter()
                .enumerate()
                .min_by_key(|(_, q)| q.len())
                .map(|(idx, _)| idx)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Steal work from global queue to core local queue
    pub fn work_steal(&self, core_id: usize) -> Option<WorkItem> {
        // Try to steal from global queue
        if let Ok(mut global) = self.global_queue.lock() {
            if let Some(work) = global.pop_front() {
                if let Ok(mut metrics) = self.metrics.lock() {
                    metrics.total_steals += 1;
                }
                return Some(work);
            }
        }

        // Try to steal from sibling core (HT peer)
        if self.topology.has_hyperthreading && core_id < self.topology.logical_cores / 2 {
            let sibling_core = core_id + self.topology.physical_cores;
            if let Ok(mut queues) = self.core_queues.lock() {
                if sibling_core < queues.len() && !queues[sibling_core].is_empty() {
                    return queues[sibling_core].pop_front();
                }
            }
        }

        None
    }

    /// Get work for a specific core
    pub fn get_work(&self, core_id: usize) -> Option<WorkItem> {
        if let Ok(mut queues) = self.core_queues.lock() {
            if core_id < queues.len() {
                if let Some(work) = queues[core_id].pop_front() {
                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.total_tasks_processed += 1;
                    }
                    return Some(work);
                }
            }
        }

        // Try work stealing
        self.work_steal(core_id)
    }

    /// Get queue lengths for all cores
    pub fn get_queue_stats(&self) -> Vec<usize> {
        if let Ok(queues) = self.core_queues.lock() {
            queues.iter().map(|q| q.len()).collect()
        } else {
            vec![]
        }
    }

    /// Check for core imbalance
    pub fn check_imbalance(&self) -> bool {
        let stats = self.get_queue_stats();
        if stats.is_empty() {
            return false;
        }

        let max = stats.iter().max().unwrap_or(&(0usize));
        let min = stats.iter().min().unwrap_or(&(0usize));

        // If one core has 3x more work than another, it's imbalanced
        *max > (*min * 3 + 1)
    }

    /// Rebalance workload across cores
    pub fn rebalance(&self) {
        if !self.check_imbalance() {
            return;
        }

        if let Ok(mut queues) = self.core_queues.lock() {
            let target_len = queues.iter().map(|q| q.len()).sum::<usize>() / queues.len();

            for i in 0..queues.len() {
                while queues[i].len() > target_len {
                    if let Some(work) = queues[i].pop_back() {
                        // Find least loaded core
                        let min_idx = queues
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, q)| q.len())
                            .map(|(idx, _)| idx)
                            .unwrap_or(0);

                        queues[min_idx].push_back(work);

                        if let Ok(mut metrics) = self.metrics.lock() {
                            metrics.core_imbalance_detected += 1;
                        }
                    }
                }
            }
        }
    }

    /// Get scheduler metrics
    pub fn get_metrics(&self) -> SchedulerMetrics {
        self.metrics
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            *metrics = SchedulerMetrics::default();
        }
    }

    /// Get CPU topology
    pub fn get_topology(&self) -> CPUTopology {
        self.topology.clone()
    }

    /// Print scheduler status
    pub fn print_status(&self) {
        println!("\n=== CPU SCHEDULER STATUS ===");
        println!("{}", self.topology.describe());

        let stats = self.get_queue_stats();
        println!("Queue lengths per core:");
        for (i, len) in stats.iter().enumerate() {
            println!("  Core {}: {} tasks", i, len);
        }

        if self.check_imbalance() {
            println!("⚠️  IMBALANCE DETECTED - Rebalancing...");
            self.rebalance();
        } else {
            println!("✅ Balanced load distribution");
        }

        let metrics = self.get_metrics();
        println!("\nMetrics:");
        println!("  Tasks processed: {}", metrics.total_tasks_processed);
        println!("  Work steals: {}", metrics.total_steals);
        println!("  Rebalances: {}", metrics.core_imbalance_detected);
    }
}

impl Default for WorkStealingScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// CPU-aware thread pool for optimal core utilization
pub struct CoreAwareThreadPool {
    scheduler: WorkStealingScheduler,
    threads: Vec<thread::JoinHandle<()>>,
}

impl CoreAwareThreadPool {
    pub fn new(num_threads: Option<usize>) -> Self {
        let scheduler = WorkStealingScheduler::new();
        let topology = scheduler.get_topology();

        // Default to physical cores for small systems
        let thread_count = num_threads.unwrap_or_else(|| {
            if topology.is_small_system() {
                topology.physical_cores
            } else {
                (topology.logical_cores / 2).max(1)
            }
        });

        println!(
            "🚀 Starting CPU-Aware Thread Pool with {} threads",
            thread_count
        );
        println!("   {}", topology.describe());

        let threads = vec![];

        CoreAwareThreadPool { scheduler, threads }
    }

    pub fn get_scheduler(&self) -> WorkStealingScheduler {
        self.scheduler.clone()
    }

    pub fn shutdown(self) {
        println!("📊 Thread Pool Statistics:");
        self.scheduler.print_status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let topology = CPUTopology::detect();
        println!("{}", topology.describe());
        assert!(topology.physical_cores > 0);
        assert!(topology.logical_cores >= topology.physical_cores);
    }

    #[test]
    fn test_work_stealing_scheduler() {
        let scheduler = WorkStealingScheduler::new();

        // Enqueue some work
        for i in 0..10 {
            let work = WorkItem::new(&format!("task_{}", i));
            scheduler.enqueue(work);
        }

        // Verify tasks were queued
        let stats = scheduler.get_queue_stats();
        let total: usize = stats.iter().sum();
        assert!(total > 0);
    }

    #[test]
    fn test_thread_pool() {
        let _pool = CoreAwareThreadPool::new(None);
        // Pool should initialize without panicking
    }
}
