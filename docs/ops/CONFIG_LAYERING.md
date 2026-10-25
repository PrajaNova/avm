# Hierarchical config, profiles, and personal overrides

AVM loads `.avm.json` from the current directory and its ancestors. It merges
`aliases`, `env`, and `tools` by key; a nearer directory wins. Global
`~/.avm.json` supplies the lowest-precedence defaults. `trusted_paths` and
`idiomatic_version_files` are settings of the global base config only.

Within **each directory**, precedence runs from lowest to highest:

1. `.avm.json`
2. `.avm.<profile>.json` for each selected profile, in the listed order
3. `.avm.local.json`

All layers in a child directory override all layers in a parent directory.
For example, `repo/apps/tv/.avm.json` overrides `repo/.avm.local.json`.
Keys absent from the child are inherited from the parent.

Set `AVM_ENV=ci,staging` to load both `.avm.ci.json` and `.avm.staging.json`;
staging wins on shared keys. Use `avm --env staging which node` or
`avm env --env production` for a single invocation. For an alias, use
`avm --env staging run build`. `--env` overrides
`AVM_ENV`, and `--env ''` selects no profiles. Names contain letters, digits,
underscores, and hyphens; paths and empty comma-separated entries are rejected.
The name `local` always refers to the personal layer at highest precedence.

A base config can mark the config traversal boundary:

```json
{
  "root": true,
  "tools": { "node": "22" },
  "env": { "MODE": "development" },
  "aliases": { "build": "npm run build" }
}
```

This directory's layers are included, and JSON configs above it are excluded.
Global defaults still apply. Only the base `.avm.json` controls this boundary.
Existing `.env`, idiomatic version-file, and lockfile traversal rules continue
independently. Explicit JSON tool pins override idiomatic version-file pins.

`avm init` adds `.avm.local.json` to `.gitignore`, preserving existing entries.
Use this file for personal settings. Config-editing commands (`add`, `env add`,
and `<tool> use`) write the current directory's base `.avm.json`, or the global
base with `-g`. `upgrade --bump` asks you to edit inherited and profile pins at
their source instead of silently creating a different pin in the child config.

## Trust and origins

Trust is checked for every contributing file. Untrusted aliases and environment
variables are skipped; tool pins still apply. Review and trust parent files
explicitly, for example `avm trust ../../.avm.json`. `avm trust` in a directory
includes the base config, personal config, selected profile files, and `.env`.
Use `avm --env production trust` to include a production profile, or pass the
file path directly. Editing any file requires trusting its new content.

`avm which build`, `avm which node`, and `avm list` show the exact origin of each
winning value. Alias suggestions include trusted aliases from every layer.

## Shell setup and cache

The existing `eval "$(avm-bin shell-init)"` setup remains the way to activate AVM.
Its prompt hooks use the same layered resolution as aliases and plain shimmed
tools. They run when the directory changes and after `avm` commands (before one only if
the directory changed since the last apply), not on every Bash/PowerShell prompt. Set `AVM_ENV` in the shell to switch the prompt and shim profile together.
`avm env --env production` only prints the exports for that invocation; select
`AVM_ENV=production` for subsequent commands to use the same profile.

Merged JSON configuration is cached under `~/.avm/cache/config-<key>.json`
(or `$AVM_HOME/cache`). Cache keys include cwd, global home, profile order, and
`AVM_TRUST_ALL`. Each hit checks file mtimes, sizes, content hashes, symlink
targets, and the trust store. Missing candidates are included so creating or
removing an override invalidates the cache. Content hashes prevent stale trust
when an edit restores the old mtime. Broken or unwritable caches fall back to
normal resolution. `prune` preserves tool pins in inactive profiles of tracked
projects as well, so switching profiles does not discard their runtimes. Cache files use the existing private atomic-write helper.
Installed runtime selection, lockfiles, package scripts, provider environment,
and idiomatic version files are read afresh.

Run the end-to-end warm-cache benchmark with:

```sh
cargo test -p avm-cli --release cached_env_prompt_benchmark -- --ignored --nocapture
```

It runs 100 `avm env` subprocesses after warming a six-directory config chain,
prints median and p95 wall time, and checks the median against the ~5 ms target.
Timing includes process startup and varies by machine. Tool provider subprocesses
can add overhead when tools are selected.

Local macOS measurement with six individually trusted config files: **4.04 ms
median, 9.99 ms p95** (100 subprocesses, release build). The median meets the
~5 ms goal; tail latency and selected tool providers can exceed it.
