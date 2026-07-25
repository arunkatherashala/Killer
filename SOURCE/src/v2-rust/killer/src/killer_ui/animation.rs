//! **Animation system** — keyframe-based property animation with easing functions.
//!
//! Animations are frame-driven: advance via `tick(dt_ms)`. Each animation targets a
//! named property on a widget ID, interpolating between `from` and `to` values.
//! Supports standard easing (linear, ease-in, ease-out, ease-in-out, cubic-bezier).

use std::collections::HashMap;

// ── Easing functions ─────────────────────────────────────────────────────────

// ── Easing functions ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Cubic bezier with 4 control points (CSS `cubic-bezier(p1, p2, p3, p4)`).
    CubicBezier(f64, f64, f64, f64),
    /// Spring physics: stiffness, damping.
    Spring(f64, f64),
    /// Discrete step function: N steps.
    Steps(u32),
    /// Bounce effect.
    Bounce,
}

impl Easing {
    /// Evaluate easing at t ∈ [0, 1] → output ∈ [0, 1].
    pub fn eval(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t * t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::CubicBezier(p1, p2, p3, p4) => {
                // Approximate cubic bezier with de Casteljau (simplified)
                let u = 1.0 - t;
                let tt = t * t;
                let uu = u * u;
                let uuu = uu * u;
                let ttt = tt * t;
                uuu * 0.0 + 3.0 * uu * t * p1 + 3.0 * u * tt * p3 + ttt * 1.0
                    + (uuu * 0.0 + 3.0 * uu * t * p2 + 3.0 * u * tt * p4 + ttt * 1.0 - t) * 0.0
                // Simplified: just use y-coordinate interpolation
            }
            Easing::Spring(stiffness, damping) => {
                let omega = stiffness.sqrt();
                let zeta = damping / (2.0 * omega);
                if zeta < 1.0 {
                    let wd = omega * (1.0 - zeta * zeta).sqrt();
                    1.0 - (-zeta * omega * t).exp() * ((zeta * omega * t / wd).cos() + zeta / (1.0 - zeta * zeta).sqrt() * (zeta * omega * t / wd).sin())
                } else {
                    1.0 - (1.0 + omega * t) * (-omega * t).exp()
                }
            }
            Easing::Steps(n) => {
                let n = (*n).max(1) as f64;
                (t * n).floor() / n
            }
            Easing::Bounce => {
                let t = 1.0 - t;
                let v = if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                };
                1.0 - v
            }
        }
    }
}

// ── Animation definition ─────────────────────────────────────────────────────

pub type AnimationId = u64;

/// State of a single running animation.
#[derive(Debug, Clone)]
pub struct Animation {
    pub id: AnimationId,
    pub widget_id: String,
    pub property: String,
    pub from: f64,
    pub to: f64,
    pub duration_ms: f64,
    pub delay_ms: f64,
    pub easing: Easing,
    pub elapsed_ms: f64,
    pub repeat: AnimRepeat,
    pub state: AnimState,
    /// Current interpolated value.
    pub current_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AnimRepeat {
    #[default]
    Once,
    Loop,
    PingPong,
    Count(u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimState {
    Pending,
    Running,
    Paused,
    Completed,
    Cancelled,
}

// ── Keyframe animation ───────────────────────────────────────────────────────

/// Multi-step keyframe animation.
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Progress [0.0, 1.0] at which this keyframe is reached.
    pub progress: f64,
    pub value: f64,
    pub easing: Easing,
}

#[derive(Debug, Clone)]
pub struct KeyframeAnimation {
    pub id: AnimationId,
    pub widget_id: String,
    pub property: String,
    pub keyframes: Vec<Keyframe>,
    pub duration_ms: f64,
    pub elapsed_ms: f64,
    pub repeat: AnimRepeat,
    pub state: AnimState,
    pub current_value: f64,
}

// ── Animation Controller ─────────────────────────────────────────────────────

/// Manages all running animations and advances them per frame.
#[derive(Debug)]
pub struct AnimationController {
    animations: HashMap<AnimationId, Animation>,
    keyframe_animations: HashMap<AnimationId, KeyframeAnimation>,
    next_id: AnimationId,
    /// Animations completed this frame (for event system integration).
    pub completed_this_frame: Vec<AnimationId>,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            keyframe_animations: HashMap::new(),
            next_id: 1,
            completed_this_frame: Vec::new(),
        }
    }

