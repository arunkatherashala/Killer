//! **Style & theme system** — CSS-like styling with theme palettes, inheritance, and scoped overrides.
//!
//! No runtime CSS parsing — styles are Rust structs built from builtins or code.
//! Themes provide a color/spacing/typography palette that widgets resolve at layout time.

use std::collections::HashMap;

// ── Color ────────────────────────────────────────────────────────────────────

/// RGBA color (0–255 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(220, 38, 38);
    pub const GREEN: Self = Self::rgb(34, 197, 94);
    pub const BLUE: Self = Self::rgb(59, 130, 246);
    pub const YELLOW: Self = Self::rgb(234, 179, 8);
    pub const GRAY: Self = Self::rgb(107, 114, 128);
    pub const DARK_GRAY: Self = Self::rgb(55, 65, 81);
    pub const LIGHT_GRAY: Self = Self::rgb(209, 213, 219);

    /// Parse "#RRGGBB" or "#RRGGBBAA" hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Self::rgb(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Self::rgba(r, g, b, a))
        } else {
            None
        }
    }

    /// Emit as "#RRGGBB" (or "#RRGGBBAA" if alpha < 255).
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Lighten by factor 0.0–1.0 (1.0 = white).
    pub fn lighten(&self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self::rgba(
            (self.r as f32 + (255.0 - self.r as f32) * f) as u8,
            (self.g as f32 + (255.0 - self.g as f32) * f) as u8,
            (self.b as f32 + (255.0 - self.b as f32) * f) as u8,
            self.a,
        )
    }

    /// Darken by factor 0.0–1.0 (1.0 = black).
    pub fn darken(&self, factor: f32) -> Self {
        let f = 1.0 - factor.clamp(0.0, 1.0);
        Self::rgba(
            (self.r as f32 * f) as u8,
            (self.g as f32 * f) as u8,
            (self.b as f32 * f) as u8,
            self.a,
        )
    }
}

impl Default for Color {
    fn default() -> Self { Self::BLACK }
}

// ── Units ────────────────────────────────────────────────────────────────────

/// CSS-like length unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Px(f64),
    Percent(f64),
    Em(f64),
    Rem(f64),
    Vw(f64),
    Vh(f64),
    Auto,
}

impl Default for Unit {
    fn default() -> Self { Unit::Auto }
}

impl Unit {
    /// Resolve to pixels given `parent_px` for % and `root_font_px` for rem, `font_px` for em,
    /// `viewport_w/h` for vw/vh.
    pub fn resolve(&self, parent_px: f64, font_px: f64, root_font_px: f64, vw: f64, vh: f64) -> f64 {
        match self {
            Unit::Px(v) => *v,
            Unit::Percent(v) => parent_px * v / 100.0,
            Unit::Em(v) => font_px * v,
            Unit::Rem(v) => root_font_px * v,
            Unit::Vw(v) => vw * v / 100.0,
            Unit::Vh(v) => vh * v / 100.0,
            Unit::Auto => 0.0,
        }
    }
}

// ── Box edges ────────────────────────────────────────────────────────────────

/// Four-sided value (margin, padding, border-width).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxEdges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl BoxEdges {
    pub const ZERO: Self = Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };
    pub fn all(v: f64) -> Self { Self { top: v, right: v, bottom: v, left: v } }
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }
    pub fn horizontal(&self) -> f64 { self.left + self.right }
    pub fn vertical(&self) -> f64 { self.top + self.bottom }
}

// ── Border ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
    Dotted,
    Double,
}

impl Default for BorderStyle {
    fn default() -> Self { BorderStyle::None }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Border {
    pub width: f64,
    pub style: BorderStyle,
    pub color: Color,
    pub radius: f64,
}

// ── Font / typography ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Thin,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl Default for FontWeight {
    fn default() -> Self { FontWeight::Normal }
}

impl FontWeight {
    pub fn numeric(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl Default for TextAlign {
    fn default() -> Self { TextAlign::Left }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextDecoration {
    None,
    Underline,
    LineThrough,
    Overline,
}

impl Default for TextDecoration {
    fn default() -> Self { TextDecoration::None }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub family: String,
    pub size: f64,
    pub weight: FontWeight,
    pub italic: bool,
    pub line_height: f64,
    pub letter_spacing: f64,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: "system-ui".to_string(),
            size: 14.0,
            weight: FontWeight::Normal,
            italic: false,
            line_height: 1.5,
            letter_spacing: 0.0,
        }
    }
}

// ── Overflow / visibility ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Default for Overflow {
    fn default() -> Self { Overflow::Visible }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Visible,
    Hidden,
    Collapsed,
}

