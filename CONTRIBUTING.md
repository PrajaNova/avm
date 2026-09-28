# Contributing to avm

`avm` is maintained as a Rust workspace. Keep contributions scoped to the current Rust implementation unless a migration note explicitly says otherwise.

## Development setup

Prerequisites:

- Rust stable
- Docker with Compose for isolated test runs
- Node.js 20+ only for npm package wrapper and release tooling

Build:

```bash
cargo build --workspace
```

Run tests:

```bash
cargo test --workspace
```

Run the full Docker suite:

```bash
docker/tests/run-docker-tests.sh
```

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
