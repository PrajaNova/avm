//! Config resolution: layering, profiles, trust, caching and corrupt-file safety.
//! Source feature: `crates/avm-cli/src/core/` (config, resolver, trust).

use crate::common::*;
use fs_err as fs;
use speculoos::prelude::*;

/// Config layers (global, parent, child, local, profiles) merge in a fixed order and every value reports where it came from.
///
/// 1. Create global, repo-root, repo-local, child, `ci` and `staging` config files with overlapping keys.
/// 2. With `AVM_ENV=ci,staging`, list aliases, env and tools: the last profile wins and origins name the file.
/// 3. `which`, `run` and a typo suggestion (`resolve paren`) use the merged result.
/// 4. A child `.avm.local.json` overrides profiles; removing it, `--env ci` or an empty `--env` change the result accordingly.
/// 5. A child `{"root":true}` stops inheritance from the parent; a broken file above a root marker is never read.
#[test]
fn hierarchical_profiles_merge_and_report_origins() {
    let p = Sandbox::new("avm-test");
    let repo = p.root.join("project");
    let nested = repo.join("apps/tv");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        p.root.join("home/.avm.json"),
        r#"{"aliases":{"global":"echo global"},"env":{"GLOBAL":"yes"},"tools":{"java":"17"}}"#,
    )
    .unwrap();
    fs::write(repo.join(".avm.json"), r#"{"root":true,"aliases":{"build":"echo root","parent":"echo inherited"},"env":{"BASE":"yes","MODE":"root"},"tools":{"node":"18"}}"#).unwrap();
    fs::write(
        repo.join(".avm.local.json"),
        r#"{"env":{"MODE":"parent-local"}}"#,
    )
    .unwrap();
    fs::write(
        nested.join(".avm.json"),
        r#"{"aliases":{"build":"echo child"},"env":{"MODE":"child"},"tools":{"node":"20"}}"#,
    )
    .unwrap();
    fs::write(
        nested.join(".avm.ci.json"),
        r#"{"env":{"MODE":"ci","CI_ONLY":"yes"},"tools":{"node":"22"}}"#,
    )
    .unwrap();
    fs::write(
        nested.join(".avm.staging.json"),
        r#"{"aliases":{"build":"echo staging"},"env":{"MODE":"staging"}}"#,
    )
    .unwrap();
    let run = |args: &[&str]| {
        success(
            p.command(args)
                .current_dir(&nested)
                .env("AVM_TRUST_ALL", "1")
                .env("AVM_ENV", "ci,staging")
                .output()
                .unwrap(),
        )
    };
    let list = [
        run(&["alias", "list"]),
        run(&["env", "list"]),
        run(&["plugin", "list"]),
    ]
    .join("\n");
    for text in [
        "build → echo staging",
        "parent → echo inherited",
        "global → echo global",
        "BASE=yes",
        "MODE=staging",
        "CI_ONLY=yes",
        "GLOBAL=yes",
        ".avm.staging.json",
        ".avm.ci.json",
    ] {
        assert!(list.contains(text), "missing {text}: {list}");
    }
    for (tool, version, scope) in [("node", "22", "local"), ("java", "17", "global")] {
        assert!(
            list.lines()
                .any(|l| l.starts_with(tool) && l.contains(version) && l.contains(scope)),
            "missing {tool} row: {list}"
        );
    }
    assert!(run(&["which", "build"]).contains(".avm.staging.json"));
    assert_eq!(run(&["run", "parent"]).trim(), "inherited");
    assert!(run(&["which", "node"]).contains(".avm.ci.json"));
    let suggestion = p
        .command(&["resolve", "paren"])
        .current_dir(&nested)
        .env("AVM_TRUST_ALL", "1")
        .output()
        .unwrap();
    assert_that(&suggestion).stderr_contains("avm parent");
    fs::write(
        nested.join(".avm.local.json"),
        r#"{"env":{"MODE":"personal"},"tools":{"node":"24"}}"#,
    )
    .unwrap();
    assert!(run(&["env", "--shell", "sh"]).contains("MODE=personal"));
    assert!(run(&["which", "node"]).contains(".avm.local.json"));
    fs::remove_file(nested.join(".avm.local.json")).unwrap();
    assert!(run(&["env", "--shell", "sh", "--env", "ci"]).contains("MODE=ci"));
    assert!(run(&["--env", "", "env", "--shell", "sh"]).contains("MODE=child"));
    // A root marker containing no maps is still a structured config.
    fs::write(nested.join(".avm.json"), r#"{"root":true}"#).unwrap();
    let list = run(&["--env", "", "alias", "list"]);
    assert!(!list.contains("parent →"));
    assert_that(&list).contains("global →");
    fs::write(repo.join(".avm.json"), "malformed above root").unwrap();
    assert!(run(&["--env", "", "env", "--shell", "sh"]).contains("GLOBAL"));
}

/// Each config layer is trusted separately, and the config cache never serves stale or untrusted content.
///
/// 1. Create a trusted-by-path parent root and an untrusted child `ci` profile.
/// 2. Before trusting the child, only the parent's alias applies; after `avm trust`, the profile applies.
/// 3. Edit the profile but restore its mtime: the change is not trusted until trusted again (content hash, not mtime).
/// 4. `trust --revoke` drops the layer again; an invalid profile makes `env` fail.
/// 5. Corrupt every cache file: commands recover because the cache is disposable.
#[test]
fn trust_and_cache_invalidation_apply_to_each_layer() {
    let p = Sandbox::new("avm-test");
    let repo = p.root.join("project");
    let child = repo.join("child");
    fs::create_dir(&child).unwrap();
    fs::write(
        repo.join(".avm.json"),
        r#"{"root":true,"aliases":{"build":"echo parent"},"env":{"MODE":"parent"}}"#,
    )
    .unwrap();
    let profile = child.join(".avm.ci.json");
    fs::write(
        &profile,
        r#"{"aliases":{"build":"echo profile"},"env":{"MODE":"profile"},"tools":{"node":"22"}}"#,
    )
    .unwrap();
    let run = |args: &[&str]| {
        success(
            p.command(args)
                .current_dir(&child)
                .env("AVM_ENV", "ci")
                .output()
                .unwrap(),
        )
    };
    success(p.run(&["trust"]));
    assert!(run(&["which", "build"]).contains("echo parent"));
    assert!(run(&["which", "node"]).contains(".avm.ci.json"));
    run(&["trust"]);
    assert!(run(&["which", "build"]).contains("echo profile"));
    assert!(run(&["env", "--shell", "sh"]).contains("MODE=profile")); // warm the cache
    let old_mtime = fs::metadata(&profile).unwrap().modified().unwrap();
    fs::write(
        &profile,
        r#"{"aliases":{"build":"echo changed"},"env":{"MODE":"changed"},"tools":{"node":"22"}}"#,
    )
    .unwrap();
    fs::File::options()
        .write(true)
        .open(&profile)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(old_mtime))
        .unwrap();
    assert!(run(&["which", "build"]).contains("echo parent"));
    run(&["trust"]);
    assert!(run(&["which", "build"]).contains("echo changed"));
    run(&["trust", "--revoke", profile.to_str().unwrap()]);
    assert!(run(&["which", "build"]).contains("echo parent"));
    fs::write(&profile, "invalid json").unwrap();
    assert!(!p
        .command(&["env"])
        .current_dir(&child)
        .env("AVM_ENV", "ci")
        .output()
        .unwrap()
        .status
        .success());
    fs::remove_file(&profile).unwrap();
    assert!(run(&["env", "--shell", "sh"]).contains("MODE=parent"));
    // Broken cache is disposable.
    for entry in fs::read_dir(p.root.join("home/.avm/cache")).unwrap() {
        fs::write(entry.unwrap().path(), "invalid cache").unwrap();
    }
    assert!(run(&["env", "--shell", "sh"]).contains("MODE=parent"));
}

