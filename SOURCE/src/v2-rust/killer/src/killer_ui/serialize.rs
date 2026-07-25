//! JSON export for [`super::KillerUiEngine`] (no serde dependency).

use super::graph::{OperatorGraph, OperatorKind, OperatorNode};
use super::patch::{UiPatch, UiWindow, Widget};
use super::KillerUiEngine;

pub fn engine_to_json(engine: &KillerUiEngine) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"killer_ui_engine_version\": {},\n", engine.version));
    s.push_str(&format!(
        "  \"schema\": {},\n",
        json_escape("killer_ui.engine.v1")
    ));
    s.push_str("  \"patch\": ");
    s.push_str(&patch_to_json(&engine.patch));
    s.push_str(",\n  \"graph\": ");
    s.push_str(&graph_to_json(&engine.graph));
    s.push_str(",\n  \"workspace\": ");
    s.push_str(&workspace_to_json(&engine.workspace));
    s.push_str(",\n  \"cooked_floats\": ");
    match engine.graph.cook_floats() {
        Ok(m) => {
            s.push_str("{\n");
            let mut first = true;
            let mut keys: Vec<_> = m.keys().cloned().collect();
            keys.sort();
            for k in keys {
                if !first {
                    s.push_str(",\n");
                }
                first = false;
                s.push_str(&format!("    {}: {}", json_escape(&k), m[&k]));
            }
            s.push_str("\n  }");
        }
        Err(e) => {
            s.push_str(&format!("{{ \"error\": {:?} }}", e));
        }
    }
    s.push_str("\n}\n");
    s
}

fn workspace_to_json(w: &super::Workspace) -> String {
    let mut s = String::from("{\n    \"cluster_id\": ");
    s.push_str(&json_escape(&w.cluster_id));
    s.push_str(",\n    \"panels\": [\n");
    for (i, p) in w.panels.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str("      { \"id\": ");
        s.push_str(&json_escape(&p.id));
        s.push_str(", \"region\": ");
        s.push_str(&json_escape(&format!("{:?}", p.region)));
        s.push_str(", \"content\": ");
        s.push_str(&json_escape(&format!("{:?}", p.content)));
        s.push_str(" }");
    }
    s.push_str("\n    ]\n  }");
    s
}

fn graph_to_json(g: &OperatorGraph) -> String {
    let mut s = String::from("{\n    \"nodes\": [\n");
    for (i, n) in g.nodes.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&node_to_json(n));
    }
    s.push_str("\n    ]\n  }");
    s
}

fn node_to_json(n: &OperatorNode) -> String {
    let kind_str = match &n.kind {
        OperatorKind::ConstantFloat(x) => format!("{{ \"t\":\"const_f\",\"v\":{} }}", x),
        OperatorKind::FloatArray(a) => {
            let nums: Vec<String> = a.iter().map(|x| format!("{}", x)).collect();
            format!("{{ \"t\":\"float_arr\",\"v\":[{}] }}", nums.join(","))
        }
        OperatorKind::Add => "{\"t\":\"add\"}".to_string(),
        OperatorKind::Mul => "{\"t\":\"mul\"}".to_string(),
        OperatorKind::TextureMeta { width, height } => {
            format!("{{ \"t\":\"tex_meta\",\"w\":{},\"h\":{} }}", width, height)
        }
        OperatorKind::DatTablePlaceholder => "{\"t\":\"dat_placeholder\"}".to_string(),
    };
    let mut ins = String::from("[");
    for (i, inp) in n.inputs.iter().enumerate() {
        if i > 0 {
            ins.push_str(", ");
        }
        ins.push_str(&json_escape(&inp.0));
    }
    ins.push(']');
    format!(
        "      {{ \"id\": {}, \"kind\": {}, \"inputs\": {} }}",
        json_escape(&n.id.0),
        kind_str,
        ins
    )
}

fn patch_to_json(p: &UiPatch) -> String {
    let mut s = String::from("{\n    \"windows\": [\n");
    for (i, w) in p.windows.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&window_to_json(w));
    }
    s.push_str("\n    ]\n  }");
    s
}

fn window_to_json(w: &UiWindow) -> String {
    let mut s = String::from("      { \"id\": ");
    s.push_str(&json_escape(&w.id));
    s.push_str(", \"title\": ");
    s.push_str(&json_escape(&w.title));
    s.push_str(", \"root\": ");
    s.push_str(&widget_to_json(&w.root));
    s.push_str(" }");
    s
}

