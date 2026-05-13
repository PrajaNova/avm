#!/usr/bin/env bash

set -euo pipefail

fail() {
  echo "[fail] $1" >&2
  exit 1
}

log() {
  echo "[info] $*"
}

run_avm() {
  local cwd="$1"
  shift
  local avm_bin="${AVM_BIN:-/workspace/target/debug/avm-bin}"
  local output
  local status

  set +e
  output="$(cd "$cwd" && env AVM_NODE_DIST_URL="${AVM_NODE_DIST_URL:-}" "$avm_bin" "$@" 2>&1)"
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
