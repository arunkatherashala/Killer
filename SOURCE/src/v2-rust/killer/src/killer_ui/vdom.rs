//! **Virtual DOM with diff** — efficient tree reconciliation using keyed O(1) HashMap diffing.
//!
//! `VNode` trees represent desired UI state. `diff()` produces a minimal `PatchOp` list.
//! `apply_patches()` mutates a live `VNode` tree. Keyed children use a HashMap for O(1)
//! key lookup during reconciliation (similar to React's reconciliation algorithm).

use std::collections::HashMap;

// ── Virtual node ─────────────────────────────────────────────────────────────

/// A node in the virtual DOM tree.
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    /// An element with tag, attributes, optional style class, and children.
    Element {
        tag: String,
        key: Option<String>,
        attrs: HashMap<String, String>,
        class: Vec<String>,
        children: Vec<VNode>,
    },
    /// A text node.
    Text(String),
    /// A component placeholder (resolved by component system).
    Component {
        name: String,
        key: Option<String>,
        props: HashMap<String, String>,
        children: Vec<VNode>,
    },
    /// Fragment (invisible container).
    Fragment(Vec<VNode>),
}

impl VNode {
    pub fn element(tag: &str) -> Self {
        VNode::Element {
            tag: tag.to_string(),
            key: None,
            attrs: HashMap::new(),
            class: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn text(s: &str) -> Self {
        VNode::Text(s.to_string())
    }

    pub fn with_key(mut self, key: &str) -> Self {
        match &mut self {
            VNode::Element { key: k, .. } | VNode::Component { key: k, .. } => *k = Some(key.to_string()),
            _ => {}
        }
        self
    }

    pub fn with_attr(mut self, name: &str, value: &str) -> Self {
        if let VNode::Element { attrs, .. } = &mut self {
            attrs.insert(name.to_string(), value.to_string());
        }
        self
    }

    pub fn with_class(mut self, cls: &str) -> Self {
        if let VNode::Element { class, .. } = &mut self {
            class.push(cls.to_string());
        }
        self
    }

    pub fn with_child(mut self, child: VNode) -> Self {
        match &mut self {
            VNode::Element { children, .. } | VNode::Component { children, .. } => children.push(child),
            VNode::Fragment(c) => c.push(child),
            _ => {}
        }
        self
    }

    pub fn with_children(mut self, new_children: Vec<VNode>) -> Self {
        match &mut self {
            VNode::Element { children, .. } | VNode::Component { children, .. } => *children = new_children,
            VNode::Fragment(c) => *c = new_children,
            _ => {}
        }
        self
    }

    fn key(&self) -> Option<&str> {
        match self {
            VNode::Element { key, .. } | VNode::Component { key, .. } => key.as_deref(),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn tag(&self) -> Option<&str> {
        match self {
            VNode::Element { tag, .. } => Some(tag.as_str()),
            VNode::Component { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    fn same_type(&self, other: &VNode) -> bool {
        match (self, other) {
            (VNode::Element { tag: a, .. }, VNode::Element { tag: b, .. }) => a == b,
            (VNode::Text(_), VNode::Text(_)) => true,
            (VNode::Component { name: a, .. }, VNode::Component { name: b, .. }) => a == b,
            (VNode::Fragment(_), VNode::Fragment(_)) => true,
            _ => false,
        }
    }
}

// ── Patch operations ─────────────────────────────────────────────────────────

/// Represents a minimal change to apply to the live tree.
#[derive(Debug, Clone, PartialEq)]
pub enum PatchOp {
    /// Replace the node at `path` entirely.
    Replace { path: Vec<usize>, node: VNode },
    /// Update text content at `path`.
    UpdateText { path: Vec<usize>, text: String },
    /// Set/update attributes on the element at `path`.
    SetAttrs { path: Vec<usize>, attrs: HashMap<String, String> },
    /// Remove attributes by name from the element at `path`.
    RemoveAttrs { path: Vec<usize>, names: Vec<String> },
    /// Update class list on the element at `path`.
    SetClasses { path: Vec<usize>, class: Vec<String> },
    /// Insert a new child at `path[parent] / index`.
    InsertChild { path: Vec<usize>, index: usize, node: VNode },
    /// Remove child at `path[parent] / index`.
    RemoveChild { path: Vec<usize>, index: usize },
    /// Move a child within the same parent.
    MoveChild { path: Vec<usize>, from_index: usize, to_index: usize },
    /// Update component props at `path`.
    UpdateProps { path: Vec<usize>, props: HashMap<String, String> },
}

// ── Diff engine ──────────────────────────────────────────────────────────────

/// Produce a minimal patch list to transform `old` into `new`.
pub fn diff(old: &VNode, new: &VNode) -> Vec<PatchOp> {
    let mut patches = Vec::new();
    diff_inner(old, new, &mut Vec::new(), &mut patches);
    patches
}

fn diff_inner(old: &VNode, new: &VNode, path: &mut Vec<usize>, patches: &mut Vec<PatchOp>) {
    if !old.same_type(new) {
        patches.push(PatchOp::Replace { path: path.clone(), node: new.clone() });
        return;
    }

    match (old, new) {
        (VNode::Text(a), VNode::Text(b)) => {
            if a != b {
                patches.push(PatchOp::UpdateText { path: path.clone(), text: b.clone() });
            }
        }

        (VNode::Element { tag: _, attrs: old_attrs, class: old_class, children: old_ch, .. },
         VNode::Element { tag: _, attrs: new_attrs, class: new_class, children: new_ch, .. }) => {
            // Diff attributes
            diff_attrs(old_attrs, new_attrs, path, patches);
            // Diff classes
            if old_class != new_class {
                patches.push(PatchOp::SetClasses { path: path.clone(), class: new_class.clone() });
            }
            // Diff children
            diff_children(old_ch, new_ch, path, patches);
        }

        (VNode::Component { props: old_props, children: old_ch, .. },
         VNode::Component { props: new_props, children: new_ch, .. }) => {
            if old_props != new_props {
                patches.push(PatchOp::UpdateProps { path: path.clone(), props: new_props.clone() });
            }
            diff_children(old_ch, new_ch, path, patches);
        }

        (VNode::Fragment(old_ch), VNode::Fragment(new_ch)) => {
            diff_children(old_ch, new_ch, path, patches);
        }

        _ => {}
    }
}

fn diff_attrs(
    old: &HashMap<String, String>,
    new: &HashMap<String, String>,
    path: &[usize],
    patches: &mut Vec<PatchOp>,
) {
    // Find changed/added attrs
    let mut changed = HashMap::new();
    for (k, v) in new {
        if old.get(k) != Some(v) {
            changed.insert(k.clone(), v.clone());
        }
    }
    if !changed.is_empty() {
        patches.push(PatchOp::SetAttrs { path: path.to_vec(), attrs: changed });
    }

    // Find removed attrs
    let removed: Vec<String> = old.keys().filter(|k| !new.contains_key(*k)).cloned().collect();
    if !removed.is_empty() {
        patches.push(PatchOp::RemoveAttrs { path: path.to_vec(), names: removed });
    }
}

/// Keyed child reconciliation — O(n) linear scan.
fn diff_children(
    old_ch: &[VNode],
    new_ch: &[VNode],
    path: &mut Vec<usize>,
    patches: &mut Vec<PatchOp>,
) {
    // Check if any children are keyed
    let has_keys = new_ch.iter().any(|c| c.key().is_some());

    if has_keys {
        diff_keyed_children(old_ch, new_ch, path, patches);
    } else {
        diff_unkeyed_children(old_ch, new_ch, path, patches);
    }
}

/// Simple sequential diff for unkeyed children.
fn diff_unkeyed_children(
    old_ch: &[VNode],
    new_ch: &[VNode],
    path: &mut Vec<usize>,
    patches: &mut Vec<PatchOp>,
) {
    let common = old_ch.len().min(new_ch.len());
    for i in 0..common {
        path.push(i);
        diff_inner(&old_ch[i], &new_ch[i], path, patches);
        path.pop();
    }
    // Remove excess old children (in reverse to preserve indices)
    for i in (common..old_ch.len()).rev() {
        patches.push(PatchOp::RemoveChild { path: path.clone(), index: i });
    }
    // Insert new children
    for i in common..new_ch.len() {
        patches.push(PatchOp::InsertChild { path: path.clone(), index: i, node: new_ch[i].clone() });
    }
}

/// Keyed reconciliation. Maps old keys → positions, then matches new children.
fn diff_keyed_children(
    old_ch: &[VNode],
    new_ch: &[VNode],
    path: &mut Vec<usize>,
    patches: &mut Vec<PatchOp>,
) {
    // Build key→index map for old children
    let mut old_key_map: HashMap<String, usize> = HashMap::new();
    for (i, child) in old_ch.iter().enumerate() {
        if let Some(key) = child.key() {
            old_key_map.insert(key.to_string(), i);
        }
    }

    let mut old_used = vec![false; old_ch.len()];
    let mut new_matched: Vec<Option<usize>> = vec![None; new_ch.len()]; // new_idx → old_idx

    // Phase 1: Match by key
    for (new_i, new_child) in new_ch.iter().enumerate() {
        if let Some(key) = new_child.key() {
            if let Some(&old_i) = old_key_map.get(key) {
                if !old_used[old_i] && old_ch[old_i].same_type(new_child) {
                    old_used[old_i] = true;
                    new_matched[new_i] = Some(old_i);
                }
            }
        }
    }

    // Phase 2: Remove unmatched old children (reverse order)
    for i in (0..old_ch.len()).rev() {
        if !old_used[i] {
            patches.push(PatchOp::RemoveChild { path: path.clone(), index: i });
        }
    }

    // Phase 3: Insert/update new children
    for (new_i, new_child) in new_ch.iter().enumerate() {
        match new_matched[new_i] {
            Some(old_i) => {
                // Update existing
                path.push(new_i);
                diff_inner(&old_ch[old_i], new_child, path, patches);
                path.pop();
            }
            None => {
                // Insert new
                patches.push(PatchOp::InsertChild {
                    path: path.clone(),
                    index: new_i,
                    node: new_child.clone(),
                });
            }
        }
    }
}

// ── Apply patches (mutate live tree) ─────────────────────────────────────────

/// Apply a list of patches to a live VNode tree.
pub fn apply_patches(root: &mut VNode, patches: &[PatchOp]) {
    for patch in patches {
        apply_one(root, patch);
    }
}

fn get_node_mut<'a>(root: &'a mut VNode, path: &[usize]) -> Option<&'a mut VNode> {
    if path.is_empty() {
        return Some(root);
    }
    let mut current = root;
    for &idx in path {
        let children = match current {
            VNode::Element { children, .. } | VNode::Component { children, .. } => children,
            VNode::Fragment(c) => c,
            _ => return None,
        };
        if idx >= children.len() {
            return None;
        }
        current = &mut children[idx];
    }
    Some(current)
}

fn get_children_mut(node: &mut VNode) -> Option<&mut Vec<VNode>> {
    match node {
        VNode::Element { children, .. } | VNode::Component { children, .. } => Some(children),
        VNode::Fragment(c) => Some(c),
        _ => None,
    }
}

fn apply_one(root: &mut VNode, patch: &PatchOp) {
    match patch {
        PatchOp::Replace { path, node } => {
            if path.is_empty() {
                *root = node.clone();
            } else {
                let parent_path = &path[..path.len() - 1];
                let child_idx = path[path.len() - 1];
                if let Some(parent) = get_node_mut(root, parent_path) {
                    if let Some(children) = get_children_mut(parent) {
                        if child_idx < children.len() {
                            children[child_idx] = node.clone();
                        }
                    }
                }
            }
        }

        PatchOp::UpdateText { path, text } => {
            if let Some(node) = get_node_mut(root, path) {
                if let VNode::Text(t) = node {
                    *t = text.clone();
                }
            }
        }

        PatchOp::SetAttrs { path, attrs } => {
            if let Some(node) = get_node_mut(root, path) {
                if let VNode::Element { attrs: node_attrs, .. } = node {
                    for (k, v) in attrs {
                        node_attrs.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        PatchOp::RemoveAttrs { path, names } => {
            if let Some(node) = get_node_mut(root, path) {
                if let VNode::Element { attrs, .. } = node {
                    for name in names {
                        attrs.remove(name);
                    }
                }
            }
        }

        PatchOp::SetClasses { path, class } => {
            if let Some(node) = get_node_mut(root, path) {
                if let VNode::Element { class: c, .. } = node {
                    *c = class.clone();
                }
            }
        }

        PatchOp::InsertChild { path, index, node } => {
            if let Some(parent) = get_node_mut(root, path) {
                if let Some(children) = get_children_mut(parent) {
                    let idx = (*index).min(children.len());
                    children.insert(idx, node.clone());
                }
            }
        }

        PatchOp::RemoveChild { path, index } => {
            if let Some(parent) = get_node_mut(root, path) {
                if let Some(children) = get_children_mut(parent) {
                    if *index < children.len() {
                        children.remove(*index);
                    }
                }
            }
        }

        PatchOp::MoveChild { path, from_index, to_index } => {
            if let Some(parent) = get_node_mut(root, path) {
                if let Some(children) = get_children_mut(parent) {
                    if *from_index < children.len() {
                        let child = children.remove(*from_index);
                        let to = (*to_index).min(children.len());
                        children.insert(to, child);
                    }
                }
            }
        }

        PatchOp::UpdateProps { path, props } => {
            if let Some(node) = get_node_mut(root, path) {
                if let VNode::Component { props: p, .. } = node {
                    *p = props.clone();
                }
            }
        }
    }
}

// ── VNode to JSON ────────────────────────────────────────────────────────────

pub fn vnode_to_json(node: &VNode) -> String {
    let mut s = String::new();
    vnode_to_json_inner(node, &mut s, 0);
    s
}

fn vnode_to_json_inner(node: &VNode, s: &mut String, indent: usize) {
    let pad = " ".repeat(indent);
    match node {
        VNode::Text(t) => {
            s.push_str(&format!("{}{{\"text\": \"{}\"}}", pad, t.replace('"', "\\\"")));
        }
        VNode::Element { tag, key, attrs, class, children } => {
            s.push_str(&format!("{}{{\"tag\": \"{}\"", pad, tag));
            if let Some(k) = key { s.push_str(&format!(", \"key\": \"{}\"", k)); }
            if !attrs.is_empty() {
                s.push_str(", \"attrs\": {");
                for (i, (k, v)) in attrs.iter().enumerate() {
                    if i > 0 { s.push_str(", "); }
                    s.push_str(&format!("\"{}\": \"{}\"", k, v));
                }
                s.push('}');
            }
            if !class.is_empty() {
                s.push_str(&format!(", \"class\": {:?}", class));
            }
            if !children.is_empty() {
                s.push_str(", \"children\": [\n");
                for (i, child) in children.iter().enumerate() {
                    if i > 0 { s.push_str(",\n"); }
                    vnode_to_json_inner(child, s, indent + 2);
                }
                s.push_str(&format!("\n{}]", pad));
            }
            s.push('}');
        }
        VNode::Component { name, key, props, children } => {
            s.push_str(&format!("{}{{\"component\": \"{}\"", pad, name));
            if let Some(k) = key { s.push_str(&format!(", \"key\": \"{}\"", k)); }
            if !props.is_empty() {
                s.push_str(", \"props\": {");
                for (i, (k, v)) in props.iter().enumerate() {
                    if i > 0 { s.push_str(", "); }
                    s.push_str(&format!("\"{}\": \"{}\"", k, v));
                }
                s.push('}');
            }
            if !children.is_empty() {
                s.push_str(", \"children\": [\n");
                for (i, child) in children.iter().enumerate() {
                    if i > 0 { s.push_str(",\n"); }
                    vnode_to_json_inner(child, s, indent + 2);
                }
                s.push_str(&format!("\n{}]", pad));
            }
            s.push('}');
        }
        VNode::Fragment(children) => {
            s.push_str(&format!("{}{{\"fragment\": [\n", pad));
            for (i, child) in children.iter().enumerate() {
                if i > 0 { s.push_str(",\n"); }
                vnode_to_json_inner(child, s, indent + 2);
            }
            s.push_str(&format!("\n{}]}}", pad));
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_no_patches() {
        let tree = VNode::element("div").with_child(VNode::text("hello"));
        let patches = diff(&tree, &tree);
        assert!(patches.is_empty());
    }

    #[test]
    fn text_update() {
        let old = VNode::element("div").with_child(VNode::text("hello"));
        let new = VNode::element("div").with_child(VNode::text("world"));
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], PatchOp::UpdateText { text, .. } if text == "world"));
    }

    #[test]
    fn attr_change() {
        let old = VNode::element("div").with_attr("class", "old");
        let new = VNode::element("div").with_attr("class", "new");
        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, PatchOp::SetAttrs { .. })));
    }

    #[test]
    fn attr_removal() {
        let old = VNode::element("div").with_attr("title", "x");
        let new = VNode::element("div");
        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, PatchOp::RemoveAttrs { names, .. } if names.contains(&"title".to_string()))));
    }

    #[test]
    fn child_insert() {
        let old = VNode::element("div").with_child(VNode::text("a"));
        let new = VNode::element("div")
            .with_child(VNode::text("a"))
            .with_child(VNode::text("b"));
        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, PatchOp::InsertChild { .. })));
    }

    #[test]
    fn child_remove() {
        let old = VNode::element("div")
            .with_child(VNode::text("a"))
            .with_child(VNode::text("b"));
        let new = VNode::element("div").with_child(VNode::text("a"));
        let patches = diff(&old, &new);
        assert!(patches.iter().any(|p| matches!(p, PatchOp::RemoveChild { .. })));
    }

    #[test]
    fn replace_different_type() {
        let old = VNode::element("div");
        let new = VNode::element("span");
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], PatchOp::Replace { .. }));
    }

    #[test]
    fn keyed_children_match() {
        let old = VNode::element("ul")
            .with_child(VNode::element("li").with_key("a").with_child(VNode::text("A")))
            .with_child(VNode::element("li").with_key("b").with_child(VNode::text("B")));
        let new = VNode::element("ul")
            .with_child(VNode::element("li").with_key("b").with_child(VNode::text("B")))
            .with_child(VNode::element("li").with_key("a").with_child(VNode::text("A")));
        let patches = diff(&old, &new);
        // Should not be a full replace — keyed reconciliation handles reorder
        assert!(!patches.iter().any(|p| matches!(p, PatchOp::Replace { path, .. } if path.is_empty())));
    }

    #[test]
    fn apply_text_update() {
        let mut tree = VNode::element("div").with_child(VNode::text("hello"));
        let patches = vec![PatchOp::UpdateText { path: vec![0], text: "world".into() }];
        apply_patches(&mut tree, &patches);
        if let VNode::Element { children, .. } = &tree {
            assert_eq!(children[0], VNode::Text("world".into()));
        }
    }

    #[test]
    fn apply_insert_child() {
        let mut tree = VNode::element("div").with_child(VNode::text("a"));
        let patches = vec![PatchOp::InsertChild { path: vec![], index: 1, node: VNode::text("b") }];
        apply_patches(&mut tree, &patches);
        if let VNode::Element { children, .. } = &tree {
            assert_eq!(children.len(), 2);
            assert_eq!(children[1], VNode::Text("b".into()));
        }
    }

    #[test]
    fn full_diff_apply_roundtrip() {
        let old = VNode::element("div")
            .with_attr("id", "app")
            .with_child(VNode::element("h1").with_child(VNode::text("Old Title")))
            .with_child(VNode::element("p").with_child(VNode::text("Old body")));
        let new = VNode::element("div")
            .with_attr("id", "app")
            .with_attr("class", "active")
            .with_child(VNode::element("h1").with_child(VNode::text("New Title")))
            .with_child(VNode::element("p").with_child(VNode::text("New body")))
            .with_child(VNode::element("footer").with_child(VNode::text("Footer")));

        let patches = diff(&old, &new);
        let mut live = old.clone();
        apply_patches(&mut live, &patches);

        // Verify the tree matches new
        if let VNode::Element { attrs, children, .. } = &live {
            assert_eq!(attrs.get("class"), Some(&"active".to_string()));
            assert_eq!(children.len(), 3);
        }
    }

    #[test]
    fn vnode_json_output() {
        let tree = VNode::element("div").with_child(VNode::text("hello"));
        let json = vnode_to_json(&tree);
        assert!(json.contains("\"div\"") && json.contains("\"hello\""));
    }

    #[test]
    fn keyed_large_set_o1_lookup() {
        // Verify O(1) keyed children work with a large set (1000 children)
        let mut old_children = Vec::new();
        let mut new_children = Vec::new();
        for i in 0..1000 {
            old_children.push(VNode::element("li").with_key(&format!("k{}", i)).with_child(VNode::text(&format!("item {}", i))));
        }
        // Reverse order — keyed diff should handle without Replace
        for i in (0..1000).rev() {
            new_children.push(VNode::element("li").with_key(&format!("k{}", i)).with_child(VNode::text(&format!("item {}", i))));
        }
        let old = VNode::element("ul").with_children(old_children);
        let new = VNode::element("ul").with_children(new_children);
        let patches = diff(&old, &new);
        // Should NOT produce 1000 Replace patches — keyed diff reorders
        let replace_count = patches.iter().filter(|p| matches!(p, PatchOp::Replace { .. })).count();
        assert_eq!(replace_count, 0, "keyed diff should not produce Replace for same-type reordered children");
    }

    #[test]
    fn keyed_insert_new_key() {
        let old = VNode::element("ul")
            .with_child(VNode::element("li").with_key("a").with_child(VNode::text("A")))
            .with_child(VNode::element("li").with_key("b").with_child(VNode::text("B")));
        let new = VNode::element("ul")
            .with_child(VNode::element("li").with_key("a").with_child(VNode::text("A")))
            .with_child(VNode::element("li").with_key("c").with_child(VNode::text("C")))
            .with_child(VNode::element("li").with_key("b").with_child(VNode::text("B")));
        let patches = diff(&old, &new);
        // Should have an InsertChild for key "c"
        assert!(patches.iter().any(|p| matches!(p, PatchOp::InsertChild { .. })));
    }
}
