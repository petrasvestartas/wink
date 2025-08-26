// Arrow shader - renders both cylinder shafts and cone heads using ArrowTransform data

// Camera uniform structure
struct CameraUniform {
    view_proj: mat4x4<f32>,
    viewport_fovy_aspect_pipe_px_radius: vec4<f32>,
    pipe_params: vec4<f32>,
    eye_pos: vec4<f32>,
    view_dir: vec4<f32>,
}

// Arrow transformation data structure - match Rust layout exactly
struct ArrowTransform {
    cylinder_transform: array<f32, 16>,  // 4x4 matrix as flat array
    cylinder_color: array<f32, 3>,       // vec3 as array
    cylinder_thickness: f32,
    cone_transform: array<f32, 16>,      // 4x4 matrix as flat array  
    cone_color: array<f32, 3>,           // vec3 as array
    cone_thickness: f32,
    padding: array<f32, 2>,              // vec2 as array
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) quad_coord: vec2<f32>,
}

// Storage buffers
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<storage, read> arrows: array<ArrowTransform>;

// Cylinder vertices (20 vertices for 10-sided cylinder)
fn get_cylinder_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 20>(
        // Base circle (10 vertices)
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
        // Top circle (10 vertices)
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

// Cone vertices (9 vertices: 1 tip + 8 base)
fn get_cone_vertex_position(vertex_id: u32) -> vec3<f32> {
    let vertices = array<vec3<f32>, 9>(
        // Tip of cone
        vec3<f32>(0.0, 0.0, 1.0),
        // Base circle (8 vertices)
        vec3<f32>(0.5, 0.0, 0.0),
        vec3<f32>(0.353553, 0.353553, 0.0),
        vec3<f32>(0.0, 0.5, 0.0),
        vec3<f32>(-0.353553, 0.353553, 0.0),
        vec3<f32>(-0.5, 0.0, 0.0),
        vec3<f32>(-0.353553, -0.353553, 0.0),
        vec3<f32>(0.0, -0.5, 0.0),
        vec3<f32>(0.353553, -0.353553, 0.0)
    );
    return vertices[vertex_id];
}

// Cylinder faces (60 vertices = 20 triangles)
fn get_cylinder_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    let triangles = array<array<u32, 3>, 20>(
        // Side faces (10 rectangles = 20 triangles)
        array<u32, 3>(0u, 1u, 10u), array<u32, 3>(1u, 11u, 10u),
        array<u32, 3>(1u, 2u, 11u), array<u32, 3>(2u, 12u, 11u),
        array<u32, 3>(2u, 3u, 12u), array<u32, 3>(3u, 13u, 12u),
        array<u32, 3>(3u, 4u, 13u), array<u32, 3>(4u, 14u, 13u),
        array<u32, 3>(4u, 5u, 14u), array<u32, 3>(5u, 15u, 14u),
        array<u32, 3>(5u, 6u, 15u), array<u32, 3>(6u, 16u, 15u),
        array<u32, 3>(6u, 7u, 16u), array<u32, 3>(7u, 17u, 16u),
        array<u32, 3>(7u, 8u, 17u), array<u32, 3>(8u, 18u, 17u),
        array<u32, 3>(8u, 9u, 18u), array<u32, 3>(9u, 19u, 18u),
        array<u32, 3>(9u, 0u, 19u), array<u32, 3>(0u, 10u, 19u)
    );
    
    let triangle = triangles[triangle_id];
    let vertex_index = triangle[vertex_in_triangle];
    return get_cylinder_vertex_position(vertex_index);
}

