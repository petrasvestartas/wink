use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;
use std::fmt;
use crate::geometry::Color;

/// Enhanced Data struct that combines metadata and serialization capabilities
/// Similar to COMPAS Data class for all serializable geometric objects
#[derive(Debug, Clone)]
pub struct Data {
    /// Object name
    name: String,
    /// Unique identifier
    guid: Uuid,
    /// Parent element's guid (optional, empty for top-level elements)
    parent: Option<Uuid>,
    /// List of guids of adjacent elements (optional, mostly empty)
    adjacency_indices: Vec<Uuid>,
    /// List of strings representing adjacency types
    adjacency_types: Vec<String>,
    /// Transformation as a flattened 4x4 matrix (column-major, 16 f64 values)
    /// Default is identity matrix
    transformation: [f64; 16],
    /// Color as RGB components [r, g, b] where each component is 0-255
    /// Default is [0, 0, 0] (black)
    color: [u8; 3],
    /// Thickness value (typically used for lines, curves, etc.)
    /// Default is 1.0
    thickness: f64,
}

// Custom serialization to make JSON more readable
impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Data", 6)?;
        state.serialize_field("name", &self.name)?; // Serialize as string
        state.serialize_field("guid", &self.guid)?;
        state.serialize_field("parent", &self.parent)?;
        state.serialize_field("adjacencyindices", &self.adjacency_indices)?;
        state.serialize_field("adjacencytypes", &self.adjacency_types)?;
        state.serialize_field("transformation", &self.transformation)?;
        state.serialize_field("color", &self.color)?;
        state.serialize_field("thickness", &self.thickness)?;
        state.end()
    }
}

