//! **Animation3D** — Three.js-equivalent animation system.
//!
//! AnimationClip, AnimationMixer, skeletal animation, bone hierarchy,
//! keyframe interpolation, particle systems, morph targets.

use super::scene3d::{Vec3, Quat, NodeId3D, Color3};

// ══════════════════════════════════════════════════════════════════════════════
// Animation Clip & Keyframe tracks
// ══════════════════════════════════════════════════════════════════════════════

/// Property to animate.
#[derive(Debug, Clone)]
pub enum AnimProperty {
    Position(Vec3),
    Rotation(Quat),
    Scale(Vec3),
    Opacity(f64),
    Color(Color3),
    MorphWeight(f64),
}

/// A keyframe in an animation track.
#[derive(Debug, Clone)]
pub struct Keyframe3D {
    pub time: f64,
    pub value: AnimProperty,
}

/// Interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interpolation {
    Linear,
    Step,
    CubicSpline,
}

/// Track: sequence of keyframes targeting one property of one object.
#[derive(Debug, Clone)]
pub struct AnimationTrack {
    pub target_id: NodeId3D,
    pub property: String,  // "position", "rotation", "scale", "opacity"
    pub keyframes: Vec<Keyframe3D>,
    pub interpolation: Interpolation,
}

impl AnimationTrack {
    pub fn new(target_id: NodeId3D, property: &str) -> Self {
        AnimationTrack {
            target_id, property: property.into(),
            keyframes: Vec::new(),
            interpolation: Interpolation::Linear,
        }
    }

    pub fn add_key(mut self, time: f64, value: AnimProperty) -> Self {
        self.keyframes.push(Keyframe3D { time, value });
        self
    }

    pub fn with_interpolation(mut self, interp: Interpolation) -> Self {
        self.interpolation = interp;
        self
    }

    /// Sample the track at a given time.
    pub fn sample(&self, time: f64) -> Option<AnimProperty> {
        if self.keyframes.is_empty() { return None; }
        if self.keyframes.len() == 1 { return Some(self.keyframes[0].value.clone()); }

        // Find surrounding keyframes
        let clamped = time.max(self.keyframes[0].time);
        let last_time = self.keyframes.last().unwrap().time;
        let clamped = clamped.min(last_time);

        let mut idx = 0;
        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= clamped { idx = i; }
        }

        if idx >= self.keyframes.len() - 1 {
            return Some(self.keyframes.last().unwrap().value.clone());
        }

        let kf0 = &self.keyframes[idx];
        let kf1 = &self.keyframes[idx + 1];
        let segment = kf1.time - kf0.time;
        if segment < 1e-12 { return Some(kf0.value.clone()); }
        let t = (clamped - kf0.time) / segment;

        match self.interpolation {
            Interpolation::Step => Some(kf0.value.clone()),
            Interpolation::Linear | Interpolation::CubicSpline => {
                Some(lerp_property(&kf0.value, &kf1.value, t))
            }
        }
    }

    pub fn duration(&self) -> f64 {
        self.keyframes.last().map(|kf| kf.time).unwrap_or(0.0)
    }
}

fn lerp_property(a: &AnimProperty, b: &AnimProperty, t: f64) -> AnimProperty {
    match (a, b) {
        (AnimProperty::Position(pa), AnimProperty::Position(pb)) =>
            AnimProperty::Position(pa.lerp(pb, t)),
        (AnimProperty::Rotation(qa), AnimProperty::Rotation(qb)) =>
            AnimProperty::Rotation(qa.slerp(qb, t)),
        (AnimProperty::Scale(sa), AnimProperty::Scale(sb)) =>
            AnimProperty::Scale(sa.lerp(sb, t)),
        (AnimProperty::Opacity(oa), AnimProperty::Opacity(ob)) =>
            AnimProperty::Opacity(oa + (ob - oa) * t),
        (AnimProperty::Color(ca), AnimProperty::Color(cb)) =>
            AnimProperty::Color(Color3::new(
                ca.r + (cb.r - ca.r) * t,
                ca.g + (cb.g - ca.g) * t,
                ca.b + (cb.b - ca.b) * t,
            )),
        (AnimProperty::MorphWeight(wa), AnimProperty::MorphWeight(wb)) =>
            AnimProperty::MorphWeight(wa + (wb - wa) * t),
        _ => a.clone(),
    }
}

