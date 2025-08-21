use cgmath::*;
use crate::timing::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

use openmodel::primitives::Xform;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

// Helpers: convert between cgmath::Matrix4<f32> and openmodel::primitives::Xform (column-major)
fn mat4f32_to_xform(m: &Matrix4<f32>) -> Xform {
    let mut out = [0.0f32; 16];
    // Column-major: index = col * 4 + row
    out[0] = m[0][0]; out[1] = m[0][1]; out[2] = m[0][2]; out[3] = m[0][3];
    out[4] = m[1][0]; out[5] = m[1][1]; out[6] = m[1][2]; out[7] = m[1][3];
    out[8] = m[2][0]; out[9] = m[2][1]; out[10] = m[2][2]; out[11] = m[2][3];
    out[12] = m[3][0]; out[13] = m[3][1]; out[14] = m[3][2]; out[15] = m[3][3];
    Xform { m: out }
}

fn xform_to_mat4f32(xf: &Xform) -> Matrix4<f32> {
    let c0 = Vector4::new(xf.m[0], xf.m[1], xf.m[2], xf.m[3]);
    let c1 = Vector4::new(xf.m[4], xf.m[5], xf.m[6], xf.m[7]);
    let c2 = Vector4::new(xf.m[8], xf.m[9], xf.m[10], xf.m[11]);
    let c3 = Vector4::new(xf.m[12], xf.m[13], xf.m[14], xf.m[15]);
    Matrix4::from_cols(c0, c1, c2, c3)
}

// Camera constraints
const MIN_ZOOM_DISTANCE: f32 = 0.5;
const MAX_ZOOM_DISTANCE: f32 = 100.0;

// Professional 3D orbit camera implementation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraViewMode {
    Perspective,
    TopView,
}

#[derive(Debug)]
pub struct Camera {
    // Eye position in 3D space
    pub position: Point3<f32>,
    // Center/target point that the camera looks at
    pub target: Point3<f32>,
    // Up direction, typically (0, 1, 0)
    pub up: Vector3<f32>,
    // Distance from target (used for zoom)
    pub distance: f32,
    // Quaternion for orientation instead of yaw/pitch
    pub orientation: Quaternion<f32>,
    // The world up direction (typically Z in 3D modeling software)
    pub world_up: Vector3<f32>,
    // Whether to maintain world up vector (turntable/orbit mode) or allow free rotation
    pub turntable_mode: bool,
    // Reference vectors to track orientation and prevent flipping
    pub reference_frame: Matrix3<f32>,  // Stable reference frame used for consistent rotations
    pub last_right: Vector3<f32>,      // Cached right vector for stable pole handling

    // Original camera settings to enable returning to default view
    pub initial_position: Point3<f32>,
    pub initial_target: Point3<f32>,
    pub initial_orientation: Quaternion<f32>,
    pub initial_distance: f32,

    // Legacy fields for compatibility
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    // Projection mode: perspective (default) or orthographic
    pub is_ortho: bool,
    // Orthographic half-height of the view volume (world units)
    pub ortho_half_height: f32,

    // Camera view mode tracking
    pub view_mode: CameraViewMode,
    // Saved perspective state for toggling
    pub saved_perspective_position: Point3<f32>,
    pub saved_perspective_target: Point3<f32>,
    pub saved_perspective_orientation: Quaternion<f32>,
    pub saved_perspective_distance: f32,
    pub saved_perspective_is_ortho: bool,
    pub saved_perspective_ortho_half_height: f32,

    // OpenModel camera pose: world_from_camera transform kept in sync with the camera state
    pub om_world_from_camera: Xform,
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        let position = Point3::new(0.0, 10.0, 10.0);  // Start above and in front of target (matching wgpu_viewer)
        let target = Point3::new(0.0, 0.0, 0.0);

        // Calculate initial distance from target
        let distance = (position - target).magnitude();