/// A config that cannot be parsed is never overwritten with an empty one (no silent data loss).
///
/// 1. Write a broken global `.avm.json` that still contains a valuable alias.
/// 2. `alias add` and `plugin prune --yes` both fail asking the user to "repair the file".
/// 3. The file's bytes are unchanged afterwards.
#[test]
fn malformed_configs_are_never_replaced_with_empty_state() {
    let p = Sandbox::new("avm-test");
    let file = p.root.join("home/.avm.json");
    let raw = br#"{"aliases":{"valuable":"echo saved"}, broken"#;
    fs::write(&file, raw).unwrap();
    for args in [
        vec!["alias", "add", "-g", "new", "echo new"],
        vec!["plugin", "prune", "--yes"],
    ] {
        let out = p.run(&args);
        assert_that(&out).fails();
        assert_that(&out).stderr_contains("repair the file");
        assert_eq!(fs::read(&file).unwrap(), raw);
    }
}

/// `.env` files: loaded from every parent directory, but only when trusted. The aliases here use
/// POSIX `$VAR` expansion, so this module is unix-only.
#[cfg(unix)]
mod dotenv {
    use super::*;

    /// A sandbox whose global alias `showdot` prints `$DOT`, so `.env` effects are visible
    /// without having to trust a project config.
    fn with_alias() -> Sandbox {
        let p = Sandbox::new("avm-test");
        success(p.run(&["alias", "add", "-g", "showdot", "echo \"[$DOT]\""]));
        p
    }

