//! **Flexbox layout engine** — CSS Flexbox algorithm for positioning widgets.
//!
//! Implements the core flexbox spec: main/cross axis, flex-grow/shrink, justify-content,
//! align-items, wrapping, and gap. Runs purely on [`LayoutNode`] trees — no rendering.

use super::style::{AlignItems, Display, FlexDirection, FlexWrap, JustifyContent, BoxEdges, Unit};

// ── Computed layout result ───────────────────────────────────────────────────

/// Final computed position and size for a layout node.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ComputedLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// ── Layout node (input tree) ─────────────────────────────────────────────────

/// A node in the layout tree. Each node has constraints and children.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub display: Display,
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub justify: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: Option<AlignItems>,
    pub gap: f64,

    // Sizing
    pub width: Unit,
    pub height: Unit,
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,

    // Flex
    pub flex_grow: f64,
    pub flex_shrink: f64,
    pub flex_basis: Unit,

    // Box model
    pub margin: BoxEdges,
    pub padding: BoxEdges,

    pub children: Vec<LayoutNode>,

    /// Computed result (filled after layout pass).
    pub computed: ComputedLayout,
}

impl Default for LayoutNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            display: Display::Flex,
            direction: FlexDirection::Column,
            wrap: FlexWrap::NoWrap,
            justify: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_self: None,
            gap: 0.0,
            width: Unit::Auto,
            height: Unit::Auto,
            min_width: 0.0,
            min_height: 0.0,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Unit::Auto,
            margin: BoxEdges::ZERO,
            padding: BoxEdges::ZERO,
            children: Vec::new(),
            computed: ComputedLayout::default(),
        }
    }
}

impl LayoutNode {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string(), ..Default::default() }
    }

    pub fn with_size(mut self, w: f64, h: f64) -> Self {
        self.width = Unit::Px(w);
        self.height = Unit::Px(h);
        self
    }

    pub fn with_flex_grow(mut self, v: f64) -> Self {
        self.flex_grow = v;
        self
    }

    pub fn with_direction(mut self, d: FlexDirection) -> Self {
        self.direction = d;
        self
    }

    pub fn with_justify(mut self, j: JustifyContent) -> Self {
        self.justify = j;
        self
    }

    pub fn with_align(mut self, a: AlignItems) -> Self {
        self.align_items = a;
        self
    }

    pub fn with_padding(mut self, p: BoxEdges) -> Self {
        self.padding = p;
        self
    }

    pub fn with_margin(mut self, m: BoxEdges) -> Self {
        self.margin = m;
        self
    }

    pub fn with_gap(mut self, g: f64) -> Self {
        self.gap = g;
        self
    }

    pub fn with_wrap(mut self, w: FlexWrap) -> Self {
        self.wrap = w;
        self
    }

    pub fn add_child(mut self, child: LayoutNode) -> Self {
        self.children.push(child);
        self
    }
}

// ── Layout engine ────────────────────────────────────────────────────────────

/// Perform layout computation on the root node given available `container_width` × `container_height`.
pub fn compute_layout(root: &mut LayoutNode, container_width: f64, container_height: f64) {
    root.computed.x = 0.0;
    root.computed.y = 0.0;
    root.computed.width = resolve_size(root.width, container_width, root.min_width, root.max_width, container_width);
    root.computed.height = resolve_size(root.height, container_height, root.min_height, root.max_height, container_height);
    layout_flex(root);
}

fn resolve_size(unit: Unit, parent: f64, min: f64, max: f64, fallback: f64) -> f64 {
    let raw = match unit {
        Unit::Px(v) => v,
        Unit::Percent(v) => parent * v / 100.0,
        Unit::Auto => fallback,
        _ => fallback,
    };
    raw.max(min).min(max)
}

