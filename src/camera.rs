use nalgebra::{Matrix4, Point3, Vector3};

pub struct Camera {
    position: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Point3::new(0.0, 2.0, 2.0),
            target: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0), // Changed to standard up direction
        }
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.position, &self.target, &self.up)
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        Matrix4::new_perspective(aspect_ratio, 75.0_f32.to_radians(), 0.1, 1000.0)
    }

    // Movement methods
    pub fn move_up(&mut self, amount: f32) {
        let direction = self.up.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    pub fn move_down(&mut self, amount: f32) {
        let direction = -self.up.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    pub fn move_left(&mut self, amount: f32) {
        let right = self.right();
        let direction = -right.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    pub fn move_right(&mut self, amount: f32) {
        let right = self.right();
        let direction = right.normalize();
        self.position += direction * amount;
        self.target += direction * amount;
    }

    pub fn move_forward(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        self.position += direction * amount;
    }

    pub fn move_backward(&mut self, amount: f32) {
        let direction = (self.target - self.position).normalize();
        self.position -= direction * amount;
    }

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
