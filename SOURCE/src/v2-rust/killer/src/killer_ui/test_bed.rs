//! **Component TestBed** — Test harness for rendering components in isolation.
//!
//! Inspired by Angular TestBed / React Testing Library.
//! Provides: `TestHost` for mounting components, `render()` for quick render,
//! query helpers (by text, role, id), event simulation, assertion helpers.

use super::patch::Widget;
use super::component::ComponentRegistry;

// ══════════════════════════════════════════════════════════════════════════════
// TestHost — mounts and manages a component under test
// ══════════════════════════════════════════════════════════════════════════════

/// A test environment that hosts a widget tree for inspection and interaction.
pub struct TestHost {
    pub root: Vec<Widget>,
    pub events_fired: Vec<TestEvent>,
    #[allow(dead_code)]
    registry: ComponentRegistry,
}

#[derive(Debug, Clone)]
pub struct TestEvent {
    pub target_id: String,
    pub event_type: String,
    pub timestamp_ms: u64,
}

impl TestHost {
    /// Create an empty test host.
    pub fn new() -> Self {
        TestHost {
            root: Vec::new(),
            events_fired: Vec::new(),
            registry: ComponentRegistry::new(),
        }
    }

    /// Mount a widget tree for testing.
    pub fn render(widgets: Vec<Widget>) -> Self {
        TestHost {
            root: widgets,
            events_fired: Vec::new(),
            registry: ComponentRegistry::new(),
        }
    }

    /// Mount a single widget.
    pub fn render_one(widget: Widget) -> Self {
        Self::render(vec![widget])
    }

    // ── Query helpers ────────────────────────────────────────────────────

    /// Find a widget by its ID.
    pub fn query_by_id(&self, id: &str) -> Option<&Widget> {
        find_by_id(&self.root, id)
    }

    /// Find all widgets matching a predicate.
    pub fn query_all<F: Fn(&Widget) -> bool>(&self, predicate: F) -> Vec<&Widget> {
        let mut results = Vec::new();
        collect_matching(&self.root, &predicate, &mut results);
        results
    }

    /// Find a widget by its text content (label text, button label, etc.)
    pub fn query_by_text(&self, text: &str) -> Option<&Widget> {
        self.query_all(|w| widget_text(w).as_deref() == Some(text)).into_iter().next()
    }

    /// Find all Buttons.
    pub fn query_buttons(&self) -> Vec<&Widget> {
        self.query_all(|w| matches!(w, Widget::Button { .. }))
    }

    /// Find all TextInputs.
    pub fn query_inputs(&self) -> Vec<&Widget> {
        self.query_all(|w| matches!(w, Widget::TextInput { .. }))
    }

    /// Find all Labels.
    pub fn query_labels(&self) -> Vec<&Widget> {
        self.query_all(|w| matches!(w, Widget::Label { .. }))
    }

    /// Count total widgets in the tree.
    pub fn widget_count(&self) -> usize {
        count_widgets(&self.root)
    }

    // ── Event simulation ─────────────────────────────────────────────────

    /// Simulate a click event on a widget by ID.
    pub fn fire_click(&mut self, id: &str) {
        self.events_fired.push(TestEvent {
            target_id: id.to_string(),
            event_type: "click".into(),
            timestamp_ms: self.events_fired.len() as u64,
        });
    }

    /// Simulate typing into an input by ID.
    pub fn fire_input(&mut self, id: &str, value: &str) {
        // Update the widget value in-tree
        set_input_value(&mut self.root, id, value);
        self.events_fired.push(TestEvent {
            target_id: id.to_string(),
            event_type: "input".into(),
            timestamp_ms: self.events_fired.len() as u64,
        });
    }

    // ── Assertions ───────────────────────────────────────────────────────

    /// Assert a widget with given ID exists.
    pub fn assert_exists(&self, id: &str) {
        assert!(self.query_by_id(id).is_some(), "Widget '{}' not found", id);
    }

    /// Assert a widget with given text content exists.
    pub fn assert_text_present(&self, text: &str) {
        assert!(self.query_by_text(text).is_some(), "Text '{}' not found in widget tree", text);
    }

    /// Assert no widget with given text exists.
    pub fn assert_text_absent(&self, text: &str) {
        assert!(self.query_by_text(text).is_none(), "Text '{}' should not be in tree", text);
    }

    /// Assert a button is disabled.
    pub fn assert_disabled(&self, id: &str) {
        if let Some(Widget::Button { disabled, .. }) = self.query_by_id(id) {
            assert!(*disabled, "Button '{}' should be disabled", id);
        } else {
            panic!("Widget '{}' is not a Button", id);
        }
    }

    /// Assert a button is enabled.
    pub fn assert_enabled(&self, id: &str) {
        if let Some(Widget::Button { disabled, .. }) = self.query_by_id(id) {
            assert!(!disabled, "Button '{}' should be enabled", id);
        } else {
            panic!("Widget '{}' is not a Button", id);
        }
    }

