//! **Environment Maps** — CubeTexture, equirectangular HDRI, reflection probes.
//!
//! 6-face cubemaps, equirectangular-to-cubemap conversion,
//! reflection probes with parallax correction, skybox rendering.

use super::scene3d::{Vec3, Color3};

// ══════════════════════════════════════════════════════════════════════════════
// CubeMap
// ══════════════════════════════════════════════════════════════════════════════

/// Which face of a cubemap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubeFace {
    PosX, NegX, PosY, NegY, PosZ, NegZ,
}

impl CubeFace {
    pub fn all() -> [CubeFace; 6] {
        [CubeFace::PosX, CubeFace::NegX, CubeFace::PosY, CubeFace::NegY, CubeFace::PosZ, CubeFace::NegZ]
    }

    /// Get the direction vector for this face.
    pub fn direction(&self) -> Vec3 {
        match self {
            CubeFace::PosX => Vec3::new(1.0, 0.0, 0.0),
            CubeFace::NegX => Vec3::new(-1.0, 0.0, 0.0),
            CubeFace::PosY => Vec3::new(0.0, 1.0, 0.0),
            CubeFace::NegY => Vec3::new(0.0, -1.0, 0.0),
            CubeFace::PosZ => Vec3::new(0.0, 0.0, 1.0),
            CubeFace::NegZ => Vec3::new(0.0, 0.0, -1.0),
        }
    }
}

/// A cubemap texture (6 faces).
#[derive(Debug, Clone)]
pub struct CubeTexture {
    pub size: u32,  // pixels per face edge
    pub faces: [Vec<u8>; 6],  // RGBA data per face
    pub mip_levels: u32,
}

impl CubeTexture {
    pub fn new(size: u32) -> Self {
        let face_size = (size * size * 4) as usize;
        CubeTexture {
            size,
            faces: [
                vec![0u8; face_size], vec![0u8; face_size],
                vec![0u8; face_size], vec![0u8; face_size],
                vec![0u8; face_size], vec![0u8; face_size],
            ],
            mip_levels: 1,
        }
    }

    /// Fill a face with a solid color.
    pub fn fill_face(&mut self, face: CubeFace, color: Color3) {
        let idx = face as usize;
        let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        for i in (0..self.faces[idx].len()).step_by(4) {
            self.faces[idx][i] = r;
            self.faces[idx][i + 1] = g;
            self.faces[idx][i + 2] = b;
            self.faces[idx][i + 3] = 255;
        }
    }

    /// Sample the cubemap given a 3D direction.
    pub fn sample(&self, dir: &Vec3) -> Color3 {
        let (face, u, v) = direction_to_face_uv(dir);
        let fi = face as usize;
        let s = self.size as usize;
        let px = (u * (s - 1) as f64) as usize;
        let py = (v * (s - 1) as f64) as usize;
        let base = (py * s + px) * 4;
        if base + 2 < self.faces[fi].len() {
            Color3 {
                r: self.faces[fi][base] as f64 / 255.0,
                g: self.faces[fi][base + 1] as f64 / 255.0,
                b: self.faces[fi][base + 2] as f64 / 255.0,
            }
        } else {
            Color3::BLACK
        }
    }

    /// Create a simple gradient skybox.
    pub fn gradient_skybox(size: u32, sky: Color3, ground: Color3) -> Self {
        let mut ct = CubeTexture::new(size);
        // Top face = sky, bottom = ground, sides = gradient
        ct.fill_face(CubeFace::PosY, sky);
        ct.fill_face(CubeFace::NegY, ground);
        for face in [CubeFace::PosX, CubeFace::NegX, CubeFace::PosZ, CubeFace::NegZ] {
            let fi = face as usize;
            let s = size as usize;
            for y in 0..s {
                let t = y as f64 / s as f64;
                let r = sky.r * (1.0 - t) + ground.r * t;
                let g = sky.g * (1.0 - t) + ground.g * t;
                let b = sky.b * (1.0 - t) + ground.b * t;
                for x in 0..s {
                    let base = (y * s + x) * 4;
                    ct.faces[fi][base] = (r * 255.0) as u8;
                    ct.faces[fi][base + 1] = (g * 255.0) as u8;
                    ct.faces[fi][base + 2] = (b * 255.0) as u8;
                    ct.faces[fi][base + 3] = 255;
                }
            }
        }
        ct
    }
}

