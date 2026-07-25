//! **Text 3D** — 3D text geometry generation.
//!
//! Generates extruded text meshes from a built-in bitmap font.
//! Supports text alignment, character spacing, line height, and bevel.

use super::scene3d::Vec3;
use super::geometry3d::{BufferGeometry, Vertex, Triangle};

// ══════════════════════════════════════════════════════════════════════════════
// Font / Glyph
// ══════════════════════════════════════════════════════════════════════════════

/// A glyph represented as a polygon outline (list of 2D contour points).
#[derive(Debug, Clone)]
pub struct Glyph {
    pub character: char,
    pub width: f64,
    pub contours: Vec<Vec<[f64; 2]>>,
}

/// Built-in bitmap font with simplified glyph outlines.
pub struct BitmapFont {
    pub name: String,
    pub line_height: f64,
    pub glyphs: Vec<Glyph>,
}

impl BitmapFont {
    /// Create a simple monospace font with rectangular glyphs.
    pub fn monospace() -> Self {
        let mut glyphs = Vec::new();
        // Generate basic rectangular outlines for printable ASCII
        for c in 32u8..=126u8 {
            let ch = c as char;
            let contour = glyph_contour(ch);
            glyphs.push(Glyph { character: ch, width: 0.6, contours: vec![contour] });
        }
        BitmapFont { name: "monospace".into(), line_height: 1.2, glyphs }
    }

    pub fn get_glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.character == c)
    }
}

/// Generate a simplified contour for a character (pixel-art style).
fn glyph_contour(ch: char) -> Vec<[f64; 2]> {
    // For non-space characters: a simple rectangular outline with notch
    if ch == ' ' { return vec![]; }
    // Simple stylized rectangle
    let w = 0.5;
    let h = 0.8;
    vec![
        [0.0, 0.0], [w, 0.0], [w, h], [0.0, h],
    ]
}

// ══════════════════════════════════════════════════════════════════════════════
// Text Geometry Config
// ══════════════════════════════════════════════════════════════════════════════

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Configuration for 3D text generation.
#[derive(Debug, Clone)]
pub struct TextGeometryConfig {
    pub size: f64,
    pub depth: f64,
    pub bevel_enabled: bool,
    pub bevel_thickness: f64,
    pub bevel_size: f64,
    pub letter_spacing: f64,
    pub line_height: f64,
    pub align: TextAlign,
}

