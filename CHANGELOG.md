# Changelog

All notable changes to `avm` are recorded here.

The format follows Keep a Changelog style, and releases use semantic versioning.

## [Unreleased]

### Added
- Reusable plugin release workflow publishes `checksums.txt` (sha256) alongside platform archives.
- `avm plugin add`/`update` verify the downloaded archive's sha256 against the release's `checksums.txt` before extracting; a mismatch aborts with nothing installed. Releases without `checksums.txt` are refused unless `AVM_ALLOW_UNVERIFIED=1`. The verified hash is recorded in the plugin's `meta.json`.
- `AVM_GITHUB_API_URL` overrides the GitHub API base used for marketplace installs.
- avm releases publish `checksums.txt`; `install.sh` and the npm installer verify `avm-bin` against it before extracting (`AVM_ALLOW_UNVERIFIED=1` to skip).
- avm and plugin release workflows attach GitHub build provenance attestations (`gh attestation verify`).

### Changed
- Aliases always run via `sh -c`; `avm resolve` prints the expanded shell command.
- `avm env` quotes values only when needed.
- `avm create <name>` now prints the `gh repo create --template PrajaNova/avm-plugin-template` command instead of scaffolding files.
- `avm shims reshim` is an alias of `avm shims install`.
- `avm-core`, `avm-shims`, and `avm-runtime` folded into `avm-cli` as modules; `avm-cli` no longer links `avm-plugin-node`.

### Removed
- Hidden `avm tool` compatibility command (use `avm <plugin> ...`), `avm all`, `avm version` (use `avm --version`), and `avm env --format`.
- Legacy alias-only plugins (`plugin.json` + `bin/export-aliases`).
- Installers no longer create an empty `~/.avm.json`; a missing global config is treated as empty.

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
