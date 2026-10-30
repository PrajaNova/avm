//! Project lifecycle: `init`, `trust`, `plugin prune`, and (unix) `plugin lock` / frozen install.
//! Source feature: `crates/avm-cli/src/project/`.

use crate::common::*;
use fs_err as fs;
use speculoos::prelude::*;

/// `avm init` keeps personal config out of git, and later edits keep the `root` marker.
///
/// 1. Create a `.gitignore` containing `target`, then run `avm init`.
/// 2. `.gitignore` gains a `.avm.local.json` line.
/// 3. Replace the config with `{"root":true}` and add an alias: `root` is still true.
#[test]
fn init_gitignores_personal_config_and_preserves_root_on_edit() {
    let p = Sandbox::new("avm-test");
    fs::write(p.root.join("project/.gitignore"), "target").unwrap();
    success(p.run(&["init"]));
    assert_eq!(
        fs::read_to_string(p.root.join("project/.gitignore")).unwrap(),
        "target\n.avm.local.json\n"
    );
    fs::write(p.root.join("project/.avm.json"), r#"{"root":true}"#).unwrap();
    success(p.run(&["alias", "add", "build", "echo build"]));
    let cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(p.root.join("project/.avm.json")).unwrap()).unwrap();
    assert_eq!(cfg["root"], true);
}

/// Trust and project-tracking stores are updated under a lock, so parallel commands keep every entry.
///
/// 1. Create 12 projects; for each, run `avm trust <dir>` and `avm plugin list` in parallel.
/// 2. All processes succeed.
/// 3. `trusted.json` has 12 entries and `tracked-configs.json` has 12 entries.
#[test]
fn concurrent_project_tracking_and_trust_preserve_every_project() {
    let p = Sandbox::new("avm-test");
    let mut children = Vec::new();
    for n in 0..12 {
        let dir = p.root.join(format!("project{n}"));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
        for args in [vec!["trust", dir.to_str().unwrap()], vec!["plugin", "list"]] {
            children.push(
                p.command(&args)
                    .current_dir(&dir)
                    .stdout(std::process::Stdio::null())
                    .spawn()
                    .unwrap(),
            );
        }
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    for (file, object) in [("trusted.json", true), ("tracked-configs.json", false)] {
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(p.root.join("home/.avm").join(file)).unwrap())
                .unwrap();
        let count = if object {
            value.as_object().unwrap().len()
        } else {
            value.as_array().unwrap().len()
        };
        assert_eq!(count, 12, "{file}");
    }
}

