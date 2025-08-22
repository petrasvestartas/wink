// Efficient GPU sphere geometry generation
// Generates sphere geometry directly in vertex shader based on vertex_index

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

// Sphere transformation matrix with color and thickness
struct SphereTransform {
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
@group(0) @binding(1) var<storage, read> spheres: array<SphereTransform>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// Sphere vertices (26 vertices) - scaled to match pipe radius
fn get_sphere_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 26>(
        vec3<f32>(-0.27823, -0.293484, -0.291871),
        vec3<f32>(-0.338033, 0.0, -0.36762),
        vec3<f32>(0.0, 0.0, -0.5),
        vec3<f32>(0.0, -0.361367, -0.344456),
        vec3<f32>(-0.357907, -0.347646, 0.0),
        vec3<f32>(-0.5, 0.0, 0.0),
        vec3<f32>(0.0, -0.5, 0.0),
        vec3<f32>(0.27823, -0.293484, -0.291871),
        vec3<f32>(0.338033, 0.0, -0.36762),
        vec3<f32>(0.357907, -0.347646, 0.0),
        vec3<f32>(0.5, 0.0, 0.0),
        vec3<f32>(-0.27823, 0.293484, -0.291871),
        vec3<f32>(0.0, 0.361367, -0.344456),
        vec3<f32>(-0.357907, 0.347646, 0.0),
        vec3<f32>(0.0, 0.5, 0.0),
        vec3<f32>(0.27823, 0.293484, -0.291871),
        vec3<f32>(0.357907, 0.347646, 0.0),
        vec3<f32>(-0.27823, -0.293484, 0.291871),
        vec3<f32>(-0.338033, 0.0, 0.36762),
        vec3<f32>(0.0, 0.0, 0.5),
        vec3<f32>(0.0, -0.361367, 0.344456),
        vec3<f32>(0.27823, -0.293484, 0.291871),
        vec3<f32>(0.338033, 0.0, 0.36762),
        vec3<f32>(-0.27823, 0.293484, 0.291871),
        vec3<f32>(0.0, 0.361367, 0.344456),
        vec3<f32>(0.27823, 0.293484, 0.291871)
    );
    // Scale to match pipe radius: pipes use 0.5 radius, spheres should match
    return vertices[vertex_id]; // No scaling - vertices already have 0.5 radius
}

// Sphere faces (48 triangles with counter-clockwise winding)
fn get_sphere_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    // 48 triangles (2 per quad face) with counter-clockwise winding
    let triangles = array<array<u32, 3>, 48>(
        // Q0: (0,1,2,3) -> triangles (0,1,2) and (0,2,3)
        array<u32, 3>(0u, 1u, 2u),
        array<u32, 3>(0u, 2u, 3u),
        // Q1: (0,4,5,1) -> triangles (0,4,5) and (0,5,1)
        array<u32, 3>(0u, 4u, 5u),
        array<u32, 3>(0u, 5u, 1u),
        // Q2: (6,4,0,3) -> triangles (6,4,0) and (6,0,3)
        array<u32, 3>(6u, 4u, 0u),
        array<u32, 3>(6u, 0u, 3u),
        // Q3: (3,2,8,7) -> triangles (3,2,8) and (3,8,7)
        array<u32, 3>(3u, 2u, 8u),
        array<u32, 3>(3u, 8u, 7u),
        // Q4: (8,10,9,7) -> triangles (8,10,9) and (8,9,7)
        array<u32, 3>(8u, 10u, 9u),
        array<u32, 3>(8u, 9u, 7u),
        // Q5: (3,7,9,6) -> triangles (3,7,9) and (3,9,6)
        array<u32, 3>(3u, 7u, 9u),
        array<u32, 3>(3u, 9u, 6u),
        // Q6: (12,2,1,11) -> triangles (12,2,1) and (12,1,11)
        array<u32, 3>(12u, 2u, 1u),
        array<u32, 3>(12u, 1u, 11u),
        // Q7: (1,5,13,11) -> triangles (1,5,13) and (1,13,11)
        array<u32, 3>(1u, 5u, 13u),
        array<u32, 3>(1u, 13u, 11u),
        // Q8: (12,11,13,14) -> triangles (12,11,13) and (12,13,14)
        array<u32, 3>(12u, 11u, 13u),
        array<u32, 3>(12u, 13u, 14u),
        // Q9: (15,8,2,12) -> triangles (15,8,2) and (15,2,12)
        array<u32, 3>(15u, 8u, 2u),
        array<u32, 3>(15u, 2u, 12u),
        // Q10: (15,16,10,8) -> triangles (15,16,10) and (15,10,8)
        array<u32, 3>(15u, 16u, 10u),
        array<u32, 3>(15u, 10u, 8u),
        // Q11: (14,16,15,12) -> triangles (14,16,15) and (14,15,12)
        array<u32, 3>(14u, 16u, 15u),
        array<u32, 3>(14u, 15u, 12u),
        // Q12: (20,19,18,17) -> triangles (20,19,18) and (20,18,17)
        array<u32, 3>(20u, 19u, 18u),
        array<u32, 3>(20u, 18u, 17u),
        // Q13: (18,5,4,17) -> triangles (18,5,4) and (18,4,17)
        array<u32, 3>(18u, 5u, 4u),
        array<u32, 3>(18u, 4u, 17u),
        // Q14: (20,17,4,6) -> triangles (20,17,4) and (20,4,6)
        array<u32, 3>(20u, 17u, 4u),
        array<u32, 3>(20u, 4u, 6u),
        // Q15: (21,22,19,20) -> triangles (21,22,19) and (21,19,20)
        array<u32, 3>(21u, 22u, 19u),
        array<u32, 3>(21u, 19u, 20u),
        // Q16: (21,9,10,22) -> triangles (21,9,10) and (21,10,22)
        array<u32, 3>(21u, 9u, 10u),
        array<u32, 3>(21u, 10u, 22u),
        // Q17: (6,9,21,20) -> triangles (6,9,21) and (6,21,20)
        array<u32, 3>(6u, 9u, 21u),
        array<u32, 3>(6u, 21u, 20u),
        // Q18: (23,18,19,24) -> triangles (23,18,19) and (23,19,24)
        array<u32, 3>(23u, 18u, 19u),
        array<u32, 3>(23u, 19u, 24u),
        // Q19: (23,13,5,18) -> triangles (23,13,5) and (23,5,18)
        array<u32, 3>(23u, 13u, 5u),
        array<u32, 3>(23u, 5u, 18u),
        // Q20: (14,13,23,24) -> triangles (14,13,23) and (14,23,24)
        array<u32, 3>(14u, 13u, 23u),
        array<u32, 3>(14u, 23u, 24u),
        // Q21: (24,19,22,25) -> triangles (24,19,22) and (24,22,25)
        array<u32, 3>(24u, 19u, 22u),
        array<u32, 3>(24u, 22u, 25u),
        // Q22: (22,10,16,25) -> triangles (22,10,16) and (22,16,25)
        array<u32, 3>(22u, 10u, 16u),
        array<u32, 3>(22u, 16u, 25u),
        // Q23: (24,25,16,14) -> triangles (24,25,16) and (24,16,14)
        array<u32, 3>(24u, 25u, 16u),
        array<u32, 3>(24u, 16u, 14u)
    );
    
    let triangle = triangles[triangle_id];
    let vertex_index = triangle[vertex_in_triangle];
    
    return get_sphere_vertex_position(vertex_index);
}

