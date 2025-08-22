// Point Cloud Shader - Instanced quad rendering based on WebGPU fundamentals
// Uses instanced rendering with shared quad geometry

// Matches Rust CameraUniform layout in camera.rs
struct CameraUniform {
    view_proj: mat4x4<f32>,
    // x: viewport width, y: viewport height, z: fovy (degrees), w: aspect
    viewport_fovy_aspect_pipe_px_radius: vec4<f32>,
    // x: pipe_px_radius, y: ortho_half_height, z: is_ortho (1.0 or 0.0), w: reserved
    pipe_params: vec4<f32>,
    // Camera eye position (world space)
    eye_pos: vec4<f32>,
    // Camera forward direction (world space)
    view_dir: vec4<f32>,
}

struct InstanceInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) size: f32,
}

struct VertexInput {
    @location(3) quad_pos: vec2<f32>, // Quad corner position (-1 to 1)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) quad_coord: vec2<f32>, // -1 to 1 quad coordinates for fragment shader
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // Use point position directly (transformation handled in instance data)
    let world_center = instance.position;
    
    // Get camera forward direction and create camera-facing basis vectors
    let camera_forward = normalize(camera.view_dir.xyz);
    let world_up = vec3<f32>(0.0, 0.0, 1.0);
    
    // Create right and up vectors for billboard
    var right = normalize(cross(camera_forward, world_up));
    if (length(cross(camera_forward, world_up)) < 0.001) {
        // Handle case where camera is looking straight up/down
        right = vec3<f32>(1.0, 0.0, 0.0);
    }
    let up = normalize(cross(right, camera_forward));
    
    // Calculate world-space size based on distance for perspective
    let distance_to_camera = length(world_center - camera.eye_pos.xyz);
    let world_size = instance.size * 1.0 * max(distance_to_camera * 0.1, 0.05);
    
    // Create billboard quad in world space
    let world_pos = world_center + 
        right * vertex.quad_pos.x * world_size +
        up * vertex.quad_pos.y * world_size;
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color = instance.color;
    out.quad_coord = vertex.quad_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create circular glyph using quad coordinates
    let dist_from_center = length(in.quad_coord);
    
    // Smooth circular falloff - discard pixels outside circle
    if (dist_from_center > 1.0) {
        discard;
    }
    
    // Smooth edge for antialiasing
    let alpha = 1.0 - smoothstep(0.8, 1.0, dist_from_center);
    
    // Use constant color without brightness variation
    return vec4<f32>(in.color, alpha);
}
