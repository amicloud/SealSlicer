// src/stl_processor.rs

#![allow(dead_code)]

use std::f32::consts::PI;
use std::fs::File;
use std::io::BufReader;
use stl_io::{self, Triangle};
use nalgebra::{Matrix3, Vector3};
use rayon::prelude::*;
use log::debug;
use geo::{Polygon, LineString, Coord};
use geo::algorithm::area::Area;
use std::collections::{HashMap, HashSet};
pub struct StlProcessor;


#[derive(Clone)]
pub struct BoundingBox {
    pub min: Vector3<f64>,
    pub max: Vector3<f64>,
}

impl StlProcessor {
    // Read the STL file and return the list of triangles
    pub fn read_stl(filename: &str) -> Result<Vec<Triangle>, std::io::Error> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let indexed_mesh = stl_io::read_stl(&mut reader)?;

        // Convert IndexedMesh into Vec<Triangle>
        let triangles = indexed_mesh.faces.iter().map(|face| {
            let vertices = [
                indexed_mesh.vertices[face.vertices[0] as usize],
                indexed_mesh.vertices[face.vertices[1] as usize],
                indexed_mesh.vertices[face.vertices[2] as usize],
            ];
            Triangle {
                normal: face.normal,
                vertices,
            }
        }).collect();

