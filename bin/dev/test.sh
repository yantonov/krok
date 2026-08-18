#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

echo "==> Running tests..."
cargo test

echo ""
echo "==> Checking formatting..."
cargo fmt --check

echo ""
echo "==> Running clippy..."
cargo clippy -- -D warnings

echo ""
echo "All checks passed."
