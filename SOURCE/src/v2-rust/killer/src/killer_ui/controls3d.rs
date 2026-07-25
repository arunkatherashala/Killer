//! **Controls 3D** — Camera controllers: Fly, FirstPerson, Trackball, MapControl.
//!
//! Replaces/supplements the OrbitControls in scene3d.rs.

use super::scene3d::{Vec3, Quat, Mat4};

// ══════════════════════════════════════════════════════════════════════════════
// Fly Controls (6 DOF)
// ══════════════════════════════════════════════════════════════════════════════

/// Free-flight camera (6 degrees of freedom).
pub struct FlyControls {
    pub position: Vec3,
    pub rotation: Quat,
    pub move_speed: f64,
    pub roll_speed: f64,
    pub drag_to_look: bool,
}

impl FlyControls {
    pub fn new() -> Self {
        FlyControls {
            position: Vec3::new(0.0, 2.0, 10.0),
            rotation: Quat::IDENTITY,
            move_speed: 5.0,
            roll_speed: 1.0,
            drag_to_look: false,
        }
    }

    pub fn forward(&self) -> Vec3 { self.rotation.rotate_vec3(&Vec3::new(0.0, 0.0, -1.0)) }
    pub fn right(&self) -> Vec3 { self.rotation.rotate_vec3(&Vec3::new(1.0, 0.0, 0.0)) }
    pub fn up(&self) -> Vec3 { self.rotation.rotate_vec3(&Vec3::UP) }

    pub fn move_forward(&mut self, dt: f64) {
        let fwd = self.forward().scale(self.move_speed * dt);
        self.position = self.position.add(&fwd);
    }

    pub fn move_backward(&mut self, dt: f64) {
        let back = self.forward().scale(-self.move_speed * dt);
        self.position = self.position.add(&back);
    }

    pub fn move_left(&mut self, dt: f64) {
        let left = self.right().scale(-self.move_speed * dt);
        self.position = self.position.add(&left);
    }

    pub fn move_right(&mut self, dt: f64) {
        let r = self.right().scale(self.move_speed * dt);
        self.position = self.position.add(&r);
    }

    pub fn move_up(&mut self, dt: f64) {
        let u = self.up().scale(self.move_speed * dt);
        self.position = self.position.add(&u);
    }

    pub fn move_down(&mut self, dt: f64) {
        let d = self.up().scale(-self.move_speed * dt);
        self.position = self.position.add(&d);
    }

    pub fn rotate_pitch(&mut self, angle: f64) {
        let pitch = Quat::from_axis_angle(&self.right(), angle);
        self.rotation = pitch.multiply(&self.rotation);
    }

    pub fn rotate_yaw(&mut self, angle: f64) {
        let yaw = Quat::from_axis_angle(&Vec3::UP, angle);
        self.rotation = yaw.multiply(&self.rotation);
    }

    pub fn rotate_roll(&mut self, angle: f64) {
        let roll = Quat::from_axis_angle(&self.forward(), angle * self.roll_speed);
        self.rotation = roll.multiply(&self.rotation);
    }

    pub fn view_matrix(&self) -> Mat4 {
        let target = self.position.add(&self.forward());
        Mat4::look_at(&self.position, &target, &self.up())
    }
}

impl Default for FlyControls {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// First-Person Controls (FPS-style, locked to ground plane)
// ══════════════════════════════════════════════════════════════════════════════

/// FPS camera — locked Y axis, mouse look, WASD movement.
pub struct FirstPersonControls {
    pub position: Vec3,
    pub yaw: f64,       // horizontal rotation (radians)
    pub pitch: f64,     // vertical rotation (radians), clamped ±89°
    pub move_speed: f64,
    pub look_speed: f64,
    pub height: f64,
    pub gravity: bool,
    pub velocity_y: f64,
}

impl FirstPersonControls {
    pub fn new() -> Self {
        FirstPersonControls {
            position: Vec3::new(0.0, 1.7, 0.0),
            yaw: 0.0, pitch: 0.0,
            move_speed: 5.0, look_speed: 0.002,
            height: 1.7, gravity: true, velocity_y: 0.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos()).normalize()
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    pub fn look_direction(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        ).normalize()
    }

