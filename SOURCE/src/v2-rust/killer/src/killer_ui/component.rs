//! **Component system** — reusable UI components with props, children, lifecycle, and refs.
//!
//! A component is a template that produces a widget tree. It has:
//! - **Props**: key-value inputs (similar to React props / Angular @Input).
//! - **Children**: nested content (like React `children` / Angular `ng-content`).
//! - **Lifecycle**: mount → update → unmount hooks (action tags for builtin dispatch).
//! - **State**: optional reactive signal IDs owned by this component instance.
//!
//! Components are registered in a [`ComponentRegistry`] by name, then instantiated via
//! [`ComponentTree`] which tracks the live instance hierarchy.

use std::collections::HashMap;

use super::reactive::SignalId;

// ── Component definition (template) ──────────────────────────────────────────

/// A registered component template.
#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    /// Expected prop keys with optional default values.
    pub prop_defs: Vec<PropDef>,
    /// Lifecycle action tags (resolved by builtin dispatcher).
    pub on_mount: Option<String>,
    pub on_update: Option<String>,
    pub on_unmount: Option<String>,
    /// Whether this component accepts children.
    pub accepts_children: bool,
    /// Slot names for named content projection (like Angular named ng-content).
    pub slots: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PropDef {
    pub name: String,
    pub required: bool,
    pub default_value: Option<PropValue>,
}

/// Prop values — lightweight, separate from VM `Value`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PropValue {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<PropValue>),
    Dict(HashMap<String, PropValue>),
    /// Reference to a reactive signal.
    SignalRef(SignalId),
}

impl std::fmt::Display for PropValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropValue::Null => write!(f, "null"),
            PropValue::Bool(b) => write!(f, "{}", b),
            PropValue::Number(n) => write!(f, "{}", n),
            PropValue::Str(s) => write!(f, "{}", s),
            PropValue::Array(a) => write!(f, "[{}]", a.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(", ")),
            PropValue::Dict(d) => write!(f, "{{{}}}", d.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")),
            PropValue::SignalRef(id) => write!(f, "signal({})", id),
        }
    }
}

// ── Component instance (live) ────────────────────────────────────────────────

pub type InstanceId = u64;

/// A live component instance in the tree.
#[derive(Debug, Clone)]
pub struct ComponentInstance {
    pub id: InstanceId,
    pub def_name: String,
    pub props: HashMap<String, PropValue>,
    pub parent_id: Option<InstanceId>,
    pub children_ids: Vec<InstanceId>,
    /// Content children (widget ID references) for default slot.
    pub content_children: Vec<String>,
    /// Named slot content.
    pub named_slots: HashMap<String, Vec<String>>,
    /// Reactive signals owned by this instance.
    pub owned_signals: Vec<SignalId>,
    /// Is this instance currently mounted?
    pub mounted: bool,
    /// Ref tag for programmatic access (like React `useRef`).
    pub ref_tag: Option<String>,
}

/// Lifecycle event kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleEvent {
    Mount(InstanceId),
    Update(InstanceId, Vec<String>), // changed prop names
    Unmount(InstanceId),
}

// ── Component registry ───────────────────────────────────────────────────────

/// Stores component definitions (templates). Register once, instantiate many.
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    defs: HashMap<String, ComponentDef>,
}

impl ComponentRegistry {
    pub fn new() -> Self { Self { defs: HashMap::new() } }

    pub fn register(&mut self, def: ComponentDef) {
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&ComponentDef> {
        self.defs.get(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.defs.keys().map(|s| s.as_str()).collect()
    }

    pub fn count(&self) -> usize {
        self.defs.len()
    }
}

// ── Component tree (live instances) ──────────────────────────────────────────

/// Manages the live component instance hierarchy.
#[derive(Debug)]
pub struct ComponentTree {
    instances: HashMap<InstanceId, ComponentInstance>,
    next_id: InstanceId,
    /// Root instance IDs (no parent).
    roots: Vec<InstanceId>,
    /// Lifecycle events pending processing.
    pub pending_lifecycle: Vec<LifecycleEvent>,
    /// Ref tag → instance ID mapping.
    refs: HashMap<String, InstanceId>,
}

impl ComponentTree {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_id: 1,
            roots: Vec::new(),
            pending_lifecycle: Vec::new(),
            refs: HashMap::new(),
        }
    }

