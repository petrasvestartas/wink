use crate::geometry::{Point, Vector};
use crate::common::Data;
use crate::common::{JsonSerializable, FromJsonData};
use crate::primitives::Xform;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use uuid::Uuid;

/// Weighting scheme for vertex normal computation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalWeighting {
    /// Weight face normals by face area (default)
    Area,
    /// Weight face normals by interior angle at the vertex
    Angle,
    /// Uniform weighting (all faces contribute equally)
    Uniform,
}

/// A halfedge mesh data structure for representing polygonal surfaces.
/// 
/// This implementation follows the COMPAS halfedge mesh design, where mesh
/// connectivity is stored using a halfedge data structure. Each edge is split
/// into two halfedges with opposite orientations, enabling efficient topological
/// queries and mesh operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    /// Halfedge connectivity: halfedge[u][v] represents the halfedge from vertex u to vertex v
    pub halfedge: HashMap<usize, HashMap<usize, Option<usize>>>,
    /// Vertices: maps vertex key to vertex data
    pub vertex: HashMap<usize, VertexData>,
    /// Faces: maps face key to list of vertex keys in order
    pub face: HashMap<usize, Vec<usize>>,
    /// Face attributes: maps face key to face attributes
    pub facedata: HashMap<usize, HashMap<String, f64>>,
    /// Edge attributes: maps edge tuple to edge attributes  
    pub edgedata: HashMap<(usize, usize), HashMap<String, f64>>,
    /// Default vertex attributes
    pub default_vertex_attributes: HashMap<String, f64>,
    /// Default face attributes
    pub default_face_attributes: HashMap<String, f64>,
    /// Default edge attributes
    pub default_edge_attributes: HashMap<String, f64>,
    /// Next available vertex key
    max_vertex: usize,
    /// Next available face key
    max_face: usize,
    /// Associated data - guid and name
    pub data: Data,
}

/// Vertex data containing position and attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    /// 3D position of the vertex
    pub x: f64,
    pub y: f64, 
    pub z: f64,
    /// Vertex attributes organized by type
    pub attributes: HashMap<String, f64>,
}

impl VertexData {
    /// Create a new vertex from a Point
    pub fn new(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
            z: point.z,
            attributes: HashMap::new(),
        }
    }
    
    /// Get the position as a Point
    pub fn position(&self) -> Point {
        Point::new(self.x, self.y, self.z)
    }
    
    /// Set the position from a Point
    pub fn set_position(&mut self, point: Point) {
        self.x = point.x;
        self.y = point.y;
        self.z = point.z;
    }
    
    // Convenience methods for common attributes
    pub fn color(&self) -> [f64; 3] {
        [
            self.attributes.get("r").copied().unwrap_or(0.5),
            self.attributes.get("g").copied().unwrap_or(0.5),
            self.attributes.get("b").copied().unwrap_or(0.5),
        ]
    }
    
    pub fn set_color(&mut self, r: f64, g: f64, b: f64) {
        self.attributes.insert("r".to_string(), r);
        self.attributes.insert("g".to_string(), g);
        self.attributes.insert("b".to_string(), b);
    }
    
    pub fn normal(&self) -> Option<[f64; 3]> {
        let nx = self.attributes.get("nx")?;
        let ny = self.attributes.get("ny")?;
        let nz = self.attributes.get("nz")?;
        Some([*nx, *ny, *nz])
    }
    
    pub fn set_normal(&mut self, nx: f64, ny: f64, nz: f64) {
        self.attributes.insert("nx".to_string(), nx);
        self.attributes.insert("ny".to_string(), ny);
        self.attributes.insert("nz".to_string(), nz);
    }
    
    pub fn tex_coords(&self) -> Option<[f64; 2]> {
        let u = self.attributes.get("u")?;
        let v = self.attributes.get("v")?;
        Some([*u, *v])
    }
    
    pub fn set_tex_coords(&mut self, u: f64, v: f64) {
        self.attributes.insert("u".to_string(), u);
        self.attributes.insert("v".to_string(), v);
    }
    
    // Generic attribute access
    pub fn get_attribute(&self, name: &str) -> Option<f64> {
        self.attributes.get(name).copied()
    }
    
    pub fn set_attribute(&mut self, name: &str, value: f64) {
        self.attributes.insert(name.to_string(), value);
    }
}



impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

impl Mesh {
    /// Create a new empty halfedge mesh.
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::Mesh;
    /// let mesh = Mesh::new();
    /// assert_eq!(mesh.number_of_vertices(), 0);
    /// assert_eq!(mesh.number_of_faces(), 0);
    /// assert!(mesh.is_empty());
    /// ```
    pub fn new() -> Self {
        let mut default_vertex_attributes = HashMap::new();
        default_vertex_attributes.insert("x".to_string(), 0.0);
        default_vertex_attributes.insert("y".to_string(), 0.0);
        default_vertex_attributes.insert("z".to_string(), 0.0);
        
        Mesh {
            halfedge: HashMap::new(),
            vertex: HashMap::new(),
            face: HashMap::new(),
            facedata: HashMap::new(),
            edgedata: HashMap::new(),
            default_vertex_attributes,
            default_face_attributes: HashMap::new(),
            default_edge_attributes: HashMap::new(),
            max_vertex: 0,
            max_face: 0,
            data: Data::with_name("Mesh"),
        }
    }

