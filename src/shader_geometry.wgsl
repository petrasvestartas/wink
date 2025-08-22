// Efficient GPU geometry generation - Direct vertex generation without instancing
// Generates pipe and sphere geometry directly in vertex shader based on vertex_index

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

// Pipe transformation matrix
struct PipeTransform {
    transform: mat4x4<f32>,
}

// Sphere transformation matrix
struct SphereTransform {
    transform: mat4x4<f32>,
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
@group(0) @binding(1) var<storage, read> spheres: array<SphereTransform>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// Generate unit pipe vertex using computed approach (8-sided cylinder)
fn get_pipe_vertex_position(vertex_id: u32) -> vec3<f32> {
    let segments = 8u;
    
    if vertex_id < 8u {
        // Top ring (z = +0.5)
        let angle = f32(vertex_id) * 2.0 * 3.14159265 / f32(segments);
        return vec3<f32>(cos(angle) * 0.5, sin(angle) * 0.5, 0.5);
    } else {
        // Bottom ring (z = -0.5)
        let ring_vertex = vertex_id - 8u;
        let angle = f32(ring_vertex) * 2.0 * 3.14159265 / f32(segments);
        return vec3<f32>(cos(angle) * 0.5, sin(angle) * 0.5, -0.5);
    }
}

// Generate unit pipe vertex using computed triangulation (8-sided cylinder, 16 triangles total)
fn get_pipe_vertex(vertex_id: u32) -> vec3<f32> {
    let segments = 8u;
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    if triangle_id < segments {
        // Side triangles (8 triangles for cylinder sides)
        let segment = triangle_id;
        let next_segment = (segment + 1u) % segments;
        
        // Vertex indices: top ring (0-7), bottom ring (8-15)
        let top_curr = segment;
        let top_next = next_segment;
        let bottom_curr = segment + 8u;
        let bottom_next = next_segment + 8u;
        
        var vertex_index: u32;
        // Triangle: bottom_curr, top_next, top_curr (fixed winding)
        if vertex_in_triangle == 0u { vertex_index = bottom_curr; }
        else if vertex_in_triangle == 1u { vertex_index = top_next; }
        else { vertex_index = top_curr; }
        
        return get_pipe_vertex_position(vertex_index);
    } else if triangle_id < segments * 2u {
        // Second set of side triangles (8 more triangles)
        let segment = triangle_id - segments;
        let next_segment = (segment + 1u) % segments;
        
        let top_next = next_segment;
        let bottom_curr = segment + 8u;
        let bottom_next = next_segment + 8u;
        
        var vertex_index: u32;
        // Triangle: bottom_curr, bottom_next, top_next (fixed winding)
        if vertex_in_triangle == 0u { vertex_index = bottom_curr; }
        else if vertex_in_triangle == 1u { vertex_index = bottom_next; }
        else { vertex_index = top_next; }
        
        return get_pipe_vertex_position(vertex_index);
    } else {
        // Should not happen with 16 triangles
        return vec3<f32>(0.0, 0.0, 0.0);
    }
}

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

// Sphere faces (24 quads, each rendered as 2 triangles)
fn get_sphere_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_quad = 6u; // 2 triangles per quad = 6 vertices
    let quad_id = vertex_id / vertices_per_quad;
    let vertex_in_quad = vertex_id % vertices_per_quad;
    
    // 24 quads with counter-clockwise winding
    let quads = array<array<u32, 4>, 24>(
        array<u32, 4>(0u, 1u, 2u, 3u),     // Q0
        array<u32, 4>(0u, 4u, 5u, 1u),     // Q1
        array<u32, 4>(6u, 4u, 0u, 3u),     // Q2
        array<u32, 4>(3u, 2u, 8u, 7u),     // Q3
        array<u32, 4>(8u, 10u, 9u, 7u),    // Q4
        array<u32, 4>(3u, 7u, 9u, 6u),     // Q5
        array<u32, 4>(12u, 2u, 1u, 11u),   // Q6
        array<u32, 4>(1u, 5u, 13u, 11u),   // Q7
        array<u32, 4>(12u, 11u, 13u, 14u), // Q8
        array<u32, 4>(15u, 8u, 2u, 12u),   // Q9
        array<u32, 4>(15u, 16u, 10u, 8u),  // Q10
        array<u32, 4>(14u, 16u, 15u, 12u), // Q11
        array<u32, 4>(20u, 19u, 18u, 17u), // Q12
        array<u32, 4>(18u, 5u, 4u, 17u),   // Q13
        array<u32, 4>(20u, 17u, 4u, 6u),   // Q14
        array<u32, 4>(21u, 22u, 19u, 20u), // Q15
        array<u32, 4>(21u, 9u, 10u, 22u),  // Q16
        array<u32, 4>(6u, 9u, 21u, 20u),   // Q17
        array<u32, 4>(23u, 18u, 19u, 24u), // Q18
        array<u32, 4>(23u, 13u, 5u, 18u),  // Q19
        array<u32, 4>(14u, 13u, 23u, 24u), // Q20
        array<u32, 4>(24u, 19u, 22u, 25u), // Q21
        array<u32, 4>(22u, 10u, 16u, 25u), // Q22
        array<u32, 4>(24u, 25u, 16u, 14u)  // Q23
    );
    
