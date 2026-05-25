# krok — Git Hook Manager

`krok` is a lightweight command-line tool that lets you attach multiple commands to any git hook without maintaining shell scripts. Install it once per hook, register jobs with a single command, and `krok` runs them sequentially every time the hook fires.

## Table of Contents

- [How it works](#how-it-works)
- [Installation](#installation)
  - [Automated installation](#automated-installation)
  - [Download prebuilt binary](#download-prebuilt-binary)
  - [Build from source](#build-from-source)
- [Commands](#commands)
  - [add](#add)
  - [run](#run)
  - [recover](#recover)
  - [config show](#config-show)
  - [config edit](#config-edit)
- [Configuration file](#configuration-file)
- [Run mode](#run-mode)
- [Error handling](#error-handling)
- [Inspired by](#inspired-by)

---

## How it works

The first time you run `krok add <hook-name> <cmd>`, krok writes a small bash wrapper to `.git/hooks/<hook-name>`. When git fires the hook, the wrapper invokes `krok run <hook-name> "$@"`, which executes every job registered for that hook in order.

Subsequent `krok add` calls for the same hook just append to the job list — the wrapper is only installed once.

Jobs are stored in `.git/krok-config.yml`, which you can inspect or edit directly.

---

## Installation

### Automated installation

Downloads the latest release and installs it to `~/bin` in one step. Requires `curl` and `jq`.

```sh
curl -fsSL https://raw.githubusercontent.com/yantonov/git-hook-runner/master/bin/download.sh | sh
```

### Download prebuilt binary

1. Go to the [Releases](https://github.com/yantonov/git-hook-runner/releases) page.
2. Download the archive for your platform:
   - `krok-linux-<version>.tar.gz` — Linux
   - `krok-macos-<version>.tar.gz` — macOS
   - `krok-windows-<version>.tar.gz` — Windows (Git Bash)
3. Extract and place the binary somewhere on your `$PATH`:

```sh
tar -xzf krok-<os>-<version>.tar.gz
mkdir -p ~/bin
cp krok ~/bin/krok
chmod +x ~/bin/krok
```

### Build from source

**Prerequisites:** Rust toolchain (`cargo`).

```sh
git clone https://github.com/yantonov/git-hook-runner.git
cd git-hook-runner
bin/install-from-source.sh
```

This builds a release binary and copies it to `~/bin/krok`.

---

Verify the installation:

```sh
krok --version
```

---

## Commands

### add

```sh
krok add <hook-name> <command> [args...]
```

Appends a new job to the named hook's job list. On the first `add` for a hook, krok also installs the wrapper script at `.git/hooks/<hook-name>`; subsequent calls only update `.git/krok-config.yml`.

- The job key is derived from the command (ASCII alphanumeric characters, spaces replaced with `-`).
- Returns an error if a job with the same key already exists for that hook.
- If a non-krok hook script already exists at `.git/hooks/<hook-name>`, it is preserved at `.git/hooks/<hook-name>-hooks/existing-<hook-name>` and registered as the first job so it continues to run.

**Examples:**

```sh
krok add pre-commit cargo test
krok add pre-commit cargo clippy -- -D warnings
krok add commit-msg ./scripts/check-message.sh
```

### run

```sh
krok run <hook-name> [hook-args...]
```

Invoked by the wrapper script that git executes — you normally do not call this yourself. It loads the job list for `<hook-name>` from `.git/krok-config.yml` and runs each command sequentially, forwarding any arguments git passed to the hook.

### recover

```sh
krok recover <hook-name>
```

Restores the wrapper script at `.git/hooks/<hook-name>` when it has drifted from what `krok` expects. Use this after another tool overwrites the hook, after you upgrade `krok` and want to bring the wrapper in sync, or after the file has been deleted.

The hook must already have a config entry (i.e. you must have previously run `krok add <hook-name> ...`); otherwise `recover` errors out. Behavior based on the current state of the wrapper file:

| Current state | Action | Message |
|---|---|---|
| Matches the canonical wrapper | nothing | `hook '<name>' is up to date` |
| File missing | write the wrapper | `wrote wrapper for '<name>'` |
| Older / modified krok wrapper | overwrite | `replaced outdated krok wrapper for '<name>'` |
| A foreign (non-krok) script | preserve it to `<hook>-hooks/existing-<hook>` and register as a job, then write the wrapper | `preserved foreign hook and wrote krok wrapper for '<name>'` |

### config show

```sh
krok config show
```

Prints the contents of `.git/krok-config.yml` to stdout. Must be run from the repository root. Errors out if no config file exists.

### config edit

```sh
krok config edit
```

Opens `.git/krok-config.yml` in the editor reported by `git var GIT_EDITOR` (which respects `$GIT_EDITOR`, `core.editor`, `$VISUAL`, `$EDITOR`, in that order). Must be run from the repository root. Errors out if no config file exists — use `krok add` first.

---

## Configuration file

Jobs are stored in `.git/krok-config.yml`:

```yaml
hooks:
  pre-commit:
  - key: cargo-test
    cmd: cargo test
  - key: cargo-clippy-D-warnings
    cmd: cargo clippy -- -D warnings
```

| Field | Description                          |
|-------|--------------------------------------|
| `key` | Unique identifier within the hook    |
| `cmd` | Shell command passed to `$SHELL -c`  |

You can edit this file directly to reorder jobs, change commands, or remove entries.

---

## Run mode

When git fires a hook, the wrapper at `.git/hooks/<hook-name>` invokes `krok run <hook-name> "$@"`, forwarding any arguments git passed. `krok` then reads `.git/krok-config.yml` and executes each job in order via:

```sh
$SHELL -c "<cmd>"
```

Output from each job is forwarded directly to the terminal.

---

## Error handling

Jobs run **sequentially**. If any job exits with a non-zero code, `krok` stops immediately and prints:

```
[krok] hook 'pre-commit' failed at job 'cargo-test' (cmd: cargo test)
```

The hook itself exits with the same non-zero code, which causes git to abort the operation.

---

## Inspired by

`krok` is inspired by — and best understood in contrast with — these existing git hook managers:

1. [hk](https://github.com/jdx/hk) [rust] written in rust (+ package manager is worth checking)
2. [On git hook managers](https://salotz.info/posts/on-git-hook-managers/) — overview post on the design space
3. [pre-commit](https://pre-commit.com/) — [python] feature-rich
4. [autohook](https://github.com/Autohook/Autohook) — nice idea, not intuitive
5. [lefthook](https://github.com/evilmartians/lefthook) — [go] extensive config options

> **Note:** The goal of `krok` is to be the simplest possible git hook manager with a minimalistic config. Where the tools above offer rich configuration, plugin ecosystems, or DSLs, `krok` deliberately stops at *"run these commands in order when this hook fires"* — nothing more.
