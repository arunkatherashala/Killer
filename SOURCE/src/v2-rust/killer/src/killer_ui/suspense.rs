//! **Suspense** — React-equivalent Suspense, lazy loading, and concurrent rendering.
//!
//! `SuspenseBoundary`: wraps children that may suspend (async data loading).
//! `Lazy`: deferred component loading with code-split point.
//! `ConcurrentScheduler`: time-sliced rendering with priority lanes.

use super::patch::{Widget, WidgetId};
use std::collections::{HashMap, VecDeque};

// ══════════════════════════════════════════════════════════════════════════════
// Suspense Boundary
// ══════════════════════════════════════════════════════════════════════════════

/// Status of an async resource backing a suspended component.
#[derive(Debug, Clone, PartialEq)]
pub enum SuspenseStatus {
    Pending,
    Resolved(String),
    Error(String),
}

/// A resource that can suspend rendering until data arrives.
#[derive(Debug, Clone)]
pub struct SuspenseResource {
    pub id: String,
    pub status: SuspenseStatus,
    pub cached_value: Option<String>,
}

impl SuspenseResource {
    pub fn new(id: &str) -> Self {
        SuspenseResource { id: id.into(), status: SuspenseStatus::Pending, cached_value: None }
    }

    pub fn resolve(&mut self, value: String) {
        self.cached_value = Some(value.clone());
        self.status = SuspenseStatus::Resolved(value);
    }

    pub fn reject(&mut self, error: String) {
        self.status = SuspenseStatus::Error(error);
    }

    pub fn is_pending(&self) -> bool { self.status == SuspenseStatus::Pending }
    pub fn is_resolved(&self) -> bool { matches!(self.status, SuspenseStatus::Resolved(_)) }

    /// Read the resource — returns value if resolved, None if pending.
    pub fn read(&self) -> Option<&str> {
        self.cached_value.as_deref()
    }
}

/// Suspense boundary wrapping child widgets.
#[derive(Debug, Clone)]
pub struct SuspenseBoundary {
    pub id: WidgetId,
    pub fallback: Vec<Widget>,
    pub children: Vec<Widget>,
    pub resources: Vec<String>,  // resource IDs this boundary tracks
    pub timeout_ms: u64,
}

impl SuspenseBoundary {
    pub fn new(id: &str) -> Self {
        SuspenseBoundary {
            id: id.into(),
            fallback: vec![Widget::Spinner { id: format!("{id}-spinner"), size: 24.0 }],
            children: Vec::new(),
            resources: Vec::new(),
            timeout_ms: 3000,
        }
    }

    pub fn with_fallback(mut self, fallback: Vec<Widget>) -> Self {
        self.fallback = fallback; self
    }

    pub fn with_children(mut self, children: Vec<Widget>) -> Self {
        self.children = children; self
    }

    pub fn track_resource(mut self, resource_id: &str) -> Self {
        self.resources.push(resource_id.into()); self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms; self
    }

    /// Resolve what to render: children if all resources ready, fallback otherwise.
    pub fn render(&self, resource_store: &ResourceStore) -> Vec<Widget> {
        let all_ready = self.resources.iter().all(|rid| {
            resource_store.get(rid).map(|r| r.is_resolved()).unwrap_or(false)
        });
        if all_ready { self.children.clone() } else { self.fallback.clone() }
    }
}

/// Central store for suspense resources.
pub struct ResourceStore {
    resources: HashMap<String, SuspenseResource>,
}

impl ResourceStore {
    pub fn new() -> Self { ResourceStore { resources: HashMap::new() } }

    pub fn create(&mut self, id: &str) -> &mut SuspenseResource {
        self.resources.entry(id.into()).or_insert_with(|| SuspenseResource::new(id))
    }

    pub fn get(&self, id: &str) -> Option<&SuspenseResource> { self.resources.get(id) }

    pub fn resolve(&mut self, id: &str, value: String) {
        if let Some(r) = self.resources.get_mut(id) { r.resolve(value); }
    }

    pub fn reject(&mut self, id: &str, error: String) {
        if let Some(r) = self.resources.get_mut(id) { r.reject(error); }
    }

    pub fn pending_count(&self) -> usize {
        self.resources.values().filter(|r| r.is_pending()).count()
    }

    pub fn all_resolved(&self) -> bool { self.pending_count() == 0 }
}

impl Default for ResourceStore {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Lazy Components
// ══════════════════════════════════════════════════════════════════════════════

/// Load status for a lazy component.
#[derive(Debug, Clone, PartialEq)]
pub enum LazyStatus {
    NotLoaded,
    Loading,
    Loaded,
    Failed(String),
}

/// Lazy-loaded component definition (code-split point).
#[derive(Debug, Clone)]
pub struct LazyComponent {
    pub component_name: String,
    pub module_path: String,
    pub status: LazyStatus,
    pub preload: bool,
}

impl LazyComponent {
    pub fn new(name: &str, module_path: &str) -> Self {
        LazyComponent {
            component_name: name.into(),
            module_path: module_path.into(),
            status: LazyStatus::NotLoaded,
            preload: false,
        }
    }