/// Core flexbox algorithm (with wrap support).
fn layout_flex(node: &mut LayoutNode) {
    if node.children.is_empty() || matches!(node.display, Display::None) {
        return;
    }

    let is_row = matches!(node.direction, FlexDirection::Row | FlexDirection::RowReverse);
    let is_reverse = matches!(node.direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);
    let do_wrap = !matches!(node.wrap, FlexWrap::NoWrap);

    let inner_width = (node.computed.width - node.padding.horizontal()).max(0.0);
    let inner_height = (node.computed.height - node.padding.vertical()).max(0.0);

    let main_size = if is_row { inner_width } else { inner_height };
    let cross_size = if is_row { inner_height } else { inner_width };

    // ── Step 1: Determine base main sizes ────────────────────────────────
    let child_count = node.children.len();
    let _total_gap = if child_count > 1 { node.gap * (child_count - 1) as f64 } else { 0.0 };

    struct FlexItem {
        base_main: f64,
        base_cross: f64,
        flex_grow: f64,
        flex_shrink: f64,
        margin_main: f64,
        margin_cross: f64,
        min_main: f64,
        max_main: f64,
    }

    let mut items: Vec<FlexItem> = Vec::with_capacity(child_count);
    for child in &node.children {
        let base_main = if is_row {
            resolve_flex_basis(child, true, inner_width)
        } else {
            resolve_flex_basis(child, false, inner_height)
        };
        let base_cross = if is_row {
            resolve_size(child.height, inner_height, child.min_height, child.max_height, 0.0)
        } else {
            resolve_size(child.width, inner_width, child.min_width, child.max_width, 0.0)
        };
        let (margin_main, margin_cross) = if is_row {
            (child.margin.horizontal(), child.margin.vertical())
        } else {
            (child.margin.vertical(), child.margin.horizontal())
        };
        let (min_main, max_main) = if is_row {
            (child.min_width, child.max_width)
        } else {
            (child.min_height, child.max_height)
        };
        items.push(FlexItem {
            base_main,
            base_cross,
            flex_grow: child.flex_grow,
            flex_shrink: child.flex_shrink,
            margin_main,
            margin_cross,
            min_main,
            max_main,
        });
    }

    // ── Step 2: Break into wrap lines (if wrapping enabled) ────────────
    let mut lines: Vec<Vec<usize>> = Vec::new();
    if do_wrap {
        let mut line: Vec<usize> = Vec::new();
        let mut line_main = 0.0_f64;
        for i in 0..child_count {
            let item_main = items[i].base_main + items[i].margin_main;
            let gap_contrib = if line.is_empty() { 0.0 } else { node.gap };
            if !line.is_empty() && line_main + gap_contrib + item_main > main_size {
                lines.push(std::mem::take(&mut line));
                line_main = 0.0;
            }
            if !line.is_empty() { line_main += node.gap; }
            line_main += item_main;
            line.push(i);
        }
        if !line.is_empty() { lines.push(line); }
    } else {
        lines.push((0..child_count).collect());
    }
    if matches!(node.wrap, FlexWrap::WrapReverse) { lines.reverse(); }

    // ── Step 3: Per-line flex distribution + positioning ──────────────────
    let line_count = lines.len();
    let cross_per_line = if line_count > 0 { cross_size / line_count as f64 } else { cross_size };
    let pad_main_start = if is_row { node.padding.left } else { node.padding.top };
    let pad_cross_start = if is_row { node.padding.top } else { node.padding.left };
    let mut cross_offset = 0.0_f64;

    for line in &lines {
        let lc = line.len();
        let line_gap = if lc > 1 { node.gap * (lc - 1) as f64 } else { 0.0 };

        // Compute main sizes with grow/shrink
        let mut main_sizes: Vec<f64> = line.iter().map(|&i| items[i].base_main).collect();
        let total_base: f64 = line.iter().enumerate().map(|(li, &i)| main_sizes[li] + items[i].margin_main).sum::<f64>() + line_gap;
        let free = main_size - total_base;
        if free > 0.0 {
            let tg: f64 = line.iter().map(|&i| items[i].flex_grow).sum();
            if tg > 0.0 {
                for (li, &i) in line.iter().enumerate() {
                    main_sizes[li] = (main_sizes[li] + items[i].flex_grow / tg * free).min(items[i].max_main).max(items[i].min_main);
                }
            }
        } else if free < 0.0 {
            let tsw: f64 = line.iter().enumerate().map(|(li, &i)| items[i].flex_shrink * main_sizes[li]).sum();
            if tsw > 0.0 {
                let over = -free;
                for (li, &i) in line.iter().enumerate() {
                    let sh = (items[i].flex_shrink * main_sizes[li]) / tsw * over;
                    main_sizes[li] = (main_sizes[li] - sh).max(items[i].min_main);
                }
            }
        }

        // Cross sizes per item in this line
        let line_cross = if do_wrap { cross_per_line } else { cross_size };
        let cross_sizes: Vec<f64> = line.iter().map(|&i| {
            let align = node.children[i].align_self.unwrap_or(node.align_items);
            if matches!(align, AlignItems::Stretch) && items[i].base_cross == 0.0 {
                (line_cross - items[i].margin_cross).max(0.0)
            } else if items[i].base_cross > 0.0 {
                items[i].base_cross
            } else {
                0.0
            }
        }).collect();

        // Justify (main-axis)
        let used: f64 = main_sizes.iter().sum::<f64>() + line.iter().map(|&i| items[i].margin_main).sum::<f64>() + line_gap;
        let remaining = (main_size - used).max(0.0);
        let (mut off_main, gap_extra) = match node.justify {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::End => (remaining, 0.0),
            JustifyContent::Center => (remaining / 2.0, 0.0),
            JustifyContent::SpaceBetween => if lc > 1 { (0.0, remaining / (lc - 1) as f64) } else { (0.0, 0.0) },
            JustifyContent::SpaceAround => { let g = remaining / lc as f64; (g / 2.0, g) },
            JustifyContent::SpaceEvenly => { let g = remaining / (lc + 1) as f64; (g, g) },
        };

        let order: Vec<usize> = if is_reverse { (0..lc).rev().collect() } else { (0..lc).collect() };
        for &li in &order {
            let i = line[li];
            let mm_before = if is_row { node.children[i].margin.left } else { node.children[i].margin.top };
            let mc_before = if is_row { node.children[i].margin.top } else { node.children[i].margin.left };
            let mm_after = if is_row { node.children[i].margin.right } else { node.children[i].margin.bottom };

            let main_pos = pad_main_start + off_main + mm_before;
            let align = node.children[i].align_self.unwrap_or(node.align_items);
            let cross_pos = pad_cross_start + cross_offset + mc_before + match align {
                AlignItems::Start | AlignItems::Stretch | AlignItems::Baseline => 0.0,
                AlignItems::End => (line_cross - cross_sizes[li] - items[i].margin_cross).max(0.0),
                AlignItems::Center => ((line_cross - cross_sizes[li] - items[i].margin_cross) / 2.0).max(0.0),
            };

            let (x, y, w, h) = if is_row {
                (main_pos, cross_pos, main_sizes[li], cross_sizes[li])
            } else {
                (cross_pos, main_pos, cross_sizes[li], main_sizes[li])
            };
            node.children[i].computed = ComputedLayout { x, y, width: w, height: h };
            off_main += mm_before + main_sizes[li] + mm_after + node.gap + gap_extra;
            layout_flex(&mut node.children[i]);
        }
        cross_offset += line_cross;
    }
}

