use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn krok_bin() -> &'static str {
    env!("CARGO_BIN_EXE_krok")
}

fn run_krok(cwd: &Path, args: &[&str]) {
    let output = Command::new(krok_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "krok {:?} failed: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_init(cwd: &Path) {
    let status = Command::new("git")
        .arg("init")
        .current_dir(cwd)
        .status()
        .expect("failed to execute git init");
    assert!(status.success(), "git init failed");
}

#[test]
fn installing_two_hooks_merges_config() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo", "one"]);
    run_krok(repo, &["add", "pre-push", "echo", "two"]);

    let config_path = repo.join(".git").join("krok-config.yml");
    let content = std::fs::read_to_string(&config_path).expect("read config");

    let value: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse yaml");
    let hooks = value
        .get("hooks")
        .and_then(|h| h.as_mapping())
        .expect("config must contain a `hooks` mapping");

    assert!(
        hooks.contains_key(serde_yaml::Value::String("pre-commit".into())),
        "config missing pre-commit after second install: {content}"
    );
    assert!(
        hooks.contains_key(serde_yaml::Value::String("pre-push".into())),
        "config missing pre-push after second install: {content}"
    );

    assert!(
        repo.join(".git").join("hooks").join("pre-commit").exists(),
        "pre-commit wrapper missing"
    );
    assert!(
        repo.join(".git").join("hooks").join("pre-push").exists(),
        "pre-push wrapper missing"
    );
}

#[test]
fn add_on_uninstalled_hook_installs_wrapper_then_adds_job() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    git_init(repo);

    // No prior `install` call.
    run_krok(repo, &["add", "pre-commit", "echo", "hello"]);

    let wrapper = repo.join(".git").join("hooks").join("pre-commit");
    assert!(
        wrapper.exists(),
        "pre-commit wrapper missing — add should have installed it"
    );

    let content =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("config must have hooks.pre-commit as a sequence");

    let has_echo_job = jobs.iter().any(|job| {
        job.get("cmd").and_then(|c| c.as_str()) == Some("echo hello")
    });
    assert!(has_echo_job, "expected 'echo hello' job in config: {content}");
}
