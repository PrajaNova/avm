# Changelog

All notable changes to `avm` are recorded here.

The format follows Keep a Changelog style, and releases use semantic versioning.

## [Unreleased]

### Fixed
- A broken or BOM-prefixed `package.json` no longer stops `node`, shims or the prompt; scripts are only read by `run`, `resolve`, `which` and `list`, which warn and continue.
- Aliases and plugin commands run through one `avm-bin run` call; the shell wrappers take their built-in lists from the CLI definition (`alias` was missing before).
- Bash and PowerShell refresh project env when the directory changes (like zsh) instead of at every prompt.
- `avm shims activate` appends to shell startup files instead of rewriting them, so non-UTF-8 files are no longer wiped.
- `avm prune` no longer holds the project-tracking lock while waiting for confirmation, so shims and prompts don't hang; it aborts if a project was tracked meanwhile.
- Plugin updates keep the working binary until the new one is in place; archives are staged in a private dir instead of shared `/tmp` and cleaned on every exit.
- Cache plugin `env-vars` per plugin build and skip the tracking lock for known projects, cutting per-prompt work; `prune` clears month-old cache files.
- Shims are written atomically and left alone when unchanged.
- Android plugin uses the shared home-dir lookup (Windows `USERPROFILE`) and announces license acceptance.
- Reject tool pins and removal arguments that escape managed directories.
- Preserve existing PowerShell profiles and malformed configuration files.
- Serialize config, trust, project-tracking and runtime-install writes; replace JSON atomically.
- Stop pruning when tracked selections cannot be resolved; keep fuzzy global pins.
- Resolve fuzzy JSON pins and share version selection across install, use and exec.
- Reject malformed version ranges and handle numeric bounds without panicking.
- Terminate timed-out plugin descendants and bound output-pipe waits.
- Pass package script names as literal arguments and restore shell environment after project changes.
- Merge Windows environment keys without case collisions and find temporary tools/package managers on the child's PATH.

### Testing
- Test native Node/Java frozen installs and archive tampering on Linux, macOS and Windows (ZIP), using plugins built from the checkout.
- Run live Node/Java/Android download suites with the checkout's plugins on Linux and Windows.

### Added
- Bare `avm` on a terminal opens a picker of the project's aliases and tools. Pickers use the alternate screen, close with Esc, support PageUp/PageDown/Home/End and wrap, show each alias's command and source, mark installed and selected versions, and offer local or global pinning without an existing `.avm.json`.
- CI runs `cargo fmt --check` and `cargo clippy -D warnings`.
- Hierarchical `.avm.json` configs merge parent values with nearest-directory precedence and support `root: true`. `which` and `list` show exact origins; merged config caching keeps prompt resolution fast.
- `.avm.local.json` personal overrides and `.avm.<profile>.json` profiles selected with `AVM_ENV` or `--env` (comma-separated, later profiles win). `init` gitignores personal overrides, and each file requires its own trust.
- `avm exec` (alias `avm x`) runs commands with temporary `tool@version` selections, auto-installing missing runtimes without changing pins or lockfiles. `avm run --with` applies the same overrides to aliases; `--no-install` uses installed versions only.
- `avm install` installs all selected tools without rewriting pins. `avm lock` writes sorted exact versions, plugin identities and per-platform download checksums to `.avm.lock`; `avm install --frozen` rejects missing or stale locks. Node and Java plugins support verified locked downloads; Android and legacy asdf plugins remain available through ordinary installs.

## [0.4.1] - 2026-10-07

### Changed
- The documentation site and plugin registry now live in `marketplace/` in this repository. The CLI reads the new registry path, and the manual Pages workflow publishes the site at `https://prajanova.github.io/avm/`.

- First-party Node, Java and Android plugins now share the AVM Cargo workspace under `plugins/`, with local API dependencies and independent namespaced releases published to `PrajaNova/avm` through the **Release node/java/android plugin** workflows. Marketplace entries can select a release with `release_tag`.

