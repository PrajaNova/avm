#!/usr/bin/env bash
# Live end-to-end test on this machine (macOS or Linux): installs the published avm into a
# throwaway HOME and runs the journey in live.sh. CI runs it on clean runners per platform
# (.github/workflows/live-e2e.yml). Needs network.
#
#   e2e/run.sh                      # newest published release (prereleases included)
#   AVM_VERSION=latest e2e/run.sh   # the stable release
#   AVM_VERSION=v0.5.0 e2e/run.sh   # one specific release
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

# install.sh installs system-wide when /usr/local/bin is writable or sudo is passwordless, which
# would replace your own avm-bin. CI runners are disposable; elsewhere require an explicit opt-in.
if [ -z "${CI:-}" ] && [ "${AVM_E2E_FORCE:-}" != 1 ] && { [ -w /usr/local/bin ] || sudo -n true 2>/dev/null; }; then
  echo "refusing to run: install.sh would write /usr/local/bin/avm-bin on this machine." >&2
  echo "Run it in CI (live-e2e.yml), on a disposable machine, or set AVM_E2E_FORCE=1." >&2
  exit 1
fi

# The scenario edits shell rc files, so never run it on the real HOME. Resolve symlinks
# (macOS temp dirs live under /var -> /private/var) so HOME and the cwd compare equal.
home="$(cd "$(mktemp -d)" && pwd -P)"
trap 'rm -rf "$home"' EXIT
echo "== $(uname -sm) (HOME=$home) =="
HOME="$home" AVM_NO_UPDATE_CHECK=1 bash "$root/e2e/live.sh"
