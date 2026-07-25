//! VM builtins for **native UI core** — [`super::KillerUiEngine`] without embedding a browser.
//!
//! - **`ui_core_version`** — engine schema version (integer + string).
//! - **`ui_headless_tick`** — run the integrated demo (patch + graph + workspace) and return cooked floats.
//! - **`ui_headless_snapshot_json`** — same tick as string **JSON** (web / HTTP panels; no extra deps).
//! - **`ui_health`** — same JSON string as HTTP `GET /health` (Tier 3 VM parity).
//! - **`ui_help`** — short builtin list (Tier 5 DX).
//! - **`ui_native_window`** — Phase B stub: prints cook summary to stderr (swap for `eframe` when enabled).

use std::collections::HashMap;
use std::cell::RefCell;

use crate::error::VmError;
use crate::value::Value;

use super::http_panel;
use super::runtime_native;
use super::snapshot;
use super::KillerUiEngine;

use super::reactive::ReactiveStore;
use super::events::{EventDispatcher, EventKind, EventData, EventPhase};
use super::style::{Style, StyleStore, Theme, Color, Unit};
use super::component::{ComponentDef, ComponentRegistry, ComponentTree, PropDef, PropValue};
use super::layout::LayoutNode;
use super::router::Router;
use super::vdom::VNode;
use super::animation::{AnimationController, Easing};
use super::devtools::PerfProfiler;

thread_local! {
    static REACTIVE: RefCell<ReactiveStore> = RefCell::new(ReactiveStore::new());
    static EVENTS: RefCell<EventDispatcher> = RefCell::new(EventDispatcher::new());
    static STYLES: RefCell<StyleStore> = RefCell::new(StyleStore::new(Theme::light()));
    static COMPONENTS_REG: RefCell<ComponentRegistry> = RefCell::new(ComponentRegistry::new());
    static COMPONENTS_TREE: RefCell<ComponentTree> = RefCell::new(ComponentTree::new());
    static ROUTER: RefCell<Router> = RefCell::new(Router::new());
    static ANIMATION: RefCell<AnimationController> = RefCell::new(AnimationController::new());
    static PROFILER: RefCell<PerfProfiler> = RefCell::new(PerfProfiler::new(120));
}

fn val_str(v: &Value) -> String {
    format!("{}", v)
}

/// Returns `killer_ui` engine version as a dict: `{ "major": N, "label": "killer_ui/N" }`.
pub fn builtin_ui_core_version(_args: &[Value]) -> Result<Value, VmError> {
    let mut d = HashMap::new();
    d.insert(
        "major".to_string(),
        Value::Number(super::KILLER_UI_ENGINE_VERSION as f64),
    );
    d.insert(
        "label".to_string(),
        Value::Str(format!("killer_ui/{}", super::KILLER_UI_ENGINE_VERSION)),
    );
    Ok(Value::Dict(Box::new(d)))
}

/// One headless frame from the canonical demo engine (`example_parallel`).
pub fn builtin_ui_headless_tick(_args: &[Value]) -> Result<Value, VmError> {
    let engine = KillerUiEngine::example_parallel();
    let frame = engine.tick_headless();
    let mut cooked = HashMap::new();
    for (k, v) in frame.cooked_floats {
        cooked.insert(k, Value::Number(v));
    }
    let mut out = HashMap::new();
    out.insert("cooked".to_string(), Value::Dict(Box::new(cooked)));
    out.insert(
        "events_pending".to_string(),
        Value::Number(frame.pending_events.len() as f64),
    );
    Ok(Value::Dict(Box::new(out)))
}

/// Same as [`builtin_ui_headless_tick`] but returns one **JSON string** (`killer_ui_engine_version`, `cooked`, `events`, `events_pending`).
pub fn builtin_ui_headless_snapshot_json(_args: &[Value]) -> Result<Value, VmError> {
    let engine = KillerUiEngine::example_parallel();
    let frame = engine.tick_headless();
    let json = snapshot::headless_frame_json(engine.version, &frame);
    Ok(Value::Str(json))
}