// Cone faces (24 vertices = 8 triangles)
fn get_cone_vertex(vertex_id: u32) -> vec3<f32> {
    let vertices_per_triangle = 3u;
    let triangle_id = vertex_id / vertices_per_triangle;
    let vertex_in_triangle = vertex_id % vertices_per_triangle;
    
    let triangles = array<array<u32, 3>, 8>(
        // Side faces (tip to base edge)
        array<u32, 3>(0u, 1u, 2u),
        array<u32, 3>(0u, 2u, 3u),
        array<u32, 3>(0u, 3u, 4u),
        array<u32, 3>(0u, 4u, 5u),
        array<u32, 3>(0u, 5u, 6u),
        array<u32, 3>(0u, 6u, 7u),
        array<u32, 3>(0u, 7u, 8u),
        array<u32, 3>(0u, 8u, 1u)
    );
    
    let triangle = triangles[triangle_id];
    let vertex_index = triangle[vertex_in_triangle];
    return get_cone_vertex_position(vertex_index);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Each arrow has 84 vertices: 60 for cylinder + 24 for cone
    let vertices_per_arrow = 84u;
    let arrow_index = vertex_index / vertices_per_arrow;
    let vertex_in_arrow = vertex_index % vertices_per_arrow;
    
    // Debug: This should help identify if indexing is working
    // Arrow 0: vertices 0-83, Arrow 1: vertices 84-167, Arrow 2: vertices 168-251
    
    // Ensure we're accessing the correct arrow based on vertex index
    let arrow = arrows[arrow_index];
    
    // Debug: Force different colors based on arrow index to verify indexing
    var debug_color = vec3<f32>(1.0, 0.0, 0.0); // Default red
    if (arrow_index == 1u) {
        debug_color = vec3<f32>(0.0, 1.0, 0.0); // Green
    } else if (arrow_index == 2u) {
        debug_color = vec3<f32>(0.0, 0.0, 1.0); // Blue
    }
    
    var local_pos: vec3<f32>;
    var color: vec3<f32>;
    var transform: mat4x4<f32>;
    var thickness: f32;
    
    if (vertex_in_arrow < 60u) {
        // Cylinder vertex
        local_pos = get_cylinder_vertex(vertex_in_arrow);
        color = vec3<f32>(arrow.cylinder_color[0], arrow.cylinder_color[1], arrow.cylinder_color[2]);
        transform = mat4x4<f32>(
            vec4<f32>(arrow.cylinder_transform[0], arrow.cylinder_transform[1], arrow.cylinder_transform[2], arrow.cylinder_transform[3]),
            vec4<f32>(arrow.cylinder_transform[4], arrow.cylinder_transform[5], arrow.cylinder_transform[6], arrow.cylinder_transform[7]),
            vec4<f32>(arrow.cylinder_transform[8], arrow.cylinder_transform[9], arrow.cylinder_transform[10], arrow.cylinder_transform[11]),
            vec4<f32>(arrow.cylinder_transform[12], arrow.cylinder_transform[13], arrow.cylinder_transform[14], arrow.cylinder_transform[15])
        );
        thickness = arrow.cylinder_thickness;
    } else {
        // Cone vertex (vertices 60-83)
        let cone_vertex_id = vertex_in_arrow - 60u;
        local_pos = get_cone_vertex(cone_vertex_id);
        color = vec3<f32>(arrow.cone_color[0], arrow.cone_color[1], arrow.cone_color[2]);
        transform = mat4x4<f32>(
            vec4<f32>(arrow.cone_transform[0], arrow.cone_transform[1], arrow.cone_transform[2], arrow.cone_transform[3]),
            vec4<f32>(arrow.cone_transform[4], arrow.cone_transform[5], arrow.cone_transform[6], arrow.cone_transform[7]),
            vec4<f32>(arrow.cone_transform[8], arrow.cone_transform[9], arrow.cone_transform[10], arrow.cone_transform[11]),
            vec4<f32>(arrow.cone_transform[12], arrow.cone_transform[13], arrow.cone_transform[14], arrow.cone_transform[15])
        );
        thickness = arrow.cone_thickness;
    }
    
    // Apply thickness scaling - but only for cylinders, cones already scaled in transform
    var scaled_pos: vec3<f32>;
    if (vertex_in_arrow < 60u) {
        // Cylinder: apply thickness scaling
        scaled_pos = vec3<f32>(
            local_pos.x * thickness,
            local_pos.y * thickness,
            local_pos.z
        );
    } else {
        // Cone: already scaled in transform matrix, use as-is
        scaled_pos = local_pos;
    }
    
    // Transform to world space
    let world_pos = transform * vec4<f32>(scaled_pos, 1.0);
    
    // Calculate normal in world space
    let local_normal = normalize(vec3<f32>(local_pos.x, local_pos.y, local_pos.z * 0.5));
    let world_normal = normalize((transform * vec4<f32>(local_normal, 0.0)).xyz);
    
    var output: VertexOutput;
    output.clip_position = camera.view_proj * world_pos;
    output.world_normal = world_normal;
    output.color = color;
    output.quad_coord = vec2<f32>(local_pos.x, local_pos.y);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
