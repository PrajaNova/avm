use std::{fs, path::PathBuf, process::{Command, Output}, sync::atomic::{AtomicUsize, Ordering}};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("avm-safety-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        Self(root)
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_avm-bin"));
        cmd.args(args).current_dir(self.0.join("project"))
            .env("HOME", self.0.join("home"))
            .env("AVM_HOME", self.0.join("home/.avm"))
            .env("AVM_PLUGIN_DIR", self.0.join("home/.avm/plugins"))
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TRUST_ALL", "0");
        cmd
    }
    fn run(&self, args: &[&str]) -> Output { self.command(args).output().unwrap() }
}
impl Drop for Project {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

#[test]
fn removal_commands_reject_paths_without_deleting_them() {
    let p = Project::new();
    let victim = p.0.join("victim");
    fs::create_dir(&victim).unwrap();
    fs::write(victim.join("keep"), "important").unwrap();
    for name in [victim.to_str().unwrap(), "../victim"] {
        assert!(!p.run(&["plugin", "remove", name]).status.success());
        assert!(!p.run(&["plugin", "update", name]).status.success());
        assert!(!p.run(&["shims", "remove", name]).status.success());
        assert_eq!(fs::read_to_string(victim.join("keep")).unwrap(), "important");
    }
}

#[test]
fn prune_aborts_when_a_tracked_project_cannot_resolve() {
    let p = Project::new();
    let version = p.0.join("home/.avm/tools/node/20.2.0");
    fs::create_dir_all(&version).unwrap();
    fs::write(version.join(".last_used"), "").unwrap();
    fs::write(p.0.join("home/.avm/tracked-configs.json"), serde_json::json!([p.0.join("project")]).to_string()).unwrap();
    fs::write(p.0.join("project/.avm.lock"), "invalid lock").unwrap();
    let out = p.run(&["prune", "--yes"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot safely prune"));
    assert!(version.exists());
    fs::remove_file(p.0.join("project/.avm.lock")).unwrap();
    fs::write(p.0.join("home/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
    assert!(p.run(&["prune", "--yes"]).status.success());
    assert!(version.exists());
}

#[test]
fn concurrent_config_edits_preserve_every_alias() {
    let p = Project::new();
    let mut children = Vec::new();
    for n in 0..24 {
        children.push(p.command(&["add", "-g", &format!("alias{n}"), "echo ok"])
            .stdout(std::process::Stdio::null()).spawn().unwrap());
    }
    for mut child in children { assert!(child.wait().unwrap().success()); }
    let config: serde_json::Value = serde_json::from_slice(&fs::read(p.0.join("home/.avm.json")).unwrap()).unwrap();
    assert_eq!(config["aliases"].as_object().unwrap().len(), 24);
}

#[test]
fn concurrent_project_tracking_and_trust_preserve_every_project() {
    let p = Project::new();
    let mut children = Vec::new();
    for n in 0..12 {
        let dir = p.0.join(format!("project{n}"));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(".avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
        for args in [vec!["trust", dir.to_str().unwrap()], vec!["list"]] {
            children.push(p.command(&args).current_dir(&dir)
                .stdout(std::process::Stdio::null()).spawn().unwrap());
        }
    }
    for mut child in children { assert!(child.wait().unwrap().success()); }
    for (file, object) in [("trusted.json", true), ("tracked-configs.json", false)] {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(p.0.join("home/.avm").join(file)).unwrap()).unwrap();
        let count = if object { value.as_object().unwrap().len() } else { value.as_array().unwrap().len() };
        assert_eq!(count, 12, "{file}");
    }
}

#[test]
fn malformed_configs_are_never_replaced_with_empty_state() {
    let p = Project::new();
    let file = p.0.join("home/.avm.json");
    let raw = br#"{"aliases":{"valuable":"echo saved"}, broken"#;
    fs::write(&file, raw).unwrap();
    for args in [vec!["add", "-g", "new", "echo new"], vec!["prune", "--yes"]] {
        let out = p.run(&args);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("repair the file"));
        assert_eq!(fs::read(&file).unwrap(), raw);
    }
}

#[test]
fn json_pins_resolve_installed_versions_in_both_scopes() {
    let p = Project::new();
    fs::create_dir_all(p.0.join("home/.avm/tools/node/20.2.0/bin")).unwrap();
    fs::create_dir_all(p.0.join("home/.avm/tools/node/18.3.0/bin")).unwrap();
    fs::write(p.0.join("home/.avm.json"), r#"{"tools":{"node":"18"}}"#).unwrap();
    let global = p.run(&["list"]);
    assert!(global.status.success());
    assert!(String::from_utf8_lossy(&global.stdout).contains("node = 18.3.0"));
    fs::write(p.0.join("project/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
    let local = p.run(&["list"]);
    assert!(local.status.success());
    assert!(String::from_utf8_lossy(&local.stdout).contains("node = 20.2.0"));
    let cfg: serde_json::Value = serde_json::from_slice(&fs::read(p.0.join("project/.avm.json")).unwrap()).unwrap();
    assert_eq!(cfg["tools"]["node"], "20");
}

#[cfg(unix)]
#[test]
fn package_script_names_and_arguments_are_literal_data() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let bin = p.0.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(bin.join("npm"), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(bin.join("npm"), fs::Permissions::from_mode(0o755)).unwrap();
    let marker = p.0.join("injected");
    for name in [format!("hello; touch {}; #", marker.display()), "$1 $(touch injected) 'quoted'".into()] {
        fs::write(p.0.join("project/package.json"), serde_json::json!({"scripts":{&name:"echo safe"}}).to_string()).unwrap();
        let out = p.command(&["run", &name, "--", "arg; touch injected"])
            .env("PATH", std::env::join_paths([bin.clone(), PathBuf::from("/usr/bin"), PathBuf::from("/bin")]).unwrap()).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8(out.stdout).unwrap(), format!("run\n{name}\narg; touch injected\n"));
        assert!(!marker.exists() && !p.0.join("project/injected").exists());
    }
}

#[cfg(unix)]
#[test]
fn shell_environment_restores_original_values_when_leaving_projects() {
    let p = Project::new();
    fs::write(p.0.join("home/.avm.json"), serde_json::json!({"aliases":{"check":r#"test "$AVM_TEST_EXISTING" = "original ' value" && test "${AVM_TEST_NEW+x}" != x"#}}).to_string()).unwrap();
    fs::write(p.0.join("project/.avm.json"), r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#).unwrap();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin")).parent().unwrap();
    let script = format!(r#"
export PATH={}:$PATH
eval "$(avm-bin shell-init)"
[ "$AVM_TEST_EXISTING" = project ] && [ "$AVM_TEST_NEW" = temporary ] || exit 10
cd ..
avm list >/dev/null
[ "$AVM_TEST_EXISTING" = "original ' value" ] && [ "${{AVM_TEST_NEW+x}}" != x ] || exit 11
cd project
_avm_apply_env
[ "$AVM_TEST_EXISTING" = project ] && [ "$AVM_TEST_NEW" = temporary ] || exit 12
avm env remove AVM_TEST_NEW >/dev/null
[ "${{AVM_TEST_NEW+x}}" != x ] || exit 13
cd ..
command avm-bin run check || exit 14
printf restored
"#, avm_plugin_api::sh_quote(&bin.to_string_lossy()));
    for shell in ["bash", "zsh"] {
        fs::write(p.0.join("project/.avm.json"), r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#).unwrap();
        let output = Command::new(shell).args(["-c", &script])
            .current_dir(p.0.join("project"))
            .env("HOME", p.0.join("home")).env("AVM_HOME", p.0.join("home/.avm"))
            .env("AVM_TRUST_ALL", "1").env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TEST_EXISTING", "original ' value").env_remove("AVM_TEST_NEW")
            .output();
        if shell == "zsh" && output.as_ref().is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) { continue; }
        let out = output.unwrap();
        assert!(out.status.success(), "{shell}: {} {:?}", String::from_utf8_lossy(&out.stderr), out.status);
        assert_eq!(out.stdout, b"restored");
    }
}

#[test]
fn untrusted_tool_pins_cannot_escape_managed_directories() {
    let p = Project::new();
    for spec in [p.0.to_string_lossy().to_string(), "../payload".into(), "C:\\payload".into()] {
        fs::write(p.0.join("project/.avm.json"), serde_json::json!({"tools":{"node":spec}}).to_string()).unwrap();
        let out = p.run(&["exec-shim", "node", "--", "--version"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("invalid version spec"));
    }
}

#[cfg(unix)]
#[test]
fn managed_package_binaries_can_have_dotted_names() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let bin = p.0.join("home/.avm/tools/node/20.1.0/bin");
    let plugin = p.0.join("home/.avm/plugins/avm-plugin-node/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&plugin).unwrap();
    for (file, script) in [(bin.join("foo.bar"), "#!/bin/sh\necho managed-package\n"),
        (plugin.join("avm-plugin"), "#!/bin/sh\necho '{\"env\":{}}'\n")] {
        fs::write(&file, script).unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(p.0.join("project/.avm.json"), r#"{"tools":{"node":"20.1.0"}}"#).unwrap();
    let out = p.run(&["exec-shim", "foo.bar"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, b"managed-package\n");
}
