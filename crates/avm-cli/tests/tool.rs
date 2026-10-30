//! Temporary tool versions: `avm exec`, `avm x`, `avm run --with` and `avm <tool> use`.
//! Source feature: `crates/avm-cli/src/tool/`. Uses `ExecProject`, whose fake plugins print
//! `<tool>-<version>` so a test can tell exactly which version ran.

use crate::common::*;
use crate::fixtures::projects::ExecProject;
use fs_err as fs;
use rstest::rstest;
use speculoos::prelude::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Output;

/// `exec` / `x` behaviour.
mod exec {
    use super::*;

    /// `--no-install` must fail for a version that is not installed, and install nothing.
    ///
    /// 1. Run `avm exec --no-install node@18 -- node` in a project without node 18.
    /// 2. The command fails.
    /// 3. Nothing was installed under `tools/node/18.1.0`.
    #[test]
    fn no_install_rejects_a_missing_version_without_installing_it() {
        let project = ExecProject::new();
        assert_that(&project.trusted(&["exec", "--no-install", "node@18", "--", "node"])).fails();
        assert_that(&project.path("home/.avm/tools/node/18.1.0")).does_not_exist();
    }

    /// `exec` installs each requested tool, then runs the command with their PATH, env vars and untouched arguments.
    ///
    /// 1. Run `avm exec node@18 java@21 -- sh -c ...` with an argument containing spaces.
    /// 2. The fake tools print their version and arguments in order.
    /// 3. `SCOPE` comes from the project config and `JAVA_HOME` from the java plugin.
    #[test]
    fn installs_every_tool_and_passes_arguments_and_env_through() {
        let project = ExecProject::new();
        let script = "node --help 'two words'; java; printf '%s\\n' \"$SCOPE\" \"$JAVA_HOME\"";
        let out =
            success(project.trusted(&["exec", "node@18", "java@21", "--", "sh", "-c", script]));
        assert_that(&out).starts_with("node-18.1.0\n--help\ntwo words\njava-21.1.0\nlocal\n");
        assert_that(&out).contains("/tools/java/21.1.0");
    }

    /// Table: how a temporary version request resolves (alias `x`, `run --with`, trust, partial, `lts`, `latest`).
    ///
    /// 1. Start from a project with node 18.1.0 and java 21.1.0 installed.
    /// 2. Run the command from the table with project config trusted or not.
    /// 3. Stdout is exactly the expected output.
    #[rstest]
    #[case::alias_x(&["x", "--no-install", "node@18", "--", "node"], true, "node-18.1.0\n")]
    #[case::run_alias_with_tools(
        &["run", "check", "--with", "node@18", "--with", "java@21", "--no-install", "--", "two words"],
        true,
        "node-18.1.0\ntwo words\njava-21.1.0\nlocal"
    )]
    #[case::untrusted_ignores_project_env(
        &["exec", "node@18.1.0", "--", "sh", "-c", "printf '%s' \"$SCOPE\""],
        false,
        "global"
    )]
    #[case::partial_version(&["exec", "node@18.1", "--", "node"], true, "node-18.1.0\n")]
    #[case::lts_channel(&["exec", "node@lts", "--", "node"], true, "node-22.1.0\n")]
    #[case::latest(&["exec", "java@latest", "--", "java"], true, "java-22.1.0\n")]
    fn prints_the_selected_version(
        #[case] args: &[&str],
        #[case] trusted: bool,
        #[case] expected: &str,
    ) {
        let project = ExecProject::with_installed();
        assert_that(&stdout_of(&project.run_as(args, trusted))).is_equal_to(expected.to_string());
    }

    /// Aliases from an untrusted project are not runnable, even with `--with` tools.
    ///
    /// 1. Run `avm run check --with node@18` without trusting the project.
    /// 2. The command fails.
    #[test]
    fn untrusted_project_cannot_use_its_aliases() {
        let project = ExecProject::with_installed();
        assert_that(&project.run_as(&["run", "check", "--with", "node@18"], false)).fails();
    }

    /// Inside an `exec` environment the `avm` shell function still resolves `x ... node@18`.
    ///
    /// 1. Run bash under `exec node@18.1.0`, load `shell-init` and call `avm x --no-install node@18 -- node`.
    /// 2. It prints `node-18.1.0`.
    #[test]
    fn the_shell_wrapper_resolves_temporary_versions() {
        let project = ExecProject::with_installed();
        let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
            .parent()
            .unwrap();
        let script = format!(
            "export PATH='{}':$PATH; eval \"$(avm-bin shell-init)\"; avm x --no-install node@18 -- node",
            bin.display()
        );
        let out = project.run_as(&["exec", "node@18.1.0", "--", "bash", "-c", &script], false);
        assert_that(&stdout_of(&out)).is_equal_to("node-18.1.0\n".to_string());
    }

    /// Temporary versions never modify pins or the lock.
    ///
    /// 1. Snapshot the project config, global config and lock.
    /// 2. Run `avm exec node@18 -- node`.
    /// 3. All three files are byte-identical.
    #[test]
    fn pins_and_lock_are_left_unchanged() {
        let project = ExecProject::with_installed();
        let files = ["project/.avm.json", "project/.avm.lock", "home/.avm.json"];
        let before: Vec<_> = files
            .iter()
            .map(|f| fs::read(project.path(f)).unwrap())
            .collect();
        success(project.trusted(&["exec", "node@18", "--", "node"]));
        for (file, original) in files.iter().zip(before) {
            assert_that(&fs::read(project.path(file)).unwrap())
                .named(file)
                .is_equal_to(original);
        }
    }

    /// `avm node use 18.1` resolves the partial version and writes the exact one to the project pin.
    ///
    /// 1. Remove the lock so the pin is the only state.
    /// 2. Run `avm node use 18.1`.
    /// 3. `.avm.json` pins node to `18.1.0`.
    #[test]
    fn use_writes_the_resolved_version_to_the_project_pin() {
        let project = ExecProject::new();
        fs::remove_file(project.path("project/.avm.lock")).unwrap();
        assert_that(&project.trusted(&["node", "use", "18.1"])).succeeds();
        let config = read_json(&project.path("project/.avm.json"));
        assert_that(&config["tools"]["node"]).is_equal_to(serde_json::json!("18.1.0"));
    }
}

