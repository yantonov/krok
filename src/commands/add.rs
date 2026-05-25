use anyhow::{Context, Result, bail};

use crate::commands::install::ensure_installed;
use crate::config::{Job, load_config, save_config};
use crate::git::find_git_root;
use crate::logger::Logger;

pub fn run(logger: &dyn Logger, hook_name: &str, args: &[String]) -> Result<()> {
    if args.is_empty() {
        bail!("add requires at least one argument (the command to register)");
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let (_repo_root, git_dir) = find_git_root(&cwd)?;

    ensure_installed(logger, &git_dir, hook_name)?;

    let mut config = load_config(&git_dir)?;
    let jobs = config.hooks.entry(hook_name.to_string()).or_default();

    let cmd = args.join(" ");
    let key = derive_key(&cmd);

    if jobs.iter().any(|j| j.key == key) {
        bail!(
            "a job with key '{}' is already registered for hook '{}'",
            key,
            hook_name
        );
    }

    jobs.push(Job {
        key: key.clone(),
        cmd: cmd.clone(),
    });
    save_config(&git_dir, &config)?;

    logger.info(&format!("added job '{}' to hook '{}'", key, hook_name));
    logger.info(&format!("  cmd: {}", cmd));
    Ok(())
}

fn derive_key(cmd: &str) -> String {
    let raw: String = cmd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    let mut key = String::new();
    let mut last_dash = true;
    for c in raw.chars() {
        if c == '-' {
            if !last_dash {
                key.push('-');
            }
            last_dash = true;
        } else {
            key.push(c);
            last_dash = false;
        }
    }
    let key = key.trim_end_matches('-').to_string();
    if key.is_empty() {
        "job".to_string()
    } else {
        key
    }
}
