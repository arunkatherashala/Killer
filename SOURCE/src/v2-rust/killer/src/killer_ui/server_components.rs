//! **Server Components** — React Server Components (RSC) equivalent.
//!
//! Client/server component boundary, streaming execution, server-side
//! data fetching with automatic serialization across the wire.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Component Boundary
// ══════════════════════════════════════════════════════════════════════════════

/// Whether a component runs on server or client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentEnv {
    Server,
    Client,
    Shared,
}

/// A server component definition.
#[derive(Debug, Clone)]
pub struct ServerComponent {
    pub name: String,
    pub env: ComponentEnv,
    pub props: HashMap<String, PropValue>,
    pub children: Vec<ServerComponent>,
    pub rendered_html: Option<String>,
    pub payload: Option<String>,  // serialized RSC payload
}

/// Prop values that cross the server/client boundary.
#[derive(Debug, Clone)]
pub enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<PropValue>),
    Map(HashMap<String, PropValue>),
    /// A reference to a client component (cannot be inlined on server)
    ClientRef(String),
}

impl ServerComponent {
    pub fn server(name: &str) -> Self {
        ServerComponent {
            name: name.into(), env: ComponentEnv::Server,
            props: HashMap::new(), children: Vec::new(),
            rendered_html: None, payload: None,
        }
    }

    pub fn client(name: &str) -> Self {
        ServerComponent {
            name: name.into(), env: ComponentEnv::Client,
            props: HashMap::new(), children: Vec::new(),
            rendered_html: None, payload: None,
        }
    }

    pub fn with_prop(mut self, key: &str, val: PropValue) -> Self {
        self.props.insert(key.into(), val);
        self
    }

    pub fn with_child(mut self, child: ServerComponent) -> Self {
        self.children.push(child);
        self
    }

    pub fn is_server(&self) -> bool { self.env == ComponentEnv::Server }
    pub fn is_client(&self) -> bool { self.env == ComponentEnv::Client }
}

// ══════════════════════════════════════════════════════════════════════════════
// RSC Renderer
// ══════════════════════════════════════════════════════════════════════════════

/// Streaming RSC renderer.
#[derive(Debug)]
pub struct RscRenderer {
    /// Server-rendered HTML chunks (streamed in order)
    pub chunks: Vec<RscChunk>,
    /// Client component references to hydrate
    pub client_refs: Vec<ClientRef>,
    /// Total server render time budget (ms)
    pub budget_ms: u64,
}

/// A streamed chunk from server rendering.
#[derive(Debug, Clone)]
pub struct RscChunk {
    pub component_name: String,
    pub html: String,
    pub boundary_id: String,
    pub is_final: bool,
}

/// A client component reference embedded in server output.
#[derive(Debug, Clone)]
pub struct ClientRef {
    pub component_name: String,
    pub placeholder_id: String,
    pub serialized_props: String,
}

impl RscRenderer {
    pub fn new() -> Self {
        RscRenderer { chunks: Vec::new(), client_refs: Vec::new(), budget_ms: 5000 }
    }

    /// Render a server component tree, producing chunks and client refs.
    pub fn render(&mut self, root: &ServerComponent) {
        self.render_node(root, 0);
    }

    fn render_node(&mut self, node: &ServerComponent, depth: usize) {
        match node.env {
            ComponentEnv::Server | ComponentEnv::Shared => {
                // Server-render: produce HTML chunk
                let mut html = format!("<div data-rsc=\"{}\" data-depth=\"{}\">", node.name, depth);
                let props_str: Vec<String> = node.props.iter()
                    .map(|(k, v)| format!("{}={}", k, prop_to_string(v)))
                    .collect();
                if !props_str.is_empty() {
                    html.push_str(&format!("<!-- props: {} -->", props_str.join(", ")));
                }
                // Render children
                for child in &node.children {
                    self.render_node(child, depth + 1);
                }
                html.push_str("</div>");
                self.chunks.push(RscChunk {
                    component_name: node.name.clone(),
                    html,
                    boundary_id: format!("rsc-{}-{}", node.name, depth),
                    is_final: node.children.is_empty(),
                });
            }
            ComponentEnv::Client => {
                // Client component: emit placeholder + ref
                let placeholder = format!("client-{}-{}", node.name, depth);
                let serialized = serialize_props(&node.props);
                self.client_refs.push(ClientRef {
                    component_name: node.name.clone(),
                    placeholder_id: placeholder.clone(),
                    serialized_props: serialized,
                });
                self.chunks.push(RscChunk {
                    component_name: node.name.clone(),
                    html: format!("<div id=\"{}\" data-client-component=\"{}\"></div>", placeholder, node.name),
                    boundary_id: placeholder,
                    is_final: true,
                });
            }
        }
    }

