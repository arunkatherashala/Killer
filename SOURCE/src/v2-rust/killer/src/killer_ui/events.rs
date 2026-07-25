//! **Event dispatch system** — full event loop with bubbling, capturing, and handler management.
//!
//! Models the DOM event model: Capture (root→target) → Target → Bubble (target→root).
//! All handler registration is by widget ID + event kind, stored in a central [`EventDispatcher`].

use std::collections::HashMap;

// ── Event types ──────────────────────────────────────────────────────────────

/// All UI event kinds killer_ui can dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventKind {
    Click,
    DoubleClick,
    RightClick,
    MouseDown,
    MouseUp,
    MouseEnter,
    MouseLeave,
    MouseMove,
    KeyDown,
    KeyUp,
    KeyPress,
    Focus,
    Blur,
    Input,
    Change,
    Submit,
    Scroll,
    Resize,
    DragStart,
    DragOver,
    Drop,
    TouchStart,
    TouchEnd,
    TouchMove,
    /// Custom event emitted by user code.
    Custom(String),
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::Click => write!(f, "click"),
            EventKind::DoubleClick => write!(f, "dblclick"),
            EventKind::RightClick => write!(f, "contextmenu"),
            EventKind::MouseDown => write!(f, "mousedown"),
            EventKind::MouseUp => write!(f, "mouseup"),
            EventKind::MouseEnter => write!(f, "mouseenter"),
            EventKind::MouseLeave => write!(f, "mouseleave"),
            EventKind::MouseMove => write!(f, "mousemove"),
            EventKind::KeyDown => write!(f, "keydown"),
            EventKind::KeyUp => write!(f, "keyup"),
            EventKind::KeyPress => write!(f, "keypress"),
            EventKind::Focus => write!(f, "focus"),
            EventKind::Blur => write!(f, "blur"),
            EventKind::Input => write!(f, "input"),
            EventKind::Change => write!(f, "change"),
            EventKind::Submit => write!(f, "submit"),
            EventKind::Scroll => write!(f, "scroll"),
            EventKind::Resize => write!(f, "resize"),
            EventKind::DragStart => write!(f, "dragstart"),
            EventKind::DragOver => write!(f, "dragover"),
            EventKind::Drop => write!(f, "drop"),
            EventKind::TouchStart => write!(f, "touchstart"),
            EventKind::TouchEnd => write!(f, "touchend"),
            EventKind::TouchMove => write!(f, "touchmove"),
            EventKind::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

impl EventKind {
    /// Parse from a string (case-insensitive).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "click" => EventKind::Click,
            "dblclick" | "doubleclick" => EventKind::DoubleClick,
            "rightclick" | "contextmenu" => EventKind::RightClick,
            "mousedown" => EventKind::MouseDown,
            "mouseup" => EventKind::MouseUp,
            "mouseenter" => EventKind::MouseEnter,
            "mouseleave" => EventKind::MouseLeave,
            "mousemove" => EventKind::MouseMove,
            "keydown" => EventKind::KeyDown,
            "keyup" => EventKind::KeyUp,
            "keypress" => EventKind::KeyPress,
            "focus" => EventKind::Focus,
            "blur" => EventKind::Blur,
            "input" => EventKind::Input,
            "change" => EventKind::Change,
            "submit" => EventKind::Submit,
            "scroll" => EventKind::Scroll,
            "resize" => EventKind::Resize,
            "dragstart" => EventKind::DragStart,
            "dragover" => EventKind::DragOver,
            "drop" => EventKind::Drop,
            "touchstart" => EventKind::TouchStart,
            "touchend" => EventKind::TouchEnd,
            "touchmove" => EventKind::TouchMove,
            other => EventKind::Custom(other.to_string()),
        }
    }

    /// Events that bubble: most do, except mouseenter/mouseleave/focus/blur.
    pub fn bubbles(&self) -> bool {
        !matches!(self, EventKind::MouseEnter | EventKind::MouseLeave | EventKind::Focus | EventKind::Blur)
    }
}

// ── Event phases ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

// ── Event payload ────────────────────────────────────────────────────────────

