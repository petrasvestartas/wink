use openmodel::model_mesh::ModelMesh;

#[test]
fn test_load_cube_from_json() {
    // Test loading the cube geometry from the sample JSON file
    let result = ModelMesh::from_json_file("sample_geometry.json");
    
    match result {
        Ok(model_mesh) => {
            // Cube should have 8 unique vertices (corners)
            assert_eq!(model_mesh.vertex_count, 8);
            
            // Cube should have 12 triangular faces (2 triangles per quad face * 6 faces)
            assert_eq!(model_mesh.triangle_count, 12);
            
            // Check vertex data length (8 vertices * 3 coordinates)
            assert_eq!(model_mesh.vertices.len(), 24);
            
            // Check index data length (12 triangles * 3 indices)
            assert_eq!(model_mesh.indices.len(), 36);
            
            // Check normals length (8 vertices * 3 components)
            assert_eq!(model_mesh.normals.len(), 24);
            
            // Check colors length (8 vertices * 3 components)
            assert_eq!(model_mesh.colors.len(), 24);
            
            // Verify that we have reasonable vertex positions (cube vertices should be 0 or 1)
            for i in 0..model_mesh.vertices.len() {
                let coord = model_mesh.vertices[i];
                assert!(coord == 0.0 || coord == 1.0, "Vertex coordinate {} should be 0.0 or 1.0, got {}", i, coord);
            }
            
            // Check that all colors are white (default)
            for color_component in &model_mesh.colors {
                assert_eq!(*color_component, 1.0);
            }
            
            // Check interleaved data format
            let interleaved = model_mesh.get_interleaved_data();
            assert_eq!(interleaved.len(), 8 * 9); // 8 vertices * 9 components (3 pos + 3 normal + 3 color)
        }
        Err(e) => {
            panic!("Failed to load cube from JSON: {}", e);
        }
    }
}

#[test]
fn test_json_file_not_found() {
    // Test handling of non-existent file
    let result = ModelMesh::from_json_file("nonexistent.json");
    assert!(result.is_err());
}

#[test]
fn test_invalid_json_format() {
    // Create a temporary invalid JSON file for testing
    use std::fs;
    
    let invalid_json = r#"{"invalid": "format"}"#;
    fs::write("temp_invalid.json", invalid_json).expect("Failed to write test file");
    
    let result = ModelMesh::from_json_file("temp_invalid.json");
    assert!(result.is_err());
    
    // Clean up
    let _ = fs::remove_file("temp_invalid.json");
}