const UI_HELP: &str = r#"killer_ui VM builtins (run with killer-native):
  ui_core_version()  ui_headless_tick()  ui_headless_snapshot_json()
  ui_health()  ui_native_window()  ui_help()

Tier 4 line sugar: ui version | ui snapshot | ui headless_tick | ui tick | ui health | ui native_window | ui help

Tier 5 CLI: killer_ui serve|demo|window | killer_ui_serve
Tier 3 HTTP: GET /health | /killer-ui/headless.json | /killer-ui/version.json  (killer_ui serve [HOST] PORT)
Tier 2 window: stub by default; add optional eframe + killer-ui-egui feature (see KILLER_UI_ENGINE.md)
"#;

/// Short reference string for tools and REPL (Tier 5).
pub fn builtin_ui_help(_args: &[Value]) -> Result<Value, VmError> {
    Ok(Value::Str(UI_HELP.to_string()))
}

/// Same payload as Tier 3 HTTP `GET /health` (JSON string).
pub fn builtin_ui_health(_args: &[Value]) -> Result<Value, VmError> {
    Ok(Value::Str(http_panel::killer_ui_health_json().to_string()))
}

/// Phase B: native window entry point — opens a real Win32 window with rendered widgets.
/// Optional first argument overrides the summary string; otherwise uses demo graph cook line.
pub fn builtin_ui_native_window(args: &[Value]) -> Result<Value, VmError> {
    let summary = if let Some(v) = args.first() {
        val_str(v)
    } else {
        let e = KillerUiEngine::example_parallel();
        runtime_native::cook_summary(&e.graph)
    };
    runtime_native::run_demo_window(summary).map_err(|e| {
        VmError::runtime_error(format!("ui_native_window: {}", e))
    })?;
    Ok(Value::Str("ok".to_string()))
}

/// Render the full widget gallery in a native window.
pub fn builtin_ui_render_gallery(_args: &[Value]) -> Result<Value, VmError> {
    let patch = super::render_widgets::demo_all_widgets();
    runtime_native::run_window_with_patch(Some(&patch), "Widget Gallery").map_err(|e| {
        VmError::runtime_error(format!("ui_render_gallery: {}", e))
    })?;
    Ok(Value::Str("ok".to_string()))
}

/// Headless render to BMP bytes (for screenshots / testing): `ui_render_screenshot(width, height)` → dict.
pub fn builtin_ui_render_screenshot(args: &[Value]) -> Result<Value, VmError> {
    let w = match args.first() { Some(Value::Number(n)) => *n as u32, _ => 800 };
    let h = match args.get(1) { Some(Value::Number(n)) => *n as u32, _ => 600 };
    let patch = super::render_widgets::demo_all_widgets();
    let theme = super::render_widgets::RenderTheme::light();
    let mut fb = super::framebuffer::Framebuffer::new(w, h);
    super::render_widgets::render_patch(&mut fb, &patch, &theme);

    let mut d = std::collections::HashMap::new();
    d.insert("width".to_string(), Value::Number(w as f64));
    d.insert("height".to_string(), Value::Number(h as f64));
    d.insert("pixels".to_string(), Value::Number(fb.pixels.len() as f64));
    // Count non-zero pixels as a simple "did render" check
    let drawn = fb.pixels.chunks_exact(4).filter(|c| c[0] != 0 || c[1] != 0 || c[2] != 0).count();
    d.insert("drawn_pixels".to_string(), Value::Number(drawn as f64));
    d.insert("status".to_string(), Value::Str("rendered".to_string()));
    Ok(Value::Dict(Box::new(d)))
}

// ========================================================================
// Reactive state builtins
// ========================================================================

use super::reactive::RxValue;

fn value_to_rx(v: &Value) -> RxValue {
    match v {
        Value::Number(n) => RxValue::Number(*n),
        Value::Str(s) => RxValue::Str(s.clone()),
        Value::Bool(b) => RxValue::Bool(*b),
        _ => RxValue::Null,
    }
}

