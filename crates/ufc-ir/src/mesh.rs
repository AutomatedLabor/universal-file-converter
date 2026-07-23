use crate::traits::{IntermediateRepresentation, ValidationError, ValidationSeverity};
use semver::Version;
use serde::{Deserialize, Serialize};

/// 3D Mesh Intermediate Representation.
///
/// Covers: OBJ, STL, PLY, GLTF/GLB (limited)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh3DIR {
    pub version: Version,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub scene_graph: Option<SceneNode>,
    pub metadata: MeshMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub normals: Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    pub material_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
    pub tex_coord: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Face {
    Triangle([u32; 3]),
    Quad([u32; 4]),
    Polygon(Vec<u32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub diffuse_color: Option<[f32; 4]>,
    pub specular_color: Option<[f32; 4]>,
    pub ambient_color: Option<[f32; 4]>,
    pub shininess: Option<f32>,
    pub transparency: Option<f32>,
    pub diffuse_texture: Option<String>,
    pub normal_map: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    pub transform: [[f32; 4]; 4], // 4x4 matrix
    pub mesh_index: Option<usize>,
    pub children: Vec<SceneNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMetadata {
    pub source_format: String,
    pub unit: Option<String>,
    pub up_axis: Option<String>,
    pub vertex_count: usize,
    pub face_count: usize,
}

impl Mesh3DIR {
    pub fn new(source_format: &str) -> Self {
        Self {
            version: crate::api_version(),
            meshes: Vec::new(),
            materials: Vec::new(),
            scene_graph: None,
            metadata: MeshMetadata {
                source_format: source_format.to_string(),
                unit: None,
                up_axis: None,
                vertex_count: 0,
                face_count: 0,
            },
        }
    }

    pub fn total_vertices(&self) -> usize {
        self.meshes.iter().map(|m| m.vertices.len()).sum()
    }

    pub fn total_faces(&self) -> usize {
        self.meshes.iter().map(|m| m.faces.len()).sum()
    }
}

impl IntermediateRepresentation for Mesh3DIR {
    fn version(&self) -> Version { self.version.clone() }
    fn ir_type(&self) -> &'static str { "Mesh3D" }
    fn memory_usage(&self) -> u64 {
        let vert_bytes: u64 = self.meshes.iter().map(|m| m.vertices.len() as u64 * 64).sum();
        let face_bytes: u64 = self.meshes.iter().map(|m| m.faces.len() as u64 * 32).sum();
        vert_bytes + face_bytes + 4096
    }
    fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.meshes.is_empty() {
            errors.push(ValidationError {
                field: "meshes".into(),
                message: "No meshes defined".into(),
                severity: ValidationSeverity::Warning,
            });
        }
        for (i, mesh) in self.meshes.iter().enumerate() {
            if mesh.vertices.is_empty() {
                errors.push(ValidationError {
                    field: format!("meshes[{}].vertices", i),
                    message: "Mesh has no vertices".into(),
                    severity: ValidationSeverity::Error,
                });
            }
        }
        errors
    }
    fn to_json(&self) -> Result<String, serde_json::Error> { serde_json::to_string_pretty(self) }
    fn is_empty(&self) -> bool { self.meshes.is_empty() }
}
