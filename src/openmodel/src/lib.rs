// Make macros available throughout the crate
#[macro_use]
mod macros;

pub mod common;
pub mod geometry;
pub mod primitives;


use geometry::{Point, Vector, Line, Arrow, Plane, Color, PointCloud, LineCloud, Pline, Mesh};
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
    pub arrows: Vec<Arrow>,
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

