use bytemuck;
use cgmath::*;
use crate::timing::{Instant, Duration};
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

// Camera constraints
const MIN_ZOOM_DISTANCE: f32 = 0.5;
const MAX_ZOOM_DISTANCE: f32 = 100.0;

// Professional 3D orbit camera implementation
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
        };

        cam.update_position();
        cam
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

        self.update_position();
    }

    // Legacy method for compatibility
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.position, self.target, self.up);
        let proj = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
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
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
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
    rotate_horizontal: f32,
    rotate_vertical: f32,
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
    orbit_invert_y: bool,
    max_rotation_per_frame: f32,

    // Reset functionality
    reset_camera_pressed: bool,
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
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
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
            zoom_speed: 0.05,    // Reduced for softer zoom
            orbit_invert_y: false, // Standard behavior in most 3D software
            max_rotation_per_frame: 0.1, // Limit to about 5.7 degrees per frame
            reset_camera_pressed: false,
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
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            KeyCode::KeyC => {
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
        let dt = dt.as_secs_f32();

        // Handle keyboard panning (WASD/arrow keys)
        let key_pan_right = (self.amount_right - self.amount_left) * self.speed * dt;
        let key_pan_up = (self.amount_up - self.amount_down) * self.speed * dt;
        if key_pan_right != 0.0 || key_pan_up != 0.0 {
            camera.pan(key_pan_right, key_pan_up);
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
        if self.is_orbiting && (self.mouse_delta_x != 0.0 || self.mouse_delta_y != 0.0) {
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
            // Adjust distance with scroll (zoom in/out) with softer effect
            camera.distance *= 1.0 + self.scroll * self.zoom_speed;

            // Ensure camera doesn't get too close or too far
            camera.distance = camera.distance.max(MIN_ZOOM_DISTANCE).min(MAX_ZOOM_DISTANCE);

            // Reset scroll and update position
            self.scroll = 0.0;
            camera.update_position();
        }

        // Handle camera reset (c key)
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
