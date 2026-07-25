//! **Material3D** — Three.js-equivalent material system.
//!
//! PBR (Physically Based Rendering), Phong, Lambert, Standard materials.
//! Texture definitions, UV mapping, shader programs (vertex/fragment),
//! alpha blending, face culling, depth testing.

use super::scene3d::Color3;

// ══════════════════════════════════════════════════════════════════════════════
// Texture
// ══════════════════════════════════════════════════════════════════════════════

/// Texture sampling modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFilter {
    Nearest,
    Linear,
    NearestMipmapNearest,
    LinearMipmapLinear,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureWrap {
    Repeat,
    ClampToEdge,
    MirroredRepeat,
}

/// Texture definition (data is stored externally in a texture manager).
#[derive(Debug, Clone)]
pub struct Texture {
    pub id: u64,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub filter_min: TextureFilter,
    pub filter_mag: TextureFilter,
    pub wrap_s: TextureWrap,
    pub wrap_t: TextureWrap,
    pub flip_y: bool,
    pub generate_mipmaps: bool,
}

impl Texture {
    pub fn new(id: u64, name: &str, width: u32, height: u32) -> Self {
        Texture {
            id, name: name.into(), width, height,
            filter_min: TextureFilter::LinearMipmapLinear,
            filter_mag: TextureFilter::Linear,
            wrap_s: TextureWrap::Repeat,
            wrap_t: TextureWrap::Repeat,
            flip_y: true,
            generate_mipmaps: true,
        }
    }
}

/// Texture data (RGBA pixels).
#[derive(Debug, Clone)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA 4 bytes per pixel
}

impl TextureData {
    pub fn solid_color(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        let pixel_count = (width * height) as usize;
        let mut pixels = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&[r, g, b, a]);
        }
        TextureData { width, height, pixels }
    }

    pub fn checkerboard(width: u32, height: u32, tile_size: u32) -> Self {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_dark = ((x / tile_size) + (y / tile_size)) % 2 == 0;
                let c = if is_dark { 64 } else { 192 };
                pixels.extend_from_slice(&[c, c, c, 255]);
            }
        }
        TextureData { width, height, pixels }
    }

    /// Sample texture at UV coordinates (bilinear).
    pub fn sample(&self, u: f64, v: f64) -> Color3 {
        let x = ((u.fract() + 1.0).fract() * self.width as f64).min(self.width as f64 - 1.0) as u32;
        let y = ((v.fract() + 1.0).fract() * self.height as f64).min(self.height as f64 - 1.0) as u32;
        let idx = ((y * self.width + x) * 4) as usize;
        if idx + 2 < self.pixels.len() {
            Color3::new(
                self.pixels[idx] as f64 / 255.0,
                self.pixels[idx + 1] as f64 / 255.0,
                self.pixels[idx + 2] as f64 / 255.0,
            )
        } else {
            Color3::BLACK
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Material types
// ══════════════════════════════════════════════════════════════════════════════

/// Material blend mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
    Additive,
    Multiply,
}

/// Face culling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CullFace {
    None,
    Front,
    Back,
}

/// Common material properties.
#[derive(Debug, Clone)]
pub struct MaterialBase {
    pub id: u64,
    pub name: String,
    pub blend_mode: BlendMode,
    pub cull_face: CullFace,
    pub depth_test: bool,
    pub depth_write: bool,
    pub opacity: f64,
    pub visible: bool,
    pub wireframe: bool,
    pub side_double: bool,
}

impl MaterialBase {
    pub fn new(id: u64, name: &str) -> Self {
        MaterialBase {
            id, name: name.into(),
            blend_mode: BlendMode::Opaque,
            cull_face: CullFace::Back,
            depth_test: true,
            depth_write: true,
            opacity: 1.0,
            visible: true,
            wireframe: false,
            side_double: false,
        }
    }
}

