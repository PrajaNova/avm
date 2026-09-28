# Heavy: two API levels pull platform-tools, platforms, build-tools, the
# emulator and system images (several GB). Override the levels with
# ANDROID_GLOBAL / ANDROID_LOCAL.
SUITE=android
source /e2e/lib.sh
avm_shell
W="$HOME/android"
P="$W/project"
G="${ANDROID_GLOBAL:-34}"
L="${ANDROID_LOCAL:-35}"
check_aliases_and_env "$W" "$P"

log "android: a JDK for sdkmanager"
if ! avm java list 2>/dev/null | grep -q openjdk; then
  avm plugin add java >/dev/null && avm java install 17 -g >/dev/null
fi
expect_contains "a managed JDK is available" "$(avm java list 2>&1)" "openjdk"

log "android: plugin"
expect_ok "avm plugin add android (sha256-verified)" avm plugin add android

log "android: global $G (install -g), local $L"
expect_ok "avm android install $G -g" avm android install "$G" -g
expect_ok "avm android install $L (in project)" in_dir "$P" avm android install "$L"
expect_contains "avm which android (outside)" "$(avm which android)" "$G"
expect_contains "avm which android (project)" "$(cd "$P" && avm which android)" "$L"

log "android: ANDROID_HOME / ANDROID_SDK_ROOT"
expect_contains "ANDROID_HOME set outside" "$(avm env)" "ANDROID_HOME="
expect_contains "ANDROID_SDK_ROOT set in project" "$(cd "$P" && avm env)" "ANDROID_SDK_ROOT="
sdk_home() { in_dir "$1" avm env | sed -n "s/^export ANDROID_HOME=//p" | tr -d "'"; }
for level_dir in "$G:$W" "$L:$P"; do
  level="${level_dir%%:*}"; sdk="$(sdk_home "${level_dir#*:}")"
  expect_ok "API $level: platforms/android-$level installed" test -d "$sdk/platforms/android-$level"
  expect_ok "API $level: build-tools installed" test -n "$(ls "$sdk/build-tools" 2>/dev/null)"
  expect_ok "API $level: a system image installed" test -d "$sdk/system-images/android-$level"
  expect_ok "API $level: emulator installed" test -x "$sdk/emulator/emulator"
done
hash -r
expect_contains "adb works" "$(cd "$P" && adb version 2>&1)" "Android Debug Bridge"
expect_contains "sdkmanager works" "$(cd "$P" && sdkmanager --version 2>&1)" "."

finish
