//! **killer_ui** — parallel start of phases **A** (patch model), **B** (native window, feature),
//! **C** (operator DAG), **D** (workspace / cluster panels).
//!
//! Roadmap: `SOURCE/docs/KILLER_UI_ENGINE.md`  
//! Related: [`crate::kala_ui`], [`crate::nova_gen`], [`crate::nova_video`].
//! Web parallels (Three.js, React, Angular, Node): see [`web_stack`] and `KILLER_UI_ENGINE.md`.
//!
//! **Core VM surface:** `ui_core_version`, `ui_headless_tick`, `ui_headless_snapshot_json`, `ui_health`, `ui_help`, `ui_native_window` — see [`builtins`].
//!
//! **Parallel lanes + shared contract** (web `kala_ui` vs native `killer_ui`): see `SOURCE/docs/KILLER_UI_ENGINE.md`
//! section *“Parallel lanes — shared contract”* — `UiPatch`, `OperatorGraph`, `HeadlessFrame`, `UiEvent` must stay aligned across backends.

pub mod animation;
pub mod builtins;
pub mod component;
pub mod devtools;
pub mod events;
pub mod framebuffer;
pub mod graph;
pub mod http_panel;
pub mod layout;
pub mod patch;
pub mod reactive;
pub mod render_widgets;
pub mod router;
pub mod runtime_headless;
pub mod runtime_native;
pub mod serialize;
pub mod snapshot;
pub mod style;
pub mod vdom;
pub mod web_stack;
pub mod window_win32;
pub mod workspace;

// Gap-closure modules (phase 3)
pub mod a11y;
pub mod form_validation;
pub mod i18n;

// Gap-closure modules (phase 4 — world-class parity)
pub mod observable;        // RxJS-like Observable, Subject, BehaviorSubject, operators, EventBus
pub mod data_fetch;        // HTTP client, interceptors, React Query-style cache, Resource
pub mod di;                // Angular-style dependency injection container, scopes, modules
pub mod ssr;               // Server-side rendering → HTML, hydration, streaming, SSG
pub mod template_parser;   // KSX/JSX template language → Widget tree compiler
pub mod forms;             // Reactive forms: FormControl, FormGroup, FormArray, validators
pub mod grid_layout;       // Full CSS Grid layout algorithm (tracks, spans, auto-placement)
pub mod test_bed;          // Component TestBed: render, query, simulate, assert
pub mod build_pipeline;    // Tree-shaking, code-splitting, minification, bundle analysis

pub use builtins::{
    builtin_ui_core_version, builtin_ui_headless_snapshot_json, builtin_ui_headless_tick,
    builtin_ui_health, builtin_ui_help, builtin_ui_native_window,
    builtin_ui_render_gallery, builtin_ui_render_screenshot,
    builtin_ui_signal_create, builtin_ui_signal_get, builtin_ui_signal_set,
    builtin_ui_computed, builtin_ui_effect, builtin_ui_batch,
    builtin_ui_on_event, builtin_ui_dispatch_event, builtin_ui_theme,
    builtin_ui_style_set, builtin_ui_component_register, builtin_ui_component_create,
    builtin_ui_layout_compute, builtin_ui_navigate, builtin_ui_route_add,
    builtin_ui_vdom_diff, builtin_ui_vdom_patch, builtin_ui_animate,
    builtin_ui_animate_keyframes, builtin_ui_inspect, builtin_ui_perf_snapshot,
};
pub use snapshot::headless_frame_json;
pub use http_panel::{killer_ui_health_json, run_headless_panel_server};

pub use graph::{CookError, NodeId, OperatorGraph, OperatorKind, OperatorNode};
pub use patch::{UiEvent, UiPatch, UiWindow, Widget};
pub use web_stack::{LayoutHints, UiParadigm};
pub use workspace::{DockRegion, PanelContent, PanelSlot, Workspace};

pub use reactive::ReactiveStore;
pub use events::EventDispatcher;
pub use style::{Style, StyleStore, Theme};
pub use component::{ComponentRegistry, ComponentTree};
pub use layout::LayoutNode;
pub use router::Router;
pub use vdom::VNode;
pub use animation::AnimationController;
pub use devtools::{InspectorSnapshot, PerfProfiler};
pub use framebuffer::Framebuffer;
pub use render_widgets::RenderTheme;

