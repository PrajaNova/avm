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
fn untrusted_tool_pins_cannot_escape_managed_directories() {
    let p = Project::new();
    for spec in [p.0.to_string_lossy().to_string(), "../payload".into(), "C:\\payload".into()] {
        fs::write(p.0.join("project/.avm.json"), serde_json::json!({"tools":{"node":spec}}).to_string()).unwrap();
        let out = p.run(&["exec-shim", "node", "--", "--version"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("invalid version spec"));
    }
}