fn widget_to_json(w: &Widget) -> String {
    match w {
        Widget::Label { id, text, .. } => {
            format!(
                "{{ \"t\":\"label\", \"id\": {}, \"text\": {} }}",
                json_escape(id),
                json_escape(text),
            )
        }
        Widget::Button { id, label, variant, disabled, .. } => {
            format!(
                "{{ \"t\":\"button\", \"id\": {}, \"label\": {}, \"variant\": \"{:?}\", \"disabled\": {} }}",
                json_escape(id),
                json_escape(label),
                variant,
                disabled,
            )
        }
        Widget::Slider {
            id,
            label,
            min,
            max,
            value,
            ..
        } => {
            format!(
                "{{ \"t\":\"slider\", \"id\": {}, \"label\": {}, \"min\":{}, \"max\":{}, \"v\":{} }}",
                json_escape(id),
                json_escape(label),
                min,
                max,
                value,
            )
        }
        Widget::Toggle { id, label, on, .. } => {
            format!(
                "{{ \"t\":\"toggle\", \"id\": {}, \"label\": {}, \"on\":{} }}",
                json_escape(id),
                json_escape(label),
                on,
            )
        }
        Widget::Column { id, children, .. } => {
            let c = children_json(children);
            format!(
                "{{ \"t\":\"column\", \"id\": {}, \"children\": {} }}",
                json_escape(id),
                c,
            )
        }
        Widget::Row { id, children, gap, .. } => {
            let c = children_json(children);
            format!(
                "{{ \"t\":\"row\", \"id\": {}, \"gap\": {}, \"children\": {} }}",
                json_escape(id), gap, c,
            )
        }
        Widget::Grid { id, children, columns, gap, .. } => {
            let c = children_json(children);
            format!(
                "{{ \"t\":\"grid\", \"id\": {}, \"columns\": {}, \"gap\": {}, \"children\": {} }}",
                json_escape(id), columns, gap, c,
            )
        }
        Widget::Card { id, title, children, elevated, .. } => {
            let c = children_json(children);
            let t = title.as_deref().unwrap_or("");
            format!(
                "{{ \"t\":\"card\", \"id\": {}, \"title\": {}, \"elevated\": {}, \"children\": {} }}",
                json_escape(id), json_escape(t), elevated, c,
            )
        }
        Widget::TextInput { id, label, value, placeholder, .. } => {
            format!(
                "{{ \"t\":\"text_input\", \"id\": {}, \"label\": {}, \"value\": {}, \"placeholder\": {} }}",
                json_escape(id), json_escape(label), json_escape(value), json_escape(placeholder),
            )
        }
        Widget::TextArea { id, label, value, rows, .. } => {
            format!(
                "{{ \"t\":\"textarea\", \"id\": {}, \"label\": {}, \"value\": {}, \"rows\": {} }}",
                json_escape(id), json_escape(label), json_escape(value), rows,
            )
        }
        Widget::Select { id, label, selected, .. } => {
            let sel = selected.as_deref().unwrap_or("");
            format!(
                "{{ \"t\":\"select\", \"id\": {}, \"label\": {}, \"selected\": {} }}",
                json_escape(id), json_escape(label), json_escape(sel),
            )
        }
        Widget::Checkbox { id, label, checked, .. } => {
            format!(
                "{{ \"t\":\"checkbox\", \"id\": {}, \"label\": {}, \"checked\": {} }}",
                json_escape(id), json_escape(label), checked,
            )
        }
        Widget::Tabs { id, labels, active, children, .. } => {
            let c = children_json(children);
            format!(
                "{{ \"t\":\"tabs\", \"id\": {}, \"labels\": {:?}, \"active\": {}, \"children\": {} }}",
                json_escape(id), labels, active, c,
            )
        }
        Widget::Table { id, headers, rows: _, sortable, .. } => {
            format!(
                "{{ \"t\":\"table\", \"id\": {}, \"headers\": {:?}, \"sortable\": {} }}",
                json_escape(id), headers, sortable,
            )
        }
        Widget::Dialog { id, title, children, open, .. } => {
            let c = children_json(children);
            format!(
                "{{ \"t\":\"dialog\", \"id\": {}, \"title\": {}, \"open\": {}, \"children\": {} }}",
                json_escape(id), json_escape(title), open, c,
            )
        }
        Widget::ProgressBar { id, value, max, .. } => {
            format!(
                "{{ \"t\":\"progress\", \"id\": {}, \"value\": {}, \"max\": {} }}",
                json_escape(id), value, max,
            )
        }
        Widget::Badge { id, text, color, .. } => {
            format!(
                "{{ \"t\":\"badge\", \"id\": {}, \"text\": {}, \"color\": {} }}",
                json_escape(id), json_escape(text), json_escape(color),
            )
        }
        Widget::Alert { id, message, severity, dismissible, .. } => {
            format!(
                "{{ \"t\":\"alert\", \"id\": {}, \"message\": {}, \"severity\": \"{:?}\", \"dismissible\": {} }}",
                json_escape(id), json_escape(message), severity, dismissible,
            )
        }
        // Catch-all for remaining widgets: emit type + id
        _ => {
            let id = widget_id(w).unwrap_or_default();
            let t = widget_type_tag(w);
            format!("{{ \"t\":{}, \"id\": {} }}", json_escape(t), json_escape(&id))
        }
    }
}

