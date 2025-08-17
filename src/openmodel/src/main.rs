use openmodel::geometry::{Point, Line, Mesh};
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
    let mut mesh = Mesh::from_polygon_earclip(polygon);
    // Assign per-vertex colors so the renderer uses them (golden yellow)
    for vd in mesh.vertex.values_mut() {
        vd.set_color(1.0, 0.84, 0.0);
    }
    mesh
}

fn make_cube_mesh() -> Mesh{
    let cube_faces = vec![
        // Bottom face (z=0)
        vec![
            Point::new(2.0, 1.0, 0.0),
            Point::new(3.0, 1.0, 0.0),
            Point::new(3.0, 0.0, 0.0),
            Point::new(2.0, 0.0, 0.0),
            
            
            
        ],
        // Top face (z=1)
        vec![
            Point::new(3.0, 0.0, 1.0),
            Point::new(3.0, 1.0, 1.0),
            Point::new(2.0, 1.0, 1.0),
            Point::new(2.0, 0.0, 1.0),
            
            
            
        ],
        // Front face (y=0)
        vec![
            Point::new(3.0, 0.0, 0.0),
            Point::new(3.0, 0.0, 1.0),
            Point::new(2.0, 0.0, 1.0),
            Point::new(2.0, 0.0, 0.0),
            
            
            
        ],
        // Back face (y=1)
        vec![
            Point::new(2.0, 1.0, 1.0),
            Point::new(3.0, 1.0, 1.0),
            Point::new(3.0, 1.0, 0.0),
            Point::new(2.0, 1.0, 0.0),
            
            
            
        ],
        // Left face (x=2)
        vec![
            Point::new(2.0, 0.0, 1.0),
            Point::new(2.0, 1.0, 1.0),
            Point::new(2.0, 1.0, 0.0),
            Point::new(2.0, 0.0, 0.0),
            
            
            
        ],
        // Right face (x=3)
        vec![
            Point::new(3.0, 1.0, 0.0),
            Point::new(3.0, 1.0, 1.0),
            Point::new(3.0, 0.0, 1.0),
            Point::new(3.0, 0.0, 0.0),
            
            
            
        ],
    ];
    let cube = Mesh::from_polygons(cube_faces, None);
    cube
}

fn main() {
    // Minimal: 10x10 grid (11 lines per direction) on Z=0 plus Z axis line
    let mut lines: Vec<Line> = Vec::new();
    let size: i32 = 5; // -5..=5 => 11 lines => 10x10 cells

    // Horizontal lines (vary X)
    for i in -size..=size {
        let y = i as f64;
        lines.push(Line::from_points(&Point::new(-(size as f64), y, 0.0), &Point::new(size as f64, y, 0.0)));
    }
    // Vertical lines (vary Y)
    for i in -size..=size {
        let x = i as f64;
        lines.push(Line::from_points(&Point::new(x, -(size as f64), 0.0), &Point::new(x, size as f64, 0.0)));
    }
    // Z axis
    lines.push(Line::from_points(&Point::new(0.0, 0.0, 0.0), &Point::new(0.0, 0.0, 1.0)));

    // Star polygon mesh alongside the grid
    let star = make_star_mesh();

    // Solid geometry example: sphere
    let sphere = Mesh::create_unit_sphere_high_res();

    // Unit cube translated +2 along X (x in [2,3], y in [0,1], z in [0,1])
    let cube = make_cube_mesh();


    let all_geometry = AllGeometryData {
        points: vec![],
        vectors: vec![],
        lines,
        planes: vec![],
        colors: vec![],
        point_clouds: vec![],
        line_clouds: vec![],
        plines: vec![],
        xforms: vec![],
        meshes: vec![star, sphere, cube],
        mesh_instances: vec![],
        pipe_mesh_index: None,
        sphere_mesh_index: None,
    };

    // Write deterministically next to this Cargo package (not dependent on current working dir)
    let out_path = format!("{}/all_geometry.json", env!("CARGO_MANIFEST_DIR"));
    json_dump(&all_geometry, &out_path);
    println!("Wrote {}", out_path);
}