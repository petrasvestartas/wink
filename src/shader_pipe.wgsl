// Pipe pipeline WGSL (placeholder):
// Matches the interface of color/solid pipelines and uses per-vertex color.

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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal_ws: vec3<f32>,
};

fn radians(deg: f32) -> f32 {
    return deg * 3.141592653589793 / 180.0;
}

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    let m = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    // Column vectors (world-space directions of local X and Y)
    let c0 = instance.model_matrix_0.xyz;
    let c1 = instance.model_matrix_1.xyz;

    // Distance from camera to centerline point at same local z (x=y=0)
    let axis_world = m * vec4<f32>(0.0, 0.0, model.position.z, 1.0);
    let eye = camera.eye_pos.xyz;
    let vdir = normalize(camera.view_dir.xyz);
    // Use absolute depth magnitude along view direction to remain stable above/below
    let depth = max(abs(dot(axis_world.xyz - eye, vdir)), 1e-6);

    // Screen-derivative based scaling (robust near-parallel):
    // Compute pixel differential along two orthogonal radial directions and
    // infer the world radius needed to achieve the requested pixel radius.
    let viewport = camera.viewport_fovy_aspect_pipe_px_radius.xy;
    let viewport_w = max(viewport.x, 1.0);
    let viewport_h = max(viewport.y, 1.0);
    let px_radius = max(camera.pipe_params.x, 0.0);
    let ortho_half_h = max(camera.pipe_params.y, 1e-6);
    let is_ortho = camera.pipe_params.z > 0.5;

    // Center point on the cylinder axis at this local z (world and clip)
    let p_center_ws = axis_world;
    let p_center_cs = camera.view_proj * p_center_ws;
    let p_center_ndc = p_center_cs.xy / p_center_cs.w;

    // Constant-pixel NDC offset path (geometry-shader-style expansion)
    let use_ndc_offset: bool = false;
    if (use_ndc_offset) {
        out.color = model.color;

        // Two orthonormal radial directions in world space (u,v) perpendicular to the axis
        let axis_world_dir = normalize(instance.model_matrix_2.xyz);
        let up_ws = vec3<f32>(0.0, 1.0, 0.0);
        let t_ws = select(up_ws, vec3<f32>(1.0, 0.0, 0.0), abs(dot(axis_world_dir, up_ws)) > 0.99);
        let u_ws = normalize(cross(axis_world_dir, t_ws));
        let v_ws = normalize(cross(axis_world_dir, u_ws));

        // Build pixel-space basis from small world steps along u and v
        let viewport = camera.viewport_fovy_aspect_pipe_px_radius.xy;
        let viewport_w = max(viewport.x, 1.0);
        let viewport_h = max(viewport.y, 1.0);
        let ndc_to_px = vec2<f32>(0.5 * viewport_w, 0.5 * viewport_h);
        // Step size proportional to the unscaled ring radius for numerical stability
        let avg_xy = 0.5 * (length(instance.model_matrix_0.xyz) + length(instance.model_matrix_1.xyz));
        let eps2 = max(0.5 * avg_xy, 1e-6);
        let pu_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + u_ws * eps2, 1.0);
        let pv_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + v_ws * eps2, 1.0);
        let pu_ndc = pu_cs.xy / pu_cs.w;
        let pv_ndc = pv_cs.xy / pv_cs.w;
        let e_u_px = (pu_ndc - p_center_ndc) * ndc_to_px;
        let e_v_px = (pv_ndc - p_center_ndc) * ndc_to_px;
        let e_u_dir = e_u_px / max(length(e_u_px), 1e-6);
        let e_v_dir = e_v_px / max(length(e_v_px), 1e-6);

        // Radial direction from the mesh's local ring
        let px_radius = max(camera.pipe_params.x, 0.0);
        let radial_len = length(model.position.xy);
        var radial_unit = vec2<f32>(0.0, 0.0);
        if (radial_len > 1e-6) {
            radial_unit = model.position.xy / radial_len;
        }
        let dir_px = e_u_dir * radial_unit.x + e_v_dir * radial_unit.y;
        let dir_px_n = dir_px / max(length(dir_px), 1e-6);
        let offset_px = px_radius * dir_px_n;
        let ndc_offset = offset_px / ndc_to_px;
        let new_ndc = p_center_ndc + ndc_offset;
        let clip_xy = new_ndc * p_center_cs.w;
        out.clip_position = vec4<f32>(clip_xy.x, clip_xy.y, p_center_cs.z, p_center_cs.w);
        // Approximate world-space normal for potential shading
        let dir_ws = normalize(u_ws * radial_unit.x + v_ws * radial_unit.y);
        out.normal_ws = dir_ws;
        return out;
    }

    // Two orthonormal radial directions in world space (u,v) perpendicular to the axis
    let axis_world_dir = normalize(instance.model_matrix_2.xyz);
    let up_ws = vec3<f32>(0.0, 1.0, 0.0);
    let t_ws = select(up_ws, vec3<f32>(1.0, 0.0, 0.0), abs(dot(axis_world_dir, up_ws)) > 0.99);
    let u_ws = normalize(cross(axis_world_dir, t_ws));
    let v_ws = normalize(cross(axis_world_dir, u_ws));
    // Step size based on desired world radius for numerical stability (small relative step)
    var world_per_pixel_eps: f32;
    let fovy_rad_eps = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
    if (is_ortho) {
        world_per_pixel_eps = (2.0 * ortho_half_h) / viewport_h;
    } else {
        world_per_pixel_eps = (2.0 * depth * tan(0.5 * fovy_rad_eps)) / viewport_h;
    }
    let desired_world_r_eps = max(px_radius * world_per_pixel_eps, 1e-6);
    let eps = max(0.1 * desired_world_r_eps, 1e-5);

    // Projected pixel delta for small world steps along 4 radial dirs (u, v, u+v, u-v)
    let d1_ws = normalize(u_ws + v_ws);
    let d2_ws = normalize(u_ws - v_ws);
    let p0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + u_ws * eps, 1.0);
    let p1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + v_ws * eps, 1.0);
    let p2_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d1_ws * eps, 1.0);
    let p3_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d2_ws * eps, 1.0);
    let p0_ndc = p0_cs.xy / p0_cs.w;
    let p1_ndc = p1_cs.xy / p1_cs.w;
    let p2_ndc = p2_cs.xy / p2_cs.w;
    let p3_ndc = p3_cs.xy / p3_cs.w;
    let ndc_to_px = vec2<f32>(0.5 * viewport_w, 0.5 * viewport_h);
    let dpx0 = length((p0_ndc - p_center_ndc) * ndc_to_px);
    let dpx1 = length((p1_ndc - p_center_ndc) * ndc_to_px);
    let dpx2 = length((p2_ndc - p_center_ndc) * ndc_to_px);
    let dpx3 = length((p3_ndc - p_center_ndc) * ndc_to_px);

    // Pixels per world for the eps step -> required world radius to reach px_radius
    let r_world0 = (px_radius * eps) / max(dpx0, 1e-6);
    let r_world1 = (px_radius * eps) / max(dpx1, 1e-6);
    let r_world2 = (px_radius * eps) / max(dpx2, 1e-6);
    let r_world3 = (px_radius * eps) / max(dpx3, 1e-6);
    let p_exp = 6.0;
    let r_world_deriv = pow((pow(r_world0, p_exp) + pow(r_world1, p_exp) + pow(r_world2, p_exp) + pow(r_world3, p_exp)) / 4.0, 1.0 / p_exp);

    // FOV-based side/cap estimates (analytical fallback)
    let fovy_rad = radians(camera.viewport_fovy_aspect_pipe_px_radius.z);
    var world_per_pixel: f32;
    if (is_ortho) {
        world_per_pixel = (2.0 * ortho_half_h) / viewport_h;
    } else {
        world_per_pixel = (2.0 * depth * tan(0.5 * fovy_rad)) / viewport_h;
    }
    let desired_world_r = max(px_radius * world_per_pixel, 1e-6);
    let sin_theta = length(cross(axis_world_dir, vdir));
    let r_world_side = desired_world_r / max(sin_theta, 1e-3);
    let r_world_cap = desired_world_r;

    // Robust aggregator: median of {cap, side, derivative}
    let a = r_world_cap;
    let b = r_world_side;
    let c = r_world_deriv;
    let mn = min(min(a, b), c);
    let mx = max(max(a, b), c);
    let r_sum = a + b + c;
    let req_world_r = r_sum - mn - mx;

    // Blend toward cap estimate when axis nearly parallels view to remove residual wobble
    let w_parallel = smoothstep(0.1, 0.3, sin_theta); // 0 near-parallel -> cap, 1 -> aggregated
    let req_world_r_soft = mix(r_world_cap, req_world_r, w_parallel);

    // Surface-based refinement for large radii: measure pixel differential at the predicted surface
    let r_sample = max(req_world_r_soft, 1e-6);
    let delta = max(0.02 * r_sample, 1e-5);
    // U direction
    let su0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + u_ws * r_sample, 1.0);
    let su1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + u_ws * (r_sample + delta), 1.0);
    let su0_ndc = su0_cs.xy / su0_cs.w;
    let su1_ndc = su1_cs.xy / su1_cs.w;
    let dpx_u = length((su1_ndc - su0_ndc) * ndc_to_px);
    let px_per_world_u = dpx_u / max(delta, 1e-6);
    let r_world_u = px_radius / max(px_per_world_u, 1e-6);
    // V direction
    let sv0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + v_ws * r_sample, 1.0);
    let sv1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + v_ws * (r_sample + delta), 1.0);
    let sv0_ndc = sv0_cs.xy / sv0_cs.w;
    let sv1_ndc = sv1_cs.xy / sv1_cs.w;
    let dpx_v = length((sv1_ndc - sv0_ndc) * ndc_to_px);
    let px_per_world_v = dpx_v / max(delta, 1e-6);
    let r_world_v = px_radius / max(px_per_world_v, 1e-6);
    // Diagonals at surface
    let sd1_0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d1_ws * r_sample, 1.0);
    let sd1_1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d1_ws * (r_sample + delta), 1.0);
    let sd2_0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d2_ws * r_sample, 1.0);
    let sd2_1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + d2_ws * (r_sample + delta), 1.0);
    let sd1_0_ndc = sd1_0_cs.xy / sd1_0_cs.w;
    let sd1_1_ndc = sd1_1_cs.xy / sd1_1_cs.w;
    let sd2_0_ndc = sd2_0_cs.xy / sd2_0_cs.w;
    let sd2_1_ndc = sd2_1_cs.xy / sd2_1_cs.w;
    let dpx_d1 = length((sd1_1_ndc - sd1_0_ndc) * ndc_to_px);
    let dpx_d2 = length((sd2_1_ndc - sd2_0_ndc) * ndc_to_px);
    let px_per_world_d1 = dpx_d1 / max(delta, 1e-6);
    let px_per_world_d2 = dpx_d2 / max(delta, 1e-6);
    let r_world_d1 = px_radius / max(px_per_world_d1, 1e-6);
    let r_world_d2 = px_radius / max(px_per_world_d2, 1e-6);
    let req_world_r_surface = pow((pow(r_world_u, p_exp) + pow(r_world_v, p_exp) + pow(r_world_d1, p_exp) + pow(r_world_d2, p_exp)) / 4.0, 1.0 / p_exp);
    // Blend refinement in to reduce nonlinearity error for big radii
    let req_world_r_refined = mix(req_world_r_soft, req_world_r_surface, 0.6);

    // One Newton-style update at r_sample using measured pixel radius
    let px_u_at_r = length((su0_ndc - p_center_ndc) * ndc_to_px);
    let px_v_at_r = length((sv0_ndc - p_center_ndc) * ndc_to_px);
    let px_d1_at_r = length((sd1_0_ndc - p_center_ndc) * ndc_to_px);
    let px_d2_at_r = length((sd2_0_ndc - p_center_ndc) * ndc_to_px);
    let r_new_u  = r_sample + (px_radius - px_u_at_r)  / max(px_per_world_u,  1e-6);
    let r_new_v  = r_sample + (px_radius - px_v_at_r)  / max(px_per_world_v,  1e-6);
    let r_new_d1 = r_sample + (px_radius - px_d1_at_r) / max(px_per_world_d1, 1e-6);
    let r_new_d2 = r_sample + (px_radius - px_d2_at_r) / max(px_per_world_d2, 1e-6);
    let r_new_soft = pow((pow(r_new_u, p_exp) + pow(r_new_v, p_exp) + pow(r_new_d1, p_exp) + pow(r_new_d2, p_exp)) / 4.0, 1.0 / p_exp);
    let req_world_r_refined2 = mix(req_world_r_refined, r_new_soft, 0.6);

    // Tighter clamp to avoid minor spikes while allowing perspective variation
    let req_world_r_clamped = clamp(req_world_r_refined2, desired_world_r * 0.75, desired_world_r * 1.5);

    out.color = model.color;
    // Direction-specific world extrusion to stabilize large radii
    let radial_len = length(model.position.xy);
    var world_pos: vec4<f32>;
    if (radial_len > 1e-6) {
        let radial_local = model.position.xy / radial_len;
        let dir_ws = normalize(u_ws * radial_local.x + v_ws * radial_local.y);

        // Centerline differential along the vertex radial direction
        let pdir_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * eps, 1.0);
        let pdir_ndc = pdir_cs.xy / pdir_cs.w;
        let dpx_dir = length((pdir_ndc - p_center_ndc) * ndc_to_px);
        let r_dir0 = (px_radius * eps) / max(dpx_dir, 1e-6);

        // Surface refine along dir_ws
        let r0 = mix(req_world_r_clamped, r_dir0, 0.8);
        let delta_dir = max(0.02 * r0, 1e-5);
        let s0_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * r0, 1.0);
        let s1_cs = camera.view_proj * vec4<f32>(p_center_ws.xyz + dir_ws * (r0 + delta_dir), 1.0);
        let s0_ndc = s0_cs.xy / s0_cs.w;
        let s1_ndc = s1_cs.xy / s1_cs.w;
        let px_at_r0 = length((s0_ndc - p_center_ndc) * ndc_to_px);
        let px_per_world_dir = length((s1_ndc - s0_ndc) * ndc_to_px) / max(delta_dir, 1e-6);
        let r1 = r0 + (px_radius - px_at_r0) / max(px_per_world_dir, 1e-6);

        let r_final = clamp(r1, desired_world_r * 0.75, desired_world_r * 1.5);
        out.normal_ws = dir_ws;
        world_pos = vec4<f32>(p_center_ws.xyz + dir_ws * r_final, 1.0);
    } else {
        out.normal_ws = axis_world_dir;
        world_pos = p_center_ws;
    }
    if (BYPASS_CAMERA) {
        out.clip_position = world_pos;
    } else {
        out.clip_position = camera.view_proj * world_pos;
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Headlight-style lambert shading to reveal seams/corners
    let N = normalize(in.normal_ws);
    let L = normalize(-camera.view_dir.xyz);
    let ndotl = max(dot(N, L), 0.0);
    let lit = in.color * (0.2 + 0.8 * ndotl);
    return vec4<f32>(lit, 1.0);
}
