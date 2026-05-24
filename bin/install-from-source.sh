#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="${HOME}/bin"

cd "$PROJECT_ROOT"

echo "==> Building release..."
cargo build --release

mkdir -p "$INSTALL_DIR"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    echo "==> Installing to ${INSTALL_DIR}/krok..."
    cp target/release/krok.exe "$INSTALL_DIR/krok"
    chmod +x "$INSTALL_DIR/krok"
    echo "Done. krok installed at ${INSTALL_DIR}/krok"
    ;;
  *)
    echo "==> Installing to ${INSTALL_DIR}/krok..."
    cp target/release/krok "$INSTALL_DIR/krok"
    chmod +x "$INSTALL_DIR/krok"
    echo "Done. krok installed at ${INSTALL_DIR}/krok"
    ;;
esac

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
  echo ""
  echo "Note: ${INSTALL_DIR} is not in your PATH."
  echo "Add the following line to your shell profile:"
  echo "  export PATH=\"\${HOME}/bin:\$PATH\""
fi
