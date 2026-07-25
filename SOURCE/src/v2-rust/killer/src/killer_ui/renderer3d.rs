//! **Renderer3D** — Software 3D renderer with z-buffer, rasterization, and shading.
//!
//! Projects 3D scene to 2D framebuffer with:
//! - Vertex transformation (model → view → projection → screen)
//! - Triangle rasterization with edge functions
//! - Depth buffer (z-buffer)
//! - Per-pixel Phong/PBR shading
//! - Shadow mapping (single directional light)
//! - Post-processing (bloom, gamma, tone mapping)
//! - LOD (Level of Detail) and frustum culling

use super::scene3d::*;
use super::geometry3d::*;
use super::material3d::*;

// ══════════════════════════════════════════════════════════════════════════════
// Render target
// ══════════════════════════════════════════════════════════════════════════════

/// RGBA framebuffer + depth buffer for software rendering.
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub color: Vec<u8>,  // RGBA, 4 bytes per pixel
    pub depth: Vec<f64>,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        RenderTarget {
            width, height,
            color: vec![0; size * 4],
            depth: vec![f64::MAX; size],
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        let size = (self.width * self.height) as usize;
        for i in 0..size {
            self.color[i * 4] = r;
            self.color[i * 4 + 1] = g;
            self.color[i * 4 + 2] = b;
            self.color[i * 4 + 3] = 255;
            self.depth[i] = f64::MAX;
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height { return; }
        let idx = ((y * self.width + x) * 4) as usize;
        self.color[idx] = r;
        self.color[idx + 1] = g;
        self.color[idx + 2] = b;
        self.color[idx + 3] = 255;
    }

    pub fn test_depth(&mut self, x: u32, y: u32, z: f64) -> bool {
        if x >= self.width || y >= self.height { return false; }
        let idx = (y * self.width + x) as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            true
        } else {
            false
        }
    }

    pub fn pixel_count(&self) -> usize { (self.width * self.height) as usize }
}

// ══════════════════════════════════════════════════════════════════════════════
// Render pipeline
// ══════════════════════════════════════════════════════════════════════════════

/// Render configuration.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub wireframe: bool,
    pub backface_cull: bool,
    pub depth_test: bool,
    pub enable_shadows: bool,
    pub shadow_map_size: u32,
    pub enable_bloom: bool,
    pub bloom_threshold: f64,
    pub bloom_intensity: f64,
    pub gamma: f64,
    pub exposure: f64,
    pub ambient_light: Color3,
    pub ambient_intensity: f64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            wireframe: false,
            backface_cull: true,
            depth_test: true,
            enable_shadows: false,
            shadow_map_size: 512,
            enable_bloom: false,
            bloom_threshold: 1.0,
            bloom_intensity: 0.3,
            gamma: 2.2,
            exposure: 1.0,
            ambient_light: Color3::WHITE,
            ambient_intensity: 0.1,
        }
    }
}

/// Transformed vertex ready for rasterization.
#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f64,
    y: f64,
    z: f64,       // depth
    nx: f64,
    ny: f64,
    nz: f64,
    #[allow(dead_code)]
    u: f64,
    #[allow(dead_code)]
    v: f64,
    wx: f64,      // world position for lighting
    wy: f64,
    wz: f64,
}

