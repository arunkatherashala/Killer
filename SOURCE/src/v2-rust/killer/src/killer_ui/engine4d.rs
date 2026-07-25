//! **Engine4D** — 4D geometry, rotation, and projection engine.
//!
//! Tesseracts, hyperspheres, 4D→3D slicing and projection, 6 rotation planes,
//! 4D camera, Vec4/Mat5 math.

use super::scene3d::{Vec3, Color3};

// ══════════════════════════════════════════════════════════════════════════════
// Vec4 — 4D vector
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct Vec4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Vec4 {
    pub const ZERO: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const ONE: Vec4 = Vec4 { x: 1.0, y: 1.0, z: 1.0, w: 1.0 };
    pub const UNIT_W: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self { Vec4 { x, y, z, w } }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let l = self.length();
        if l < 1e-12 { return *self; }
        Vec4 { x: self.x / l, y: self.y / l, z: self.z / l, w: self.w / l }
    }

    pub fn dot(&self, other: &Vec4) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    pub fn add(&self, other: &Vec4) -> Self {
        Vec4 { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z, w: self.w + other.w }
    }

    pub fn sub(&self, other: &Vec4) -> Self {
        Vec4 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z, w: self.w - other.w }
    }

    pub fn scale(&self, s: f64) -> Self {
        Vec4 { x: self.x * s, y: self.y * s, z: self.z * s, w: self.w * s }
    }

    pub fn lerp(&self, other: &Vec4, t: f64) -> Self {
        self.add(&other.sub(self).scale(t))
    }

    pub fn distance(&self, other: &Vec4) -> f64 { self.sub(other).length() }

    /// Project 4D → 3D by dropping w (orthographic).
    pub fn to_vec3_ortho(&self) -> Vec3 { Vec3::new(self.x, self.y, self.z) }

    /// Project 4D → 3D via perspective from w-axis (viewer at w=d).
    pub fn to_vec3_perspective(&self, d: f64) -> Vec3 {
        let scale = d / (d - self.w);
        Vec3::new(self.x * scale, self.y * scale, self.z * scale)
    }

    /// Stereographic projection from 4D → 3D.
    pub fn to_vec3_stereographic(&self) -> Vec3 {
        let denom = 1.0 - self.w;
        if denom.abs() < 1e-12 { return Vec3::new(self.x * 1e6, self.y * 1e6, self.z * 1e6); }
        Vec3::new(self.x / denom, self.y / denom, self.z / denom)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Mat5 — 5×5 matrix for 4D homogeneous transforms
// ══════════════════════════════════════════════════════════════════════════════

/// 5×5 matrix stored row-major for 4D transformations.
#[derive(Debug, Clone, Copy)]
pub struct Mat5 {
    pub m: [f64; 25],
}

impl Mat5 {
    pub const IDENTITY: Mat5 = Mat5 { m: [
        1.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 1.0,
    ]};

    fn at(&self, row: usize, col: usize) -> f64 { self.m[row * 5 + col] }
    fn set(&mut self, row: usize, col: usize, v: f64) { self.m[row * 5 + col] = v; }

    pub fn multiply(&self, other: &Mat5) -> Mat5 {
        let mut r = Mat5 { m: [0.0; 25] };
        for i in 0..5 {
            for j in 0..5 {
                let mut s = 0.0;
                for k in 0..5 { s += self.at(i, k) * other.at(k, j); }
                r.set(i, j, s);
            }
        }
        r
    }

    pub fn transform(&self, v: &Vec4) -> Vec4 {
        let arr = [v.x, v.y, v.z, v.w, 1.0];
        let mut out = [0.0f64; 5];
        for i in 0..5 {
            for j in 0..5 { out[i] += self.at(i, j) * arr[j]; }
        }
        let w = if out[4].abs() < 1e-12 { 1.0 } else { out[4] };
        Vec4::new(out[0] / w, out[1] / w, out[2] / w, out[3] / w)
    }

    pub fn translation(dx: f64, dy: f64, dz: f64, dw: f64) -> Mat5 {
        let mut m = Self::IDENTITY;
        m.set(0, 4, dx); m.set(1, 4, dy); m.set(2, 4, dz); m.set(3, 4, dw);
        m
    }

    pub fn scaling(sx: f64, sy: f64, sz: f64, sw: f64) -> Mat5 {
        let mut m = Mat5 { m: [0.0; 25] };
        m.set(0, 0, sx); m.set(1, 1, sy); m.set(2, 2, sz); m.set(3, 3, sw); m.set(4, 4, 1.0);
        m
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 4D Rotations — 6 planes of rotation
// ══════════════════════════════════════════════════════════════════════════════

/// The 6 simple rotation planes in 4D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationPlane4D {
    XY, XZ, XW, YZ, YW, ZW,
}

/// Build a 4D rotation matrix in the given plane by angle (radians).
pub fn rotation_4d(plane: RotationPlane4D, angle: f64) -> Mat5 {
    let c = angle.cos();
    let s = angle.sin();
    let mut m = Mat5::IDENTITY;
    match plane {
        RotationPlane4D::XY => { m.set(0,0,c); m.set(0,1,-s); m.set(1,0,s); m.set(1,1,c); }
        RotationPlane4D::XZ => { m.set(0,0,c); m.set(0,2,-s); m.set(2,0,s); m.set(2,2,c); }
        RotationPlane4D::XW => { m.set(0,0,c); m.set(0,3,-s); m.set(3,0,s); m.set(3,3,c); }
        RotationPlane4D::YZ => { m.set(1,1,c); m.set(1,2,-s); m.set(2,1,s); m.set(2,2,c); }
        RotationPlane4D::YW => { m.set(1,1,c); m.set(1,3,-s); m.set(3,1,s); m.set(3,3,c); }
        RotationPlane4D::ZW => { m.set(2,2,c); m.set(2,3,-s); m.set(3,2,s); m.set(3,3,c); }
    }
    m
}

/// Compose a double rotation (two independent planes, like the Clifford torus).
pub fn double_rotation(plane_a: RotationPlane4D, angle_a: f64,
                        plane_b: RotationPlane4D, angle_b: f64) -> Mat5 {
    rotation_4d(plane_a, angle_a).multiply(&rotation_4d(plane_b, angle_b))
}

// ══════════════════════════════════════════════════════════════════════════════
// 4D Geometry Primitives
// ══════════════════════════════════════════════════════════════════════════════

/// 4D mesh — vertices + edges (we visualize wireframes since 4D faces are volumes).
#[derive(Debug, Clone)]
pub struct Mesh4D {
    pub vertices: Vec<Vec4>,
    pub edges: Vec<(usize, usize)>,
    pub faces: Vec<Vec<usize>>,  // optional face indices for slicing
    pub color: Color3,
}

/// Tesseract (4D hypercube) centered at origin with half-size s.
pub fn tesseract(s: f64) -> Mesh4D {
    let mut vertices = Vec::new();
    for i in 0..16u8 {
        let x = if i & 1 != 0 { s } else { -s };
        let y = if i & 2 != 0 { s } else { -s };
        let z = if i & 4 != 0 { s } else { -s };
        let w = if i & 8 != 0 { s } else { -s };
        vertices.push(Vec4::new(x, y, z, w));
    }
    // Connect vertices that differ in exactly one coordinate
    let mut edges = Vec::new();
    for i in 0..16usize {
        for j in (i+1)..16usize {
            let diff = (i ^ j) as u32;
            if diff.count_ones() == 1 { edges.push((i, j)); }
        }
    }
    Mesh4D { vertices, edges, faces: Vec::new(), color: Color3::WHITE }
}

/// 4D Simplex (5-cell / pentachoron).
pub fn simplex_4d(s: f64) -> Mesh4D {
    let a = s / (2.0f64.sqrt());
    let vertices = vec![
        Vec4::new(a, a, a, -a / 5.0f64.sqrt()),
        Vec4::new(a, -a, -a, -a / 5.0f64.sqrt()),
        Vec4::new(-a, a, -a, -a / 5.0f64.sqrt()),
        Vec4::new(-a, -a, a, -a / 5.0f64.sqrt()),
        Vec4::new(0.0, 0.0, 0.0, a * 4.0 / 5.0f64.sqrt()),
    ];
    let mut edges = Vec::new();
    for i in 0..5usize {
        for j in (i+1)..5usize { edges.push((i, j)); }
    }
    Mesh4D { vertices, edges, faces: Vec::new(), color: Color3::new(0.0, 1.0, 1.0) }
}

/// 4D Hypersphere approximation — vertices on the 3-sphere.
pub fn hypersphere_4d(radius: f64, rings: usize, segments: usize) -> Mesh4D {
    let mut vertices = Vec::new();
    let pi = std::f64::consts::PI;
    for i in 0..=rings {
        let phi = pi * i as f64 / rings as f64;
        for j in 0..segments {
            let theta = 2.0 * pi * j as f64 / segments as f64;
            for k in 0..segments {
                let psi = 2.0 * pi * k as f64 / segments as f64;
                vertices.push(Vec4::new(
                    radius * phi.sin() * theta.sin() * psi.sin(),
                    radius * phi.sin() * theta.sin() * psi.cos(),
                    radius * phi.sin() * theta.cos(),
                    radius * phi.cos(),
                ));
            }
        }
    }
    // Connect adjacent vertices in each ring
    let mut edges = Vec::new();
    let per_ring = segments * segments;
    for i in 0..vertices.len() {
        let next = i + 1;
        if next % segments != 0 && next < vertices.len() { edges.push((i, next)); }
        let below = i + per_ring;
        if below < vertices.len() { edges.push((i, below)); }
    }
    Mesh4D { vertices, edges, faces: Vec::new(), color: Color3::new(1.0, 0.5, 0.0) }
}

/// 16-cell (4D cross-polytope / hyperoctahedron).
pub fn cross_polytope_4d(s: f64) -> Mesh4D {
    let vertices = vec![
        Vec4::new(s, 0.0, 0.0, 0.0), Vec4::new(-s, 0.0, 0.0, 0.0),
        Vec4::new(0.0, s, 0.0, 0.0), Vec4::new(0.0, -s, 0.0, 0.0),
        Vec4::new(0.0, 0.0, s, 0.0), Vec4::new(0.0, 0.0, -s, 0.0),
        Vec4::new(0.0, 0.0, 0.0, s), Vec4::new(0.0, 0.0, 0.0, -s),
    ];
    let mut edges = Vec::new();
    for i in 0..8usize {
        for j in (i+1)..8usize {
            // Connect all non-antipodal pairs
            if i / 2 != j / 2 { edges.push((i, j)); }
        }
    }
    Mesh4D { vertices, edges, faces: Vec::new(), color: Color3::GREEN }
}

/// 24-cell (self-dual 4D regular polytope).
pub fn cell24(s: f64) -> Mesh4D {
    let mut vertices = Vec::new();
    // 8 vertices like 16-cell
    let perms = [(s,0.0,0.0,0.0),(-s,0.0,0.0,0.0),(0.0,s,0.0,0.0),(0.0,-s,0.0,0.0),
                 (0.0,0.0,s,0.0),(0.0,0.0,-s,0.0),(0.0,0.0,0.0,s),(0.0,0.0,0.0,-s)];
    for &(a,b,c,d) in &perms { vertices.push(Vec4::new(a,b,c,d)); }
    // 16 vertices at (±s/2, ±s/2, ±s/2, ±s/2)
    let h = s / 2.0;
    for i in 0..16u8 {
        let x = if i & 1 != 0 { h } else { -h };
        let y = if i & 2 != 0 { h } else { -h };
        let z = if i & 4 != 0 { h } else { -h };
        let w = if i & 8 != 0 { h } else { -h };
        vertices.push(Vec4::new(x, y, z, w));
    }
    // Edges: vertices at distance s from each other
    let mut edges = Vec::new();
    let thr = s * s + 0.01;
    for i in 0..vertices.len() {
        for j in (i+1)..vertices.len() {
            let d2 = vertices[i].sub(&vertices[j]).dot(&vertices[i].sub(&vertices[j]));
            if d2 < thr && d2 > thr * 0.4 { edges.push((i, j)); }
        }
    }
    Mesh4D { vertices, edges, faces: Vec::new(), color: Color3::new(1.0, 1.0, 0.0) }
}

// ══════════════════════════════════════════════════════════════════════════════
// 4D Slice — hyperplane cross-section
// ══════════════════════════════════════════════════════════════════════════════

/// Take a 3D slice of a 4D mesh at w = w_plane.
pub fn slice_4d(mesh: &Mesh4D, w_plane: f64) -> Vec<(Vec3, Vec3)> {
    let mut segments = Vec::new();
    for &(a, b) in &mesh.edges {
        let va = &mesh.vertices[a];
        let vb = &mesh.vertices[b];
        let dw = vb.w - va.w;
        if dw.abs() < 1e-12 {
            // Edge is parallel to w-plane
            if (va.w - w_plane).abs() < 0.01 {
                segments.push((va.to_vec3_ortho(), vb.to_vec3_ortho()));
            }
            continue;
        }
        let t = (w_plane - va.w) / dw;
        if t < 0.0 || t > 1.0 { continue; }
        let intersection = va.lerp(vb, t).to_vec3_ortho();
        // Find other edges sharing vertices to form cross-section segments
        segments.push((intersection, intersection));
    }
    segments
}

// ══════════════════════════════════════════════════════════════════════════════
// 4D Camera / Projector
// ══════════════════════════════════════════════════════════════════════════════

/// 4D Camera for projecting 4D scenes to 3D.
#[derive(Debug, Clone)]
pub struct Camera4D {
    pub position: Vec4,
    pub target: Vec4,
    pub projection_distance: f64,
    pub projection: Projection4D,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection4D {
    Perspective,
    Orthographic,
    Stereographic,
}

impl Camera4D {
    pub fn new() -> Self {
        Camera4D {
            position: Vec4::new(0.0, 0.0, 0.0, 5.0),
            target: Vec4::ZERO,
            projection_distance: 5.0,
            projection: Projection4D::Perspective,
        }
    }

    /// Project a 4D mesh to 3D vertices.
    pub fn project_mesh(&self, mesh: &Mesh4D) -> Vec<Vec3> {
        mesh.vertices.iter().map(|v| self.project_point(v)).collect()
    }

    pub fn project_point(&self, v: &Vec4) -> Vec3 {
        let translated = v.sub(&self.position);
        match self.projection {
            Projection4D::Perspective => translated.to_vec3_perspective(self.projection_distance),
            Projection4D::Orthographic => translated.to_vec3_ortho(),
            Projection4D::Stereographic => translated.to_vec3_stereographic(),
        }
    }
}

impl Default for Camera4D {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// 4D Transform Node
// ══════════════════════════════════════════════════════════════════════════════

/// A node in a 4D scene graph.
#[derive(Debug, Clone)]
pub struct Object4D {
    pub id: usize,
    pub name: String,
    pub position: Vec4,
    pub rotation_angles: [(RotationPlane4D, f64); 6],
    pub scale: Vec4,
    pub mesh: Option<Mesh4D>,
    pub children: Vec<usize>,
    pub visible: bool,
}

impl Object4D {
    pub fn new(id: usize, name: &str) -> Self {
        Object4D {
            id, name: name.into(),
            position: Vec4::ZERO,
            rotation_angles: [
                (RotationPlane4D::XY, 0.0), (RotationPlane4D::XZ, 0.0),
                (RotationPlane4D::XW, 0.0), (RotationPlane4D::YZ, 0.0),
                (RotationPlane4D::YW, 0.0), (RotationPlane4D::ZW, 0.0),
            ],
            scale: Vec4::ONE,
            mesh: None,
            children: Vec::new(),
            visible: true,
        }
    }

    /// Compute the combined transform matrix.
    pub fn transform_matrix(&self) -> Mat5 {
        let mut rot = Mat5::IDENTITY;
        for &(plane, angle) in &self.rotation_angles {
            if angle.abs() > 1e-12 {
                rot = rot.multiply(&rotation_4d(plane, angle));
            }
        }
        let t = Mat5::translation(self.position.x, self.position.y, self.position.z, self.position.w);
        let s = Mat5::scaling(self.scale.x, self.scale.y, self.scale.z, self.scale.w);
        t.multiply(&rot).multiply(&s)
    }

    /// Apply transform to mesh and return transformed vertices.
    pub fn transformed_vertices(&self) -> Vec<Vec4> {
        let mat = self.transform_matrix();
        match &self.mesh {
            Some(m) => m.vertices.iter().map(|v| mat.transform(v)).collect(),
            None => Vec::new(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn vec4_ops() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let c = a.add(&b);
        assert!((c.x - 6.0).abs() < 1e-10);
        assert!((a.dot(&b) - 70.0).abs() < 1e-10); // 5+12+21+32=70
        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn mat5_identity() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let out = Mat5::IDENTITY.transform(&v);
        assert!((out.x - 1.0).abs() < 1e-10);
        assert!((out.w - 4.0).abs() < 1e-10);
    }

    #[test]
    fn mat5_translation() {
        let mat = Mat5::translation(10.0, 20.0, 30.0, 40.0);
        let v = Vec4::ZERO;
        let out = mat.transform(&v);
        assert!((out.x - 10.0).abs() < 1e-10);
        assert!((out.w - 40.0).abs() < 1e-10);
    }

    #[test]
    fn rotation_xy() {
        let rot = rotation_4d(RotationPlane4D::XY, PI / 2.0);
        let v = Vec4::new(1.0, 0.0, 0.0, 0.0);
        let out = rot.transform(&v);
        assert!((out.x).abs() < 1e-10);
        assert!((out.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tesseract_structure() {
        let t = tesseract(1.0);
        assert_eq!(t.vertices.len(), 16); // 2^4 vertices
        assert_eq!(t.edges.len(), 32);    // 4 edges per vertex, counted once
    }

    #[test]
    fn simplex_structure() {
        let s = simplex_4d(1.0);
        assert_eq!(s.vertices.len(), 5);
        assert_eq!(s.edges.len(), 10); // C(5,2)
    }

    #[test]
    fn cross_polytope_structure() {
        let c = cross_polytope_4d(1.0);
        assert_eq!(c.vertices.len(), 8);
        assert_eq!(c.edges.len(), 24); // Each of 8 vertices connects to 6 others
    }

    #[test]
    fn perspective_projection() {
        let v = Vec4::new(1.0, 2.0, 3.0, 0.0);
        let p = v.to_vec3_perspective(5.0);
        // scale = 5/(5-0) = 1.0
        assert!((p.x - 1.0).abs() < 1e-10);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 2.5);
        let p2 = v2.to_vec3_perspective(5.0);
        // scale = 5/2.5 = 2.0
        assert!((p2.x - 2.0).abs() < 1e-10);
    }

    #[test]
    fn camera_project() {
        let cam = Camera4D::new();
        let cube = tesseract(1.0);
        let projected = cam.project_mesh(&cube);
        assert_eq!(projected.len(), 16);
    }

    #[test]
    fn object4d_transform() {
        let mut obj = Object4D::new(0, "box4d");
        obj.mesh = Some(tesseract(1.0));
        obj.position = Vec4::new(10.0, 0.0, 0.0, 0.0);
        let verts = obj.transformed_vertices();
        assert_eq!(verts.len(), 16);
        // All x-coords should be near 10 ± 1
        for v in &verts {
            assert!(v.x > 8.5 && v.x < 11.5);
        }
    }

    #[test]
    fn double_rotation_compose() {
        let rot = double_rotation(RotationPlane4D::XY, PI / 4.0, RotationPlane4D::ZW, PI / 4.0);
        let v = Vec4::new(1.0, 0.0, 0.0, 1.0);
        let out = rot.transform(&v);
        assert!((out.length() - v.length()).abs() < 1e-10); // Rotation preserves length
    }

    #[test]
    fn slice_tesseract() {
        let t = tesseract(1.0);
        let sliced = slice_4d(&t, 0.0);
        assert!(!sliced.is_empty());
    }
}
