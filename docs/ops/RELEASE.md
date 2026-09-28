# Releasing

Every release is started by hand. Pushing a commit or a tag never
publishes anything; CI (tests) is the only workflow that runs on its own.

## avm

1. On a branch, set the version (no `v`) in `package.json`,
   `crates/avm-cli/Cargo.toml` and `crates/avm-plugin-api/Cargo.toml`, run
   `cargo build` to refresh `Cargo.lock`, and move the CHANGELOG
   `[Unreleased]` entries under `## [<version>] - <date>`. Merge to `main`.
2. Run the release, from `main`:
   - GitHub: **Actions → Release → Run workflow**, enter the version, or
   - CLI: `gh workflow run release.yml -R PrajaNova/avm -f version=0.4.0`
3. The workflow checks that the version matches `package.json`, that the
   CHANGELOG has it, and that the tag doesn't already exist. It then:
   - builds linux amd64/arm64, macOS arm64/Intel and Windows, and
     smoke-tests each binary,
   - creates the `v<version>` tag and GitHub release with `checksums.txt`
     and build provenance attestations,
   - **stages** the npm package (versions with a `-`, such as
     `0.4.0-beta-1`, target the `beta` dist-tag; everything else targets
     `latest`). It isn't public until a maintainer approves it with 2FA:
     npmjs.com → `@prajanova/avm` → staged versions → Approve, or
     `npm stage approve @prajanova/avm@<version>`,
   - updates the Homebrew formula in `prajanova/homebrew-tap`.

Follow a run with `gh run watch -R PrajaNova/avm`.

If only the npm step failed, fix the cause, then retry npm alone for the
same version: `gh workflow run release.yml -R PrajaNova/avm -f version=0.4.0 -f npm_only=true`
(or tick **npm_only** in the Run workflow form). Trusted publishing
requires `package.json` `repository.url` to match `PrajaNova/avm` exactly,
case included.

### Secrets and one-time setup

| Needed for | What | Where |
| --- | --- | --- |
| Homebrew | `HOMEBREW_TAP_GITHUB_TOKEN`: a token with push access to `prajanova/homebrew-tap` | repo → Settings → Secrets → Actions |
| npm | **Trusted publishing**, no secret: on npmjs.com, open `@prajanova/avm` → Settings → Trusted Publisher → GitHub Actions, organization `PrajaNova`, repository `avm`, workflow `release.yml`, environment empty. Allowed actions: `npm stage publish` only (the workflow never publishes directly) | npmjs.com |

## Plugins (avm-plugin-node, -java, -android, and third-party)

1. Bump `version` in the plugin's `Cargo.toml` and merge to `main`.
2. In the plugin repo: **Actions → Release → Run workflow** with that
   version, or `gh workflow run release.yml -R PrajaNova/avm-plugin-node -f version=0.2.1`.

The shared `PrajaNova/avm/.github/workflows/plugin-release.yml` checks the
version against `Cargo.toml`, builds linux amd64/arm64 and macOS
arm64/Intel, and publishes the tag, archives, `checksums.txt` and
attestations. `avm plugin add` always installs a plugin's latest release,
so publish plugins as normal releases, not prereleases.

## Marketplace site

**Actions → Deploy Documentation & Marketplace to GitHub Pages → Run
workflow** in `PrajaNova/avm-marketplace`. Merging to `main` doesn't
deploy.
