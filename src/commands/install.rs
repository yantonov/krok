use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::{Job, load_config, save_config};
use crate::git::find_git_root;

pub fn run(hook_name: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let (repo_root, git_dir) = find_git_root(&cwd)?;

    if cwd != repo_root {
        bail!(
            "install must be run from the repository root ({})",
            repo_root.display()
        );
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir).context("failed to create hooks directory")?;

    let hook_path = hooks_dir.join(hook_name);
    let mut config = load_config(&git_dir)?;
    let jobs = config.hooks.entry(hook_name.to_string()).or_default();

    // If a non-krok hook already exists, preserve it.
    if hook_path.exists() && !is_krok_script(&hook_path) {
        let saved_dir = hooks_dir.join(format!("{}-hooks", hook_name));
        fs::create_dir_all(&saved_dir)
            .with_context(|| format!("failed to create {}", saved_dir.display()))?;

        let saved_name = format!("existing-{}", hook_name);
        let saved_path = saved_dir.join(&saved_name);
        fs::copy(&hook_path, &saved_path)
            .with_context(|| format!("failed to copy existing hook to {}", saved_path.display()))?;
        set_executable(&saved_path)?;

        let relative_cmd = format!("{}-hooks/{}", hook_name, saved_name);
        if !jobs.iter().any(|j| j.key == "existing-hook") {
            jobs.push(Job {
                key: "existing-hook".to_string(),
                title: "existing tool".to_string(),
                cmd: relative_cmd,
            });
            println!("preserved existing hook as {}", saved_path.display());
        }
    }

    let script = format!(
        "#!/usr/bin/env bash\n# git hook manager wrapper\nexec krok run {hook_name} \"$@\"\n"
    );
    fs::write(&hook_path, script)
        .with_context(|| format!("failed to write hook script to {}", hook_path.display()))?;
    set_executable(&hook_path)?;

    save_config(&git_dir, &config)?;

    println!(
        "installed krok as hook '{}' at {}",
        hook_name,
        hook_path.display()
    );
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

fn is_krok_script(path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        return content.contains("git hook manager wrapper");
    }
    false
}
