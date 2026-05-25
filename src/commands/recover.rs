use anyhow::{Context, Result, bail};

use crate::config::{load_config, save_config};
use crate::git::find_git_root;
use crate::logger::Logger;
use crate::wrapper::{WrapperStatus, preserve_foreign_hook, wrapper_status, write_wrapper};

pub fn run(logger: &dyn Logger, hook_name: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let (_repo_root, git_dir) = find_git_root(&cwd)?;

    let mut config = load_config(&git_dir)?;
    if !config.hooks.contains_key(hook_name) {
        bail!(
            "nothing to recover — '{}' was never installed; use 'krok add' first",
            hook_name
        );
    }

    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).context("failed to create hooks directory")?;
    let hook_path = hooks_dir.join(hook_name);

    match wrapper_status(&hook_path, hook_name) {
        WrapperStatus::Aligned => {
            logger.notice(&format!("hook '{}' is up to date", hook_name));
        }
        WrapperStatus::Missing => {
            write_wrapper(&hook_path, hook_name)?;
            logger.notice(&format!("wrote wrapper for '{}'", hook_name));
        }
        WrapperStatus::DriftedKrok => {
            write_wrapper(&hook_path, hook_name)?;
            logger.notice(&format!(
                "replaced outdated krok wrapper for '{}'",
                hook_name
            ));
        }
        WrapperStatus::DriftedForeign => {
            let original = config.clone();
            let jobs = config.hooks.get_mut(hook_name).expect("entry exists");
            preserve_foreign_hook(logger, &hooks_dir, &hook_path, hook_name, jobs)?;
            write_wrapper(&hook_path, hook_name)?;
            if config != original {
                save_config(&git_dir, &config)?;
            }
            logger.notice(&format!(
                "preserved foreign hook and wrote krok wrapper for '{}'",
                hook_name
            ));
        }
    }

    Ok(())
}
