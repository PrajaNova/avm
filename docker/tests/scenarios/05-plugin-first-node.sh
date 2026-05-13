#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
DISTDIR="$(mk_workdir)"
trap 'rm -rf "$WORKDIR" "$DISTDIR"' EXIT

log "Scenario 05: plugin-first node commands"
write_json_file "$WORKDIR/.avm.json" '{
  "aliases": {},
  "env": {},
  "tools": {}
}'
write_json_file "$DISTDIR/index.json" '[
  {"version":"v21.1.0","lts":false,"security":false},
  {"version":"v20.11.1","lts":"Iron","security":true},
  {"version":"v19.9.0","lts":false,"security":false}
]'

out="$(run_avm "$WORKDIR" node use 20.11.1)"
assert_contains "$out" "Set local node version to 20.11.1" "node plugin use should set local version"

out="$(AVM_NODE_DIST_URL="$DISTDIR" run_avm "$WORKDIR" node latest versions)"
assert_contains "$out" "Available node versions:" "node plugin latest should list versions"
assert_contains "$out" "21.1.0" "latest filter should return newest version"

out="$(AVM_NODE_DIST_URL="$DISTDIR" run_avm "$WORKDIR" node 19 versions)"
assert_contains "$out" "19.9.0" "major filter should return matching version"

log "Scenario 05 passed"
