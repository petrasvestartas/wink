// Efficient GPU cylinder/pipe geometry generation
// Generates pipe geometry directly in vertex shader based on vertex_index

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

// Pipe transformation matrix with color and thickness
struct PipeTransform {
    transform: mat4x4<f32>,
    color: vec3<f32>,
    thickness: f32,
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) quad_coord: vec2<f32>, // For sphere circular rendering
}

fn radians(deg: f32) -> f32 {
    return deg * 3.141592653589793 / 180.0;
}

fn tan(x: f32) -> f32 {
    return sin(x) / cos(x);
}

// Storage buffers
@group(0) @binding(0) var<storage, read> pipes: array<PipeTransform>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// Cylinder vertices (20 vertices) - 10-sided cylinder with hardcoded coordinates
fn get_pipe_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 20>(
        vec3<f32>(0.5, 0.0, -0.5),
        vec3<f32>(0.404508, 0.293893, -0.5),
        vec3<f32>(0.154508, 0.475528, -0.5),
        vec3<f32>(-0.154508, 0.475528, -0.5),
        vec3<f32>(-0.404508, 0.293893, -0.5),
        vec3<f32>(-0.5, 0.0, -0.5),
        vec3<f32>(-0.404508, -0.293893, -0.5),
        vec3<f32>(-0.154508, -0.475528, -0.5),
        vec3<f32>(0.154508, -0.475528, -0.5),
        vec3<f32>(0.404508, -0.293893, -0.5),
        vec3<f32>(0.5, 0.0, 0.5),
        vec3<f32>(0.404508, 0.293893, 0.5),
        vec3<f32>(0.154508, 0.475528, 0.5),
        vec3<f32>(-0.154508, 0.475528, 0.5),
        vec3<f32>(-0.404508, 0.293893, 0.5),
        vec3<f32>(-0.5, 0.0, 0.5),
        vec3<f32>(-0.404508, -0.293893, 0.5),
        vec3<f32>(-0.154508, -0.475528, 0.5),
        vec3<f32>(0.154508, -0.475528, 0.5),
        vec3<f32>(0.404508, -0.293893, 0.5)
    );
    return vertices[vertex_id];
}

// Cylinder faces (20 triangles with counter-clockwise winding)
fn get_pipe_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    // 20 triangles (2 per quad face) with counter-clockwise winding
    let triangles = array<array<u32, 3>, 20>(
        // Q0: (0,1,11,10) -> triangles (0,1,11) and (0,11,10)
        array<u32, 3>(0u, 1u, 11u),
        array<u32, 3>(0u, 11u, 10u),
        // Q1: (1,2,12,11) -> triangles (1,2,12) and (1,12,11)
        array<u32, 3>(1u, 2u, 12u),
        array<u32, 3>(1u, 12u, 11u),
        // Q2: (2,3,13,12) -> triangles (2,3,13) and (2,13,12)
        array<u32, 3>(2u, 3u, 13u),
        array<u32, 3>(2u, 13u, 12u),
        // Q3: (3,4,14,13) -> triangles (3,4,14) and (3,14,13)
        array<u32, 3>(3u, 4u, 14u),
        array<u32, 3>(3u, 14u, 13u),
        // Q4: (4,5,15,14) -> triangles (4,5,15) and (4,15,14)
        array<u32, 3>(4u, 5u, 15u),
        array<u32, 3>(4u, 15u, 14u),
        // Q5: (5,6,16,15) -> triangles (5,6,16) and (5,16,15)
        array<u32, 3>(5u, 6u, 16u),
        array<u32, 3>(5u, 16u, 15u),
        // Q6: (6,7,17,16) -> triangles (6,7,17) and (6,17,16)
        array<u32, 3>(6u, 7u, 17u),
        array<u32, 3>(6u, 17u, 16u),
        // Q7: (7,8,18,17) -> triangles (7,8,18) and (7,18,17)
        array<u32, 3>(7u, 8u, 18u),
        array<u32, 3>(7u, 18u, 17u),
        // Q8: (8,9,19,18) -> triangles (8,9,19) and (8,19,18)
        array<u32, 3>(8u, 9u, 19u),
        array<u32, 3>(8u, 19u, 18u),
        // Q9: (9,0,10,19) -> triangles (9,0,10) and (9,10,19)
        array<u32, 3>(9u, 0u, 10u),
        array<u32, 3>(9u, 10u, 19u)
    );
    
    let triangle = triangles[triangle_id];
    let vertex_index = triangle[vertex_in_triangle];
    
    return get_pipe_vertex_position(vertex_index);
}

// Vertex shader for pipes - 3D geometry with proper lighting
@vertex
fn vs_pipes(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices_per_pipe = 60u; // 10 quads * 6 vertices (2 triangles per quad)
    let pipe_index = vertex_index / vertices_per_pipe;
    let local_vertex_index = vertex_index % vertices_per_pipe;
    
    // Safety check
    if (pipe_index >= arrayLength(&pipes)) {
        var out: VertexOutput;
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
        out.color = vec3<f32>(1.0, 0.0, 0.0); // Error color
        return out;
    }
    
    let pipe_data = pipes[pipe_index];
    let pipe_transform = pipe_data.transform;
    let pipe_thickness = pipe_data.thickness;

    
    // Get unit pipe vertex
    let local_pos = get_pipe_vertex(local_vertex_index);
    
    // Calculate proper cylindrical normal for lighting (before scaling)
    let local_normal = normalize(vec3<f32>(local_pos.x, local_pos.y, 0.0));
    
    // Apply pixel-based radius scaling to local position BEFORE transformation
    // This preserves pipe length while scaling only the radius
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
        let world_center = pipe_transform * vec4<f32>(0.0, 0.0, 0.0, 1.0);
        let distance_to_camera = length(world_center.xyz - camera.eye_pos.xyz);
        let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
        world_per_pixel = (2.0 * distance_to_camera * tan(fovy_rad * 0.5)) / viewport_h;
    }
    
    let desired_world_r = px_radius * world_per_pixel;
    
    // Scale ONLY the radius (X,Y) while preserving length (Z)
    // Apply thickness multiplier to radius scaling
    let scaled_local_pos = vec3<f32>(
        local_pos.x * desired_world_r * 2.0 * pipe_thickness,  // Scale radius with thickness
        local_pos.y * desired_world_r * 2.0 * pipe_thickness,  // Scale radius with thickness
        local_pos.z                                            // Preserve length
    );
    
    // Transform to world space with scaled radius but original length
    let final_world_position = pipe_transform * vec4<f32>(scaled_local_pos, 1.0);
    
    // Transform normal properly - use inverse transpose for non-uniform scaling
    // For uniform scaling, we can use the transform matrix directly
    let normal_matrix = mat3x3<f32>(
        normalize(pipe_transform[0].xyz),
        normalize(pipe_transform[1].xyz), 
        normalize(pipe_transform[2].xyz)
    );
    let world_normal = normalize(normal_matrix * local_normal);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * final_world_position;
    out.world_normal = world_normal;
    out.color = pipe_data.color;
    out.quad_coord = vec2<f32>(0.0, 0.0); // Not used for pipes
    return out;
}

// Fragment shader with proper lighting for pipes
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Balanced lighting to show colors properly
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0)); // Light coming from front-right-top
    let ambient = 0.4;  // Moderate ambient
    let diffuse = max(0.0, dot(in.world_normal, -light_dir));
    let lighting = ambient + diffuse * 0.4;  // Moderate diffuse
    
    // Use the actual color from vertex shader
    let final_color = in.color * lighting;
    return vec4<f32>(final_color, 1.0);
}
