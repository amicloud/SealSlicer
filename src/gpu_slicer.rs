// gpu_slicer.rs
// Distributed under the GNU Affero General Public License v3.0 or later.
// See accompanying file LICENSE or https://www.gnu.org/licenses/agpl-3.0.html for details.

use glow::HasContext;
use image::{ImageBuffer, Luma};
use imageproc::drawing::draw_polygon_mut;
use imageproc::point::Point;
use nalgebra::Vector3;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::rc::Rc;
use stl_io::Triangle;

use glow::Context as GlowContext;
use std::error::Error;

use crate::body::Body;
pub struct GPUSlicer {
    gl: Rc<GlowContext>,
    x: u32,
    y: u32,
    slice_thickness: f64,
    physical_x: f64,
    physical_y: f64,
}

impl GPUSlicer {
    pub fn new(
        gl: Rc<GlowContext>,
        x: u32,
        y: u32,
        slice_thickness: f64,
        physical_x: f64,
        physical_y: f64,
    ) -> Self {
        println!(
            "OpenGL Major Version: {}. OpenGL Minor Version: {}, GLSL Version:{}",
            glow::MAJOR_VERSION,
            glow::MINOR_VERSION,
            glow::SHADING_LANGUAGE_VERSION
        );
        unsafe {
            let extensions = gl.get_parameter_string(glow::EXTENSIONS);
            // println!("Supported Extensions: {}", extensions);

            if !extensions.contains("GL_EXT_map_buffer_range") {
                panic!("Buffer mapping is not supported on this device.");
            }
        }
        Self {
            gl,
            x,
            y,
            slice_thickness,
            physical_x,
            physical_y,
        }
    }

    pub fn slice_bodies(
        &self,
        _bodies: Vec<Rc<RefCell<Body>>>,
    ) -> Result<Vec<ImageBuffer<Luma<u8>, Vec<u8>>>, Box<dyn std::error::Error>> {
        let triangles: Vec<Triangle> = Vec::new();
        self.generate_slice_images(&triangles)
    }
    // Function to generate slice images
    fn generate_slice_images(
        &self,
        triangles: &[Triangle],
    ) -> Result<Vec<ImageBuffer<Luma<u8>, Vec<u8>>>, Box<dyn Error>> {
        let gl = &self.gl;

        // Read and compile the compute shader
        let shader_source = fs::read_to_string("shaders/slicer_shader.glsl")?;
        let compute_program = self.compile_compute_shader(&shader_source)?;

        // Transfer triangles to GPU
        // We'll need to flatten the triangle data
        let mut vertices = Vec::with_capacity(triangles.len() * 9); // 3 vertices * 3 components
        for triangle in triangles {
            for vertex in &triangle.vertices {
                vertices.push(vertex[0]);
                vertices.push(vertex[1]);
                vertices.push(vertex[2]);
            }
        }

        // Create mesh SSBO (binding point 0)
        let mesh_ssbo = self.create_ssbo(&vertices, 0)?;

        // Generate slice_z_values
        let (min_z, max_z) = self.z_range(triangles);
        let slice_z_values = self.generate_slice_z_values(min_z, max_z, self.slice_thickness);

        // Create slice planes SSBO (binding point 1)
        let slice_z_values_f32: Vec<f32> = slice_z_values.iter().map(|&z| z as f32).collect();
        let slice_ssbo = self.create_ssbo(&slice_z_values_f32, 1)?;

        println!("Number of slice planes: {}", slice_z_values.len());
        // println!("Slice Z-Values: {:?}", slice_z_values);

        // Estimate max segments (rough estimation)
        let estimated_max_segments = triangles.len() * slice_z_values.len(); // Adjust estimation as needed
        const MAX_SEGMENTS: usize = 10_000_000; // Define a reasonable limit
        let max_segments = estimated_max_segments.min(MAX_SEGMENTS);

        // Check buffer size limits
        let size_in_bytes = max_segments * 5 * std::mem::size_of::<f32>();
        if size_in_bytes > i32::MAX as usize {
            return Err("Output buffer size exceeds maximum allowable limit.".into());
        }

        // Create output SSBO (binding point 2)
        let output_ssbo = self.create_output_ssbo(size_in_bytes, 2)?; // Each segment has 5 floats

        // Create atomic counter buffer (binding point 3)
        let atomic_counter_buffer = self.create_atomic_counter_buffer(3)?;

        // Reset atomic counter
        self.reset_atomic_counter(atomic_counter_buffer)?;

        // Dispatch compute shader
        let num_triangles = triangles.len();
        let local_size_x = 256;
        let num_workgroups = (num_triangles + local_size_x - 1) / local_size_x;
        println!("Number of triangles: {}", num_triangles);
        println!("Local size X: {}", local_size_x);
        println!("Number of workgroups: {}", num_workgroups);
        unsafe {
            gl.use_program(Some(compute_program));
            gl.dispatch_compute(num_workgroups as u32, 1, 1);
            gl.memory_barrier(
                glow::SHADER_STORAGE_BARRIER_BIT
                    | glow::ATOMIC_COUNTER_BARRIER_BIT
                    | glow::BUFFER_UPDATE_BARRIER_BIT,
            );

            // Check for OpenGL errors after dispatch
            let error_code = gl.get_error();
            if error_code != glow::NO_ERROR {
                return Err(
                    format!("OpenGL Error after dispatch_compute: 0x{:X}", error_code).into(),
                );
            }
        }

        // Retrieve the number of segments written
        let segment_count = self.get_atomic_counter_value(atomic_counter_buffer)?;
        println!("Segment count: {}", segment_count);
        if segment_count == 0 {
            // No segments were generated
            return Err("Compute shader did not generate any segments.".into());
        }

        // Retrieve the segments from the output SSBO
        let total_floats = segment_count as usize * 5; // Now 5 floats per segment
        if total_floats * std::mem::size_of::<f32>() > i32::MAX as usize {
            return Err("Mapped buffer size exceeds i32::MAX".into());
        }
        let mut segments = vec![0f32; total_floats];
        unsafe {
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(output_ssbo));
            let is_mapped =
                gl.get_buffer_parameter_i32(glow::SHADER_STORAGE_BUFFER, glow::BUFFER_MAPPED) != 0;
            if is_mapped {
                gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
            }
            let ptr = gl.map_buffer_range(
                glow::SHADER_STORAGE_BUFFER,
                0,
                (total_floats * std::mem::size_of::<f32>()) as i32,
                glow::MAP_READ_BIT,
            ) as *const f32;

            // Check for mapping failure
            if ptr.is_null() {
                let error_code = gl.get_error();
                gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
                return Err(format!(
                    "Failed to map output SSBO. OpenGL Error: 0x{:X}",
                    error_code
                )
                .into());
            }

            let data_slice = std::slice::from_raw_parts(ptr, total_floats);
            segments.copy_from_slice(data_slice);

            gl.unmap_buffer(glow::SHADER_STORAGE_BUFFER);
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
        }

