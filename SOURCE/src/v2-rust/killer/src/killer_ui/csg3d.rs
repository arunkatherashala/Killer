//! **CSG 3D** — Constructive Solid Geometry (Boolean operations on meshes).
//!
//! Union, Intersect, Subtract operations using BSP tree decomposition.
//! Also: extrude path, lathe (revolve), and parametric surface generators.

use super::scene3d::Vec3;
use super::geometry3d::{BufferGeometry, Vertex, Triangle};

// ══════════════════════════════════════════════════════════════════════════════
// CSG Polygon / Plane
// ══════════════════════════════════════════════════════════════════════════════

/// A polygon in the CSG tree (convex, 3+ vertices).
#[derive(Debug, Clone)]
pub struct CsgPolygon {
    pub vertices: Vec<Vec3>,
    pub normal: Vec3,
}

impl CsgPolygon {
    pub fn from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let ab = b.sub(&a);
        let ac = c.sub(&a);
        let normal = ab.cross(&ac).normalize();
        CsgPolygon { vertices: vec![a, b, c], normal }
    }

    pub fn flip(&mut self) {
        self.vertices.reverse();
        self.normal = self.normal.scale(-1.0);
    }

    pub fn plane(&self) -> CsgPlane {
        CsgPlane { normal: self.normal, w: self.normal.dot(&self.vertices[0]) }
    }
}

/// Plane defined by normal · p = w.
#[derive(Debug, Clone, Copy)]
pub struct CsgPlane {
    pub normal: Vec3,
    pub w: f64,
}

const EPSILON: f64 = 1e-5;

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;

impl CsgPlane {
    pub fn classify_point(&self, p: &Vec3) -> u8 {
        let t = self.normal.dot(p) - self.w;
        if t > EPSILON { FRONT }
        else if t < -EPSILON { BACK }
        else { COPLANAR }
    }

