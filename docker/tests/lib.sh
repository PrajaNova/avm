#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "[fail] $1" >&2
  exit 1
}

log() {
  echo "[info] $*"
}

# Scenarios write .avm.json by hand, so run_avm trusts everything unless a
# scenario sets AVM_TRUST_ALL itself (see 07-trust.sh).
run_avm() {
  local cwd="$1"
  shift
  local avm_bin="${AVM_BIN:-/workspace/target/debug/avm-bin}"
  local output
  local status

  set +e
  output="$(cd "$cwd" && env AVM_NODE_DIST_URL="${AVM_NODE_DIST_URL:-}" AVM_TRUST_ALL="${AVM_TRUST_ALL-1}" "$avm_bin" "$@" 2>&1)"
  status=$?
  set -e

  echo "$output"
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output" >&2
    fail "avm command failed: avm $*"
  fi
  return "$status"
}

assert_contains() {
  local output="$1"
  local expected="$2"
  local context="$3"
  if [[ "$output" != *"$expected"* ]]; then
    fail "$context | expected to contain: $expected"
  fi
}

assert_equals() {
  local output="$1"
  local expected="$2"
  local context="$3"
  if [[ "$output" != "$expected" ]]; then
    fail "$context | expected: [$expected], got: [$output]"
  fi
}

mk_workdir() {
  mktemp -d
}

write_json_file() {
  local path="$1"
  local content="$2"
  printf '%s\n' "$content" > "$path"
}

# Like run_avm, but for commands that must fail: prints output, fails if it succeeded.
run_avm_expect_fail() {
  local cwd="$1"
  shift
  local avm_bin="${AVM_BIN:-/workspace/target/debug/avm-bin}"
  local output
  if output="$(cd "$cwd" && env AVM_TRUST_ALL="${AVM_TRUST_ALL-1}" "$avm_bin" "$@" 2>&1)"; then
    printf '%s\n' "$output" >&2
    fail "expected failure: avm $*"
  fi
  echo "$output"
}
