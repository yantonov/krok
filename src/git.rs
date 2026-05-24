use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// Walk up from `start` until a `.git` directory is found.
/// Returns `(repo_root, git_dir)`.
pub fn find_git_root(start: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut current = start.to_path_buf();
    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return Ok((current, git_dir));
        }
        if !current.pop() {
            bail!("not inside a git repository (no .git directory found)");
        }
    }
}