        Ok(triangles)
    }

    // Rotate triangles using a rotation matrix
    pub fn rotate_triangles(triangles: &[Triangle], rotation_matrix: &Matrix3<f64>) -> Vec<Triangle> {
        triangles
            .par_iter()
            .map(|tri| StlProcessor::rotate_triangle(tri, rotation_matrix))
            .collect()
    }

    // Function to rotate a triangle
    fn rotate_triangle(triangle: &Triangle, rotation_matrix: &Matrix3<f64>) -> Triangle {
        let vertices = [
            StlProcessor::rotate_point(&triangle.vertices[0], rotation_matrix),
            StlProcessor::rotate_point(&triangle.vertices[1], rotation_matrix),
            StlProcessor::rotate_point(&triangle.vertices[2], rotation_matrix),
        ];

        Triangle {
            normal: triangle.normal, // Retain original normal or recalculate if necessary
            vertices,
        }
    }

    // Function to rotate a point using a rotation matrix
    fn rotate_point(vertex: &[f32; 3], rotation_matrix: &Matrix3<f64>) -> [f32; 3] {
        let point = Vector3::new(vertex[0] as f64, vertex[1] as f64, vertex[2] as f64);
        let rotated_point = rotation_matrix * point;
        [rotated_point[0] as f32, rotated_point[1] as f32, rotated_point[2] as f32]
    }

    // Rotation matrix for X-axis rotation
    pub fn rotation_matrix_x(theta: f64) -> Matrix3<f64> {
        let rad = theta.to_radians();
        let cos_rad = rad.cos();
        let sin_rad = rad.sin();
        Matrix3::new(
            1.0, 0.0,      0.0,
            0.0, cos_rad, -sin_rad,
            0.0, sin_rad,  cos_rad,
        )
    }

    // Rotation matrix for Y-axis rotation
    pub fn rotation_matrix_y(theta: f64) -> Matrix3<f64> {
        let rad = theta.to_radians();
        let cos_rad = rad.cos();
        let sin_rad = rad.sin();
        Matrix3::new(
            cos_rad,  0.0, sin_rad,
            0.0,      1.0, 0.0,
        -sin_rad, 0.0, cos_rad,
        )
    }

    // Rotation matrix for Z-axis rotation
    pub fn rotation_matrix_z(theta: f64) -> Matrix3<f64> {
        let rad = theta.to_radians();
        let cos_rad = rad.cos();
        let sin_rad = rad.sin();
        Matrix3::new(
            cos_rad, -sin_rad, 0.0,
            sin_rad,  cos_rad, 0.0,
            0.0,      0.0,     1.0,
        )
    }

    // Translate the model so that its centroid is at the origin
    pub fn translate_to_origin(triangles: &[Triangle]) -> Vec<Triangle> {
        let all_points: Vec<Vector3<f64>> = triangles
            .iter()
            .flat_map(|tri| tri.vertices.iter().map(|v| Vector3::new(v[0] as f64, v[1] as f64, v[2] as f64)))
            .collect();

        let num_points = all_points.len() as f64;
        let centroid = all_points.iter().fold(Vector3::zeros(), |acc, p| acc + p) / num_points;

        triangles
            .iter()
            .map(|tri| {
                let translated_vertices = tri.vertices.map(|v| {
                    let point = Vector3::new(v[0] as f64, v[1] as f64, v[2] as f64) - centroid;
                    [point[0] as f32, point[1] as f32, point[2] as f32]
                });
                Triangle {
                    normal: tri.normal,
                    vertices: translated_vertices,
                }
            })
            .collect()
    }

    // Determine the Z-axis range of the model
    fn z_range(triangles: &[Triangle]) -> (f64, f64) {
        let z_coords: Vec<f64> = triangles
            .iter()
            .flat_map(|tri| tri.vertices.iter().map(|v| v[2] as f64))
            .collect();

        let min_z = z_coords.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_z = z_coords.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        (min_z, max_z)
    }

    // Compute the intersection of a triangle with a horizontal plane at z = plane_z
    fn intersect_triangle_with_plane(triangle: &Triangle, plane_z: f64) -> Vec<Vector3<f64>> {
        let epsilon = 1e-6; // Tolerance for floating-point comparisons

        let points: Vec<Vector3<f64>> = triangle
            .vertices
            .iter()
            .map(|v| Vector3::new(v[0] as f64, v[1] as f64, v[2] as f64))
            .collect();

        let distances: Vec<f64> = points.iter().map(|p| p[2] - plane_z).collect();

        // Check if all points are on one side of the plane
        let mut positive = false;
        let mut negative = false;

        for &distance in &distances {
            if distance > epsilon {
                positive = true;
            } else if distance < -epsilon {
                negative = true;
            }
        }

        // No intersection if all points are on one side
        if !(positive && negative) {
            return vec![];
        }

        // Find intersection points
        let mut intersections = Vec::new();

        for i in 0..3 {
            let p1 = points[i];
            let p2 = points[(i + 1) % 3];
            let d1 = distances[i];
            let d2 = distances[(i + 1) % 3];

            if (d1 > epsilon && d2 < -epsilon) || (d1 < -epsilon && d2 > epsilon) {
                let t = d1 / (d1 - d2);
                let intersection = p1 + (p2 - p1) * t;
                intersections.push(intersection);
            } else if d1.abs() <= epsilon && d2.abs() <= epsilon {
                // Both points lie on the plane
                intersections.push(p1);
                intersections.push(p2);
            } else if d1.abs() <= epsilon {
                // p1 lies on the plane
                intersections.push(p1);
            } else if d2.abs() <= epsilon {
                // p2 lies on the plane
                intersections.push(p2);
            }
        }

        // Remove duplicate points
        intersections.sort_by(|a, b| {
            a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal)
                .then(a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
                .then(a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal))
        });
        intersections.dedup_by(|a, b| a.metric_distance(b) < epsilon);

        intersections
    }

    // Collect all intersection segments at a given plane_z
    fn collect_intersection_segments(triangles: &[Triangle], plane_z: f64) -> Vec<(Vector3<f64>, Vector3<f64>)> {
        let mut segments = Vec::new();

        for triangle in triangles {
            let intersection_points = StlProcessor::intersect_triangle_with_plane(triangle, plane_z);

            if intersection_points.len() == 2 {
                segments.push((intersection_points[0], intersection_points[1]));
            } else if intersection_points.len() > 2 {
                debug!("Skipped a triangle intersecting the plane in multiple points at z={}", plane_z);
            }
        }

        segments
    }

    /// Assembles segments into closed polygons.
    fn assemble_polygons(segments: &[(Vector3<f64>, Vector3<f64>)]) -> Vec<Vec<Vector3<f64>>> {
        fn point_to_key(p: &Vector3<f64>, epsilon: f64) -> (i64, i64) {
            let scale = 1.0 / epsilon;
            let x = (p[0] * scale).round() as i64;
            let y = (p[1] * scale).round() as i64;
            (x, y)
        }

        let epsilon = 1e-6;
        let mut point_coords: HashMap<(i64, i64), Vector3<f64>> = HashMap::new();
        let mut adjacency: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();

        // Build adjacency map
        for &(ref start, ref end) in segments {
            let start_key = point_to_key(start, epsilon);
            let end_key = point_to_key(end, epsilon);

            point_coords.entry(start_key).or_insert_with(|| start.clone());
            point_coords.entry(end_key).or_insert_with(|| end.clone());

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
                            if neighbor_key != polygon_keys[polygon_keys.len() - 2]
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
                if polygon_keys.len() >= 3 && current_key == start_key {
                    let polygon = polygon_keys
                        .into_iter()
                        .map(|key| point_coords[&key].clone())
                        .collect();
                    polygons.push(polygon);
                }
            }
        }

        polygons
    }


    // Calculate the area of a polygon using the Shoelace formula
    fn polygon_area(polygon: &[Vector3<f64>]) -> f64 {
        let coords: Vec<Coord<f64>> = polygon
            .iter()
            .map(|p| Coord { x: p[0], y: p[1] })
            .collect();

        let linestring = LineString::from(coords);
        let polygon = Polygon::new(linestring, vec![]);

        let area = polygon.unsigned_area();
        debug!("Polygon area: {} ", area);
        area
    }

    // Compute the intersection area of a single slice
    fn compute_slice_area(triangles: &[Triangle], plane_z: f64) -> Vec<f64> {
        let segments = StlProcessor::collect_intersection_segments(triangles, plane_z);
        debug!("Intersection segments: {}", segments.iter().len());

        if segments.is_empty() {
            // No intersections at this plane_z
            return vec![];
        }

        let polygons = StlProcessor::assemble_polygons(&segments);

        let slice_areas: Vec<f64> = polygons
            .iter()
            .map(|polygon| StlProcessor::polygon_area(polygon))
            .collect();

        slice_areas
    }

    // Compute cross-sectional areas across all slices using parallel processing
    pub fn compute_cross_sections(
        triangles: &[Triangle],
        _vertical_axis: usize,
        slice_increment: f64,
    ) -> (f64, f64, f64) {
        let (min_z, max_z) = StlProcessor::z_range(triangles);

        let mut slice_z_values = Vec::new();
        let mut z = min_z;
        while z <= max_z {
            slice_z_values.push(z);
            z += slice_increment;
        }

        let areas: Vec<f64> = slice_z_values
            .par_iter()
            .flat_map(|&plane_z| {
                StlProcessor::compute_slice_area(triangles, plane_z)
            })
            .filter(|&area| area > 0.0)
            .collect();
        let min_area = areas.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_area = areas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let average_area = if areas.is_empty() {
            0.0
        } else {
            areas.iter().sum::<f64>() / areas.len() as f64
        };

        (min_area, max_area, average_area)
    }

    pub fn combined_analysis(triangles:&[Triangle]) -> ((f64, i32, f64), f64, BoundingBox) {
        let threshold_angle = 46.0;
        let z_axis = Vector3::new(0.0, 0.0, 1.0);
        let mut total_supported_area = 0.0;
        let mut total_supported_faces:i32 = 0;
        let mut support_volume_estimated:f64 = 0.0;
        let mut total_volume = 0.0;
        let mut min = Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        
        for triangle in triangles {
            // Calculate normal vector
            let v0 = Vector3::new(triangle.vertices[0][0] as f64, triangle.vertices[0][1] as f64, triangle.vertices[0][2] as f64);
            let v1 = Vector3::new(triangle.vertices[1][0] as f64, triangle.vertices[1][1] as f64, triangle.vertices[1][2] as f64);
            let v2 = Vector3::new(triangle.vertices[2][0] as f64, triangle.vertices[2][1] as f64, triangle.vertices[2][2] as f64);

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal_vector = edge1.cross(&edge2).normalize();

            let cos_theta = normal_vector.dot(&z_axis);

            // Calculate the overhang angle in degrees
            let overhang_angle = cos_theta.acos().to_degrees();

            // If support is likely to be needed
            if cos_theta < 0.0 && overhang_angle > threshold_angle {
                let height_center = (&triangle.vertices[0][2] + &triangle.vertices[1][2] + &triangle.vertices[2][2])/3.0;
                // Calculate the area of the triangle
                let area: f64 = 0.5 * edge1.cross(&edge2).norm();
                total_supported_faces += 1;
                total_supported_area += area;
                
                // this is
                let cylinder_diameter: f64 = 1.0; // millimeters
                let contact_point:f64 = 0.35; // millimeters
                if area > (contact_point * PI as f64) * 2.0 {
                    let cylinder_spacing: f64 = 2.5; // millimeters
                    let number_of_cylinders: f64 = area /  ((cylinder_diameter + cylinder_spacing) as f64).powf(2.0);
                    let vol: f64 = number_of_cylinders * ((cylinder_diameter/2.0).powf(2.0) * height_center as f64)*1.25; 
                    support_volume_estimated += vol;
                }
            
                debug!("Face requires support:");
                debug!("  Normal Vector: {:?}", normal_vector);
                debug!("  Overhang Angle: {:.2} degrees", overhang_angle);
                debug!("  Area: {:.4} units²", area);
            }
            
            total_volume += v0.cross(&v1).dot(&v2) / 6.0;

            for vertex in &triangle.vertices {
                min[0] = min[0].min(vertex[0] as f64);
                min[1] = min[1].min(vertex[1] as f64);
                min[2] = min[2].min(vertex[2] as f64);
    
                max[0] = max[0].max(vertex[0] as f64);
                max[1] = max[1].max(vertex[1] as f64);
                max[2] = max[2].max(vertex[2] as f64);
            }
            
        }
        let bounding_box = BoundingBox { min, max };
        ((total_supported_area, total_supported_faces, support_volume_estimated.abs()/100.0), total_volume.abs()/1000.0, bounding_box)
    }
}
