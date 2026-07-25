//! **SSR Engine** — Server-Side Rendering for Killer UI.
//!
//! Renders the widget tree to HTML strings with hydration markers.
//! Supports streaming SSR, head management, and static site generation.
//!
//! Competitive with Next.js SSR / Angular Universal.

use std::collections::HashMap;

use super::patch::Widget;

// ══════════════════════════════════════════════════════════════════════════════
// HTML Renderer
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for the SSR renderer.
pub struct SsrConfig {
    pub pretty: bool,
    pub hydration_markers: bool,
    pub lang: String,
    pub charset: String,
}

impl Default for SsrConfig {
    fn default() -> Self {
        SsrConfig {
            pretty: false,
            hydration_markers: true,
            lang: "en".into(),
            charset: "utf-8".into(),
        }
    }
}

/// SSR renderer — converts Widget tree to HTML string.
pub struct SsrRenderer {
    config: SsrConfig,
    next_hydration_id: u64,
}

impl SsrRenderer {
    pub fn new(config: SsrConfig) -> Self {
        SsrRenderer { config, next_hydration_id: 0 }
    }

    pub fn with_defaults() -> Self {
        Self::new(SsrConfig::default())
    }

    /// Render a full HTML page with head, body, and hydration script.
    pub fn render_page(&mut self, title: &str, widgets: &[Widget], head_extra: &str) -> String {
        let body_html = self.render_widgets(widgets);
        let hydration_script = if self.config.hydration_markers {
            r#"<script>window.__KILLER_HYDRATED__=true;</script>"#
        } else { "" };

        format!(
            "<!DOCTYPE html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"{}\">\n<title>{}</title>\n{}</head>\n<body>\n<div id=\"killer-root\">\n{}</div>\n{}\n</body>\n</html>",
            html_escape(&self.config.lang),
            html_escape(&self.config.charset),
            html_escape(title),
            head_extra,
            body_html,
            hydration_script
        )
    }

    /// Render a list of widgets to an HTML fragment (no <html> wrapper).
    pub fn render_fragment(&mut self, widgets: &[Widget]) -> String {
        self.render_widgets(widgets)
    }

    fn render_widgets(&mut self, widgets: &[Widget]) -> String {
        let nl = if self.config.pretty { "\n" } else { "" };
        let mut out = String::new();
        for w in widgets {
            out.push_str(&self.render_widget(w));
            out.push_str(nl);
        }
        out
    }

    fn hydration_attr(&mut self) -> String {
        if self.config.hydration_markers {
            let id = self.next_hydration_id;
            self.next_hydration_id += 1;
            format!(" data-kid=\"{}\"", id)
        } else {
            String::new()
        }
    }

