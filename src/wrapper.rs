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
