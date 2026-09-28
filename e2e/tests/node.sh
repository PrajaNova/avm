SUITE=node
source /e2e/lib.sh
avm_shell
W="$HOME/node"
P="$W/project"
check_aliases_and_env "$W" "$P"

log "node: plugin"
expect_ok "avm plugin add node (sha256-verified)" avm plugin add node
expect_contains "plugin recorded as verified" "$(cat ~/.avm/plugins/avm-plugin-node/meta.json)" '"verified":true'

log "node: global 20 (install -g), local 22"
expect_ok "avm node install 20 -g" avm node install 20 -g
expect_ok "avm node install 22 (in project)" in_dir "$P" avm node install 22
hash -r
expect_contains "node outside the project is 20" "$(node -v)" "v20."
expect_contains "node in the project is 22" "$(cd "$P" && node -v)" "v22."
expect_contains "npm works through the shim" "$(cd "$P" && npm -v)" "."
expect_contains "avm which node (project)" "$(cd "$P" && avm which node)" "(local)"
expect_contains "avm which node (outside)" "$(avm which node)" "(global)"

log "node: global npm package survives a version switch"
(cd "$W" && npm install -g cowsay >/dev/null 2>&1)
hash -r
expect_contains "cowsay runs outside (installed under node 20)" "$(cowsay moo 2>&1)" "moo"
expect_contains "cowsay runs in the project pinned to node 22" "$(cd "$P" && cowsay moo 2>&1)" "moo"

log "node: version files"
mkdir -p "$W/nvmrc" && echo "20" > "$W/nvmrc/.nvmrc"
expect_contains ".nvmrc 20 → newest installed 20.x" "$(cd "$W/nvmrc" && avm which node)" "from ./.nvmrc"
expect_contains "node follows .nvmrc" "$(cd "$W/nvmrc" && node -v)" "v20."

log "node: trust"
mkdir -p "$W/cloned" && echo '{"aliases":{"hello":"echo hello-untrusted"},"env":{"AVM_E2E_SCOPE":"untrusted"}}' > "$W/cloned/.avm.json"
expect_eq "untrusted project falls back to the global alias" "$(cd "$W/cloned" && avm hello 2>/dev/null)" "hello-global-node"
expect_not_contains "untrusted env is not exported" "$(cd "$W/cloned" && avm env 2>/dev/null)" "untrusted"
(cd "$W/cloned" && avm trust >/dev/null)
expect_eq "after avm trust the local alias runs" "$(cd "$W/cloned" && avm hello)" "hello-untrusted"

finish