/// How the child process's result is reported.
mod child_process {
    use super::*;

    fn run_installed(script: &str) -> Output {
        ExecProject::with_installed().trusted(&["exec", "node@18.1.0", "--", "sh", "-c", script])
    }

    /// `exec` exits with the child's exit code.
    ///
    /// 1. Run `exit 37` under `exec`.
    /// 2. The exit code is 37.
    #[test]
    fn exit_code_is_propagated() {
        assert_that(&run_installed("exit 37")).exits_with(37);
    }

    /// `exec` reports a signal-killed child as the same signal.
    ///
    /// 1. Run `kill -TERM $$` under `exec`.
    /// 2. The process status shows signal 15.
    #[test]
    fn a_fatal_signal_is_propagated() {
        let out = run_installed("kill -TERM $$");
        assert_that(&out.status.signal()).is_equal_to(Some(15));
    }

    /// `exec` of a command that does not exist fails.
    ///
    /// 1. Run `avm exec node@18.1.0 -- avm-no-such-command`.
    /// 2. The command fails.
    #[test]
    fn an_unknown_command_fails() {
        let project = ExecProject::with_installed();
        assert_that(&project.trusted(&["exec", "node@18.1.0", "--", "avm-no-such-command"]))
            .fails();
    }
}

/// `avm <tool> list / versions / install / use / set / uninstall`, served by the tool's plugin.
/// Starts from `ExecProject::bare()` (plugins only) so pins come only from what a test does.
mod provider {
    use super::*;
    use serde_json::json;

    /// `avm node list` shows the selection and the installed versions.
    ///
    /// 1. Run `avm node list` with no pins.
    /// 2. It prints "Plugin: node", "Selected version: none" and the installed versions.
    #[test]
    fn list_shows_the_selection_and_installed_versions() {
        let p = ExecProject::bare();
        assert_that(&p.run(&["node", "list"]))
            .succeeds()
            .stdout_contains("Plugin: node")
            .stdout_contains("Selected version: none")
            .stdout_contains("Installed node versions: 18.1.0, 21.1.0");
    }

    /// A bare `avm node` is the same as `avm node list`.
    ///
    /// 1. Run both.
    /// 2. The output is identical.
    #[test]
    fn a_bare_tool_name_lists() {
        let p = ExecProject::bare();
        assert_that(&success(p.run(&["node"]))).is_equal_to(success(p.run(&["node", "list"])));
    }

    /// `versions` queries the plugin for what can be installed, optionally narrowed by major.
    ///
    /// 1. Run `avm node versions`, `avm node 18 versions` and `avm node latest versions`.
    /// 2. They list 22.1.0, 18.1.0 and 22.1.0, each followed by usage hints.
    #[rstest::rstest]
    #[case::all(&["node", "versions"], "22.1.0")]
    #[case::one_major(&["node", "18", "versions"], "18.1.0")]
    #[case::latest(&["node", "latest", "versions"], "22.1.0")]
    fn versions_lists_what_the_plugin_offers(#[case] args: &[&str], #[case] expected: &str) {
        let p = ExecProject::bare();
        assert_that(&p.run(args))
            .succeeds()
            .stdout_contains("Available node versions:")
            .stdout_contains(expected)
            .stdout_contains("avm node install <version>");
    }

