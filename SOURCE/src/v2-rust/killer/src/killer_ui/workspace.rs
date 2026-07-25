//! **Phase D** — multi-panel cluster: docking regions + binding to patches / graph views.

/// Dock region in a simple 5-zone layout (TD-style flexible docking comes later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockRegion {
    North,
    South,
    East,
    West,
    Center,
}

#[derive(Debug, Clone)]
pub enum PanelContent {
    /// Renders UiPatch window id
    PatchWindow(String),
    /// Shows operator subgraph (all nodes for now)
    GraphOverview,
    /// Placeholder for remote web mirror (kala_ui URL)
    WebMirror { base_url: String },
}

#[derive(Debug, Clone)]
pub struct PanelSlot {
    pub id: String,
    pub region: DockRegion,
    pub content: PanelContent,
}

/// Workspace = cluster of panels sharing one engine process.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub cluster_id: String,
    pub panels: Vec<PanelSlot>,
}

impl Workspace {
    pub fn demo_d() -> Self {
        Self {
            cluster_id: "local-1".into(),
            panels: vec![
                PanelSlot {
                    id: "p-center".into(),
                    region: DockRegion::Center,
                    content: PanelContent::PatchWindow("main".into()),
                },
                PanelSlot {
                    id: "p-east".into(),
                    region: DockRegion::East,
                    content: PanelContent::GraphOverview,
                },
                PanelSlot {
                    id: "p-south".into(),
                    region: DockRegion::South,
                    content: PanelContent::WebMirror {
                        base_url: "http://127.0.0.1:8765/".into(),
                    },
                },
            ],
        }
    }
}
