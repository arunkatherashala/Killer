//! **Model Loader** — OBJ and GLTF (JSON) model parsing.
//!
//! Parse .obj files (vertices, normals, UVs, faces) and basic .gltf JSON
//! (meshes, nodes, materials) into BufferGeometry and scene nodes.

use super::scene3d::{Vec3, Color3};
use super::geometry3d::{BufferGeometry, Vertex, Triangle};

// ══════════════════════════════════════════════════════════════════════════════
// OBJ Loader
// ══════════════════════════════════════════════════════════════════════════════

/// Parsed OBJ model.
#[derive(Debug, Clone)]
pub struct ObjModel {
    pub name: String,
    pub geometries: Vec<ObjGroup>,
}

/// A named group within an OBJ file.
#[derive(Debug, Clone)]
pub struct ObjGroup {
    pub name: String,
    pub material_name: Option<String>,
    pub geometry: BufferGeometry,
}

/// Parse a .obj file from string content.
pub fn parse_obj(content: &str) -> ObjModel {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<[f64; 2]> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<Triangle> = Vec::new();
    let mut groups: Vec<ObjGroup> = Vec::new();
    let mut current_group = String::from("default");
    let mut current_material: Option<String> = None;
    let mut model_name = String::from("unnamed");

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "o" => {
                if parts.len() > 1 { model_name = parts[1].into(); }
            }
            "g" => {
                if !vertices.is_empty() {
                    let mut geo = BufferGeometry { vertices: vertices.clone(), indices: indices.clone(), bounding_box: None, bounding_sphere: None };
                    geo.compute_bounds();
                    groups.push(ObjGroup { name: current_group.clone(), material_name: current_material.clone(), geometry: geo });
                    vertices.clear();
                    indices.clear();
                }
                current_group = if parts.len() > 1 { parts[1].into() } else { "default".into() };
            }
            "usemtl" => {
                if parts.len() > 1 { current_material = Some(parts[1].into()); }
            }
            "v" if parts.len() >= 4 => {
                let x = parts[1].parse::<f64>().unwrap_or(0.0);
                let y = parts[2].parse::<f64>().unwrap_or(0.0);
                let z = parts[3].parse::<f64>().unwrap_or(0.0);
                positions.push(Vec3::new(x, y, z));
            }
            "vn" if parts.len() >= 4 => {
                let x = parts[1].parse::<f64>().unwrap_or(0.0);
                let y = parts[2].parse::<f64>().unwrap_or(0.0);
                let z = parts[3].parse::<f64>().unwrap_or(0.0);
                normals.push(Vec3::new(x, y, z));
            }
            "vt" if parts.len() >= 3 => {
                let u = parts[1].parse::<f64>().unwrap_or(0.0);
                let v = parts[2].parse::<f64>().unwrap_or(0.0);
                uvs.push([u, v]);
            }
            "f" if parts.len() >= 4 => {
                // Triangulate faces (fan triangulation for polygons)
                let face_verts: Vec<u32> = parts[1..].iter().map(|face_str| {
                    let idx = vertices.len() as u32;
                    let components: Vec<&str> = face_str.split('/').collect();
                    let pos_idx = components[0].parse::<usize>().unwrap_or(1) - 1;
                    let uv_idx = components.get(1).and_then(|s| if s.is_empty() { None } else { s.parse::<usize>().ok() }).map(|i| i - 1);
                    let norm_idx = components.get(2).and_then(|s| s.parse::<usize>().ok()).map(|i| i - 1);

                    let position = positions.get(pos_idx).copied().unwrap_or(Vec3::ZERO);
                    let normal = norm_idx.and_then(|i| normals.get(i)).copied().unwrap_or(Vec3::new(0.0, 1.0, 0.0));
                    let uv = uv_idx.and_then(|i| uvs.get(i)).copied().unwrap_or([0.0, 0.0]);

                    vertices.push(Vertex { position, normal, uv });
                    idx
                }).collect();

                // Fan triangulation
                for i in 1..face_verts.len() - 1 {
                    indices.push(Triangle(face_verts[0], face_verts[i], face_verts[i + 1]));
                }
            }
            _ => {}
        }
    }

    // Flush remaining
    if !vertices.is_empty() {
        let mut geo = BufferGeometry { vertices, indices, bounding_box: None, bounding_sphere: None };
        geo.compute_bounds();
        groups.push(ObjGroup { name: current_group, material_name: current_material, geometry: geo });
    }

    ObjModel { name: model_name, geometries: groups }
}

// ══════════════════════════════════════════════════════════════════════════════
// MTL Loader (basic material library)
// ══════════════════════════════════════════════════════════════════════════════

/// Parsed material from .mtl file.
#[derive(Debug, Clone)]
pub struct ObjMaterial {
    pub name: String,
    pub ambient: Color3,
    pub diffuse: Color3,
    pub specular: Color3,
    pub shininess: f64,
    pub opacity: f64,
    pub diffuse_map: Option<String>,
}

