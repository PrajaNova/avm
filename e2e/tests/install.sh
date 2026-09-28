# Fresh install the way a user does it: install.sh.
#   AVM_E2E_INSTALL=local   → this checkout's install.sh and its avm-bin build (PRs)
#   AVM_E2E_INSTALL=release → the published install.sh and release
SUITE=install
source /e2e/lib.sh

if [ "${AVM_E2E_INSTALL:-release}" = "local" ]; then
  log "install.sh refuses a tampered or unverifiable archive"
  bad_dist="$(mktemp -d)"
  cp /opt/dist/* "$bad_dist/"
  echo "0000000000000000000000000000000000000000000000000000000000000000  avm_linux_amd64.tar.gz" > "$bad_dist/checksums.txt"
  out="$(HOME="$(mktemp -d)" AVM_DOWNLOAD_BASE="file://$bad_dist" bash /opt/install.sh 2>&1)"
  expect_contains "checksum mismatch is refused" "$out" "checksum mismatch"
  rm "$bad_dist/checksums.txt"
  out="$(HOME="$(mktemp -d)" AVM_DOWNLOAD_BASE="file://$bad_dist" bash /opt/install.sh 2>&1)"
  expect_contains "a release without checksums.txt is refused" "$out" "can't be verified"

  log "install avm from this checkout with install.sh"
  AVM_DOWNLOAD_BASE=file:///opt/dist bash /opt/install.sh
else
  log "install avm ${AVM_VERSION:-latest} with the published install.sh"
  curl -fsSL https://raw.githubusercontent.com/PrajaNova/avm/main/install.sh | bash
fi

expect_ok "avm-bin installed to ~/.local/bin" test -x "$HOME/.local/bin/avm-bin"
expect_contains "install.sh added the shell hook to ~/.bashrc" "$(cat ~/.bashrc)" 'eval "$(avm-bin shell-init)"'

avm_shell
expect_contains "avm --version" "$(avm --version)" "avm "
expect_contains "avm -v" "$(avm -v 2>&1)" "avm "
expect_fail "avm-bin -V is not a flag" avm-bin -V
expect_ok "shims dir created" test -d "$HOME/.avm/shims"

finish