/// `plugin prune` refuses to delete anything when a tracked project's config cannot be resolved.
///
/// 1. Install node 20.2.0 (marked used), track the project and give it an invalid lock.
/// 2. `plugin prune --yes` fails with "cannot safely prune" and the version stays.
/// 3. Fix the project and pin node 20 globally: prune succeeds and still keeps the version.
#[test]
fn prune_aborts_when_a_tracked_project_cannot_resolve() {
    let p = Sandbox::new("avm-test");
    let version = p.root.join("home/.avm/tools/node/20.2.0");
    fs::create_dir_all(&version).unwrap();
    fs::write(version.join(".last_used"), "").unwrap();
    fs::write(
        p.root.join("home/.avm/tracked-configs.json"),
        serde_json::json!([p.root.join("project")]).to_string(),
    )
    .unwrap();
    fs::write(p.root.join("project/.avm.lock"), "invalid lock").unwrap();
    let out = p.run(&["plugin", "prune", "--yes"]);
    assert_that(&out).fails();
    assert_that(&out).stderr_contains("cannot safely prune");
    assert_that(&version).exists();
    fs::remove_file(p.root.join("project/.avm.lock")).unwrap();
    fs::write(p.root.join("home/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
    assert_that(&p.run(&["plugin", "prune", "--yes"])).succeeds();
    assert_that(&version).exists();
}

/// Versions pinned only by an inactive profile (`.avm.ci.json`) are still protected from prune.
///
/// 1. Install node 20.1.0, 22.1.0 and 24.1.0; the project pins 20 and its `ci` profile pins 22.
/// 2. A dry-run prune lists only 24.1.0.
/// 3. Break the profile file: a real prune fails and 24.1.0 is kept.
#[test]
fn prune_keeps_inactive_profile_pins() {
    let p = Sandbox::new("avm-test");
    let home = p.root.join("home/.avm");
    for version in ["20.1.0", "22.1.0", "24.1.0"] {
        fs::create_dir_all(home.join("tools/node").join(version)).unwrap();
    }
    let repo = p.root.join("project");
    fs::write(
        repo.join(".avm.json"),
        r#"{"root":true,"tools":{"node":"20"}}"#,
    )
    .unwrap();
    fs::write(repo.join(".avm.ci.json"), r#"{"tools":{"node":"22"}}"#).unwrap();
    fs::write(
        home.join("tracked-configs.json"),
        serde_json::json!([repo]).to_string(),
    )
    .unwrap();
    let out = success(p.run(&["plugin", "prune", "--dry-run", "--include-unrecorded"]));
    assert_that(&out).contains("node 24.1.0");
    assert!(!out.contains("node 20.1.0") && !out.contains("node 22.1.0"));
    fs::write(repo.join(".avm.ci.json"), "broken profile").unwrap();
    assert!(!p
        .run(&["plugin", "prune", "--yes", "--include-unrecorded"])
        .status
        .success());
    assert_that(&home.join("tools/node/24.1.0")).exists();
}

/// Prune clears config-cache files older than 30 days and keeps recent ones.
///
/// 1. Create two cache files and age one to 31 days.
/// 2. Run `plugin prune --yes`.
/// 3. The old file is gone and the fresh file remains.
#[test]
fn prune_removes_only_stale_config_cache_files() {
    let p = Sandbox::new("avm-test");
    let cache = p.root.join("home/.avm/cache");
    fs::create_dir_all(&cache).unwrap();
    let old = cache.join("config-0000000000000001.json");
    let fresh = cache.join("config-0000000000000002.json");
    fs::write(&old, "{}").unwrap();
    fs::write(&fresh, "{}").unwrap();
    let month_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 86_400);
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(month_ago)
        .unwrap();
    fs::write(p.root.join("home/.avm.json"), "{}").unwrap();
    assert_that(&p.run(&["plugin", "prune", "--yes"])).succeeds();
    assert!(!old.exists() && fresh.exists());
}

/// `plugin lock` and `plugin install --frozen`, using `LockProject` (python plugins that "download"
/// a local archive and log every install, so a test can prove nothing was installed).
#[cfg(unix)]
mod lock {
    use super::*;
    use crate::fixtures::projects::{locked_platform, LockProject};
    use rstest::rstest;
    use serde_json::{json, Value};
    use std::os::unix::fs::PermissionsExt;

    /// `plugin lock` run from a subdirectory updates the project's lock, and profiles get their own lock next to their config.
    ///
    /// 1. From `packages/web`, lock with no existing lock: the lock is written at the project root, not in the subdirectory.
    /// 2. Change the remote version and relock: the root lock updates and keeps its file mode (0640).
    /// 3. Relock over an invalid lock file: it is replaced by a valid one.
    /// 4. With a `ci` profile in the subdirectory, `--env ci plugin lock` writes a separate lock there and leaves the parent lock alone.
    /// 5. `--env ci -p install --frozen` succeeds against that lock.
    #[test]
    fn nested_lock_commands_update_the_project_lock() {
        let project = LockProject::new();
        fs::create_dir_all(project.root.join("project/packages/web")).unwrap();
        // No existing lock: discover the project config rather than the home config.
        success(project.run_in(&["plugin", "lock"], "project/packages/web"));
        assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.2.0");
        assert_that(&project.root.join("project/packages/web/.avm.lock")).does_not_exist();
        fs::set_permissions(
            project.root.join("project/.avm.lock"),
            std::fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        fs::write(project.root.join("remote"), "20.3.0").unwrap();
        success(project.run_in(&["plugin", "lock"], "project/packages/web"));
        assert_eq!(
            fs::metadata(project.root.join("project/.avm.lock"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
        assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.3.0");
        assert_that(&project.root.join("project/packages/web/.avm.lock")).does_not_exist();
        fs::write(project.root.join("project/.avm.lock"), "invalid lock").unwrap();
        success(project.run_in(&["plugin", "lock"], "project/packages/web"));
        assert_eq!(project.lock()["tools"].as_object().unwrap().len(), 1);
        let parent_lock = project.lock();
        let child = project.root.join("project/packages/web");
        fs::write(child.join(".avm.ci.json"), r#"{"tools":{"node":"20.2.0"}}"#).unwrap();
        success(project.run_in(&["--env", "ci", "plugin", "lock"], "project/packages/web"));
        let lock: Value =
            serde_json::from_slice(&fs::read(child.join(".avm.lock")).unwrap()).unwrap();
        assert_eq!(lock["tools"]["node"]["requested"], "20.2.0");
        assert_eq!(project.lock(), parent_lock);
        success(project.run_in(
            &["--env", "ci", "-p", "install", "--frozen"],
            "project/packages/web",
        ));
    }

    /// End-to-end lock lifecycle: lock, frozen install, drift detection and tamper detection, always checked before anything is installed.
    ///
    /// 1. Lock the project (personal global pins are excluded); frozen install ignores global pin changes.
    /// 2. Lock several platforms: output is stable regardless of platform order and records resolved versions and assets.
    /// 3. Frozen install installs exactly the locked versions, in order; a normal install does not reinstall or touch configs.
    /// 4. A modified or re-versioned plugin binary is rejected; a missing binary pin only warns.
    /// 5. A changed pin makes frozen install fail and other commands warn "out of date".
    /// 6. A tampered archive fails with "checksum mismatch" and installs nothing.
    /// 7. Relocking for another platform makes frozen install fail with "has no ..." for the host; an unsupported platform name fails without touching the lock.
    /// 8. Plain `install` recreates a missing lock; legacy asdf plugins install but cannot be locked.
    #[test]
    fn locks_installs_and_rejects_drift_before_installing() {
        let project = LockProject::new();
        // A teammate's home pins must not enter or invalidate this project's lock.
        success(project.run(&["plugin", "lock"]));
        let local_lock = fs::read(project.root.join("project/.avm.lock")).unwrap();
        assert_eq!(project.lock()["tools"].as_object().unwrap().len(), 1);
        assert!(project.lock()["tools"].get("java").is_none());
        fs::write(
            project.root.join("home/.avm.json"),
            r#"{"tools":{"java":"17","node":"99"}}"#,
        )
        .unwrap();
        success(project.run(&["-p", "install", "--frozen"]));
        assert_that(&success(project.run(&["which", "node"]))).contains("20.2.0");
        assert_that(&project.root.join("home/.avm/tools/java")).does_not_exist();
        success(project.run(&["node", "use", "22.0.0", "--global"]));
        success(project.run(&["-p", "install", "--frozen"]));
        assert_that(&success(project.run(&["which", "node"]))).contains("20.2.0");
        fs::write(project.root.join("home/.avm.json"), "{}").unwrap();
        success(project.run(&["-p", "install", "--frozen"]));
        assert_eq!(
            fs::read(project.root.join("project/.avm.lock")).unwrap(),
            local_lock
        );
        fs::remove_dir_all(project.root.join("home/.avm/tools")).unwrap();
        fs::remove_file(project.root.join("installs")).unwrap();
        fs::remove_file(project.root.join("project/.avm.lock")).unwrap();
        fs::write(
            project.root.join("home/.avm.json"),
            r#"{"tools":{"java":"21"}}"#,
        )
        .unwrap();
        fs::write(
            project.root.join("project/.avm.json"),
            r#"{"tools":{"node":"20","java":"21"}}"#,
        )
        .unwrap();
        let config = fs::read(project.root.join("project/.avm.json")).unwrap();
        let global = fs::read(project.root.join("home/.avm.json")).unwrap();
        failure(
            project.run(&["-p", "install", "--frozen"]),
            "requires .avm.lock",
        );
        success(project.run(&[
            "plugin",
            "lock",
            "--platform",
            "linux_amd64,darwin_arm64,windows_amd64",
        ]));
        assert_that(&project.root.join("installs")).does_not_exist();
        let first = fs::read(project.root.join("project/.avm.lock")).unwrap();
        success(project.run(&[
            "plugin",
            "lock",
            "--platform",
            "windows_amd64,linux_amd64,darwin_arm64",
        ]));
        assert_eq!(
            first,
            fs::read(project.root.join("project/.avm.lock")).unwrap()
        );
        let lock = project.lock();
        assert_eq!(lock["tools"]["node"]["requested"], "20");
        assert_eq!(lock["tools"]["node"]["resolved"], "20.2.0");
        assert_eq!(
            lock["tools"]["node"]["assets"].as_object().unwrap().len(),
            3
        );
        fs::write(project.root.join("remote"), "20.3.0").unwrap();
        assert_that(&success(project.run(&["which", "node"]))).contains("20.2.0");
        fs::create_dir(project.root.join("project/child")).unwrap();
        fs::write(project.root.join("project/.nvmrc"), "20\n").unwrap();
        fs::write(project.root.join("project/.tool-versions"), "java 21\n").unwrap();
        assert_that(&success(
            project.run_in(&["which", "node"], "project/child"),
        ))
        .contains("20.2.0");
        success(project.run(&["-p", "install", "--frozen"]));
        let installs = fs::read(project.root.join("installs")).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&installs),
            "java:openjdk-21.1.0+1\nnode:20.2.0\n"
        );
        success(project.run(&["-p", "install"]));
        assert_eq!(fs::read(project.root.join("installs")).unwrap(), installs);
        assert_eq!(
            fs::read(project.root.join("project/.avm.json")).unwrap(),
            config
        );
        assert_eq!(
            fs::read(project.root.join("home/.avm.json")).unwrap(),
            global
        );

        let plugin = project
            .root
            .join("home/.avm/plugins/avm-plugin-node/bin/avm-plugin");
        let original_plugin = fs::read_to_string(&plugin).unwrap();
        fs::write(
            &plugin,
            format!("{original_plugin}\n# altered executable\n"),
        )
        .unwrap();
        failure(
            project.run(&["-p", "install", "--frozen"]),
            "plugin checksum differs",
        );
        fs::write(
            &plugin,
            original_plugin.replace("version='1.0.0'", "version='2.0.0'"),
        )
        .unwrap();
        failure(
            project.run(&["-p", "install", "--frozen"]),
            "plugin differs",
        );
        fs::write(&plugin, &original_plugin).unwrap();

        let mut unpinned = lock.clone();
        unpinned["tools"]["node"]["plugin"]["binaries"] = json!({});
        project.save_lock(&unpinned);
        let out = project.run(&["-p", "install", "--frozen"]);
        let warning = String::from_utf8_lossy(&out.stderr).into_owned();
        success(out);
        assert!(
            warning.contains(&format!(
                "node plugin binary not pinned for {}",
                avm_plugin_sdk::download::host_platform().unwrap()
            )),
            "{warning}"
        );
        project.save_lock(&lock);

        fs::write(
            project.root.join("project/.avm.json"),
            r#"{"tools":{"node":"21"}}"#,
        )
        .unwrap();
        failure(project.run(&["-p", "install", "--frozen"]), "out of date");
        let env = project.run(&["env"]);
        assert_that(&env).stderr_contains("out of date");
        success(env);
        assert_eq!(fs::read(project.root.join("installs")).unwrap(), installs);
        fs::write(project.root.join("project/.avm.json"), config).unwrap();

        fs::remove_dir_all(project.root.join("home/.avm/tools/node")).unwrap();
        fs::write(project.root.join("archive"), "tampered").unwrap();
        failure(
            project.run(&["-p", "install", "--frozen"]),
            "checksum mismatch",
        );
        failure(
            project.run(&["node", "install", "20", "--no-pin"]),
            "checksum mismatch",
        );
        failure(project.run(&["node", "use", "20.2.0"]), "checksum mismatch");
        assert_eq!(fs::read(project.root.join("installs")).unwrap(), installs);
        assert_that(&project.root.join("home/.avm/tools/node/20.2.0")).does_not_exist();

        // Temporary overrides resolve independently of the project's locked pin.
        assert_eq!(
            success(project.run(&["exec", "node@20", "--", "node"])),
            "20.3.0\n"
        );
        assert_eq!(project.lock(), lock);

        // Regeneration recovers from a stale lock and keeps all writes atomic.
        success(project.run(&["plugin", "lock"]));
        let host_lock = project.lock();
        let host = host_lock["tools"]["node"]["assets"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap();
        let other = if host == "linux_amd64" {
            "darwin_arm64"
        } else {
            "linux_amd64"
        };
        success(project.run(&["plugin", "lock", "--platform", other]));
        failure(project.run(&["-p", "install", "--frozen"]), "has no");

        let old = fs::read(project.root.join("project/.avm.lock")).unwrap();
        failure(
            project.run(&["plugin", "lock", "--platform", "linux_x64"]),
            "unsupported lock platform",
        );
        assert_eq!(
            old,
            fs::read(project.root.join("project/.avm.lock")).unwrap()
        );
        fs::remove_file(project.root.join("project/.avm.lock")).unwrap();
        success(project.run(&["-p", "install"]));
        assert_that(&project.root.join("project/.avm.lock")).exists();
        fs::remove_file(project.root.join("project/.avm.lock")).unwrap();
        assert!(fs::read_to_string(project.root.join("installs"))
            .unwrap()
            .contains("node:20.3.0"));

        let legacy = project.root.join("home/.avm/plugins/asdf-ruby/bin");
        fs::create_dir_all(&legacy).unwrap();
        let versions = (1..=12)
            .map(|n| format!("{n}.0.0"))
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(
            legacy.join("list-all"),
            format!("#!/bin/sh\necho '{versions}'\n"),
        )
        .unwrap();
        fs::write(legacy.join("install"), "#!/bin/sh\nmkdir -p \"$ASDF_INSTALL_PATH/bin\"\nprintf '#!/bin/sh\\necho ruby\\n' > \"$ASDF_INSTALL_PATH/bin/ruby\"\nchmod +x \"$ASDF_INSTALL_PATH/bin/ruby\"\n").unwrap();
        for name in ["list-all", "install"] {
            fs::set_permissions(legacy.join(name), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(
            project.root.join("project/.avm.json"),
            r#"{"tools":{"node":"20","ruby":"latest"}}"#,
        )
        .unwrap();
        success(project.run(&["-p", "install"]));
        assert!(project
            .root
            .join("home/.avm/tools/ruby/12.0.0/bin/ruby")
            .exists());
        failure(project.run(&["plugin", "lock"]), "requires a native plugin");
        assert_that(&project.root.join("project/.avm.lock")).does_not_exist();
    }

    mod frozen_install_rejects {
        use super::*;

        /// Table: a frozen install rejects each kind of corrupted lock before installing anything.
        ///
        /// 1. Lock a project pinning node 20 and java 21.
        /// 2. Apply the corruption from the table and save the lock.
        /// 3. `plugin install --frozen` fails with the expected message.
        #[rstest]
        #[case::malformed_checksum(
            |lock: &mut Value| {
                let platform = locked_platform(lock);
                lock["tools"]["node"]["assets"][&platform]["sha256"] = json!("not-a-hash");
            },
            "invalid sha256"
        )]
        #[case::java_resolved_outside_request(
            |lock: &mut Value| {
                lock["tools"]["java"]["requested"] = json!("openjdk-21.1.0+1");
                lock["tools"]["java"]["resolved"] = json!("openjdk-21.1.0+2");
            },
            "does not satisfy"
        )]
        #[case::node_resolved_outside_range(
            |lock: &mut Value| lock["tools"]["node"]["resolved"] = json!("22.0.0"),
            "does not satisfy"
        )]
        #[case::path_traversal_version(
            |lock: &mut Value| lock["tools"]["node"]["resolved"] = json!("../escape"),
            "invalid resolved version"
        )]
        #[case::future_lock_version(
            |lock: &mut Value| lock["version"] = json!(99),
            "unsupported .avm.lock"
        )]
        fn a_corrupted_lock(#[case] corrupt: fn(&mut Value), #[case] expected: &str) {
            let (project, mut lock) = LockProject::locked();
            corrupt(&mut lock);
            project.save_lock(&lock);
            failure(project.run(&["-p", "install", "--frozen"]), expected);
        }
    }

    /// A plain `plugin install` creates `.avm.lock` when the project has none.
    ///
    /// 1. Run `avm -p install` in a project without a lock.
    /// 2. The lock records node resolved to 20.2.0.
    #[test]
    fn install_writes_the_lock_when_missing() {
        let project = LockProject::new();
        success(project.run(&["-p", "install"]));
        assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.2.0");
    }

    /// `install` in a project with no tools explains itself and writes no lock.
    ///
    /// 1. Use an empty project config.
    /// 2. `avm -p install` says "No tools in .avm.json" and lists the global java pin.
    /// 3. No lock file is written.
    /// 4. Running it again reports "already installed".
    #[test]
    fn install_in_a_project_without_tools_says_so() {
        let project = LockProject::new();
        project.write_config("{}");
        let out = success(project.run(&["-p", "install"]));
        assert_that(&out).contains("No tools in .avm.json");
        assert!(out.contains("java") && out.contains("(global)"), "{out}");
        assert_that(&project.root.join("project/.avm.lock")).does_not_exist();
        let again = success(project.run(&["-p", "install"]));
        assert_that(&again).contains("already installed");
    }

    /// `plugin upgrade` with nothing pinned prints a clear message.
    ///
    /// 1. Empty project and global configs.
    /// 2. `avm plugin upgrade` prints "No tools pinned here."
    #[test]
    fn upgrade_without_pins_says_so() {
        let project = LockProject::new();
        project.write_config("{}");
        fs::write(project.root.join("home/.avm.json"), "{}").unwrap();
        assert_that(&success(project.run(&["plugin", "upgrade"])))
            .contains("No tools pinned here.");
    }

    /// `lock` and `install --frozen` without tools say there is nothing to do and create no lock.
    ///
    /// 1. Use an empty project config.
    /// 2. `plugin lock` prints "Nothing to lock"; no lock file exists.
    /// 3. `install --frozen` prints "Nothing to install".
    #[test]
    fn lock_and_frozen_install_without_tools_are_explicit() {
        let project = LockProject::new();
        project.write_config("{}");
        assert_that(&success(project.run(&["plugin", "lock"]))).contains("Nothing to lock");
        assert_that(&project.root.join("project/.avm.lock")).does_not_exist();
        assert_that(&success(project.run(&["-p", "install", "--frozen"])))
            .contains("Nothing to install");
    }

    /// A lock that no longer matches the config only blocks frozen installs; everyday commands warn.
    ///
    /// 1. Lock, then change the project pin to node 21.
    /// 2. `which node` succeeds with an "out of date" warning; `env` succeeds.
    /// 3. `install --frozen` fails with "out of date".
    #[test]
    fn a_stale_lock_warns_but_does_not_break_commands() {
        let project = LockProject::new();
        success(project.run(&["plugin", "lock"]));
        project.write_config(r#"{"tools":{"node":"21"}}"#);
        let which = project.run(&["which", "node"]);
        assert_that(&which).stderr_contains("out of date");
        success(which);
        success(project.run(&["env"]));
        failure(project.run(&["-p", "install", "--frozen"]), "out of date");
    }

    /// Changing a pin with `node use` regenerates an existing lock instead of leaving it stale.
    ///
    /// 1. Lock the project.
    /// 2. Run `avm node use 20.2.0`.
    /// 3. `env` shows no "out of date" warning and the lock requests 20.2.0.
    #[test]
    fn changing_a_pin_refreshes_an_existing_lock() {
        let project = LockProject::new();
        success(project.run(&["plugin", "lock"]));
        success(project.run(&["node", "use", "20.2.0"]));
        let out = project.run(&["env"]);
        assert_that(&out).stderr_lacks("out of date");
        assert_eq!(project.lock()["tools"]["node"]["requested"], "20.2.0");
    }

    /// A plugin too old to support locking gets an actionable error.
    ///
    /// 1. Break the node plugin's `resolve` command.
    /// 2. `plugin lock` fails telling the user to run `avm plugin install node`.
    /// 3. A plain `install` still works and prints the same hint on stderr.
    #[test]
    fn old_plugins_are_told_how_to_update() {
        let project = LockProject::new();
        let plugin = project
            .root
            .join("home/.avm/plugins/avm-plugin-node/bin/avm-plugin");
        let script = fs::read_to_string(&plugin).unwrap();
        fs::write(&plugin, script.replace("cmd == 'resolve'", "cmd == 'old'")).unwrap();
        failure(project.run(&["plugin", "lock"]), "avm plugin install node");
        let out = project.run(&["-p", "install"]);
        assert_that(&out).stderr_contains("avm plugin install node");
        success(out);
    }
}

