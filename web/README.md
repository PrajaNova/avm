# AVM Documentation & Marketplace

The official plugin registry and documentation portal for [avm (Any Version Manager)](https://github.com/PrajaNova/avm).

`avm plugin install <name>` resolves a plugin from GitHub Releases (a prebuilt native binary, never source): names are looked up in `registry.json` here (`{ "<name>": { "repo", "desc", "tags" } }`), which also feeds this site; third-party plugins install via `avm plugin install <owner>/<repo>`.

This directory also contains the modern **avm Documentation & Marketplace Web Portal**, deployable to GitHub Pages at `https://getavm.dev/`.

---

## Publishing a plugin

Add the `avm-plugin` topic to your GitHub repo so `avm plugin search` finds it.

Requirements for the repo:

- Its GitHub Releases must include an asset named
  `avm-plugin-<name>_<os>_<arch>.tar.gz` for each platform you support —
  `<os>` is `linux` or `darwin`, `<arch>` is `amd64` or `arm64` (matching
  avm's own release convention, including Intel Macs as `darwin_amd64`;
  the reusable release workflow builds all four).
- Each archive contains exactly one file: the plugin executable, named
  `avm-plugin-<name>`.
- The release also includes `checksums.txt` — `sha256sum` output over the
  archives. avm verifies the archive against it before extracting and
  refuses releases without it (unless the user sets
  `AVM_ALLOW_UNVERIFIED=1`). The reusable
  `PrajaNova/avm/.github/workflows/plugin-release.yml` generates it (plus a
  build provenance attestation) for you.
- The executable speaks avm's plugin protocol (JSON-over-stdio commands:
  `manifest`, `versions`, `is-installed`, `installed-versions`,
  `executable-path`, `env-vars`, `install`, `uninstall`) — see
  `avm_plugin_sdk::runner` / `avm_plugin_sdk::protocol` in
  [PrajaNova/avm](https://github.com/PrajaNova/avm), or any of the
  `avm-plugin-{node,java,android}` repos, for the reference shape.

`avm plugin install <name>` works the moment your repo has one matching release
published: `avm plugin install <owner>/<repo>`.

---

## Documentation & Marketplace Web Portal

The web portal is a modern, responsive React + TypeScript + Vite + Tailwind CSS application inspired by `brew.sh`. It features:

- **Interactive Install Hero** (curl, brew, npm, cargo tabs with 1-click copy)
- **Live Plugin Explorer** querying `registry.json` dynamically
- **Interactive Terminal Preview** demonstrating workflows
- **Comprehensive Guide** for `.avm.json`, shell shims, and runtime resolution
- **Full CLI Command Reference** with real examples
- **Plugin Authoring Guide** with code samples and workflow definitions

### Development

```bash
# From the AVM repository root
cd web

# Install dependencies
pnpm install --frozen-lockfile

# Start local development server
pnpm dev

# Check public CLI coverage, guide routes and JSON examples
pnpm check:docs

# Build for production
pnpm build

# Preview production build locally
pnpm preview
```

### GitHub Pages Deployment

CI checks documentation coverage and builds the site on every PR. Command entries live in `src/data/cliReference.ts`, feature guides in `src/data/featureGuides.ts`, and navigation in `src/components/docs/DocsSidebar.tsx`. Update them alongside CLI changes. Deployment is manual via [`.github/workflows/release-web.yml`](../.github/workflows/release-web.yml) in `PrajaNova/avm`.

To enable GitHub Pages in your repository settings:
1. Navigate to **Settings > Pages** in your GitHub repository.
2. Under **Build and deployment > Source**, select **GitHub Actions**.
3. After merging to `main`, run **Actions → Release web (marketplace) → Run workflow**, or `gh workflow run release-web.yml -R PrajaNova/avm`.