fn rx_to_value(rx: &RxValue) -> Value {
    match rx {
        RxValue::Number(f) => Value::Number(*f),
        RxValue::Str(s) => Value::Str(s.clone()),
        RxValue::Bool(b) => Value::Bool(*b),
        RxValue::Null => Value::Null,
        RxValue::Array(a) => Value::Number(a.len() as f64),
        RxValue::Dict(d) => Value::Number(d.len() as f64),
    }
}

/// Create a signal: `ui_signal_create(initial_value)` → signal_id (number)
pub fn builtin_ui_signal_create(args: &[Value]) -> Result<Value, VmError> {
    let init = args.first().cloned().unwrap_or(Value::Null);
    let rx = value_to_rx(&init);
    let id = REACTIVE.with(|r| r.borrow_mut().create_signal(rx));
    Ok(Value::Number(id as f64))
}

/// Get a signal value: `ui_signal_get(signal_id)` → value
pub fn builtin_ui_signal_get(args: &[Value]) -> Result<Value, VmError> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err(VmError::runtime_error("ui_signal_get: need signal_id")),
    };
    REACTIVE.with(|r| {
        let store = r.borrow();
        match store.get(id) {
            Some(v) => Ok(rx_to_value(v)),
            None => Ok(Value::Null),
        }
    })
}

/// Set a signal value: `ui_signal_set(signal_id, new_value)` → "ok"
pub fn builtin_ui_signal_set(args: &[Value]) -> Result<Value, VmError> {
    let id = match args.first() {
        Some(Value::Number(n)) => *n as u64,
        _ => return Err(VmError::runtime_error("ui_signal_set: need signal_id")),
    };
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let rx = value_to_rx(&val);
    REACTIVE.with(|r| {
        let _ = r.borrow_mut().set(id, rx);
    });
    Ok(Value::Str("ok".into()))
}

/// Create computed signal: `ui_computed("sum", dep_id1, dep_id2, ...)` → signal_id
pub fn builtin_ui_computed(args: &[Value]) -> Result<Value, VmError> {
    let expr_str = args.first().map(val_str).unwrap_or_else(|| "sum".into());
    let deps: Vec<u64> = args[1..]
        .iter()
        .filter_map(|v| match v {
            Value::Number(n) => Some(*n as u64),
            _ => None,
        })
        .collect();
    let expr = match expr_str.as_str() {
        "sum" => super::reactive::ComputedExpr::Sum,
        "product" => super::reactive::ComputedExpr::Product,
        "concat" => super::reactive::ComputedExpr::Concat,
        "min" => super::reactive::ComputedExpr::Min,
        "max" => super::reactive::ComputedExpr::Max,
        "avg" | "average" => super::reactive::ComputedExpr::Average,
        "negate" => super::reactive::ComputedExpr::Negate,
        "count" => super::reactive::ComputedExpr::Count,
        "identity" => super::reactive::ComputedExpr::Identity,
        "to_bool" => super::reactive::ComputedExpr::ToBool,
        _ => super::reactive::ComputedExpr::Sum,
    };
    let result = REACTIVE.with(|r| r.borrow_mut().create_computed(deps, expr));
    match result {
        Ok(id) => Ok(Value::Number(id as f64)),
        Err(e) => Err(VmError::runtime_error(format!("ui_computed: {:?}", e))),
    }
}

/// Register an effect: `ui_effect(action_name, dep_id1, dep_id2, ...)` → effect_id
pub fn builtin_ui_effect(args: &[Value]) -> Result<Value, VmError> {
    let action = args.first().map(val_str).unwrap_or_default();
    let deps: Vec<u64> = args[1..]
        .iter()
        .filter_map(|v| match v {
            Value::Number(n) => Some(*n as u64),
            _ => None,
        })
        .collect();
    let result = REACTIVE.with(|r| r.borrow_mut().create_effect(deps, action));
    match result {
        Ok(id) => Ok(Value::Number(id as f64)),
        Err(e) => Err(VmError::runtime_error(format!("ui_effect: {:?}", e))),
    }
}