/// Material types (like Three.js MeshStandardMaterial, MeshPhongMaterial, etc.).
#[derive(Debug, Clone)]
pub enum Material {
    /// Physically Based Rendering material (metallic-roughness workflow).
    Standard {
        base: MaterialBase,
        color: Color3,
        metalness: f64,
        roughness: f64,
        emissive: Color3,
        emissive_intensity: f64,
        normal_scale: f64,
        ao_intensity: f64,
        color_map: Option<u64>,
        normal_map: Option<u64>,
        metalness_map: Option<u64>,
        roughness_map: Option<u64>,
        ao_map: Option<u64>,
        emissive_map: Option<u64>,
    },
    /// Phong shading (specular highlight).
    Phong {
        base: MaterialBase,
        color: Color3,
        specular: Color3,
        shininess: f64,
        emissive: Color3,
        color_map: Option<u64>,
        normal_map: Option<u64>,
    },
    /// Lambert shading (diffuse only, no specular).
    Lambert {
        base: MaterialBase,
        color: Color3,
        emissive: Color3,
        color_map: Option<u64>,
    },
    /// Flat color, no lighting.
    Basic {
        base: MaterialBase,
        color: Color3,
        color_map: Option<u64>,
    },
    /// Custom shader material.
    Shader {
        base: MaterialBase,
        vertex_shader: String,
        fragment_shader: String,
        uniforms: Vec<ShaderUniform>,
    },
    /// Line material.
    Line {
        base: MaterialBase,
        color: Color3,
        line_width: f64,
    },
    /// Point cloud material.
    Points {
        base: MaterialBase,
        color: Color3,
        point_size: f64,
    },
}

/// Shader uniform value.
#[derive(Debug, Clone)]
pub enum ShaderUniform {
    Float(String, f64),
    Vec3(String, [f64; 3]),
    Vec4(String, [f64; 4]),
    Mat4(String, [f64; 16]),
    Int(String, i32),
    Texture(String, u64),
}

// ══════════════════════════════════════════════════════════════════════════════
// Material factory functions
// ══════════════════════════════════════════════════════════════════════════════

/// Create a standard PBR material.
pub fn standard_material(id: u64, name: &str, color: Color3) -> Material {
    Material::Standard {
        base: MaterialBase::new(id, name),
        color, metalness: 0.0, roughness: 0.5,
        emissive: Color3::BLACK, emissive_intensity: 0.0,
        normal_scale: 1.0, ao_intensity: 1.0,
        color_map: None, normal_map: None, metalness_map: None,
        roughness_map: None, ao_map: None, emissive_map: None,
    }
}

/// Create a Phong material.
pub fn phong_material(id: u64, name: &str, color: Color3, shininess: f64) -> Material {
    Material::Phong {
        base: MaterialBase::new(id, name),
        color, specular: Color3::WHITE, shininess,
        emissive: Color3::BLACK, color_map: None, normal_map: None,
    }
}

/// Create a basic (unlit) material.
pub fn basic_material(id: u64, name: &str, color: Color3) -> Material {
    Material::Basic {
        base: MaterialBase::new(id, name),
        color, color_map: None,
    }
}

/// Get the base properties of any material.
pub fn material_base(mat: &Material) -> &MaterialBase {
    match mat {
        Material::Standard { base, .. } => base,
        Material::Phong { base, .. } => base,
        Material::Lambert { base, .. } => base,
        Material::Basic { base, .. } => base,
        Material::Shader { base, .. } => base,
        Material::Line { base, .. } => base,
        Material::Points { base, .. } => base,
    }
}

/// Compute Phong lighting for a single light.
pub fn compute_phong(
    normal: &super::scene3d::Vec3,
    light_dir: &super::scene3d::Vec3,
    view_dir: &super::scene3d::Vec3,
    color: &Color3,
    specular: &Color3,
    shininess: f64,
    light_color: &Color3,
    light_intensity: f64,
) -> Color3 {
    let n_dot_l = normal.dot(light_dir).max(0.0);
    let diffuse = color.multiply(light_color).scale(n_dot_l * light_intensity);

    let reflect = super::scene3d::Vec3 {
        x: 2.0 * n_dot_l * normal.x - light_dir.x,
        y: 2.0 * n_dot_l * normal.y - light_dir.y,
        z: 2.0 * n_dot_l * normal.z - light_dir.z,
    };
    let spec_factor = reflect.dot(view_dir).max(0.0).powf(shininess);
    let spec = specular.multiply(light_color).scale(spec_factor * light_intensity);

    diffuse.add(&spec)
}