impl Default for Visibility {
    fn default() -> Self { Visibility::Visible }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor {
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
    Crosshair,
    Grab,
    Grabbing,
    ColResize,
    RowResize,
    Wait,
    Progress,
}

impl Default for Cursor {
    fn default() -> Self { Cursor::Default }
}

// ── Shadow ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
    pub spread: f64,
    pub color: Color,
    pub inset: bool,
}

// ── Transform ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transform {
    None,
    Translate(f64, f64),
    Scale(f64, f64),
    Rotate(f64),
    Skew(f64, f64),
}

impl Default for Transform {
    fn default() -> Self { Transform::None }
}

// ── Style (the big one) ──────────────────────────────────────────────────────

/// Complete style object — CSS-level coverage.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    // -- Box model --
    pub width: Unit,
    pub height: Unit,
    pub min_width: Unit,
    pub min_height: Unit,
    pub max_width: Unit,
    pub max_height: Unit,
    pub margin: BoxEdges,
    pub padding: BoxEdges,
    pub border: Border,

    // -- Colors --
    pub background: Color,
    pub color: Color,
    pub opacity: f64,

    // -- Typography --
    pub font: Font,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub white_space_nowrap: bool,
    pub text_overflow_ellipsis: bool,

    // -- Layout hints (used by layout.rs flexbox) --
    pub flex_grow: f64,
    pub flex_shrink: f64,
    pub flex_basis: Unit,
    pub align_self: Option<AlignItems>,

    // -- Overflow / visibility --
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub visibility: Visibility,
    pub cursor: Cursor,
    pub z_index: i32,

    // -- Effects --
    pub box_shadow: Option<BoxShadow>,
    pub transform: Transform,

    // -- Transition --
    pub transition_property: Option<String>,
    pub transition_duration_ms: f64,

    // -- Custom properties --
    pub custom: HashMap<String, String>,
}

/// Alignment on the cross-axis (shared with layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignItems {
    Start,
    End,
    Center,
    Stretch,
    Baseline,
}

impl Default for AlignItems {
    fn default() -> Self { AlignItems::Stretch }
}

/// Justify on the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JustifyContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for JustifyContent {
    fn default() -> Self { JustifyContent::Start }
}

/// Flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Default for FlexDirection {
    fn default() -> Self { FlexDirection::Row }
}

/// Flex wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for FlexWrap {
    fn default() -> Self { FlexWrap::NoWrap }
}

/// Position type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Default for Position {
    fn default() -> Self { Position::Static }
}

/// Display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Display {
    Block,
    Flex,
    InlineFlex,
    Grid,
    Inline,
    InlineBlock,
    None,
}

impl Default for Display {
    fn default() -> Self { Display::Flex }
}

// ── Theme ────────────────────────────────────────────────────────────────────

/// Application-wide theme (color palette + typography + spacing scale).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    // Color palette
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub surface: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    pub on_primary: Color,
    pub on_secondary: Color,
    pub on_background: Color,
    pub on_surface: Color,
    pub on_error: Color,
    pub divider: Color,
    // Typography
    pub font_family: String,
    pub font_size_base: f64,
    pub font_size_sm: f64,
    pub font_size_lg: f64,
    pub font_size_xl: f64,
    pub font_size_h1: f64,
    pub font_size_h2: f64,
    pub font_size_h3: f64,
    // Spacing scale (multiples of base_spacing)
    pub base_spacing: f64,
    pub border_radius: f64,
    pub border_radius_lg: f64,
    // Shadows
    pub shadow_sm: BoxShadow,
    pub shadow_md: BoxShadow,
    pub shadow_lg: BoxShadow,
}

