use openmodel::geometry::{Line, Arrow, Mesh, PointCloud};
use openmodel::primitives::{Point, Vector, Color, Xform};
use openmodel::AllGeometryData;

// Minimal star polygon mesh (concave, non-self-intersecting)
fn make_star_mesh() -> Mesh {
    let polygon = vec![
        Point::new(0.12821, 0.514321, 3.0),
        Point::new(-0.103219, 0.282757, 3.0),
        Point::new(-0.430101, 0.264609, 3.0),
        Point::new(-0.281387, -0.02705, 3.0),
        Point::new(-0.365139, -0.343542, 3.0),
        Point::new(-0.041799, -0.292234, 3.0),
        Point::new(0.233322, -0.469688, 3.0),
        Point::new(0.284442, -0.146318, 3.0),
        Point::new(0.538228, 0.0605, 3.0),
        Point::new(0.246482, 0.209046, 3.0),
    ];
    let mut mesh = Mesh::from_polygons(vec![polygon], None);
    for vd in mesh.vertex.values_mut() {
        vd.set_color(1.0, 0.84, 0.0);
    }
    // Set edge color and thickness using existing data field
    mesh.data.set_color([0, 255, 0]); // Bright yellow (RGB 0-255)
    mesh.data.set_thickness(0.1);
    mesh
}

fn make_cube_mesh() -> Mesh{
    let cube_faces = vec![
        // Bottom face (z=0) - CCW when viewed from below (outward normal -Z)
        vec![
            Point::new(2.0, 0.0, 0.0),
            Point::new(2.0, 1.0, 0.0),
            Point::new(3.0, 1.0, 0.0),
            Point::new(3.0, 0.0, 0.0),
        ],
        // Top face (z=1) - CCW when viewed from above (outward normal +Z)
        vec![
            Point::new(2.0, 0.0, 1.0),
            Point::new(3.0, 0.0, 1.0),
            Point::new(3.0, 1.0, 1.0),
            Point::new(2.0, 1.0, 1.0),
        ],
        // Front face (y=0) - CCW when viewed from front (outward normal -Y)
        vec![
            Point::new(2.0, 0.0, 0.0),
            Point::new(3.0, 0.0, 0.0),
            Point::new(3.0, 0.0, 1.0),
            Point::new(2.0, 0.0, 1.0),
        ],
        // Back face (y=1) - CCW when viewed from back (outward normal +Y)
        vec![
            Point::new(3.0, 1.0, 0.0),
            Point::new(2.0, 1.0, 0.0),
            Point::new(2.0, 1.0, 1.0),
            Point::new(3.0, 1.0, 1.0),
        ],
        // Left face (x=2) - CCW when viewed from left (outward normal -X)
        vec![
            Point::new(2.0, 1.0, 0.0),
            Point::new(2.0, 0.0, 0.0),
            Point::new(2.0, 0.0, 1.0),
            Point::new(2.0, 1.0, 1.0),
        ],
        // Right face (x=3) - CCW when viewed from right (outward normal +X)
        vec![
            Point::new(3.0, 0.0, 0.0),
            Point::new(3.0, 1.0, 0.0),
            Point::new(3.0, 1.0, 1.0),
            Point::new(3.0, 0.0, 1.0),
        ],
    ];
    let mut cube = Mesh::from_polygons(cube_faces, None);
    // Set edge color and thickness using existing data field
    cube.data.set_color([0, 0, 255]); // Blue (RGB 0-255)
    cube.data.set_thickness(0.1);
    cube
}

