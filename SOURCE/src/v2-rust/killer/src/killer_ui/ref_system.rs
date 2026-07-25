//! **Ref System** — React ref forwarding, useImperativeHandle, Profiler.
//!
//! Forward refs through component trees, expose imperative APIs,
//! and measure render performance per subtree.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Refs
// ══════════════════════════════════════════════════════════════════════════════

/// A ref to a DOM node or component instance.
#[derive(Debug, Clone)]
pub struct Ref {
    pub id: String,
    pub current: Option<RefValue>,
}

/// What a ref can point to.
#[derive(Debug, Clone)]
pub enum RefValue {
    DomNode(String),
    ComponentInstance(String),
    Custom(HashMap<String, String>),
}

impl Ref {
    pub fn new(id: &str) -> Self { Ref { id: id.into(), current: None } }

    pub fn attach(&mut self, val: RefValue) { self.current = Some(val); }
    pub fn detach(&mut self) { self.current = None; }
    pub fn is_attached(&self) -> bool { self.current.is_some() }
}

/// Forward a ref through a component boundary.
#[derive(Debug, Clone)]
pub struct ForwardedRef {
    pub source_component: String,
    pub target_component: String,
    pub ref_id: String,
}

/// Registry for forwarded refs.
#[derive(Debug, Default)]
pub struct RefRegistry {
    pub refs: HashMap<String, Ref>,
    pub forwards: Vec<ForwardedRef>,
}

impl RefRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn create_ref(&mut self, id: &str) -> &Ref {
        self.refs.entry(id.into()).or_insert_with(|| Ref::new(id));
        &self.refs[id]
    }

    pub fn attach(&mut self, id: &str, val: RefValue) {
        if let Some(r) = self.refs.get_mut(id) { r.attach(val); }
    }

    pub fn forward(&mut self, source: &str, target: &str, ref_id: &str) {
        self.forwards.push(ForwardedRef {
            source_component: source.into(),
            target_component: target.into(),
            ref_id: ref_id.into(),
        });
    }

    pub fn get(&self, id: &str) -> Option<&Ref> { self.refs.get(id) }
}

// ══════════════════════════════════════════════════════════════════════════════
// Imperative Handle
// ══════════════════════════════════════════════════════════════════════════════

/// Methods exposed via useImperativeHandle.
#[derive(Debug, Clone)]
pub struct ImperativeHandle {
    pub component_id: String,
    pub methods: HashMap<String, String>,  // method_name -> description
}

impl ImperativeHandle {
    pub fn new(component_id: &str) -> Self {
        ImperativeHandle { component_id: component_id.into(), methods: HashMap::new() }
    }

    pub fn expose(&mut self, method: &str, description: &str) {
        self.methods.insert(method.into(), description.into());
    }

    pub fn has_method(&self, name: &str) -> bool { self.methods.contains_key(name) }

