use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use crate::SharedSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub username: String,
    pub theme: String,
    pub auto_save: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RendererSettings {
    pub render_scale: f32,
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
            editor: RendererSettings { render_scale: 1.0 },
            network: NetworkSettings {
                timeout: 30,
                use_https: true,
            },
        }
    }
}

impl Settings {
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&content)?;
        Ok(settings)
    }

    pub fn load_user_settings() -> SharedSettings {
        let settings_file = Path::new("config/settings/user_settings.toml");
        // Load settings from file, or create new defaults if file doesn't exist
        let settings = if settings_file.exists() {
            match Settings::load_from_file(settings_file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to load user settings file: {:?}", e);
                    match Settings::load_from_file(Path::new(
                        "config/settings/default_settings.toml",
                    )) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to load default settings file: {:?}", e);
                            println!("Loading hardcoded defaults");
                            Settings::default()
                        }
                    }
                }
            }
        } else {
            println!("User settings file not found, loading default settings file");
            match Settings::load_from_file(Path::new("config/settings/default_settings.toml")) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to load default settings file: {:?}", e);
                    println!("Loading hardcoded defaults");
                    Settings::default()
                }
            }
        };
        Rc::new(RefCell::new(settings))
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        // Check if the directory exists, and if not, create it
        if !Path::new(path).exists() {
            fs::create_dir_all(path).expect("Failed to create directory");
        }

        let content = toml::to_string(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}