    /// Assert a checkbox is checked.
    pub fn assert_checked(&self, id: &str) {
        if let Some(Widget::Checkbox { checked, .. }) = self.query_by_id(id) {
            assert!(*checked, "Checkbox '{}' should be checked", id);
        } else {
            panic!("Widget '{}' is not a Checkbox", id);
        }
    }

    /// Assert input has a specific value.
    pub fn assert_input_value(&self, id: &str, expected: &str) {
        if let Some(Widget::TextInput { value, .. }) = self.query_by_id(id) {
            assert_eq!(value, expected, "Input '{}' value mismatch", id);
        } else {
            panic!("Widget '{}' is not a TextInput", id);
        }
    }

    /// Get debug tree as a string (for snapshots/debugging).
    pub fn debug_tree(&self) -> String {
        let mut out = String::new();
        for w in &self.root {
            debug_widget(w, 0, &mut out);
        }
        out
    }
}

impl Default for TestHost {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tree walker utilities
// ══════════════════════════════════════════════════════════════════════════════

fn find_by_id<'a>(widgets: &'a [Widget], id: &str) -> Option<&'a Widget> {
    for w in widgets {
        if widget_id(w) == id { return Some(w); }
        if let Some(found) = find_by_id(widget_children(w), id) {
            return Some(found);
        }
    }
    None
}

fn collect_matching<'a, F: Fn(&Widget) -> bool>(
    widgets: &'a [Widget], predicate: &F, results: &mut Vec<&'a Widget>
) {
    for w in widgets {
        if predicate(w) { results.push(w); }
        collect_matching(widget_children(w), predicate, results);
    }
}

fn count_widgets(widgets: &[Widget]) -> usize {
    let mut count = 0;
    for w in widgets {
        count += 1;
        count += count_widgets(widget_children(w));
    }
    count
}

