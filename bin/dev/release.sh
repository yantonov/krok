#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    EXECUTABLE="krok.exe"
    ;;
  *)
    EXECUTABLE="krok"
    ;;
esac

echo "==> Building (release)..."
cargo build --release

echo "Binary: ${PROJECT_ROOT}/target/release/${EXECUTABLE}"
