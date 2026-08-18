#!/usr/bin/env sh

set -eu

# Detect OS
case "$(uname -s)" in
  Linux*)
    OS="linux"
    ;;
  Darwin*)
    OS="macos"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    OS="windows"
    ;;
  *)
    echo "Unsupported OS: $(uname -s)"
    exit 1
    ;;
esac

REPO="yantonov/krok"

# A release may be named, so that a caller which has already settled on one can
# say so and have the binary come from that one rather than from whichever is
# newest by the time this runs.
RELEASE="${1:-}"

# Otherwise the version comes from the latest published release rather than from
# the tag list. A tag exists the moment it is pushed, while the release built
# from it stays a draft until someone publishes it, so the newest tag readily
# names assets that cannot be downloaded yet. Following the redirect of the
# 'latest release' page also keeps this clear of a json parser and of the
# unauthenticated api rate limit.
if [ -z "${RELEASE}" ]; then
  RELEASE="$(
    curl -fsSLo /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" \
    | sed 's#.*/tag/##'
  )"
fi

# With nothing published, the redirect ends at the releases page instead of at a
# tag, and what sed leaves behind is a url rather than a version.
case "${RELEASE}" in
  ''|*/*)
    echo "Cannot detect the latest published release of ${REPO}"
    exit 1
    ;;
esac

APP_NAME="krok"
ARCHIVE_NAME="${APP_NAME}-${OS}-${RELEASE}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${RELEASE}/${ARCHIVE_NAME}"

echo "Release: ${RELEASE}"
echo "Downloading: ${DOWNLOAD_URL}"

TMP_DIR="$(mktemp -d)"
# Named after the published asset, so that the archive and the checksum beside it
# can be checked by hand afterwards with the usual sha256sum -c.
ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE_NAME}"
CHECKSUM_PATH="${ARCHIVE_PATH}.sha256"
UNPACK_DIR="${TMP_DIR}/unpacked"

# Download archive and the checksum published beside it
curl -fL "${DOWNLOAD_URL}" -o "${ARCHIVE_PATH}"
curl -fL "${DOWNLOAD_URL}.sha256" -o "${CHECKSUM_PATH}"

# Verified before unpacking, not after. Two tools because no one of them is
# everywhere: sha256sum on linux and in git bash, shasum on macos.
if command -v sha256sum > /dev/null 2>&1; then
  ACTUAL_CHECKSUM="$(sha256sum "${ARCHIVE_PATH}" | awk '{print $1}')"
elif command -v shasum > /dev/null 2>&1; then
  ACTUAL_CHECKSUM="$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{print $1}')"
else
  echo "Neither sha256sum nor shasum is available to verify the download"
  rm -rf "${TMP_DIR}"
  exit 1
fi

# Only the hash is compared, so the name recorded in the checksum file does not
# have to match the one it was downloaded as.
EXPECTED_CHECKSUM="$(awk '{print $1}' "${CHECKSUM_PATH}")"

if [ "${ACTUAL_CHECKSUM}" != "${EXPECTED_CHECKSUM}" ]; then
  echo "Checksum mismatch for ${ARCHIVE_NAME}"
  echo "  expected ${EXPECTED_CHECKSUM}"
  echo "  actual   ${ACTUAL_CHECKSUM}"
  rm -rf "${TMP_DIR}"
  exit 1
fi

echo "Checksum ok: ${ACTUAL_CHECKSUM}"

# Extract archive, away from the two files that were downloaded
mkdir -p "${UNPACK_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${UNPACK_DIR}"

# Find binary inside extracted files
BIN_PATH="$(find "${UNPACK_DIR}" -type f -exec sh -c 'test -x "$1"' _ {} \; -print | head -n 1)"

if [ -z "${BIN_PATH}" ]; then
  echo "Executable ${APP_NAME} is not found in the archive ${ARCHIVE_NAME}"
  rm -rf "${TMP_DIR}"
  exit 1
fi

TARGET_DIR="${HOME}/.local/bin"
mkdir -p "${TARGET_DIR}"

# Copy binary to the target directory
cp "${BIN_PATH}" "${TARGET_DIR}/${APP_NAME}"
chmod +x "${TARGET_DIR}/${APP_NAME}"

# Cleanup
rm -rf "${TMP_DIR}"

echo "Installed: ${TARGET_DIR}/${APP_NAME}"
