#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
trap 'rm -rf "$WORKDIR"' EXIT

log "Scenario 01: basic alias and local .avm.json"
write_json_file "$WORKDIR/.avm.json" '{
  "aliases": {
    "dev": "echo basic-start:$1",
    "check": "echo local-check:$1:$2"
  },
  "env": {
    "ENV_SCOPE": "local"
  },
  "tools": {
    "node": "20.11.1"
  }
}'

out="$(run_avm "$WORKDIR" run dev world)"
assert_contains "$out" "basic-start:world" "basic dev alias run"

out="$(run_avm "$WORKDIR" resolve dev world)"
assert_equals "$out" "echo basic-start:world" "resolve should expand placeholders"

out="$(run_avm "$WORKDIR" which dev)"
assert_contains "$out" "local alias 'dev'" "alias source should be local"

out="$(run_avm "$WORKDIR" env)"
assert_contains "$out" "export ENV_SCOPE=local" "local env should be merged and exported"

log "Scenario 01 passed"