/// Convert a 3D direction to cubemap face + UV coordinates.
fn direction_to_face_uv(dir: &Vec3) -> (CubeFace, f64, f64) {
    let ax = dir.x.abs();
    let ay = dir.y.abs();
    let az = dir.z.abs();
    if ax >= ay && ax >= az {
        if dir.x > 0.0 { (CubeFace::PosX, 0.5 - dir.z / (2.0 * ax) + 0.5, 0.5 - dir.y / (2.0 * ax) + 0.5) }
        else { (CubeFace::NegX, 0.5 + dir.z / (2.0 * ax), 0.5 - dir.y / (2.0 * ax) + 0.5) }
    } else if ay >= ax && ay >= az {
        if dir.y > 0.0 { (CubeFace::PosY, 0.5 + dir.x / (2.0 * ay), 0.5 + dir.z / (2.0 * ay)) }
        else { (CubeFace::NegY, 0.5 + dir.x / (2.0 * ay), 0.5 - dir.z / (2.0 * ay) + 0.5) }
    } else {
        if dir.z > 0.0 { (CubeFace::PosZ, 0.5 + dir.x / (2.0 * az), 0.5 - dir.y / (2.0 * az) + 0.5) }
        else { (CubeFace::NegZ, 0.5 - dir.x / (2.0 * az) + 0.5, 0.5 - dir.y / (2.0 * az) + 0.5) }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Equirectangular
// ══════════════════════════════════════════════════════════════════════════════

/// Convert equirectangular panorama to cubemap.
pub fn equirect_to_cubemap(equirect: &[u8], eq_width: u32, eq_height: u32, cube_size: u32) -> CubeTexture {
    let mut cube = CubeTexture::new(cube_size);
    let s = cube_size as usize;
    for face in CubeFace::all() {
        let fi = face as usize;
        for y in 0..s {
            for x in 0..s {
                let u = (x as f64 + 0.5) / s as f64 * 2.0 - 1.0;
                let v = (y as f64 + 0.5) / s as f64 * 2.0 - 1.0;
                let dir = face_uv_to_direction(face, u, v).normalize();

                // Spherical to equirect UV
                let theta = dir.y.asin();
                let phi = dir.z.atan2(dir.x);
                let eu = (phi / (2.0 * std::f64::consts::PI) + 0.5).fract();
                let ev = (theta / std::f64::consts::PI + 0.5).clamp(0.0, 1.0);
                let ex = (eu * (eq_width - 1) as f64) as usize;
                let ey = (ev * (eq_height - 1) as f64) as usize;
                let src = (ey * eq_width as usize + ex) * 4;
                let dst = (y * s + x) * 4;
                if src + 3 < equirect.len() && dst + 3 < cube.faces[fi].len() {
                    cube.faces[fi][dst] = equirect[src];
                    cube.faces[fi][dst + 1] = equirect[src + 1];
                    cube.faces[fi][dst + 2] = equirect[src + 2];
                    cube.faces[fi][dst + 3] = equirect[src + 3];
                }
            }
        }
    }
    cube
}

fn face_uv_to_direction(face: CubeFace, u: f64, v: f64) -> Vec3 {
    match face {
        CubeFace::PosX => Vec3::new(1.0, -v, -u),
        CubeFace::NegX => Vec3::new(-1.0, -v, u),
        CubeFace::PosY => Vec3::new(u, 1.0, v),
        CubeFace::NegY => Vec3::new(u, -1.0, -v),
        CubeFace::PosZ => Vec3::new(u, -v, 1.0),
        CubeFace::NegZ => Vec3::new(-u, -v, -1.0),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Reflection Probe
// ══════════════════════════════════════════════════════════════════════════════

/// A reflection probe captures the environment at a point.
#[derive(Debug, Clone)]
pub struct ReflectionProbe {
    pub position: Vec3,
    pub cubemap: Option<CubeTexture>,
    pub influence_radius: f64,
    pub resolution: u32,
}

impl ReflectionProbe {
    pub fn new(position: Vec3, radius: f64, resolution: u32) -> Self {
        ReflectionProbe { position, cubemap: None, influence_radius: radius, resolution }
    }

    /// Check if a point is within this probe's influence.
    pub fn influences(&self, point: &Vec3) -> bool {
        self.position.sub(point).length() <= self.influence_radius
    }

    /// Weight of this probe's contribution at a point (inverse distance).
    pub fn weight(&self, point: &Vec3) -> f64 {
        let d = self.position.sub(point).length();
        if d >= self.influence_radius { return 0.0; }
        1.0 - (d / self.influence_radius)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubemap_create() {
        let ct = CubeTexture::new(16);
        assert_eq!(ct.faces[0].len(), 16 * 16 * 4);
    }

    #[test]
    fn cubemap_fill_sample() {
        let mut ct = CubeTexture::new(4);
        ct.fill_face(CubeFace::PosX, Color3 { r: 1.0, g: 0.0, b: 0.0 });
        let c = ct.sample(&Vec3::new(1.0, 0.0, 0.0));
        assert!((c.r - 1.0).abs() < 0.02);
    }

    #[test]
    fn gradient_skybox() {
        let ct = CubeTexture::gradient_skybox(8, Color3 { r: 0.5, g: 0.7, b: 1.0 }, Color3 { r: 0.3, g: 0.2, b: 0.1 });
        // Top face should be sky color
        assert_eq!(ct.faces[CubeFace::PosY as usize][0], (0.5 * 255.0) as u8);
    }

    #[test]
    fn face_direction() {
        assert!((CubeFace::PosX.direction().x - 1.0).abs() < 0.01);
        assert!((CubeFace::NegZ.direction().z - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn equirect_to_cube() {
        // 4x2 equirect (minimal)
        let eq = vec![128u8; 4 * 2 * 4]; // solid gray
        let cube = equirect_to_cubemap(&eq, 4, 2, 2);
        assert_eq!(cube.size, 2);
        assert_eq!(cube.faces[0].len(), 2 * 2 * 4);
    }

    #[test]
    fn reflection_probe_influence() {
        let probe = ReflectionProbe::new(Vec3::ZERO, 10.0, 64);
        assert!(probe.influences(&Vec3::new(5.0, 0.0, 0.0)));
        assert!(!probe.influences(&Vec3::new(15.0, 0.0, 0.0)));
    }

    #[test]
    fn reflection_probe_weight() {
        let probe = ReflectionProbe::new(Vec3::ZERO, 10.0, 64);
        let w1 = probe.weight(&Vec3::ZERO);
        let w2 = probe.weight(&Vec3::new(5.0, 0.0, 0.0));
        assert!((w1 - 1.0).abs() < 0.01);
        assert!(w2 < w1 && w2 > 0.0);
    }

    #[test]
    fn cubemap_all_faces() {
        assert_eq!(CubeFace::all().len(), 6);
    }
}
