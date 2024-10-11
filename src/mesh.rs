use crate::stl_processor::StlProcessor;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Vector3, Vector4};
use std::collections::{HashMap, HashSet};
use stl_io::Triangle;

#[repr(C)]
#[derive(Default, Clone, Pod, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

unsafe impl Zeroable for Vertex {
    fn zeroed() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 0.0],
        }
    }
}

pub struct Mesh {
    triangles: Vec<Triangle>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<[usize; 3]>,
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Mesh {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            position: Vector3::zeros(),
            rotation: Vector4::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Mesh {
    // Cross product of two [f32; 3] arrays
    fn cross(v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
        [
            v1[1] * v2[2] - v1[2] * v2[1],
            v1[2] * v2[0] - v1[0] * v2[2],
            v1[0] * v2[1] - v1[1] * v2[0],
        ]
    }

    // Dot product of two [f32; 3] arrays
    fn dot(v1: [f32; 3], v2: [f32; 3]) -> f32 {
        v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
    }

    // Normalize a [f32; 3] array
    fn normalize(v: [f32; 3]) -> [f32; 3] {
        let norm = (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt();
        if norm > 1e-6 {
            [v[0] / norm, v[1] / norm, v[2] / norm]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    // Vector subtraction of two [f32; 3] arrays
    fn subtract(v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
        [v1[0] - v2[0], v1[1] - v2[1], v1[2] - v2[2]]
    }

    //TODO: Make this asynchonous or use it asynchonously
    pub fn import_stl(&mut self, filename: &str) {
        let mut imported_triangles =
            StlProcessor::read_stl(filename).expect("Error processing STL file");
        self.triangles.append(&mut imported_triangles);

        // Generate vertices and compute normals
        let vertices = self.generate_vertices();
        let mut vertex_data = vertices.clone();
        let mut indices = self.generate_indices();
        Mesh::compute_vertex_normals(&mut vertex_data, &indices);
        Mesh::ensure_consistent_winding(&vertices, &mut indices);
        Mesh::remove_degenerate_triangles(&mut indices, &vertices);
        // Assign computed mesh data
        self.vertices = vertex_data;
        self.indices = indices;
    }

    // Generate vertices from triangles
    fn generate_vertices(&self) -> Vec<Vertex> {
        self.triangles
            .iter()
            .flat_map(|triangle| {
                triangle.vertices.iter().map(|vertex| Vertex {
                    position: [vertex[0], vertex[1], vertex[2]],
                    normal: [0.0, 0.0, 0.0],
                })
            })
            .collect()
    }

    // Generate indices for each triangle
    fn generate_indices(&self) -> Vec<[usize; 3]> {
        let mut indices = Vec::new();
        for (idx, _) in self.triangles.iter().enumerate() {
            indices.push([idx * 3, idx * 3 + 1, idx * 3 + 2]);
        }
        indices
    }

    // Compute vertex normals from STL faces
    fn compute_vertex_normals(vertices: &mut Vec<Vertex>, indices: &Vec<[usize; 3]>) {
        let mut normal_accumulator: HashMap<usize, [f32; 3]> = HashMap::new();

        for triangle in indices {
            let v0 = vertices[triangle[0]].position;
            let v1 = vertices[triangle[1]].position;
            let v2 = vertices[triangle[2]].position;

            let edge1 = Self::subtract(v1, v0);
            let edge2 = Self::subtract(v2, v0);
            let face_normal = Self::normalize(Self::cross(edge1, edge2));

            for &vertex_index in triangle.iter() {
                normal_accumulator
                    .entry(vertex_index)
                    .and_modify(|n| {
                        n[0] += face_normal[0];
                        n[1] += face_normal[1];
                        n[2] += face_normal[2];
                    })
                    .or_insert(face_normal);
            }
        }

        for (vertex_index, normal) in &normal_accumulator {
            vertices[*vertex_index].normal = Self::normalize(*normal);
        }
    }

    fn is_winding_correct(
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        reference_normal: [f32; 3],
    ) -> bool {
        let edge1 = Self::subtract(v1.position, v0.position);
        let edge2 = Self::subtract(v2.position, v0.position);
        let face_normal = Self::cross(edge1, edge2);

        Self::dot(reference_normal, face_normal) >= 0.0
    }

    /// Function to correct the winding order of a triangle if it's incorrect.
    /// It takes the triangle indices and flips v1 and v2 to correct the order.
    fn correct_winding_order(triangle: &mut [usize; 3]) {
        triangle.swap(1, 2);
    }

    /// Function to ensure all triangles have a consistent winding order.
    /// It propagates a consistent winding across the entire mesh.
    fn ensure_consistent_winding(vertices: &Vec<Vertex>, indices: &mut Vec<[usize; 3]>) {
        // A set to keep track of visited triangles
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: Vec<usize> = Vec::new();

        // Start with the first triangle as the reference
        if let Some(_first_triangle) = indices.get(0) {
            queue.push(0);
            visited.insert(0);
        } else {
            return; // No triangles to process
        }

        while let Some(current_idx) = queue.pop() {
            let current_triangle = indices[current_idx];
            let v0 = &vertices[current_triangle[0]];
            let v1 = &vertices[current_triangle[1]];
            let v2 = &vertices[current_triangle[2]];

            // Calculate the reference normal for this triangle
            let edge1 = Self::subtract(v1.position, v0.position);
            let edge2 = Self::subtract(v2.position, v0.position);
            let reference_normal = Self::normalize(Self::cross(edge1, edge2));

            // Iterate over all other triangles to find adjacent ones
            for (i, triangle) in indices.iter_mut().enumerate() {
                if visited.contains(&i) {
                    continue;
                }

                // Check if this triangle shares an edge with the current triangle
                let shared_vertices: HashSet<usize> = current_triangle.iter().copied().collect();
                let triangle_vertices: HashSet<usize> = triangle.iter().copied().collect();
                let shared_count = shared_vertices.intersection(&triangle_vertices).count();

                if shared_count >= 2 {
                    // If the triangle shares an edge, check the winding
                    let v0 = &vertices[triangle[0]];
                    let v1 = &vertices[triangle[1]];
                    let v2 = &vertices[triangle[2]];

                    if !Self::is_winding_correct(v0, v1, v2, reference_normal) {
                        // Correct the winding if needed
                        Self::correct_winding_order(triangle);
                    }

                    // Mark this triangle as visited and add it to the queue
                    visited.insert(i);
                    queue.push(i);
                }
            }
        }
    }

    fn remove_degenerate_triangles(indices: &mut Vec<[usize; 3]>, vertices: &Vec<Vertex>) {
        indices.retain(|triangle| {
            let v0 = vertices[triangle[0]].position;
            let v1 = vertices[triangle[1]].position;
            let v2 = vertices[triangle[2]].position;

            let edge1 = Self::subtract(v1, v0);
            let edge2 = Self::subtract(v2, v0);

            let cross = Self::cross(edge1, edge2);
            let norm = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();
            norm > 1e-6
        });
    }

    pub fn change_position(&mut self, delta: Vector3<f32>) {
        self.position = delta + self.position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::relative_eq;
    use nalgebra::Vector3;

    const EPSILON: f32 = 1e-4;

    // Helper function to create a triangle given three vertices
    fn create_triangle(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> Triangle {
        Triangle {
            normal: [0.0, 0.0, 1.0], // Placeholder, will be recalculated
            vertices: [v0, v1, v2],
        }
    }

    #[test]
    fn test_default() {
        let mesh = Mesh::default();

        assert!(
            mesh.triangles.is_empty(),
            "Default triangles should be empty"
        );
        assert!(mesh.vertices.is_empty(), "Default vertices should be empty");
        assert!(mesh.indices.is_empty(), "Default indices should be empty");
        assert_eq!(
            mesh.position,
            Vector3::zeros(),
            "Default position should be zero"
        );
        assert_eq!(
            mesh.rotation,
            Vector4::identity(),
            "Default rotation should be identity"
        );
        assert_eq!(
            mesh.scale,
            Vector3::new(1.0, 1.0, 1.0),
            "Default scale should be (1.0, 1.0, 1.0)"
        );
    }

    #[test]
    fn test_change_position() {
        let mut mesh = Mesh::default();
        let initial_position = mesh.position;

        let delta = Vector3::new(10.0, -5.0, 3.0);
        mesh.change_position(delta);

        let expected_position = initial_position + delta;
        assert!(
            relative_eq!(mesh.position, expected_position, epsilon = EPSILON),
            "Mesh position not updated correctly. Expected {:?}, got {:?}",
            expected_position,
            mesh.position
        );
    }

    #[test]
    fn test_generate_vertices() {
        let mesh = Mesh {
            triangles: vec![
                create_triangle([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                create_triangle([1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
            ],
            ..Default::default()
        };

        let generated_vertices = mesh.generate_vertices();

        let expected_vertices = vec![
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
        ];

        assert_eq!(
            generated_vertices.len(),
            expected_vertices.len(),
            "Generated vertices count mismatch"
        );

        for (generated, expected) in generated_vertices.iter().zip(expected_vertices.iter()) {
            // Convert [f32; 3] to Vector3<f32> for comparison
            let generated_pos = Vector3::from(generated.position);
            let expected_pos = Vector3::from(expected.position);

            assert!(
                relative_eq!(generated_pos, expected_pos, epsilon = EPSILON),
                "Vertex position mismatch. Expected {:?}, got {:?}",
                expected.position,
                generated.position
            );

            // Normals are zero at this point
            let generated_norm = Vector3::from(generated.normal);
            let expected_norm = Vector3::from(expected.normal);

            assert!(
                relative_eq!(generated_norm, expected_norm, epsilon = EPSILON),
                "Vertex normal mismatch. Expected {:?}, got {:?}",
                expected.normal,
                generated.normal
            );
        }
    }

    #[test]
    fn test_generate_indices() {
        let mesh = Mesh {
            triangles: vec![
                create_triangle([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
                create_triangle([1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
            ],
            ..Default::default()
        };

        let generated_indices = mesh.generate_indices();

        let expected_indices = vec![[0, 1, 2], [3, 4, 5]];

        assert_eq!(
            generated_indices, expected_indices,
            "Generated indices do not match expected indices"
        );
    }

    #[test]
    fn test_compute_vertex_normals() {
        let mut vertices = vec![
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
        ];

        let indices = vec![[0, 1, 2], [3, 4, 5]];

        Mesh::compute_vertex_normals(&mut vertices, &indices);

        let expected_normal = [0.0, 0.0, 1.0];

        for vertex in vertices.iter() {
            let generated_norm = Vector3::from(vertex.normal);
            let expected_norm = Vector3::from(expected_normal);

            assert!(
                relative_eq!(generated_norm, expected_norm, epsilon = EPSILON),
                "Vertex normal incorrect. Expected {:?}, got {:?}",
                expected_normal,
                vertex.normal
            );
        }
    }

    #[test]
    fn test_ensure_consistent_winding() {
        let vertices = vec![
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
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
        ];

        let mut indices = vec![[0, 1, 2], [3, 2, 1]]; // Second triangle has consistent winding

        Mesh::ensure_consistent_winding(&vertices, &mut indices);

        // After correction, both triangles should have the same winding
        // Expected indices: [[0,1,2], [3,2,1]]
        let expected_indices = vec![[0, 1, 2], [3, 2, 1]];

        assert_eq!(
            indices, expected_indices,
            "Indices winding not consistent after correction"
        );
    }

    #[test]
    fn test_ensure_consistent_winding_correction() {
        let vertices = vec![
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
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
        ];

        let mut indices = vec![[0, 1, 2], [3, 1, 2]]; // Second triangle has inconsistent winding

        Mesh::ensure_consistent_winding(&vertices, &mut indices);

        // After correction, the second triangle should have consistent winding
        // Expected indices: [[0,1,2], [3,2,1]]
        let expected_indices = vec![[0, 1, 2], [3, 2, 1]];

        assert_eq!(
            indices, expected_indices,
            "Indices winding was not corrected as expected"
        );
    }

    #[test]
    fn test_remove_degenerate_triangles() {
        let vertices = vec![
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            },
        ];

        let mut indices = vec![[0, 1, 2], [3, 4, 5]]; // Second triangle is degenerate

        Mesh::remove_degenerate_triangles(&mut indices, &vertices);

        let expected_indices = vec![[0, 1, 2]];

        assert_eq!(
            indices, expected_indices,
            "Degenerate triangles were not removed correctly"
        );
    }

    #[test]
    fn test_import_stl() {
        assert!(true, "Skipped import_stl test due to external dependencies");
        // TODO: Implement mocking somehow
    }
}