    pub fn with_preload(mut self) -> Self { self.preload = true; self }

    pub fn start_loading(&mut self) { self.status = LazyStatus::Loading; }
    pub fn mark_loaded(&mut self) { self.status = LazyStatus::Loaded; }
    pub fn mark_failed(&mut self, err: &str) { self.status = LazyStatus::Failed(err.into()); }
    pub fn is_ready(&self) -> bool { self.status == LazyStatus::Loaded }
}

/// Registry of lazy components.
pub struct LazyRegistry {
    components: HashMap<String, LazyComponent>,
}

impl LazyRegistry {
    pub fn new() -> Self { LazyRegistry { components: HashMap::new() } }

    pub fn register(&mut self, lazy: LazyComponent) {
        self.components.insert(lazy.component_name.clone(), lazy);
    }

    pub fn get(&self, name: &str) -> Option<&LazyComponent> { self.components.get(name) }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut LazyComponent> { self.components.get_mut(name) }

    pub fn preload_all(&mut self) {
        for c in self.components.values_mut() {
            if c.preload && c.status == LazyStatus::NotLoaded {
                c.start_loading();
            }
        }
    }

    pub fn loaded_count(&self) -> usize {
        self.components.values().filter(|c| c.is_ready()).count()
    }

    pub fn total(&self) -> usize { self.components.len() }
}

impl Default for LazyRegistry {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Concurrent Scheduler (React Concurrent Mode equivalent)
// ══════════════════════════════════════════════════════════════════════════════

/// Priority lanes for work scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderPriority {
    Immediate,   // User input responses
    UserBlocking, // Transitions that block UI
    Normal,      // Regular state updates
    Low,         // Background precomputation
    Idle,        // Deferred / offscreen
}

/// Unit of render work.
#[derive(Debug, Clone)]
pub struct RenderTask {
    pub id: u64,
    pub priority: RenderPriority,
    pub component_id: String,
    pub description: String,
    pub completed: bool,
    pub estimated_ms: f64,
}

/// Time-sliced concurrent scheduler.
pub struct ConcurrentScheduler {
    tasks: VecDeque<RenderTask>,
    completed: Vec<u64>,
    next_id: u64,
    time_slice_ms: f64,
    pending_transitions: Vec<u64>,
}

impl ConcurrentScheduler {
    pub fn new() -> Self {
        ConcurrentScheduler {
            tasks: VecDeque::new(),
            completed: Vec::new(),
            next_id: 1,
            time_slice_ms: 5.0,  // 5ms time slices (React uses ~5ms)
            pending_transitions: Vec::new(),
        }
    }

    pub fn set_time_slice(&mut self, ms: f64) { self.time_slice_ms = ms; }

    /// Schedule a render task.
    pub fn schedule(&mut self, priority: RenderPriority, component_id: &str, description: &str, estimated_ms: f64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let task = RenderTask {
            id, priority, component_id: component_id.into(),
            description: description.into(), completed: false, estimated_ms,
        };
        // Insert in priority order
        let pos = self.tasks.iter().position(|t| t.priority > priority).unwrap_or(self.tasks.len());
        self.tasks.insert(pos, task);
        id
    }

    /// Start a transition (lower priority update that can be interrupted).
    pub fn start_transition(&mut self, component_id: &str, description: &str) -> u64 {
        let id = self.schedule(RenderPriority::Normal, component_id, description, 2.0);
        self.pending_transitions.push(id);
        id
    }

    /// Process tasks within the time slice budget.
    pub fn flush(&mut self, available_ms: f64) -> Vec<u64> {
        let mut budget = available_ms.min(self.time_slice_ms);
        let mut completed_this_flush = Vec::new();

        while budget > 0.0 {
            if let Some(mut task) = self.tasks.pop_front() {
                if task.estimated_ms <= budget {
                    budget -= task.estimated_ms;
                    task.completed = true;
                    completed_this_flush.push(task.id);
                    self.completed.push(task.id);
                } else {
                    // Re-queue remaining work
                    task.estimated_ms -= budget;
                    self.tasks.push_front(task);
                    break;
                }
            } else {
                break;
            }
        }

        self.pending_transitions.retain(|id| !completed_this_flush.contains(id));
        completed_this_flush
    }

    /// Check if there are pending tasks.
    pub fn is_pending(&self) -> bool { !self.tasks.is_empty() }

    /// Check if we're inside a transition.
    pub fn is_transitioning(&self) -> bool { !self.pending_transitions.is_empty() }

    pub fn pending_count(&self) -> usize { self.tasks.len() }
    pub fn completed_count(&self) -> usize { self.completed.len() }

