//! Plugins: `avm plugin list/uninstall`, version resolution, and native plugin locking.
//! Source feature: `crates/avm-cli/src/plugins/`.

use crate::common::*;
use crate::fixtures::native::{archive, Disco};
#[cfg(unix)]
use crate::fixtures::plugins::install_plugin;
use fs_err as fs;
use serde_json::{json, Value};
use speculoos::prelude::*;
use std::{path::Path, process::Command};

/// `plugin uninstall` and `shims remove` take names, not paths, so they can never delete arbitrary directories.
///
/// 1. Create a directory with a file outside avm's home.
/// 2. Pass its absolute path and `../victim` to both commands.
/// 3. Both commands fail every time and the file is untouched.
#[test]
fn removal_commands_reject_paths_without_deleting_them() {
    let p = Sandbox::new("avm-test");
    let victim = p.root.join("victim");
    fs::create_dir(&victim).unwrap();
    fs::write(victim.join("keep"), "important").unwrap();
    for name in [victim.to_str().unwrap(), "../victim"] {
        assert_that(&p.run(&["plugin", "uninstall", name])).fails();
        assert_that(&p.run(&["shims", "remove", name])).fails();
        assert_eq!(
            fs::read_to_string(victim.join("keep")).unwrap(),
            "important"
        );
    }
}

/// A partial pin (`18`, `20`) resolves to the installed matching version, per scope, without rewriting the config.
///
/// 1. Install node 18.3.0 and 20.2.0 on disk; pin `18` globally.
/// 2. `plugin list` shows node 18.3.0 as global.
/// 3. Pin `20` in the project: the list shows 20.2.0 as local.
/// 4. The project config still says `20`.
#[test]
fn json_pins_resolve_installed_versions_in_both_scopes() {
    let p = Sandbox::new("avm-test");
    fs::create_dir_all(p.root.join("home/.avm/tools/node/20.2.0/bin")).unwrap();
    fs::create_dir_all(p.root.join("home/.avm/tools/node/18.3.0/bin")).unwrap();
    fs::write(p.root.join("home/.avm.json"), r#"{"tools":{"node":"18"}}"#).unwrap();
    let global = p.run(&["plugin", "list"]);
    assert_that(&global).succeeds();
    let text = String::from_utf8_lossy(&global.stdout);
    assert!(text.contains("node") && text.contains("18.3.0") && text.contains("global"));
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"tools":{"node":"20"}}"#,
    )
    .unwrap();
    let local = p.run(&["plugin", "list"]);
    assert_that(&local).succeeds();
    let text = String::from_utf8_lossy(&local.stdout);
    assert!(text.contains("node") && text.contains("20.2.0") && text.contains("local"));
    let cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(p.root.join("project/.avm.json")).unwrap()).unwrap();
    assert_eq!(cfg["tools"]["node"], "20");
}

