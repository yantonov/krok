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

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to execute git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_init_with_a_commit(cwd: &Path) {
    git_init(cwd);
    git(
        cwd,
        &[
            "-c",
            "user.email=krok@example.invalid",
            "-c",
            "user.name=krok",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
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

    let has_echo_job = jobs
        .iter()
        .any(|job| job.get("cmd").and_then(|c| c.as_str()) == Some("echo hello"));
    assert!(
        has_echo_job,
        "expected 'echo hello' job in config: {content}"
    );
}

fn fwd_slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn write_script(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create script directory");
    }
    std::fs::write(path, format!("#!/usr/bin/env sh\n{body}\n")).expect("write script");

    // Git bash takes the leading '#!' as the executable bit, so this only has
    // to be done where the bit is real.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .expect("script metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("set script permissions");
    }
}

#[test]
fn run_executes_jobs_in_registered_order() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let log = repo.join("order.log");
    let log_str = fwd_slash(&log);

    run_krok(
        repo,
        &["add", "pre-commit", &format!("echo first >> {log_str}")],
    );
    run_krok(
        repo,
        &["add", "pre-commit", &format!("echo second >> {log_str}")],
    );
    run_krok(
        repo,
        &["add", "pre-commit", &format!("echo third >> {log_str}")],
    );

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
    run_krok(
        repo,
        &["add", "pre-commit", &format!("echo done > {marker_str}")],
    );

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

    let content =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
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
fn two_commands_that_derive_one_key_both_register() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(
        repo,
        &[
            "add",
            "pre-commit",
            "cargo",
            "clippy",
            "--",
            "-D",
            "warnings",
        ],
    );
    run_krok(
        repo,
        &["add", "pre-commit", "cargo", "clippy", "-D", "warnings"],
    );

    let config =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&config).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("hooks.pre-commit must be a sequence");

    assert_eq!(
        jobs.len(),
        2,
        "both commands should be registered: {config}"
    );
    let keys: Vec<&str> = jobs
        .iter()
        .filter_map(|job| job.get("key").and_then(|k| k.as_str()))
        .collect();
    assert_ne!(keys[0], keys[1], "the two jobs share a key: {config}");
}

// The script that bootstraps a checkout registers the same job every time it
// runs, under `set -e`. Asking for one already registered has to be an answer,
// not a failure.
#[test]
fn add_of_a_job_already_registered_changes_nothing() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo same"]);
    let after_first =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");

    let output = Command::new(krok_bin())
        .args(["add", "pre-commit", "echo same"])
        .current_dir(repo)
        .output()
        .expect("failed to execute krok");

    assert!(
        output.status.success(),
        "registering a job twice failed; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("already registered"),
        "expected to be told the job was already there, got stdout: {stdout}"
    );

    let after_second =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
    assert_eq!(
        after_second, after_first,
        "the second add wrote to the config"
    );
}

// Re-running that script is also how a wrapper someone deleted comes back, so
// answering that the job is already registered may not come first.
#[test]
fn add_of_a_job_already_registered_still_writes_the_wrapper_back() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo same"]);
    let wrapper = repo.join(".git").join("hooks").join("pre-commit");
    std::fs::remove_file(&wrapper).expect("remove the wrapper");

    run_krok(repo, &["add", "pre-commit", "echo same"]);

    assert!(wrapper.exists(), "the wrapper was not written back");
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

    let preserved = repo
        .join(".git")
        .join("krok")
        .join("pre-commit")
        .join("existing");
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

    let config =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
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
    run_krok(
        repo,
        &["add", "pre-commit", &format!("echo > {captured_str}")],
    );

    run_krok(repo, &["run", "pre-commit", "passed-arg"]);

    let content = std::fs::read_to_string(&captured).expect("read captured file");
    assert!(
        content.contains("passed-arg"),
        "hook arg not forwarded to job: {content}"
    );
}

#[test]
fn run_executes_a_script_of_the_repository() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    // The form the readme suggests: a path relative to the repository root,
    // which is the directory git fires hooks from.
    let marker = repo.join("script-ran.txt");
    write_script(
        &repo.join("scripts").join("check.sh"),
        &format!("echo ran > {}", fwd_slash(&marker)),
    );

    run_krok(repo, &["add", "commit-msg", "./scripts/check.sh"]);
    run_krok(repo, &["run", "commit-msg"]);

    assert!(
        marker.exists(),
        "a job holding a path relative to the repository root did not run"
    );
}

#[test]
fn run_starts_jobs_at_the_repository_root() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "pre-commit", "echo here > from-job.txt"]);

    let subdir = repo.join("deep").join("sub");
    std::fs::create_dir_all(&subdir).expect("create subdir");
    run_krok(&subdir, &["run", "pre-commit"]);

    assert!(
        repo.join("from-job.txt").exists(),
        "the job did not start at the repository root"
    );
    assert!(
        !subdir.join("from-job.txt").exists(),
        "the job started in the directory krok was invoked from"
    );
}

