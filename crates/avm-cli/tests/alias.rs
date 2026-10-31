//! Aliases: `avm alias`, package.json scripts as aliases, and name validation.
//! Source feature: `crates/avm-cli/src/alias/` and the package.json reader in `common/state.rs`.

use crate::common::*;
use fs_err as fs;
use speculoos::prelude::*;
use std::path::PathBuf;

/// Concurrent `alias add` processes must not lose each other's writes (config edits are locked and atomic).
///
/// 1. Spawn 24 `avm alias add -g aliasN` processes at the same time.
/// 2. Wait for every process to exit successfully.
/// 3. The global `.avm.json` holds all 24 aliases.
#[test]
fn concurrent_config_edits_preserve_every_alias() {
    let p = Sandbox::new("avm-test");
    let mut children = Vec::new();
    for n in 0..24 {
        children.push(
            p.command(&["alias", "add", "-g", &format!("alias{n}"), "echo ok"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(p.root.join("home/.avm.json")).unwrap()).unwrap();
    assert_eq!(config["aliases"].as_object().unwrap().len(), 24);
}

/// Built-in command words are reserved: an alias must never hide `avm run`, `avm plugin`, etc.
///
/// 1. Run `avm init`, then try to add aliases named `alias`, `run`, `plugin`, `x`, `help`, `-x` and an empty name.
/// 2. Every attempt fails; the `run` attempt reports that the name is "reserved".
/// 3. `list` is no longer a built-in, so adding it as an alias succeeds.
#[test]
fn alias_names_cannot_shadow_built_in_commands() {
    let p = Sandbox::new("avm-test");
    assert_that(&p.run(&["init"])).succeeds();
    for name in ["alias", "run", "plugin", "x", "help", "-x", ""] {
        let out = p.run(&["alias", "add", name, "echo no"]);
        assert_that(&out).fails();
    }
    let out = p.run(&["alias", "add", "run", "echo no"]);
    assert_that(&out).stderr_contains("reserved");
    // `list` stopped being a built-in, so it is a plain alias name now.
    assert_that(&p.run(&["alias", "add", "list", "echo ok"])).succeeds();
}

/// package.json script names and CLI arguments reach the package manager as literal arguments, never through a shell.
///
/// 1. Put a fake `npm` that echoes its arguments first on PATH.
/// 2. For script names containing `;`, `$(...)` and quotes, define a script of that name in package.json.
/// 3. Run `avm run <name> -- "arg; touch injected"`.
/// 4. npm receives exactly `run`, the name and the argument, and no injected file is created.
#[cfg(unix)]
#[test]
fn package_script_names_and_arguments_are_literal_data() {
    use std::os::unix::fs::PermissionsExt;
    let p = Sandbox::new("avm-test");
    let bin = p.root.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(bin.join("npm"), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(bin.join("npm"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let marker = p.root.join("injected");
    for name in [
        format!("hello; touch {}; #", marker.display()),
        "$1 $(touch injected) 'quoted'".into(),
    ] {
        fs::write(
            p.root.join("project/package.json"),
            serde_json::json!({"scripts":{&name:"echo safe"}}).to_string(),
        )
        .unwrap();
        let out = p
            .command(&["run", &name, "--", "arg; touch injected"])
            .env(
                "PATH",
                std::env::join_paths([
                    bin.clone(),
                    PathBuf::from("/usr/bin"),
                    PathBuf::from("/bin"),
                ])
                .unwrap(),
            )
            .output()
            .unwrap();
        assert_that(&out).succeeds();
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!("run\n{name}\narg; touch injected\n")
        );
        assert!(!marker.exists() && !p.root.join("project/injected").exists());
    }
}

/// A UTF-8 BOM at the start of package.json must not stop its scripts from becoming aliases.
///
/// 1. Write a package.json that starts with a BOM and defines a `dev` script.
/// 2. `avm resolve dev` succeeds.
#[test]
fn package_json_with_bom_still_provides_script_aliases() {
    let p = Sandbox::new("avm-test");
    fs::write(
        p.root.join("project/package.json"),
        b"\xef\xbb\xbf{\"scripts\":{\"dev\":\"echo hi\"}}",
    )
    .unwrap();
    let out = p.run(&["resolve", "dev"]);
    assert_that(&out).succeeds();
}

/// A malformed package.json is ignored: it must never break `env`, shims or listing.
///
/// 1. Write `{ not json` as package.json.
/// 2. `avm env` still succeeds.
/// 3. A shim call for an unknown command does not mention package.json in its error.
/// 4. `avm plugin list` succeeds and warns "ignoring package.json".
#[test]
fn broken_package_json_never_blocks_env_or_shims() {
    let p = Sandbox::new("avm-test");
    fs::write(p.root.join("project/package.json"), "{ not json").unwrap();
    assert_that(&p.run(&["env"])).succeeds();
    assert!(!String::from_utf8_lossy(
        &p.run(&["exec-shim", "definitely-not-a-command", "--"])
            .stderr
    )
    .contains("package.json"));
    // Alias commands warn and carry on instead of failing.
    let out = p.run(&["plugin", "list"]);
    assert_that(&out).succeeds();
    assert_that(&out).stderr_contains("ignoring package.json");
}

/// `avm alias add / remove / list` and running an alias by its bare name.
mod commands {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    /// `alias add` without `-g` edits the project config and leaves the global one alone.
    ///
    /// 1. Run `avm init`, then `avm alias add hi echo hello world`.
    /// 2. The project `.avm.json` holds `hi` = "echo hello world" (words joined).
    /// 3. No global config was created.
    #[test]
    fn add_writes_to_the_project_config_by_default() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        assert_that(&p.run(&["alias", "add", "hi", "echo", "hello", "world"]))
            .succeeds()
            .stdout_contains("Added local alias 'hi'");
        assert_that(&p.read_json("project/.avm.json")["aliases"]["hi"])
            .is_equal_to(json!("echo hello world"));
        assert_that(&p.path("home/.avm.json")).does_not_exist();
    }

    /// `-g` / `--global` write to the home config instead.
    ///
    /// 1. Run `avm alias add <flag> gg "echo g"`.
    /// 2. The home `.avm.json` holds `gg`; the project has no config.
    #[rstest]
    #[case::short("-g")]
    #[case::long("--global")]
    fn add_global_writes_to_the_home_config(#[case] flag: &str) {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["alias", "add", flag, "gg", "echo g"]))
            .succeeds()
            .stdout_contains("Added global alias 'gg'");
        assert_that(&p.read_json("home/.avm.json")["aliases"]["gg"]).is_equal_to(json!("echo g"));
        assert_that(&p.path("project/.avm.json")).does_not_exist();
    }

    /// Every spelling of `list` prints each alias with the file that defines it.
    ///
    /// 1. Run `avm init`, then add one global and one project alias.
    /// 2. Run the list command; both aliases show `name → command (from <file>)`.
    #[rstest]
    #[case::list(&["alias", "list"])]
    #[case::ls(&["alias", "ls"])]
    #[case::short_flag(&["-al", "list"])]
    fn list_shows_each_alias_with_its_source(#[case] args: &[&str]) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["alias", "add", "-g", "gg", "echo g"]));
        success(p.run(&["alias", "add", "hi", "echo hi"]));
        let out = p.run(args);
        assert_that(&out)
            .succeeds()
            .stdout_contains("gg → echo g (from ")
            .stdout_contains("hi → echo hi (from ")
            .stdout_contains("home/.avm.json")
            .stdout_contains("project/.avm.json");
    }

    /// `remove` / `rm` delete only the named alias in the chosen scope.
    ///
    /// 1. Run `avm init`; add a project alias `a`, a project alias `b` and a global alias `a`.
    /// 2. Remove the project `a` with the given spelling.
    /// 3. Project `b` and global `a` are still configured.
    #[rstest]
    #[case::remove("remove")]
    #[case::rm("rm")]
    fn remove_deletes_only_the_named_alias_in_that_scope(#[case] verb: &str) {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        success(p.run(&["alias", "add", "a", "echo local-a"]));
        success(p.run(&["alias", "add", "b", "echo local-b"]));
        success(p.run(&["alias", "add", "-g", "a", "echo global-a"]));
        assert_that(&p.run(&["alias", verb, "a"]))
            .succeeds()
            .stdout_contains("Removed local alias 'a'");
        let local = p.read_json("project/.avm.json");
        assert_that(&local["aliases"].get("a")).is_none();
        assert_that(&local["aliases"]["b"]).is_equal_to(json!("echo local-b"));
        assert_that(&p.read_json("home/.avm.json")["aliases"]["a"])
            .is_equal_to(json!("echo global-a"));
    }

    /// `remove -g` targets the home config.
    ///
    /// 1. Add a global alias, then run `avm alias remove -g gg`.
    /// 2. It reports a global removal and the config no longer has the alias.
    #[test]
    fn remove_global_edits_the_home_config() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "gg", "echo g"]));
        assert_that(&p.run(&["alias", "remove", "-g", "gg"]))
            .succeeds()
            .stdout_contains("Removed global alias 'gg'");
        assert_that(&p.read_json("home/.avm.json")["aliases"].get("gg")).is_none();
    }

    /// Removing something that is not there is an error, not a silent no-op.
    ///
    /// 1. Run `avm init`, then `avm alias remove ghost` with no such alias.
    /// 2. It fails with "alias 'ghost' not found".
    #[test]
    fn removing_a_missing_alias_fails() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["init"]));
        assert_that(&p.run(&["alias", "remove", "ghost"])).fails_with("alias 'ghost' not found");
    }

    /// Project-scoped commands need a project config; they point the user at `avm init`.
    ///
    /// 1. In a directory without `.avm.json`, run `avm alias add hi "echo hi"`.
    /// 2. It fails with "no .avm.json found" and "avm init".
    #[test]
    fn a_local_add_needs_an_initialised_project() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["alias", "add", "hi", "echo hi"]))
            .fails_with("no .avm.json found")
            .stderr_contains("avm init");
        assert_that(&p.path("project/.avm.json")).does_not_exist();
    }

    /// An empty alias name is rejected.
    ///
    /// 1. Run `avm alias add "" "echo x"`.
    /// 2. It fails with "invalid alias name".
    #[test]
    fn adding_an_empty_name_fails() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["alias", "add", "", "echo x"])).fails_with("invalid alias name");
    }

    /// Adding a name that already exists asks before replacing it; `-y` skips the question.
    ///
    /// 1. Add `gg` = `echo old`, then add `gg` = `echo new` with no terminal to ask on.
    /// 2. It fails, names both commands, and tells the user to re-run with `-y`.
    /// 3. The alias still runs `echo old`; with `-y` the same add replaces it.
    #[test]
    fn adding_an_existing_alias_asks_before_overriding() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "gg", "echo old"]));
        assert_that(&p.run(&["alias", "add", "-g", "gg", "echo new"]))
            .fails_with("alias 'gg' already exists")
            .fails_with("current: echo old")
            .fails_with("Re-run with -y");
        assert_that(&p.run(&["gg"]))
            .succeeds()
            .stdout_contains("old");
        success(p.run(&["alias", "add", "-y", "-g", "gg", "echo new"]));
        assert_that(&p.run(&["gg"]))
            .succeeds()
            .stdout_contains("new");
    }

    /// `avm <name>` is shorthand for `avm run <name>`, forwarding arguments.
    ///
    /// 1. Add the global alias `greet` = `echo hello`.
    /// 2. `avm greet` prints "hello"; `avm greet there` appends the extra argument.
    #[test]
    fn a_bare_alias_name_runs_it() {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "greet", "echo hello"]));
        assert_that(&success(p.run(&["greet"]))).is_equal_to("hello\n".to_string());
        assert_that(&success(p.run(&["greet", "there"]))).is_equal_to("hello there\n".to_string());
    }

    /// An unknown name fails instead of running anything.
    ///
    /// 1. Run `avm no-such-thing` and `avm run no-such-thing`.
    /// 2. Both fail with "alias 'no-such-thing' not found".
    #[rstest]
    #[case::bare(&["no-such-thing"])]
    #[case::run(&["run", "no-such-thing"])]
    fn an_unknown_alias_fails(#[case] args: &[&str]) {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(args)).fails_with("alias 'no-such-thing' not found");
    }
}
