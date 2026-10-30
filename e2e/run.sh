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

# The scenario installs avm and edits shell rc files, so never run it on the real HOME.
home="$(mktemp -d)"
trap 'rm -rf "$home"' EXIT
echo "== $(uname -sm) (HOME=$home) =="
HOME="$home" AVM_NO_UPDATE_CHECK=1 bash "$root/e2e/live.sh"
