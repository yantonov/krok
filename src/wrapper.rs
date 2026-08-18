use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Job;
use crate::env::HOOKS_DIR_VAR;
use crate::logger::Logger;

const KROK_MARKER: &str = "git hook manager wrapper";

/// Reserved job key: the hook that was already in place when krok took over.
pub const EXISTING_HOOK_KEY: &str = "existing-hook";

pub enum WrapperStatus {
    Aligned,
    Missing,
    DriftedKrok,
    DriftedForeign,
}

pub fn expected_wrapper(hook_name: &str) -> String {
    format!("#!/usr/bin/env sh\n# {KROK_MARKER}\nexec krok run {hook_name} \"$@\"\n")
}

pub fn wrapper_status(hook_path: &Path, hook_name: &str) -> WrapperStatus {
    let Ok(content) = fs::read_to_string(hook_path) else {
        return WrapperStatus::Missing;
    };
    if content == expected_wrapper(hook_name) {
        WrapperStatus::Aligned
    } else if content.contains(KROK_MARKER) {
        WrapperStatus::DriftedKrok
    } else {
        WrapperStatus::DriftedForeign
    }
}

pub fn write_wrapper(hook_path: &Path, hook_name: &str) -> Result<()> {
    let script = expected_wrapper(hook_name);
    fs::write(hook_path, script)
        .with_context(|| format!("failed to write hook script to {}", hook_path.display()))?;
    set_executable(hook_path)
}

/// Where a foreign hook is kept once krok has taken over its file.
pub fn preserved_path(hooks_dir: &Path, hook_name: &str) -> PathBuf {
    hooks_dir
        .join(format!("{hook_name}-hooks"))
        .join(format!("existing-{hook_name}"))
}

/// The command that runs the preserved hook.
///
/// It locates itself through the environment krok run exports, so that the
/// runner can hand every job to the shell exactly as written. Spelled as a bare
/// relative path instead, it would need the runner to recognise and rewrite it,
/// and no rule for that can tell it apart from a path of the user's own, such
/// as the './scripts/check.sh' the readme suggests registering.
///
/// Quoted, because the path of the repository may well contain a space.
pub fn preserved_cmd(hook_name: &str) -> String {
    format!("\"${HOOKS_DIR_VAR}/{hook_name}-hooks/existing-{hook_name}\"")
}

pub fn preserve_foreign_hook(
    logger: &dyn Logger,
    hooks_dir: &Path,
    hook_path: &Path,
    hook_name: &str,
    jobs: &mut Vec<Job>,
) -> Result<()> {
    let saved_path = preserved_path(hooks_dir, hook_name);
    let saved_dir = saved_path
        .parent()
        .expect("the preserved path is built with a parent");
    fs::create_dir_all(saved_dir)
        .with_context(|| format!("failed to create {}", saved_dir.display()))?;

    fs::copy(hook_path, &saved_path)
        .with_context(|| format!("failed to copy existing hook to {}", saved_path.display()))?;
    set_executable(&saved_path)?;

    if !jobs.iter().any(|j| j.key == EXISTING_HOOK_KEY) {
        jobs.push(Job {
            key: EXISTING_HOOK_KEY.to_string(),
            cmd: preserved_cmd(hook_name),
        });
        logger.debug(&format!(
            "preserved existing hook as {}",
            saved_path.display()
        ));
    }
    Ok(())
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .with_context(|| format!("failed to read metadata of {}", path.display()))?
            .permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(path, perms)
            .with_context(|| format!("failed to set executable bit on {}", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn the_wrapper_runs_krok_for_the_hook_it_belongs_to() {
        let wrapper = expected_wrapper("pre-commit");
        assert!(wrapper.starts_with("#!/usr/bin/env sh\n"), "{wrapper}");
        assert!(wrapper.contains(KROK_MARKER), "{wrapper}");
        assert!(
            wrapper.contains("exec krok run pre-commit \"$@\""),
            "{wrapper}"
        );
    }

    #[test]
    fn a_file_that_is_not_there_is_missing_rather_than_foreign() {
        let tmp = TempDir::new().expect("tempdir");
        let status = wrapper_status(&tmp.path().join("pre-commit"), "pre-commit");
        assert!(matches!(status, WrapperStatus::Missing));
    }

    #[test]
    fn the_wrapper_as_written_is_aligned() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("pre-commit");
        fs::write(&path, expected_wrapper("pre-commit")).expect("write");
        assert!(matches!(
            wrapper_status(&path, "pre-commit"),
            WrapperStatus::Aligned
        ));
    }

    // The marker is what separates a wrapper an older krok wrote, which may be
    // replaced, from a script belonging to someone else, which may not.
    #[test]
    fn the_marker_decides_who_a_changed_file_belongs_to() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("pre-commit");

        fs::write(
            &path,
            format!("#!/bin/sh\n# {KROK_MARKER}\nsomething else\n"),
        )
        .expect("write");
        assert!(matches!(
            wrapper_status(&path, "pre-commit"),
            WrapperStatus::DriftedKrok
        ));

        fs::write(&path, "#!/bin/sh\necho mine\n").expect("write");
        assert!(matches!(
            wrapper_status(&path, "pre-commit"),
            WrapperStatus::DriftedForeign
        ));
    }

    // A wrapper written for one hook is not the wrapper of another, or recover
    // would call an unrelated hook up to date.
    #[test]
    fn a_wrapper_for_another_hook_has_drifted() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("pre-push");
        fs::write(&path, expected_wrapper("pre-commit")).expect("write");
        assert!(matches!(
            wrapper_status(&path, "pre-push"),
            WrapperStatus::DriftedKrok
        ));
    }

    #[test]
    fn the_preserved_hook_is_named_after_the_hook_it_replaced() {
        let path = preserved_path(Path::new("/repo/.git/hooks"), "pre-commit");
        assert!(
            path.ends_with("pre-commit-hooks/existing-pre-commit"),
            "{path:?}"
        );
    }

    // The command has to name the file preserved_path returns, through the
    // variable krok run exports rather than as a path of its own.
    #[test]
    fn the_preserved_command_points_at_the_preserved_file() {
        let cmd = preserved_cmd("pre-commit");
        assert_eq!(
            cmd,
            format!("\"${HOOKS_DIR_VAR}/pre-commit-hooks/existing-pre-commit\"")
        );

        let tail = preserved_path(Path::new("hooks"), "pre-commit");
        let tail = tail.strip_prefix("hooks").expect("built from that prefix");
        assert!(
            cmd.contains(&crate::shell::shell_path(tail)),
            "{cmd} does not name {tail:?}"
        );
    }
}