    /// Create and mount a new component instance.
    pub fn create(
        &mut self,
        registry: &ComponentRegistry,
        def_name: &str,
        props: HashMap<String, PropValue>,
        parent_id: Option<InstanceId>,
    ) -> Result<InstanceId, ComponentError> {
        let def = registry.get(def_name)
            .ok_or_else(|| ComponentError::UnknownComponent(def_name.to_string()))?;

        // Validate required props
        for prop_def in &def.prop_defs {
            if prop_def.required && !props.contains_key(&prop_def.name) {
                return Err(ComponentError::MissingProp(def_name.to_string(), prop_def.name.clone()));
            }
        }

        // Validate parent exists
        if let Some(pid) = parent_id {
            if !self.instances.contains_key(&pid) {
                return Err(ComponentError::UnknownInstance(pid));
            }
        }

        let id = self.next_id;
        self.next_id += 1;

        // Merge defaults for missing props
        let mut full_props = HashMap::new();
        for prop_def in &def.prop_defs {
            if let Some(v) = props.get(&prop_def.name) {
                full_props.insert(prop_def.name.clone(), v.clone());
            } else if let Some(def_val) = &prop_def.default_value {
                full_props.insert(prop_def.name.clone(), def_val.clone());
            }
        }
        // Pass through any extra props
        for (k, v) in &props {
            if !full_props.contains_key(k) {
                full_props.insert(k.clone(), v.clone());
            }
        }

        let instance = ComponentInstance {
            id,
            def_name: def_name.to_string(),
            props: full_props,
            parent_id,
            children_ids: Vec::new(),
            content_children: Vec::new(),
            named_slots: HashMap::new(),
            owned_signals: Vec::new(),
            mounted: true,
            ref_tag: None,
        };

        self.instances.insert(id, instance);

        // Register with parent
        if let Some(pid) = parent_id {
            self.instances.get_mut(&pid).unwrap().children_ids.push(id);
        } else {
            self.roots.push(id);
        }

        self.pending_lifecycle.push(LifecycleEvent::Mount(id));
        Ok(id)
    }

    /// Update props on a mounted instance. Returns changed prop names.
    pub fn update_props(
        &mut self,
        id: InstanceId,
        new_props: HashMap<String, PropValue>,
    ) -> Result<Vec<String>, ComponentError> {
        let instance = self.instances.get_mut(&id)
            .ok_or(ComponentError::UnknownInstance(id))?;
        let mut changed = Vec::new();
        for (k, v) in &new_props {
            let old = instance.props.get(k);
            if old != Some(v) {
                changed.push(k.clone());
                instance.props.insert(k.clone(), v.clone());
            }
        }
        if !changed.is_empty() {
            self.pending_lifecycle.push(LifecycleEvent::Update(id, changed.clone()));
        }
        Ok(changed)
    }

    /// Unmount and remove a component instance (and all descendants).
    pub fn unmount(&mut self, id: InstanceId) -> Result<(), ComponentError> {
        if !self.instances.contains_key(&id) {
            return Err(ComponentError::UnknownInstance(id));
        }

        // Collect all descendants depth-first
        let mut to_remove = Vec::new();
        self.collect_descendants(id, &mut to_remove);
        to_remove.push(id);

        // Unmount in reverse order (children before parents)
        for &remove_id in &to_remove {
            self.pending_lifecycle.push(LifecycleEvent::Unmount(remove_id));
            if let Some(inst) = self.instances.get(&remove_id) {
                if let Some(ref_tag) = &inst.ref_tag {
                    self.refs.remove(ref_tag);
                }
            }
            self.instances.remove(&remove_id);
        }

        // Remove from parent's children
        if let Some(inst) = self.instances.values().find(|i| i.children_ids.contains(&id)) {
            let pid = inst.id;
            self.instances.get_mut(&pid).unwrap().children_ids.retain(|c| *c != id);
        }
        self.roots.retain(|r| *r != id);

        Ok(())
    }