/// Batch reactive updates: `ui_batch("begin")` or `ui_batch("end")` → "ok"
pub fn builtin_ui_batch(args: &[Value]) -> Result<Value, VmError> {
    let cmd = args.first().map(val_str).unwrap_or_default();
    REACTIVE.with(|r| {
        let mut store = r.borrow_mut();
        match cmd.as_str() {
            "begin" => store.batch_begin(),
            "end" => store.batch_end(),
            _ => {}
        }
    });
    Ok(Value::Str("ok".into()))
}

// ========================================================================
// Event builtins
// ========================================================================

/// Register event handler: `ui_on_event(widget_id, event_kind_str, action_name)` → handler_id
pub fn builtin_ui_on_event(args: &[Value]) -> Result<Value, VmError> {
    let widget = args.first().map(val_str).unwrap_or_default();
    let kind_str = args.get(1).map(val_str).unwrap_or_else(|| "click".into());
    let action = args.get(2).map(val_str).unwrap_or_else(|| "default".into());
    let kind = EventKind::from_str_loose(&kind_str);
    let hid = EVENTS.with(|e| {
        e.borrow_mut().on(&widget, kind, EventPhase::Bubble, action, false)
    });
    Ok(Value::Number(hid as f64))
}

/// Dispatch event: `ui_dispatch_event(widget_id, event_kind_str)` → number of handlers fired
pub fn builtin_ui_dispatch_event(args: &[Value]) -> Result<Value, VmError> {
    let widget = args.first().map(val_str).unwrap_or_default();
    let kind_str = args.get(1).map(val_str).unwrap_or_else(|| "click".into());
    let kind = EventKind::from_str_loose(&kind_str);
    let mut event = EventData::new(kind, widget);
    let fired = EVENTS.with(|e| {
        e.borrow_mut().dispatch(&mut event)
    });
    Ok(Value::Number(fired.len() as f64))
}

// ========================================================================
// Style / Theme builtins
// ========================================================================

/// Set theme: `ui_theme("dark")` or `ui_theme("light")` → theme JSON
pub fn builtin_ui_theme(args: &[Value]) -> Result<Value, VmError> {
    let name = args.first().map(val_str).unwrap_or_else(|| "light".into());
    let theme = match name.as_str() {
        "dark" => Theme::dark(),
        _ => Theme::light(),
    };
    let json = format!(
        "{{\"name\":\"{}\",\"primary\":\"{}\",\"bg\":\"{}\",\"on_background\":\"{}\"}}",
        theme.name,
        theme.primary.to_hex(),
        theme.background.to_hex(),
        theme.on_background.to_hex(),
    );
    Ok(Value::Str(json))
}

/// Set style on widget: `ui_style_set(widget_id, property, value)` → "ok"
pub fn builtin_ui_style_set(args: &[Value]) -> Result<Value, VmError> {
    let widget_id = args.first().map(val_str).unwrap_or_default();
    let prop = args.get(1).map(val_str).unwrap_or_default();
    let val = args.get(2).map(val_str).unwrap_or_default();
    let mut style = Style::default();
    match prop.as_str() {
        "width" => style.width = parse_unit(&val),
        "height" => style.height = parse_unit(&val),
        "color" => if let Some(c) = Color::from_hex(&val) { style.color = c; },
        "background" | "bg" => if let Some(c) = Color::from_hex(&val) { style.background = c; },
        "font_size" => style.font.size = val.parse::<f64>().unwrap_or(16.0),
        "opacity" => style.opacity = val.parse::<f64>().unwrap_or(1.0),
        _ => {}
    }
    STYLES.with(|s| s.borrow_mut().set_id_style(&widget_id, style));
    Ok(Value::Str("ok".into()))
}