/// Render a mesh to the render target.
pub fn render_mesh(
    target: &mut RenderTarget,
    geometry: &BufferGeometry,
    material: &Material,
    model_matrix: &Mat4,
    view_matrix: &Mat4,
    proj_matrix: &Mat4,
    config: &RenderConfig,
    lights: &[(Vec3, Color3, f64)], // (direction, color, intensity)
    camera_pos: &Vec3,
) -> u32 {
    let mvp = proj_matrix.multiply(view_matrix).multiply(model_matrix);
    let w = target.width as f64;
    let h = target.height as f64;

    // Transform vertices
    let screen_verts: Vec<ScreenVertex> = geometry.vertices.iter().map(|vert| {
        let world = model_matrix.transform_point(&vert.position);
        let clip = mvp.transform_point(&vert.position);
        // NDC to screen
        let sx = (clip.x * 0.5 + 0.5) * w;
        let sy = (1.0 - (clip.y * 0.5 + 0.5)) * h;
        // Transform normal by model matrix (ignoring translation)
        let wn = Vec3::new(
            model_matrix.m[0] * vert.normal.x + model_matrix.m[4] * vert.normal.y + model_matrix.m[8] * vert.normal.z,
            model_matrix.m[1] * vert.normal.x + model_matrix.m[5] * vert.normal.y + model_matrix.m[9] * vert.normal.z,
            model_matrix.m[2] * vert.normal.x + model_matrix.m[6] * vert.normal.y + model_matrix.m[10] * vert.normal.z,
        ).normalize();
        ScreenVertex {
            x: sx, y: sy, z: clip.z,
            nx: wn.x, ny: wn.y, nz: wn.z,
            u: vert.uv[0], v: vert.uv[1],
            wx: world.x, wy: world.y, wz: world.z,
        }
    }).collect();

    let mut triangles_drawn = 0u32;

    for tri in &geometry.indices {
        let v0 = &screen_verts[tri.0 as usize];
        let v1 = &screen_verts[tri.1 as usize];
        let v2 = &screen_verts[tri.2 as usize];

        // Backface culling
        if config.backface_cull {
            let edge1x = v1.x - v0.x;
            let edge1y = v1.y - v0.y;
            let edge2x = v2.x - v0.x;
            let edge2y = v2.y - v0.y;
            let cross = edge1x * edge2y - edge1y * edge2x;
            if cross < 0.0 { continue; }
        }

        // Bounding box
        let min_x = v0.x.min(v1.x).min(v2.x).max(0.0) as u32;
        let max_x = v0.x.max(v1.x).max(v2.x).min(w - 1.0) as u32;
        let min_y = v0.y.min(v1.y).min(v2.y).max(0.0) as u32;
        let max_y = v0.y.max(v1.y).max(v2.y).min(h - 1.0) as u32;

        if min_x > max_x || min_y > max_y { continue; }

        // Edge function rasterization
        let area = edge_function(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
        if area.abs() < 1e-6 { continue; }
        let inv_area = 1.0 / area;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let cx = px as f64 + 0.5;
                let cy = py as f64 + 0.5;

                let w0 = edge_function(v1.x, v1.y, v2.x, v2.y, cx, cy) * inv_area;
                let w1 = edge_function(v2.x, v2.y, v0.x, v0.y, cx, cy) * inv_area;
                let w2 = 1.0 - w0 - w1;

                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 { continue; }

                // Interpolate depth
                let z = w0 * v0.z + w1 * v1.z + w2 * v2.z;

                if config.depth_test && !target.test_depth(px, py, z) { continue; }

                // Interpolate normal
                let normal = Vec3::new(
                    w0 * v0.nx + w1 * v1.nx + w2 * v2.nx,
                    w0 * v0.ny + w1 * v1.ny + w2 * v2.ny,
                    w0 * v0.nz + w1 * v1.nz + w2 * v2.nz,
                ).normalize();

                // Interpolate world position
                let frag_pos = Vec3::new(
                    w0 * v0.wx + w1 * v1.wx + w2 * v2.wx,
                    w0 * v0.wy + w1 * v1.wy + w2 * v2.wy,
                    w0 * v0.wz + w1 * v1.wz + w2 * v2.wz,
                );

                let view_dir = camera_pos.sub(&frag_pos).normalize();

                // Shade the pixel
                let color = shade_pixel(material, &normal, &view_dir, lights, config);
                let (r, g, b) = apply_tone_mapping(&color, config.gamma, config.exposure);
                target.set_pixel(px, py, r, g, b);
            }
        }
        triangles_drawn += 1;
    }

    triangles_drawn
}

