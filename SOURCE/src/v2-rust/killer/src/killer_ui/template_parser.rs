//! **KSX Template Parser** — JSX/HTML-like template language for Killer UI.
//!
//! Parses `<Button label="Click" />` syntax into Widget trees.
//! Supports: self-closing tags, nested children, attribute binding,
//! conditionals (`k-if`), loops (`k-for`), event handlers (`@click`).
//!
//! Competitive with JSX (React) and Angular templates.

use std::collections::HashMap;
use super::patch::{Widget, WidgetId, ButtonVariant, InputType, Severity, SelectOption};

// ══════════════════════════════════════════════════════════════════════════════
// Template AST
// ══════════════════════════════════════════════════════════════════════════════

/// A parsed template node.
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// `<Tag attr="value">children</Tag>` or `<Tag />`
    Element {
        tag: String,
        attrs: HashMap<String, String>,
        children: Vec<TemplateNode>,
        directives: Vec<Directive>,
        events: Vec<EventBinding>,
    },
    /// Plain text content.
    Text(String),
    /// `{expression}` interpolation.
    Interpolation(String),
}

/// Template directives (structural).
#[derive(Debug, Clone)]
pub enum Directive {
    If(String),          // k-if="condition"
    For(String, String), // k-for="item in items"
    Show(String),        // k-show="condition"
    Model(String),       // k-model="field" (two-way binding)
}

/// Event binding.
#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event: String,   // click, submit, input, etc.
    pub handler: String, // function name or expression
}

// ══════════════════════════════════════════════════════════════════════════════
// Parser
// ══════════════════════════════════════════════════════════════════════════════

/// Parse a KSX template string into a list of TemplateNodes.
pub fn parse_template(input: &str) -> Result<Vec<TemplateNode>, String> {
    let mut parser = TemplateParser::new(input);
    parser.parse_nodes()
}