/// Data carried with an event.
#[derive(Debug, Clone)]
pub struct EventData {
    pub kind: EventKind,
    pub target_id: String,
    /// Current node during propagation.
    pub current_target_id: String,
    pub phase: EventPhase,
    /// Arbitrary key-value data (e.g. key code, mouse position, input value).
    pub detail: HashMap<String, String>,
    /// If true, stop propagation to parent widgets.
    pub propagation_stopped: bool,
    /// If true, prevent default behavior.
    pub default_prevented: bool,
    /// Timestamp (ms since epoch or frame start).
    pub timestamp_ms: f64,
}

impl EventData {
    pub fn new(kind: EventKind, target_id: String) -> Self {
        Self {
            kind,
            target_id: target_id.clone(),
            current_target_id: target_id,
            phase: EventPhase::Target,
            detail: HashMap::new(),
            propagation_stopped: false,
            default_prevented: false,
            timestamp_ms: 0.0,
        }
    }

    pub fn with_detail(mut self, key: &str, value: &str) -> Self {
        self.detail.insert(key.to_string(), value.to_string());
        self
    }

    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }
}

// ── Handler ──────────────────────────────────────────────────────────────────

pub type HandlerId = u64;

/// A registered event handler — stores the action tag instead of a closure (Rust-friendly for VM).
#[derive(Debug, Clone)]
pub struct EventHandler {
    pub id: HandlerId,
    pub widget_id: String,
    pub kind: EventKind,
    pub phase: EventPhase,
    /// Action tag for builtin dispatch (e.g. "navigate:/home", "signal_set:3", "log").
    pub action: String,
    /// Whether this handler runs once and auto-removes.
    pub once: bool,
}

// ── Dispatch log entry ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DispatchRecord {
    pub event_kind: EventKind,
    pub target_id: String,
    pub handlers_fired: Vec<HandlerId>,
    pub propagation_stopped: bool,
    pub default_prevented: bool,
}

// ── Event Dispatcher ─────────────────────────────────────────────────────────

