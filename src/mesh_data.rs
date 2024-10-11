use crate::stl_processor::StlProcessor;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Quaternion, UnitQuaternion, Vector3, Vector4};
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

pub struct MeshData {
    triangles: Vec<Triangle>,
    vertex_normal_array: Vec<f32>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<[usize; 3]>,
    pub position: Vector3<f32>,
    pub rotation: Vector4<f32>,
    pub scale: Vector3<f32>,
}

impl Default for MeshData {
    fn default() -> Self {
        Self {
            triangles: Vec::new(),
            vertex_normal_array: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            position: Vector3::zeros(),
            rotation: Vector4::zeros(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl MeshData {
    fn get_model_matrix(&self) -> Matrix4<f32> {
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

    fn as_vertex_normal_array(&self) -> Vec<f32> {
        let mut vertex_normal_array = Vec::new();
        for vertex in &self.vertices {
            // Push vertex coordinates
            vertex_normal_array.push(vertex.position[0]);
            vertex_normal_array.push(vertex.position[1]);
            vertex_normal_array.push(vertex.position[2]);

            // Push normal coordinates
            vertex_normal_array.push(vertex.normal[0]);
            vertex_normal_array.push(vertex.normal[1]);
            vertex_normal_array.push(vertex.normal[2]);
        }
        vertex_normal_array
    }
    //TODO: Make this asynchonous or use it asynchonously
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
