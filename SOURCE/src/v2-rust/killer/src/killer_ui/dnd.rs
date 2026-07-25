//! **Drag & Drop** — Built-in DnD manager (React DnD / dnd-kit equivalent).
//!
//! Draggable items, drop zones, sortable lists, transfer data,
//! drag preview, and collision detection.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Core Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum DragStatus {
    Idle,
    Dragging,
    Hovering(String),
    Dropped,
}

#[derive(Debug, Clone)]
pub struct DragItem {
    pub id: String,
    pub kind: String,
    pub data: HashMap<String, String>,
    pub source_zone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DropZone {
    pub id: String,
    pub accept: Vec<String>,  // accepted drag item kinds
    pub items: Vec<String>,   // item IDs currently in this zone
    pub max_items: Option<usize>,
    pub x: f64, pub y: f64, pub width: f64, pub height: f64,
}

impl DropZone {
    pub fn new(id: &str, x: f64, y: f64, w: f64, h: f64) -> Self {
        DropZone { id: id.into(), accept: Vec::new(), items: Vec::new(), max_items: None, x, y, width: w, height: h }
    }

    pub fn accept_kind(mut self, kind: &str) -> Self { self.accept.push(kind.into()); self }
    pub fn with_max(mut self, max: usize) -> Self { self.max_items = Some(max); self }

    pub fn can_accept(&self, item: &DragItem) -> bool {
        if self.accept.is_empty() { return true; }
        self.accept.contains(&item.kind)
    }

    pub fn is_full(&self) -> bool {
        self.max_items.map(|m| self.items.len() >= m).unwrap_or(false)
    }

