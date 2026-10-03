# Contributing to avm

`avm` contains the Rust workspace and the documentation and marketplace frontend.

## Development setup

Prerequisites:

- Rust stable
- Docker for the end-to-end suites
- Node.js 22+ and pnpm 11 for the marketplace frontend; Node.js also supports npm package wrapper and release tooling

Build:

```bash
cargo build --workspace
```

Run tests:

```bash
cargo test --workspace
bash scripts/check-plugin-releases.sh
```

Run the end-to-end suites in a clean container (Docker required; CI runs
these on every PR):

```bash
e2e/run.sh              # all suites
e2e/run.sh core node    # some suites
```

## Code layout

- `crates/avm-cli` is the `avm-bin` binary:
  - `src/cli/`: command routing and handlers
  - `src/config.rs`, `src/resolver.rs`, `src/version_files.rs`: `.avm.json`, version files, and alias/env/tool resolution
  - `src/trust.rs`: trust store for project config
  - `src/shims.rs`: shims and PATH lookup
  - `src/runtime.rs`: plugin discovery, the protocol host, the verified marketplace installer, and the asdf adapter
  - `src/update.rs`: `self-update` and the update notice
- `crates/avm-plugin-api` is the one crate a plugin depends on: the `ToolProvider` trait, the wire protocol, the runner, and the sha256 helpers.
- `plugins/avm-plugin-{node,java,android}` are members of this Cargo workspace. They use the local plugin API and remain separate executables, installed at runtime like third-party plugins.
- `marketplace/` contains the React frontend and `registry.json`. From that directory, run `pnpm install --frozen-lockfile`, `pnpm dev`, or `pnpm build`. CI builds it on every PR.

Build or test a single plugin with `cargo build -p avm-plugin-node` or
`cargo test -p avm-plugin-node`. All workspace binaries go into the root
`target/` directory.

More: [Architecture](docs/architecture/ARCHITECTURE.md) · [Creating a plugin](docs/plugins/CREATING_A_PLUGIN.md) · [Releasing](docs/ops/RELEASE.md) · agent/LLM context in [llms.txt](llms.txt).

## End-to-end tests

`e2e/run.sh` builds avm from your checkout and runs the suites in a fresh
Ubuntu container (`--rm`), installing avm with this repo's `install.sh`:

| Suite | Covers |
| --- | --- |
| `install` | install.sh refuses tampered/unverified archives, shell hook, `-v`/`--version` |
| `core` | plugin verification, asdf plugins, missing-version fallback, `package.json` scripts, `self-update`, the update notice |
| `node`, `java`, `android` | global + local aliases, env and versions, `-g` installs, tool env, version files; node also `outdated`/`upgrade`/`prune` and `avm trust` |

`e2e/run.sh release [suites]` tests the published release instead, and
`e2e/run.sh shell` opens the clean container. `e2e/windows.ps1` is the
Windows suite (CI's `windows` job). CI runs both on every PR.

## Pull requests

Before opening a PR, run:

```bash
cargo build --workspace
cargo test --workspace
npm run changelog:check
```

Recommended before larger PRs:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
```

## Code rules

- Keep command wiring in `crates/avm-cli/src/cli`.
- Keep config and resolver logic in `crates/avm-cli/src/{config,resolver}.rs`.
- Keep shim behavior in `crates/avm-cli/src/shims.rs`.
- Keep provider contracts in `crates/avm-plugin-api`.
- Keep external plugin execution in `crates/avm-cli/src/runtime.rs`.
- Avoid panics in runtime paths.
- Preserve `.avm.json` compatibility.
- Preserve local-first then global precedence.

## Docs

Update docs when behavior changes:

- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Runtime flow](docs/architecture/FLOW.md)
- [Testing](docs/ops/TESTING.md)
- [Release](docs/ops/RELEASE.md)
- [Migration](docs/migration/RUST_REWRITE.md)