    /// Collect all HTML chunks into a single document.
    pub fn to_html(&self) -> String {
        self.chunks.iter().map(|c| c.html.as_str()).collect::<Vec<_>>().join("\n")
    }

    /// Generate the RSC payload (JSON-like serialization of client refs).
    pub fn to_payload(&self) -> String {
        let refs: Vec<String> = self.client_refs.iter().map(|r| {
            format!("{{\"component\":\"{}\",\"id\":\"{}\",\"props\":{}}}", r.component_name, r.placeholder_id, r.serialized_props)
        }).collect();
        format!("[{}]", refs.join(","))
    }
}

impl Default for RscRenderer {
    fn default() -> Self { Self::new() }
}

fn prop_to_string(v: &PropValue) -> String {
    match v {
        PropValue::Null => "null".into(),
        PropValue::Bool(b) => b.to_string(),
        PropValue::Int(i) => i.to_string(),
        PropValue::Float(f) => format!("{:.6}", f),
        PropValue::Str(s) => format!("\"{}\"", s),
        PropValue::List(l) => format!("[{}]", l.iter().map(prop_to_string).collect::<Vec<_>>().join(",")),
        PropValue::Map(m) => format!("{{{}}}", m.iter().map(|(k,v)| format!("\"{}\":{}", k, prop_to_string(v))).collect::<Vec<_>>().join(",")),
        PropValue::ClientRef(name) => format!("$ref:{}", name),
    }
}

fn serialize_props(props: &HashMap<String, PropValue>) -> String {
    let entries: Vec<String> = props.iter()
        .map(|(k, v)| format!("\"{}\":{}", k, prop_to_string(v)))
        .collect();
    format!("{{{}}}", entries.join(","))
}

// ══════════════════════════════════════════════════════════════════════════════
// Selective Hydration
// ══════════════════════════════════════════════════════════════════════════════

/// Priority for hydration scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HydrationPriority {
    Critical = 0,
    Visible = 1,
    Idle = 2,
}

/// A hydration task for a client component boundary.
#[derive(Debug, Clone)]
pub struct HydrationTask {
    pub placeholder_id: String,
    pub component_name: String,
    pub priority: HydrationPriority,
    pub hydrated: bool,
}

/// Manages selective hydration of client components.
#[derive(Debug)]
pub struct HydrationScheduler {
    pub tasks: Vec<HydrationTask>,
}

impl HydrationScheduler {
    pub fn new() -> Self { HydrationScheduler { tasks: Vec::new() } }

    pub fn from_client_refs(refs: &[ClientRef]) -> Self {
        let tasks = refs.iter().map(|r| HydrationTask {
            placeholder_id: r.placeholder_id.clone(),
            component_name: r.component_name.clone(),
            priority: HydrationPriority::Idle,
            hydrated: false,
        }).collect();
        HydrationScheduler { tasks }
    }

