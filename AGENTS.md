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
- `crates/avm-cli/src/cli`: command parsing, shell protocol, user-facing behavior.
- `crates/avm-cli/src/{config,resolver}.rs`: `.avm.json`, resolve/merge rules, aliases, env, tools.
- `crates/avm-cli/src/shims.rs`: shim generation and PATH integration scripts.
- `crates/avm-plugin-api`: plugin/host contracts, `ToolProvider` trait, wire protocol types (`protocol` module), and the `runner` module every plugin executable's `main.rs` dispatches through.
- `crates/avm-cli/src/runtime.rs`: plugin discovery (protocol/asdf tiers), `PluginProcess` (the protocol host runner), the marketplace installer, and the legacy asdf adapter.
- `crates/avm-plugin-node`, `crates/avm-plugin-java`, `crates/avm-plugin-android`: each both a library (the `ToolProvider` impl + version/install logic) and a standalone `[[bin]]` executable speaking the plugin protocol — see `docs/migration/PLUGIN_PROTOCOL.md`.

## Plugin and compatibility policy
- Every provider — first-party or third-party — speaks the same JSON-over-stdio plugin protocol (`docs/migration/PLUGIN_PROTOCOL.md`) and is discovered in tiers: builtin (bundled next to `avm-bin`) → user-installed third-party (`~/.avm/plugins/<dir>/bin/avm-plugin`) → legacy asdf adapter.
- Keep the asdf compatibility adapter (`AsdfToolProvider`) for community plugins that haven't adopted the native protocol; it's the permanent fallback tier, not a temporary v1 shim.
- "Host-first form" now means "speaks the plugin protocol," not "compiled into `avm-cli`" — no `ToolProvider` implementation should be linked directly into `avm-cli` going forward; wrap it in a plugin executable (`avm_plugin_api::runner::run`) instead, even for first-party tools.
- New provider loading should isolate failures by command and continue fallback flow.

## Release/runtime bridge
- Keep `bin/avm-bin.js` as the npm entrypoint wrapper.
- Keep shell behavior backward compatible:
  - `avm shell-init` prints wrapper script.
  - plain commands (`node`, `npm`, etc.) should be intercepted through shims.
  - no changes to shell behavior should be shipped without updating `shell-init` docs.

## Workspace docs
- Store migration and rollout artifacts under `docs`
