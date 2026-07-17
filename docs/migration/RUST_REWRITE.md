# Rust Rewrite Migration

## Current direction

`avm` is now Rust-first. The legacy runtime, command folders, and plugin implementation are no longer part of the active source tree.

## Migration rules

- Keep `.avm.json` compatibility.
- Support both structured and legacy flat-map configs.
- Preserve local-first then global precedence.
- Keep Node package script discovery.
- Use shims for plain binary interception.
- Do not auto-install tools during command execution.

## Replacement map

| Old responsibility | New Rust location |
| --- | --- |
| CLI commands | `crates/avm-cli` |
| Config parsing | `crates/avm-core` |
| Alias/env/tool resolver | `crates/avm-core` |
| Plugin manifest/types | `crates/avm-plugin-api` |
| Plugin runtime | `crates/avm-runtime` |
| asdf-compatible provider behavior | `crates/avm-runtime` |
| Node plugin behavior | `crates/avm-plugin-node` |
| Shell/binary interception | `crates/avm-shims` |

## Compatibility status

| Area | Status |
| --- | --- |
| Legacy flat `.avm.json` | Supported |
| Structured `.avm.json` | Supported |
| Local/global aliases | Supported |
| Local/global env | Supported |
| Local/global tools | Supported |
| Node script aliases | Supported |
| Shim fallback | Supported |
| Node install/download | Supported |
| asdf plugin install/download | Supported through compatible plugin scripts |
| External executable plugins | Compatibility adapter |

## Acceptance tests

The migration acceptance suite is:

```bash
docker/tests/run-docker-tests.sh
```