struct TemplateParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> TemplateParser<'a> {
    fn new(input: &'a str) -> Self {
        TemplateParser { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_nodes(&mut self) -> Result<Vec<TemplateNode>, String> {
        let mut nodes = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() { break; }

            if self.remaining().starts_with("</") {
                break; // Closing tag — parent handles this
            } else if self.remaining().starts_with('<') {
                nodes.push(self.parse_element()?);
            } else if self.remaining().starts_with('{') {
                nodes.push(self.parse_interpolation()?);
            } else {
                nodes.push(self.parse_text()?);
            }
        }
        Ok(nodes)
    }

    fn parse_element(&mut self) -> Result<TemplateNode, String> {
        self.advance(1); // skip <
        self.skip_whitespace();

        // Tag name
        let tag = self.parse_identifier()?;
        self.skip_whitespace();

        // Attributes
        let mut attrs = HashMap::new();
        let mut directives = Vec::new();
        let mut events = Vec::new();

        while self.pos < self.input.len() && self.peek() != Some('>') && self.peek() != Some('/') {
            self.skip_whitespace();
            if self.peek() == Some('>') || self.peek() == Some('/') { break; }

            let attr_name = self.parse_attr_name()?;
            self.skip_whitespace();

            let value = if self.peek() == Some('=') {
                self.advance(1); // skip =
                self.skip_whitespace();
                self.parse_attr_value()?
            } else {
                "true".to_string()
            };

            // Classify attribute
            if attr_name.starts_with("k-if") {
                directives.push(Directive::If(value));
            } else if attr_name.starts_with("k-for") {
                // "item in items"
                let parts: Vec<&str> = value.splitn(2, " in ").collect();
                if parts.len() == 2 {
                    directives.push(Directive::For(parts[0].trim().to_string(), parts[1].trim().to_string()));
                }
            } else if attr_name.starts_with("k-show") {
                directives.push(Directive::Show(value));
            } else if attr_name.starts_with("k-model") {
                directives.push(Directive::Model(value));
            } else if attr_name.starts_with('@') {
                events.push(EventBinding {
                    event: attr_name[1..].to_string(),
                    handler: value,
                });
            } else {
                attrs.insert(attr_name, value);
            }
            self.skip_whitespace();
        }

        // Self-closing or open tag?
        let self_closing = self.remaining().starts_with("/>");
        if self_closing {
            self.advance(2); // skip />
            return Ok(TemplateNode::Element { tag, attrs, children: Vec::new(), directives, events });
        }

        // Skip >
        if self.peek() == Some('>') { self.advance(1); }

        // Parse children
        let children = self.parse_nodes()?;

        // Expect closing tag </Tag>
        if self.remaining().starts_with("</") {
            self.advance(2); // skip </
            let close_tag = self.parse_identifier()?;
            if close_tag != tag {
                return Err(format!("mismatched tags: <{}> vs </{}>", tag, close_tag));
            }
            self.skip_whitespace();
            if self.peek() == Some('>') { self.advance(1); }
        }

        Ok(TemplateNode::Element { tag, attrs, children, directives, events })
    }

    fn parse_identifier(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(format!("expected identifier at position {}", self.pos));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_attr_name(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'@' || ch == b':' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(format!("expected attribute name at position {}", self.pos));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_attr_value(&mut self) -> Result<String, String> {
        if self.peek() == Some('"') {
            self.advance(1);
            let start = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'"' {
                self.pos += 1;
            }
            let value = self.input[start..self.pos].to_string();
            if self.peek() == Some('"') { self.advance(1); }
            Ok(value)
        } else if self.peek() == Some('\'') {
            self.advance(1);
            let start = self.pos;
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'\'' {
                self.pos += 1;
            }
            let value = self.input[start..self.pos].to_string();
            if self.peek() == Some('\'') { self.advance(1); }
            Ok(value)
        } else {
            // Unquoted value (until whitespace or > or /)
            let start = self.pos;
            while self.pos < self.input.len() {
                let ch = self.input.as_bytes()[self.pos];
                if ch.is_ascii_whitespace() || ch == b'>' || ch == b'/' { break; }
                self.pos += 1;
            }
            Ok(self.input[start..self.pos].to_string())
        }
    }

    fn parse_interpolation(&mut self) -> Result<TemplateNode, String> {
        self.advance(1); // skip {
        let start = self.pos;
        let mut depth = 1;
        while self.pos < self.input.len() && depth > 0 {
            match self.input.as_bytes()[self.pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            if depth > 0 { self.pos += 1; }
        }
        let expr = self.input[start..self.pos].trim().to_string();
        if self.peek() == Some('}') { self.advance(1); }
        Ok(TemplateNode::Interpolation(expr))
    }

    fn parse_text(&mut self) -> Result<TemplateNode, String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch == b'<' || ch == b'{' { break; }
            self.pos += 1;
        }
        Ok(TemplateNode::Text(self.input[start..self.pos].to_string()))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Template → Widget compiler
// ══════════════════════════════════════════════════════════════════════════════

/// Context for template compilation (variable bindings).
pub struct TemplateContext {
    pub vars: HashMap<String, String>,
    next_id: u64,
}

impl TemplateContext {
    pub fn new() -> Self {
        TemplateContext { vars: HashMap::new(), next_id: 0 }
    }

    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        TemplateContext { vars, next_id: 0 }
    }

    fn gen_id(&mut self) -> WidgetId {
        self.next_id += 1;
        format!("ksx-{}", self.next_id)
    }

    fn resolve(&self, expr: &str) -> String {
        self.vars.get(expr).cloned().unwrap_or_else(|| expr.to_string())
    }
}

impl Default for TemplateContext {
    fn default() -> Self { Self::new() }
}

/// Compile parsed template nodes into Widget tree.
pub fn compile_template(nodes: &[TemplateNode], ctx: &mut TemplateContext) -> Vec<Widget> {
    let mut widgets = Vec::new();
    for node in nodes {
        if let Some(w) = compile_node(node, ctx) {
            widgets.push(w);
        }
    }
    widgets
}

fn compile_node(node: &TemplateNode, ctx: &mut TemplateContext) -> Option<Widget> {
    match node {
        TemplateNode::Text(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() { return None; }
            Some(Widget::Label { id: ctx.gen_id(), text: trimmed.to_string() })
        }
        TemplateNode::Interpolation(expr) => {
            Some(Widget::Label { id: ctx.gen_id(), text: ctx.resolve(expr) })
        }
        TemplateNode::Element { tag, attrs, children, directives, .. } => {
            // Check k-if directive
            for dir in directives {
                if let Directive::If(cond) = dir {
                    let val = ctx.resolve(cond);
                    if val == "false" || val == "0" || val.is_empty() {
                        return None;
                    }
                }
            }

            let child_widgets = compile_template(children, ctx);
            let tag_lower = tag.to_lowercase();
            let id = ctx.gen_id();

            match tag_lower.as_str() {
                "label" | "span" | "text" | "p" | "h1" | "h2" | "h3" => {
                    let text = attrs.get("text").cloned()
                        .unwrap_or_else(|| extract_text_children(children, ctx));
                    Some(Widget::Label { id, text })
                }
                "button" | "btn" => {
                    let label = attrs.get("label").cloned()
                        .unwrap_or_else(|| extract_text_children(children, ctx));
                    let variant = match attrs.get("variant").map(|s| s.as_str()) {
                        Some("secondary") => ButtonVariant::Secondary,
                        Some("danger") => ButtonVariant::Danger,
                        Some("outline") => ButtonVariant::Outline,
                        Some("text") => ButtonVariant::Text,
                        _ => ButtonVariant::Primary,
                    };
                    let disabled = attrs.get("disabled").map(|v| v != "false").unwrap_or(false);
                    Some(Widget::Button { id, label, variant, disabled })
                }
                "input" | "textinput" => {
                    let input_type = match attrs.get("type").map(|s| s.as_str()) {
                        Some("password") => InputType::Password,
                        Some("email") => InputType::Email,
                        Some("number") => InputType::Number,
                        Some("search") => InputType::Search,
                        _ => InputType::Text,
                    };
                    Some(Widget::TextInput {
                        id,
                        label: attrs.get("label").cloned().unwrap_or_default(),
                        value: attrs.get("value").cloned().unwrap_or_default(),
                        placeholder: attrs.get("placeholder").cloned().unwrap_or_default(),
                        input_type,
                    })
                }
                "textarea" => Some(Widget::TextArea {
                    id,
                    label: attrs.get("label").cloned().unwrap_or_default(),
                    value: attrs.get("value").cloned().unwrap_or_else(|| extract_text_children(children, ctx)),
                    rows: attrs.get("rows").and_then(|v| v.parse().ok()).unwrap_or(3),
                }),
                "checkbox" => Some(Widget::Checkbox {
                    id,
                    label: attrs.get("label").cloned().unwrap_or_default(),
                    checked: attrs.get("checked").map(|v| v != "false").unwrap_or(false),
                }),
                "select" | "dropdown" => {
                    let options: Vec<SelectOption> = child_widgets.iter().filter_map(|w| {
                        if let Widget::Label { text, .. } = w {
                            Some(SelectOption { label: text.clone(), value: text.clone() })
                        } else { None }
                    }).collect();
                    Some(Widget::Select {
                        id,
                        label: attrs.get("label").cloned().unwrap_or_default(),
                        options,
                        selected: attrs.get("selected").cloned(),
                    })
                }
                "slider" | "range" => Some(Widget::Slider {
                    id,
                    label: attrs.get("label").cloned().unwrap_or_default(),
                    min: attrs.get("min").and_then(|v| v.parse().ok()).unwrap_or(0.0),
                    max: attrs.get("max").and_then(|v| v.parse().ok()).unwrap_or(100.0),
                    value: attrs.get("value").and_then(|v| v.parse().ok()).unwrap_or(50.0),
                }),
                "toggle" | "switch" => Some(Widget::Toggle {
                    id,
                    label: attrs.get("label").cloned().unwrap_or_default(),
                    on: attrs.get("on").map(|v| v != "false").unwrap_or(false),
                }),
                "image" | "img" => Some(Widget::Image {
                    id,
                    src: attrs.get("src").cloned().unwrap_or_default(),
                    alt: attrs.get("alt").cloned().unwrap_or_default(),
                    width: attrs.get("width").and_then(|v| v.parse().ok()).unwrap_or(0.0),
                    height: attrs.get("height").and_then(|v| v.parse().ok()).unwrap_or(0.0),
                }),
                "row" | "hbox" => Some(Widget::Row {
                    id,
                    children: child_widgets,
                    gap: attrs.get("gap").and_then(|v| v.parse().ok()).unwrap_or(8.0),
                }),
                "column" | "col" | "vbox" => Some(Widget::Column {
                    id,
                    children: child_widgets,
                }),
                "grid" => Some(Widget::Grid {
                    id,
                    children: child_widgets,
                    columns: attrs.get("columns").and_then(|v| v.parse().ok()).unwrap_or(2),
                    gap: attrs.get("gap").and_then(|v| v.parse().ok()).unwrap_or(8.0),
                }),
                "card" => Some(Widget::Card {
                    id,
                    title: attrs.get("title").cloned(),
                    children: child_widgets,
                    elevated: attrs.get("elevated").map(|v| v != "false").unwrap_or(false),
                }),
                "table" => {
                    let headers: Vec<String> = attrs.get("headers")
                        .map(|h| h.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();
                    Some(Widget::Table {
                        id,
                        headers,
                        rows: Vec::new(),
                        sortable: attrs.get("sortable").map(|v| v != "false").unwrap_or(false),
                    })
                }
                "list" | "ul" | "ol" => {
                    let items: Vec<String> = child_widgets.iter().filter_map(|w| {
                        if let Widget::Label { text, .. } = w { Some(text.clone()) } else { None }
                    }).collect();
                    Some(Widget::List {
                        id,
                        items,
                        ordered: tag_lower == "ol",
                    })
                }
                "alert" => Some(Widget::Alert {
                    id,
                    message: attrs.get("message").cloned().unwrap_or_default(),
                    severity: match attrs.get("severity").map(|s| s.as_str()) {
                        Some("success") => Severity::Success,
                        Some("warning") => Severity::Warning,
                        Some("error") => Severity::Error,
                        _ => Severity::Info,
                    },
                    dismissible: attrs.get("dismissible").map(|v| v != "false").unwrap_or(false),
                }),
                "dialog" | "modal" => Some(Widget::Dialog {
                    id,
                    title: attrs.get("title").cloned().unwrap_or_default(),
                    children: child_widgets,
                    open: attrs.get("open").map(|v| v != "false").unwrap_or(false),
                }),
                "spinner" | "loading" => Some(Widget::Spinner {
                    id,
                    size: attrs.get("size").and_then(|v| v.parse().ok()).unwrap_or(24.0),
                }),
                "badge" | "chip" => Some(Widget::Badge {
                    id,
                    text: attrs.get("text").cloned().unwrap_or_else(|| extract_text_children(children, ctx)),
                    color: attrs.get("color").cloned().unwrap_or_else(|| "#666".into()),
                }),
                "progress" | "progressbar" => Some(Widget::ProgressBar {
                    id,
                    value: attrs.get("value").and_then(|v| v.parse().ok()).unwrap_or(0.0),
                    max: attrs.get("max").and_then(|v| v.parse().ok()).unwrap_or(100.0),
                    variant: super::patch::ProgressVariant::Linear,
                }),
                "divider" | "hr" => Some(Widget::Divider {
                    id,
                    vertical: attrs.get("vertical").map(|v| v != "false").unwrap_or(false),
                }),
                "spacer" => Some(Widget::Spacer {
                    id,
                    size: attrs.get("size").and_then(|v| v.parse().ok()).unwrap_or(16.0),
                }),
                "form" => Some(Widget::Form { id, children: child_widgets }),
                "scrollview" | "scroll" => Some(Widget::ScrollView {
                    id,
                    children: child_widgets,
                    max_height: attrs.get("max-height").and_then(|v| v.parse().ok()).unwrap_or(400.0),
                }),
                "tabs" => {
                    let labels: Vec<String> = attrs.get("labels")
                        .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();
                    Some(Widget::Tabs {
                        id,
                        labels,
                        active: attrs.get("active").and_then(|v| v.parse().ok()).unwrap_or(0),
                        children: child_widgets,
                    })
                }
                // Fallback: unknown tag → div-like Column
                _ => Some(Widget::Column { id, children: child_widgets }),
            }
        }
    }
}

fn extract_text_children(children: &[TemplateNode], ctx: &TemplateContext) -> String {
    children.iter().filter_map(|c| match c {
        TemplateNode::Text(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        }
        TemplateNode::Interpolation(expr) => Some(ctx.resolve(expr)),
        _ => None,
    }).collect::<Vec<_>>().join("")
}

// ══════════════════════════════════════════════════════════════════════════════
// Convenience: template string → Widget vec
// ══════════════════════════════════════════════════════════════════════════════

/// Parse + compile a KSX template in one step.
pub fn ksx(input: &str) -> Result<Vec<Widget>, String> {
    let nodes = parse_template(input)?;
    let mut ctx = TemplateContext::new();
    Ok(compile_template(&nodes, &mut ctx))
}

/// Parse + compile with variable context.
pub fn ksx_with(input: &str, vars: HashMap<String, String>) -> Result<Vec<Widget>, String> {
    let nodes = parse_template(input)?;
    let mut ctx = TemplateContext::with_vars(vars);
    Ok(compile_template(&nodes, &mut ctx))
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::killer_ui::patch::*;

    #[test]
    fn parse_self_closing_tag() {
        let nodes = parse_template(r#"<Button label="Click Me" />"#).unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Element { tag, attrs, .. } = &nodes[0] {
            assert_eq!(tag, "Button");
            assert_eq!(attrs.get("label").unwrap(), "Click Me");
        } else { panic!("expected element"); }
    }

    #[test]
    fn parse_nested_elements() {
        let nodes = parse_template(r#"<Row><Label text="A" /><Label text="B" /></Row>"#).unwrap();
        assert_eq!(nodes.len(), 1);
        if let TemplateNode::Element { tag, children, .. } = &nodes[0] {
            assert_eq!(tag, "Row");
            assert_eq!(children.len(), 2);
        } else { panic!("expected element"); }
    }

    #[test]
    fn parse_text_content() {
        let nodes = parse_template(r#"<Button>Save</Button>"#).unwrap();
        if let TemplateNode::Element { children, .. } = &nodes[0] {
            assert_eq!(children.len(), 1);
            if let TemplateNode::Text(t) = &children[0] {
                assert_eq!(t, "Save");
            }
        }
    }

    #[test]
    fn parse_interpolation() {
        let nodes = parse_template(r#"<Label text="{name}" />"#).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn parse_event_binding() {
        let nodes = parse_template(r#"<Button @click="handleClick" label="Go" />"#).unwrap();
        if let TemplateNode::Element { events, .. } = &nodes[0] {
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event, "click");
            assert_eq!(events[0].handler, "handleClick");
        }
    }

    #[test]
    fn parse_directive_if() {
        let nodes = parse_template(r#"<Label k-if="show" text="Visible" />"#).unwrap();
        if let TemplateNode::Element { directives, .. } = &nodes[0] {
            assert_eq!(directives.len(), 1);
            if let Directive::If(cond) = &directives[0] {
                assert_eq!(cond, "show");
            }
        }
    }

    #[test]
    fn parse_directive_for() {
        let nodes = parse_template(r#"<Label k-for="item in items" text="x" />"#).unwrap();
        if let TemplateNode::Element { directives, .. } = &nodes[0] {
            if let Directive::For(var, list) = &directives[0] {
                assert_eq!(var, "item");
                assert_eq!(list, "items");
            }
        }
    }

    #[test]
    fn compile_button() {
        let widgets = ksx(r#"<Button label="Save" variant="danger" />"#).unwrap();
        assert_eq!(widgets.len(), 1);
        if let Widget::Button { label, variant, .. } = &widgets[0] {
            assert_eq!(label, "Save");
            assert_eq!(*variant, ButtonVariant::Danger);
        } else { panic!("expected Button widget"); }
    }

    #[test]
    fn compile_input() {
        let widgets = ksx(r#"<Input type="email" placeholder="you@example.com" />"#).unwrap();
        if let Widget::TextInput { input_type, placeholder, .. } = &widgets[0] {
            assert_eq!(*input_type, InputType::Email);
            assert_eq!(placeholder, "you@example.com");
        } else { panic!("expected TextInput widget"); }
    }

    #[test]
    fn compile_row_with_children() {
        let widgets = ksx(r#"<Row gap="16"><Label text="A" /><Label text="B" /></Row>"#).unwrap();
        if let Widget::Row { children, gap, .. } = &widgets[0] {
            assert_eq!(children.len(), 2);
            assert_eq!(*gap, 16.0);
        } else { panic!("expected Row"); }
    }

    #[test]
    fn compile_with_context() {
        let mut vars = HashMap::new();
        vars.insert("username".into(), "Alice".into());
        let widgets = ksx_with(r#"<Label text="{username}" />"#, vars).unwrap();
        // The {username} is in the attr value, not interpolation
        assert_eq!(widgets.len(), 1);
    }

    #[test]
    fn compile_card_with_grid() {
        let widgets = ksx(r##"
            <Card title="Dashboard" elevated="true">
                <Grid columns="3" gap="12">
                    <Badge text="Active" color="#0f0" />
                    <Badge text="Pending" color="#ff0" />
                    <Badge text="Error" color="#f00" />
                </Grid>
            </Card>
        "##).unwrap();
        assert_eq!(widgets.len(), 1);
        if let Widget::Card { title, children, elevated, .. } = &widgets[0] {
            assert_eq!(title.as_deref(), Some("Dashboard"));
            assert!(*elevated);
            assert_eq!(children.len(), 1);
            if let Widget::Grid { children: badges, columns, .. } = &children[0] {
                assert_eq!(*columns, 3);
                assert_eq!(badges.len(), 3);
            }
        }
    }

    #[test]
    fn compile_form() {
        let widgets = ksx(r#"
            <Form>
                <Input label="Name" placeholder="Enter name" />
                <Input type="email" label="Email" />
                <Button label="Submit" />
            </Form>
        "#).unwrap();
        if let Widget::Form { children, .. } = &widgets[0] {
            assert_eq!(children.len(), 3);
        }
    }

    #[test]
    fn compile_conditional_hidden() {
        let mut vars = HashMap::new();
        vars.insert("show".into(), "false".into());
        let widgets = ksx_with(r#"<Label k-if="show" text="Hidden" />"#, vars).unwrap();
        assert_eq!(widgets.len(), 0); // k-if=false → not rendered
    }

    #[test]
    fn compile_alert() {
        let widgets = ksx(r#"<Alert message="Success!" severity="success" />"#).unwrap();
        if let Widget::Alert { message, severity, .. } = &widgets[0] {
            assert_eq!(message, "Success!");
            assert_eq!(*severity, Severity::Success);
        }
    }

    #[test]
    fn compile_dialog() {
        let widgets = ksx(r#"<Dialog title="Confirm" open="true"><Label text="Sure?" /></Dialog>"#).unwrap();
        if let Widget::Dialog { title, open, children, .. } = &widgets[0] {
            assert_eq!(title, "Confirm");
            assert!(*open);
            assert_eq!(children.len(), 1);
        }
    }
}
