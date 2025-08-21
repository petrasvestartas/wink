// GPU-based pipe generation using compute shader + instanced rendering

struct LineData {
    start: vec3<f32>,
    end: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
}

struct PipeInstance {
    transform: mat4x4<f32>,
    color: vec4<f32>,
}

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

// Per-pipe transform (T * R * S_z for length)
struct PipeTransform {
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

// Storage buffers: pipes and camera
@group(0) @binding(0) var<storage, read> pipes: array<PipeTransform>;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// Hardcoded unit pipe (radius 0.5, length 1, aligned to +Z)
const SEGMENTS: u32 = 8u; // 8-sided cylinder

// 16 vertices: 0..7 top ring (z=+0.5), 8..15 bottom ring (z=-0.5)
const UNIT_VERTS: array<vec3<f32>, 16> = array<vec3<f32>, 16>(
    // Top ring
    vec3<f32>( 0.5,       0.0,       0.5),
    vec3<f32>( 0.353553,  0.353553,  0.5),
    vec3<f32>( 0.0,       0.5,       0.5),
    vec3<f32>(-0.353553,  0.353553,  0.5),
    vec3<f32>(-0.5,       0.0,       0.5),
    vec3<f32>(-0.353553, -0.353553,  0.5),
    vec3<f32>( 0.0,      -0.5,       0.5),
    vec3<f32>( 0.353553, -0.353553,  0.5),
    // Bottom ring
    vec3<f32>( 0.5,       0.0,      -0.5),
    vec3<f32>( 0.353553,  0.353553, -0.5),
    vec3<f32>( 0.0,       0.5,      -0.5),
    vec3<f32>(-0.353553,  0.353553, -0.5),
    vec3<f32>(-0.5,       0.0,      -0.5),
    vec3<f32>(-0.353553, -0.353553, -0.5),
    vec3<f32>( 0.0,      -0.5,      -0.5),
    vec3<f32>( 0.353553, -0.353553, -0.5)
);

// 16 side triangles (two per segment), indices into UNIT_VERTS (no caps) => 48 vertices total
const SIDE_TRIS: array<vec3<u32>, 16> = array<vec3<u32>, 16>(
    // First triangle per segment
    vec3<u32>( 8u,  9u,  1u),
    vec3<u32>( 9u, 10u,  2u),
    vec3<u32>(10u, 11u,  3u),
    vec3<u32>(11u, 12u,  4u),
    vec3<u32>(12u, 13u,  5u),
    vec3<u32>(13u, 14u,  6u),
    vec3<u32>(14u, 15u,  7u),
    vec3<u32>(15u,  8u,  0u),
    // Second triangle per segment
    vec3<u32>( 8u,  1u,  0u),
    vec3<u32>( 9u,  2u,  1u),
    vec3<u32>(10u,  3u,  2u),
    vec3<u32>(11u,  4u,  3u),
    vec3<u32>(12u,  5u,  4u),
    vec3<u32>(13u,  6u,  5u),
    vec3<u32>(14u,  7u,  6u),
    vec3<u32>(15u,  0u,  7u)
);

fn total_triangles() -> u32 { return 16u; }

fn get_local_pos(local_vertex_index: u32) -> vec3<f32> {
    let tri_id = local_vertex_index / 3u;       // 0..15
    let corner = local_vertex_index % 3u;       // 0..2
    let idx = select(select(SIDE_TRIS[tri_id].z, SIDE_TRIS[tri_id].y, corner == 1u), SIDE_TRIS[tri_id].x, corner == 0u);
    return UNIT_VERTS[idx];
}

@vertex
fn vs_pipes(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let verts_per_pipe = total_triangles() * 3u;
    let pipe_index = vertex_index / verts_per_pipe;
    let local_vertex_index = vertex_index % verts_per_pipe;

    // Bounds check
    if (pipe_index >= arrayLength(&pipes)) {
        var out: VertexOutput;
        out.clip_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        out.world_normal = vec3<f32>(0.0, 0.0, 1.0);
        out.color = vec3<f32>(1.0, 0.0, 0.0);
        return out;
    }

    let pipe_transform = pipes[pipe_index].transform;

    // Procedural local position (unit cylinder)
    let local_pos = get_local_pos(local_vertex_index);

    // Apply dynamic thickness in LOCAL XY before transform (respects orientation)
    // Convert desired pixel radius to world radius at this depth and orientation
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

    // World units per pixel at this depth
    let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
    var world_per_pixel: f32;
    if (is_ortho) {
        world_per_pixel = (2.0 * ortho_half_h) / viewport_h;
    } else {
        world_per_pixel = (2.0 * depth * tan(0.5 * fovy_rad)) / viewport_h;
    }
    let desired_world_r = max(px_radius * world_per_pixel, 1e-6);

    // Orientation compensation to keep apparent thickness when axis aligns with view
    let sin_theta = length(cross(axis_world_dir, vdir));
    let r_world_side = desired_world_r / max(sin_theta, 1e-3);
    let r_world_cap = desired_world_r;
    let w_parallel = smoothstep(0.1, 0.3, sin_theta);
    let req_world_r = mix(r_world_cap, r_world_side, w_parallel);

    // Local XY scale: unit radius is 0.5 -> scale factor = 2 * world radius
    let pipe_scale_xy = 2.0 * req_world_r;
    let scaled_local_pos = vec3<f32>(local_pos.x * pipe_scale_xy, local_pos.y * pipe_scale_xy, local_pos.z);

    // Compute local normal: radial for sides
    var local_normal: vec3<f32> = normalize(vec3<f32>(local_pos.x, local_pos.y, 0.0));

    // Transform position and normal to world space
    let world_position = pipe_transform * vec4<f32>(scaled_local_pos, 1.0);
    let normal_matrix = mat3x3<f32>(
        pipe_transform[0].xyz,
        pipe_transform[1].xyz,
        pipe_transform[2].xyz
    );
    let world_normal = normalize(normal_matrix * local_normal);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.color = vec3<f32>(0.3, 0.7, 0.4); // green-ish pipes
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let ndotl = max(dot(in.world_normal, light_dir), 0.1);
    let color = in.color * ndotl;
    return vec4<f32>(color, 1.0);
}