impl Theme {
    /// Material-inspired light theme.
    pub fn light() -> Self {
        Self {
            name: "light".into(),
            primary: Color::rgb(25, 118, 210),
            secondary: Color::rgb(156, 39, 176),
            accent: Color::rgb(0, 150, 136),
            background: Color::rgb(250, 250, 250),
            surface: Color::WHITE,
            error: Color::RED,
            warning: Color::YELLOW,
            success: Color::GREEN,
            info: Color::BLUE,
            on_primary: Color::WHITE,
            on_secondary: Color::WHITE,
            on_background: Color::rgb(33, 33, 33),
            on_surface: Color::rgb(33, 33, 33),
            on_error: Color::WHITE,
            divider: Color::LIGHT_GRAY,
            font_family: "system-ui, -apple-system, sans-serif".into(),
            font_size_base: 14.0,
            font_size_sm: 12.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_h1: 32.0,
            font_size_h2: 24.0,
            font_size_h3: 20.0,
            base_spacing: 8.0,
            border_radius: 4.0,
            border_radius_lg: 8.0,
            shadow_sm: BoxShadow { offset_x: 0.0, offset_y: 1.0, blur: 2.0, spread: 0.0, color: Color::rgba(0, 0, 0, 40), inset: false },
            shadow_md: BoxShadow { offset_x: 0.0, offset_y: 2.0, blur: 8.0, spread: 0.0, color: Color::rgba(0, 0, 0, 50), inset: false },
            shadow_lg: BoxShadow { offset_x: 0.0, offset_y: 4.0, blur: 16.0, spread: 0.0, color: Color::rgba(0, 0, 0, 60), inset: false },
        }
    }

    /// Dark theme.
    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            primary: Color::rgb(100, 180, 246),
            secondary: Color::rgb(206, 147, 216),
            accent: Color::rgb(128, 203, 196),
            background: Color::rgb(18, 18, 18),
            surface: Color::rgb(30, 30, 30),
            error: Color::rgb(239, 83, 80),
            warning: Color::rgb(255, 167, 38),
            success: Color::rgb(102, 187, 106),
            info: Color::rgb(66, 165, 245),
            on_primary: Color::BLACK,
            on_secondary: Color::BLACK,
            on_background: Color::rgb(224, 224, 224),
            on_surface: Color::rgb(224, 224, 224),
            on_error: Color::BLACK,
            divider: Color::DARK_GRAY,
            font_family: "system-ui, -apple-system, sans-serif".into(),
            font_size_base: 14.0,
            font_size_sm: 12.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_h1: 32.0,
            font_size_h2: 24.0,
            font_size_h3: 20.0,
            base_spacing: 8.0,
            border_radius: 4.0,
            border_radius_lg: 8.0,
            shadow_sm: BoxShadow { offset_x: 0.0, offset_y: 1.0, blur: 3.0, spread: 0.0, color: Color::rgba(0, 0, 0, 100), inset: false },
            shadow_md: BoxShadow { offset_x: 0.0, offset_y: 2.0, blur: 10.0, spread: 0.0, color: Color::rgba(0, 0, 0, 120), inset: false },
            shadow_lg: BoxShadow { offset_x: 0.0, offset_y: 4.0, blur: 20.0, spread: 0.0, color: Color::rgba(0, 0, 0, 140), inset: false },
        }
    }

    /// Spacing multiplier: `theme.sp(2)` → `base_spacing * 2`.
    pub fn sp(&self, mult: f64) -> f64 {
        self.base_spacing * mult
    }
}

impl Default for Theme {
    fn default() -> Self { Self::light() }
}

// ── Style store (scoped styles) ──────────────────────────────────────────────

/// Global style registry: maps widget IDs or class names to styles. Supports inheritance.
#[derive(Debug, Default)]
pub struct StyleStore {
    /// Styles keyed by widget ID.
    pub by_id: HashMap<String, Style>,
    /// Styles keyed by class name (shared across widgets).
    pub by_class: HashMap<String, Style>,
    /// Active theme.
    pub theme: Theme,
}

impl StyleStore {
    pub fn new(theme: Theme) -> Self {
        Self {
            by_id: HashMap::new(),
            by_class: HashMap::new(),
            theme,
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn set_id_style(&mut self, id: &str, style: Style) {
        self.by_id.insert(id.to_string(), style);
    }

    pub fn set_class_style(&mut self, class: &str, style: Style) {
        self.by_class.insert(class.to_string(), style);
    }

    /// Resolve the style for a widget: ID style overrides class style overrides default.
    pub fn resolve(&self, widget_id: &str, class_names: &[&str]) -> Style {
        let mut result = Style::default();
        // Apply class styles in order
        for class in class_names {
            if let Some(s) = self.by_class.get(*class) {
                merge_style(&mut result, s);
            }
        }
        // Apply ID style (highest priority)
        if let Some(s) = self.by_id.get(widget_id) {
            merge_style(&mut result, s);
        }
        result
    }

    /// Emit the full store as JSON.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        s.push_str(&format!("  \"theme\": \"{}\",\n", self.theme.name));
        s.push_str("  \"id_styles\": {\n");
        for (i, (id, _style)) in self.by_id.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!("    \"{}\": {{}}", id));
        }
        s.push_str("\n  },\n  \"class_styles\": {\n");
        for (i, (cls, _style)) in self.by_class.iter().enumerate() {
            if i > 0 { s.push_str(",\n"); }
            s.push_str(&format!("    \"{}\": {{}}", cls));
        }
        s.push_str("\n  }\n}\n");
        s
    }
}

