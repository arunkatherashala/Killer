//! **3D Audio** — Positional audio, listener, distance attenuation.
//!
//! Spatial sound system with distance models, doppler effect,
//! panning, and multiple audio sources in 3D space.

use super::scene3d::Vec3;

// ══════════════════════════════════════════════════════════════════════════════
// Distance Models
// ══════════════════════════════════════════════════════════════════════════════

/// How volume attenuates with distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceModel {
    Linear,
    Inverse,
    Exponential,
}

impl DistanceModel {
    /// Compute gain for a given distance.
    pub fn gain(&self, distance: f64, ref_distance: f64, max_distance: f64, rolloff: f64) -> f64 {
        let d = distance.max(ref_distance);
        match self {
            DistanceModel::Linear => {
                1.0 - rolloff * (d - ref_distance) / (max_distance - ref_distance)
            }
            DistanceModel::Inverse => {
                ref_distance / (ref_distance + rolloff * (d - ref_distance))
            }
            DistanceModel::Exponential => {
                (d / ref_distance).powf(-rolloff)
            }
        }.clamp(0.0, 1.0)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Audio Listener
// ══════════════════════════════════════════════════════════════════════════════

/// The listener (usually attached to the camera).
#[derive(Debug, Clone)]
pub struct AudioListener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
    pub master_volume: f64,
}

impl AudioListener {
    pub fn new() -> Self {
        AudioListener {
            position: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::ZERO,
            master_volume: 1.0,
        }
    }

    pub fn set_position(&mut self, pos: Vec3) { self.position = pos; }
    pub fn set_orientation(&mut self, forward: Vec3, up: Vec3) {
        self.forward = forward.normalize();
        self.up = up.normalize();
    }

    pub fn right(&self) -> Vec3 { self.forward.cross(&self.up).normalize() }
}

impl Default for AudioListener {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Positional Audio Source
// ══════════════════════════════════════════════════════════════════════════════

/// A spatial audio source in 3D space.
#[derive(Debug, Clone)]
pub struct PositionalAudio {
    pub id: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub volume: f64,
    pub pitch: f64,
    pub looping: bool,
    pub playing: bool,
    pub distance_model: DistanceModel,
    pub ref_distance: f64,
    pub max_distance: f64,
    pub rolloff_factor: f64,
    pub cone_inner: f64,   // degrees
    pub cone_outer: f64,   // degrees
    pub cone_outer_gain: f64,
    pub direction: Option<Vec3>,
}

impl PositionalAudio {
    pub fn new(id: &str, position: Vec3) -> Self {
        PositionalAudio {
            id: id.into(), position, velocity: Vec3::ZERO,
            volume: 1.0, pitch: 1.0, looping: false, playing: false,
            distance_model: DistanceModel::Inverse,
            ref_distance: 1.0, max_distance: 100.0, rolloff_factor: 1.0,
            cone_inner: 360.0, cone_outer: 360.0, cone_outer_gain: 0.0,
            direction: None,
        }
    }

    pub fn play(&mut self) { self.playing = true; }
    pub fn stop(&mut self) { self.playing = false; }
    pub fn pause(&mut self) { self.playing = false; }

    /// Compute the effective volume at the listener position.
    pub fn effective_volume(&self, listener: &AudioListener) -> f64 {
        if !self.playing { return 0.0; }
        let dist = self.position.sub(&listener.position).length();
        let dist_gain = self.distance_model.gain(dist, self.ref_distance, self.max_distance, self.rolloff_factor);
        let cone_gain = self.cone_gain(listener);
        self.volume * dist_gain * cone_gain * listener.master_volume
    }

    /// Compute stereo panning (-1 = left, 0 = center, 1 = right).
    pub fn stereo_pan(&self, listener: &AudioListener) -> f64 {
        let to_source = self.position.sub(&listener.position).normalize();
        let right = listener.right();
        right.dot(&to_source).clamp(-1.0, 1.0)
    }

    /// Compute doppler pitch shift.
    pub fn doppler_pitch(&self, listener: &AudioListener, speed_of_sound: f64) -> f64 {
        let to_listener = listener.position.sub(&self.position).normalize();
        let v_listener = listener.velocity.dot(&to_listener);
        let v_source = self.velocity.dot(&to_listener);
        let denom = speed_of_sound - v_source;
        if denom.abs() < 0.001 { return self.pitch; }
        self.pitch * (speed_of_sound - v_listener) / denom
    }