#[test]
fn run_exports_the_repository_root_and_the_hooks_directory() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    // The job's own exit code carries the assertion: run_krok requires success.
    run_krok(
        repo,
        &[
            "add",
            "pre-commit",
            "test -d \"$KROK_REPO_ROOT/.git\" && test -f \"$KROK_HOOKS_DIR/pre-commit\"",
        ],
    );
    run_krok(repo, &["run", "pre-commit"]);
}

#[test]
fn preserved_foreign_hook_runs() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let marker = repo.join("foreign-ran.txt");
    write_script(
        &repo.join(".git").join("hooks").join("pre-commit"),
        &format!("echo ran > {}", fwd_slash(&marker)),
    );

    run_krok(repo, &["add", "pre-commit", "true"]);
    run_krok(repo, &["run", "pre-commit"]);

    assert!(
        marker.exists(),
        "the hook krok took over from was registered but never ran"
    );
}

// The command registered for a preserved hook is quoted, and a quoted command
// is what windows hands to sh least reliably. A repository whose path holds a
// space is the case that needs the quotes in the first place.
#[test]
fn preserved_foreign_hook_runs_from_a_path_holding_a_space() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path().join("my repo dir");
    std::fs::create_dir_all(&repo).expect("create repo directory");
    git_init(&repo);

    let marker = repo.join("foreign-ran.txt");
    write_script(
        &repo.join(".git").join("hooks").join("pre-commit"),
        &format!("echo ran > \"{}\"", fwd_slash(&marker)),
    );

    run_krok(&repo, &["add", "pre-commit", "true"]);
    run_krok(&repo, &["run", "pre-commit"]);

    assert!(
        marker.exists(),
        "the preserved hook did not run from a path holding a space"
    );
}

#[test]
fn preserved_foreign_hook_of_an_older_config_runs() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let marker = repo.join("legacy-ran.txt");
    write_script(
        &repo.join(".git").join("hooks").join("pre-commit"),
        &format!("echo ran > {}", fwd_slash(&marker)),
    );
    run_krok(repo, &["add", "pre-commit", "true"]);

    // Back into the shape krok used to leave: the file under the hooks
    // directory, named by a bare path read against it.
    let legacy_dir = repo.join(".git").join("hooks").join("pre-commit-hooks");
    std::fs::create_dir_all(&legacy_dir).expect("create legacy directory");
    std::fs::rename(
        repo.join(".git")
            .join("krok")
            .join("pre-commit")
            .join("existing"),
        legacy_dir.join("existing-pre-commit"),
    )
    .expect("move the preserved hook to where an older krok left it");

    let config_path = repo.join(".git").join("krok-config.yml");
    let config = std::fs::read_to_string(&config_path).expect("read config");
    let legacy = config.replace(
        "\"$KROK_GIT_DIR/krok/pre-commit/existing\"",
        "pre-commit-hooks/existing-pre-commit",
    );
    assert_ne!(legacy, config, "the config to rewrite was not found");
    std::fs::write(&config_path, legacy).expect("write legacy config");

    run_krok(repo, &["run", "pre-commit"]);

    assert!(
        marker.exists(),
        "a config written by an earlier krok stopped working"
    );
}

// The case this all turns on. Husky runs `husky install` from a build step and
// sets core.hooksPath, on every fresh checkout, which is to say after krok
// installed. A preserved hook kept under the hooks directory is left behind by
// that, and the job naming it stops resolving.
#[test]
fn a_preserved_hook_outlives_core_hooks_path_moving() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let marker = repo.join("foreign-ran.txt");
    write_script(
        &repo.join(".git").join("hooks").join("pre-commit"),
        &format!("echo ran > {}", fwd_slash(&marker)),
    );
    run_krok(repo, &["add", "pre-commit", "true"]);

    git(repo, &["config", "core.hooksPath", "my-hooks"]);

    run_krok(repo, &["run", "pre-commit"]);

    assert!(
        marker.exists(),
        "the preserved hook was lost when core.hooksPath moved"
    );
}

#[test]
fn run_forwards_a_hook_argument_holding_a_space_whole() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    let seen = repo.join("seen.txt");
    write_script(&repo.join("show.sh"), "printf '[%s]\\n' \"$@\"");
    run_krok(
        repo,
        &[
            "add",
            "commit-msg",
            &format!("./show.sh > {}", fwd_slash(&seen)),
        ],
    );

    run_krok(repo, &["run", "commit-msg", "my file.txt"]);

    let content = std::fs::read_to_string(&seen).expect("read seen file");
    assert_eq!(
        content.lines().collect::<Vec<_>>(),
        vec!["[my file.txt]"],
        "the argument did not arrive whole: {content}"
    );
}

#[test]
fn a_hook_argument_is_not_read_as_shell_syntax() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);

    run_krok(repo, &["add", "commit-msg", "true"]);

    run_krok(repo, &["run", "commit-msg", "; echo pwned > pwned.txt"]);

    assert!(
        !repo.join("pwned.txt").exists(),
        "a hook argument was read as shell syntax"
    );
}

