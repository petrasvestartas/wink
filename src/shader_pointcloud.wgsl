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
    
    // Transform point to clip space first
    let clip_pos = camera.view_proj * vec4<f32>(world_center, 1.0);
    
    // Fixed pixel size - restore original size
    let pixel_radius = instance.size * 50.0; // Original size
    let viewport_width = camera.viewport_fovy_aspect_pipe_px_radius.x;
    let viewport_height = camera.viewport_fovy_aspect_pipe_px_radius.y;
    
    // Simplified NDC calculation
    let ndc_size_x = pixel_radius / viewport_width;
    let ndc_size_y = pixel_radius / viewport_height;
    
    // Apply quad offset in clip space
    var final_clip_pos = clip_pos;
    final_clip_pos.x += vertex.quad_pos.x * ndc_size_x * clip_pos.w;
    final_clip_pos.y += vertex.quad_pos.y * ndc_size_y * clip_pos.w;
    
    var out: VertexOutput;
    out.clip_position = final_clip_pos;
    out.color = instance.color;
    out.quad_coord = vertex.quad_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simplified fragment shader - remove expensive length calculation
    let dist_sq = dot(in.quad_coord, in.quad_coord);
    
    // Fast circular falloff using squared distance
    if (dist_sq > 1.0) {
        discard;
    }
    
    // Simplified alpha without smoothstep
    let alpha = 1.0 - dist_sq * 0.2;
    
    return vec4<f32>(in.color, alpha);
}
