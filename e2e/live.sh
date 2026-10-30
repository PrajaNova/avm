#!/usr/bin/env bash
# The scenario, run as a normal user on a clean machine:
#   1. install avm with the published install.sh
#   2. install the node plugin from the live registry
#   3. global alias + env, run from anywhere
#   4. two project folders, each with its own local alias/env/node version
#   5. which / list / run show which one wins where; removing a local one falls back to global
set -uo pipefail
PASS=0; FAIL=0
log() { printf '\n\033[1;34m== %s\033[0m\n' "$*"; }
ok() { PASS=$((PASS + 1)); printf '  \033[32m✓\033[0m %s\n' "$1"; }
bad() { FAIL=$((FAIL + 1)); printf '  \033[31m✗ %s\033[0m\n      %s\n' "$1" "$2"; }
eq() { if [ "$2" = "$3" ]; then ok "$1"; else bad "$1" "expected [$3], got [$2]"; fi; }
has() { if [[ "$2" == *"$3"* ]]; then ok "$1"; else bad "$1" "expected to contain [$3], got [$(printf '%s' "$2" | head -c 300)]"; fi; }
hasnt() { if [[ "$2" != *"$3"* ]]; then ok "$1"; else bad "$1" "did not expect [$3] in [$(printf '%s' "$2" | head -c 300)]"; fi; }
works() { local d="$1"; shift; local out; if out="$("$@" 2>&1)"; then ok "$d"; else bad "$d" "$* -> $(printf '%s' "$out" | tail -3)"; fi; }
# Run a command (or the `avm` shell function) from another directory.
in_dir() { local d="$1"; shift; (cd "$d" && "$@"); }
fails() { local d="$1"; shift; if "$@" >/dev/null 2>&1; then bad "$d" "expected failure: $*"; else ok "$d"; fi; }

log "1. install avm (published install.sh -> published release)"
# Default to the newest published build, prereleases included (the stable `latest` can be an
# older CLI generation). AVM_VERSION=latest installs the stable release; AVM_VERSION=v0.5.0 pins one.
if [ -z "${AVM_VERSION:-}" ]; then
  auth=(); [ -n "${GITHUB_TOKEN:-}" ] && auth=(-H "Authorization: Bearer $GITHUB_TOKEN")
  AVM_VERSION="$(curl -fsSL ${auth[@]+"${auth[@]}"} "https://api.github.com/repos/PrajaNova/avm/releases?per_page=30" \
    | sed -n 's/.*"tag_name": *"\(v[^"]*\)".*/\1/p' | head -1)"
fi
echo "installing avm ${AVM_VERSION}"
export AVM_VERSION
curl -fsSL https://raw.githubusercontent.com/PrajaNova/avm/main/install.sh | bash
works "avm-bin is installed" test -x "$HOME/.local/bin/avm-bin"
has "install.sh added the shell hook" "$(cat ~/.bashrc ~/.zshrc ~/.bash_profile ~/.profile 2>/dev/null)" 'avm-bin shell-init'
export PATH="$HOME/.local/bin:$PATH"
eval "$(avm-bin shell-init)"
has "avm -v prints a version" "$(avm -v)" "avm "
[ "$AVM_VERSION" != latest ] && has "installed the requested release" "$(avm -v)" "${AVM_VERSION#v}"

log "2. install a plugin from the live registry"
# The published CLI generation decides the spelling: `plugin add|remove|available` (older)
# or `plugin install|uninstall|search` (newer). Everything else below is the same in both.
if avm-bin plugin install --help >/dev/null 2>&1; then
  plugin_add() { avm plugin install "$@"; }; plugin_remove() { avm plugin uninstall "$@"; }
  plugin_find() { avm plugin search "$@"; }
else
  plugin_add() { avm plugin add "$@"; }; plugin_remove() { avm plugin remove "$@"; }
  plugin_find() { avm plugin available; }
fi
has "the registry lists node" "$(plugin_find node 2>&1)" "node"
works "install node from the registry" plugin_add node
works "node plugin binary is in place" test -x "$HOME/.avm/plugins/avm-plugin-node/bin/avm-plugin"
has "avm plugin list shows node" "$(avm plugin list 2>&1)" "node"
fails "an unknown plugin name is rejected" plugin_add no-such-plugin-xyz
has "the rejection names the plugin" "$(plugin_add no-such-plugin-xyz 2>&1)" "no-such-plugin-xyz"

log "3. global alias and env, usable from anywhere"
cd "$HOME"
works "add a global alias" avm alias add -g hello "echo hello-global"
works "add a global env var" avm env add -g WHERE global
works "add a global alias that reads the env" avm alias add -g where 'echo where=$WHERE'
eq "run the global alias" "$(avm hello)" "hello-global"
eq "avm run does the same" "$(avm run hello)" "hello-global"
has "which says it is a global alias" "$(avm which hello)" "global alias"
has "alias list shows it" "$(avm alias list)" "hello → echo hello-global"
eq "the alias sees the global env" "$(avm where)" "where=global"
has "avm env exports it" "$(avm env)" "WHERE=global"

