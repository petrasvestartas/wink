use openmodel::geometry::{Point, Vector, Line, Plane, Color, PointCloud, LineCloud, Pline, Mesh};
use openmodel::primitives::Xform;
use openmodel::common::{JsonSerializable, FromJsonData, HasJsonData, json_dump, json_load};
use serde::{Serialize, Deserialize};
use serde_json;

// Comprehensive geometry data structure with all geometry types
#[derive(Serialize, Deserialize, Debug)]
struct AllGeometryData {
    points: Vec<Point>,
    vectors: Vec<Vector>,
    lines: Vec<Line>,
    planes: Vec<Plane>,
    colors: Vec<Color>,
    point_clouds: Vec<PointCloud>,
    line_clouds: Vec<LineCloud>,
    plines: Vec<Pline>,
    xforms: Vec<Xform>,
    meshes: Vec<Mesh>,
}

// Implement JsonSerializable for AllGeometryData to work with json_dump/json_load
impl JsonSerializable for AllGeometryData {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

// Implement FromJsonData for AllGeometryData to work with json_load
impl FromJsonData for AllGeometryData {
    fn from_json_data(data: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }
}

fn main() {
    println!("=== Testing Concave Mesh with Ear Clipping ===\n");
    
    // Create a star polygon using the provided coordinates
    // These coordinates create a proper star shape
    let concave_polygon = vec![
        Point::new(0.12821, 0.514321+3.0, 3.0),    // Point 1
        Point::new(-0.103219, 0.282757+3.0, 3.0),  // Point 2
        Point::new(-0.430101, 0.264609+3.0, 3.0),  // Point 3
        Point::new(-0.281387, -0.02705+3.0, 3.0),  // Point 4
        Point::new(-0.365139, -0.343542+3.0, 3.0), // Point 5
        Point::new(-0.041799, -0.292234+3.0, 3.0), // Point 6
        Point::new(0.233322, -0.469688+3.0, 3.0),  // Point 7
        Point::new(0.284442, -0.146318+3.0, 3.0),  // Point 8
        Point::new(0.538228, 0.0605+3.0, 3.0),     // Point 9
        Point::new(0.246482, 0.209046+3.0, 3.0),   // Point 10
    ];
    
    println!("Creating star mesh with {} vertices using provided coordinates...", concave_polygon.len());
    
    // ASCII visualization of the star polygon
    println!("\n=== Star Polygon Visualization (1x1 screen) ===");
    println!("Y");
    println!("1.0 ┌─────────────────────────────────────┐");
    println!("    │                                     │");
    println!("0.8 │                ⭐                   │");
    println!("    │                                     │");
    println!("0.6 │                                     │");
    println!("    │                                     │");
    println!("0.4 │                                     │");
    println!("    │                                     │");
    println!("0.2 │                                     │");
    println!("    │                                     │");
    println!("0.0 └─────────────────────────────────────┘");
    println!("    0.0   0.2   0.4   0.6   0.8   1.0   X");
    println!();
    println!("Polygon vertices (in order):");
    for (i, point) in concave_polygon.iter().enumerate() {
        println!("  {}: ({:.6}, {:.6}, {:.6})", i, point.x, point.y, point.z);
    }
    
    let mut mesh = Mesh::from_polygon_earclip(concave_polygon);
    
    // Add colors to vertices
    let colors = [
        [1.0, 0.0, 0.0],   // Red
        [0.0, 1.0, 0.0],   // Green
        [0.0, 0.0, 1.0],   // Blue
        [1.0, 1.0, 0.0],   // Yellow
        [1.0, 0.0, 1.0],   // Magenta
        [0.0, 1.0, 1.0],   // Cyan
        [1.0, 0.5, 0.0],   // Orange
        [0.5, 0.0, 1.0],   // Purple
        [0.0, 0.5, 0.5],   // Teal
        [0.5, 0.5, 0.0],   // Olive
    ];
    
    // Add colors to each vertex
    for (i, (vertex_key, vertex_data)) in mesh.vertex.iter_mut().enumerate() {
        let color = colors[i % colors.len()];
        vertex_data.set_color(color[0], color[1], color[2]);
    }
    
    println!("\n✅ Star mesh created successfully!");
    println!("   Vertices: {}", mesh.number_of_vertices());
    println!("   Faces: {}", mesh.number_of_faces());
    println!("   Edges: {}", mesh.number_of_edges());
    println!("   Euler characteristic: {}", mesh.euler());
    
    // Print face information with vertex coordinates
    println!("\n=== Triangulation Details ===");
    for (face_key, vertices) in mesh.get_face_data() {
        print!("Face {}: ", face_key);
        for (i, &vertex_key) in vertices.iter().enumerate() {
            if let Some(pos) = mesh.vertex_position(vertex_key) {
                if i > 0 { print!(" -> "); }
                print!("({:.6}, {:.6})", pos.x, pos.y);
            }
        }
        println!();
    }
    
    // Verify the mesh properties
    println!("\n=== Mesh Properties ===");
    println!("Is empty: {}", mesh.is_empty());
    println!("Number of vertices: {}", mesh.number_of_vertices());
    println!("Number of faces: {}", mesh.number_of_faces());
    println!("Number of edges: {}", mesh.number_of_edges());
    println!("Euler characteristic: {}", mesh.euler());
    
    // Check if all vertices are on boundary (they should be for a simple polygon)
    let boundary_vertices: Vec<usize> = mesh.vertex.keys().cloned().collect();
    let boundary_count = boundary_vertices.iter()
        .filter(|&&v| mesh.is_vertex_on_boundary(v))
        .count();
    println!("Vertices on boundary: {}/{}", boundary_count, mesh.number_of_vertices());
    
    // Test edge extraction
    let edges = mesh.extract_edges_as_lines();
    println!("Unique edges extracted: {}", edges.len());
    
    // Test JSON serialization
    println!("\n=== Testing JSON Serialization ===");
    
    // Test simple serde_json serialization
    match serde_json::to_string_pretty(&mesh) {
        Ok(json_str) => {
            println!("✅ JSON serialization successful!");
            println!("JSON length: {} characters", json_str.len());
            println!("First 200 characters of JSON:");
            println!("{}", &json_str[..std::cmp::min(200, json_str.len())]);
            
            // Test deserialization
            match serde_json::from_str::<Mesh>(&json_str) {
                Ok(deserialized_mesh) => {
                    println!("✅ JSON deserialization successful!");
                    println!("Deserialized mesh has {} vertices and {} faces", 
                             deserialized_mesh.number_of_vertices(), 
                             deserialized_mesh.number_of_faces());
                    
                    // Verify the deserialized mesh matches the original
                    if deserialized_mesh.number_of_vertices() == mesh.number_of_vertices() &&
                       deserialized_mesh.number_of_faces() == mesh.number_of_faces() {
                        println!("✅ Serialization/deserialization round-trip successful!");
                    } else {
                        println!("❌ Serialization/deserialization round-trip failed!");
                    }
                }
                Err(e) => {
                    println!("❌ JSON deserialization failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ JSON serialization failed: {}", e);
        }
    }
    
    // Test the custom json_dump/json_load functions
    println!("\n=== Testing json_dump/json_load ===");
    json_dump(&mesh, "star_mesh.json");
    println!("✅ json_dump completed!");
    
    // Debug: Let's see what the JSON looks like
    let json_content = std::fs::read_to_string("star_mesh.json").unwrap();
    println!("JSON file size: {} characters", json_content.len());
    println!("First 300 characters of JSON:");
    println!("{}", &json_content[..std::cmp::min(300, json_content.len())]);
    
    let loaded_mesh = json_load::<Mesh>("star_mesh.json");
    println!("✅ json_load successful!");
    println!("Loaded mesh has {} vertices and {} faces", 
             loaded_mesh.number_of_vertices(), 
             loaded_mesh.number_of_faces());
    
    if loaded_mesh.number_of_vertices() == mesh.number_of_vertices() &&
       loaded_mesh.number_of_faces() == mesh.number_of_faces() {
        println!("✅ json_dump/json_load round-trip successful!");
    } else {
        println!("❌ json_dump/json_load round-trip failed!");
    }
    
    println!("\n🎉 Star mesh with ear clipping triangulation works perfectly!");
    println!("⭐ The {}-vertex star polygon was triangulated into {} triangles", 
             mesh.number_of_vertices(), mesh.number_of_faces());
    println!("🔗 The mesh has {} unique edges", edges.len());
    
    // Add the star mesh to AllGeometryData and save to all_geometry.json
    println!("\n=== Adding Star Mesh to All Geometry ===");
    
    // Create AllGeometryData with the star mesh
    let all_geometry = AllGeometryData {
        points: vec![],
        vectors: vec![],
        lines: vec![],
        planes: vec![],
        colors: vec![],
        point_clouds: vec![],
        line_clouds: vec![],
        plines: vec![],
        xforms: vec![],
        meshes: vec![mesh.clone()], // Add the star mesh
    };
    
    // Save to all_geometry.json
    json_dump(&all_geometry, "all_geometry.json");
    println!("✅ Star mesh saved to all_geometry.json");
    
    // Test loading from all_geometry.json
    let loaded_all_geometry = json_load::<AllGeometryData>("all_geometry.json");
    println!("✅ Successfully loaded all_geometry.json");
    println!("   Contains {} meshes", loaded_all_geometry.meshes.len());
    if let Some(loaded_mesh) = loaded_all_geometry.meshes.first() {
        println!("   Loaded mesh has {} vertices and {} faces", 
                 loaded_mesh.number_of_vertices(), 
                 loaded_mesh.number_of_faces());
        if loaded_mesh.number_of_vertices() == mesh.number_of_vertices() &&
           loaded_mesh.number_of_faces() == mesh.number_of_faces() {
            println!("✅ All geometry round-trip successful!");
        } else {
            println!("❌ All geometry round-trip failed!");
        }
    }
    
    println!("💾 Star mesh is now serialized to all_geometry.json");
}
 