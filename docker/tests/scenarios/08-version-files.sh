#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
trap 'rm -rf "$WORKDIR"' EXIT

log "Scenario 08: idiomatic version files (.nvmrc, .node-version, package.json, .tool-versions, .java-version, .sdkmanrc)"
fake_install() {
  mkdir -p "$HOME/.avm/tools/$1/$2/bin"
  printf '#!/bin/sh\necho %s-%s\n' "$1" "$2" > "$HOME/.avm/tools/$1/$2/bin/$1"
  chmod +x "$HOME/.avm/tools/$1/$2/bin/$1"
}
for v in 18.19.0 20.9.0 20.11.1 21.1.0 22.3.0; do fake_install node "$v"; done
for v in openjdk-17.0.9+9 openjdk-21.0.2+13; do fake_install java "$v"; done

expect_node() {
  out="$(run_avm "$WORKDIR" which node)"
  assert_equals "$out" "tool 'node': $1" "$2"
}

echo "lts/iron" > "$WORKDIR/.nvmrc"
expect_node "20.11.1 (from ./.nvmrc)" ".nvmrc lts/<codename>"
echo "lts/*" > "$WORKDIR/.nvmrc"
expect_node "22.3.0 (from ./.nvmrc)" ".nvmrc lts/*"
rm "$WORKDIR/.nvmrc"

echo "v20.9.0" > "$WORKDIR/.node-version"
expect_node "20.9.0 (from ./.node-version)" ".node-version"
rm "$WORKDIR/.node-version"

echo '{"engines":{"node":">=18 <21"}}' > "$WORKDIR/package.json"
expect_node "20.11.1 (from ./package.json)" "engines.node range"
echo '{"engines":{"node":">=18"},"volta":{"node":"18.19.0"}}' > "$WORKDIR/package.json"
expect_node "18.19.0 (from ./package.json)" "volta.node beats engines"

echo "nodejs 21.1.0" > "$WORKDIR/.tool-versions"
expect_node "21.1.0 (from ./.tool-versions)" ".tool-versions beats idiomatic files"

mkdir -p "$WORKDIR/sub"
echo "20" > "$WORKDIR/sub/.nvmrc"
out="$(run_avm "$WORKDIR/sub" which node)"
assert_equals "$out" "tool 'node': 20.11.1 (from ./.nvmrc)" "nearest directory wins"

write_json_file "$WORKDIR/.avm.json" '{"tools":{"node":"22.3.0"}}'
expect_node "22.3.0 (local)" ".avm.json tools beat version files"

out="$(cd "$WORKDIR/sub" && env AVM_TRUST_ALL=1 "${AVM_BIN:-/workspace/target/debug/avm-bin}" exec-shim node -- )"
assert_equals "$out" "node-20.11.1" "shims run the version-file version"

echo "17" > "$WORKDIR/.java-version"
out="$(run_avm "$WORKDIR" which java)"
assert_equals "$out" "tool 'java': openjdk-17.0.9+9 (from ./.java-version)" ".java-version"
rm "$WORKDIR/.java-version"
echo "java=21.0.2-tem" > "$WORKDIR/.sdkmanrc"
out="$(run_avm "$WORKDIR" which java)"
assert_equals "$out" "tool 'java': openjdk-21.0.2+13 (from ./.sdkmanrc)" ".sdkmanrc"

write_json_file "$HOME/.avm.json" '{"idiomatic_version_files":false}'
out="$(run_avm "$WORKDIR" which java)"
assert_equals "$out" "No mapping found for 'java'." "idiomatic_version_files=false disables .sdkmanrc"
rm "$HOME/.avm.json"

log "Scenario 08 passed"