/// Merge `source` into `target` (non-default values overwrite).
fn merge_style(target: &mut Style, source: &Style) {
    if source.background != Color::default() { target.background = source.background; }
    if source.color != Color::default() { target.color = source.color; }
    if source.opacity != 0.0 { target.opacity = source.opacity; }
    if !matches!(source.width, Unit::Auto) { target.width = source.width; }
    if !matches!(source.height, Unit::Auto) { target.height = source.height; }
    if source.padding != BoxEdges::ZERO { target.padding = source.padding; }
    if source.margin != BoxEdges::ZERO { target.margin = source.margin; }
    if source.border.style != BorderStyle::None { target.border = source.border; }
    if source.font.size != 14.0 { target.font = source.font.clone(); }
    if source.text_align != TextAlign::Left { target.text_align = source.text_align.clone(); }
    if source.flex_grow != 0.0 { target.flex_grow = source.flex_grow; }
    if source.flex_shrink != 0.0 { target.flex_shrink = source.flex_shrink; }
    if source.z_index != 0 { target.z_index = source.z_index; }
    if source.box_shadow.is_some() { target.box_shadow = source.box_shadow; }
    if source.cursor != Cursor::Default { target.cursor = source.cursor; }
    if source.visibility != Visibility::Visible { target.visibility = source.visibility; }
    if !matches!(source.transform, Transform::None) { target.transform = source.transform; }
    if source.transition_duration_ms != 0.0 {
        target.transition_duration_ms = source.transition_duration_ms;
        target.transition_property = source.transition_property.clone();
    }
    // Merge custom properties
    for (k, v) in &source.custom {
        target.custom.insert(k.clone(), v.clone());
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex_roundtrip() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.to_hex(), "#ff8000");
        let parsed = Color::from_hex("#ff8000").unwrap();
        assert_eq!(parsed, c);
    }

    #[test]
    fn color_lighten_darken() {
        let c = Color::rgb(100, 100, 100);
        let light = c.lighten(0.5);
        assert!(light.r > 100 && light.g > 100 && light.b > 100);
        let dark = c.darken(0.5);
        assert!(dark.r < 100 && dark.g < 100 && dark.b < 100);
    }

    #[test]
    fn unit_resolve_px() {
        let u = Unit::Px(42.0);
        assert_eq!(u.resolve(0.0, 0.0, 0.0, 0.0, 0.0), 42.0);
    }

    #[test]
    fn unit_resolve_percent() {
        let u = Unit::Percent(50.0);
        assert_eq!(u.resolve(200.0, 0.0, 0.0, 0.0, 0.0), 100.0);
    }

    #[test]
    fn theme_spacing() {
        let t = Theme::light();
        assert_eq!(t.sp(2.0), 16.0);
    }

    #[test]
    fn style_store_resolve_priority() {
        let mut store = StyleStore::new(Theme::light());
        store.set_class_style("btn", Style {
            background: Color::BLUE,
            ..Default::default()
        });
        store.set_id_style("submit", Style {
            background: Color::GREEN,
            ..Default::default()
        });
        let resolved = store.resolve("submit", &["btn"]);
        // ID style wins
        assert_eq!(resolved.background, Color::GREEN);
    }

    #[test]
    fn box_edges_symmetric() {
        let e = BoxEdges::symmetric(10.0, 20.0);
        assert_eq!(e.horizontal(), 40.0);
        assert_eq!(e.vertical(), 20.0);
    }

    #[test]
    fn dark_theme_creates() {
        let t = Theme::dark();
        assert_eq!(t.name, "dark");
        assert_ne!(t.background, Theme::light().background);
    }
}
