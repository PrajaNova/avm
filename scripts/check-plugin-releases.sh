#!/usr/bin/env bash
# After cargo build --workspace: verifies legacy and shared-repo plugin releases locally.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/avm-bin"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
export HOME="$WORK/home" AVM_PLUGIN_DIR="$WORK/home/.avm/plugins" AVM_NO_UPDATE_CHECK=1
export AVM_MARKETPLACE_URL="$WORK/registry.json" AVM_GITHUB_API_URL="$WORK/api"
mkdir -p "$HOME" "$WORK/build" "$WORK/api/repos/test/shared/releases/tags"
case "$(uname -s)" in Darwin) os=darwin ;; Linux) os=linux ;; *) exit 1 ;; esac
case "$(uname -m)" in arm64|aarch64) arch=arm64 ;; x86_64) arch=amd64 ;; *) exit 1 ;; esac
archive="avm-plugin-fake_${os}_${arch}.tar.gz"
cat > "$WORK/build/avm-plugin-fake" <<'PLUGIN'
#!/bin/sh
printf '%s\n' '{"name":"fake","version":"1.0.0","api_version":1}'
PLUGIN
chmod +x "$WORK/build/avm-plugin-fake"
tar -czf "$WORK/$archive" -C "$WORK/build" avm-plugin-fake
(cd "$WORK" && shasum -a 256 "$archive") > "$WORK/checksums.txt"
assets="[{\"name\":\"$archive\",\"browser_download_url\":\"file://$WORK/$archive\"},{\"name\":\"checksums.txt\",\"browser_download_url\":\"file://$WORK/checksums.txt\"}]"
printf '{"tag_name":"v1.0.0","assets":%s}\n' "$assets" > "$WORK/api/repos/test/shared/releases/latest"
printf '{"plugins":[{"name":"fake","description":"test","repo":"test/shared"}]}\n' > "$AVM_MARKETPLACE_URL"
"$BIN" plugin add fake
[[ "$(cat "$AVM_PLUGIN_DIR/avm-plugin-fake/meta.json")" == *'"version":"v1.0.0"'* ]]
printf '{"tag_name":"avm-plugin-fake-v2.0.0","assets":%s}\n' "$assets" > "$WORK/api/repos/test/shared/releases/tags/avm-plugin-fake-v2.0.0"
printf '{"plugins":[{"name":"fake","description":"test","repo":"test/shared","release_tag":"avm-plugin-fake-v2.0.0"}]}\n' > "$AVM_MARKETPLACE_URL"
# The repository's latest release is the CLI and contains no plugin assets.
printf '{"tag_name":"v9.0.0","assets":[]}\n' > "$WORK/api/repos/test/shared/releases/latest"
[[ "$("$BIN" plugin outdated --json)" == *'"latest": "avm-plugin-fake-v2.0.0"'* ]]
"$BIN" plugin update fake
[[ "$(cat "$AVM_PLUGIN_DIR/avm-plugin-fake/meta.json")" == *'"version":"avm-plugin-fake-v2.0.0"'* ]]
"$BIN" plugin remove fake
"$BIN" plugin add fake
[[ "$(cat "$AVM_PLUGIN_DIR/avm-plugin-fake/meta.json")" == *'"verified":true'* ]]
printf 'Legacy and pinned plugin add/update/outdated checks passed.\n'
