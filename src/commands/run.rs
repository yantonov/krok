use std::path::Path;
use std::process;

use anyhow::{Context, Result};

use crate::config::load_config;
use crate::git::find_git_root;

pub fn run(hook_name: &str, hook_args: &[String]) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let (_repo_root, git_dir) = find_git_root(&cwd)?;
    let hooks_dir = git_dir.join("hooks");

    let config = load_config(&git_dir)?;
    let jobs = match config.hooks.get(hook_name) {
        Some(j) if !j.is_empty() => j,
        Some(_) | None => return Ok(()),
    };

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

    for job in jobs {
        println!("[krok] running '{}': {}", job.title, job.cmd);

        let resolved = resolve_cmd(&job.cmd, &hooks_dir);
        let cmd = if hook_args.is_empty() {
            resolved
        } else {
            format!("{} {}", resolved, hook_args.join(" "))
        };

        let status = process::Command::new(&shell)
            .arg("-c")
            .arg(&cmd)
            .status()
            .with_context(|| format!("failed to start shell for job '{}'", job.key))?;

        if !status.success() {
            let code = status.code().unwrap_or(1);
            eprintln!(
                "[krok] hook '{}' failed at job '{}' (title: {}, cmd: {})",
                hook_name, job.key, job.title, job.cmd
            );
            process::exit(code);
        }
    }

    Ok(())
}

fn resolve_cmd(cmd: &str, hooks_dir: &Path) -> String {
    let first_word = cmd.split_whitespace().next().unwrap_or(cmd);
    if first_word.contains('/') && !first_word.starts_with('/') {
        let hooks_dir_str = hooks_dir.to_string_lossy().replace('\\', "/");
        format!("{}/{}", hooks_dir_str, cmd)
    } else {
        cmd.to_string()
    }
}
