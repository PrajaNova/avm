# AVM Workspace Instruction
## Quality and engineering standards
- Rust 2021, deterministic crate boundaries.
- No panics in runtime logic.
- Use typed errors (`anyhow` for command/application boundaries, `thiserror` for domain structs).
- Keep functions small, single-purpose, and dependency-injected where practical.
- Respect plugin boundaries and timeouts:
  - plugin discovery and execution paths should enforce timeouts.
  - malformed plugin output must not crash the CLI.
- Avoid noisy diagnostics unless `AVM_DEBUG=1`.
- Keep changes deterministic (sorted output for aliases/tools, stable ordering for lists).

## Folder boundaries
- `crates/avm-cli`: command parsing, shell protocol, user-facing behavior.
- `crates/avm-core`: `.avm.json`, resolve/merge rules, aliases, env, tools.
- `crates/avm-shims`: shim generation and PATH integration scripts.
- `crates/avm-plugin-api`: plugin/host contracts and version/install provider traits.
- `crates/avm-runtime`: plugin loading, manifest validation, plugin execution and adapter behavior.
- `crates/avm-plugin-node`: merged Node plugin for scripts, versions, and install contract.

## Plugin and compatibility policy
- Keep a plugin compatibility adapter during v1 rollout.
- New provider contracts live in host-first form; legacy executable plugins remain supported until v1.1 parity is proven.
- New provider loading should isolate failures by command and continue fallback flow.

## Release/runtime bridge
- Keep `bin/avm-bin.js` as the npm entrypoint wrapper.
- Keep shell behavior backward compatible:
  - `avm shell-init` prints wrapper script.
  - plain commands (`node`, `npm`, etc.) should be intercepted through shims.
  - no changes to shell behavior should be shipped without updating `shell-init` docs.

## Workspace docs
- Store migration and rollout artifacts under `docs`