// Custom deserialization to handle both string and byte array formats
impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Name,
            Guid,
            Parent,
            AdjacencyIndices,
            AdjacencyTypes,
            Transformation,
            Color,
            Thickness,
        }

        struct DataVisitor;

        impl<'de> Visitor<'de> for DataVisitor {
            type Value = Data;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Data")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Data, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                let mut guid: Option<Uuid> = None;
                let mut parent: Option<Option<Uuid>> = None;
                let mut adjacency_indices: Option<Vec<Uuid>> = None;
                let mut adjacency_types: Option<Vec<String>> = None;
                let mut transformation: Option<[f64; 16]> = None;
                let mut color: Option<[u8; 3]> = None;
                let mut thickness: Option<f64> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            // Handle both string and byte array formats
                            let value = map.next_value::<serde_json::Value>()?;
                            match value {
                                serde_json::Value::String(s) => {
                                    name = Some(s);
                                }
                                serde_json::Value::Array(arr) => {
                                    // Convert byte array back to string for backward compatibility
                                    let mut bytes = [0u8; 32];
                                    for (i, val) in arr.iter().enumerate().take(32) {
                                        if let Some(byte_val) = val.as_u64() {
                                            bytes[i] = byte_val as u8;
                                        }
                                    }
                                    // Convert byte array to string (backward compatibility)
                                    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                                    let name_str = std::str::from_utf8(&bytes[..end]).unwrap_or("");
                                    name = Some(name_str.to_string());
                                }
                                _ => return Err(de::Error::custom("name must be string or byte array")),
                            }
                        }
                        Field::Guid => {
                            guid = Some(map.next_value()?);
                        }
                        Field::Parent => {
                            parent = Some(map.next_value()?);
                        }
                        Field::AdjacencyIndices => {
                            adjacency_indices = Some(map.next_value()?);
                        }
                        Field::AdjacencyTypes => {
                            adjacency_types = Some(map.next_value()?);
                        }
                        Field::Transformation => {
                            // Handle both array and vector formats
                            let value = map.next_value::<serde_json::Value>()?;
                            if let serde_json::Value::Array(arr) = value {
                                let mut matrix = [0.0; 16];
                                for (i, val) in arr.iter().enumerate().take(16) {
                                    if let Some(num_val) = val.as_f64() {
                                        matrix[i] = num_val;
                                    }
                                }
                                transformation = Some(matrix);
                            } else {
                                return Err(de::Error::custom("transformation must be an array of 16 numbers"));
                            }
                        }
                        Field::Color => {
                            // Handle color as an array of RGB values
                            let value = map.next_value::<serde_json::Value>()?;
                            if let serde_json::Value::Array(arr) = value {
                                let mut rgb = [0u8; 3];
                                for (i, val) in arr.iter().enumerate().take(3) {
                                    if let Some(num_val) = val.as_u64() {
                                        rgb[i] = num_val as u8;
                                    }
                                }
                                color = Some(rgb);
                            } else {
                                // Default to black if not provided or malformed
                                color = Some([0, 0, 0]);
                            }
                        }
                        Field::Thickness => {
                            // Handle thickness as a float
                            let value = map.next_value::<serde_json::Value>()?;
                            if let Some(val) = value.as_f64() {
                                thickness = Some(val);
                            } else {
                                // Default to 1.0 if not provided or malformed
                                thickness = Some(1.0);
                            }
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let guid = guid.ok_or_else(|| de::Error::missing_field("guid"))?;
                let parent = parent.unwrap_or(None);
                let adjacency_indices = adjacency_indices.unwrap_or_else(Vec::new);
                let adjacency_types = adjacency_types.unwrap_or_else(Vec::new);
                let transformation = transformation.unwrap_or_else(Data::identity_matrix);
                let color = color.unwrap_or([0, 0, 0]); // Default color is black
                let thickness = thickness.unwrap_or(1.0); // Default thickness is 1.0

                Ok(Data {
                    name,
                    guid,
                    parent,
                    adjacency_indices,
                    adjacency_types,
                    transformation,
                    color,
                    thickness,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["name", "guid", "parent", "adjacency_indices", "adjacency_types", "transformation"];
        deserializer.deserialize_struct("Data", FIELDS, DataVisitor)
    }
}

impl Data {
    /// Create a new Data instance with given name
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            guid: Uuid::new_v4(),
            parent: None,
            adjacency_indices: Vec::new(),
            adjacency_types: Vec::new(),
            transformation: Self::identity_matrix(),
            color: [0, 0, 0], // Default black color
            thickness: 1.0,    // Default thickness
        }
    }
    
    /// Create a new Data instance with default name
    pub fn new() -> Self {
        Self::with_name("Data")
    }
    
    /// Get the object's GUID
    pub fn guid(&self) -> Uuid {
        self.guid
    }
    
    /// Get the object's name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Set the object's name
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    
    /// Get the parent GUID
    pub fn parent(&self) -> Option<Uuid> {
        self.parent
    }
    
    /// Set the parent GUID
    pub fn set_parent(&mut self, parent: Option<Uuid>) {
        self.parent = parent;
    }
    
    /// Get the adjacency indices
    pub fn adjacency_indices(&self) -> &[Uuid] {
        &self.adjacency_indices
    }
    
    /// Get the adjacency types
    pub fn adjacency_types(&self) -> &[String] {
        &self.adjacency_types
    }
    
    /// Add an adjacency relationship
    pub fn add_adjacency(&mut self, guid: Uuid, adjacency_type: &str) {
        self.adjacency_indices.push(guid);
        self.adjacency_types.push(adjacency_type.to_string());
    }
    
    /// Clear all adjacencies
    pub fn clear_adjacencies(&mut self) {
        self.adjacency_indices.clear();
        self.adjacency_types.clear();
    }
    

    
    /// Create a 4x4 identity matrix flattened to 16 values (column-major)
    pub fn identity_matrix() -> [f64; 16] {
        [
            1.0, 0.0, 0.0, 0.0,  // First column
            0.0, 1.0, 0.0, 0.0,  // Second column
            0.0, 0.0, 1.0, 0.0,  // Third column
            0.0, 0.0, 0.0, 1.0,  // Fourth column
        ]
    }
    
    /// Get the transformation matrix
    pub fn transformation(&self) -> &[f64; 16] {
        &self.transformation
    }
    
    /// Set the transformation matrix
    pub fn set_transformation(&mut self, matrix: [f64; 16]) {
        self.transformation = matrix;
    }
    
    /// Reset transformation to identity matrix
    pub fn reset_transformation(&mut self) {
        self.transformation = Self::identity_matrix();
    }
    
    /// Get the color as an RGB array [r, g, b]
    pub fn get_color(&self) -> [u8; 3] {
        self.color
    }
    
    /// Set the color as an RGB array [r, g, b]
    pub fn set_color(&mut self, color: [u8; 3]) {
        self.color = color;
    }
    
    /// Set the color from a Color struct
    pub fn set_color_from(&mut self, color: &Color) {
        self.set_color([color.r, color.g, color.b]);
    }
    
    /// Get the thickness value
    pub fn get_thickness(&self) -> f64 {
        self.thickness
    }
    
    /// Set the thickness value
    pub fn set_thickness(&mut self, thickness: f64) {
        self.thickness = thickness;
    }
    
    /// Check if the data has a non-default color
    pub fn has_color(&self) -> bool {
        // Check if color is not black (default)
        self.color != [0, 0, 0]
    }
    
    /// Apply data fields from JSON to this Data instance
    /// This is a helper function used by FromJsonData implementations
    pub fn apply_from_json(&mut self, data: &serde_json::Value) {
        // Set name if available
        if let Some(name) = data["name"].as_str() {
            self.set_name(name);
        }
        
        // Set guid if available
        if let Some(guid_str) = data["guid"].as_str() {
            if let Ok(guid) = uuid::Uuid::parse_str(guid_str) {
                self.guid = guid;
            }
        }
        
        // Set parent if available
        if data["parent"].is_null() {
            self.parent = None;
        } else if let Some(parent_str) = data["parent"].as_str() {
            if let Ok(parent_guid) = uuid::Uuid::parse_str(parent_str) {
                self.parent = Some(parent_guid);
            }
        }
        
        // Set adjacency_indices if available
        self.adjacency_indices.clear();
        if let Some(indices) = data["adjacency_indices"].as_array() {
            for idx in indices {
                if let Some(idx_str) = idx.as_str() {
                    if let Ok(idx_guid) = uuid::Uuid::parse_str(idx_str) {
                        self.adjacency_indices.push(idx_guid);
                    }
                }
            }
        }
        
        // Set adjacency_types if available
        self.adjacency_types.clear();
        if let Some(types) = data["adjacency_types"].as_array() {
            for adj_type in types {
                if let Some(type_str) = adj_type.as_str() {
                    self.adjacency_types.push(type_str.to_string());
                }
            }
        }
        
        // Set transformation if available
        if let Some(transform) = data["transformation"].as_array() {
            if transform.len() == 16 {
                let mut matrix = [0.0; 16];
                for (i, val) in transform.iter().enumerate().take(16) {
                    if let Some(num) = val.as_f64() {
                        matrix[i] = num;
                    }
                }
                self.transformation = matrix;
            } else if transform.is_empty() {
                // Empty array means identity matrix
                self.reset_transformation();
            }
        }
        
        // Set color if available
        if let Some(color_arr) = data["color"].as_array() {
            if color_arr.len() >= 3 {
                let mut rgb = [0u8; 3];
                for (i, val) in color_arr.iter().enumerate().take(3) {
                    if let Some(num) = val.as_u64() {
                        rgb[i] = num as u8;
                    }
                }
                self.color = rgb;
            }
        }
        
        // Set thickness if available
        if let Some(thickness) = data["thickness"].as_f64() {
            self.thickness = thickness;
        }
    }
    
    /// Create a structured JSON representation similar to COMPAS
    pub fn to_json_data(&self, dtype: &'static str, data: Value, minimal: bool) -> serde_json::Value {
        if minimal {
            serde_json::json!({
                "dtype": dtype,
                "data": data
            })
        } else {
            serde_json::json!({
                "dtype": dtype,
                "data": data,
                "guid": self.guid,
                "name": self.name(),
                "parent": self.parent,
                "adjacency_indices": self.adjacency_indices,
                "adjacency_types": self.adjacency_types,
                "transformation": self.transformation
            })
        }
    }
    
    /// Create a copy of the data with optional GUID copying
    pub fn copy(&self, copy_guid: bool) -> Self {
        Self {
            name: self.name.clone(),
            guid: if copy_guid { self.guid } else { Uuid::new_v4() },
            parent: self.parent,
            adjacency_indices: self.adjacency_indices.clone(),
            adjacency_types: self.adjacency_types.clone(),
            transformation: self.transformation,
            color: self.color,
            thickness: self.thickness,
        }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            "Data {{ name: {}, guid: {}, parent: {:?}, adjacencies: {}, has_transform: {} }}", 
            self.name(), 
            self.guid,
            self.parent,
            self.adjacency_indices.len(),
            !self.transformation.iter().enumerate().all(|(i, &val)| {
                (i % 5 == 0 && (val - 1.0).abs() < f64::EPSILON) || // Diagonal elements are 1.0
                (i % 5 != 0 && val.abs() < f64::EPSILON) // Non-diagonal elements are 0.0
            })
        )
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}

