use openmodel::geometry::{Line, LineCloud};
use openmodel::geometry::Color;

#[test]
fn test_line_pipe_visualization() {
    // Create a simple vertical line
    let mut line = Line::new(0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    
    // Set thickness in data
    line.data.set_thickness(0.1);
    
    // Initially there's no mesh
    assert!(line.mesh.is_none());
    
    // Get the mesh - this should trigger creation
    let mesh = line.get_mesh().unwrap();
    
    // Check that the mesh has the expected number of vertices and faces
    // For a pipe with 8 sides (fixed number), we expect:
    // - 2 center points (one at each end)
    // - 12 vertices for the rim at each end (24 total)
    // - 12 triangular faces at each end (24 total)
    // Check mesh structure - pipe should have vertices and faces
    assert_eq!(mesh.number_of_vertices(), 2 + 12 * 2);  // 2 centers + 12*2 rim vertices
    assert_eq!(mesh.number_of_faces(), 12 + 12 + 12*2);   // Bottom cap + top cap + sides
    
    // Test setting color in data
    let color = Color::new(255, 0, 0, 255); // Red
    line.data.set_color_from(&color);
}

#[test]
fn test_linecloud_pipe_visualization() {
    // Create two lines
    let line1 = Line::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    let line2 = Line::new(0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
    
    // Create two colors
    let color1 = Color::new(255, 0, 0, 255); // Red
    let color2 = Color::new(0, 255, 0, 255); // Green
    
    // Create a LineCloud with these lines and colors
    let mut cloud = LineCloud::new(
        vec![line1, line2],
        vec![color1, color2]
    );
    
    // Set thickness in data
    cloud.data.set_thickness(0.05);
    
    // Initially there are no meshes
    assert!(cloud.meshes.is_empty());
    
    // Get the meshes - this should trigger mesh creation
    let meshes = cloud.get_meshes();
    
    // Check that there are two meshes (one per line)
    assert_eq!(meshes.len(), 2);
    
    // Check that each mesh has the expected properties
    for mesh in meshes {
        // For a pipe with 12 sides (fixed number), we expect:
        // - 2 center points (one at each end)
        // - 12 vertices for the rim at each end (24 total)
        // - 12 triangular faces at each end (24 total)
        // - 12*2 triangular side faces
        assert_eq!(mesh.number_of_vertices(), 2 + 12 * 2);  // 2 centers + 12*2 rim vertices
        assert_eq!(mesh.number_of_faces(), 12 + 12 + 12*2); // Bottom cap + top cap + sides
        
        // Check that the mesh has color data from the colors array
        // We know it should have either red or green color
        let rgb = mesh.data.get_color();
        assert!(rgb == [255, 0, 0] || rgb == [0, 255, 0]);
    }
    
    // Test that we can update meshes multiple times
    let v = openmodel::geometry::Vector::new(1.0, 1.0, 1.0);
    cloud += &v;
    
    // Force mesh update
    cloud.update_meshes();
    
    // Check that meshes were updated (should still have correct count)
    let updated_meshes = cloud.get_meshes();
    assert_eq!(updated_meshes.len(), 2);
}
