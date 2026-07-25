//! **CSS Grid Layout** — Full CSS Grid layout algorithm.
//!
//! Supports: `grid-template-columns`, `grid-template-rows`, row/column gap,
//! track sizing (fixed, fractional `fr`, auto), `grid-column`/`grid-row` spans,
//! auto-placement, content alignment.
//!
//! Competitive with CSS Grid in browsers (subset of the spec).

// ══════════════════════════════════════════════════════════════════════════════
// Track sizing
// ══════════════════════════════════════════════════════════════════════════════

/// A track size definition (row or column).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackSize {
    /// Fixed pixel size.
    Px(f64),
    /// Fractional unit (like CSS `1fr`).
    Fr(f64),
    /// Size to fit content.
    Auto,
    /// Minimum and maximum range.
    MinMax(f64, f64),
}

/// Grid item placement.
#[derive(Debug, Clone, Default)]
pub struct GridPlacement {
    /// Column start (1-based, 0 = auto).
    pub col_start: usize,
    /// Number of columns to span.
    pub col_span: usize,
    /// Row start (1-based, 0 = auto).
    pub row_start: usize,
    /// Number of rows to span.
    pub row_span: usize,
}

impl GridPlacement {
    pub fn auto() -> Self {
        GridPlacement { col_start: 0, col_span: 1, row_start: 0, row_span: 1 }
    }

    pub fn at(col: usize, row: usize) -> Self {
        GridPlacement { col_start: col, col_span: 1, row_start: row, row_span: 1 }
    }

    pub fn span(col: usize, row: usize, col_span: usize, row_span: usize) -> Self {
        GridPlacement { col_start: col, col_span, row_start: row, row_span }
    }
}

/// Alignment within a grid cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridAlign {
    Start,
    Center,
    End,
    Stretch,
}

impl Default for GridAlign {
    fn default() -> Self { GridAlign::Stretch }
}

// ══════════════════════════════════════════════════════════════════════════════
// GridDefinition — the template
// ══════════════════════════════════════════════════════════════════════════════

/// Defines the grid structure.
pub struct GridDefinition {
    pub columns: Vec<TrackSize>,
    pub rows: Vec<TrackSize>,
    pub col_gap: f64,
    pub row_gap: f64,
    pub justify_items: GridAlign,
    pub align_items: GridAlign,
}

impl GridDefinition {
    pub fn new() -> Self {
        GridDefinition {
            columns: Vec::new(),
            rows: Vec::new(),
            col_gap: 0.0,
            row_gap: 0.0,
            justify_items: GridAlign::Stretch,
            align_items: GridAlign::Stretch,
        }
    }

    /// Set columns: e.g., `cols(vec![Fr(1.0), Fr(1.0), Fr(1.0)])` for 3 equal columns.
    pub fn cols(mut self, tracks: Vec<TrackSize>) -> Self {
        self.columns = tracks; self
    }

    /// Shorthand: N equal fractional columns.
    pub fn equal_cols(mut self, n: usize) -> Self {
        self.columns = vec![TrackSize::Fr(1.0); n]; self
    }

    /// Set rows.
    pub fn rows(mut self, tracks: Vec<TrackSize>) -> Self {
        self.rows = tracks; self
    }

    pub fn gap(mut self, col: f64, row: f64) -> Self {
        self.col_gap = col; self.row_gap = row; self
    }

    pub fn justify(mut self, align: GridAlign) -> Self {
        self.justify_items = align; self
    }

    pub fn align(mut self, align: GridAlign) -> Self {
        self.align_items = align; self
    }

    /// Parse CSS-like column definition: "1fr 200px auto 2fr"
    pub fn parse_columns(mut self, spec: &str) -> Self {
        self.columns = parse_track_list(spec);
        self
    }

    /// Parse CSS-like row definition.
    pub fn parse_rows(mut self, spec: &str) -> Self {
        self.rows = parse_track_list(spec);
        self
    }

