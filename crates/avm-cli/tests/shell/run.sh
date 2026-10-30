#!/usr/bin/env bash
# avm end-to-end tests in a throwaway container (--rm): every run starts from
# a clean Ubuntu and installs avm fresh.
#
#   crates/avm-cli/tests/shell/run.sh                   # build avm from this checkout; all suites
#   crates/avm-cli/tests/shell/run.sh node java         # just some suites (core | node | java | android)
#   crates/avm-cli/tests/shell/run.sh release [suites]  # test the published release instead
#   crates/avm-cli/tests/shell/run.sh shell             # a shell in the clean container
set -euo pipefail
root="$(cd "$(dirname "$0")/../../../.." && pwd)"

target=local
if [ "${1:-}" = "release" ]; then target=release; shift; fi

DOCKER_BUILDKIT=1 docker build --platform linux/amd64 -q --target "$target" \
  -t "avm-shell:$target" -f "$root/crates/avm-cli/tests/shell/Dockerfile" "$root" >/dev/null

tty=(); [ -t 1 ] && tty=(-t)
if [ "${1:-}" = "shell" ]; then
  exec docker run --rm -it --platform linux/amd64 -e AVM_VERSION --entrypoint bash "avm-shell:$target"
fi
exec docker run --rm ${tty[@]+"${tty[@]}"} --platform linux/amd64 -e AVM_VERSION -e GITHUB_TOKEN \
  -e ANDROID_GLOBAL -e ANDROID_LOCAL "avm-shell:$target" "$@"
