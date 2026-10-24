# AGENTS.md

## Project
`krok` — git hook manager: attaches multiple commands to any git hook, no shell scripts.
Stack: Rust (edition 2024, MSRV 1.85), Cargo, no framework or database.

## Commands
Run `make help` for the full target list. The one you'll use most: **`make check`** (lint + fmt-check + test).

## Starting a session
Read `PROGRESS.md` before starting. Update it before ending the session.

## Definition of Done
`make check` is green and `tests/cli.rs` covers the new behavior. A compiling change is unfinished.

## Hard constraints (MUST / MUST NOT)
1. Hook names are validated against the `githooks(5)` list; `--force` admits non-standard hooks.
2. Jobs run sequentially in order; the first failure stops the hook.
3. A non-krok script at `.git/hooks/<name>` moves to `.git/krok/<hook>/existing` and becomes the first job.
4. Config lives only at `.git/krok-config.yml`.
5. The wrapper installs to `.git/hooks/<hook-name>`, whatever `core.hooksPath` says.
6. Every job gets `KROK_REPO_ROOT`, `KROK_HOOKS_DIR`, `KROK_GIT_DIR`.
7. `--version` prints the binary version + the git hash embedded by `build.rs` at build time.

## Working Rules
- One feature at a time; start the next only after `make check` passes.
- Touch only what the feature needs.

## Escalate to human (stop and ask)
- Changing the public API or CLI interface.
- Adding a new dependency.
- Changing how the wrapper script is generated (breaks existing installs).
- Deleting anything that cannot be recovered.
