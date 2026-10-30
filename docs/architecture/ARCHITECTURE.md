# avm Architecture

`avm` is a Rust-native CLI for project-local aliases, runtime selection,
shims, and a runtime plugin marketplace.

## Workspace crates

| Crate | Responsibility |
| --- | --- |
| `crates/avm-cli` | The `avm-bin` binary. Modules: `cli/` (command routing, shell protocol), `config`/`resolver` (`.avm.json` parsing, local/global merge rules, alias/env/tool resolution), `shims` (shim generation, PATH lookup), `runtime` (plugin discovery, the protocol host runner `PluginProcess`, the marketplace installer, the legacy asdf compatibility adapter). |
| `crates/avm-plugin-sdk` | The `ToolProvider` trait, the plugin wire-protocol types (`protocol` module), and the `runner` module every plugin's `main.rs` calls. This is the one crate a plugin repo depends on. |

**Nothing else is compiled into `avm-bin`.** node, java, python, and android are
workspace members under `plugins/` with local API dependencies
([avm-plugin-node](https://github.com/PrajaNova/avm/tree/main/plugins/avm-plugin-node),
[avm-plugin-java](https://github.com/PrajaNova/avm/tree/main/plugins/avm-plugin-java),
[avm-plugin-python](https://github.com/PrajaNova/avm/tree/main/plugins/avm-plugin-python),
[avm-plugin-android](https://github.com/PrajaNova/avm/tree/main/plugins/avm-plugin-android)),
fetched at runtime the same way a third-party plugin would be. See
[Creating a plugin](../plugins/CREATING_A_PLUGIN.md) to add one.

## Config model

```json
{
  "aliases": { "dev": "pnpm run dev" },
  "env": { "NODE_ENV": "development" },
  "tools": { "node": "20.11.1" }
}
```

Legacy flat-map files (`{"dev": "pnpm run dev"}`) are still accepted and
migrated on read.

Resolution order: local `.avm.json` → global `~/.avm.json` → plugin/provider
aliases → system command fallback.

`avm exec` (`avm x`) and `avm run --with` overlay temporary tool selections
in memory. They use the same providers to resolve and install versions, apply
provider and trusted config env, and prepend the selected executable directories
to the child PATH. They never save pins or lockfiles. `exec` replaces the process
on Unix; other platforms wait and return the command's exit code.

`avm -p install` installs all effective tool selections without changing pins.
`avm plugin lock` resolves project selections, excluding personal global pins, and
updates the nearest existing lock or creates one next to the nearest project
config. Resolution uses the nearest current or ancestor lock;
changed selections invalidate the lock rather than silently changing versions.
`avm -p install --frozen` also requires a lock and download metadata for the host.
The host checks native plugin versions and recorded executable hashes, then
passes the original locked asset to `install-locked`. The plugin downloads
and verifies the archive before extraction.
Android and asdf plugins support
ordinary installs but lack the optional locked-download protocol.

## The plugin marketplace

`avm plugin install <name>` is the only install path — nothing works out of
the box. A bare name is looked up in `marketplace/registry.json` (name to repo
and description), then resolved from that repo's GitHub releases:

```mermaid
flowchart TD
  A["avm plugin install node"] --> B{"Bare name?"}
  B -->|Yes| C["Registry: node to repo,\nthen newest tag avm-plugin-node-v*\n(else the repo's latest release)"]
  B -->|"owner/repo"| D["Use that repo's latest release"]
  B -->|"git URL or path"| F["git-clone / copy (asdf-style plugin)"]
  C --> H["Find asset avm-plugin-<name>_<os>_<arch>.tar.gz"]
  D --> H
  H --> I["Download, verify sha256, extract (compiled binary, never source)"]
  I --> J["Install to ~/.avm/plugins/avm-plugin-<name>/bin/avm-plugin"]
```

`avm plugin search <keyword>` lists the same first-party release names plus GitHub
repos with the `avm-plugin` topic. `avm plugin uninstall <name>` removes a plugin;
re-running `avm plugin install <name>` is the update path.

`provider_by_name` (`crates/avm-cli/src/global/provider_handler.rs`) is the
single lookup every command goes through, in two tiers:

1. `PluginManager::protocol_provider` — a marketplace-installed plugin
   under `~/.avm/plugins/avm-plugin-<name>/bin/avm-plugin`.
2. `PluginManager::asdf_provider` — the legacy compatibility adapter, for
   community asdf-style plugins (`bin/list-all`, `bin/install`, ...) that
   haven't adopted the native protocol. Permanent fallback tier, not
   temporary.

If neither resolves the name, the error tells you to run
`avm plugin install <name>`.

## The plugin protocol

Every provider — first-party or third-party — is a standalone executable
speaking one JSON-over-stdio contract:
`avm-plugin-<name> <command> [args...]`, one JSON document on stdout per
"read" command (`manifest`, `versions`, `is-installed`,
`installed-versions`, `executable-path`, `env-vars`); `install`/`uninstall`
instead inherit stdio (so progress streams live) and signal success via
exit code alone. `PluginProcess` (`avm-cli/src/plugins/`) is the host-side runner:
it spawns the executable, enforces a timeout, and parses the JSON —
malformed output is a normal `Err`, never a panic.

Full protocol reference and a build-your-own-plugin walkthrough:
[Creating a plugin](../plugins/CREATING_A_PLUGIN.md).

## Runtime boundaries

- CLI commands stay in `avm-cli/src/cli`.
- Config and resolver logic stays in `avm-cli/src/{config,resolver}.rs`.
- Provider contracts and the wire protocol stay in `avm-plugin-sdk`.
- Plugin discovery, the protocol host runner, the marketplace installer,
  and the asdf adapter stay in `avm-cli/src/plugins/`.
- Shim creation and PATH handling stay in `avm-cli/src/core/shims/`.

`package.json` script aliases are read in-process (`common/state.rs`), not via
the node plugin: it's a hot path (alias resolution runs on every command).

## Global packages across local version switches

A binary installed by a global package manager invocation (e.g.
`npm install -g <pkg>` under the globally-pinned Node) stays reachable even
when a *different* local version is active in a project. Shim resolution
(`resolve_managed_binary` in `crates/avm-cli/src/shims/shims_handler.rs`)
tries the local pin's bin directory first, then falls back to the global
pin's, for any binary that isn't itself a pinned tool name — so `node`
itself always resolves to whichever version is actually selected
(local-over-global), while a global-only tool like a globally-installed CLI
package is found via the fallback.