    /// Cancel a scheduled task.
    pub fn cancel(&mut self, task_id: u64) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != task_id);
        self.tasks.len() < before
    }

    /// Use Transition — wraps update in lower priority (React's useTransition).
    pub fn use_transition(&mut self, component_id: &str, updates: Vec<(&str, f64)>) -> Vec<u64> {
        updates.iter().map(|(desc, _est)| {
            self.start_transition(component_id, desc)
        }).collect()
    }

    /// useDeferredValue — schedule at Idle priority.
    pub fn use_deferred_value(&mut self, component_id: &str, description: &str) -> u64 {
        self.schedule(RenderPriority::Idle, component_id, description, 1.0)
    }
}

impl Default for ConcurrentScheduler {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspense_resource_lifecycle() {
        let mut store = ResourceStore::new();
        store.create("users");
        assert!(store.get("users").unwrap().is_pending());
        assert_eq!(store.pending_count(), 1);
        store.resolve("users", r#"[{"name":"Alice"}]"#.into());
        assert!(store.get("users").unwrap().is_resolved());
        assert_eq!(store.pending_count(), 0);
    }

    #[test]
    fn suspense_boundary_renders_fallback() {
        let store = ResourceStore::new();
        let boundary = SuspenseBoundary::new("sb1")
            .track_resource("data")
            .with_children(vec![Widget::Label { id: "content".into(), text: "Loaded!".into() }]);
        let rendered = boundary.render(&store);
        // Resource doesn't exist → fallback
        assert!(matches!(rendered[0], Widget::Spinner { .. }));
    }

    #[test]
    fn suspense_boundary_renders_children() {
        let mut store = ResourceStore::new();
        store.create("data");
        store.resolve("data", "done".into());
        let boundary = SuspenseBoundary::new("sb2")
            .track_resource("data")
            .with_children(vec![Widget::Label { id: "ok".into(), text: "Done".into() }]);
        let rendered = boundary.render(&store);
        assert!(matches!(rendered[0], Widget::Label { .. }));
    }

    #[test]
    fn lazy_component() {
        let mut registry = LazyRegistry::new();
        registry.register(LazyComponent::new("Dashboard", "pages/dashboard.killer").with_preload());
        registry.register(LazyComponent::new("Settings", "pages/settings.killer"));
        assert_eq!(registry.total(), 2);
        assert_eq!(registry.loaded_count(), 0);
        registry.preload_all();
        assert!(matches!(registry.get("Dashboard").unwrap().status, LazyStatus::Loading));
        assert!(matches!(registry.get("Settings").unwrap().status, LazyStatus::NotLoaded));
        registry.get_mut("Dashboard").unwrap().mark_loaded();
        assert_eq!(registry.loaded_count(), 1);
    }

    #[test]
    fn concurrent_scheduler_priority() {
        let mut sched = ConcurrentScheduler::new();
        sched.schedule(RenderPriority::Low, "bg", "precompute", 1.0);
        sched.schedule(RenderPriority::Immediate, "input", "keystroke", 0.5);
        sched.schedule(RenderPriority::Normal, "list", "update", 2.0);
        assert_eq!(sched.pending_count(), 3);
        let done = sched.flush(1.0);
        // Immediate (0.5ms) should complete first
        assert!(!done.is_empty());
    }

    #[test]
    fn concurrent_flush_time_slice() {
        let mut sched = ConcurrentScheduler::new();
        sched.set_time_slice(3.0);
        sched.schedule(RenderPriority::Normal, "a", "task1", 1.0);
        sched.schedule(RenderPriority::Normal, "b", "task2", 1.0);
        sched.schedule(RenderPriority::Normal, "c", "task3", 1.0);
        sched.schedule(RenderPriority::Normal, "d", "task4", 1.0);
        let done = sched.flush(3.0);
        assert_eq!(done.len(), 3); // 3 fit in 3ms budget
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn use_transition() {
        let mut sched = ConcurrentScheduler::new();
        let ids = sched.use_transition("search", vec![("filter", 2.0), ("sort", 1.0)]);
        assert_eq!(ids.len(), 2);
        assert!(sched.is_transitioning());
        sched.flush(10.0);
        assert!(!sched.is_transitioning());
    }

    #[test]
    fn use_deferred_value() {
        let mut sched = ConcurrentScheduler::new();
        sched.schedule(RenderPriority::Immediate, "x", "fast", 0.5);
        sched.use_deferred_value("search", "filter results");
        // Immediate runs first
        let done = sched.flush(1.0);
        assert!(done.len() >= 1);
    }

    #[test]
    fn cancel_task() {
        let mut sched = ConcurrentScheduler::new();
        let id = sched.schedule(RenderPriority::Normal, "x", "test", 1.0);
        assert!(sched.cancel(id));
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn resource_reject() {
        let mut store = ResourceStore::new();
        store.create("bad");
        store.reject("bad", "404 Not Found".into());
        assert!(matches!(store.get("bad").unwrap().status, SuspenseStatus::Error(_)));
    }
}
