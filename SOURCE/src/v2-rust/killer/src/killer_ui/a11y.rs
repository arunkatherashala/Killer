//! **Accessibility runtime** — ARIA roles, labels, focus management, and live regions.

use std::collections::HashMap;

/// ARIA role for a widget.
#[derive(Debug, Clone, PartialEq)]
pub enum AriaRole {
    Button, Link, Checkbox, Radio, Slider, Textbox, Combobox, Listbox,
    Menu, MenuItem, MenuBar, Tab, TabPanel, TabList, Tree, TreeItem,
    Dialog, AlertDialog, Alert, Status, Log, Timer, Progressbar,
    Navigation, Main, Banner, Contentinfo, Complementary, Search, Form,
    Table, Row, Cell, ColumnHeader, RowHeader, Grid, GridCell,
    Region, Group, Separator, Img, Figure, Tooltip, Feed, Article,
    None,
}

/// Accessibility properties for a widget.
#[derive(Debug, Clone)]
pub struct AriaProps {
    pub role: AriaRole,
    pub label: Option<String>,
    pub described_by: Option<String>,
    pub live: Option<AriaLive>,
    pub expanded: Option<bool>,
    pub selected: Option<bool>,
    pub checked: Option<AriaChecked>,
    pub disabled: bool,
    pub hidden: bool,
    pub value_now: Option<f64>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_text: Option<String>,
    pub level: Option<u32>,
    pub controls: Option<String>,
    pub owns: Option<String>,
    pub tab_index: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AriaLive { Off, Polite, Assertive }

#[derive(Debug, Clone, PartialEq)]
pub enum AriaChecked { False, True, Mixed }

impl Default for AriaProps {
    fn default() -> Self {
        Self {
            role: AriaRole::None, label: None, described_by: None, live: None,
            expanded: None, selected: None, checked: None, disabled: false,
            hidden: false, value_now: None, value_min: None, value_max: None,
            value_text: None, level: None, controls: None, owns: None, tab_index: None,
        }
    }
}

/// Focus manager — tracks tab order and current focus.
#[derive(Debug)]
pub struct FocusManager {
    /// Ordered focusable widget IDs.
    pub tab_order: Vec<String>,
    /// Currently focused widget ID.
    pub focused: Option<String>,
    /// Focus trap stack for modals.
    pub trap_stack: Vec<Vec<String>>,
}

impl FocusManager {
    pub fn new() -> Self { Self { tab_order: Vec::new(), focused: None, trap_stack: Vec::new() } }

    pub fn register(&mut self, widget_id: &str, tab_index: i32) {
        if tab_index >= 0 {
            self.tab_order.push(widget_id.to_string());
        }
    }

    pub fn focus(&mut self, widget_id: &str) {
        self.focused = Some(widget_id.to_string());
    }

    pub fn blur(&mut self) { self.focused = None; }

    pub fn focus_next(&mut self) {
        let active = self.active_order();
        if active.is_empty() { return; }
        let idx = self.focused.as_ref().and_then(|f| active.iter().position(|w| w == f)).unwrap_or(active.len() - 1);
        let next = (idx + 1) % active.len();
        self.focused = Some(active[next].clone());
    }

    pub fn focus_prev(&mut self) {
        let active = self.active_order();
        if active.is_empty() { return; }
        let idx = self.focused.as_ref().and_then(|f| active.iter().position(|w| w == f)).unwrap_or(0);
        let prev = if idx == 0 { active.len() - 1 } else { idx - 1 };
        self.focused = Some(active[prev].clone());
    }

    /// Push a focus trap (for dialogs/modals).
    pub fn push_trap(&mut self, widget_ids: Vec<String>) {
        self.trap_stack.push(widget_ids);
        if let Some(trap) = self.trap_stack.last() {
            if let Some(first) = trap.first() { self.focused = Some(first.clone()); }
        }
    }

    /// Pop focus trap, restore previous focus scope.
    pub fn pop_trap(&mut self) { self.trap_stack.pop(); }

    fn active_order(&self) -> &Vec<String> {
        self.trap_stack.last().unwrap_or(&self.tab_order)
    }
}

impl Default for FocusManager {
    fn default() -> Self { Self::new() }
}

/// Accessibility tree for screen reader output.
#[derive(Debug)]
pub struct A11yTree {
    pub nodes: HashMap<String, AriaProps>,
}

impl A11yTree {
    pub fn new() -> Self { Self { nodes: HashMap::new() } }

    pub fn set(&mut self, widget_id: &str, props: AriaProps) {
        self.nodes.insert(widget_id.to_string(), props);
    }

    pub fn get(&self, widget_id: &str) -> Option<&AriaProps> { self.nodes.get(widget_id) }

    /// Generate screen-reader-friendly text summary.
    pub fn announce(&self, widget_id: &str) -> String {
        if let Some(p) = self.nodes.get(widget_id) {
            let role = format!("{:?}", p.role).to_lowercase();
            let label = p.label.as_deref().unwrap_or("");
            let state = if p.disabled { ", disabled" } else { "" };
            format!("{} {}{}", role, label, state)
        } else {
            String::new()
        }
    }

    /// All live region announcements.
    pub fn live_announcements(&self) -> Vec<(String, String)> {
        self.nodes.iter()
            .filter(|(_, p)| matches!(p.live, Some(AriaLive::Polite) | Some(AriaLive::Assertive)))
            .filter_map(|(id, p)| p.label.as_ref().map(|l| (id.clone(), l.clone())))
            .collect()
    }
}

impl Default for A11yTree {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_next_cycles() {
        let mut fm = FocusManager::new();
        fm.register("a", 0);
        fm.register("b", 0);
        fm.register("c", 0);
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("a"));
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("b"));
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("c"));
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("a")); // wraps
    }

    #[test]
    fn focus_trap_constrains() {
        let mut fm = FocusManager::new();
        fm.register("a", 0);
        fm.register("b", 0);
        fm.push_trap(vec!["x".into(), "y".into()]);
        assert_eq!(fm.focused.as_deref(), Some("x"));
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("y"));
        fm.focus_next();
        assert_eq!(fm.focused.as_deref(), Some("x")); // trapped
        fm.pop_trap();
    }

    #[test]
    fn a11y_tree_announce() {
        let mut tree = A11yTree::new();
        tree.set("btn1", AriaProps { role: AriaRole::Button, label: Some("Submit".into()), ..Default::default() });
        assert!(tree.announce("btn1").contains("button"));
        assert!(tree.announce("btn1").contains("Submit"));
    }
}
