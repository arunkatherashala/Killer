//! Headless tick: advance operator graph + merge patch state without a window (A+C+D without B).

use std::collections::HashMap;

use super::graph::OperatorGraph;
use super::patch::{UiEvent, UiPatch};
use super::workspace::Workspace;

/// One engine frame: graph cooks, optional UI events consumed elsewhere.
#[derive(Debug, Default)]
pub struct HeadlessFrame {
    pub cooked_floats: HashMap<String, f64>,
    pub pending_events: Vec<UiEvent>,
}

pub fn tick_headless(_patch: &UiPatch, graph: &OperatorGraph, _workspace: &Workspace) -> HeadlessFrame {
    let cooked = graph.cook_floats().unwrap_or_default();
    HeadlessFrame {
        cooked_floats: cooked,
        pending_events: Vec::new(),
    }
}

/// Push demo graph cook result into patch labels (toy binding).
pub fn sync_graph_to_patch_label(graph: &OperatorGraph, patch: &mut UiPatch) {
    if let Ok(v) = graph.cook_floats() {
        if let Some(sum) = v.get("sum") {
            let s = format!("Cook sum = {:.4}", sum);
            for w in &mut patch.windows {
                sync_widget_tree(&mut w.root, &s);
            }
        }
    }
}

fn sync_widget_tree(w: &mut super::patch::Widget, s: &str) {
    match w {
        super::patch::Widget::Label { id, text } if id == "cook_lbl" => {
            *text = s.to_string();
        }
        super::patch::Widget::Column { children, .. } => {
            for ch in children {
                sync_widget_tree(ch, s);
            }
        }
        _ => {}
    }
}
