#![cfg(unix)]

use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "avm-lock-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        fs::write(root.join("home/.avm.json"), r#"{"tools":{"java":"21"}}"#).unwrap();
        fs::write(root.join("project/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
        fs::write(root.join("archive"), "verified archive").unwrap();
        fs::write(root.join("remote"), "20.2.0").unwrap();
        for tool in ["node", "java"] {
            let bin = root.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
            fs::create_dir_all(&bin).unwrap();
            let script = r#"#!/usr/bin/env python3
import hashlib, json, os, pathlib, sys
tool = 'TOOL'
root = pathlib.Path(os.environ['HOME']).parent
cmd, *args = sys.argv[1:]
def version():
    return root.joinpath('remote').read_text() if tool == 'node' else 'openjdk-21.1.0+1'
def target(v):
    return root / 'home/.avm/tools' / tool / v / 'bin' / tool
def install(v):
    with root.joinpath('installs').open('a') as f: f.write(tool + ':' + v + '\n')
    path = target(v)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('#!/bin/sh\necho ' + v + '\n')
    path.chmod(0o755)
if cmd == 'manifest':
    print(json.dumps(dict(name=tool, version='1.0.0', api_version=1)))
elif cmd == 'versions':
    print(json.dumps(dict(versions=[dict(version=version(), label=version(), channel=None, is_lts=True, is_security=False)])))
elif cmd == 'resolve':
    req = json.loads(args[0])
    asset = dict(url=str(root / 'archive'), sha256=hashlib.sha256(root.joinpath('archive').read_bytes()).hexdigest())
    print(json.dumps(dict(version=req['version'], assets={p: asset for p in req['platforms']})))
elif cmd == 'is-installed':
    print(json.dumps(dict(installed=target(args[0]).exists())))
elif cmd == 'installed-versions':
    print('{"versions":[]}')
elif cmd == 'executable-path':
    print(json.dumps(dict(path=str(target(args[0])) if target(args[0]).exists() else None)))
elif cmd == 'env-vars':
    print('{"env":{}}')
elif cmd == 'install':
    install(args[0])
elif cmd == 'install-locked':
    asset = json.loads(args[1])
    assert asset['url'] == str(root / 'archive'), 'host must pass the original asset'
    if hashlib.sha256(pathlib.Path(asset['url']).read_bytes()).hexdigest() != asset['sha256']:
        sys.exit('checksum mismatch')
    install(args[0])
else:
    sys.exit(1)
"#.replace("TOOL", tool);
            let path = bin.join("avm-plugin");
            fs::write(&path, script).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        self.run_in(args, "project")
    }

    fn run_in(&self, args: &[&str], directory: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_avm-bin"))
            .args(args)
            .current_dir(self.0.join(directory))
            .env("HOME", self.0.join("home"))
            .env("AVM_HOME", self.0.join("home/.avm"))
            .env("AVM_PLUGIN_DIR", self.0.join("home/.avm/plugins"))
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TRUST_ALL", "0")
            .env_remove("AVM_ENV")
            .output()
            .unwrap()
    }

    fn lock(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("project/.avm.lock")).unwrap()).unwrap()
    }

    fn save_lock(&self, lock: &Value) {
        fs::write(
            self.0.join("project/.avm.lock"),
            serde_json::to_vec(lock).unwrap(),
        )
        .unwrap();
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn failure(output: Output, expected: &str) {
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains(expected), "{err}");
}

