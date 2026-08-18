use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::config::config_path;
use crate::git::{self, Repository};
use crate::logger::Logger;
use crate::shell::shell_path;

pub fn show(logger: &dyn Logger) -> Result<()> {
    let repo = ensure_repo_root()?;
    let path = config_path(&repo.git_dir);
    if !path.exists() {
        bail!(
            "no config at {}; use 'krok add' to create one",
            path.display()
        );
    }
    logger.debug(&format!("showing {}", path.display()));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    logger.info(content.trim_end());
    Ok(())
}

pub fn path(logger: &dyn Logger) -> Result<()> {
    let repo = ensure_repo_root()?;
    let path = config_path(&repo.git_dir);
    if !path.exists() {
        bail!(
            "no config at {}; use 'krok add' to create one",
            path.display()
        );
    }
    logger.info(&path.display().to_string());
    Ok(())
}

pub fn edit(logger: &dyn Logger) -> Result<()> {
    let repo = ensure_repo_root()?;
    let path = config_path(&repo.git_dir);
    if !path.exists() {
        bail!(
            "no config at {}; use 'krok add' to create one",
            path.display()
        );
    }
    let editor = git_editor(&repo.root)?;
    logger.debug(&format!("opening editor for {}", path.display()));

    let path_str = shell_path(&path);
    let cmd = format!("{} \"{}\"", editor, path_str);
    let status = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&repo.root)
        .status()
        .context("failed to spawn editor via sh")?;
    if !status.success() {
        bail!("editor exited with code {:?}", status.code());
    }
    Ok(())
}

fn ensure_repo_root() -> Result<Repository> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let repo = git::discover(&cwd)?;
    if !repo.at_root {
        bail!(
            "config must be run from the repository root ({})",
            repo.root.display()
        );
    }
    Ok(repo)
}

fn git_editor(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["var", "GIT_EDITOR"])
        .current_dir(repo_root)
        .output()
        .context("failed to run `git var GIT_EDITOR`")?;
    if !output.status.success() {
        bail!(
            "`git var GIT_EDITOR` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let editor = String::from_utf8(output.stdout)
        .context("`git var GIT_EDITOR` returned non-utf8 output")?
        .trim()
        .to_string();
    if editor.is_empty() {
        bail!("`git var GIT_EDITOR` returned an empty editor");
    }
    Ok(editor)
}