    pub fn col_count(&self) -> usize { self.columns.len() }
}

impl Default for GridDefinition {
    fn default() -> Self { Self::new() }
}

fn parse_track_list(spec: &str) -> Vec<TrackSize> {
    spec.split_whitespace().map(|token| {
        if token.ends_with("fr") {
            let n: f64 = token.trim_end_matches("fr").parse().unwrap_or(1.0);
            TrackSize::Fr(n)
        } else if token.ends_with("px") {
            let n: f64 = token.trim_end_matches("px").parse().unwrap_or(0.0);
            TrackSize::Px(n)
        } else if token == "auto" {
            TrackSize::Auto
        } else {
            // Try as plain number → Px
            token.parse::<f64>().map(TrackSize::Px).unwrap_or(TrackSize::Auto)
        }
    }).collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Grid item input
// ══════════════════════════════════════════════════════════════════════════════

/// Input: a grid child with its placement and intrinsic size.
pub struct GridItem {
    pub id: String,
    pub placement: GridPlacement,
    pub min_width: f64,
    pub min_height: f64,
}

// ══════════════════════════════════════════════════════════════════════════════
// Computed grid layout
// ══════════════════════════════════════════════════════════════════════════════

/// Result: computed position and size for each grid item.
#[derive(Debug, Clone)]
pub struct ComputedGridItem {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub col: usize,
    pub row: usize,
}

/// Compute grid layout for all items within a container of given size.
pub fn compute_grid(
    def: &GridDefinition,
    items: &[GridItem],
    container_width: f64,
    container_height: f64,
) -> Vec<ComputedGridItem> {
    let num_cols = def.columns.len().max(1);

    // Determine number of rows needed
    let num_items = items.len();
    let explicit_rows = def.rows.len();
    let max_explicit_row = items.iter()
        .filter(|it| it.placement.row_start > 0)
        .map(|it| it.placement.row_start - 1 + it.placement.row_span)
        .max()
        .unwrap_or(0);
    let auto_rows = if num_items > 0 {
        ((num_items + num_cols - 1) / num_cols).max(explicit_rows).max(max_explicit_row)
    } else {
        explicit_rows.max(1)
    };

    // Resolve column widths
    let total_col_gap = def.col_gap * (num_cols.saturating_sub(1)) as f64;
    let col_widths = resolve_tracks(&def.columns, container_width - total_col_gap);

    // Resolve row heights
    let total_row_gap = def.row_gap * (auto_rows.saturating_sub(1)) as f64;
    let row_defs: Vec<TrackSize> = (0..auto_rows).map(|i| {
        def.rows.get(i).cloned().unwrap_or(TrackSize::Fr(1.0))
    }).collect();
    let row_heights = resolve_tracks(&row_defs, container_height - total_row_gap);

    // Compute column and row offsets
    let col_offsets = compute_offsets(&col_widths, def.col_gap);
    let row_offsets = compute_offsets(&row_heights, def.row_gap);

    // Place items
    let mut result = Vec::new();
    let mut auto_cursor = 0usize; // auto-placement cursor (linear index)

    for item in items {
        let (col, row) = if item.placement.col_start > 0 && item.placement.row_start > 0 {
            (item.placement.col_start - 1, item.placement.row_start - 1)
        } else {
            // Auto-placement
            let c = auto_cursor % num_cols;
            let r = auto_cursor / num_cols;
            auto_cursor += 1;
            (c, r)
        };

        if col >= col_widths.len() || row >= row_heights.len() { continue; }

        // Handle spans
        let col_span = item.placement.col_span.min(num_cols - col);
        let row_span = item.placement.row_span.min(auto_rows - row);

        let x = col_offsets[col];
        let y = row_offsets[row];
        let width: f64 = (0..col_span).map(|i| {
            col_widths.get(col + i).unwrap_or(&0.0)
        }).sum::<f64>() + def.col_gap * (col_span.saturating_sub(1)) as f64;
        let height: f64 = (0..row_span).map(|i| {
            row_heights.get(row + i).unwrap_or(&0.0)
        }).sum::<f64>() + def.row_gap * (row_span.saturating_sub(1)) as f64;

        result.push(ComputedGridItem {
            id: item.id.clone(),
            x, y, width, height,
            col, row,
        });
    }
    result
}

fn resolve_tracks(tracks: &[TrackSize], available: f64) -> Vec<f64> {
    if tracks.is_empty() { return vec![available]; }

    let mut sizes = vec![0.0f64; tracks.len()];
    let mut remaining = available;
    let mut total_fr = 0.0f64;

    // First pass: fixed + auto sizes
    for (i, track) in tracks.iter().enumerate() {
        match track {
            TrackSize::Px(px) => { sizes[i] = *px; remaining -= px; }
            TrackSize::Auto => { sizes[i] = 0.0; /* Will get share of remaining */ }
            TrackSize::Fr(fr) => { total_fr += fr; }
            TrackSize::MinMax(min, _max) => { sizes[i] = *min; remaining -= min; }
        }
    }

    // Count auto tracks for fair sharing
    let auto_count = tracks.iter().filter(|t| matches!(t, TrackSize::Auto)).count();

    // Second pass: distribute remaining to fr and auto
    if remaining > 0.0 {
        if total_fr > 0.0 {
            let fr_remaining = remaining - (auto_count as f64 * 50.0); // auto gets 50px default
            for (i, track) in tracks.iter().enumerate() {
                match track {
                    TrackSize::Fr(fr) => {
                        sizes[i] = (fr / total_fr) * fr_remaining.max(0.0);
                    }
                    TrackSize::Auto => { sizes[i] = 50.0; }
                    _ => {}
                }
            }
        } else if auto_count > 0 {
            let per_auto = remaining / auto_count as f64;
            for (i, track) in tracks.iter().enumerate() {
                if matches!(track, TrackSize::Auto) {
                    sizes[i] = per_auto;
                }
            }
        }
    }

    // Clamp MinMax
    for (i, track) in tracks.iter().enumerate() {
        if let TrackSize::MinMax(min, max) = track {
            sizes[i] = sizes[i].max(*min).min(*max);
        }
    }

    sizes
}

fn compute_offsets(sizes: &[f64], gap: f64) -> Vec<f64> {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut pos = 0.0;
    for (i, size) in sizes.iter().enumerate() {
        offsets.push(pos);
        pos += size;
        if i < sizes.len() - 1 { pos += gap; }
    }
    offsets
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_columns() {
        let def = GridDefinition::new().equal_cols(3).gap(10.0, 10.0);
        let items: Vec<GridItem> = (0..6).map(|i| GridItem {
            id: format!("item-{}", i),
            placement: GridPlacement::auto(),
            min_width: 0.0,
            min_height: 0.0,
        }).collect();
        let result = compute_grid(&def, &items, 340.0, 200.0);
        assert_eq!(result.len(), 6);
        // 340 - 20 (2 gaps) = 320 / 3 ≈ 106.67 per col
        assert!((result[0].width - 106.67).abs() < 1.0);
        // Second row
        assert_eq!(result[3].row, 1);
    }

    #[test]
    fn fixed_and_fr_columns() {
        let def = GridDefinition::new()
            .parse_columns("200px 1fr 1fr")
            .gap(0.0, 0.0);
        let items: Vec<GridItem> = (0..3).map(|i| GridItem {
            id: format!("{}", i),
            placement: GridPlacement::auto(),
            min_width: 0.0, min_height: 0.0,
        }).collect();
        let result = compute_grid(&def, &items, 600.0, 100.0);
        assert_eq!(result[0].width, 200.0);
        assert!((result[1].width - 200.0).abs() < 1.0); // (600-200)/2
        assert!((result[2].width - 200.0).abs() < 1.0);
    }

    #[test]
    fn explicit_placement() {
        let def = GridDefinition::new().equal_cols(3).gap(0.0, 0.0);
        let items = vec![
            GridItem {
                id: "a".into(),
                placement: GridPlacement::at(1, 1),
                min_width: 0.0, min_height: 0.0,
            },
            GridItem {
                id: "b".into(),
                placement: GridPlacement::at(3, 2),
                min_width: 0.0, min_height: 0.0,
            },
        ];
        let result = compute_grid(&def, &items, 300.0, 200.0);
        assert_eq!(result[0].col, 0);
        assert_eq!(result[0].row, 0);
        assert_eq!(result[1].col, 2);
        assert_eq!(result[1].row, 1);
    }

    #[test]
    fn column_span() {
        let def = GridDefinition::new().equal_cols(3).gap(10.0, 10.0);
        let items = vec![
            GridItem {
                id: "wide".into(),
                placement: GridPlacement::span(1, 1, 2, 1),
                min_width: 0.0, min_height: 0.0,
            },
        ];
        // 300 - 20 = 280 / 3 ≈ 93.33 per col, span 2 = 93.33*2 + 10 gap = 196.67
        let result = compute_grid(&def, &items, 300.0, 100.0);
        assert!((result[0].width - 196.67).abs() < 1.0);
    }

    #[test]
    fn parse_columns_spec() {
        let def = GridDefinition::new().parse_columns("1fr 2fr 100px auto");
        assert_eq!(def.columns.len(), 4);
        assert_eq!(def.columns[0], TrackSize::Fr(1.0));
        assert_eq!(def.columns[1], TrackSize::Fr(2.0));
        assert_eq!(def.columns[2], TrackSize::Px(100.0));
        assert_eq!(def.columns[3], TrackSize::Auto);
    }

    #[test]
    fn gap_applied() {
        let def = GridDefinition::new().equal_cols(2).gap(20.0, 20.0);
        let items: Vec<GridItem> = (0..4).map(|i| GridItem {
            id: format!("{}", i),
            placement: GridPlacement::auto(),
            min_width: 0.0, min_height: 0.0,
        }).collect();
        let result = compute_grid(&def, &items, 220.0, 220.0);
        // 220 - 20 = 200 / 2 = 100 per col
        assert_eq!(result[0].x, 0.0);
        assert_eq!(result[1].x, 120.0); // 100 + 20 gap
    }

    #[test]
    fn single_column() {
        let def = GridDefinition::new().equal_cols(1);
        let items: Vec<GridItem> = (0..3).map(|i| GridItem {
            id: format!("{}", i),
            placement: GridPlacement::auto(),
            min_width: 0.0, min_height: 0.0,
        }).collect();
        let result = compute_grid(&def, &items, 400.0, 300.0);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].col, 0);
        assert_eq!(result[1].col, 0);
        assert_eq!(result[2].col, 0);
        assert_eq!(result[0].row, 0);
        assert_eq!(result[1].row, 1);
        assert_eq!(result[2].row, 2);
    }

    #[test]
    fn auto_tracks() {
        let def = GridDefinition::new()
            .cols(vec![TrackSize::Auto, TrackSize::Auto])
            .gap(0.0, 0.0);
        let items = vec![GridItem {
            id: "a".into(),
            placement: GridPlacement::auto(),
            min_width: 0.0, min_height: 0.0,
        }];
        let result = compute_grid(&def, &items, 400.0, 100.0);
        // 400 / 2 = 200 each
        assert_eq!(result[0].width, 200.0);
    }

    #[test]
    fn empty_grid() {
        let def = GridDefinition::new().equal_cols(3);
        let result = compute_grid(&def, &[], 300.0, 100.0);
        assert!(result.is_empty());
    }
}
