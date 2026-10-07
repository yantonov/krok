# AGENTS.md

`krok` — git hook manager: attaches multiple commands to any git hook, no shell scripts.

## Invariants
- Hook names are validated against the `githooks(5)` list; `--force` admits non-standard hooks.
- Jobs run sequentially in order; the first failure stops the hook.
- A non-krok script at `.git/hooks/<name>` moves to `.git/krok/<hook>/existing` and becomes the first job.
- Config lives only at `.git/krok-config.yml`.
- The wrapper installs to `.git/hooks/<hook-name>`, whatever `core.hooksPath` says.
- Every job gets `KROK_REPO_ROOT`, `KROK_HOOKS_DIR`, `KROK_GIT_DIR`.
- `--version` prints the binary version + the git hash embedded by `build.rs` at build time.

## Done
`make check` is green and `tests/cli.rs` covers the new behavior. A compiling change is unfinished.

## Working Rules
- One feature at a time; start the next only after `make check` passes.
- Touch only what the feature needs.
- Update `PROGRESS.md` before ending a session.
