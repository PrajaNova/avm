//! Performance benchmarks for the commands that run on every prompt, shim call or script.
//!
//! All tests are `#[ignore]`d because wall-clock budgets are only meaningful on a quiet machine
//! and a release build. Run them with:
//!
//! ```text
//! cargo test -p avm-cli --release --test cli -- --ignored --nocapture bench::
//! ```
//!
//! Each test prints `min / median / mean / p95 / max` over many subprocess runs and fails when
//! the median exceeds its budget. Budgets are for release builds; debug builds get 10x.
//! Set `AVM_BENCH_OUT=<file>` to append `name,median,p95,mean` CSV rows for tracking over time.

use crate::common::bench::{assert_median_under, budget_ms, measure};
use crate::common::*;
#[cfg(unix)]
use crate::fixtures::plugins::{counting_env_plugins, install_plugin};
use fs_err as fs;

const WARMUP: usize = 10;
const RUNS: usize = 100;

/// Runs `args` in the sandbox's project dir, panicking if the command fails.
fn run_ok(p: &Sandbox, args: &[&str]) {
    let out = p.run_trusted(args);
    assert!(out.status.success(), "{args:?}: {}", stderr_of(&out));
}

/// Measures `args` and fails if the median is over `release_ms` (10x in debug builds).
fn bench_command(name: &str, p: &Sandbox, args: &[&str], runs: usize, release_ms: f64) {
    let stats = measure(name, WARMUP, runs, || run_ok(p, args));
    assert_median_under(name, &stats, budget_ms(release_ms));
}

/// Creates `depth` nested trusted config layers, each exporting one variable; returns the
/// deepest directory relative to the sandbox root.
fn nested_layers(p: &Sandbox, depth: usize) -> String {
    let mut dir = "project".to_string();
    for level in 0..depth {
        let config = serde_json::json!({"root": level == 0, "env": {format!("LAYER_{level}"): level.to_string()}});
        p.write(&format!("{dir}/.avm.json"), config.to_string());
        success(p.run(&["trust", p.path(&dir).to_str().unwrap()]));
        dir = format!("{dir}/child");
    }
    fs::create_dir_all(p.path(&dir)).unwrap();
    dir
}

/// Process start-up: argument parsing and exit, with no config involved.
///
/// 1. Time `avm --version` and `avm --help` over 100 runs each.
/// 2. Both medians stay under 15 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn startup() {
    let p = Sandbox::new("avm-bench");
    bench_command("startup: --version", &p, &["--version"], RUNS, 15.0);
    bench_command("startup: --help", &p, &["--help"], RUNS, 15.0);
}

/// The shell prompt hook (`avm env`) with a warm cache across six config layers.
///
/// 1. Create and trust six nested layers; warm up.
/// 2. Time 100 `avm env` runs from the deepest directory.
/// 3. Median stays under 5 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn env_cached_six_layers() {
    let p = Sandbox::new("avm-bench");
    let dir = nested_layers(&p, 6);
    let stats = measure("env: cached, 6 layers", WARMUP, RUNS, || {
        let out = p.command_in(&["env"], &dir).output().unwrap();
        assert!(out.status.success());
    });
    assert_median_under("env: cached, 6 layers", &stats, budget_ms(5.0));
}

/// Config depth must not make the prompt hook meaningfully slower.
///
/// 1. Time `avm env` with 1, 6 and 12 trusted layers.
/// 2. The 12-layer median stays under 8 ms and under 4x the 1-layer median plus 2 ms.
#[test]
#[ignore = "benchmark: see module docs"]
fn env_scales_with_config_depth() {
    let mut medians = Vec::new();
    for depth in [1usize, 6, 12] {
        let p = Sandbox::new("avm-bench");
        let dir = nested_layers(&p, depth);
        let name = format!("env: cached, {depth} layers");
        let stats = measure(&name, WARMUP, RUNS, || {
            assert!(p
                .command_in(&["env"], &dir)
                .output()
                .unwrap()
                .status
                .success());
        });
        medians.push(stats.median);
    }
    assert!(
        medians[2] < budget_ms(8.0),
        "12 layers: {:.2} ms",
        medians[2]
    );
    assert!(
        medians[2] < medians[0] * 4.0 + 2.0,
        "depth scaling: {medians:?}"
    );
}

/// The prompt hook with three pinned tools whose plugins export variables (cached after run one).
///
/// 1. Install three counting plugins and pin them.
/// 2. Time 100 `avm env` runs.
/// 3. Median stays under 10 ms (release).
#[cfg(unix)]
#[test]
#[ignore = "benchmark: see module docs"]
fn env_three_pinned_tools() {
    let p = Sandbox::new("avm-bench");
    counting_env_plugins(&p, &["node", "java", "android"]);
    bench_command("env: 3 pinned tools", &p, &["env"], RUNS, 10.0);
}

