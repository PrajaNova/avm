#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
GLOBAL_BACKUP="$(mktemp)"

restore_global_config() {
  if [ -f "$GLOBAL_BACKUP" ]; then
    if [ -s "$GLOBAL_BACKUP" ]; then
      mv "$GLOBAL_BACKUP" "$HOME/.avm.json"
    else
      rm -f "$HOME/.avm.json"
    fi
  else
    rm -f "$HOME/.avm.json"
  fi
}
if [ -f "$HOME/.avm.json" ]; then
  cp "$HOME/.avm.json" "$GLOBAL_BACKUP"
else
  : > "$GLOBAL_BACKUP"
fi
trap 'restore_global_config; rm -rf "$WORKDIR"' EXIT

log "Scenario 02: local and global precedence"
write_json_file "$HOME/.avm.json" '{
  "aliases": {
    "build": "echo global-build",
    "legacy": "echo global-only"
  },
  "env": {
    "SCOPE": "global",
    "SHARED": "yes"
  },
  "tools": {
    "node": "20.11.1"
  }
}'

write_json_file "$WORKDIR/.avm.json" '{
  "aliases": {
    "build": "echo local-build"
  },
  "env": {
    "SCOPE": "local"
  }
}'

out="$(run_avm "$WORKDIR" which build)"
assert_contains "$out" "local alias 'build': echo local-build" "local alias must override global"

out="$(run_avm "$WORKDIR" which legacy)"
assert_contains "$out" "global alias 'legacy': echo global-only" "global alias fallback should work"

out="$(run_avm "$WORKDIR" env)"
assert_contains "$out" "export SHARED=yes" "global env must be present"
assert_contains "$out" "export SCOPE=local" "local env must override global"

log "Scenario 02 passed"
