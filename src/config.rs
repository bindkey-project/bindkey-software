use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub api_url: String,
}

impl AppConfig {
    pub fn load() -> Self {
        match fs::read_to_string("bindkey_config.toml") {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|_| Self::default()),
            Err(_) => Self::default(),
        }
    }

    pub fn default() -> Self {
        Self {
            api_url: "https://api.bindkey.local".to_string(),
        }
    }
}
