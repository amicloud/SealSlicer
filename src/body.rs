#![allow(dead_code)]
use nalgebra::{Matrix3, Matrix4, Vector3};

use crate::mesh_data::MeshData;
pub struct Body {
    pub position: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub mesh_data: MeshData,
    pub enabled: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
            mesh_data: MeshData::default(),
            enabled: true,
        }
    }
}

impl Body {
    fn get_model_matrix(&self) -> Matrix4<f32> {
        let mut model = Matrix4::identity();
        model *= Matrix4::new_translation(&self.position);


        model
    }
}