#[test]
fn nested_lock_commands_update_the_project_lock() {
    let project = Project::new();
    fs::create_dir_all(project.0.join("project/packages/web")).unwrap();
    // No existing lock: discover the project config rather than the home config.
    success(project.run_in(&["lock"], "project/packages/web"));
    assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.2.0");
    assert!(!project.0.join("project/packages/web/.avm.lock").exists());
    fs::set_permissions(
        project.0.join("project/.avm.lock"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    fs::write(project.0.join("remote"), "20.3.0").unwrap();
    success(project.run_in(&["lock"], "project/packages/web"));
    assert_eq!(
        fs::metadata(project.0.join("project/.avm.lock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.3.0");
    assert!(!project.0.join("project/packages/web/.avm.lock").exists());
    fs::write(project.0.join("project/.avm.lock"), "invalid lock").unwrap();
    success(project.run_in(&["lock"], "project/packages/web"));
    assert_eq!(project.lock()["tools"].as_object().unwrap().len(), 1);
    let parent_lock = project.lock();
    let child = project.0.join("project/packages/web");
    fs::write(child.join(".avm.ci.json"), r#"{"tools":{"node":"20.2.0"}}"#).unwrap();
    success(project.run_in(&["--env", "ci", "lock"], "project/packages/web"));
    let lock: Value = serde_json::from_slice(&fs::read(child.join(".avm.lock")).unwrap()).unwrap();
    assert_eq!(lock["tools"]["node"]["requested"], "20.2.0");
    assert_eq!(project.lock(), parent_lock);
    success(project.run_in(
        &["--env", "ci", "-p", "install", "--frozen"],
        "project/packages/web",
    ));
}

#[test]
fn locks_installs_and_rejects_drift_before_installing() {
    let project = Project::new();
    // A teammate's home pins must not enter or invalidate this project's lock.
    success(project.run(&["lock"]));
    let local_lock = fs::read(project.0.join("project/.avm.lock")).unwrap();
    assert_eq!(project.lock()["tools"].as_object().unwrap().len(), 1);
    assert!(project.lock()["tools"].get("java").is_none());
    fs::write(
        project.0.join("home/.avm.json"),
        r#"{"tools":{"java":"17","node":"99"}}"#,
    )
    .unwrap();
    success(project.run(&["-p", "install", "--frozen"]));
    assert!(success(project.run(&["which", "node"])).contains("20.2.0"));
    assert!(!project.0.join("home/.avm/tools/java").exists());
    success(project.run(&["node", "use", "22.0.0", "--global"]));
    success(project.run(&["-p", "install", "--frozen"]));
    assert!(success(project.run(&["which", "node"])).contains("20.2.0"));
    fs::write(project.0.join("home/.avm.json"), "{}").unwrap();
    success(project.run(&["-p", "install", "--frozen"]));
    assert_eq!(
        fs::read(project.0.join("project/.avm.lock")).unwrap(),
        local_lock
    );
    fs::remove_dir_all(project.0.join("home/.avm/tools")).unwrap();
    fs::remove_file(project.0.join("installs")).unwrap();
    fs::remove_file(project.0.join("project/.avm.lock")).unwrap();
    fs::write(
        project.0.join("home/.avm.json"),
        r#"{"tools":{"java":"21"}}"#,
    )
    .unwrap();
    fs::write(
        project.0.join("project/.avm.json"),
        r#"{"tools":{"node":"20","java":"21"}}"#,
    )
    .unwrap();
    let config = fs::read(project.0.join("project/.avm.json")).unwrap();
    let global = fs::read(project.0.join("home/.avm.json")).unwrap();
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "requires .avm.lock",
    );
    success(project.run(&[
        "lock",
        "--platform",
        "linux_amd64,darwin_arm64,windows_amd64",
    ]));
    assert!(!project.0.join("installs").exists());
    let first = fs::read(project.0.join("project/.avm.lock")).unwrap();
    success(project.run(&[
        "lock",
        "--platform",
        "windows_amd64,linux_amd64,darwin_arm64",
    ]));
    assert_eq!(
        first,
        fs::read(project.0.join("project/.avm.lock")).unwrap()
    );
    let lock = project.lock();
    assert_eq!(lock["tools"]["node"]["requested"], "20");
    assert_eq!(lock["tools"]["node"]["resolved"], "20.2.0");
    assert_eq!(
        lock["tools"]["node"]["assets"].as_object().unwrap().len(),
        3
    );
    fs::write(project.0.join("remote"), "20.3.0").unwrap();
    assert!(success(project.run(&["which", "node"])).contains("20.2.0"));
    fs::create_dir(project.0.join("project/child")).unwrap();
    fs::write(project.0.join("project/.nvmrc"), "20\n").unwrap();
    fs::write(project.0.join("project/.tool-versions"), "java 21\n").unwrap();
    assert!(success(project.run_in(&["which", "node"], "project/child")).contains("20.2.0"));
    success(project.run(&["-p", "install", "--frozen"]));
    let installs = fs::read(project.0.join("installs")).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&installs),
        "java:openjdk-21.1.0+1\nnode:20.2.0\n"
    );
    success(project.run(&["-p", "install"]));
    assert_eq!(fs::read(project.0.join("installs")).unwrap(), installs);
    assert_eq!(
        fs::read(project.0.join("project/.avm.json")).unwrap(),
        config
    );
    assert_eq!(fs::read(project.0.join("home/.avm.json")).unwrap(), global);

    let plugin = project
        .0
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
            avm_plugin_api::download::host_platform().unwrap()
        )),
        "{warning}"
    );
    project.save_lock(&lock);

    fs::write(
        project.0.join("project/.avm.json"),
        r#"{"tools":{"node":"21"}}"#,
    )
    .unwrap();
    failure(project.run(&["-p", "install", "--frozen"]), "out of date");
    let env = project.run(&["env"]);
    assert!(String::from_utf8_lossy(&env.stderr).contains("out of date"));
    success(env);
    assert_eq!(fs::read(project.0.join("installs")).unwrap(), installs);
    fs::write(project.0.join("project/.avm.json"), config).unwrap();

    let mut bad_hash = lock.clone();
    let platform = bad_hash["tools"]["node"]["assets"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    bad_hash["tools"]["node"]["assets"][&platform]["sha256"] = json!("not-a-hash");
    project.save_lock(&bad_hash);
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "invalid sha256",
    );

    let mut invalid = lock.clone();
    invalid["tools"]["java"]["requested"] = json!("openjdk-21.1.0+1");
    invalid["tools"]["java"]["resolved"] = json!("openjdk-21.1.0+2");
    project.save_lock(&invalid);
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "does not satisfy",
    );
    invalid = lock.clone();
    invalid["tools"]["node"]["resolved"] = json!("22.0.0");
    project.save_lock(&invalid);
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "does not satisfy",
    );
    invalid["tools"]["node"]["resolved"] = json!("../escape");
    project.save_lock(&invalid);
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "invalid resolved version",
    );
    invalid = lock.clone();
    invalid["version"] = json!(99);
    project.save_lock(&invalid);
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "unsupported .avm.lock",
    );
    project.save_lock(&lock);

    fs::remove_dir_all(project.0.join("home/.avm/tools/node")).unwrap();
    fs::write(project.0.join("archive"), "tampered").unwrap();
    failure(
        project.run(&["-p", "install", "--frozen"]),
        "checksum mismatch",
    );
    failure(
        project.run(&["node", "install", "20", "--no-pin"]),
        "checksum mismatch",
    );
    failure(project.run(&["node", "use", "20.2.0"]), "checksum mismatch");
    assert_eq!(fs::read(project.0.join("installs")).unwrap(), installs);
    assert!(!project.0.join("home/.avm/tools/node/20.2.0").exists());

    // Temporary overrides resolve independently of the project's locked pin.
    assert_eq!(
        success(project.run(&["exec", "node@20", "--", "node"])),
        "20.3.0\n"
    );
    assert_eq!(project.lock(), lock);

    // Regeneration recovers from a stale lock and keeps all writes atomic.
    success(project.run(&["lock"]));
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
    success(project.run(&["lock", "--platform", other]));
    failure(project.run(&["-p", "install", "--frozen"]), "has no");

    let old = fs::read(project.0.join("project/.avm.lock")).unwrap();
    failure(
        project.run(&["lock", "--platform", "linux_x64"]),
        "unsupported lock platform",
    );
    assert_eq!(old, fs::read(project.0.join("project/.avm.lock")).unwrap());
    fs::remove_file(project.0.join("project/.avm.lock")).unwrap();
    success(project.run(&["-p", "install"]));
    assert!(project.0.join("project/.avm.lock").exists());
    fs::remove_file(project.0.join("project/.avm.lock")).unwrap();
    assert!(fs::read_to_string(project.0.join("installs"))
        .unwrap()
        .contains("node:20.3.0"));

    let legacy = project.0.join("home/.avm/plugins/asdf-ruby/bin");
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
        fs::set_permissions(legacy.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(
        project.0.join("project/.avm.json"),
        r#"{"tools":{"node":"20","ruby":"latest"}}"#,
    )
    .unwrap();
    success(project.run(&["-p", "install"]));
    assert!(project
        .0
        .join("home/.avm/tools/ruby/12.0.0/bin/ruby")
        .exists());
    failure(project.run(&["lock"]), "requires a native plugin");
    assert!(!project.0.join("project/.avm.lock").exists());
}

