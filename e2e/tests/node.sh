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


log "node: outdated / upgrade (#26)"
O="$W/old"
mkdir -p "$O"
in_dir "$O" avm init >/dev/null
expect_ok "pin an older 20.x in another project" in_dir "$O" avm node use 20.9.0
(cd "$O" && npm install -g cowsay >/dev/null 2>&1) # a global package under 20.9.0, for prune's warning
json="$(in_dir "$O" avm outdated --json)"
in_range="$(printf '%s' "$json" | grep -o '"latest_in_range": "[^"]*"' | head -1 | cut -d'"' -f4)"
expect_contains "outdated --json: a newer 20.x is in range" "$in_range" "20."
expect_contains "outdated table shows current and range" "$(in_dir "$O" avm outdated)" "20.9.0"
expect_contains "upgrade --dry-run shows the plan" "$(in_dir "$O" avm upgrade --dry-run)" "20.9.0 → $in_range"
expect_contains "--dry-run changes nothing" "$(cat "$O/.avm.json")" '"node": "20.9.0"'
expect_ok "avm upgrade" in_dir "$O" avm upgrade
expect_contains ".avm.json pin moved" "$(cat "$O/.avm.json")" "\"node\": \"$in_range\""
expect_eq "node follows the new pin" "$(cd "$O" && node -v)" "v$in_range"
expect_contains "plugin outdated" "$(avm plugin outdated)" "up to date"

log "node: prune (#27)"
mkdir -p "$HOME/.avm/tools/node/1.0.0/bin" # installed before avm recorded use
plan="$(avm prune --dry-run)"
expect_not_contains "a version never seen used is skipped by default" "$plan" "node 1.0.0"
expect_contains "and the skip is reported" "$plan" "Skipped 1 version(s)"
expect_contains "--include-unrecorded lists it" "$(avm prune --include-unrecorded --dry-run)" "node 1.0.0"
expect_contains "the unused 20.9.0 is listed with its size" "$plan" "node 20.9.0"
expect_contains "warns about its global packages" "$plan" "holds global packages (cowsay, cowthink)"
expect_not_contains "keeps the project's node 22" "$plan" "node 22."
expect_not_contains "keeps the global pin" "$plan" "node $in_range"
expect_ok "avm prune -y" avm prune -y
expect_fail "20.9.0 is gone" test -d "$HOME/.avm/tools/node/20.9.0"
expect_contains "the project's node 22 still runs" "$(cd "$P" && node -v)" "v22."
expect_contains "nothing left to prune" "$(avm prune --dry-run)" "Nothing to prune"

finish