    /// Split polygon by this plane into front/back/coplanar lists.
    pub fn split_polygon(&self, polygon: &CsgPolygon, coplanar_front: &mut Vec<CsgPolygon>,
                          coplanar_back: &mut Vec<CsgPolygon>, front: &mut Vec<CsgPolygon>, back: &mut Vec<CsgPolygon>) {
        let mut polygon_type = 0u8;
        let types: Vec<u8> = polygon.vertices.iter().map(|v| {
            let t = self.classify_point(v);
            polygon_type |= t;
            t
        }).collect();

        match polygon_type {
            COPLANAR => {
                if self.normal.dot(&polygon.normal) > 0.0 {
                    coplanar_front.push(polygon.clone());
                } else {
                    coplanar_back.push(polygon.clone());
                }
            }
            FRONT => front.push(polygon.clone()),
            BACK => back.push(polygon.clone()),
            _ => {
                // SPANNING — split into front and back
                let mut f_verts = Vec::new();
                let mut b_verts = Vec::new();
                for i in 0..polygon.vertices.len() {
                    let j = (i + 1) % polygon.vertices.len();
                    let ti = types[i];
                    let tj = types[j];
                    let vi = &polygon.vertices[i];
                    let vj = &polygon.vertices[j];

                    if ti != BACK { f_verts.push(*vi); }
                    if ti != FRONT { b_verts.push(*vi); }

                    if (ti | tj) == SPANNING {
                        let t = (self.w - self.normal.dot(vi)) / self.normal.dot(&vj.sub(vi));
                        let v = vi.lerp(vj, t);
                        f_verts.push(v);
                        b_verts.push(v);
                    }
                }
                if f_verts.len() >= 3 {
                    front.push(CsgPolygon { vertices: f_verts, normal: polygon.normal });
                }
                if b_verts.len() >= 3 {
                    back.push(CsgPolygon { vertices: b_verts, normal: polygon.normal });
                }
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CSG BSP Node
// ══════════════════════════════════════════════════════════════════════════════

/// BSP tree node for CSG operations.
pub struct CsgNode {
    plane: Option<CsgPlane>,
    front: Option<Box<CsgNode>>,
    back: Option<Box<CsgNode>>,
    polygons: Vec<CsgPolygon>,
}

impl CsgNode {
    pub fn new(polygons: Vec<CsgPolygon>) -> Self {
        let mut node = CsgNode { plane: None, front: None, back: None, polygons: Vec::new() };
        if !polygons.is_empty() { node.build(polygons); }
        node
    }

    fn invert(&mut self) {
        for poly in &mut self.polygons { poly.flip(); }
        if let Some(ref mut plane) = self.plane {
            plane.normal = plane.normal.scale(-1.0);
            plane.w = -plane.w;
        }
        if let Some(ref mut f) = self.front { f.invert(); }
        if let Some(ref mut b) = self.back { b.invert(); }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    fn clip_polygons(&self, polygons: &[CsgPolygon]) -> Vec<CsgPolygon> {
        let plane = match &self.plane {
            Some(p) => p,
            None => return polygons.to_vec(),
        };
        let mut front = Vec::new();
        let mut back = Vec::new();
        for poly in polygons {
            let mut cf = Vec::new();
            let mut cb = Vec::new();
            plane.split_polygon(poly, &mut cf, &mut cb, &mut front, &mut back);
            front.extend(cf);
            back.extend(cb);
        }
        if let Some(ref f) = self.front { front = f.clip_polygons(&front); }
        if let Some(ref b) = self.back { back = b.clip_polygons(&back); } else { back.clear(); }
        front.extend(back);
        front
    }

    fn clip_to(&mut self, bsp: &CsgNode) {
        self.polygons = bsp.clip_polygons(&self.polygons);
        if let Some(ref mut f) = self.front { f.clip_to(bsp); }
        if let Some(ref mut b) = self.back { b.clip_to(bsp); }
    }

    fn all_polygons(&self) -> Vec<CsgPolygon> {
        let mut polys = self.polygons.clone();
        if let Some(ref f) = self.front { polys.extend(f.all_polygons()); }
        if let Some(ref b) = self.back { polys.extend(b.all_polygons()); }
        polys
    }

    fn build(&mut self, polygons: Vec<CsgPolygon>) {
        if polygons.is_empty() { return; }
        if self.plane.is_none() {
            self.plane = Some(polygons[0].plane());
        }
        let plane = self.plane.unwrap();
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut coplanar: Vec<CsgPolygon> = Vec::new();
        for poly in &polygons {
            let mut co_front: Vec<CsgPolygon> = Vec::new();
            let mut co_back: Vec<CsgPolygon> = Vec::new();
            plane.split_polygon(poly, &mut co_front, &mut co_back, &mut front, &mut back);
            coplanar.extend(co_front);
            coplanar.extend(co_back);
        }
        self.polygons.extend(coplanar);
        if !front.is_empty() {
            if self.front.is_none() { self.front = Some(Box::new(CsgNode { plane: None, front: None, back: None, polygons: Vec::new() })); }
            self.front.as_mut().unwrap().build(front);
        }
        if !back.is_empty() {
            if self.back.is_none() { self.back = Some(Box::new(CsgNode { plane: None, front: None, back: None, polygons: Vec::new() })); }
            self.back.as_mut().unwrap().build(back);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CSG Operations
// ══════════════════════════════════════════════════════════════════════════════

/// Convert BufferGeometry to CSG polygons.
pub fn geometry_to_csg(geo: &BufferGeometry) -> Vec<CsgPolygon> {
    geo.indices.iter().map(|tri| {
        let a = geo.vertices[tri.0 as usize].position;
        let b = geo.vertices[tri.1 as usize].position;
        let c = geo.vertices[tri.2 as usize].position;
        CsgPolygon::from_triangle(a, b, c)
    }).collect()
}

/// Convert CSG polygons back to BufferGeometry.
pub fn csg_to_geometry(polygons: &[CsgPolygon]) -> BufferGeometry {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for poly in polygons {
        let base = vertices.len() as u32;
        for v in &poly.vertices {
            vertices.push(Vertex { position: *v, normal: poly.normal, uv: [0.0, 0.0] });
        }
        // Fan triangulate
        for i in 1..poly.vertices.len() as u32 - 1 {
            indices.push(Triangle(base, base + i, base + i + 1));
        }
    }
    let mut geo = BufferGeometry { vertices, indices, bounding_box: None, bounding_sphere: None };
    geo.compute_bounds();
    geo
}

/// CSG Union: a ∪ b.
pub fn csg_union(a: &BufferGeometry, b: &BufferGeometry) -> BufferGeometry {
    let mut node_a = CsgNode::new(geometry_to_csg(a));
    let mut node_b = CsgNode::new(geometry_to_csg(b));
    node_a.clip_to(&node_b);
    node_b.clip_to(&node_a);
    node_b.invert();
    node_b.clip_to(&node_a);
    node_b.invert();
    node_a.build(node_b.all_polygons());
    csg_to_geometry(&node_a.all_polygons())
}

/// CSG Subtract: a - b.
pub fn csg_subtract(a: &BufferGeometry, b: &BufferGeometry) -> BufferGeometry {
    let mut node_a = CsgNode::new(geometry_to_csg(a));
    let mut node_b = CsgNode::new(geometry_to_csg(b));
    node_a.invert();
    node_a.clip_to(&node_b);
    node_b.clip_to(&node_a);
    node_b.invert();
    node_b.clip_to(&node_a);
    node_a.build(node_b.all_polygons());
    node_a.invert();
    csg_to_geometry(&node_a.all_polygons())
}

/// CSG Intersect: a ∩ b.
pub fn csg_intersect(a: &BufferGeometry, b: &BufferGeometry) -> BufferGeometry {
    let mut node_a = CsgNode::new(geometry_to_csg(a));
    let mut node_b = CsgNode::new(geometry_to_csg(b));
    node_a.invert();
    node_b.clip_to(&node_a);
    node_b.invert();
    node_a.clip_to(&node_b);
    node_b.clip_to(&node_a);
    node_a.build(node_b.all_polygons());
    node_a.invert();
    csg_to_geometry(&node_a.all_polygons())
}

// ══════════════════════════════════════════════════════════════════════════════
// Extrude & Lathe
// ══════════════════════════════════════════════════════════════════════════════

/// Extrude a 2D path along a direction.
pub fn extrude(path: &[Vec3], direction: &Vec3, cap: bool) -> BufferGeometry {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let n = path.len();

    // Bottom ring
    for p in path {
        vertices.push(Vertex { position: *p, normal: direction.scale(-1.0).normalize(), uv: [0.0, 0.0] });
    }
    // Top ring
    for p in path {
        let top = p.add(direction);
        vertices.push(Vertex { position: top, normal: direction.normalize(), uv: [0.0, 1.0] });
    }
    // Side quads
    for i in 0..n {
        let j = (i + 1) % n;
        let b0 = i as u32;
        let b1 = j as u32;
        let t0 = (n + i) as u32;
        let t1 = (n + j) as u32;
        indices.push(Triangle(b0, b1, t0));
        indices.push(Triangle(b1, t1, t0));
    }
    // Caps
    if cap && n >= 3 {
        for i in 1..n as u32 - 1 {
            indices.push(Triangle(0, i + 1, i));
            indices.push(Triangle(n as u32, n as u32 + i, n as u32 + i + 1));
        }
    }
    let mut geo = BufferGeometry { vertices, indices, bounding_box: None, bounding_sphere: None };
    geo.compute_bounds();
    geo
}

/// Lathe (revolve) a 2D profile around the Y axis.
pub fn lathe(profile: &[Vec3], segments: usize) -> BufferGeometry {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let pi2 = std::f64::consts::PI * 2.0;

    for s in 0..=segments {
        let angle = pi2 * s as f64 / segments as f64;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        for p in profile {
            let x = p.x * cos_a;
            let z = p.x * sin_a;
            let normal = Vec3::new(cos_a, 0.0, sin_a);
            vertices.push(Vertex { position: Vec3::new(x, p.y, z), normal, uv: [s as f64 / segments as f64, 0.0] });
        }
    }

    let rows = profile.len();
    for s in 0..segments {
        for r in 0..rows - 1 {
            let a = (s * rows + r) as u32;
            let b = ((s + 1) * rows + r) as u32;
            let c = ((s + 1) * rows + r + 1) as u32;
            let d = (s * rows + r + 1) as u32;
            indices.push(Triangle(a, b, d));
            indices.push(Triangle(b, c, d));
        }
    }
    let mut geo = BufferGeometry { vertices, indices, bounding_box: None, bounding_sphere: None };
    geo.compute_bounds();
    geo
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::geometry3d::box_geometry;

    #[test]
    fn polygon_from_triangle() {
        let p = CsgPolygon::from_triangle(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        assert!((p.normal.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn polygon_flip() {
        let mut p = CsgPolygon::from_triangle(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let n_before = p.normal.z;
        p.flip();
        assert!((p.normal.z + n_before).abs() < 0.01);
    }

    #[test]
    fn geometry_csg_roundtrip() {
        let box_geo = box_geometry(1.0, 1.0, 1.0);
        let polys = geometry_to_csg(&box_geo);
        assert!(!polys.is_empty());
        let result = csg_to_geometry(&polys);
        assert!(!result.vertices.is_empty());
    }

    #[test]
    fn csg_union_two_boxes() {
        let a = box_geometry(2.0, 2.0, 2.0);
        let b = box_geometry(2.0, 2.0, 2.0);
        let result = csg_union(&a, &b);
        assert!(!result.vertices.is_empty());
    }

    #[test]
    fn csg_subtract_boxes() {
        let a = box_geometry(2.0, 2.0, 2.0);
        let b = box_geometry(1.0, 1.0, 1.0);
        let result = csg_subtract(&a, &b);
        assert!(!result.vertices.is_empty());
    }

    #[test]
    fn csg_intersect_boxes() {
        let a = box_geometry(2.0, 2.0, 2.0);
        let b = box_geometry(2.0, 2.0, 2.0);
        let result = csg_intersect(&a, &b);
        assert!(!result.vertices.is_empty());
    }

    #[test]
    fn extrude_path() {
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let result = extrude(&path, &Vec3::new(0.0, 2.0, 0.0), true);
        assert!(!result.vertices.is_empty());
        assert_eq!(result.vertices.len(), 8);
    }

    #[test]
    fn lathe_profile() {
        let profile = vec![
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.0, 0.5, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let result = lathe(&profile, 8);
        assert!(!result.vertices.is_empty());
        assert!(!result.indices.is_empty());
    }
}
