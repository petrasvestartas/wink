// Efficient GPU cone geometry generation
// Generates cone geometry directly in vertex shader based on vertex_index

// Camera uniform structure
struct CameraUniform {
    view_proj: mat4x4<f32>,
    // x: viewport width, y: viewport height, z: fovy (degrees), w: aspect
    viewport_fovy_aspect_pipe_px_radius: vec4<f32>,
    // x: pipe pixel radius, y: ortho_half_height, z: is_ortho (1 or 0), w: reserved
    pipe_params: vec4<f32>,
    // eye position in world space
    eye_pos: vec4<f32>,
    // camera forward direction (world space)
    view_dir: vec4<f32>,
}

// cone transformation matrix with color and thickness
struct coneTransform {
    transform: mat4x4<f32>,
    color: vec3<f32>,
    thickness: f32,
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) quad_coord: vec2<f32>, // For cone circular rendering
}

fn radians(deg: f32) -> f32 {
    return deg * 3.141592653589793 / 180.0;
}

fn tan(x: f32) -> f32 {
    return sin(x) / cos(x);
}

// Storage buffers
@group(0) @binding(2) var<storage, read> cones: array<coneTransform>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// cone vertices (26 vertices) - scaled to match pipe radius
fn get_cone_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 9>(
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.5, 0.0, -1.0),
        vec3<f32>(0.353553, -0.353553, -1.0),
        vec3<f32>(0.0, -0.5, -1.0),
        vec3<f32>(-0.353553, -0.353553, -1.0),
        vec3<f32>(-0.5, 0.0, -1.0),
        vec3<f32>(-0.353553, 0.353553, -1.0),
        vec3<f32>(0.0, 0.5, -1.0),
        vec3<f32>(0.353553, 0.353553, -1.0)
    );
    // Scale to match pipe radius: pipes use 0.5 radius, cones should match
    return vertices[vertex_id]; // No scaling - vertices already have 0.5 radius
}

// cone faces (48 triangles with counter-clockwise winding)
fn get_cone_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;

    // 8 triangles forming a fan around vertex 0
    let triangles = array<array<u32, 3>, 8>(
        array<u32, 3>(0u, 2u, 1u),
        array<u32, 3>(0u, 3u, 2u),
        array<u32, 3>(0u, 4u, 3u),
        array<u32, 3>(0u, 5u, 4u),
        array<u32, 3>(0u, 6u, 5u),
        array<u32, 3>(0u, 7u, 6u),
        array<u32, 3>(0u, 8u, 7u),
        array<u32, 3>(0u, 1u, 8u)
    );

    let triangle = triangles[triangle_id];
    let vertex_index = triangle[vertex_in_triangle];

    return get_cone_vertex_position(vertex_index);
}


// Vertex shader for cones - 3D geometry with proper lighting
@vertex
fn vs_cones(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices_per_cone = 144u; // 24 quads * 6 vertices (2 triangles per quad)
    let cone_index = vertex_index / vertices_per_cone;
    let local_vertex_index = vertex_index % vertices_per_cone;
    
    // Safety check
    if (cone_index >= arrayLength(&cones)) {
        var out: VertexOutput;
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
        out.color = vec3<f32>(1.0, 0.0, 0.0); // Error color
        return out;
    }
    
    let cone_data = cones[cone_index];
    let cone_transform = cone_data.transform;
    let cone_color = cone_data.color;
    let cone_thickness = cone_data.thickness;
    
    // Get unit cone vertex using the same geometry we defined earlier
    let local_pos = get_cone_vertex(local_vertex_index);
    let local_normal = normalize(local_pos); // cone normal is normalized position
    
    // Use IDENTICAL scaling logic as pipes
    let px_radius = camera.pipe_params.x;
    let viewport_h = camera.viewport_fovy_aspect_pipe_px_radius.y;
    let is_ortho = camera.pipe_params.z > 0.5;
    
    // Calculate world-space radius based on camera distance
    var world_per_pixel: f32;
    if is_ortho {
        let ortho_half_height = camera.pipe_params.y;
        world_per_pixel = (2.0 * ortho_half_height) / viewport_h;
    } else {
        // For perspective: calculate based on actual distance to geometry
        let world_center = cone_transform * vec4<f32>(0.0, 0.0, 0.0, 1.0);
        let distance_to_camera = length(world_center.xyz - camera.eye_pos.xyz);
        let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
        world_per_pixel = (2.0 * distance_to_camera * tan(fovy_rad * 0.5)) / viewport_h;
    }
    
    let desired_world_r = px_radius * world_per_pixel;
    
    // Scale cone uniformly to match pipe radius exactly
    // Pipes scale radius by desired_world_r * 2.0, so cones should match
    // Scale ONLY the radius while preserving cone shape
    // Apply thickness multiplier to radius scaling
    let scaled_local_pos = local_pos * desired_world_r * 2.0 * cone_thickness;
    
    // Transform to world space
    let world_position = cone_transform * vec4<f32>(scaled_local_pos, 1.0);
    
    // Transform normal properly - use normalized axes for correct lighting
    let normal_matrix = mat3x3<f32>(
        normalize(cone_transform[0].xyz),
        normalize(cone_transform[1].xyz),
        normalize(cone_transform[2].xyz)
    );
    let world_normal = normalize(normal_matrix * local_normal);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.color = cone_color; // Use cone color from data
    out.quad_coord = vec2<f32>(0.0, 0.0); // Not used for cones
    return out;
}

// Fragment shader with proper lighting for cones
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Balanced lighting to show colors properly
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0)); // Light coming from front-right-top
    let ambient = 0.4;  // Moderate ambient
    let diffuse = max(0.0, dot(in.world_normal, -light_dir));
    let lighting = ambient + diffuse * 0.4;  // Moderate diffuse
    
    // Use the cone color from vertex shader
    let final_color = in.color * lighting;
    
    return vec4<f32>(final_color, 1.0);
}
