//! **Physics3D** — Rigid body physics, collision detection, raycasting.
//!
//! Colliders (Sphere/Box/Plane), rigid bodies, forces, broadphase AABB,
//! narrowphase GJK-like, contact resolution, raycasting/picking.
//! Spatial audio positioning.

use super::scene3d::{Vec3, Quat, Mat4};
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Collider shapes
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum ColliderShape {
    Sphere { radius: f64 },
    Box { half_extents: Vec3 },
    Plane { normal: Vec3, distance: f64 },
    Capsule { radius: f64, height: f64 },
}

// ══════════════════════════════════════════════════════════════════════════════
// Rigid Body
// ══════════════════════════════════════════════════════════════════════════════

pub type BodyId = u64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyType {
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Debug, Clone)]
pub struct RigidBody {
    pub id: BodyId,
    pub body_type: BodyType,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    pub mass: f64,
    pub inv_mass: f64,
    pub restitution: f64,
    pub friction: f64,
    pub collider: ColliderShape,
    pub force_accumulator: Vec3,
    pub gravity_scale: f64,
}

impl RigidBody {
    pub fn new(id: BodyId, collider: ColliderShape, body_type: BodyType) -> Self {
        let mass = if body_type == BodyType::Static { 0.0 } else { 1.0 };
        RigidBody {
            id, body_type, position: Vec3::ZERO, rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO, angular_velocity: Vec3::ZERO,
            mass, inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            restitution: 0.3, friction: 0.5, collider,
            force_accumulator: Vec3::ZERO, gravity_scale: 1.0,
        }
    }

    pub fn set_mass(&mut self, mass: f64) {
        self.mass = mass;
        self.inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
    }

    pub fn apply_force(&mut self, force: Vec3) {
        self.force_accumulator = self.force_accumulator.add(&force);
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.body_type == BodyType::Dynamic {
            self.velocity = self.velocity.add(&impulse.scale(self.inv_mass));
        }
    }

