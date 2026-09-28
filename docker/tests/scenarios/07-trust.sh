#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
trap 'rm -rf "$WORKDIR"' EXIT
export AVM_TRUST_ALL=0
unset _AVM_TRUST_NOTICE

log "Scenario 07: untrusted project config (clone → cd → blocked → trust → edit → blocked)"
write_json_file "$WORKDIR/.avm.json" '{
  "aliases": { "hi": "echo trusted-hi" },
  "env": { "NODE_OPTIONS": "--require ./evil.js" },
  "tools": {}
}'

out="$(run_avm_expect_fail "$WORKDIR" run hi)"
assert_contains "$out" "is not trusted" "untrusted alias should be refused with a hint"

out="$(run_avm "$WORKDIR" env)"
[[ "$out" != *"NODE_OPTIONS"* ]] || fail "untrusted env must not be exported"
assert_contains "$out" "avm trust" "avm env should emit a trust notice"

out="$(run_avm "$WORKDIR" trust)"
assert_contains "$out" "alias hi → echo trusted-hi" "avm trust should show what it trusts"

out="$(run_avm "$WORKDIR" run hi)"
assert_contains "$out" "trusted-hi" "trusted alias should run"
out="$(run_avm "$WORKDIR" env)"
assert_contains "$out" "NODE_OPTIONS" "trusted env should be exported"

write_json_file "$WORKDIR/.avm.json" '{ "aliases": { "hi": "echo edited" } }'
out="$(run_avm_expect_fail "$WORKDIR" run hi)"
assert_contains "$out" "is not trusted" "editing the file should re-block it"

log "Scenario 07 passed"
