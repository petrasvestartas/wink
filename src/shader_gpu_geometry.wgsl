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
}

fn radians(deg: f32) -> f32 {
    return deg * 3.141592653589793 / 180.0;
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

// Simple icosphere vertices (12 vertices)
fn get_sphere_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 12>(
        vec3<f32>(0.0, 0.525731, 0.850651),
        vec3<f32>(0.0, -0.525731, 0.850651),
        vec3<f32>(0.0, 0.525731, -0.850651),
        vec3<f32>(0.0, -0.525731, -0.850651),
        vec3<f32>(0.850651, 0.0, 0.525731),
        vec3<f32>(-0.850651, 0.0, 0.525731),
        vec3<f32>(0.850651, 0.0, -0.525731),
        vec3<f32>(-0.850651, 0.0, -0.525731),
        vec3<f32>(0.525731, 0.850651, 0.0),
        vec3<f32>(-0.525731, 0.850651, 0.0),
        vec3<f32>(0.525731, -0.850651, 0.0),
        vec3<f32>(-0.525731, -0.850651, 0.0)
    );
    return normalize(vertices[vertex_id]) * 0.5;
}

// Simple icosphere faces (20 triangles)
fn get_sphere_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    // 20 triangle faces for icosphere
    let faces = array<array<u32, 3>, 20>(
        array<u32, 3>(0u, 1u, 4u), array<u32, 3>(0u, 4u, 8u), array<u32, 3>(0u, 8u, 9u), array<u32, 3>(0u, 9u, 5u), array<u32, 3>(0u, 5u, 1u),
        array<u32, 3>(1u, 5u, 11u), array<u32, 3>(5u, 9u, 7u), array<u32, 3>(9u, 8u, 2u), array<u32, 3>(8u, 4u, 6u), array<u32, 3>(4u, 1u, 10u),
        array<u32, 3>(1u, 11u, 10u), array<u32, 3>(11u, 5u, 7u), array<u32, 3>(5u, 7u, 9u), array<u32, 3>(7u, 2u, 9u), array<u32, 3>(2u, 8u, 9u),
        array<u32, 3>(8u, 6u, 2u), array<u32, 3>(6u, 4u, 8u), array<u32, 3>(4u, 10u, 6u), array<u32, 3>(10u, 1u, 4u), array<u32, 3>(3u, 7u, 2u)
    );
    
    let face = faces[triangle_id];
    let vertex_index = face[vertex_in_triangle];
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
    
    let pipe_transform = pipes[pipe_index].transform;
    
    // Get unit pipe vertex
    let local_pos = get_pipe_vertex(local_vertex_index);
    
    // Apply pipe thickness scaling in LOCAL coordinate system (before rotation)
    // Scale only XY (radius), preserve Z (length) - this respects pipe orientation
    // Compute world-space radius from desired pixel radius in the camera uniform
    let viewport = camera.viewport_fovy_aspect_pipe_px_radius.xy;
    let viewport_w = max(viewport.x, 1.0);
    let viewport_h = max(viewport.y, 1.0);
    let px_radius = max(camera.pipe_params.x, 0.0);
    let ortho_half_h = max(camera.pipe_params.y, 1e-6);
    let is_ortho = camera.pipe_params.z > 0.5;

    // Centerline world position at this local z (x=y=0)
    let axis_world = pipe_transform * vec4<f32>(0.0, 0.0, local_pos.z, 1.0);
    let eye = camera.eye_pos.xyz;
    let vdir = normalize(camera.view_dir.xyz);
    let depth = max(abs(dot(axis_world.xyz - eye, vdir)), 1e-6);

    // Axis direction in world space (column 2 of transform: local Z)
    let axis_world_dir = normalize(pipe_transform[2].xyz);

    // Convert pixel radius to world radius at this depth and projection
    let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
    var world_per_pixel: f32;
    if (is_ortho) {
        world_per_pixel = (2.0 * ortho_half_h) / viewport_h;
    } else {
        world_per_pixel = (2.0 * depth * tan(0.5 * fovy_rad)) / viewport_h;
    }
    let desired_world_r = max(px_radius * world_per_pixel, 1e-6);

    // Orientation compensation: when axis is parallel to view, side contribution vanishes
    let sin_theta = length(cross(axis_world_dir, vdir));
    let r_world_side = desired_world_r / max(sin_theta, 1e-3);
    let r_world_cap = desired_world_r;
    let w_parallel = smoothstep(0.1, 0.3, sin_theta);
    let req_world_r = mix(r_world_cap, r_world_side, w_parallel);

    // Local XY scale: unit pipe radius is 0.5, so scale factor = 2 * world_radius
    let pipe_scale_xy = 2.0 * req_world_r;
    let scaled_local_pos = vec3<f32>(
        local_pos.x * pipe_scale_xy,
        local_pos.y * pipe_scale_xy,
        local_pos.z
    );
    
    // Calculate proper cylindrical normal for lighting (before scaling)
    let local_normal = normalize(vec3<f32>(local_pos.x, local_pos.y, 0.0));
    
    // Transform to world space - OpenModel matrices are T*R*S (Translation*Rotation*Scale)
    let world_position = pipe_transform * vec4<f32>(scaled_local_pos, 1.0);
    
    // Transform normal properly (use upper 3x3 of transform matrix)
    let normal_matrix = mat3x3<f32>(
        pipe_transform[0].xyz,
        pipe_transform[1].xyz, 
        pipe_transform[2].xyz
    );
    let world_normal = normalize(normal_matrix * local_normal);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.color = vec3<f32>(0.3, 0.7, 0.4); // Green pipe color
    return out;
}

// Vertex shader for spheres - NO INSTANCING, direct geometry generation
@vertex
fn vs_spheres(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertices_per_sphere = 60u; // 20 triangles * 3 vertices
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
    
    // Get unit sphere vertex
    let local_pos = get_sphere_vertex(local_vertex_index);
    let local_normal = normalize(local_pos); // Sphere normal is normalized position
    
    // Transform to world space - OpenModel matrices are column-major
    let world_position = sphere_transform * vec4<f32>(local_pos, 1.0);
    
    // Transform normal properly (use transpose of inverse for non-uniform scaling)
    // For uniform scaling, we can use the upper 3x3 of the transform matrix
    let normal_matrix = mat3x3<f32>(
        sphere_transform[0].xyz,
        sphere_transform[1].xyz,
        sphere_transform[2].xyz
    );
    let world_normal = normalize(normal_matrix * local_normal);
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.color = vec3<f32>(0.8, 0.4, 0.2); // Orange-ish sphere color
    return out;
}

// Fragment shader with simple lighting
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let ndotl = max(dot(normalize(in.world_normal), light_dir), 0.1);
    let color = in.color * ndotl;
    return vec4<f32>(color, 1.0);
}