/// `avm init` and `avm trust`.
mod init_and_trust {
    use super::*;
    use serde_json::json;

    /// `init` writes an empty, well-formed project config.
    ///
    /// 1. Run `avm init` in a directory without config.
    /// 2. It prints "Created .avm.json" and the file has empty `aliases`, `env` and `tools`.
    #[test]
    fn init_creates_an_empty_config() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["init"]))
            .succeeds()
            .stdout_contains("Created .avm.json");
        assert_that(&p.read_json("project/.avm.json"))
            .is_equal_to(json!({"aliases": {}, "env": {}, "tools": {}}));
    }

    /// `init` never overwrites an existing config.
    ///
    /// 1. Write a config with an alias, then run `avm init`.
    /// 2. It fails with ".avm.json already exists" and the file is unchanged.
    #[test]
    fn init_refuses_to_overwrite() {
        let p = Sandbox::new("avm-test");
        let original = r#"{"aliases":{"keep":"echo me"}}"#;
        p.write("project/.avm.json", original);
        failure(p.run(&["init"]), ".avm.json already exists");
        assert_that(&p.read("project/.avm.json")).is_equal_to(original.to_string());
    }

    /// Trusting a directory previews what it will apply and records it.
    ///
    /// 1. Write a project config with an alias and env var, plus a `.env` file.
    /// 2. `avm trust` lists the alias, the env var and the `.env` entries and trusts both files.
    /// 3. `avm trust --list` names both files.
    #[test]
    fn trust_previews_and_records_a_project() {
        let p = Sandbox::new("avm-test");
        p.write(
            "project/.avm.json",
            r#"{"aliases":{"a":"echo A"},"env":{"K":"v"}}"#,
        );
        p.write("project/.env", "DOT=1\n");
        assert_that(&p.run(&["trust"]))
            .succeeds()
            .stdout_contains("alias a → echo A")
            .stdout_contains("env   K=v")
            .stdout_contains("DOT=1")
            .stdout_contains("Trusted ");
        let list = success(p.run(&["trust", "--list"]));
        assert_that(&list).contains("project/.avm.json");
        assert_that(&list).contains("project/.env");
    }

    /// A path argument trusts that directory or file instead of the cwd.
    ///
    /// 1. Create `sub/.avm.json` and run `avm trust sub` from the project root.
    /// 2. Only `sub/.avm.json` is listed as trusted.
    #[test]
    fn trust_accepts_a_path() {
        let p = Sandbox::new("avm-test");
        p.write("project/sub/.avm.json", r#"{"aliases":{"a":"echo A"}}"#);
        success(p.run(&["trust", "sub"]));
        let list = success(p.run(&["trust", "--list"]));
        assert_that(&list).contains("sub/.avm.json");
        assert_that(&list.lines().count()).is_equal_to(1);
    }

    /// `--revoke` removes trust again.
    ///
    /// 1. Trust a project, then run `avm trust --revoke`.
    /// 2. It prints "Revoked trust" and `--list` is empty.
    #[test]
    fn revoke_removes_trust() {
        let p = Sandbox::new("avm-test");
        p.write("project/.avm.json", "{}");
        success(p.run(&["trust"]));
        assert_that(&p.run(&["trust", "--revoke"]))
            .succeeds()
            .stdout_contains("Revoked trust");
        assert_that(&success(p.run(&["trust", "--list"])).trim().is_empty()).is_true();
    }

    /// `--revoke` and `--list` cannot be combined.
    ///
    /// 1. Run `avm trust --revoke --list`.
    /// 2. clap exits 2 reporting the conflict.
    #[test]
    fn revoke_conflicts_with_list() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["trust", "--revoke", "--list"]))
            .exits_with(2)
            .stderr_contains("cannot be used with");
    }

    /// Trusting a path that does not exist fails.
    ///
    /// 1. Run `avm trust /definitely/not/here`.
    /// 2. It fails and the trust list stays empty.
    #[test]
    fn trusting_a_missing_path_fails() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["trust", "/definitely/not/here"])).fails();
        assert_that(&success(p.run(&["trust", "--list"])).trim().is_empty()).is_true();
    }
}

