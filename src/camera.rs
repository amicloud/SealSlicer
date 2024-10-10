use nalgebra::{Matrix4, Point3, Vector3};

pub struct Camera {
    position: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
    yaw: f32,        // Rotation around the Y axis
    pitch: f32,      // Rotation around the X axis
    sensitivity: f32, // Mouse sensitivity
    distance: f32,    // Distance from the target for zooming
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            target: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, -1.0, 0.0),
            yaw: -90.0,        // Initialized to look towards 
            pitch: -45.0,        // Initialized to 
            sensitivity: 0.1,  // Adjust as needed for mouse sensitivity
            distance: 200.0,     // Initial distance from the target
        }
    }

    /// Returns the view matrix calculated using LookAt.
    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.position, &self.target, &self.up)
    }

    /// Returns the projection matrix using a perspective projection.
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        Matrix4::new_perspective(aspect_ratio, 75.0_f32.to_radians(), 0.1, 1000.0)
    }

    /// Processes input received from a mouse input system.
    /// Expects the offset value in both the x and y direction.
    pub fn process_mouse_movement(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * self.sensitivity;
        self.pitch += delta_y * self.sensitivity;

        // Constrain the pitch to prevent screen flip
        if self.pitch > 89.9 {
            self.pitch = 89.9;
        }
        if self.pitch < -89.9 {
            self.pitch = -89.9;
        }

        // Update camera position based on new yaw and pitch
        self.update_camera_position();
    }

    /// Updates the camera position based on yaw and pitch angles.
    fn update_camera_position(&mut self) {
        // Convert angles to radians
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        // Calculate the new direction vector
        let direction = Vector3::new(
            yaw_rad.cos() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.sin() * pitch_rad.cos(),
        )
        .normalize();

        // Update position based on the direction and distance
        self.position = (self.target - direction) * self.distance;
    }

    /// Zooms the camera in or out by adjusting the distance from the target.
    pub fn zoom(&mut self, delta: f32) {
        self.distance -= delta;
        if self.distance < 1.0 {
            self.distance = 1.0; // Prevent zooming too close
        }
        if self.distance > 1000.0 {
            self.distance = 1000.0; // Prevent zooming too far
        }
        self.update_camera_position();
    }

    /// Moves the camera up along the up vector.
    pub fn move_up(&mut self, amount: f32) {
        let direction = self.up.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    /// Moves the camera down along the up vector.
    pub fn move_down(&mut self, amount: f32) {
        let direction = -self.up.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    /// Moves the camera to the left relative to the current view.
    pub fn move_left(&mut self, amount: f32) {
        let right = self.right();
        let direction = -right.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    /// Moves the camera to the right relative to the current view.
    pub fn move_right(&mut self, amount: f32) {
        let right = self.right();
        let direction = right.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    /// Calculates the right vector based on the current view.
    fn right(&self) -> Vector3<f32> {
        (self.target - self.position).cross(&self.up).normalize()
    }
}

pub enum CameraMove {
    Up,
    Down,
    Left,
    Right,
    ZoomIn,
    ZoomOut,
}
