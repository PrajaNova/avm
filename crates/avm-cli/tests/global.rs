//! Lookups across layers and version upgrades: `which`, `resolve`, `run`, `outdated`, `upgrade`.
//! Source feature: `crates/avm-cli/src/global/`.

use crate::common::*;
#[cfg(unix)]
use crate::fixtures::plugins::install_plugin;
use fs_err as fs;
use speculoos::prelude::*;

/// Without a terminal there is no picker to show, so a bare `avm` prints usage and exits 0.
///
/// 1. Run `avm` with no arguments and no TTY.
/// 2. It succeeds and prints "Usage: avm".
#[test]
fn bare_avm_without_a_terminal_prints_help_instead_of_failing() {
    let p = Sandbox::new("avm-test");
    let out = p.run(&[]);
    assert_that(&out).succeeds();
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: avm"));
}

/// `plugin upgrade` edits only the config that owns a pin; inherited pins are reported, not copied down.
///
/// 1. Pin `node 20.1.0` in the project root; the fake plugin offers 22.1.0.
/// 2. From the root, a dry-run `--bump` plans "20.1.0 → 22.1.0 (local pin)".
/// 3. From a child directory, it says to edit the inherited pin to move to 22.1.0.
/// 4. No `.avm.json` is created in the child.
#[cfg(unix)]
#[test]
fn upgrade_keeps_inherited_pins_at_their_source() {
    let p = Sandbox::new("avm-test");
    install_plugin(&p, "node", "#!/bin/sh\necho '{\"versions\":[{\"version\":\"22.1.0\",\"label\":\"22\",\"is_lts\":true,\"is_security\":false},{\"version\":\"20.2.0\",\"label\":\"20\",\"is_lts\":true,\"is_security\":false}]}'\n");
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"root":true,"tools":{"node":"20.1.0"}}"#,
    )
    .unwrap();
    let local = success(p.run(&["plugin", "upgrade", "--bump", "--dry-run"]));
    assert_that(&local).contains("20.1.0 → 22.1.0 (local pin)");
    let child = p.root.join("project/child");
    fs::create_dir(&child).unwrap();
    let inherited = success(
        p.command(&["plugin", "upgrade", "--bump", "--dry-run"])
            .current_dir(&child)
            .output()
            .unwrap(),
    );
    assert!(
        inherited.contains("edit it to move to 22.1.0"),
        "{inherited}"
    );
    assert_that(&child.join(".avm.json")).does_not_exist();
}

/// `plugin outdated` queries every tool's plugin in parallel, not one after another.
///
/// 1. Install two plugins whose `versions` call sleeps 2 s, and pin both tools.
/// 2. Run `avm plugin outdated`.
/// 3. Both tools show 2.0.0 and the total time is under 3.5 s (a serial run would take 4 s).
#[cfg(unix)]
#[test]
fn outdated_lists_versions_for_all_tools_concurrently() {
    let p = Sandbox::new("avm-test");
    for tool in ["node", "java"] {
        let script = "#!/bin/sh\nif [ \"$1\" = versions ]; then sleep 2; echo '{\"versions\":[{\"version\":\"2.0.0\",\"label\":\"2.0.0\",\"is_lts\":false,\"channel\":null,\"is_security\":false}]}'; fi\n";
        install_plugin(&p, tool, script);
    }
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"tools":{"node":"1.0.0","java":"1.0.0"}}"#,
    )
    .unwrap();
    let start = std::time::Instant::now();
    let out = p
        .command(&["plugin", "outdated"])
        .env("AVM_TRUST_ALL", "1")
        .output()
        .unwrap();
    let text = success(out);
    assert!(
        text.contains("node") && text.contains("java") && text.contains("2.0.0"),
        "{text}"
    );
    assert!(
        start.elapsed() < std::time::Duration::from_millis(3500),
        "{:?}",
        start.elapsed()
    );
}

/// `which`, `resolve`, `run`, version/help output and `plugin outdated/upgrade`.
mod lookups {
    use super::*;
    use rstest::rstest;

