use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub favorites: Vec<String>,
    pub last_selected: Vec<String>,
}

fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("rustycat");
    Ok(dir)
}

fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("favorites.json"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let data = fs::read_to_string(&path).context("Failed to read config file")?;
    let config: Config = serde_json::from_str(&data).context("Failed to parse config file")?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).context("Failed to create config directory")?;
    let path = config_path()?;
    let data = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&path, data).context("Failed to write config file")?;
    Ok(())
}