/// Alias lookups on a config with many aliases.
///
/// 1. Write 200 global aliases.
/// 2. Time `which`, `resolve` and `alias list`.
/// 3. Medians stay under 10, 10 and 15 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn alias_lookups_with_200_aliases() {
    let p = Sandbox::new("avm-bench");
    let aliases: serde_json::Map<_, _> = (0..200)
        .map(|n| {
            (
                format!("alias{n}"),
                serde_json::json!(format!("echo {n} $1")),
            )
        })
        .collect();
    p.write(
        "home/.avm.json",
        serde_json::json!({ "aliases": aliases }).to_string(),
    );
    bench_command(
        "which: 1 of 200 aliases",
        &p,
        &["which", "alias150"],
        RUNS,
        10.0,
    );
    bench_command(
        "resolve: 1 of 200 aliases",
        &p,
        &["resolve", "alias150", "x"],
        RUNS,
        10.0,
    );
    bench_command(
        "alias list: 200 aliases",
        &p,
        &["alias", "list"],
        RUNS,
        15.0,
    );
}

/// Running an alias end to end: lookup plus spawning the shell.
///
/// 1. Define `hello` = `echo hello`.
/// 2. Time `avm run hello` and the bare `avm hello` form.
/// 3. Medians stay under 25 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn run_alias() {
    let p = Sandbox::new("avm-bench");
    success(p.run(&["alias", "add", "-g", "hello", "echo hello"]));
    bench_command("run: alias", &p, &["run", "hello"], RUNS, 25.0);
    bench_command("run: bare alias name", &p, &["hello"], RUNS, 25.0);
}

/// The write path: adding aliases takes a file lock and rewrites the config atomically.
///
/// 1. Time 50 `alias add -g` calls with distinct names.
/// 2. Median stays under 20 ms (release) and the config ends up with all 50 aliases.
#[test]
#[ignore = "benchmark: see module docs"]
fn alias_add_write_path() {
    let p = Sandbox::new("avm-bench");
    let mut n = 0;
    let stats = measure("alias add: write path", 5, 50, || {
        n += 1;
        run_ok(&p, &["alias", "add", "-g", &format!("a{n}"), "echo x"]);
    });
    assert_median_under("alias add: write path", &stats, budget_ms(20.0));
    assert_eq!(
        p.read_json("home/.avm.json")["aliases"]
            .as_object()
            .unwrap()
            .len(),
        55
    );
}

/// Listing plugins starts each plugin process once, so cost grows with plugin count.
///
/// 1. Install five trivial plugins.
/// 2. Time `avm plugin list`.
/// 3. Median stays under 60 ms (release).
#[cfg(unix)]
#[test]
#[ignore = "benchmark: see module docs"]
fn plugin_list_five_plugins() {
    let p = Sandbox::new("avm-bench");
    for tool in ["node", "java", "python", "go", "ruby"] {
        install_plugin(&p, tool, "#!/bin/sh\nif [ \"$1\" = installed-versions ]; then echo '{\"versions\":[\"1.0.0\"]}'; fi\n");
    }
    bench_command("plugin list: 5 plugins", &p, &["plugin", "list"], 50, 60.0);
}

/// The shim hot path: every `node`/`npm` call from a script goes through `exec-shim`.
///
/// 1. Install a plugin and a managed binary `hello`; pin the tool.
/// 2. Time `avm exec-shim hello`.
/// 3. Median stays under 30 ms (release).
#[cfg(unix)]
#[test]
#[ignore = "benchmark: see module docs"]
fn exec_shim_dispatch() {
    use std::os::unix::fs::PermissionsExt;
    let p = Sandbox::new("avm-bench");
    install_plugin(&p, "node", "#!/bin/sh\necho '{\"env\":{}}'\n");
    let bin = p.path("home/.avm/tools/node/20.1.0/bin/hello");
    fs::create_dir_all(bin.parent().unwrap()).unwrap();
    fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    p.write("project/.avm.json", r#"{"tools":{"node":"20.1.0"}}"#);
    bench_command(
        "exec-shim: managed binary",
        &p,
        &["exec-shim", "hello"],
        RUNS,
        30.0,
    );
}

/// Shell integration generation is on the critical path of every new terminal.
///
/// 1. Time `avm shell-init` (it runs `shims install`).
/// 2. Median stays under 30 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn shell_init() {
    let p = Sandbox::new("avm-bench");
    bench_command("shell-init", &p, &["shell-init"], 50, 30.0);
}

/// Many avm processes at once (parallel build scripts) must finish promptly and not fail.
///
/// 1. Define an alias, then start 32 `avm which` processes simultaneously.
/// 2. All succeed and the whole batch finishes within 500 ms (release).
#[test]
#[ignore = "benchmark: see module docs"]
fn parallel_invocations() {
    let p = Sandbox::new("avm-bench");
    success(p.run(&["alias", "add", "-g", "hello", "echo hello"]));
    let stats = measure("parallel: 32 x which", 2, 10, || {
        let children: Vec<_> = (0..32)
            .map(|_| {
                p.command(&["which", "hello"])
                    .stdout(std::process::Stdio::null())
                    .spawn()
                    .unwrap()
            })
            .collect();
        for mut child in children {
            assert!(child.wait().unwrap().success());
        }
    });
    assert_median_under("parallel: 32 x which", &stats, budget_ms(500.0));
}
