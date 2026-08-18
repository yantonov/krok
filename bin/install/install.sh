#!/usr/bin/env sh

set -eu

REPO="yantonov/krok"

# One release provides both the binary and the script that installs it. Taken
# from master instead, the script would be whatever landed there a minute ago,
# and could be paired with a binary it has never seen.
# Set KROK_VERSION to install a specific release.
VERSION="${KROK_VERSION:-$(
  curl -fsSLo /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" \
  | sed 's#.*/tag/##'
)}"

# With nothing published, the redirect ends at the releases page instead of at a
# tag, and what sed leaves behind is a url rather than a version.
case "${VERSION}" in
  ''|*/*)
    echo "Cannot detect the latest published release of ${REPO}"
    exit 1
    ;;
esac

SCRIPTS="https://raw.githubusercontent.com/${REPO}/${VERSION}/bin/install"

echo "Installing krok from release ${VERSION}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

# Fetched to a file rather than piped into a shell: in 'curl | sh' the exit code
# belongs to the shell, and an empty input is a script that succeeds, so a script
# that never arrived would go unnoticed and fail later as something unrelated.
if ! curl -fsSL "${SCRIPTS}/download.sh" -o "${TMP_DIR}/download.sh"; then
  echo "Cannot fetch ${SCRIPTS}/download.sh"
  echo "Release ${VERSION} may not carry the installer scripts yet; set KROK_VERSION to a release that does"
  exit 1
fi

sh "${TMP_DIR}/download.sh" "${VERSION}"
