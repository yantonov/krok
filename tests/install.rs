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

fn fwd_slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[test]
fn run_executes_jobs_in_registered_order() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let log = repo.join("order.log");
    let log_str = fwd_slash(&log);

    run_krok(repo, &["add", "pre-commit", &format!("echo first >> {log_str}")]);
    run_krok(repo, &["add", "pre-commit", &format!("echo second >> {log_str}")]);
    run_krok(repo, &["add", "pre-commit", &format!("echo third >> {log_str}")]);

    run_krok(repo, &["run", "pre-commit"]);

    let content = std::fs::read_to_string(&log).expect("read log file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(
        lines,
        vec!["first", "second", "third"],
        "jobs ran out of order: {content}"
    );
}

#[test]
fn run_fails_when_any_job_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let marker = repo.join("after.txt");
    let marker_str = fwd_slash(&marker);

    run_krok(repo, &["add", "pre-commit", "true"]);
    run_krok(repo, &["add", "pre-commit", "false"]);
    run_krok(repo, &["add", "pre-commit", &format!("echo done > {marker_str}")]);

    let output = Command::new(krok_bin())
        .args(["run", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok run");

    assert!(
        !output.status.success(),
        "krok run should fail when a job fails; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !marker.exists(),
        "third job ran despite earlier failure — marker file should not exist"
    );
}

#[test]
fn add_appends_multiple_jobs_to_same_hook() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo one"]);
    run_krok(repo, &["add", "pre-commit", "echo two"]);
    run_krok(repo, &["add", "pre-commit", "echo three"]);

    let content = std::fs::read_to_string(repo.join(".git").join("krok-config.yml"))
        .expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&content).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("hooks.pre-commit must be a sequence");

    assert_eq!(
        jobs.len(),
        3,
        "expected 3 jobs after three adds, got: {content}"
    );
}

#[test]
fn add_rejects_duplicate_key() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo same"]);

    let output = Command::new(krok_bin())
        .args(["add", "pre-commit", "echo same"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");

    assert!(
        !output.status.success(),
        "duplicate add should fail; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already registered"),
        "expected duplicate-key error, got stderr: {stderr}"
    );
}

#[test]
fn add_with_no_command_args_bails() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["add", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");

    assert!(
        !output.status.success(),
        "add without a command should fail; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn info_silent_without_krok_debug() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["add", "pre-commit", "echo hi"])
        .current_dir(repo)
        .env_remove("KROK_DEBUG")
        .output()
        .expect("failed to execute krok");

    assert!(
        output.status.success(),
        "krok add failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "expected no stdout without KROK_DEBUG, got: {stdout}"
    );
}

#[test]
fn info_visible_with_krok_debug() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["add", "pre-commit", "echo hi"])
        .current_dir(repo)
        .env("KROK_DEBUG", "1")
        .output()
        .expect("failed to execute krok");

    assert!(
        output.status.success(),
        "krok add failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("added job"),
        "expected 'added job' line with KROK_DEBUG=1, got: {stdout}"
    );
}

#[test]
fn add_preserves_existing_non_krok_hook() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let hooks_dir = repo.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir).expect("create hooks dir");

    let existing_hook = hooks_dir.join("pre-commit");
    let existing_content = "#!/usr/bin/env bash\necho 'original hook'\n";
    std::fs::write(&existing_hook, existing_content).expect("write existing hook");

    run_krok(repo, &["add", "pre-commit", "echo new"]);

    let preserved = hooks_dir
        .join("pre-commit-hooks")
        .join("existing-pre-commit");
    assert!(
        preserved.exists(),
        "preserved hook file not found at {}",
        preserved.display()
    );
    let preserved_content = std::fs::read_to_string(&preserved).expect("read preserved hook");
    assert_eq!(
        preserved_content, existing_content,
        "preserved hook content does not match original"
    );

    let config = std::fs::read_to_string(repo.join(".git").join("krok-config.yml"))
        .expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&config).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("hooks.pre-commit must be a sequence");

    assert!(
        jobs.len() >= 2,
        "expected at least 2 jobs (preserved + new), got: {config}"
    );
    let first = &jobs[0];
    assert_eq!(
        first.get("key").and_then(|k| k.as_str()),
        Some("existing-hook"),
        "preserved hook should be registered as the first job: {config}"
    );

    // Wrapper at .git/hooks/pre-commit should now be the krok wrapper, not the original.
    let wrapper_content = std::fs::read_to_string(&existing_hook).expect("read wrapper");
    assert!(
        wrapper_content.contains("krok run"),
        "wrapper should now invoke krok run, got: {wrapper_content}"
    );
}

