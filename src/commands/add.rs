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

    // Asking for a job that is already registered is what re-running the script
    // that bootstraps a checkout does, and the job list already says what it
    // asked for, so there is nothing to add and nothing to complain about.
    // Refusing failed those scripts, which run under `set -e`, and in a
    // workspace of several repositories took the ones not reached yet with them.
    // The wrapper was seen to either way: ensure_installed ran above.
    if let Some(existing) = jobs.iter().find(|j| j.cmd == cmd) {
        logger.info(&format!(
            "'{}' is already registered for hook '{}', as '{}'",
            cmd, hook_name, existing.key
        ));
        return Ok(());
    }

    let key = unique_key(&derive_key(&cmd), jobs);

    jobs.push(Job {
        key: key.clone(),
        cmd: cmd.clone(),
    });
    save_config(&repo.git_dir, &config)?;

    logger.debug(&format!("added job '{}' to hook '{}'", key, hook_name));
    logger.debug(&format!("  cmd: {}", cmd));
    Ok(())
}

/// `cargo clippy -- -D warnings` and `cargo clippy -D warnings` derive the same
/// key. Both are jobs worth having, so the second is numbered, not refused.
fn unique_key(derived: &str, jobs: &[Job]) -> String {
    let taken = |candidate: &str| jobs.iter().any(|job| job.key == candidate);

    if !taken(derived) {
        return derived.to_string();
    }

    let mut n = 2;
    loop {
        let candidate = format!("{derived}-{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
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
    use super::{Job, derive_key, unique_key};

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

    fn jobs(keys: &[&str]) -> Vec<Job> {
        keys.iter()
            .map(|key| Job {
                key: key.to_string(),
                cmd: format!("cmd for {key}"),
            })
            .collect()
    }

    #[test]
    fn a_key_nothing_holds_is_used_as_derived() {
        assert_eq!(
            unique_key("cargo-test", &jobs(&["cargo-clippy"])),
            "cargo-test"
        );
    }

    #[test]
    fn a_key_already_taken_is_numbered_from_two() {
        assert_eq!(
            unique_key(
                "cargo-clippy-D-warnings",
                &jobs(&["cargo-clippy-D-warnings"])
            ),
            "cargo-clippy-D-warnings-2"
        );
    }

    #[test]
    fn numbering_carries_on_past_a_number_already_taken() {
        let taken = jobs(&["lint", "lint-2", "lint-3"]);
        assert_eq!(unique_key("lint", &taken), "lint-4");
    }

    #[test]
    fn a_command_of_nothing_but_punctuation_still_has_a_key() {
        assert_eq!(derive_key("---"), "job");
        assert_eq!(derive_key(""), "job");
    }
}
