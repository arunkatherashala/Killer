//! **DOM Bridge** — Abstract browser DOM rendering layer.
//!
//! Provides `DomNode`, `DomRenderer`, `DomEvent` abstraction that can target:
//! - Real browser DOM (via WASM/JS interop)
//! - Headless DOM (for SSR testing)
//! - Native rendering (maps to framebuffer)
//!
//! Widget → DomNode tree conversion, virtual DOM diffing to DOM patches,
//! event delegation, attribute/property management.

use std::collections::HashMap;

use super::patch::Widget;

// ══════════════════════════════════════════════════════════════════════════════
// DOM Node abstraction
// ══════════════════════════════════════════════════════════════════════════════

/// Unique handle for a DOM node.
pub type DomHandle = u64;

/// Represents a node in the abstract DOM tree.
#[derive(Debug, Clone)]
pub enum DomNode {
    Element {
        handle: DomHandle,
        tag: String,
        attributes: HashMap<String, String>,
        styles: HashMap<String, String>,
        classes: Vec<String>,
        children: Vec<DomNode>,
        event_listeners: Vec<DomEventListener>,
    },
    Text {
        handle: DomHandle,
        content: String,
    },
    Fragment {
        children: Vec<DomNode>,
    },
    Comment {
        content: String,
    },
}

impl DomNode {
    pub fn element(tag: &str) -> DomNodeBuilder {
        DomNodeBuilder {
            tag: tag.into(),
            attributes: HashMap::new(),
            styles: HashMap::new(),
            classes: Vec::new(),
            children: Vec::new(),
            listeners: Vec::new(),
        }
    }

    pub fn text(content: &str) -> DomNode {
        DomNode::Text { handle: 0, content: content.into() }
    }

    pub fn handle(&self) -> DomHandle {
        match self {
            DomNode::Element { handle, .. } => *handle,
            DomNode::Text { handle, .. } => *handle,
            DomNode::Fragment { .. } => 0,
            DomNode::Comment { .. } => 0,
        }
    }

    pub fn child_count(&self) -> usize {
        match self {
            DomNode::Element { children, .. } => children.len(),
            DomNode::Fragment { children } => children.len(),
            _ => 0,
        }
    }
}

/// Builder for constructing DOM elements.
pub struct DomNodeBuilder {
    tag: String,
    attributes: HashMap<String, String>,
    styles: HashMap<String, String>,
    classes: Vec<String>,
    children: Vec<DomNode>,
    listeners: Vec<DomEventListener>,
}

impl DomNodeBuilder {
    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn style(mut self, prop: &str, value: &str) -> Self {
        self.styles.insert(prop.into(), value.into());
        self
    }

    pub fn class(mut self, name: &str) -> Self {
        self.classes.push(name.into());
        self
    }

    pub fn child(mut self, node: DomNode) -> Self {
        self.children.push(node);
        self
    }

    pub fn children(mut self, nodes: Vec<DomNode>) -> Self {
        self.children.extend(nodes);
        self
    }

    pub fn on(mut self, event: &str, handler_id: &str) -> Self {
        self.listeners.push(DomEventListener {
            event_type: event.into(),
            handler_id: handler_id.into(),
            capture: false,
            passive: false,
        });
        self
    }

