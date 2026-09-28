# Changelog

All notable changes to `avm` are recorded here.

The format follows Keep a Changelog style, and releases use semantic versioning.

## [Unreleased]

### Changed
- Releases are manual only: `Release` workflows (avm and plugins) run from Actions → Run workflow with a version input and create the tag themselves; tag pushes no longer publish. npm supports trusted publishing (OIDC) or `NPM_TOKEN`. See `docs/ops/RELEASE.md`.

### Fixed
- The Homebrew release job read a non-existent `HOMEBREW_TAP_TOKEN` secret; it now uses `HOMEBREW_TAP_GITHUB_TOKEN`.

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
