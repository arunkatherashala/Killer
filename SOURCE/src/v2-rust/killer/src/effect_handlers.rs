// Week 7: Effect Handlers & Async Runtime Integration
// Interprets effects and runs async tasks with work-stealing scheduler

use crate::effect_system::{
    Effect, EffectSet, FunctionEffectSignature, IODirection, MemoryKind, 
    AsyncKind, ConcurrentKind, Mutability, EffectAnnotation,
};
use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

thread_local! {
    static EFFECT_HANDLER_RUNTIME: RefCell<EffectHandlerRuntime> = 
        RefCell::new(EffectHandlerRuntime::new());
}

// ============================================================================
// Part 1: Effect Handlers - Interpret effects and produce values
// ============================================================================

/// Trait for effect handlers
pub trait EffectHandler: Send + Sync {
    /// Handle an effect and return a value
    fn handle(&self, effect: &Effect) -> String;
    
    /// Get handler name
    fn name(&self) -> &'static str;
}

/// IO Effect Handler - handles file/console operations
pub struct IOEffectHandler;

impl EffectHandler for IOEffectHandler {
    fn handle(&self, effect: &Effect) -> String {
        match effect {
            Effect::IO { direction, resource } => {
                format!("IOHandle({:?}, {})", direction, resource)
            }
            _ => "InvalidIOEffect".to_string(),
        }
    }

    fn name(&self) -> &'static str {
        "IOEffectHandler"
    }
}

/// Memory Effect Handler - handles memory operations
pub struct MemoryEffectHandler;

impl EffectHandler for MemoryEffectHandler {
    fn handle(&self, effect: &Effect) -> String {
        match effect {
            Effect::Memory { kind, mutability } => {
                format!("MemoryHandle({:?}, {:?})", kind, mutability)
            }
            _ => "InvalidMemoryEffect".to_string(),
        }
    }

    fn name(&self) -> &'static str {
        "MemoryEffectHandler"
    }
}

/// Network Effect Handler - handles network operations
pub struct NetworkEffectHandler;

impl EffectHandler for NetworkEffectHandler {
    fn handle(&self, effect: &Effect) -> String {
        match effect {
            Effect::Network { protocol, direction } => {
                format!("NetworkHandle({}, {:?})", protocol, direction)
            }
            _ => "InvalidNetworkEffect".to_string(),
        }
    }

    fn name(&self) -> &'static str {
        "NetworkEffectHandler"
    }
}

/// Async Effect Handler - handles async operations
pub struct AsyncEffectHandler;

impl EffectHandler for AsyncEffectHandler {
    fn handle(&self, effect: &Effect) -> String {
        match effect {
            Effect::Async { kind } => {
                format!("AsyncHandle({:?})", kind)
            }
            _ => "InvalidAsyncEffect".to_string(),
        }
    }

    fn name(&self) -> &'static str {
        "AsyncEffectHandler"
    }
}

/// Concurrent Effect Handler - handles concurrent operations
pub struct ConcurrentEffectHandler;

impl EffectHandler for ConcurrentEffectHandler {
    fn handle(&self, effect: &Effect) -> String {
        match effect {
            Effect::Concurrent { kind } => {
                format!("ConcurrentHandle({:?})", kind)
            }
            _ => "InvalidConcurrentEffect".to_string(),
        }
    }

    fn name(&self) -> &'static str {
        "ConcurrentEffectHandler"
    }
}

// ============================================================================
// Part 2: Async Task Representation
// ============================================================================

/// Unique task identifier
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TaskId(u64);

/// Represents an async task for execution
#[derive(Clone, Debug)]
pub struct AsyncTask {
    id: TaskId,
    name: String,
    effects: EffectSet,
    priority: u32,
    spawned_at: u64,  // Timestamp
    completed: bool,
}

impl AsyncTask {
    pub fn new(id: TaskId, name: String, effects: EffectSet) -> Self {
        AsyncTask {
            id,
            name,
            effects,
            priority: 0,
            spawned_at: 0,
            completed: false,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }
}

// ============================================================================
// Part 3: Work-Stealing Scheduler for Async Tasks
// ============================================================================

/// Work-stealing queue for concurrent task execution
pub struct WorkStealingQueue {
    /// Local queues per worker thread
    queues: Vec<VecDeque<AsyncTask>>,
    /// Number of workers
    num_workers: usize,
    /// Current worker ID
    current_worker: usize,
    /// Total tasks processed
    total_tasks: u64,
}

impl WorkStealingQueue {
    pub fn new(num_workers: usize) -> Self {
        WorkStealingQueue {
            queues: vec![VecDeque::new(); num_workers],
            num_workers,
            current_worker: 0,
            total_tasks: 0,
        }
    }