// Phase 3 re-exports
pub use a11y::{AriaProps, AriaRole, FocusManager, A11yTree};
pub use form_validation::{FormValidator, FieldValidator, ValidationRule};
pub use i18n::I18n;
pub use animation::{AnimationSequence, Transform3D, TransformStack};
pub use component::{ErrorBoundary, SlotProjection};
pub use router::{RouteGuard, GuardKind, GuardResult};

// Phase 4 re-exports (world-class parity)
pub use observable::{Subject, BehaviorSubject, Observable, EventBus, StreamValue, StreamEvent, Pipe};
pub use data_fetch::{HttpClient, HttpResponse, QueryClient, Resource, ResourceState, Interceptor};
pub use di::{Container, ServiceValue, Scope, Module as DiModule};
pub use ssr::{SsrRenderer, SsrConfig, HeadManager, StaticSiteGenerator, StaticRoute};
pub use template_parser::{parse_template, compile_template, ksx, ksx_with, TemplateNode, TemplateContext};
pub use forms::{FormControl, FormGroup, FormArray, Validators, ValidationError as FormValidationError};
pub use grid_layout::{GridDefinition, GridPlacement, GridItem, ComputedGridItem, TrackSize, GridAlign, compute_grid};
pub use test_bed::TestHost;
pub use build_pipeline::{ModuleGraph, ModuleNode, BuildConfig, BuildTarget, TreeShakeResult, tree_shake, code_split, build, minify};

// Phase 5 modules — React/Angular gap closure + 3D/4D/5D engines
pub mod error_boundary;
pub mod component_library;
pub mod dom_bridge;
pub mod cli_scaffold;
pub mod scene3d;
pub mod geometry3d;
pub mod material3d;
pub mod renderer3d;
pub mod physics3d;
pub mod animation3d;
pub mod engine4d;
pub mod engine5d;

// Phase 5 re-exports
pub use error_boundary::{ErrorBoundaryDef, ErrorBoundaryManager, ErrorFallback, RecoveryStrategy, RenderError};
pub use component_library::{MaterialTheme, SnackbarQueue};
pub use dom_bridge::{DomNode, DomNodeBuilder, DomPatch, DomRenderer, HeadlessDom, DomEvent, DomEventData};
pub use cli_scaffold::{ProjectTemplate, CliCommand, ProjectConfig};
pub use scene3d::{Vec3, Quat, Mat4, Color3, Object3D, Object3DKind, Scene, OrbitControls, CameraProjection, LightKind};
pub use geometry3d::{BufferGeometry, Vertex, Triangle};
pub use material3d::{Material, MaterialBase, Texture, TextureData, ShaderUniform};
pub use renderer3d::{RenderTarget, RenderConfig, RenderStats, ShadowMap, LodGroup, MeshInstance};
pub use physics3d::{RigidBody, ColliderShape, PhysicsWorld, Ray, RayHit, AudioSource};
pub use animation3d::{AnimationClip, AnimationMixer, AnimationTrack, AnimProperty, Skeleton, Bone, ParticleSystem};
pub use engine4d::{Vec4, Mat5, Mesh4D, Object4D, Camera4D, RotationPlane4D};
pub use engine5d::{Vec5, Mat6, Mesh5D, ProjectionPipeline, RotationPlane5D};

// Phase 6 modules — React Suspense/Portals/Context, Angular Pipes, CSS-in-JS, 3D extras
pub mod suspense;
pub mod context;
pub mod portal;
pub mod pipes;
pub mod css_engine;
pub mod model_loader;
pub mod gpu_bridge;
pub mod postprocess3d;
pub mod controls3d;
pub mod csg3d;
pub mod text3d;

// Phase 6 re-exports
pub use suspense::{SuspenseBoundary, SuspenseResource, ResourceStore, LazyComponent, LazyRegistry, ConcurrentScheduler, RenderPriority};
pub use context::{ContextValue, ContextDef, ContextProvider, ContextStore};
pub use portal::{PortalTarget, Portal, PortalManager};
pub use pipes::{PipeValue, PipeRegistry};
pub use css_engine::{CssProp, PseudoSelector, MediaQuery, CssKeyframes, StyledComponent, CssEngine};
pub use model_loader::{ObjModel, ObjGroup, ObjMaterial, GltfScene, parse_obj, parse_mtl, parse_gltf_json};
pub use gpu_bridge::{GpuDevice, GpuCommandBuffer, GpuCommand, GpuPipeline, GpuBuffer, GpuTexture, GpuShader, GpuBackend};
pub use postprocess3d::{PostProcessPipeline, SsaoConfig, DofConfig, ColorGrading};
pub use controls3d::{FlyControls, FirstPersonControls, TrackballControls, MapControls};
pub use csg3d::{CsgNode, CsgPolygon, CsgPlane, csg_union, csg_subtract, csg_intersect};
pub use text3d::{BitmapFont, TextGeometryConfig, TextAlign};

