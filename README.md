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
  - [config path](#config-path)
- [Configuration file](#configuration-file)
- [Run mode](#run-mode)
- [Error handling](#error-handling)
- [Inspired by](#inspired-by)

---

## How it works

The first time you run `krok add <hook-name> <cmd>`, krok writes a small sh wrapper to `.git/hooks/<hook-name>`. When git fires the hook, the wrapper invokes `krok run <hook-name> "$@"`, which executes every job registered for that hook in order.

Subsequent `krok add` calls for the same hook just append to the job list — the wrapper is only installed once.

Jobs are stored in `.git/krok-config.yml`, which you can inspect or edit directly.

---

## Installation

On Windows, krok runs under the sh that git ships, so it expects Git Bash.

### Automated installation

Downloads the latest release and installs it to `~/.local/bin` in one step. Requires `curl`.

```sh
curl -fsSL https://raw.githubusercontent.com/yantonov/krok/master/bin/install/install.sh | sh
```

### Download prebuilt binary

If you don't like to use curl + bash approach due to security reasons, for example, you can download prebuilt binary.

1. Go to the [Releases](https://github.com/yantonov/krok/releases) page.
2. Download the archive for your platform.
3. Extract and place the binary somewhere on your `$PATH`:

```sh
tar -xzf <archive>.tar.gz
mkdir -p ~/.local/bin
cp krok ~/.local/bin/krok
chmod +x ~/.local/bin/krok
```

### Build from source

**Prerequisites:** Rust toolchain (`cargo`).

```sh
git clone https://github.com/yantonov/krok.git
cd krok
bin/install/install-from-source.sh
```

This builds a release binary and copies it to `~/.local/bin/krok`.

---

Verify the installation:

```sh
krok --version
```

---

## Commands

### add

```sh
krok add [--force|-f] <hook-name> <command> [args...]
```

Appends a new job to the named hook's job list. On the first `add` for a hook, krok also installs the wrapper script at `.git/hooks/<hook-name>`; subsequent calls only update `.git/krok-config.yml`.

- `<hook-name>` is validated against the list of built-in git hook names from [`githooks(5)`](https://git-scm.com/docs/githooks). Pass `--force` (`-f`) to skip the check — useful when working with a git fork or with a hook newer than the krok release you have installed.
- The job key is derived from the command (ASCII alphanumeric characters, spaces replaced with `-`).
- Registering a command already registered for that hook changes nothing: krok says so, leaves the config as it is, and succeeds, so a script that bootstraps a checkout can be re-run. A different command that happens to derive the same key is numbered rather than refused.
- If a non-krok hook script already exists at `.git/hooks/<hook-name>`, it is preserved at `.git/krok/<hook-name>/existing` and registered as the first job so it continues to run. It is kept under the git directory rather than beside the hook, because `core.hooksPath` may be repointed later — husky does this from a build step — which would leave a copy under the hooks directory behind.

**Examples:**

```sh
krok add pre-commit cargo test
krok add pre-commit cargo clippy -- -D warnings
krok add commit-msg ./scripts/check-message.sh
krok add --force custom-experimental-hook ./scripts/custom.sh
```

### run

```sh
krok run <hook-name> [hook-args...]
```

Invoked by the wrapper script that git executes — you normally do not call this yourself. It loads the job list for `<hook-name>` from `.git/krok-config.yml` and runs each command sequentially, forwarding any arguments git passed to the hook.

### recover

```sh
krok recover [--force|-f] <hook-name>
```

Restores the wrapper script at `.git/hooks/<hook-name>` when it has drifted from what `krok` expects. `<hook-name>` is validated the same way as in [`add`](#add); use `--force` (`-f`) to skip the check. Use this after another tool overwrites the hook, after you upgrade `krok` and want to bring the wrapper in sync, or after the file has been deleted.

The hook must already have a config entry (i.e. you must have previously run `krok add <hook-name> ...`); otherwise `recover` errors out. Behavior based on the current state of the wrapper file:

| Current state | Action | Message |
|---|---|---|
| Matches the canonical wrapper | nothing | `hook '<name>' is up to date` |
| File missing | write the wrapper | `wrote wrapper for '<name>'` |
| Older / modified krok wrapper | overwrite | `replaced outdated krok wrapper for '<name>'` |
| A foreign (non-krok) script | preserve it to `.git/krok/<hook>/existing` and register as a job, then write the wrapper | `preserved foreign hook and wrote krok wrapper for '<name>'` |

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

### config path

```sh
krok config path
```

Prints the absolute path to `.git/krok-config.yml`. Must be run from the repository root. Errors out if no config file exists — use `krok add` first.

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

| Field | Description                     |
|-------|---------------------------------|
| `key` | Unique identifier within the hook |
| `cmd` | Shell command passed to `sh -c` |

You can edit this file directly to reorder jobs, change commands, or remove entries.

Three variables are exported to every job:

| Variable         | Value                                          |
|------------------|------------------------------------------------|
| `KROK_REPO_ROOT` | Top level of the working tree                  |
| `KROK_HOOKS_DIR` | Where hook scripts live, honouring `core.hooksPath` |
| `KROK_GIT_DIR`   | The git directory shared by every worktree     |

---

## Run mode

When git fires a hook, the wrapper at `.git/hooks/<hook-name>` invokes `krok run <hook-name> "$@"`, forwarding any arguments git passed. `krok` then reads `.git/krok-config.yml` and executes each job in order via:

```sh
sh -c '<cmd> "$@"' <hook-name> <arguments git passed>
```

The arguments reach the job as positional parameters, so it can read them individually as `$1`, `$2`.

Each job starts at the repository root, the directory git fires hooks from, whichever directory `krok run` was invoked from. Output from each job is forwarded directly to the terminal.

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
