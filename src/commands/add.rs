use anyhow::{Context, Result, bail};

use crate::commands::install::ensure_installed;
use crate::config::{Job, load_config, save_config};
use crate::git;
use crate::hooks;
use crate::logger::Logger;

pub fn run(logger: &dyn Logger, hook_name: &str, args: &[String], force: bool) -> Result<()> {
    if args.is_empty() {
        bail!("add requires at least one argument (the command to register)");
    }

    hooks::ensure_valid(hook_name, force)?;

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let repo = git::discover(&cwd)?;

    ensure_installed(logger, &repo, hook_name)?;

    let mut config = load_config(&repo.git_dir)?;
    let jobs = config.hooks.entry(hook_name.to_string()).or_default();

    let cmd = args.join(" ");
    let key = derive_key(&cmd);

    if jobs.iter().any(|j| j.key == key) {
        bail!(
            "a job with key '{}' is already registered for hook '{}'",
            key,
            hook_name
        );
    }

    jobs.push(Job {
        key: key.clone(),
        cmd: cmd.clone(),
    });
    save_config(&repo.git_dir, &config)?;

    logger.debug(&format!("added job '{}' to hook '{}'", key, hook_name));
    logger.debug(&format!("  cmd: {}", cmd));
    Ok(())
}

fn derive_key(cmd: &str) -> String {
    let raw: String = cmd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();

    let mut key = String::new();
    let mut last_dash = true;
    for c in raw.chars() {
        if c == '-' {
            if !last_dash {
                key.push('-');
            }
            last_dash = true;
        } else {
            key.push(c);
            last_dash = false;
        }
    }
    let key = key.trim_end_matches('-').to_string();
    if key.is_empty() {
        "job".to_string()
    } else {
        key
    }
}

#[cfg(test)]
mod tests {
    use super::derive_key;

    #[test]
    fn a_command_becomes_its_words_joined() {
        assert_eq!(derive_key("cargo test"), "cargo-test");
    }

    // The example the readme gives, which is also the one that shows a run of
    // separators collapsing rather than repeating.
    #[test]
    fn punctuation_collapses_into_single_separators() {
        assert_eq!(
            derive_key("cargo clippy -- -D warnings"),
            "cargo-clippy-D-warnings"
        );
        assert_eq!(derive_key("./scripts/check.sh"), "scripts-check-sh");
    }

    #[test]
    fn separators_at_either_end_are_left_off() {
        assert_eq!(derive_key("  echo hi  "), "echo-hi");
        assert_eq!(derive_key("--echo--"), "echo");
    }

    #[test]
    fn a_command_of_nothing_but_punctuation_still_has_a_key() {
        assert_eq!(derive_key("---"), "job");
        assert_eq!(derive_key(""), "job");
    }
}