fn edge_function(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> f64 {
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
}

fn shade_pixel(
    material: &Material,
    normal: &Vec3,
    view_dir: &Vec3,
    lights: &[(Vec3, Color3, f64)],
    config: &RenderConfig,
) -> Color3 {
    let ambient = config.ambient_light.scale(config.ambient_intensity);
    match material {
        Material::Basic { color, .. } => *color,
        Material::Lambert { color, emissive, .. } => {
            let mut result = ambient.multiply(color);
            for (dir, lcol, intensity) in lights {
                let n_dot_l = normal.dot(dir).max(0.0);
                result = result.add(&color.multiply(lcol).scale(n_dot_l * intensity));
            }
            result.add(emissive)
        }
        Material::Phong { color, specular, shininess, emissive, .. } => {
            let mut result = ambient.multiply(color);
            for (dir, lcol, intensity) in lights {
                result = result.add(&compute_phong(normal, dir, view_dir, color, specular, *shininess, lcol, *intensity));
            }
            result.add(emissive)
        }
        Material::Standard { color, metalness, roughness, emissive, emissive_intensity, .. } => {
            let mut result = ambient.multiply(color);
            for (dir, lcol, intensity) in lights {
                result = result.add(&compute_pbr(normal, dir, view_dir, color, *metalness, *roughness, lcol, *intensity));
            }
            result.add(&emissive.scale(*emissive_intensity))
        }
        _ => Color3::new(1.0, 0.0, 1.0), // magenta = missing shader
    }
}

fn apply_tone_mapping(color: &Color3, gamma: f64, exposure: f64) -> (u8, u8, u8) {
    // Reinhard tone mapping
    let inv_gamma = 1.0 / gamma;
    let r = (color.r * exposure / (color.r * exposure + 1.0)).powf(inv_gamma);
    let g = (color.g * exposure / (color.g * exposure + 1.0)).powf(inv_gamma);
    let b = (color.b * exposure / (color.b * exposure + 1.0)).powf(inv_gamma);
    ((r * 255.0).clamp(0.0, 255.0) as u8,
     (g * 255.0).clamp(0.0, 255.0) as u8,
     (b * 255.0).clamp(0.0, 255.0) as u8)
}

// ══════════════════════════════════════════════════════════════════════════════
// Shadow map
// ══════════════════════════════════════════════════════════════════════════════

/// Shadow map for a single directional light.
pub struct ShadowMap {
    pub size: u32,
    pub depth: Vec<f64>,
    pub light_matrix: Mat4,
}

impl ShadowMap {
    pub fn new(size: u32) -> Self {
        ShadowMap {
            size,
            depth: vec![f64::MAX; (size * size) as usize],
            light_matrix: Mat4::IDENTITY,
        }
    }

    /// Clear shadow map.
    pub fn clear(&mut self) {
        for d in &mut self.depth { *d = f64::MAX; }
    }

    /// Test if a world-space point is in shadow.
    pub fn is_shadowed(&self, world_pos: &Vec3, bias: f64) -> bool {
        let lp = self.light_matrix.transform_point(world_pos);
        let sx = ((lp.x * 0.5 + 0.5) * self.size as f64) as u32;
        let sy = ((lp.y * 0.5 + 0.5) * self.size as f64) as u32;
        if sx >= self.size || sy >= self.size { return false; }
        let idx = (sy * self.size + sx) as usize;
        lp.z - bias > self.depth[idx]
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Post-processing effects
// ══════════════════════════════════════════════════════════════════════════════

/// Apply bloom post-process to a render target.
pub fn post_bloom(target: &mut RenderTarget, threshold: f64, intensity: f64) {
    let size = target.pixel_count();
    let mut bright: Vec<(f64, f64, f64)> = Vec::with_capacity(size);

    // Extract bright pixels
    for i in 0..size {
        let r = target.color[i * 4] as f64 / 255.0;
        let g = target.color[i * 4 + 1] as f64 / 255.0;
        let b = target.color[i * 4 + 2] as f64 / 255.0;
        let lum = r * 0.2126 + g * 0.7152 + b * 0.0722;
        if lum > threshold {
            bright.push((r, g, b));
        } else {
            bright.push((0.0, 0.0, 0.0));
        }
    }

    // Simple box blur (single pass for performance)
    let w = target.width as i32;
    let h = target.height as i32;
    let radius = 2;
    for y in 0..h {
        for x in 0..w {
            let mut sr = 0.0;
            let mut sg = 0.0;
            let mut sb = 0.0;
            let mut count = 0.0;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= 0 && nx < w && ny >= 0 && ny < h {
                        let idx = (ny * w + nx) as usize;
                        sr += bright[idx].0;
                        sg += bright[idx].1;
                        sb += bright[idx].2;
                        count += 1.0;
                    }
                }
            }
            let idx = ((y * w + x) * 4) as usize;
            let bloom_r = sr / count * intensity;
            let bloom_g = sg / count * intensity;
            let bloom_b = sb / count * intensity;
            target.color[idx] = ((target.color[idx] as f64 / 255.0 + bloom_r).min(1.0) * 255.0) as u8;
            target.color[idx + 1] = ((target.color[idx + 1] as f64 / 255.0 + bloom_g).min(1.0) * 255.0) as u8;
            target.color[idx + 2] = ((target.color[idx + 2] as f64 / 255.0 + bloom_b).min(1.0) * 255.0) as u8;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Scene renderer — high-level render function
// ══════════════════════════════════════════════════════════════════════════════

/// Render statistics.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub triangles_drawn: u32,
    pub meshes_rendered: u32,
    pub pixels_shaded: u64,
    pub draw_calls: u32,
}

/// Render an entire scene to a target.
pub fn render_scene(
    target: &mut RenderTarget,
    scene: &Scene,
    geometries: &[(u64, BufferGeometry)],
    materials: &[(u64, Material)],
    config: &RenderConfig,
) -> RenderStats {
    let mut stats = RenderStats::default();

    target.clear(0, 0, 0);

    // Apply background
    match &scene.background {
        Background::Color(c) => {
            let (r, g, b) = c.to_rgb8();
            target.clear(r, g, b);
        }
        Background::Gradient { top, bottom } => {
            for y in 0..target.height {
                let t = y as f64 / target.height as f64;
                let c = top.scale(1.0 - t).add(&bottom.scale(t));
                let (r, g, b) = c.to_rgb8();
                for x in 0..target.width {
                    target.set_pixel(x, y, r, g, b);
                }
            }
        }
        _ => {}
    }

    let view = scene.view_matrix();
    let proj = scene.projection_matrix();
    let cam_pos = scene.camera().map(|c| c.position).unwrap_or(Vec3::ZERO);

    // Collect lights
    let lights: Vec<(Vec3, Color3, f64)> = scene.lights().iter().filter_map(|obj| {
        match &obj.kind {
            Object3DKind::Light(LightKind::Directional { color, intensity, direction }) =>
                Some((direction.scale(-1.0).normalize(), *color, *intensity)),
            Object3DKind::Light(LightKind::Point { color, intensity, .. }) =>
                Some((obj.position.normalize(), *color, *intensity)),
            _ => None,
        }
    }).collect();

    // Render each mesh
    for obj in scene.meshes() {
        if !obj.visible { continue; }
        if let Object3DKind::Mesh { geometry_id, material_id } = &obj.kind {
            let geo = geometries.iter().find(|(id, _)| id == geometry_id).map(|(_, g)| g);
            let mat = materials.iter().find(|(id, _)| id == material_id).map(|(_, m)| m);
            if let (Some(geo), Some(mat)) = (geo, mat) {
                let model = scene.world_matrix(obj.id);
                let tris = render_mesh(target, geo, mat, &model, &view, &proj, config, &lights, &cam_pos);
                stats.triangles_drawn += tris;
                stats.meshes_rendered += 1;
                stats.draw_calls += 1;
            }
        }
    }

    if config.enable_bloom {
        post_bloom(target, config.bloom_threshold, config.bloom_intensity);
    }

    stats
}

// ══════════════════════════════════════════════════════════════════════════════
// LOD (Level of Detail)
// ══════════════════════════════════════════════════════════════════════════════

/// LOD level with a distance threshold.
#[derive(Debug, Clone)]
pub struct LodLevel {
    pub distance: f64,
    pub geometry_id: u64,
}

/// LOD group — selects geometry based on camera distance.
pub struct LodGroup {
    pub levels: Vec<LodLevel>,
}

impl LodGroup {
    pub fn new() -> Self { LodGroup { levels: Vec::new() } }

    pub fn add_level(mut self, distance: f64, geometry_id: u64) -> Self {
        self.levels.push(LodLevel { distance, geometry_id });
        self.levels.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        self
    }

    pub fn select(&self, camera_distance: f64) -> Option<u64> {
        for level in self.levels.iter().rev() {
            if camera_distance >= level.distance {
                return Some(level.geometry_id);
            }
        }
        self.levels.first().map(|l| l.geometry_id)
    }
}

impl Default for LodGroup {
    fn default() -> Self { Self::new() }
}

// ══════════════════════════════════════════════════════════════════════════════
// Instanced rendering
// ══════════════════════════════════════════════════════════════════════════════

/// An instance of a mesh with its own transform.
#[derive(Debug, Clone)]
pub struct MeshInstance {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color_tint: Option<Color3>,
}

/// Render many instances of the same mesh.
pub fn render_instanced(
    target: &mut RenderTarget,
    geometry: &BufferGeometry,
    material: &Material,
    instances: &[MeshInstance],
    view_matrix: &Mat4,
    proj_matrix: &Mat4,
    config: &RenderConfig,
    lights: &[(Vec3, Color3, f64)],
    camera_pos: &Vec3,
) -> u32 {
    let mut total = 0;
    for inst in instances {
        let model = Mat4::compose(&inst.position, &inst.rotation, &inst.scale);
        total += render_mesh(target, geometry, material, &model, view_matrix, proj_matrix, config, lights, camera_pos);
    }
    total
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_basic() {
        let mut rt = RenderTarget::new(100, 100);
        rt.clear(128, 128, 128);
        assert_eq!(rt.pixel_count(), 10000);
        assert_eq!(rt.color[0], 128);
    }

    #[test]
    fn depth_buffer() {
        let mut rt = RenderTarget::new(10, 10);
        assert!(rt.test_depth(5, 5, 0.5));
        assert!(!rt.test_depth(5, 5, 0.8));
        assert!(rt.test_depth(5, 5, 0.3));
    }

    #[test]
    fn render_triangle() {
        let mut rt = RenderTarget::new(64, 64);
        rt.clear(0, 0, 0);
        let geo = plane_geometry(2.0, 2.0, 1, 1);
        let mat = basic_material(1, "flat", Color3::RED);
        let model = Mat4::IDENTITY;
        let view = Mat4::look_at(
            &Vec3::new(0.0, 2.0, 2.0),
            &Vec3::ZERO,
            &Vec3::UP,
        );
        let proj = Mat4::perspective(1.0, 1.0, 0.1, 100.0);
        let config = RenderConfig::default();
        let lights = vec![(Vec3::new(0.0, 1.0, 0.0), Color3::WHITE, 1.0)];
        let tris = render_mesh(&mut rt, &geo, &mat, &model, &view, &proj, &config, &lights, &Vec3::new(0.0, 2.0, 2.0));
        assert!(tris > 0);
    }

    #[test]
    fn render_scene_basic() {
        let mut scene = Scene::new("test");
        let cam = scene.add_perspective_camera("cam", 1.0, 1.0, 0.1, 100.0);
        scene.get_mut(cam).unwrap().position = Vec3::new(0.0, 2.0, 5.0);
        scene.add_light("sun", LightKind::Directional {
            color: Color3::WHITE, intensity: 1.0,
            direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
        });
        let _mesh = scene.add_mesh("cube", 100, 200);

        let geo = box_geometry(1.0, 1.0, 1.0);
        let mat = standard_material(200, "pbr_red", Color3::RED);
        let geometries = vec![(100u64, geo)];
        let materials = vec![(200u64, mat)];

        let mut target = RenderTarget::new(32, 32);
        let config = RenderConfig::default();
        let stats = render_scene(&mut target, &scene, &geometries, &materials, &config);
        assert_eq!(stats.meshes_rendered, 1);
    }

    #[test]
    fn shadow_map_basic() {
        let mut sm = ShadowMap::new(64);
        sm.clear();
        assert_eq!(sm.depth.len(), 64 * 64);
        // Without proper data, nothing is shadowed
        assert!(!sm.is_shadowed(&Vec3::ZERO, 0.001));
    }

    #[test]
    fn lod_selection() {
        let lod = LodGroup::new()
            .add_level(0.0, 100)   // high detail
            .add_level(50.0, 101)  // medium
            .add_level(100.0, 102); // low
        assert_eq!(lod.select(10.0), Some(100));
        assert_eq!(lod.select(75.0), Some(101));
        assert_eq!(lod.select(150.0), Some(102));
    }

    #[test]
    fn instanced_rendering() {
        let geo = box_geometry(1.0, 1.0, 1.0);
        let mat = basic_material(1, "flat", Color3::GREEN);
        let instances = vec![
            MeshInstance { position: Vec3::new(-2.0, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE, color_tint: None },
            MeshInstance { position: Vec3::new(2.0, 0.0, 0.0), rotation: Quat::IDENTITY, scale: Vec3::ONE, color_tint: None },
        ];
        let mut target = RenderTarget::new(32, 32);
        target.clear(0, 0, 0);
        let view = Mat4::look_at(&Vec3::new(0.0, 5.0, 10.0), &Vec3::ZERO, &Vec3::UP);
        let proj = Mat4::perspective(1.0, 1.0, 0.1, 100.0);
        let config = RenderConfig::default();
        let total = render_instanced(&mut target, &geo, &mat, &instances, &view, &proj, &config, &[], &Vec3::new(0.0, 5.0, 10.0));
        assert!(total > 0);
    }

    #[test]
    fn tone_mapping() {
        let (r, g, b) = apply_tone_mapping(&Color3::new(2.0, 1.0, 0.5), 2.2, 1.0);
        assert!(r > g);
        assert!(g > b);
        assert!(r > 0);
    }

    #[test]
    fn gradient_background() {
        let mut scene = Scene::new("test");
        scene.background = Background::Gradient { top: Color3::BLUE, bottom: Color3::WHITE };
        let cam = scene.add_perspective_camera("cam", 1.0, 1.0, 0.1, 100.0);
        scene.get_mut(cam).unwrap().position = Vec3::new(0.0, 0.0, 5.0);
        let mut target = RenderTarget::new(8, 8);
        let stats = render_scene(&mut target, &scene, &[], &[], &RenderConfig::default());
        assert_eq!(stats.meshes_rendered, 0);
        // Top should be bluish, bottom whitish
        assert!(target.color[2] > target.color[0]); // B > R at top
    }
}
