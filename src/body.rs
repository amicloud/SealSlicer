#![allow(dead_code)]
use nalgebra::Vector3;

use crate::mesh_data::MeshData;
struct Body {
    position: Vector3<f32>,
    rotation: Vector3<f32>,
    scale: Vector3<f32>,
    mesh_data: MeshData,
    enabled: bool,
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

impl Body {}
