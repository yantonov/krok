use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::config::config_path;
use crate::git::find_git_root;
use crate::logger::Logger;

pub fn show(logger: &dyn Logger) -> Result<()> {
    let (_repo_root, git_dir) = ensure_repo_root()?;
    let path = config_path(&git_dir);
    if !path.exists() {
        bail!(
            "no config at {}; use 'krok add' to create one",
            path.display()
        );
    }
    logger.info(&format!("showing {}", path.display()));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    logger.notice(content.trim_end());
    Ok(())
}

pub fn edit(logger: &dyn Logger) -> Result<()> {
    let (repo_root, git_dir) = ensure_repo_root()?;
    let path = config_path(&git_dir);
    if !path.exists() {
        bail!(
            "no config at {}; use 'krok add' to create one",
            path.display()
        );
    }
    let editor = git_editor(&repo_root)?;
    logger.info(&format!("opening editor for {}", path.display()));

    let path_str = path.to_string_lossy().replace('\\', "/");
    let cmd = format!("{} \"{}\"", editor, path_str);
    let status = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&repo_root)
        .status()
        .context("failed to spawn editor via sh")?;
    if !status.success() {
        bail!("editor exited with code {:?}", status.code());
    }
    Ok(())
}

fn ensure_repo_root() -> Result<(PathBuf, PathBuf)> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let (repo_root, git_dir) = find_git_root(&cwd)?;
    if cwd != repo_root {
        bail!(
            "config must be run from the repository root ({})",
            repo_root.display()
        );
    }
    Ok((repo_root, git_dir))
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
