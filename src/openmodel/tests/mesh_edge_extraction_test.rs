use openmodel::geometry::{Mesh, Point};

#[test]
fn test_extract_edges_as_lines_triangle() {
    // Create a simple triangle mesh
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    mesh.add_face(vec![v0, v1, v2], None);
    
    // Extract edges as lines
    let edges = mesh.extract_edges_as_lines();
    
    // Triangle should have exactly 3 unique edges
    assert_eq!(edges.len(), 3);
    
    // Verify that each edge connects two different vertices
    for line in &edges {
        let start = Point::new(line.x0, line.y0, line.z0);
        let end = Point::new(line.x1, line.y1, line.z1);
        assert_ne!(start, end); // Start and end should be different
    }
}

#[test]
fn test_extract_edges_as_lines_square() {
    // Create a square mesh (two triangles)
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    let v2 = mesh.add_vertex(Point::new(1.0, 1.0, 0.0), None);
    let v3 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    
    // Add two triangular faces to form a square
    mesh.add_face(vec![v0, v1, v2], None);
    mesh.add_face(vec![v0, v2, v3], None);
    
    // Extract edges as lines
    let edges = mesh.extract_edges_as_lines();
    
    // Square should have exactly 5 unique edges (4 outer + 1 diagonal)
    assert_eq!(edges.len(), 5);
}

#[test]
fn test_extract_edges_as_pipes_triangle() {
    // Create a simple triangle mesh
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    mesh.add_face(vec![v0, v1, v2], None);
    
    // Extract edges as pipe meshes
    let radius = 0.05;
    let sides = 8; // This parameter is ignored, create_pipe always uses 12 sides
    let pipe_meshes = mesh.extract_edges_as_pipes(radius, Some(sides));
    
    // Triangle should have exactly 3 pipe meshes
    assert_eq!(pipe_meshes.len(), 3);
    
    // Each pipe mesh should have the correct structure (always 12-sided)
    for pipe_mesh in &pipe_meshes {
        // Each pipe should have 2 + 12*2 vertices (2 centers + 12*2 rim vertices)
        assert_eq!(pipe_mesh.number_of_vertices(), 2 + 12 * 2);
        // Each pipe should have 12 + 12 + 12*2 faces (bottom cap + top cap + sides)
        assert_eq!(pipe_mesh.number_of_faces(), 12 + 12 + 12 * 2);
    }
}

#[test]
fn test_extract_edges_as_pipes_with_default_sides() {
    // Create a simple triangle mesh
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    let v2 = mesh.add_vertex(Point::new(0.0, 1.0, 0.0), None);
    mesh.add_face(vec![v0, v1, v2], None);
    
    // Extract edges as pipe meshes with default sides (8)
    let radius = 0.05;
    let pipe_meshes = mesh.extract_edges_as_pipes(radius, None);
    
    // Triangle should have exactly 3 pipe meshes
    assert_eq!(pipe_meshes.len(), 3);
    
    // Each pipe mesh always uses 12 sides (sides parameter is ignored)
    for pipe_mesh in &pipe_meshes {
        assert_eq!(pipe_mesh.number_of_vertices(), 2 + 12 * 2); // 2 + 12*2 = 26
        assert_eq!(pipe_mesh.number_of_faces(), 12 + 12 + 12 * 2); // 12 + 12 + 24 = 48
    }
}

#[test]
fn test_extract_edges_empty_mesh() {
    // Create an empty mesh
    let mesh = Mesh::new();
    
    // Extract edges from empty mesh
    let edges = mesh.extract_edges_as_lines();
    let pipe_meshes = mesh.extract_edges_as_pipes(0.05, Some(8));
    
    // Empty mesh should have no edges
    assert_eq!(edges.len(), 0);
    assert_eq!(pipe_meshes.len(), 0);
}

#[test]
fn test_extract_edges_no_duplicates() {
    // Create a mesh with shared edges
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(Point::new(0.0, 0.0, 0.0), None);
    let v1 = mesh.add_vertex(Point::new(1.0, 0.0, 0.0), None);
    let v2 = mesh.add_vertex(Point::new(0.5, 1.0, 0.0), None);
    let v3 = mesh.add_vertex(Point::new(0.5, -1.0, 0.0), None);
    
    // Add two triangles sharing an edge (v0-v1)
    mesh.add_face(vec![v0, v1, v2], None);
    mesh.add_face(vec![v0, v1, v3], None);
    
    // Extract edges as lines
    let edges = mesh.extract_edges_as_lines();
    
    // Should have 5 unique edges total (no duplicates for shared edge v0-v1)
    assert_eq!(edges.len(), 5);
    
    // Extract edges as pipes
    let pipe_meshes = mesh.extract_edges_as_pipes(0.05, Some(8));
    
    // Should have 5 unique pipe meshes (no duplicates for shared edge v0-v1)
    assert_eq!(pipe_meshes.len(), 5);
}
