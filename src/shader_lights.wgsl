// Lights pipeline WGSL
// - Uses unified instancing (locations 5..8) for model matrix per instance
// - Shares CameraUniform layout with pipe shader
// - Flat shading: compute geometric normal in fragment via derivatives (dpdx/dpdy) of world_pos
// - Ignores per-vertex normals; handles instancing and non-uniform scale robustly
// - Simple headlight Lambert shading with small ambient

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

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
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

const BYPASS_CAMERA: bool = false;
const AMBIENT: f32 = 0.1;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Rebuild instance model matrix (column-major)
    let M = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let world_pos = M * vec4<f32>(model.position, 1.0);
    out.world_pos = world_pos.xyz;
    out.color = model.color;

    // Normal to world space (upper-left 3x3 of model). Assumes orthonormal transform.
    let M3 = mat3x3<f32>(M[0].xyz, M[1].xyz, M[2].xyz);
    out.world_normal = normalize(M3 * model.normal);

    if (BYPASS_CAMERA) {
        out.clip_position = world_pos;
    } else {
        out.clip_position = camera.view_proj * world_pos;
    }

    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    // Flat normal via derivatives of world position (per-face)
    let dpdx_v = dpdx(in.world_pos);
    let dpdy_v = dpdy(in.world_pos);
    var N = normalize(cross(dpdy_v, dpdx_v));
    // Headlight: light from the viewer direction
    let V = normalize(camera.eye_pos.xyz - in.world_pos);
    // Ensure normal faces the viewer regardless of winding
    if (dot(N, V) < 0.0) { N = -N; }
    let ndotl = max(dot(N, V), 0.0);

    let lit = in.color * (AMBIENT + (1.0 - AMBIENT) * ndotl);
    return vec4<f32>(lit, 1.0);
}