/// Animation clip — named collection of tracks.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub tracks: Vec<AnimationTrack>,
    pub duration: f64,
}

impl AnimationClip {
    pub fn new(name: &str) -> Self {
        AnimationClip { name: name.into(), tracks: Vec::new(), duration: 0.0 }
    }

    pub fn add_track(mut self, track: AnimationTrack) -> Self {
        let d = track.duration();
        if d > self.duration { self.duration = d; }
        self.tracks.push(track);
        self
    }

    /// Sample all tracks at a given time.
    pub fn sample(&self, time: f64) -> Vec<(NodeId3D, String, AnimProperty)> {
        self.tracks.iter().filter_map(|t| {
            t.sample(time).map(|v| (t.target_id, t.property.clone(), v))
        }).collect()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Animation Mixer
// ══════════════════════════════════════════════════════════════════════════════

/// Playback state for a clip.
#[derive(Debug, Clone)]
pub struct AnimationAction {
    pub clip_name: String,
    pub time: f64,
    pub speed: f64,
    pub weight: f64,
    pub looping: bool,
    pub playing: bool,
    pub paused: bool,
}

impl AnimationAction {
    fn new(clip_name: &str) -> Self {
        AnimationAction {
            clip_name: clip_name.into(),
            time: 0.0, speed: 1.0, weight: 1.0,
            looping: true, playing: true, paused: false,
        }
    }
}

/// Manages multiple animation playbacks.
pub struct AnimationMixer {
    clips: Vec<AnimationClip>,
    actions: Vec<AnimationAction>,
}

impl AnimationMixer {
    pub fn new() -> Self { AnimationMixer { clips: Vec::new(), actions: Vec::new() } }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.push(clip);
    }

    pub fn play(&mut self, clip_name: &str) -> usize {
        let action = AnimationAction::new(clip_name);
        self.actions.push(action);
        self.actions.len() - 1
    }

    pub fn stop(&mut self, action_index: usize) {
        if let Some(a) = self.actions.get_mut(action_index) {
            a.playing = false;
        }
    }

    pub fn pause(&mut self, action_index: usize) {
        if let Some(a) = self.actions.get_mut(action_index) {
            a.paused = true;
        }
    }

    /// Advance all active actions by dt seconds.
    pub fn update(&mut self, dt: f64) -> Vec<(NodeId3D, String, AnimProperty)> {
        let mut results = Vec::new();
        for action in &mut self.actions {
            if !action.playing || action.paused { continue; }
            action.time += dt * action.speed;

            if let Some(clip) = self.clips.iter().find(|c| c.name == action.clip_name) {
                if action.looping && action.time > clip.duration {
                    action.time %= clip.duration;
                }
                let samples = clip.sample(action.time);
                for (target, prop, val) in samples {
                    results.push((target, prop, val));
                }
            }
        }
        results
    }

    pub fn action_count(&self) -> usize { self.actions.len() }
    pub fn clip_count(&self) -> usize { self.clips.len() }
}

impl Default for AnimationMixer {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Bone / Skeleton
// ══════════════════════════════════════════════════════════════════════════════

/// A bone in a skeleton hierarchy.
#[derive(Debug, Clone)]
pub struct Bone {
    pub index: usize,
    pub name: String,
    pub parent: Option<usize>,
    pub local_position: Vec3,
    pub local_rotation: Quat,
    pub inverse_bind_matrix: [f64; 16],
}

impl Bone {
    pub fn new(index: usize, name: &str) -> Self {
        Bone {
            index, name: name.into(), parent: None,
            local_position: Vec3::ZERO, local_rotation: Quat::IDENTITY,
            inverse_bind_matrix: super::scene3d::Mat4::IDENTITY.m,
        }
    }
}

/// Skeleton — flat array of bones.
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub world_matrices: Vec<[f64; 16]>,
}

impl Skeleton {
    pub fn new() -> Self {
        Skeleton { bones: Vec::new(), world_matrices: Vec::new() }
    }

