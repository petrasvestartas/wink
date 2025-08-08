use crate::geometry::{Mesh, Point};
use crate::primitives::Vector;
use serde::{Deserialize, Serialize};

/// Represents a 3D model mesh with vertices, faces, and rendering data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMesh {
    /// Vertex positions as flat array [x0, y0, z0, x1, y1, z1, ...]
    pub vertices: Vec<f32>,
    /// Triangle indices as flat array [i0, i1, i2, i3, i4, i5, ...]
    pub indices: Vec<u32>,
    /// Vertex normals as flat array [nx0, ny0, nz0, nx1, ny1, nz1, ...]
    pub normals: Vec<f32>,
    /// Vertex colors as flat array [r0, g0, b0, r1, g1, b1, ...]
    pub colors: Vec<f32>,
    /// Number of vertices
    pub vertex_count: usize,
    /// Number of triangles
    pub triangle_count: usize,
}

impl ModelMesh {
    /// Create a new empty model mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            vertex_count: 0,
            triangle_count: 0,
        }
    }

    /// Create a model mesh from a halfedge mesh
    pub fn from_halfedge_mesh(mesh: &Mesh) -> Self {
        use std::collections::HashMap;
        
        let mut model_mesh = Self::new();
        
        // Create a mapping from halfedge vertex keys to sequential indices
        let mut vertex_key_to_index = HashMap::new();
        let mut vertex_positions = Vec::new();
        
        // First pass: collect all unique vertex keys and positions
        // Get all face data directly from the mesh's internal storage
        let face_data: Vec<(&usize, &Vec<usize>)> = mesh.get_face_data().collect();
        
        for (_face_key, face_vertices) in &face_data {
            for &vertex_key in *face_vertices {
                if !vertex_key_to_index.contains_key(&vertex_key) {
                    if let Some(position) = mesh.vertex_position(vertex_key) {
                        let index = vertex_positions.len();
                        vertex_key_to_index.insert(vertex_key, index);
                        vertex_positions.push(position.clone());
                        
                        // Add to flat vertex array
                        model_mesh.vertices.push(position.x as f32);
                        model_mesh.vertices.push(position.y as f32);
                        model_mesh.vertices.push(position.z as f32);
                    }
                }
            }
        }
        model_mesh.vertex_count = vertex_positions.len();
        
        // Second pass: create triangles using the mapped indices
        for (_face_key, face_vertices) in &face_data {
            if face_vertices.len() >= 3 {
                // Triangulate face (simple fan triangulation)
                for i in 1..face_vertices.len() - 1 {
                    if let (Some(&idx0), Some(&idx1), Some(&idx2)) = (
                        vertex_key_to_index.get(&face_vertices[0]),
                        vertex_key_to_index.get(&face_vertices[i]),
                        vertex_key_to_index.get(&face_vertices[i + 1])
                    ) {
                        model_mesh.indices.push(idx0 as u32);
                        model_mesh.indices.push(idx1 as u32);
                        model_mesh.indices.push(idx2 as u32);
                        model_mesh.triangle_count += 1;
                    }
                }
            }
        }
        
        // Calculate vertex normals
        model_mesh.calculate_normals();
        
        // Set default white colors
        model_mesh.colors = vec![1.0; model_mesh.vertex_count * 3];
        
        model_mesh
    }
    
    /// Create a model mesh from polygons with duplicate point removal
    pub fn from_polygons(polygons: Vec<Vec<Point>>, precision: Option<f64>) -> Self {
        let mesh = Mesh::from_polygons_with_merge(polygons, precision);
        Self::from_halfedge_mesh(&mesh)
    }
    
    /// Load geometry from JSON file
    pub fn from_json_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(file_path)?;
        let json_data: serde_json::Value = serde_json::from_str(&content)?;
        
        // Parse cube geometry from JSON
        if let Some(cube_data) = json_data.get("cube") {
            if let Some(polygons_data) = cube_data.get("polygons") {
                let mut polygons = Vec::new();
                
                if let Some(polygon_array) = polygons_data.as_array() {
                    for polygon_data in polygon_array {
                        if let Some(vertices_data) = polygon_data.get("vertices") {
                            if let Some(vertex_array) = vertices_data.as_array() {
                                let mut polygon = Vec::new();
                                
                                for vertex_data in vertex_array {
                                    if let (Some(x), Some(y), Some(z)) = (
                                        vertex_data.get("x").and_then(|v| v.as_f64()),
                                        vertex_data.get("y").and_then(|v| v.as_f64()),
                                        vertex_data.get("z").and_then(|v| v.as_f64()),
                                    ) {
                                        polygon.push(Point::new(x, y, z));
                                    }
                                }
                                
                                if !polygon.is_empty() {
                                    polygons.push(polygon);
                                }
                            }
                        }
                    }
                }
                
                return Ok(Self::from_polygons(polygons, None));
            }
        }
        
        Err("Invalid JSON format or missing cube data".into())
    }
    
    /// Calculate vertex normals using face normals
    fn calculate_normals(&mut self) {
        self.normals = vec![0.0; self.vertex_count * 3];
        let mut vertex_normal_counts = vec![0; self.vertex_count];
        
        // Calculate face normals and accumulate to vertices
        for triangle_idx in 0..self.triangle_count {
            let i0 = self.indices[triangle_idx * 3] as usize;
            let i1 = self.indices[triangle_idx * 3 + 1] as usize;
            let i2 = self.indices[triangle_idx * 3 + 2] as usize;
            
            // Get vertex positions
            let v0 = Vector::new(
                self.vertices[i0 * 3] as f64,
                self.vertices[i0 * 3 + 1] as f64,
                self.vertices[i0 * 3 + 2] as f64,
            );
            let v1 = Vector::new(
                self.vertices[i1 * 3] as f64,
                self.vertices[i1 * 3 + 1] as f64,
                self.vertices[i1 * 3 + 2] as f64,
            );
            let v2 = Vector::new(
                self.vertices[i2 * 3] as f64,
                self.vertices[i2 * 3 + 1] as f64,
                self.vertices[i2 * 3 + 2] as f64,
            );
            
            // Calculate face normal using cross product
            let edge1 = v1 - v0.clone();
            let edge2 = v2 - v0;
            let normal = edge1.cross(&edge2).normalize();
            
            // Accumulate normal to each vertex of the triangle
            for &vertex_idx in &[i0, i1, i2] {
                self.normals[vertex_idx * 3] += normal.x as f32;
                self.normals[vertex_idx * 3 + 1] += normal.y as f32;
                self.normals[vertex_idx * 3 + 2] += normal.z as f32;
                vertex_normal_counts[vertex_idx] += 1;
            }
        }
        
        // Normalize accumulated normals
        for vertex_idx in 0..self.vertex_count {
            if vertex_normal_counts[vertex_idx] > 0 {
                let count = vertex_normal_counts[vertex_idx] as f32;
                let nx = self.normals[vertex_idx * 3] / count;
                let ny = self.normals[vertex_idx * 3 + 1] / count;
                let nz = self.normals[vertex_idx * 3 + 2] / count;
                
                // Normalize the normal vector
                let length = (nx * nx + ny * ny + nz * nz).sqrt();
                if length > 0.0 {
                    self.normals[vertex_idx * 3] = nx / length;
                    self.normals[vertex_idx * 3 + 1] = ny / length;
                    self.normals[vertex_idx * 3 + 2] = nz / length;
                }
            }
        }
    }
    
    /// Set vertex colors from a color array
    pub fn set_colors(&mut self, colors: Vec<[f32; 3]>) {
        self.colors.clear();
        for color in colors {
            self.colors.extend_from_slice(&color);
        }
        
        // Pad with white if not enough colors
        while self.colors.len() < self.vertex_count * 3 {
            self.colors.extend_from_slice(&[1.0, 1.0, 1.0]);
        }
    }
    
    /// Get vertex data as interleaved array [x, y, z, nx, ny, nz, r, g, b, ...]
    pub fn get_interleaved_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity(self.vertex_count * 9);
        
        for i in 0..self.vertex_count {
            // Position
            data.push(self.vertices[i * 3]);
            data.push(self.vertices[i * 3 + 1]);
            data.push(self.vertices[i * 3 + 2]);
            
            // Normal
            data.push(self.normals[i * 3]);
            data.push(self.normals[i * 3 + 1]);
            data.push(self.normals[i * 3 + 2]);
            
            // Color
            data.push(self.colors[i * 3]);
            data.push(self.colors[i * 3 + 1]);
            data.push(self.colors[i * 3 + 2]);
        }
        
        data
    }
}

impl Default for ModelMesh {
    fn default() -> Self {
        Self::new()
    }
}