fn make_dodecahedron_mesh() -> Mesh {
    // Golden ratio
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let edge_length = 1.0f32; // L = 2.0
    let a = edge_length / 2.0;
    let b = a * phi;
    let c = a + b;
    
    // 20 vertices of regular dodecahedron (moved +3 units on Y axis)
    let vertices = vec![
        Point::new(-b, -b + 3.0, -b), // 0
        Point::new( b, -b + 3.0, -b), // 1
        Point::new(-b,  b + 3.0, -b), // 2
        Point::new( b,  b + 3.0, -b), // 3
        Point::new(-b, -b + 3.0,  b), // 4
        Point::new( b, -b + 3.0,  b), // 5
        Point::new(-b,  b + 3.0,  b), // 6
        Point::new( b,  b + 3.0,  b), // 7
        Point::new( c, -a + 3.0,  0.0), // 8
        Point::new( c,  a + 3.0,  0.0), // 9
        Point::new(-c, -a + 3.0,  0.0), // 10
        Point::new(-c,  a + 3.0,  0.0), // 11
        Point::new( a,  0.0 + 3.0, -c), // 12
        Point::new(-a,  0.0 + 3.0, -c), // 13
        Point::new( a,  0.0 + 3.0,  c), // 14
        Point::new(-a,  0.0 + 3.0,  c), // 15
        Point::new( 0.0, -c + 3.0, -a), // 16
        Point::new( 0.0, -c + 3.0,  a), // 17
        Point::new( 0.0,  c + 3.0, -a), // 18
        Point::new( 0.0,  c + 3.0,  a), // 19
    ];
    
    // 12 pentagonal faces (counterclockwise when viewed from outside)
    let faces = vec![
        vec![vertices[1].clone(), vertices[12].clone(), vertices[3].clone(), vertices[9].clone(), vertices[8].clone()],   // Face 0
        vec![vertices[5].clone(), vertices[8].clone(), vertices[9].clone(), vertices[7].clone(), vertices[14].clone()],   // Face 1
        vec![vertices[0].clone(), vertices[10].clone(), vertices[11].clone(), vertices[2].clone(), vertices[13].clone()], // Face 2
        vec![vertices[4].clone(), vertices[15].clone(), vertices[6].clone(), vertices[11].clone(), vertices[10].clone()], // Face 3
        vec![vertices[1].clone(), vertices[16].clone(), vertices[0].clone(), vertices[13].clone(), vertices[12].clone()], // Face 4
        vec![vertices[3].clone(), vertices[12].clone(), vertices[13].clone(), vertices[2].clone(), vertices[18].clone()], // Face 5
        vec![vertices[5].clone(), vertices[14].clone(), vertices[15].clone(), vertices[4].clone(), vertices[17].clone()], // Face 6
        vec![vertices[7].clone(), vertices[19].clone(), vertices[6].clone(), vertices[15].clone(), vertices[14].clone()], // Face 7
        vec![vertices[1].clone(), vertices[8].clone(), vertices[5].clone(), vertices[17].clone(), vertices[16].clone()],  // Face 8
        vec![vertices[0].clone(), vertices[16].clone(), vertices[17].clone(), vertices[4].clone(), vertices[10].clone()], // Face 9
        vec![vertices[3].clone(), vertices[18].clone(), vertices[19].clone(), vertices[7].clone(), vertices[9].clone()],  // Face 10
        vec![vertices[2].clone(), vertices[11].clone(), vertices[6].clone(), vertices[19].clone(), vertices[18].clone()], // Face 11
    ];
    
    let mut dodecahedron = Mesh::from_polygons(faces, None);
    // Set edge color and thickness using existing data field
    dodecahedron.data.set_color([100, 100, 100]); // Red (RGB 0-255)
    dodecahedron.data.set_thickness(0.1);
    
    dodecahedron
}

fn make_point_cloud() -> PointCloud {
    
    let mut points = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    
    // Generate 10x10x10 = 1,000 points within 10x10x10 bounds
    let grid_size = 10;
    let bound = 1.0f32; // -5 to +5 = 10 units
    let step = (2.0 * bound) / (grid_size as f32 - 1.0);
    
    for i in 0..grid_size {
        for j in 0..grid_size {
            for k in 0..grid_size {
                let x = -bound + (i as f32) * step;
                let y = -bound + (j as f32) * step + 5.0;
                let z = -bound + (k as f32) * step;
                
                points.push(Point::new(x, y, z));
                
                // Generate upward-pointing normals
                normals.push(Vector::new(0.0, 0.0, 1.0));
                
                // Generate colors based on position (rainbow gradient)
                let r = ((i as f32 / grid_size as f32) * 255.0) as u8;
                let g = ((j as f32 / grid_size as f32) * 255.0) as u8;
                let b = ((k as f32 / grid_size as f32) * 255.0) as u8;
                colors.push(Color::new(r, g, b, 255));
            }
        }
    }
    
    
    // Create a point cloud with transformation matrix
    let mut point_cloud = PointCloud::new(points, normals, colors);
    
    // Apply a transformation: translate by (2, 0, 1) and rotate 45 degrees around Z-axis
    let cos_45 = 0.7071067811865476f32; // cos(45°)
    let sin_45 = 0.7071067811865475f32; // sin(45°)
    
    point_cloud.xform = Xform::from_matrix([
        cos_45*0.0+1.0, -sin_45*0.0, 0.0, 0.0,  // Rotate + translate X
        sin_45*0.0,  cos_45*0.0+1.0, 0.0, 0.0,  // Rotate + translate Y  
        0.0,     0.0,    1.0, 0.0,  // No rotation in Z + translate Z
        0.0,     0.0,    0.0, 1.0   // Homogeneous coordinate
    ]);
    
    point_cloud
}