    /// A plain install pins the version locally and, when nothing is pinned globally, globally too.
    ///
    /// 1. Run `avm node install 18.1.0`.
    /// 2. Output says "(local + global pin set)"; project and home configs both pin 18.1.0.
    #[test]
    fn install_pins_locally_and_globally_when_unpinned() {
        let p = ExecProject::bare();
        assert_that(&p.run(&["node", "install", "18.1.0"]))
            .succeeds()
            .stdout_contains("Installed node 18.1.0 (local + global pin set)");
        assert_that(&p.read_json("project/.avm.json")["tools"]["node"])
            .is_equal_to(json!("18.1.0"));
        assert_that(&p.read_json("home/.avm.json")["tools"]["node"]).is_equal_to(json!("18.1.0"));
        assert_that(&p.path("home/.avm/tools/node/18.1.0/bin/node")).exists();
    }

    /// `--global` / `-g` pin only the home config.
    ///
    /// 1. Run `avm node install 18.1.0 <flag>`.
    /// 2. Output says "(global pin set)"; the home config pins it and no project config is written.
    #[rstest::rstest]
    #[case::long("--global")]
    #[case::short("-g")]
    fn install_global_pins_only_the_home_config(#[case] flag: &str) {
        let p = ExecProject::bare();
        assert_that(&p.run(&["node", "install", "18.1.0", flag]))
            .succeeds()
            .stdout_contains("(global pin set)");
        assert_that(&p.read_json("home/.avm.json")["tools"]["node"]).is_equal_to(json!("18.1.0"));
        assert_that(&p.path("project/.avm.json")).does_not_exist();
    }

    /// `--no-pin` installs without touching any config.
    ///
    /// 1. Run `avm node install 18.1.0 --no-pin`.
    /// 2. The version is installed; neither config file exists.
    #[test]
    fn install_no_pin_leaves_configs_alone() {
        let p = ExecProject::bare();
        assert_that(&p.run(&["node", "install", "18.1.0", "--no-pin"]))
            .succeeds()
            .stdout_contains("Installed node 18.1.0");
        assert_that(&p.path("home/.avm/tools/node/18.1.0/bin/node")).exists();
        assert_that(&p.path("project/.avm.json")).does_not_exist();
        assert_that(&p.path("home/.avm.json")).does_not_exist();
    }

    /// `use` and its alias `set` pin an installed version locally, or globally with `-g`.
    ///
    /// 1. Run `avm node <verb> 18.1.0 [-g]`.
    /// 2. Output says which scope was set and only that scope's config has the pin.
    #[rstest::rstest]
    #[case::use_local("use", &[], "Set local node version to 18.1.0", "project/.avm.json", "home/.avm.json")]
    #[case::set_local("set", &[], "Set local node version to 18.1.0", "project/.avm.json", "home/.avm.json")]
    #[case::use_global("use", &["-g"], "Set global node version to 18.1.0", "home/.avm.json", "project/.avm.json")]
    #[case::set_global("set", &["--global"], "Set global node version to 18.1.0", "home/.avm.json", "project/.avm.json")]
    fn use_and_set_pin_a_scope(
        #[case] verb: &str,
        #[case] flags: &[&str],
        #[case] message: &str,
        #[case] pinned: &str,
        #[case] untouched: &str,
    ) {
        let p = ExecProject::bare();
        success(p.run(&["node", "install", "18.1.0", "--no-pin"]));
        let mut args = vec!["node", verb, "18.1.0"];
        args.extend(flags);
        assert_that(&p.run(&args))
            .succeeds()
            .stdout_contains(message);
        assert_that(&p.read_json(pinned)["tools"]["node"]).is_equal_to(json!("18.1.0"));
        assert_that(&p.path(untouched)).does_not_exist();
    }

    /// A failure inside the plugin is reported with the command that failed.
    ///
    /// 1. Run `avm node uninstall 99` against a plugin that cannot uninstall.
    /// 2. It fails with "plugin 'node' `uninstall 99` failed".
    #[test]
    fn a_plugin_failure_names_the_failed_command() {
        let p = ExecProject::bare();
        failure(
            p.run(&["node", "uninstall", "99"]),
            "plugin 'node' `uninstall 99` failed",
        );
    }
}

/// Argument validation for `avm exec` (the success paths are in `exec` above).
mod exec_arguments {
    use super::*;

