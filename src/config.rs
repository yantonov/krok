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

    let temporary = path.with_file_name(format!(
        "{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));

    std::fs::write(&temporary, content)
        .with_context(|| format!("failed to write config at {}", temporary.display()))?;

    if let Err(error) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error)
            .with_context(|| format!("failed to replace config at {}", path.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn one_job(cmd: &str) -> Config {
        let mut config = Config::default();
        config.hooks.insert(
            "pre-commit".to_string(),
            vec![Job {
                key: "job".to_string(),
                cmd: cmd.to_string(),
            }],
        );
        config
    }

    #[test]
    fn a_saved_config_loads_back() {
        let tmp = TempDir::new().expect("tempdir");
        let config = one_job("cargo test");
        save_config(tmp.path(), &config).expect("save");
        assert_eq!(load_config(tmp.path()).expect("load"), config);
    }

    // Landing on a file that already exists is the part that differs by platform.
    #[test]
    fn saving_replaces_a_config_already_there() {
        let tmp = TempDir::new().expect("tempdir");
        save_config(tmp.path(), &one_job("first")).expect("save");
        save_config(tmp.path(), &one_job("second")).expect("save again");
        assert_eq!(load_config(tmp.path()).expect("load"), one_job("second"));
    }

    #[test]
    fn no_working_file_is_left_beside_the_config() {
        let tmp = TempDir::new().expect("tempdir");
        save_config(tmp.path(), &one_job("cargo test")).expect("save");

        let left: Vec<String> = std::fs::read_dir(tmp.path())
            .expect("read dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into())
            .collect();
        assert_eq!(left, vec!["krok-config.yml".to_string()], "{left:?}");
    }

    #[test]
    fn a_directory_with_no_config_loads_as_empty() {
        let tmp = TempDir::new().expect("tempdir");
        assert_eq!(load_config(tmp.path()).expect("load"), Config::default());
    }
}
