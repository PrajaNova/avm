# Changelog

All notable changes to `avm` are recorded here.

The format follows Keep a Changelog style, and releases use semantic versioning.

## [Unreleased]

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