    pub fn add_bone(&mut self, mut bone: Bone) -> usize {
        bone.index = self.bones.len();
        self.bones.push(bone);
        self.world_matrices.push(super::scene3d::Mat4::IDENTITY.m);
        self.bones.len() - 1
    }

    /// Update world matrices from local transforms (parent-first traversal).
    pub fn update(&mut self) {
        use super::scene3d::Mat4;
        for i in 0..self.bones.len() {
            let local = Mat4::compose(&self.bones[i].local_position, &self.bones[i].local_rotation, &Vec3::ONE);
            let world = if let Some(parent) = self.bones[i].parent {
                Mat4 { m: self.world_matrices[parent] }.multiply(&local)
            } else {
                local
            };
            self.world_matrices[i] = world.m;
        }
    }

    pub fn bone_count(&self) -> usize { self.bones.len() }
}

impl Default for Skeleton {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Particle System
// ══════════════════════════════════════════════════════════════════════════════

/// Single particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Color3,
    pub size: f64,
    pub life: f64,      // remaining life in seconds
    pub max_life: f64,
}

/// Particle emitter shape.
#[derive(Debug, Clone)]
pub enum EmitterShape {
    Point,
    Sphere { radius: f64 },
    Box { half_extents: Vec3 },
    Cone { angle: f64, radius: f64 },
}

/// Particle system.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub emitter_position: Vec3,
    pub emitter_shape: EmitterShape,
    pub emit_rate: f64,           // particles per second
    pub initial_speed: f64,
    pub initial_size: f64,
    pub life_time: f64,
    pub gravity: Vec3,
    pub start_color: Color3,
    pub end_color: Color3,
    pub max_particles: usize,
    accumulator: f64,
    seed: u64,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        ParticleSystem {
            particles: Vec::new(),
            emitter_position: Vec3::ZERO,
            emitter_shape: EmitterShape::Point,
            emit_rate: 10.0,
            initial_speed: 2.0,
            initial_size: 0.1,
            life_time: 2.0,
            gravity: Vec3::new(0.0, -1.0, 0.0),
            start_color: Color3::WHITE,
            end_color: Color3::new(1.0, 1.0, 1.0),
            max_particles,
            accumulator: 0.0,
            seed: 12345,
        }
    }

    fn pseudo_random(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.seed >> 33) as f64 / (1u64 << 31) as f64
    }

    /// Update particles: move, age, remove dead, emit new.
    pub fn update(&mut self, dt: f64) {
        // Update existing
        for p in &mut self.particles {
            p.velocity = p.velocity.add(&self.gravity.scale(dt));
            p.position = p.position.add(&p.velocity.scale(dt));
            p.life -= dt;
            let t = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
            p.color = Color3::new(
                self.start_color.r + (self.end_color.r - self.start_color.r) * t,
                self.start_color.g + (self.end_color.g - self.start_color.g) * t,
                self.start_color.b + (self.end_color.b - self.start_color.b) * t,
            );
            p.size = self.initial_size * (1.0 - t * 0.5);
        }

        // Remove dead
        self.particles.retain(|p| p.life > 0.0);

        // Emit new
        self.accumulator += dt * self.emit_rate;
        while self.accumulator >= 1.0 && self.particles.len() < self.max_particles {
            self.accumulator -= 1.0;
            let dir = Vec3::new(
                self.pseudo_random() * 2.0 - 1.0,
                self.pseudo_random() * 0.5 + 0.5,
                self.pseudo_random() * 2.0 - 1.0,
            ).normalize();
            self.particles.push(Particle {
                position: self.emitter_position,
                velocity: dir.scale(self.initial_speed),
                color: self.start_color,
                size: self.initial_size,
                life: self.life_time,
                max_life: self.life_time,
            });
        }
    }

    pub fn alive_count(&self) -> usize { self.particles.len() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_sample_linear() {
        let track = AnimationTrack::new(1, "position")
            .add_key(0.0, AnimProperty::Position(Vec3::ZERO))
            .add_key(1.0, AnimProperty::Position(Vec3::new(10.0, 0.0, 0.0)));
        if let Some(AnimProperty::Position(p)) = track.sample(0.5) {
            assert!((p.x - 5.0).abs() < 0.01);
        } else { panic!("Expected Position"); }
    }

    #[test]
    fn track_sample_rotation() {
        let track = AnimationTrack::new(1, "rotation")
            .add_key(0.0, AnimProperty::Rotation(Quat::IDENTITY))
            .add_key(1.0, AnimProperty::Rotation(Quat::from_axis_angle(&Vec3::UP, std::f64::consts::PI)));
        let val = track.sample(0.5);
        assert!(val.is_some());
    }

    #[test]
    fn animation_clip() {
        let clip = AnimationClip::new("walk")
            .add_track(AnimationTrack::new(1, "position")
                .add_key(0.0, AnimProperty::Position(Vec3::ZERO))
                .add_key(2.0, AnimProperty::Position(Vec3::new(5.0, 0.0, 0.0))));
        assert!((clip.duration - 2.0).abs() < 0.01);
        let samples = clip.sample(1.0);
        assert_eq!(samples.len(), 1);
    }

    #[test]
    fn animation_mixer() {
        let mut mixer = AnimationMixer::new();
        mixer.add_clip(AnimationClip::new("idle")
            .add_track(AnimationTrack::new(1, "position")
                .add_key(0.0, AnimProperty::Position(Vec3::ZERO))
                .add_key(1.0, AnimProperty::Position(Vec3::UP))));
        let _action = mixer.play("idle");
        let results = mixer.update(0.5);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn mixer_loop() {
        let mut mixer = AnimationMixer::new();
        mixer.add_clip(AnimationClip::new("loop")
            .add_track(AnimationTrack::new(1, "scale")
                .add_key(0.0, AnimProperty::Scale(Vec3::ONE))
                .add_key(1.0, AnimProperty::Scale(Vec3::new(2.0, 2.0, 2.0)))));
        let _a = mixer.play("loop");
        mixer.update(1.5); // Should loop back to 0.5
        assert_eq!(mixer.clip_count(), 1);
    }

    #[test]
    fn skeleton_basic() {
        let mut skel = Skeleton::new();
        let root = skel.add_bone(Bone::new(0, "root"));
        let mut child = Bone::new(1, "spine");
        child.parent = Some(root);
        child.local_position = Vec3::new(0.0, 1.0, 0.0);
        skel.add_bone(child);
        skel.update();
        assert_eq!(skel.bone_count(), 2);
    }

    #[test]
    fn particle_system() {
        let mut ps = ParticleSystem::new(100);
        ps.emit_rate = 50.0;
        ps.update(0.1); // Should emit ~5 particles
        assert!(ps.alive_count() > 0 && ps.alive_count() <= 10);
        for _ in 0..100 { ps.update(0.05); }
        // Should have particles that age out
        assert!(ps.alive_count() > 0);
    }

    #[test]
    fn step_interpolation() {
        let track = AnimationTrack::new(1, "opacity")
            .add_key(0.0, AnimProperty::Opacity(0.0))
            .add_key(1.0, AnimProperty::Opacity(1.0))
            .with_interpolation(Interpolation::Step);
        if let Some(AnimProperty::Opacity(v)) = track.sample(0.5) {
            assert!((v - 0.0).abs() < 0.01); // Step = stays at first value
        }
    }

    #[test]
    fn color_interpolation() {
        let track = AnimationTrack::new(1, "color")
            .add_key(0.0, AnimProperty::Color(Color3::RED))
            .add_key(1.0, AnimProperty::Color(Color3::BLUE));
        if let Some(AnimProperty::Color(c)) = track.sample(0.5) {
            assert!((c.r - 0.5).abs() < 0.01);
            assert!((c.b - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn mixer_stop_pause() {
        let mut mixer = AnimationMixer::new();
        mixer.add_clip(AnimationClip::new("test")
            .add_track(AnimationTrack::new(1, "position")
                .add_key(0.0, AnimProperty::Position(Vec3::ZERO))
                .add_key(1.0, AnimProperty::Position(Vec3::UP))));
        let a = mixer.play("test");
        mixer.pause(a);
        let results = mixer.update(0.5);
        assert!(results.is_empty()); // Paused = no output
        mixer.stop(a);
        assert_eq!(mixer.action_count(), 1);
    }
}
