#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$SCRIPT_DIR/lib.sh"

WORKDIR="$(mk_workdir)"
PLUGIN_ROOT="$(mk_workdir)"
ARCHIVE_ROOT="$(mk_workdir)"
export AVM_PLUGIN_DIR="$WORKDIR/plugins"
trap 'rm -rf "$WORKDIR" "$PLUGIN_ROOT" "$ARCHIVE_ROOT"' EXIT

log "Scenario 06: asdf-compatible java plugin install and shim"
write_json_file "$WORKDIR/.avm.json" '{
  "aliases": {},
  "env": {},
  "tools": {}
}'

RUNTIME_DIR="$ARCHIVE_ROOT/fake-java-runtime"
mkdir -p "$RUNTIME_DIR/bin"
cat > "$RUNTIME_DIR/bin/java" <<'EOF'
#!/usr/bin/env sh
echo asdf-java-runtime
EOF
chmod +x "$RUNTIME_DIR/bin/java"
tar -C "$ARCHIVE_ROOT" -czf "$ARCHIVE_ROOT/fake-java-runtime.tar.gz" "fake-java-runtime"

PLUGIN_DIR="$PLUGIN_ROOT/asdf-java"
mkdir -p "$PLUGIN_DIR/bin"
printf 'file://%s\n' "$ARCHIVE_ROOT/fake-java-runtime.tar.gz" > "$PLUGIN_DIR/archive-url"
cat > "$PLUGIN_DIR/bin/list-all" <<'EOF'
#!/usr/bin/env sh
printf 'temurin-22.0.0+1 temurin-21.0.1+1\n'
EOF
cat > "$PLUGIN_DIR/bin/install" <<'EOF'
#!/usr/bin/env sh
set -eu
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$(cat "$ASDF_DIR/archive-url")" -o "$tmp/java.tar.gz"
tar -xzf "$tmp/java.tar.gz" -C "$tmp"
mkdir -p "$ASDF_INSTALL_PATH"
cp -R "$tmp/fake-java-runtime/." "$ASDF_INSTALL_PATH/"
EOF
cat > "$PLUGIN_DIR/bin/uninstall" <<'EOF'
#!/usr/bin/env sh
rm -rf "$ASDF_INSTALL_PATH"
EOF
chmod +x "$PLUGIN_DIR/bin/list-all" "$PLUGIN_DIR/bin/install" "$PLUGIN_DIR/bin/uninstall"

out="$(run_avm "$WORKDIR" plugin add "$PLUGIN_DIR")"
assert_contains "$out" "Installed plugin" "asdf plugin should install"

out="$(run_avm "$WORKDIR" java latest versions)"
assert_contains "$out" "Available java versions:" "asdf java should list versions"
assert_contains "$out" "temurin-22.0.0+1" "latest java version should come from list-all"

out="$(run_avm "$WORKDIR" java use temurin-21.0.1+1)"
assert_contains "$out" "Installing java temurin-21.0.1+1" "asdf java use should auto-install"
assert_contains "$out" "Installed java temurin-21.0.1+1" "asdf java install should finish"
assert_contains "$out" "Set local java version to temurin-21.0.1+1" "asdf java use should update config"
test -x "$HOME/.avm/tools/java/temurin-21.0.1+1/bin/java" || fail "java binary should be installed"

run_avm "$WORKDIR" shims install
out="$(cd "$WORKDIR" && PATH="$HOME/.avm/shims:/workspace/target/debug:$PATH" "$HOME/.avm/shims/java")"
assert_contains "$out" "asdf-java-runtime" "java shim should execute asdf-installed binary"

log "Scenario 06 passed"
