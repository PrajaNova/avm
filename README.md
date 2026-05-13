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
- a plugin system with pluggable providers
- fallback behavior: if a managed version is not installed, avm uses the host/system command and warns

For positioning versus popular alternatives, see [Comparison with asdf and vfox](#comparison-with-asdf-and-vfox).

## Comparison with asdf and vfox

| Capability | avm (this project) | asdf | vfox |
| --- | --- | --- | --- |
| Runtime model | Native Rust binary | Ruby/plugin ecosystem with Bash integrations | Rust shell-hook engine with plugin runtime |
| Tool interception | PATH shims in `~/.avm/shims` | Shim generation + dispatch by plugin hooks | Shell hook updates PATH dynamically |
| Plugin ecosystem | Provider-first + optional adapter layer | Bash-style plugins | Lua-style plugins |
| Node support strategy | Merged Node provider (`package.json` + version resolver) | External Node plugin scripts | Provider-based Node integrations |
| Fallback if requested node version missing | Uses system node with warning | Typically triggers plugin install flow | Typically triggers plugin install flow |
| Configuration default | `.avm.json` with local/global + legacy compatibility | `.tool-versions` | `.tool-versions` |
| Security / isolation | Rust host + plugin runtime boundary (WASM/exec adapters) | Shell scripts (higher host access) | In-process plugin runtime (less isolated than strict sandbox) |

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
- `avm add [--global] <alias> <command>` adds an alias
- `avm remove [--global] <alias>` removes an alias
- `avm list` shows merged aliases, env, tools, and plugin aliases
- `avm which <alias-or-tool>` prints the origin and resolved value
- `avm run <alias> [args...]` executes resolved command
- `avm env` prints shell-safe `export` lines
- `avm resolve <alias> [args...]` prints the expanded shell command
- `avm plugin add|list|remove|update` manages plugins
- `avm <plugin> versions` lists installable versions, for example `avm node versions`
- `avm <plugin> <major> versions` filters installable versions, for example `avm node 20 versions`
- `avm <plugin> latest versions` shows the latest installable version
- `avm <plugin> use <version>` sets plugin version locally
- `avm <plugin> use <version> --global` sets plugin version globally
- `avm <plugin> install <version>` is reserved for plugin installers; the current Node baseline does not auto-install
- `avm <plugin> uninstall <version>` removes an installed managed version when present
- `avm tool ...` remains a compatibility alias for older scripts, but new docs and examples use plugin-first commands
- `avm shims install|remove|path` controls shim lifecycle
- `avm shell-init` prints shell bootstrap script
- `avm version` prints current CLI version
- `avm all` prints grouped help for aliases, plugins, plugin commands, shell, and shims

## Package layout (workspace crates)

This repository is organized as a Rust workspace:

- `crates/avm-cli`
  - Clap-based binary entrypoint and command routing
- `crates/avm-core`
  - config parsing, alias/tool/env resolution, and shared types
- `crates/avm-shims`
  - shim generation and shim execution hooks
- `crates/avm-plugin-api`
  - host/plugin contract interfaces and manifest/schema model
- `crates/avm-plugin-node`
  - merged Node provider
  - Node version management and package-script alias discovery
- `crates/avm-runtime`
  - plugin runtime and external plugin execution abstraction

Architecture docs:

- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Runtime flow](docs/architecture/FLOW.md)
- [Docker and test workflow](docs/ops/TESTING.md)
- [Release and publishing](docs/ops/RELEASE.md)
- [Rust rewrite migration](docs/migration/RUST_REWRITE.md)

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

## Plugin behavior

The Node provider currently powers:

- project `package.json` alias extraction
- version selection for `node` through `avm node ...`
- manager fallback to an existing system installation when managed version is missing

The architecture is plugin-first, so additional providers can be added without changing the CLI flow.

## Notes for contributors

- Keep all runtime logic in Rust crates.
- Prefer explicit, typed errors (`thiserror` / `anyhow`) over panics in runtime paths.
- Follow workspace standards in [AGENTS.md](./AGENTS.md).
- Use shim execution as the default integration model for plain command resolution.

## License

MIT