    /// A sandbox whose home config defines `echo-args` = `echo $1-$2`.
    fn with_alias() -> Sandbox {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "echo-args", "echo $1-$2"]));
        p
    }

    /// `which` says where an alias comes from.
    ///
    /// 1. Run `avm init`; add a global alias `gg` and a project alias `hi`.
    /// 2. `which gg` names "global alias" and the home file; `which hi` names "local alias" and the project file.
    #[test]
    fn which_reports_an_alias_and_where_it_is_defined() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["alias", "add", "-g", "gg", "echo g"]));
        success(p.run(&["alias", "add", "hi", "echo hi"]));
        assert_that(&p.run_trusted(&["which", "gg"]))
            .succeeds()
            .stdout_contains("global alias 'gg': echo g")
            .stdout_contains("home/.avm.json");
        assert_that(&p.run_trusted(&["which", "hi"]))
            .succeeds()
            .stdout_contains("local alias 'hi': echo hi")
            .stdout_contains("project/.avm.json");
    }

    /// `which` also explains a tool's pinned version.
    ///
    /// 1. Pin `node` to 20.1.0 in the project config.
    /// 2. `which node` names the tool, the version and the config file.
    #[test]
    fn which_reports_a_tool_pin() {
        let p = Sandbox::new("avm-test");
        p.write("project/.avm.json", r#"{"tools":{"node":"20.1.0"}}"#);
        assert_that(&p.run_trusted(&["which", "node"]))
            .succeeds()
            .stdout_contains("tool 'node': 20.1.0")
            .stdout_contains("project/.avm.json");
    }

    /// `which` for an unknown key is informational, not a failure.
    ///
    /// 1. Run `avm which nothing`.
    /// 2. It exits 0 and prints "No mapping found for 'nothing'."
    #[test]
    fn which_for_an_unknown_key_is_not_an_error() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["which", "nothing"]))
            .succeeds()
            .stdout_contains("No mapping found for 'nothing'.");
    }

    /// `resolve` prints the command an alias expands to, substituting `$1`, `$2`.
    ///
    /// 1. Define `echo-args` = `echo $1-$2`.
    /// 2. `avm resolve echo-args a b` prints `echo a-b` and runs nothing.
    #[test]
    fn resolve_expands_positional_placeholders() {
        let p = with_alias();
        assert_that(&success(p.run(&["resolve", "echo-args", "a", "b"])))
            .is_equal_to("echo a-b\n".to_string());
    }

    /// A placeholder with no matching argument is an error, for both `resolve` and `run`.
    ///
    /// 1. Define `echo-args` = `echo $1-$2`.
    /// 2. Call it with only one argument.
    /// 3. It fails with "placeholder $2 out of bounds".
    #[rstest]
    #[case::resolve("resolve")]
    #[case::run("run")]
    fn a_missing_placeholder_argument_fails(#[case] verb: &str) {
        let p = with_alias();
        assert_that(&p.run(&[verb, "echo-args", "only-one"]))
            .fails_with("placeholder $2 out of bounds");
    }

    /// `run` executes the expanded command and passes extra arguments through.
    ///
    /// 1. Define `echo-args`; run it with two arguments.
    /// 2. Stdout is `a-b`.
    #[test]
    fn run_executes_the_expanded_alias() {
        let p = with_alias();
        assert_that(&success(p.run(&["run", "echo-args", "a", "b"])))
            .is_equal_to("a-b\n".to_string());
    }

    /// The alias command's own exit status becomes avm's.
    ///
    /// 1. Define the alias `boom` = `exit 7`.
    /// 2. `avm run boom` exits with 7.
    #[cfg(unix)]
    #[test]
    fn run_propagates_the_alias_exit_code() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "boom", "exit 7"]));
        assert_that(&p.run(&["run", "boom"])).exits_with(7);
    }

    /// `run` without a key is a usage error.
    ///
    /// 1. Run `avm run`.
    /// 2. clap exits 2 and asks for `<KEY>`.
    #[test]
    fn run_requires_an_alias_name() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["run"]))
            .exits_with(2)
            .stderr_contains("<KEY>");
    }

    /// Both version flags print `avm <version>`.
    ///
    /// 1. Run `avm -v` and `avm --version`.
    /// 2. Stdout starts with "avm " and carries the crate version.
    #[rstest]
    #[case::short("-v")]
    #[case::long("--version")]
    fn the_version_flags_print_the_version(#[case] flag: &str) {
        let p = Sandbox::new("avm-test");
        let out = success(p.run(&[flag]));
        assert_that(&out).starts_with(format!("avm {}", env!("CARGO_PKG_VERSION")));
    }

    /// `--help` lists every command a user can type.
    ///
    /// 1. Run `avm --help`.
    /// 2. Each top-level command appears in the list.
    #[rstest]
    #[case("alias")]
    #[case("plugin")]
    #[case("env")]
    #[case("shims")]
    #[case("init")]
    #[case("trust")]
    #[case("exec")]
    #[case("which")]
    #[case("resolve")]
    #[case("run")]
    #[case("create")]
    #[case("shell-init")]
    #[case("self-update")]
    fn help_lists_every_command(#[case] command: &str) {
        let p = Sandbox::new("avm-test");
        let out = success(p.run(&["--help"]));
        assert_that(&out.lines().any(|l| l.trim_start().starts_with(command))).is_true();
    }

    /// An unknown command-line flag is a usage error, not an alias lookup.
    ///
    /// 1. Run `avm --definitely-not-a-flag`.
    /// 2. It fails with clap's "unexpected argument".
    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["--definitely-not-a-flag"]))
            .exits_with(2)
            .stderr_contains("unexpected argument");
    }
}