### Fixed
- Windows `shell-init`: `avm-bin env --shell pwsh` returning nothing (normal when no tool needs env vars yet) crashed shell setup on every fresh install — an empty string piped into `Invoke-Expression` threw a pipeline binding error that bypassed the surrounding try/catch.
- `install.ps1`: creating the PowerShell profile failed when its parent directory didn't exist yet; profile setup failures no longer abort the install (the binary is already in place and on PATH by that point).

## [0.4.0] - 2026-09-29

### Added
- GitHub API calls (plugin installs, `outdated`, `self-update`) send `GITHUB_TOKEN`/`GH_TOKEN` when set, avoiding the shared 60/hour unauthenticated limit.

### Changed
- README is now a short overview that links to the website for commands, configuration, version files, security and the comparison; contributor docs (code layout, e2e suites) moved to CONTRIBUTING.md.

### Fixed
- `avm plugin outdated` said "not from the marketplace" when GitHub couldn't be reached; it now says so.
- `avm outdated`/`upgrade` sorted Java builds wrong: `openjdk-17+35` (17.0.0) ranked above `17.0.2`. The `+N` build is now a tiebreaker, not a version component.
- `avm prune` no longer removes versions avm has never seen used (e.g. everything installed before this release) unless you pass `--include-unrecorded`; it reports how many it skipped.

## [0.4.0-beta-3] - 2026-09-29