    /// Start a simple from→to animation. Returns the animation ID.
    pub fn animate(
        &mut self,
        widget_id: &str,
        property: &str,
        from: f64,
        to: f64,
        duration_ms: f64,
        easing: Easing,
    ) -> AnimationId {
        let id = self.next_id;
        self.next_id += 1;
        self.animations.insert(id, Animation {
            id,
            widget_id: widget_id.to_string(),
            property: property.to_string(),
            from,
            to,
            duration_ms,
            delay_ms: 0.0,
            easing,
            elapsed_ms: 0.0,
            repeat: AnimRepeat::Once,
            state: AnimState::Running,
            current_value: from,
        });
        id
    }

    /// Start a keyframe animation.
    pub fn animate_keyframes(
        &mut self,
        widget_id: &str,
        property: &str,
        keyframes: Vec<Keyframe>,
        duration_ms: f64,
    ) -> AnimationId {
        let id = self.next_id;
        self.next_id += 1;
        let initial = keyframes.first().map(|k| k.value).unwrap_or(0.0);
        self.keyframe_animations.insert(id, KeyframeAnimation {
            id,
            widget_id: widget_id.to_string(),
            property: property.to_string(),
            keyframes,
            duration_ms,
            elapsed_ms: 0.0,
            repeat: AnimRepeat::Once,
            state: AnimState::Running,
            current_value: initial,
        });
        id
    }

    /// Set repeat mode on an animation.
    pub fn set_repeat(&mut self, id: AnimationId, repeat: AnimRepeat) {
        if let Some(a) = self.animations.get_mut(&id) { a.repeat = repeat; }
        if let Some(a) = self.keyframe_animations.get_mut(&id) { a.repeat = repeat; }
    }

    /// Pause an animation.
    pub fn pause(&mut self, id: AnimationId) {
        if let Some(a) = self.animations.get_mut(&id) { a.state = AnimState::Paused; }
        if let Some(a) = self.keyframe_animations.get_mut(&id) { a.state = AnimState::Paused; }
    }

    /// Resume a paused animation.
    pub fn resume(&mut self, id: AnimationId) {
        if let Some(a) = self.animations.get_mut(&id) {
            if a.state == AnimState::Paused { a.state = AnimState::Running; }
        }
        if let Some(a) = self.keyframe_animations.get_mut(&id) {
            if a.state == AnimState::Paused { a.state = AnimState::Running; }
        }
    }

    /// Cancel an animation.
    pub fn cancel(&mut self, id: AnimationId) {
        if let Some(a) = self.animations.get_mut(&id) { a.state = AnimState::Cancelled; }
        if let Some(a) = self.keyframe_animations.get_mut(&id) { a.state = AnimState::Cancelled; }
    }

    /// Advance all running animations by `dt_ms`. Returns widget→property→value map of changes.
    pub fn tick(&mut self, dt_ms: f64) -> HashMap<String, HashMap<String, f64>> {
        self.completed_this_frame.clear();
        let mut changes: HashMap<String, HashMap<String, f64>> = HashMap::new();

        // Simple animations
        let anim_ids: Vec<AnimationId> = self.animations.keys().cloned().collect();
        for id in anim_ids {
            let anim = self.animations.get_mut(&id).unwrap();
            if anim.state != AnimState::Running { continue; }

            anim.elapsed_ms += dt_ms;
            let t_raw = (anim.elapsed_ms - anim.delay_ms) / anim.duration_ms;

            if t_raw < 0.0 { continue; } // still in delay

            let (t, finished) = handle_repeat(t_raw, &anim.repeat);
            let eased = anim.easing.eval(t);
            anim.current_value = anim.from + (anim.to - anim.from) * eased;

            changes.entry(anim.widget_id.clone())
                .or_default()
                .insert(anim.property.clone(), anim.current_value);

            if finished {
                anim.state = AnimState::Completed;
                self.completed_this_frame.push(id);
            }
        }

        // Keyframe animations
        let kf_ids: Vec<AnimationId> = self.keyframe_animations.keys().cloned().collect();
        for id in kf_ids {
            let anim = self.keyframe_animations.get_mut(&id).unwrap();
            if anim.state != AnimState::Running { continue; }

            anim.elapsed_ms += dt_ms;
            let t_raw = anim.elapsed_ms / anim.duration_ms;
            let (t, finished) = handle_repeat(t_raw, &anim.repeat);
            anim.current_value = interpolate_keyframes(&anim.keyframes, t);

            changes.entry(anim.widget_id.clone())
                .or_default()
                .insert(anim.property.clone(), anim.current_value);

            if finished {
                anim.state = AnimState::Completed;
                self.completed_this_frame.push(id);
            }
        }

        // Clean up completed/cancelled
        self.animations.retain(|_, a| !matches!(a.state, AnimState::Completed | AnimState::Cancelled));
        self.keyframe_animations.retain(|_, a| !matches!(a.state, AnimState::Completed | AnimState::Cancelled));

        changes
    }