impl Default for TextGeometryConfig {
    fn default() -> Self {
        TextGeometryConfig {
            size: 1.0,
            depth: 0.2,
            bevel_enabled: false,
            bevel_thickness: 0.02,
            bevel_size: 0.02,
            letter_spacing: 0.1,
            line_height: 1.4,
            align: TextAlign::Left,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Text Geometry Generation
// ══════════════════════════════════════════════════════════════════════════════

/// Generate 3D geometry for a text string.
pub fn text_geometry(text: &str, font: &BitmapFont, config: &TextGeometryConfig) -> BufferGeometry {
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    let lines: Vec<&str> = text.split('\n').collect();
    let mut cursor_y = 0.0;

    for line in &lines {
        let line_width = line.chars().count() as f64 * (config.size * 0.6 + config.letter_spacing) - config.letter_spacing;
        let offset_x = match config.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => -line_width / 2.0,
            TextAlign::Right => -line_width,
        };

        let mut cursor_x = offset_x;
        for ch in line.chars() {
            if ch == ' ' {
                cursor_x += config.size * 0.6 + config.letter_spacing;
                continue;
            }
            if let Some(glyph) = font.get_glyph(ch) {
                for contour in &glyph.contours {
                    if contour.len() < 3 { continue; }
                    let base_idx = all_vertices.len() as u32;
                    // Extrude the contour to create 3D geometry
                    extrude_contour(contour, cursor_x, cursor_y, config, &mut all_vertices, &mut all_indices, base_idx);
                }
                cursor_x += glyph.width * config.size + config.letter_spacing;
            } else {
                cursor_x += config.size * 0.6 + config.letter_spacing;
            }
        }
        cursor_y -= config.size * config.line_height;
    }

    let mut geo = BufferGeometry { vertices: all_vertices, indices: all_indices, bounding_box: None, bounding_sphere: None };
    geo.compute_bounds();
    geo
}

fn extrude_contour(contour: &[[f64; 2]], x_off: f64, y_off: f64, config: &TextGeometryConfig,
                    vertices: &mut Vec<Vertex>, indices: &mut Vec<Triangle>, base: u32) {
    let n = contour.len();
    let s = config.size;
    let d = config.depth;

    // Front face vertices
    for pt in contour {
        vertices.push(Vertex {
            position: Vec3::new(pt[0] * s + x_off, pt[1] * s + y_off, 0.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            uv: [pt[0], pt[1]],
        });
    }
    // Back face vertices
    for pt in contour {
        vertices.push(Vertex {
            position: Vec3::new(pt[0] * s + x_off, pt[1] * s + y_off, -d),
            normal: Vec3::new(0.0, 0.0, -1.0),
            uv: [pt[0], pt[1]],
        });
    }

    // Front face triangles (fan)
    for i in 1..n as u32 - 1 {
        indices.push(Triangle(base, base + i, base + i + 1));
    }
    // Back face triangles (fan, reversed winding)
    let back_base = base + n as u32;
    for i in 1..n as u32 - 1 {
        indices.push(Triangle(back_base, back_base + i + 1, back_base + i));
    }
    // Side quads
    for i in 0..n {
        let j = (i + 1) % n;
        let f0 = base + i as u32;
        let f1 = base + j as u32;
        let b0 = back_base + i as u32;
        let b1 = back_base + j as u32;
        indices.push(Triangle(f0, f1, b0));
        indices.push(Triangle(f1, b1, b0));
    }

    // Bevel (simple offset)
    if config.bevel_enabled {
        let bevel_base = vertices.len() as u32;
        let bs = config.bevel_size;
        for pt in contour {
            vertices.push(Vertex {
                position: Vec3::new(pt[0] * s + x_off + bs, pt[1] * s + y_off + bs, bs),
                normal: Vec3::new(0.0, 0.0, 1.0),
                uv: [pt[0], pt[1]],
            });
        }
        for i in 0..n {
            let j = (i + 1) % n;
            let orig = base + i as u32;
            let orig_next = base + j as u32;
            let bev = bevel_base + i as u32;
            let bev_next = bevel_base + j as u32;
            indices.push(Triangle(orig, orig_next, bev));
            indices.push(Triangle(orig_next, bev_next, bev));
        }
    }
}

/// Create centered 3D text.
pub fn centered_text(text: &str, size: f64, depth: f64) -> BufferGeometry {
    let font = BitmapFont::monospace();
    let config = TextGeometryConfig {
        size, depth, align: TextAlign::Center,
        ..Default::default()
    };
    text_geometry(text, &font, &config)
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monospace_font() {
        let font = BitmapFont::monospace();
        assert!(font.get_glyph('A').is_some());
        assert!(font.get_glyph(' ').is_some());
        assert!(font.get_glyph('z').is_some());
    }

    #[test]
    fn text_geometry_basic() {
        let font = BitmapFont::monospace();
        let geo = text_geometry("Hi", &font, &TextGeometryConfig::default());
        assert!(!geo.vertices.is_empty());
        assert!(!geo.indices.is_empty());
    }

    #[test]
    fn text_geometry_space() {
        let font = BitmapFont::monospace();
        let geo = text_geometry("A B", &font, &TextGeometryConfig::default());
        // Space should not generate geometry
        let geo_no_space = text_geometry("AB", &font, &TextGeometryConfig::default());
        // Both should have same number of drawn characters
        assert!(geo.vertices.len() == geo_no_space.vertices.len());
    }

    #[test]
    fn text_geometry_multiline() {
        let font = BitmapFont::monospace();
        let geo = text_geometry("AB\nCD", &font, &TextGeometryConfig::default());
        assert!(!geo.vertices.is_empty());
    }

    #[test]
    fn text_centered() {
        let geo = centered_text("Test", 1.0, 0.2);
        assert!(geo.bounding_box.is_some());
        let bb = geo.bounding_box.as_ref().unwrap();
        // Center alignment means min.x should be negative and max.x positive
        assert!(bb.min.x < 0.0);
        assert!(bb.max.x > 0.0);
    }

    #[test]
    fn text_with_bevel() {
        let font = BitmapFont::monospace();
        let config = TextGeometryConfig {
            bevel_enabled: true,
            bevel_size: 0.05,
            ..Default::default()
        };
        let geo = text_geometry("X", &font, &config);
        assert!(!geo.vertices.is_empty());
        // Bevel adds extra vertices
        let geo_no_bevel = text_geometry("X", &font, &TextGeometryConfig::default());
        assert!(geo.vertices.len() > geo_no_bevel.vertices.len());
    }

    #[test]
    fn text_alignment_left() {
        let font = BitmapFont::monospace();
        let config = TextGeometryConfig { align: TextAlign::Left, ..Default::default() };
        let geo = text_geometry("A", &font, &config);
        let bb = geo.bounding_box.as_ref().unwrap();
        assert!(bb.min.x >= -0.01); // Left-aligned starts at ~0
    }

    #[test]
    fn text_alignment_right() {
        let font = BitmapFont::monospace();
        let config = TextGeometryConfig { align: TextAlign::Right, ..Default::default() };
        let geo = text_geometry("A", &font, &config);
        let bb = geo.bounding_box.as_ref().unwrap();
        assert!(bb.max.x <= 0.01); // Right-aligned ends at ~0
    }

    #[test]
    fn empty_text() {
        let font = BitmapFont::monospace();
        let geo = text_geometry("", &font, &TextGeometryConfig::default());
        assert!(geo.vertices.is_empty());
    }
}
