# Workflows

One CI per component, each triggered **only** by changes to that component (native `paths:` filters,
on pull requests and on pushes to `main`), plus manual release/deploy workflows.

| Workflow | Runs when | What it does |
| --- | --- | --- |
| `avm-ci.yml` | `crates/**`, `Cargo.*`, installers, `scripts/**`, `CHANGELOG.md`, `registry.json` change | format, clippy (`-D warnings`), tests on Linux/macOS/Windows, release build, repo checks, Docker and Windows end-to-end suites |
| `web-ci.yml` | `web/**`, the registry, CLI command definitions change | registry check, docs-coverage check, build |
| `plugin-<name>-ci.yml` (node, java, python, android) | `plugins/avm-plugin-<name>/**`, `crates/avm-plugin-sdk/**`, `Cargo.*` change | format, clippy, tests, release build + `manifest` smoke test (Linux, macOS, Windows) |
| `action-ci.yml` | `action/**` changes | smoke test of the GitHub Action on four runners |
| `avm-release.yml` | manual | build, GitHub release, npm, Homebrew |
| `plugin-<name>-release.yml` | manual | release one plugin (calls `plugin-release.yml`) |
| `web-deploy.yml` | manual | build and deploy the site to Vercel (preview, or production) |
| `live-e2e.yml` | manual | install the *published* avm and run the user journey on Linux, macOS and Windows |
| `plugin-ci.yml`, `plugin-release.yml` | called by others | reusable; also called by plugin repositories outside this one, so keep their names and inputs stable |

A PR that touches only `web/` runs only Web CI; one that touches only `plugins/avm-plugin-node/` runs
only Plugin node CI. `main` has no required status checks, so a workflow that does not run does not
block a merge.
