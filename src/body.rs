// src/body.rs

use crate::mesh::Mesh;
use crate::stl_processor::StlProcessorTrait;
use nalgebra::{Matrix4, Quaternion, UnitQuaternion, Vector3, Vector4};

pub struct Body {
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
    pub scale: Vector3<f32>,
    pub mesh: Mesh,
    pub enabled: bool,
    pub selected: bool
}

impl Default for Body {
    fn default() -> Self {
        Self {
            position: Vector3::zeros(),
            rotation: Vector4::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            mesh: Mesh::default(),
            enabled: true,
            selected: true
        }
    }
}

impl Body {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Body::default()
    }

    pub fn new_from_stl<P: AsRef<str>, Processor: StlProcessorTrait>(
        filename: P,
        processor: &Processor,
    ) -> Self {
        let mut body = Body::default();
        body.mesh.import_stl(filename, processor);
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

    pub fn translate(&mut self, x:f32, y:f32, z: f32){
        self.position += Vector3::new(x,y,z);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Vertex;
    use crate::stl_processor::StlProcessorTrait;
    use approx::relative_eq;
    use nalgebra::{Matrix4, UnitQuaternion, Vector3, Vector4};
    use stl_io::Triangle;

    const EPSILON: f32 = 1e-4;

    // Mock implementation of StlProcessorTrait without cloning
    struct MockStlProcessor;

    impl StlProcessorTrait for MockStlProcessor {
        fn read_stl(&self, _filename: &str) -> Result<Vec<Triangle>, std::io::Error> {
            Ok(vec![
                create_triangle([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                create_triangle([1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
            ])
        }
    }

    // Helper function to create a triangle
    fn create_triangle(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> Triangle {
        Triangle {
            normal: [0.0, 0.0, 1.0], // Placeholder; Mesh::import_stl will recalculate normals
            vertices: [v0, v1, v2],
        }
    }

    #[test]
    fn test_default() {
        let body = Body::default();

        assert_eq!(
            body.position,
            Vector3::zeros(),
            "Default position should be zero"
        );
        assert_eq!(
            body.rotation,
            Vector4::identity(),
            "Default rotation should be identity"
        );
        assert_eq!(
            body.scale,
            Vector3::new(1.0, 1.0, 1.0),
            "Default scale should be (1.0, 1.0, 1.0)"
        );
        assert!(
            body.mesh.triangles.is_empty(),
            "Default Mesh should have no triangles"
        );
        assert!(
            body.mesh.vertices.is_empty(),
            "Default Mesh should have no vertices"
        );
        assert!(
            body.mesh.indices.is_empty(),
            "Default Mesh should have no indices"
        );
    }

    #[test]
    fn test_new() {
        let body_new = Body::new();
        let body_default = Body::default();

        assert_eq!(
            body_new.position, body_default.position,
            "Body::new() should match Body::default()"
        );
        assert_eq!(
            body_new.rotation, body_default.rotation,
            "Body::new() should match Body::default()"
        );
        assert_eq!(
            body_new.scale, body_default.scale,
            "Body::new() should match Body::default()"
        );
        assert_eq!(
            body_new.mesh.triangles, body_default.mesh.triangles,
            "Body::new() Mesh triangles should match Body::default()"
        );
        assert_eq!(
            body_new.mesh.vertices, body_default.mesh.vertices,
            "Body::new() Mesh vertices should match Body::default()"
        );
        assert_eq!(
            body_new.mesh.indices, body_default.mesh.indices,
            "Body::new() Mesh indices should match Body::default()"
        );
    }

    #[test]
    fn test_new_from_stl() {
        // Arrange: Create a mock processor
        let mock_processor = MockStlProcessor;

        // Act: Create Body from STL using mock processor
        let body = Body::new_from_stl("dummy_filename.stl", &mock_processor);

        // Assert: Mesh should contain the imported triangles
        assert_eq!(
            body.mesh.triangles.len(),
            2,
            "Mesh should contain the same number of triangles as imported"
        );

        // Define expected triangles
        let expected_triangles = vec![
            create_triangle([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            create_triangle([1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ];

        for (imported, expected) in body.mesh.triangles.iter().zip(expected_triangles.iter()) {
            assert_eq!(
                imported.vertices, expected.vertices,
                "Imported triangle vertices do not match expected"
            );
            // Normals are recalculated in Mesh::import_stl, assuming they are correct
        }

        // Additionally, check that vertices and indices are generated correctly
        let expected_vertices = vec![
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
        ];

        let expected_indices = vec![[0, 1, 2], [3, 4, 5]];

        assert_eq!(
            body.mesh.vertices.len(),
            expected_vertices.len(),
            "Mesh should have the correct number of vertices after import"
        );

        for (imported, expected) in body.mesh.vertices.iter().zip(expected_vertices.iter()) {
            let imported_pos = Vector3::from(imported.position);
            let expected_pos = Vector3::from(expected.position);
            assert!(
                relative_eq!(imported_pos, expected_pos, epsilon = EPSILON),
                "Vertex position mismatch. Expected {:?}, got {:?}",
                expected.position,
                imported.position
            );

            let imported_norm = Vector3::from(imported.normal);
            let expected_norm = Vector3::from(expected.normal);
            assert!(
                relative_eq!(imported_norm, expected_norm, epsilon = EPSILON),
                "Vertex normal mismatch. Expected {:?}, got {:?}",
                expected.normal,
                imported.normal
            );
        }

        assert_eq!(
            body.mesh.indices, expected_indices,
            "Mesh indices do not match expected after import"
        );
    }

    #[test]
    fn test_get_model_matrix() {
        // Arrange: Create a Body with known position, rotation, and scale
        let position = Vector3::new(10.0, -5.0, 3.0);
        let rotation = UnitQuaternion::from_euler_angles(0.0, 0.0, std::f32::consts::FRAC_PI_2); // 90 degrees around Z-axis
        let rotation_quat = rotation.quaternion();
        let scale = Vector3::new(2.0, 3.0, 4.0);

        let body = Body {
            position,
            rotation: Vector4::new(
                rotation_quat.i,
                rotation_quat.j,
                rotation_quat.k,
                rotation_quat.w,
            ),
            scale,
            mesh: Mesh::default(),
            selected: true,
            enabled: true
        };

        // Act: Compute the model matrix
        let model_matrix = body.get_model_matrix();

        // Compute expected model matrix manually
        let translation_matrix = Matrix4::new_translation(&position);
        let rotation_matrix = rotation.to_homogeneous();
        let scaling_matrix = Matrix4::new_nonuniform_scaling(&scale);

        let expected_model = translation_matrix * rotation_matrix * scaling_matrix;

        // Assert: The computed model matrix matches the expected matrix
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    relative_eq!(
                        model_matrix[(i, j)],
                        expected_model[(i, j)],
                        epsilon = EPSILON
                    ),
                    "Model matrix element ({}, {}) mismatch. Expected {}, got {}",
                    i,
                    j,
                    expected_model[(i, j)],
                    model_matrix[(i, j)]
                );
            }
        }
    }
}
