# Releasing

Every release is started by hand. Pushing a commit or a tag never
publishes anything; CI (tests) is the only workflow that runs on its own.

## avm

1. On a branch, set the version (no `v`) in `package.json`,
   `crates/avm-cli/Cargo.toml` and `crates/avm-plugin-sdk/Cargo.toml`, run
   `cargo build` to refresh `Cargo.lock`, and move the CHANGELOG
   `[Unreleased]` entries under `## [<version>] - <date>`. Merge to `main`.
2. Run the release, from `main`:
   - GitHub: **Actions → AVM release → Run workflow**, enter the version, or
   - CLI: `gh workflow run avm-release.yml -R PrajaNova/avm -f version=0.4.0`
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
same version: `gh workflow run avm-release.yml -R PrajaNova/avm -f version=0.4.0 -f npm_only=true`
(or tick **npm_only** in the Run workflow form). Trusted publishing
requires `package.json` `repository.url` to match `PrajaNova/avm` exactly,
case included.

### Secrets and one-time setup

| Needed for | What | Where |
| --- | --- | --- |
| Homebrew | `HOMEBREW_TAP_GITHUB_TOKEN`: a token with push access to `prajanova/homebrew-tap` | repo → Settings → Secrets → Actions |
| npm | **Trusted publishing**, no secret: on npmjs.com, open `@prajanova/avm` → Settings → Trusted Publisher → GitHub Actions, organization `PrajaNova`, repository `avm`, workflow `avm-release.yml`, environment empty. Allowed actions: `npm stage publish` only (the workflow never publishes directly) | npmjs.com |

## First-party plugins

1. Bump `version` in `plugins/avm-plugin-<name>/Cargo.toml`, refresh the
   root `Cargo.lock`, and merge to `main`.
2. Run the matching workflow in `PrajaNova/avm`:
   **Actions → Plugin node release / Plugin java release / Plugin python release / Plugin android
   release → Run workflow**, or:
   `gh workflow run plugin-node-release.yml -R PrajaNova/avm -f version=0.3.2`
   (swap `node` for `java`/`python`/`android` in the workflow name as needed).
3. Nothing else: avm installs the newest `avm-plugin-<name>-v*` release, so the registry
   needs no change for a new plugin version. (Releases of avm before this behaviour still
   install the `release_tag` pinned in the frozen `marketplace/registry.json`; bump it
   there only if you still need those older clients to pick up the new plugin.)

The workspace workflow builds linux amd64/arm64, macOS arm64/Intel and
Windows, with checksums and attestations. Plugin releases have namespaced
tags and are marked as GitHub prereleases to keep `/releases/latest`
pointing to the CLI. `avm plugin install <name>` looks the name up in
`registry.json` (name to repo and description), then installs
the newest `avm-plugin-<name>-v*` release from that repo (its latest release when
the repo has no such tags). A new plugin needs one registry entry; a new version
of an existing plugin needs none, since the registry stores no versions.
Third-party `owner/repo` installs skip the registry.

### Registry file

`registry.json` is `{ "schema_version": 1, "plugins": [{ "name": "<name>", "description": "...", "section_label": "...", "repo": "<owner>/<repo>" }] }`.
Only `name`, `repo` and `description` matter to current avm (optional `tags` feed `plugin search`);
`section_label` is for the website. There are no release pins: avm reads the release from GitHub. Older avm
releases fetch `marketplace/registry.json` instead, a frozen copy that keeps `release_tag` pins
(`scripts/check-registry.sh` keeps its plugin list in step with this file).
The CLI fetches the file from `main` (override with `AVM_REGISTRY_URL`, a URL or a file path),
so merging an entry is all it takes to publish a plugin.

The old `avm-plugin-node`/`avm-plugin-java`/`avm-plugin-android` repositories
have been deleted now that their code and release history live in this
workspace.

## Third-party plugins

Standalone repositories can continue calling the reusable
`plugin-ci.yml` and `plugin-release.yml` workflows; they keep `v<version>`
tags and stable releases.

## Marketplace site

**Actions → Web deploy → Run workflow** in `PrajaNova/avm`
(tick *production* for the live site, otherwise it publishes a preview URL),
or run `gh workflow run web-deploy.yml -R PrajaNova/avm -f production=true`.
Merging to `main` doesn't deploy. The workflow builds `web/` on the
runner and uploads it with the Vercel CLI; the Vercel project has no Git
connection and its Root Directory is left empty. Repo secrets required:
`VERCEL_TOKEN`, `VERCEL_ORG_ID`, `VERCEL_PROJECT_ID`. The site is served at
`https://getavm.dev/`.

The old `avm-marketplace` repository has been deleted; `registry.json`
in this repo is the only registry.