// Vertex shader for spheres - 3D geometry with proper lighting
@vertex
fn vs_spheres(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices_per_sphere = 144u; // 24 quads * 6 vertices (2 triangles per quad)
    let sphere_index = vertex_index / vertices_per_sphere;
    let local_vertex_index = vertex_index % vertices_per_sphere;
    
    // Safety check
    if (sphere_index >= arrayLength(&spheres)) {
        var out: VertexOutput;
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
        out.color = vec3<f32>(1.0, 0.0, 0.0); // Error color
        return out;
    }
    
    let sphere_data = spheres[sphere_index];
    let sphere_transform = sphere_data.transform;
    let sphere_color = sphere_data.color;
    let sphere_thickness = sphere_data.thickness;
    
    // Get unit sphere vertex using the same geometry we defined earlier
    let local_pos = get_sphere_vertex(local_vertex_index);
    let local_normal = normalize(local_pos); // Sphere normal is normalized position
    
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
        let world_center = sphere_transform * vec4<f32>(0.0, 0.0, 0.0, 1.0);
        let distance_to_camera = length(world_center.xyz - camera.eye_pos.xyz);
        let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
        world_per_pixel = (2.0 * distance_to_camera * tan(fovy_rad * 0.5)) / viewport_h;
    }
    
    let desired_world_r = px_radius * world_per_pixel;
    
    // Scale sphere uniformly to match pipe radius exactly
    // Pipes scale radius by desired_world_r * 2.0, so spheres should match
    // Scale ONLY the radius while preserving sphere shape
    // Apply thickness multiplier to radius scaling
    let scaled_local_pos = local_pos * desired_world_r * 2.0 * sphere_thickness;
    
    // Transform to world space
    let world_position = sphere_transform * vec4<f32>(scaled_local_pos, 1.0);
    
    // Transform normal properly - use normalized axes for correct lighting
    let normal_matrix = mat3x3<f32>(
        normalize(sphere_transform[0].xyz),
        normalize(sphere_transform[1].xyz),
        normalize(sphere_transform[2].xyz)
    );
    let world_normal = normalize(normal_matrix * local_normal);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.color = sphere_color; // Use sphere color from data
    out.quad_coord = vec2<f32>(0.0, 0.0); // Not used for spheres
    return out;
}

// Fragment shader with proper lighting for spheres
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Balanced lighting to show colors properly
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0)); // Light coming from front-right-top
    let ambient = 0.4;  // Moderate ambient
    let diffuse = max(0.0, dot(in.world_normal, -light_dir));
    let lighting = ambient + diffuse * 0.4;  // Moderate diffuse
    
    // Use the sphere color from vertex shader
    let final_color = in.color * lighting;
    
    return vec4<f32>(final_color, 1.0);
}
