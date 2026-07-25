//! **Web-stack concepts** mapped onto `killer_ui` — **ideas and APIs**, not an embedded browser engine.
//!
//! Killer stays **Rust + `.killer`**. We **do not** ship Angular/React/Three.js/Node binaries inside `killer-native`.
//! Instead, these names describe **behaviour** you can implement or **interop** with (e.g. serve a Three.js page from `kala_ui`).

/// Tag for documentation / telemetry: “this panel follows React-like state rules.”
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiParadigm {
    /// Declarative tree, single source of truth, minimal mutable view (React-ish).
    ReactLike,
    /// Modules, structured templates, service-style builtins (Angular-ish).
    AngularLike,
    /// Scene + camera + renderable objects (Three.js–ish); may target WebGL later via served assets.
    ThreeJsLike,
    /// Server-driven UI, routes, tooling process (Node-ish host role — often `kala_ui` + HTTP).
    NodeHostLike,
    /// Immediate-mode / game-loop style (egui-style when Phase B lands).
    ImmediateMode,
}

/// Reserved hook IDs for future bridges (strings stable for saved patches).
pub mod hooks {
    pub const THREE_SCENE: &str = "killer_ui.web.three_scene";
    pub const REACT_STATE_ROOT: &str = "killer_ui.web.react_state";
    pub const ANGULAR_MODULE: &str = "killer_ui.web.ng_module";
    pub const NODE_IPC_CHANNEL: &str = "killer_ui.node.ipc";
}

/// Future: flex/grid constraints (CSS-inspired), a11y roles, i18n keys — extend as layout engine grows.
#[derive(Debug, Clone, Default)]
pub struct LayoutHints {
    /// CSS-like flex grow hint (0 = default).
    pub flex_grow: f32,
    /// Semantic role string for accessibility (e.g. "button", "navigation").
    pub a11y_role: Option<String>,
    /// i18n message key; resolution deferred to host.
    pub i18n_key: Option<String>,
}
