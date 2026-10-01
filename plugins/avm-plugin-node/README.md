# avm-plugin-node

avm's native Node.js provider: version install/switching plus automatic
`package.json` script aliases. Works with [avm](https://github.com/PrajaNova/avm)
via avm's plugin marketplace — install with:

```bash
avm plugin add node
```

That fetches the compiled release selected by the marketplace for your platform from
GitHub — no Rust toolchain or network access needed beyond the one
download. See [Development and releases](#development-and-releases) below if you're
building or publishing this workspace plugin.

## Features

- **Live version index** — `avm node versions` reads
  [`nodejs.org/dist/index.json`](https://nodejs.org/dist/index.json)
  directly, so every real Node release is available immediately, no
  hardcoded/stale list.
- **Version install & switching** — per-project (local) and machine-wide
  (global) pins, same model as nvm/fnm/asdf, resolved through avm's config.
- **Verified downloads** — every archive is checked against the release's
  `SHASUMS256.txt` (from the same mirror) before extraction; a mismatch or
  missing checksum file aborts the install (`AVM_ALLOW_UNVERIFIED=1` skips it).
- **`package.json` script aliases** — if a project has a `package.json`
  with a `scripts` block, avm exposes each script as a runnable alias
  (`avm <script-name>`) automatically, with the right package manager
  (`npm`/`pnpm`/`yarn`/`bun`) chosen from the lockfile present. This part
  runs inside `avm-cli` itself (not through the plugin protocol), so it
  works even before you've run `avm plugin add node`.
- **Global packages resolve across local version switches** — `npm install
  -g <pkg>` under your global Node version stays reachable even when a
  different local Node version is active in a project (avm's shim
  resolution tries the local pin's bin dir first, then falls back to the
  global pin's).

## Commands

Once installed (`avm plugin add node`), everything is under `avm node`:

| Command | What it does |
| --- | --- |
| `avm node` | Interactive menu (list / browse versions / install latest / uninstall / help) |
| `avm node list` | Show the selected and installed versions |
| `avm node versions` | Browse the 10 most recent releases |
| `avm node <major> versions` | e.g. `avm node 20 versions` — every release on that major line |
| `avm node latest versions` | Just the newest release |
| `avm node use <version> [-g\|--global]` | Select an installed version, locally (default) or globally |
| `avm node set <version> [-g\|--global]` | Alias for `use` |
| `avm node install <version\|latest\|N>` | Install (if missing) + auto-pin: local, and global too if nothing's pinned globally yet |
| `avm node install <version> --global` | Install + pin globally only |
| `avm node install <version> --no-pin` | Install without touching any pin |
| `avm node uninstall <version>` | Remove a managed version |

`<version>` accepts an exact version (`20.11.1`), a bare major (`20` —
resolves to that major's latest), or `latest`.

Package.json scripts work without any extra command — from a directory
with a `package.json`:

```bash
avm which start     # shows where the "start" alias resolves to
avm start            # runs it (npm/pnpm/yarn/bun run start, manager auto-detected)
```

## Environment

No special env vars — Node doesn't need one (unlike `JAVA_HOME` or
`ANDROID_HOME`). avm just puts the selected version's `bin/` on `PATH`.

## Development and releases

From the AVM workspace root:

```bash
cargo build --package avm-plugin-node
cargo test --package avm-plugin-node
./target/debug/avm-plugin-node manifest
```

Release through **Actions → Release workspace plugins** in `PrajaNova/avm`, selecting
`node` and the version from this plugin's `Cargo.toml`. Releases use
`avm-plugin-node-v<version>` tags. See [Releasing](../../docs/ops/RELEASE.md).
