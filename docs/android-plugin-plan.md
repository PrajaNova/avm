# Android support for avm — plan

Status: **planning** (no code yet). Branch: `feat/android-support`.

Goal: let a user set up and manage an Android dev environment through avm —
install the SDK / platform-tools / build-tools, and create + run emulators —
with the same dir-aware, per-project ergonomics avm gives node and java.

## What avm already gives us (no core changes needed)

avm has two plugin surfaces, and one plugin directory can be **both** at once:

| Surface | Required files | Yields | Install location |
|---|---|---|---|
| **asdf provider** | `bin/list-all`, `bin/install`, `bin/uninstall` | `avm android versions / install <v> / use <v>`; PATH auto-injected for the selected version by directory | `~/.avm/tools/android/<version>/` |
| **AVM alias plugin** | `plugin.json`, `bin/export-aliases` | named commands via `avm <alias>` | n/a (aliases only) |

Install path for either: `avm plugin add <git-url-or-path>`. Detection is by the
files present (`is_asdf_plugin_source` / `is_avm_plugin_source` in
`crates/avm-runtime/src/lib.rs`). A hybrid dir loads aliases **and** registers
the asdf provider.

Relevant mechanics already in place:
- asdf `install` runs sandboxed: `env_clear`, minimal PATH (`/usr/bin:/bin:...`),
  `HOME` + `TMPDIR` passed through, plus `ASDF_INSTALL_VERSION` and
  `ASDF_INSTALL_PATH=~/.avm/tools/android/<v>`.
- `is_installed(v)` checks `~/.avm/tools/android/<v>/bin/<toolname>` — so the
  install hook must leave something at `bin/android` (a wrapper/symlink) for avm
  to consider the version installed and to wire PATH.
- Install timeout is `AVM_ASDF_INSTALL_TIMEOUT` (default 120s) — too short for a
  multi-GB SDK pull; the plugin must document raising it.

## Why Android is not just another `node`

1. **Multi-component, not one binary.** SDK = cmdline-tools + platform-tools +
   `platforms;android-N` + `build-tools;N` + `emulator` + `system-images` + AVDs.
   We map "an avm version" to a cmdline-tools/SDK baseline and delegate component
   management to `sdkmanager`.
2. **Needs a JDK.** `sdkmanager`/`avdmanager` require Java, but the asdf sandbox
   clears env (no `JAVA_HOME`). Must resolve a JDK explicitly.
3. **Licenses + size.** `sdkmanager --licenses` must be accepted (scriptable with
   `yes`); a baseline pull is several GB and slow.
4. **Emulator/AVD lifecycle** (create/list/delete/boot) is outside a version
   manager's model → belongs on the alias surface.

## Chosen approach: **asdf provider**

`avm android install <v>` / `avm android use <v>`, per-project pinning, dir-aware
PATH. The "version" is the **Android platform / API level** (e.g. `34`, `35`) —
the number Android devs already think in — with a pinned build-tools + a fixed
cmdline-tools underneath.

Plugin layout (installable via `avm plugin add <git-url>`):
```
android/                 # dir name "asdf-android" → tool name "android"
  bin/
    list-all             # print installable API levels, space-separated
    install              # install one API-level baseline into ASDF_INSTALL_PATH
    uninstall            # remove it
```
No `plugin.json` needed for a pure asdf plugin (runtime synthesizes a manifest).

### Mapping the SDK onto avm's one-version-one-bindir model
avm treats `~/.avm/tools/android/<v>/bin/android` as the "is it installed?" marker
and injects `~/.avm/tools/android/<v>/bin` onto PATH when selected. Android has no
`android` binary anymore, so `install` lays down a **self-contained SDK per
version** and a `bin/` of wrappers:
```
~/.avm/tools/android/34/
  sdk/                   # ANDROID_HOME for this version (cmdline-tools, platform-tools,
                         #   platforms;android-34, build-tools;34.0.0, emulator, system image)
  bin/
    android              # marker + `exec sdkmanager "$@"` (satisfies is_installed)
    adb  sdkmanager  avdmanager  emulator   # thin wrappers exporting ANDROID_HOME=../sdk
```
Result: selecting the version in a project dir puts `adb`, `sdkmanager`,
`avdmanager`, `emulator` on PATH pointed at that version's SDK.

