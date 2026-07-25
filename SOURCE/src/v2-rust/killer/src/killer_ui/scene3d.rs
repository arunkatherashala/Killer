//! **Scene3D** — Three.js-equivalent scene graph engine.
//!
//! Scene, Object3D hierarchy, Camera (Perspective/Orthographic),
//! Lights (Ambient/Directional/Point/Spot), Fog, Background.
//! Complete 3D scene management with matrix transforms and parent-child chains.

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// 3D Math types
// ══════════════════════════════════════════════════════════════════════════════

/// 3D vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const UP: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const FORWARD: Vec3 = Vec3 { x: 0.0, y: 0.0, z: -1.0 };
    pub const RIGHT: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self { Vec3 { x, y, z } }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Vec3 {
        let len = self.length();
        if len < 1e-12 { return Vec3::ZERO; }
        Vec3 { x: self.x / len, y: self.y / len, z: self.z / len }
    }

    pub fn dot(&self, other: &Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn add(&self, other: &Vec3) -> Vec3 {
        Vec3 { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }

    pub fn sub(&self, other: &Vec3) -> Vec3 {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }

    pub fn scale(&self, s: f64) -> Vec3 {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }

    pub fn lerp(&self, other: &Vec3, t: f64) -> Vec3 {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    pub fn distance(&self, other: &Vec3) -> f64 {
        self.sub(other).length()
    }
}

/// Quaternion for rotation.
#[derive(Debug, Clone, Copy)]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub fn from_axis_angle(axis: &Vec3, angle_rad: f64) -> Self {
        let half = angle_rad * 0.5;
        let s = half.sin();
        let n = axis.normalize();
        Quat { x: n.x * s, y: n.y * s, z: n.z * s, w: half.cos() }
    }

    pub fn from_euler(x: f64, y: f64, z: f64) -> Self {
        let (sx, cx) = (x * 0.5).sin_cos();
        let (sy, cy) = (y * 0.5).sin_cos();
        let (sz, cz) = (z * 0.5).sin_cos();
        Quat {
            x: sx * cy * cz - cx * sy * sz,
            y: cx * sy * cz + sx * cy * sz,
            z: cx * cy * sz - sx * sy * cz,
            w: cx * cy * cz + sx * sy * sz,
        }
    }

    pub fn multiply(&self, other: &Quat) -> Quat {
        Quat {
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        }
    }

    pub fn rotate_vec3(&self, v: &Vec3) -> Vec3 {
        let u = Vec3::new(self.x, self.y, self.z);
        let s = self.w;
        let d = u.dot(v);
        let c = u.cross(v);
        u.scale(2.0 * d)
            .add(&v.scale(s * s - u.dot(&u)))
            .add(&c.scale(2.0 * s))
    }

    pub fn slerp(&self, other: &Quat, t: f64) -> Quat {
        let mut dot = self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w;
        let (ox, oy, oz, ow) = if dot < 0.0 {
            dot = -dot;
            (-other.x, -other.y, -other.z, -other.w)
        } else {
            (other.x, other.y, other.z, other.w)
        };
        if dot > 0.9995 {
            return Quat {
                x: self.x + (ox - self.x) * t,
                y: self.y + (oy - self.y) * t,
                z: self.z + (oz - self.z) * t,
                w: self.w + (ow - self.w) * t,
            };
        }
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;
        Quat {
            x: self.x * a + ox * b,
            y: self.y * a + oy * b,
            z: self.z * a + oz * b,
            w: self.w * a + ow * b,
        }
    }
}

