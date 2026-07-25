//! **Sprite & Billboard** — Camera-facing quads for particles, labels, imposters.
//!
//! Also includes InstancedMesh for efficient rendering of many copies,
//! and Fog (linear + exponential) for scene atmosphere.

use super::scene3d::{Vec3, Color3, Mat4};

// ══════════════════════════════════════════════════════════════════════════════
// Sprite
// ══════════════════════════════════════════════════════════════════════════════

/// A camera-facing 2D sprite in 3D space.
#[derive(Debug, Clone)]
pub struct Sprite {
    pub position: Vec3,
    pub scale: Vec3,
    pub color: Color3,
    pub opacity: f64,
    pub texture_id: Option<u64>,
    pub center: [f64; 2],  // anchor point (0.5, 0.5) = center
}

impl Sprite {
    pub fn new(position: Vec3) -> Self {
        Sprite {
            position, scale: Vec3::new(1.0, 1.0, 1.0),
            color: Color3::WHITE, opacity: 1.0,
            texture_id: None, center: [0.5, 0.5],
        }
    }

    pub fn with_scale(mut self, w: f64, h: f64) -> Self {
        self.scale = Vec3::new(w, h, 1.0); self
    }

    pub fn with_color(mut self, c: Color3) -> Self { self.color = c; self }
    pub fn with_texture(mut self, id: u64) -> Self { self.texture_id = Some(id); self }