    /// Check if the mesh is empty.
    /// 
    /// # Returns
    /// True if the mesh has no vertices and no faces
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::Mesh;
    /// let mesh = Mesh::new();
    /// assert!(mesh.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.vertex.is_empty() && self.face.is_empty()
    }

    /// Clear the mesh, removing all vertices and faces.
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// assert!(!mesh.is_empty());
    /// mesh.clear();
    /// assert!(mesh.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.halfedge.clear();
        self.vertex.clear();
        self.face.clear();
        self.facedata.clear();
        self.edgedata.clear();
        self.max_vertex = 0;
        self.max_face = 0;
    }

    /// Get the number of vertices in the mesh.
    /// 
    /// # Returns
    /// The total number of vertices
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// assert_eq!(mesh.number_of_vertices(), 0);
    /// mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// assert_eq!(mesh.number_of_vertices(), 1);
    /// ```
    pub fn number_of_vertices(&self) -> usize {
        self.vertex.len()
    }

    /// Get the number of faces in the mesh.
    /// 
    /// # Returns
    /// The total number of faces
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// assert_eq!(mesh.number_of_faces(), 0);
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// assert_eq!(mesh.number_of_faces(), 1);
    /// ```
    pub fn number_of_faces(&self) -> usize {
        self.face.len()
    }

    /// Get the number of edges in the mesh.
    /// 
    /// # Returns
    /// The total number of edges (undirected)
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// assert_eq!(mesh.number_of_edges(), 3);
    /// ```
    pub fn number_of_edges(&self) -> usize {
        let mut seen = HashSet::new();
        let mut count = 0;
        
        for u in self.halfedge.keys() {
            if let Some(neighbors) = self.halfedge.get(u) {
                for v in neighbors.keys() {
                    let edge = if u < v { (*u, *v) } else { (*v, *u) };
                    if seen.insert(edge) {
                        count += 1;
                    }
                }
            }
        }
        
        count
    }

    /// Compute the Euler characteristic (V - E + F) of the mesh.
    /// 
    /// # Returns
    /// The Euler characteristic
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// assert_eq!(mesh.euler(), 1); // V=3, E=3, F=1 -> 3-3+1=1
    /// ```
    pub fn euler(&self) -> i32 {
        let v = self.number_of_vertices() as i32;
        let e = self.number_of_edges() as i32;
        let f = self.number_of_faces() as i32;
        v - e + f
    }

    /// Add a vertex to the mesh.
    /// 
    /// # Arguments
    /// * `position` - The 3D position of the vertex
    /// * `key` - Optional specific key for the vertex. If None, auto-generates.
    /// 
    /// # Returns
    /// The key of the added vertex
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let vertex_key = mesh.add_vertex(Point::new(1.0, 2.0, 3.0), None);
    /// assert_eq!(mesh.number_of_vertices(), 1);
    /// ```
    pub fn add_vertex(&mut self, position: Point, key: Option<usize>) -> usize {
        let vertex_key = key.unwrap_or_else(|| {
            self.max_vertex += 1;
            self.max_vertex
        });
        
        // Update max_vertex if explicit key is larger
        if vertex_key >= self.max_vertex {
            self.max_vertex = vertex_key + 1;
        }
        
        let vertex_data = VertexData::new(position);
        self.vertex.insert(vertex_key, vertex_data);
        
        // Initialize halfedge connectivity for this vertex
        self.halfedge.entry(vertex_key).or_insert_with(HashMap::new);
        
        vertex_key
    }

    /// Add a face to the mesh.
    /// 
    /// # Arguments
    /// * `vertices` - List of vertex keys defining the face in order
    /// * `fkey` - Optional specific key for the face. If None, auto-generates.
    /// 
    /// # Returns
    /// The key of the added face, or None if the face is invalid
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let face_key = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// assert_eq!(mesh.number_of_faces(), 1);
    /// ```
    pub fn add_face(&mut self, vertices: Vec<usize>, fkey: Option<usize>) -> Option<usize> {
        // Validate the face
        if vertices.len() < 3 {
            return None;
        }
        
        // Check that all vertices exist
        if !vertices.iter().all(|v| self.vertex.contains_key(v)) {
            return None;
        }
        
        // Check for duplicate vertices
        let mut unique_vertices = HashSet::new();
        for vertex in &vertices {
            if !unique_vertices.insert(*vertex) {
                return None; // Duplicate vertex found
            }
        }
        
        let face_key = fkey.unwrap_or_else(|| {
            self.max_face += 1;
            self.max_face
        });
        
        // Update max_face if explicit key is larger
        if face_key >= self.max_face {
            self.max_face = face_key + 1;
        }
        
        // Add the face
        self.face.insert(face_key, vertices.clone());
        
        // Update halfedge connectivity
        for i in 0..vertices.len() {
            let u = vertices[i];
            let v = vertices[(i + 1) % vertices.len()];
            
            // Ensure both vertices have halfedge entries
            self.halfedge.entry(u).or_insert_with(HashMap::new);
            self.halfedge.entry(v).or_insert_with(HashMap::new);
            
            // Set the halfedge from u to v to point to this face
            self.halfedge.get_mut(&u).unwrap().insert(v, Some(face_key));
            
            // Set the reverse halfedge from v to u (boundary halfedge if no face exists)
            if !self.halfedge.get(&v).unwrap().contains_key(&u) {
                self.halfedge.get_mut(&v).unwrap().insert(u, None);
            }
        }
        
        Some(face_key)
    }

    /// Get the position of a vertex.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// 
    /// # Returns
    /// The position of the vertex, or None if vertex doesn't exist
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v = mesh.add_vertex(Point::new(1.0, 2.0, 3.0), None);
    /// let pos = mesh.vertex_position(v).unwrap();
    /// assert_eq!(pos.x, 1.0);
    /// assert_eq!(pos.y, 2.0);
    /// assert_eq!(pos.z, 3.0);
    /// ```
    pub fn vertex_position(&self, vertex_key: usize) -> Option<Point> {
        self.vertex.get(&vertex_key).map(|v| v.position())
    }

    /// Get the vertices of a face.
    /// 
    /// # Arguments
    /// * `face_key` - The key of the face
    /// 
    /// # Returns
    /// A list of vertex keys defining the face, or None if face doesn't exist
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let vertices = mesh.face_vertices(f).unwrap();
    /// assert_eq!(*vertices, vec![v0, v1, v2]);
    /// ```
    pub fn face_vertices(&self, face_key: usize) -> Option<&Vec<usize>> {
        self.face.get(&face_key)
    }
    
    /// Get all face data as an iterator over (face_key, face_vertices) pairs.
    /// 
    /// This method provides access to all faces in the mesh for iteration.
    /// Useful for converting to other mesh representations.
    /// 
    /// # Returns
    /// An iterator over (face_key, face_vertices) pairs
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// 
    /// for (face_key, face_vertices) in mesh.get_face_data() {
    ///     println!("Face {}: {:?}", face_key, face_vertices);
    /// }
    /// ```
    pub fn get_face_data(&self) -> impl Iterator<Item = (&usize, &Vec<usize>)> {
        self.face.iter()
    }

    /// Check if a vertex is on the boundary of the mesh.
    /// 
    /// A vertex is on the boundary if it has at least one incident halfedge
    /// that points to None (no face), indicating a boundary edge.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// 
    /// # Returns
    /// True if the vertex is on the boundary
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// assert!(mesh.is_vertex_on_boundary(v0)); // All vertices of a single triangle are on boundary
    /// ```
    pub fn is_vertex_on_boundary(&self, vertex_key: usize) -> bool {
        if let Some(neighbors) = self.halfedge.get(&vertex_key) {
            for face_option in neighbors.values() {
                if face_option.is_none() {
                    return true; // This halfedge points to no face, so it's on the boundary
                }
            }
        }
        false
    }

    /// Get the neighbors of a vertex.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// 
    /// # Returns
    /// A vector of vertex keys that are adjacent to the given vertex
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// let neighbors = mesh.vertex_neighbors(v0);
    /// assert_eq!(neighbors.len(), 2);
    /// assert!(neighbors.contains(&v1));
    /// assert!(neighbors.contains(&v2));
    /// ```
    pub fn vertex_neighbors(&self, vertex_key: usize) -> Vec<usize> {
        if let Some(neighbors) = self.halfedge.get(&vertex_key) {
            neighbors.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get all faces incident to a vertex.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// 
    /// # Returns
    /// A vector of face keys that are incident to the given vertex
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let faces = mesh.vertex_faces(v0);
    /// assert_eq!(faces.len(), 1);
    /// assert_eq!(faces[0], f);
    /// ```
    pub fn vertex_faces(&self, vertex_key: usize) -> Vec<usize> {
        let mut faces = Vec::new();
        
        for (face_key, vertices) in &self.face {
            if vertices.contains(&vertex_key) {
                faces.push(*face_key);
            }
        }
        
        faces
    }

    /// Compute the normal vector of a face.
    /// 
    /// The normal is computed using the cross product of the first two edges of the face.
    /// For planar faces, this gives the unit normal vector.
    /// 
    /// # Arguments
    /// * `face_key` - The key of the face
    /// 
    /// # Returns
    /// The unit normal vector of the face, or None if the face is invalid
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point, Vector};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normal = mesh.face_normal(f).unwrap();
    /// assert!((normal.z - 1.0).abs() < 1e-10); // Normal should point in +Z direction
    /// ```
    pub fn face_normal(&self, face_key: usize) -> Option<Vector> {
        let vertices = self.face.get(&face_key)?;
        if vertices.len() < 3 {
            return None;
        }
        
        let p0 = self.vertex_position(vertices[0])?;
        let p1 = self.vertex_position(vertices[1])?;
        let p2 = self.vertex_position(vertices[2])?;
        
        let edge1 = Vector::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z);
        let edge2 = Vector::new(p2.x - p0.x, p2.y - p0.y, p2.z - p0.z);
        
        let mut normal = edge1.cross(&edge2);
        normal.unitize();
        Some(normal)
    }

    /// Compute the angle subtended by a vertex in a face.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// * `face_key` - The key of the face
    /// 
    /// # Returns
    /// The angle in radians, or None if the vertex is not part of the face
    fn vertex_angle_in_face(&self, vertex_key: usize, face_key: usize) -> Option<f64> {
        let face_vertices = self.face_vertices(face_key)?;
        let vertex_pos = self.vertex_position(vertex_key)?;
        
        // Find the vertex in the face
        let vertex_index = face_vertices.iter().position(|&v| v == vertex_key)?;
        let n = face_vertices.len();
        
        // Get the two adjacent vertices
        let prev_vertex = face_vertices[(vertex_index + n - 1) % n];
        let next_vertex = face_vertices[(vertex_index + 1) % n];
        
        let prev_pos = self.vertex_position(prev_vertex)?;
        let next_pos = self.vertex_position(next_vertex)?;
        
        // Compute vectors from vertex to its neighbors
        let v1 = Vector::new(
            prev_pos.x - vertex_pos.x,
            prev_pos.y - vertex_pos.y,
            prev_pos.z - vertex_pos.z,
        );
        let v2 = Vector::new(
            next_pos.x - vertex_pos.x,
            next_pos.y - vertex_pos.y,
            next_pos.z - vertex_pos.z,
        );
        
        // Compute angle using dot product
        let dot = v1.dot(&v2);
        let len1 = v1.length();
        let len2 = v2.length();
        
        if len1 > 0.0 && len2 > 0.0 {
            let cos_angle = dot / (len1 * len2);
            // Clamp to avoid numerical issues
            let cos_angle = cos_angle.max(-1.0).min(1.0);
            Some(cos_angle.acos())
        } else {
            None
        }
    }
    
    /// Compute the normal vector of a vertex with configurable weighting.
    /// 
    /// The vertex normal is computed as the weighted average of the normals of all faces
    /// incident to the vertex, using the specified weighting scheme.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// * `weighting` - The weighting scheme to use
    /// 
    /// # Returns
    /// The unit normal vector of the vertex, or None if the vertex is invalid
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// use openmodel::geometry::mesh::NormalWeighting;
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normal = mesh.vertex_normal_weighted(v0, NormalWeighting::Area).unwrap();
    /// assert!((normal.z - 1.0).abs() < 1e-10); // Normal should point in +Z direction
    /// ```
    pub fn vertex_normal_weighted(&self, vertex_key: usize, weighting: NormalWeighting) -> Option<Vector> {
        let faces = self.vertex_faces(vertex_key);
        if faces.is_empty() {
            return None;
        }
        
        let mut normal_sum = Vector::new(0.0, 0.0, 0.0);
        let mut total_weight = 0.0;
        
        for face_key in faces {
            if let Some(face_normal) = self.face_normal(face_key) {
                let weight = match weighting {
                    NormalWeighting::Area => self.face_area(face_key).unwrap_or(0.0),
                    NormalWeighting::Angle => self.vertex_angle_in_face(vertex_key, face_key).unwrap_or(0.0),
                    NormalWeighting::Uniform => 1.0,
                };
                
                normal_sum.x += face_normal.x * weight;
                normal_sum.y += face_normal.y * weight;
                normal_sum.z += face_normal.z * weight;
                total_weight += weight;
            }
        }
        
        if total_weight > 0.0 {
            normal_sum.x /= total_weight;
            normal_sum.y /= total_weight;
            normal_sum.z /= total_weight;
            normal_sum.unitize();
            Some(normal_sum)
        } else {
            None
        }
    }
    
    /// Compute the normal vector of a vertex using area weighting (default).
    /// 
    /// This is a convenience method that uses area weighting for backward compatibility.
    /// 
    /// # Arguments
    /// * `vertex_key` - The key of the vertex
    /// 
    /// # Returns
    /// The unit normal vector of the vertex, or None if the vertex is invalid
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normal = mesh.vertex_normal(v0).unwrap();
    /// assert!((normal.z - 1.0).abs() < 1e-10); // Normal should point in +Z direction
    /// ```
    pub fn vertex_normal(&self, vertex_key: usize) -> Option<Vector> {
        self.vertex_normal_weighted(vertex_key, NormalWeighting::Area)
    }

    /// Compute the area of a face.
    /// 
    /// For faces with more than 3 vertices, the area is computed by triangulating
    /// the face and summing the areas of the triangles.
    /// 
    /// # Arguments
    /// * `face_key` - The key of the face
    /// 
    /// # Returns
    /// The area of the face, or None if the face is invalid
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let area = mesh.face_area(f).unwrap();
    /// assert!((area - 0.5).abs() < 1e-10); // Area of triangle with base=1, height=1
    /// ```
    pub fn face_area(&self, face_key: usize) -> Option<f64> {
        let vertices = self.face.get(&face_key)?;
        if vertices.len() < 3 {
            return None;
        }
        
        let mut area = 0.0;
        
        // Triangulate the face and sum triangle areas
        for i in 1..vertices.len() - 1 {
            let p0 = self.vertex_position(vertices[0])?;
            let p1 = self.vertex_position(vertices[i])?;
            let p2 = self.vertex_position(vertices[i + 1])?;
            
            let edge1 = Vector::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z);
            let edge2 = Vector::new(p2.x - p0.x, p2.y - p0.y, p2.z - p0.z);
            
            let cross = edge1.cross(&edge2);
            area += cross.length() * 0.5;
        }
        
        Some(area)
    }

    /// Compute normals for all faces in the mesh.
    /// 
    /// # Returns
    /// A HashMap mapping face keys to their normal vectors
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normals = mesh.face_normals();
    /// assert_eq!(normals.len(), 1);
    /// assert!(normals.contains_key(&f));
    /// ```
    pub fn face_normals(&self) -> HashMap<usize, Vector> {
        let mut normals = HashMap::new();
        
        for face_key in self.face.keys() {
            if let Some(normal) = self.face_normal(*face_key) {
                normals.insert(*face_key, normal);
            }
        }
        
        normals
    }

    /// Compute normals for all vertices in the mesh with configurable weighting.
    /// 
    /// # Arguments
    /// * `weighting` - The weighting scheme to use
    /// 
    /// # Returns
    /// A HashMap mapping vertex keys to their normal vectors
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// use openmodel::geometry::mesh::NormalWeighting;
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normals = mesh.vertex_normals_weighted(NormalWeighting::Angle);
    /// assert_eq!(normals.len(), 3);
    /// assert!(normals.contains_key(&v0));
    /// ```
    pub fn vertex_normals_weighted(&self, weighting: NormalWeighting) -> HashMap<usize, Vector> {
        let mut normals = HashMap::new();
        
        for vertex_key in self.vertex.keys() {
            if let Some(normal) = self.vertex_normal_weighted(*vertex_key, weighting) {
                normals.insert(*vertex_key, normal);
            }
        }
        
        normals
    }
    
    /// Compute normals for all vertices in the mesh using area weighting (default).
    /// 
    /// This is a convenience method that uses area weighting for backward compatibility.
    /// 
    /// # Returns
    /// A HashMap mapping vertex keys to their normal vectors
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
    /// let normals = mesh.vertex_normals();
    /// assert_eq!(normals.len(), 3);
    /// assert!(normals.contains_key(&v0));
    /// ```
    pub fn vertex_normals(&self) -> HashMap<usize, Vector> {
        self.vertex_normals_weighted(NormalWeighting::Area)
    }
    
    /// Create a pipe mesh from a line segment.
    /// 
    /// Creates a cylindrical mesh along a line segment defined by two points.
    /// The pipe has circular cross-sections perpendicular to the line direction.
    /// This optimized version uses hardcoded coordinates for a 12-sided cylinder
    /// and transforms them directly for maximum performance.
    /// 
    /// # Arguments
    /// * `start` - Starting point of the line
    /// * `end` - Ending point of the line
    /// * `radius` - Radius of the pipe
    /// 
    /// # Returns
    /// A new Mesh representing a 12-sided pipe along the specified line
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let start = Point::new(0.0, 0.0, 0.0);
    /// let end = Point::new(0.0, 0.0, 1.0);
    /// let radius = 0.2;
    /// let pipe_mesh = Mesh::create_pipe(start, end, radius);
    /// ```
    pub fn create_pipe(start: Point, end: Point, radius: f64) -> Self {
        let mut mesh = Mesh::new();
        
        // Compute transformation matrix for the pipe segment
        let direction = Vector::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let length = direction.length();
        
        if length < 1e-6 {
            return mesh; // Degenerate case
        }
        
        let axis = direction.normalize();
        
        // Create transformation matrix
        // 1. Scale: radius in X,Y and length in Z
        let scale = Xform::scaling(radius, radius, length);
        
        // 2. Rotation: align Z-axis with line direction
        let z_axis = Vector::new(0.0, 0.0, 1.0);
        let rotation = if (axis.dot(&z_axis) - 1.0).abs() < 1e-6 {
            // Already aligned with Z
            Xform::identity()
        } else if (axis.dot(&z_axis) + 1.0).abs() < 1e-6 {
            // Opposite to Z, rotate 180 degrees around X
            Xform::rotation_x(PI)
        } else {
            // General case: rotate around cross product
            let rotation_axis = z_axis.cross(&axis).normalize();
            let angle = z_axis.dot(&axis).acos();
            Xform::rotation(&rotation_axis, angle)
        };
        
        // 3. Translation: move to midpoint of segment
        let midpoint = Point::new(
            (start.x + end.x) / 2.0,
            (start.y + end.y) / 2.0,
            (start.z + end.z) / 2.0,
        );
        let translation = Xform::translation(midpoint.x, midpoint.y, midpoint.z);
        
        // Combine transformations: T * R * S
        let transform = translation * rotation * scale;
        
        // Hardcoded vertices for 12-sided unit cylinder
        let unit_vertices = vec![
            Point::new(0.0, 0.0, -0.5), // Bottom center
            Point::new(1.0000000000, 0.0000000000, -0.5), // Bottom rim 0
            Point::new(0.8660254038, 0.5000000000, -0.5), // Bottom rim 1
            Point::new(0.5000000000, 0.8660254038, -0.5), // Bottom rim 2
            Point::new(0.0000000000, 1.0000000000, -0.5), // Bottom rim 3
            Point::new(-0.5000000000, 0.8660254038, -0.5), // Bottom rim 4
            Point::new(-0.8660254038, 0.5000000000, -0.5), // Bottom rim 5
            Point::new(-1.0000000000, 0.0000000000, -0.5), // Bottom rim 6
            Point::new(-0.8660254038, -0.5000000000, -0.5), // Bottom rim 7
            Point::new(-0.5000000000, -0.8660254038, -0.5), // Bottom rim 8
            Point::new(-0.0000000000, -1.0000000000, -0.5), // Bottom rim 9
            Point::new(0.5000000000, -0.8660254038, -0.5), // Bottom rim 10
            Point::new(0.8660254038, -0.5000000000, -0.5), // Bottom rim 11
            Point::new(0.0, 0.0, 0.5), // Top center
            Point::new(1.0000000000, 0.0000000000, 0.5), // Top rim 0
            Point::new(0.8660254038, 0.5000000000, 0.5), // Top rim 1
            Point::new(0.5000000000, 0.8660254038, 0.5), // Top rim 2
            Point::new(0.0000000000, 1.0000000000, 0.5), // Top rim 3
            Point::new(-0.5000000000, 0.8660254038, 0.5), // Top rim 4
            Point::new(-0.8660254038, 0.5000000000, 0.5), // Top rim 5
            Point::new(-1.0000000000, 0.0000000000, 0.5), // Top rim 6
            Point::new(-0.8660254038, -0.5000000000, 0.5), // Top rim 7
            Point::new(-0.5000000000, -0.8660254038, 0.5), // Top rim 8
            Point::new(-0.0000000000, -1.0000000000, 0.5), // Top rim 9
            Point::new(0.5000000000, -0.8660254038, 0.5), // Top rim 10
            Point::new(0.8660254038, -0.5000000000, 0.5), // Top rim 11
        ];
        
        // Transform and add all vertices to mesh
        let mut vertex_keys = Vec::with_capacity(unit_vertices.len());
        for vertex in unit_vertices {
            let transformed_vertex = transform.transform_point(&vertex);
            vertex_keys.push(mesh.add_vertex(transformed_vertex, None));
        }
        
        // Hardcoded faces for 12-sided unit cylinder
        let faces = vec![
            vec![0, 2, 1], vec![0, 3, 2], vec![0, 4, 3], vec![0, 5, 4], // Bottom cap
            vec![0, 6, 5], vec![0, 7, 6], vec![0, 8, 7], vec![0, 9, 8],
            vec![0, 10, 9], vec![0, 11, 10], vec![0, 12, 11], vec![0, 1, 12],
            vec![13, 14, 15], vec![13, 15, 16], vec![13, 16, 17], vec![13, 17, 18], // Top cap
            vec![13, 18, 19], vec![13, 19, 20], vec![13, 20, 21], vec![13, 21, 22],
            vec![13, 22, 23], vec![13, 23, 24], vec![13, 24, 25], vec![13, 25, 14],
            vec![1, 2, 14], vec![14, 2, 15], vec![2, 3, 15], vec![15, 3, 16], // Sides
            vec![3, 4, 16], vec![16, 4, 17], vec![4, 5, 17], vec![17, 5, 18],
            vec![5, 6, 18], vec![18, 6, 19], vec![6, 7, 19], vec![19, 7, 20],
            vec![7, 8, 20], vec![20, 8, 21], vec![8, 9, 21], vec![21, 9, 22],
            vec![9, 10, 22], vec![22, 10, 23], vec![10, 11, 23], vec![23, 11, 24],
            vec![11, 12, 24], vec![24, 12, 25], vec![12, 1, 25], vec![25, 1, 14],
        ];
        
        // Add all faces to mesh
        for face_indices in faces {
            let face_vertices: Vec<usize> = face_indices.iter().map(|&i| vertex_keys[i]).collect();
            mesh.add_face(face_vertices, None);
        }
        
        mesh
    }



    /// Create a halfedge mesh from a list of polygons.
    /// 
    /// Each polygon is defined by a list of 3D points. Vertices are merged
    /// based on coordinate precision to avoid duplicates.
    /// 
    /// # Arguments
    /// * `polygons` - List of polygons, each defined by a list of 3D points
    /// * `precision` - Precision for merging vertices (default: 1e-10)
    /// 
    /// # Returns
    /// A new halfedge mesh constructed from the polygons
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let triangle = vec![
    ///     Point::new(0.0, 0.0, 0.0),
    ///     Point::new(1.0, 0.0, 0.0),
    ///     Point::new(0.0, 1.0, 0.0),
    /// ];
    /// let mesh = Mesh::from_polygons(vec![triangle], None);
    /// assert_eq!(mesh.number_of_vertices(), 3);
    /// assert_eq!(mesh.number_of_faces(), 1);
    /// ```
    pub fn from_polygons(polygons: Vec<Vec<Point>>, precision: Option<f64>) -> Self {
        let precision = precision.unwrap_or(1e-10);
        let mut mesh = Mesh::new();
        let mut vertex_map: HashMap<String, usize> = HashMap::new();
        
        for polygon in polygons {
            if polygon.len() < 3 {
                continue; // Skip invalid polygons
            }
            
            let mut face_vertices = Vec::new();
            
            for point in polygon {
                // Create a key for the point based on its coordinates with precision
                let key = format!(
                    "{:.10}_{:.10}_{:.10}",
                    (point.x / precision).round() * precision,
                    (point.y / precision).round() * precision,
                    (point.z / precision).round() * precision
                );
                
                // Check if vertex already exists
                let vertex_key = if let Some(&existing_key) = vertex_map.get(&key) {
                    existing_key
                } else {
                    // Add new vertex
                    let new_key = mesh.add_vertex(point, None);
                    vertex_map.insert(key, new_key);
                    new_key
                };
                
                face_vertices.push(vertex_key);
            }
            
            // Add the face if it has valid vertices
            if face_vertices.len() >= 3 {
                mesh.add_face(face_vertices, None);
            }
        }
        
        mesh
    }
    
    /// Create a mesh from a polygon using ear clipping triangulation.
    /// 
    /// The polygon is assumed to be planar and non-self-intersecting.
    /// Points should be provided in counter-clockwise order for correct triangulation.
    /// 
    /// # Arguments
    /// * `polygon_points` - List of 3D points defining the polygon boundary
    /// 
    /// # Returns
    /// A new halfedge mesh representing the triangulated polygon
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let square = vec![
    ///     Point::new(0.0, 0.0, 0.0),
    ///     Point::new(1.0, 0.0, 0.0),
    ///     Point::new(1.0, 1.0, 0.0),
    ///     Point::new(0.0, 1.0, 0.0),
    /// ];
    /// let mesh = Mesh::from_polygon_earclip(square);
    /// assert_eq!(mesh.number_of_vertices(), 4);
    /// assert_eq!(mesh.number_of_faces(), 2); // Square triangulated into 2 triangles
    /// ```
    pub fn from_polygon_earclip(polygon_points: Vec<Point>) -> Self {
        if polygon_points.len() < 3 {
            return Self::new();
        }
        
        // Convert 3D points to 2D for triangulation (assuming planar polygon)
        let points_2d: Vec<[f64; 2]> = polygon_points.iter()
            .map(|p| [p.x, p.y])
            .collect();
        
        // Perform ear clipping triangulation
        let triangles = match earclip_triangulate(&points_2d) {
            Ok(tris) => tris,
            Err(_) => return Self::new(), // Return empty mesh on error
        };
        
        // Create mesh from triangulated result
        let mut mesh = Self::new();
        
        // Add all vertices
        let mut vertex_keys = Vec::new();
        for point in polygon_points {
            let key = mesh.add_vertex(point, None);
            vertex_keys.push(key);
        }
        
        // Add triangular faces
        for triangle in triangles {
            let face_vertices = vec![
                vertex_keys[triangle[0]],
                vertex_keys[triangle[1]], 
                vertex_keys[triangle[2]]
            ];
            mesh.add_face(face_vertices, None);
        }
        
        mesh
    }
    
    /// Extract all unique edges from the mesh as Line objects.
    /// 
    /// This method traverses all faces and collects unique edges (no duplicates).
    /// Each edge is represented as a Line connecting two vertices.
    /// 
    /// # Returns
    /// A vector of Line objects representing the unique edges of the mesh
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// 
    /// let edges = mesh.extract_edges_as_lines();
    /// assert_eq!(edges.len(), 3); // Triangle has 3 edges
    /// ```
    pub fn extract_edges_as_lines(&self) -> Vec<crate::geometry::Line> {
        use std::collections::HashSet;
        use crate::geometry::Line;
        
        let mut unique_edges = HashSet::new();
        let mut lines = Vec::new();
        
        // Iterate through all faces to collect edges
        for face_vertices in self.face.values() {
            let n = face_vertices.len();
            for i in 0..n {
                let v1 = face_vertices[i];
                let v2 = face_vertices[(i + 1) % n];
                
                // Create a normalized edge (smaller vertex index first) to avoid duplicates
                let edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
                
                if unique_edges.insert(edge) {
                    // Get vertex positions
                    if let (Some(pos1), Some(pos2)) = (
                        self.vertex_position(v1),
                        self.vertex_position(v2)
                    ) {
                        let line = Line::new(
                            pos1.x, pos1.y, pos1.z,
                            pos2.x, pos2.y, pos2.z
                        );
                        lines.push(line);
                    }
                }
            }
        }
        
        lines
    }
    
    /// Extract all unique edges from the mesh as pipe meshes (cylinders).
    /// 
    /// This method creates cylindrical pipe meshes for each unique edge in the mesh.
    /// The pipes use the specified radius and number of sides for the cross-section.
    /// 
    /// # Arguments
    /// * `radius` - Radius of the pipe cylinders
    /// * `sides` - Number of sides for the cylindrical cross-section (default: 8)
    /// 
    /// # Returns
    /// A vector of Mesh objects representing the pipe meshes for each edge
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// let mut mesh = Mesh::new();
    /// let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    /// let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    /// let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    /// mesh.add_face(vec![v0, v1, v2], None);
    /// 
    /// let pipe_meshes = mesh.extract_edges_as_pipes(0.05, Some(8));
    /// assert_eq!(pipe_meshes.len(), 3); // Triangle has 3 edges
    /// ```
    pub fn extract_edges_as_pipes(&self, radius: f64, _sides: Option<usize>) -> Vec<Mesh> {
        use std::collections::HashSet;
        let mut unique_edges = HashSet::new();
        let mut pipe_meshes = Vec::new();
        
        // Iterate through all faces to collect edges
        for face_vertices in self.face.values() {
            let n = face_vertices.len();
            for i in 0..n {
                let v1 = face_vertices[i];
                let v2 = face_vertices[(i + 1) % n];
                
                // Create a normalized edge (smaller vertex index first) to avoid duplicates
                let edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
                
                if unique_edges.insert(edge) {
                    // Get vertex positions
                    if let (Some(pos1), Some(pos2)) = (
                        self.vertex_position(v1),
                        self.vertex_position(v2)
                    ) {
                        // Create pipe mesh for this edge
                        let pipe_mesh = Mesh::create_pipe(pos1, pos2, radius);
                        pipe_meshes.push(pipe_mesh);
                    }
                }
            }
        }
        
        pipe_meshes
    }
    
    /// Create a mesh from a list of polygons with automatic duplicate point removal.
    /// 
    /// This method takes a list of polygons (each defined by a list of 3D points) and
    /// creates a unified mesh by merging duplicate vertices based on coordinate precision.
    /// Polygons with 4 or more vertices are automatically triangulated using ear clipping.
    /// 
    /// # Arguments
    /// * `polygons` - List of polygons, each defined by a list of 3D points
    /// * `precision` - Precision for merging vertices (default: 1e-10)
    /// 
    /// # Returns
    /// A new halfedge mesh constructed from the polygons with merged vertices
    /// 
    /// # Example
    /// 
    /// ```
    /// use openmodel::geometry::{Mesh, Point};
    /// 
    /// // Create a cube using quad faces
    /// let cube_faces = vec![
    ///     // Front face
    ///     vec![
    ///         Point::new(0.0, 0.0, 0.0),
    ///         Point::new(1.0, 0.0, 0.0),
    ///         Point::new(1.0, 1.0, 0.0),
    ///         Point::new(0.0, 1.0, 0.0),
    ///     ],
    ///     // Back face
    ///     vec![
    ///         Point::new(1.0, 0.0, 1.0),
    ///         Point::new(0.0, 0.0, 1.0),
    ///         Point::new(0.0, 1.0, 1.0),
    ///         Point::new(1.0, 1.0, 1.0),
    ///     ],
    /// ];
    /// 
    /// let mesh = Mesh::from_polygons_with_merge(cube_faces, None);
    /// assert_eq!(mesh.number_of_vertices(), 8); // Cube has 8 unique vertices
    /// ```
    pub fn from_polygons_with_merge(polygons: Vec<Vec<Point>>, precision: Option<f64>) -> Self {
        use std::collections::HashMap;
        
        let precision = precision.unwrap_or(1e-10);
        let mut mesh = Self::new();
        
        // Map to store unique vertices and their keys
        let mut vertex_map: HashMap<String, usize> = HashMap::new();
        let mut unique_vertices: Vec<Point> = Vec::new();
        
        // Helper function to create a key for a point based on precision
        let point_key = |p: &Point| -> String {
            let factor = 1.0 / precision;
            let x = (p.x * factor).round() as i64;
            let y = (p.y * factor).round() as i64;
            let z = (p.z * factor).round() as i64;
            format!("{},{},{}", x, y, z)
        };
        
        // First pass: collect all unique vertices
        for polygon in &polygons {
            for point in polygon {
                let key = point_key(point);
                if !vertex_map.contains_key(&key) {
                    let vertex_key = unique_vertices.len();
                    vertex_map.insert(key, vertex_key);
                    unique_vertices.push(point.clone());
                }
            }
        }
        
        // Add all unique vertices to the mesh
        let mut mesh_vertex_keys = Vec::new();
        for vertex in unique_vertices {
            let key = mesh.add_vertex(vertex, None);
            mesh_vertex_keys.push(key);
        }
        
        // Second pass: create faces using the merged vertices
        for polygon in polygons {
            if polygon.len() < 3 {
                continue; // Skip degenerate polygons
            }
            
            // Map polygon points to mesh vertex keys
            let face_vertices: Vec<usize> = polygon.iter()
                .map(|point| {
                    let key = point_key(point);
                    let vertex_index = vertex_map[&key];
                    mesh_vertex_keys[vertex_index]
                })
                .collect();
            
            if polygon.len() == 3 {
                // Triangle - add directly
                mesh.add_face(face_vertices, None);
            } else {
                // Polygon with 4+ vertices - triangulate using ear clipping
                // First, project the 3D polygon to 2D for triangulation
                let points_2d = project_polygon_to_2d(&polygon);
                
                match earclip_triangulate(&points_2d) {
                    Ok(triangles) => {
                        for triangle in triangles {
                            let triangle_vertices = vec![
                                face_vertices[triangle[0]],
                                face_vertices[triangle[1]],
                                face_vertices[triangle[2]]
                            ];
                            mesh.add_face(triangle_vertices, None);
                        }
                    }
                    Err(_) => {
                        // If triangulation fails, use simple fan triangulation as fallback
                        for i in 1..polygon.len() - 1 {
                            let triangle_vertices = vec![
                                face_vertices[0],
                                face_vertices[i],
                                face_vertices[i + 1]
                            ];
                            mesh.add_face(triangle_vertices, None);
                        }
                    }
                }
            }
        }
        
        mesh
    }
}

/// Project a 3D polygon to 2D for triangulation.
/// 
/// This function finds the best 2D projection plane for the polygon by calculating
/// the polygon's normal vector and projecting to the plane with the largest component.
fn project_polygon_to_2d(polygon: &[Point]) -> Vec<[f64; 2]> {
    use crate::primitives::Vector;
    
    if polygon.len() < 3 {
        return Vec::new();
    }
    
    // Calculate polygon normal using Newell's method for robustness
    let mut normal = Vector::new(0.0, 0.0, 0.0);
    for i in 0..polygon.len() {
        let current = &polygon[i];
        let next = &polygon[(i + 1) % polygon.len()];
        
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    
    // Normalize the normal vector
    let length = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    if length > 1e-10 {
        normal.x /= length;
        normal.y /= length;
        normal.z /= length;
    } else {
        // Degenerate polygon, use XY plane
        normal = Vector::new(0.0, 0.0, 1.0);
    }
    
    // Choose the projection plane based on the largest component of the normal
    let abs_x = normal.x.abs();
    let abs_y = normal.y.abs();
    let abs_z = normal.z.abs();
    
    let points_2d: Vec<[f64; 2]> = if abs_z >= abs_x && abs_z >= abs_y {
        // Project to XY plane (drop Z)
        polygon.iter().map(|p| [p.x, p.y]).collect()
    } else if abs_y >= abs_x && abs_y >= abs_z {
        // Project to XZ plane (drop Y)
        polygon.iter().map(|p| [p.x, p.z]).collect()
    } else {
        // Project to YZ plane (drop X)
        polygon.iter().map(|p| [p.y, p.z]).collect()
    };
    
    points_2d
}

// Ear clipping triangulation implementation
// Based on COMPAS reference: https://github.com/compas-dev/compas/blob/main/src/compas/geometry/triangulation_earclip.py

/// Triangulate a polygon using the ear clipping algorithm
/// 
/// # Arguments
/// * `points` - Array of 2D points defining the polygon boundary
/// 
/// # Returns
/// Result containing triangles as arrays of vertex indices, or error message
fn earclip_triangulate(points: &[[f64; 2]]) -> Result<Vec<[usize; 3]>, &'static str> {
    if points.len() < 3 {
        return Err("Polygon must have at least 3 vertices");
    }
    
    if points.len() == 3 {
        return Ok(vec![[0, 1, 2]]);
    }
    
    // Check winding order and reverse if clockwise
    let mut polygon_points = points.to_vec();
    let signed_area = compute_signed_area(&polygon_points);
    let was_reversed = signed_area > 0.0; // Note: > 0 means clockwise in our coordinate system
    
    if was_reversed {
        polygon_points.reverse();
    }
    
    // Simple ear clipping implementation
    let mut triangles = Vec::new();
    let mut indices: Vec<usize> = (0..polygon_points.len()).collect();
    
    while indices.len() > 3 {
        let mut ear_found = false;
        
        for i in 0..indices.len() {
            let prev_idx = if i == 0 { indices.len() - 1 } else { i - 1 };
            let next_idx = (i + 1) % indices.len();
            
            let prev = indices[prev_idx];
            let curr = indices[i];
            let next = indices[next_idx];
            
            // Check if this forms a valid ear
            if is_ear(&polygon_points, &indices, prev, curr, next) {
                // Add triangle
                triangles.push([prev, curr, next]);
                
                // Remove the ear vertex
                indices.remove(i);
                ear_found = true;
                break;
            }
        }
        
        if !ear_found {
            return Err("Unable to find valid ear for triangulation");
        }
    }
    
    // Add the final triangle
    if indices.len() == 3 {
        triangles.push([indices[0], indices[1], indices[2]]);
    }
    
    // If we reversed the points, adjust triangle indices back
    if was_reversed {
        let n = points.len();
        for triangle in &mut triangles {
            triangle[0] = n - 1 - triangle[0];
            triangle[1] = n - 1 - triangle[1];
            triangle[2] = n - 1 - triangle[2];
        }
    }
    
    Ok(triangles)
}

/// Check if three consecutive vertices form a valid ear
fn is_ear(points: &[[f64; 2]], indices: &[usize], prev: usize, curr: usize, next: usize) -> bool {
    let a = points[prev];
    let b = points[curr];
    let c = points[next];
    
    // Check if the angle at curr is convex (less than 180 degrees)
    let ab = [b[0] - a[0], b[1] - a[1]];
    let bc = [c[0] - b[0], c[1] - b[1]];
    let cross = ab[0] * bc[1] - ab[1] * bc[0];
    
    if cross <= 0.0 {
        return false; // Not convex
    }
    
    // Check if any other vertex lies inside the triangle
    for &idx in indices {
        if idx != prev && idx != curr && idx != next {
            if point_in_triangle(points[idx], a, b, c) {
                return false;
            }
        }
    }
    
    true
}

/// Check if a point is inside a triangle using barycentric coordinates
fn point_in_triangle(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    
    !(has_neg && has_pos)
}

/// Helper function for point-in-triangle test
fn sign(p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> f64 {
    (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
}

/// Compute the signed area of a 2D polygon
/// Positive area indicates counter-clockwise winding, negative indicates clockwise
fn compute_signed_area(points: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0;
    let n = points.len();
    
    for i in 0..n {
        let p0 = points[i];
        let p1 = points[(i + 1) % n];
        sum += (p1[0] - p0[0]) * (p1[1] + p0[1]);
    }
    
    sum * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_halfedge_mesh_new() {
        let mesh = Mesh::new();
        assert_eq!(mesh.number_of_vertices(), 0);
        assert_eq!(mesh.number_of_faces(), 0);
        assert!(mesh.is_empty());
        assert_eq!(mesh.euler(), 0);
    }

    #[test]
    fn test_add_vertex() {
        let mut mesh = Mesh::new();
        let vertex_key = mesh.add_vertex(Point::new(1.0, 2.0, 3.0), None);
        assert_eq!(mesh.number_of_vertices(), 1);
        assert!(!mesh.is_empty());
        
        let pos = mesh.vertex_position(vertex_key).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
        assert_eq!(pos.z, 3.0);
    }

    #[test]
    fn test_add_vertex_with_specific_key() {
        let mut mesh = Mesh::new();
        let vertex_key = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), Some(42));
        assert_eq!(vertex_key, 42);
        assert_eq!(mesh.number_of_vertices(), 1);
    }

    #[test]
    fn test_add_face() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _face_key = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        assert_eq!(mesh.number_of_faces(), 1);
        assert_eq!(mesh.number_of_edges(), 3);
        assert_eq!(mesh.euler(), 1); // V=3, E=3, F=1 -> 3-3+1=1
    }

    #[test]
    fn test_add_face_invalid() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        
        // Too few vertices
        assert!(mesh.add_face(vec![v0, v1], None).is_none());
        
        // Non-existent vertex
        assert!(mesh.add_face(vec![v0, v1, 999], None).is_none());
        
        // Duplicate vertices
        assert!(mesh.add_face(vec![v0, v1, v0], None).is_none());
    }

    #[test]
    fn test_face_vertices() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let vertices = mesh.face_vertices(f).unwrap();
        assert_eq!(vertices, &vec![v0, v1, v2]);
    }

    #[test]
    fn test_vertex_neighbors() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        mesh.add_face(vec![v0, v1, v2], None);
        
        let neighbors = mesh.vertex_neighbors(v0);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&v1));
        assert!(neighbors.contains(&v2));
    }

    #[test]
    fn test_vertex_faces() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        let v3 = mesh.add_vertex(Point::new(1.0, 1.0, 0.0), None);
        
        let f1 = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let f2 = mesh.add_face(vec![v1, v3, v2], None).unwrap();
        
        let faces = mesh.vertex_faces(v1);
        assert_eq!(faces.len(), 2);
        assert!(faces.contains(&f1));
        assert!(faces.contains(&f2));
    }

    #[test]
    fn test_is_vertex_on_boundary() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        mesh.add_face(vec![v0, v1, v2], None);
        
        // All vertices of a single triangle are on boundary
        assert!(mesh.is_vertex_on_boundary(v0));
        assert!(mesh.is_vertex_on_boundary(v1));
        assert!(mesh.is_vertex_on_boundary(v2));
    }

    #[test]
    fn test_face_normal() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normal = mesh.face_normal(f).unwrap();
        
        // Normal should point in +Z direction for this triangle
        assert!((normal.z - 1.0).abs() < 1e-10);
        assert!(normal.x.abs() < 1e-10);
        assert!(normal.y.abs() < 1e-10);
    }

    #[test]
    fn test_vertex_normal() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normal = mesh.vertex_normal(v0).unwrap();
        
        // Normal should point in +Z direction
        assert!((normal.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_face_area() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let area = mesh.face_area(f).unwrap();
        
        // Area of triangle with base=1, height=1 should be 0.5
        assert!((area - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_face_normals() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normals = mesh.face_normals();
        
        assert_eq!(normals.len(), 1);
        assert!(normals.contains_key(&f));
        let normal = normals.get(&f).unwrap();
        assert!((normal.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vertex_normals() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normals = mesh.vertex_normals();
        
        assert_eq!(normals.len(), 3);
        assert!(normals.contains_key(&v0));
        assert!(normals.contains_key(&v1));
        assert!(normals.contains_key(&v2));
    }
    
    #[test]
    fn test_vertex_normal_weighted_area() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normal = mesh.vertex_normal_weighted(v0, NormalWeighting::Area).unwrap();
        
        // Should be the same as the default vertex_normal method
        let normal_default = mesh.vertex_normal(v0).unwrap();
        assert!((normal.x - normal_default.x).abs() < 1e-10);
        assert!((normal.y - normal_default.y).abs() < 1e-10);
        assert!((normal.z - normal_default.z).abs() < 1e-10);
    }
    
    #[test]
    fn test_vertex_normal_weighted_angle() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normal = mesh.vertex_normal_weighted(v0, NormalWeighting::Angle).unwrap();
        
        // For a single triangle, angle weighting should give same direction as area
        // Normal should point in +Z direction
        assert!((normal.z - 1.0).abs() < 1e-10);
        assert!(normal.x.abs() < 1e-10);
        assert!(normal.y.abs() < 1e-10);
    }
    
    #[test]
    fn test_vertex_normal_weighted_uniform() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normal = mesh.vertex_normal_weighted(v0, NormalWeighting::Uniform).unwrap();
        
        // For a single triangle, uniform weighting should give same direction
        // Normal should point in +Z direction
        assert!((normal.z - 1.0).abs() < 1e-10);
        assert!(normal.x.abs() < 1e-10);
        assert!(normal.y.abs() < 1e-10);
    }
    
    #[test]
    fn test_vertex_normals_weighted() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let _f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        let normals = mesh.vertex_normals_weighted(NormalWeighting::Angle);
        
        assert_eq!(normals.len(), 3);
        assert!(normals.contains_key(&v0));
        assert!(normals.contains_key(&v1));
        assert!(normals.contains_key(&v2));
        
        // All vertex normals should point in +Z direction
        let normal_v0 = normals.get(&v0).unwrap();
        assert!((normal_v0.z - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_vertex_angle_in_face() {
        let mut mesh = Mesh::new();
        // Create a right triangle
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        
        let f = mesh.add_face(vec![v0, v1, v2], None).unwrap();
        
        // Angle at v0 should be 90 degrees (π/2 radians)
        let angle = mesh.vertex_angle_in_face(v0, f).unwrap();
        assert!((angle - std::f64::consts::PI / 2.0).abs() < 1e-10);
        
        // Angles at v1 and v2 should be 45 degrees (π/4 radians) each
        let angle1 = mesh.vertex_angle_in_face(v1, f).unwrap();
        let angle2 = mesh.vertex_angle_in_face(v2, f).unwrap();
        assert!((angle1 - std::f64::consts::PI / 4.0).abs() < 1e-10);
        assert!((angle2 - std::f64::consts::PI / 4.0).abs() < 1e-10);
        
        // Sum of angles should be π
        let total_angle = angle + angle1 + angle2;
        assert!((total_angle - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_from_polygons_simple() {
        let triangle = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];
        
        let mesh = Mesh::from_polygons(vec![triangle], None);
        assert_eq!(mesh.number_of_vertices(), 3);
        assert_eq!(mesh.number_of_faces(), 1);
        assert_eq!(mesh.number_of_edges(), 3);
    }

    #[test]
    fn test_from_polygons_vertex_merging() {
        let triangle1 = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];
        let triangle2 = vec![
            Point::new(1.0, 0.0, 0.0), // Shared vertex
            Point::new(0.0, 1.0, 0.0), // Shared vertex
            Point::new(1.0, 1.0, 0.0),
        ];
        
        let mesh = Mesh::from_polygons(vec![triangle1, triangle2], None);
        assert_eq!(mesh.number_of_vertices(), 4); // Should merge shared vertices
        assert_eq!(mesh.number_of_faces(), 2);
    }

    #[test]
    fn test_from_polygons_precision() {
        let triangle1 = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];
        let triangle2 = vec![
            Point::new(1.0000001, 0.0, 0.0), // Very close to (1,0,0)
            Point::new(0.0, 1.0000001, 0.0), // Very close to (0,1,0)
            Point::new(1.0, 1.0, 0.0),
        ];
        
        let mesh = Mesh::from_polygons(vec![triangle1, triangle2], Some(1e-6));
        assert_eq!(mesh.number_of_vertices(), 4); // Should merge vertices within precision
        assert_eq!(mesh.number_of_faces(), 2);
    }

    #[test]
    fn test_from_polygons_invalid_polygons() {
        let invalid_polygon = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0), // Only 2 points
        ];
        let valid_triangle = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];
        
        let mesh = Mesh::from_polygons(vec![invalid_polygon, valid_triangle], None);
        assert_eq!(mesh.number_of_vertices(), 3); // Only valid triangle should be added
        assert_eq!(mesh.number_of_faces(), 1);
    }

    #[test]
    fn test_from_polygons_cube() {
        // Create a cube using 6 faces
        let faces = vec![
            // Bottom face (z=0)
            vec![
                Point::new(0.0, 0.0, 0.0),
                Point::new(1.0, 0.0, 0.0),
                Point::new(1.0, 1.0, 0.0),
                Point::new(0.0, 1.0, 0.0),
            ],
            // Top face (z=1)
            vec![
                Point::new(0.0, 0.0, 1.0),
                Point::new(0.0, 1.0, 1.0),
                Point::new(1.0, 1.0, 1.0),
                Point::new(1.0, 0.0, 1.0),
            ],
            // Front face (y=0)
            vec![
                Point::new(0.0, 0.0, 0.0),
                Point::new(0.0, 0.0, 1.0),
                Point::new(1.0, 0.0, 1.0),
                Point::new(1.0, 0.0, 0.0),
            ],
            // Back face (y=1)
            vec![
                Point::new(0.0, 1.0, 0.0),
                Point::new(1.0, 1.0, 0.0),
                Point::new(1.0, 1.0, 1.0),
                Point::new(0.0, 1.0, 1.0),
            ],
            // Left face (x=0)
            vec![
                Point::new(0.0, 0.0, 0.0),
                Point::new(0.0, 1.0, 0.0),
                Point::new(0.0, 1.0, 1.0),
                Point::new(0.0, 0.0, 1.0),
            ],
            // Right face (x=1)
            vec![
                Point::new(1.0, 0.0, 0.0),
                Point::new(1.0, 0.0, 1.0),
                Point::new(1.0, 1.0, 1.0),
                Point::new(1.0, 1.0, 0.0),
            ],
        ];
        
        let mesh = Mesh::from_polygons(faces, None);
        assert_eq!(mesh.number_of_vertices(), 8); // A cube has 8 vertices
        assert_eq!(mesh.number_of_faces(), 6);    // A cube has 6 faces
        assert_eq!(mesh.number_of_edges(), 12);   // A cube has 12 edges
        assert_eq!(mesh.euler(), 2);             // Euler characteristic for a cube: V-E+F = 8-12+6 = 2
    }

    #[test]
    fn test_clear() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
        let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
        let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
        mesh.add_face(vec![v0, v1, v2], None);
        
        assert!(!mesh.is_empty());
        mesh.clear();
        assert!(mesh.is_empty());
        assert_eq!(mesh.number_of_vertices(), 0);
        assert_eq!(mesh.number_of_faces(), 0);
    }
}

