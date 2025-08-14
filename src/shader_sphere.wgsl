// Sphere pipeline WGSL: constant pixel radius spheres using same camera uniform as pipes.

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
    // x: pipe pixel radius (also used for sphere), yzw: reserved
    pipe_params: vec4<f32>,
    // eye position in world space
    eye_pos: vec4<f32>,
    // camera forward direction (world space)
    view_dir: vec4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

const BYPASS_CAMERA: bool = false;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal_ws: vec3<f32>,
};

fn radians(deg: f32) -> f32 { return deg * 3.141592653589793 / 180.0; }

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Instance model matrix (column-major)
    let m = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    // Columns (world-space directions of local axes)
    let c0 = instance.model_matrix_0.xyz;
    let c1 = instance.model_matrix_1.xyz;
    let c2 = instance.model_matrix_2.xyz;

    // Sphere center in world space (model origin)
    let p_center_ws = m * vec4<f32>(0.0, 0.0, 0.0, 1.0);

    // Camera params
    let viewport = camera.viewport_fovy_aspect_pipe_px_radius.xy;
    let viewport_w = max(viewport.x, 1.0);
    let viewport_h = max(viewport.y, 1.0);
    let ndc_to_px = vec2<f32>(0.5 * viewport_w, 0.5 * viewport_h);
    let px_radius = max(camera.pipe_params.x, 0.0);
    let ortho_half_h = max(camera.pipe_params.y, 1e-6);
    let is_ortho = camera.pipe_params.z > 0.5;

    // Depth along view direction (stable above/below)
    let eye = camera.eye_pos.xyz;
    let vdir = normalize(camera.view_dir.xyz);
    let depth = max(abs(dot(p_center_ws.xyz - eye, vdir)), 1e-6);

    // FOV-based direct estimate
    let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
    var world_per_pixel: f32;
    if (is_ortho) {
        world_per_pixel = (2.0 * ortho_half_h) / viewport_h;
    } else {
        world_per_pixel = (2.0 * depth * tan(0.5 * fovy_rad)) / viewport_h;
    }
    let desired_world_r = max(px_radius * world_per_pixel, 1e-6);

    // Build world-space orthonormal directions from instance axes for normals/extrusion.
    let u_ws = normalize(c0);
    let v_ws = normalize(c1);
    let w_ws = normalize(c2);
    // Center in clip and NDC for refinement measurements
    let p_center_cs = camera.view_proj * p_center_ws;
    let p_center_ndc = p_center_cs.xy / p_center_cs.w;

    // Per-vertex: direction from center and final extrusion
    let radial_len = length(model.position);
    var world_pos: vec4<f32>;
    if (radial_len > 1e-6) {
        let radial_local = model.position / radial_len;
        let dir_ws = normalize(u_ws * radial_local.x + v_ws * radial_local.y + w_ws * radial_local.z);

        // Minimal refinement (Option B): measure pixels-per-world along dir_ws at centerline
        let eps = max(0.1 * desired_world_r, 1e-5);
        let pdir_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * eps, 1.0);
        let pdir_ndc = pdir_cs.xy / pdir_cs.w;
        let dpx_dir = length((pdir_ndc - p_center_ndc) * ndc_to_px);
        let r_dir0 = (px_radius * eps) / max(dpx_dir, 1e-6);

        // Start from a mix of analytic and measured estimate, then one Newton-style update
        let r0 = mix(desired_world_r, r_dir0, 0.8);
        let delta = max(0.02 * r0, 1e-5);
        let s0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * r0, 1.0);
        let s1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * (r0 + delta), 1.0);
        let s0_ndc = s0_cs.xy / s0_cs.w;
        let s1_ndc = s1_cs.xy / s1_cs.w;
        let px_at_r0 = length((s0_ndc - p_center_ndc) * ndc_to_px);
        let px_per_world_dir = length((s1_ndc - s0_ndc) * ndc_to_px) / max(delta, 1e-6);
        let r1 = r0 + (px_radius - px_at_r0) / max(px_per_world_dir, 1e-6);
        let r_final = clamp(r1, desired_world_r * 0.75, desired_world_r * 1.5);

        out.normal_ws = dir_ws;
        world_pos = vec4<f32>(p_center_ws.xyz + dir_ws * r_final, 1.0);
    } else {
        // Center vertex (should not happen on a proper sphere mesh)
        out.normal_ws = vec3<f32>(0.0, 0.0, 1.0);
        world_pos = p_center_ws;
    }

    out.color = model.color;
    out.clip_position = select(camera.view_proj * world_pos, world_pos, BYPASS_CAMERA);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Headlight-style lambert shading to reveal seams/corners
    // let N = normalize(in.normal_ws);
    // let L = normalize(-camera.view_dir.xyz);
    // let ndotl = max(dot(N, L), 0.0);
    // let lit = in.color * (0.2 + 0.8 * ndotl);
    // return vec4<f32>(lit, 1.0);
    return vec4<f32>(0.03, 0.03, 0.03, 1.0);
}