    /// An untrusted `.env` is not applied.
    ///
    /// 1. Write `project/.env` with `DOT=from-env`; do not trust it.
    /// 2. `avm run showdot` prints an empty value.
    #[test]
    fn untrusted_env_files_are_ignored() {
        let p = with_alias();
        p.write("project/.env", "DOT=from-env\n");
        assert_that(&success(p.run(&["run", "showdot"]))).is_equal_to("[]\n".to_string());
    }

    /// Once trusted, the `.env` applies.
    ///
    /// 1. Write `project/.env`, run `avm trust .env`.
    /// 2. `avm run showdot` prints `from-env`.
    #[test]
    fn trusted_env_files_are_applied() {
        let p = with_alias();
        p.write("project/.env", "DOT=from-env\n");
        success(p.run(&["trust", ".env"]));
        assert_that(&success(p.run(&["run", "showdot"]))).is_equal_to("[from-env]\n".to_string());
    }

    /// `KEY=VALUE`, `export KEY=VALUE`, comments and blank lines are understood.
    ///
    /// 1. Write a `.env` mixing those forms and run with everything trusted.
    /// 2. The `export`ed assignment takes effect.
    #[test]
    fn the_env_file_syntax_is_parsed() {
        let p = with_alias();
        p.write(
            "project/.env",
            "# a comment\n\nOTHER=1\nexport DOT=exported\n",
        );
        assert_that(&success(p.run_trusted(&["run", "showdot"])))
            .is_equal_to("[exported]\n".to_string());
    }

    /// A `.env` nearer the working directory overrides one further up.
    ///
    /// 1. Put `DOT=parent` in `project/.env` and `DOT=child` in `project/child/.env`.
    /// 2. Running from `project/child` prints `child`; from `project` it prints `parent`.
    #[test]
    fn nearer_env_files_win() {
        let p = with_alias();
        p.write("project/.env", "DOT=parent\n");
        p.write("project/child/.env", "DOT=child\n");
        let run_in = |dir: &str| {
            stdout_of(
                &p.command_in(&["run", "showdot"], dir)
                    .env("AVM_TRUST_ALL", "1")
                    .output()
                    .unwrap(),
            )
        };
        assert_that(&run_in("project/child")).is_equal_to("[child]\n".to_string());
        assert_that(&run_in("project")).is_equal_to("[parent]\n".to_string());
    }

    /// Variables already in the process environment are never overwritten by a `.env`.
    ///
    /// 1. Write `DOT=inner` in a trusted `.env` and start avm with `DOT=outer`.
    /// 2. `avm run showdot` prints `outer`.
    #[test]
    fn existing_process_variables_are_protected() {
        let p = with_alias();
        p.write("project/.env", "DOT=inner\n");
        let out = p
            .command(&["run", "showdot"])
            .env("AVM_TRUST_ALL", "1")
            .env("DOT", "outer")
            .output()
            .unwrap();
        assert_that(&stdout_of(&out)).is_equal_to("[outer]\n".to_string());
    }

    /// A line that is not an assignment is reported with its location.
    ///
    /// 1. Write a trusted `.env` whose second line has no `=`.
    /// 2. avm fails with "invalid env assignment" and `.env:2`.
    #[test]
    fn a_malformed_line_is_an_error() {
        let p = with_alias();
        p.write("project/.env", "GOOD=1\nthis is not an assignment\n");
        assert_that(&p.run_trusted(&["run", "showdot"]))
            .fails_with("invalid env assignment")
            .stderr_contains(".env:2");
    }
}