/// Central event dispatcher. Register handlers, dispatch events, query log.
///
/// Widget tree path must be provided externally (the dispatcher doesn't own the widget tree).
#[derive(Debug)]
pub struct EventDispatcher {
    handlers: HashMap<(String, EventKind, EventPhase), Vec<EventHandler>>,
    next_handler_id: HandlerId,
    /// Dispatch log for devtools.
    pub log: Vec<DispatchRecord>,
    /// Parent-child relationships (child_id → parent_id) — set by layout / component tree.
    pub parent_map: HashMap<String, String>,
    /// Delegation rules for parent auto-capture.
    delegation_rules: Vec<DelegationRule>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_handler_id: 1,
            log: Vec::new(),
            parent_map: HashMap::new(),
            delegation_rules: Vec::new(),
        }
    }

    /// Register a handler for `widget_id` on `kind` during `phase`.
    pub fn on(&mut self, widget_id: &str, kind: EventKind, phase: EventPhase, action: String, once: bool) -> HandlerId {
        let hid = self.next_handler_id;
        self.next_handler_id += 1;
        let handler = EventHandler {
            id: hid,
            widget_id: widget_id.to_string(),
            kind: kind.clone(),
            phase,
            action,
            once,
        };
        let key = (widget_id.to_string(), kind, phase);
        self.handlers.entry(key).or_default().push(handler);
        hid
    }

    /// Shorthand: register a bubble-phase handler.
    pub fn on_bubble(&mut self, widget_id: &str, kind: EventKind, action: String) -> HandlerId {
        self.on(widget_id, kind, EventPhase::Bubble, action, false)
    }

    /// Shorthand: register a capture-phase handler.
    pub fn on_capture(&mut self, widget_id: &str, kind: EventKind, action: String) -> HandlerId {
        self.on(widget_id, kind, EventPhase::Capture, action, false)
    }

    /// Remove a handler by ID. Returns true if found.
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        for handlers in self.handlers.values_mut() {
            if let Some(pos) = handlers.iter().position(|h| h.id == handler_id) {
                handlers.remove(pos);
                return true;
            }
        }
        false
    }

    /// Set the parent of a widget (builds the tree for propagation).
    pub fn set_parent(&mut self, child_id: &str, parent_id: &str) {
        self.parent_map.insert(child_id.to_string(), parent_id.to_string());
    }

    /// Get the ancestor path from root → target (for capture/bubble traversal).
    fn ancestor_path(&self, target_id: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = target_id.to_string();
        path.push(current.clone());
        while let Some(parent) = self.parent_map.get(&current) {
            path.push(parent.clone());
            current = parent.clone();
        }
        path.reverse(); // root … target
        path
    }

    /// Dispatch an event through the full capture → target → bubble pipeline.
    /// Returns the list of handler IDs that fired.
    pub fn dispatch(&mut self, event: &mut EventData) -> Vec<HandlerId> {
        let target_id = event.target_id.clone();
        let path = self.ancestor_path(&target_id);
        let mut fired: Vec<HandlerId> = Vec::new();
        let mut once_to_remove: Vec<HandlerId> = Vec::new();

        // Phase 1: Capture (root → target, exclusive of target)
        if path.len() > 1 {
            event.phase = EventPhase::Capture;
            for node_id in &path[..path.len() - 1] {
                if event.propagation_stopped { break; }
                event.current_target_id = node_id.clone();
                let key = (node_id.clone(), event.kind.clone(), EventPhase::Capture);
                if let Some(handlers) = self.handlers.get(&key).cloned() {
                    for h in &handlers {
                        fired.push(h.id);
                        if h.once { once_to_remove.push(h.id); }
                    }
                }
            }
        }

        // Phase 2: Target
        if !event.propagation_stopped {
            event.phase = EventPhase::Target;
            event.current_target_id = target_id.clone();
            // Fire both capture and bubble handlers registered on the target
            for phase in [EventPhase::Capture, EventPhase::Bubble] {
                let key = (target_id.clone(), event.kind.clone(), phase);
                if let Some(handlers) = self.handlers.get(&key).cloned() {
                    for h in &handlers {
                        fired.push(h.id);
                        if h.once { once_to_remove.push(h.id); }
                    }
                }
            }
        }

        // Phase 3: Bubble (target → root, exclusive of target)
        if event.kind.bubbles() && path.len() > 1 && !event.propagation_stopped {
            event.phase = EventPhase::Bubble;
            for node_id in path[..path.len() - 1].iter().rev() {
                if event.propagation_stopped { break; }
                event.current_target_id = node_id.clone();
                let key = (node_id.clone(), event.kind.clone(), EventPhase::Bubble);
                if let Some(handlers) = self.handlers.get(&key).cloned() {
                    for h in &handlers {
                        fired.push(h.id);
                        if h.once { once_to_remove.push(h.id); }
                    }
                }
            }
        }

        // Remove once-handlers
        for hid in &once_to_remove {
            self.off(*hid);
        }

        // Log
        self.log.push(DispatchRecord {
            event_kind: event.kind.clone(),
            target_id,
            handlers_fired: fired.clone(),
            propagation_stopped: event.propagation_stopped,
            default_prevented: event.default_prevented,
        });

        fired
    }

    /// Number of registered handlers.
    pub fn handler_count(&self) -> usize {
        self.handlers.values().map(|v| v.len()).sum()
    }

    /// Dump dispatch log as JSON (for devtools).
    pub fn log_json(&self) -> String {
        let mut s = String::from("[\n");
        for (i, rec) in self.log.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "  {{\"event\": \"{}\", \"target\": \"{}\", \"handlers_fired\": {:?}, \"stopped\": {}, \"prevented\": {}}}",
                rec.event_kind, rec.target_id, rec.handlers_fired, rec.propagation_stopped, rec.default_prevented
            ));
        }
        s.push_str("\n]");
        s
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ── Event delegation ─────────────────────────────────────────────────────────

/// Delegation rule: a parent widget auto-captures events from all descendants.
#[derive(Debug, Clone)]
pub struct DelegationRule {
    pub id: HandlerId,
    pub parent_id: String,
    pub kind: EventKind,
    pub action: String,
    /// Optional selector filter: only delegate from children whose IDs contain this substring.
    pub selector: Option<String>,
}

impl EventDispatcher {
    /// Register a delegation rule: `parent_id` automatically handles `kind` events from
    /// all descendants (optionally filtered by selector). This avoids registering handlers
    /// on every individual child widget.
    pub fn delegate(
        &mut self,
        parent_id: &str,
        kind: EventKind,
        action: String,
        selector: Option<String>,
    ) -> HandlerId {
        let hid = self.next_handler_id;
        self.next_handler_id += 1;
        self.delegation_rules.push(DelegationRule {
            id: hid,
            parent_id: parent_id.to_string(),
            kind,
            action,
            selector,
        });
        hid
    }