    /// Mark a component as visible (promotes to higher priority).
    pub fn mark_visible(&mut self, placeholder_id: &str) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.placeholder_id == placeholder_id) {
            t.priority = HydrationPriority::Visible;
        }
    }

    /// Mark a component as critical (e.g., user interacted with it).
    pub fn mark_critical(&mut self, placeholder_id: &str) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.placeholder_id == placeholder_id) {
            t.priority = HydrationPriority::Critical;
        }
    }

    /// Process hydration tasks within time budget, highest priority first.
    pub fn flush(&mut self, max_tasks: usize) -> Vec<String> {
        self.tasks.sort_by_key(|t| t.priority);
        let mut hydrated = Vec::new();
        let mut count = 0;
        for task in &mut self.tasks {
            if task.hydrated || count >= max_tasks { continue; }
            task.hydrated = true;
            hydrated.push(task.placeholder_id.clone());
            count += 1;
        }
        hydrated
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.hydrated).count()
    }

    pub fn all_hydrated(&self) -> bool {
        self.tasks.iter().all(|t| t.hydrated)
    }
}

impl Default for HydrationScheduler {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_component_basic() {
        let sc = ServerComponent::server("App")
            .with_prop("title", PropValue::Str("Hello".into()));
        assert!(sc.is_server());
        assert_eq!(sc.props.len(), 1);
    }

    #[test]
    fn client_component_boundary() {
        let sc = ServerComponent::client("Counter");
        assert!(sc.is_client());
    }

    #[test]
    fn rsc_render_server_only() {
        let tree = ServerComponent::server("Layout")
            .with_child(ServerComponent::server("Header"))
            .with_child(ServerComponent::server("Footer"));
        let mut r = RscRenderer::new();
        r.render(&tree);
        assert!(r.chunks.len() >= 3);
        assert!(r.client_refs.is_empty());
        let html = r.to_html();
        assert!(html.contains("data-rsc=\"Layout\""));
    }

    #[test]
    fn rsc_render_mixed() {
        let tree = ServerComponent::server("Page")
            .with_child(ServerComponent::client("InteractiveWidget")
                .with_prop("count", PropValue::Int(0)));
        let mut r = RscRenderer::new();
        r.render(&tree);
        assert_eq!(r.client_refs.len(), 1);
        assert_eq!(r.client_refs[0].component_name, "InteractiveWidget");
        let html = r.to_html();
        assert!(html.contains("data-client-component="));
    }

    #[test]
    fn rsc_payload() {
        let tree = ServerComponent::server("App")
            .with_child(ServerComponent::client("Btn").with_prop("label", PropValue::Str("Click".into())));
        let mut r = RscRenderer::new();
        r.render(&tree);
        let payload = r.to_payload();
        assert!(payload.contains("\"component\":\"Btn\""));
        assert!(payload.contains("\"label\":\"Click\""));
    }

    #[test]
    fn hydration_scheduler_priority() {
        let refs = vec![
            ClientRef { component_name: "A".into(), placeholder_id: "p-a".into(), serialized_props: "{}".into() },
            ClientRef { component_name: "B".into(), placeholder_id: "p-b".into(), serialized_props: "{}".into() },
            ClientRef { component_name: "C".into(), placeholder_id: "p-c".into(), serialized_props: "{}".into() },
        ];
        let mut sched = HydrationScheduler::from_client_refs(&refs);
        sched.mark_critical("p-b");
        sched.mark_visible("p-c");
        let hydrated = sched.flush(2);
        // Critical first, then visible
        assert_eq!(hydrated[0], "p-b");
        assert_eq!(hydrated[1], "p-c");
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn hydration_scheduler_flush_all() {
        let refs = vec![
            ClientRef { component_name: "X".into(), placeholder_id: "p-x".into(), serialized_props: "{}".into() },
        ];
        let mut sched = HydrationScheduler::from_client_refs(&refs);
        sched.flush(10);
        assert!(sched.all_hydrated());
    }

    #[test]
    fn prop_serialization() {
        let mut props = HashMap::new();
        props.insert("name".into(), PropValue::Str("test".into()));
        props.insert("count".into(), PropValue::Int(42));
        let s = serialize_props(&props);
        assert!(s.contains("\"name\":\"test\""));
        assert!(s.contains("\"count\":42"));
    }
}