    pub fn build(self, handle: DomHandle) -> DomNode {
        DomNode::Element {
            handle,
            tag: self.tag,
            attributes: self.attributes,
            styles: self.styles,
            classes: self.classes,
            children: self.children,
            event_listeners: self.listeners,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// DOM Events
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct DomEventListener {
    pub event_type: String,
    pub handler_id: String,
    pub capture: bool,
    pub passive: bool,
}

/// Abstract DOM event.
#[derive(Debug, Clone)]
pub struct DomEvent {
    pub event_type: String,
    pub target_handle: DomHandle,
    pub current_handle: DomHandle,
    pub prevented: bool,
    pub stopped: bool,
    pub data: DomEventData,
}

/// Event payload variants.
#[derive(Debug, Clone)]
pub enum DomEventData {
    Click { x: f64, y: f64, button: u8 },
    Input { value: String },
    Key { key: String, code: String, ctrl: bool, shift: bool, alt: bool },
    Focus,
    Blur,
    Submit,
    Scroll { x: f64, y: f64 },
    Resize { width: f64, height: f64 },
    Custom(HashMap<String, String>),
}

// ══════════════════════════════════════════════════════════════════════════════
// DOM Patch operations (diff results)
// ══════════════════════════════════════════════════════════════════════════════

/// Patch operations to apply to the DOM.
#[derive(Debug, Clone)]
pub enum DomPatch {
    CreateElement { parent: DomHandle, tag: String, handle: DomHandle },
    CreateText { parent: DomHandle, content: String, handle: DomHandle },
    RemoveNode { handle: DomHandle },
    SetAttribute { handle: DomHandle, key: String, value: String },
    RemoveAttribute { handle: DomHandle, key: String },
    SetStyle { handle: DomHandle, prop: String, value: String },
    SetText { handle: DomHandle, content: String },
    AddClass { handle: DomHandle, class: String },
    RemoveClass { handle: DomHandle, class: String },
    InsertBefore { parent: DomHandle, child: DomHandle, before: DomHandle },
    ReplaceNode { old: DomHandle, new_node: DomNode },
    AddEventListener { handle: DomHandle, listener: DomEventListener },
    RemoveEventListener { handle: DomHandle, event_type: String },
}

// ══════════════════════════════════════════════════════════════════════════════
// DOM Renderer trait
// ══════════════════════════════════════════════════════════════════════════════

/// Abstract renderer interface — implemented by each backend.
pub trait DomRenderer {
    fn create_element(&mut self, tag: &str) -> DomHandle;
    fn create_text(&mut self, content: &str) -> DomHandle;
    fn set_attribute(&mut self, handle: DomHandle, key: &str, value: &str);
    fn remove_attribute(&mut self, handle: DomHandle, key: &str);
    fn set_style(&mut self, handle: DomHandle, prop: &str, value: &str);
    fn set_text(&mut self, handle: DomHandle, content: &str);
    fn append_child(&mut self, parent: DomHandle, child: DomHandle);
    fn remove_child(&mut self, parent: DomHandle, child: DomHandle);
    fn insert_before(&mut self, parent: DomHandle, child: DomHandle, before: DomHandle);
    fn add_event_listener(&mut self, handle: DomHandle, event: &str, handler_id: &str);
    fn remove_event_listener(&mut self, handle: DomHandle, event: &str);
    fn apply_patches(&mut self, patches: &[DomPatch]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Headless DOM — for testing/SSR
// ══════════════════════════════════════════════════════════════════════════════

/// In-memory DOM implementation for testing and SSR.
pub struct HeadlessDom {
    nodes: HashMap<DomHandle, HeadlessDomNode>,
    next_handle: DomHandle,
    root: DomHandle,
}

struct HeadlessDomNode {
    tag: Option<String>,
    text: Option<String>,
    attributes: HashMap<String, String>,
    styles: HashMap<String, String>,
    classes: Vec<String>,
    children: Vec<DomHandle>,
    listeners: Vec<String>,
}

impl HeadlessDom {
    pub fn new() -> Self {
        let mut dom = HeadlessDom {
            nodes: HashMap::new(),
            next_handle: 1,
            root: 0,
        };
        dom.nodes.insert(0, HeadlessDomNode {
            tag: Some("body".into()),
            text: None,
            attributes: HashMap::new(),
            styles: HashMap::new(),
            classes: Vec::new(),
            children: Vec::new(),
            listeners: Vec::new(),
        });
        dom
    }

    pub fn root(&self) -> DomHandle { self.root }

    pub fn node_count(&self) -> usize { self.nodes.len() }

    pub fn get_tag(&self, handle: DomHandle) -> Option<&str> {
        self.nodes.get(&handle).and_then(|n| n.tag.as_deref())
    }

    pub fn get_text(&self, handle: DomHandle) -> Option<&str> {
        self.nodes.get(&handle).and_then(|n| n.text.as_deref())
    }

    pub fn get_attribute(&self, handle: DomHandle, key: &str) -> Option<&str> {
        self.nodes.get(&handle)?.attributes.get(key).map(|s| s.as_str())
    }

    pub fn get_children(&self, handle: DomHandle) -> Vec<DomHandle> {
        self.nodes.get(&handle).map(|n| n.children.clone()).unwrap_or_default()
    }

    /// Render to HTML string (for SSR).
    pub fn to_html(&self, handle: DomHandle) -> String {
        let node = match self.nodes.get(&handle) {
            Some(n) => n,
            None => return String::new(),
        };
        if let Some(ref text) = node.text {
            return html_escape(text);
        }
        let tag = node.tag.as_deref().unwrap_or("div");
        let mut html = format!("<{}", tag);
        for (k, v) in &node.attributes {
            html.push_str(&format!(" {}=\"{}\"", k, html_escape(v)));
        }
        if !node.classes.is_empty() {
            html.push_str(&format!(" class=\"{}\"", node.classes.join(" ")));
        }
        if !node.styles.is_empty() {
            let style_str: Vec<String> = node.styles.iter()
                .map(|(k, v)| format!("{}:{}", k, v)).collect();
            html.push_str(&format!(" style=\"{}\"", style_str.join(";")));
        }
        html.push('>');
        for child in &node.children {
            html.push_str(&self.to_html(*child));
        }
        html.push_str(&format!("</{}>", tag));
        html
    }

    fn alloc_handle(&mut self) -> DomHandle {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl Default for HeadlessDom {
    fn default() -> Self { Self::new() }
}

impl DomRenderer for HeadlessDom {
    fn create_element(&mut self, tag: &str) -> DomHandle {
        let h = self.alloc_handle();
        self.nodes.insert(h, HeadlessDomNode {
            tag: Some(tag.into()), text: None,
            attributes: HashMap::new(), styles: HashMap::new(),
            classes: Vec::new(), children: Vec::new(), listeners: Vec::new(),
        });
        h
    }

    fn create_text(&mut self, content: &str) -> DomHandle {
        let h = self.alloc_handle();
        self.nodes.insert(h, HeadlessDomNode {
            tag: None, text: Some(content.into()),
            attributes: HashMap::new(), styles: HashMap::new(),
            classes: Vec::new(), children: Vec::new(), listeners: Vec::new(),
        });
        h
    }

    fn set_attribute(&mut self, handle: DomHandle, key: &str, value: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.attributes.insert(key.into(), value.into());
        }
    }

    fn remove_attribute(&mut self, handle: DomHandle, key: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.attributes.remove(key);
        }
    }

    fn set_style(&mut self, handle: DomHandle, prop: &str, value: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.styles.insert(prop.into(), value.into());
        }
    }

    fn set_text(&mut self, handle: DomHandle, content: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.text = Some(content.into());
        }
    }

    fn append_child(&mut self, parent: DomHandle, child: DomHandle) {
        if let Some(n) = self.nodes.get_mut(&parent) {
            n.children.push(child);
        }
    }

    fn remove_child(&mut self, parent: DomHandle, child: DomHandle) {
        if let Some(n) = self.nodes.get_mut(&parent) {
            n.children.retain(|&c| c != child);
        }
    }

    fn insert_before(&mut self, parent: DomHandle, child: DomHandle, before: DomHandle) {
        if let Some(n) = self.nodes.get_mut(&parent) {
            if let Some(pos) = n.children.iter().position(|&c| c == before) {
                n.children.insert(pos, child);
            } else {
                n.children.push(child);
            }
        }
    }

    fn add_event_listener(&mut self, handle: DomHandle, event: &str, _handler_id: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.listeners.push(event.into());
        }
    }

    fn remove_event_listener(&mut self, handle: DomHandle, event: &str) {
        if let Some(n) = self.nodes.get_mut(&handle) {
            n.listeners.retain(|e| e != event);
        }
    }

    fn apply_patches(&mut self, patches: &[DomPatch]) {
        for patch in patches {
            match patch {
                DomPatch::CreateElement { parent, tag, handle } => {
                    self.nodes.insert(*handle, HeadlessDomNode {
                        tag: Some(tag.clone()), text: None,
                        attributes: HashMap::new(), styles: HashMap::new(),
                        classes: Vec::new(), children: Vec::new(), listeners: Vec::new(),
                    });
                    self.append_child(*parent, *handle);
                }
                DomPatch::CreateText { parent, content, handle } => {
                    self.nodes.insert(*handle, HeadlessDomNode {
                        tag: None, text: Some(content.clone()),
                        attributes: HashMap::new(), styles: HashMap::new(),
                        classes: Vec::new(), children: Vec::new(), listeners: Vec::new(),
                    });
                    self.append_child(*parent, *handle);
                }
                DomPatch::RemoveNode { handle } => { self.nodes.remove(handle); }
                DomPatch::SetAttribute { handle, key, value } => { self.set_attribute(*handle, key, value); }
                DomPatch::RemoveAttribute { handle, key } => { self.remove_attribute(*handle, key); }
                DomPatch::SetStyle { handle, prop, value } => { self.set_style(*handle, prop, value); }
                DomPatch::SetText { handle, content } => { self.set_text(*handle, content); }
                DomPatch::AddClass { handle, class } => {
                    if let Some(n) = self.nodes.get_mut(handle) { n.classes.push(class.clone()); }
                }
                DomPatch::RemoveClass { handle, class } => {
                    if let Some(n) = self.nodes.get_mut(handle) { n.classes.retain(|c| c != class); }
                }
                DomPatch::InsertBefore { parent, child, before } => { self.insert_before(*parent, *child, *before); }
                DomPatch::ReplaceNode { old, new_node: _ } => { self.nodes.remove(old); }
                DomPatch::AddEventListener { handle, listener } => {
                    self.add_event_listener(*handle, &listener.event_type, &listener.handler_id);
                }
                DomPatch::RemoveEventListener { handle, event_type } => {
                    self.remove_event_listener(*handle, event_type);
                }
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Widget → DomNode converter
// ══════════════════════════════════════════════════════════════════════════════

/// Convert a Killer Widget tree to an abstract DomNode tree.
pub fn widget_to_dom(widget: &Widget, handle_counter: &mut DomHandle) -> DomNode {
    let alloc = |ctr: &mut DomHandle| -> DomHandle { let h = *ctr; *ctr += 1; h };

    match widget {
        Widget::Label { text, id, .. } => {
            DomNode::element("span")
                .attr("id", id)
                .attr("data-widget", "label")
                .child(DomNode::text(text))
                .build(alloc(handle_counter))
        }
        Widget::Button { label, id, disabled, variant, .. } => {
            let cls = match variant {
                super::patch::ButtonVariant::Primary => "btn-primary",
                super::patch::ButtonVariant::Secondary => "btn-secondary",
                super::patch::ButtonVariant::Outline => "btn-outline",
                super::patch::ButtonVariant::Text => "btn-text",
                super::patch::ButtonVariant::Danger => "btn-danger",
            };
            let mut b = DomNode::element("button")
                .attr("id", id)
                .class(cls)
                .on("click", id)
                .child(DomNode::text(label));
            if *disabled { b = b.attr("disabled", "true"); }
            b.build(alloc(handle_counter))
        }
        Widget::TextInput { id, label, value, placeholder, .. } => {
            DomNode::element("div")
                .attr("id", &format!("{}_wrap", id))
                .child(DomNode::element("label").child(DomNode::text(label)).build(alloc(handle_counter)))
                .child(DomNode::element("input")
                    .attr("id", id)
                    .attr("value", value)
                    .attr("placeholder", placeholder)
                    .on("input", id)
                    .build(alloc(handle_counter)))
                .build(alloc(handle_counter))
        }
        Widget::Column { id, children, .. } => {
            let child_nodes: Vec<DomNode> = children.iter()
                .map(|c| widget_to_dom(c, handle_counter)).collect();
            DomNode::element("div")
                .attr("id", id)
                .class("flex-col")
                .children(child_nodes)
                .build(alloc(handle_counter))
        }
        Widget::Row { id, children, gap, .. } => {
            let child_nodes: Vec<DomNode> = children.iter()
                .map(|c| widget_to_dom(c, handle_counter)).collect();
            DomNode::element("div")
                .attr("id", id)
                .class("flex-row")
                .style("gap", &format!("{}px", gap))
                .children(child_nodes)
                .build(alloc(handle_counter))
        }
        Widget::Card { id, title, children, elevated, .. } => {
            let mut card = DomNode::element("div")
                .attr("id", id)
                .class("card");
            if *elevated { card = card.class("card-elevated"); }
            if let Some(t) = title {
                card = card.child(DomNode::element("h3").child(DomNode::text(t)).build(alloc(handle_counter)));
            }
            for child in children {
                card = card.child(widget_to_dom(child, handle_counter));
            }
            card.build(alloc(handle_counter))
        }
        Widget::Alert { id, message, severity, .. } => {
            let sev_class = match severity {
                super::patch::Severity::Info => "alert-info",
                super::patch::Severity::Success => "alert-success",
                super::patch::Severity::Warning => "alert-warning",
                super::patch::Severity::Error => "alert-error",
            };
            DomNode::element("div")
                .attr("id", id)
                .attr("role", "alert")
                .class(sev_class)
                .child(DomNode::text(message))
                .build(alloc(handle_counter))
        }
        // Default passthrough for other widget types
        _ => {
            DomNode::element("div")
                .attr("data-widget", "unknown")
                .build(alloc(handle_counter))
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_dom_create_element() {
        let mut dom = HeadlessDom::new();
        let div = dom.create_element("div");
        dom.set_attribute(div, "id", "test");
        dom.append_child(dom.root(), div);
        assert_eq!(dom.get_tag(div), Some("div"));
        assert_eq!(dom.get_attribute(div, "id"), Some("test"));
        assert_eq!(dom.get_children(dom.root()).len(), 1);
    }

    #[test]
    fn headless_dom_text_node() {
        let mut dom = HeadlessDom::new();
        let txt = dom.create_text("Hello World");
        dom.append_child(dom.root(), txt);
        assert_eq!(dom.get_text(txt), Some("Hello World"));
    }

    #[test]
    fn headless_dom_to_html() {
        let mut dom = HeadlessDom::new();
        let div = dom.create_element("div");
        dom.set_attribute(div, "class", "test");
        let txt = dom.create_text("Hello");
        dom.append_child(div, txt);
        dom.append_child(dom.root(), div);
        let html = dom.to_html(div);
        assert!(html.contains("<div"));
        assert!(html.contains("class=\"test\""));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn headless_dom_apply_patches() {
        let mut dom = HeadlessDom::new();
        dom.apply_patches(&[
            DomPatch::CreateElement { parent: 0, tag: "p".into(), handle: 100 },
            DomPatch::SetAttribute { handle: 100, key: "id".into(), value: "para".into() },
            DomPatch::CreateText { parent: 100, content: "Text".into(), handle: 101 },
        ]);
        assert_eq!(dom.get_tag(100), Some("p"));
        assert_eq!(dom.get_text(101), Some("Text"));
        assert_eq!(dom.get_children(0).len(), 1);
    }

    #[test]
    fn dom_node_builder() {
        let node = DomNode::element("button")
            .attr("type", "submit")
            .class("btn")
            .class("btn-primary")
            .style("color", "red")
            .on("click", "handler1")
            .child(DomNode::text("Click"))
            .build(42);
        if let DomNode::Element { tag, attributes, classes, children, event_listeners, .. } = &node {
            assert_eq!(tag, "button");
            assert_eq!(attributes.get("type").unwrap(), "submit");
            assert_eq!(classes.len(), 2);
            assert_eq!(children.len(), 1);
            assert_eq!(event_listeners.len(), 1);
        } else { panic!("Expected Element"); }
    }

    #[test]
    fn widget_to_dom_label() {
        let w = Widget::Label { id: "lbl".into(), text: "Hello".into() };
        let mut ctr = 1;
        let node = widget_to_dom(&w, &mut ctr);
        if let DomNode::Element { tag, children, .. } = &node {
            assert_eq!(tag, "span");
            assert_eq!(children.len(), 1);
        } else { panic!("Expected Element"); }
    }

    #[test]
    fn widget_to_dom_button() {
        let w = Widget::Button {
            id: "btn".into(), label: "Click".into(),
            variant: super::super::patch::ButtonVariant::Primary, disabled: false,
        };
        let mut ctr = 1;
        let node = widget_to_dom(&w, &mut ctr);
        if let DomNode::Element { tag, classes, .. } = &node {
            assert_eq!(tag, "button");
            assert!(classes.contains(&"btn-primary".to_string()));
        } else { panic!("Expected Element"); }
    }

    #[test]
    fn widget_to_dom_column() {
        let w = Widget::Column {
            id: "col".into(),
            children: vec![
                Widget::Label { id: "a".into(), text: "A".into() },
                Widget::Label { id: "b".into(), text: "B".into() },
            ],
        };
        let mut ctr = 1;
        let node = widget_to_dom(&w, &mut ctr);
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn headless_dom_remove() {
        let mut dom = HeadlessDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div);
        assert_eq!(dom.get_children(dom.root()).len(), 1);
        dom.remove_child(dom.root(), div);
        assert_eq!(dom.get_children(dom.root()).len(), 0);
    }

    #[test]
    fn headless_dom_styles() {
        let mut dom = HeadlessDom::new();
        let div = dom.create_element("div");
        dom.set_style(div, "color", "red");
        dom.set_style(div, "font-size", "16px");
        dom.append_child(dom.root(), div);
        let html = dom.to_html(div);
        assert!(html.contains("style="));
    }

    #[test]
    fn dom_event_data() {
        let evt = DomEvent {
            event_type: "click".into(),
            target_handle: 42,
            current_handle: 42,
            prevented: false,
            stopped: false,
            data: DomEventData::Click { x: 100.0, y: 200.0, button: 0 },
        };
        assert_eq!(evt.event_type, "click");
        if let DomEventData::Click { x, y, .. } = &evt.data {
            assert_eq!(*x, 100.0);
            assert_eq!(*y, 200.0);
        }
    }
}