/// 4×4 matrix (column-major).
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    pub m: [f64; 16],
}

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4 { m: [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]};

    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        let mut m = Self::IDENTITY;
        m.m[12] = x; m.m[13] = y; m.m[14] = z;
        m
    }

    pub fn scaling(x: f64, y: f64, z: f64) -> Self {
        let mut m = Self::IDENTITY;
        m.m[0] = x; m.m[5] = y; m.m[10] = z;
        m
    }

    pub fn from_quat(q: &Quat) -> Self {
        let (xx, yy, zz) = (q.x * q.x, q.y * q.y, q.z * q.z);
        let (xy, xz, yz) = (q.x * q.y, q.x * q.z, q.y * q.z);
        let (wx, wy, wz) = (q.w * q.x, q.w * q.y, q.w * q.z);
        Mat4 { m: [
            1.0 - 2.0 * (yy + zz), 2.0 * (xy + wz), 2.0 * (xz - wy), 0.0,
            2.0 * (xy - wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz + wx), 0.0,
            2.0 * (xz + wy), 2.0 * (yz - wx), 1.0 - 2.0 * (xx + yy), 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]}
    }

    pub fn perspective(fov_rad: f64, aspect: f64, near: f64, far: f64) -> Self {
        let f = 1.0 / (fov_rad * 0.5).tan();
        let range_inv = 1.0 / (near - far);
        Mat4 { m: [
            f / aspect, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (far + near) * range_inv, -1.0,
            0.0, 0.0, 2.0 * far * near * range_inv, 0.0,
        ]}
    }

    pub fn orthographic(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        let w = right - left;
        let h = top - bottom;
        let d = far - near;
        Mat4 { m: [
            2.0 / w, 0.0, 0.0, 0.0,
            0.0, 2.0 / h, 0.0, 0.0,
            0.0, 0.0, -2.0 / d, 0.0,
            -(right + left) / w, -(top + bottom) / h, -(far + near) / d, 1.0,
        ]}
    }

    pub fn look_at(eye: &Vec3, target: &Vec3, up: &Vec3) -> Self {
        let z = eye.sub(target).normalize();
        let x = up.cross(&z).normalize();
        let y = z.cross(&x);
        Mat4 { m: [
            x.x, y.x, z.x, 0.0,
            x.y, y.y, z.y, 0.0,
            x.z, y.z, z.z, 0.0,
            -x.dot(eye), -y.dot(eye), -z.dot(eye), 1.0,
        ]}
    }

    pub fn multiply(&self, other: &Mat4) -> Mat4 {
        let mut r = [0.0f64; 16];
        for i in 0..4 {
            for j in 0..4 {
                r[i * 4 + j] = (0..4).map(|k| self.m[k * 4 + j] * other.m[i * 4 + k]).sum();
            }
        }
        Mat4 { m: r }
    }

    pub fn transform_point(&self, p: &Vec3) -> Vec3 {
        let w = self.m[3] * p.x + self.m[7] * p.y + self.m[11] * p.z + self.m[15];
        let inv_w = if w.abs() > 1e-12 { 1.0 / w } else { 1.0 };
        Vec3 {
            x: (self.m[0] * p.x + self.m[4] * p.y + self.m[8] * p.z + self.m[12]) * inv_w,
            y: (self.m[1] * p.x + self.m[5] * p.y + self.m[9] * p.z + self.m[13]) * inv_w,
            z: (self.m[2] * p.x + self.m[6] * p.y + self.m[10] * p.z + self.m[14]) * inv_w,
        }
    }

    pub fn compose(pos: &Vec3, rot: &Quat, scale: &Vec3) -> Mat4 {
        let r = Mat4::from_quat(rot);
        let t = Mat4::translation(pos.x, pos.y, pos.z);
        let s = Mat4::scaling(scale.x, scale.y, scale.z);
        t.multiply(&r).multiply(&s)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Color3
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct Color3 {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color3 {
    pub const WHITE: Color3 = Color3 { r: 1.0, g: 1.0, b: 1.0 };
    pub const BLACK: Color3 = Color3 { r: 0.0, g: 0.0, b: 0.0 };
    pub const RED: Color3 = Color3 { r: 1.0, g: 0.0, b: 0.0 };
    pub const GREEN: Color3 = Color3 { r: 0.0, g: 1.0, b: 0.0 };
    pub const BLUE: Color3 = Color3 { r: 0.0, g: 0.0, b: 1.0 };

    pub fn new(r: f64, g: f64, b: f64) -> Self { Color3 { r, g, b } }

    pub fn scale(&self, s: f64) -> Color3 {
        Color3 { r: self.r * s, g: self.g * s, b: self.b * s }
    }

    pub fn add(&self, other: &Color3) -> Color3 {
        Color3 { r: self.r + other.r, g: self.g + other.g, b: self.b + other.b }
    }

    pub fn multiply(&self, other: &Color3) -> Color3 {
        Color3 { r: self.r * other.r, g: self.g * other.g, b: self.b * other.b }
    }

    pub fn clamp(&self) -> Color3 {
        Color3 { r: self.r.clamp(0.0, 1.0), g: self.g.clamp(0.0, 1.0), b: self.b.clamp(0.0, 1.0) }
    }

    pub fn to_rgb8(&self) -> (u8, u8, u8) {
        let c = self.clamp();
        ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Object3D — base scene graph node
// ══════════════════════════════════════════════════════════════════════════════

pub type NodeId3D = u64;

/// Every object in the scene graph.
#[derive(Debug, Clone)]
pub struct Object3D {
    pub id: NodeId3D,
    pub name: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub visible: bool,
    pub parent: Option<NodeId3D>,
    pub children: Vec<NodeId3D>,
    pub kind: Object3DKind,
}

/// What kind of 3D object this is.
#[derive(Debug, Clone)]
pub enum Object3DKind {
    Empty,
    Mesh { geometry_id: u64, material_id: u64 },
    Camera(CameraProjection),
    Light(LightKind),
    Group,
    Bone { index: usize },
}

/// Camera projection types.
#[derive(Debug, Clone)]
pub enum CameraProjection {
    Perspective { fov: f64, aspect: f64, near: f64, far: f64 },
    Orthographic { left: f64, right: f64, top: f64, bottom: f64, near: f64, far: f64 },
}

/// Light types.
#[derive(Debug, Clone)]
pub enum LightKind {
    Ambient { color: Color3, intensity: f64 },
    Directional { color: Color3, intensity: f64, direction: Vec3 },
    Point { color: Color3, intensity: f64, range: f64 },
    Spot { color: Color3, intensity: f64, direction: Vec3, angle: f64, penumbra: f64 },
    Hemisphere { sky_color: Color3, ground_color: Color3, intensity: f64 },
}

impl Object3D {
    pub fn new(id: NodeId3D, name: &str) -> Self {
        Object3D {
            id, name: name.into(),
            position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE,
            visible: true, parent: None, children: Vec::new(),
            kind: Object3DKind::Empty,
        }
    }

    pub fn world_matrix(&self) -> Mat4 {
        Mat4::compose(&self.position, &self.rotation, &self.scale)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Fog / Background
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Fog {
    None,
    Linear { color: Color3, near: f64, far: f64 },
    Exponential { color: Color3, density: f64 },
}

#[derive(Debug, Clone)]
pub enum Background {
    Color(Color3),
    Skybox([String; 6]), // +X, -X, +Y, -Y, +Z, -Z face paths
    Gradient { top: Color3, bottom: Color3 },
}

// ══════════════════════════════════════════════════════════════════════════════
// Scene — top-level container
// ══════════════════════════════════════════════════════════════════════════════

/// The 3D scene — holds all objects, cameras, lights.
pub struct Scene {
    pub name: String,
    objects: HashMap<NodeId3D, Object3D>,
    next_id: NodeId3D,
    pub background: Background,
    pub fog: Fog,
    pub active_camera: Option<NodeId3D>,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Scene {
            name: name.into(),
            objects: HashMap::new(),
            next_id: 1,
            background: Background::Color(Color3::BLACK),
            fog: Fog::None,
            active_camera: None,
        }
    }

    fn alloc_id(&mut self) -> NodeId3D {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Add an empty object.
    pub fn add_empty(&mut self, name: &str) -> NodeId3D {
        let id = self.alloc_id();
        self.objects.insert(id, Object3D::new(id, name));
        id
    }

    /// Add a group (container).
    pub fn add_group(&mut self, name: &str) -> NodeId3D {
        let id = self.alloc_id();
        let mut obj = Object3D::new(id, name);
        obj.kind = Object3DKind::Group;
        self.objects.insert(id, obj);
        id
    }

    /// Add a mesh.
    pub fn add_mesh(&mut self, name: &str, geometry_id: u64, material_id: u64) -> NodeId3D {
        let id = self.alloc_id();
        let mut obj = Object3D::new(id, name);
        obj.kind = Object3DKind::Mesh { geometry_id, material_id };
        self.objects.insert(id, obj);
        id
    }

    /// Add a perspective camera.
    pub fn add_perspective_camera(&mut self, name: &str, fov: f64, aspect: f64, near: f64, far: f64) -> NodeId3D {
        let id = self.alloc_id();
        let mut obj = Object3D::new(id, name);
        obj.kind = Object3DKind::Camera(CameraProjection::Perspective { fov, aspect, near, far });
        self.objects.insert(id, obj);
        if self.active_camera.is_none() { self.active_camera = Some(id); }
        id
    }

    /// Add an orthographic camera.
    pub fn add_ortho_camera(&mut self, name: &str, left: f64, right: f64, top: f64, bottom: f64, near: f64, far: f64) -> NodeId3D {
        let id = self.alloc_id();
        let mut obj = Object3D::new(id, name);
        obj.kind = Object3DKind::Camera(CameraProjection::Orthographic { left, right, top, bottom, near, far });
        self.objects.insert(id, obj);
        id
    }

    /// Add a light.
    pub fn add_light(&mut self, name: &str, kind: LightKind) -> NodeId3D {
        let id = self.alloc_id();
        let mut obj = Object3D::new(id, name);
        obj.kind = Object3DKind::Light(kind);
        self.objects.insert(id, obj);
        id
    }

    /// Set parent-child relationship.
    pub fn set_parent(&mut self, child_id: NodeId3D, parent_id: NodeId3D) {
        // Remove from old parent
        if let Some(child) = self.objects.get(&child_id) {
            if let Some(old_parent_id) = child.parent {
                if let Some(old_parent) = self.objects.get_mut(&old_parent_id) {
                    old_parent.children.retain(|&c| c != child_id);
                }
            }
        }
        if let Some(parent) = self.objects.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
        if let Some(child) = self.objects.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
    }

    pub fn get(&self, id: NodeId3D) -> Option<&Object3D> { self.objects.get(&id) }
    pub fn get_mut(&mut self, id: NodeId3D) -> Option<&mut Object3D> { self.objects.get_mut(&id) }
    pub fn object_count(&self) -> usize { self.objects.len() }

    /// Get all lights in the scene.
    pub fn lights(&self) -> Vec<&Object3D> {
        self.objects.values().filter(|o| matches!(o.kind, Object3DKind::Light(_))).collect()
    }

    /// Get the active camera.
    pub fn camera(&self) -> Option<&Object3D> {
        self.active_camera.and_then(|id| self.objects.get(&id))
    }

    /// Get all meshes.
    pub fn meshes(&self) -> Vec<&Object3D> {
        self.objects.values().filter(|o| matches!(o.kind, Object3DKind::Mesh { .. })).collect()
    }

    /// Compute world matrix for an object (traversing parent chain).
    pub fn world_matrix(&self, id: NodeId3D) -> Mat4 {
        let mut chain = Vec::new();
        let mut current = id;
        while let Some(obj) = self.objects.get(&current) {
            chain.push(obj.world_matrix());
            match obj.parent {
                Some(pid) => current = pid,
                None => break,
            }
        }
        chain.iter().rev().fold(Mat4::IDENTITY, |acc, m| acc.multiply(m))
    }

    /// Get view matrix from the active camera.
    pub fn view_matrix(&self) -> Mat4 {
        if let Some(cam) = self.camera() {
            let pos = cam.position;
            let forward = cam.rotation.rotate_vec3(&Vec3::FORWARD);
            let target = pos.add(&forward);
            Mat4::look_at(&pos, &target, &Vec3::UP)
        } else {
            Mat4::IDENTITY
        }
    }

    /// Get projection matrix from the active camera.
    pub fn projection_matrix(&self) -> Mat4 {
        if let Some(cam) = self.camera() {
            match &cam.kind {
                Object3DKind::Camera(CameraProjection::Perspective { fov, aspect, near, far }) => {
                    Mat4::perspective(*fov, *aspect, *near, *far)
                }
                Object3DKind::Camera(CameraProjection::Orthographic { left, right, top, bottom, near, far }) => {
                    Mat4::orthographic(*left, *right, *bottom, *top, *near, *far)
                }
                _ => Mat4::IDENTITY,
            }
        } else {
            Mat4::IDENTITY
        }
    }

    /// Remove an object and detach children.
    pub fn remove(&mut self, id: NodeId3D) {
        if let Some(obj) = self.objects.remove(&id) {
            if let Some(pid) = obj.parent {
                if let Some(parent) = self.objects.get_mut(&pid) {
                    parent.children.retain(|&c| c != id);
                }
            }
            for child_id in &obj.children {
                if let Some(child) = self.objects.get_mut(child_id) {
                    child.parent = None;
                }
            }
        }
    }

    /// Traverse all objects depth-first.
    pub fn traverse<F: FnMut(&Object3D)>(&self, mut f: F) {
        for obj in self.objects.values() {
            f(obj);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Camera Controls
// ══════════════════════════════════════════════════════════════════════════════

/// Orbit camera controller (like Three.js OrbitControls).
pub struct OrbitControls {
    pub target: Vec3,
    pub distance: f64,
    pub azimuth: f64,   // horizontal angle (radians)
    pub elevation: f64, // vertical angle (radians)
    pub min_distance: f64,
    pub max_distance: f64,
    pub min_elevation: f64,
    pub max_elevation: f64,
}

impl OrbitControls {
    pub fn new(target: Vec3, distance: f64) -> Self {
        OrbitControls {
            target, distance,
            azimuth: 0.0, elevation: 0.4,
            min_distance: 0.1, max_distance: 1000.0,
            min_elevation: -std::f64::consts::FRAC_PI_2 + 0.01,
            max_elevation: std::f64::consts::FRAC_PI_2 - 0.01,
        }
    }

    pub fn rotate(&mut self, dx: f64, dy: f64) {
        self.azimuth += dx;
        self.elevation = (self.elevation + dy).clamp(self.min_elevation, self.max_elevation);
    }

    pub fn zoom(&mut self, delta: f64) {
        self.distance = (self.distance * (1.0 - delta)).clamp(self.min_distance, self.max_distance);
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        let right = Vec3::new(self.azimuth.cos(), 0.0, -self.azimuth.sin());
        let up = Vec3::UP;
        self.target = self.target.add(&right.scale(dx)).add(&up.scale(dy));
    }

    /// Compute camera position from orbit parameters.
    pub fn camera_position(&self) -> Vec3 {
        Vec3 {
            x: self.target.x + self.distance * self.elevation.cos() * self.azimuth.sin(),
            y: self.target.y + self.distance * self.elevation.sin(),
            z: self.target.z + self.distance * self.elevation.cos() * self.azimuth.cos(),
        }
    }

    /// Apply to a camera object.
    pub fn apply_to(&self, camera: &mut Object3D) {
        camera.position = self.camera_position();
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_operations() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert!((a.dot(&b) - 32.0).abs() < 1e-6);
        let c = a.cross(&b);
        assert!((c.x + 3.0).abs() < 1e-6);
        assert!((a.length() - (14.0_f64).sqrt()).abs() < 1e-6);
        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn quaternion_rotation() {
        let q = Quat::from_axis_angle(&Vec3::UP, std::f64::consts::FRAC_PI_2);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let r = q.rotate_vec3(&v);
        assert!((r.x).abs() < 0.01);
        assert!((r.z - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn mat4_identity() {
        let m = Mat4::IDENTITY;
        let p = Vec3::new(1.0, 2.0, 3.0);
        let r = m.transform_point(&p);
        assert!((r.x - 1.0).abs() < 1e-6);
        assert!((r.y - 2.0).abs() < 1e-6);
        assert!((r.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn mat4_translation() {
        let m = Mat4::translation(10.0, 20.0, 30.0);
        let p = Vec3::ZERO;
        let r = m.transform_point(&p);
        assert!((r.x - 10.0).abs() < 1e-6);
        assert!((r.y - 20.0).abs() < 1e-6);
    }

    #[test]
    fn scene_basic() {
        let mut scene = Scene::new("test");
        let cam = scene.add_perspective_camera("cam", 1.0, 1.5, 0.1, 100.0);
        let mesh = scene.add_mesh("cube", 0, 0);
        let light = scene.add_light("sun", LightKind::Directional {
            color: Color3::WHITE, intensity: 1.0, direction: Vec3::new(0.0, -1.0, 0.0),
        });
        assert_eq!(scene.object_count(), 3);
        assert!(scene.camera().is_some());
        assert_eq!(scene.lights().len(), 1);
        assert_eq!(scene.meshes().len(), 1);
        assert!(scene.get(cam).is_some());
        assert!(scene.get(mesh).is_some());
        assert!(scene.get(light).is_some());
    }

    #[test]
    fn scene_parent_child() {
        let mut scene = Scene::new("test");
        let parent = scene.add_group("parent");
        let child = scene.add_empty("child");
        scene.set_parent(child, parent);
        assert_eq!(scene.get(parent).unwrap().children.len(), 1);
        assert_eq!(scene.get(child).unwrap().parent, Some(parent));
    }

    #[test]
    fn scene_world_matrix() {
        let mut scene = Scene::new("test");
        let parent = scene.add_group("parent");
        scene.get_mut(parent).unwrap().position = Vec3::new(10.0, 0.0, 0.0);
        let child = scene.add_mesh("child", 0, 0);
        scene.get_mut(child).unwrap().position = Vec3::new(5.0, 0.0, 0.0);
        scene.set_parent(child, parent);
        let wm = scene.world_matrix(child);
        let world_pos = wm.transform_point(&Vec3::ZERO);
        assert!((world_pos.x - 15.0).abs() < 1e-6);
    }

    #[test]
    fn scene_remove() {
        let mut scene = Scene::new("test");
        let id = scene.add_empty("removeme");
        assert_eq!(scene.object_count(), 1);
        scene.remove(id);
        assert_eq!(scene.object_count(), 0);
    }

    #[test]
    fn orbit_controls() {
        let mut ctrl = OrbitControls::new(Vec3::ZERO, 10.0);
        let pos = ctrl.camera_position();
        assert!((pos.length() - 10.0).abs() < 0.1);
        ctrl.zoom(0.1);
        assert!(ctrl.distance < 10.0);
        ctrl.rotate(0.5, 0.1);
        assert!(ctrl.azimuth != 0.0);
    }

    #[test]
    fn color3_operations() {
        let c = Color3::RED.add(&Color3::GREEN);
        assert!((c.r - 1.0).abs() < 1e-6);
        assert!((c.g - 1.0).abs() < 1e-6);
        let (r, g, _b) = c.to_rgb8();
        assert_eq!(r, 255);
        assert_eq!(g, 255);
    }

    #[test]
    fn perspective_projection() {
        let p = Mat4::perspective(std::f64::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
        // Point at origin should project to 0,0
        let r = p.transform_point(&Vec3::ZERO);
        assert!(r.x.is_finite());
    }

    #[test]
    fn quaternion_slerp() {
        let a = Quat::IDENTITY;
        let b = Quat::from_axis_angle(&Vec3::UP, std::f64::consts::PI);
        let mid = a.slerp(&b, 0.5);
        // At t=0.5, should be ~90 degrees
        let v = mid.rotate_vec3(&Vec3::new(1.0, 0.0, 0.0));
        assert!((v.z).abs() > 0.9); // Should be rotated ~90 deg
    }

    #[test]
    fn vec3_lerp() {
        let a = Vec3::ZERO;
        let b = Vec3::new(10.0, 10.0, 10.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-6);
    }

    #[test]
    fn fog_and_background() {
        let mut scene = Scene::new("test");
        scene.fog = Fog::Linear { color: Color3::WHITE, near: 10.0, far: 100.0 };
        scene.background = Background::Gradient { top: Color3::BLUE, bottom: Color3::WHITE };
        if let Fog::Linear { near, far, .. } = &scene.fog {
            assert_eq!(*near, 10.0);
            assert_eq!(*far, 100.0);
        }
    }
}
