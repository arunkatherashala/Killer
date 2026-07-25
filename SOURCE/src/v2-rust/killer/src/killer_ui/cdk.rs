//! **CDK Primitives** — Angular CDK equivalent.
//!
//! Virtual scroll, overlay positioning, clipboard, breakpoint observer,
//! focus trap, drag container, platform detection.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Virtual Scroll
// ══════════════════════════════════════════════════════════════════════════════

/// Virtual scroll viewport — renders only visible items.
#[derive(Debug, Clone)]
pub struct VirtualScroll {
    pub total_items: usize,
    pub item_height: f64,
    pub viewport_height: f64,
    pub scroll_offset: f64,
    pub buffer_items: usize,
}

impl VirtualScroll {
    pub fn new(total: usize, item_h: f64, viewport_h: f64) -> Self {
        VirtualScroll { total_items: total, item_height: item_h, viewport_height: viewport_h, scroll_offset: 0.0, buffer_items: 3 }
    }

    pub fn set_scroll(&mut self, offset: f64) {
        self.scroll_offset = offset.max(0.0).min(self.total_height() - self.viewport_height);
    }

    pub fn total_height(&self) -> f64 { self.total_items as f64 * self.item_height }

    /// Range of visible item indices (start..end inclusive).
    pub fn visible_range(&self) -> (usize, usize) {
        let start = ((self.scroll_offset / self.item_height) as usize).saturating_sub(self.buffer_items);
        let end = (((self.scroll_offset + self.viewport_height) / self.item_height).ceil() as usize + self.buffer_items)
            .min(self.total_items.saturating_sub(1));
        (start, end)
    }

    pub fn visible_count(&self) -> usize {
        let (s, e) = self.visible_range();
        e - s + 1
    }