    /// Number of active (running + paused) animations.
    pub fn active_count(&self) -> usize {
        self.animations.len() + self.keyframe_animations.len()
    }
}

impl Default for AnimationController {
    fn default() -> Self { Self::new() }
}

// ── Animation Sequence ───────────────────────────────────────────────────────

/// Chain multiple animations A → B → C: each starts when the previous completes.
#[derive(Debug, Clone)]
pub struct AnimationSequence {
    /// Steps in order. Each step is (widget_id, property, from, to, duration_ms, easing).
    pub steps: Vec<SequenceStep>,
    /// Index of the currently active step.
    pub current_step: usize,
    /// The animation ID of the currently running step (managed by AnimationController).
    pub active_anim_id: Option<AnimationId>,
    /// Is the sequence finished?
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub widget_id: String,
    pub property: String,
    pub from: f64,
    pub to: f64,
    pub duration_ms: f64,
    pub easing: Easing,
}

impl AnimationSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new(), current_step: 0, active_anim_id: None, completed: false }
    }

    /// Add a step to the sequence.
    pub fn then(mut self, widget_id: &str, property: &str, from: f64, to: f64, duration_ms: f64, easing: Easing) -> Self {
        self.steps.push(SequenceStep {
            widget_id: widget_id.to_string(),
            property: property.to_string(),
            from, to, duration_ms, easing,
        });
        self
    }

    /// Start the sequence (launches the first step).
    pub fn start(&mut self, ctrl: &mut AnimationController) {
        self.current_step = 0;
        self.completed = false;
        self.launch_current(ctrl);
    }

    /// Advance the sequence: call after each tick. If current step completed, starts next.
    pub fn advance(&mut self, ctrl: &mut AnimationController) {
        if self.completed { return; }
        if let Some(anim_id) = self.active_anim_id {
            if ctrl.completed_this_frame.contains(&anim_id) {
                self.current_step += 1;
                if self.current_step >= self.steps.len() {
                    self.completed = true;
                    self.active_anim_id = None;
                } else {
                    self.launch_current(ctrl);
                }
            }
        }
    }

    fn launch_current(&mut self, ctrl: &mut AnimationController) {
        if let Some(step) = self.steps.get(self.current_step) {
            let id = ctrl.animate(&step.widget_id, &step.property, step.from, step.to, step.duration_ms, step.easing.clone());
            self.active_anim_id = Some(id);
        }
    }

    pub fn step_count(&self) -> usize { self.steps.len() }
}

impl Default for AnimationSequence {
    fn default() -> Self { Self::new() }
}

// ── 3D Transforms ────────────────────────────────────────────────────────────

/// 3D transform operations for widgets.
#[derive(Debug, Clone, PartialEq)]
pub enum Transform3D {
    Translate3D { x: f64, y: f64, z: f64 },
    Rotate3D { x: f64, y: f64, z: f64, angle_deg: f64 },
    Scale3D { x: f64, y: f64, z: f64 },
    Perspective(f64),
    RotateX(f64),
    RotateY(f64),
    RotateZ(f64),
}

/// A transform stack: multiple transforms composed in order.
#[derive(Debug, Clone, Default)]
pub struct TransformStack {
    pub transforms: Vec<Transform3D>,
}

impl TransformStack {
    pub fn new() -> Self { Self::default() }

    pub fn push(mut self, t: Transform3D) -> Self {
        self.transforms.push(t);
        self
    }

    /// Compute the flat 4×4 matrix (column-major) from the transform stack.
    /// Returns [f64; 16] suitable for GPU or software rendering.
    pub fn to_matrix(&self) -> [f64; 16] {
        let mut result = identity_4x4();
        for t in &self.transforms {
            let m = transform_to_matrix(t);
            result = mul_4x4(&result, &m);
        }
        result
    }
}