    let quad = quads[quad_id];
    
    // Convert quad to triangles: (0,1,2) and (0,2,3)
    var vertex_index: u32;
    if vertex_in_quad == 0u { vertex_index = quad[0]; }      // Triangle 1, vertex 0
    else if vertex_in_quad == 1u { vertex_index = quad[1]; } // Triangle 1, vertex 1
    else if vertex_in_quad == 2u { vertex_index = quad[2]; } // Triangle 1, vertex 2
    else if vertex_in_quad == 3u { vertex_index = quad[0]; } // Triangle 2, vertex 0
    else if vertex_in_quad == 4u { vertex_index = quad[2]; } // Triangle 2, vertex 1
    else { vertex_index = quad[3]; }                         // Triangle 2, vertex 2
    
    return get_sphere_vertex_position(vertex_index);
}


// Vertex shader for pipes - NO INSTANCING, direct geometry generation
@vertex
fn vs_pipes(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices_per_pipe = 48u; // 16 triangles * 3 vertices
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
    
    // Get unit pipe vertex
    let local_pos = get_pipe_vertex(local_vertex_index);
    
    // Calculate proper cylindrical normal for lighting (before scaling)
    let local_normal = normalize(vec3<f32>(local_pos.x, local_pos.y, 0.0));
    
    // Apply pixel-based radius scaling to local position BEFORE transformation
    // This preserves pipe length while scaling only the radius
    let px_radius = camera.pipe_params.x;
    let viewport_h = camera.viewport_fovy_aspect_pipe_px_radius.y;
    let is_ortho = camera.pipe_params.z > 0.5;
    
    // Use fixed reference distance for consistent scaling across all objects
    var world_per_pixel: f32;
    if is_ortho {
        let ortho_half_height = camera.pipe_params.y;
        world_per_pixel = (2.0 * ortho_half_height) / viewport_h;
    } else {
        // Use a fixed reference distance for consistent perspective scaling
        let reference_distance = 10.0; // Fixed distance for consistent scaling
        let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
        world_per_pixel = (2.0 * reference_distance * tan(fovy_rad * 0.5)) / viewport_h;
    }
    
    let desired_world_r = px_radius * world_per_pixel;
    
    // Scale ONLY the radius (X,Y) while preserving length (Z)
    let scaled_local_pos = vec3<f32>(
        local_pos.x * desired_world_r * 2.0,  // Scale radius
        local_pos.y * desired_world_r * 2.0,  // Scale radius  
        local_pos.z                           // Preserve length
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
    out.color = vec3<f32>(0.0, 0.0, 0.0); // Black pipe color
    out.quad_coord = vec2<f32>(0.0, 0.0); // Not used for pipes
    return out;
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
    
    let sphere_transform = spheres[sphere_index].transform;
    
    // Get unit sphere vertex using the same geometry we defined earlier
    let local_pos = get_sphere_vertex(local_vertex_index);
    let local_normal = normalize(local_pos); // Sphere normal is normalized position
    
    // Use IDENTICAL scaling logic as pipes
    let px_radius = camera.pipe_params.x;
    let viewport_h = camera.viewport_fovy_aspect_pipe_px_radius.y;
    let is_ortho = camera.pipe_params.z > 0.5;
    
    // Use fixed reference distance for consistent scaling (identical to pipes)
    var world_per_pixel: f32;
    if is_ortho {
        let ortho_half_height = camera.pipe_params.y;
        world_per_pixel = (2.0 * ortho_half_height) / viewport_h;
    } else {
        // Use same fixed reference distance as pipes
        let reference_distance = 10.0; // Fixed distance for consistent scaling
        let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
        world_per_pixel = (2.0 * reference_distance * tan(fovy_rad * 0.5)) / viewport_h;
    }
    
    let desired_world_r = px_radius * world_per_pixel;
    
    // Scale sphere uniformly to match pipe radius exactly
    // Pipes scale radius by desired_world_r * 2.0, so spheres should match
    let sphere_radius_scale = desired_world_r * 2.0;
    let scaled_local_pos = local_pos * sphere_radius_scale;
    
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
    out.color = vec3<f32>(0.0, 0.0, 0.0); // Black sphere color
    out.quad_coord = vec2<f32>(0.0, 0.0); // Not used for 3D spheres
    return out;
}

// Fragment shader with proper lighting for both pipes and spheres
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0)); // Light coming from front-right-top
    let ambient = 0.3;
    let diffuse = max(0.0, dot(in.world_normal, -light_dir));
    let lighting = ambient + diffuse * 0.7;
    
    // Base color - dark gray for better visibility
    let base_color = vec3<f32>(1.0, 0.0, 0.0);
    let final_color = base_color * lighting;
    
    return vec4<f32>(final_color, 1.0);
}
