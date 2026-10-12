#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("avm-exec-test-{}", std::process::id()));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        for tool in ["node", "java"] {
            let plugin = root.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
            fs::create_dir_all(&plugin).unwrap();
            let script = r#"#!/bin/sh
tool=TOOL
dir="$HOME/.avm/tools/$tool/$2"
case "$1" in
  is-installed) if [ -f "$dir/bin/$tool" ]; then echo '{"installed":true}'; else echo '{"installed":false}'; fi ;;
  installed-versions) echo '{"versions":["18.1.0","21.1.0"]}' ;;
  versions)
    case "$3" in
      major:18) version=18.1.0 ;; major:21) version=21.1.0 ;; *) version=22.1.0 ;;
    esac
    echo "{\"versions\":[{\"version\":\"$version\",\"label\":\"$version\",\"is_lts\":true,\"channel\":\"Iron\",\"is_security\":false}]}" ;;
  install)
    mkdir -p "$dir/bin"
    printf '#!/bin/sh\nprintf "%%s\\n" "%s-%s" "$@"\n' "$tool" "$2" > "$dir/bin/$tool"
    chmod +x "$dir/bin/$tool" ;;
  executable-path) echo "{\"path\":\"$dir/bin/$tool\"}" ;;
  env-vars)
    case "$tool" in java) key=JAVA_HOME ;; *) key=NODE_HOME ;; esac
    echo "{\"env\":{\"$key\":\"$dir\"}}" ;;
  *) exit 1 ;;
esac
"#.replace("TOOL", tool);
            let file = plugin.join("avm-plugin");
            fs::write(&file, script).unwrap();
            fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
        }
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
        Self(root)
    }

    fn run(&self, args: &[&str], trusted: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_avm-bin"));
        command
            .args(args)
            .current_dir(self.0.join("project"))
            .env("HOME", self.0.join("home"))
            .env("AVM_HOME", self.0.join("home/.avm"))
            .env("AVM_PLUGIN_DIR", self.0.join("home/.avm/plugins"))
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TRUST_ALL", if trusted { "1" } else { "0" })
            .env_remove("SCOPE");
        command.output().unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn temporary_versions_install_execute_and_leave_pins_unchanged() {
    let project = Project::new();
    let files = ["project/.avm.json", "project/.avm.lock", "home/.avm.json"];
    let before: Vec<_> = files
        .iter()
        .map(|p| fs::read(project.0.join(p)).unwrap())
        .collect();

    let missing = project.run(&["exec", "--no-install", "node@18", "--", "node"], true);
    assert!(!missing.status.success());
    assert!(!project.0.join("home/.avm/tools/node/18.1.0").exists());

    let out = stdout(project.run(
        &[
            "exec",
            "node@18",
            "java@21",
            "--",
            "sh",
            "-c",
            "node --help 'two words'; java; printf '%s\\n' \"$SCOPE\" \"$JAVA_HOME\"",
        ],
        true,
    ));
    assert!(
        out.starts_with("node-18.1.0\n--help\ntwo words\njava-21.1.0\nlocal\n"),
        "{out}"
    );
    assert!(out.contains("/tools/java/21.1.0"));
    assert_eq!(
        stdout(project.run(&["x", "--no-install", "node@18", "--", "node"], true)),
        "node-18.1.0\n"
    );
    assert_eq!(
        stdout(project.run(
            &[
                "run",
                "check",
                "--with",
                "node@18",
                "--with",
                "java@21",
                "--no-install",
                "--",
                "two words"
            ],
            true
        )),
        "node-18.1.0\ntwo words\njava-21.1.0\nlocal"
    );
    assert_eq!(
        stdout(project.run(
            &[
                "exec",
                "node@18.1.0",
                "--",
                "sh",
                "-c",
                "printf '%s' \"$SCOPE\""
            ],
            false
        )),
        "global"
    );
    assert!(!project
        .run(&["run", "check", "--with", "node@18"], false)
        .status
        .success());
    assert_eq!(
        project
            .run(&["exec", "node@18.1.0", "--", "sh", "-c", "exit 37"], true)
            .status
            .code(),
        Some(37)
    );
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        project
            .run(
                &["exec", "node@18.1.0", "--", "sh", "-c", "kill -TERM $$"],
                true
            )
            .status
            .signal(),
        Some(15)
    );
    assert!(!project
        .run(&["exec", "node@18.1.0", "--", "avm-no-such-command"], true)
        .status
        .success());
    assert_eq!(
        stdout(project.run(&["exec", "node@lts", "--", "node"], true)),
        "node-22.1.0\n"
    );
    assert_eq!(
        stdout(project.run(&["exec", "java@latest", "--", "java"], true)),
        "java-22.1.0\n"
    );
    let binary_dir = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
        .parent()
        .unwrap();
    let script = format!("export PATH='{}':$PATH; eval \"$(avm-bin shell-init)\"; avm x --no-install node@18 -- node", binary_dir.display());
    assert_eq!(
        stdout(project.run(&["exec", "node@18.1.0", "--", "bash", "-c", &script], false)),
        "node-18.1.0\n"
    );
    for (file, original) in files.iter().zip(before) {
        assert_eq!(
            fs::read(project.0.join(file)).unwrap(),
            original,
            "{file} changed"
        );
    }
}
