use crate::{body::Body, mesh::Vertex};
use nalgebra::Vector3;
use std::{
    collections::{HashMap, HashSet},
    f32::EPSILON,
};
pub struct MeshIslandAnalyzer;

impl MeshIslandAnalyzer {
    /// Analyzes the mesh and returns a list of unique island vertex references.
    ///
    /// # Arguments
    ///
    /// * `body` - A reference to the body containing the mesh to analyze.
    ///
    /// # Returns
    ///
    /// A vector containing references to vertices identified as unique islands.
    pub fn analyze_islands(body: &Body) -> Vec<&Vertex> {
        let mesh = &body.mesh;
        let up_direction = Vector3::new(0.0, 0.0, -1.0); // Negative Z is up
        let build_platform_z = 0.0; // Assuming build platform is at z = 0

        // Create a HashMap to track unique edges from each vertex
        let mut vertex_edges: HashMap<u32, HashSet<u32>> = HashMap::new();

        // Populate vertex_edges with connections, excluding self-references and ensuring uniqueness
        for i in (0..mesh.indices.len()).step_by(3) {
            let triangle = [mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]];
            for &index in &triangle {
                let entry = vertex_edges.entry(index).or_default();
                for &connected_index in &triangle {
                    if connected_index != index {
                        entry.insert(connected_index);
                    }
                }
            }
        }

        // Ensure all vertices are included in vertex_edges, even if they have no edges
        for vertex_index in 0..mesh.vertices.len() as u32 {
            vertex_edges.entry(vertex_index).or_default();
        }

        // To track unique positions of island vertices
        let mut unique_positions = HashSet::new();
        let mut unique_islands = Vec::new();

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
                // If dot <= 0, the edge is pointing down or horizontal
                if normalized_direction.dot(&up_direction) <= 0.0 {
                    // If the edge is pointing down or horizontal, this vertex is not an island
                    is_island = false;
                    break;
                }
            }

            if is_island {
                let vertex = mesh.vertices.get(vertex_index as usize).unwrap();
                // Create a key based on position with fixed decimal precision
                // This helps in identifying unique positions
                let pos_key = format!(
                    "{:.6},{:.6},{:.6}",
                    vertex.position[0], vertex.position[1], vertex.position[2]
                );
                if !unique_positions.contains(&pos_key) {
                    unique_positions.insert(pos_key);
                    unique_islands.push(vertex);
                }
            }
        }

        unique_islands
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        mesh::{Mesh, Vertex},
        stl_processor::StlProcessor,
    };

    use super::*;

    /// Helper function to create a mesh from vertices and indices
    fn create_mesh(vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        Mesh { vertices, indices }
    }

    #[test]
    fn test_no_islands_flat_mesh_on_build_plate() {
        // Test a flat mesh with no islands
        // Square in the XY-plane at z=0 (build platform)
        let v0 = Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        let v1 = Vertex::new([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        let v2 = Vertex::new([1.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        let v3 = Vertex::new([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]);

        let vertices = vec![v0, v1, v2, v3];
        let indices = vec![
            0, 1, 2, // First triangle
            0, 2, 3, // Second triangle
        ];

        let mesh = create_mesh(vertices, indices);
        let body = Body::new(mesh);
        let islands = MeshIslandAnalyzer::analyze_islands(&body);

        // Since all vertices are on the build platform, expect no islands
        assert!(
            islands.is_empty(),
            "Expected no islands, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_from_stl_flat_overhang() {
        let filename = "test_stls/flat_overhang_4_points.stl";
        let processor = StlProcessor::new();
        let mut mesh = Mesh::default();
        mesh.import_stl(filename, &processor);
        let body = Body::new(mesh);
        let islands = MeshIslandAnalyzer::analyze_islands(&body);

        islands.iter().for_each(|el| println!("{:?}", el.position));
        assert_eq!(
            islands.len(),
            5,
            "Expected 5 islands, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_from_stl_pointed_overhang_1_point() {
        let filename = "test_stls/pointed_overhang_1_point.stl";
        let processor = StlProcessor::new();
        let mut mesh = Mesh::default();
        mesh.import_stl(filename, &processor);
        let body = Body::new(mesh);
        let islands = MeshIslandAnalyzer::analyze_islands(&body);

        islands.iter().for_each(|el| println!("{:?}", el.position));
        // Because of the way the STL gets triangulated i guess 6  is correct
        assert_eq!(
            islands.len(),
            6,
            "Expected 6 islands, but found: {:?}",
            islands
        );
    }

    #[test]
    fn test_from_stl_pointed_overhang_2_points() {
        let filename = "test_stls/pointed_overhang_2_points.stl";
        let processor: StlProcessor = StlProcessor::new();
        let mut mesh = Mesh::default();
        mesh.import_stl(filename, &processor);
        // for ele in &mesh.vertices {
        //     println!("{:?}", ele);
        // }
        let body = Body::new(mesh);
        let islands = MeshIslandAnalyzer::analyze_islands(&body);

        islands.iter().for_each(|el| println!("{:?}", el.position));
        // Because of the way the STL gets triangulated i think 6 is correct
        assert_eq!(
            islands.len(),
            6,
            "Expected 6 islands, but found: {:?}",
            islands
        );
    }
}
