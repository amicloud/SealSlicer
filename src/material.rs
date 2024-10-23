use nalgebra::Vector3;
pub struct Material {
    pub roughness: f32,
    pub albedo: Vector3<f32>,
    pub base_reflectance: Vector3<f32>,
    pub metallicity: f32,
}

impl Material {
    pub fn default_resin() -> Material {
        let reflectance_b = 0.5;
        Self {
            roughness: 0.1,
            albedo: Vector3::new(0.75,0.75,0.35),
            base_reflectance: Vector3::new(reflectance_b,reflectance_b,reflectance_b),
            metallicity: 0.01
        }
    }
}