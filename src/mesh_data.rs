use crate::stl_processor::StlProcessor;
use nalgebra::Vector3;
use std::collections::{HashMap, HashSet};
use stl_io::Triangle;
#[derive(Default, Clone)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
}

pub struct MeshData {
    triangles: Vec<Triangle>,
    vertex_normal_array: Vec<f32>,
    pub vertices: Vec<Vertex>, 
    pub indices: Vec<[usize;3]>,
}

impl Default for MeshData {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
            vertex_normal_array: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl MeshData {
    fn as_vertex_normal_array(&self) -> Vec<f32> {
        let mut vertex_normal_array = Vec::new();
            for vertex in &self.vertices {
                // Push vertex coordinates
                vertex_normal_array.push(vertex.position.x);
                vertex_normal_array.push(vertex.position.y);
                vertex_normal_array.push(vertex.position.z);

                // Push normal coordinates
                vertex_normal_array.push(vertex.normal.x);
                vertex_normal_array.push(vertex.normal.y);
                vertex_normal_array.push(vertex.normal.z);
            } 
        vertex_normal_array
    }

    pub fn import_stl(&mut self, filename: &str) {
        let mut imported_triangles =
            StlProcessor::read_stl(filename).expect("Error processing STL file");
        self.triangles.append(&mut imported_triangles);
        self.vertex_normal_array = self.as_vertex_normal_array();

        // Generate vertices and compute normals
        let vertices = self.generate_vertices();
        let mut vertex_data = vertices.clone();
        let mut indices = self.generate_indices();
        MeshData::compute_vertex_normals(&mut vertex_data, &indices);
        MeshData::ensure_consistent_winding(&vertices, &mut indices);
        MeshData::remove_degenerate_triangles(&mut indices, &vertices);
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
                    position: Vector3::new(vertex[0], vertex[1], vertex[2]),
                    normal: Vector3::zeros(),
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

    // Function to compute vertex normals from STL faces
    fn compute_vertex_normals(vertices: &mut Vec<Vertex>, indices: &Vec<[usize; 3]>) {
        // Create a mapping to store the accumulated normals per vertex
        let mut normal_accumulator: HashMap<usize, Vector3<f32>> = HashMap::new();

        // Iterate over all triangles
        for triangle in indices {
            // Extract vertex positions
            let v0 = vertices[triangle[0]].position;
            let v1 = vertices[triangle[1]].position;
            let v2 = vertices[triangle[2]].position;

            // Calculate the face normal using cross product
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let face_normal = edge1.cross(&edge2).normalize();

            // Accumulate face normal for each vertex in the triangle
            for &vertex_index in triangle.iter() {
                normal_accumulator
                    .entry(vertex_index)
                    .and_modify(|n| *n += face_normal)
                    .or_insert(face_normal);
            }
        }

        // Normalize the accumulated normals and assign them to each vertex
        for (vertex_index, normal) in &normal_accumulator {
            vertices[*vertex_index].normal = normal.normalize();
        }
    }

    /// Function to check if a triangle's winding is correct based on a reference normal.
    /// Returns true if the winding order is correct, false otherwise.
    fn is_winding_correct(
        v0: &Vertex,
        v1: &Vertex,
        v2: &Vertex,
        reference_normal: Vector3<f32>,
    ) -> bool {
        // Calculate the face normal using the cross product
        let edge1 = v1.position - v0.position;
        let edge2 = v2.position - v0.position;
        let face_normal = edge1.cross(&edge2);

        // Check if the face normal and reference normal are pointing in the same direction
        // If the dot product is positive, the winding is correct
        reference_normal.dot(&face_normal) >= 0.0
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
            let edge1 = v1.position - v0.position;
            let edge2 = v2.position - v0.position;
            let reference_normal = edge1.cross(&edge2).normalize();

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
    
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
    
            // Calculate the cross product to find area, if near-zero, it's degenerate
            let cross = edge1.cross(&edge2);
            cross.norm() > 1e-6
        });
    }
}