    fn render_widget(&mut self, widget: &Widget) -> String {
        let h = self.hydration_attr();
        match widget {
            Widget::Label { text, .. } =>
                format!("<span{}>{}</span>", h, html_escape(text)),

            Widget::Button { label, variant, disabled, .. } => {
                let cls = match variant {
                    super::patch::ButtonVariant::Primary => "k-btn k-btn-primary",
                    super::patch::ButtonVariant::Secondary => "k-btn k-btn-secondary",
                    super::patch::ButtonVariant::Danger => "k-btn k-btn-danger",
                    super::patch::ButtonVariant::Outline => "k-btn k-btn-outline",
                    super::patch::ButtonVariant::Text => "k-btn k-btn-text",
                };
                let dis = if *disabled { " disabled" } else { "" };
                format!("<button class=\"{}\"{}{}>{}</button>", cls, dis, h, html_escape(label))
            }

            Widget::TextInput { placeholder, value, input_type, .. } => {
                let it = match input_type {
                    super::patch::InputType::Text => "text",
                    super::patch::InputType::Password => "password",
                    super::patch::InputType::Email => "email",
                    super::patch::InputType::Number => "number",
                    super::patch::InputType::Search => "search",
                    super::patch::InputType::Url => "url",
                    super::patch::InputType::Tel => "tel",
                };
                format!("<input type=\"{}\" placeholder=\"{}\" value=\"{}\"{}>",
                    it, html_escape(placeholder), html_escape(value), h)
            }

            Widget::TextArea { value, rows, .. } =>
                format!("<textarea rows=\"{}\"{}>{}</textarea>", rows, h, html_escape(value)),

            Widget::Checkbox { label, checked, .. } =>
                format!("<label{}><input type=\"checkbox\"{}> {}</label>",
                    h, if *checked { " checked" } else { "" }, html_escape(label)),

            Widget::Select { options, selected, .. } => {
                let mut s = format!("<select{}>", h);
                for opt in options {
                    let sel = if selected.as_deref() == Some(&opt.value) { " selected" } else { "" };
                    s.push_str(&format!("<option value=\"{}\"{}>{}</option>",
                        html_escape(&opt.value), sel, html_escape(&opt.label)));
                }
                s.push_str("</select>");
                s
            }

            Widget::RadioGroup { options, selected, label, .. } => {
                let mut s = format!("<fieldset{}><legend>{}</legend>", h, html_escape(label));
                for opt in options {
                    let chk = if selected.as_deref() == Some(opt) { " checked" } else { "" };
                    s.push_str(&format!("<label><input type=\"radio\" value=\"{}\"{}> {}</label>",
                        html_escape(opt), chk, html_escape(opt)));
                }
                s.push_str("</fieldset>");
                s
            }

            Widget::Image { src, alt, width, height, .. } =>
                format!("<img src=\"{}\" alt=\"{}\" width=\"{}\" height=\"{}\"{}>",
                    html_escape(src), html_escape(alt), width, height, h),

            Widget::Slider { min, max, value, .. } =>
                format!("<input type=\"range\" min=\"{}\" max=\"{}\" value=\"{}\"{}>",
                    min, max, value, h),

            Widget::Toggle { on, label, .. } => {
                let checked = if *on { " checked" } else { "" };
                format!("<label class=\"k-toggle\"{}><input type=\"checkbox\"{}> {}</label>",
                    h, checked, html_escape(label))
            }

            Widget::ProgressBar { value, max, variant, .. } => {
                let cls = format!("k-progress k-progress-{:?}", variant).to_lowercase();
                let pct = if *max > 0.0 { (value / max * 100.0) as u32 } else { 0 };
                format!("<div class=\"{}\"{}/><div class=\"k-progress-bar\" style=\"width:{}%\"></div></div>",
                    cls, h, pct)
            }

            Widget::Spinner { size, .. } =>
                format!("<div class=\"k-spinner\" style=\"width:{}px;height:{}px\"{}/></div>", size, size, h),

            Widget::Badge { text, color, .. } =>
                format!("<span class=\"k-badge\" style=\"background:{}\"{}/>{}</span>",
                    html_escape(color), h, html_escape(text)),

            Widget::Alert { message, severity, dismissible, .. } => {
                let cls = format!("k-alert k-alert-{:?}", severity).to_lowercase();
                let dismiss = if *dismissible { " data-dismissible" } else { "" };
                format!("<div class=\"{}\" role=\"alert\"{}{}>{}</div>", cls, dismiss, h, html_escape(message))
            }

            Widget::Tooltip { text, child, .. } => {
                let child_html = self.render_widget(child);
                format!("<span class=\"k-tooltip-wrapper\"{}>{}<span class=\"k-tooltip\">{}</span></span>",
                    h, child_html, html_escape(text))
            }

            Widget::Divider { vertical, .. } => {
                if *vertical {
                    format!("<div class=\"k-divider-v\"{}/></div>", h)
                } else {
                    format!("<hr{}>", h)
                }
            }

            Widget::Spacer { size, .. } =>
                format!("<div style=\"height:{}px\"{}/></div>", size, h),

            Widget::Card { title, children, elevated, .. } => {
                let elev = if *elevated { " k-card-elevated" } else { "" };
                let mut s = format!("<div class=\"k-card{}\"{}>", elev, h);
                if let Some(t) = title {
                    s.push_str(&format!("<div class=\"k-card-header\">{}</div>", html_escape(t)));
                }
                s.push_str("<div class=\"k-card-body\">");
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</div></div>");
                s
            }

            Widget::Column { children, .. } => {
                let mut s = format!("<div class=\"k-col\"{}>", h);
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</div>");
                s
            }

            Widget::Row { children, gap, .. } => {
                let mut s = format!("<div class=\"k-row\" style=\"display:flex;gap:{}px\"{}>", gap, h);
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</div>");
                s
            }

            Widget::Grid { children, columns, gap, .. } => {
                let mut s = format!(
                    "<div class=\"k-grid\" style=\"display:grid;grid-template-columns:repeat({},1fr);gap:{}px\"{}>",
                    columns, gap, h
                );
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</div>");
                s
            }

            Widget::Tabs { labels, active, .. } => {
                let mut s = format!("<div class=\"k-tabs\"{}>", h);
                s.push_str("<div class=\"k-tab-list\" role=\"tablist\">");
                for (i, label) in labels.iter().enumerate() {
                    let active_cls = if i == *active { " k-tab-active" } else { "" };
                    s.push_str(&format!("<button class=\"k-tab{}\" role=\"tab\">{}</button>",
                        active_cls, html_escape(label)));
                }
                s.push_str("</div></div>");
                s
            }

            Widget::Table { headers, rows, .. } => {
                let mut s = format!("<table class=\"k-table\"{}>", h);
                s.push_str("<thead><tr>");
                for hdr in headers {
                    s.push_str(&format!("<th>{}</th>", html_escape(hdr)));
                }
                s.push_str("</tr></thead><tbody>");
                for row in rows {
                    s.push_str("<tr>");
                    for cell in row {
                        s.push_str(&format!("<td>{}</td>", html_escape(cell)));
                    }
                    s.push_str("</tr>");
                }
                s.push_str("</tbody></table>");
                s
            }

            Widget::List { items, ordered, .. } => {
                let tag = if *ordered { "ol" } else { "ul" };
                let mut s = format!("<{} class=\"k-list\"{}>", tag, h);
                for item in items {
                    s.push_str(&format!("<li>{}</li>", html_escape(item)));
                }
                s.push_str(&format!("</{}>", tag));
                s
            }

            Widget::Dialog { title, children, open, .. } => {
                let open_attr = if *open { " open" } else { "" };
                let mut s = format!("<dialog class=\"k-dialog\"{}{}>", open_attr, h);
                s.push_str(&format!("<h2>{}</h2>", html_escape(title)));
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</dialog>");
                s
            }

            Widget::Snackbar { message, open, .. } => {
                let vis = if *open { "" } else { " hidden" };
                format!("<div class=\"k-snackbar\" role=\"status\"{}{}>{}</div>",
                    vis, h, html_escape(message))
            }

            Widget::ScrollView { children, max_height, .. } => {
                let mut s = format!(
                    "<div class=\"k-scroll\" style=\"overflow:auto;max-height:{}px\"{}>",
                    max_height, h);
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</div>");
                s
            }

            Widget::Form { children, .. } => {
                let mut s = format!("<form class=\"k-form\"{}>", h);
                for child in children { s.push_str(&self.render_widget(child)); }
                s.push_str("</form>");
                s
            }

            Widget::DatePicker { value, label, .. } =>
                format!("<label{}>{} <input type=\"date\" value=\"{}\"></label>",
                    h, html_escape(label), html_escape(value)),

            Widget::ColorPicker { value, label, .. } =>
                format!("<label{}>{} <input type=\"color\" value=\"{}\"></label>",
                    h, html_escape(label), html_escape(value)),

            Widget::FileUpload { accept, multiple, label, .. } => {
                let mult = if *multiple { " multiple" } else { "" };
                format!("<label{}>{} <input type=\"file\" accept=\"{}\"{}></label>",
                    h, html_escape(label), html_escape(accept), mult)
            }

            Widget::Avatar { text, src, size, .. } => {
                if let Some(s) = src {
                    format!("<img class=\"k-avatar\" src=\"{}\" alt=\"{}\" style=\"width:{}px;height:{}px\"{}>",
                        html_escape(s), html_escape(text), size, size, h)
                } else {
                    format!("<div class=\"k-avatar\" style=\"width:{}px;height:{}px\"{}>{}</div>",
                        size, size, h, html_escape(text))
                }
            }

            Widget::Canvas { width, height, .. } =>
                format!("<canvas width=\"{}\" height=\"{}\"{}/></canvas>", width, height, h),

            Widget::Icon { name, size, .. } =>
                format!("<span class=\"k-icon\" data-icon=\"{}\" style=\"font-size:{}px\"{}/></span>",
                    html_escape(name), size, h),

            Widget::Pagination { total_pages, current_page, .. } => {
                let mut s = format!("<nav class=\"k-pagination\" aria-label=\"pagination\"{}>", h);
                for p in 1..=*total_pages {
                    let cls = if p == *current_page { " k-page-active" } else { "" };
                    s.push_str(&format!("<button class=\"k-page{}\">{}</button>", cls, p));
                }
                s.push_str("</nav>");
                s
            }

            _ => format!("<div class=\"k-widget\"{}><!-- unsupported --></div>", h),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Head Manager — manages <title>, <meta>, <link>, <script> tags
// ══════════════════════════════════════════════════════════════════════════════

/// Collects head tags for SSR pages (SEO, OpenGraph, stylesheets, etc.)
pub struct HeadManager {
    pub title: String,
    pub meta_tags: Vec<(String, String)>,
    pub link_tags: Vec<HashMap<String, String>>,
    pub scripts: Vec<String>,
    pub styles: Vec<String>,
}

impl HeadManager {
    pub fn new(title: &str) -> Self {
        HeadManager {
            title: title.to_string(),
            meta_tags: Vec::new(),
            link_tags: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
        }
    }

    pub fn meta(mut self, name: &str, content: &str) -> Self {
        self.meta_tags.push((name.to_string(), content.to_string()));
        self
    }

    pub fn og(self, property: &str, content: &str) -> Self {
        self.meta(&format!("og:{}", property), content)
    }

    pub fn stylesheet(mut self, href: &str) -> Self {
        let mut attrs = HashMap::new();
        attrs.insert("rel".into(), "stylesheet".into());
        attrs.insert("href".into(), href.into());
        self.link_tags.push(attrs);
        self
    }

    pub fn script(mut self, src: &str) -> Self {
        self.scripts.push(src.to_string());
        self
    }

    pub fn inline_style(mut self, css: &str) -> Self {
        self.styles.push(css.to_string());
        self
    }

    /// Generate head HTML content (without <head> tags).
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (name, content) in &self.meta_tags {
            out.push_str(&format!("<meta name=\"{}\" content=\"{}\">\n",
                html_escape(name), html_escape(content)));
        }
        for attrs in &self.link_tags {
            out.push_str("<link");
            for (k, v) in attrs {
                out.push_str(&format!(" {}=\"{}\"", k, html_escape(v)));
            }
            out.push_str(">\n");
        }
        for css in &self.styles {
            out.push_str(&format!("<style>{}</style>\n", css));
        }
        for src in &self.scripts {
            out.push_str(&format!("<script src=\"{}\"></script>\n", html_escape(src)));
        }
        out
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Static Site Generator
// ══════════════════════════════════════════════════════════════════════════════

/// A route for static generation.
pub struct StaticRoute {
    pub path: String,
    pub title: String,
    pub widgets: Vec<Widget>,
    pub head: HeadManager,
}

/// Static site generator — pre-renders routes to HTML files.
pub struct StaticSiteGenerator {
    routes: Vec<StaticRoute>,
    config: SsrConfig,
}

impl StaticSiteGenerator {
    pub fn new() -> Self {
        StaticSiteGenerator {
            routes: Vec::new(),
            config: SsrConfig { pretty: true, ..Default::default() },
        }
    }

    pub fn add_route(mut self, route: StaticRoute) -> Self {
        self.routes.push(route);
        self
    }

    /// Generate all routes, returning (path, html) pairs.
    pub fn generate(&self) -> Vec<(String, String)> {
        let mut pages = Vec::new();
        for route in &self.routes {
            let mut renderer = SsrRenderer::new(SsrConfig {
                pretty: self.config.pretty,
                hydration_markers: self.config.hydration_markers,
                lang: self.config.lang.clone(),
                charset: self.config.charset.clone(),
            });
            let html = renderer.render_page(&route.title, &route.widgets, &route.head.render());
            let file_path = if route.path == "/" {
                "index.html".to_string()
            } else {
                format!("{}/index.html", route.path.trim_start_matches('/'))
            };
            pages.push((file_path, html));
        }
        pages
    }

    pub fn route_count(&self) -> usize { self.routes.len() }
}

impl Default for StaticSiteGenerator {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Streaming SSR
// ══════════════════════════════════════════════════════════════════════════════

/// Streaming SSR breaks the page into chunks for progressive rendering.
pub struct StreamingChunk {
    pub html: String,
    pub is_shell: bool,
}

/// Generate streaming SSR chunks: shell first, then content.
pub fn streaming_render(title: &str, shell_widgets: &[Widget], content_widgets: &[Widget]) -> Vec<StreamingChunk> {
    let mut renderer = SsrRenderer::with_defaults();
    let mut chunks = Vec::new();

    // Chunk 1: HTML shell (head + skeleton)
    let shell_html = format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title></head><body><div id=\"killer-root\">",
        html_escape(title)
    );
    chunks.push(StreamingChunk { html: shell_html, is_shell: true });

    // Chunk 2: Shell widgets (nav, sidebar, etc.)
    let shell_content = renderer.render_fragment(shell_widgets);
    chunks.push(StreamingChunk { html: shell_content, is_shell: true });

    // Chunk 3: Main content (rendered asynchronously)
    let main_content = renderer.render_fragment(content_widgets);
    chunks.push(StreamingChunk { html: main_content, is_shell: false });

    // Chunk 4: Close
    chunks.push(StreamingChunk {
        html: "</div><script>window.__KILLER_HYDRATED__=true;</script></body></html>".into(),
        is_shell: false,
    });

    chunks
}

// ══════════════════════════════════════════════════════════════════════════════
// HTML escape
// ══════════════════════════════════════════════════════════════════════════════

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::killer_ui::patch::*;

    fn wid() -> WidgetId { String::new() }

    #[test]
    fn ssr_label() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Label { id: wid(), text: "Hello <World>".into() }]);
        assert!(html.contains("Hello &lt;World&gt;"));
        assert!(html.contains("data-kid="));
    }