fn write_project(project: &Project, config: &str) {
    fs::write(project.0.join("project/.avm.json"), config).unwrap();
}

#[test]
fn install_writes_the_lock_when_missing() {
    let project = Project::new();
    success(project.run(&["-p", "install"]));
    assert_eq!(project.lock()["tools"]["node"]["resolved"], "20.2.0");
}

#[test]
fn install_in_a_project_without_tools_says_so() {
    let project = Project::new();
    write_project(&project, "{}");
    let out = success(project.run(&["-p", "install"]));
    assert!(out.contains("No tools in .avm.json"), "{out}");
    assert!(out.contains("java") && out.contains("(global)"), "{out}");
    assert!(!project.0.join("project/.avm.lock").exists());
    let again = success(project.run(&["-p", "install"]));
    assert!(again.contains("already installed"), "{again}");
}

#[test]
fn upgrade_without_pins_says_so() {
    let project = Project::new();
    write_project(&project, "{}");
    fs::write(project.0.join("home/.avm.json"), "{}").unwrap();
    assert!(success(project.run(&["upgrade"])).contains("No tools pinned here."));
}

#[test]
fn lock_and_frozen_install_without_tools_are_explicit() {
    let project = Project::new();
    write_project(&project, "{}");
    assert!(success(project.run(&["lock"])).contains("Nothing to lock"));
    assert!(!project.0.join("project/.avm.lock").exists());
    assert!(success(project.run(&["-p", "install", "--frozen"])).contains("Nothing to install"));
}