fn children_json(children: &[Widget]) -> String {
    let mut c = String::from("[");
    for (i, ch) in children.iter().enumerate() {
        if i > 0 { c.push_str(", "); }
        c.push_str(&widget_to_json(ch));
    }
    c.push(']');
    c
}

fn widget_id(w: &Widget) -> Option<String> {
    match w {
        Widget::Label { id, .. } | Widget::Button { id, .. } | Widget::Slider { id, .. }
        | Widget::Toggle { id, .. } | Widget::Icon { id, .. } | Widget::TextInput { id, .. }
        | Widget::TextArea { id, .. } | Widget::Select { id, .. } | Widget::Checkbox { id, .. }
        | Widget::RadioGroup { id, .. } | Widget::DatePicker { id, .. } | Widget::ColorPicker { id, .. }
        | Widget::FileUpload { id, .. } | Widget::Form { id, .. } | Widget::Table { id, .. }
        | Widget::List { id, .. } | Widget::Badge { id, .. } | Widget::ProgressBar { id, .. }
        | Widget::Spinner { id, .. } | Widget::Avatar { id, .. } | Widget::Tooltip { id, .. }
        | Widget::TreeView { id, .. } | Widget::Pagination { id, .. } | Widget::Tabs { id, .. }
        | Widget::Accordion { id, .. } | Widget::Breadcrumb { id, .. } | Widget::Menu { id, .. }
        | Widget::NavSidebar { id, .. } | Widget::Column { id, .. } | Widget::Row { id, .. }
        | Widget::Grid { id, .. } | Widget::Card { id, .. } | Widget::Divider { id, .. }
        | Widget::Spacer { id, .. } | Widget::ScrollView { id, .. } | Widget::Dialog { id, .. }
        | Widget::Snackbar { id, .. } | Widget::Alert { id, .. } | Widget::Image { id, .. }
        | Widget::Canvas { id, .. } => Some(id.clone()),
    }
}

fn widget_type_tag(w: &Widget) -> &'static str {
    match w {
        Widget::Label { .. } => "label", Widget::Button { .. } => "button",
        Widget::Slider { .. } => "slider", Widget::Toggle { .. } => "toggle",
        Widget::Icon { .. } => "icon", Widget::TextInput { .. } => "text_input",
        Widget::TextArea { .. } => "textarea", Widget::Select { .. } => "select",
        Widget::Checkbox { .. } => "checkbox", Widget::RadioGroup { .. } => "radio_group",
        Widget::DatePicker { .. } => "date_picker", Widget::ColorPicker { .. } => "color_picker",
        Widget::FileUpload { .. } => "file_upload", Widget::Form { .. } => "form",
        Widget::Table { .. } => "table", Widget::List { .. } => "list",
        Widget::Badge { .. } => "badge", Widget::ProgressBar { .. } => "progress",
        Widget::Spinner { .. } => "spinner", Widget::Avatar { .. } => "avatar",
        Widget::Tooltip { .. } => "tooltip", Widget::TreeView { .. } => "tree_view",
        Widget::Pagination { .. } => "pagination", Widget::Tabs { .. } => "tabs",
        Widget::Accordion { .. } => "accordion", Widget::Breadcrumb { .. } => "breadcrumb",
        Widget::Menu { .. } => "menu", Widget::NavSidebar { .. } => "nav_sidebar",
        Widget::Column { .. } => "column", Widget::Row { .. } => "row",
        Widget::Grid { .. } => "grid", Widget::Card { .. } => "card",
        Widget::Divider { .. } => "divider", Widget::Spacer { .. } => "spacer",
        Widget::ScrollView { .. } => "scroll_view", Widget::Dialog { .. } => "dialog",
        Widget::Snackbar { .. } => "snackbar", Widget::Alert { .. } => "alert",
        Widget::Image { .. } => "image", Widget::Canvas { .. } => "canvas",
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_roundtrip_shape() {
        let e = KillerUiEngine::example_parallel();
        let j = engine_to_json(&e);
        assert!(j.contains("\"killer_ui_engine_version\""));
        assert!(j.contains("\"sum\": 3"));
    }
}