fn identity_4x4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn transform_to_matrix(t: &Transform3D) -> [f64; 16] {
    match t {
        Transform3D::Translate3D { x, y, z } => {
            let mut m = identity_4x4();
            m[12] = *x; m[13] = *y; m[14] = *z;
            m
        }
        Transform3D::Scale3D { x, y, z } => {
            let mut m = identity_4x4();
            m[0] = *x; m[5] = *y; m[10] = *z;
            m
        }
        Transform3D::RotateZ(deg) => {
            let rad = deg.to_radians();
            let c = rad.cos();
            let s = rad.sin();
            let mut m = identity_4x4();
            m[0] = c; m[1] = s;
            m[4] = -s; m[5] = c;
            m
        }
        Transform3D::RotateX(deg) => {
            let rad = deg.to_radians();
            let c = rad.cos();
            let s = rad.sin();
            let mut m = identity_4x4();
            m[5] = c; m[6] = s;
            m[9] = -s; m[10] = c;
            m
        }
        Transform3D::RotateY(deg) => {
            let rad = deg.to_radians();
            let c = rad.cos();
            let s = rad.sin();
            let mut m = identity_4x4();
            m[0] = c; m[2] = -s;
            m[8] = s; m[10] = c;
            m
        }
        Transform3D::Rotate3D { x, y, z, angle_deg } => {
            let rad = angle_deg.to_radians();
            let c = rad.cos();
            let s = rad.sin();
            let t = 1.0 - c;
            let len = (x * x + y * y + z * z).sqrt();
            if len < 1e-10 { return identity_4x4(); }
            let (ux, uy, uz) = (x / len, y / len, z / len);
            [
                t * ux * ux + c,       t * ux * uy + s * uz, t * ux * uz - s * uy, 0.0,
                t * ux * uy - s * uz, t * uy * uy + c,       t * uy * uz + s * ux, 0.0,
                t * ux * uz + s * uy, t * uy * uz - s * ux, t * uz * uz + c,       0.0,
                0.0,                   0.0,                   0.0,                   1.0,
            ]
        }
        Transform3D::Perspective(d) => {
            let mut m = identity_4x4();
            if *d != 0.0 { m[11] = -1.0 / d; }
            m
        }
    }
}

fn mul_4x4(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let mut r = [0.0; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            r[col * 4 + row] = sum;
        }
    }
    r
}

fn handle_repeat(t_raw: f64, repeat: &AnimRepeat) -> (f64, bool) {
    match repeat {
        AnimRepeat::Once => {
            if t_raw >= 1.0 { (1.0, true) } else { (t_raw, false) }
        }
        AnimRepeat::Loop => {
            (t_raw.fract(), false)
        }
        AnimRepeat::PingPong => {
            let cycle = t_raw.floor() as u32;
            let frac = t_raw.fract();
            if cycle % 2 == 0 { (frac, false) } else { (1.0 - frac, false) }
        }
        AnimRepeat::Count(n) => {
            if t_raw >= *n as f64 {
                (1.0, true)
            } else {
                (t_raw.fract(), false)
            }
        }
    }
}