    /// Push task to current worker's queue
    pub fn push_task(&mut self, task: AsyncTask) {
        if self.current_worker < self.num_workers {
            self.queues[self.current_worker].push_back(task);
            self.total_tasks += 1;
        }
    }

    /// Try to pop task from current queue (or steal from others)
    pub fn pop_task(&mut self) -> Option<AsyncTask> {
        // Try current worker queue first
        if let Some(task) = self.queues[self.current_worker].pop_front() {
            return Some(task);
        }

        // Try to steal from other queues
        for i in 0..self.num_workers {
            if i != self.current_worker {
                if let Some(task) = self.queues[i].pop_back() {
                    return Some(task);
                }
            }
        }

        None
    }

    /// Get stats
    pub fn stats(&self) -> (usize, u64) {
        let pending: usize = self.queues.iter().map(|q| q.len()).sum();
        (pending, self.total_tasks)
    }

    /// Set active worker
    pub fn set_worker(&mut self, worker_id: usize) {
        self.current_worker = worker_id.min(self.num_workers - 1);
    }
}

// ============================================================================
// Part 4: Effect Handler Runtime
// ============================================================================

/// Runtime for executing effects and managing async tasks
pub struct EffectHandlerRuntime {
    // Effect handlers
    io_handler: Arc<dyn EffectHandler>,
    memory_handler: Arc<dyn EffectHandler>,
    network_handler: Arc<dyn EffectHandler>,
    async_handler: Arc<dyn EffectHandler>,
    concurrent_handler: Arc<dyn EffectHandler>,

    // Task management
    task_queue: WorkStealingQueue,
    task_counter: u64,
    completed_tasks: u64,

    // Execution context
    execution_stats: HashMap<String, u64>,
}

impl EffectHandlerRuntime {
    pub fn new() -> Self {
        EffectHandlerRuntime {
            io_handler: Arc::new(IOEffectHandler),
            memory_handler: Arc::new(MemoryEffectHandler),
            network_handler: Arc::new(NetworkEffectHandler),
            async_handler: Arc::new(AsyncEffectHandler),
            concurrent_handler: Arc::new(ConcurrentEffectHandler),
            task_queue: WorkStealingQueue::new(4),  // 4 default workers
            task_counter: 0,
            completed_tasks: 0,
            execution_stats: HashMap::new(),
        }
    }

    /// Handle an effect
    pub fn handle_effect(&mut self, effect: &Effect) -> String {
        match effect {
            Effect::IO { .. } => {
                let result = self.io_handler.handle(effect);
                *self.execution_stats.entry("io_handled".to_string()).or_insert(0) += 1;
                result
            }
            Effect::Memory { .. } => {
                let result = self.memory_handler.handle(effect);
                *self.execution_stats.entry("memory_handled".to_string()).or_insert(0) += 1;
                result
            }
            Effect::Network { .. } => {
                let result = self.network_handler.handle(effect);
                *self.execution_stats.entry("network_handled".to_string()).or_insert(0) += 1;
                result
            }
            Effect::Async { .. } => {
                let result = self.async_handler.handle(effect);
                *self.execution_stats.entry("async_handled".to_string()).or_insert(0) += 1;
                result
            }
            Effect::Concurrent { .. } => {
                let result = self.concurrent_handler.handle(effect);
                *self.execution_stats.entry("concurrent_handled".to_string()).or_insert(0) += 1;
                result
            }
            _ => "HandledEffect".to_string(),
        }
    }

    /// Spawn an async task
    pub fn spawn_task(&mut self, name: String, effects: EffectSet) -> TaskId {
        let id = TaskId(self.task_counter);
        self.task_counter += 1;

        let task = AsyncTask::new(id, name, effects);
        self.task_queue.push_task(task);

        id
    }

    /// Execute all pending tasks
    pub fn run_all_tasks(&mut self) -> u64 {
        while let Some(mut task) = self.task_queue.pop_task() {
            // Process task effects
            for effect in task.effects.all() {
                self.handle_effect(&effect);
            }

            task.mark_completed();
            self.completed_tasks += 1;
        }

        self.completed_tasks
    }

