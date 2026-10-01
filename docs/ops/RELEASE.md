# Releasing

Every release is started by hand. Pushing a commit or a tag never
publishes anything; CI (tests) is the only workflow that runs on its own.

## avm

1. On a branch, set the version (no `v`) in `package.json`,
   `crates/avm-cli/Cargo.toml` and `crates/avm-plugin-api/Cargo.toml`, run
   `cargo build` to refresh `Cargo.lock`, and move the CHANGELOG
   `[Unreleased]` entries under `## [<version>] - <date>`. Merge to `main`.
2. Run the release, from `main`:
   - GitHub: **Actions → Release AVM CLI → Run workflow**, enter the version, or
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

## First-party plugins

1. Bump `version` in `plugins/avm-plugin-<name>/Cargo.toml`, refresh the
   root `Cargo.lock`, and merge to `main`.
2. Run **Actions → Release workspace plugins** in `PrajaNova/avm`, selecting the plugin
   and version, or:
   `gh workflow run release-plugins.yml -R PrajaNova/avm -f plugin=node -f version=0.3.2`.
3. After the release succeeds, update that plugin's marketplace entry:

   ```json
   {
     "name": "node",
     "description": "Node.js",
     "repo": "PrajaNova/avm",
     "release_tag": "avm-plugin-node-v0.3.2"
   }
   ```

The workspace workflow builds linux amd64/arm64, macOS arm64/Intel and
Windows, with checksums and attestations. Plugin releases have namespaced
tags and are marked as GitHub prereleases to keep `/releases/latest`
pointing to the CLI; the registry explicitly selects the plugin tag.
Update `release_tag` after each plugin release. Third-party entries without
`release_tag` continue to install their repository's latest stable release.

### Moving existing installations

Publish the CLI supporting `release_tag` before switching registry entries.
Keep existing registry entries on the old plugin repositories until the
corresponding workspace releases exist. Older CLI versions ignore the new
field and cannot install plugins from the shared repository: upgrade AVM
before using the switched entries. Existing installed plugins keep working;
`avm plugin update` uses the current registry entry.

The original local checkouts were preserved. Once the workspace is merged
and releases are migrated, the old repositories can be archived.

## Third-party plugins

Standalone repositories can continue calling the reusable
`plugin-ci.yml` and `plugin-release.yml` workflows; they keep `v<version>`
tags and stable releases.

## Marketplace site

**Actions → Deploy Documentation & Marketplace to GitHub Pages → Run
workflow** in `PrajaNova/avm-marketplace`. Merging to `main` doesn't
deploy.
