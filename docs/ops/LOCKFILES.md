# Reproducible tool installs

`avm install` installs the tools selected by local `.avm.json`, idiomatic
version files and global pins, with the existing local-first precedence.
It does not create pins or change `.avm.json`. Java installs before other tools
so a selected Android SDK can use the managed JDK.

Project locks contain only local `.avm.json` tools and project version-file
selections. Personal global pins are neither locked nor installed by a frozen
install; changing them does not invalidate the project lock. Unlocked installs
continue to include global selections.

```bash
avm lock
avm lock --platform linux_amd64,darwin_arm64,windows_amd64
avm install --frozen
```

`lock` fetches exact release metadata and checksums without downloading or
installing the runtimes. It writes `.avm.lock` in the current directory through
a temporary file and rename, only after all tools and platforms resolve.
Tools, platforms and plugin fingerprints are sorted for stable diffs.

```json
{
  "version": 1,
  "tools": {
    "node": {
      "requested": "20",
      "resolved": "20.11.1",
      "plugin": {
        "name": "node",
        "version": "0.3.3",
        "binaries": { "darwin_arm64": "<plugin executable sha256>" }
      },
      "assets": {
        "darwin_arm64": {
          "url": "https://nodejs.org/dist/v20.11.1/node-v20.11.1-darwin-arm64.tar.gz",
          "sha256": "<archive sha256>"
        }
      }
    }
  }
}
```

Commit the lockfile. Aliases, shims, `avm env`, `avm which`
and ordinary `avm install` use its resolved versions. Changing a selected tool
or its requested spec fails with `run avm lock`; explicit `avm lock` reads
unlocked config so it can regenerate a stale or malformed lock. Resolution uses
the nearest lock in the current directory or an ancestor. Subdirectories must
still select the same tools (for example through inherited version files), or
the lock fails rather than drifting. Parent `.avm.json` merging remains separate
work; run from the config directory if it does not apply in a child directory.
Environment and alias edits do not invalidate the lock.

`--frozen` requires a lockfile, identical effective tool selections, and an
asset for this platform. Installed native plugins must match the locked name
and version. The host also checks the executable hash on platforms for which
one was recorded. Cross-platform assets can be resolved remotely; executable
hashes are recorded only for the host running `avm lock`, so other hosts check
the plugin version until they record their own fingerprint. Installs warn
explicitly when this platform's plugin binary has no pinned hash.

Missing runtimes download from the locked URL and verify the locked SHA-256
before the plugin runs. Node and Java plugins reverify the local archive before
extracting it. `AVM_ALLOW_UNVERIFIED` cannot bypass these checks. Already installed
versions are reused; this is archive verification on installation, not a file
integrity scan of existing runtimes. Frozen installs validate the entire config
and plugin plan before beginning installations; a later download failure may
leave earlier successfully installed tools in place.

Node and Java implement the optional `resolve` and `install-locked` protocol.
Upgrade these plugins before locking. Android installs several packages through
`sdkmanager`, whose component revisions are not yet represented in the lock;
Android and legacy asdf plugins therefore support ordinary `avm install` only.
Trying to lock them fails clearly and preserves any existing lockfile.

Without a lockfile, ordinary `avm install` works with all installed providers,
resolves fuzzy specs normally and keeps their existing checksum policy.