    /// Get runtime statistics
    pub fn get_stats(&self) -> RuntimeStats {
        let (pending, total) = self.task_queue.stats();

        RuntimeStats {
            total_tasks_spawned: self.task_counter,
            tasks_completed: self.completed_tasks,
            tasks_pending: pending as u64,
            io_effects_handled: self.execution_stats.get("io_handled").copied().unwrap_or(0),
            memory_effects_handled: self.execution_stats.get("memory_handled").copied().unwrap_or(0),
            network_effects_handled: self.execution_stats.get("network_handled").copied().unwrap_or(0),
            async_effects_handled: self.execution_stats.get("async_handled").copied().unwrap_or(0),
            concurrent_effects_handled: self.execution_stats.get("concurrent_handled").copied().unwrap_or(0),
            execution_stats: self.execution_stats.clone(),
        }
    }
}

impl Default for EffectHandlerRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Part 5: Runtime Statistics
// ============================================================================

#[derive(Clone, Debug)]
pub struct RuntimeStats {
    pub total_tasks_spawned: u64,
    pub tasks_completed: u64,
    pub tasks_pending: u64,
    pub io_effects_handled: u64,
    pub memory_effects_handled: u64,
    pub network_effects_handled: u64,
    pub async_effects_handled: u64,
    pub concurrent_effects_handled: u64,
    pub execution_stats: HashMap<String, u64>,
}

// ============================================================================
// Public API
// ============================================================================

pub fn handle_effect(effect: &Effect) -> String {
    EFFECT_HANDLER_RUNTIME.with(|runtime| {
        runtime.borrow_mut().handle_effect(effect)
    })
}

pub fn spawn_async_task(name: String, effects: EffectSet) -> TaskId {
    EFFECT_HANDLER_RUNTIME.with(|runtime| {
        runtime.borrow_mut().spawn_task(name, effects)
    })
}

pub fn run_all_tasks() -> u64 {
    EFFECT_HANDLER_RUNTIME.with(|runtime| {
        runtime.borrow_mut().run_all_tasks()
    })
}

pub fn get_runtime_stats() -> RuntimeStats {
    EFFECT_HANDLER_RUNTIME.with(|runtime| {
        runtime.borrow().get_stats()
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_effect_handler() {
        let handler = IOEffectHandler;
        let effect = Effect::IO {
            direction: IODirection::Read,
            resource: "file.txt".to_string(),
        };
        let result = handler.handle(&effect);
        assert!(result.contains("IOHandle"));
    }

    #[test]
    fn test_async_task_creation() {
        let task = AsyncTask::new(TaskId(0), "test".to_string(), EffectSet::pure());
        assert_eq!(task.id, TaskId(0));
        assert!(!task.completed);
    }

    #[test]
    fn test_work_stealing_queue() {
        let mut queue = WorkStealingQueue::new(2);
        let task = AsyncTask::new(TaskId(0), "test".to_string(), EffectSet::pure());
        queue.push_task(task);

        assert!(queue.pop_task().is_some());
    }

    #[test]
    fn test_effect_handler_runtime() {
        EFFECT_HANDLER_RUNTIME.with(|runtime| {
            let mut r = runtime.borrow_mut();
            let effect = Effect::IO {
                direction: IODirection::Read,
                resource: "test".to_string(),
            };
            let result = r.handle_effect(&effect);
            assert!(!result.is_empty());
        });
    }

    #[test]
    fn test_spawn_and_run_tasks() {
        EFFECT_HANDLER_RUNTIME.with(|runtime| {
            let mut r = runtime.borrow_mut();

            // Spawn task
            r.spawn_task("test_task".to_string(), EffectSet::pure());

            // Execute
            let completed = r.run_all_tasks();
            assert_eq!(completed, 1);
        });
    }

    #[test]
    fn test_runtime_statistics() {
        EFFECT_HANDLER_RUNTIME.with(|runtime| {
            let mut r = runtime.borrow_mut();

            r.spawn_task("task1".to_string(), EffectSet::pure());
            r.spawn_task("task2".to_string(), EffectSet::pure());

            r.run_all_tasks();

            let stats = r.get_stats();
            assert_eq!(stats.tasks_completed, 2);
        });
    }

    #[test]
    fn test_async_with_effects() {
        EFFECT_HANDLER_RUNTIME.with(|runtime| {
            let mut r = runtime.borrow_mut();

            let mut effects = EffectSet::new();
            effects.add(Effect::IO {
                direction: IODirection::Read,
                resource: "input".to_string(),
            });

            r.spawn_task("io_task".to_string(), effects);
            r.run_all_tasks();

            let stats = r.get_stats();
            assert!(stats.io_effects_handled > 0);
        });
    }
}
