# avm-cli tests

Two suites drive the real `avm` binary:

| Suite | Where | Runs |
| --- | --- | --- |
| Rust | this folder (`main.rs` + modules) | `cargo test -p avm-cli --test cli` (offline, fake plugins, throwaway sandboxes) |
| Shell | `shell/suites/*.sh` | `shell/run.sh` in Docker (real Node/Java/Android downloads) |

## Running

From the repo root (`npm run` or `pnpm`, both work):

| Command | Runs |
| --- | --- |
| `npm run test:cli` | the Rust suite in this folder (offline, ~6 s) |
| `npm run test:cli -- alias::` | only tests whose path contains `alias::` (any module or test name) |
| `npm run test:unit` | the unit tests inside `src/` |
| `npm test` | everything: `cargo test --workspace` |
| `npm run test:bench` | the latency benchmarks (release build, prints a timing table) |
| `npm run lint` | clippy over all targets, tests included |
| `npm run e2e` | the Docker shell suite (needs Docker and network) |

## Rust suite layout (`tests/`)

One test binary (`main.rs`), one module per source feature (`crates/avm-cli/src/<feature>/`):

| Module | Covers |
| --- | --- |
| `alias` | `alias add/remove/list` (all spellings, scopes, errors), bare `avm <alias>`, reserved names, package.json scripts as aliases |
| `app` | `create`, `shell-init` (sh/pwsh/cmd, valid bash/zsh), the shell wrapper (bash, zsh) |
| `core` | config layering, profiles, trust, cache, corrupt-file safety, `.env` loading and trust |
| `env` | `env add/remove/list/ea`, key validation, export syntax per shell, plugin env caching, profile validation |
| `global` | `which`/`resolve`/`run`, placeholders, exit codes, `-v`/`--help`, bare `avm`, `plugin outdated/upgrade` |
| `plugins` | `plugin list/uninstall/install` arguments, version resolution, native plugin lock + offline install |
| `project` | `init`, `trust` (path/list/revoke), `plugin prune` (all flags), `plugin lock` / `install --frozen` (`project::lock`, unix) |
| `shims` | `shims path/install/reshim/remove/activate`, `exec-shim` |
| `tool` | `exec`/`x`/`run --with`, `exec` argument validation, `<tool> list/versions/install/use/set/uninstall` (unix) |
| `bench` | latency benchmarks (ignored by default, see below) |

Negative cases live next to the feature they exercise, in modules named for what goes wrong:
`global::usage_errors` (typos, missing and extra arguments, exit code 2), `plugins::invalid_plugin_names`,
`tool::provider_failures` (unmatched versions, failing/malformed plugins), `tool::plugin_timeouts`,
`project::lock_input`, `project::unwritable` (read-only files, skipped as root), plus `exec_arguments`,
`core::dotenv`, `env::vars` and `project::lock` (corrupted locks). Each asserts the specific message.

Not covered because they need the network: `plugin install <name|repo|url>`, `plugin search`,
`self-update`.

Shared code:

- `common/` is the harness. `Sandbox` is a temp `home/` + `project/` that is deleted on drop;
  `sandbox.run(&[..])` runs `avm-bin` with an isolated environment. Assertions are fluent:
  `assert_that(&out).succeeds().stdout_contains("x")`, `.fails_with("msg")`, `.exits_with(3)`.
- `fixtures/` is test data: `scripts/` holds the fake plugin scripts, `plugins.rs` installs
  fakes, `projects.rs` has the prebuilt `ExecProject` and `LockProject`, `native.rs` has the
  mock download server.

## Writing a test

Put it in the module of the feature it exercises. Every test has a doc comment saying what
behaviour is protected, then numbered steps that match the body:

```rust
/// `plugin prune` keeps versions pinned only by an inactive profile.
///
/// 1. Install node 20, 22 and 24; the project pins 20, its `ci` profile pins 22.
/// 2. A dry-run prune lists only 24.
#[test]
fn prune_keeps_inactive_profile_pins() { /* ... */ }
```

Several cases of one behaviour go in an `rstest` table (`#[case::name(..)]`), not a loop, so
each case reports separately. A new module needs a `mod` line in `main.rs`.

## Benchmarks

`bench.rs` times the hot paths (start-up, `env`, `which`/`resolve`/`run`, `alias add`, `plugin list`,
`exec-shim`, `shell-init`, 32 parallel runs, config depth scaling). They are `#[ignore]`d because
budgets only make sense on a quiet machine and a release build:

```sh
cargo test -p avm-cli --release --test cli -- --ignored --nocapture --test-threads=1 bench::
```

Each prints min / median / mean / p95 / max over many subprocess runs and fails when the median is
over its budget (budgets are for release; debug builds get 10x). Set `AVM_BENCH_OUT=<file>` to append
`name,median,p95,mean` CSV rows and compare across commits. `common/bench.rs` has the helper.
