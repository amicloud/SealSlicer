// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.
use crate::stl_processor::StlProcessorTrait;
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr, hash::Hasher, hash::Hash,
};
use stl_io::Triangle;

#[repr(C)]
#[derive(Default, Clone, Pod, Copy, PartialEq, Debug)]
pub struct Vertex {
    // Currently this is basically a redo of stl_io::Triangle or geo::Triangle but this will be extended later
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
impl Eq for Vertex {}

impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.position_bits().hash(state);
        self.normal_bits().hash(state);
    }
}

impl Vertex {
    #[allow(dead_code)]
    pub fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Self { position, normal }
    }
    /// Helper method to get bit representation of position
    fn position_bits(&self) -> [u32; 3] {
        self.position.map(|f| f.to_bits())
    }

    /// Helper method to get bit representation of normal
    fn normal_bits(&self) -> [u32; 3] {
        self.normal.map(|f| f.to_bits())
    }
}

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub triangles_for_slicing: Vec<Triangle>,
}

impl Mesh {
    pub fn ready_for_slicing(&mut self) {
        // This is hacky i don't like it i will fix it later
        self.triangles_for_slicing = self.into_triangle_vec();
    }
    fn generate_vertices_and_indices(&mut self, original_triangles: &Vec<Triangle> ) {
        let mut unique_vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map: HashMap<Vertex, u32> = HashMap::new();

        for triangle in original_triangles {
            for &vertex_pos in &triangle.vertices {
                let vertex = Vertex {
                    position: vertex_pos,
                    normal: [0.0, 0.0, 0.0], // Initialize normals; will compute later
                };

                // Insert the vertex into the map if it's not already present
                let index = if let Some(&existing_index) = vertex_map.get(&vertex) {
                    existing_index
                } else {
                    let new_index = unique_vertices.len() as u32;
                    unique_vertices.push(vertex.clone());
                    vertex_map.insert(vertex, new_index);
                    new_index
                };
                indices.push(index);
            }
        }

        self.vertices = unique_vertices;
        self.indices = indices;
    }

    fn into_triangle_vec(&self) -> Vec<Triangle> {
        // Ensure that the number of indices is a multiple of 3
        assert!(
            self.indices.len() % 3 == 0,
            "Number of indices is not a multiple of 3"
        );

        self.indices.chunks(3).map(|triplet| {
            let v0 = &self.vertices[triplet[0] as usize];
            let v1 = &self.vertices[triplet[1] as usize];
            let v2 = &self.vertices[triplet[2] as usize];

            // Sum the vertex normals
            let summed_normal = Vector3::new(
                v0.normal[0] + v1.normal[0] + v2.normal[0],
                v0.normal[1] + v1.normal[1] + v2.normal[1],
                v0.normal[2] + v1.normal[2] + v2.normal[2],
            );

            // Normalize the summed normal
            let normalized_normal = Self::normalize_vector(summed_normal);

            // Construct the Triangle
            Triangle {
                vertices: [v0.position, v1.position, v2.position],
                normal: [
                    normalized_normal.x,
                    normalized_normal.y,
                    normalized_normal.z,
                ],
            }
        }).collect()
    }
    fn normalize_vector(vec: Vector3<f32>) -> Vector3<f32> {
        if vec.norm() != 0.0 {
            vec.normalize()
        } else {
            Vector3::new(0.0, 0.0, 1.0) // Default normal
        }
    }