fn resolve_flex_basis(child: &LayoutNode, is_row: bool, parent_main: f64) -> f64 {
    let basis = &child.flex_basis;
    match basis {
        Unit::Px(v) => *v,
        Unit::Percent(v) => parent_main * v / 100.0,
        Unit::Auto => {
            // Fall back to explicit width/height
            let size = if is_row { child.width } else { child.height };
            match size {
                Unit::Px(v) => v,
                Unit::Percent(v) => parent_main * v / 100.0,
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

// ── JSON output ──────────────────────────────────────────────────────────────

/// Serialize the layout tree as JSON (for devtools / headless rendering).
pub fn layout_to_json(node: &LayoutNode) -> String {
    let mut s = String::new();
    layout_to_json_inner(node, &mut s, 0);
    s
}

fn layout_to_json_inner(node: &LayoutNode, s: &mut String, indent: usize) {
    let pad = " ".repeat(indent);
    s.push_str(&format!("{}{{\"id\": \"{}\", \"x\": {:.1}, \"y\": {:.1}, \"w\": {:.1}, \"h\": {:.1}",
        pad, node.id, node.computed.x, node.computed.y, node.computed.width, node.computed.height));
    if !node.children.is_empty() {
        s.push_str(", \"children\": [\n");
        for (i, child) in node.children.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            layout_to_json_inner(child, s, indent + 2);
        }
        s.push_str(&format!("\n{}]", pad));
    }
    s.push('}');
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_column_layout() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 600.0)
            .add_child(LayoutNode::new("a").with_size(400.0, 100.0))
            .add_child(LayoutNode::new("b").with_size(400.0, 200.0));
        compute_layout(&mut root, 400.0, 600.0);
        assert_eq!(root.children[0].computed.y, 0.0);
        assert_eq!(root.children[0].computed.height, 100.0);
        assert_eq!(root.children[1].computed.y, 100.0);
        assert_eq!(root.children[1].computed.height, 200.0);
    }

    #[test]
    fn row_layout() {
        let mut root = LayoutNode::new("root")
            .with_size(600.0, 100.0)
            .with_direction(FlexDirection::Row)
            .add_child(LayoutNode::new("a").with_size(200.0, 50.0))
            .add_child(LayoutNode::new("b").with_size(300.0, 50.0));
        compute_layout(&mut root, 600.0, 100.0);
        assert_eq!(root.children[0].computed.x, 0.0);
        assert_eq!(root.children[0].computed.width, 200.0);
        assert_eq!(root.children[1].computed.x, 200.0);
        assert_eq!(root.children[1].computed.width, 300.0);
    }

    #[test]
    fn flex_grow_distributes_space() {
        let mut root = LayoutNode::new("root")
            .with_size(600.0, 100.0)
            .with_direction(FlexDirection::Row)
            .add_child(LayoutNode::new("a").with_size(100.0, 50.0).with_flex_grow(1.0))
            .add_child(LayoutNode::new("b").with_size(100.0, 50.0).with_flex_grow(2.0));
        compute_layout(&mut root, 600.0, 100.0);
        // 400px free space: a gets 133.3, b gets 266.7
        let a_w = root.children[0].computed.width;
        let b_w = root.children[1].computed.width;
        assert!((a_w - 233.3).abs() < 1.0); // 100 + 133.3
        assert!((b_w - 366.7).abs() < 1.0); // 100 + 266.7
    }

    #[test]
    fn justify_center() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 100.0)
            .with_direction(FlexDirection::Row)
            .with_justify(JustifyContent::Center)
            .add_child(LayoutNode::new("a").with_size(100.0, 50.0));
        compute_layout(&mut root, 400.0, 100.0);
        assert!((root.children[0].computed.x - 150.0).abs() < 1.0);
    }

    #[test]
    fn justify_space_between() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 100.0)
            .with_direction(FlexDirection::Row)
            .with_justify(JustifyContent::SpaceBetween)
            .add_child(LayoutNode::new("a").with_size(50.0, 50.0))
            .add_child(LayoutNode::new("b").with_size(50.0, 50.0));
        compute_layout(&mut root, 400.0, 100.0);
        assert_eq!(root.children[0].computed.x, 0.0);
        assert!((root.children[1].computed.x - 350.0).abs() < 1.0);
    }

    #[test]
    fn align_items_center() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 200.0)
            .with_direction(FlexDirection::Row)
            .with_align(AlignItems::Center)
            .add_child(LayoutNode::new("a").with_size(100.0, 50.0));
        compute_layout(&mut root, 400.0, 200.0);
        assert!((root.children[0].computed.y - 75.0).abs() < 1.0);
    }

    #[test]
    fn stretch_cross_axis() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 200.0)
            .with_direction(FlexDirection::Row)
            .with_align(AlignItems::Stretch)
            .add_child(LayoutNode::new("a").with_size(100.0, 0.0)); // height=auto
        root.children[0].height = Unit::Auto;
        compute_layout(&mut root, 400.0, 200.0);
        assert_eq!(root.children[0].computed.height, 200.0);
    }

    #[test]
    fn gap_between_children() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 400.0)
            .with_gap(10.0)
            .add_child(LayoutNode::new("a").with_size(400.0, 50.0))
            .add_child(LayoutNode::new("b").with_size(400.0, 50.0));
        compute_layout(&mut root, 400.0, 400.0);
        assert_eq!(root.children[1].computed.y, 60.0); // 50 + 10 gap
    }

    #[test]
    fn padding_offsets_children() {
        let mut root = LayoutNode::new("root")
            .with_size(400.0, 400.0)
            .with_padding(BoxEdges::all(20.0))
            .add_child(LayoutNode::new("a").with_size(100.0, 50.0));
        compute_layout(&mut root, 400.0, 400.0);
        assert_eq!(root.children[0].computed.x, 20.0);
        assert_eq!(root.children[0].computed.y, 20.0);
    }

    #[test]
    fn nested_layout() {
        let mut root = LayoutNode::new("root")
            .with_size(800.0, 600.0)
            .with_direction(FlexDirection::Row)
            .add_child(
                LayoutNode::new("sidebar").with_size(200.0, 600.0)
                    .add_child(LayoutNode::new("nav").with_size(200.0, 300.0))
                    .add_child(LayoutNode::new("ads").with_size(200.0, 300.0))
            )
            .add_child(
                LayoutNode::new("main").with_size(600.0, 600.0)
            );
        compute_layout(&mut root, 800.0, 600.0);
        assert_eq!(root.children[0].computed.x, 0.0);
        assert_eq!(root.children[1].computed.x, 200.0);
        // Nested children positioned within sidebar
        assert_eq!(root.children[0].children[0].computed.y, 0.0);
        assert_eq!(root.children[0].children[1].computed.y, 300.0);
    }

    #[test]
    fn layout_to_json_works() {
        let mut root = LayoutNode::new("root")
            .with_size(100.0, 100.0)
            .add_child(LayoutNode::new("child").with_size(50.0, 50.0));
        compute_layout(&mut root, 100.0, 100.0);
        let json = layout_to_json(&root);
        assert!(json.contains("\"root\""));
        assert!(json.contains("\"child\""));
    }

    #[test]
    fn flex_wrap_creates_lines() {
        let mut root = LayoutNode::new("root")
            .with_size(200.0, 200.0)
            .with_direction(FlexDirection::Row)
            .with_wrap(FlexWrap::Wrap)
            .add_child(LayoutNode::new("a").with_size(120.0, 40.0))
            .add_child(LayoutNode::new("b").with_size(120.0, 40.0));
        compute_layout(&mut root, 200.0, 200.0);
        // b should wrap to line 2 (y > 0)
        assert_eq!(root.children[0].computed.y, 0.0);
        assert!(root.children[1].computed.y > 0.0, "b should be on line 2");
    }

    #[test]
    fn grid_layout_basic() {
        let grid = GridLayout::new(3, 2, 10.0, 10.0);
        let cells = grid.compute(600.0, 400.0);
        assert_eq!(cells.len(), 6);
        assert!((cells[0].width - 193.3).abs() < 1.0); // (600 - 2*10) / 3
        assert!((cells[3].y - 205.0).abs() < 1.0); // row1_y = 195 + 10
    }

    #[test]
    fn responsive_breakpoint() {
        let mut bp = BreakpointSet::new();
        bp.add("sm", 0.0);
        bp.add("md", 768.0);
        bp.add("lg", 1024.0);
        assert_eq!(bp.active(500.0), "sm");
        assert_eq!(bp.active(800.0), "md");
        assert_eq!(bp.active(1200.0), "lg");
    }
}

