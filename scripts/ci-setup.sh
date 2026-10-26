#!/bin/bash
# CI entrypoint for any platform: install avm, add plugins, install the project's tools.
#   AVM_VERSION   release tag to install (default latest)
#   AVM_PLUGINS   space/comma-separated plugins for `avm plugin add` (marketplace names, paths or URLs)
#   AVM_INSTALL   "false" to skip installing tools
# Uses the lock when one exists (`avm install --frozen`), else a plain install.
# Afterwards: eval "$(avm-bin env)" and put "$HOME/.local/bin:$HOME/.avm/shims" on PATH.
set -euo pipefail

# Piped from curl there is no script file; only trust an install.sh that sits next to this one.
here="$(cd "$(dirname "${BASH_SOURCE[0]:-/nonexistent/x}")" 2>/dev/null && pwd || true)"
if [ -n "$here" ] && [ -f "$here/../install.sh" ] && [ -f "$here/ci-setup.sh" ]; then
  bash "$here/../install.sh"
else
  curl -fsSL "https://raw.githubusercontent.com/PrajaNova/avm/main/install.sh" | bash
fi
export PATH="$HOME/.local/bin:/usr/local/bin:$PATH"
# CI runs a checkout the workflow author chose to build; there is no one to run `avm trust`.
export AVM_TRUST_ALL=1

for plugin in $(echo "${AVM_PLUGINS:-}" | tr ',' ' '); do
  avm-bin plugin add "$plugin"
done

[ "${AVM_INSTALL:-true}" = "true" ] || exit 0

# avm looks for the lock in parent directories too.
dir="$PWD"
frozen=()
while :; do
  if [ -f "$dir/.avm.lock" ]; then frozen=(--frozen); break; fi
  [ "$dir" = / ] && break
  dir="$(dirname "$dir")"
done
avm-bin install ${frozen[@]+"${frozen[@]}"}
