// Make macros available throughout the crate
#[macro_use]
mod macros;

pub mod common;
pub mod geometry;
pub mod primitives;


use geometry::{Point, Vector, Line, Plane, Color, PointCloud, LineCloud, Pline, Mesh, PipeFromSegments, SphereFromSegments, dedupe_sphere_transforms};
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
    /// - Pipes from segment sources: mesh boundary edges, polylines, line clouds, and standalone lines
    ///   (instanced from a high-res unit pipe geometry).
    /// - Spheres at mesh boundary vertices and at all segment endpoints across the above sources
    ///   (instanced from a high-res unit sphere), deduplicated by position.
    ///
    /// Notes:
    /// - The shared unit pipe mesh is aligned to +Z with radius=0.5 and length=1.0.
    ///   Transforms are generated via the `PipeFromSegments` trait with XY scale fixed at 1.0
    ///   and Z scaled by segment length. Pipe thickness is controlled in the shader by a
    ///   pixel-space radius uniform, not by instance XY scaling.
    /// - The shared unit sphere mesh has radius=0.5; sphere instances use translation-only
    ///   transforms so the final world-space radius stays 0.5 unless further scaled by the user.
    pub fn augment_with_procedural(&mut self) {
        // DISABLED: Skip all procedural geometry on web for maximum performance
        // #[cfg(target_arch = "wasm32")]
        // {
        //     return; // Complete skip on web - no pipes, no spheres
        // }
        
        // Use low-res procedural geometry for better native performance
        // Pipes: 8 segments instead of 32 (4x fewer vertices)
        // Spheres: 1 subdivision instead of 3 (16x fewer vertices)

        // Reset procedural indices before augmentation
        self.pipe_mesh_index = None;
        self.sphere_mesh_index = None;

        // Only process the meshes that existed prior to augmentation to avoid reprocessing
        // the procedural unit meshes we will append below.
        let original_mesh_count = self.meshes.len();

        // 1) Pipes from multiple segment sources (meshes, plines, line clouds, lines)
        let mut pipe_transforms: Vec<Xform> = Vec::new();

        // All mesh edges (only original meshes) - use extract_edges_as_lines for complete wireframe
        for i in 0..original_mesh_count {
            let lines = self.meshes[i].extract_edges_as_lines();
            pipe_transforms.extend(lines.pipe_transforms());
        }

        // Polyline segments
        for pl in &self.plines {
            pipe_transforms.extend(pl.pipe_transforms());
        }

        // Line clouds
        for lc in &self.line_clouds {
            pipe_transforms.extend(lc.pipe_transforms());
        }

        // Standalone lines
        if !self.lines.is_empty() {
            pipe_transforms.extend(self.lines.pipe_transforms());
        }

        if !pipe_transforms.is_empty() {
            let pipe_mesh_index = self.meshes.len();
            self.meshes.push(Mesh::create_unit_pipe_low_res()); // Use low-res for performance
            self.mesh_instances.push(MeshInstances {
                mesh_index: pipe_mesh_index,
                transforms: pipe_transforms,
            });
            self.pipe_mesh_index = Some(pipe_mesh_index);
        }



        // 2) Spheres at segment vertices (deduplicated across all sources)
        let eps: f64 = 1e-6;
        let mut all_sphere_points: Vec<Point> = Vec::new();

        // Mesh boundary vertices (only original meshes)
        for i in 0..original_mesh_count {
            all_sphere_points.extend(self.meshes[i].sphere_points());
        }

        // Pline vertices
        for pl in &self.plines {
            all_sphere_points.extend(pl.sphere_points());
        }

        // LineCloud endpoints
        for lc in &self.line_clouds {
            all_sphere_points.extend(lc.sphere_points());
        }

        // Standalone Line endpoints
        if !self.lines.is_empty() {
            all_sphere_points.extend(self.lines.sphere_points());
        }

        let sphere_transforms: Vec<Xform> = dedupe_sphere_transforms(all_sphere_points, eps);

        if !sphere_transforms.is_empty() {
            let sphere_mesh_index = self.meshes.len();
            self.meshes.push(Mesh::create_unit_sphere_low_res()); // Use low-res for performance
            self.mesh_instances.push(MeshInstances {
                mesh_index: sphere_mesh_index,
                transforms: sphere_transforms,
            });
            self.sphere_mesh_index = Some(sphere_mesh_index);
        }
    }
}