fn make_lines() -> Vec<Line> {
    // Create lines with varying thickness and color
    let mut lines: Vec<Line> = Vec::new();

    // Grid lines with default thickness
    let size: i32 = 40; // -5..=5 => 11 lines => 10x10 cells
    let thickness = 0.02;

    // Horizontal lines (vary X) - red for x-axis (y=0), black for others
    for i in -size..=size {
        let y = i as f32;
        let mut line = Line::from_points(&Point::new(-(size as f32), y, 0.0), &Point::new(size as f32, y, 0.0));
        line.data.set_thickness(thickness);
        line.data.set_color([0, 0, 0]); // Red for x-axis (y=0)
        lines.push(line);
    }
    // Vertical lines (vary Y) - green for y-axis (x=0), black for others
    for i in -size..=size {
        let x = i as f32;
        let mut line = Line::from_points(&Point::new(x, -(size as f32), 0.0), &Point::new(x, size as f32, 0.0));
        line.data.set_thickness(thickness);
        line.data.set_color([0, 0, 0]); // Green for y-axis (x=0)
        lines.push(line);
    }
    let axes_scale = 2.0;
    let mut line_x = Line::from_points(&Point::new(0.0, 0.0, 0.0), &Point::new(size as f32, 0.0, 0.0));
    line_x.data.set_thickness(thickness*axes_scale);
    line_x.data.set_color([255, 0, 0]); // Red for x-axis (y=0)
    lines.push(line_x);
    let mut line_y = Line::from_points(&Point::new(0.0, 0.0, 0.0), &Point::new(0.0, size as f32, 0.0));
    line_y.data.set_thickness(thickness*axes_scale);
    line_y.data.set_color([0, 255, 0]); // Green for y-axis (x=0)
    lines.push(line_y);
    let mut line_z = Line::from_points(&Point::new(0.0, 0.0, 0.0), &Point::new(0.0, 0.0, size as f32));
    line_z.data.set_thickness(thickness*axes_scale);
    line_z.data.set_color([0, 0, 255]); // Blue for z-axis (x=0)
    lines.push(line_z);

    lines
}

fn make_arrows() -> Vec<Arrow> {
    let mut arrows: Vec<Arrow> = Vec::new();
    let thickness = 0.3;
    
    // Create arrows at origin with larger size and bright colors for visibility
    let mut arrow_x = Arrow::new(0.0+4.0, 0.0, 0.0, 20.0+4.0, 0.0, 0.0);
    arrow_x.data.set_color([255, 0, 0]); // Red
    arrow_x.data.set_thickness(thickness);
    arrows.push(arrow_x);
    
    let mut arrow_y = Arrow::new(0.0+4.0, 0.0, 0.0, 0.0+4.0, 3.0, 0.0);
    arrow_y.data.set_color([0, 255, 0]); // Green
    arrow_y.data.set_thickness(thickness);
    arrows.push(arrow_y);
    
    let mut arrow_z = Arrow::new(0.0+4.0, 0.0, 0.0, 0.0+4.0, 0.0, 3.0);
    arrow_z.data.set_color([0, 0, 255]); // Blue
    arrow_z.data.set_thickness(thickness);
    arrows.push(arrow_z);
    
    arrows
}

fn main() {
    
    
    let star = make_star_mesh();
    let _sphere = Mesh::create_unit_sphere_high_res();
    let cube = make_cube_mesh();
    let dodecahedron = make_dodecahedron_mesh();
    let point_cloud = make_point_cloud();
    let lines = make_lines();
    let arrows = make_arrows();

    let all_geometry = AllGeometryData {
        points: vec![],
        vectors: vec![],
        lines, // Restore lines but without RGB axis lines
        arrows,
        planes: vec![],
        colors: vec![],
        point_clouds: vec![point_cloud],
        line_clouds: vec![],
        plines: vec![],
        xforms: vec![],
        meshes: vec![star, cube, dodecahedron], //sphere
        mesh_instances: vec![],
        pipe_mesh_index: None,
        sphere_mesh_index: None,
    };

    // Write deterministically next to this Cargo package (not dependent on current working dir)
    let out_path = format!("{}/all_geometry.json", env!("CARGO_MANIFEST_DIR"));
    let json_string = serde_json::to_string_pretty(&all_geometry).unwrap();
    std::fs::write(&out_path, json_string).unwrap();
}