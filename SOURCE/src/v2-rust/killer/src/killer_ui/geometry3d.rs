//! **Geometry3D** — Three.js-equivalent geometry primitives.
//!
//! Box, Sphere, Plane, Cylinder, Torus, Cone, Ring, BufferGeometry.
//! Vertex/normal/UV generation. Bounding box/sphere computation.

use super::scene3d::Vec3;

// ══════════════════════════════════════════════════════════════════════════════
// BufferGeometry — vertex data container
// ══════════════════════════════════════════════════════════════════════════════

/// Vertex with position, normal, and UV.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: [f64; 2],
}

/// Triangle defined by 3 vertex indices.
#[derive(Debug, Clone, Copy)]
pub struct Triangle(pub u32, pub u32, pub u32);

/// Generic buffer geometry (like Three.js BufferGeometry).
#[derive(Debug, Clone)]
pub struct BufferGeometry {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Triangle>,
    pub bounding_box: Option<BoundingBox>,
    pub bounding_sphere: Option<BoundingSphere>,
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f64,
}

impl BufferGeometry {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<Triangle>) -> Self {
        let mut geo = BufferGeometry { vertices, indices, bounding_box: None, bounding_sphere: None };
        geo.compute_bounds();
        geo
    }

    pub fn vertex_count(&self) -> usize { self.vertices.len() }
    pub fn triangle_count(&self) -> usize { self.indices.len() }

    pub fn compute_bounds(&mut self) {
        if self.vertices.is_empty() { return; }
        let mut min = Vec3::new(f64::MAX, f64::MAX, f64::MAX);
        let mut max = Vec3::new(f64::MIN, f64::MIN, f64::MIN);
        for v in &self.vertices {
            min.x = min.x.min(v.position.x);
            min.y = min.y.min(v.position.y);
            min.z = min.z.min(v.position.z);
            max.x = max.x.max(v.position.x);
            max.y = max.y.max(v.position.y);
            max.z = max.z.max(v.position.z);
        }
        self.bounding_box = Some(BoundingBox { min, max });
        let center = Vec3::new(
            (min.x + max.x) * 0.5, (min.y + max.y) * 0.5, (min.z + max.z) * 0.5
        );
        let radius = self.vertices.iter()
            .map(|v| v.position.distance(&center))
            .fold(0.0f64, f64::max);
        self.bounding_sphere = Some(BoundingSphere { center, radius });
    }

    /// Merge another geometry into this one.
    pub fn merge(&mut self, other: &BufferGeometry) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        for tri in &other.indices {
            self.indices.push(Triangle(tri.0 + offset, tri.1 + offset, tri.2 + offset));
        }
        self.compute_bounds();
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Box Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn box_geometry(width: f64, height: f64, depth: f64) -> BufferGeometry {
    let (hw, hh, hd) = (width * 0.5, height * 0.5, depth * 0.5);
    let faces: [(Vec3, Vec3, [usize; 4]); 6] = [
        (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, hd), [0,1,2,3]),   // front
        (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, -hd), [4,5,6,7]), // back
        (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, hh, 0.0), [8,9,10,11]), // top
        (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, -hh, 0.0), [12,13,14,15]), // bottom
        (Vec3::new(1.0, 0.0, 0.0), Vec3::new(hw, 0.0, 0.0), [16,17,18,19]), // right
        (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-hw, 0.0, 0.0), [20,21,22,23]), // left
    ];

    let corners = [
        Vec3::new(-hw, -hh, hd), Vec3::new(hw, -hh, hd), Vec3::new(hw, hh, hd), Vec3::new(-hw, hh, hd),     // front
        Vec3::new(hw, -hh, -hd), Vec3::new(-hw, -hh, -hd), Vec3::new(-hw, hh, -hd), Vec3::new(hw, hh, -hd), // back
        Vec3::new(-hw, hh, hd), Vec3::new(hw, hh, hd), Vec3::new(hw, hh, -hd), Vec3::new(-hw, hh, -hd),     // top
        Vec3::new(-hw, -hh, -hd), Vec3::new(hw, -hh, -hd), Vec3::new(hw, -hh, hd), Vec3::new(-hw, -hh, hd), // bottom
        Vec3::new(hw, -hh, hd), Vec3::new(hw, -hh, -hd), Vec3::new(hw, hh, -hd), Vec3::new(hw, hh, hd),     // right
        Vec3::new(-hw, -hh, -hd), Vec3::new(-hw, -hh, hd), Vec3::new(-hw, hh, hd), Vec3::new(-hw, hh, -hd), // left
    ];

    let uvs = [[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
    let mut vertices = Vec::with_capacity(24);
    for (i, corner) in corners.iter().enumerate() {
        let face_idx = i / 4;
        vertices.push(Vertex {
            position: *corner,
            normal: faces[face_idx].0,
            uv: uvs[i % 4],
        });
    }

    let mut indices = Vec::with_capacity(12);
    for face in 0..6 {
        let base = (face * 4) as u32;
        indices.push(Triangle(base, base + 1, base + 2));
        indices.push(Triangle(base, base + 2, base + 3));
    }

    BufferGeometry::new(vertices, indices)
}

// ══════════════════════════════════════════════════════════════════════════════
// Sphere Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn sphere_geometry(radius: f64, segments_w: usize, segments_h: usize) -> BufferGeometry {
    let sw = segments_w.max(3);
    let sh = segments_h.max(2);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for j in 0..=sh {
        let v = j as f64 / sh as f64;
        let phi = v * std::f64::consts::PI;
        for i in 0..=sw {
            let u = i as f64 / sw as f64;
            let theta = u * std::f64::consts::TAU;
            let x = -(phi.sin() * theta.cos());
            let y = phi.cos();
            let z = phi.sin() * theta.sin();
            vertices.push(Vertex {
                position: Vec3::new(x * radius, y * radius, z * radius),
                normal: Vec3::new(x, y, z),
                uv: [u, v],
            });
        }
    }

    for j in 0..sh {
        for i in 0..sw {
            let a = (j * (sw + 1) + i) as u32;
            let b = a + 1;
            let c = ((j + 1) * (sw + 1) + i) as u32;
            let d = c + 1;
            if j != 0 { indices.push(Triangle(a, b, c)); }
            if j != sh - 1 { indices.push(Triangle(b, d, c)); }
        }
    }

    BufferGeometry::new(vertices, indices)
}

