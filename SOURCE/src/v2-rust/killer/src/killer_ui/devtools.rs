//! **Dev tools** — runtime inspector, performance profiler, and component/state viewer.
//!
//! Zero-dependency JSON output for integration with headless panels, HTTP endpoints, or
//! future graphical inspector overlays. Aggregates data from the reactive store, component
//! tree, event dispatcher, animation controller, and router.

use std::collections::HashMap;
use std::time::Instant;

use super::animation::AnimationController;
use super::component::ComponentTree;
use super::events::EventDispatcher;
use super::reactive::ReactiveStore;
use super::router::Router;

// ── Performance profiler ─────────────────────────────────────────────────────

/// Tracks frame times and per-system durations.
#[derive(Debug)]
pub struct PerfProfiler {
    /// Frame times in ms (ring buffer).
    frame_times: Vec<f64>,
    max_frames: usize,
    /// Per-system timing for the last frame.
    pub system_times: HashMap<String, f64>,
    /// Total frames rendered.
    pub total_frames: u64,
    /// Timer reference.
    last_frame: Option<Instant>,
}

impl PerfProfiler {
    pub fn new(max_frames: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(max_frames),
            max_frames,
            system_times: HashMap::new(),
            total_frames: 0,
            last_frame: None,
        }
    }

    /// Call at the start of each frame. Records dt from last frame.
    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_frame {
            let dt = now.duration_since(last).as_secs_f64() * 1000.0;
            if self.frame_times.len() >= self.max_frames {
                self.frame_times.remove(0);
            }
            self.frame_times.push(dt);
        }
        self.last_frame = Some(now);
        self.total_frames += 1;
        self.system_times.clear();
    }

    /// Record a system's execution time for this frame.
    pub fn record_system(&mut self, name: &str, duration_ms: f64) {
        self.system_times.insert(name.to_string(), duration_ms);
    }

    /// Average frame time in ms (over the buffer).
    pub fn avg_frame_ms(&self) -> f64 {
        if self.frame_times.is_empty() { return 0.0; }
        self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64
    }

    /// Min frame time in the buffer.
    pub fn min_frame_ms(&self) -> f64 {
        self.frame_times.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    /// Max frame time in the buffer.
    pub fn max_frame_ms(&self) -> f64 {
        self.frame_times.iter().cloned().fold(0.0_f64, f64::max)
    }

    /// Estimated FPS from average frame time.
    pub fn estimated_fps(&self) -> f64 {
        let avg = self.avg_frame_ms();
        if avg > 0.0 { 1000.0 / avg } else { 0.0 }
    }

    /// Produce JSON summary.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"total_frames\": {},\n", self.total_frames));
        s.push_str(&format!("  \"avg_frame_ms\": {:.2},\n", self.avg_frame_ms()));
        s.push_str(&format!("  \"min_frame_ms\": {:.2},\n", self.min_frame_ms()));
        s.push_str(&format!("  \"max_frame_ms\": {:.2},\n", self.max_frame_ms()));
        s.push_str(&format!("  \"estimated_fps\": {:.1},\n", self.estimated_fps()));
        s.push_str("  \"system_times\": {\n");
        let mut systems: Vec<_> = self.system_times.iter().collect();
        systems.sort_by(|a, b| a.0.cmp(b.0));
        for (i, (name, dur)) in systems.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!("    \"{}\": {:.3}", name, dur));
        }
        s.push_str("\n  }\n}\n");
        s
    }
}

impl Default for PerfProfiler {
    fn default() -> Self { Self::new(120) }
}

// ── Auto-timing wrapper ──────────────────────────────────────────────────────

/// Auto-timer: wraps a closure and records its execution time in the profiler.
/// Usage: `auto_time(&mut profiler, "layout", || { compute_layout(); });`
pub fn auto_time<F, R>(profiler: &mut PerfProfiler, system_name: &str, f: F) -> R
where F: FnOnce() -> R {
    let start = Instant::now();
    let result = f();
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    profiler.record_system(system_name, elapsed_ms);
    result
}

/// Frame profiler: auto-instruments an entire frame with multiple systems.
/// Call `begin()` at frame start, then `time()` each system, then `end()` to finalize.
#[derive(Debug)]
pub struct FrameProfiler<'a> {
    profiler: &'a mut PerfProfiler,
}

impl<'a> FrameProfiler<'a> {
    /// Begin a new profiled frame.
    pub fn begin(profiler: &'a mut PerfProfiler) -> Self {
        profiler.begin_frame();
        Self { profiler }
    }

    /// Time a named system within the current frame.
    pub fn time<F, R>(&mut self, system_name: &str, f: F) -> R
    where F: FnOnce() -> R {
        auto_time(self.profiler, system_name, f)
    }

    /// Finalize the frame (no-op, but makes intent clear and returns profiler).
    pub fn end(self) -> &'a mut PerfProfiler {
        self.profiler
    }
}

// ── Inspector snapshot ───────────────────────────────────────────────────────

/// Full inspector snapshot of the UI engine state.
#[derive(Debug)]
pub struct InspectorSnapshot {
    pub signal_count: usize,
    pub effect_count: usize,
    pub component_count: usize,
    pub handler_count: usize,
    pub animation_count: usize,
    pub route_count: usize,
    pub current_route: String,
    pub perf: PerfSnapshot,
    pub reactive_json: String,
    pub component_json: String,
    pub event_log_json: String,
    pub router_json: String,
}

#[derive(Debug, Clone)]
pub struct PerfSnapshot {
    pub total_frames: u64,
    pub avg_frame_ms: f64,
    pub estimated_fps: f64,
}

