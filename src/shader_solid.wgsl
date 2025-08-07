// Vertex shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Debug: bypass camera to isolate pipeline vs uniform issues
const BYPASS_CAMERA: bool = false; // camera ON by default; set true to bypass for debugging
// Debug: color faces by orientation to visualize culling
const DEBUG_FACE_COLORING: bool = true; // set false to use vertex color

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    if (BYPASS_CAMERA) {
        // Render in NDC-ish space without camera to verify drawing path
        out.clip_position = vec4<f32>(model.position.xy * 0.5, model.position.z, 1.0);
    } else {
        out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    }
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    if (DEBUG_FACE_COLORING) {
        // Green for front faces, Red for back faces
        var col: vec3<f32>;
        if (is_front) {
            col = vec3<f32>(0.2, 0.9, 0.2);
        } else {
            col = vec3<f32>(0.9, 0.2, 0.2);
        }
        return vec4<f32>(col, 1.0);
    } else {
        return vec4<f32>(in.color, 1.0);
    }
}
