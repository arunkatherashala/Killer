//! **Visual Helpers** — Debug/dev helpers for 3D scenes.
//!
//! AxesHelper, GridHelper, BoundingBoxHelper, SkeletonHelper, ArrowHelper.
//! Generates line-based wireframe geometry for debugging visualisation.

use super::scene3d::{Vec3, Color3};

// ══════════════════════════════════════════════════════════════════════════════
// Line Vertex
// ══════════════════════════════════════════════════════════════════════════════

/// A colored line segment vertex pair.
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color3,
}

// ══════════════════════════════════════════════════════════════════════════════
// Axes Helper
// ══════════════════════════════════════════════════════════════════════════════

/// Shows X (red), Y (green), Z (blue) axes.
#[derive(Debug, Clone)]
pub struct AxesHelper {
    pub size: f64,
    pub origin: Vec3,
}

impl AxesHelper {
    pub fn new(size: f64) -> Self {
        AxesHelper { size, origin: Vec3::ZERO }
    }

    pub fn with_origin(mut self, origin: Vec3) -> Self {
        self.origin = origin;
        self
    }

    pub fn lines(&self) -> Vec<LineSegment> {
        let o = &self.origin;
        vec![
            LineSegment { start: o.clone(), end: Vec3::new(o.x + self.size, o.y, o.z), color: Color3::new(1.0, 0.0, 0.0) },
            LineSegment { start: o.clone(), end: Vec3::new(o.x, o.y + self.size, o.z), color: Color3::new(0.0, 1.0, 0.0) },
            LineSegment { start: o.clone(), end: Vec3::new(o.x, o.y, o.z + self.size), color: Color3::new(0.0, 0.0, 1.0) },
        ]
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Grid Helper
// ══════════════════════════════════════════════════════════════════════════════

/// Ground-plane grid.
#[derive(Debug, Clone)]
pub struct GridHelper {
    pub size: f64,
    pub divisions: usize,
    pub color_center: Color3,
    pub color_grid: Color3,
}

impl GridHelper {
    pub fn new(size: f64, divisions: usize) -> Self {
        GridHelper {
            size, divisions,
            color_center: Color3::new(0.5, 0.5, 0.5),
            color_grid: Color3::new(0.3, 0.3, 0.3),
        }
    }

    pub fn lines(&self) -> Vec<LineSegment> {
        let half = self.size / 2.0;
        let step = self.size / self.divisions as f64;
        let mut out = Vec::new();
        for i in 0..=self.divisions {
            let t = -half + i as f64 * step;
            let is_center = t.abs() < step * 0.01;
            let color = if is_center { self.color_center.clone() } else { self.color_grid.clone() };
            // line along X
            out.push(LineSegment { start: Vec3::new(-half, 0.0, t), end: Vec3::new(half, 0.0, t), color: color.clone() });
            // line along Z
            out.push(LineSegment { start: Vec3::new(t, 0.0, -half), end: Vec3::new(t, 0.0, half), color });
        }
        out
    }

    pub fn line_count(&self) -> usize { (self.divisions + 1) * 2 }
}

// ══════════════════════════════════════════════════════════════════════════════
// Bounding Box Helper
// ══════════════════════════════════════════════════════════════════════════════

/// Axis-Aligned Bounding Box wireframe.
#[derive(Debug, Clone)]
pub struct BoundingBoxHelper {
    pub min: Vec3,
    pub max: Vec3,
    pub color: Color3,
}

impl BoundingBoxHelper {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        BoundingBoxHelper { min, max, color: Color3::new(1.0, 1.0, 0.0) }
    }

    pub fn with_color(mut self, c: Color3) -> Self { self.color = c; self }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }

    pub fn extents(&self) -> Vec3 {
        Vec3::new(self.max.x - self.min.x, self.max.y - self.min.y, self.max.z - self.min.z)
    }