    pub fn contains_point(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// DnD Manager
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct DragEvent {
    pub kind: DragEventKind,
    pub item_id: String,
    pub zone_id: Option<String>,
    pub x: f64, pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEventKind {
    Start,
    Move,
    Enter,
    Leave,
    Over,
    Drop,
    Cancel,
}

/// Main DnD manager.
#[derive(Debug)]
pub struct DndManager {
    pub zones: HashMap<String, DropZone>,
    pub items: HashMap<String, DragItem>,
    pub active_drag: Option<String>,
    pub status: DragStatus,
    pub events: Vec<DragEvent>,
}

impl DndManager {
    pub fn new() -> Self {
        DndManager {
            zones: HashMap::new(), items: HashMap::new(),
            active_drag: None, status: DragStatus::Idle, events: Vec::new(),
        }
    }

    pub fn add_zone(&mut self, zone: DropZone) {
        self.zones.insert(zone.id.clone(), zone);
    }

    pub fn register_item(&mut self, item: DragItem) {
        self.items.insert(item.id.clone(), item);
    }

    /// Begin dragging an item.
    pub fn start_drag(&mut self, item_id: &str, x: f64, y: f64) -> bool {
        if !self.items.contains_key(item_id) { return false; }
        self.active_drag = Some(item_id.into());
        self.status = DragStatus::Dragging;
        self.events.push(DragEvent { kind: DragEventKind::Start, item_id: item_id.into(), zone_id: None, x, y });
        true
    }

    /// Move the dragged item to a new position, check zone hover.
    pub fn move_drag(&mut self, x: f64, y: f64) -> Option<String> {
        let item_id = self.active_drag.clone()?;
        let mut hovered_zone = None;
        for (zid, zone) in &self.zones {
            if zone.contains_point(x, y) {
                hovered_zone = Some(zid.clone());
                break;
            }
        }
        self.status = match &hovered_zone {
            Some(zid) => DragStatus::Hovering(zid.clone()),
            None => DragStatus::Dragging,
        };
        self.events.push(DragEvent { kind: DragEventKind::Move, item_id, zone_id: hovered_zone.clone(), x, y });
        hovered_zone
    }

    /// Drop the item at current position.
    pub fn drop(&mut self, x: f64, y: f64) -> Option<DropResult> {
        let item_id = self.active_drag.take()?;
        let item = self.items.get(&item_id)?;
        let source = item.source_zone.clone();

        // Find the target zone id first
        let target_zid = self.zones.iter().find_map(|(zid, zone)| {
            if zone.contains_point(x, y) && zone.can_accept(item) && !zone.is_full() {
                Some(zid.clone())
            } else {
                None
            }
        });

        if let Some(zid) = target_zid {
            // Remove from source zone
            if let Some(ref src_id) = source {
                if let Some(src_zone) = self.zones.get_mut(src_id.as_str()) {
                    src_zone.items.retain(|id| id != &item_id);
                }
            }
            // Add to target zone
            if let Some(target_zone) = self.zones.get_mut(zid.as_str()) {
                target_zone.items.push(item_id.clone());
            }
            self.status = DragStatus::Dropped;
            self.events.push(DragEvent {
                kind: DragEventKind::Drop, item_id: item_id.clone(),
                zone_id: Some(zid.clone()), x, y,
            });
            return Some(DropResult { item_id, target_zone: zid, source_zone: source });
        }
        // No valid drop target
        self.cancel_drag();
        None
    }

    pub fn cancel_drag(&mut self) {
        if let Some(item_id) = self.active_drag.take() {
            self.events.push(DragEvent {
                kind: DragEventKind::Cancel, item_id, zone_id: None, x: 0.0, y: 0.0,
            });
        }
        self.status = DragStatus::Idle;
    }

    pub fn is_dragging(&self) -> bool { self.active_drag.is_some() }
}

impl Default for DndManager {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct DropResult {
    pub item_id: String,
    pub target_zone: String,
    pub source_zone: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Sortable List
// ══════════════════════════════════════════════════════════════════════════════

/// A sortable list (reorderable items via drag & drop).
#[derive(Debug)]
pub struct SortableList {
    pub id: String,
    pub items: Vec<String>,
}

impl SortableList {
    pub fn new(id: &str, items: Vec<String>) -> Self {
        SortableList { id: id.into(), items }
    }

    pub fn move_item(&mut self, from: usize, to: usize) {
        if from >= self.items.len() || to >= self.items.len() { return; }
        let item = self.items.remove(from);
        self.items.insert(to, item);
    }

    pub fn index_of(&self, item_id: &str) -> Option<usize> {
        self.items.iter().position(|id| id == item_id)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> DndManager {
        let mut mgr = DndManager::new();
        mgr.add_zone(DropZone::new("zone-a", 0.0, 0.0, 100.0, 100.0).accept_kind("card"));
        mgr.add_zone(DropZone::new("zone-b", 200.0, 0.0, 100.0, 100.0).accept_kind("card"));
        mgr.register_item(DragItem {
            id: "card-1".into(), kind: "card".into(),
            data: HashMap::new(), source_zone: Some("zone-a".into()),
        });
        mgr.zones.get_mut("zone-a").unwrap().items.push("card-1".into());
        mgr
    }

    #[test]
    fn drag_start() {
        let mut mgr = setup();
        assert!(mgr.start_drag("card-1", 10.0, 10.0));
        assert!(mgr.is_dragging());
        assert_eq!(mgr.status, DragStatus::Dragging);
    }

    #[test]
    fn drag_move_hover() {
        let mut mgr = setup();
        mgr.start_drag("card-1", 10.0, 10.0);
        let zone = mgr.move_drag(250.0, 50.0);
        assert_eq!(zone.as_deref(), Some("zone-b"));
    }

    #[test]
    fn drag_drop_success() {
        let mut mgr = setup();
        mgr.start_drag("card-1", 10.0, 10.0);
        let result = mgr.drop(250.0, 50.0).unwrap();
        assert_eq!(result.target_zone, "zone-b");
        assert_eq!(result.source_zone.as_deref(), Some("zone-a"));
        assert!(!mgr.zones["zone-a"].items.contains(&"card-1".into()));
        assert!(mgr.zones["zone-b"].items.contains(&"card-1".into()));
    }

    #[test]
    fn drag_drop_outside() {
        let mut mgr = setup();
        mgr.start_drag("card-1", 10.0, 10.0);
        let result = mgr.drop(500.0, 500.0);
        assert!(result.is_none());
        assert_eq!(mgr.status, DragStatus::Idle);
    }

    #[test]
    fn zone_max_items() {
        let mut mgr = DndManager::new();
        mgr.add_zone(DropZone::new("small", 0.0, 0.0, 100.0, 100.0).with_max(1));
        mgr.zones.get_mut("small").unwrap().items.push("existing".into());
        mgr.register_item(DragItem { id: "new".into(), kind: "any".into(), data: HashMap::new(), source_zone: None });
        mgr.start_drag("new", 50.0, 50.0);
        let result = mgr.drop(50.0, 50.0);
        assert!(result.is_none()); // zone full
    }

    #[test]
    fn zone_reject_kind() {
        let mut mgr = DndManager::new();
        mgr.add_zone(DropZone::new("images-only", 0.0, 0.0, 100.0, 100.0).accept_kind("image"));
        mgr.register_item(DragItem { id: "text-1".into(), kind: "text".into(), data: HashMap::new(), source_zone: None });
        mgr.start_drag("text-1", 50.0, 50.0);
        let result = mgr.drop(50.0, 50.0);
        assert!(result.is_none());
    }

    #[test]
    fn cancel_drag() {
        let mut mgr = setup();
        mgr.start_drag("card-1", 10.0, 10.0);
        mgr.cancel_drag();
        assert!(!mgr.is_dragging());
        assert_eq!(mgr.status, DragStatus::Idle);
    }

    #[test]
    fn sortable_move() {
        let mut list = SortableList::new("list", vec!["a".into(), "b".into(), "c".into()]);
        list.move_item(0, 2);
        assert_eq!(list.items, vec!["b", "c", "a"]);
    }

    #[test]
    fn drag_events_recorded() {
        let mut mgr = setup();
        mgr.start_drag("card-1", 10.0, 10.0);
        mgr.move_drag(250.0, 50.0);
        mgr.drop(250.0, 50.0);
        assert!(mgr.events.len() >= 3);
        assert_eq!(mgr.events[0].kind, DragEventKind::Start);
    }
}
