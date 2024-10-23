use nalgebra::Vector3;
use std::collections::HashMap;

use crate::mesh::Mesh;

pub struct MeshIslandAnalyzer;

impl MeshIslandAnalyzer {
    pub fn analyze_islands(mesh: &Mesh) -> Vec<u32> {
        let up_direction = Vector3::new(0.0, 0.0, -1.0); // World up direction (negative Z in this case)
        let mut islands = Vec::new();

        // Create a HashMap to track edges from each vertex
        let mut vertex_edges: HashMap<u32, Vec<u32>> = HashMap::new();

        // Populate vertex_edges with connections
        for i in (0..mesh.indices.len()).step_by(3) {
            let indices = [mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]];
            for &index in &indices {
                vertex_edges.entry(index).or_insert_with(Vec::new).extend_from_slice(&indices);
            }
        }

        // Analyze each vertex to check if it's an island
        for (vertex_index, edges) in vertex_edges {
            let mut is_island = true;

            for &edge_index in &edges {
                if edge_index != vertex_index { // Ignore self-references
                    let direction = Vector3::new(
                        mesh.vertices[edge_index as usize].position[0] - mesh.vertices[vertex_index as usize].position[0],
                        mesh.vertices[edge_index as usize].position[1] - mesh.vertices[vertex_index as usize].position[1],
                        mesh.vertices[edge_index as usize].position[2] - mesh.vertices[vertex_index as usize].position[2],
                    );

                    // Normalize the direction
                    let normalized_direction = direction.normalize();

                    // Check if the direction aligns with the up direction
                    if normalized_direction.dot(&up_direction) <= 0.0 { // If the edge is not pointing up
                        is_island = false;
                        break;
                    }
                }
            }

            if is_island {
                islands.push(vertex_index); // Add this vertex index to the islands list
            }
        }
        islands
    }
}