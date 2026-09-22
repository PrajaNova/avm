/// `avm create <name>` — scaffold a new avm plugin project, the same shape
/// as avm-plugin-node/-java/-android (a `ToolProvider` impl + a thin
/// `main.rs` wrapping it via `avm_plugin_api::runner`), ready to build,
/// customize, and publish. Same spirit as `brew create` or `cargo new`: it
/// produces a working skeleton, not a finished plugin — the TODOs in
/// `src/lib.rs` are the actual work.
fn cmd_create(args: CreateArgs) -> Result<()> {
    validate_plugin_name(&args.name)?;

    let base = match args.path {
        Some(path) => path,
        None => std::env::current_dir().context("failed to read current directory")?,
    };
    let dir_name = format!("avm-plugin-{}", args.name);
    let target = base.join(&dir_name);

    if target.exists() {
        return Err(anyhow!("{} already exists", target.display()));
    }

    let struct_name = pascal_case(&args.name);

    fs::create_dir_all(target.join("src")).context("failed to create src directory")?;
    fs::create_dir_all(target.join(".github").join("workflows"))
        .context("failed to create .github/workflows directory")?;

    write_scaffold_file(
        &target.join("Cargo.toml"),
        &scaffold_cargo_toml(&args.name),
    )?;
    write_scaffold_file(
        &target.join("src").join("lib.rs"),
        &scaffold_lib_rs(&args.name, &struct_name),
    )?;
    write_scaffold_file(
        &target.join("src").join("main.rs"),
        &scaffold_main_rs(&args.name, &struct_name),
    )?;
    write_scaffold_file(&target.join("README.md"), &scaffold_readme(&args.name))?;
    write_scaffold_file(&target.join(".gitignore"), "/target\n")?;
    write_scaffold_file(
        &target.join(".github").join("workflows").join("ci.yml"),
        &scaffold_ci_yml(&args.name),
    )?;
    write_scaffold_file(
        &target.join(".github").join("workflows").join("release.yml"),
        &scaffold_release_yml(&args.name),
    )?;

    println!("✓ Created {}", target.display());
    println!();
    println!("Next steps:");
    println!("  1. cd {}", target.display());
    println!("  2. Implement the TODOs in src/lib.rs (available_versions, install, ...)");
    println!("     — see https://github.com/PrajaNova/avm-plugin-node for a real example.");
    println!("  3. cargo build && ./target/debug/avm-plugin-{} manifest    # sanity check", args.name);
    println!("  4. git init && git add -A && git commit -m \"Initial plugin\"");
    println!("  5. Push to a new GitHub repo (gh repo create <you>/{dir_name} --public --source=. --push)");
    println!("  6. Tag a release: git tag v0.1.0 && git push origin v0.1.0");
    println!("     — this builds and publishes avm-plugin-{}_<os>_<arch>.tar.gz for", args.name);
    println!("       linux_amd64, linux_arm64, and darwin_arm64 automatically.");
    println!("  7. Open a PR adding an entry to registry.json in");
    println!("     https://github.com/PrajaNova/avm-marketplace so `avm plugin add {}` finds it.", args.name);
    Ok(())
}

fn validate_plugin_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("plugin name required"));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid || name.starts_with('-') || name.ends_with('-') {
        return Err(anyhow!(
            "invalid plugin name '{name}' — use lowercase letters, digits, and hyphens only"
        ));
    }
    Ok(())
}

fn pascal_case(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn write_scaffold_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn scaffold_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "avm-plugin-{name}"
version = "0.1.0"
edition = "2021"
description = "avm plugin for {name}"
license = "MIT"

[dependencies]
# Points at avm's main branch — the shared protocol contract
# (ToolProvider, the wire types, and the runner every plugin's main.rs
# calls). Pin to a tag/rev instead if you need a specific avm-plugin-api
# version.
avm-plugin-api = {{ git = "https://github.com/PrajaNova/avm" }}
anyhow = "1.0"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
"#
    )
}

