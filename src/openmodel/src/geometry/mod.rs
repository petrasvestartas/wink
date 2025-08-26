pub mod line;
pub mod arrow;
pub mod plane;
pub mod pointcloud;
pub mod linecloud;
pub mod pline;
pub mod mesh;
pub mod pipe;

// Re-export primitive types for backward compatibility
pub use crate::primitives::{Vector, Color, Xform, Point};

pub use line::Line;
pub use arrow::Arrow;
pub use plane::Plane;
pub use pointcloud::PointCloud;
pub use linecloud::LineCloud;
pub use pline::Pline;
pub use mesh::Mesh;
pub use pipe::{PipeFromSegments, SphereFromSegments, dedupe_sphere_transforms};