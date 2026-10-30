# Live end-to-end test

A real user journey against what is **published**, on a clean machine. Nothing is
built locally and nothing from this checkout is installed except the scenario script.

```sh
e2e/run.sh                        # on this machine (macOS or Linux), in a throwaway HOME
AVM_VERSION=latest e2e/run.sh     # the stable release (default: newest, prereleases included)
AVM_VERSION=v0.5.0 e2e/run.sh     # one specific release
npm run e2e:live                  # same as e2e/run.sh
```

CI runs it on every platform; there is no Docker image for it. `install.sh` installs to
`/usr/local/bin` when that is writable or sudo is passwordless (as on CI runners), which would
replace your own `avm-bin`, so `run.sh` refuses to run outside CI unless that is not the case or you set
`AVM_E2E_FORCE=1`. Prefer a disposable machine.

## Every platform, in GitHub Actions

`.github/workflows/live-e2e.yml` runs the same journey on a clean runner per platform (daily, on
demand with an optional version, and on PRs that touch `e2e/`):

| Platform | Runner | Runs |
| --- | --- | --- |
| Linux amd64 | `ubuntu-latest` | `e2e/run.sh` |
| Linux arm64 | `ubuntu-24.04-arm` | `e2e/run.sh` |
| macOS (self-hosted Mac, `SGPMac`) | `[self-hosted, macOS]` | `e2e/run.sh` |
| Windows | `windows-latest` | `pwsh -File e2e/live.ps1` |

`e2e/run.sh` uses a throwaway HOME so it is safe locally; `live.ps1` edits your PowerShell
profile and user PATH, so only run it on a disposable Windows machine.

## What it does (`live.sh`, as a normal user, ~1-2 minutes)

1. Installs avm with the published `install.sh` (checksum-verified release download).
2. Installs the `node` plugin from the live registry; checks list, and that an unknown plugin name is rejected.
3. Global alias and env var, run from anywhere (`avm hello`, `avm run`, `which`, `alias list`, `env`).
4. Creates `proj-a`, `proj-b` and `plain` folders; `proj-a`/`proj-b` get a local alias with the same
   name as the global one, an alias of their own and a local env var.
5. Checks which alias wins in each folder (local over global, subfolders inherit, no leaking
   between projects), what `which` and `resolve` report, and env precedence.
6. Real node installs: global 20, local 22 in `proj-a` only; `node --version`, `which node` and
   `avm node list` per folder.
7. Removes the local alias (falls back to the global one) and the global one (gone everywhere).
8. Wrong input: unknown alias, missing alias, typo'd subcommand, bad env name, bad node version.

The published CLI generation decides the plugin spelling (`plugin add|remove|available` or
`plugin install|uninstall|search`), detected at runtime, so the scenario stays valid when a new
release ships.

The Rust suite (`crates/avm-cli/tests`) is offline and tests this checkout; the Docker suite in
`crates/avm-cli/tests/shell` builds avm from the checkout. This one tests the shipped product.
