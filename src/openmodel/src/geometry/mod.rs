pub mod point;
pub mod line;
pub mod plane;
pub mod pointcloud;
pub mod linecloud;
pub mod pline;
pub mod mesh;

// Re-export primitive types for backward compatibility
pub use crate::primitives::{Vector, Color, Xform};

pub use point::Point;
pub use line::Line;
pub use plane::Plane;
pub use pointcloud::PointCloud;
pub use linecloud::LineCloud;
pub use pline::Pline;
pub use mesh::Mesh;