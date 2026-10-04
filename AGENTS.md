# AGENTS.md

## Project
`krok` — Git Hook Manager. Rust (edition 2024, MSRV 1.85), CLI tool with clap + serde.
Attaches multiple commands to any git hook without shell scripts.

## Commands
- Build: `make build`
- Tests: `make test`
- Lint: `make lint`
- Format: `make fmt`
- Full verification: `make check`   # lint + fmt-check + test

## Hard Constraints (MUST)
- Hook names are validated against `githooks(5)` list; `--force` bypasses it for non-standard hooks.
- Jobs run sequentially in order; first failure stops the hook immediately.
- Non-krok hook scripts at `.git/hooks/<name>` are preserved to `.git/krok/<hook>/existing` and registered as the first job.
- Config lives at `.git/krok-config.yml`. Never write config anywhere else.
- Wrapper always installs to `.git/hooks/<hook-name>`, ignoring `core.hooksPath`.
- Three env vars exported to every job: `KROK_REPO_ROOT`, `KROK_HOOKS_DIR`, `KROK_GIT_DIR`.
- `--version` prints the binary version + git hash (embedded at build time via `build.rs`). Never shell out to git for it.

## Definition of Done
Feature is done = `make check` is green + integration tests in `tests/cli.rs` cover the new behavior.
"Code compiles" is not done.

## Working Rules
- One feature at a time. Do not start a second until the first passes full verification.
- No drive-by refactoring while a feature is in progress.
- Update `PROGRESS.md` before ending a session.

## Where to Look for Details
- `src/cli.rs` — CLI argument parsing (clap derive)
- `src/commands/` — one module per subcommand (add, run, recover, config)
- `src/config.rs` — YAML config read/write
- `src/wrapper.rs` — hook wrapper script generation
- `src/git.rs` — git directory detection
- `src/hooks.rs` — known hook names and validation
- `src/shell.rs` — command execution
- `src/env.rs` — environment variable handling
- `tests/cli.rs` — integration tests
- `build.rs` — embeds git hash at compile time
- `README.md` — user-facing documentation
