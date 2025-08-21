use openmodel::geometry::{Point, Line, Mesh, PointCloud, Vector, Color};
use openmodel::common::json_dump;
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
    // Assign per-vertex colors so the renderer uses them (golden yellow)
    for vd in mesh.vertex.values_mut() {
        vd.set_color(1.0, 0.84, 0.0);
    }
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
    let cube = Mesh::from_polygons(cube_faces, None);
    cube
}

fn make_dodecahedron_mesh() -> Mesh {
    // Golden ratio
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let edge_length = 1.0; // L = 2.0
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
    
    let dodecahedron = Mesh::from_polygons(faces, None);
    
    // Debug dodecahedron faces
    println!("Dodecahedron face details:");
    for (fkey, vertices) in dodecahedron.get_face_data() {
        println!("  Face {}: {} vertices", fkey, vertices.len());
    }
    
    dodecahedron
}

fn make_point_cloud() -> PointCloud {
    println!("Generating 1000 points within 10x10x10 bounds...");
    
    let mut points = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    
    // Generate 10x10x10 = 1,000 points within 10x10x10 bounds
    let grid_size = 10;
    let bound = 1.0; // -5 to +5 = 10 units
    let step = (2.0 * bound) / (grid_size as f64 - 1.0);
    
    for i in 0..grid_size {
        for j in 0..grid_size {
            for k in 0..grid_size {
                let x = -bound + (i as f64) * step;
                let y = -bound + (j as f64) * step+5.0;
                let z = -bound + (k as f64) * step;
                
                points.push(Point::new(x, y, z));
                
                // Generate upward-pointing normals
                normals.push(Vector::new(0.0, 0.0, 1.0));
                
                // Generate colors based on position (rainbow gradient)
                let r = ((i as f64 / grid_size as f64) * 255.0) as u8;
                let g = ((j as f64 / grid_size as f64) * 255.0) as u8;
                let b = ((k as f64 / grid_size as f64) * 255.0) as u8;
                colors.push(Color::new(r, g, b, 255));
            }
        }
    }
    
    println!("Generated {} points", points.len());
    PointCloud::new(points, normals, colors)
}

fn main() {
    // Minimal: 10x10 grid (11 lines per direction) on Z=0 plus Z axis line
    let mut lines: Vec<Line> = Vec::new();
    let size: i32 = 5; // -5..=5 => 11 lines => 10x10 cells

    // // Horizontal lines (vary X)
    // for i in -size..=size {
    //     let y = i as f64;
    //     lines.push(Line::from_points(&Point::new(-(size as f64), y, 0.0), &Point::new(size as f64, y, 0.0)));
    // }
    // // Vertical lines (vary Y)
    // for i in -size..=size {
    //     let x = i as f64;
    //     lines.push(Line::from_points(&Point::new(x, -(size as f64), 0.0), &Point::new(x, size as f64, 0.0)));
    // }
    // Z axis
    lines.push(Line::from_points(&Point::new(0.0, 0.0, 0.0), &Point::new(0.0, 0.0, 1.0)));

    // Star polygon mesh alongside the grid
    let star = make_star_mesh();

    // Solid geometry example: sphere
    let sphere = Mesh::create_unit_sphere_high_res();

    // Unit cube translated +2 along X (x in [2,3], y in [0,1], z in [0,1])
    let cube = make_cube_mesh();

    // Dodecahedron positioned at y+3
    let dodecahedron = make_dodecahedron_mesh();

    // Generate point cloud
    let point_cloud = make_point_cloud();

    println!("Created {} meshes:", 4);
    println!("  Star: {} vertices, {} faces", star.number_of_vertices(), star.number_of_faces());
    println!("  Sphere: {} vertices, {} faces", sphere.number_of_vertices(), sphere.number_of_faces());
    println!("  Cube: {} vertices, {} faces", cube.number_of_vertices(), cube.number_of_faces());
    println!("  Dodecahedron: {} vertices, {} faces", dodecahedron.number_of_vertices(), dodecahedron.number_of_faces());
    println!("  Point Cloud: {} points", point_cloud.points.len());

    let all_geometry = AllGeometryData {
        points: vec![],
        vectors: vec![],
        lines,
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
    println!("Wrote {}", out_path);
}