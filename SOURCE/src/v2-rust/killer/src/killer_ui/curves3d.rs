//! **Parametric Curves** — CatmullRom, Bezier, NURBS paths, path extrusion.
//!
//! Curve evaluation, arc-length parameterization, and extrusion
//! of 2D profiles along 3D curves.

use super::scene3d::Vec3;
use super::geometry3d::{BufferGeometry, Vertex, Triangle};

// ══════════════════════════════════════════════════════════════════════════════
// Curve Trait
// ══════════════════════════════════════════════════════════════════════════════

/// Common interface for all parametric curves.
pub trait Curve {
    /// Evaluate the curve at parameter t ∈ [0, 1].
    fn point_at(&self, t: f64) -> Vec3;

    /// Tangent vector at parameter t.
    fn tangent_at(&self, t: f64) -> Vec3 {
        let dt = 0.0001;
        let t0 = (t - dt).max(0.0);
        let t1 = (t + dt).min(1.0);
        self.point_at(t1).sub(&self.point_at(t0)).normalize()
    }

    /// Sample N points along the curve.
    fn sample(&self, n: usize) -> Vec<Vec3> {
        (0..=n).map(|i| self.point_at(i as f64 / n as f64)).collect()
    }

    /// Approximate arc length with N segments.
    fn arc_length(&self, segments: usize) -> f64 {
        let pts = self.sample(segments);
        pts.windows(2).map(|pair| pair[0].sub(&pair[1]).length()).sum()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// CatmullRom Spline
// ══════════════════════════════════════════════════════════════════════════════

/// Catmull-Rom spline through control points.
#[derive(Debug, Clone)]
pub struct CatmullRomCurve {
    pub points: Vec<Vec3>,
    pub tension: f64,
    pub closed: bool,
}

impl CatmullRomCurve {
    pub fn new(points: Vec<Vec3>) -> Self {
        CatmullRomCurve { points, tension: 0.5, closed: false }
    }

    pub fn closed(mut self) -> Self { self.closed = true; self }
    pub fn with_tension(mut self, t: f64) -> Self { self.tension = t; self }
}

impl Curve for CatmullRomCurve {
    fn point_at(&self, t: f64) -> Vec3 {
        let n = self.points.len();
        if n < 2 { return if n == 1 { self.points[0] } else { Vec3::ZERO }; }
        let total = if self.closed { n } else { n - 1 };
        let scaled = t * total as f64;
        let i = (scaled.floor() as usize).min(total - 1);
        let frac = scaled - i as f64;

        let get = |idx: i64| -> Vec3 {
            let idx = if self.closed {
                ((idx % n as i64) + n as i64) as usize % n
            } else {
                (idx.max(0) as usize).min(n - 1)
            };
            self.points[idx]
        };

        let p0 = get(i as i64 - 1);
        let p1 = get(i as i64);
        let p2 = get(i as i64 + 1);
        let p3 = get(i as i64 + 2);

        catmull_rom_interp(p0, p1, p2, p3, frac, self.tension)
    }
}

fn catmull_rom_interp(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f64, tension: f64) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    let s = tension;
    let interp = |a: f64, b: f64, c: f64, d: f64| -> f64 {
        let m1 = s * (c - a);
        let m2 = s * (d - b);
        (2.0 * t3 - 3.0 * t2 + 1.0) * b + (t3 - 2.0 * t2 + t) * m1
            + (-2.0 * t3 + 3.0 * t2) * c + (t3 - t2) * m2
    };
    Vec3::new(
        interp(p0.x, p1.x, p2.x, p3.x),
        interp(p0.y, p1.y, p2.y, p3.y),
        interp(p0.z, p1.z, p2.z, p3.z),
    )
}

// ══════════════════════════════════════════════════════════════════════════════
// Cubic Bezier
// ══════════════════════════════════════════════════════════════════════════════

/// Cubic Bezier curve with 4 control points.
#[derive(Debug, Clone)]
pub struct CubicBezier {
    pub p0: Vec3, pub p1: Vec3, pub p2: Vec3, pub p3: Vec3,
}

impl CubicBezier {
    pub fn new(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        CubicBezier { p0, p1, p2, p3 }
    }
}

impl Curve for CubicBezier {
    fn point_at(&self, t: f64) -> Vec3 {
        let u = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        let t2 = t * t;
        let t3 = t2 * t;
        Vec3::new(
            u3 * self.p0.x + 3.0 * u2 * t * self.p1.x + 3.0 * u * t2 * self.p2.x + t3 * self.p3.x,
            u3 * self.p0.y + 3.0 * u2 * t * self.p1.y + 3.0 * u * t2 * self.p2.y + t3 * self.p3.y,
            u3 * self.p0.z + 3.0 * u2 * t * self.p1.z + 3.0 * u * t2 * self.p2.z + t3 * self.p3.z,
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Quadratic Bezier
// ══════════════════════════════════════════════════════════════════════════════

/// Quadratic Bezier curve with 3 control points.
#[derive(Debug, Clone)]
pub struct QuadraticBezier {
    pub p0: Vec3, pub p1: Vec3, pub p2: Vec3,
}

impl Curve for QuadraticBezier {
    fn point_at(&self, t: f64) -> Vec3 {
        let u = 1.0 - t;
        Vec3::new(
            u * u * self.p0.x + 2.0 * u * t * self.p1.x + t * t * self.p2.x,
            u * u * self.p0.y + 2.0 * u * t * self.p1.y + t * t * self.p2.y,
            u * u * self.p0.z + 2.0 * u * t * self.p1.z + t * t * self.p2.z,
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Line Curve
// ══════════════════════════════════════════════════════════════════════════════

/// Simple line segment between two points.
#[derive(Debug, Clone)]
pub struct LineCurve {
    pub start: Vec3, pub end: Vec3,
}

impl Curve for LineCurve {
    fn point_at(&self, t: f64) -> Vec3 {
        Vec3::new(
            self.start.x + (self.end.x - self.start.x) * t,
            self.start.y + (self.end.y - self.start.y) * t,
            self.start.z + (self.end.z - self.start.z) * t,
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Path Extrusion
// ══════════════════════════════════════════════════════════════════════════════

/// Extrude a circular tube along a curve.
pub fn extrude_along_curve(curve: &dyn Curve, radius: f64, segments: usize, radial: usize) -> BufferGeometry {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let pos = curve.point_at(t);
        let tangent = curve.tangent_at(t);

        // Build local frame (Frenet frame)
        let up = if tangent.y.abs() < 0.99 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
        let normal = tangent.cross(&up).normalize();
        let binormal = tangent.cross(&normal).normalize();

        for j in 0..radial {
            let angle = 2.0 * std::f64::consts::PI * j as f64 / radial as f64;
            let (cos_a, sin_a) = (angle.cos(), angle.sin());
            let n = normal.scale(cos_a).add(&binormal.scale(sin_a));
            let p = pos.add(&n.scale(radius));
            vertices.push(Vertex { position: p, normal: n, uv: [t, j as f64 / radial as f64] });
        }
    }

    // Generate indices
    for i in 0..segments {
        for j in 0..radial {
            let a = (i * radial + j) as u32;
            let b = (i * radial + (j + 1) % radial) as u32;
            let c = ((i + 1) * radial + j) as u32;
            let d = ((i + 1) * radial + (j + 1) % radial) as u32;
            indices.push(Triangle(a, b, c));
            indices.push(Triangle(b, d, c));
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

    #[test]
    fn catmull_rom_basic() {
        let curve = CatmullRomCurve::new(vec![
            Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 0.0),
        ]);
        let start = curve.point_at(0.0);
        let end = curve.point_at(1.0);
        assert!((start.x - 0.0).abs() < 0.01);
        assert!((end.x - 3.0).abs() < 0.01);
    }

    #[test]
    fn catmull_rom_closed() {
        let curve = CatmullRomCurve::new(vec![
            Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0),
        ]).closed();
        let p0 = curve.point_at(0.0);
        let p1 = curve.point_at(1.0);
        assert!((p0.x - p1.x).abs() < 0.01);
    }

    #[test]
    fn cubic_bezier() {
        let curve = CubicBezier::new(
            Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0),
        );
        let mid = curve.point_at(0.5);
        assert!((mid.x - 0.5).abs() < 0.1);
        assert!(mid.y > 0.3);
    }

    #[test]
    fn quadratic_bezier() {
        let curve = QuadraticBezier {
            p0: Vec3::ZERO, p1: Vec3::new(0.5, 1.0, 0.0), p2: Vec3::new(1.0, 0.0, 0.0),
        };
        let mid = curve.point_at(0.5);
        assert!((mid.x - 0.5).abs() < 0.01);
        assert!((mid.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn line_curve() {
        let curve = LineCurve { start: Vec3::ZERO, end: Vec3::new(10.0, 0.0, 0.0) };
        assert!((curve.point_at(0.5).x - 5.0).abs() < 0.01);
        assert!((curve.arc_length(100) - 10.0).abs() < 0.1);
    }

    #[test]
    fn curve_sample() {
        let curve = LineCurve { start: Vec3::ZERO, end: Vec3::new(1.0, 0.0, 0.0) };
        let pts = curve.sample(10);
        assert_eq!(pts.len(), 11);
    }

    #[test]
    fn curve_tangent() {
        let curve = LineCurve { start: Vec3::ZERO, end: Vec3::new(1.0, 0.0, 0.0) };
        let t = curve.tangent_at(0.5);
        assert!((t.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn tube_extrude() {
        let curve = LineCurve { start: Vec3::ZERO, end: Vec3::new(0.0, 5.0, 0.0) };
        let geo = extrude_along_curve(&curve, 0.5, 8, 6);
        assert!(geo.vertices.len() > 0);
        assert!(geo.indices.len() > 0);
    }
}
