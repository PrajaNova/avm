# avm — Any Version Manager

**One version manager for Node, Java and the Android SDK.** avm picks the
right runtime for every project, and runs the project's aliases and env too.
It's a single Rust binary for macOS, Linux and Windows.

**📖 Docs, guides and full command reference: [prajanova.github.io/avm-marketplace](https://prajanova.github.io/avm-marketplace/)**

```console
~/shop-app $ node -v
v20.19.5
~/shop-app $ avm which node
tool 'node': 20.19.5 (from ./.nvmrc)
```

## Why avm

- **Works with the files you already have:** `.nvmrc`, `.node-version`, `.tool-versions`, `.java-version`, `.sdkmanrc` and `package.json` engines.
- **Global npm tools survive version switches:** install a CLI once, use it in projects pinned to other Node versions.
- **Safe by default:** avm, plugins and runtimes are sha256-verified before install, and a cloned repo's aliases and env stay off until you run `avm trust`.
- **Android SDK, build tools and emulator** in one install, next to Node and Java.
- **Everywhere:** macOS (Apple Silicon and Intel), Linux (x64 and arm64), and Windows (PowerShell, with `.exe` shims your IDE can run).
- **Stays current:** `avm outdated`, `avm upgrade` and `avm self-update`.

## Install

```bash
brew install prajanova/tap/avm                                              # macOS / Linux
curl -fsSL https://raw.githubusercontent.com/PrajaNova/avm/main/install.sh | bash  # macOS / Linux
npm install -g @prajanova/avm                                               # any OS with Node
```

```powershell
irm https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | iex  # Windows
```

Then add the shell hook ([details for each shell](https://prajanova.github.io/avm-marketplace/#/docs/guide/shell-setup)):

```bash
eval "$(avm-bin shell-init)"   # in ~/.zshrc or ~/.bashrc
```

## Quick start

```bash
avm plugin add node        # plugins are compiled binaries, verified on install
avm node use 22            # pin Node 22 for this project (installs it if needed)
avm add dev "npm run dev"  # a project alias → run it with `avm dev`
```

## Learn more

- [Getting started](https://prajanova.github.io/avm-marketplace/#/docs/guide/getting-started) · [All commands](https://prajanova.github.io/avm-marketplace/#/docs/manage/commands) · [Configuration](https://prajanova.github.io/avm-marketplace/#/docs/manage/configuration)
- [Version files](https://prajanova.github.io/avm-marketplace/#/docs/manage/version-files) · [Security & trust](https://prajanova.github.io/avm-marketplace/#/docs/manage/security) · [Comparison with asdf, mise, proto](https://prajanova.github.io/avm-marketplace/#/docs/manage/comparison)
- [Plugin marketplace](https://prajanova.github.io/avm-marketplace/#/marketplace) · [What's new](https://prajanova.github.io/avm-marketplace/#/changelog) · [Write a plugin](docs/plugins/CREATING_A_PLUGIN.md)

## Contributing

Issues and PRs are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup,
the code layout and the end-to-end tests, and [SECURITY.md](SECURITY.md) to
report a vulnerability.

## License

[MIT](LICENSE)

## Rust workspace

The CLI and first-party plugins share one Cargo workspace:

```text
crates/avm-cli
crates/avm-plugin-api
plugins/avm-plugin-node
plugins/avm-plugin-java
plugins/avm-plugin-android
```

```bash
cargo build --workspace
cargo test --workspace
cargo build -p avm-plugin-node  # one plugin
```

Plugins remain separate executables; the CLI discovers them at runtime.
See [Contributing](CONTRIBUTING.md) and [Releasing](docs/ops/RELEASE.md).
