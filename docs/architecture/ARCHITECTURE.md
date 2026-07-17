# avm Architecture

`avm` is a Rust-native CLI for project-local aliases, runtime selection, shims, and plugins.

## Workspace crates

| Crate | Responsibility |
| --- | --- |
| `crates/avm-cli` | Binary entrypoint, command routing, shell protocol, and user-facing behavior. |
| `crates/avm-core` | `.avm.json` parsing, config migration, local/global merge rules, alias/env/tool resolution. |
| `crates/avm-shims` | Shim directory management and executable shim generation. |
| `crates/avm-plugin-api` | Shared plugin manifest, alias response, and version/install provider contracts. |
| `crates/avm-runtime` | Plugin discovery, manifest validation, timeout handling, and legacy executable adapter. |
| `crates/avm-plugin-node` | Built-in Node provider for package scripts and Node version lookup. |

## Config model

Primary config file:

```json
{
  "aliases": {
    "dev": "pnpm run dev"
  },
  "env": {
    "NODE_ENV": "development"
  },
  "tools": {
    "node": "20.11.1"
  }
}
```

Legacy flat-map files are still accepted:

```json
{
  "dev": "pnpm run dev"
}
```

Resolution order:

1. Local `.avm.json`
2. Global `~/.avm.json`
3. Plugin/provider aliases
4. System command fallback

## Runtime boundaries

- CLI commands stay in `avm-cli`.
- Config and resolver logic stays in `avm-core`.
- Provider contracts stay in `avm-plugin-api`.
- Built-in Node behavior stays in `avm-plugin-node`.
- External plugin execution and compatible asdf adapters stay in `avm-runtime`.
- Shim creation and path handling stays in `avm-shims`.

## Plugin command behavior

User-facing runtime commands are plugin-first:

```bash
avm node versions
avm node 20 versions
avm node latest versions
avm node use 20.11.1
avm node install 20.11.1
avm plugin add https://github.com/halcyon/asdf-java.git
avm java versions
avm java latest versions
avm java use 21.0.1
avm java install 21.0.1
```

Internally, built-in providers and compatible asdf plugins implement a common provider contract so the CLI can call any plugin through the same version/install API.

`avm` does not auto-install missing tools during plain command execution. If a configured Node version is missing during shim execution, avm warns and falls back to the next matching system binary outside the avm shim directory.

Interactive version selection and `avm node use <version>` install a missing Node version first, then write the selected local/global version to `.avm.json`.

## Plugin behavior

The runtime supports executable-style plugins through the compatibility adapter. It also supports compatible asdf-style tool plugins that expose `bin/list-all` and `bin/install`; plugin directories named like `asdf-java` are surfaced as the `java` provider.

New provider work should use host-owned contracts first and keep plugin failures isolated from the main CLI command.