    fn collect_descendants(&self, id: InstanceId, out: &mut Vec<InstanceId>) {
        if let Some(inst) = self.instances.get(&id) {
            for &child_id in &inst.children_ids {
                self.collect_descendants(child_id, out);
                out.push(child_id);
            }
        }
    }

    /// Set a ref tag on an instance for lookup by name.
    pub fn set_ref(&mut self, id: InstanceId, ref_tag: &str) -> Result<(), ComponentError> {
        let inst = self.instances.get_mut(&id).ok_or(ComponentError::UnknownInstance(id))?;
        inst.ref_tag = Some(ref_tag.to_string());
        self.refs.insert(ref_tag.to_string(), id);
        Ok(())
    }

    /// Look up an instance by ref tag.
    pub fn get_by_ref(&self, ref_tag: &str) -> Option<&ComponentInstance> {
        self.refs.get(ref_tag).and_then(|id| self.instances.get(id))
    }

    /// Get instance by ID.
    pub fn get(&self, id: InstanceId) -> Option<&ComponentInstance> {
        self.instances.get(&id)
    }

    /// Total live instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Root instances.
    pub fn root_ids(&self) -> &[InstanceId] {
        &self.roots
    }

    /// Drain pending lifecycle events.
    pub fn take_lifecycle_events(&mut self) -> Vec<LifecycleEvent> {
        std::mem::take(&mut self.pending_lifecycle)
    }

    /// Dump tree as JSON.
    pub fn debug_json(&self) -> String {
        let mut s = String::from("{\n  \"instances\": [\n");
        let mut ids: Vec<InstanceId> = self.instances.keys().cloned().collect();
        ids.sort();
        for (i, id) in ids.iter().enumerate() {
            let inst = &self.instances[id];
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!(
                "    {{\"id\": {}, \"def\": \"{}\", \"parent\": {:?}, \"children\": {:?}, \"mounted\": {}, \"ref\": {:?}}}",
                inst.id, inst.def_name, inst.parent_id, inst.children_ids, inst.mounted, inst.ref_tag
            ));
        }
        s.push_str(&format!("\n  ],\n  \"roots\": {:?}\n}}\n", self.roots));
        s
    }
}

impl Default for ComponentTree {
    fn default() -> Self { Self::new() }
}

// ── Slot projection ──────────────────────────────────────────────────────────

/// Projected slot content: maps slot names to widget ID lists.
/// The component template declares named slots; the parent provides content for each.
#[derive(Debug, Clone, Default)]
pub struct SlotProjection {
    /// slot_name → list of projected widget IDs.
    pub slots: HashMap<String, Vec<String>>,
    /// Default slot content (unnamed / fallback).
    pub default_content: Vec<String>,
}

impl SlotProjection {
    pub fn new() -> Self { Self::default() }

    /// Project content into a named slot.
    pub fn project(&mut self, slot_name: &str, widget_ids: Vec<String>) {
        self.slots.insert(slot_name.to_string(), widget_ids);
    }

    /// Set default slot content.
    pub fn set_default(&mut self, widget_ids: Vec<String>) {
        self.default_content = widget_ids;
    }

    /// Resolve a slot: returns projected content, or fallback if none projected.
    pub fn resolve(&self, slot_name: &str, fallback: &[String]) -> Vec<String> {
        if let Some(content) = self.slots.get(slot_name) {
            if !content.is_empty() { return content.clone(); }
        }
        fallback.to_vec()
    }

    /// Resolve default slot.
    pub fn resolve_default(&self, fallback: &[String]) -> Vec<String> {
        if !self.default_content.is_empty() {
            self.default_content.clone()
        } else {
            fallback.to_vec()
        }
    }
}

impl ComponentTree {
    /// Set named slot content on an instance.
    pub fn set_slot_content(
        &mut self,
        id: InstanceId,
        slot_name: &str,
        widget_ids: Vec<String>,
    ) -> Result<(), ComponentError> {
        let inst = self.instances.get_mut(&id).ok_or(ComponentError::UnknownInstance(id))?;
        inst.named_slots.insert(slot_name.to_string(), widget_ids);
        Ok(())
    }

