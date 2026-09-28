# Shared helpers. Each suite sources this, then calls `finish` at the end.
set -uo pipefail
PASS=0
FAIL=0

log() { printf '\n\033[1;34m== %s\033[0m\n' "$*"; }
ok() { PASS=$((PASS + 1)); printf '  \033[32m✓\033[0m %s\n' "$1"; }
bad() { FAIL=$((FAIL + 1)); printf '  \033[31m✗ %s\033[0m\n      %s\n' "$1" "$2"; }

expect_eq() { if [ "$2" = "$3" ]; then ok "$1"; else bad "$1" "expected [$3], got [$2]"; fi; }
expect_contains() { if [[ "$2" == *"$3"* ]]; then ok "$1"; else bad "$1" "expected to contain [$3], got [$(printf '%s' "$2" | head -c 400)]"; fi; }
expect_not_contains() { if [[ "$2" != *"$3"* ]]; then ok "$1"; else bad "$1" "did not expect [$3] in [$(printf '%s' "$2" | head -c 400)]"; fi; }
expect_ok() { local d="$1"; shift; local out; if out="$("$@" 2>&1)"; then ok "$d"; else bad "$d" "command failed: $* → $(printf '%s' "$out" | tail -3)"; fi; }
expect_fail() { local d="$1"; shift; if "$@" >/dev/null 2>&1; then bad "$d" "expected failure: $*"; else ok "$d"; fi; }

# Run a command (or avm function) from another directory.
in_dir() { local d="$1"; shift; (cd "$d" && "$@"); }

# The same setup install.sh writes to ~/.bashrc (which bash skips when
# non-interactive): avm-bin on PATH, then the shell hook.
avm_shell() {
  export PATH="$HOME/.local/bin:$PATH"
  eval "$(avm-bin shell-init)"
}

finish() {
  printf '\n%s: %d passed, %d failed\n' "$SUITE" "$PASS" "$FAIL"
  [ "$FAIL" -eq 0 ]
}

# Global + local alias with the same name, global + local env var, and an
# alias that reads the env. $1 = suite dir, $2 = project dir inside it.
check_aliases_and_env() {
  local dir="$1" project="$2" tag="$SUITE"
  mkdir -p "$project"
  cd "$dir"

  log "$tag: global and local alias with the same name"
  avm add -g hello "echo hello-global-$tag" >/dev/null
  (cd "$project" && avm init >/dev/null && avm add hello "echo hello-local-$tag" >/dev/null)
  expect_eq "outside the project the global alias runs" "$(avm hello)" "hello-global-$tag"
  expect_eq "inside the project the local alias wins" "$(cd "$project" && avm hello)" "hello-local-$tag"
  expect_contains "avm which hello (project)" "$(cd "$project" && avm which hello)" "local alias"
  expect_contains "avm which hello (outside)" "$(avm which hello)" "global alias"

  log "$tag: global and local env vars"
  avm env add -g AVM_E2E_SCOPE "global-$tag" >/dev/null
  (cd "$project" && avm env add AVM_E2E_SCOPE "local-$tag" >/dev/null)
  expect_contains "avm env outside → global value" "$(avm env)" "AVM_E2E_SCOPE=global-$tag"
  expect_contains "avm env in project → local value" "$(cd "$project" && avm env)" "AVM_E2E_SCOPE=local-$tag"
  avm add -g scope 'echo scope=$AVM_E2E_SCOPE' >/dev/null
  expect_eq "alias sees the global env outside" "$(avm scope)" "scope=global-$tag"
  expect_eq "alias sees the local env in the project" "$(cd "$project" && avm scope)" "scope=local-$tag"
}