        // Calculate initial orientation based on position
        let dir = (target - position).normalize();

        // Define world up vector (Z-up for professional 3D software standard)
        let world_up = Vector3::unit_z();

        // Calculate initial orientation quaternion
        let orientation = Quaternion::look_at(dir, world_up);

        // Initialize stable reference frame
        let forward = -dir;
        let right = if (forward.dot(world_up) as f32).abs() > 0.99 {
            // If aligned with pole, pick an arbitrary but consistent right vector
            Vector3::unit_x()
        } else {
            // Normal case - get perpendicular right vector
            forward.cross(world_up).normalize()
        };
        let up = right.cross(forward).normalize();

        // Create reference frame matrix from orthogonal basis vectors
        let reference_frame = Matrix3::from_cols(right, up, forward);

        // Create Camera with professional default settings
        let mut cam = Self {
            position,
            target,
            up: world_up,  // Z-up coordinate system (professional 3D software standard)
            distance,
            orientation,
            world_up: Vector3::unit_z(),  // Z-up for turntable orbit mode
            turntable_mode: true,  // Default to turntable mode (professional standard)
            reference_frame,
            last_right: right,

            // Store initial camera settings for reset functionality
            initial_position: position,
            initial_target: target,
            initial_orientation: orientation,
            initial_distance: distance,

            // Legacy fields
            aspect: width / height,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
            // Start in perspective; initialize ortho scale to roughly match current view at target
            is_ortho: false,
            ortho_half_height: {
                let fovy_rad = 0.5f32 * 45.0f32.to_radians();
                distance * fovy_rad.tan()
            },

            // Camera view mode tracking
            view_mode: CameraViewMode::Perspective,
            // Initialize saved perspective state with current values
            saved_perspective_position: position,
            saved_perspective_target: target,
            saved_perspective_orientation: orientation,
            saved_perspective_distance: distance,
            saved_perspective_is_ortho: false,
            saved_perspective_ortho_half_height: {
                let fovy_rad = 0.5f32 * 45.0f32.to_radians();
                distance * fovy_rad.tan()
            },

            // OpenModel camera pose (initialized to identity; updated below)
            om_world_from_camera: Xform::identity(),
        };