/// `plugin list` sorts versions numerically, marks the active one and shows paths relative to the project.
///
/// 1. Install node 9.1.0, 10.2.0 and 18.3.0; pin `10` globally.
/// 2. With no project config the list says "Project: none here", names `~/.avm.json` and prints `9.1.0  10.2.0*  18.3.0`.
/// 3. Pin an uninstalled `99` in the project: it shows as local from `./.avm.json`, "(not installed)", with no absolute sandbox path.
#[cfg(unix)]
#[test]
fn list_shows_scope_active_and_numerically_sorted_installs() {
    let p = Sandbox::new("avm-test");
    install_plugin(&p, "node", "#!/bin/sh\ncase \"$1\" in\nmanifest) echo '{\"name\":\"node\",\"version\":\"1.0.0\",\"api_version\":1}';;\ninstalled-versions) echo '{\"versions\":[\"10.2.0\",\"18.3.0\",\"9.1.0\"]}';;\n*) exit 1;;\nesac\n");
    for v in ["9.1.0", "10.2.0", "18.3.0"] {
        fs::create_dir_all(p.root.join("home/.avm/tools/node").join(v).join("bin")).unwrap();
    }
    fs::write(p.root.join("home/.avm.json"), r#"{"tools":{"node":"10"}}"#).unwrap();
    let out = success(p.run(&["plugin", "list"]));
    assert_that(&out).contains("Project: none here");
    assert_that(&out).contains("~/.avm.json");
    assert_that(&out).contains("9.1.0  10.2.0*  18.3.0");
    fs::write(
        p.root.join("project/.avm.json"),
        r#"{"tools":{"node":"99"}}"#,
    )
    .unwrap();
    let out = success(p.run(&["plugin", "list"]));
    assert!(
        out.contains("local") && out.contains("./.avm.json"),
        "{out}"
    );
    assert_that(&out).contains("(not installed)");
    assert!(!out.contains(p.root.to_str().unwrap()), "{out}");
}

/// `plugin list` starts each plugin process once, however many facts it needs.
///
/// 1. Install two plugins that log every call.
/// 2. Run `avm plugin list`; each tool appears with its installed version.
/// 3. The call log has exactly two lines, one per plugin.
#[cfg(unix)]
#[test]
fn list_starts_each_plugin_once() {
    let p = Sandbox::new("avm-test");
    let calls = p.root.join("calls");
    for tool in ["node", "java"] {
        let script = format!("#!/bin/sh\necho \"{tool} $1\" >> '{}'\nif [ \"$1\" = installed-versions ]; then echo '{{\"versions\":[\"1.0.0\"]}}'; fi\n", calls.display());
        install_plugin(&p, tool, &script);
    }
    let out = success(p.run(&["plugin", "list"]));
    for tool in ["node", "java"] {
        assert!(
            out.lines()
                .any(|l| l.starts_with(tool) && l.ends_with("1.0.0")),
            "{out}"
        );
    }
    let log = fs::read_to_string(calls).unwrap();
    assert_eq!(log.lines().count(), 2, "one start per plugin: {log}");
}

/// Native (Rust) plugins can lock and then install fully offline, and reject tampering.
///
/// 1. Build a fake node mirror and a mock Java discovery server; install the real native plugins.
/// 2. `install --frozen` fails without a lock; `plugin lock` records node 20.1.2 and the java asset checksum.
/// 3. Stop the servers and delete the mirror: `install --frozen` and shim execution still work from the lock.
/// 4. A wrong plugin binary hash, then a tampered java archive, both make frozen install fail without installing.
#[test]
fn native_plugins_lock_and_install_offline_on_this_platform() {
    let root = std::env::temp_dir().join(format!("avm-native-lock-{}", std::process::id()));
    let bin = Path::new(env!("CARGO_BIN_EXE_avm-bin")).parent().unwrap();
    let ext = std::env::consts::EXE_SUFFIX;
    let platform = avm_plugin_sdk::download::host_platform().unwrap();
    let node_platform = platform
        .replace("windows", "win")
        .replace("_amd64", "-x64")
        .replace("_arm64", "-arm64");
    let suffix = if cfg!(windows) { "zip" } else { "tar.gz" };
    let node_name = format!("node-v20.1.2-{node_platform}");
    let release = root.join("mirror/v20.1.2");
    fs::create_dir_all(&release).unwrap();
    fs::create_dir_all(root.join("project")).unwrap();
    for tool in ["node", "java"] {
        let native = bin.join(format!("avm-plugin-{tool}{ext}"));
        assert!(
            native.exists(),
            "build the workspace plugins before running this integration test"
        );
        let plugin = root.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
        fs::create_dir_all(&plugin).unwrap();
        fs::copy(&native, plugin.join(format!("avm-plugin{ext}"))).unwrap();
        let path = if tool == "node" {
            if cfg!(windows) {
                root.join(&node_name)
            } else {
                root.join(&node_name).join("bin")
            }
        } else {
            root.join("jdk/bin")
        };
        fs::create_dir_all(&path).unwrap();
        if tool == "node" && cfg!(windows) {
            fs::create_dir_all(path.join("node_modules/npm")).unwrap();
        }
        if cfg!(windows) {
            fs::copy(native, path.join(format!("{tool}.exe"))).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file = path.join(tool);
            fs::write(
                &file,
                format!("#!/bin/sh\nprintf '%s\\n' '{{\"name\":\"{tool}\"}}'\n"),
            )
            .unwrap();
            fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let node_archive = release.join(format!("{node_name}.{suffix}"));
    let java_archive = root.join(format!("jdk.{suffix}"));
    archive(&root, &node_name, &node_archive);
    archive(&root, "jdk", &java_archive);
    let node_hash = avm_plugin_sdk::sha256_file(&node_archive).unwrap();
    let java_hash = avm_plugin_sdk::sha256_file(&java_archive).unwrap();
    fs::write(
        release.join("SHASUMS256.txt"),
        format!("{node_hash}  {node_name}.{suffix}\n"),
    )
    .unwrap();
    fs::write(
        root.join("mirror/index.json"),
        json!([{"version":"v20.1.2","lts":"Iron"}]).to_string(),
    )
    .unwrap();
    fs::write(
        root.join("project/.avm.json"),
        r#"{"tools":{"node":"20.1","java":"21"}}"#,
    )
    .unwrap();
    let disco = Disco::new(
        json!({"java_version":"21.0.1+12","distribution_version":"21.0.1+12","major_version":21,"term_of_support":"lts","links":{"pkg_download_redirect":java_archive}}),
        &java_hash,
    );
    let disco_url = disco.url.clone();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_avm-bin"))
            .args(args)
            .current_dir(root.join("project"))
            .env("HOME", root.join("home"))
            .env("AVM_HOME", root.join("home/.avm"))
            .env("AVM_PLUGIN_DIR", root.join("home/.avm/plugins"))
            .env("AVM_NODE_DIST_URL", root.join("mirror"))
            .env("AVM_JAVA_DISCO_URL", &disco_url)
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env_remove("_AVM_ENV_BASELINE")
            .output()
            .unwrap()
    };
    assert!(!run(&["-p", "install", "--frozen"]).status.success());
    success(run(&["plugin", "lock"]));
    let lock_file = root.join("project/.avm.lock");
    let lock_bytes = fs::read(&lock_file).unwrap();
    let lock: Value = serde_json::from_slice(&lock_bytes).unwrap();
    assert_eq!(lock["tools"]["node"]["resolved"], "20.1.2");
    assert_eq!(
        lock["tools"]["java"]["assets"][&platform]["sha256"],
        java_hash
    );
    drop(disco);
    fs::remove_file(root.join("mirror/index.json")).unwrap();
    fs::remove_file(release.join("SHASUMS256.txt")).unwrap();
    success(run(&["-p", "install", "--frozen"]));
    for tool in ["node", "java"] {
        let out = success(run(&["exec-shim", tool, "--", "manifest"]));
        let manifest: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(manifest["name"], tool);
    }
    success(run(&[
        "exec",
        "--no-install",
        "node@20.1.2",
        "--",
        "node",
        "manifest",
    ]));
    assert_eq!(fs::read(&lock_file).unwrap(), lock_bytes);
    fs::remove_dir_all(root.join("home/.avm/tools")).unwrap();
    let mut wrong = lock.clone();
    wrong["tools"]["node"]["plugin"]["binaries"][&platform] = json!("0".repeat(64));
    fs::write(&lock_file, wrong.to_string()).unwrap();
    assert!(!run(&["-p", "install", "--frozen"]).status.success());
    assert_that(&root.join("home/.avm/tools/java/openjdk-21.0.1+12")).does_not_exist();
    fs::write(&lock_file, &lock_bytes).unwrap();
    fs::write(&java_archive, "tampered ZIP or tarball").unwrap();
    assert!(!run(&["-p", "install", "--frozen"]).status.success());
    assert_that(&root.join("home/.avm/tools/java/openjdk-21.0.1+12")).does_not_exist();
    fs::remove_dir_all(root).unwrap();
}

/// `plugin list` / `uninstall` / `install` argument handling.
mod commands {
    use super::*;

    /// With nothing installed `plugin list` explains how to get started.
    ///
    /// 1. Run `avm plugin list` in an empty sandbox.
    /// 2. It says "No plugins or tools yet" and shows the install commands.
    #[test]
    fn list_in_an_empty_sandbox_shows_getting_started() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["plugin", "list"]))
            .succeeds()
            .stdout_contains("No plugins or tools yet")
            .stdout_contains("avm plugin install <plugin>");
    }

    /// `ls` is an alias of `list`.
    ///
    /// 1. Run `avm plugin ls`.
    /// 2. The output matches `avm plugin list`.
    #[test]
    fn ls_is_an_alias_of_list() {
        let p = Sandbox::new("avm-test");
        assert_that(&success(p.run(&["plugin", "ls"])))
            .is_equal_to(success(p.run(&["plugin", "list"])));
    }

    /// `uninstall` removes the plugin's directory, one or several at a time.
    ///
    /// 1. Create plugin directories `avm-plugin-foo` and `avm-plugin-bar`.
    /// 2. `plugin uninstall foo bar` reports both removed and both directories are gone.
    #[test]
    fn uninstall_removes_each_named_plugin() {
        let p = Sandbox::new("avm-test");
        for name in ["foo", "bar"] {
            p.write(
                &format!("home/.avm/plugins/avm-plugin-{name}/bin/avm-plugin"),
                "#!/bin/sh\n",
            );
        }
        assert_that(&p.run(&["plugin", "uninstall", "foo", "bar"]))
            .succeeds()
            .stdout_contains("Plugin 'foo' removed.")
            .stdout_contains("Plugin 'bar' removed.");
        assert_that(&p.path("home/.avm/plugins/avm-plugin-foo")).does_not_exist();
        assert_that(&p.path("home/.avm/plugins/avm-plugin-bar")).does_not_exist();
    }

    /// `install --frozen` takes no plugin names.
    ///
    /// 1. Run `avm plugin install node --frozen`.
    /// 2. clap exits 2 reporting the conflict.
    #[test]
    fn frozen_install_conflicts_with_plugin_names() {
        let p = Sandbox::new("avm-test");
        assert_that(&p.run(&["plugin", "install", "node", "--frozen"]))
            .exits_with(2)
            .stderr_contains("cannot be used with");
    }

    /// Without names, `install` installs the project's tools, and says so when there are none.
    ///
    /// 1. In a project with an empty config run `plugin install` and `plugin install --frozen`.
    /// 2. They say "No tools in .avm.json" and "Nothing to install".
    #[test]
    fn install_without_names_and_without_tools_explains_itself() {
        let p = Sandbox::new("avm-test");
        p.write("project/.avm.json", "{}");
        assert_that(&p.run(&["plugin", "install"]))
            .succeeds()
            .stdout_contains("No tools in .avm.json");
        assert_that(&p.run(&["plugin", "install", "--frozen"]))
            .succeeds()
            .stdout_contains("Nothing to install");
    }
}
