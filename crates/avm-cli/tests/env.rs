//! `avm env`: exported variables, plugin env caching and prompt latency.
//! Source feature: `crates/avm-cli/src/env/`.

use crate::common::*;
#[cfg(unix)]
use crate::fixtures::plugins::counting_env_plugins;
use fs_err as fs;
use speculoos::prelude::*;

/// Plugin `env-vars` output is cached, so the prompt hook does not spawn plugins on every call.
///
/// 1. Install three counting plugins (node, java, android) and pin all three.
/// 2. Run `avm env` three times; each run exports `JAVA_HOME` and `ANDROID_HOME`.
/// 3. The spawn log shows exactly one `env-vars` call per tool.
#[cfg(unix)]
#[test]
fn plugin_env_vars_are_cached_after_the_first_prompt() {
    let p = Sandbox::new("avm-test");
    let counter = counting_env_plugins(&p, &["node", "java", "android"]);
    for _ in 0..3 {
        let out = success(p.run(&["env"]));
        assert!(
            out.contains("JAVA_HOME") && out.contains("ANDROID_HOME"),
            "{out}"
        );
    }
    assert_eq!(
        fs::read_to_string(counter).unwrap().lines().count(),
        3,
        "one plugin spawn per tool, ever"
    );
}

/// A pinned tool with no installed plugin must not trigger marketplace lookups from `avm env`.
///
/// 1. Pin `ruby` with no plugin installed and point the GitHub API URL at a local listener.
/// 2. Run `avm env`.
/// 3. The listener never receives a connection and the command finishes in under 5 seconds.
#[test]
fn pinned_tool_without_plugin_makes_no_network_call() {
    let p = Sandbox::new("avm-test");
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"tools":{"ruby":"3.2.0"}}"#,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let start = std::time::Instant::now();
    p.command(&["env"])
        .env("AVM_GITHUB_API_URL", url)
        .output()
        .unwrap();
    assert!(listener.accept().is_err(), "marketplace was contacted");
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

/// `--env` / `AVM_ENV` profile names are validated so they can never address other paths.
///
/// 1. Run `avm env --env <name>` with path-like or malformed names.
/// 2. Every run fails with "invalid environment profile".
#[rstest::rstest]
#[case::parent_dir("../secret")]
#[case::empty_segment("ci,,staging")]
#[case::slash("prod/path")]
#[case::dot(".")]
#[case::backslash("ci\\prod")]
fn invalid_profile_names_are_rejected(#[case] profile: &str) {
    let p = Sandbox::new("avm-test");
    failure(
        p.run(&["env", "--env", profile]),
        "invalid environment profile",
    );
}