#[test]
fn a_stale_lock_warns_but_does_not_break_commands() {
    let project = Project::new();
    success(project.run(&["lock"]));
    write_project(&project, r#"{"tools":{"node":"21"}}"#);
    let which = project.run(&["which", "node"]);
    assert!(String::from_utf8_lossy(&which.stderr).contains("out of date"));
    success(which);
    success(project.run(&["env"]));
    failure(project.run(&["-p", "install", "--frozen"]), "out of date");
}

#[test]
fn changing_a_pin_refreshes_an_existing_lock() {
    let project = Project::new();
    success(project.run(&["lock"]));
    success(project.run(&["node", "use", "20.2.0"]));
    let out = project.run(&["env"]);
    assert!(!String::from_utf8_lossy(&out.stderr).contains("out of date"));
    assert_eq!(project.lock()["tools"]["node"]["requested"], "20.2.0");
}

#[test]
fn old_plugins_are_told_how_to_update() {
    let project = Project::new();
    let plugin = project
        .0
        .join("home/.avm/plugins/avm-plugin-node/bin/avm-plugin");
    let script = fs::read_to_string(&plugin).unwrap();
    fs::write(&plugin, script.replace("cmd == 'resolve'", "cmd == 'old'")).unwrap();
    failure(project.run(&["lock"]), "avm plugin install node");
    let out = project.run(&["-p", "install"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("avm plugin install node"));
    success(out);
}
