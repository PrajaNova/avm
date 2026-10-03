# AVM Documentation & Marketplace

The official plugin registry and documentation portal for [avm (Any Version Manager)](https://github.com/PrajaNova/avm).

`avm-cli` fetches [`registry.json`](registry.json) from `marketplace/registry.json` on the AVM repo's `main` branch (no server, no API — just a file over `raw.githubusercontent.com`) to turn a short plugin name into a GitHub repo, then fetches that repo's **latest GitHub Release** (a prebuilt native binary, never source) for the current platform.

This directory also contains the modern **avm Documentation & Marketplace Web Portal**, deployable to GitHub Pages at `https://prajanova.github.io/avm/`.

---

## Adding a plugin to the marketplace

Open a PR adding an entry to `registry.json`:

```json
{
  "name": "kotlin",
  "description": "One line describing what it manages",
  "section_label": "Kotlin",
  "repo": "yourname/avm-plugin-kotlin"
}
```

Requirements for `repo`:

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
  `avm_plugin_api::runner` / `avm_plugin_api::protocol` in
  [PrajaNova/avm](https://github.com/PrajaNova/avm), or any of the
  `avm-plugin-{node,java,android}` repos, for the reference shape.

`avm plugin add <name>` works the moment your repo has one matching release
published — no changes needed on the avm side beyond the registry entry.

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
cd marketplace

# Install dependencies
pnpm install --frozen-lockfile

# Start local development server
pnpm dev

# Build for production
pnpm build

# Preview production build locally
pnpm preview
```

### GitHub Pages Deployment

CI builds the site on every PR. Deployment is manual via [`.github/workflows/deploy.yml`](../.github/workflows/deploy.yml) in `PrajaNova/avm`.

To enable GitHub Pages in your repository settings:
1. Navigate to **Settings > Pages** in your GitHub repository.
2. Under **Build and deployment > Source**, select **GitHub Actions**.
3. After merging to `main`, run **Actions → Deploy Documentation & Marketplace to GitHub Pages → Run workflow**, or `gh workflow run deploy.yml -R PrajaNova/avm`.