/// `plugin outdated` and `plugin upgrade` against the lock fixture's plugins, whose newest
/// node release is whatever `<root>/remote` contains (20.2.0).
#[cfg(unix)]
mod versions {
    use super::*;
    use crate::fixtures::projects::LockProject;

    /// With nothing pinned both commands say so.
    ///
    /// 1. Use empty global and project configs.
    /// 2. `plugin outdated` and `plugin upgrade` print "No tools pinned here."
    #[test]
    fn nothing_pinned_is_reported() {
        let p = LockProject::new();
        p.write_config("{}");
        p.write("home/.avm.json", "{}");
        assert_that(&p.run(&["plugin", "outdated"]))
            .succeeds()
            .stdout_contains("No tools pinned here.");
        assert_that(&p.run(&["plugin", "upgrade"]))
            .succeeds()
            .stdout_contains("No tools pinned here.");
    }

    /// `outdated` shows requested, current and newest versions per pinned tool.
    ///
    /// 1. Pin node 20 in the project.
    /// 2. The table has a `node` row requesting 20 whose newest version is 20.2.0.
    #[test]
    fn outdated_prints_a_table() {
        let p = LockProject::new();
        let out = success(p.run(&["plugin", "outdated"]));
        assert_that(&out).contains("REQUESTED");
        assert_that(
            &out.lines()
                .any(|l| l.starts_with("node") && l.contains("20.2.0")),
        )
        .is_true();
    }

    /// `outdated --json` is machine-readable.
    ///
    /// 1. Run `avm plugin outdated --json`.
    /// 2. Stdout parses as JSON and mentions node and 20.2.0.
    #[test]
    fn outdated_json_parses() {
        let p = LockProject::new();
        let out = success(p.run(&["plugin", "outdated", "--json"]));
        let json: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_that(&json.to_string()).contains("node");
        assert_that(&json.to_string()).contains("20.2.0");
    }

    /// `upgrade --dry-run` plans without installing or editing the pin.
    ///
    /// 1. Pin node 20 and run `plugin upgrade --dry-run`.
    /// 2. The plan mentions 20.2.0; nothing was installed and `.avm.json` is unchanged.
    #[test]
    fn upgrade_dry_run_changes_nothing() {
        let p = LockProject::new();
        let before = p.read("project/.avm.json");
        assert_that(&p.run(&["plugin", "upgrade", "--dry-run"]))
            .succeeds()
            .stdout_contains("20.2.0");
        assert_that(&p.path("installs")).does_not_exist();
        assert_that(&p.read("project/.avm.json")).is_equal_to(before);
    }

