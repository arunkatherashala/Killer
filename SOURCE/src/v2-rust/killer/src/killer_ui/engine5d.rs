//! **Engine5D** — 5D geometry, manifold slicing, and dimensional projection.
//!
//! Vec5, Mat6, 5D polytopes, 5D→3D cascaded projection,
//! 10 rotation planes, dimensional slicing for visualization.

use super::scene3d::{Vec3, Color3};
use super::engine4d::Vec4;

// ══════════════════════════════════════════════════════════════════════════════
// Vec5 — 5D vector
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct Vec5 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub v: f64,
}

impl Vec5 {
    pub const ZERO: Vec5 = Vec5 { x: 0.0, y: 0.0, z: 0.0, w: 0.0, v: 0.0 };
    pub const ONE: Vec5 = Vec5 { x: 1.0, y: 1.0, z: 1.0, w: 1.0, v: 1.0 };

    pub fn new(x: f64, y: f64, z: f64, w: f64, v: f64) -> Self { Vec5 { x, y, z, w, v } }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z
         + self.w * self.w + self.v * self.v).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let l = self.length();
        if l < 1e-12 { return *self; }
        Vec5 { x: self.x / l, y: self.y / l, z: self.z / l, w: self.w / l, v: self.v / l }
    }

    pub fn dot(&self, o: &Vec5) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w + self.v * o.v
    }

    pub fn add(&self, o: &Vec5) -> Vec5 {
        Vec5 { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z, w: self.w + o.w, v: self.v + o.v }
    }

    pub fn sub(&self, o: &Vec5) -> Vec5 {
        Vec5 { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z, w: self.w - o.w, v: self.v - o.v }
    }

    pub fn scale(&self, s: f64) -> Vec5 {
        Vec5 { x: self.x * s, y: self.y * s, z: self.z * s, w: self.w * s, v: self.v * s }
    }

    pub fn lerp(&self, o: &Vec5, t: f64) -> Vec5 { self.add(&o.sub(self).scale(t)) }

    pub fn distance(&self, o: &Vec5) -> f64 { self.sub(o).length() }

    /// Drop v coordinate → Vec4.
    pub fn to_vec4(&self) -> Vec4 { Vec4::new(self.x, self.y, self.z, self.w) }

    /// Drop w and v → Vec3.
    pub fn to_vec3(&self) -> Vec3 { Vec3::new(self.x, self.y, self.z) }

    /// Perspective projection 5D→4D with viewer at v=d.
    pub fn project_to_4d(&self, d: f64) -> Vec4 {
        let s = d / (d - self.v);
        Vec4::new(self.x * s, self.y * s, self.z * s, self.w * s)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Mat6 — 6×6 matrix for 5D homogeneous transforms
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct Mat6 {
    pub m: [f64; 36],
}

impl Mat6 {
    pub const IDENTITY: Mat6 = Mat6 { m: [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]};

    fn at(&self, r: usize, c: usize) -> f64 { self.m[r * 6 + c] }
    fn set(&mut self, r: usize, c: usize, v: f64) { self.m[r * 6 + c] = v; }

    pub fn multiply(&self, other: &Mat6) -> Mat6 {
        let mut out = Mat6 { m: [0.0; 36] };
        for i in 0..6 {
            for j in 0..6 {
                let mut s = 0.0;
                for k in 0..6 { s += self.at(i, k) * other.at(k, j); }
                out.set(i, j, s);
            }
        }
        out
    }

    pub fn transform(&self, v: &Vec5) -> Vec5 {
        let arr = [v.x, v.y, v.z, v.w, v.v, 1.0];
        let mut out = [0.0f64; 6];
        for i in 0..6 {
            for j in 0..6 { out[i] += self.at(i, j) * arr[j]; }
        }
        let h = if out[5].abs() < 1e-12 { 1.0 } else { out[5] };
        Vec5::new(out[0] / h, out[1] / h, out[2] / h, out[3] / h, out[4] / h)
    }

    pub fn translation(dx: f64, dy: f64, dz: f64, dw: f64, dv: f64) -> Mat6 {
        let mut m = Self::IDENTITY;
        m.set(0, 5, dx); m.set(1, 5, dy); m.set(2, 5, dz); m.set(3, 5, dw); m.set(4, 5, dv);
        m
    }

    pub fn scaling(sx: f64, sy: f64, sz: f64, sw: f64, sv: f64) -> Mat6 {
        let mut m = Mat6 { m: [0.0; 36] };
        m.set(0, 0, sx); m.set(1, 1, sy); m.set(2, 2, sz);
        m.set(3, 3, sw); m.set(4, 4, sv); m.set(5, 5, 1.0);
        m
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 5D Rotations — 10 planes C(5,2)
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationPlane5D {
    XY, XZ, XW, XV,
    YZ, YW, YV,
    ZW, ZV,
    WV,
}

impl RotationPlane5D {
    /// Returns the two axis indices (0=x, 1=y, 2=z, 3=w, 4=v).
    pub fn axes(&self) -> (usize, usize) {
        match self {
            RotationPlane5D::XY => (0, 1), RotationPlane5D::XZ => (0, 2),
            RotationPlane5D::XW => (0, 3), RotationPlane5D::XV => (0, 4),
            RotationPlane5D::YZ => (1, 2), RotationPlane5D::YW => (1, 3),
            RotationPlane5D::YV => (1, 4), RotationPlane5D::ZW => (2, 3),
            RotationPlane5D::ZV => (2, 4), RotationPlane5D::WV => (3, 4),
        }
    }
}

/// Build a 5D rotation in the given plane.
pub fn rotation_5d(plane: RotationPlane5D, angle: f64) -> Mat6 {
    let c = angle.cos();
    let s = angle.sin();
    let mut m = Mat6::IDENTITY;
    let (a, b) = plane.axes();
    m.set(a, a, c); m.set(a, b, -s);
    m.set(b, a, s); m.set(b, b, c);
    m
}

// ══════════════════════════════════════════════════════════════════════════════
// 5D Polytopes
// ══════════════════════════════════════════════════════════════════════════════

/// 5D mesh (wireframe-based).
#[derive(Debug, Clone)]
pub struct Mesh5D {
    pub vertices: Vec<Vec5>,
    pub edges: Vec<(usize, usize)>,
    pub color: Color3,
}

/// Penteract — 5D hypercube (32 vertices, 80 edges).
pub fn penteract(s: f64) -> Mesh5D {
    let mut vertices = Vec::new();
    for i in 0..32u8 {
        let x = if i & 1 != 0 { s } else { -s };
        let y = if i & 2 != 0 { s } else { -s };
        let z = if i & 4 != 0 { s } else { -s };
        let w = if i & 8 != 0 { s } else { -s };
        let v = if i & 16 != 0 { s } else { -s };
        vertices.push(Vec5::new(x, y, z, w, v));
    }
    let mut edges = Vec::new();
    for i in 0..32usize {
        for j in (i+1)..32usize {
            let diff = (i ^ j) as u32;
            if diff.count_ones() == 1 { edges.push((i, j)); }
        }
    }
    Mesh5D { vertices, edges, color: Color3::WHITE }
}

/// 5-simplex (6 vertices, 15 edges).
pub fn simplex_5d(s: f64) -> Mesh5D {
    // Regular 5-simplex via standard construction
    let vertices = vec![
        Vec5::new(s, 0.0, 0.0, 0.0, 0.0),
        Vec5::new(0.0, s, 0.0, 0.0, 0.0),
        Vec5::new(0.0, 0.0, s, 0.0, 0.0),
        Vec5::new(0.0, 0.0, 0.0, s, 0.0),
        Vec5::new(0.0, 0.0, 0.0, 0.0, s),
        Vec5::new(-s/5.0, -s/5.0, -s/5.0, -s/5.0, -s/5.0),
    ];
    let mut edges = Vec::new();
    for i in 0..6usize {
        for j in (i+1)..6usize { edges.push((i, j)); }
    }
    Mesh5D { vertices, edges, color: Color3::new(0.0, 1.0, 1.0) }
}

/// 5D cross-polytope (10 vertices, 40 edges).
pub fn cross_polytope_5d(s: f64) -> Mesh5D {
    let mut vertices = Vec::new();
    for d in 0..5 {
        let mut pos = [0.0; 5];
        pos[d] = s;
        vertices.push(Vec5::new(pos[0], pos[1], pos[2], pos[3], pos[4]));
        pos[d] = -s;
        vertices.push(Vec5::new(pos[0], pos[1], pos[2], pos[3], pos[4]));
    }
    let mut edges = Vec::new();
    for i in 0..10usize {
        for j in (i+1)..10usize {
            if i / 2 != j / 2 { edges.push((i, j)); }
        }
    }
    Mesh5D { vertices, edges, color: Color3::GREEN }
}

// ══════════════════════════════════════════════════════════════════════════════
// 5D → 3D Cascaded Projection Pipeline
// ══════════════════════════════════════════════════════════════════════════════

/// Projection method for each dimension drop.
#[derive(Debug, Clone, Copy)]
pub enum DimProjection {
    Perspective(f64), // distance
    Orthographic,
}

/// Full 5D → 3D projection pipeline.
pub struct ProjectionPipeline {
    pub stage_5to4: DimProjection,
    pub stage_4to3: DimProjection,
}

impl ProjectionPipeline {
    pub fn perspective(d5: f64, d4: f64) -> Self {
        ProjectionPipeline {
            stage_5to4: DimProjection::Perspective(d5),
            stage_4to3: DimProjection::Perspective(d4),
        }
    }

    pub fn orthographic() -> Self {
        ProjectionPipeline {
            stage_5to4: DimProjection::Orthographic,
            stage_4to3: DimProjection::Orthographic,
        }
    }

    /// Project a 5D point all the way down to 3D.
    pub fn project(&self, v: &Vec5) -> Vec3 {
        let v4 = match self.stage_5to4 {
            DimProjection::Perspective(d) => v.project_to_4d(d),
            DimProjection::Orthographic => v.to_vec4(),
        };
        match self.stage_4to3 {
            DimProjection::Perspective(d) => v4.to_vec3_perspective(d),
            DimProjection::Orthographic => v4.to_vec3_ortho(),
        }
    }

    /// Project an entire 5D mesh to 3D edge pairs for rendering.
    pub fn project_mesh(&self, mesh: &Mesh5D) -> Vec<(Vec3, Vec3)> {
        let projected: Vec<Vec3> = mesh.vertices.iter().map(|v| self.project(v)).collect();
        mesh.edges.iter().map(|&(a, b)| (projected[a], projected[b])).collect()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 5D Hyperplane Slicing
// ══════════════════════════════════════════════════════════════════════════════

/// Slice a 5D mesh with a v=constant hyperplane, producing a 4D mesh.
pub fn slice_5d_to_4d(mesh: &Mesh5D, v_plane: f64) -> Vec<(Vec4, Vec4)> {
    let mut segments = Vec::new();
    for &(a, b) in &mesh.edges {
        let va = &mesh.vertices[a];
        let vb = &mesh.vertices[b];
        let dv = vb.v - va.v;
        if dv.abs() < 1e-12 {
            if (va.v - v_plane).abs() < 0.01 {
                segments.push((va.to_vec4(), vb.to_vec4()));
            }
            continue;
        }
        let t = (v_plane - va.v) / dv;
        if t < 0.0 || t > 1.0 { continue; }
        let pt = va.lerp(vb, t).to_vec4();
        segments.push((pt, pt));
    }
    segments
}

/// Double-slice: 5D → v-plane → w-plane → 3D geometry.
pub fn slice_5d_to_3d(mesh: &Mesh5D, v_plane: f64, w_plane: f64) -> Vec<Vec3> {
    let mut points = Vec::new();
    for &(a, b) in &mesh.edges {
        let va = &mesh.vertices[a];
        let vb = &mesh.vertices[b];
        let dv = vb.v - va.v;
        if dv.abs() < 1e-12 { continue; }
        let t1 = (v_plane - va.v) / dv;
        if t1 < 0.0 || t1 > 1.0 { continue; }
        let p4 = va.lerp(vb, t1);
        // Now slice at w = w_plane
        let dw = p4.w - w_plane;
        if dw.abs() < 0.5 { // Accept points near the w-plane
            points.push(p4.to_vec3());
        }
    }
    points
}

// ══════════════════════════════════════════════════════════════════════════════
// 5D Data Manifold Visualization
// ══════════════════════════════════════════════════════════════════════════════

/// Maps 5D data points to 3D with color encoding the 4th and 5th dimensions.
pub fn manifold_to_visual(data: &[Vec5], pipeline: &ProjectionPipeline) -> Vec<(Vec3, Color3)> {
    if data.is_empty() { return Vec::new(); }
    // Find range of w and v for color mapping
    let w_min = data.iter().map(|d| d.w).fold(f64::MAX, f64::min);
    let w_max = data.iter().map(|d| d.w).fold(f64::MIN, f64::max);
    let v_min = data.iter().map(|d| d.v).fold(f64::MAX, f64::min);
    let v_max = data.iter().map(|d| d.v).fold(f64::MIN, f64::max);
    let w_range = (w_max - w_min).max(1e-12);
    let v_range = (v_max - v_min).max(1e-12);

    data.iter().map(|pt| {
        let pos = pipeline.project(pt);
        let w_norm = (pt.w - w_min) / w_range;
        let v_norm = (pt.v - v_min) / v_range;
        // Encode w as red→green hue, v as brightness
        let color = Color3::new(
            1.0 - w_norm,
            w_norm,
            v_norm,
        );
        (pos, color)
    }).collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec5_ops() {
        let a = Vec5::new(1.0, 2.0, 3.0, 4.0, 5.0);
        let b = Vec5::new(5.0, 4.0, 3.0, 2.0, 1.0);
        assert!((a.dot(&b) - 35.0).abs() < 1e-10); // 5+8+9+8+5=35
        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn mat6_identity() {
        let v = Vec5::new(1.0, 2.0, 3.0, 4.0, 5.0);
        let out = Mat6::IDENTITY.transform(&v);
        assert!((out.x - 1.0).abs() < 1e-10);
        assert!((out.v - 5.0).abs() < 1e-10);
    }

    #[test]
    fn mat6_translation() {
        let m = Mat6::translation(10.0, 20.0, 30.0, 40.0, 50.0);
        let out = m.transform(&Vec5::ZERO);
        assert!((out.x - 10.0).abs() < 1e-10);
        assert!((out.v - 50.0).abs() < 1e-10);
    }

    #[test]
    fn rotation_5d_preserves_length() {
        let rot = rotation_5d(RotationPlane5D::XV, std::f64::consts::PI / 3.0);
        let v = Vec5::new(1.0, 2.0, 3.0, 4.0, 5.0);
        let out = rot.transform(&v);
        assert!((out.length() - v.length()).abs() < 1e-10);
    }

    #[test]
    fn penteract_structure() {
        let p = penteract(1.0);
        assert_eq!(p.vertices.len(), 32); // 2^5
        assert_eq!(p.edges.len(), 80);    // 5 * 2^4
    }

    #[test]
    fn simplex_5d_structure() {
        let s = simplex_5d(1.0);
        assert_eq!(s.vertices.len(), 6);
        assert_eq!(s.edges.len(), 15); // C(6,2)
    }

    #[test]
    fn cross_polytope_5d_structure() {
        let c = cross_polytope_5d(1.0);
        assert_eq!(c.vertices.len(), 10);
        assert_eq!(c.edges.len(), 40); // Each connects to 8 others, /2
    }

    #[test]
    fn projection_pipeline() {
        let pipe = ProjectionPipeline::perspective(5.0, 5.0);
        let v = Vec5::new(1.0, 2.0, 3.0, 0.0, 0.0);
        let p = pipe.project(&v);
        // At v=0,w=0 with d=5: both scalings = 5/5 = 1.0
        assert!((p.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn project_penteract() {
        let p = penteract(1.0);
        let pipe = ProjectionPipeline::perspective(5.0, 5.0);
        let edges_3d = pipe.project_mesh(&p);
        assert_eq!(edges_3d.len(), 80);
    }

    #[test]
    fn manifold_visualization() {
        let data: Vec<Vec5> = (0..10).map(|i| {
            let t = i as f64 / 9.0;
            Vec5::new(t, t * 2.0, t * 3.0, t, 1.0 - t)
        }).collect();
        let pipe = ProjectionPipeline::orthographic();
        let vis = manifold_to_visual(&data, &pipe);
        assert_eq!(vis.len(), 10);
        for (_, c) in &vis {
            assert!(c.r >= 0.0 && c.r <= 1.0);
            assert!(c.g >= 0.0 && c.g <= 1.0);
        }
    }

    #[test]
    fn slice_5d() {
        let p = penteract(1.0);
        let sliced = slice_5d_to_4d(&p, 0.0);
        assert!(!sliced.is_empty());
    }
}
