# Killer UI engine (killer_ui) — roadmap

**Goal:** Native UI and “operator-style” creative tooling in **Killer** (`.killer` + VM/runtime), inspired in *spirit* by TouchDesigner (node networks, realtime previews, panels)—**not** a clone of derivative IP.

**Core runtime support:** UI is a **first-class VM surface**, not an optional sidecar. Builtin entry points (always available in `killer-native`):

| Builtin | Role |
|---------|------|
| **`ui_core_version()`** | Returns `killer_ui` engine version (`major` + `label`). |
| **`ui_headless_tick()`** | Runs the integrated demo (patch + operator graph + workspace) and returns **cooked** floats + event queue size (headless). |
| **`ui_headless_snapshot_json()`** | Same demo tick as one **JSON string** (`killer_ui_engine_version`, `cooked`, `events`, `events_pending`) — web / HTTP panels; see `killer_ui::snapshot`. |
| **`ui_health()`** | Same JSON string as HTTP **`GET /health`** (`killer_ui::http_panel::killer_ui_health_json`). |
| **`ui_help()`** | Short text list of builtins, sugar, CLI, and HTTP routes. |
| **`ui_native_window()`** | Phase B: **stub** today (logs cook summary to stderr); intended swap-in for `eframe` / real window when enabled. |

**Reality check:** TouchDesigner is a multi-decade product (GPU stack, operators, licensing, full DMX/NDI ecosystem). `killer_ui` ships **layers** that stay maintainable inside `killer-native`; heavyweight GPU/window deps remain **optional Cargo features** when added.

## Parallel lanes — shared contract (start here)

All tracks advance **in parallel** (web, native, future `.killer` UI syntax). They must **not** invent three different UI models — they share one **logical contract**:

| Lane | Responsibility | Code anchor |
|------|----------------|-------------|
| **Native model** | Authoritative **data structures**: patch tree, operator DAG, workspace, headless tick. | `killer_ui::patch`, `killer_ui::graph`, `killer_ui::workspace`, `killer_ui::runtime_headless` |
| **VM bridge** | `.killer` calls **builtins** that delegate to the engine (no reimplementation in userland). | `ui_*` in `builtin.rs` → `killer_ui::builtins` (see table above) |
| **Web shell** | **DOM/CSS/JS** and HTTP — browsers stay the renderer; Killer **serves** or **mirrors** state. | `kala_ui` (`kala_serve`), `web_framework` for routes; *future:* JSON snapshot of the same patch/graph for a static page |
| **Transpiler (`killer_super`)** | Native codegen for **subset** of Killer — **not** the VM builtin surface. Use **`killer-native`** for `ui_*`. | `src/bin/killer_super.rs` |

### Shared semantics (do not fork casually)

1. **`UiPatch`** — windows → widget tree (`Label`, `Button`, `Slider`, `Toggle`, `Column`); `state` / `toggles` bags for host sync. See `killer_ui::patch::UiPatch`.
2. **`OperatorGraph`** — DAG; **`cook_floats()`** → map of named floats (demo uses `sum`). See `killer_ui::graph`.
3. **`HeadlessFrame`** — **`cooked_floats`** + **`pending_events`** (`Vec<UiEvent>`). Produced by `runtime_headless::tick_headless`.
4. **`UiEvent`** — `ButtonClicked`, `SliderChanged`, `ToggleChanged` (string ids). Same enum for native and any future web bridge.

### Runnable checks (today)

```bash
# VM + builtins (full engine bridge)
cargo run --bin killer-native -- examples/ui_builtin_demo.killer

# Rust-only integrated demo (no .killer syntax for layout yet)
cargo run --release --bin killer_ui_demo
```

### Next wiring (incremental — parallel OK)

| Step | Web lane | Native lane |
|------|----------|-------------|
| 1 | **Done (baseline):** binary **`killer_ui_serve`** — `GET /killer-ui/headless.json` (JSON from `killer_ui::headless_frame_json`), `GET /`, `OPTIONS` for CORS. `cargo run --bin killer_ui_serve -- 8787` | Keep **headless** tests green; extend `Widget` / graph only behind tests. |
| 2 | Optional **WebSocket** or HTTP POST from browser → VM-hosted handler (pattern from `web_framework`). | Replace **`run_demo_window` stub** with `eframe` behind **`killer-ui-egui`**. |
| 3 | Map **`LayoutHints`** / `web_stack` to generated HTML/CSS. | Map same hints to **egui** layout when Phase B lands. |

## What exists today (build on this)