/// Test module
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_data_creation() {
        let data = Data::new();
        assert_eq!(data.name(), "Data");
        assert!(!data.guid().is_nil());
        assert_eq!(data.parent(), None);
        assert_eq!(data.adjacency_indices().len(), 0);
        assert_eq!(data.adjacency_types().len(), 0);
        assert_eq!(data.transformation(), &Data::identity_matrix());
    }
    
    #[test]
    fn test_data_with_name() {
        let data = Data::with_name("test");
        assert_eq!(data.name(), "test");
    }
    
    #[test]
    fn test_set_name() {
        let mut data = Data::new();
        data.set_name("modified");
        assert_eq!(data.name(), "modified");
    }
    
    #[test]
    fn test_to_json_data() {
        let data = Data::with_name("test");
        let json = data.to_json_data("test_type", serde_json::json!({"value": 42}), false);
        assert_eq!(json["dtype"], "test_type");
        assert_eq!(json["data"]["value"], 42);
        assert_eq!(json["name"], "test");
        assert!(json["adjacency_indices"].is_array());
        assert!(json["adjacency_types"].is_array());
        assert!(json["transformation"].is_array());
        assert_eq!(json["transformation"][0], 1.0); // First element of identity matrix
    }
    
    #[test]
    fn test_adjacency() {
        let mut data = Data::with_name("test");
        let other_guid = Uuid::new_v4();
        
        // Add adjacency
        data.add_adjacency(other_guid, "connected_to");
        assert_eq!(data.adjacency_indices().len(), 1);
        assert_eq!(data.adjacency_types().len(), 1);
        assert_eq!(data.adjacency_indices()[0], other_guid);
        assert_eq!(data.adjacency_types()[0], "connected_to");
        
        // Clear adjacencies
        data.clear_adjacencies();
        assert_eq!(data.adjacency_indices().len(), 0);
        assert_eq!(data.adjacency_types().len(), 0);
    }
    
    #[test]
    fn test_parent() {
        let mut data = Data::with_name("test");
        let parent_guid = Uuid::new_v4();
        
        // Initially no parent
        assert_eq!(data.parent(), None);
        
        // Set parent
        data.set_parent(Some(parent_guid));
        assert_eq!(data.parent(), Some(parent_guid));
        
        // Clear parent
        data.set_parent(None);
        assert_eq!(data.parent(), None);
    }
    
    #[test]
    fn test_transformation() {
        let mut data = Data::with_name("test");
        
        // Default should be identity matrix
        let identity = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert_eq!(data.transformation(), &identity);
        
        // Set custom transformation
        let custom_transform = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            1.0, 2.0, 3.0, 1.0,
        ];
        data.set_transformation(custom_transform);
        assert_eq!(data.transformation(), &custom_transform);
        
        // Reset to identity
        data.reset_transformation();
        assert_eq!(data.transformation(), &identity);
    }
}