### Added
- `avm outdated [--json]` and `avm plugin outdated` (#26): pinned tools with the newest version in each pin's range and overall, and plugins against their latest releases.
- `avm upgrade [tool…]` (#26): installs the newest in-range version and moves `.avm.json` pins; `--bump` goes to the latest across majors (with confirmation), `--dry-run` previews. Version files (`.nvmrc`, …) are never rewritten.
- `avm prune` (#27): removes installed versions that neither a global pin nor a project avm has seen (recorded in `~/.avm/tracked-configs.json`) uses. `--older-than 90d` uses per-version last-use (shims mark it daily), `--dry-run` lists sizes, and it warns about versions holding global npm packages.

## [0.4.0-beta-2] - 2026-09-28

### Added
- `avm self-update [--version X]` (#28): updates install.sh/install.ps1 installs in place from the GitHub release, sha256-verified before the atomic swap (on Windows the running exe is moved aside, then shims are relinked). Homebrew, npm, cargo and Scoop installs print their own upgrade command instead.
- Update notice (#28): after interactive commands, at most once a day, a one-line stderr notice when a newer release exists. The check runs in a detached background process, so commands never wait on the network. Never shown in pipes, CI, shims, `avm env`, or with `AVM_NO_UPDATE_CHECK=1`.
- Windows, phases 2–3 (#23): shims are `node.exe`/`npm.exe`/… hard links to `avm-bin.exe` (it dispatches on the name it was started as), so IDEs and debuggers that launch `node.exe` directly are covered; `.cmd` shims are removed. A Windows e2e suite (`e2e/windows.ps1`, run by CI on every PR) installs with `install.ps1` and drives real node/java/android installs through the `.exe` shims; `install.ps1` accepts `AVM_DOWNLOAD_BASE` (URL or local folder). Managed tools resolve `.exe`/`.cmd` binaries. `avm plugin add` installs `avm-plugin-<name>_windows_amd64.zip` releases; the reusable plugin release workflow builds them with `windows: true`. Each avm release publishes a Scoop manifest (`avm.json`).

### Changed
- Tests: one end-to-end harness, `e2e/run.sh`, replaces the Docker scenario suite and the Rust CLI integration tests. It builds avm from the checkout, installs it with `install.sh` on a clean Ubuntu container, and runs the core, node, java and android suites; CI runs it on every PR. `install.sh` accepts `AVM_DOWNLOAD_BASE` for alternate archive locations.
- The version flag is `-v`/`--version` (was clap's `-V`/`--version`), in avm-bin and the sh/bash/zsh and PowerShell hooks. `-V` is no longer accepted.
- Releases are manual only: `Release` workflows (avm and plugins) run from Actions → Run workflow with a version input and create the tag themselves; tag pushes no longer publish. npm supports trusted publishing (OIDC) or `NPM_TOKEN`. See `docs/ops/RELEASE.md`.

### Fixed
- Errors now show their cause (e.g. `couldn't update …: checksum mismatch …`) instead of only the outermost message.
- Windows: plugins failed with `HOME not set` when only `USERPROFILE` exists; avm now passes its home to plugins as `HOME`.
- Windows: refreshing shims from inside a shim (after `npm i -g`) deleted `npm.exe` and skipped the new package's shim. Shims now link to the real avm-bin, in-use shims are moved aside, and one failure no longer stops the rest. A failed post-install refresh now prints a warning.
- The Homebrew release job read a non-existent `HOMEBREW_TAP_TOKEN` secret; it now uses `HOMEBREW_TAP_GITHUB_TOKEN`.
- npm trusted publishing was refused because `package.json` `repository.url` said `prajanova/avm`; npm compares it case-sensitively with `PrajaNova/avm`. The release workflow gains an `npm_only` input to retry npm for an existing release.
- npm releases are staged (`npm stage publish`) and go public only after a maintainer approves them with 2FA.

## [0.4.0-beta-1] - 2026-09-28

### Added
- `avm trust` (`--list`, `--revoke`): a project's `.avm.json` aliases/env and its `.env` files are ignored until trusted; edits made outside avm re-block them. `trusted_paths` globs in the global config and `AVM_TRUST_ALL=1` trust without a hash (#20).
- Version files projects already have now pin tools: `.tool-versions`, `.nvmrc`, `.node-version`, `package.json` (`volta.node`, `engines.node` ranges), `.java-version`, `.sdkmanrc`. The nearest directory wins, `.avm.json` `tools` beats them, and partial specs (`20`, `lts/*`, `>=18 <21`) resolve to the newest installed match. `avm which` shows the origin file; `"idiomatic_version_files": false` in the global config turns the tool-specific ones off (#21).
- Intel macOS (`darwin_amd64`) builds for avm-bin and, via the reusable workflow, every plugin; `install.sh`, npm and Homebrew install them. Release builds smoke-test each binary. A plugin with no build for the host now lists the platforms it does have (#22).
- Windows, phase 1 (#23): `avm_windows_amd64.zip` release builds, `install.ps1` (sha256-verified), npm on `win32`; `avm shell-init pwsh`; `avm env --shell sh|pwsh|cmd`; aliases run via `cmd /C`; `.cmd` shims with `PATHEXT` lookup; `%USERPROFILE%` as home when `HOME` is unset. CI runs `cargo test` and a PowerShell smoke test on `windows-latest`. Windows plugin assets and an `avm-shim.exe` dispatcher are still to come.
- Reusable plugin release workflow publishes `checksums.txt` (sha256) alongside platform archives.
- `avm plugin add`/`update` verify the downloaded archive's sha256 against the release's `checksums.txt` before extracting; a mismatch aborts with nothing installed. Releases without `checksums.txt` are refused unless `AVM_ALLOW_UNVERIFIED=1`. The verified hash is recorded in the plugin's `meta.json`.
- `AVM_GITHUB_API_URL` overrides the GitHub API base used for marketplace installs.
- avm releases publish `checksums.txt`; `install.sh` and the npm installer verify `avm-bin` against it before extracting (`AVM_ALLOW_UNVERIFIED=1` to skip).
- avm and plugin release workflows attach GitHub build provenance attestations (`gh attestation verify`).

### Changed
- README: the comparison table now covers mise and proto, has corrected asdf (Go) and vfox (Go/Lua) facts with sources, and links avm's gaps to roadmap issues. `agent.md`, `agent.skill.md` and `llm.text` are merged into `llms.txt` (#24).
- Aliases always run via `sh -c`; `avm resolve` prints the expanded shell command.
- `avm env` quotes values only when needed.
- `avm create <name>` now prints the `gh repo create --template PrajaNova/avm-plugin-template` command instead of scaffolding files.
- `avm shims reshim` is an alias of `avm shims install`.
- `avm-core`, `avm-shims`, and `avm-runtime` folded into `avm-cli` as modules; `avm-cli` no longer links `avm-plugin-node`.

### Removed
- Hidden `avm tool` compatibility command (use `avm <plugin> ...`), `avm all`, `avm version` (use `avm --version`), and `avm env --format`.
- Legacy alias-only plugins (`plugin.json` + `bin/export-aliases`).
- Installers no longer create an empty `~/.avm.json`; a missing global config is treated as empty.

### Fixed
- Homebrew formula test called the removed `avm-bin version`; it now uses `--version`.

## [0.3.0] - 2026-09-23

### Added
- Decentralized runtime plugin marketplace architecture (`avm plugin add`, `avm plugin remove`, `avm plugin list`, `avm plugin update`) with prebuilt binary asset resolution.
- Native Android SDK and Java Temurin providers operating over the typed wire protocol in `avm-plugin-api`.
- Real-time interactive version picker with type-to-search filtering.
- Dedicated `avm alias` and `avm env` namespaces (add, remove, list) matching `avm plugin`, plus root alias invocation.
- Plugin-defined custom subcommands beyond fixed protocol verbs.

### Fixed
- In-place overwrite handling on macOS to avoid Gatekeeper execution and quarantine races when updating plugins.
- Fixed `EXDEV` cross-device file system move errors during plugin installation.
- Preserved shims-first PATH order in `avm env` to prevent path shadowing.
- Fixed pipe buffer deadlocks in `run_with_timeout` for plugins with large stdout streams.

## [0.2.8] - 2026-07-17

### Added
- `avm shims reshim` and automatic reshimming after `npm`/`yarn`/`pnpm`/`bun` install commands, so globally-installed package binaries (e.g. `tsc`, `eslint`) become runnable without a manual step.
- `avm shims activate` to persist `~/.avm/shims` onto PATH in `.zshenv`/`.bashrc`/`.profile`, so directory-aware resolution works in closed environments that reset PATH.
- Interactive next-action menu for bare `avm <plugin>` (e.g. `avm java`) that chains into the chosen action, uniform across every plugin.

### Fixed
- Shim dispatch now resolves any binary across managed versions (local pin → global pin → other tools), so a globally-installed tool runs regardless of the active project version.

### Changed
- `avm shims install` now also reshims installed versions.
- Removed dead duplicated resolver methods.

## [0.2.7] - 2026-05-13

### Added
- Rust workspace release candidate for npm and Homebrew publishing.
- Shell-mode alias execution for chained commands, pipes, redirects, command substitution, globs, and environment expansion.
- Install pinning flow for provider versions, including local/global auto-pin behavior and `--no-pin`.
- Timeout handling for Node archive downloads/extraction and git-backed plugin install/update operations.
- Recovery path for malformed `.avm.json` files by backing up broken config and continuing with an empty config.

### Changed
- Aligned npm package and Rust binary versions for release automation.
- `avm node install` can resolve `latest` and major versions before installing.

### Added
- Rust CLI workspace with `avm-cli`, `avm-core`, `avm-shims`, `avm-runtime`, `avm-plugin-api`, and `avm-plugin-node`.
- Docker-based test harness and Rust integration test coverage.
- User-facing LLM and agent onboarding docs.

### Changed
- Package scope moved to `@prajanova/avm`.
- Project name updated to Any Version Manager.

## [0.2.6] - 2026-05-13

### Added
- Baseline Rust rewrite structure.
- Node provider direction for package script discovery and Node version resolution.
- Shim model for plain command interception.

### Changed
- Replaced the legacy project layout with Rust workspace boundaries.
- Updated npm package ownership and repository links to Prajanova.