// Phase 7 modules — ALL remaining React/Angular/Three.js gap closure
pub mod server_components;
pub mod actions;
pub mod ref_system;
pub mod dnd;
pub mod defer;
pub mod cdk;
pub mod sprite3d;
pub mod envmap3d;
pub mod curves3d;
pub mod audio3d;
pub mod helpers3d;

// Phase 7 re-exports
pub use server_components::{ServerComponent, ComponentEnv, PropValue, RscRenderer, RscChunk, HydrationScheduler, HydrationTask, HydrationPriority};
pub use actions::{FormActionStatus, FormStatus, OptimisticState, ServerAction, ActionDispatcher};
pub use ref_system::{Ref, RefValue, RefRegistry, ForwardedRef, ImperativeHandle, Profiler, ProfilerEntry, RenderPhase, StrictMode};
pub use dnd::{DragItem, DropZone, DndManager, DragEvent, DragEventKind, SortableList, DropResult};
pub use defer::{DeferTrigger, DeferBlock, DeferState, DeferManager, RouteResolver, ResolverRegistry, ChangeDetector, ChangeDetectionStrategy, SwitchBlock};
pub use cdk::{VirtualScroll, OverlayManager, OverlayConfig, Clipboard, BreakpointObserver, FocusTrap, Platform};
pub use sprite3d::{Sprite, InstancedMesh, Fog};
pub use envmap3d::{CubeFace, CubeTexture, ReflectionProbe};
pub use curves3d::{Curve, CatmullRomCurve, CubicBezier, QuadraticBezier, LineCurve};
pub use audio3d::{DistanceModel, AudioListener, PositionalAudio, AudioScene, AudioMixEntry};
pub use helpers3d::{LineSegment, AxesHelper, GridHelper, BoundingBoxHelper, ArrowHelper, SkeletonHelper};

/// Engine version for saved patches / compatibility (bump when serialization breaks).
pub const KILLER_UI_ENGINE_VERSION: u32 = 2;

/// Integrated engine: **A + C + D** in-tree; **B** via [`runtime_native::run_demo_window`] (stub until `eframe` — see `SOURCE/docs/KILLER_UI_ENGINE.md`).
#[derive(Debug, Clone)]
pub struct KillerUiEngine {
    pub version: u32,
    pub patch: UiPatch,
    pub graph: OperatorGraph,
    pub workspace: Workspace,
}

impl KillerUiEngine {
    pub fn new() -> Self {
        Self {
            version: KILLER_UI_ENGINE_VERSION,
            patch: UiPatch::default(),
            graph: OperatorGraph::default(),
            workspace: Workspace {
                cluster_id: "default".into(),
                panels: vec![],
            },
        }
    }

    /// One-shot demo: populated **A**, **C**, **D**; headless tick syncs graph → patch label.
    pub fn example_parallel() -> Self {
        let mut patch = UiPatch::demo_a();
        let graph = OperatorGraph::demo_c();
        let workspace = Workspace::demo_d();
        runtime_headless::sync_graph_to_patch_label(&graph, &mut patch);
        Self {
            version: KILLER_UI_ENGINE_VERSION,
            patch,
            graph,
            workspace,
        }
    }

    pub fn tick_headless(&self) -> runtime_headless::HeadlessFrame {
        runtime_headless::tick_headless(&self.patch, &self.graph, &self.workspace)
    }
}

impl Default for KillerUiEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod integration_parallel {
    use super::*;

    #[test]
    fn parallel_scaffold_runs() {
        let e = KillerUiEngine::example_parallel();
        let frame = e.tick_headless();
        assert!((frame.cooked_floats["sum"] - 3.0).abs() < 1e-9);
        let lbl = find_cook_label(&e.patch);
        assert!(lbl.contains("3"));
    }

    fn find_cook_label(patch: &UiPatch) -> String {
        for w in &patch.windows {
            if let Some(s) = scan(&w.root) {
                return s;
            }
        }
        String::new()
    }

    fn scan(w: &Widget) -> Option<String> {
        match w {
            Widget::Label { id, text } if id == "cook_lbl" => Some(text.clone()),
            Widget::Column { children, .. } => {
                for c in children {
                    if let Some(s) = scan(c) {
                        return Some(s);
                    }
                }
                None
            }
            _ => None,
        }
    }
}