fn parse_unit(s: &str) -> Unit {
    if s.ends_with("px") {
        Unit::Px(s.trim_end_matches("px").parse().unwrap_or(0.0))
    } else if s.ends_with('%') {
        Unit::Percent(s.trim_end_matches('%').parse().unwrap_or(0.0))
    } else if s.ends_with("em") {
        Unit::Em(s.trim_end_matches("em").parse().unwrap_or(1.0))
    } else if s.ends_with("rem") {
        Unit::Rem(s.trim_end_matches("rem").parse().unwrap_or(1.0))
    } else if s == "auto" {
        Unit::Auto
    } else {
        Unit::Px(s.parse().unwrap_or(0.0))
    }
}

// ========================================================================
// Component builtins
// ========================================================================

/// Register a component: `ui_component_register(name, prop1, prop2, ...)` → "ok"
pub fn builtin_ui_component_register(args: &[Value]) -> Result<Value, VmError> {
    let name = args.first().map(val_str).unwrap_or_default();
    let prop_defs: Vec<PropDef> = args[1..]
        .iter()
        .map(|v| {
            let s = val_str(v);
            PropDef {
                name: s,
                required: false,
                default_value: Some(PropValue::Null),
            }
        })
        .collect();
    let def = ComponentDef {
        name: name.clone(),
        prop_defs,
        on_mount: None,
        on_update: None,
        on_unmount: None,
        accepts_children: true,
        slots: vec!["default".into()],
    };
    COMPONENTS_REG.with(|r| r.borrow_mut().register(def));
    Ok(Value::Str("ok".into()))
}

/// Create a component instance: `ui_component_create(component_name, parent_id_or_null, prop_name1, val1, ...)` → instance_id
pub fn builtin_ui_component_create(args: &[Value]) -> Result<Value, VmError> {
    let comp_name = args.first().map(val_str).unwrap_or_default();
    let parent: Option<u64> = match args.get(1) {
        Some(Value::Number(n)) => Some(*n as u64),
        _ => None,
    };
    let mut props = HashMap::new();
    let mut i = 2;
    while i + 1 < args.len() {
        let k = val_str(&args[i]);
        let v = match &args[i + 1] {
            Value::Number(n) => PropValue::Number(*n),
            Value::Str(s) => PropValue::Str(s.clone()),
            Value::Bool(b) => PropValue::Bool(*b),
            _ => PropValue::Null,
        };
        props.insert(k, v);
        i += 2;
    }
    // Need both registry and tree — borrow both within one closure scope
    let result = COMPONENTS_REG.with(|reg| {
        COMPONENTS_TREE.with(|tree| {
            let registry = reg.borrow();
            let mut t = tree.borrow_mut();
            t.create(&registry, &comp_name, props, parent)
        })
    });
    match result {
        Ok(id) => Ok(Value::Number(id as f64)),
        Err(e) => Err(VmError::runtime_error(format!("ui_component_create: {:?}", e))),
    }
}

// ========================================================================
// Layout builtins
// ========================================================================

/// Compute flexbox layout: `ui_layout_compute(width, height, children_count)` → JSON
pub fn builtin_ui_layout_compute(args: &[Value]) -> Result<Value, VmError> {
    let w = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => 800.0,
    };
    let h = match args.get(1) {
        Some(Value::Number(n)) => *n,
        _ => 600.0,
    };
    let n_children = match args.get(2) {
        Some(Value::Number(n)) => *n as usize,
        _ => 0,
    };
    let mut root = LayoutNode::new("root").with_size(w, h);
    for i in 0..n_children {
        root = root.add_child(LayoutNode::new(&format!("child_{}", i)));
    }
    super::layout::compute_layout(&mut root, w, h);
    let json = super::layout::layout_to_json(&root);
    Ok(Value::Str(json))
}

// ========================================================================
// Router builtins
// ========================================================================