#[test]
fn run_forwards_hook_args_to_jobs() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let captured = repo.join("captured.txt");
    let captured_str = fwd_slash(&captured);

    // Stored cmd: "echo > /path/captured.txt". At run time, hook_args are appended,
    // so sh sees `echo > /path/captured.txt passed-arg` and writes "passed-arg" to the file.
    run_krok(repo, &["add", "pre-commit", &format!("echo > {captured_str}")]);

    run_krok(repo, &["run", "pre-commit", "passed-arg"]);

    let content = std::fs::read_to_string(&captured).expect("read captured file");
    assert!(
        content.contains("passed-arg"),
        "hook arg not forwarded to job: {content}"
    );
}

#[test]
fn recover_aligned_is_noop() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo hi"]);
    let wrapper = repo.join(".git").join("hooks").join("pre-commit");
    let before = std::fs::read_to_string(&wrapper).expect("read wrapper");

    let output = Command::new(krok_bin())
        .args(["recover", "pre-commit"])
        .current_dir(repo)
        .env_remove("KROK_DEBUG")
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "krok recover failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = std::fs::read_to_string(&wrapper).expect("read wrapper");
    assert_eq!(before, after, "wrapper changed on no-op recover");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("up to date"),
        "notice not visible on stdout (without KROK_DEBUG), got: {stdout}"
    );
}

#[test]
fn recover_writes_wrapper_when_missing() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo hi"]);
    let wrapper = repo.join(".git").join("hooks").join("pre-commit");
    std::fs::remove_file(&wrapper).expect("remove wrapper");

    let output = Command::new(krok_bin())
        .args(["recover", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "krok recover failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(wrapper.exists(), "wrapper not restored");
    let content = std::fs::read_to_string(&wrapper).expect("read wrapper");
    assert!(
        content.contains("exec krok run pre-commit"),
        "wrapper content unexpected: {content}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wrote wrapper"), "stdout: {stdout}");
}

#[test]
fn recover_replaces_drifted_krok_wrapper() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo hi"]);
    let wrapper = repo.join(".git").join("hooks").join("pre-commit");

    // Drift: still contains the krok marker, but the rest of the content differs.
    let drifted = "#!/usr/bin/env bash\n# git hook manager wrapper (old)\nexec krok run pre-commit \"$@\"\n";
    std::fs::write(&wrapper, drifted).expect("write drifted wrapper");

    let output = Command::new(krok_bin())
        .args(["recover", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "krok recover failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let after = std::fs::read_to_string(&wrapper).expect("read wrapper");
    assert_ne!(after, drifted, "drifted content not replaced");
    assert!(
        after.contains("exec krok run pre-commit"),
        "wrapper content unexpected: {after}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("replaced outdated"), "stdout: {stdout}");
}

#[test]
fn recover_preserves_foreign_hook() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo hi"]);
    let wrapper = repo.join(".git").join("hooks").join("pre-commit");

    let foreign = "#!/usr/bin/env bash\necho 'someone replaced the wrapper'\n";
    std::fs::write(&wrapper, foreign).expect("write foreign wrapper");

    let output = Command::new(krok_bin())
        .args(["recover", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "krok recover failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let preserved = repo
        .join(".git")
        .join("hooks")
        .join("pre-commit-hooks")
        .join("existing-pre-commit");
    assert!(preserved.exists(), "preserved file missing");
    let preserved_content = std::fs::read_to_string(&preserved).expect("read preserved");
    assert_eq!(preserved_content, foreign, "preserved content mismatch");

    let after_wrapper = std::fs::read_to_string(&wrapper).expect("read wrapper");
    assert!(
        after_wrapper.contains("exec krok run pre-commit"),
        "wrapper not restored to krok form: {after_wrapper}"
    );

    let config = std::fs::read_to_string(repo.join(".git").join("krok-config.yml"))
        .expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&config).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("hooks.pre-commit must be a sequence");
    assert!(
        jobs.iter().any(|j| {
            j.get("key").and_then(|k| k.as_str()) == Some("existing-hook")
        }),
        "existing-hook job not registered: {config}"
    );
}

#[test]
fn recover_without_config_entry_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["recover", "pre-commit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "recover should fail when hook was never installed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("never installed") || stderr.contains("nothing to recover"),
        "expected 'nothing to recover' error, got stderr: {stderr}"
    );
}

#[test]
fn config_show_outputs_config_file_content() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    run_krok(repo, &["add", "pre-commit", "echo hi"]);

    let output = Command::new(krok_bin())
        .args(["config", "show"])
        .current_dir(repo)
        .env_remove("KROK_DEBUG")
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "config show failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let file_content =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
    assert_eq!(
        stdout.trim_end(),
        file_content.trim_end(),
        "config show output did not match file content"
    );
}

