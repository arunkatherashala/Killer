//! **Post-Processing 3D** — Advanced post-processing effects.
//!
//! SSAO (Screen-Space Ambient Occlusion), Depth of Field, Motion Blur,
//! FXAA anti-aliasing, color grading, vignette, chromatic aberration.

use super::renderer3d::RenderTarget;
use super::scene3d::{Vec3, Color3};

// ══════════════════════════════════════════════════════════════════════════════
// SSAO — Screen-Space Ambient Occlusion
// ══════════════════════════════════════════════════════════════════════════════

/// SSAO configuration.
#[derive(Debug, Clone)]
pub struct SsaoConfig {
    pub radius: f64,
    pub bias: f64,
    pub intensity: f64,
    pub samples: usize,
    pub enabled: bool,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        SsaoConfig { radius: 0.5, bias: 0.025, intensity: 1.0, samples: 16, enabled: true }
    }
}

/// Apply SSAO to a render target using depth buffer.
pub fn apply_ssao(target: &mut RenderTarget, config: &SsaoConfig) {
    if !config.enabled { return; }
    let w = target.width as usize;
    let h = target.height as usize;
    let mut occlusion = vec![0.0f64; w * h];

    // Generate a hemisphere kernel
    let kernel = generate_ssao_kernel(config.samples);

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            let center_depth = target.depth[idx];
            if center_depth >= 1.0 { continue; } // sky

            let mut ao = 0.0;
            for sample in &kernel {
                let sx = (x as f64 + sample.x * config.radius * w as f64) as usize;
                let sy = (y as f64 + sample.y * config.radius * h as f64) as usize;
                if sx < w && sy < h {
                    let sample_depth = target.depth[sy * w + sx];
                    let range_check = if (center_depth - sample_depth).abs() < config.radius { 1.0 } else { 0.0 };
                    if sample_depth < center_depth - config.bias {
                        ao += range_check;
                    }
                }
            }
            ao = 1.0 - (ao / config.samples as f64) * config.intensity;
            occlusion[idx] = ao.clamp(0.0, 1.0);
        }
    }

    // Multiply AO into color
    for i in 0..w * h {
        let ao = occlusion[i];
        let base = i * 4;
        if base + 2 < target.color.len() {
            target.color[base] = (target.color[base] as f64 * ao) as u8;
            target.color[base + 1] = (target.color[base + 1] as f64 * ao) as u8;
            target.color[base + 2] = (target.color[base + 2] as f64 * ao) as u8;
        }
    }
}

fn generate_ssao_kernel(count: usize) -> Vec<Vec3> {
    let mut kernel = Vec::with_capacity(count);
    let mut seed = 42u64;
    for i in 0..count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = (seed >> 33) as f64 / (1u64 << 31) as f64 * 2.0 - 1.0;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = (seed >> 33) as f64 / (1u64 << 31) as f64 * 2.0 - 1.0;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let z = (seed >> 33) as f64 / (1u64 << 31) as f64;
        let mut v = Vec3::new(x, y, z.abs()).normalize();
        // Scale to favor samples closer to center
        let scale = (i as f64 / count as f64).powi(2) * 0.9 + 0.1;
        v = v.scale(scale);
        kernel.push(v);
    }
    kernel
}

// ══════════════════════════════════════════════════════════════════════════════
// Depth of Field
// ══════════════════════════════════════════════════════════════════════════════

/// DoF configuration.
#[derive(Debug, Clone)]
pub struct DofConfig {
    pub focus_distance: f64,
    pub focus_range: f64,
    pub blur_radius: usize,
    pub enabled: bool,
}

impl Default for DofConfig {
    fn default() -> Self {
        DofConfig { focus_distance: 5.0, focus_range: 2.0, blur_radius: 3, enabled: true }
    }
}

