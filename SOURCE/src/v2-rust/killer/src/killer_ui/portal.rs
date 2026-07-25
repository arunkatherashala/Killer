//! **Portal** — React-style portals for rendering into different DOM targets.
//!
//! Render children outside their parent's subtree while maintaining event bubbling
//! and context propagation. Used for modals, tooltips, dropdowns, toasts.

use super::patch::{Widget, WidgetId};
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Portal
// ══════════════════════════════════════════════════════════════════════════════

/// A portal target — a named mount point in the widget tree.
#[derive(Debug, Clone)]
pub struct PortalTarget {
    pub id: String,
    pub container_id: WidgetId,
}

impl PortalTarget {
    pub fn new(id: &str, container_id: &str) -> Self {
        PortalTarget { id: id.into(), container_id: container_id.into() }
    }
}

/// A portal outlet — renders children at a specific portal target.
#[derive(Debug, Clone)]
pub struct Portal {
    pub id: String,
    pub target_id: String,
    pub children: Vec<Widget>,
    pub source_component: String,
    pub priority: i32,
}

impl Portal {
    pub fn new(id: &str, target_id: &str) -> Self {
        Portal {
            id: id.into(),
            target_id: target_id.into(),
            children: Vec::new(),
            source_component: String::new(),
            priority: 0,
        }
    }

    pub fn with_children(mut self, children: Vec<Widget>) -> Self {
        self.children = children; self
    }

    pub fn with_source(mut self, component: &str) -> Self {
        self.source_component = component.into(); self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority; self
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Portal Manager
// ══════════════════════════════════════════════════════════════════════════════

/// Manages portal targets and active portals.
pub struct PortalManager {
    targets: HashMap<String, PortalTarget>,
    portals: Vec<Portal>,
}

impl PortalManager {
    pub fn new() -> Self {
        PortalManager { targets: HashMap::new(), portals: Vec::new() }
    }

    /// Register a portal mount point.
    pub fn register_target(&mut self, target: PortalTarget) {
        self.targets.insert(target.id.clone(), target);
    }

    /// Create a default "root" portal target (modal/toast layer).
    pub fn register_root_target(&mut self) {
        self.register_target(PortalTarget::new("root", "__portal_root__"));
        self.register_target(PortalTarget::new("modal", "__modal_root__"));
        self.register_target(PortalTarget::new("toast", "__toast_root__"));
        self.register_target(PortalTarget::new("tooltip", "__tooltip_root__"));
    }

    /// Open a portal (render children at target).
    pub fn open(&mut self, portal: Portal) {
        self.portals.push(portal);
    }

    /// Close a portal by id.
    pub fn close(&mut self, portal_id: &str) {
        self.portals.retain(|p| p.id != portal_id);
    }

    /// Close all portals targeting a specific target.
    pub fn close_all_at(&mut self, target_id: &str) {
        self.portals.retain(|p| p.target_id != target_id);
    }

    /// Collect all widgets for a given target (sorted by priority).
    pub fn render_target(&self, target_id: &str) -> Vec<Widget> {
        let mut matching: Vec<&Portal> = self.portals.iter()
            .filter(|p| p.target_id == target_id)
            .collect();
        matching.sort_by_key(|p| p.priority);
        matching.into_iter().flat_map(|p| p.children.clone()).collect()
    }

    /// Check if any portals are open at a target.
    pub fn has_portals_at(&self, target_id: &str) -> bool {
        self.portals.iter().any(|p| p.target_id == target_id)
    }

    pub fn target_count(&self) -> usize { self.targets.len() }
    pub fn portal_count(&self) -> usize { self.portals.len() }

    /// Get all target IDs.
    pub fn target_ids(&self) -> Vec<&str> {
        self.targets.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for PortalManager {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_target() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        assert_eq!(mgr.target_count(), 4);
    }

    #[test]
    fn open_portal() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        mgr.open(Portal::new("modal-1", "modal")
            .with_children(vec![Widget::Label { id: "msg".into(), text: "Hello".into() }]));
        assert_eq!(mgr.portal_count(), 1);
        assert!(mgr.has_portals_at("modal"));
    }

    #[test]
    fn render_target_collects_children() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        mgr.open(Portal::new("t1", "toast")
            .with_children(vec![Widget::Label { id: "a".into(), text: "Toast 1".into() }]));
        mgr.open(Portal::new("t2", "toast")
            .with_children(vec![Widget::Label { id: "b".into(), text: "Toast 2".into() }]));
        let rendered = mgr.render_target("toast");
        assert_eq!(rendered.len(), 2);
    }

    #[test]
    fn priority_ordering() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        mgr.open(Portal::new("low", "modal").with_priority(10)
            .with_children(vec![Widget::Label { id: "lo".into(), text: "Low".into() }]));
        mgr.open(Portal::new("high", "modal").with_priority(1)
            .with_children(vec![Widget::Label { id: "hi".into(), text: "High".into() }]));
        let rendered = mgr.render_target("modal");
        if let Widget::Label { text, .. } = &rendered[0] {
            assert_eq!(text, "High");
        }
    }

    #[test]
    fn close_portal() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        mgr.open(Portal::new("m1", "modal").with_children(vec![]));
        mgr.close("m1");
        assert_eq!(mgr.portal_count(), 0);
        assert!(!mgr.has_portals_at("modal"));
    }

    #[test]
    fn close_all_at() {
        let mut mgr = PortalManager::new();
        mgr.register_root_target();
        mgr.open(Portal::new("t1", "toast").with_children(vec![]));
        mgr.open(Portal::new("t2", "toast").with_children(vec![]));
        mgr.open(Portal::new("m1", "modal").with_children(vec![]));
        mgr.close_all_at("toast");
        assert_eq!(mgr.portal_count(), 1);
        assert!(!mgr.has_portals_at("toast"));
        assert!(mgr.has_portals_at("modal"));
    }

    #[test]
    fn empty_target_renders_nothing() {
        let mgr = PortalManager::new();
        assert!(mgr.render_target("nonexistent").is_empty());
    }
}