    /// `upgrade --yes` installs the newest in-range version and moves the pin.
    ///
    /// 1. Pin node 20 and run `plugin upgrade --yes`.
    /// 2. node 20.2.0 was installed and the project pin now names 20.2.0.
    #[test]
    fn upgrade_installs_and_updates_the_pin() {
        let p = LockProject::new();
        assert_that(&p.run(&["plugin", "upgrade", "--yes"])).succeeds();
        assert_that(&p.read("installs")).contains("node:20.2.0");
        assert_that(&p.read_json("project/.avm.json")["tools"]["node"].to_string())
            .contains("20.2.0");
    }
}

/// Wrong, mistyped or incomplete command lines: clap rejects them with exit code 2 and a message
/// that names the problem, before avm touches any config or plugin.
mod usage_errors {
    use super::*;
    use rstest::rstest;

    /// Each invocation fails with usage exit code 2 and the expected message.
    ///
    /// 1. Run a command line with a typo, a missing argument or an extra argument.
    /// 2. It exits 2 and stderr contains the expected text (including clap's "did you mean" tip).
    #[rstest]
    #[case::typo_plugin(&["plugin", "instal"], "unrecognized subcommand 'instal'")]
    #[case::typo_plugin_tip(&["plugin", "instal"], "similar subcommand")]
    #[case::typo_alias(&["alias", "ad"], "unrecognized subcommand 'ad'")]
    #[case::typo_env(&["env", "foo"], "unrecognized subcommand 'foo'")]
    #[case::typo_shims(&["shims", "foo"], "unrecognized subcommand 'foo'")]
    #[case::alias_add_no_key(&["alias", "add"], "<KEY>")]
    #[case::alias_remove_no_key(&["alias", "remove"], "<KEY>")]
    #[case::env_add_nothing(&["env", "add"], "<KEY>")]
    #[case::env_add_no_value(&["env", "add", "K"], "<VALUE>")]
    #[case::env_remove_no_key(&["env", "remove"], "<KEY>")]
    #[case::search_no_keyword(&["plugin", "search"], "<KEYWORD>")]
    #[case::shims_remove_no_tool(&["shims", "remove"], "<TOOL>")]
    #[case::trust_two_paths(&["trust", "a", "b"], "unexpected argument 'b'")]
    #[case::which_no_key(&["which"], "<KEY>")]
    #[case::resolve_no_key(&["resolve"], "<KEY>")]
    #[case::lock_platform_no_value(&["plugin", "lock", "--platform"], "a value is required")]
    #[case::unknown_flag_on_subcommand(&["plugin", "list", "--nope"], "unexpected argument '--nope'")]
    fn bad_command_lines_exit_2(#[case] args: &[&str], #[case] message: &str) {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(args))
            .exits_with(2)
            .stderr_contains(message);
        assert_that(&p.path("home/.avm.json")).does_not_exist();
        assert_that(&p.path("project/.avm.json")).does_not_exist();
    }

    /// Command groups with no subcommand print their usage instead of doing something.
    ///
    /// 1. Run `avm plugin`, `avm alias` and `avm shims` alone.
    /// 2. They fail and show "Usage:" with the required `<COMMAND>`.
    #[rstest]
    #[case("plugin")]
    #[case("alias")]
    #[case("shims")]
    fn a_group_without_a_subcommand_shows_usage(#[case] group: &str) {
        let p = Sandbox::new("avm-test");
        let out = p.run(&[group]);
        assert_that(&out).fails();
        let text = format!("{}{}", stdout_of(&out), stderr_of(&out));
        assert_that(&text).contains("Usage:");
        assert_that(&text).contains("<COMMAND>");
    }
}
