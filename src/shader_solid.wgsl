// Vertex shader

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};
 

struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Debug: bypass camera to isolate pipeline vs uniform issues
const BYPASS_CAMERA: bool = false; // camera ON by default; set true to bypass for debugging
// Debug: color faces by orientation to visualize culling
// For the SOLID pipeline, keep this OFF to render a constant light gray.
const DEBUG_FACE_COLORING: bool = true; // false -> constant gray

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
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Reassemble model matrix (WGSL matrices are column-major; these are vec4 columns)
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    // Pass-through color (used in color pipeline, ignored by solid unless debug)
    out.color = model.color;

    // Apply model first (world), then camera (view-proj)
    let world_pos = model_matrix * vec4<f32>(model.position, 1.0);

    if (BYPASS_CAMERA) {
        out.clip_position = world_pos;
    } else {
        out.clip_position = camera.view_proj * world_pos;
    }
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    if (DEBUG_FACE_COLORING) {
        // Debug mode: keep backs BLACK; fronts constant gray for SOLID pipeline
        if (is_front) {
            return vec4<f32>(0.7, 0.7, 0.7, 1.0);
        } else {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    } else {
        // Default SOLID look: constant light gray
        return vec4<f32>(0.7, 0.7, 0.7, 1.0);
    }
}
