use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{Config, load_config, save_config};
use crate::logger::Logger;
use crate::wrapper::{WrapperStatus, preserve_foreign_hook, wrapper_status, write_wrapper};

pub fn ensure_installed(logger: &dyn Logger, git_dir: &Path, hook_name: &str) -> Result<()> {
    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).context("failed to create hooks directory")?;

    let hook_path = hooks_dir.join(hook_name);
    let mut config = load_config(git_dir)?;

    if is_fully_installed(&hook_path, &hooks_dir, hook_name, &config) {
        return Ok(());
    }

    let original = config.clone();
    let jobs = config.hooks.entry(hook_name.to_string()).or_default();

    if matches!(
        wrapper_status(&hook_path, hook_name),
        WrapperStatus::DriftedForeign
    ) {
        preserve_foreign_hook(logger, &hooks_dir, &hook_path, hook_name, jobs)?;
    }

    write_wrapper(&hook_path, hook_name)?;

    if config != original {
        save_config(git_dir, &config)?;
    }

    logger.debug(&format!(
        "installed krok as hook '{}' at {}",
        hook_name,
        hook_path.display()
    ));
    Ok(())
}

fn is_fully_installed(
    hook_path: &Path,
    hooks_dir: &Path,
    hook_name: &str,
    config: &Config,
) -> bool {
    if !matches!(wrapper_status(hook_path, hook_name), WrapperStatus::Aligned) {
        return false;
    }
    let Some(jobs) = config.hooks.get(hook_name) else {
        return false;
    };
    jobs.iter()
        .filter(|j| j.key == "existing-hook")
        .all(|j| hooks_dir.join(&j.cmd).exists())
}
