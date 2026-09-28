#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT_DIR/package.json" | head -n 1)"
TAG="${RELEASE_TAG:-}"

if [ -z "$TAG" ]; then
  echo "RELEASE_TAG is not set"
  exit 1
fi

if [ "$TAG" != "v$VERSION" ]; then
  echo "Release tag $TAG does not match package.json version v$VERSION"
  exit 1
fi

bash "$ROOT_DIR/scripts/check-changelog.sh"
