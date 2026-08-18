use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{Config, load_config, save_config};
use crate::git::Repository;
use crate::logger::Logger;
use crate::wrapper::{
    EXISTING_HOOK_KEY, WrapperStatus, locate_preserved, preserve_foreign_hook, wrapper_status,
    write_wrapper,
};

pub fn ensure_installed(logger: &dyn Logger, repo: &Repository, hook_name: &str) -> Result<()> {
    let hooks_dir = &repo.hooks_dir;
    fs::create_dir_all(hooks_dir).context("failed to create hooks directory")?;

    let hook_path = hooks_dir.join(hook_name);
    let mut config = load_config(&repo.git_dir)?;

    if is_fully_installed(&hook_path, repo, hook_name, &config) {
        return Ok(());
    }

    let original = config.clone();
    let jobs = config.hooks.entry(hook_name.to_string()).or_default();

    if matches!(
        wrapper_status(&hook_path, hook_name),
        WrapperStatus::DriftedForeign
    ) {
        preserve_foreign_hook(logger, &repo.git_dir, &hook_path, hook_name, jobs)?;
    }

    write_wrapper(&hook_path, hook_name)?;

    if config != original {
        save_config(&repo.git_dir, &config)?;
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
    repo: &Repository,
    hook_name: &str,
    config: &Config,
) -> bool {
    if !matches!(wrapper_status(hook_path, hook_name), WrapperStatus::Aligned) {
        return false;
    }
    let Some(jobs) = config.hooks.get(hook_name) else {
        return false;
    };
    // Wherever it was left, and not only where krok would put it now, or an
    // upgrade would read every hook installed by an earlier one as unfinished.
    !jobs.iter().any(|j| j.key == EXISTING_HOOK_KEY)
        || locate_preserved(&repo.git_dir, &repo.hooks_dir, hook_name).is_some()
}