        // Compute bounding box for coordinate mapping
        let (min_x, max_x, min_y, max_y) = self.compute_bounding_box(triangles);
        let model_width = max_x - min_x;
        let model_height = max_y - min_y;
        let scale_x = self.x as f64 / model_width;
        let scale_y = self.y as f64 / model_height;
        let scale = scale_x.min(scale_y);
        let x_offset = ((self.x as f64 - model_width * scale) / 2.0) as i32;
        let y_offset = ((self.y as f64 - model_height * scale) / 2.0) as i32;

        // Organize segments per slice plane
        let plane_segments = self.organize_segments(&segments, &slice_z_values);

        // For each slice plane, assemble polygons and generate image
        let images: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = slice_z_values
            .iter()
            .enumerate()
            .map(|(slice_index, &_z)| {
                let default = Vec::new();
                let segments = plane_segments.get(&slice_index).unwrap_or(&default);
                let polygons = self.assemble_polygons(segments);
                let image_width = self.x;
                let image_height = self.y;
                self.generate_slice_image(
                    &polygons,
                    image_width,
                    image_height,
                    min_x,
                    min_y,
                    scale,
                    x_offset,
                    y_offset,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Clean up resources
        unsafe {
            gl.delete_buffer(mesh_ssbo);
            gl.delete_buffer(slice_ssbo);
            gl.delete_buffer(output_ssbo);
            gl.delete_buffer(atomic_counter_buffer);
            gl.delete_program(compute_program);
        }

        Ok(images)
    }
    // Function to load and compile the compute shader
    fn compile_compute_shader(&self, shader_source: &str) -> Result<glow::Program, String> {
        let gl = &self.gl;
        unsafe {
            let shader = gl.create_shader(glow::COMPUTE_SHADER)?;
            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);

            if !gl.get_shader_compile_status(shader) {
                let log = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                return Err(format!("Compute shader compilation failed: {}", log));
            }

            let program = gl.create_program()?;
            gl.attach_shader(program, shader);
            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                gl.delete_shader(shader);
                return Err(format!("Compute shader linking failed: {}", log));
            }

            gl.delete_shader(shader); // Shader is no longer needed after linking
            Ok(program)
        }
    }

