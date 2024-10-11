use nalgebra::{Matrix4, Quaternion, UnitQuaternion, Vector3, Vector4};

use crate::mesh::Mesh;
pub struct Body {
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
    pub scale: Vector3<f32>,
    pub mesh: Mesh,
    pub enabled: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: Vector3::zeros(),
            rotation: Vector4::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            mesh: Mesh::default(),
            enabled: true,

        }
    }
}

impl Body {
    pub fn new() -> Self{
        Body::default()
    }

    pub fn new_from_stl(filename:&str) -> Self{
        let mut body = Body::default();
        body.mesh.import_stl(filename);
        body

    }

    pub fn get_model_matrix(&self) -> Matrix4<f32> {
        let mut model = Matrix4::identity();
        model *= Matrix4::new_translation(&self.position);
        let rotation_quat = UnitQuaternion::from_quaternion(Quaternion::new(
            self.rotation.w,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        ));
        model *= rotation_quat.to_homogeneous();
        model *= Matrix4::new_nonuniform_scaling(&self.scale);
        model
    }
}