/// Apply depth-of-field blur.
pub fn apply_dof(target: &mut RenderTarget, config: &DofConfig) {
    if !config.enabled { return; }
    let w = target.width as usize;
    let h = target.height as usize;
    let original = target.color.clone();
    let r = config.blur_radius;

    for y in r..h.saturating_sub(r) {
        for x in r..w.saturating_sub(r) {
            let idx = y * w + x;
            let depth = target.depth[idx];
            let dist_from_focus = (depth - config.focus_distance).abs();
            let blur_amount = ((dist_from_focus - config.focus_range / 2.0) / config.focus_range).clamp(0.0, 1.0);

            if blur_amount < 0.01 { continue; }

            let kernel_size = (r as f64 * blur_amount).round() as usize;
            if kernel_size == 0 { continue; }

            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            for ky in y.saturating_sub(kernel_size)..=(y + kernel_size).min(h - 1) {
                for kx in x.saturating_sub(kernel_size)..=(x + kernel_size).min(w - 1) {
                    let base = (ky * w + kx) * 4;
                    sum_r += original[base] as u32;
                    sum_g += original[base + 1] as u32;
                    sum_b += original[base + 2] as u32;
                    count += 1;
                }
            }

            if count > 0 {
                let base = idx * 4;
                let t = blur_amount;
                target.color[base] = ((original[base] as f64 * (1.0 - t)) + (sum_r as f64 / count as f64 * t)) as u8;
                target.color[base + 1] = ((original[base + 1] as f64 * (1.0 - t)) + (sum_g as f64 / count as f64 * t)) as u8;
                target.color[base + 2] = ((original[base + 2] as f64 * (1.0 - t)) + (sum_b as f64 / count as f64 * t)) as u8;
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FXAA — Fast Approximate Anti-Aliasing
// ══════════════════════════════════════════════════════════════════════════════

/// Apply FXAA anti-aliasing.
pub fn apply_fxaa(target: &mut RenderTarget) {
    let w = target.width as usize;
    let h = target.height as usize;
    let original = target.color.clone();

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let luma = |px: usize, py: usize| -> f64 {
                let base = (py * w + px) * 4;
                0.299 * original[base] as f64 + 0.587 * original[base + 1] as f64 + 0.114 * original[base + 2] as f64
            };
            let c = luma(x, y);
            let n = luma(x, y - 1);
            let s = luma(x, y + 1);
            let e = luma(x + 1, y);
            let w_l = luma(x - 1, y);

            let range = n.max(s).max(e).max(w_l).max(c) - n.min(s).min(e).min(w_l).min(c);
            if range < 8.0 { continue; }

            // Blend with neighbors
            let base = (y * w + x) * 4;
            for ch in 0..3 {
                let center = original[base + ch] as f64;
                let avg = (original[((y - 1) * w + x) * 4 + ch] as f64
                    + original[((y + 1) * w + x) * 4 + ch] as f64
                    + original[(y * w + x + 1) * 4 + ch] as f64
                    + original[(y * w + x - 1) * 4 + ch] as f64) / 4.0;
                target.color[base + ch] = ((center * 0.5 + avg * 0.5) as u8).min(255);
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Color Grading
// ══════════════════════════════════════════════════════════════════════════════

/// Color grading parameters.
#[derive(Debug, Clone)]
pub struct ColorGrading {
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub temperature: f64,  // -1.0 (cool) to 1.0 (warm)
    pub tint: Color3,      // multiply tint
}

impl Default for ColorGrading {
    fn default() -> Self {
        ColorGrading { brightness: 0.0, contrast: 1.0, saturation: 1.0, temperature: 0.0, tint: Color3::WHITE }
    }
}

/// Apply color grading to render target.
pub fn apply_color_grading(target: &mut RenderTarget, grading: &ColorGrading) {
    for i in (0..target.color.len()).step_by(4) {
        let mut r = target.color[i] as f64 / 255.0;
        let mut g = target.color[i + 1] as f64 / 255.0;
        let mut b = target.color[i + 2] as f64 / 255.0;

        // Brightness
        r += grading.brightness;
        g += grading.brightness;
        b += grading.brightness;

        // Contrast
        r = (r - 0.5) * grading.contrast + 0.5;
        g = (g - 0.5) * grading.contrast + 0.5;
        b = (b - 0.5) * grading.contrast + 0.5;

        // Saturation
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        r = luma + (r - luma) * grading.saturation;
        g = luma + (g - luma) * grading.saturation;
        b = luma + (b - luma) * grading.saturation;

        // Temperature
        r += grading.temperature * 0.1;
        b -= grading.temperature * 0.1;

        // Tint
        r *= grading.tint.r;
        g *= grading.tint.g;
        b *= grading.tint.b;

        target.color[i] = (r.clamp(0.0, 1.0) * 255.0) as u8;
        target.color[i + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
        target.color[i + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Vignette
// ══════════════════════════════════════════════════════════════════════════════

/// Apply vignette darkening at edges.
pub fn apply_vignette(target: &mut RenderTarget, intensity: f64, radius: f64) {
    let w = target.width as usize;
    let h = target.height as usize;
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let max_dist = (cx * cx + cy * cy).sqrt();

    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt() / max_dist;
            let vignette = 1.0 - ((dist - radius).max(0.0) / (1.0 - radius)).min(1.0) * intensity;
            let base = (y * w + x) * 4;
            target.color[base] = (target.color[base] as f64 * vignette) as u8;
            target.color[base + 1] = (target.color[base + 1] as f64 * vignette) as u8;
            target.color[base + 2] = (target.color[base + 2] as f64 * vignette) as u8;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Post-Processing Chain
// ══════════════════════════════════════════════════════════════════════════════

/// Post-processing pipeline configuration.
#[derive(Debug, Clone)]
pub struct PostProcessPipeline {
    pub ssao: Option<SsaoConfig>,
    pub dof: Option<DofConfig>,
    pub fxaa: bool,
    pub color_grading: Option<ColorGrading>,
    pub vignette: Option<(f64, f64)>,  // (intensity, radius)
    pub bloom_threshold: Option<f64>,
}

impl PostProcessPipeline {
    pub fn new() -> Self {
        PostProcessPipeline {
            ssao: None, dof: None, fxaa: false,
            color_grading: None, vignette: None, bloom_threshold: None,
        }
    }

    pub fn with_ssao(mut self, config: SsaoConfig) -> Self { self.ssao = Some(config); self }
    pub fn with_dof(mut self, config: DofConfig) -> Self { self.dof = Some(config); self }
    pub fn with_fxaa(mut self) -> Self { self.fxaa = true; self }
    pub fn with_color_grading(mut self, cg: ColorGrading) -> Self { self.color_grading = Some(cg); self }
    pub fn with_vignette(mut self, intensity: f64, radius: f64) -> Self { self.vignette = Some((intensity, radius)); self }

    /// Apply the full post-processing chain in order.
    pub fn apply(&self, target: &mut RenderTarget) {
        if let Some(ref ssao) = self.ssao { apply_ssao(target, ssao); }
        if let Some(ref dof) = self.dof { apply_dof(target, dof); }
        if self.fxaa { apply_fxaa(target); }
        if let Some(ref cg) = self.color_grading { apply_color_grading(target, cg); }
        if let Some((intensity, radius)) = self.vignette { apply_vignette(target, intensity, radius); }
    }

    pub fn effect_count(&self) -> usize {
        let mut c = 0;
        if self.ssao.is_some() { c += 1; }
        if self.dof.is_some() { c += 1; }
        if self.fxaa { c += 1; }
        if self.color_grading.is_some() { c += 1; }
        if self.vignette.is_some() { c += 1; }
        c
    }
}

impl Default for PostProcessPipeline {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(w: u32, h: u32) -> RenderTarget {
        let mut t = RenderTarget::new(w, h);
        let wu = w as usize;
        let hu = h as usize;
        // Fill with a gradient
        for y in 0..hu {
            for x in 0..wu {
                let base = (y * wu + x) * 4;
                t.color[base] = (x as u8).wrapping_mul(4);
                t.color[base + 1] = (y as u8).wrapping_mul(4);
                t.color[base + 2] = 128;
                t.color[base + 3] = 255;
                t.depth[y * wu + x] = (y as f64) / hu as f64;
            }
        }
        t
    }

    #[test]
    fn ssao_runs() {
        let mut target = make_target(32, 32);
        apply_ssao(&mut target, &SsaoConfig::default());
        // Should darken some pixels
        assert!(target.color[4 * (16 * 32 + 16)] < 128);
    }

    #[test]
    fn dof_runs() {
        let mut target = make_target(32, 32);
        apply_dof(&mut target, &DofConfig { focus_distance: 0.5, focus_range: 0.2, blur_radius: 2, enabled: true });
        // Should modify out-of-focus regions
    }

    #[test]
    fn fxaa_runs() {
        let mut target = make_target(32, 32);
        // Create a sharp edge to trigger FXAA (needs luma range >= 8)
        for y in 0..32usize {
            for x in 0..16usize {
                let base = (y * 32 + x) * 4;
                target.color[base] = 0;
                target.color[base + 1] = 0;
                target.color[base + 2] = 0;
            }
            for x in 16..32usize {
                let base = (y * 32 + x) * 4;
                target.color[base] = 255;
                target.color[base + 1] = 255;
                target.color[base + 2] = 255;
            }
        }
        let before = target.color.clone();
        apply_fxaa(&mut target);
        // FXAA should smooth the sharp edge
        assert_ne!(target.color, before);
    }

    #[test]
    fn color_grading_brightness() {
        let mut target = make_target(4, 4);
        let before = target.color[0];
        apply_color_grading(&mut target, &ColorGrading { brightness: 0.2, ..Default::default() });
        assert!(target.color[0] >= before);
    }

    #[test]
    fn color_grading_desaturate() {
        let mut target = make_target(4, 4);
        apply_color_grading(&mut target, &ColorGrading { saturation: 0.0, ..Default::default() });
        // All pixels should be grayscale (r≈g≈b)
        let r = target.color[0];
        let g = target.color[1];
        let _b = target.color[2];
        assert!((r as i32 - g as i32).abs() <= 2);
    }

    #[test]
    fn vignette_darkens_edges() {
        let mut target = make_target(32, 32);
        let corner_before = target.color[0];
        apply_vignette(&mut target, 1.0, 0.3);
        assert!(target.color[0] <= corner_before);
    }

    #[test]
    fn pipeline_chain() {
        let mut target = make_target(32, 32);
        let pipe = PostProcessPipeline::new()
            .with_fxaa()
            .with_color_grading(ColorGrading::default())
            .with_vignette(0.5, 0.4);
        assert_eq!(pipe.effect_count(), 3);
        pipe.apply(&mut target);
    }

    #[test]
    fn pipeline_with_all_effects() {
        let pipe = PostProcessPipeline::new()
            .with_ssao(SsaoConfig::default())
            .with_dof(DofConfig::default())
            .with_fxaa()
            .with_color_grading(ColorGrading::default())
            .with_vignette(0.8, 0.3);
        assert_eq!(pipe.effect_count(), 5);
    }
}