    /// Compute billboard model matrix (always faces camera).
    pub fn billboard_matrix(&self, camera_right: &Vec3, camera_up: &Vec3) -> Mat4 {
        let r = camera_right;
        let u = camera_up;
        let f = r.cross(u).normalize();
        let p = self.position;
        let sx = self.scale.x;
        let sy = self.scale.y;
        // column-major flat [f64;16]
        Mat4 { m: [
            r.x * sx, r.y * sx, r.z * sx, 0.0,
            u.x * sy, u.y * sy, u.z * sy, 0.0,
            f.x,      f.y,      f.z,      0.0,
            p.x,      p.y,      p.z,      1.0,
        ]}
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Instanced Mesh
// ══════════════════════════════════════════════════════════════════════════════

/// Per-instance data for instanced rendering.
#[derive(Debug, Clone)]
pub struct InstanceData {
    pub transform: Mat4,
    pub color: Option<Color3>,
}

/// Instanced mesh manager — renders many copies of same geometry efficiently.
#[derive(Debug, Clone)]
pub struct InstancedMesh {
    pub geometry_id: u64,
    pub material_id: u64,
    pub instances: Vec<InstanceData>,
    pub max_count: usize,
    pub frustum_culled: bool,
}

impl InstancedMesh {
    pub fn new(geometry_id: u64, material_id: u64, max: usize) -> Self {
        InstancedMesh {
            geometry_id, material_id,
            instances: Vec::with_capacity(max),
            max_count: max, frustum_culled: true,
        }
    }

    pub fn add_instance(&mut self, transform: Mat4, color: Option<Color3>) -> Option<usize> {
        if self.instances.len() >= self.max_count { return None; }
        let idx = self.instances.len();
        self.instances.push(InstanceData { transform, color });
        Some(idx)
    }

    pub fn set_transform(&mut self, idx: usize, transform: Mat4) {
        if let Some(inst) = self.instances.get_mut(idx) { inst.transform = transform; }
    }

    pub fn set_color(&mut self, idx: usize, color: Color3) {
        if let Some(inst) = self.instances.get_mut(idx) { inst.color = Some(color); }
    }

    pub fn count(&self) -> usize { self.instances.len() }

    pub fn clear(&mut self) { self.instances.clear(); }
}

// ══════════════════════════════════════════════════════════════════════════════
// Fog
// ══════════════════════════════════════════════════════════════════════════════

/// Scene fog types.
#[derive(Debug, Clone)]
pub enum Fog {
    /// Linear fog: fully transparent at near, fully opaque at far.
    Linear { color: Color3, near: f64, far: f64 },
    /// Exponential fog: density-based falloff.
    Exponential { color: Color3, density: f64 },
    /// Exponential squared fog (denser).
    ExponentialSquared { color: Color3, density: f64 },
}

impl Fog {
    pub fn linear(color: Color3, near: f64, far: f64) -> Self {
        Fog::Linear { color, near, far }
    }

    pub fn exp(color: Color3, density: f64) -> Self {
        Fog::Exponential { color, density }
    }

    pub fn exp2(color: Color3, density: f64) -> Self {
        Fog::ExponentialSquared { color, density }
    }

    /// Compute fog factor (0 = no fog, 1 = fully fogged) for a given distance.
    pub fn factor(&self, distance: f64) -> f64 {
        match self {
            Fog::Linear { near, far, .. } => {
                ((far - distance) / (far - near)).clamp(0.0, 1.0)
            }
            Fog::Exponential { density, .. } => {
                (-density * distance).exp().clamp(0.0, 1.0)
            }
            Fog::ExponentialSquared { density, .. } => {
                let d = density * distance;
                (-d * d).exp().clamp(0.0, 1.0)
            }
        }
    }

    /// Apply fog to a pixel color.
    pub fn apply(&self, pixel: &Color3, distance: f64) -> Color3 {
        let f = self.factor(distance);
        let fog_color = match self {
            Fog::Linear { color, .. } | Fog::Exponential { color, .. } | Fog::ExponentialSquared { color, .. } => color,
        };
        Color3 {
            r: pixel.r * f + fog_color.r * (1.0 - f),
            g: pixel.g * f + fog_color.g * (1.0 - f),
            b: pixel.b * f + fog_color.b * (1.0 - f),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_basic() {
        let s = Sprite::new(Vec3::new(1.0, 2.0, 3.0)).with_scale(2.0, 2.0);
        assert!((s.scale.x - 2.0).abs() < 0.01);
    }

    #[test]
    fn sprite_billboard() {
        let s = Sprite::new(Vec3::ZERO);
        let right = Vec3::new(1.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let mat = s.billboard_matrix(&right, &up);
        assert!((mat.m[0] - 1.0).abs() < 0.01); // right.x * scale
    }

    #[test]
    fn instanced_mesh_add() {
        let mut im = InstancedMesh::new(1, 1, 100);
        let idx = im.add_instance(Mat4::IDENTITY, None).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(im.count(), 1);
    }

    #[test]
    fn instanced_mesh_max() {
        let mut im = InstancedMesh::new(1, 1, 2);
        im.add_instance(Mat4::IDENTITY, None);
        im.add_instance(Mat4::IDENTITY, None);
        assert!(im.add_instance(Mat4::IDENTITY, None).is_none());
    }

    #[test]
    fn instanced_mesh_color() {
        let mut im = InstancedMesh::new(1, 1, 10);
        im.add_instance(Mat4::IDENTITY, None);
        im.set_color(0, Color3 { r: 1.0, g: 0.0, b: 0.0 });
        assert!(im.instances[0].color.is_some());
    }

    #[test]
    fn fog_linear() {
        let fog = Fog::linear(Color3::WHITE, 10.0, 100.0);
        let f = fog.factor(10.0);
        assert!((f - 1.0).abs() < 0.01); // at near => no fog
        let f = fog.factor(100.0);
        assert!((f - 0.0).abs() < 0.01); // at far => full fog
    }

    #[test]
    fn fog_exp() {
        let fog = Fog::exp(Color3::WHITE, 0.1);
        let f = fog.factor(0.0);
        assert!((f - 1.0).abs() < 0.01); // at origin => no fog
        let f = fog.factor(50.0);
        assert!(f < 0.01); // far away => heavy fog
    }

    #[test]
    fn fog_apply_color() {
        let fog = Fog::linear(Color3 { r: 0.5, g: 0.5, b: 0.5 }, 0.0, 100.0);
        let pixel = Color3 { r: 1.0, g: 0.0, b: 0.0 };
        let fogged = fog.apply(&pixel, 50.0);
        assert!((fogged.r - 0.75).abs() < 0.01);
        assert!((fogged.g - 0.25).abs() < 0.01);
    }

    #[test]
    fn fog_exp_squared() {
        let fog = Fog::exp2(Color3::WHITE, 0.05);
        let f0 = fog.factor(0.0);
        let f10 = fog.factor(10.0);
        assert!((f0 - 1.0).abs() < 0.01);
        assert!(f10 < f0);
    }
}