    /// Build a SlotProjection from an instance's current slot data.
    pub fn slot_projection(&self, id: InstanceId) -> Result<SlotProjection, ComponentError> {
        let inst = self.instances.get(&id).ok_or(ComponentError::UnknownInstance(id))?;
        let mut proj = SlotProjection::new();
        proj.default_content = inst.content_children.clone();
        for (name, ids) in &inst.named_slots {
            proj.project(name, ids.clone());
        }
        Ok(proj)
    }
}

// ── Error boundary ───────────────────────────────────────────────────────────

/// Error boundary: wraps a subtree and catches errors during rendering/lifecycle.
/// When an error occurs, the boundary shows a fallback instead of crashing the whole tree.
#[derive(Debug, Clone)]
pub struct ErrorBoundary {
    pub id: InstanceId,
    /// Component instance IDs covered by this boundary.
    pub covered_ids: Vec<InstanceId>,
    /// Current error, if any.
    pub error: Option<BoundaryError>,
    /// Fallback action tag (rendered when error occurs).
    pub fallback_action: String,
    /// Number of resets/retries.
    pub reset_count: u32,
}

#[derive(Debug, Clone)]
pub struct BoundaryError {
    pub source_id: InstanceId,
    pub message: String,
    pub lifecycle_phase: String,
}

impl ErrorBoundary {
    pub fn new(id: InstanceId, fallback_action: &str) -> Self {
        Self {
            id,
            covered_ids: Vec::new(),
            error: None,
            fallback_action: fallback_action.to_string(),
            reset_count: 0,
        }
    }

    /// Add a component to this boundary's coverage.
    pub fn cover(&mut self, instance_id: InstanceId) {
        if !self.covered_ids.contains(&instance_id) {
            self.covered_ids.push(instance_id);
        }
    }

    /// Report an error from a covered component.
    pub fn catch_error(&mut self, source_id: InstanceId, message: &str, phase: &str) -> bool {
        if self.covered_ids.contains(&source_id) {
            self.error = Some(BoundaryError {
                source_id,
                message: message.to_string(),
                lifecycle_phase: phase.to_string(),
            });
            true
        } else {
            false
        }
    }

    /// Check if this boundary is in error state.
    pub fn has_error(&self) -> bool { self.error.is_some() }

    /// Reset the boundary (retry rendering).
    pub fn reset(&mut self) {
        self.error = None;
        self.reset_count += 1;
    }

    /// Get the action to render (fallback if error, normal otherwise).
    pub fn render_action(&self) -> Option<&str> {
        if self.has_error() {
            Some(&self.fallback_action)
        } else {
            None
        }
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    UnknownComponent(String),
    UnknownInstance(InstanceId),
    MissingProp(String, String),
}

impl std::fmt::Display for ComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentError::UnknownComponent(name) => write!(f, "unknown component '{}'", name),
            ComponentError::UnknownInstance(id) => write!(f, "unknown instance {}", id),
            ComponentError::MissingProp(comp, prop) => write!(f, "component '{}' missing required prop '{}'", comp, prop),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_button_def() -> ComponentDef {
        ComponentDef {
            name: "Button".to_string(),
            prop_defs: vec![
                PropDef { name: "label".into(), required: true, default_value: None },
                PropDef { name: "variant".into(), required: false, default_value: Some(PropValue::Str("primary".into())) },
                PropDef { name: "disabled".into(), required: false, default_value: Some(PropValue::Bool(false)) },
            ],
            on_mount: Some("button_mounted".into()),
            on_update: Some("button_updated".into()),
            on_unmount: Some("button_unmounted".into()),
            accepts_children: false,
            slots: vec![],
        }
    }

    fn make_card_def() -> ComponentDef {
        ComponentDef {
            name: "Card".to_string(),
            prop_defs: vec![
                PropDef { name: "title".into(), required: true, default_value: None },
                PropDef { name: "elevated".into(), required: false, default_value: Some(PropValue::Bool(false)) },
            ],
            on_mount: None,
            on_update: None,
            on_unmount: None,
            accepts_children: true,
            slots: vec!["header".into(), "footer".into()],
        }
    }