    // Function to create and fill SSBOs
    fn create_ssbo<T>(&self, data: &[T], binding_point: u32) -> Result<glow::Buffer, String> {
        let gl = &self.gl;
        unsafe {
            let ssbo = gl.create_buffer()?;
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(ssbo));
            gl.buffer_data_u8_slice(
                glow::SHADER_STORAGE_BUFFER,
                core::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<T>(),
                ),
                glow::STATIC_DRAW,
            );
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, binding_point, Some(ssbo));
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

            Ok(ssbo)
        }
    }

    // Function to create an SSBO for output data
    fn create_output_ssbo(
        &self,
        size_in_bytes: usize,
        binding_point: u32,
    ) -> Result<glow::Buffer, String> {
        let gl = &self.gl;
        unsafe {
            let ssbo = gl.create_buffer()?;
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(ssbo));
            gl.buffer_data_size(
                glow::SHADER_STORAGE_BUFFER,
                size_in_bytes as i32,
                glow::STREAM_READ,
            );
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, binding_point, Some(ssbo));
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);

            Ok(ssbo)
        }
    }

    // Function to create and initialize atomic counter buffer
    fn create_atomic_counter_buffer(&self, binding_point: u32) -> Result<glow::Buffer, String> {
        let gl = &self.gl;
        unsafe {
            let buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, Some(buffer));
            let zero: u32 = 0;
            let zero_bytes = &zero.to_ne_bytes();
            gl.buffer_data_u8_slice(glow::ATOMIC_COUNTER_BUFFER, zero_bytes, glow::DYNAMIC_DRAW);
            gl.bind_buffer_base(glow::ATOMIC_COUNTER_BUFFER, binding_point, Some(buffer));
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, None);
            Ok(buffer)
        }
    }

    // Function to reset atomic counter buffer to zero
    fn reset_atomic_counter(&self, buffer: glow::Buffer) -> Result<(), String> {
        let gl = &self.gl;
        unsafe {
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, Some(buffer));
            let zero: u32 = 0;
            let zero_bytes = &zero.to_ne_bytes();
            gl.buffer_sub_data_u8_slice(glow::ATOMIC_COUNTER_BUFFER, 0, zero_bytes);
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, None);
            Ok(())
        }
    }

    // Function to get the value of the atomic counter buffer
    fn get_atomic_counter_value(&self, buffer: glow::Buffer) -> Result<u32, String> {
        let gl = &self.gl;
        unsafe {
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, Some(buffer));
            let counter_data = gl.map_buffer_range(
                glow::ATOMIC_COUNTER_BUFFER,
                0,
                std::mem::size_of::<u32>() as i32,
                glow::MAP_READ_BIT,
            ) as *const u32;

            if counter_data.is_null() {
                gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, None);
                return Err("Failed to map atomic counter buffer".to_string());
            }

            let counter_value = *counter_data;

            gl.unmap_buffer(glow::ATOMIC_COUNTER_BUFFER);
            gl.bind_buffer(glow::ATOMIC_COUNTER_BUFFER, None);
            Ok(counter_value)
        }
    }

    // Function to generate slice_z_values
    fn generate_slice_z_values(&self, min_z: f64, max_z: f64, slice_increment: f64) -> Vec<f64> {
        let mut slice_z_values = Vec::new();
        let mut z = min_z;
        while z <= max_z {
            slice_z_values.push(z);
            z += slice_increment;
        }
        slice_z_values
    }

    // Function to compute z range
    fn z_range(&self, triangles: &[Triangle]) -> (f64, f64) {
        let z_coords: Vec<f64> = triangles
            .iter()
            .flat_map(|tri| tri.vertices.iter().map(|v| v[2] as f64))
            .collect();

        let min_z = z_coords.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_z = z_coords.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("Model Z-Range: min_z = {}, max_z = {}", min_z, max_z);
        (min_z, max_z)
    }

    // Function to compute bounding box
    fn compute_bounding_box(&self, triangles: &[Triangle]) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for triangle in triangles {
            for vertex in &triangle.vertices {
                min_x = min_x.min(vertex[0] as f64);
                max_x = max_x.max(vertex[0] as f64);
                min_y = min_y.min(vertex[1] as f64);
                max_y = max_y.max(vertex[1] as f64);
            }
        }
        (min_x, max_x, min_y, max_y)
    }

    // Function to map model coordinates to image coordinates
    // it seems like this not being used is a problem lol
    fn model_to_image_coords(
        &self,
        model_point: &Vector3<f64>,
        min_x: f64,
        min_y: f64,
        scale: f64,
        image_height: u32,
        x_offset: i32,
        y_offset: i32,
    ) -> (i32, i32) {
        let x = ((model_point[0] - min_x) * scale) as i32 + x_offset;
        let y = image_height as i32 - ((model_point[1] - min_y) * scale) as i32 + y_offset;
        (x, y)
    }

    // Function to organize segments per slice plane
    fn organize_segments(
        &self,
        segments: &[f32],
        slice_z_values: &[f64],
    ) -> HashMap<usize, Vec<((f32, f32), (f32, f32))>> {
        let mut plane_segments = HashMap::new();
        let segment_count = segments.len() / 5;

        for i in 0..segment_count {
            let idx = i * 5;
            let x1 = segments[idx];
            let y1 = segments[idx + 1];
            let x2 = segments[idx + 2];
            let y2 = segments[idx + 3];
            let slice_index = segments[idx + 4] as usize;
            if slice_index >= slice_z_values.len() {
                continue;
            }
            plane_segments
                .entry(slice_index)
                .or_insert_with(Vec::new)
                .push(((x1, y1), (x2, y2)));
        }

        plane_segments
    }

    // Function to assemble polygons from segments

    fn assemble_polygons(&self, segments: &[((f32, f32), (f32, f32))]) -> Vec<Vec<Vector3<f64>>> {
        fn point_to_key(p: &(f32, f32), epsilon: f32) -> (i64, i64) {
            let scale = 1.0 / epsilon;
            let x = (p.0 * scale).round() as i64;
            let y = (p.1 * scale).round() as i64;
            (x, y)
        }

        let epsilon = 1e-6;
        let mut point_coords: HashMap<(i64, i64), (f32, f32)> = HashMap::new();
        let mut adjacency: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();

        // Build adjacency map
        for &(start, end) in segments {
            let start_key = point_to_key(&start, epsilon);
            let end_key = point_to_key(&end, epsilon);

            point_coords.entry(start_key).or_insert(start);
            point_coords.entry(end_key).or_insert(end);

            adjacency.entry(start_key).or_default().push(end_key);
            adjacency.entry(end_key).or_default().push(start_key);
        }

        let mut polygons = Vec::new();
        let mut visited_edges: HashSet<((i64, i64), (i64, i64))> = HashSet::new();

        // Traverse the graph to assemble polygons
        for &start_key in adjacency.keys() {
            for &next_key in &adjacency[&start_key] {
                let edge = (start_key, next_key);
                if visited_edges.contains(&edge) || visited_edges.contains(&(next_key, start_key)) {
                    continue;
                }

                let mut polygon_keys = vec![start_key];
                let mut current_key = next_key;
                visited_edges.insert(edge);

                loop {
                    polygon_keys.push(current_key);

                    if let Some(neighbors) = adjacency.get(&current_key) {
                        // Find the next neighbor that hasn't been visited
                        let mut found = false;
                        for &neighbor_key in neighbors {
                            let edge = (current_key, neighbor_key);
                            if neighbor_key != *polygon_keys.get(polygon_keys.len() - 2).unwrap()
                                && !visited_edges.contains(&edge)
                                && !visited_edges.contains(&(neighbor_key, current_key))
                            {
                                visited_edges.insert(edge);
                                current_key = neighbor_key;
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            break;
                        }

                        // Check if the polygon is closed
                        if current_key == start_key {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Verify if we have a closed polygon
                if polygon_keys.len() >= 3 {
                    if current_key == start_key {
                        // Remove the last point if it's the same as the first to avoid duplication
                        polygon_keys.pop();
                    }

                    let polygon = polygon_keys
                        .into_iter()
                        .map(|key| {
                            let (x, y) = point_coords[&key];
                            Vector3::new(x as f64, y as f64, 0.0)
                        })
                        .collect();
                    polygons.push(polygon);
                }
            }
        }
        polygons
    }

    // Function to generate slice image from polygons
    fn generate_slice_image(
        &self,
        polygons: &[Vec<Vector3<f64>>],
        image_width: u32,
        image_height: u32,
        min_x: f64,
        min_y: f64,
        scale: f64,
        x_offset: i32,
        y_offset: i32,
    ) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>, Box<dyn Error>> {
        let mut image = ImageBuffer::from_pixel(image_width, image_height, Luma([0u8]));

        for polygon in polygons {
            let mut points: Vec<Point<i32>> = polygon
                .iter()
                .map(|p| {
                    let x = ((p[0] - min_x) * scale) as i32 + x_offset;
                    let y = image_height as i32 - ((p[1] - min_y) * scale) as i32 + y_offset;
                    Point::new(x, y)
                })
                .collect();

            // Check if the first and last points are the same
            while points.len() >= 3 && points.first() == points.last() {
                println!("Removing duplicate point");
                points.pop();
            }

            // Ensure the polygon has at least 3 points
            if points.len() >= 3 {
                draw_polygon_mut(&mut image, &points, Luma([255u8]));
            } else {
                // Optionally log or handle polygons that are too small
                println!("Skipping invalid polygon with less than 3 points.");
            }
        }
        Ok(image)
    }
}
