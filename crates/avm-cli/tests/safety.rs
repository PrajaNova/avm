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