/// Add a route: `ui_route_add(pattern, action)` → route_id
pub fn builtin_ui_route_add(args: &[Value]) -> Result<Value, VmError> {
    let pattern = args.first().map(val_str).unwrap_or_else(|| "/".into());
    let action = args.get(1).map(val_str).unwrap_or_else(|| "default".into());
    let id = ROUTER.with(|r| r.borrow_mut().add_route(&pattern, &action));
    Ok(Value::Number(id as f64))
}

/// Navigate: `ui_navigate(path)` → JSON navigation event
pub fn builtin_ui_navigate(args: &[Value]) -> Result<Value, VmError> {
    let path = args.first().map(val_str).unwrap_or_else(|| "/".into());
    let nav = ROUTER.with(|r| {
        r.borrow_mut().navigate(&path)
    });
    let blocked = match &nav.blocked_by {
        Some(g) => format!("\"{}\"", g),
        None => "null".into(),
    };
    let mut params_json = String::from("{");
    let mut first = true;
    for (k, v) in &nav.params {
        if !first { params_json.push(','); }
        first = false;
        params_json.push_str(&format!("\"{}\":\"{}\"", k, v));
    }
    params_json.push('}');
    let json = format!(
        "{{\"to\":\"{}\",\"blocked_by\":{},\"params\":{}}}",
        nav.to, blocked, params_json,
    );
    Ok(Value::Str(json))
}

// ========================================================================
// VDOM builtins
// ========================================================================

/// Build and diff two VNode trees: `ui_vdom_diff(old_tag, new_tag)` → patch count
pub fn builtin_ui_vdom_diff(args: &[Value]) -> Result<Value, VmError> {
    let old_tag = args.first().map(val_str).unwrap_or_else(|| "div".into());
    let new_tag = args.get(1).map(val_str).unwrap_or_else(|| "div".into());
    let old = VNode::element(&old_tag);
    let new_node = VNode::element(&new_tag);
    let patches = super::vdom::diff(&old, &new_node);
    Ok(Value::Number(patches.len() as f64))
}

/// Apply patches to a vnode: `ui_vdom_patch(tag, new_text)` → JSON
pub fn builtin_ui_vdom_patch(args: &[Value]) -> Result<Value, VmError> {
    let tag = args.first().map(val_str).unwrap_or_else(|| "div".into());
    let new_text = args.get(1).map(val_str).unwrap_or_else(|| "hello".into());
    let mut old = VNode::element(&tag).with_child(VNode::text("old"));
    let new_node = VNode::element(&tag).with_child(VNode::text(&new_text));
    let patches = super::vdom::diff(&old, &new_node);
    super::vdom::apply_patches(&mut old, &patches);
    let json = super::vdom::vnode_to_json(&old);
    Ok(Value::Str(json))
}

// ========================================================================
// Animation builtins
// ========================================================================

/// Start animation: `ui_animate(widget_id, property, from, to, duration_ms)` → anim_id
pub fn builtin_ui_animate(args: &[Value]) -> Result<Value, VmError> {
    let widget = args.first().map(val_str).unwrap_or_default();
    let prop = args.get(1).map(val_str).unwrap_or_else(|| "opacity".into());
    let from = match args.get(2) { Some(Value::Number(n)) => *n, _ => 0.0 };
    let to = match args.get(3) { Some(Value::Number(n)) => *n, _ => 1.0 };
    let dur = match args.get(4) { Some(Value::Number(n)) => *n, _ => 300.0 };
    let id = ANIMATION.with(|a| {
        a.borrow_mut().animate(&widget, &prop, from, to, dur, Easing::EaseInOut)
    });
    Ok(Value::Number(id as f64))
}