    /// Bad invocations fail before anything is installed or run.
    ///
    /// 1. Run `avm exec` with a missing command, missing tools, a malformed spec, a duplicated
    ///    tool or an unsafe version.
    /// 2. Each fails with the expected message (exit 2 for usage errors) and installs nothing.
    #[rstest::rstest]
    #[case::no_command(&["exec", "node@18"], Some(2), "<COMMAND>")]
    #[case::no_tools(&["exec", "--", "true"], Some(2), "<TOOLS>")]
    #[case::not_a_spec(&["exec", "badspec", "--", "true"], Some(1), "expected tool@version")]
    #[case::duplicate(&["exec", "node@18", "node@20", "--", "true"], Some(1), "specified more than once")]
    #[case::path_traversal(&["exec", "node@../x", "--", "true"], Some(1), "invalid tool specification")]
    #[case::empty_version(&["exec", "node@", "--", "true"], Some(1), "invalid tool specification")]
    fn invalid_invocations_are_rejected(
        #[case] args: &[&str],
        #[case] code: Option<i32>,
        #[case] message: &str,
    ) {
        let p = ExecProject::bare();
        let out = p.run(args);
        assert_that(&out).stderr_contains(message);
        assert_that(&out.status.code()).is_equal_to(code);
        assert_that(&p.path("home/.avm/tools")).does_not_exist();
    }
}

/// Versions and plugin failures that avm itself detects and explains.
mod provider_failures {
    use super::*;
    use crate::fixtures::plugins::install_plugin;
    use rstest::rstest;

    /// Versions no plugin offers are rejected with the version named.
    ///
    /// 1. Run `avm node use|install <version>` with a version that matches nothing, a path, or
    ///    illegal characters.
    /// 2. It fails with "no node version matches '<version>'" and no config is written.
    #[rstest]
    #[case::use_unknown("use", "99.9")]
    #[case::use_path("use", "../x")]
    #[case::install_unknown("install", "99.9")]
    #[case::install_spaces("install", "a b")]
    fn a_version_nothing_matches_is_rejected(#[case] verb: &str, #[case] version: &str) {
        let p = ExecProject::bare();
        failure(
            p.run(&["node", verb, version]),
            &format!("no node version matches '{version}'"),
        );
        assert_that(&p.path("project/.avm.json")).does_not_exist();
        assert_that(&p.path("home/.avm.json")).does_not_exist();
    }

    /// A major filter must be `latest` or a number.
    ///
    /// 1. Run `avm node abc versions`.
    /// 2. It fails with "unknown version filter: abc".
    #[test]
    fn an_invalid_version_filter_is_rejected() {
        let p = ExecProject::bare();
        failure(
            p.run(&["node", "abc", "versions"]),
            "unknown version filter: abc",
        );
    }

    /// A plugin that exits non-zero is reported with the command it failed on.
    ///
    /// 1. Install a node plugin that always exits 1.
    /// 2. `avm node versions` fails with "failed on `versions --query recent`".
    /// 3. `avm exec node@18 -- true` fails the same way, naming `major:18`, and runs nothing.
    #[test]
    fn a_failing_plugin_is_named_in_the_error() {
        let p = Sandbox::new("avm-test");
        install_plugin(&p, "node", "#!/bin/sh\nexit 1\n");
        let out = p.run(&["node", "versions"]);
        assert_that(&out)
            .fails_with("plugin 'node'")
            .stderr_contains("failed on `versions --query recent`");
        assert_that(&p.run(&["exec", "node@18", "--", "true"]))
            .fails_with("failed on `versions --query major:18`");
    }

    /// A plugin that prints something that is not the protocol's JSON is reported as malformed.
    ///
    /// 1. Install a node plugin that prints `not-json`.
    /// 2. `avm node versions` and `avm exec node@18 -- true` fail with "returned malformed output".
    #[rstest]
    #[case::versions(&["node", "versions"], "returned malformed output for `versions --query recent`")]
    #[case::exec(&["exec", "node@18", "--", "true"], "returned malformed output for `versions --query major:18`")]
    fn a_plugin_with_malformed_output_is_reported(#[case] args: &[&str], #[case] message: &str) {
        let p = Sandbox::new("avm-test");
        install_plugin(&p, "node", "#!/bin/sh\necho not-json\n");
        failure(p.run(args), message);
    }
}

/// A plugin that never answers must not hang avm forever.
mod plugin_timeouts {
    use super::*;
    use crate::fixtures::plugins::install_plugin;

    /// With a short read timeout a silent plugin is abandoned and named in the error.
    ///
    /// 1. Install a node plugin that sleeps for 20 s and set `AVM_PLUGIN_READ_TIMEOUT=1` (seconds).
    /// 2. `avm node versions` fails within 10 s and the error says the plugin timed out.
    #[test]
    fn a_hanging_plugin_times_out() {
        let p = Sandbox::new("avm-test");
        install_plugin(&p, "node", "#!/bin/sh\nsleep 20\n");
        let start = std::time::Instant::now();
        let out = p
            .command(&["node", "versions"])
            .env("AVM_PLUGIN_READ_TIMEOUT", "1")
            .output()
            .unwrap();
        assert_that(&out).fails_with("timed out");
        assert_that(&(start.elapsed() < std::time::Duration::from_secs(10))).is_true();
    }
}