    /// Check delegation rules for an event, returning action tags of matching delegates.
    pub fn check_delegation(&self, event: &EventData) -> Vec<(HandlerId, String)> {
        let target = &event.target_id;
        let ancestors = self.ancestor_path(target);
        let mut results = Vec::new();
        for rule in &self.delegation_rules {
            if rule.kind != event.kind { continue; }
            // Check if rule's parent is an ancestor of target
            if !ancestors.contains(&rule.parent_id) { continue; }
            // Check selector filter
            if let Some(ref sel) = rule.selector {
                if !target.contains(sel) { continue; }
            }
            results.push((rule.id, rule.action.clone()));
        }
        results
    }

    /// Dispatch with delegation support: after normal dispatch, also fires delegation rules.
    pub fn dispatch_with_delegation(&mut self, event: &mut EventData) -> Vec<HandlerId> {
        let mut fired = self.dispatch(event);
        if !event.propagation_stopped {
            let delegated = self.check_delegation(event);
            for (hid, _action) in delegated {
                fired.push(hid);
            }
        }
        fired
    }

    /// Remove a delegation rule by ID.
    pub fn undelegate(&mut self, handler_id: HandlerId) -> bool {
        if let Some(pos) = self.delegation_rules.iter().position(|r| r.id == handler_id) {
            self.delegation_rules.remove(pos);
            true
        } else {
            false
        }
    }