    /// 12 edges of the AABB wireframe.
    pub fn lines(&self) -> Vec<LineSegment> {
        let mn = &self.min;
        let mx = &self.max;
        let c = &self.color;
        // 8 corners
        let c000 = Vec3::new(mn.x, mn.y, mn.z);
        let c100 = Vec3::new(mx.x, mn.y, mn.z);
        let c010 = Vec3::new(mn.x, mx.y, mn.z);
        let c110 = Vec3::new(mx.x, mx.y, mn.z);
        let c001 = Vec3::new(mn.x, mn.y, mx.z);
        let c101 = Vec3::new(mx.x, mn.y, mx.z);
        let c011 = Vec3::new(mn.x, mx.y, mx.z);
        let c111 = Vec3::new(mx.x, mx.y, mx.z);
        vec![
            // bottom face
            LineSegment { start: c000.clone(), end: c100.clone(), color: c.clone() },
            LineSegment { start: c100.clone(), end: c101.clone(), color: c.clone() },
            LineSegment { start: c101.clone(), end: c001.clone(), color: c.clone() },
            LineSegment { start: c001.clone(), end: c000.clone(), color: c.clone() },
            // top face
            LineSegment { start: c010.clone(), end: c110.clone(), color: c.clone() },
            LineSegment { start: c110.clone(), end: c111.clone(), color: c.clone() },
            LineSegment { start: c111.clone(), end: c011.clone(), color: c.clone() },
            LineSegment { start: c011.clone(), end: c010.clone(), color: c.clone() },
            // vertical edges
            LineSegment { start: c000, end: c010, color: c.clone() },
            LineSegment { start: c100, end: c110, color: c.clone() },
            LineSegment { start: c101, end: c111, color: c.clone() },
            LineSegment { start: c001, end: c011, color: c.clone() },
        ]
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Arrow Helper
// ══════════════════════════════════════════════════════════════════════════════

/// An arrow pointing in a direction (shaft + arrowhead).
#[derive(Debug, Clone)]
pub struct ArrowHelper {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f64,
    pub color: Color3,
    pub head_length: f64,
    pub head_width: f64,
}

impl ArrowHelper {
    pub fn new(origin: Vec3, direction: Vec3, length: f64) -> Self {
        ArrowHelper {
            origin, direction: direction.normalize(), length,
            color: Color3::new(1.0, 1.0, 0.0),
            head_length: length * 0.2,
            head_width: length * 0.06,
        }
    }

    pub fn with_color(mut self, c: Color3) -> Self { self.color = c; self }

    pub fn tip(&self) -> Vec3 {
        Vec3::new(
            self.origin.x + self.direction.x * self.length,
            self.origin.y + self.direction.y * self.length,
            self.origin.z + self.direction.z * self.length,
        )
    }

    /// Shaft line + simple arrowhead cross lines.
    pub fn lines(&self) -> Vec<LineSegment> {
        let tip = self.tip();
        let head_base = Vec3::new(
            tip.x - self.direction.x * self.head_length,
            tip.y - self.direction.y * self.head_length,
            tip.z - self.direction.z * self.head_length,
        );
        // Find a perpendicular vector
        let perp = if self.direction.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let side1 = self.direction.cross(&perp).normalize();
        let side2 = self.direction.cross(&side1).normalize();
        let hw = self.head_width;
        vec![
            // shaft
            LineSegment { start: self.origin.clone(), end: tip.clone(), color: self.color.clone() },
            // arrowhead
            LineSegment { start: tip.clone(), end: Vec3::new(head_base.x + side1.x * hw, head_base.y + side1.y * hw, head_base.z + side1.z * hw), color: self.color.clone() },
            LineSegment { start: tip.clone(), end: Vec3::new(head_base.x - side1.x * hw, head_base.y - side1.y * hw, head_base.z - side1.z * hw), color: self.color.clone() },
            LineSegment { start: tip.clone(), end: Vec3::new(head_base.x + side2.x * hw, head_base.y + side2.y * hw, head_base.z + side2.z * hw), color: self.color.clone() },
            LineSegment { start: tip, end: Vec3::new(head_base.x - side2.x * hw, head_base.y - side2.y * hw, head_base.z - side2.z * hw), color: self.color.clone() },
        ]
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Skeleton Helper
// ══════════════════════════════════════════════════════════════════════════════

/// A bone in a skeleton hierarchy.
#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub position: Vec3,
    pub parent: Option<usize>,  // index into skeleton's bones array
}

/// Visualises a bone hierarchy.
#[derive(Debug)]
pub struct SkeletonHelper {
    pub bones: Vec<Bone>,
    pub color: Color3,
}

impl SkeletonHelper {
    pub fn new() -> Self {
        SkeletonHelper { bones: Vec::new(), color: Color3::new(0.0, 1.0, 1.0) }
    }

    pub fn add_bone(&mut self, name: &str, position: Vec3, parent: Option<usize>) -> usize {
        let idx = self.bones.len();
        self.bones.push(Bone { name: name.into(), position, parent });
        idx
    }

    /// Line from each bone to its parent.
    pub fn lines(&self) -> Vec<LineSegment> {
        self.bones.iter().filter_map(|b| {
            b.parent.map(|pi| LineSegment {
                start: self.bones[pi].position.clone(),
                end: b.position.clone(),
                color: self.color.clone(),
            })
        }).collect()
    }

    pub fn bone_count(&self) -> usize { self.bones.len() }
}

impl Default for SkeletonHelper {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_helper_lines() {
        let axes = AxesHelper::new(5.0);
        let lines = axes.lines();
        assert_eq!(lines.len(), 3);
        assert!((lines[0].end.x - 5.0).abs() < 0.01);
        assert!((lines[0].color.r - 1.0).abs() < 0.01); // red X
    }

    #[test]
    fn grid_helper_lines() {
        let grid = GridHelper::new(10.0, 10);
        let lines = grid.lines();
        assert_eq!(lines.len(), grid.line_count());
        assert_eq!(grid.line_count(), 22);
    }

    #[test]
    fn bounding_box_wireframe() {
        let bb = BoundingBoxHelper::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let lines = bb.lines();
        assert_eq!(lines.len(), 12);
        let c = bb.center();
        assert!(c.x.abs() < 0.01 && c.y.abs() < 0.01 && c.z.abs() < 0.01);
    }

    #[test]
    fn bounding_box_extents() {
        let bb = BoundingBoxHelper::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0, 4.0, 5.0));
        let ext = bb.extents();
        assert!((ext.x - 3.0).abs() < 0.01);
        assert!((ext.y - 4.0).abs() < 0.01);
        assert!((ext.z - 5.0).abs() < 0.01);
    }

    #[test]
    fn arrow_helper_lines() {
        let arrow = ArrowHelper::new(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 10.0);
        let lines = arrow.lines();
        assert_eq!(lines.len(), 5); // 1 shaft + 4 head
        let tip = arrow.tip();
        assert!((tip.y - 10.0).abs() < 0.01);
    }

    #[test]
    fn skeleton_helper_bones() {
        let mut skel = SkeletonHelper::new();
        let root = skel.add_bone("root", Vec3::ZERO, None);
        let _spine = skel.add_bone("spine", Vec3::new(0.0, 1.0, 0.0), Some(root));
        let _head = skel.add_bone("head", Vec3::new(0.0, 2.0, 0.0), Some(1));
        assert_eq!(skel.bone_count(), 3);
        let lines = skel.lines();
        assert_eq!(lines.len(), 2); // spine→root, head→spine
    }

    #[test]
    fn grid_center_line_color() {
        let grid = GridHelper::new(10.0, 4);
        let lines = grid.lines();
        // division=4, step=2.5, center line at t=0 has index 2
        let center_line = &lines[4]; // 5th line (index 4 = 3rd pair z-line at t=0)
        assert!((center_line.color.r - 0.5).abs() < 0.01); // center color
    }

    #[test]
    fn axes_with_origin() {
        let axes = AxesHelper::new(3.0).with_origin(Vec3::new(1.0, 2.0, 3.0));
        let lines = axes.lines();
        assert!((lines[0].start.x - 1.0).abs() < 0.01);
        assert!((lines[0].end.x - 4.0).abs() < 0.01);
    }
}