log "4. two project folders with local aliases, env and node versions"
mkdir -p "$HOME/work/proj-a/src/deep" "$HOME/work/proj-b" "$HOME/work/plain"
(cd "$HOME/work/proj-a" && avm init >/dev/null && avm alias add hello "echo hello-a" >/dev/null \
  && avm alias add only-a "echo only-in-a" >/dev/null && avm env add WHERE proj-a >/dev/null)
(cd "$HOME/work/proj-b" && avm init >/dev/null && avm alias add hello "echo hello-b" >/dev/null \
  && avm alias add only-b "echo only-in-b" >/dev/null && avm env add WHERE proj-b >/dev/null)
works "files were created" test -f "$HOME/work/proj-a/.avm.json" -a -f "$HOME/work/proj-b/.avm.json"

log "5. the right alias wins in each folder"
eq "proj-a: local alias wins" "$(cd "$HOME/work/proj-a" && avm hello)" "hello-a"
eq "proj-b: local alias wins" "$(cd "$HOME/work/proj-b" && avm hello)" "hello-b"
eq "plain folder: global alias" "$(cd "$HOME/work/plain" && avm hello)" "hello-global"
eq "home: global alias" "$(cd "$HOME" && avm hello)" "hello-global"
eq "a subfolder inherits the project's alias" "$(cd "$HOME/work/proj-a/src/deep" && avm hello)" "hello-a"
eq "an alias only in proj-a runs there" "$(cd "$HOME/work/proj-a" && avm only-a)" "only-in-a"
hasnt "proj-a's alias does not leak into proj-b" "$(cd "$HOME/work/proj-b" && avm only-a 2>&1)" "only-in-a"
has "proj-b reports it as not found" "$(cd "$HOME/work/proj-b" && avm only-a 2>&1)" "not found"
has "which: local alias in proj-a" "$(cd "$HOME/work/proj-a" && avm which hello)" "local alias"
has "which: points at proj-a's file" "$(cd "$HOME/work/proj-a" && avm which hello)" "proj-a/.avm.json"
has "which: global alias in a plain folder" "$(cd "$HOME/work/plain" && avm which hello)" "global alias"
has "alias list in proj-b shows the local and the global" "$(cd "$HOME/work/proj-b" && avm alias list)" "hello → echo hello-b"
has "alias list in proj-b still lists the global alias" "$(cd "$HOME/work/proj-b" && avm alias list)" "where → "
has "resolve prints the command, not its output" "$(cd "$HOME/work/proj-a" && avm resolve hello)" "echo hello-a"

log "6. env: local overrides global, global fills the gaps"
eq "proj-a env wins" "$(cd "$HOME/work/proj-a" && avm where)" "where=proj-a"
eq "proj-b env wins" "$(cd "$HOME/work/proj-b" && avm where)" "where=proj-b"
eq "plain folder sees the global env" "$(cd "$HOME/work/plain" && avm where)" "where=global"
has "avm env in proj-a exports the local value" "$(cd "$HOME/work/proj-a" && avm env)" "WHERE=proj-a"
has "avm env list shows where each comes from" "$(cd "$HOME/work/proj-b" && avm env list)" "proj-b/.avm.json"

log "7. node: global 20, local 22 in proj-a only"
works "install node 20 globally" avm node install 20 -g
works "install node 22 in proj-a" in_dir "$HOME/work/proj-a" avm node install 22
hash -r
has "plain folder uses the global node" "$(cd "$HOME/work/plain" && node --version)" "v20."
has "proj-a uses its own node" "$(cd "$HOME/work/proj-a" && node --version)" "v22."
has "proj-b has no pin, so it uses the global node" "$(cd "$HOME/work/proj-b" && node --version)" "v20."
has "a subfolder of proj-a inherits node 22" "$(cd "$HOME/work/proj-a/src/deep" && node --version)" "v22."
has "which node in proj-a is the local pin" "$(cd "$HOME/work/proj-a" && avm which node)" "22"
has "which node in a plain folder is the global pin" "$(cd "$HOME/work/plain" && avm which node)" "20"
has "avm node list shows the selection" "$(cd "$HOME/work/proj-a" && avm node list)" "Selected version: 22"

log "8. removing a local alias falls back to the global one"
works "remove proj-a's local hello" in_dir "$HOME/work/proj-a" avm alias remove hello
eq "proj-a now runs the global hello" "$(cd "$HOME/work/proj-a" && avm hello)" "hello-global"
works "remove the global hello" avm alias remove -g hello
has "now it is gone everywhere" "$(cd "$HOME/work/plain" && avm hello 2>&1)" "not found"

log "9. wrong input is reported, not ignored"
has "unknown alias" "$(avm no-such-alias 2>&1)" "alias 'no-such-alias' not found"
has "removing a missing alias" "$(avm alias remove -g ghost 2>&1)" "not found"
has "a typo'd subcommand" "$(avm-bin plugin instal 2>&1)" "unrecognized subcommand"
has "a bad env name" "$(avm env add 'BAD KEY' v 2>&1)" "invalid env key"
fails "a bad node version" avm node use 99.99

printf '\nlive: %d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
