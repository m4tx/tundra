use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use log::warn;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Default, Serialize)]
pub struct Config {
    pub mal: MALConfig,
}

#[derive(Deserialize, Default, Serialize)]
pub struct MALConfig {
    pub access_token: String,
    pub refresh_token: String,
}

impl Config {
    fn new() -> Self {
        Default::default()
    }

    fn config_path() -> PathBuf {
        let project_dirs = ProjectDirs::from("com", "m4tx", "tundra")
            .expect("config directory could not be obtained");
        project_dirs.config_dir().to_owned()
    }

    pub fn load() -> Self {
        let config_file = Self::config_path().join("config.toml");
        let config_file_str = fs::read_to_string(config_file);
        if let Ok(str) = config_file_str {
            toml::from_str(&str).expect("Could not parse config")
        } else {
            warn!("No config file found - loading defaults");
            Self::new()
        }
    }

    pub fn save(&self) {
        let config_path = Self::config_path();
        fs::create_dir_all(&config_path).expect("Could not create config directory");
        let config_file = config_path.join("config.toml");

        let toml = toml::to_string(self).expect("Could not serialize config");
        fs::write(config_file, toml).expect("Could not write config to file");
    }

    pub fn is_mal_authenticated(&self) -> bool {
        !self.mal.access_token.is_empty() && !self.mal.refresh_token.is_empty()
    }
}