    pub fn mouse_move(&mut self, dx: f64, dy: f64) {
        self.yaw += dx * self.look_speed;
        self.pitch -= dy * self.look_speed;
        let limit = 89.0f64.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    pub fn move_forward(&mut self, dt: f64) {
        let fwd = self.forward().scale(self.move_speed * dt);
        self.position = self.position.add(&fwd);
    }

    pub fn move_backward(&mut self, dt: f64) {
        let back = self.forward().scale(-self.move_speed * dt);
        self.position = self.position.add(&back);
    }

    pub fn strafe_left(&mut self, dt: f64) {
        let left = self.right().scale(-self.move_speed * dt);
        self.position = self.position.add(&left);
    }

    pub fn strafe_right(&mut self, dt: f64) {
        let r = self.right().scale(self.move_speed * dt);
        self.position = self.position.add(&r);
    }

    pub fn jump(&mut self, strength: f64) {
        if self.gravity && (self.position.y - self.height).abs() < 0.1 {
            self.velocity_y = strength;
        }
    }

    /// Tick physics (gravity).
    pub fn update(&mut self, dt: f64) {
        if self.gravity {
            self.velocity_y -= 9.81 * dt;
            self.position.y += self.velocity_y * dt;
            if self.position.y < self.height {
                self.position.y = self.height;
                self.velocity_y = 0.0;
            }
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let target = self.position.add(&self.look_direction());
        Mat4::look_at(&self.position, &target, &Vec3::UP)
    }
}

impl Default for FirstPersonControls {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Trackball Controls
// ══════════════════════════════════════════════════════════════════════════════

/// Trackball camera — rotate freely around target in all axes.
pub struct TrackballControls {
    pub target: Vec3,
    pub rotation: Quat,
    pub distance: f64,
    pub rotate_speed: f64,
    pub zoom_speed: f64,
    pub pan_speed: f64,
    pub min_distance: f64,
    pub max_distance: f64,
}

impl TrackballControls {
    pub fn new() -> Self {
        TrackballControls {
            target: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            distance: 10.0,
            rotate_speed: 1.0,
            zoom_speed: 1.0,
            pan_speed: 1.0,
            min_distance: 0.1,
            max_distance: 1000.0,
        }
    }

    pub fn rotate(&mut self, dx: f64, dy: f64) {
        let yaw = Quat::from_axis_angle(&Vec3::UP, -dx * self.rotate_speed);
        let right = self.rotation.rotate_vec3(&Vec3::new(1.0, 0.0, 0.0));
        let pitch = Quat::from_axis_angle(&right, -dy * self.rotate_speed);
        self.rotation = yaw.multiply(&pitch).multiply(&self.rotation);
    }

    pub fn zoom(&mut self, delta: f64) {
        self.distance *= 1.0 - delta * self.zoom_speed * 0.1;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        let right = self.rotation.rotate_vec3(&Vec3::new(1.0, 0.0, 0.0));
        let up = self.rotation.rotate_vec3(&Vec3::UP);
        let offset = right.scale(-dx * self.pan_speed).add(&up.scale(dy * self.pan_speed));
        self.target = self.target.add(&offset);
    }

    pub fn camera_position(&self) -> Vec3 {
        let back = self.rotation.rotate_vec3(&Vec3::new(0.0, 0.0, 1.0));
        self.target.add(&back.scale(self.distance))
    }

    pub fn view_matrix(&self) -> Mat4 {
        let pos = self.camera_position();
        let up = self.rotation.rotate_vec3(&Vec3::UP);
        Mat4::look_at(&pos, &self.target, &up)
    }
}

impl Default for TrackballControls {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Map / Top-Down Controls
// ══════════════════════════════════════════════════════════════════════════════

/// Top-down map camera (strategy game / map viewer style).
pub struct MapControls {
    pub center: Vec3,
    pub height: f64,
    pub rotation: f64,  // yaw rotation in radians
    pub tilt: f64,      // tilt angle (0 = top-down, PI/4 = 45°)
    pub zoom_speed: f64,
    pub pan_speed: f64,
    pub min_height: f64,
    pub max_height: f64,
}

impl MapControls {
    pub fn new() -> Self {
        MapControls {
            center: Vec3::ZERO,
            height: 20.0,
            rotation: 0.0,
            tilt: 0.8,
            zoom_speed: 1.0,
            pan_speed: 1.0,
            min_height: 1.0,
            max_height: 100.0,
        }
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        self.center.x += (dx * cos_r - dy * sin_r) * self.pan_speed;
        self.center.z += (dx * sin_r + dy * cos_r) * self.pan_speed;
    }

    pub fn zoom(&mut self, delta: f64) {
        self.height *= 1.0 - delta * self.zoom_speed * 0.1;
        self.height = self.height.clamp(self.min_height, self.max_height);
    }

    pub fn rotate(&mut self, angle: f64) { self.rotation += angle; }

    pub fn camera_position(&self) -> Vec3 {
        Vec3::new(
            self.center.x - self.tilt.sin() * self.rotation.sin() * self.height,
            self.center.y + self.tilt.cos() * self.height,
            self.center.z - self.tilt.sin() * self.rotation.cos() * self.height,
        )
    }

    pub fn view_matrix(&self) -> Mat4 {
        let pos = self.camera_position();
        Mat4::look_at(&pos, &self.center, &Vec3::UP)
    }
}

impl Default for MapControls {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fly_forward() {
        let mut fly = FlyControls::new();
        let start = fly.position;
        fly.move_forward(1.0);
        assert!(fly.position.distance(&start) > 0.1);
    }

    #[test]
    fn fly_rotation() {
        let mut fly = FlyControls::new();
        fly.rotate_yaw(std::f64::consts::PI / 4.0);
        let fwd = fly.forward();
        assert!((fwd.x.abs() - fwd.z.abs()).abs() < 0.1);
    }

    #[test]
    fn fly_view_matrix() {
        let fly = FlyControls::new();
        let _mat = fly.view_matrix();
        // Should produce valid view matrix
    }

    #[test]
    fn fps_look() {
        let mut fps = FirstPersonControls::new();
        fps.mouse_move(100.0, 50.0);
        assert!(fps.yaw.abs() > 0.0);
        assert!(fps.pitch.abs() > 0.0);
    }

    #[test]
    fn fps_pitch_clamp() {
        let mut fps = FirstPersonControls::new();
        fps.mouse_move(0.0, -100000.0);
        assert!(fps.pitch <= 90.0f64.to_radians());
    }

    #[test]
    fn fps_movement() {
        let mut fps = FirstPersonControls::new();
        let start = fps.position;
        fps.move_forward(1.0);
        assert!(fps.position.distance(&start) > 0.1);
    }

    #[test]
    fn fps_gravity() {
        let mut fps = FirstPersonControls::new();
        fps.position.y = 10.0;
        for _ in 0..100 { fps.update(0.016); }
        // Should fall back to height
        assert!((fps.position.y - fps.height).abs() < 0.5);
    }

    #[test]
    fn fps_jump() {
        let mut fps = FirstPersonControls::new();
        fps.jump(5.0);
        assert!(fps.velocity_y > 0.0);
    }

    #[test]
    fn trackball_zoom() {
        let mut tb = TrackballControls::new();
        let d1 = tb.distance;
        tb.zoom(1.0);
        assert!(tb.distance < d1);
    }

    #[test]
    fn trackball_rotate() {
        let mut tb = TrackballControls::new();
        let pos1 = tb.camera_position();
        tb.rotate(0.5, 0.0);
        let pos2 = tb.camera_position();
        assert!(pos1.distance(&pos2) > 0.01);
    }

    #[test]
    fn map_controls() {
        let mut mc = MapControls::new();
        mc.pan(5.0, 0.0);
        assert!(mc.center.x.abs() > 0.1);
        mc.zoom(2.0);
        assert!(mc.height < 20.0);
    }

    #[test]
    fn map_view_matrix() {
        let mc = MapControls::new();
        let _mat = mc.view_matrix();
    }
}