    #[test]
    fn ssr_button() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Button {
            id: wid(),
            label: "Click Me".into(),
            variant: ButtonVariant::Primary,
            disabled: false,
        }]);
        assert!(html.contains("<button"));
        assert!(html.contains("k-btn-primary"));
        assert!(html.contains("Click Me"));
    }

    #[test]
    fn ssr_text_input() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::TextInput {
            id: wid(),
            label: "Name".into(),
            placeholder: "Enter name".into(),
            value: "John".into(),
            input_type: InputType::Text,
        }]);
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("value=\"John\""));
    }

    #[test]
    fn ssr_table() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Table {
            id: wid(),
            headers: vec!["Name".into(), "Age".into()],
            rows: vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ],
            sortable: false,
        }]);
        assert!(html.contains("<table"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<td>Alice</td>"));
    }

    #[test]
    fn ssr_full_page() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_page("My App", &[
            Widget::Label { id: wid(), text: "Welcome".into() },
        ], "");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>My App</title>"));
        assert!(html.contains("killer-root"));
        assert!(html.contains("__KILLER_HYDRATED__"));
    }

    #[test]
    fn ssr_no_hydration() {
        let mut r = SsrRenderer::new(SsrConfig {
            hydration_markers: false,
            ..Default::default()
        });
        let html = r.render_fragment(&[Widget::Label { id: wid(), text: "Plain".into() }]);
        assert!(!html.contains("data-kid"));
    }

    #[test]
    fn ssr_nested_widgets() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[
            Widget::Card {
                id: wid(),
                title: Some("My Card".into()),
                children: vec![
                    Widget::Label { id: wid(), text: "Inside card".into() },
                    Widget::Button { id: wid(), label: "OK".into(), variant: ButtonVariant::Primary, disabled: false },
                ],
                elevated: false,
            }
        ]);
        assert!(html.contains("k-card"));
        assert!(html.contains("Inside card"));
        assert!(html.contains("OK"));
    }

    #[test]
    fn head_manager_render() {
        let head = HeadManager::new("My App")
            .meta("description", "A killer app")
            .og("title", "My App")
            .stylesheet("/css/app.css")
            .script("/js/app.js");
        let html = head.render();
        assert!(html.contains("description"));
        assert!(html.contains("og:title"));
        assert!(html.contains("stylesheet"));
        assert!(html.contains("/js/app.js"));
    }

    #[test]
    fn static_site_generator() {
        let ssg = StaticSiteGenerator::new()
            .add_route(StaticRoute {
                path: "/".into(),
                title: "Home".into(),
                widgets: vec![Widget::Label { id: wid(), text: "Welcome".into() }],
                head: HeadManager::new("Home"),
            })
            .add_route(StaticRoute {
                path: "/about".into(),
                title: "About".into(),
                widgets: vec![Widget::Label { id: wid(), text: "About us".into() }],
                head: HeadManager::new("About"),
            });
        assert_eq!(ssg.route_count(), 2);
        let pages = ssg.generate();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].0, "index.html");
        assert_eq!(pages[1].0, "about/index.html");
        assert!(pages[0].1.contains("Welcome"));
    }

    #[test]
    fn streaming_ssr_chunks() {
        let chunks = streaming_render(
            "App",
            &[Widget::Label { id: wid(), text: "Nav".into() }],
            &[Widget::Label { id: wid(), text: "Content".into() }],
        );
        assert_eq!(chunks.len(), 4);
        assert!(chunks[0].is_shell);
        assert!(chunks[0].html.contains("<!DOCTYPE html>"));
        assert!(chunks[1].html.contains("Nav"));
        assert!(chunks[2].html.contains("Content"));
    }

    #[test]
    fn html_escape_xss() {
        let escaped = html_escape("<script>alert('xss')</script>");
        assert!(!escaped.contains("<script>"));
        assert!(escaped.contains("&lt;script&gt;"));
    }

    #[test]
    fn ssr_grid_layout() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Grid {
            id: wid(),
            children: vec![
                Widget::Label { id: wid(), text: "A".into() },
                Widget::Label { id: wid(), text: "B".into() },
            ],
            columns: 3,
            gap: 16.0,
        }]);
        assert!(html.contains("grid-template-columns:repeat(3,1fr)"));
        assert!(html.contains("gap:16px"));
    }

    #[test]
    fn ssr_select() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Select {
            id: wid(),
            label: "Color".into(),
            options: vec![
                SelectOption { label: "Red".into(), value: "red".into() },
                SelectOption { label: "Green".into(), value: "green".into() },
                SelectOption { label: "Blue".into(), value: "blue".into() },
            ],
            selected: Some("green".into()),
        }]);
        assert!(html.contains("<select"));
        assert!(html.contains("selected>Green"));
    }

    #[test]
    fn ssr_checkbox() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Checkbox {
            id: wid(),
            label: "Accept".into(),
            checked: true,
        }]);
        assert!(html.contains("checked"));
        assert!(html.contains("Accept"));
    }

    #[test]
    fn ssr_dialog() {
        let mut r = SsrRenderer::with_defaults();
        let html = r.render_fragment(&[Widget::Dialog {
            id: wid(),
            title: "Confirm".into(),
            children: vec![Widget::Label { id: wid(), text: "Are you sure?".into() }],
            open: true,
        }]);
        assert!(html.contains("<dialog"));
        assert!(html.contains(" open"));
        assert!(html.contains("Confirm"));
    }
}