/// Start keyframe animation: `ui_animate_keyframes(widget_id, property, duration_ms, v1, p1, v2, p2, ...)` → anim_id
pub fn builtin_ui_animate_keyframes(args: &[Value]) -> Result<Value, VmError> {
    let widget = args.first().map(val_str).unwrap_or_default();
    let prop = args.get(1).map(val_str).unwrap_or_else(|| "opacity".into());
    let dur = match args.get(2) { Some(Value::Number(n)) => *n, _ => 1000.0 };
    let mut keyframes = Vec::new();
    let mut i = 3;
    while i + 1 < args.len() {
        let val = match &args[i] { Value::Number(n) => *n, _ => 0.0 };
        let progress = match &args[i + 1] { Value::Number(n) => *n, _ => 0.0 };
        keyframes.push(super::animation::Keyframe {
            progress,
            value: val,
            easing: Easing::Linear,
        });
        i += 2;
    }
    let id = ANIMATION.with(|a| {
        a.borrow_mut().animate_keyframes(&widget, &prop, keyframes, dur)
    });
    Ok(Value::Number(id as f64))
}

// ========================================================================
// DevTools builtins
// ========================================================================

/// Inspect UI engine state: `ui_inspect()` → JSON snapshot
pub fn builtin_ui_inspect(_args: &[Value]) -> Result<Value, VmError> {
    let json = REACTIVE.with(|reactive| {
        COMPONENTS_TREE.with(|tree| {
            EVENTS.with(|events| {
                ANIMATION.with(|anim| {
                    ROUTER.with(|router| {
                        PROFILER.with(|prof| {
                            let snap = super::devtools::inspect(
                                &reactive.borrow(),
                                &tree.borrow(),
                                &events.borrow(),
                                &anim.borrow(),
                                &router.borrow(),
                                &prof.borrow(),
                            );
                            super::devtools::snapshot_to_json(&snap)
                        })
                    })
                })
            })
        })
    });
    Ok(Value::Str(json))
}

/// Perf snapshot: `ui_perf_snapshot()` → JSON with avg_ms, fps, etc.
pub fn builtin_ui_perf_snapshot(_args: &[Value]) -> Result<Value, VmError> {
    PROFILER.with(|p| {
        let prof = p.borrow();
        let json = format!(
            "{{\"avg_ms\":{:.2},\"min_ms\":{:.2},\"max_ms\":{:.2},\"fps\":{:.1},\"frames\":{}}}",
            prof.avg_frame_ms(), prof.min_frame_ms(), prof.max_frame_ms(),
            prof.estimated_fps(), prof.total_frames,
        );
        Ok(Value::Str(json))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_core_version_shape() {
        let v = builtin_ui_core_version(&[]).unwrap();
        let Value::Dict(d) = v else {
            panic!("expected dict");
        };
        assert_eq!(d.get("major"), Some(&Value::Number(2.0)));
        assert!(matches!(d.get("label"), Some(Value::Str(s)) if s.starts_with("killer_ui/")));
    }

    #[test]
    fn ui_headless_tick_has_sum() {
        let v = builtin_ui_headless_tick(&[]).unwrap();
        let Value::Dict(d) = v else {
            panic!("expected dict");
        };
        let cooked = d.get("cooked").unwrap();
        let Value::Dict(c) = cooked else {
            panic!("expected cooked dict");
        };
        let sum = match c.get("sum").unwrap() {
            Value::Number(n) => *n,
            _ => panic!("sum not a number"),
        };
        assert!((sum - 3.0).abs() < 1e-9);
    }

    #[test]
    fn ui_headless_snapshot_json_is_object() {
        let v = builtin_ui_headless_snapshot_json(&[]).unwrap();
        let Value::Str(s) = v else {
            panic!("expected string");
        };
        assert!(s.starts_with('{') && s.contains("\"cooked\"") && s.contains("\"sum\""));
    }

    #[test]
    fn ui_help_non_empty() {
        let v = builtin_ui_help(&[]).unwrap();
        let Value::Str(s) = v else {
            panic!("expected string");
        };
        assert!(s.contains("ui_core_version") && s.contains("killer_ui"));
    }

    #[test]
    fn ui_health_json_matches_http_panel() {
        let v = builtin_ui_health(&[]).unwrap();
        let Value::Str(s) = v else {
            panic!("expected string");
        };
        assert!(s.contains("\"ok\":true") && s.contains("killer_ui"));
        assert_eq!(s, super::http_panel::killer_ui_health_json());
    }
}