/// Implementation of DataObject trait for Mesh to support COMPAS-style JSON serialization
impl Mesh {
    /// Get the type identifier for polymorphic deserialization
    pub fn dtype(&self) -> &'static str {
        "openmodel.geometry/Mesh"
    }
    
    /// Get the object's mesh data for serialization
    pub fn geometric_data(&self) -> serde_json::Value {
        // Convert vertex data to serializable format
        let vertices: HashMap<String, serde_json::Value> = self.vertex.iter()
            .map(|(k, v)| (k.to_string(), serde_json::json!({
                "x": v.x,
                "y": v.y,
                "z": v.z,
                "attributes": v.attributes
            })))
            .collect();
        
        // Convert face data to serializable format
        let faces: HashMap<String, Vec<usize>> = self.face.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        
        // Convert halfedge data to serializable format
        let halfedges: HashMap<String, serde_json::Value> = self.halfedge.iter()
            .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
            .collect();
        
        serde_json::json!({
            "vertex": vertices,
            "face": faces,
            "halfedge": halfedges,
            "facedata": self.facedata,
            "edgedata": self.edgedata,
            "default_vertex_attributes": self.default_vertex_attributes,
            "default_face_attributes": self.default_face_attributes,
            "default_edge_attributes": self.default_edge_attributes,
            "max_vertex": self.max_vertex,
            "max_face": self.max_face
        })
    }
    
    /// Get the object's GUID
    pub fn guid(&self) -> Uuid {
        self.data.guid()
    }
    
    /// Get the object's name
    pub fn name(&self) -> &str {
        self.data.name()
    }
    
    /// Set the object's name
    pub fn set_name(&mut self, name: &str) {
        self.data.set_name(name);
    }
    
    /// Create a structured JSON representation similar to COMPAS
    pub fn to_json_data(&self, minimal: bool) -> serde_json::Value {
        self.data.to_json_data(self.dtype(), self.geometric_data(), minimal)
    }
}

// JSON serialization support
impl JsonSerializable for Mesh {
    fn to_json_value(&self) -> serde_json::Value {
        // Use direct serialization for consistency with struct definition
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

impl FromJsonData for Mesh {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        // Try direct deserialization first
        if let Ok(mesh) = serde_json::from_value(data.clone()) {
            return Some(mesh);
        }
        
        // Try COMPAS format (extract from data field)
        if let Some(data_obj) = data.get("data") {
            if let Ok(mesh) = serde_json::from_value(data_obj.clone()) {
                return Some(mesh);
            }
        }
        
        None
    }
}

// Simple automatic serialization/deserialization with derive macros
