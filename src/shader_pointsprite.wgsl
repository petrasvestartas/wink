// High-performance point sprite shader for millions of points
// Uses GPU point primitives instead of instanced quads

struct CameraUniform {
    view_proj: mat4x4<f32>,
    viewport_fovy_aspect_pipe_px_radius: vec4<f32>,
    pipe_params: vec4<f32>,
    eye_pos: vec4<f32>,
    view_dir: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) size: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Built-in point coordinate (0,0) to (1,1)
    let coord = gl_PointCoord - vec2<f32>(0.5);
    let dist_sq = dot(coord, coord);
    
    if (dist_sq > 0.25) { // Circle radius 0.5
        discard;
    }
    
    return vec4<f32>(input.color, 1.0);
}
