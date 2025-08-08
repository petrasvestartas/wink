//! Primitive geometric types without metadata
//!
//! Only includes types that cannot be visualized and therefore don't need Data fields.

pub mod vector;
pub mod color;
pub mod xform;

pub use vector::Vector;
pub use color::Color;
pub use xform::Xform;
