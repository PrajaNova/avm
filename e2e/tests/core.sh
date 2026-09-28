# Tool-independent behavior: plugin verification, the asdf adapter, missing-
# version fallback, and package.json scripts.
SUITE=core
source /e2e/lib.sh
avm_shell
W="$HOME/core"
mkdir -p "$W" && cd "$W"

log "core: marketplace plugin verification (fake local release)"
rel="$W/fake-release"
mkdir -p "$rel/build" "$rel/api/repos/o/fake/releases"
printf '#!/bin/sh\necho fake\n' > "$rel/build/avm-plugin-fake" && chmod +x "$rel/build/avm-plugin-fake"
tar -czf "$rel/avm-plugin-fake_linux_amd64.tar.gz" -C "$rel/build" avm-plugin-fake
echo '{"plugins":[{"name":"fake","description":"x","repo":"o/fake"}]}' > "$rel/registry.json"
publish() { # $1 = checksums.txt contents, or "" for none
  local assets="{\"name\":\"avm-plugin-fake_linux_amd64.tar.gz\",\"browser_download_url\":\"file://$rel/avm-plugin-fake_linux_amd64.tar.gz\"}"
  if [ -n "$1" ]; then
    printf '%s\n' "$1" > "$rel/checksums.txt"
    assets="$assets,{\"name\":\"checksums.txt\",\"browser_download_url\":\"file://$rel/checksums.txt\"}"
  fi
  echo "{\"tag_name\":\"v1.0.0\",\"assets\":[$assets]}" > "$rel/api/repos/o/fake/releases/latest"
}
add_fake() { AVM_MARKETPLACE_URL="$rel/registry.json" AVM_GITHUB_API_URL="$rel/api" avm-bin plugin add fake 2>&1; }

publish "$(printf '0%.0s' $(seq 64))  avm-plugin-fake_linux_amd64.tar.gz"
expect_contains "tampered plugin is refused" "$(add_fake)" "checksum mismatch"
expect_fail "nothing installed after a mismatch" test -e "$HOME/.avm/plugins/avm-plugin-fake"
publish ""
expect_contains "plugin without checksums.txt is refused" "$(add_fake)" "no checksums.txt"
publish "$(cd "$rel" && sha256sum avm-plugin-fake_linux_amd64.tar.gz)"
expect_contains "verified plugin installs" "$(add_fake)" "Installed fake"
expect_contains "meta.json records verified" "$(cat "$HOME/.avm/plugins/avm-plugin-fake/meta.json")" '"verified":true'
avm plugin remove fake >/dev/null 2>&1

log "core: asdf-style plugin"
ap="$W/asdf-kotlin"
mkdir -p "$ap/bin"
printf '#!/bin/sh\necho 1.9.0 2.0.0\n' > "$ap/bin/list-all"
printf '#!/bin/sh\nmkdir -p "$ASDF_INSTALL_PATH/bin"\nprintf "#!/bin/sh\\necho kotlin-$ASDF_INSTALL_VERSION\\n" > "$ASDF_INSTALL_PATH/bin/kotlin"\nchmod +x "$ASDF_INSTALL_PATH/bin/kotlin"\n' > "$ap/bin/install"
chmod +x "$ap/bin/list-all" "$ap/bin/install"
expect_contains "avm plugin add <asdf plugin dir>" "$(avm plugin add "$ap" 2>&1)" "Installed plugin"
expect_contains "asdf plugin lists versions" "$(avm kotlin versions 2>&1)" "2.0.0"
expect_ok "avm kotlin use 2.0.0 (installs via bin/install)" avm kotlin use 2.0.0
expect_contains "avm which kotlin" "$(avm which kotlin)" "2.0.0"

log "core: pinned but missing version falls back to the system binary"
mkdir -p "$W/missing/fake-bin"
printf '#!/bin/sh\necho system-node "$@"\n' > "$W/missing/fake-bin/node" && chmod +x "$W/missing/fake-bin/node"
echo '{"tools":{"node":"99.9.9"}}' > "$W/missing/.avm.json"
out="$(cd "$W/missing" && PATH="$HOME/.avm/shims:$W/missing/fake-bin:$PATH" node -v 2>&1)"
expect_contains "warns that the pinned version isn't installed" "$out" "node 99.9.9 is not installed"
expect_contains "runs the system node" "$out" "system-node -v"

log "core: package.json scripts become aliases"
mkdir -p "$W/pkg" && echo '{"scripts":{"start":"echo from-start"}}' > "$W/pkg/package.json" && touch "$W/pkg/pnpm-lock.yaml"
expect_contains "script alias comes from package.json" "$(cd "$W/pkg" && avm which start)" "plugin alias 'start' from node"
expect_contains "pnpm lockfile picks pnpm" "$(cd "$W/pkg" && avm which start)" "pnpm run start"

finish
