#!/usr/bin/env sh
set -o errexit -o nounset

PROJECT_ROOT="$(cd "$(dirname "$0")/../../" && pwd)"

cd "${PROJECT_ROOT}"

cargo outdated -R
