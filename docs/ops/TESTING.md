# Testing avm

## Local Rust tests

```bash
cargo test --workspace
```

The main CLI integration tests live in:

```text
crates/avm-cli/tests/cli_scenarios.rs
```

They cover:

- local alias resolution
- local/global precedence
- Node `package.json` script discovery
- plugin-first command routing with `avm node ...`
- shim fallback when a managed Node version is missing

## Docker tests

Run the complete isolated test suite:

```bash
docker/tests/run-docker-tests.sh
```

This runs:

- `cargo test --workspace`
- scenario shell tests under `docker/tests/scenarios/`

Run one scenario:

```bash
docker/tests/run-docker-tests.sh 01
docker/tests/run-docker-tests.sh 03-shim-fallback.sh
```

## Scenario files

| Scenario | Purpose |
| --- | --- |
| `01-basic-alias.sh` | Alias run, resolve, source, and env export. |
| `02-local-global-precedence.sh` | Local override and global fallback behavior. |
| `03-shim-fallback.sh` | Missing managed Node version warning and system fallback. |
| `04-node-package-scripts.sh` | Node provider script aliases and lockfile manager detection. |

## CI checks

The CI workflow should run:

```bash
cargo build --workspace
cargo test --workspace
bash scripts/check-changelog.sh
```

Before enforcing format or clippy as blocking release gates, run and normalize:

```bash
cargo fmt
cargo clippy --workspace --all-targets
```