    // Cross product of two [f32; 3] arrays
    fn cross(v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
        [
            v1[1] * v2[2] - v1[2] * v2[1],
            v1[2] * v2[0] - v1[0] * v2[2],
            v1[0] * v2[1] - v1[1] * v2[0],
        ]
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
    pub fn import_stl<P: AsRef<OsStr>, Processor: StlProcessorTrait>(
        &mut self,
        filename: P,
        processor: &Processor,
    ) {
        let imported_triangles: Vec<Triangle> = processor
            .read_stl(filename.as_ref())
            .expect("Error processing STL file");
        self.generate_vertices_and_indices(&imported_triangles);
        self.compute_vertex_normals();
        self.remove_degenerate_triangles();
        self.ready_for_slicing();
    }

    // Compute vertex normals from STL faces
    pub fn compute_vertex_normals(&mut self) {
        // Reset all vertex normals to zero
        for vertex in &mut self.vertices {
            vertex.normal = [0.0, 0.0, 0.0];
        }
    
        // Iterate over each triangle and accumulate normals
        for triplet in self.indices.chunks(3) {
            let v0 = self.vertices[triplet[0] as usize].position;
            let v1 = self.vertices[triplet[1] as usize].position;
            let v2 = self.vertices[triplet[2] as usize].position;
    
            let edge1 = Mesh::subtract(v1, v0);
            let edge2 = Mesh::subtract(v2, v0);
            let face_normal = Mesh::normalize(Mesh::cross(edge1, edge2));
    
            // Accumulate the face normal to each vertex normal
            self.vertices[triplet[0] as usize].normal = [
                self.vertices[triplet[0] as usize].normal[0] + face_normal[0],
                self.vertices[triplet[0] as usize].normal[1] + face_normal[1],
                self.vertices[triplet[0] as usize].normal[2] + face_normal[2],
            ];
            self.vertices[triplet[1] as usize].normal = [
                self.vertices[triplet[1] as usize].normal[0] + face_normal[0],
                self.vertices[triplet[1] as usize].normal[1] + face_normal[1],
                self.vertices[triplet[1] as usize].normal[2] + face_normal[2],
            ];
            self.vertices[triplet[2] as usize].normal = [
                self.vertices[triplet[2] as usize].normal[0] + face_normal[0],
                self.vertices[triplet[2] as usize].normal[1] + face_normal[1],
                self.vertices[triplet[2] as usize].normal[2] + face_normal[2],
            ];
        }
    
        // Normalize all vertex normals
        for vertex in &mut self.vertices {
            let normalized = Mesh::normalize(vertex.normal);
            vertex.normal = normalized;
        }
    }

    /// Removes degenerate triangles (triangles with zero area).
    pub fn remove_degenerate_triangles(&mut self) {
        let mut valid_indices = Vec::new();

        for triplet in self.indices.chunks(3) {
            let v0 = self.vertices[triplet[0] as usize].position;
            let v1 = self.vertices[triplet[1] as usize].position;
            let v2 = self.vertices[triplet[2] as usize].position;

            let edge1 = Mesh::subtract(v1, v0);
            let edge2 = Mesh::subtract(v2, v0);

            let cross = Mesh::cross(edge1, edge2);
            let norm = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();

            if norm > 1e-6 {
                valid_indices.extend_from_slice(triplet);
            }
        }

        self.indices = valid_indices;
    }

}
#[cfg(test)]
mod tests {
    use super::*;
    use stl_io::Triangle;

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


        assert!(mesh.vertices.is_empty(), "Default vertices should be empty");
        assert!(mesh.indices.is_empty(), "Default indices should be empty");
        assert!(
            mesh.triangles_for_slicing.is_empty(),
            "Default triangles_for_slicing should be empty"
        );
    }

