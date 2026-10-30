#!/usr/bin/env bash
# `registry.json` (repo root) is the plugin registry; it carries no release pins because avm
# installs the newest `avm-plugin-<name>-v*` release. `marketplace/registry.json` is a frozen
# copy, with its `release_tag` pins, at the path already-published avm releases fetch from
# `main`: those releases need the pins, so it stays until they are no longer supported. The two
# must list the same plugins (pins aside), or older avm cannot install a plugin newer avm can.
set -euo pipefail
cd "$(dirname "$0")/.."
if jq -e '[.plugins[] | has("release_tag")] | any' registry.json >/dev/null; then
  echo "registry.json must not carry release_tag (avm reads the latest release); it belongs only in the legacy copy" >&2
  exit 1
fi
if ! diff -u <(jq -S . registry.json) <(jq -S 'del(.plugins[].release_tag)' marketplace/registry.json); then
  echo "registry.json and marketplace/registry.json list different plugins; update the legacy copy to match (keep its release_tag pins)" >&2
  exit 1
fi
echo "registry.json matches the legacy marketplace/registry.json (pins aside)"