fn set_input_value(widgets: &mut [Widget], id: &str, new_value: &str) {
    for w in widgets {
        match w {
            Widget::TextInput { id: wid, ref mut value, .. } if wid == id => {
                *value = new_value.to_string();
                return;
            }
            _ => {
                // Recurse into children (mutable)
                match w {
                    Widget::Column { children, .. } |
                    Widget::Row { children, .. } |
                    Widget::Grid { children, .. } |
                    Widget::Card { children, .. } |
                    Widget::ScrollView { children, .. } |
                    Widget::Form { children, .. } |
                    Widget::Dialog { children, .. } |
                    Widget::Tabs { children, .. } => {
                        set_input_value(children, id, new_value);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn widget_id(w: &Widget) -> &str {
    match w {
        Widget::Label { id, .. } | Widget::Button { id, .. } |
        Widget::TextInput { id, .. } | Widget::TextArea { id, .. } |
        Widget::Checkbox { id, .. } | Widget::Select { id, .. } |
        Widget::Slider { id, .. } | Widget::Toggle { id, .. } |
        Widget::Image { id, .. } | Widget::Column { id, .. } |
        Widget::Row { id, .. } | Widget::Grid { id, .. } |
        Widget::Card { id, .. } | Widget::Table { id, .. } |
        Widget::List { id, .. } | Widget::Dialog { id, .. } |
        Widget::Alert { id, .. } | Widget::Badge { id, .. } |
        Widget::Spinner { id, .. } | Widget::ProgressBar { id, .. } |
        Widget::Divider { id, .. } | Widget::Spacer { id, .. } |
        Widget::Tabs { id, .. } | Widget::ScrollView { id, .. } |
        Widget::Form { id, .. } | Widget::Snackbar { id, .. } |
        Widget::Avatar { id, .. } | Widget::Tooltip { id, .. } |
        Widget::Canvas { id, .. } | Widget::Icon { id, .. } |
        Widget::RadioGroup { id, .. } | Widget::DatePicker { id, .. } |
        Widget::ColorPicker { id, .. } | Widget::FileUpload { id, .. } |
        Widget::TreeView { id, .. } | Widget::Pagination { id, .. } |
        Widget::Accordion { id, .. } | Widget::Breadcrumb { id, .. } |
        Widget::Menu { id, .. } | Widget::NavSidebar { id, .. } => id,
    }
}

fn widget_text(w: &Widget) -> Option<String> {
    match w {
        Widget::Label { text, .. } => Some(text.clone()),
        Widget::Button { label, .. } => Some(label.clone()),
        Widget::Badge { text, .. } => Some(text.clone()),
        Widget::Alert { message, .. } => Some(message.clone()),
        _ => None,
    }
}

fn widget_children(w: &Widget) -> &[Widget] {
    match w {
        Widget::Column { children, .. } |
        Widget::Row { children, .. } |
        Widget::Grid { children, .. } |
        Widget::Card { children, .. } |
        Widget::ScrollView { children, .. } |
        Widget::Form { children, .. } |
        Widget::Dialog { children, .. } |
        Widget::Tabs { children, .. } => children,
        _ => &[],
    }
}

fn debug_widget(w: &Widget, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    match w {
        Widget::Label { id, text } => { out.push_str(&format!("{}Label#{}: \"{}\"\n", indent, id, text)); }
        Widget::Button { id, label, .. } => { out.push_str(&format!("{}Button#{}: \"{}\"\n", indent, id, label)); }
        Widget::TextInput { id, value, .. } => { out.push_str(&format!("{}Input#{}: \"{}\"\n", indent, id, value)); }
        Widget::Checkbox { id, checked, .. } => { out.push_str(&format!("{}Checkbox#{}: {}\n", indent, id, checked)); }
        other => {
            out.push_str(&format!("{}{:?}\n", indent, std::mem::discriminant(other)));
            for child in widget_children(other) {
                debug_widget(child, depth + 1, out);
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::killer_ui::patch::*;

    fn w(id: &str) -> WidgetId { id.to_string() }

    #[test]
    fn render_and_query() {
        let host = TestHost::render(vec![
            Widget::Label { id: w("title"), text: "Hello".into() },
            Widget::Button { id: w("btn"), label: "Click".into(), variant: ButtonVariant::Primary, disabled: false },
        ]);
        assert_eq!(host.widget_count(), 2);
        host.assert_exists("title");
        host.assert_text_present("Hello");
    }

    #[test]
    fn query_by_text() {
        let host = TestHost::render(vec![
            Widget::Label { id: w("a"), text: "Foo".into() },
            Widget::Label { id: w("b"), text: "Bar".into() },
        ]);
        let found = host.query_by_text("Bar").unwrap();
        assert_eq!(widget_id(found), "b");
    }

    #[test]
    fn query_buttons() {
        let host = TestHost::render(vec![
            Widget::Button { id: w("b1"), label: "Save".into(), variant: ButtonVariant::Primary, disabled: false },
            Widget::Label { id: w("l1"), text: "Info".into() },
            Widget::Button { id: w("b2"), label: "Cancel".into(), variant: ButtonVariant::Secondary, disabled: true },
        ]);
        let buttons = host.query_buttons();
        assert_eq!(buttons.len(), 2);
    }

    #[test]
    fn nested_query() {
        let host = TestHost::render(vec![
            Widget::Card {
                id: w("card"),
                title: Some("Details".into()),
                children: vec![
                    Widget::Label { id: w("name"), text: "Alice".into() },
                    Widget::Column {
                        id: w("col"),
                        children: vec![
                            Widget::Button { id: w("deep-btn"), label: "Deep".into(), variant: ButtonVariant::Primary, disabled: false },
                        ],
                    },
                ],
                elevated: false,
            },
        ]);
        host.assert_exists("deep-btn");
        host.assert_text_present("Alice");
        assert_eq!(host.widget_count(), 4); // card + label + col + btn
    }

    #[test]
    fn fire_click_records_event() {
        let mut host = TestHost::render(vec![
            Widget::Button { id: w("btn"), label: "Go".into(), variant: ButtonVariant::Primary, disabled: false },
        ]);
        host.fire_click("btn");
        assert_eq!(host.events_fired.len(), 1);
        assert_eq!(host.events_fired[0].event_type, "click");
        assert_eq!(host.events_fired[0].target_id, "btn");
    }

    #[test]
    fn fire_input_updates_value() {
        let mut host = TestHost::render(vec![
            Widget::TextInput {
                id: w("name"),
                label: "Name".into(),
                value: "".into(),
                placeholder: "Enter name".into(),
                input_type: InputType::Text,
            },
        ]);
        host.fire_input("name", "Bob");
        host.assert_input_value("name", "Bob");
    }

    #[test]
    fn assert_disabled_enabled() {
        let host = TestHost::render(vec![
            Widget::Button { id: w("dis"), label: "No".into(), variant: ButtonVariant::Danger, disabled: true },
            Widget::Button { id: w("en"), label: "Yes".into(), variant: ButtonVariant::Primary, disabled: false },
        ]);
        host.assert_disabled("dis");
        host.assert_enabled("en");
    }

    #[test]
    fn assert_checked() {
        let host = TestHost::render(vec![
            Widget::Checkbox { id: w("cb"), label: "Accept".into(), checked: true },
        ]);
        host.assert_checked("cb");
    }

    #[test]
    fn debug_tree_output() {
        let host = TestHost::render(vec![
            Widget::Label { id: w("l1"), text: "Hello".into() },
            Widget::Button { id: w("b1"), label: "Click".into(), variant: ButtonVariant::Primary, disabled: false },
        ]);
        let tree = host.debug_tree();
        assert!(tree.contains("Label#l1: \"Hello\""));
        assert!(tree.contains("Button#b1: \"Click\""));
    }

    #[test]
    fn text_absent() {
        let host = TestHost::render(vec![
            Widget::Label { id: w("a"), text: "Present".into() },
        ]);
        host.assert_text_present("Present");
        host.assert_text_absent("Missing");
    }

    #[test]
    fn empty_host() {
        let host = TestHost::new();
        assert_eq!(host.widget_count(), 0);
        assert!(host.query_by_id("anything").is_none());
    }
}