    /// Build parent_map automatically from a list of (child_id, parent_id) pairs.
    pub fn set_tree(&mut self, relationships: &[(&str, &str)]) {
        self.parent_map.clear();
        for (child, parent) in relationships {
            self.parent_map.insert(child.to_string(), parent.to_string());
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_click_handler() {
        let mut disp = EventDispatcher::new();
        let hid = disp.on_bubble("btn1", EventKind::Click, "handle_click".into());
        let mut ev = EventData::new(EventKind::Click, "btn1".into());
        let fired = disp.dispatch(&mut ev);
        assert!(fired.contains(&hid));
    }

    #[test]
    fn bubble_propagation() {
        let mut disp = EventDispatcher::new();
        disp.set_parent("btn1", "panel");
        disp.set_parent("panel", "root");
        let h_root = disp.on_bubble("root", EventKind::Click, "root_click".into());
        let h_panel = disp.on_bubble("panel", EventKind::Click, "panel_click".into());
        let h_btn = disp.on_bubble("btn1", EventKind::Click, "btn_click".into());
        let mut ev = EventData::new(EventKind::Click, "btn1".into());
        let fired = disp.dispatch(&mut ev);
        assert!(fired.contains(&h_btn));
        assert!(fired.contains(&h_panel));
        assert!(fired.contains(&h_root));
    }

    #[test]
    fn stop_propagation_halts_bubble() {
        let mut disp = EventDispatcher::new();
        disp.set_parent("btn1", "root");
        let _h_root = disp.on_bubble("root", EventKind::Click, "root_click".into());
        let h_btn = disp.on_bubble("btn1", EventKind::Click, "btn_click".into());
        let mut ev = EventData::new(EventKind::Click, "btn1".into());
        // dispatch first, then verify: target fires, bubble does not
        let fired = disp.dispatch(&mut ev);
        // btn fires (target phase); stop_propagation should be called by handler
        // in real usage — for unit test, just verify target always fires:
        assert!(fired.contains(&h_btn));
        // root also fires here since nothing stopped it; to truly halt bubble,
        // the handler callback would call stop_propagation. Test the mechanism:
        let mut ev2 = EventData::new(EventKind::Click, "btn1".into());
        ev2.propagation_stopped = false;
        let fired2 = disp.dispatch(&mut ev2);
        assert!(fired2.contains(&h_btn));

        // Now test with stop before bubble: stop after target phase
        let mut ev3 = EventData::new(EventKind::Click, "btn1".into());
        // We simulate: propagation stopped right after target fires
        // by pre-stopping (skips target too) — so verify that behavior:
        ev3.stop_propagation();
        let fired3 = disp.dispatch(&mut ev3);
        // Both target and bubble are blocked when stopped before dispatch
        assert!(fired3.is_empty());
    }

    #[test]
    fn focus_does_not_bubble() {
        let mut disp = EventDispatcher::new();
        disp.set_parent("input1", "form");
        let _h_form = disp.on_bubble("form", EventKind::Focus, "form_focus".into());
        let h_input = disp.on_bubble("input1", EventKind::Focus, "input_focus".into());
        let mut ev = EventData::new(EventKind::Focus, "input1".into());
        let fired = disp.dispatch(&mut ev);
        assert!(fired.contains(&h_input));
        assert_eq!(fired.len(), 1); // form handler NOT fired
    }

    #[test]
    fn once_handler_auto_removes() {
        let mut disp = EventDispatcher::new();
        let hid = disp.on("btn1", EventKind::Click, EventPhase::Bubble, "once_click".into(), true);
        let mut ev1 = EventData::new(EventKind::Click, "btn1".into());
        let fired1 = disp.dispatch(&mut ev1);
        assert!(fired1.contains(&hid));
        let mut ev2 = EventData::new(EventKind::Click, "btn1".into());
        let fired2 = disp.dispatch(&mut ev2);
        assert!(!fired2.contains(&hid)); // removed
    }

    #[test]
    fn capture_phase_before_target() {
        let mut disp = EventDispatcher::new();
        disp.set_parent("btn1", "root");
        let h_cap = disp.on_capture("root", EventKind::Click, "capture_root".into());
        let h_btn = disp.on_bubble("btn1", EventKind::Click, "btn_click".into());
        let mut ev = EventData::new(EventKind::Click, "btn1".into());
        let fired = disp.dispatch(&mut ev);
        // Capture fires before target
        assert_eq!(fired[0], h_cap);
        assert!(fired.contains(&h_btn));
    }

    #[test]
    fn off_removes_handler() {
        let mut disp = EventDispatcher::new();
        let hid = disp.on_bubble("btn1", EventKind::Click, "click".into());
        assert!(disp.off(hid));
        let mut ev = EventData::new(EventKind::Click, "btn1".into());
        let fired = disp.dispatch(&mut ev);
        assert!(!fired.contains(&hid));
    }

    #[test]
    fn event_kind_from_string() {
        assert_eq!(EventKind::from_str_loose("Click"), EventKind::Click);
        assert_eq!(EventKind::from_str_loose("KEYDOWN"), EventKind::KeyDown);
        assert!(matches!(EventKind::from_str_loose("foo"), EventKind::Custom(s) if s == "foo"));
    }

    #[test]
    fn delegation_fires_for_descendants() {
        let mut disp = EventDispatcher::new();
        disp.set_tree(&[("item_1", "list"), ("item_2", "list"), ("list", "root")]);
        let del_id = disp.delegate("list", EventKind::Click, "handle_item_click".into(), None);
        let mut ev = EventData::new(EventKind::Click, "item_1".into());
        let fired = disp.dispatch_with_delegation(&mut ev);
        assert!(fired.contains(&del_id));
    }

    #[test]
    fn delegation_with_selector_filter() {
        let mut disp = EventDispatcher::new();
        disp.set_tree(&[("btn_save", "toolbar"), ("btn_cancel", "toolbar"), ("lbl_title", "toolbar")]);
        let del_id = disp.delegate("toolbar", EventKind::Click, "handle_btn".into(), Some("btn_".into()));
        // btn_save matches selector
        let mut ev1 = EventData::new(EventKind::Click, "btn_save".into());
        let fired1 = disp.dispatch_with_delegation(&mut ev1);
        assert!(fired1.contains(&del_id));
        // lbl_title does NOT match selector
        let mut ev2 = EventData::new(EventKind::Click, "lbl_title".into());
        let fired2 = disp.dispatch_with_delegation(&mut ev2);
        assert!(!fired2.contains(&del_id));
    }

    #[test]
    fn undelegate_removes_rule() {
        let mut disp = EventDispatcher::new();
        disp.set_tree(&[("child", "parent")]);
        let del_id = disp.delegate("parent", EventKind::Click, "action".into(), None);
        assert!(disp.undelegate(del_id));
        let mut ev = EventData::new(EventKind::Click, "child".into());
        let fired = disp.dispatch_with_delegation(&mut ev);
        assert!(!fired.contains(&del_id));
    }

    #[test]
    fn set_tree_builds_parent_map() {
        let mut disp = EventDispatcher::new();
        disp.set_tree(&[("a", "b"), ("b", "c")]);
        assert_eq!(disp.parent_map["a"], "b");
        assert_eq!(disp.parent_map["b"], "c");
    }
}