/// `avm env add / remove / list` and the shell export list.
mod vars {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    /// Adds go to the project config by default and to home with `-g`.
    ///
    /// 1. Run `avm init`, then `env add FOO bar` and `env add -g GLOB 1`.
    /// 2. The project config has FOO, the home config has GLOB; output names the scope.
    #[test]
    fn add_writes_to_the_chosen_scope() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        assert_that(&p.run(&["env", "add", "FOO", "bar"]))
            .succeeds()
            .stdout_contains("Added local env var 'FOO=bar'");
        assert_that(&p.run(&["env", "add", "-g", "GLOB", "1"]))
            .succeeds()
            .stdout_contains("Added global env var 'GLOB=1'");
        assert_that(&p.read_json("project/.avm.json")["env"]["FOO"]).is_equal_to(json!("bar"));
        assert_that(&p.read_json("home/.avm.json")["env"]["GLOB"]).is_equal_to(json!("1"));
    }

    /// `ea` is a hidden shortcut for `env add`.
    ///
    /// 1. Run `avm init`, then `avm ea X y`.
    /// 2. The project config has X=y.
    #[test]
    fn the_ea_shortcut_adds_a_variable() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["ea", "X", "y"]));
        assert_that(&p.read_json("project/.avm.json")["env"]["X"]).is_equal_to(json!("y"));
    }

    /// Variable names must be valid shell identifiers.
    ///
    /// 1. Run `avm env add <key> v` for names with a space, a leading digit, `=` or a dash.
    /// 2. Each fails with "invalid env key".
    #[rstest]
    #[case::space("BAD KEY")]
    #[case::leading_digit("1X")]
    #[case::equals("A=B")]
    #[case::dash("A-B")]
    fn invalid_names_are_rejected(#[case] key: &str) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        failure(p.run(&["env", "add", key, "v"]), "invalid env key");
    }

    /// Both list spellings print merged variables with their source, project over global.
    ///
    /// 1. Set `MODE` globally and override it locally; add a global-only `ONLY`.
    /// 2. The list shows the local `MODE`, the global `ONLY`, and each file.
    #[rstest]
    #[case::list(&["env", "list"])]
    #[case::ls(&["env", "ls"])]
    fn list_shows_the_merged_variables_with_sources(#[case] args: &[&str]) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["env", "add", "-g", "MODE", "global"]));
        success(p.run(&["env", "add", "-g", "ONLY", "1"]));
        success(p.run(&["env", "add", "MODE", "local"]));
        let out = p.run_trusted(args);
        assert_that(&out)
            .succeeds()
            .stdout_contains("MODE=local (from ")
            .stdout_contains("ONLY=1 (from ")
            .stdout_contains("project/.avm.json")
            .stdout_contains("home/.avm.json");
        assert_that(&stdout_of(&out).contains("MODE=global")).is_false();
    }

    /// `remove` / `rm` delete a variable; removing a missing one is an error.
    ///
    /// 1. Add FOO, then remove it with the given spelling: success, config no longer has it.
    /// 2. Remove it again: fails with "env var 'FOO' not found".
    #[rstest]
    #[case::remove("remove")]
    #[case::rm("rm")]
    fn remove_deletes_the_variable(#[case] verb: &str) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["env", "add", "FOO", "bar"]));
        assert_that(&p.run(&["env", verb, "FOO"]))
            .succeeds()
            .stdout_contains("Removed local env var 'FOO'");
        assert_that(&p.read_json("project/.avm.json")["env"].get("FOO")).is_none();
        failure(p.run(&["env", verb, "FOO"]), "env var 'FOO' not found");
    }

    /// The export list uses the syntax of the requested shell.
    ///
    /// 1. Add `FOO=bar` and run `avm env --shell <shell>`.
    /// 2. Output has the shell's assignment and unset forms and the baseline marker.
    #[rstest]
    #[case::sh("sh", "export FOO=bar", "unset JAVA_HOME")]
    #[case::pwsh("pwsh", "$env:FOO = 'bar'", "Remove-Item Env:JAVA_HOME")]
    #[case::cmd("cmd", "set \"FOO=bar\"", "set \"JAVA_HOME=\"")]
    fn the_export_list_matches_the_shell(
        #[case] shell: &str,
        #[case] assign: &str,
        #[case] unset: &str,
    ) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["env", "add", "FOO", "bar"]));
        let out = p
            .command(&["env", "--shell", shell])
            .env("AVM_TRUST_ALL", "1")
            // The shell hook's saved baseline: avm restores (here: unsets) what it recorded.
            .env("_AVM_ENV_BASELINE", r#"{"JAVA_HOME":null}"#)
            .output()
            .unwrap();
        assert_that(&out)
            .succeeds()
            .stdout_contains(assign)
            .stdout_contains(unset)
            .stdout_contains("_AVM_ENV_BASELINE");
    }

    /// An unknown shell name is rejected by the argument parser.
    ///
    /// 1. Run `avm env --shell fish`.
    /// 2. clap exits 2 listing the valid values.
    #[test]
    fn an_unknown_shell_is_a_usage_error() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["env", "--shell", "fish"]))
            .exits_with(2)
            .stderr_contains("possible values");
    }

    /// With nothing configured the export list still clears provider variables.
    ///
    /// 1. Run `avm env --shell sh` in an empty sandbox whose saved baseline lists the three
    ///    provider variables as originally unset (as the shell hook records them).
    /// 2. It unsets ANDROID_HOME, ANDROID_SDK_ROOT and JAVA_HOME and exports nothing else.
    #[test]
    fn an_empty_project_only_unsets_provider_variables() {
        let p = Sandbox::new("avm-test");
        let out = success(
            p.command(&["env", "--shell", "sh"])
                .env(
                    "_AVM_ENV_BASELINE",
                    r#"{"ANDROID_HOME":null,"ANDROID_SDK_ROOT":null,"JAVA_HOME":null}"#,
                )
                .output()
                .unwrap(),
        );
        for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT", "JAVA_HOME"] {
            assert_that(&out).contains(format!("unset {var}").as_str());
        }
        assert_that(&out.lines().filter(|l| l.starts_with("export FOO")).count()).is_equal_to(0);
    }
}
