// Make macros available throughout the crate
#[macro_use]
mod macros;

pub mod common;
pub mod geometry;
pub mod primitives;
pub mod model_mesh;

use geometry::{Point, Vector, Line, Plane, Color, PointCloud, LineCloud, Pline, Mesh};
use primitives::Xform;
use common::{JsonSerializable, FromJsonData};
use serde::{Serialize, Deserialize};

// MeshInstances: 
#[derive(Serialize, Deserialize, Debug)]
pub struct MeshInstances {
    pub mesh_index: usize,              // or mesh GUID
    pub transforms: Vec<primitives::Xform>,
}

// Comprehensive geometry data structure with all geometry types
#[derive(Serialize, Deserialize, Debug)]
pub struct AllGeometryData {
    pub points: Vec<Point>,
    pub vectors: Vec<Vector>,
    pub lines: Vec<Line>,
    pub planes: Vec<Plane>,
    pub colors: Vec<Color>,
    pub point_clouds: Vec<PointCloud>,
    pub line_clouds: Vec<LineCloud>,
    pub plines: Vec<Pline>,
    pub xforms: Vec<Xform>,
    pub meshes: Vec<Mesh>,
    #[serde(default)]
    pub mesh_instances: Vec<MeshInstances>,
    #[serde(skip)]
    pub pipe_mesh_index: Option<usize>,
    #[serde(skip)]
    pub sphere_mesh_index: Option<usize>,
}

// Implement JsonSerializable for AllGeometryData to work with json_dump/json_load
impl JsonSerializable for AllGeometryData {
    fn to_json_value(&self) -> serde_json::Value {
        // Use direct serialization for consistency
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

// Implement FromJsonData for AllGeometryData to work with json_load
impl FromJsonData for AllGeometryData {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        // Use direct deserialization for consistency
        serde_json::from_value(data.clone()).ok()
    }
}

// Procedural augmentation: add high-res unit pipe and sphere meshes and instance transforms
impl AllGeometryData {
    /// Augment the geometry by adding procedural instances:
    /// - Pipes along boundary edges of existing meshes (using high-res unit pipe geometry)
    /// - Spheres at boundary vertices of existing meshes (using high-res unit sphere geometry)
    ///
    /// Notes:
    /// - The shared unit pipe mesh is aligned to +Z with radius=0.5 and length=1.0.
    ///   `Mesh::extract_edge_pipe_transforms(radius)` returns transforms that map this unit
    ///   pipe onto mesh boundary edges with the requested world-space radius.
    /// - The shared unit sphere mesh has radius=0.5; sphere instances use translation-only
    ///   transforms so the final world-space radius stays 0.5 unless further scaled by the user.
    pub fn augment_with_procedural(&mut self) {
        let pipe_radius: f64 = 0.02; // world-space radius for pipes

        // Reset procedural indices before augmentation
        self.pipe_mesh_index = None;
        self.sphere_mesh_index = None;

        // Only process the meshes that existed prior to augmentation to avoid reprocessing
        // the procedural unit meshes we will append below.
        let original_mesh_count = self.meshes.len();

        // 1) Edge pipes: collect per-edge transforms from all original meshes
        let mut pipe_transforms: Vec<Xform> = Vec::new();
        for i in 0..original_mesh_count {
            let tfs = self.meshes[i].extract_edge_pipe_transforms(pipe_radius);
            if !tfs.is_empty() {
                pipe_transforms.extend(tfs);
            }
        }
        if !pipe_transforms.is_empty() {
            let pipe_mesh_index = self.meshes.len();
            self.meshes.push(Mesh::create_unit_pipe_high_res());
            self.mesh_instances.push(MeshInstances {
                mesh_index: pipe_mesh_index,
                transforms: pipe_transforms,
            });
            self.pipe_mesh_index = Some(pipe_mesh_index);
        }

        // 2) Boundary spheres: place a sphere at every boundary vertex of original meshes
        let mut sphere_transforms: Vec<Xform> = Vec::new();
        for i in 0..original_mesh_count {
            let m = &self.meshes[i];
            for vk in m.vertex.keys() {
                if m.is_vertex_on_boundary(*vk) {
                    if let Some(p) = m.vertex_position(*vk) {
                        sphere_transforms.push(Xform::translation(p.x, p.y, p.z));
                    }
                }
            }
        }
        if !sphere_transforms.is_empty() {
            let sphere_mesh_index = self.meshes.len();
            self.meshes.push(Mesh::create_unit_sphere_high_res());
            self.mesh_instances.push(MeshInstances {
                mesh_index: sphere_mesh_index,
                transforms: sphere_transforms,
            });
            self.sphere_mesh_index = Some(sphere_mesh_index);
        }
    }
}