// ══════════════════════════════════════════════════════════════════════════════
// Plane Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn plane_geometry(width: f64, height: f64, seg_w: usize, seg_h: usize) -> BufferGeometry {
    let sw = seg_w.max(1);
    let sh = seg_h.max(1);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let hw = width * 0.5;
    let hh = height * 0.5;

    for j in 0..=sh {
        let v = j as f64 / sh as f64;
        for i in 0..=sw {
            let u = i as f64 / sw as f64;
            vertices.push(Vertex {
                position: Vec3::new(u * width - hw, 0.0, v * height - hh),
                normal: Vec3::UP,
                uv: [u, v],
            });
        }
    }

    for j in 0..sh {
        for i in 0..sw {
            let a = (j * (sw + 1) + i) as u32;
            let b = a + 1;
            let c = ((j + 1) * (sw + 1) + i) as u32;
            let d = c + 1;
            indices.push(Triangle(a, b, c));
            indices.push(Triangle(b, d, c));
        }
    }

    BufferGeometry::new(vertices, indices)
}

// ══════════════════════════════════════════════════════════════════════════════
// Cylinder Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn cylinder_geometry(radius_top: f64, radius_bottom: f64, height: f64, segments: usize) -> BufferGeometry {
    let seg = segments.max(3);
    let hh = height * 0.5;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Side
    for j in 0..=1 {
        let y = if j == 0 { hh } else { -hh };
        let r = if j == 0 { radius_top } else { radius_bottom };
        for i in 0..=seg {
            let u = i as f64 / seg as f64;
            let theta = u * std::f64::consts::TAU;
            let x = theta.cos() * r;
            let z = theta.sin() * r;
            let slope = (radius_bottom - radius_top) / height;
            let normal = Vec3::new(theta.cos(), slope, theta.sin()).normalize();
            vertices.push(Vertex { position: Vec3::new(x, y, z), normal, uv: [u, j as f64] });
        }
    }
    for i in 0..seg {
        let a = i as u32;
        let b = a + 1;
        let c = (seg + 1 + i) as u32;
        let d = c + 1;
        indices.push(Triangle(a, b, c));
        indices.push(Triangle(b, d, c));
    }

    // Top cap
    let top_center = vertices.len() as u32;
    vertices.push(Vertex { position: Vec3::new(0.0, hh, 0.0), normal: Vec3::UP, uv: [0.5, 0.5] });
    for i in 0..=seg {
        let u = i as f64 / seg as f64;
        let theta = u * std::f64::consts::TAU;
        vertices.push(Vertex {
            position: Vec3::new(theta.cos() * radius_top, hh, theta.sin() * radius_top),
            normal: Vec3::UP, uv: [theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5],
        });
    }
    for i in 0..seg {
        let base = top_center + 1;
        indices.push(Triangle(top_center, base + i as u32, base + i as u32 + 1));
    }

    // Bottom cap
    let bot_center = vertices.len() as u32;
    let down = Vec3::new(0.0, -1.0, 0.0);
    vertices.push(Vertex { position: Vec3::new(0.0, -hh, 0.0), normal: down, uv: [0.5, 0.5] });
    for i in 0..=seg {
        let u = i as f64 / seg as f64;
        let theta = u * std::f64::consts::TAU;
        vertices.push(Vertex {
            position: Vec3::new(theta.cos() * radius_bottom, -hh, theta.sin() * radius_bottom),
            normal: down, uv: [theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5],
        });
    }
    for i in 0..seg {
        let base = bot_center + 1;
        indices.push(Triangle(bot_center, base + i as u32 + 1, base + i as u32));
    }

    BufferGeometry::new(vertices, indices)
}