#[test]
fn add_from_a_linked_worktree_reaches_the_shared_git_directory() {
    let tmp = TempDir::new().expect("tempdir");
    let main = tmp.path().join("main");
    std::fs::create_dir_all(&main).expect("create main checkout");
    git_init_with_a_commit(&main);

    let worktree = tmp.path().join("wt");
    git(
        &main,
        &["worktree", "add", "-q", &worktree.to_string_lossy()],
    );

    run_krok(
        &worktree,
        &["add", "pre-commit", "echo here > from-job.txt"],
    );

    // Worktrees share one hooks directory and one config, both belonging to the
    // git directory of the main checkout.
    assert!(
        main.join(".git").join("hooks").join("pre-commit").exists(),
        "the wrapper was not written to the shared hooks directory"
    );
    assert!(
        main.join(".git").join("krok-config.yml").exists(),
        "the config was not written to the shared git directory"
    );

    run_krok(&worktree, &["run", "pre-commit"]);

    assert!(
        worktree.join("from-job.txt").exists(),
        "the job did not start at the top level of the worktree"
    );
}

#[test]
fn add_inside_a_submodule_reaches_the_git_directory_of_the_module() {
    let tmp = TempDir::new().expect("tempdir");

    let library = tmp.path().join("library");
    std::fs::create_dir_all(&library).expect("create library");
    git_init_with_a_commit(&library);

    let superproject = tmp.path().join("superproject");
    std::fs::create_dir_all(&superproject).expect("create superproject");
    git_init_with_a_commit(&superproject);
    git(
        &superproject,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            &library.to_string_lossy(),
            "vendor/library",
        ],
    );

    let submodule = superproject.join("vendor").join("library");
    run_krok(&submodule, &["add", "pre-commit", "echo hi"]);

    let module_git_dir = superproject
        .join(".git")
        .join("modules")
        .join("vendor")
        .join("library");
    assert!(
        module_git_dir.join("hooks").join("pre-commit").exists(),
        "the wrapper was not written to the git directory of the submodule"
    );
    assert!(
        module_git_dir.join("krok-config.yml").exists(),
        "the config was not written to the git directory of the submodule"
    );
}

#[test]
fn add_honours_core_hooks_path() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();
    git_init(repo);
    git(repo, &["config", "core.hooksPath", "my-hooks"]);

    run_krok(repo, &["add", "pre-commit", "echo hi"]);

    assert!(
        repo.join("my-hooks").join("pre-commit").exists(),
        "the wrapper ignored core.hooksPath"
    );
    assert!(
        !repo.join(".git").join("hooks").join("pre-commit").exists(),
        "the wrapper was written to the default hooks directory as well"
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
    let drifted =
        "#!/usr/bin/env bash\n# git hook manager wrapper (old)\nexec krok run pre-commit \"$@\"\n";
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
        .join("krok")
        .join("pre-commit")
        .join("existing");
    assert!(preserved.exists(), "preserved file missing");
    let preserved_content = std::fs::read_to_string(&preserved).expect("read preserved");
    assert_eq!(preserved_content, foreign, "preserved content mismatch");

    let after_wrapper = std::fs::read_to_string(&wrapper).expect("read wrapper");
    assert!(
        after_wrapper.contains("exec krok run pre-commit"),
        "wrapper not restored to krok form: {after_wrapper}"
    );

    let config =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
    let value: serde_yaml::Value = serde_yaml::from_str(&config).expect("parse yaml");
    let jobs = value
        .get("hooks")
        .and_then(|h| h.get("pre-commit"))
        .and_then(|j| j.as_sequence())
        .expect("hooks.pre-commit must be a sequence");
    assert!(
        jobs.iter()
            .any(|j| { j.get("key").and_then(|k| k.as_str()) == Some("existing-hook") }),
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
    let printed = Path::new(stdout.trim_end());
    let expected = repo.join(".git").join("krok-config.yml");

    // Compared as the file system resolves them, not as text. One directory can
    // be spelled in more than one way: on macos a temporary directory is handed
    // out below /var, which is a symlink to /private/var, and the getcwd of the
    // process krok runs in answers with the second. Canonicalising both also
    // asks that what was printed exists, which is the point of the command.
    assert_eq!(
        std::fs::canonicalize(printed).expect("the printed path names a file"),
        std::fs::canonicalize(&expected).expect("the expected path names a file"),
        "config path printed {} rather than {}",
        printed.display(),
        expected.display()
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
    let config =
        std::fs::read_to_string(repo.join(".git").join("krok-config.yml")).expect("read config");
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
    run_krok(
        repo,
        &["add", "--force", "custom-experimental-hook", "echo hi"],
    );

    // Delete the wrapper, then recover with --force (config entry exists)
    let wrapper = repo
        .join(".git")
        .join("hooks")
        .join("custom-experimental-hook");
    std::fs::remove_file(&wrapper).expect("remove wrapper");

    run_krok(repo, &["recover", "--force", "custom-experimental-hook"]);

    assert!(
        wrapper.exists(),
        "wrapper should have been restored by recover --force"
    );
}
