use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub username: String,
    pub theme: String,
    pub auto_save: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RendererSettings {
    pub internal_render_width: u32,
    pub internal_render_height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub timeout: u32,
    pub use_https: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub general: GeneralSettings,
    pub editor: RendererSettings,
    pub network: NetworkSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            general: GeneralSettings {
                username: String::from("Egg"),
                theme: String::from("system"),
                auto_save: true,
            },
            editor: RendererSettings {
                internal_render_width: 1920,
                internal_render_height: 1080,
            },
            network: NetworkSettings {
                timeout: 30,
                use_https: true,
            },
        }
    }
}

impl Settings {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&content)?;
        Ok(settings)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}