    /// Translate offset for a given item index (for absolute positioning).
    pub fn item_offset(&self, index: usize) -> f64 {
        index as f64 * self.item_height - self.scroll_offset
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Overlay
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPosition {
    TopLeft, TopCenter, TopRight,
    CenterLeft, Center, CenterRight,
    BottomLeft, BottomCenter, BottomRight,
}

#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub position: OverlayPosition,
    pub has_backdrop: bool,
    pub backdrop_click_closes: bool,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        OverlayConfig {
            position: OverlayPosition::Center,
            has_backdrop: true,
            backdrop_click_closes: true,
            width: None, height: None, offset_x: 0.0, offset_y: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverlayRef {
    pub id: String,
    pub config: OverlayConfig,
    pub open: bool,
}

/// Overlay manager.
#[derive(Debug, Default)]
pub struct OverlayManager {
    pub overlays: HashMap<String, OverlayRef>,
    next_id: u32,
}

impl OverlayManager {
    pub fn new() -> Self { Self::default() }

    pub fn create(&mut self, config: OverlayConfig) -> String {
        let id = format!("overlay-{}", self.next_id);
        self.next_id += 1;
        self.overlays.insert(id.clone(), OverlayRef { id: id.clone(), config, open: false });
        id
    }

    pub fn open(&mut self, id: &str) {
        if let Some(o) = self.overlays.get_mut(id) { o.open = true; }
    }

    pub fn close(&mut self, id: &str) {
        if let Some(o) = self.overlays.get_mut(id) { o.open = false; }
    }

    pub fn close_all(&mut self) {
        for o in self.overlays.values_mut() { o.open = false; }
    }

    pub fn open_count(&self) -> usize {
        self.overlays.values().filter(|o| o.open).count()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Clipboard
// ══════════════════════════════════════════════════════════════════════════════

/// In-memory clipboard (no actual OS clipboard in pure Rust).
#[derive(Debug, Default)]
pub struct Clipboard {
    pub content: Option<String>,
    pub history: Vec<String>,
}

impl Clipboard {
    pub fn new() -> Self { Self::default() }

    pub fn copy(&mut self, text: &str) {
        self.content = Some(text.into());
        self.history.push(text.into());
    }

    pub fn paste(&self) -> Option<&str> { self.content.as_deref() }

    pub fn clear(&mut self) { self.content = None; }
}

// ══════════════════════════════════════════════════════════════════════════════
// Breakpoint Observer
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub name: String,
    pub min_width: f64,
    pub max_width: f64,
}

impl Breakpoint {
    pub fn new(name: &str, min: f64, max: f64) -> Self {
        Breakpoint { name: name.into(), min_width: min, max_width: max }
    }
}

/// Observes viewport width and reports active breakpoints.
#[derive(Debug)]
pub struct BreakpointObserver {
    pub breakpoints: Vec<Breakpoint>,
    pub current_width: f64,
}

impl BreakpointObserver {
    pub fn new() -> Self {
        BreakpointObserver {
            breakpoints: vec![
                Breakpoint::new("xs", 0.0, 599.0),
                Breakpoint::new("sm", 600.0, 959.0),
                Breakpoint::new("md", 960.0, 1279.0),
                Breakpoint::new("lg", 1280.0, 1919.0),
                Breakpoint::new("xl", 1920.0, f64::MAX),
            ],
            current_width: 1024.0,
        }
    }

    pub fn set_width(&mut self, w: f64) { self.current_width = w; }

    pub fn active_breakpoint(&self) -> Option<&str> {
        self.breakpoints.iter()
            .find(|b| self.current_width >= b.min_width && self.current_width <= b.max_width)
            .map(|b| b.name.as_str())
    }

    pub fn is_matched(&self, name: &str) -> bool {
        self.active_breakpoint() == Some(name)
    }
}

impl Default for BreakpointObserver {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Focus Trap
// ══════════════════════════════════════════════════════════════════════════════

/// Traps tab focus within a container (for dialogs/modals).
#[derive(Debug)]
pub struct FocusTrap {
    pub container_id: String,
    pub focusable_ids: Vec<String>,
    pub current_index: usize,
    pub active: bool,
}

impl FocusTrap {
    pub fn new(container_id: &str, focusables: Vec<String>) -> Self {
        FocusTrap { container_id: container_id.into(), focusable_ids: focusables, current_index: 0, active: false }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.current_index = 0;
    }

    pub fn deactivate(&mut self) { self.active = false; }

    pub fn focus_next(&mut self) -> Option<&str> {
        if !self.active || self.focusable_ids.is_empty() { return None; }
        self.current_index = (self.current_index + 1) % self.focusable_ids.len();
        Some(&self.focusable_ids[self.current_index])
    }

    pub fn focus_prev(&mut self) -> Option<&str> {
        if !self.active || self.focusable_ids.is_empty() { return None; }
        self.current_index = if self.current_index == 0 { self.focusable_ids.len() - 1 } else { self.current_index - 1 };
        Some(&self.focusable_ids[self.current_index])
    }

    pub fn current(&self) -> Option<&str> {
        if self.active { self.focusable_ids.get(self.current_index).map(|s| s.as_str()) } else { None }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Platform Detection
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Platform {
    pub is_browser: bool,
    pub is_server: bool,
    pub is_ios: bool,
    pub is_android: bool,
    pub supports_touch: bool,
    pub supports_webgl: bool,
}

impl Platform {
    /// Returns server-side platform (pure Rust).
    pub fn detect() -> Self {
        Platform {
            is_browser: false, is_server: true,
            is_ios: false, is_android: false,
            supports_touch: false, supports_webgl: false,
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
    fn virtual_scroll_basic() {
        let vs = VirtualScroll::new(1000, 40.0, 400.0);
        let (start, end) = vs.visible_range();
        assert_eq!(start, 0);
        assert!(end <= 16); // ~10 visible + 3 buffer
    }

    #[test]
    fn virtual_scroll_offset() {
        let mut vs = VirtualScroll::new(1000, 40.0, 400.0);
        vs.set_scroll(2000.0); // scroll to item 50
        let (start, _end) = vs.visible_range();
        assert!(start >= 44); // 50 - buffer
    }

    #[test]
    fn virtual_scroll_total_height() {
        let vs = VirtualScroll::new(1000, 40.0, 400.0);
        assert!((vs.total_height() - 40000.0).abs() < 0.01);
    }

    #[test]
    fn overlay_lifecycle() {
        let mut mgr = OverlayManager::new();
        let id = mgr.create(OverlayConfig::default());
        assert_eq!(mgr.open_count(), 0);
        mgr.open(&id);
        assert_eq!(mgr.open_count(), 1);
        mgr.close(&id);
        assert_eq!(mgr.open_count(), 0);
    }

    #[test]
    fn clipboard_copy_paste() {
        let mut cb = Clipboard::new();
        cb.copy("hello world");
        assert_eq!(cb.paste(), Some("hello world"));
        cb.clear();
        assert_eq!(cb.paste(), None);
        assert_eq!(cb.history.len(), 1);
    }

    #[test]
    fn breakpoint_observer() {
        let mut bp = BreakpointObserver::new();
        bp.set_width(800.0);
        assert_eq!(bp.active_breakpoint(), Some("sm"));
        bp.set_width(1500.0);
        assert_eq!(bp.active_breakpoint(), Some("lg"));
    }

    #[test]
    fn focus_trap_cycle() {
        let mut ft = FocusTrap::new("dialog", vec!["btn-ok".into(), "btn-cancel".into(), "input-name".into()]);
        ft.activate();
        assert_eq!(ft.focus_next(), Some("btn-cancel"));
        assert_eq!(ft.focus_next(), Some("input-name"));
        assert_eq!(ft.focus_next(), Some("btn-ok")); // wraps around
    }

    #[test]
    fn focus_trap_prev() {
        let mut ft = FocusTrap::new("modal", vec!["a".into(), "b".into(), "c".into()]);
        ft.activate();
        assert_eq!(ft.focus_prev(), Some("c")); // wraps to end
    }

    #[test]
    fn platform_detect() {
        let p = Platform::detect();
        assert!(p.is_server);
        assert!(!p.is_browser);
    }
}
