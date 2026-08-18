use std::path::Path;
use std::process;

use anyhow::{Context, Result};

use crate::config::{Job, load_config};
use crate::env::{HOOKS_DIR_VAR, REPO_ROOT_VAR};
use crate::git;
use crate::logger::Logger;
use crate::shell::{shell_command, shell_path};
use crate::wrapper::EXISTING_HOOK_KEY;

pub fn run(logger: &dyn Logger, hook_name: &str, hook_args: &[String]) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let repo = git::discover(&cwd)?;

    let config = load_config(&repo.git_dir)?;
    let jobs = match config.hooks.get(hook_name) {
        Some(j) if !j.is_empty() => j,
        Some(_) | None => return Ok(()),
    };

    // Not $SHELL: a hook has to behave the same for everyone working in the
    // repository, and fish or nu would not read the posix syntax jobs are
    // written in.
    let shell = "sh";

    for job in jobs {
        logger.debug(&format!("[krok] running '{}': {}", job.key, job.cmd));

        let cmd = legacy_preserved_cmd(job, &repo.hooks_dir).unwrap_or_else(|| job.cmd.clone());

        let script = format!("{cmd} \"$@\"");

        let status = shell_command(shell, &script, hook_name, hook_args)
            // Where git itself fires hooks from.
            .current_dir(&repo.root)
            .env(REPO_ROOT_VAR, shell_path(&repo.root))
            .env(HOOKS_DIR_VAR, shell_path(&repo.hooks_dir))
            .status()
            .with_context(|| format!("failed to start shell for job '{}'", job.key))?;

        if !status.success() {
            let code = status.code().unwrap_or(1);
            logger.error(&format!(
                "[krok] hook '{}' failed at job '{}' (cmd: {})",
                hook_name, job.key, job.cmd
            ));
            process::exit(code);
        }
    }

    Ok(())
}

/// Configs written before the preserved hook named its own location through the
/// environment hold it as a bare path relative to the hooks directory.
///
/// The one reserved key is read that way, and nothing else: a job of the user's
/// own reaches the shell as written, which is what lets it be a relative path.
fn legacy_preserved_cmd(job: &Job, hooks_dir: &Path) -> Option<String> {
    if job.key != EXISTING_HOOK_KEY || job.cmd.contains(HOOKS_DIR_VAR) {
        return None;
    }
    Some(format!("\"{}/{}\"", shell_path(hooks_dir), job.cmd))
}
