# avm — Any Version Manager

**One version manager for Node, Java and the Android SDK.** avm picks the
right runtime for every project, and runs the project's aliases and env too.
It's a single Rust binary for macOS, Linux and Windows.

**📖 Docs, guides and full command reference: [prajanova.github.io/avm](https://prajanova.github.io/avm/)**

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

Then add the shell hook ([details for each shell](https://prajanova.github.io/avm/#/docs/guide/shell-setup)):

```bash
eval "$(avm-bin shell-init)"   # in ~/.zshrc or ~/.bashrc
```

Shell setup refreshes project environment variables at the Bash/PowerShell
prompt, on directory changes in zsh, and before and after `avm` commands.
Leaving a project restores each variable's original value, or unsets it if it
was originally absent. Re-run shell setup in existing sessions after upgrading.

The Windows installer also adds an `avm` command for `cmd.exe`. It forwards
CLI commands to the installed binary; automatic shell environment updates
still require the PowerShell hook.

## Quick start

```bash
avm plugin add node        # plugins are compiled binaries, verified on install
avm node use 22            # pin Node 22 for this project (installs it if needed)
avm add dev "npm run dev"  # a project alias → run it with `avm dev`
```

Try another runtime without changing project or global pins:

```bash
avm exec node@18 -- npm test
avm x node@22 java@21 -- ./gradlew build
avm run dev --with node@18 -- --port 3000
```

Missing versions install automatically; `--no-install` uses installed versions
only, including the newest installed match for fuzzy specs. Plugins must already
be installed. Repeat `--with` for multiple tools, before any alias arguments.
These commands inherit trusted project env and provider vars, prepend the
selected tools to PATH, and leave `.avm.json` and lockfiles unchanged. The shell
hook forwards `exec` and `x` directly to the CLI.

Install every tool selected by project, version-file and global pins:

```bash
avm install
avm lock --platform linux_amd64,darwin_arm64,windows_amd64
avm install --frozen       # CI: require the committed lockfile to match config
```

Commit `.avm.lock` to keep fuzzy versions on the same exact release. Locking
fetches download metadata without installing; locked downloads always verify
their SHA-256. A stale lock fails with instructions to run `avm lock`.
Node and Java support lockfiles with the updated plugins. Android and legacy
asdf plugins support ordinary `avm install`, but cannot yet be locked.

## Learn more

- [Getting started](https://prajanova.github.io/avm/#/docs/guide/getting-started) · [All commands](https://prajanova.github.io/avm/#/docs/manage/commands) · [Configuration](https://prajanova.github.io/avm/#/docs/manage/configuration)
- [Version files](https://prajanova.github.io/avm/#/docs/manage/version-files) · [Security & trust](https://prajanova.github.io/avm/#/docs/manage/security) · [Comparison with asdf, mise, proto](https://prajanova.github.io/avm/#/docs/manage/comparison)
- [Plugin marketplace](https://prajanova.github.io/avm/#/marketplace) · [What's new](https://prajanova.github.io/avm/#/changelog) · [Write a plugin](docs/plugins/CREATING_A_PLUGIN.md)

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
