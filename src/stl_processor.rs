// src/stl_processor.rs

use std::fs::File;
use std::io::BufReader;
use stl_io::{self, Triangle};
pub struct StlProcessor;

// Define a trait for processing STL files
pub trait StlProcessorTrait {
    fn read_stl(&self, filename: &str) -> Result<Vec<Triangle>, std::io::Error>;
}

// Implement the trait for the actual `StlProcessor`
impl StlProcessorTrait for StlProcessor {
    fn read_stl(&self, filename: &str) -> Result<Vec<Triangle>, std::io::Error> {
        StlProcessor::read_stl(filename)
    }
}
impl StlProcessor {
    pub fn new() -> Self {
        Self {}
    }
    // Read the STL file and return the list of triangles
    pub fn read_stl(filename: &str) -> Result<Vec<Triangle>, std::io::Error> {
        let file = File::open(filename)?;
        let mut reader = BufReader::new(file);
        let indexed_mesh = stl_io::read_stl(&mut reader)?;

        // Convert IndexedMesh into Vec<Triangle>
        let triangles = indexed_mesh
            .faces
            .iter()
            .map(|face| {
                let vertices = [
                    indexed_mesh.vertices[face.vertices[0] as usize],
                    indexed_mesh.vertices[face.vertices[1] as usize],
                    indexed_mesh.vertices[face.vertices[2] as usize],
                ];
                Triangle {
                    normal: face.normal,
                    vertices,
                }
            })
            .collect();

        Ok(triangles)
    }
}
