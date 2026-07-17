#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
DISTDIR="$(mk_workdir)"
trap 'rm -rf "$WORKDIR" "$DISTDIR"' EXIT

log "Scenario 05: plugin-first node commands"
write_json_file "$WORKDIR/.avm.json" '{
  "aliases": {},
  "env": {},
  "tools": {}
}'
write_json_file "$DISTDIR/index.json" '[
  {"version":"v21.1.0","lts":false,"security":false},
  {"version":"v20.11.1","lts":"Iron","security":true},
  {"version":"v19.9.0","lts":false,"security":false}
]'

case "$(uname -m)" in
  aarch64|arm64) NODE_PLATFORM="linux-arm64" ;;
  x86_64|amd64) NODE_PLATFORM="linux-x64" ;;
  *) fail "unsupported test architecture: $(uname -m)" ;;
esac
mkdir -p "$DISTDIR/v20.11.1/node-v20.11.1-$NODE_PLATFORM/bin"
cat > "$DISTDIR/v20.11.1/node-v20.11.1-$NODE_PLATFORM/bin/node" <<'EOF'
#!/usr/bin/env sh
echo fake-node
EOF
chmod +x "$DISTDIR/v20.11.1/node-v20.11.1-$NODE_PLATFORM/bin/node"
tar -C "$DISTDIR/v20.11.1" -czf "$DISTDIR/v20.11.1/node-v20.11.1-$NODE_PLATFORM.tar.gz" "node-v20.11.1-$NODE_PLATFORM"

out="$(AVM_NODE_DIST_URL="$DISTDIR" run_avm "$WORKDIR" node use 20.11.1)"
assert_contains "$out" "Installing node 20.11.1" "node plugin use should auto-install missing version"
assert_contains "$out" "Installed node 20.11.1" "node plugin use should install selected version"
assert_contains "$out" "Set local node version to 20.11.1" "node plugin use should set local version"
test -x "$HOME/.avm/tools/node/20.11.1/bin/node" || fail "node binary should be installed"

out="$(AVM_NODE_DIST_URL="$DISTDIR" run_avm "$WORKDIR" node latest versions)"
assert_contains "$out" "Available node versions:" "node plugin latest should list versions"
assert_contains "$out" "21.1.0" "latest filter should return newest version"

out="$(AVM_NODE_DIST_URL="$DISTDIR" run_avm "$WORKDIR" node 19 versions)"
assert_contains "$out" "19.9.0" "major filter should return matching version"

log "Scenario 05 passed"