    pub fn method_count(&self) -> usize { self.methods.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Profiler
// ══════════════════════════════════════════════════════════════════════════════

/// Render timing for a profiled subtree.
#[derive(Debug, Clone)]
pub struct ProfilerEntry {
    pub component_id: String,
    pub phase: RenderPhase,
    pub actual_duration_ms: f64,
    pub base_duration_ms: f64,
    pub start_time_ms: f64,
    pub commit_time_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    Mount,
    Update,
    NestedUpdate,
}

/// Profiler wrapping a subtree to measure render performance.
#[derive(Debug)]
pub struct Profiler {
    pub id: String,
    pub entries: Vec<ProfilerEntry>,
    pub enabled: bool,
}

impl Profiler {
    pub fn new(id: &str) -> Self {
        Profiler { id: id.into(), entries: Vec::new(), enabled: true }
    }

    pub fn record(&mut self, entry: ProfilerEntry) {
        if self.enabled { self.entries.push(entry); }
    }

    pub fn average_render_ms(&self) -> f64 {
        if self.entries.is_empty() { return 0.0; }
        let sum: f64 = self.entries.iter().map(|e| e.actual_duration_ms).sum();
        sum / self.entries.len() as f64
    }

    pub fn max_render_ms(&self) -> f64 {
        self.entries.iter().map(|e| e.actual_duration_ms).fold(0.0, f64::max)
    }

    pub fn mount_count(&self) -> usize {
        self.entries.iter().filter(|e| e.phase == RenderPhase::Mount).count()
    }

    pub fn update_count(&self) -> usize {
        self.entries.iter().filter(|e| e.phase == RenderPhase::Update).count()
    }

    pub fn clear(&mut self) { self.entries.clear(); }
}

// ══════════════════════════════════════════════════════════════════════════════
// Strict Mode
// ══════════════════════════════════════════════════════════════════════════════

/// Strict mode double-invocation tracker.
#[derive(Debug)]
pub struct StrictMode {
    pub enabled: bool,
    pub double_render_log: Vec<String>,
    pub side_effect_warnings: Vec<String>,
}

impl StrictMode {
    pub fn new(enabled: bool) -> Self {
        StrictMode { enabled, double_render_log: Vec::new(), side_effect_warnings: Vec::new() }
    }

    /// Simulate double-mount to detect side effects.
    pub fn check_mount(&mut self, component_id: &str) {
        if !self.enabled { return; }
        self.double_render_log.push(format!("double-mount: {}", component_id));
    }

    /// Record a side-effect warning.
    pub fn warn_side_effect(&mut self, component_id: &str, msg: &str) {
        if !self.enabled { return; }
        self.side_effect_warnings.push(format!("{}: {}", component_id, msg));
    }

    pub fn warning_count(&self) -> usize { self.side_effect_warnings.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_create_attach() {
        let mut reg = RefRegistry::new();
        reg.create_ref("input-1");
        reg.attach("input-1", RefValue::DomNode("dom-42".into()));
        assert!(reg.get("input-1").unwrap().is_attached());
    }

    #[test]
    fn ref_forward() {
        let mut reg = RefRegistry::new();
        reg.create_ref("my-ref");
        reg.forward("Parent", "FancyInput", "my-ref");
        assert_eq!(reg.forwards.len(), 1);
        assert_eq!(reg.forwards[0].target_component, "FancyInput");
    }

    #[test]
    fn imperative_handle() {
        let mut handle = ImperativeHandle::new("VideoPlayer");
        handle.expose("play", "Start playback");
        handle.expose("pause", "Pause playback");
        handle.expose("seek", "Seek to timestamp");
        assert_eq!(handle.method_count(), 3);
        assert!(handle.has_method("play"));
    }

    #[test]
    fn profiler_timing() {
        let mut p = Profiler::new("App");
        p.record(ProfilerEntry {
            component_id: "App".into(), phase: RenderPhase::Mount,
            actual_duration_ms: 5.0, base_duration_ms: 3.0,
            start_time_ms: 0.0, commit_time_ms: 5.0,
        });
        p.record(ProfilerEntry {
            component_id: "App".into(), phase: RenderPhase::Update,
            actual_duration_ms: 2.0, base_duration_ms: 3.0,
            start_time_ms: 100.0, commit_time_ms: 102.0,
        });
        assert_eq!(p.mount_count(), 1);
        assert_eq!(p.update_count(), 1);
        assert!((p.average_render_ms() - 3.5).abs() < 0.01);
        assert!((p.max_render_ms() - 5.0).abs() < 0.01);
    }

    #[test]
    fn strict_mode_double_mount() {
        let mut sm = StrictMode::new(true);
        sm.check_mount("Counter");
        sm.check_mount("Timer");
        assert_eq!(sm.double_render_log.len(), 2);
    }

    #[test]
    fn strict_mode_side_effect() {
        let mut sm = StrictMode::new(true);
        sm.warn_side_effect("Timer", "fetch in render");
        assert_eq!(sm.warning_count(), 1);
    }

    #[test]
    fn strict_mode_disabled() {
        let mut sm = StrictMode::new(false);
        sm.check_mount("X");
        sm.warn_side_effect("X", "bad");
        assert_eq!(sm.double_render_log.len(), 0);
        assert_eq!(sm.warning_count(), 0);
    }
}