| Piece | Role |
|-------|------|
| **`kala_ui`** | Rich **web** chat UI over pure TCP (`kala_serve`) — patterns for embedded HTML/CSS/JS and HTTP-ish serving without extra crates. |
| **`web_framework` / HTTP** | Request/response, routing concepts — good for **remote panels** and tool UIs. |
| **`nova_gen` / `nova_video` / `nova_audio`** | Procedural **image**, **GIF**, **WAV** — preview assets and data-driven media from Killer. |
| **`vision`** | Image load/describe — ties UI previews to content. |
| VM + **`killer_super`** | Language surface for **logic**; UI should expose **thin builtins** that delegate to Rust. |

## Target architecture (phased)

### Phase A — **killer_ui core** (language + data model)

- **Scene graph / patch** types in Rust: windows, panels, widgets, bindings to `Value`.
- **`.killer` API** sketch: `ui.window`, `ui.button`, `ui.slider`, `ui.label`, `ui.on`, events as Killer callbacks.
- **No GPU yet** — logical layout + event model + serialization (save/load patch as JSON or KORE-like blob later).

### Phase B — **Native shell** (desktop window)

- Add **windowing + GPU or CPU present** behind a feature flag, e.g. `killer-ui-wgpu` or `killer-ui-egui` (pulls `winit` + `wgpu` or `egui` + `eframe` — **new deps**, intentional).
- First runnable: one window, immediate-mode UI, bind Phase A model to pixels.

### Phase C — **“TouchDesigner-ish” operators** (subset)

- **Dataflow nodes** (DAG): sources → transforms → sinks (numbers, tables, textures metadata first).
- **TIMELINE / cook**: deterministic tick; later optional realtime.
- Parallels (conceptual): TOP-style **textures** (bridge from `nova_gen`), CHOP-style **channels** (arrays/float lanes), DAT-style **tables** (reuse NOVA/KORE tables where fit).

### Phase D — **Cluster / multi-panel**

- Multiple panels, docking, save layout; optional **second process** or **web mirror** using same patch model (reuse `kala_ui` patterns).

## Web-stack concepts (Three.js, Angular, React, Node, and friends)

These ecosystems inform **behaviour**, not bundled runtimes. `killer-native` does not embed V8, npm, or React’s reconciler. Use this table to plan features and **interop** (e.g. serve a Three.js page from **`kala_ui`**).

| Familiar idea | Role in `killer_ui` / Killer | Implementation direction |
|---------------|------------------------------|---------------------------|
| **Three.js** (scene, camera, mesh, material, WebGL) | 3D **scene descriptors** + render back-ends | Phase: extend patch/graph with `SceneNode`, camera, mesh handles; output to **WebGL** via static HTML/JS served by **`kala_ui`**, or **wgpu** in-process when GPU deps land. |
| **React** (components, props, state, one-way flow, hooks/effects) | **Declarative `UiPatch`** tree; state drives widgets | `UiPatch` = props tree; Killer/`Value` map = state; “hooks” = **callbacks** on frame (`tick_headless`) or events (`UiEvent`). Optional future **diff** minimal updates. |
| **Angular** (modules, templates, DI, services) | **Panels + modules**; **builtins as services** | `Workspace` regions = module boundaries; inject “services” as **VM builtins** or boxed `Value` registries; template = future `.killer` `ui.*` block syntax. |
| **Node.js** (HTTP server, fs, process, IPC, tooling) | **Tooling / panel host** — not a second language | **`kala_serve`**, `web_framework`, file I/O from stdlib; **IPC** = sockets/HTTP between UI shell and VM; **no Node** unless you explicitly spawn/embed later. |
| **Vue/Svelte/Solid** (reactivity) | **Fine-grained updates** | Optional: mark widgets **reactive** to `Value` paths (future); start with explicit `sync_graph_to_patch_label` style bindings. |
| **CSS / layout** | **Layout hints** | `web_stack::LayoutHints`: flex-grow, future grid; map to egui/layout or generated CSS in web shell. |
| **a11y / i18n** | **Accessibility + locales** | `LayoutHints::a11y_role`, `i18n_key`; web mirror uses ARIA in `kala_ui` HTML. |

### Code anchors

- `killer_ui::web_stack` — `UiParadigm`, stable **`hooks::*`** string IDs, `LayoutHints`.
- `killer_ui::patch` — React-like **declarative** widgets.
- `killer_ui::workspace` — Angular-like **multi-panel** shells.
- **`kala_ui`** — Node-host-like **HTTP panel** without npm.

## Principles

1. **Killer-first:** authors stay in **`*.killer`**; Rust is engine only (matches `POSITIONING.md`).
2. **Optional heavyweight deps:** windowing/GPU behind **Cargo features** so headless CI stays lean.
3. **Interop:** FFI / sockets to **existing** TD or other tools is a valid **integration** path if “everything native” is relaxed for one channel.
4. **Web concepts without Web bloat:** reuse **names and patterns** from Three/React/Angular/Node; ship **Rust + optional static assets**, not a full JS stack in-process unless you opt in.