### The three hooks (pseudocode)
`bin/list-all`
```sh
# Emit API levels we support. Start with a curated static list; later derive from
# `sdkmanager --list`. Space-separated, ascending — avm truncates for recent/latest.
echo "30 31 32 33 34 35"
```
`bin/install`  (env: ASDF_INSTALL_VERSION=api level, ASDF_INSTALL_PATH=target dir)
```sh
set -e
api="$ASDF_INSTALL_VERSION"; root="$ASDF_INSTALL_PATH"; sdk="$root/sdk"
require_jdk            # see JDK note; fail early with instructions if absent
mkdir -p "$sdk/cmdline-tools"
curl -fL "$CMDLINE_TOOLS_URL" -o "$TMPDIR/clt.zip"       # host-appropriate URL
unzip -q "$TMPDIR/clt.zip" -d "$sdk/cmdline-tools" && mv .../cmdline-tools "$sdk/cmdline-tools/latest"
sm="$sdk/cmdline-tools/latest/bin/sdkmanager"
yes | "$sm" --sdk_root="$sdk" --licenses
"$sm" --sdk_root="$sdk" \
    "platform-tools" "platforms;android-$api" "build-tools;$api.0.0" \
    "emulator" "system-images;android-$api;google_apis;$(host_abi)"
mkdir -p "$root/bin"
write_wrappers "$root/bin" "$sdk"    # adb/sdkmanager/avdmanager/emulator + android marker
```
`bin/uninstall` → `rm -rf "$ASDF_INSTALL_PATH"` (avm also removes the dir).

### Wrinkles this design must handle
- **JDK:** `sdkmanager` needs Java, but the asdf sandbox clears env (no
  `JAVA_HOME`). v1: `require_jdk` probes `java` on the minimal PATH and errors with
  "install a JDK, or run `avm java use <v>`" if missing. Phase 2: auto-resolve an
  avm-managed JDK.
- **Timeout:** baseline pull is multi-GB / minutes; default
  `AVM_ASDF_INSTALL_TIMEOUT` is 120s. README must tell users to raise it (e.g.
  `AVM_ASDF_INSTALL_TIMEOUT=1800`). Consider proposing a larger default upstream.
- **Host archive:** cmdline-tools URL + system-image ABI differ by OS/arch
  (darwin-arm64 vs linux-x64). `host_abi`/`CMDLINE_TOOLS_URL` branch on `uname`.
- **Licenses:** `yes | sdkmanager --licenses` for non-interactive acceptance.
- **`ANDROID_HOME`:** wrappers export it; a fuller fix exports it via avm `env`
  (Phase 2) so non-wrapped tools (Gradle) see it.

## Remaining minor decisions (sensible defaults chosen — override if you disagree)
1. **Plugin location** → standalone repo `prajanova/asdf-android`, installed via
   `avm plugin add`. (Keeps avm core clean; matches asdf-java flow.)
2. **JDK source** → **system JDK** for v1, error if missing; `avm java`
   auto-resolution deferred to Phase 2.
3. **AVD/emulator** → shipped as PATH wrappers (`avdmanager`, `emulator`), so
   `avdmanager create avd ...` works once a version is selected. No custom
   `android-avd-*` aliases in v1.
4. **"version" meaning** → Android **API level** (34, 35), build-tools pinned to
   `<api>.0.0`.

## Next step
Scaffold the `asdf-android` plugin repo (three hook scripts + README + a smoke
test that `list-all` prints levels and `install` lays down a working `bin/adb`),
then `avm plugin add ./asdf-android` and verify `avm android install 34` /
`avm android use 34` end to end.