fn interpolate_keyframes(keyframes: &[Keyframe], t: f64) -> f64 {
    if keyframes.is_empty() { return 0.0; }
    if keyframes.len() == 1 { return keyframes[0].value; }

    // Find the two keyframes surrounding t
    let mut prev = &keyframes[0];
    for kf in &keyframes[1..] {
        if t <= kf.progress {
            let span = kf.progress - prev.progress;
            if span <= 0.0 { return kf.value; }
            let local_t = (t - prev.progress) / span;
            let eased = kf.easing.eval(local_t);
            return prev.value + (kf.value - prev.value) * eased;
        }
        prev = kf;
    }
    keyframes.last().unwrap().value
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_linear() {
        let e = Easing::Linear;
        assert_eq!(e.eval(0.0), 0.0);
        assert_eq!(e.eval(0.5), 0.5);
        assert_eq!(e.eval(1.0), 1.0);
    }

    #[test]
    fn easing_ease_in_out() {
        let e = Easing::EaseInOut;
        assert!(e.eval(0.5) > 0.4 && e.eval(0.5) < 0.6);
        assert!(e.eval(0.1) < 0.1); // slow start
        assert!(e.eval(0.9) > 0.9); // slow end
    }

    #[test]
    fn easing_steps() {
        let e = Easing::Steps(4);
        assert_eq!(e.eval(0.1), 0.0);
        assert_eq!(e.eval(0.3), 0.25);
        assert_eq!(e.eval(0.6), 0.5);
    }

    #[test]
    fn simple_animation_completes() {
        let mut ctrl = AnimationController::new();
        let id = ctrl.animate("box", "opacity", 0.0, 1.0, 100.0, Easing::Linear);
        let changes = ctrl.tick(50.0);
        assert!((changes["box"]["opacity"] - 0.5).abs() < 0.01);
        let changes = ctrl.tick(50.0);
        assert!((changes["box"]["opacity"] - 1.0).abs() < 0.01);
        assert!(ctrl.completed_this_frame.contains(&id));
        assert_eq!(ctrl.active_count(), 0);
    }

    #[test]
    fn pause_resume() {
        let mut ctrl = AnimationController::new();
        let id = ctrl.animate("box", "x", 0.0, 100.0, 200.0, Easing::Linear);
        ctrl.tick(50.0);
        ctrl.pause(id);
        let changes = ctrl.tick(100.0);
        // Paused → no change
        assert!(changes.is_empty() || !changes.contains_key("box"));
        ctrl.resume(id);
        let changes = ctrl.tick(50.0);
        assert!(changes.contains_key("box"));
    }

    #[test]
    fn loop_repeat_never_completes() {
        let mut ctrl = AnimationController::new();
        let id = ctrl.animate("spinner", "rotation", 0.0, 360.0, 100.0, Easing::Linear);
        ctrl.set_repeat(id, AnimRepeat::Loop);
        ctrl.tick(150.0); // 1.5 loops
        assert_eq!(ctrl.active_count(), 1); // still running
    }

    #[test]
    fn keyframe_animation() {
        let mut ctrl = AnimationController::new();
        let kfs = vec![
            Keyframe { progress: 0.0, value: 0.0, easing: Easing::Linear },
            Keyframe { progress: 0.5, value: 100.0, easing: Easing::Linear },
            Keyframe { progress: 1.0, value: 50.0, easing: Easing::Linear },
        ];
        ctrl.animate_keyframes("box", "y", kfs, 200.0);
        let changes = ctrl.tick(100.0); // t=0.5
        assert!((changes["box"]["y"] - 100.0).abs() < 1.0);
        let changes = ctrl.tick(100.0); // t=1.0
        assert!((changes["box"]["y"] - 50.0).abs() < 1.0);
    }

    #[test]
    fn sequence_chains_animations() {
        let mut ctrl = AnimationController::new();
        let mut seq = AnimationSequence::new()
            .then("box", "x", 0.0, 100.0, 100.0, Easing::Linear)
            .then("box", "y", 0.0, 200.0, 100.0, Easing::Linear);
        assert_eq!(seq.step_count(), 2);
        seq.start(&mut ctrl);
        assert_eq!(seq.current_step, 0);
        // Complete step 0
        ctrl.tick(100.0);
        seq.advance(&mut ctrl);
        assert_eq!(seq.current_step, 1);
        assert!(!seq.completed);
        // Complete step 1
        ctrl.tick(100.0);
        seq.advance(&mut ctrl);
        assert!(seq.completed);
    }

    #[test]
    fn transform_3d_translate() {
        let stack = TransformStack::new()
            .push(Transform3D::Translate3D { x: 10.0, y: 20.0, z: 30.0 });
        let m = stack.to_matrix();
        assert!((m[12] - 10.0).abs() < 1e-9);
        assert!((m[13] - 20.0).abs() < 1e-9);
        assert!((m[14] - 30.0).abs() < 1e-9);
    }

    #[test]
    fn transform_3d_scale() {
        let stack = TransformStack::new()
            .push(Transform3D::Scale3D { x: 2.0, y: 3.0, z: 4.0 });
        let m = stack.to_matrix();
        assert!((m[0] - 2.0).abs() < 1e-9);
        assert!((m[5] - 3.0).abs() < 1e-9);
        assert!((m[10] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn transform_3d_rotate_z_90() {
        let stack = TransformStack::new()
            .push(Transform3D::RotateZ(90.0));
        let m = stack.to_matrix();
        // cos(90)≈0, sin(90)≈1
        assert!(m[0].abs() < 1e-9);
        assert!((m[1] - 1.0).abs() < 1e-9);
    }
}
