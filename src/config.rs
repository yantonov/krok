use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub hooks: HashMap<String, Vec<Job>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Job {
    pub key: String,
    pub title: String,
    pub cmd: String,
}

pub fn config_path(git_dir: &Path) -> std::path::PathBuf {
    git_dir.join("krok-config.yml")
}

pub fn load_config(git_dir: &Path) -> Result<Config> {
    let path = config_path(git_dir);
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse config at {}", path.display()))?;
    Ok(config)
}

pub fn save_config(git_dir: &Path, config: &Config) -> Result<()> {
    let path = config_path(git_dir);
    let content = serde_yaml::to_string(config).context("failed to serialize config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("failed to write config at {}", path.display()))?;
    Ok(())
}