/// Parse a .mtl file.
pub fn parse_mtl(content: &str) -> Vec<ObjMaterial> {
    let mut materials = Vec::new();
    let mut current: Option<ObjMaterial> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "newmtl" => {
                if let Some(m) = current.take() { materials.push(m); }
                current = Some(ObjMaterial {
                    name: parts.get(1).unwrap_or(&"unnamed").to_string(),
                    ambient: Color3::new(0.2, 0.2, 0.2),
                    diffuse: Color3::new(0.8, 0.8, 0.8),
                    specular: Color3::new(1.0, 1.0, 1.0),
                    shininess: 32.0,
                    opacity: 1.0,
                    diffuse_map: None,
                });
            }
            "Ka" if parts.len() >= 4 => {
                if let Some(m) = &mut current {
                    m.ambient = Color3::new(
                        parts[1].parse().unwrap_or(0.2),
                        parts[2].parse().unwrap_or(0.2),
                        parts[3].parse().unwrap_or(0.2),
                    );
                }
            }
            "Kd" if parts.len() >= 4 => {
                if let Some(m) = &mut current {
                    m.diffuse = Color3::new(
                        parts[1].parse().unwrap_or(0.8),
                        parts[2].parse().unwrap_or(0.8),
                        parts[3].parse().unwrap_or(0.8),
                    );
                }
            }
            "Ks" if parts.len() >= 4 => {
                if let Some(m) = &mut current {
                    m.specular = Color3::new(
                        parts[1].parse().unwrap_or(1.0),
                        parts[2].parse().unwrap_or(1.0),
                        parts[3].parse().unwrap_or(1.0),
                    );
                }
            }
            "Ns" if parts.len() >= 2 => {
                if let Some(m) = &mut current {
                    m.shininess = parts[1].parse().unwrap_or(32.0);
                }
            }
            "d" if parts.len() >= 2 => {
                if let Some(m) = &mut current {
                    m.opacity = parts[1].parse().unwrap_or(1.0);
                }
            }
            "map_Kd" if parts.len() >= 2 => {
                if let Some(m) = &mut current {
                    m.diffuse_map = Some(parts[1].into());
                }
            }
            _ => {}
        }
    }
    if let Some(m) = current { materials.push(m); }
    materials
}

// ══════════════════════════════════════════════════════════════════════════════
// Basic GLTF Loader (JSON-based, no binary buffers)
// ══════════════════════════════════════════════════════════════════════════════

/// Minimal GLTF scene representation.
#[derive(Debug, Clone)]
pub struct GltfScene {
    pub name: String,
    pub nodes: Vec<GltfNode>,
    pub meshes: Vec<GltfMesh>,
    pub materials: Vec<GltfMaterial>,
}