    #[test]
    fn register_and_create() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let mut props = HashMap::new();
        props.insert("label".into(), PropValue::Str("Click me".into()));
        let id = tree.create(&reg, "Button", props, None).unwrap();
        assert_eq!(tree.instance_count(), 1);
        let inst = tree.get(id).unwrap();
        assert_eq!(inst.props["label"], PropValue::Str("Click me".into()));
        assert_eq!(inst.props["variant"], PropValue::Str("primary".into())); // default
    }

    #[test]
    fn missing_required_prop_errors() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let result = tree.create(&reg, "Button", HashMap::new(), None);
        assert!(matches!(result, Err(ComponentError::MissingProp(_, _))));
    }

    #[test]
    fn parent_child_hierarchy() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_card_def());
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let card_id = tree.create(&reg, "Card", HashMap::from([("title".into(), PropValue::Str("Hi".into()))]), None).unwrap();
        let btn_id = tree.create(&reg, "Button", HashMap::from([("label".into(), PropValue::Str("OK".into()))]), Some(card_id)).unwrap();
        let card = tree.get(card_id).unwrap();
        assert!(card.children_ids.contains(&btn_id));
        let btn = tree.get(btn_id).unwrap();
        assert_eq!(btn.parent_id, Some(card_id));
    }

    #[test]
    fn update_props_fires_lifecycle() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let id = tree.create(&reg, "Button", HashMap::from([("label".into(), PropValue::Str("A".into()))]), None).unwrap();
        tree.take_lifecycle_events(); // drain mount
        tree.update_props(id, HashMap::from([("label".into(), PropValue::Str("B".into()))])).unwrap();
        let events = tree.take_lifecycle_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], LifecycleEvent::Update(_, changed) if changed == &["label"]));
    }

    #[test]
    fn unmount_removes_descendants() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_card_def());
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let card = tree.create(&reg, "Card", HashMap::from([("title".into(), PropValue::Str("Hi".into()))]), None).unwrap();
        let _btn = tree.create(&reg, "Button", HashMap::from([("label".into(), PropValue::Str("OK".into()))]), Some(card)).unwrap();
        assert_eq!(tree.instance_count(), 2);
        tree.unmount(card).unwrap();
        assert_eq!(tree.instance_count(), 0);
    }

    #[test]
    fn ref_tag_lookup() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_button_def());
        let mut tree = ComponentTree::new();
        let id = tree.create(&reg, "Button", HashMap::from([("label".into(), PropValue::Str("X".into()))]), None).unwrap();
        tree.set_ref(id, "submit_btn").unwrap();
        let found = tree.get_by_ref("submit_btn").unwrap();
        assert_eq!(found.id, id);
    }

    #[test]
    fn slot_projection_named() {
        let mut reg = ComponentRegistry::new();
        reg.register(make_card_def());
        let mut tree = ComponentTree::new();
        let card_id = tree.create(&reg, "Card", HashMap::from([("title".into(), PropValue::Str("Card".into()))]), None).unwrap();
        tree.set_slot_content(card_id, "header", vec!["logo_widget".into(), "nav_widget".into()]).unwrap();
        tree.set_slot_content(card_id, "footer", vec!["copyright_widget".into()]).unwrap();
        let proj = tree.slot_projection(card_id).unwrap();
        assert_eq!(proj.resolve("header", &[]), vec!["logo_widget", "nav_widget"]);
        assert_eq!(proj.resolve("footer", &[]), vec!["copyright_widget"]);
    }

    #[test]
    fn slot_fallback_on_empty() {
        let proj = SlotProjection::new();
        let fallback = vec!["default_header".to_string()];
        assert_eq!(proj.resolve("header", &fallback), fallback);
    }

    #[test]
    fn error_boundary_catches() {
        let mut boundary = ErrorBoundary::new(100, "show_error_page");
        boundary.cover(42);
        boundary.cover(43);
        assert!(!boundary.has_error());
        assert!(boundary.catch_error(42, "render failed", "mount"));
        assert!(boundary.has_error());
        assert_eq!(boundary.render_action(), Some("show_error_page"));
        boundary.reset();
        assert!(!boundary.has_error());
        assert_eq!(boundary.reset_count, 1);
    }

    #[test]
    fn error_boundary_ignores_uncovered() {
        let mut boundary = ErrorBoundary::new(100, "fallback");
        boundary.cover(42);
        assert!(!boundary.catch_error(99, "some error", "update"));
        assert!(!boundary.has_error());
    }
}