    fn cone_gain(&self, listener: &AudioListener) -> f64 {
        let dir = match &self.direction {
            Some(d) => d,
            None => return 1.0,
        };
        let to_listener = listener.position.sub(&self.position).normalize();
        let angle = dir.normalize().dot(&to_listener).acos().to_degrees();
        let half_inner = self.cone_inner / 2.0;
        let half_outer = self.cone_outer / 2.0;
        if angle <= half_inner { 1.0 }
        else if angle >= half_outer { self.cone_outer_gain }
        else {
            let t = (angle - half_inner) / (half_outer - half_inner);
            1.0 + (self.cone_outer_gain - 1.0) * t
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Audio Scene
// ══════════════════════════════════════════════════════════════════════════════

/// Manages all positional audio sources and the listener.
#[derive(Debug)]
pub struct AudioScene {
    pub listener: AudioListener,
    pub sources: Vec<PositionalAudio>,
    pub speed_of_sound: f64,
}

impl AudioScene {
    pub fn new() -> Self {
        AudioScene { listener: AudioListener::new(), sources: Vec::new(), speed_of_sound: 343.0 }
    }

    pub fn add_source(&mut self, source: PositionalAudio) {
        self.sources.push(source);
    }

    /// Compute audio mix for all playing sources.
    pub fn compute_mix(&self) -> Vec<AudioMixEntry> {
        self.sources.iter().filter(|s| s.playing).map(|s| {
            AudioMixEntry {
                source_id: s.id.clone(),
                volume: s.effective_volume(&self.listener),
                pan: s.stereo_pan(&self.listener),
                pitch: s.doppler_pitch(&self.listener, self.speed_of_sound),
            }
        }).collect()
    }

    pub fn playing_count(&self) -> usize {
        self.sources.iter().filter(|s| s.playing).count()
    }
}

impl Default for AudioScene {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct AudioMixEntry {
    pub source_id: String,
    pub volume: f64,
    pub pan: f64,
    pub pitch: f64,
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_linear() {
        let g = DistanceModel::Linear.gain(50.0, 1.0, 100.0, 1.0);
        assert!(g > 0.0 && g < 1.0);
    }

    #[test]
    fn distance_inverse() {
        let g1 = DistanceModel::Inverse.gain(1.0, 1.0, 100.0, 1.0);
        let g10 = DistanceModel::Inverse.gain(10.0, 1.0, 100.0, 1.0);
        assert!((g1 - 1.0).abs() < 0.01);
        assert!(g10 < g1);
    }

    #[test]
    fn distance_exponential() {
        let g = DistanceModel::Exponential.gain(2.0, 1.0, 100.0, 2.0);
        assert!((g - 0.25).abs() < 0.01); // (2/1)^-2 = 0.25
    }

    #[test]
    fn listener_orientation() {
        let mut l = AudioListener::new();
        l.set_orientation(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0));
        let right = l.right();
        assert!((right.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn positional_audio_volume() {
        let listener = AudioListener::new();
        let mut src = PositionalAudio::new("music", Vec3::new(10.0, 0.0, 0.0));
        src.play();
        let vol = src.effective_volume(&listener);
        assert!(vol > 0.0 && vol < 1.0);
    }

    #[test]
    fn stereo_pan() {
        let listener = AudioListener::new();
        let src = PositionalAudio::new("left", Vec3::new(-5.0, 0.0, 0.0));
        let pan = src.stereo_pan(&listener);
        assert!(pan < -0.5); // should be panned left
    }

    #[test]
    fn doppler_effect() {
        let listener = AudioListener::new();
        let mut src = PositionalAudio::new("car", Vec3::new(10.0, 0.0, 0.0));
        src.velocity = Vec3::new(-50.0, 0.0, 0.0); // approaching
        let pitch = src.doppler_pitch(&listener, 343.0);
        assert!(pitch > 1.0); // higher pitch when approaching
    }

    #[test]
    fn audio_scene_mix() {
        let mut scene = AudioScene::new();
        let mut s1 = PositionalAudio::new("a", Vec3::new(5.0, 0.0, 0.0));
        s1.play();
        let mut s2 = PositionalAudio::new("b", Vec3::new(-5.0, 0.0, 0.0));
        s2.play();
        scene.add_source(s1);
        scene.add_source(s2);
        let mix = scene.compute_mix();
        assert_eq!(mix.len(), 2);
        assert_eq!(scene.playing_count(), 2);
    }

    #[test]
    fn audio_stopped_no_volume() {
        let listener = AudioListener::new();
        let src = PositionalAudio::new("quiet", Vec3::new(1.0, 0.0, 0.0));
        assert!((src.effective_volume(&listener) - 0.0).abs() < 0.001);
    }
}
