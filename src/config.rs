use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: Option<f32>,
    // Optional keys for other providers
    pub openai_api_key: Option<String>,
    pub xai_api_key: Option<String>, // Grok/xAI
    pub grok_api_key: Option<String>,
    pub groq_api_key: Option<String>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().expect("Could not find config directory");
        path.push("rusty-cli");
        path.push("config.toml");
        path
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