/// `plugin prune` behaviour beyond the safety checks above.
mod prune {
    use super::*;
    use rstest::rstest;

    /// Installs empty tool dirs; returns their paths. `used` marks which count as recently used.
    fn install(p: &Sandbox, versions: &[&str], used: &[&str]) {
        for v in versions {
            let dir = format!("home/.avm/tools/node/{v}");
            p.write(&format!("{dir}/bin/.keep"), "");
            if used.contains(v) {
                p.write(&format!("{dir}/.last_used"), "");
            }
        }
    }

    /// With nothing installed prune has nothing to do.
    ///
    /// 1. Run `avm plugin prune` in an empty sandbox.
    /// 2. It prints "Nothing to prune."
    #[test]
    fn nothing_installed_means_nothing_to_prune() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["plugin", "prune"]))
            .succeeds()
            .stdout_contains("Nothing to prune.");
    }

    /// `--dry-run` lists candidates and deletes nothing.
    ///
    /// 1. Install node 18.0.0 and 20.0.0 (both used); pin 20 globally.
    /// 2. `plugin prune --dry-run` lists 18.0.0 but not 20.0.0 and reports space to reclaim.
    /// 3. Both directories still exist.
    #[test]
    fn dry_run_lists_without_deleting() {
        let p = Sandbox::new("avm-test");
        install(&p, &["18.0.0", "20.0.0"], &["18.0.0", "20.0.0"]);
        p.write("home/.avm.json", r#"{"tools":{"node":"20"}}"#);
        let out = success(p.run(&["plugin", "prune", "--dry-run"]));
        assert_that(&out).contains("node 18.0.0");
        assert_that(&out.contains("node 20.0.0")).is_false();
        assert_that(&out).contains("to reclaim");
        assert_that(&p.path("home/.avm/tools/node/18.0.0")).exists();
        assert_that(&p.path("home/.avm/tools/node/20.0.0")).exists();
    }

    /// `--yes` removes only versions nothing pins.
    ///
    /// 1. Install node 18.0.0 and 20.0.0 (both used); pin 20 globally.
    /// 2. `plugin prune --yes` removes 18.0.0 and keeps 20.0.0.
    #[test]
    fn yes_removes_only_unpinned_versions() {
        let p = Sandbox::new("avm-test");
        install(&p, &["18.0.0", "20.0.0"], &["18.0.0", "20.0.0"]);
        p.write("home/.avm.json", r#"{"tools":{"node":"20"}}"#);
        assert_that(&p.run(&["plugin", "prune", "--yes"]))
            .succeeds()
            .stdout_contains("Removed node 18.0.0");
        assert_that(&p.path("home/.avm/tools/node/18.0.0")).does_not_exist();
        assert_that(&p.path("home/.avm/tools/node/20.0.0")).exists();
    }

    /// Versions avm never saw used are skipped unless asked for.
    ///
    /// 1. Install node 18.0.0 with no recorded use.
    /// 2. Plain `plugin prune --dry-run` says it skipped 1 version and suggests `--include-unrecorded`.
    /// 3. With `--include-unrecorded` it lists 18.0.0 as "no recorded use".
    #[test]
    fn unrecorded_versions_need_an_explicit_flag() {
        let p = Sandbox::new("avm-test");
        install(&p, &["18.0.0"], &[]);
        assert_that(&p.run(&["plugin", "prune", "--dry-run"]))
            .succeeds()
            .stdout_contains("Skipped 1 version(s)")
            .stdout_contains("--include-unrecorded");
        assert_that(&p.run(&["plugin", "prune", "--dry-run", "--include-unrecorded"]))
            .succeeds()
            .stdout_contains("node 18.0.0")
            .stdout_contains("no recorded use");
    }

    /// `--older-than` keeps recently used versions.
    ///
    /// 1. Install node 18.0.0 used just now.
    /// 2. `plugin prune --older-than 90d --dry-run` prints "Nothing to prune."
    #[test]
    fn older_than_keeps_recent_versions() {
        let p = Sandbox::new("avm-test");
        install(&p, &["18.0.0"], &["18.0.0"]);
        assert_that(&p.run(&["plugin", "prune", "--older-than", "90d", "--dry-run"]))
            .succeeds()
            .stdout_contains("Nothing to prune.");
    }

    /// `--older-than` takes a number of days.
    ///
    /// 1. Run `plugin prune --older-than <value>` with a value that is not `<days>d`.
    /// 2. It fails with "--older-than takes days".
    #[rstest]
    #[case::words("xx")]
    #[case::fractional("1.5d")]
    #[case::unit("5weeks")]
    fn older_than_validates_its_value(#[case] value: &str) {
        let p = Sandbox::new("avm-test");
        failure(
            p.run(&["plugin", "prune", "--older-than", value, "--dry-run"]),
            "--older-than takes days",
        );
    }
}