fn scaffold_lib_rs(name: &str, struct_name: &str) -> String {
    format!(
        r#"use anyhow::{{anyhow, Result}};
use avm_plugin_api::{{ToolProvider, ToolVersion, ToolVersionQuery}};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct {struct_name}Provider;

impl {struct_name}Provider {{
    pub fn new() -> Self {{
        Self
    }}

    fn tools_root(&self) -> Result<PathBuf> {{
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME not set"))?;
        Ok(PathBuf::from(home).join(".avm").join("tools").join("{name}"))
    }}

    fn bin_path_for(&self, version: &str, binary: &str) -> Result<Option<PathBuf>> {{
        let candidate = self.tools_root()?.join(version).join("bin").join(binary);
        Ok(candidate.exists().then_some(candidate))
    }}
}}

impl ToolProvider for {struct_name}Provider {{
    fn name(&self) -> &str {{
        "{name}"
    }}

    fn is_installed(&self, version: &str) -> bool {{
        self.bin_path_for(version, "{name}").ok().flatten().is_some()
    }}

    fn installed_versions(&self) -> Result<Vec<String>> {{
        // TODO: scan tools_root() for installed version directories.
        // See avm-plugin-node/src/lib.rs::installed_versions for a real
        // example (fs::read_dir + filter on is_installed).
        Ok(Vec::new())
    }}

    fn available_versions(&self, query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {{
        // TODO: fetch the real list of installable versions — an
        // index.json, a GitHub Releases API, whatever this tool publishes.
        // ToolVersionQuery is Recent / Latest / Major(n); match on it to
        // decide how much to return. See avm-plugin-node/src/versions.rs
        // (nodejs.org/dist/index.json) or avm-plugin-java/src/versions.rs
        // (the foojay Disco API) for two real, working examples.
        let _ = query;
        Ok(Vec::new())
    }}

    fn executable_path(&self, version: &str) -> Result<Option<PathBuf>> {{
        self.bin_path_for(version, "{name}")
    }}

    fn env_vars(&self, _version: &str) -> Result<HashMap<String, String>> {{
        // TODO: return any env vars this tool needs when selected, e.g.
        // JAVA_HOME / ANDROID_HOME (see avm-plugin-java or
        // avm-plugin-android for real examples). Most tools don't need
        // any — if that's you, delete this whole method and the default
        // (empty map) applies.
        Ok(HashMap::new())
    }}

    fn install(&self, version: &str) -> Result<()> {{
        // TODO: download and install `version` into
        // tools_root()?.join(version), so executable_path/is_installed
        // above find it afterward. Shell out to `curl`/`tar` via
        // std::process::Command rather than adding an HTTP client
        // dependency — see avm-plugin-node/src/install.rs for the pattern
        // (download, extract, move into place).
        let _ = version;
        Err(anyhow!("install not implemented yet"))
    }}

    fn uninstall(&self, version: &str) -> Result<()> {{
        let target = self.tools_root()?.join(version);
        if target.exists() {{
            std::fs::remove_dir_all(target)?;
        }}
        Ok(())
    }}
}}
"#
    )
}

fn scaffold_main_rs(name: &str, struct_name: &str) -> String {
    format!(
        r#"use avm_plugin_api::{{runner, Manifest}};
use avm_plugin_{name}::{struct_name}Provider;
use std::process::ExitCode;

fn main() -> ExitCode {{
    let manifest = Manifest {{
        name: "{name}".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: Some(1),
        description: Some("TODO: one line describing what this plugin manages".to_string()),
        section_label: Some("{struct_name}".to_string()),
        homepage: Some("https://github.com/<you>/avm-plugin-{name}".to_string()),
    }};
    runner::run(manifest, &{struct_name}Provider::new())
}}
"#
    )
}

fn scaffold_readme(name: &str) -> String {
    format!(
        r#"# avm-plugin-{name}

An [avm](https://github.com/PrajaNova/avm) plugin for {name}.

Scaffolded with `avm create {name}` — replace this README once
`src/lib.rs`'s TODOs are filled in and the shape below matches reality.

## Install

```bash
avm plugin add {name}
```

## Commands

Once installed, everything is under `avm {name}`:

| Command | What it does |
| --- | --- |
| `avm {name}` | Interactive menu |
| `avm {name} list` | Show the selected and installed versions |
| `avm {name} versions` | Browse recent installable versions |
| `avm {name} <major> versions` | Versions on one major/feature line |
| `avm {name} latest versions` | Just the newest version |
| `avm {name} use <version> [-g\|--global]` | Select an installed version |
| `avm {name} install <version\|latest\|N>` | Install (if missing) + auto-pin |
| `avm {name} uninstall <version>` | Remove a managed version |

## Release process

Tag `vX.Y.Z` to trigger `.github/workflows/release.yml`, which builds
`avm-plugin-{name}_<os>_<arch>.tar.gz` for `linux_amd64`, `linux_arm64`, and
`darwin_arm64` and publishes them as a GitHub Release — that's what `avm
plugin add {name}` downloads. See
[PrajaNova/avm-marketplace](https://github.com/PrajaNova/avm-marketplace)
for how to get this plugin listed under a bare name instead of a full
repo URL, and the main [avm repo](https://github.com/PrajaNova/avm)'s
`docs/plugins/CREATING_A_PLUGIN.md` for the full protocol reference.
"#
    )
}

fn scaffold_ci_yml(_name: &str) -> String {
    r#"name: CI

on:
  pull_request:
  push:
    branches:
      - main

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release
"#
    .to_string()
}

fn scaffold_release_yml(name: &str) -> String {
    format!(
        r#"name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target_os: linux
            target_arch: amd64
          - os: ubuntu-24.04-arm
            target_os: linux
            target_arch: arm64
          - os: macos-latest
            target_os: darwin
            target_arch: arm64
    runs-on: ${{{{ matrix.os }}}}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build release binary
        run: cargo build --release

      - name: Create release archive
        shell: bash
        run: |
          archive="avm-plugin-{name}_${{{{ matrix.target_os }}}}_${{{{ matrix.target_arch }}}}.tar.gz"
          mkdir -p dist
          cp target/release/avm-plugin-{name} dist/avm-plugin-{name}
          tar -C dist -czf "$archive" avm-plugin-{name}
          echo "ARCHIVE=$archive" >> "$GITHUB_ENV"

      - name: Upload archive
        uses: actions/upload-artifact@v4
        with:
          name: ${{{{ env.ARCHIVE }}}}
          path: ${{{{ env.ARCHIVE }}}}

  github-release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download release archives
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Publish GitHub release
        uses: softprops/action-gh-release@v2
        with:
          files: dist/*.tar.gz
          generate_release_notes: true
"#
    )
}
