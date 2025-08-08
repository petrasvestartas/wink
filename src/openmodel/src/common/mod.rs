pub mod data;
pub mod json_serialization;

// Re-export commonly used types and functions
pub use data::Data;
pub use json_serialization::{JsonSerializable, JsonData, json_dump, json_load, HasJsonData, FromJsonData};