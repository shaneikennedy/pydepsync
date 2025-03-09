use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

/// Configuration that can be defined in a .pydepsync.toml
#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub exclude_dirs: Option<Vec<String>>,
    pub extra_indexes: Option<Vec<String>>,
    pub preferred_index: Option<String>,
    pub remap: Option<HashMap<String, String>>,
}

/// Load possible config from .pydepsync.toml
pub fn load_config() -> Config {
    // Check repo root first, then home directory
    let path = Path::new(".pydepsync.toml");

    if path.exists() {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => "".to_string(),
        };
        return match toml::from_str(&contents) {
            Ok(config) => config,
            Err(_) => {
                eprintln!("Warning: Failed to parse config file at {:?}", path);
                Config::default()
            }
        };
    }
    Config::default()
}
