# Creating an avm plugin

A plugin is a **standalone executable** that manages one tool's versions
(install, list, switch, env vars) and speaks a small JSON-over-stdio
protocol so `avm-bin` can drive it without linking against it. Nothing is
compiled into `avm-bin` — first-party plugins (node, java, android) work
exactly the same way a third-party one does.

## Quick start

```bash
avm create kotlin
cd avm-plugin-kotlin
```

This scaffolds a working (if unimplemented) plugin: `Cargo.toml`, a
`ToolProvider` skeleton in `src/lib.rs` with `TODO`s, a `main.rs` wired to
the protocol runner, a `README.md`, and `.github/workflows/{ci,release}.yml`
that build and publish it on a tag push. It builds and runs immediately —
`cargo build && ./target/debug/avm-plugin-kotlin manifest` prints a valid
(if placeholder) manifest before you've written a line of logic.

The rest of this doc is what to fill in, and how it all fits together.

## The `ToolProvider` trait

Everything a plugin does is one Rust trait
(`avm_plugin_api::ToolProvider`, from the
[main avm repo](https://github.com/PrajaNova/avm)):

```rust
pub trait ToolProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_installed(&self, version: &str) -> bool;
    fn installed_versions(&self) -> anyhow::Result<Vec<String>>;
    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>>;
    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>>;
    fn env_vars(&self, version: &str) -> anyhow::Result<HashMap<String, String>> { Ok(HashMap::new()) }
    fn install(&self, version: &str) -> anyhow::Result<()>;
    fn uninstall(&self, version: &str) -> anyhow::Result<()>;
}
```

| Method | Called when | Notes |
| --- | --- | --- |
| `available_versions` | `avm <tool> versions`, `<major> versions`, `latest versions` | `ToolVersionQuery` is `Recent` / `Latest` / `Major(n)` — filter your version source accordingly. This is almost always where you fetch a live index (see [Real examples](#real-examples)) rather than hardcoding a list. |
| `install` | `avm <tool> install <version>` | Download + unpack into `~/.avm/tools/<name>/<version>/`, so a later `executable_path`/`is_installed` finds it. Shell out to `curl`/`tar` via `std::process::Command` — don't add an HTTP client dependency, it's the one thing every existing plugin agrees on for staying lightweight. |
| `is_installed` / `installed_versions` / `executable_path` | version listing, shim resolution | Pure filesystem checks under your tool's own `~/.avm/tools/<name>/` — no network. |
| `env_vars` | `avm env`, every shim exec | Return only what needs exporting for the selected version (e.g. `JAVA_HOME`). Compute it directly from the version string — **never shell out and diff two captured environments**; that's the exact bug ([see history](#why-the-protocol-looks-like-this)) that started this whole redesign. Most tools don't need any env vars at all — the trait's default (empty map) is correct for those; just don't implement the method. |
| `uninstall` | `avm <tool> uninstall <version>` | Remove `~/.avm/tools/<name>/<version>/`. |

Your `main.rs` wraps an instance of your provider with the shared runner:

```rust
use avm_plugin_api::{runner, Manifest};

fn main() -> std::process::ExitCode {
    let manifest = Manifest {
        name: "kotlin".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: Some(1),
        description: Some("Kotlin compiler versions".to_string()),
        section_label: Some("Kotlin".to_string()),
        homepage: Some("https://github.com/you/avm-plugin-kotlin".to_string()),
    };
    runner::run(manifest, &KotlinProvider::new())
}
```

`runner::run` handles argv parsing and JSON serialization for you — you
never touch the wire format directly unless you're implementing the
protocol in a language other than Rust (see below).

## The wire protocol

This is what `runner::run` speaks on your behalf, and what `avm-bin`'s
`PluginProcess` (the host-side runner) calls. Documented in full because
the protocol is plain JSON over stdio — a plugin doesn't have to be Rust.

Invocation: `<plugin-executable> <command> [args...]`. "Read" commands
print one JSON document to stdout; `install`/`uninstall` instead inherit
stdio (so download/extract progress streams live to the user) and signal
success purely via exit code — no JSON envelope for those two.

| Command | Args | stdout (JSON) |
| --- | --- | --- |
| `manifest` | — | `{"name","version","api_version","description","section_label","homepage"}` |
| `versions` | `--query recent\|latest\|major:<N>` | `{"versions":[{"version","label","channel","is_lts","is_security"}]}` |
| `is-installed` | `<version>` | `{"installed": bool}` |
| `installed-versions` | — | `{"versions": [string]}` |
| `executable-path` | `<version>` | `{"path": string\|null}` |
| `env-vars` | `<version>` | `{"env": {string: string}}` |
| `install` | `<version>` | *(no JSON — stdio inherited, exit code only)* |
| `uninstall` | `<version>` | *(no JSON — stdio inherited, exit code only)* |

A minimal manual test without `avm` involved at all:

```bash
cargo build
./target/debug/avm-plugin-kotlin manifest
./target/debug/avm-plugin-kotlin versions --query recent
./target/debug/avm-plugin-kotlin is-installed 1.9.20
./target/debug/avm-plugin-kotlin install 1.9.20   # progress streams to your terminal
./target/debug/avm-plugin-kotlin is-installed 1.9.20   # should now be true
```

## Testing through avm itself

Point avm at your local build directly — no publish step needed yet:

```bash
mkdir -p ~/.avm/plugins/avm-plugin-kotlin/bin
cp target/debug/avm-plugin-kotlin ~/.avm/plugins/avm-plugin-kotlin/bin/avm-plugin
avm kotlin versions
avm kotlin install 1.9.20
avm kotlin use 1.9.20
```

That's exactly the layout a real marketplace install produces
(`~/.avm/plugins/avm-plugin-<name>/bin/avm-plugin`) — `avm`'s discovery
doesn't distinguish "installed by hand for testing" from "installed via
`avm plugin add`".

## Publishing

1. **Push to GitHub.** Any repo name works, but `avm-plugin-<name>` is the
   convention every first-party plugin and the marketplace's directory
   naming (`asdf-<name>` → `<name>`, mirrored here) assumes.
2. **Tag a release**: `git tag v0.1.0 && git push origin v0.1.0`. The
   scaffolded `.github/workflows/release.yml` builds
   `avm-plugin-<name>_<os>_<arch>.tar.gz` for `linux_amd64`, `linux_arm64`,
   and `darwin_arm64`, and publishes them as GitHub Release assets. This
   is the **required contract** — `avm plugin add` queries your repo's
   `GET /repos/<owner>/<repo>/releases/latest` and looks for an asset
   named exactly `avm-plugin-<name>_<os>_<arch>.tar.gz`, containing exactly
   one file, `avm-plugin-<name>`. It always fetches a compiled release —
   never source — so anyone installing your plugin needs no Rust
   toolchain, same as `avm plugin add node` today.
3. **List it in the marketplace** (optional but recommended): open a PR
   adding an entry to `registry.json` in
   [PrajaNova/avm-marketplace](https://github.com/PrajaNova/avm-marketplace)
   — `{"name", "description", "section_label", "repo"}`. Once merged,
   `avm plugin add <name>` works with just the bare name instead of a full
   URL, and it shows up in `avm plugin available`.
4. Without a marketplace entry, people can still install your plugin via
   `avm plugin add <path-or-url>` — same asset-naming contract applies,
   avm just resolves the repo from the URL you gave it instead of a
   registry lookup.

## Real examples

Three working, published plugins to read as reference implementations —
each does something genuinely different (a live JSON index, a multi-vendor
API, a several-GB SDK install with env vars), so between them they cover
most of what a new plugin needs:

| Plugin | What's interesting about it |
| --- | --- |
| [avm-plugin-node](https://github.com/PrajaNova/avm-plugin-node) | Simplest full example — `curl`+parse a version index, `curl`+`tar` install. Also shows the one legitimate exception to "plugins are standalone": `avm-cli` links it as a *library* too, for in-process `package.json` script parsing (not part of the `ToolProvider` protocol surface — a separate concern that happens to live in the same crate). |
| [avm-plugin-java](https://github.com/PrajaNova/avm-plugin-java) | Version index from a third-party aggregator API (foojay Disco), filtered to one vendor (Temurin). `env_vars` returns `JAVA_HOME`. |
| [avm-plugin-android](https://github.com/PrajaNova/avm-plugin-android) | The most involved `install`: multiple SDK components via `sdkmanager`, a JDK dependency check, wrapper scripts written to `bin/` so `adb`/`sdkmanager`/`avdmanager`/`emulator` all resolve correctly. `env_vars` returns both `ANDROID_HOME` and `ANDROID_SDK_ROOT`. |

## Why the protocol looks like this

Early in avm's history, "java" and "android" support worked by wrapping
`asdf`-style bash plugins, including an `env_vars` implementation that
sourced a `bin/exec-env` script in bash and diffed the resulting
environment against a baseline to find what changed. That approach has a
fatal flaw: once a variable like `ANDROID_HOME` is exported into the
user's shell once, every later diff sees it already present in *both* the
baseline and the "after sourcing" runs, and reports no change — so
`avm env` silently stopped exporting it, permanently, for that shell
session. The fix wasn't "no subprocess" (every plugin here still forks a
process) — it's "the plugin computes and returns its env vars directly, as
data, with no diffing step for ambient state to poison." That's the
`env-vars` protocol command, and it's why `env_vars` implementations
should always compute the value from the version string, never read
today's environment to decide what to report.

The legacy asdf adapter (`bin/list-all`, `bin/install`, ...) still exists
in `avm-runtime` as a fallback tier for community plugins that haven't
adopted this protocol — it's not going away, just no longer what avm's own
first-party tools use.