        cam.update_position();
        cam.update_om_xform();
        cam
    }

    pub fn forward_dir(&self) -> Vector3<f32> {
        // Forward points from eye toward target
        (self.target - self.position).normalize()
    }

    // Update the camera position based on quaternion orientation and distance
    pub fn update_position(&mut self) {
        if self.turntable_mode {
            // Pure quaternion-based camera implementation for seamless orbit
            // This eliminates Euler angles entirely and properly avoids gimbal lock

            // Step 1: Calculate position from orientation quaternion
            // The initial view direction is along -Y in our coordinate system
            let initial_offset = Vector3::new(0.0, -self.distance, 0.0);

            // Apply the orientation quaternion to get the final position offset
            let final_offset = self.orientation.rotate_vector(initial_offset);

            self.position = self.target + final_offset;

            // Get forward vector from current orientation
            let forward = -self.orientation.rotate_vector(Vector3::unit_y());

            // Update reference frame to maintain continuity
            // When we get close to the poles, we use the previous reference frame's right vector
            // as a stable reference, rather than recomputing it from scratch
            let alignment = (forward.dot(self.world_up) as f32).abs();

            let right = if alignment > 0.98 {
                // Near pole - use the last stable right vector
                // This prevents the sudden 180-degree flip when crossing poles
                self.last_right
            } else {
                // Normal case - compute right vector perpendicular to forward and world up
                let computed_right = forward.cross(self.world_up).normalize();

                // To prevent instability when approaching the pole,
                // we ensure the new right vector doesn't flip relative to the previous one
                if computed_right.dot(self.last_right) < 0.0 {
                    -computed_right // Flip to maintain consistency with last frame
                } else {
                    computed_right
                }
            };

            // Store right vector for next frame
            self.last_right = right;

            // Compute up vector from right and forward to complete orthogonal basis
            // This ensures the up vector is always perpendicular to the view direction
            let up = right.cross(forward).normalize();

            // Update reference frame matrix
            self.reference_frame = Matrix3::from_cols(right, up, forward);

            // Use the up vector from our continuously tracked reference frame
            self.up = up;
        } else {
            // Free orbit mode - use quaternion directly
            let initial_offset = Vector3::new(0.0, 0.0, -self.distance);
            let final_offset = self.orientation.rotate_vector(initial_offset);
            self.position = self.target + final_offset;
            self.up = self.orientation.rotate_vector(Vector3::unit_y());
        }

        // Keep OpenModel camera pose in sync with the current camera state
        self.update_om_xform();
    }

    /// Update the OpenModel world_from_camera transform from the current position/target/up
    fn update_om_xform(&mut self) {
        let view = Matrix4::look_at_rh(self.position, self.target, self.up);
        let world_from_cam = view.invert().unwrap_or(Matrix4::identity());
        self.om_world_from_camera = mat4f32_to_xform(&world_from_cam);
    }

    // Reset the camera to its initial position and orientation
    pub fn reset_to_initial(&mut self) {
        self.position = self.initial_position;
        self.target = self.initial_target;
        self.orientation = self.initial_orientation;
        self.distance = self.initial_distance;

        // Reset reference frame
        let dir = (self.target - self.position).normalize();
        let forward = -dir;
        let right = forward.cross(self.world_up).normalize();
        let up = right.cross(forward).normalize();
        self.reference_frame = Matrix3::from_cols(right, up, forward);
        self.last_right = right;

    }
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
    // In TopView, force an exact view with +Y as up and -Z forward to avoid any roll.
    // Otherwise, prefer OpenModel world_from_camera; fall back to look_at if non-invertible.
    let view = if self.view_mode == CameraViewMode::TopView && self.is_ortho {
        Matrix4::look_at_rh(self.position, self.target, Vector3::unit_y())
    } else {
        let w_from_c = xform_to_mat4f32(&self.om_world_from_camera);
        if let Some(inv) = w_from_c.invert() { inv } else { Matrix4::look_at_rh(self.position, self.target, self.up) }
    };
    let proj = if self.is_ortho {
        let half_h = self.ortho_half_height.max(1e-6);
        let half_w = half_h * self.aspect.max(1e-6);
        // Use much larger near/far planes to avoid clipping geometry
        ortho(-half_w, half_w, -half_h, half_h, -1000.0, 1000.0)
    } else {
        perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar)
    };
    OPENGL_TO_WGPU_MATRIX * proj * view
    }
    /// and stores the OpenModel transform for view construction.
    pub fn set_om_world_from_camera(&mut self, xf: Xform) {
        // When in TopView, ignore external OM camera transforms to prevent overriding
        // the enforced orthographic top-down orientation and target/position.
        if self.view_mode == CameraViewMode::TopView {
            log::warn!(
                "Ignoring external OM transform while in TopView to prevent rotation override"
            );
            return;
        }
        // Extract basis vectors and translation from column-major matrix
        let right_ws = Vector3::new(xf.m[0] as f32, xf.m[1] as f32, xf.m[2] as f32).normalize();
        let up_ws    = Vector3::new(xf.m[4] as f32, xf.m[5] as f32, xf.m[6] as f32).normalize();
        let cam_z_ws = Vector3::new(xf.m[8] as f32, xf.m[9] as f32, xf.m[10] as f32).normalize();
        let pos_ws   = Point3::new(xf.m[12] as f32, xf.m[13] as f32, xf.m[14] as f32);

        // Camera looks along -Z in its local space; world forward is therefore -cam_z column
        let forward_ws = (-cam_z_ws).normalize();

        // Update OM transform first (authoritative for view)
        self.om_world_from_camera = xf;

        // Update camera spatial fields
        self.position = pos_ws;
        self.up = up_ws;
        self.target = self.position + forward_ws * self.distance;

        // Update reference frame and cached right for stability
        self.reference_frame = Matrix3::from_cols(right_ws, up_ws, forward_ws);
        self.last_right = right_ws;

        // Update orientation to roughly match the view direction
        self.orientation = Quaternion::look_at(forward_ws, self.world_up).normalize();
    }

    // Pan camera in view plane (right and up vectors)
    pub fn pan(&mut self, right_amount: f32, up_amount: f32) {
        // Get current view vectors from reference frame
        let right = self.reference_frame.x;
        let up = self.reference_frame.y;

        // Move target and position together to maintain relative positioning
        let pan_offset = right * right_amount + up * up_amount;
        self.target += pan_offset;
        self.position += pan_offset;

        // Update initial target for reset functionality
        self.initial_target += pan_offset;
        self.initial_position += pan_offset;

        // Update OM transform after panning
        self.update_om_xform();
    }

    // Dolly camera along its forward direction (moves position and target together)
    pub fn dolly(&mut self, amount: f32) {
        if amount == 0.0 { return; }
        let dir = self.forward_dir();
        let delta = dir * amount;
        self.position += delta;
        self.target += delta;
        // Keep OpenModel camera pose in sync
        self.update_om_xform();
    }

    // Toggle between perspective and orthographic projection
    pub fn toggle_view_mode(&mut self) {
        if self.is_ortho {
            // Switch to perspective and restore saved perspective state (if coming from TopView)
            self.is_ortho = false;
            self.view_mode = CameraViewMode::Perspective;

            // Restore saved perspective camera pose/orientation
            self.position = self.saved_perspective_position;
            self.target = self.saved_perspective_target;
            self.orientation = self.saved_perspective_orientation;
            self.distance = self.saved_perspective_distance;

            // Rebuild reference frame and up vector from restored pose
            let dir = (self.target - self.position).normalize();
            let forward = -dir;
            let right = forward.cross(self.world_up).normalize();
            let up = right.cross(forward).normalize();
            self.reference_frame = Matrix3::from_cols(right, up, forward);
            self.last_right = right;
            self.up = up;

            // Sync OM transform so build_view_projection uses the restored view
            self.update_om_xform();

            log::info!("Switched to perspective projection (restored)");
        } else {
            // Switch to orthographic
            self.is_ortho = true;
            self.ortho_half_height = 6.0;
            log::info!("Switched to orthographic projection with half_height={}", self.ortho_half_height);
        }
    }
    
    // Cycle through: Perspective -> TopView -> Parallel (orthographic, free-orbit)
    pub fn cycle_view_triple(&mut self) {
        if !self.is_ortho && self.view_mode != CameraViewMode::TopView {
            // State A: Perspective -> go to TopView (locked, ortho)
            self.set_top_view();
            return;
        }

        if self.view_mode == CameraViewMode::TopView {
            // State B: TopView -> go to Parallel-Ortho (free orbit, keep ortho scale)
            // Use the saved perspective pose to avoid starting exactly at the pole
            self.view_mode = CameraViewMode::Perspective; // re-enable orbit
            self.is_ortho = true; // stay orthographic

            // Restore saved perspective spatial state (pose), but remain in orthographic projection
            self.position = self.saved_perspective_position;
            self.target = self.saved_perspective_target;
            self.orientation = self.saved_perspective_orientation;
            self.distance = self.saved_perspective_distance;
            // Note: keep current ortho_half_height so user-controlled scale is preserved

            // Apply a 45° yaw around world Z to the restored pose for the Parallel view
            let yaw_rotation = Quaternion::from_axis_angle(self.world_up, Rad(std::f32::consts::FRAC_PI_4));
            self.orientation = (yaw_rotation * self.orientation).normalize();
            // Recompute position/up/reference frame from orientation
            self.update_position();

            // Rebuild reference frame from restored pose
            let dir = (self.target - self.position).normalize();
            let forward = -dir;
            let right = forward.cross(self.world_up).normalize();
            let up = right.cross(forward).normalize();
            self.reference_frame = Matrix3::from_cols(right, up, forward);
            self.last_right = right;
            self.up = up;

            // Keep OM transform in sync
            self.update_om_xform();
            log::info!(
                "Cycled to Parallel-Ortho (free orbit) using saved perspective pose. ortho_half_height={:.3}",
                self.ortho_half_height
            );
            return;
        }

        // State C: Parallel-Ortho -> go back to Perspective (restore saved perspective state)
        if self.is_ortho {
            self.toggle_view_mode();
            return;
        }
    }
    
    // Set camera to top view (looking down along -Z axis)
    fn set_top_view(&mut self) {
        // True top-down: look straight down -Z with +Y as up.
        // Center on world origin: target = (0,0,0), position = (0,0,distance)
        // Save current perspective state so we can restore when exiting TopView
        if self.view_mode != CameraViewMode::TopView {
            self.saved_perspective_position = self.position;
            self.saved_perspective_target = self.target;
            self.saved_perspective_orientation = self.orientation;
            self.saved_perspective_distance = self.distance;
            self.saved_perspective_is_ortho = self.is_ortho;
            self.saved_perspective_ortho_half_height = self.ortho_half_height;
        }
        self.is_ortho = true;
        // Ensure a reasonable ortho scale
        if self.ortho_half_height < 1.0 {
            self.ortho_half_height = 6.0;
        } else {
            self.ortho_half_height = self.ortho_half_height.max(6.0);
        }

        // Set fixed top-view orientation and reference frame
        self.up = Vector3::unit_y();
        // Center at origin (x,y = 0) and keep distance along +Z so view is along -Z
        self.target = Point3::new(0.0, 0.0, 0.0);
        self.position = Point3::new(0.0, 0.0, self.distance);
        // Use a +90° rotation about +X so that rot(+Y) = +Z, hence forward = -rot(+Y) = -Z
        self.orientation = Quaternion::from_axis_angle(Vector3::unit_x(), Rad(std::f32::consts::FRAC_PI_2));
        let forward = -Vector3::unit_z();
        let right = Vector3::unit_x();
        let up = Vector3::unit_y();
        self.reference_frame = Matrix3::from_cols(right, up, forward);
        self.last_right = right;

        // Mark view mode and sync OM transform
        self.view_mode = CameraViewMode::TopView;
        self.update_om_xform();

        log::info!(
            "Top view set (ortho): pos=({:.3},{:.3},{:.3}) target=({:.3},{:.3},{:.3}) ortho_half_height={:.3}",
            self.position.x, self.position.y, self.position.z,
            self.target.x, self.target.y, self.target.z,
            self.ortho_half_height
        );
    }

    // Set orthographic camera with scene bounds for proper framing
    pub fn set_top_view_with_bounds(&mut self, scene_center: Point3<f32>, scene_size: Vector3<f32>) {
        // Position camera above the scene center
        let view_height = (scene_size.z * 0.5 + scene_size.x.max(scene_size.y) * 0.5).max(10.0);
        self.position = Point3::new(scene_center.x, scene_center.y, scene_center.z + view_height);
        self.target = scene_center;
        
        // Look down along -Z axis
        self.up = Vector3::unit_y(); // Y is up in top view
        
        // Set orthographic projection for top view
        self.is_ortho = true;
        // Set ortho bounds to fit the scene with some padding
        let padding_factor = 1.2; // 20% padding around scene
        let max_scene_extent = scene_size.x.max(scene_size.y) * 0.5 * padding_factor;
        self.ortho_half_height = max_scene_extent.max(1.0);
        
        // Set orientation to look straight down
        self.orientation = Quaternion::look_at(-Vector3::unit_z(), Vector3::unit_y());
        
        // Update reference frame for top view
        let forward = -Vector3::unit_z();
        let right = Vector3::unit_x();
        let up = Vector3::unit_y();
        self.reference_frame = Matrix3::from_cols(right, up, forward);
        self.last_right = right;
        
        self.update_om_xform();
        
        log::info!(
            "Orthographic top view set: center=({:.3},{:.3},{:.3}) size=({:.3},{:.3},{:.3}) ortho_half_height={:.3}",
            scene_center.x, scene_center.y, scene_center.z,
            scene_size.x, scene_size.y, scene_size.z,
            self.ortho_half_height
        );
    }

    // Legacy compatibility - map position to eye
    pub fn eye(&self) -> Point3<f32> {
        self.position
    }
}

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    // x: viewport width, y: viewport height, z: fovy (degrees), w: aspect
    pub viewport_fovy_aspect_pipe_px_radius: [f32; 4],
    // x: pipe pixel radius, yzw: reserved
    pub pipe_params: [f32; 4],
    // Camera eye position (world space), w unused
    pub eye_pos: [f32; 4],
    // Camera forward direction (world space), w unused
    pub view_dir: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
            viewport_fovy_aspect_pipe_px_radius: [0.0, 0.0, 45.0, 1.0],
            pipe_params: [2.0, 0.0, 0.0, 0.0],
            eye_pos: [0.0, 0.0, 0.0, 0.0],
            view_dir: [0.0, 0.0, -1.0, 0.0],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }

    pub fn set_view_params(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        fovy_degrees: f32,
        aspect: f32,
        pipe_px_radius: f32,
        is_ortho: bool,
        ortho_half_height: f32,
    ) {
        self.viewport_fovy_aspect_pipe_px_radius = [
            viewport_width,
            viewport_height,
            fovy_degrees,
            aspect,
        ];
        self.pipe_params = [
            pipe_px_radius,
            ortho_half_height,
            if is_ortho { 1.0 } else { 0.0 },
            0.0,
        ];
    }

    pub fn set_eye(&mut self, camera: &Camera) {
        self.eye_pos = [camera.position.x, camera.position.y, camera.position.z, 0.0];
    }

    pub fn set_eye_dir(&mut self, camera: &Camera) {
        self.eye_pos = [camera.position.x, camera.position.y, camera.position.z, 0.0];
        let f = camera.forward_dir();
        self.view_dir = [f.x, f.y, f.z, 0.0];
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,

    // Mouse state for different interaction modes
    is_orbiting: bool,      // Right mouse button for orbit
    is_panning: bool,       // Middle mouse button for pan

    // Mouse delta tracking
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    mouse_pan_x: f32,
    mouse_pan_y: f32,

    // Camera control settings
    orbit_speed: f32,
    zoom_speed: f32,
    ortho_zoom_speed: f32,
    orbit_invert_y: bool,
    max_rotation_per_frame: f32,

    // Camera view toggle functionality
    toggle_view_pressed: bool,
    // Top view trigger
    top_view_pressed: bool,
    // Reset functionality
    reset_camera_pressed: bool,
    // New: single-key cycle (Perspective -> TopView -> Parallel-Ortho)
    cycle_view_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            is_orbiting: false,
            is_panning: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            mouse_pan_x: 0.0,
            mouse_pan_y: 0.0,
            orbit_speed: 1.5,    // Increased orbit speed for responsive control
            zoom_speed: 0.05,    // Reduced for softer zoom (perspective)
            ortho_zoom_speed: 0.15, // Exponential zoom factor for orthographic
            orbit_invert_y: false, // Standard behavior in most 3D software
            max_rotation_per_frame: 0.1, // Limit to about 5.7 degrees per frame
            toggle_view_pressed: false,
            top_view_pressed: false,
            reset_camera_pressed: false,
            cycle_view_pressed: false,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::KeyE => {
                self.amount_up = amount;
                true
            }
            KeyCode::KeyQ | KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            KeyCode::KeyC => {
                if state == ElementState::Pressed {
                    log::info!("KeyC is deprecated. Use 'P' to cycle: Perspective -> TopView -> Parallel");
                }
                true
            }
            KeyCode::KeyT => {
                if state == ElementState::Pressed {
                    log::info!("KeyT is deprecated. Use 'P' to cycle: Perspective -> TopView -> Parallel");
                }
                true
            }
            KeyCode::KeyP => {
                if state == ElementState::Pressed {
                    log::info!("KeyP pressed: scheduling view cycle (Perspective -> TopView -> Parallel)");
                    self.cycle_view_pressed = true;
                }
                true
            }
            KeyCode::KeyF => {
                if state == ElementState::Pressed {
                    self.reset_camera_pressed = true;
                }
                true
            }
            _ => false,
        }
    }

    // Process mouse movement for orbit and panning based on which mouse button is pressed
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.is_orbiting {
            self.mouse_delta_x = mouse_dx as f32;
            self.mouse_delta_y = mouse_dy as f32;
        } else if self.is_panning {
            self.mouse_pan_x = mouse_dx as f32;
            self.mouse_pan_y = mouse_dy as f32;
        }
    }

    // Process mouse button presses
    pub fn process_mouse_button(&mut self, state: ElementState, button: MouseButton) -> bool {
        match button {
            MouseButton::Right => {
                self.is_orbiting = state == ElementState::Pressed;
                if !self.is_orbiting {
                    // Reset mouse deltas when releasing
                    self.mouse_delta_x = 0.0;
                    self.mouse_delta_y = 0.0;
                }
                true
            }
            MouseButton::Middle => {
                self.is_panning = state == ElementState::Pressed;
                if !self.is_panning {
                    // Reset pan deltas when releasing
                    self.mouse_pan_x = 0.0;
                    self.mouse_pan_y = 0.0;
                }
                true
            }
            _ => false,
        }
    }

    // Process scroll wheel for zoom
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => *y as f32 * 0.01,
        };
    }
    // Update the professional orbit camera - Z-up turntable style (Blender/Maya)
    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        self.update_camera_with_bounds(camera, dt, None);
    }

    // Update camera with optional scene bounds for orthographic framing
    pub fn update_camera_with_bounds(&mut self, camera: &mut Camera, dt: Duration, _scene_bounds: Option<(cgmath::Point3<f32>, cgmath::Vector3<f32>)>) {
        let dt = dt.as_secs_f32();

        // Handle keyboard panning (WASD/arrow keys)
        let key_pan_right = (self.amount_right - self.amount_left) * self.speed * dt;
        let key_pan_up = (self.amount_up - self.amount_down) * self.speed * dt;
        if key_pan_right != 0.0 || key_pan_up != 0.0 {
            camera.pan(key_pan_right, key_pan_up);
        }

        // Handle keyboard dolly (W/S)
        let key_dolly = (self.amount_forward - self.amount_backward) * self.speed * dt;
        if key_dolly != 0.0 {
            camera.dolly(key_dolly);
        }

        // Handle mouse panning (middle button drag)
        if self.is_panning && (self.mouse_pan_x != 0.0 || self.mouse_pan_y != 0.0) {
            // Apply pan with a sensitivity factor
            let mouse_pan_speed = self.speed * self.sensitivity * 0.1;

            // In Z-up world, panning should move in view-aligned XY plane
            let mouse_pan_right = -self.mouse_pan_x * mouse_pan_speed;
            let mouse_pan_up = self.mouse_pan_y * mouse_pan_speed;

            camera.pan(mouse_pan_right, mouse_pan_up);
        }

        // Handle orbit rotation (right button drag) - Z-up turntable style
        if self.is_orbiting
            && (self.mouse_delta_x != 0.0 || self.mouse_delta_y != 0.0)
            && camera.view_mode != CameraViewMode::TopView
        {
            // In Z-up turntable mode (like Blender/Maya):
            // X mouse movement -> rotate around Z world axis (yaw)
            // Y mouse movement -> rotate around horizontal axis (pitch)

            // Apply orbit with configured sensitivity
            let orbit_multiplier = self.orbit_speed * self.sensitivity * dt;

            // Calculate raw delta values with clamping
            let yaw_delta = (self.mouse_delta_x * orbit_multiplier)
                .clamp(-self.max_rotation_per_frame, self.max_rotation_per_frame);

            // Calculate pitch delta with inversion if configured
            let pitch_delta = if self.orbit_invert_y {
                self.mouse_delta_y * orbit_multiplier
            } else {
                -self.mouse_delta_y * orbit_multiplier
            };

            // Clamp pitch delta as well
            let pitch_delta = pitch_delta
                .clamp(-self.max_rotation_per_frame, self.max_rotation_per_frame);

            // In a quaternion orbit system with reference frame tracking:
            // 1. Yaw rotates around world up (Z) - unchanged
            // 2. Pitch rotates around reference frame's tracked right vector

            // First, create quaternions for the rotations
            let yaw_rotation = Quaternion::from_axis_angle(camera.world_up, Rad(yaw_delta));

            // Instead of computing the right vector from orientation,
            // use the tracked reference right vector for stable pitch rotation
            let right = camera.last_right;

            // Create pitch rotation around tracked right vector
            let pitch_rotation = Quaternion::from_axis_angle(right.normalize(), Rad(pitch_delta));

            // Apply rotations to camera orientation (pitch then yaw)
            // Order matters: yaw * (pitch * orientation) gives proper turntable feel
            camera.orientation = yaw_rotation * pitch_rotation * camera.orientation;

            // Keep quaternion normalized to prevent drift
            camera.orientation = camera.orientation.normalize();

            // Update camera position after rotation
            camera.update_position();
        }

        // Handle zooming with scroll wheel (standard in all 3D software)
        if self.scroll != 0.0 {
            if camera.is_ortho {
                // Orthographic: use exponential zoom for a consistent feel
                // factor = 2^(scroll * ortho_zoom_speed)
                let factor = (2.0_f32).powf(self.scroll * self.ortho_zoom_speed);
                camera.ortho_half_height = (camera.ortho_half_height * factor)
                    .max(1e-4)
                    .min(1.0e6);
                // No change to camera.position for ortho zoom; keep it stable
            } else {
                // Perspective mode: adjust camera distance
                camera.distance *= 1.0 + self.scroll * self.zoom_speed;
                camera.distance = camera.distance.max(MIN_ZOOM_DISTANCE).min(MAX_ZOOM_DISTANCE);
                camera.update_position();
            }
            // Reset scroll accumulator
            self.scroll = 0.0;
        }

        // Handle top view trigger (t key)
        if self.top_view_pressed {
            camera.set_top_view();
            self.top_view_pressed = false;
        }

        // Handle 'P' triple-cycle: Perspective -> TopView -> Parallel (orthographic, free-orbit)
        if self.cycle_view_pressed {
            camera.cycle_view_triple();
            self.cycle_view_pressed = false;
        }

        // Handle camera view toggle (c key)
        if self.toggle_view_pressed {
            camera.toggle_view_mode();
            self.toggle_view_pressed = false;
        }

        // Handle camera reset (f key)
        if self.reset_camera_pressed {
            camera.reset_to_initial();
            self.reset_camera_pressed = false;
        }

        // Reset mouse deltas after processing
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        self.mouse_pan_x = 0.0;
        self.mouse_pan_y = 0.0;
    }
}