#[test]
fn config_show_without_config_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["config", "show"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "config show should fail when no config exists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no config"),
        "expected 'no config' error, got stderr: {stderr}"
    );
}

#[test]
fn config_must_run_from_repo_root() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    run_krok(repo, &["add", "pre-commit", "echo hi"]);

    let subdir = repo.join("sub");
    std::fs::create_dir(&subdir).expect("create subdir");

    let output = Command::new(krok_bin())
        .args(["config", "show"])
        .current_dir(&subdir)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "config show should fail when not at repo root"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repository root"),
        "expected 'repository root' error, got stderr: {stderr}"
    );
}

#[test]
fn config_path_prints_config_path() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    run_krok(repo, &["add", "pre-commit", "echo hi"]);

    let output = Command::new(krok_bin())
        .args(["config", "path"])
        .current_dir(repo)
        .env_remove("KROK_DEBUG")
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "config path failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = repo.join(".git").join("krok-config.yml");
    assert_eq!(
        stdout.trim_end(),
        expected.display().to_string(),
        "config path output did not match expected path"
    );
}

#[test]
fn config_path_without_config_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["config", "path"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "config path should fail when no config exists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no config"),
        "expected 'no config' error, got stderr: {stderr}"
    );
}

#[test]
fn config_path_must_run_from_repo_root() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let subdir = repo.join("sub");
    std::fs::create_dir(&subdir).expect("create subdir");

    let output = Command::new(krok_bin())
        .args(["config", "path"])
        .current_dir(&subdir)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "config path should fail when not at repo root"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repository root"),
        "expected 'repository root' error, got stderr: {stderr}"
    );
}

#[test]
fn config_edit_invokes_git_editor() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    run_krok(repo, &["add", "pre-commit", "echo hi"]);

    let marker = repo.join("editor-ran.txt");
    let marker_str = fwd_slash(&marker);
    let editor = format!("touch {marker_str}");

    let output = Command::new(krok_bin())
        .args(["config", "edit"])
        .current_dir(repo)
        .env("GIT_EDITOR", &editor)
        .output()
        .expect("failed to execute krok");
    assert!(
        output.status.success(),
        "config edit failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        marker.exists(),
        "editor command did not run (marker file missing)"
    );
}

#[test]
fn add_rejects_unknown_hook_name_by_default() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["add", "pre-comit", "echo hi"]) // typo: should be pre-commit
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "add should reject unknown hook name without --force"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a known git hook name"),
        "expected validation error, got stderr: {stderr}"
    );
    assert!(
        stderr.contains("--force"),
        "error should mention --force escape hatch, got: {stderr}"
    );

    // Wrapper should not have been written
    let wrapper = repo.join(".git").join("hooks").join("pre-comit");
    assert!(
        !wrapper.exists(),
        "wrapper should not exist when validation fails"
    );
}

#[test]
fn add_accepts_unknown_hook_name_with_force() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    // -f short flag
    run_krok(repo, &["add", "-f", "custom-experimental-hook", "echo hi"]);

    let wrapper = repo
        .join(".git")
        .join("hooks")
        .join("custom-experimental-hook");
    assert!(
        wrapper.exists(),
        "wrapper should exist for custom hook with --force"
    );
    let config = std::fs::read_to_string(repo.join(".git").join("krok-config.yml"))
        .expect("read config");
    assert!(
        config.contains("custom-experimental-hook"),
        "config should contain custom hook entry: {config}"
    );
}

#[test]
fn recover_rejects_unknown_hook_name_by_default() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let output = Command::new(krok_bin())
        .args(["recover", "pre-comit"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");
    assert!(
        !output.status.success(),
        "recover should reject unknown hook name without --force"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a known git hook name"),
        "expected validation error, got stderr: {stderr}"
    );
}

#[test]
fn recover_accepts_unknown_hook_name_with_force() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    // Bootstrap: install a custom hook via add --force, so config has an entry
    run_krok(repo, &["add", "--force", "custom-experimental-hook", "echo hi"]);

    // Delete the wrapper, then recover with --force (config entry exists)
    let wrapper = repo
        .join(".git")
        .join("hooks")
        .join("custom-experimental-hook");
    std::fs::remove_file(&wrapper).expect("remove wrapper");

    run_krok(
        repo,
        &["recover", "--force", "custom-experimental-hook"],
    );

    assert!(
        wrapper.exists(),
        "wrapper should have been restored by recover --force"
    );
}
