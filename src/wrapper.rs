use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Job;
use crate::logger::Logger;

const KROK_MARKER: &str = "git hook manager wrapper";

pub enum WrapperStatus {
    Aligned,
    Missing,
    DriftedKrok,
    DriftedForeign,
}

pub fn expected_wrapper(hook_name: &str) -> String {
    format!("#!/usr/bin/env bash\n# {KROK_MARKER}\nexec krok run {hook_name} \"$@\"\n")
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

pub fn preserve_foreign_hook(
    logger: &dyn Logger,
    hooks_dir: &Path,
    hook_path: &Path,
    hook_name: &str,
    jobs: &mut Vec<Job>,
) -> Result<()> {
    let saved_dir = hooks_dir.join(format!("{}-hooks", hook_name));
    fs::create_dir_all(&saved_dir)
        .with_context(|| format!("failed to create {}", saved_dir.display()))?;

    let saved_name = format!("existing-{}", hook_name);
    let saved_path = saved_dir.join(&saved_name);
    fs::copy(hook_path, &saved_path)
        .with_context(|| format!("failed to copy existing hook to {}", saved_path.display()))?;
    set_executable(&saved_path)?;

    if !jobs.iter().any(|j| j.key == "existing-hook") {
        let relative_cmd = format!("{}-hooks/{}", hook_name, saved_name);
        jobs.push(Job {
            key: "existing-hook".to_string(),
            cmd: relative_cmd,
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
