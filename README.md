# avm — Any Version Manager

`avm` is a Rust version manager: it picks the right `node`, `java` or
Android SDK per directory, and it runs project aliases and env from one
`.avm.json`. Each tool comes from a separately installed, compiled plugin.

What sets it apart:

- **Global packages survive version switches.** A CLI you `npm i -g` under
  one Node version stays runnable in a project pinned to another.
- **System fallback with a warning** when a pinned version isn't installed,
  instead of a hard failure.
- **Plugins are compiled binaries** speaking a typed JSON protocol, run as
  separate processes, and sha256-verified on install. Plugins can add
  their own subcommands (`avm android avd ...`).
- **First-class mobile/TV toolchains**: the Android SDK plus an emulator
  in one install.

### What avm handles today

- project and global alias resolution from `.avm.json`
- directory-aware execution with local-first precedence
- runtime environment injection via `PATH` and explicit `env` values
- Node package-script discovery from `package.json` (npm/yarn/pnpm/bun)
- shim-based command interception (for `node`, `npm`, and future tool shims)
- a runtime plugin marketplace — `avm plugin add <name>` fetches a
  compiled binary for your platform, on demand, from that plugin's own
  GitHub repo (never bundled into avm itself, never built from source)
- global packages installed under one version stay reachable even when a
  different local version is active elsewhere (see
  [Architecture](docs/architecture/ARCHITECTURE.md#global-packages-across-local-version-switches))
- `avm create <name>` points you at the plugin template repo to start a new plugin
- fallback behavior: if a managed version is not installed, avm uses the host/system command and warns
- existing version files (`.nvmrc`, `.tool-versions`, `.java-version`, …) work as-is
- `avm trust` gates project aliases/env, so cloning a repo never runs its config
- Linux and macOS (Apple Silicon and Intel); Windows via PowerShell ([phase 1](https://github.com/PrajaNova/avm/issues/23))

For positioning versus popular alternatives, see [Comparison with asdf, vfox, mise, and proto](#comparison-with-asdf-vfox-mise-and-proto).

## Comparison with asdf, vfox, mise, and proto

Checked against each project's docs in September 2026 (sources below).
"Not documented" means we couldn't find it, not that it's confirmed absent.
Where avm is behind, the row links the roadmap issue.

| | avm | asdf | vfox | mise | proto |
| --- | --- | --- | --- | --- | --- |
| Implementation | Rust | Go (rewritten from Bash in v0.16) | Go | Rust | Rust |
| Plugin model | Compiled executable per tool, separate process, JSON-over-stdio; asdf plugins via an adapter | Bash scripts | Lua | asdf and vfox plugins plus backends (aqua, npm, cargo, …) | WASM plugins, or TOML/JSON/YAML definitions |
| Download verification | sha256 on by default for plugins, avm-bin and first-party runtimes (fail closed); build provenance attestations published | Left to each plugin | Plugin-supplied checksum | aqua backend: checksums plus cosign/minisign/SLSA/attestations, on by default | Checksums, minisign, GPG |
| Existing version files | `.tool-versions`, `.nvmrc`, `.node-version`, `package.json`, `.java-version`, `.sdkmanrc`, on by default | `.tool-versions`; others opt-in | `.tool-versions`, `.nvmrc`, `.node-version`, `.sdkmanrc` | Opt-in per tool | `.nvmrc` etc. on by default |
| Lockfile | No ([#25](https://github.com/PrajaNova/avm/issues/25)) | No | Not documented | `mise.lock` (opt-in) | `.protolock` (unstable) |
| Tasks | Aliases and `package.json` scripts; no deps/caching ([#33](https://github.com/PrajaNova/avm/issues/33)) | No | No | Yes | No (moon is separate) |
| Env / secrets | `env` and `.env`; no secrets ([#32](https://github.com/PrajaNova/avm/issues/32)) | No | Not documented | `[env]`; secrets via fnox, sops/age | `[env]` and dotenv files |
| Config trust | `avm trust` (hash-pinned) | Not documented | Not documented | `mise trust` | Not documented |
| Windows | PowerShell phase 1; Windows plugins pending ([#23](https://github.com/PrajaNova/avm/issues/23)) | WSL only | Native | Native | Native |
| Activation | Shims ([PATH mode: #39](https://github.com/PrajaNova/avm/issues/39)) | Shims | Shell hook | Shims or `activate` | Shims, bin links, or `activate` |
| Pinned version missing | Falls back to system binary with a warning | Error | Not documented | Auto-install | Error (auto-install opt-in) |
| Plugin subcommands | Yes | Yes | No | Not documented | Not documented |
| Global npm packages across switches | Built in | Per plugin (default-packages file) | Not documented | Default-packages file (deprecated) | Shared globals dir |

<details><summary>Sources</summary>

- asdf: [v0.16.0 release](https://github.com/asdf-vm/asdf/releases/tag/v0.16.0), [configuration](https://asdf-vm.com/manage/configuration.html), [plugin commands](https://asdf-vm.com/plugins/create.html), [FAQ (Windows)](https://asdf-vm.com/more/faq.html), [asdf-nodejs](https://github.com/asdf-vm/asdf-nodejs)
- vfox: [repo](https://github.com/version-fox/vfox), [plugin how-to](https://vfox.dev/plugins/create/howto.html), [configuration](https://vfox.dev/guides/configuration.html), [core commands](https://vfox.dev/usage/core-commands.html)
- mise: [repo](https://github.com/jdx/mise), [plugins](https://mise.jdx.dev/plugins.html), [aqua backend](https://mise.jdx.dev/dev-tools/backends/aqua.html), [settings](https://mise.jdx.dev/configuration/settings.html), [mise.lock](https://mise.jdx.dev/dev-tools/mise-lock.html), [tasks](https://mise.jdx.dev/tasks/), [secrets](https://mise.jdx.dev/environments/secrets/), [trust](https://mise.jdx.dev/cli/trust.html), [installing](https://mise.jdx.dev/installing-mise.html), [node](https://mise.jdx.dev/lang/node.html)
- proto: [overview](https://moonrepo.dev/docs/proto), [plugins](https://moonrepo.dev/docs/proto/plugins), [non-WASM plugins](https://moonrepo.dev/docs/proto/non-wasm-plugin), [config](https://moonrepo.dev/docs/proto/config), [detection](https://moonrepo.dev/docs/proto/detection), [workflows](https://moonrepo.dev/docs/proto/workflows), [v0.31 globals](https://moonrepo.dev/blog/proto-v0.31)

</details>

## Quick start

```bash
avm init
avm add dev "pnpm run dev"
avm plugin add node
avm node use 20.11.1
avm run dev
```

### Shell setup (for plain command interception)

```bash
eval "$(avm shell-init)"
```

This enables direct execution via shims. For example, if `node` is managed in `.avm.json`, `node` will resolve through avm-managed versions before falling back.

## `.avm.json` format

```json
{
  "aliases": {
    "dev": "pnpm run dev",
    "release": "npm run release $1"
  },
  "env": {
    "NODE_ENV": "development",
    "API_URL": "https://api.local"
  },
  "tools": {
    "node": "20.11.1"
  }
}
```

`avm` also reads legacy flat-map `.avm.json` files and migrates them into the structured object form on read.

Precedence rules:

- local `.avm.json` overrides global `~/.avm.json`
- alias suggestions respect override ordering
- environment is merged with local values overriding global values
- tool version lookup is local first, then global

### Existing version files

No need to rewrite config to try avm: tool versions are also read from the
files projects already have. From nearest to farthest:

1. `.avm.json` `tools` (current directory)
2. `.tool-versions` (asdf format; `nodejs` maps to `node`)
3. `.nvmrc`, `.node-version`, `package.json` (`volta.node`, else the
   `engines.node` range), `.java-version`, `.sdkmanrc`
4. global `~/.avm.json`

Version files are searched upward from the current directory, and the
nearest directory wins. Partial specs resolve to the newest installed
match: `20` → `20.11.1`, `lts/*`, `lts/iron`, `>=18 <21`, `^20.1`, and
`17` → `openjdk-17.0.9+9`. `avm which node` shows which file won, e.g.
`20.11.1 (from ./.nvmrc)`. Set `"idiomatic_version_files": false` in
`~/.avm.json` to ignore the tool-specific files (item 3).

## Commands

- `avm init` initializes `.avm.json` in the current directory
- `avm alias add [--global] <alias> <command>` adds an alias (short: `avm aa ...`;
  `avm add ...` also still works, unchanged)
- `avm alias remove [--global] <alias>` removes an alias (`avm remove ...` still works too)
- `avm alias list` lists configured aliases
- `avm list` shows merged aliases, env, tools, and plugin aliases
- `avm which <alias-or-tool>` prints the origin and resolved value
- `avm run <alias> [args...]` executes resolved command
- `avm env` prints shell-safe `export` lines (this is what shell-init evals on every command)
- `avm env add [--global] <KEY> <value>` adds a custom env var to `.avm.json` (short: `avm ea ...`)
- `avm env remove [--global] <KEY>` removes one; `avm env list` lists configured ones
- `avm resolve <alias> [args...]` prints the expanded shell command
- `avm plugin add <name>` installs a plugin (short: `avm pa <name>`) — resolves `<name>` against the
  [marketplace](https://github.com/PrajaNova/avm-marketplace) and fetches
  a compiled release for your platform; an `org/repo` or full URL installs
  from source instead (asdf-style plugins, or one not yet in the marketplace)
- `avm plugin list` shows installed plugins; `avm plugin available` shows
  the marketplace
- `avm plugin remove <name>` / `avm plugin update <name>` manage installed plugins
- `avm create <name>` prints the `gh repo create ... --template PrajaNova/avm-plugin-template`
  command to start a new plugin — see [Creating a plugin](docs/plugins/CREATING_A_PLUGIN.md)
- `avm <plugin> versions` lists installable versions, for example `avm node versions`
- `avm <plugin> <major> versions` filters installable versions, for example `avm node 20 versions`
- `avm <plugin> latest versions` shows the latest installable version
- `avm <plugin> use <version>` sets plugin version locally
- `avm <plugin> use <version> --global` sets plugin version globally
- `avm <plugin> install <version>` installs a managed plugin version when supported
- `avm <plugin> uninstall <version>` removes an installed managed version when present
- `avm shims install|remove|path|activate` controls shim lifecycle (`reshim` is an alias of `install`)
- `avm shell-init` prints shell bootstrap script
- `avm --version` prints current CLI version

## Package layout (workspace crates)

This repository is organized as a Rust workspace:

- `crates/avm-cli` — the `avm-bin` binary
  - `src/cli/` — Clap command routing and handlers
  - `src/config.rs`, `src/resolver.rs` — `.avm.json` parsing, alias/tool/env resolution
  - `src/shims.rs` — shim generation and PATH lookup
  - `src/runtime.rs` — plugin discovery, the protocol host runner, the
    marketplace installer, and the legacy asdf compatibility adapter
- `crates/avm-plugin-api`
  - the `ToolProvider` trait, the plugin wire protocol, and the `runner`
    module every plugin's `main.rs` uses — the one crate a plugin depends on

node/java/android are **not** workspace crates — they're separate repos
([avm-plugin-node](https://github.com/PrajaNova/avm-plugin-node),
[avm-plugin-java](https://github.com/PrajaNova/avm-plugin-java),
[avm-plugin-android](https://github.com/PrajaNova/avm-plugin-android)),
fetched at runtime via `avm plugin add`, same as any third-party plugin.

Docs:

- [Architecture](docs/architecture/ARCHITECTURE.md) — crates, the
  marketplace, the wire protocol
- [Creating a plugin](docs/plugins/CREATING_A_PLUGIN.md) — the plugin template,
  the `ToolProvider` reference, testing, publishing, getting listed

Agent and LLM context: [llms.txt](llms.txt)

## Installation

Use any option supported in your environment:

```bash
brew install avm
```

```bash
npm install -g @prajanova/avm
```

```bash
cargo install --path .
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | iex
```


`install.sh` and the npm installer verify the release archive against the
release's `checksums.txt` before extracting. Every avm and first-party
plugin release also carries a GitHub build provenance attestation:

```bash
gh attestation verify avm_linux_amd64.tar.gz -R PrajaNova/avm
gh attestation verify avm-plugin-node_linux_amd64.tar.gz -R PrajaNova/avm-plugin-node
```

## Docker-based test suite

Run the full Rust and scenario suite in an isolated container:

```bash
docker/tests/run-docker-tests.sh
```

Run only Rust tests locally:

```bash
cargo test --workspace
```

Run one scenario:

```bash
docker/tests/run-docker-tests.sh 01
docker/tests/run-docker-tests.sh 01-basic-alias.sh
docker/tests/run-docker-tests.sh docker/tests/scenarios/01-basic-alias.sh
```

Scenario files:
- `docker/tests/scenarios/01-basic-alias.sh`
- `docker/tests/scenarios/02-local-global-precedence.sh`
- `docker/tests/scenarios/03-shim-fallback.sh`
- `docker/tests/scenarios/04-node-package-scripts.sh`
- `docker/tests/scenarios/05-plugin-first-node.sh`
- `docker/tests/scenarios/06-asdf-java-plugin.sh`
- `docker/tests/scenarios/07-trust.sh`
- `docker/tests/scenarios/08-version-files.sh`

## Plugin behavior

`avm plugin add <name>` is the only install path — nothing ships with
`avm-bin`, not even node, java, or android. Every provider is a
standalone executable speaking a small JSON-over-stdio protocol, resolved
in two tiers:

1. **Marketplace-installed** (`~/.avm/plugins/avm-plugin-<name>/bin/avm-plugin`) —
   `avm plugin add node` looks up `node` in the
   [marketplace registry](https://github.com/PrajaNova/avm-marketplace),
   fetches [avm-plugin-node](https://github.com/PrajaNova/avm-plugin-node)'s
   latest GitHub Release for your platform (a compiled binary, never
   source), and installs it there.
2. **Legacy asdf adapter** — community asdf-style plugins
   (`bin/list-all`, `bin/install`) that haven't adopted the native
   protocol still work: `avm plugin add https://github.com/halcyon/asdf-java.git`
   exposes the provider as `java` and `avm java ...` drives the plugin
   scripts. This tier is permanent, not a migration shim.

Both tiers power the same command surface — version selection, automatic
install when a selected version is missing, env var export, and shim
routing through `~/.avm/tools/<tool>/<version>/bin`.

Want to add support for another tool? Start from the plugin template
(`avm create <name>` prints the command) — see
[Creating a plugin](docs/plugins/CREATING_A_PLUGIN.md).

## Security model

A project's `.avm.json` aliases and `env`, and its `.env` files, run with
your privileges. Once `avm shell-init` is active they reach every shimmed
`node`/`java`, so avm ignores them until you trust that exact file content:

```bash
cd cloned-repo
avm trust            # shows the aliases/env it will enable, then trusts them
avm trust --list     # everything you've trusted
avm trust --revoke   # stop trusting this directory
```

- Trust is stored in `~/.avm/trusted.json` as path → sha256. Any edit made
  outside avm makes the file untrusted again. `avm init`, `avm add` and
  `avm env add` keep an already-trusted file trusted.
- Tool pins (`tools`, `.nvmrc`, `.tool-versions`, …) only pick a version
  and are always honored.
- Your global `~/.avm.json` is always trusted. It can list
  `"trusted_paths": ["~/work/**"]` to trust whole trees.
- `AVM_TRUST_ALL=1` trusts everything. It's meant for CI and is never
  implied by `CI=true`.
- Downloads are verified too; see [SECURITY.md](./SECURITY.md).

## Notes for contributors

- Keep all runtime logic in Rust crates.
- Prefer explicit, typed errors (`thiserror` / `anyhow`) over panics in runtime paths.
- Follow workspace standards in [AGENTS.md](./AGENTS.md).
- Use shim execution as the default integration model for plain command resolution.

## License

MIT