## Parallel start (A+B+C+D) — current

| Phase | Status in tree |
|-------|----------------|
| **A** | `killer_ui::patch` — `UiPatch`, `Widget`, `UiEvent`, `UiWindow`. |
| **B** | `killer_ui::runtime_native` — **`run_demo_window` is a stub** (prints to stderr). Add `eframe` behind a Cargo feature when crates.io works; swap function body. |
| **C** | `killer_ui::graph` — `OperatorGraph`, topo sort, `cook_floats`, tests. |
| **D** | `killer_ui::workspace` — `Workspace`, `PanelSlot`, `DockRegion`, `PanelContent`. |
| **Integration** | `KillerUiEngine::example_parallel()`, `runtime_headless::tick_headless`, `killer_ui_demo`, **`killer_ui_serve`** / **`killer_ui serve`**, `http_panel` (`/killer-ui/headless.json`, `/killer-ui/version.json`). |
| **.killer** | Tier 4 **line sugar** + builtins: `examples/ui_tier4_sugar.killer`; full `ui { }` blocks still future (`ui_parallel_sketch.killer`). |

## Tier program (1–6) — Tiers **2–5** wired (Tier 2 window = stub until optional `eframe`)

| Tier | Theme | Items | Status |
|------|-------|-------|--------|
| **1** | Core contract | Shared model, VM `ui_*` builtins, `ui_headless_snapshot_json`, docs, `ui_builtin_demo.killer` | **done** (baseline) |
| **2** | Native shell | `run_demo_window` hook, **`killer_ui window`** / demo; real pixels → add optional **`eframe`** + `killer-ui-egui` | **done** (stub) / **next** (pixels) |
| **3** | Web lane | `killer_ui::http_panel`, **`killer_ui_serve`**, `GET /killer-ui/headless.json` + **`version.json`**, CORS `OPTIONS` | **done** (baseline) |
| **4** | `.killer` UI surface | `preprocess_ui_sugar`: `ui version`, `x = ui snapshot`, etc.; `ui_help()`; `examples/ui_tier4_sugar.killer` | **done** (sugar + builtins) / **next** (block AST) |
| **5** | DX | **`killer_ui`** CLI (`serve` / `demo` / `window` / `help`); `ui_help()`; tests | **done** (baseline) |
| **6** | Expectations | No npm/React parity; interop + same-class UX | **started** — Principles + web-stack table |

## Related files

- Engine: `SOURCE/src/v2-rust/killer/src/killer_ui/`
- Chat UI reference: `src/kala_ui.rs`
- Demo: `cargo run --release --bin killer_ui_demo`
- Tier 3 HTTP: `cargo run --bin killer_ui_serve -- 8787` or `cargo run --bin killer_ui -- serve 8787` → `http://127.0.0.1:8787/killer-ui/headless.json` — also `GET /health` (`{"ok":true,"service":"killer_ui"}`); bind e.g. `killer_ui serve 0.0.0.0 8787`.

## Enabling a real Phase B window (Tier 2 — `eframe`)

Requires **crates.io** access (e.g. `cargo add eframe@0.29 --optional` or paste below), then build with `--features killer-ui-egui`.

1. In `Cargo.toml` under `[dependencies]`:
   ```toml
   eframe = { version = "0.29", optional = true }
   ```
   Under `[features]`:
   ```toml
   killer-ui-egui = ["dep:eframe"]
   ```
2. Replace the body of `run_demo_window` in `runtime_native.rs` with `eframe::run_native` (see [eframe `run_native`](https://docs.rs/eframe/0.29.0/eframe/fn.run_native.html) and `killer_ui` demo summary string in the UI).
3. Run: `cargo run --bin killer_ui --features killer-ui-egui -- window` (or `killer_ui_demo`).

**Tracker:** `tracker.csv` rows **77–94** log killer_ui work and follow-ups (2026-04-14). Row **94** is a line-compiler fix for top-level reassignment vs slots (unblocks multiline `while` test). Refresh the Tier 2 row when Phase B is no longer a stub.

### Troubleshooting: `cargo` / crates.io on Windows (SSL)

If `cargo add eframe` fails with **CRYPT_E_NO_REVOCATION_CHECK** or similar, fix the machine trust store / corporate SSL inspection, or try (admin / policy permitting): `git config --global http.sslBackend schannel` and retry; see [Rustup SSL](https://rust-lang.github.io/rustup/installation/windows.html) and Cargo network docs. Tier 2 (`eframe`) requires a successful crates.io fetch once.