// ── CSS Grid layout ──────────────────────────────────────────────────────────

/// Simple CSS Grid: fixed column/row count with gap.
#[derive(Debug, Clone)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: usize,
    pub column_gap: f64,
    pub row_gap: f64,
    /// Optional explicit column widths (fr units). Empty = equal.
    pub column_tracks: Vec<f64>,
    /// Optional explicit row heights (fr units). Empty = equal.
    pub row_tracks: Vec<f64>,
}

/// A computed grid cell position.
#[derive(Debug, Clone, Copy)]
pub struct GridCell {
    pub col: usize,
    pub row: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl GridLayout {
    pub fn new(columns: usize, rows: usize, column_gap: f64, row_gap: f64) -> Self {
        Self { columns: columns.max(1), rows: rows.max(1), column_gap, row_gap, column_tracks: Vec::new(), row_tracks: Vec::new() }
    }

    /// Compute cell positions for the given container size.
    pub fn compute(&self, container_w: f64, container_h: f64) -> Vec<GridCell> {
        let col_widths = resolve_tracks(&self.column_tracks, self.columns, container_w, self.column_gap);
        let row_heights = resolve_tracks(&self.row_tracks, self.rows, container_h, self.row_gap);

        let mut cells = Vec::with_capacity(self.columns * self.rows);
        let mut cy = 0.0_f64;
        for r in 0..self.rows {
            let mut cx = 0.0_f64;
            for c in 0..self.columns {
                cells.push(GridCell { col: c, row: r, x: cx, y: cy, width: col_widths[c], height: row_heights[r] });
                cx += col_widths[c] + self.column_gap;
            }
            cy += row_heights[r] + self.row_gap;
        }
        cells
    }

    /// Lay out LayoutNodes into grid cells, filling row-major.
    pub fn layout_children(&self, children: &mut [LayoutNode], container_w: f64, container_h: f64) {
        let cells = self.compute(container_w, container_h);
        for (i, child) in children.iter_mut().enumerate() {
            if i < cells.len() {
                child.computed = ComputedLayout { x: cells[i].x, y: cells[i].y, width: cells[i].width, height: cells[i].height };
                layout_flex(child);
            }
        }
    }
}

fn resolve_tracks(tracks: &[f64], count: usize, total: f64, gap: f64) -> Vec<f64> {
    let avail = total - gap * (count.saturating_sub(1)) as f64;
    if tracks.is_empty() {
        let w = avail / count as f64;
        vec![w; count]
    } else {
        let fr_total: f64 = tracks.iter().sum();
        if fr_total <= 0.0 {
            vec![avail / count as f64; count]
        } else {
            let mut out: Vec<f64> = tracks.iter().map(|&fr| avail * fr / fr_total).collect();
            while out.len() < count { out.push(avail / count as f64); }
            out.truncate(count);
            out
        }
    }
}

// ── Absolute / Fixed positioning ─────────────────────────────────────────────

/// Position an absolutely-positioned node relative to a container.
pub fn apply_absolute_position(node: &mut LayoutNode, container_w: f64, container_h: f64,
    left: Option<f64>, top: Option<f64>, right: Option<f64>, bottom: Option<f64>) {
    let w = resolve_size(node.width, container_w, node.min_width, node.max_width, 0.0);
    let h = resolve_size(node.height, container_h, node.min_height, node.max_height, 0.0);
    let x = if let Some(l) = left { l }
            else if let Some(r) = right { container_w - w - r }
            else { 0.0 };
    let y = if let Some(t) = top { t }
            else if let Some(b) = bottom { container_h - h - b }
            else { 0.0 };
    node.computed = ComputedLayout { x, y, width: w, height: h };
    layout_flex(node);
}

// ── Responsive breakpoints ───────────────────────────────────────────────────

/// Named breakpoints with min-width thresholds (like CSS media queries).
#[derive(Debug, Clone)]
pub struct BreakpointSet {
    breakpoints: Vec<(String, f64)>, // sorted ascending by threshold
}

impl BreakpointSet {
    pub fn new() -> Self { Self { breakpoints: Vec::new() } }

    /// Standard Material Design breakpoints.
    pub fn material() -> Self {
        let mut bp = Self::new();
        bp.add("xs", 0.0);
        bp.add("sm", 600.0);
        bp.add("md", 960.0);
        bp.add("lg", 1280.0);
        bp.add("xl", 1920.0);
        bp
    }

    pub fn add(&mut self, name: &str, min_width: f64) {
        self.breakpoints.push((name.to_string(), min_width));
        self.breakpoints.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    }

    /// Which breakpoint is active for the given viewport width?
    pub fn active(&self, viewport_width: f64) -> &str {
        let mut result = "";
        for (name, threshold) in &self.breakpoints {
            if viewport_width >= *threshold { result = name; }
        }
        result
    }

    /// Get the number of grid columns for the current breakpoint (responsive grid).
    pub fn grid_columns(&self, viewport_width: f64) -> usize {
        match self.active(viewport_width) {
            "xs" => 1, "sm" => 2, "md" => 3, "lg" => 4, "xl" => 6,
            _ => 3
        }
    }
}