/// Gather a full snapshot from all subsystems.
pub fn inspect(
    reactive: &ReactiveStore,
    components: &ComponentTree,
    events: &EventDispatcher,
    animations: &AnimationController,
    router: &Router,
    profiler: &PerfProfiler,
) -> InspectorSnapshot {
    InspectorSnapshot {
        signal_count: reactive.signal_count(),
        effect_count: reactive.fired_effects().len(),
        component_count: components.instance_count(),
        handler_count: events.handler_count(),
        animation_count: animations.active_count(),
        route_count: router.route_count(),
        current_route: router.current_path.clone(),
        perf: PerfSnapshot {
            total_frames: profiler.total_frames,
            avg_frame_ms: profiler.avg_frame_ms(),
            estimated_fps: profiler.estimated_fps(),
        },
        reactive_json: reactive.debug_json(),
        component_json: components.debug_json(),
        event_log_json: events.log_json(),
        router_json: router.routes_json(),
    }
}

/// Convert an InspectorSnapshot to JSON.
pub fn snapshot_to_json(snap: &InspectorSnapshot) -> String {
    let mut s = String::from("{\n");
    s.push_str(&format!("  \"signal_count\": {},\n", snap.signal_count));
    s.push_str(&format!("  \"effect_count\": {},\n", snap.effect_count));
    s.push_str(&format!("  \"component_count\": {},\n", snap.component_count));
    s.push_str(&format!("  \"handler_count\": {},\n", snap.handler_count));
    s.push_str(&format!("  \"animation_count\": {},\n", snap.animation_count));
    s.push_str(&format!("  \"route_count\": {},\n", snap.route_count));
    s.push_str(&format!("  \"current_route\": \"{}\",\n", snap.current_route));
    s.push_str(&format!("  \"perf\": {{\n    \"total_frames\": {},\n    \"avg_frame_ms\": {:.2},\n    \"estimated_fps\": {:.1}\n  }},\n",
        snap.perf.total_frames, snap.perf.avg_frame_ms, snap.perf.estimated_fps));
    s.push_str(&format!("  \"reactive\": {},\n", snap.reactive_json));
    s.push_str(&format!("  \"components\": {},\n", snap.component_json));
    s.push_str(&format!("  \"event_log\": {},\n", snap.event_log_json));
    s.push_str(&format!("  \"routes\": {}\n", snap.router_json));
    s.push_str("}\n");
    s
}

// ── Render tree debug ────────────────────────────────────────────────────────

/// Produce a human-readable tree view of the component hierarchy.
pub fn component_tree_ascii(tree: &ComponentTree) -> String {
    let mut out = String::from("Component Tree:\n");
    for &root_id in tree.root_ids() {
        tree_ascii_inner(tree, root_id, &mut out, 0);
    }
    out
}

fn tree_ascii_inner(tree: &ComponentTree, id: u64, out: &mut String, depth: usize) {
    let indent = "  ".repeat(depth);
    if let Some(inst) = tree.get(id) {
        let ref_str = inst.ref_tag.as_deref().unwrap_or("");
        out.push_str(&format!("{}├── {} (id={}{}) [{}]\n",
            indent, inst.def_name, inst.id,
            if ref_str.is_empty() { String::new() } else { format!(", ref=\"{}\"", ref_str) },
            if inst.mounted { "mounted" } else { "unmounted" }
        ));
        for &child_id in &inst.children_ids {
            tree_ascii_inner(tree, child_id, out, depth + 1);
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_avg_frame_time() {
        let mut prof = PerfProfiler::new(10);
        prof.frame_times = vec![16.0, 17.0, 15.0]; // fake data
        prof.total_frames = 3;
        assert!((prof.avg_frame_ms() - 16.0).abs() < 0.1);
        assert!((prof.estimated_fps() - 62.5).abs() < 1.0);
    }

    #[test]
    fn profiler_json_output() {
        let prof = PerfProfiler::new(10);
        let json = prof.to_json();
        assert!(json.contains("\"total_frames\""));
        assert!(json.contains("\"estimated_fps\""));
    }

    #[test]
    fn full_inspect_snapshot() {
        let reactive = ReactiveStore::new();
        let components = ComponentTree::new();
        let events = EventDispatcher::new();
        let animations = AnimationController::new();
        let router = Router::new();
        let profiler = PerfProfiler::new(10);

        let snap = inspect(&reactive, &components, &events, &animations, &router, &profiler);
        assert_eq!(snap.signal_count, 0);
        assert_eq!(snap.component_count, 0);
        let json = snapshot_to_json(&snap);
        assert!(json.contains("\"signal_count\": 0"));
    }

    #[test]
    fn auto_time_records_system() {
        let mut prof = PerfProfiler::new(10);
        prof.begin_frame();
        let result = auto_time(&mut prof, "test_system", || { 42 });
        assert_eq!(result, 42);
        assert!(prof.system_times.contains_key("test_system"));
        assert!(prof.system_times["test_system"] >= 0.0);
    }

    #[test]
    fn frame_profiler_workflow() {
        let mut prof = PerfProfiler::new(10);
        {
            let mut fp = FrameProfiler::begin(&mut prof);
            let _ = fp.time("layout", || { 1 + 1 });
            let _ = fp.time("render", || { 2 + 2 });
            fp.end();
        }
        assert!(prof.system_times.contains_key("layout"));
        assert!(prof.system_times.contains_key("render"));
        assert_eq!(prof.total_frames, 1);
    }
}