/// Cone = cylinder with radius_top = 0.
pub fn cone_geometry(radius: f64, height: f64, segments: usize) -> BufferGeometry {
    cylinder_geometry(0.0, radius, height, segments)
}

// ══════════════════════════════════════════════════════════════════════════════
// Torus Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn torus_geometry(radius: f64, tube: f64, radial_seg: usize, tubular_seg: usize) -> BufferGeometry {
    let rs = radial_seg.max(3);
    let ts = tubular_seg.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for j in 0..=rs {
        for i in 0..=ts {
            let u = i as f64 / ts as f64 * std::f64::consts::TAU;
            let v = j as f64 / rs as f64 * std::f64::consts::TAU;
            let x = (radius + tube * v.cos()) * u.cos();
            let y = tube * v.sin();
            let z = (radius + tube * v.cos()) * u.sin();
            let nx = v.cos() * u.cos();
            let ny = v.sin();
            let nz = v.cos() * u.sin();
            vertices.push(Vertex {
                position: Vec3::new(x, y, z),
                normal: Vec3::new(nx, ny, nz).normalize(),
                uv: [i as f64 / ts as f64, j as f64 / rs as f64],
            });
        }
    }

    for j in 0..rs {
        for i in 0..ts {
            let a = (j * (ts + 1) + i) as u32;
            let b = a + 1;
            let c = ((j + 1) * (ts + 1) + i) as u32;
            let d = c + 1;
            indices.push(Triangle(a, b, c));
            indices.push(Triangle(b, d, c));
        }
    }

    BufferGeometry::new(vertices, indices)
}

// ══════════════════════════════════════════════════════════════════════════════
// Ring Geometry
// ══════════════════════════════════════════════════════════════════════════════

pub fn ring_geometry(inner_radius: f64, outer_radius: f64, segments: usize) -> BufferGeometry {
    let seg = segments.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=seg {
        let u = i as f64 / seg as f64;
        let theta = u * std::f64::consts::TAU;
        let c = theta.cos();
        let s = theta.sin();
        vertices.push(Vertex {
            position: Vec3::new(c * inner_radius, 0.0, s * inner_radius),
            normal: Vec3::UP, uv: [u, 0.0],
        });
        vertices.push(Vertex {
            position: Vec3::new(c * outer_radius, 0.0, s * outer_radius),
            normal: Vec3::UP, uv: [u, 1.0],
        });
    }

    for i in 0..seg {
        let base = (i * 2) as u32;
        indices.push(Triangle(base, base + 1, base + 2));
        indices.push(Triangle(base + 1, base + 3, base + 2));
    }

    BufferGeometry::new(vertices, indices)
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_has_24_vertices() {
        let b = box_geometry(1.0, 1.0, 1.0);
        assert_eq!(b.vertex_count(), 24);
        assert_eq!(b.triangle_count(), 12);
        assert!(b.bounding_box.is_some());
    }

    #[test]
    fn sphere_geometry_test() {
        let s = sphere_geometry(1.0, 16, 8);
        assert!(s.vertex_count() > 0);
        assert!(s.triangle_count() > 0);
        let bs = s.bounding_sphere.unwrap();
        assert!((bs.radius - 1.0).abs() < 0.01);
    }

    #[test]
    fn plane_geometry_test() {
        let p = plane_geometry(10.0, 10.0, 1, 1);
        assert_eq!(p.vertex_count(), 4);
        assert_eq!(p.triangle_count(), 2);
    }

    #[test]
    fn cylinder_geometry_test() {
        let c = cylinder_geometry(1.0, 1.0, 2.0, 8);
        assert!(c.vertex_count() > 0);
        assert!(c.triangle_count() > 0);
    }

    #[test]
    fn cone_geometry_test() {
        let c = cone_geometry(1.0, 2.0, 8);
        assert!(c.vertex_count() > 0);
    }

    #[test]
    fn torus_geometry_test() {
        let t = torus_geometry(1.0, 0.4, 8, 6);
        assert!(t.vertex_count() > 0);
        assert!(t.triangle_count() > 0);
    }

    #[test]
    fn ring_geometry_test() {
        let r = ring_geometry(0.5, 1.0, 16);
        assert!(r.vertex_count() > 0);
    }

    #[test]
    fn geometry_merge() {
        let mut a = box_geometry(1.0, 1.0, 1.0);
        let b = box_geometry(1.0, 1.0, 1.0);
        let v_before = a.vertex_count();
        a.merge(&b);
        assert_eq!(a.vertex_count(), v_before * 2);
    }

    #[test]
    fn bounding_box_correct() {
        let b = box_geometry(2.0, 4.0, 6.0);
        let bb = b.bounding_box.unwrap();
        assert!((bb.min.x + 1.0).abs() < 1e-6);
        assert!((bb.max.x - 1.0).abs() < 1e-6);
        assert!((bb.min.y + 2.0).abs() < 1e-6);
        assert!((bb.max.y - 2.0).abs() < 1e-6);
    }
}
