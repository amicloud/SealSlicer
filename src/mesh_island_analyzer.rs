use nalgebra::Vector3;
use std::collections::HashMap;
use crate::mesh::Mesh;

pub struct MeshIslandAnalyzer;

impl MeshIslandAnalyzer {
    pub fn analyze_islands(mesh: &Mesh) -> Vec<u32> {
        let up_direction = Vector3::new(0.0, 0.0, -1.0); // Negative Z is up
        let build_platform_z = 0.0; // Assuming build platform is at z = 0
        const EPSILON: f32 = 1e-6; // Tolerance for floating-point comparison
        let mut islands = Vec::new();

        // Create a HashMap to track edges from each vertex
        let mut vertex_edges: HashMap<u32, Vec<u32>> = HashMap::new();

        // Populate vertex_edges with connections, excluding self-references
        for i in (0..mesh.indices.len()).step_by(3) {
            let triangle = [mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]];
            for &index in &triangle {
                vertex_edges
                    .entry(index)
                    .or_insert_with(Vec::new)
                    .extend(triangle.iter().filter(|&&i| i != index));
            }
        }

        // Ensure all vertices are included in vertex_edges, even if they have no edges
        for vertex_index in 0..mesh.vertices.len() as u32 {
            vertex_edges.entry(vertex_index).or_insert_with(Vec::new);
        }

        // Analyze each vertex to check if it's an island
        for (vertex_index, edges) in vertex_edges {
            // Exclude vertices on the build platform
            let vertex_z = mesh.vertices[vertex_index as usize].position[2];
            if (vertex_z - build_platform_z).abs() < EPSILON {
                continue; // Not an island
            }

            let mut is_island = true;

            for &edge_index in &edges {
                let connected_vertex = &mesh.vertices[edge_index as usize];
                let current_vertex = &mesh.vertices[vertex_index as usize];

                // Compute the direction vector from connected vertex to current vertex
                let direction = Vector3::new(
                    current_vertex.position[0] - connected_vertex.position[0],
                    current_vertex.position[1] - connected_vertex.position[1],
                    current_vertex.position[2] - connected_vertex.position[2],
                );

                // Avoid zero-length vectors
                if direction.norm() == 0.0 {
                    continue; // Ignore zero-length edges
                }

                // Normalize the direction
                let normalized_direction = direction.normalize();

                // Check if the direction aligns with the up direction
                if normalized_direction.dot(&up_direction) < 0.0 {
                    // If the edge is pointing down
                    is_island = false;
                    break;
                }
            }

            if is_island {
                islands.push(vertex_index); // Add this vertex index to the islands list
            }
        }
        islands
    }
}

#[cfg(test)]
mod tests {
    use crate::mesh::Vertex;

    use super::*;

    /// Helper function to create a mesh from vertices and indices
    fn create_mesh(vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        Mesh {
            vertices,
            indices,
            triangles_for_slicing: Vec::new(),
        }
    }

    #[test]
    fn test_no_islands_flat_mesh() {
        // Test a flat mesh with no islands
        // Square in the XY-plane at z=0 (build platform)
        let v0 = Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let v1 = Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let v2 = Vertex::new([1.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let v3 = Vertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);

        let vertices = vec![v0, v1, v2, v3];
        let indices = vec![
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        let mesh = create_mesh(vertices, indices);
        let islands = MeshIslandAnalyzer::analyze_islands(&mesh);

        // Since all vertices are on the build platform, expect no islands
        assert!(
            islands.is_empty(),
            "Expected no islands, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_plane_above_plate() {
        // Test a flat mesh with no islands
        // Square in the XY-plane at z=0 (build platform)
        let v0 = Vertex::new([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]);
        let v1 = Vertex::new([1.0, 0.0, 1.0], [0.0, 0.0, 1.0]);
        let v2 = Vertex::new([1.0, 1.0, 1.0], [0.0, 0.0, 1.0]);
        let v3 = Vertex::new([0.0, 1.0, 1.0], [0.0, 0.0, 1.0]);

        let vertices = vec![v0, v1, v2, v3];
        let indices = vec![
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        let mesh = create_mesh(vertices, indices);
        let islands = MeshIslandAnalyzer::analyze_islands(&mesh);

        // Since all vertices are above the build plate, all of the vertices should be islands
        assert!(
            islands.len() == 4,
            "Expected no islands, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_isolated_island_vertex() {
        // Test a vertex not connected to any other vertex (isolated)
        let v0 = Vertex::new([0.0, 0.0, 1.0], [0.0, 0.0, -1.0]); // Isolated vertex

        let vertices = vec![v0];
        let indices = vec![]; // No triangles

        let mesh = create_mesh(vertices, indices);
        let islands = MeshIslandAnalyzer::analyze_islands(&mesh);

        // Since the vertex has no edges and is not on the build platform, it should be considered an island
        assert_eq!(
            islands,
            vec![0],
            "Expected vertex 0 to be an island, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_floating_triangle_simple() {
        // Test a mesh with multiple islands
        // Base square at z=0 and a floating triangle above it at z=1
        let base_v0 = Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let base_v1 = Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let base_v2 = Vertex::new([1.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let base_v3 = Vertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);

        let floating_v0 = Vertex::new([0.5, 0.5, 1.0], [0.0, 0.0, -1.0]);
        let floating_v1 = Vertex::new([1.5, 0.5, 1.0], [0.0, 0.0, -1.0]);
        let floating_v2 = Vertex::new([0.5, 1.5, 1.0], [0.0, 0.0, -1.0]);

        let vertices = vec![
            base_v0, base_v1, base_v2, base_v3, floating_v0, floating_v1, floating_v2,
        ];
        let indices = vec![
            0, 1, 2, // Base triangle 1
            0, 2, 3, // Base triangle 2
            4, 5, 6, // Floating triangle (island)
        ];

        let mesh = create_mesh(vertices, indices);
        let islands = MeshIslandAnalyzer::analyze_islands(&mesh);

        // The floating triangle vertices should be identified as islands
        assert_eq!(
            islands.len(),
            3,
            "Expected 3 islands, but found: {:?}",
            islands
        );
        assert!(islands.contains(&4));
        assert!(islands.contains(&5));
        assert!(islands.contains(&6));
    }

    #[test]
    fn test_mesh_with_horizontal_edges() {
        // Test a mesh where edges are horizontal (edges with zero Z difference)
        let v0 = Vertex::new([0.0, 0.0, 0.5], [0.0, 0.0, 1.0]); // z = 0.5
        let v1 = Vertex::new([1.0, 0.0, 0.5], [0.0, 0.0, 1.0]); // z = 0.5
        let v2 = Vertex::new([0.5, 1.0, 0.5], [0.0, 0.0, 1.0]); // z = 0.5

        let vertices = vec![v0, v1, v2];
        let indices = vec![0, 1, 2];

        let mesh = create_mesh(vertices, indices);
        let islands = MeshIslandAnalyzer::analyze_islands(&mesh);

        // Since all edges are horizontal and the vertices are not on the build platform,
        // they should be considered as islands
        assert_eq!(
            islands.len(),
            3,
            "Expected 3 islands, but found: {:?}",
            islands
        );
        assert!(islands.contains(&0));
        assert!(islands.contains(&1));
        assert!(islands.contains(&2));
    }
}
