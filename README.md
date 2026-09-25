# avm — Any Version Manager

`avm` is a Rust-native, monorepo-based tooling layer for local command aliases, project-level runtime selection, and plugin-driven command discovery.

It solves three practical problems:

- command drift across projects
- manual setup of project-specific runtime versions
- repetitive shell configuration for ad hoc aliases

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

For positioning versus popular alternatives, see [Comparison with asdf and vfox](#comparison-with-asdf-and-vfox).

## Comparison with asdf and vfox

| Capability | avm (this project) | asdf | vfox |
| --- | --- | --- | --- |
| Runtime model | Native Rust binary | Ruby/plugin ecosystem with Bash integrations | Rust shell-hook engine with plugin runtime |
| Tool interception | PATH shims in `~/.avm/shims` | Shim generation + dispatch by plugin hooks | Shell hook updates PATH dynamically |
| Plugin ecosystem | Runtime marketplace — compiled binaries fetched from each plugin's own GitHub repo, JSON-over-stdio protocol | Bash-style plugins | Lua-style plugins |
| Node support strategy | Native plugin (`avm-plugin-node`): live version index + `package.json` script resolver | External Node plugin scripts | Provider-based Node integrations |
| Fallback if requested node version missing | Uses system node with warning | Typically triggers plugin install flow | Typically triggers plugin install flow |
| Configuration default | `.avm.json` with local/global + legacy compatibility | `.tool-versions` | `.tool-versions` |
| Security / isolation | Plugins run as separate OS processes (never linked into `avm-bin`), a typed JSON contract instead of shared bash scripts; plugin downloads are sha256-verified against the release's `checksums.txt` before install | Shell scripts (higher host access) | In-process plugin runtime (less isolated than strict sandbox) |

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

Agent and LLM docs:

- [Agent guide](agent.md)
- [Agent skill](agent.skill.md)
- [LLM context](llm.txt)
- [LLM text context](llm.text)

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

## Notes for contributors

- Keep all runtime logic in Rust crates.
- Prefer explicit, typed errors (`thiserror` / `anyhow`) over panics in runtime paths.
- Follow workspace standards in [AGENTS.md](./AGENTS.md).
- Use shim execution as the default integration model for plain command resolution.

## License

MIT