    #[test]
    fn test_single_triangle_normal() {
        // Create a mesh with a single triangle lying on the XY-plane
        let mesh = Mesh {
            vertices: vec![
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
            ],
            indices: vec![0, 1, 2],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(triangles.len(), 1, "There should be exactly one triangle.");

        let expected_normal = [0.0, 0.0, 1.0];
        let calculated_normal = triangles[0].normal;

        for i in 0..3 {
            assert!(
                (calculated_normal[i] - expected_normal[i]).abs() < 1e-5,
                "Normal component {} does not match expected value.",
                i
            );
        }
    }

    #[test]
    fn test_multiple_triangles_normals() {
        // Create a mesh with two triangles forming a square on the XY-plane
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
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
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(triangles.len(), 2, "There should be exactly two triangles.");

        let expected_normal = [0.0, 0.0, 1.0];
        for (i, triangle) in triangles.iter().enumerate() {
            for j in 0..3 {
                assert!(
                    (triangle.normal[j] - expected_normal[j]).abs() < 1e-5,
                    "Triangle {}: Normal component {} does not match expected value.",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn test_degenerate_triangle_normal() {
        // Create a mesh with a degenerate triangle (all vertices have the same position)
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(triangles.len(), 1, "There should be exactly one triangle.");

        // The summed normal is [1,1,1], normalized to [1/sqrt(3), 1/sqrt(3), 1/sqrt(3)]
        let expected_normal = [
            1.0 / (3.0_f32).sqrt(),
            1.0 / (3.0_f32).sqrt(),
            1.0 / (3.0_f32).sqrt(),
        ];
        let calculated_normal = triangles[0].normal;

        for i in 0..3 {
            assert!(
                (calculated_normal[i] - expected_normal[i]).abs() < 1e-5,
                "Normal component {} does not match expected value for degenerate triangle.",
                i
            );
        }
    }

    #[test]
    fn test_zero_normal_triangle() {
        // Create a mesh with a triangle where vertex normals sum to zero
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 0.0],
                },
            ],
            indices: vec![0, 1, 2],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(triangles.len(), 1, "There should be exactly one triangle.");

        // Since the summed normal is [0,0,0], it should default to [0,0,1]
        let expected_normal = [0.0, 0.0, 1.0];
        let calculated_normal = triangles[0].normal;

        for i in 0..3 {
            assert!(
                (calculated_normal[i] - expected_normal[i]).abs() < 1e-5,
                "Normal component {} does not match expected default value for zero normal.",
                i
            );
        }
    }

    #[test]
    fn test_non_uniform_vertex_normals() {
        // Create a mesh with a single triangle with non-uniform vertex normals
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(triangles.len(), 1, "There should be exactly one triangle.");

        // The summed normal is [1,1,1], normalized to [1/sqrt(3), 1/sqrt(3), 1/sqrt(3)]
        let expected_normal = [
            1.0 / (3.0_f32).sqrt(),
            1.0 / (3.0_f32).sqrt(),
            1.0 / (3.0_f32).sqrt(),
        ];
        let calculated_normal = triangles[0].normal;

        for i in 0..3 {
            assert!(
                (calculated_normal[i] - expected_normal[i]).abs() < 1e-5,
                "Normal component {} does not match expected value for non-uniform vertex normals.",
                i
            );
        }
    }

    #[test]
    fn test_large_mesh_normals() {
        // Create a mesh with multiple triangles forming a cube
        let mesh = Mesh {
            vertices: vec![
                // Front face
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0],
                },
                // Back face
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, -1.0],
                },
                // Left face
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [-1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [-1.0, 0.0, 0.0],
                },
                // Right face
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                },
                // Top face
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                },
                // Bottom face
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0],
                },
            ],
            indices: vec![
                // Front face
                0, 1, 2,
                0, 2, 3,
                // Back face
                4, 5, 6,
                4, 6, 7,
                // Left face
                8, 9, 10,
                8, 10, 11,
                // Right face
                12, 13, 14,
                12, 14, 15,
                // Top face
                16, 17, 18,
                16, 18, 19,
                // Bottom face
                20, 21, 22,
                20, 22, 23,
            ],
            triangles_for_slicing: Vec::new(),
        };

        let triangles: Vec<Triangle> = mesh.into_triangle_vec();

        assert_eq!(
            triangles.len(),
            12,
            "There should be exactly twelve triangles for a cube."
        );

        // Define expected normals for each face
        let expected_normals = vec![
            [0.0, 0.0, 1.0], // Front face
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0], // Back face
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0], // Left face
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0], // Right face
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0], // Top face
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0], // Bottom face
            [0.0, -1.0, 0.0],
        ];

        for (i, triangle) in triangles.iter().enumerate() {
            let expected_normal = expected_normals[i];
            let calculated_normal = triangle.normal;

            for j in 0..3 {
                assert!(
                    (calculated_normal[j] - expected_normal[j]).abs() < 1e-5,
                    "Triangle {}: Normal component {} does not match expected value.",
                    i,
                    j
                );
            }
        }
    }
}