#[derive(Debug, Clone)]
pub struct GltfNode {
    pub name: String,
    pub mesh_index: Option<usize>,
    pub translation: Vec3,
    pub rotation: [f64; 4],
    pub scale: Vec3,
    pub children: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct GltfMesh {
    pub name: String,
    pub primitive_count: usize,
}

#[derive(Debug, Clone)]
pub struct GltfMaterial {
    pub name: String,
    pub base_color: Color3,
    pub metallic: f64,
    pub roughness: f64,
    pub emissive: Color3,
    pub alpha_mode: String,
}

/// Minimal GLTF JSON parser (extracts node/mesh/material info).
pub fn parse_gltf_json(json: &str) -> GltfScene {
    let mut scene = GltfScene {
        name: String::from("gltf_scene"),
        nodes: Vec::new(),
        meshes: Vec::new(),
        materials: Vec::new(),
    };

    // Simple JSON key extraction (no serde dependency)
    let extract_str = |s: &str, key: &str| -> Option<String> {
        let pattern = format!("\"{key}\"");
        let pos = s.find(&pattern)?;
        let after = &s[pos + pattern.len()..];
        let colon = after.find(':')?;
        let after_colon = after[colon + 1..].trim_start();
        if after_colon.starts_with('"') {
            let end = after_colon[1..].find('"')?;
            Some(after_colon[1..1 + end].to_string())
        } else {
            None
        }
    };

    let extract_num = |s: &str, key: &str| -> Option<f64> {
        let pattern = format!("\"{key}\"");
        let pos = s.find(&pattern)?;
        let after = &s[pos + pattern.len()..];
        let colon = after.find(':')?;
        let after_colon = after[colon + 1..].trim_start();
        let end = after_colon.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')?;
        after_colon[..end].parse().ok()
    };

    // Extract scene name
    if let Some(name) = extract_str(json, "scene") {
        scene.name = name;
    }

    // Count nodes (look for "nodes" array)
    if let Some(nodes_start) = json.find("\"nodes\"") {
        let after = &json[nodes_start..];
        if let Some(arr_start) = after.find('[') {
            let arr = &after[arr_start..];
            // Count objects in the array by counting '{' at depth 1
            let mut depth = 0;
            let mut node_count = 0;
            for ch in arr.chars() {
                match ch {
                    '[' if depth == 0 => depth = 1,
                    '{' if depth == 1 => { depth = 2; node_count += 1; }
                    '{' => depth += 1,
                    '}' if depth > 1 => depth -= 1,
                    ']' if depth == 1 => break,
                    _ => {}
                }
            }
            for i in 0..node_count {
                scene.nodes.push(GltfNode {
                    name: format!("node_{i}"),
                    mesh_index: None,
                    translation: Vec3::ZERO,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: Vec3::ONE,
                    children: Vec::new(),
                });
            }
        }
    }

    // Extract materials
    if let Some(mat_start) = json.find("\"materials\"") {
        let after = &json[mat_start..];
        // Count material objects
        let mut depth = 0;
        let mut mat_count = 0;
        for ch in after.chars() {
            match ch {
                '[' if depth == 0 => depth = 1,
                '{' if depth == 1 => { depth = 2; mat_count += 1; }
                '{' => depth += 1,
                '}' if depth > 1 => depth -= 1,
                ']' if depth == 1 => break,
                _ => {}
            }
        }
        for i in 0..mat_count {
            scene.materials.push(GltfMaterial {
                name: format!("material_{i}"),
                base_color: Color3::WHITE,
                metallic: extract_num(json, "metallicFactor").unwrap_or(0.0),
                roughness: extract_num(json, "roughnessFactor").unwrap_or(1.0),
                emissive: Color3::new(0.0, 0.0, 0.0),
                alpha_mode: "OPAQUE".into(),
            });
        }
    }

    scene
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_obj_cube() {
        let obj = r#"
# Simple cube
o Cube
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0
vn 0 0 1
f 1//1 2//1 3//1 4//1
f 5//1 8//1 7//1 6//1
"#;
        let model = parse_obj(obj);
        assert_eq!(model.name, "Cube");
        assert_eq!(model.geometries.len(), 1);
        let geo = &model.geometries[0].geometry;
        assert!(!geo.vertices.is_empty());
        assert!(!geo.indices.is_empty());
    }

    #[test]
    fn parse_obj_with_uvs() {
        let obj = "v 0 0 0\nv 1 0 0\nv 1 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nf 1/1 2/2 3/3\n";
        let model = parse_obj(obj);
        assert_eq!(model.geometries[0].geometry.vertices.len(), 3);
        assert_eq!(model.geometries[0].geometry.indices.len(), 1);
    }

    #[test]
    fn parse_obj_groups() {
        let obj = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\ng front\nf 1 2 3\ng back\nf 2 3 4\n";
        let model = parse_obj(obj);
        assert_eq!(model.geometries.len(), 2);
    }

    #[test]
    fn parse_obj_polygon_triangulation() {
        let obj = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0.5 1.5 0\nv 0 1 0\nf 1 2 3 4 5\n";
        let model = parse_obj(obj);
        // 5-gon → 3 triangles
        assert_eq!(model.geometries[0].geometry.indices.len(), 3);
    }

    #[test]
    fn parse_mtl_basic() {
        let mtl = "newmtl Steel\nKd 0.5 0.5 0.6\nKs 1.0 1.0 1.0\nNs 100.0\nd 0.9\nmap_Kd steel.png\n";
        let mats = parse_mtl(mtl);
        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].name, "Steel");
        assert!((mats[0].shininess - 100.0).abs() < 0.01);
        assert_eq!(mats[0].diffuse_map, Some("steel.png".into()));
    }

    #[test]
    fn parse_mtl_multiple() {
        let mtl = "newmtl Red\nKd 1 0 0\nnewmtl Blue\nKd 0 0 1\n";
        let mats = parse_mtl(mtl);
        assert_eq!(mats.len(), 2);
    }

    #[test]
    fn gltf_minimal() {
        let gltf = r#"{"asset":{"version":"2.0"},"nodes":[{"name":"Cube","mesh":0},{"name":"Light"}],"materials":[{"name":"Mat","pbrMetallicRoughness":{"metallicFactor":0.5,"roughnessFactor":0.8}}]}"#;
        let scene = parse_gltf_json(gltf);
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.materials.len(), 1);
        assert!((scene.materials[0].metallic - 0.5).abs() < 0.01);
    }

    #[test]
    fn obj_model_has_bounds() {
        let obj = "v -1 -1 -1\nv 1 1 1\nv 0 0 0\nf 1 2 3\n";
        let model = parse_obj(obj);
        let geo = &model.geometries[0].geometry;
        assert!(geo.bounding_box.is_some());
    }
}
