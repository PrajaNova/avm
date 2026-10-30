//! Prebuilt sandboxes for the suites that need a realistic project.
use super::plugins::install_plugin;
use crate::common::{read_json, success, Sandbox};
use fs_err as fs;
use serde_json::Value;
use std::ops::Deref;
use std::process::Output;

const EXEC_PLUGIN: &str = include_str!("scripts/exec_plugin.sh");
const LOCK_PLUGIN: &str = include_str!("scripts/lock_plugin.py");

/// Sandbox for `exec` / `run --with`: shell-script `node` and `java` plugins that print
/// `<tool>-<version>` and the environment each tool exports, plus a global pin
/// (`node 16`), a project pin (`node 20`, `SCOPE=local`), a `check` alias and a lock.
pub struct ExecProject(Sandbox);

impl Deref for ExecProject {
    type Target = Sandbox;
    fn deref(&self) -> &Sandbox {
        &self.0
    }
}

impl ExecProject {
    /// Only the fake `node` and `java` plugins: no config, no pins, no lock.
    pub fn bare() -> Self {
        let sandbox = Sandbox::new("avm-exec");
        for tool in ["node", "java"] {
            install_plugin(&sandbox, tool, &EXEC_PLUGIN.replace("TOOL", tool));
        }
        Self(sandbox)
    }

    /// The plugins plus the global/project pins, `check` alias and lock described above.
    pub fn new() -> Self {
        let sandbox = Self::bare().0;
        let root = &sandbox.root;
        fs::write(root.join("home/.avm.json"), r#"{"tools":{"node":"16.0.0"},"env":{"SCOPE":"global"},"aliases":{"x":"echo wrong-alias"}}"#).unwrap();
        fs::write(root.join("project/.avm.json"), r#"{"tools":{"node":"20.0.0"},"env":{"SCOPE":"local","PATH":"/usr/bin:/bin"},"aliases":{"check":"node $1; java; printf '%s' \"$SCOPE\""}}"#).unwrap();
        let lock = serde_json::json!({
            "version": 1,
            "tools": {"node": {
                "requested": "20.0.0", "resolved": "20.0.0",
                "plugin": {"name": "node", "version": "1.0.0", "binaries": {}},
                "assets": {"linux_amd64": {"url": "/tmp/unused-node-archive", "sha256": "0".repeat(64)}}
            }}
        });
        fs::write(
            root.join("project/.avm.lock"),
            serde_json::to_vec(&lock).unwrap(),
        )
        .unwrap();
        Self(sandbox)
    }

    /// Runs `avm-bin`; `trusted` is whether project config may apply (`AVM_TRUST_ALL`).
    pub fn run_as(&self, args: &[&str], trusted: bool) -> Output {
        self.command(args)
            .env("AVM_TRUST_ALL", if trusted { "1" } else { "0" })
            .env_remove("SCOPE")
            .output()
            .unwrap()
    }

    pub fn trusted(&self, args: &[&str]) -> Output {
        self.run_as(args, true)
    }

    /// Same project with node 18.1.0 and java 21.1.0 already installed, for `--no-install` runs.
    pub fn with_installed() -> Self {
        let project = Self::new();
        success(project.trusted(&["exec", "node@18", "java@21", "--", "true"]));
        project
    }
}

/// Sandbox for lock / install: python `node` and `java` plugins that serve the version in
/// `<root>/remote`, "download" `<root>/archive`, and log every install to `<root>/installs`.
/// Global pin `java 21`, project pin `node 20`.
pub struct LockProject(Sandbox);

impl Deref for LockProject {
    type Target = Sandbox;
    fn deref(&self) -> &Sandbox {
        &self.0
    }
}

impl LockProject {
    pub fn new() -> Self {
        let sandbox = Sandbox::new("avm-lock");
        let root = &sandbox.root;
        fs::write(root.join("home/.avm.json"), r#"{"tools":{"java":"21"}}"#).unwrap();
        fs::write(root.join("project/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
        fs::write(root.join("archive"), "verified archive").unwrap();
        fs::write(root.join("remote"), "20.2.0").unwrap();
        for tool in ["node", "java"] {
            install_plugin(&sandbox, tool, &LOCK_PLUGIN.replace("TOOL", tool));
        }
        Self(sandbox)
    }

    /// Project locked cleanly for the host with both tools pinned, plus its parsed lock.
    pub fn locked() -> (Self, Value) {
        let project = Self::new();
        project.write_config(r#"{"tools":{"node":"20","java":"21"}}"#);
        success(project.run(&["plugin", "lock"]));
        let lock = project.lock();
        (project, lock)
    }

    pub fn write_config(&self, config: &str) {
        fs::write(self.path("project/.avm.json"), config).unwrap();
    }

    pub fn lock(&self) -> Value {
        read_json(&self.path("project/.avm.lock"))
    }

    pub fn save_lock(&self, lock: &Value) {
        fs::write(
            self.path("project/.avm.lock"),
            serde_json::to_vec(lock).unwrap(),
        )
        .unwrap();
    }
}

/// The platform key `plugin lock` wrote assets for (this host).
pub fn locked_platform(lock: &Value) -> String {
    lock["tools"]["node"]["assets"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone()
}