/// Compute PBR (Cook-Torrance) BRDF approximation.
pub fn compute_pbr(
    normal: &super::scene3d::Vec3,
    light_dir: &super::scene3d::Vec3,
    view_dir: &super::scene3d::Vec3,
    albedo: &Color3,
    metalness: f64,
    roughness: f64,
    light_color: &Color3,
    light_intensity: f64,
) -> Color3 {
    let n_dot_l = normal.dot(light_dir).max(0.0);
    let n_dot_v = normal.dot(view_dir).max(0.0);
    let h = light_dir.add(view_dir).normalize();
    let n_dot_h = normal.dot(&h).max(0.0);

    // Roughness squared
    let a = roughness * roughness;
    let a2 = a * a;

    // GGX Normal Distribution
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let ndf = a2 / (std::f64::consts::PI * denom * denom);

    // Schlick-GGX Geometry
    let k = (roughness + 1.0).powi(2) / 8.0;
    let g1_l = n_dot_l / (n_dot_l * (1.0 - k) + k);
    let g1_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let g = g1_l * g1_v;

    // Fresnel (Schlick)
    let f0 = Color3::new(0.04, 0.04, 0.04).scale(1.0 - metalness).add(&albedo.scale(metalness));
    let fresnel = Color3::new(
        f0.r + (1.0 - f0.r) * (1.0 - n_dot_h).powi(5),
        f0.g + (1.0 - f0.g) * (1.0 - n_dot_h).powi(5),
        f0.b + (1.0 - f0.b) * (1.0 - n_dot_h).powi(5),
    );

    let denom_brdf = (4.0 * n_dot_l * n_dot_v).max(0.001);
    let specular = Color3::new(
        fresnel.r * ndf * g / denom_brdf,
        fresnel.g * ndf * g / denom_brdf,
        fresnel.b * ndf * g / denom_brdf,
    );

    let ks = fresnel;
    let kd = Color3::new(1.0 - ks.r, 1.0 - ks.g, 1.0 - ks.b).scale(1.0 - metalness);
    let diffuse = Color3::new(kd.r * albedo.r, kd.g * albedo.g, kd.b * albedo.b)
        .scale(1.0 / std::f64::consts::PI);

    diffuse.add(&specular).multiply(light_color).scale(n_dot_l * light_intensity)
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::scene3d::Vec3;

    #[test]
    fn basic_material_creation() {
        let mat = basic_material(1, "test", Color3::RED);
        let base = material_base(&mat);
        assert_eq!(base.name, "test");
        assert!(!base.wireframe);
    }

    #[test]
    fn standard_material_creation() {
        let mat = standard_material(1, "pbr", Color3::WHITE);
        if let Material::Standard { metalness, roughness, .. } = &mat {
            assert_eq!(*metalness, 0.0);
            assert_eq!(*roughness, 0.5);
        } else { panic!("Expected Standard"); }
    }

    #[test]
    fn phong_material_creation() {
        let mat = phong_material(1, "shiny", Color3::BLUE, 64.0);
        if let Material::Phong { shininess, .. } = &mat {
            assert_eq!(*shininess, 64.0);
        } else { panic!("Expected Phong"); }
    }

    #[test]
    fn texture_creation() {
        let tex = Texture::new(1, "diffuse", 512, 512);
        assert_eq!(tex.width, 512);
        assert!(tex.flip_y);
    }

    #[test]
    fn texture_data_solid() {
        let td = TextureData::solid_color(2, 2, 255, 0, 0, 255);
        assert_eq!(td.pixels.len(), 16);
        let c = td.sample(0.0, 0.0);
        assert!((c.r - 1.0).abs() < 0.01);
    }

    #[test]
    fn texture_checkerboard() {
        let td = TextureData::checkerboard(4, 4, 2);
        assert_eq!(td.pixels.len(), 64);
    }

    #[test]
    fn phong_lighting() {
        let n = Vec3::UP;
        let l = Vec3::new(0.0, 1.0, 0.0);
        let v = Vec3::new(0.0, 1.0, 0.0);
        let result = compute_phong(&n, &l, &v, &Color3::WHITE, &Color3::WHITE, 32.0, &Color3::WHITE, 1.0);
        assert!(result.r > 0.0);
    }

    #[test]
    fn pbr_lighting() {
        let n = Vec3::UP;
        let l = Vec3::new(0.0, 1.0, 0.0).normalize();
        let v = Vec3::new(0.0, 1.0, 0.5).normalize();
        let result = compute_pbr(&n, &l, &v, &Color3::WHITE, 0.0, 0.5, &Color3::WHITE, 1.0);
        assert!(result.r > 0.0);
    }

    #[test]
    fn material_blend_modes() {
        let mut base = MaterialBase::new(1, "test");
        base.blend_mode = BlendMode::AlphaBlend;
        base.opacity = 0.5;
        assert_eq!(base.opacity, 0.5);
    }

    #[test]
    fn shader_uniforms() {
        let mat = Material::Shader {
            base: MaterialBase::new(1, "custom"),
            vertex_shader: "void main() {}".into(),
            fragment_shader: "void main() {}".into(),
            uniforms: vec![
                ShaderUniform::Float("time".into(), 0.0),
                ShaderUniform::Vec3("lightPos".into(), [0.0, 10.0, 0.0]),
            ],
        };
        if let Material::Shader { uniforms, .. } = &mat {
            assert_eq!(uniforms.len(), 2);
        }
    }
}