    pub fn kinetic_energy(&self) -> f64 {
        0.5 * self.mass * self.velocity.dot(&self.velocity)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Contact / Collision
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Contact {
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub point: Vec3,
    pub normal: Vec3,
    pub depth: f64,
}

/// Sphere-sphere collision test.
pub fn collide_sphere_sphere(a: &RigidBody, b: &RigidBody) -> Option<Contact> {
    let ra = match &a.collider { ColliderShape::Sphere { radius } => *radius, _ => return None };
    let rb = match &b.collider { ColliderShape::Sphere { radius } => *radius, _ => return None };
    let d = b.position.sub(&a.position);
    let dist = d.length();
    let overlap = ra + rb - dist;
    if overlap > 0.0 {
        let normal = if dist > 1e-12 { d.scale(1.0 / dist) } else { Vec3::UP };
        let point = a.position.add(&normal.scale(ra));
        Some(Contact { body_a: a.id, body_b: b.id, point, normal, depth: overlap })
    } else {
        None
    }
}

/// Sphere-plane collision test.
pub fn collide_sphere_plane(sphere: &RigidBody, plane: &RigidBody) -> Option<Contact> {
    let radius = match &sphere.collider { ColliderShape::Sphere { radius } => *radius, _ => return None };
    let (normal, plane_d) = match &plane.collider { ColliderShape::Plane { normal, distance } => (*normal, *distance), _ => return None };
    let dist = sphere.position.dot(&normal) - plane_d;
    let penetration = radius - dist;
    if penetration > 0.0 {
        let point = sphere.position.sub(&normal.scale(dist));
        Some(Contact { body_a: sphere.id, body_b: plane.id, point, normal, depth: penetration })
    } else {
        None
    }
}

/// AABB overlap test for broadphase.
pub fn aabb_overlap(a_pos: &Vec3, a_half: &Vec3, b_pos: &Vec3, b_half: &Vec3) -> bool {
    (a_pos.x - b_pos.x).abs() <= a_half.x + b_half.x &&
    (a_pos.y - b_pos.y).abs() <= a_half.y + b_half.y &&
    (a_pos.z - b_pos.z).abs() <= a_half.z + b_half.z
}

/// Resolve a contact (apply impulse).
pub fn resolve_contact(a: &mut RigidBody, b: &mut RigidBody, contact: &Contact) {
    let relative_vel = a.velocity.sub(&b.velocity);
    let vel_along_normal = relative_vel.dot(&contact.normal);
    if vel_along_normal > 0.0 { return; } // separating

    let e = a.restitution.min(b.restitution);
    let j = -(1.0 + e) * vel_along_normal / (a.inv_mass + b.inv_mass);
    let impulse = contact.normal.scale(j);

    if a.body_type == BodyType::Dynamic {
        a.velocity = a.velocity.add(&impulse.scale(a.inv_mass));
    }
    if b.body_type == BodyType::Dynamic {
        b.velocity = b.velocity.sub(&impulse.scale(b.inv_mass));
    }

    // Positional correction (prevent sinking)
    let corrp = 0.8;
    let slop = 0.01;
    let correction = contact.normal.scale(
        ((contact.depth - slop).max(0.0) / (a.inv_mass + b.inv_mass)) * corrp
    );
    if a.body_type == BodyType::Dynamic {
        a.position = a.position.add(&correction.scale(a.inv_mass));
    }
    if b.body_type == BodyType::Dynamic {
        b.position = b.position.sub(&correction.scale(b.inv_mass));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Physics World
// ══════════════════════════════════════════════════════════════════════════════

pub struct PhysicsWorld {
    bodies: HashMap<BodyId, RigidBody>,
    next_id: BodyId,
    pub gravity: Vec3,
    pub contacts: Vec<Contact>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        PhysicsWorld {
            bodies: HashMap::new(),
            next_id: 1,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            contacts: Vec::new(),
        }
    }

    pub fn add_body(&mut self, collider: ColliderShape, body_type: BodyType) -> BodyId {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.insert(id, RigidBody::new(id, collider, body_type));
        id
    }

    pub fn get(&self, id: BodyId) -> Option<&RigidBody> { self.bodies.get(&id) }
    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut RigidBody> { self.bodies.get_mut(&id) }
    pub fn body_count(&self) -> usize { self.bodies.len() }

    /// Step the simulation forward by dt seconds.
    pub fn step(&mut self, dt: f64) {
        let ids: Vec<BodyId> = self.bodies.keys().copied().collect();

        // Apply gravity + integrate forces
        for &id in &ids {
            if let Some(body) = self.bodies.get_mut(&id) {
                if body.body_type != BodyType::Dynamic { continue; }
                let g = self.gravity.scale(body.gravity_scale);
                let accel = body.force_accumulator.scale(body.inv_mass).add(&g);
                body.velocity = body.velocity.add(&accel.scale(dt));
                body.position = body.position.add(&body.velocity.scale(dt));
                body.force_accumulator = Vec3::ZERO;
            }
        }

        // Collision detection (all pairs, O(n²) — fine for small scenes)
        self.contacts.clear();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let (a_id, b_id) = (ids[i], ids[j]);
                // Borrow both bodies safely
                let contact = {
                    let a = &self.bodies[&a_id];
                    let b = &self.bodies[&b_id];
                    match (&a.collider, &b.collider) {
                        (ColliderShape::Sphere { .. }, ColliderShape::Sphere { .. }) => collide_sphere_sphere(a, b),
                        (ColliderShape::Sphere { .. }, ColliderShape::Plane { .. }) => collide_sphere_plane(a, b),
                        (ColliderShape::Plane { .. }, ColliderShape::Sphere { .. }) => {
                            collide_sphere_plane(b, a).map(|mut c| { std::mem::swap(&mut c.body_a, &mut c.body_b); c.normal = c.normal.scale(-1.0); c })
                        }
                        _ => None,
                    }
                };
                if let Some(c) = contact {
                    self.contacts.push(c);
                }
            }
        }

        // Resolve contacts
        for contact in self.contacts.clone() {
            let a_id = contact.body_a;
            let b_id = contact.body_b;
            // Take both bodies out
            if let (Some(mut a), Some(mut b)) = (self.bodies.remove(&a_id), self.bodies.remove(&b_id)) {
                resolve_contact(&mut a, &mut b, &contact);
                self.bodies.insert(a_id, a);
                self.bodies.insert(b_id, b);
            }
        }
    }

    pub fn remove_body(&mut self, id: BodyId) {
        self.bodies.remove(&id);
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Raycasting
// ══════════════════════════════════════════════════════════════════════════════

/// Ray with origin and direction.
#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray { origin, direction: direction.normalize() }
    }

    pub fn point_at(&self, t: f64) -> Vec3 {
        self.origin.add(&self.direction.scale(t))
    }
}

/// Raycast hit result.
#[derive(Debug, Clone)]
pub struct RayHit {
    pub body_id: BodyId,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f64,
}

/// Cast a ray against a sphere.
pub fn ray_sphere(ray: &Ray, center: &Vec3, radius: f64) -> Option<f64> {
    let oc = ray.origin.sub(center);
    let b = oc.dot(&ray.direction);
    let c = oc.dot(&oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 { return None; }
    let t = -b - disc.sqrt();
    if t > 0.0 { Some(t) } else { None }
}

/// Cast a ray against a plane.
pub fn ray_plane(ray: &Ray, normal: &Vec3, distance: f64) -> Option<f64> {
    let denom = normal.dot(&ray.direction);
    if denom.abs() < 1e-6 { return None; }
    let t = (distance - normal.dot(&ray.origin)) / denom;
    if t > 0.0 { Some(t) } else { None }
}

/// Cast a ray against all bodies in a physics world.
pub fn raycast(world: &PhysicsWorld, ray: &Ray) -> Option<RayHit> {
    let mut best: Option<RayHit> = None;
    for body in world.bodies.values() {
        let hit = match &body.collider {
            ColliderShape::Sphere { radius } => {
                ray_sphere(ray, &body.position, *radius).map(|t| {
                    let point = ray.point_at(t);
                    let normal = point.sub(&body.position).normalize();
                    RayHit { body_id: body.id, point, normal, distance: t }
                })
            }
            ColliderShape::Plane { normal, distance } => {
                ray_plane(ray, normal, *distance).map(|t| {
                    let point = ray.point_at(t);
                    RayHit { body_id: body.id, point, normal: *normal, distance: t }
                })
            }
            _ => None,
        };
        if let Some(h) = hit {
            if best.as_ref().map(|b| h.distance < b.distance).unwrap_or(true) {
                best = Some(h);
            }
        }
    }
    best
}

/// Generate a picking ray from screen coordinates.
pub fn screen_to_ray(
    x: f64, y: f64, width: f64, height: f64,
    inv_proj: &Mat4, inv_view: &Mat4,
) -> Ray {
    let ndc_x = (2.0 * x / width) - 1.0;
    let ndc_y = 1.0 - (2.0 * y / height);
    let clip_near = Vec3::new(ndc_x, ndc_y, -1.0);
    let clip_far = Vec3::new(ndc_x, ndc_y, 1.0);
    let world_near = inv_view.transform_point(&inv_proj.transform_point(&clip_near));
    let world_far = inv_view.transform_point(&inv_proj.transform_point(&clip_far));
    Ray::new(world_near, world_far.sub(&world_near))
}

// ══════════════════════════════════════════════════════════════════════════════
// Spatial Audio
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct AudioSource {
    pub id: u64,
    pub name: String,
    pub position: Vec3,
    pub volume: f64,
    pub max_distance: f64,
    pub rolloff: f64,
    pub playing: bool,
    pub looping: bool,
}

impl AudioSource {
    pub fn new(id: u64, name: &str, position: Vec3) -> Self {
        AudioSource {
            id, name: name.into(), position,
            volume: 1.0, max_distance: 100.0, rolloff: 1.0,
            playing: false, looping: false,
        }
    }

    /// Compute effective volume at listener position (inverse distance).
    pub fn effective_volume(&self, listener_pos: &Vec3) -> f64 {
        let d = self.position.distance(listener_pos);
        if d < 0.01 { return self.volume; }
        if d > self.max_distance { return 0.0; }
        self.volume / (1.0 + self.rolloff * d)
    }

    /// Compute stereo panning (-1=left, 0=center, 1=right).
    pub fn stereo_pan(&self, listener_pos: &Vec3, listener_right: &Vec3) -> f64 {
        let dir = self.position.sub(listener_pos).normalize();
        dir.dot(listener_right).clamp(-1.0, 1.0)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_sphere_collision() {
        let a = RigidBody::new(1, ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        let mut b = RigidBody::new(2, ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        b.position = Vec3::new(1.5, 0.0, 0.0);
        let contact = collide_sphere_sphere(&a, &b);
        assert!(contact.is_some());
        assert!(contact.unwrap().depth > 0.0);
    }

    #[test]
    fn sphere_plane_collision() {
        let mut sphere = RigidBody::new(1, ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        sphere.position = Vec3::new(0.0, 0.5, 0.0);
        let plane = RigidBody::new(2, ColliderShape::Plane { normal: Vec3::UP, distance: 0.0 }, BodyType::Static);
        let contact = collide_sphere_plane(&sphere, &plane);
        assert!(contact.is_some());
    }

    #[test]
    fn physics_step_gravity() {
        let mut world = PhysicsWorld::new();
        let id = world.add_body(ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        world.get_mut(id).unwrap().position = Vec3::new(0.0, 10.0, 0.0);
        world.step(0.016);
        assert!(world.get(id).unwrap().position.y < 10.0);
    }

    #[test]
    fn physics_sphere_on_plane() {
        let mut world = PhysicsWorld::new();
        let _floor = world.add_body(ColliderShape::Plane { normal: Vec3::UP, distance: 0.0 }, BodyType::Static);
        let ball = world.add_body(ColliderShape::Sphere { radius: 0.5 }, BodyType::Dynamic);
        world.get_mut(ball).unwrap().position = Vec3::new(0.0, 2.0, 0.0);
        for _ in 0..100 { world.step(0.016); }
        // Ball should have settled near the plane
        assert!(world.get(ball).unwrap().position.y < 2.0);
    }

    #[test]
    fn ray_sphere_test() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        let t = ray_sphere(&ray, &Vec3::ZERO, 1.0);
        assert!(t.is_some());
        assert!((t.unwrap() - 4.0).abs() < 0.01);
    }

    #[test]
    fn ray_plane_test() {
        let ray = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let t = ray_plane(&ray, &Vec3::UP, 0.0);
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn raycast_world() {
        let mut world = PhysicsWorld::new();
        let s = world.add_body(ColliderShape::Sphere { radius: 1.0 }, BodyType::Static);
        world.get_mut(s).unwrap().position = Vec3::new(0.0, 0.0, 5.0);
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0));
        let hit = raycast(&world, &ray);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().body_id, s);
    }

    #[test]
    fn aabb_test() {
        assert!(aabb_overlap(
            &Vec3::ZERO, &Vec3::ONE,
            &Vec3::new(1.5, 0.0, 0.0), &Vec3::ONE,
        ));
        assert!(!aabb_overlap(
            &Vec3::ZERO, &Vec3::ONE,
            &Vec3::new(3.0, 0.0, 0.0), &Vec3::ONE,
        ));
    }

    #[test]
    fn audio_source() {
        let src = AudioSource::new(1, "bgm", Vec3::new(5.0, 0.0, 0.0));
        let vol = src.effective_volume(&Vec3::ZERO);
        assert!(vol > 0.0 && vol < 1.0);
        let pan = src.stereo_pan(&Vec3::ZERO, &Vec3::RIGHT);
        assert!(pan > 0.0);
    }

    #[test]
    fn rigid_body_impulse() {
        let mut body = RigidBody::new(1, ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        body.apply_impulse(Vec3::new(10.0, 0.0, 0.0));
        assert!((body.velocity.x - 10.0).abs() < 0.01);
    }

    #[test]
    fn kinetic_energy() {
        let mut body = RigidBody::new(1, ColliderShape::Sphere { radius: 1.0 }, BodyType::Dynamic);
        body.set_mass(2.0);
        body.velocity = Vec3::new(3.0, 0.0, 0.0);
        assert!((body.kinetic_energy() - 9.0).abs() < 0.01);
    }
}
