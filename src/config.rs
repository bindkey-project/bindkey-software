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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.api_url, "https://api.bindkey.local");
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            api_url: "https://test.local".to_string(),
        };
        let toml_string = toml::to_string(&config).unwrap();
        assert!(toml_string.contains("https://test.local"));
    }
}
