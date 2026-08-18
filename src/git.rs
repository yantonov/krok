use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct Repository {
    /// Top level of the working tree.
    pub root: PathBuf,
    /// The git directory shared by every worktree. The config belongs here
    /// rather than in the one of the worktree it was written from, because the
    /// hooks it drives are shared too.
    pub git_dir: PathBuf,
    /// Honours `core.hooksPath`.
    pub hooks_dir: PathBuf,
    /// Whether `start` was the top level itself.
    pub at_root: bool,
}

/// Ask git where things are, rather than looking for a `.git` directory.
///
/// Only the plainest of checkouts has one. A linked worktree and a submodule
/// have a `.git` file naming a directory elsewhere, that directory is not the
/// one hooks live in, and `core.hooksPath` can move them somewhere else again.
pub fn discover(start: &Path) -> Result<Repository> {
    let output = Command::new("git")
        .args([
            "rev-parse",
            "--show-toplevel",
            "--git-common-dir",
            "--git-path",
            "hooks",
            "--show-prefix",
        ])
        .current_dir(start)
        .output()
        .context("failed to run `git rev-parse`")?;

    if !output.status.success() {
        let complaint = String::from_utf8_lossy(&output.stderr);
        bail!(
            "not inside a git repository with a working tree: {}",
            complaint.trim()
        );
    }

    let text = String::from_utf8(output.stdout).context("`git rev-parse` returned non-utf8")?;
    let mut answers = text.lines();
    let mut next = |what: &str| -> Result<String> {
        answers
            .next()
            .map(str::to_string)
            .with_context(|| format!("`git rev-parse` did not report the {what}"))
    };

    let root = next("repository root")?;
    let git_dir = next("git directory")?;
    let hooks_dir = next("hooks directory")?;
    // Empty at the top level, and the last line, so git leaves nothing to read.
    let prefix = answers.next().unwrap_or_default();

    Ok(Repository {
        root: absolute(start, &root),
        git_dir: absolute(start, &git_dir),
        hooks_dir: absolute(start, &hooks_dir),
        at_root: prefix.is_empty(),
    })
}

/// Git answers with a path relative to the directory it was asked in unless it
/// has reason to do otherwise.
fn absolute(start: &Path, answer: &str) -> PathBuf {
    let path = Path::new(answer);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        start.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Git answers with a path relative to the directory it was asked in, so a
    // relative answer only means anything alongside that directory.
    #[test]
    fn a_relative_answer_is_read_against_the_directory_git_was_asked_in() {
        let start = Path::new("/repo/deep/sub");
        assert_eq!(absolute(start, ".git"), start.join(".git"));
        assert_eq!(
            absolute(start, "../../.git/hooks"),
            start.join("../../.git/hooks")
        );
    }

    #[test]
    fn an_absolute_answer_is_taken_as_it_is() {
        // Whatever counts as absolute here: the shape differs by platform.
        let absolute_here = std::env::current_dir().expect("a current directory");
        assert!(absolute_here.is_absolute());
        assert_eq!(
            absolute(
                Path::new("/somewhere/else"),
                &absolute_here.to_string_lossy()
            ),
            absolute_here
        );
    }
}
