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


log "core: self-update (fake local release)"
su="$W/self-update"
mkdir -p "$su/bin" "$su/pkg" "$su/api/repos/PrajaNova/avm/releases/tags"
printf '#!/bin/sh\necho "avm 9.9.9"\n' > "$su/pkg/avm-bin" && chmod +x "$su/pkg/avm-bin"
tar -czf "$su/avm_linux_amd64.tar.gz" -C "$su/pkg" avm-bin
current="$(avm-bin --version | cut -d' ' -f2)"
avm_release() { # $1 = tag, $2 = checksums.txt contents; prints the release JSON
  printf '%s\n' "$2" > "$su/checksums.txt"
  echo "{\"tag_name\":\"$1\",\"assets\":[{\"name\":\"avm_linux_amd64.tar.gz\",\"browser_download_url\":\"file://$su/avm_linux_amd64.tar.gz\"},{\"name\":\"checksums.txt\",\"browser_download_url\":\"file://$su/checksums.txt\"}]}"
}
fresh_copy() { cp "$HOME/.local/bin/avm-bin" "$su/bin/avm-bin"; }
su_run() { AVM_GITHUB_API_URL="$su/api" "$su/bin/avm-bin" "$@" 2>&1; }
good_sum="$(cd "$su" && sha256sum avm_linux_amd64.tar.gz)"

fresh_copy
avm_release v9.9.9 "$(printf '0%.0s' $(seq 64))  avm_linux_amd64.tar.gz" > "$su/api/repos/PrajaNova/avm/releases/latest"
expect_contains "tampered update is refused" "$(su_run self-update)" "checksum mismatch"
expect_eq "binary untouched after a refused update" "$("$su/bin/avm-bin" --version)" "avm $current"

avm_release v9.9.9 "$good_sum" > "$su/api/repos/PrajaNova/avm/releases/latest"
expect_contains "verified update to the latest release" "$(su_run self-update)" "Updated avm $current → 9.9.9"
expect_eq "new binary is in place" "$("$su/bin/avm-bin" --version)" "avm 9.9.9"
expect_eq "no staging files left behind" "$(ls -A "$su/bin")" "avm-bin"

fresh_copy
avm_release v9.9.9 "$good_sum" > "$su/api/repos/PrajaNova/avm/releases/tags/v9.9.9"
avm_release "v$current" "$good_sum" > "$su/api/repos/PrajaNova/avm/releases/latest"
expect_contains "already on the latest release" "$(su_run self-update)" "is up to date"
expect_contains "--version pins a release" "$(su_run self-update --version 9.9.9)" "→ 9.9.9"

mkdir -p "$su/node_modules/@prajanova/avm/bin" && cp "$HOME/.local/bin/avm-bin" "$su/node_modules/@prajanova/avm/bin/"
expect_contains "npm installs are left to npm" "$("$su/node_modules/@prajanova/avm/bin/avm-bin" self-update 2>&1)" "npm install -g @prajanova/avm"

log "core: update notice"
cache="$HOME/.avm/update-check.json"
# `script` gives the command a pseudo-terminal, like a real shell.
notice_env() { env -u AVM_NO_UPDATE_CHECK -u CI AVM_GITHUB_API_URL="$su/api" "$@"; }
echo "{\"checked_at\":$(date +%s),\"notified_at\":0,\"latest\":\"9.9.9\"}" > "$cache"
expect_contains "notice at a terminal when a newer release is known" "$(notice_env script -qec "avm-bin list" /dev/null)" "avm 9.9.9 is available"
expect_not_contains "at most once a day" "$(notice_env script -qec "avm-bin list" /dev/null)" "is available"
echo "{\"checked_at\":$(date +%s),\"notified_at\":0,\"latest\":\"9.9.9\"}" > "$cache"
expect_not_contains "no notice when stderr isn't a terminal" "$(notice_env avm-bin list 2>&1)" "is available"
expect_not_contains "no notice in CI" "$(notice_env CI=true script -qec "avm-bin list" /dev/null)" "is available"
expect_not_contains "no notice with AVM_NO_UPDATE_CHECK=1" "$(AVM_NO_UPDATE_CHECK=1 script -qec "avm-bin list" /dev/null)" "is available"
expect_not_contains "no notice from shims or avm env" "$(notice_env script -qec "avm-bin env" /dev/null)" "is available"

avm_release v9.9.9 "$good_sum" > "$su/api/repos/PrajaNova/avm/releases/latest"
echo '{"checked_at":0,"notified_at":0}' > "$cache"
notice_env script -qec "avm-bin list" /dev/null >/dev/null
for _ in 1 2 3 4 5 6 7 8 9 10; do grep -q 9.9.9 "$cache" && break; sleep 1; done
expect_contains "background check records the latest release" "$(cat "$cache")" '"latest":"9.9.9"'
rm -f "$cache"

finish
