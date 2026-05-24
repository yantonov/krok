# krok — Git Hook Manager

`krok` is a lightweight command-line tool that lets you attach multiple commands to any git hook without maintaining shell scripts. Install it once per hook, register jobs with a single command, and `krok` runs them sequentially every time the hook fires.

## Table of Contents

- [How it works](#how-it-works)
- [Installation](#installation)
  - [Automated installation](#automated-installation)
  - [Download prebuilt binary](#download-prebuilt-binary)
  - [Build from source](#build-from-source)
- [Commands](#commands)
  - [install](#install)
  - [add](#add)
- [Configuration file](#configuration-file)
- [Run mode](#run-mode)
- [Error handling](#error-handling)

---

## How it works

When you run `krok install <hook-name>`, the binary copies itself into `.git/hooks/<hook-name>`. Git then calls that binary when the hook fires. Because the executable name matches the hook name, `krok` detects it is in run mode and executes all registered jobs in order.

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

### install

```sh
krok install <hook-name>
```

Must be run from the **repository root** (the directory containing `.git`).

- Creates `.git/krok-config.yml` if it does not exist.
- If a hook script already exists at `.git/hooks/<hook-name>`, it is moved to `.git/hooks/<hook-name>-hooks/existing-<hook-name>` and registered as the first job so it continues to run.
- Copies the `krok` binary to `.git/hooks/<hook-name>`.
- Adds a default `hello` job (`echo "test hook"`) as a smoke-test placeholder.

**Example:**

```sh
krok install pre-commit
```

### add

```sh
krok add <hook-name> <command> [args...]
```

Appends a new job to the named hook's job list. The hook must already be installed.

- The job key is derived from the command (ASCII alphanumeric characters, spaces replaced with `-`).
- Returns an error if a job with the same key already exists.

**Examples:**

```sh
krok add pre-commit cargo test
krok add pre-commit cargo clippy -- -D warnings
krok add commit-msg ./scripts/check-message.sh
```

---

## Configuration file

Jobs are stored in `.git/krok-config.yml`:

```yaml
hooks:
  pre-commit:
  - key: hello
    title: test hook
    cmd: echo "test hook"
  - key: cargo-test
    title: cargo
    cmd: cargo test
  - key: cargo-clippy-D-warnings
    title: cargo
    cmd: cargo clippy -- -D warnings
```

| Field   | Description                                      |
|---------|--------------------------------------------------|
| `key`   | Unique identifier within the hook                |
| `title` | Human-readable label shown during execution      |
| `cmd`   | Shell command passed to `$SHELL -c`              |

You can edit this file directly to reorder jobs, change commands, or remove entries.

---

## Run mode

When git fires a hook, it calls the `krok` binary installed at `.git/hooks/<hook-name>`. `krok` detects it has no subcommand arguments, determines the hook name from its own executable filename, and runs each job via:

```sh
$SHELL -c "<cmd>"
```

Output from each job is forwarded directly to the terminal.

---

## Error handling

Jobs run **sequentially**. If any job exits with a non-zero code, `krok` stops immediately and prints:

```
[krok] hook 'pre-commit' failed at job 'cargo-test' (title: cargo, cmd: cargo test)
```

The hook itself exits with the same non-zero code, which causes git to abort the operation.
