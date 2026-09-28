#!/usr/bin/env bash
# Install avm fresh (the real way), then run each requested suite.
set -uo pipefail
cd /e2e
suites=("$@")
[ "${#suites[@]}" -eq 0 ] && suites=(core node java android)

failed=()
bash install.sh || failed+=(install)
[ -x "$HOME/.local/bin/avm-bin" ] || { echo "avm-bin not installed; skipping suites"; exit 1; }

for suite in "${suites[@]}"; do
  [ -f "$suite.sh" ] || { echo "unknown suite: $suite (core | node | java | android)"; exit 2; }
  bash "$suite.sh" || failed+=("$suite")
done

echo
if [ "${#failed[@]}" -eq 0 ]; then
  printf '\033[32mALL PASSED\033[0m (install %s)\n' "${suites[*]}"
else
  printf '\033[31mFAILED:\033[0m %s\n' "${failed[*]}"
  exit 1
fi
