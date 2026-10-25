use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "avm-safety-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("project")).unwrap();
        Self(root)
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_avm-bin"));
        cmd.args(args)
            .current_dir(self.0.join("project"))
            .env("HOME", self.0.join("home"))
            .env("AVM_HOME", self.0.join("home/.avm"))
            .env("AVM_PLUGIN_DIR", self.0.join("home/.avm/plugins"))
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TRUST_ALL", "0")
            .env_remove("AVM_ENV");
        cmd
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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
        assert_eq!(
            fs::read_to_string(victim.join("keep")).unwrap(),
            "important"
        );
    }
}

#[test]
fn prune_aborts_when_a_tracked_project_cannot_resolve() {
    let p = Project::new();
    let version = p.0.join("home/.avm/tools/node/20.2.0");
    fs::create_dir_all(&version).unwrap();
    fs::write(version.join(".last_used"), "").unwrap();
    fs::write(
        p.0.join("home/.avm/tracked-configs.json"),
        serde_json::json!([p.0.join("project")]).to_string(),
    )
    .unwrap();
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
        children.push(
            p.command(&["add", "-g", &format!("alias{n}"), "echo ok"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(p.0.join("home/.avm.json")).unwrap()).unwrap();
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
            serde_json::from_slice(&fs::read(p.0.join("home/.avm").join(file)).unwrap()).unwrap();
        let count = if object {
            value.as_object().unwrap().len()
        } else {
            value.as_array().unwrap().len()
        };
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
    let text = String::from_utf8_lossy(&global.stdout);
    assert!(text.contains("node") && text.contains("18.3.0") && text.contains("global"));
    fs::write(p.0.join("project/.avm.json"), r#"{"tools":{"node":"20"}}"#).unwrap();
    let local = p.run(&["list"]);
    assert!(local.status.success());
    let text = String::from_utf8_lossy(&local.stdout);
    assert!(text.contains("node") && text.contains("20.2.0") && text.contains("local"));
    let cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(p.0.join("project/.avm.json")).unwrap()).unwrap();
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
    for name in [
        format!("hello; touch {}; #", marker.display()),
        "$1 $(touch injected) 'quoted'".into(),
    ] {
        fs::write(
            p.0.join("project/package.json"),
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
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!("run\n{name}\narg; touch injected\n")
        );
        assert!(!marker.exists() && !p.0.join("project/injected").exists());
    }
}

#[cfg(unix)]
#[test]
fn shell_environment_restores_original_values_when_leaving_projects() {
    let p = Project::new();
    fs::write(p.0.join("home/.avm.json"), serde_json::json!({"aliases":{"check":r#"test "$AVM_TEST_EXISTING" = "original ' value" && test "${AVM_TEST_NEW+x}" != x"#}}).to_string()).unwrap();
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#,
    )
    .unwrap();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
        .parent()
        .unwrap();
    let script = format!(
        r#"
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
"#,
        avm_plugin_api::sh_quote(&bin.to_string_lossy())
    );
    for shell in ["bash", "zsh"] {
        fs::write(
            p.0.join("project/.avm.json"),
            r#"{"env":{"AVM_TEST_EXISTING":"project","AVM_TEST_NEW":"temporary"}}"#,
        )
        .unwrap();
        let output = Command::new(shell)
            .args(["-c", &script])
            .current_dir(p.0.join("project"))
            .env("HOME", p.0.join("home"))
            .env("AVM_HOME", p.0.join("home/.avm"))
            .env("AVM_TRUST_ALL", "1")
            .env("AVM_NO_UPDATE_CHECK", "1")
            .env("AVM_TEST_EXISTING", "original ' value")
            .env_remove("AVM_TEST_NEW")
            .output();
        if shell == "zsh"
            && output
                .as_ref()
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        let out = output.unwrap();
        assert!(
            out.status.success(),
            "{shell}: {} {:?}",
            String::from_utf8_lossy(&out.stderr),
            out.status
        );
        assert_eq!(out.stdout, b"restored");
    }
}

#[test]
fn untrusted_tool_pins_cannot_escape_managed_directories() {
    let p = Project::new();
    for spec in [
        p.0.to_string_lossy().to_string(),
        "../payload".into(),
        "C:\\payload".into(),
    ] {
        fs::write(
            p.0.join("project/.avm.json"),
            serde_json::json!({"tools":{"node":spec}}).to_string(),
        )
        .unwrap();
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
    for (file, script) in [
        (bin.join("foo.bar"), "#!/bin/sh\necho managed-package\n"),
        (
            plugin.join("avm-plugin"),
            "#!/bin/sh\necho '{\"env\":{}}'\n",
        ),
    ] {
        fs::write(&file, script).unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"tools":{"node":"20.1.0"}}"#,
    )
    .unwrap();
    let out = p.run(&["exec-shim", "foo.bar"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"managed-package\n");
}

fn stdout(out: Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn hierarchical_profiles_merge_and_report_origins() {
    let p = Project::new();
    let repo = p.0.join("project");
    let nested = repo.join("apps/tv");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        p.0.join("home/.avm.json"),
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
        stdout(
            p.command(args)
                .current_dir(&nested)
                .env("AVM_TRUST_ALL", "1")
                .env("AVM_ENV", "ci,staging")
                .output()
                .unwrap(),
        )
    };
    let list = run(&["list"]);
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
    assert!(String::from_utf8_lossy(&suggestion.stderr).contains("avm parent"));
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
    let list = run(&["--env", "", "list"]);
    assert!(!list.contains("parent →"));
    assert!(list.contains("global →"));
    fs::write(repo.join(".avm.json"), "malformed above root").unwrap();
    assert!(run(&["--env", "", "env", "--shell", "sh"]).contains("GLOBAL"));
}

#[test]
fn trust_and_cache_invalidation_apply_to_each_layer() {
    let p = Project::new();
    let repo = p.0.join("project");
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
        stdout(
            p.command(args)
                .current_dir(&child)
                .env("AVM_ENV", "ci")
                .output()
                .unwrap(),
        )
    };
    stdout(p.run(&["trust"]));
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
        .set_times(fs::FileTimes::new().set_modified(old_mtime))
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
    for entry in fs::read_dir(p.0.join("home/.avm/cache")).unwrap() {
        fs::write(entry.unwrap().path(), "invalid cache").unwrap();
    }
    assert!(run(&["env", "--shell", "sh"]).contains("MODE=parent"));
}

#[test]
fn init_gitignores_personal_config_and_preserves_root_on_edit() {
    let p = Project::new();
    fs::write(p.0.join("project/.gitignore"), "target").unwrap();
    stdout(p.run(&["init"]));
    assert_eq!(
        fs::read_to_string(p.0.join("project/.gitignore")).unwrap(),
        "target\n.avm.local.json\n"
    );
    fs::write(p.0.join("project/.avm.json"), r#"{"root":true}"#).unwrap();
    stdout(p.run(&["add", "build", "echo build"]));
    let cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(p.0.join("project/.avm.json")).unwrap()).unwrap();
    assert_eq!(cfg["root"], true);
    for profile in ["../secret", "ci,,staging", "prod/path", ".", "ci\\prod"] {
        let out = p.run(&["env", "--env", profile]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("invalid environment profile"));
    }
}

#[test]
#[ignore = "benchmark: run with cargo test -p avm-cli --release cached_env_prompt_benchmark -- --ignored --nocapture"]
fn cached_env_prompt_benchmark() {
    let p = Project::new();
    let mut dir = p.0.join("project");
    for depth in 0..6 {
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".avm.json"), serde_json::json!({"root":depth == 0,"env":{format!("LAYER_{depth}"):depth.to_string()}}).to_string()).unwrap();
        stdout(p.run(&["trust", dir.to_str().unwrap()]));
        dir = dir.join("child");
    }
    fs::create_dir_all(&dir).unwrap();
    let run = || {
        let start = std::time::Instant::now();
        stdout(p.command(&["env"]).current_dir(&dir).output().unwrap());
        start.elapsed().as_secs_f64() * 1000.0
    };
    for _ in 0..10 {
        run();
    }
    let mut ms: Vec<_> = (0..100).map(|_| run()).collect();
    ms.sort_by(f64::total_cmp);
    println!(
        "cached avm env, six config layers, 100 subprocesses: median {:.2} ms, p95 {:.2} ms",
        ms[50], ms[95]
    );
    assert!(
        ms[50] < 5.0,
        "median prompt overhead {:.2} ms exceeds 5 ms",
        ms[50]
    );
}

#[test]
fn prune_keeps_inactive_profile_pins() {
    let p = Project::new();
    let home = p.0.join("home/.avm");
    for version in ["20.1.0", "22.1.0", "24.1.0"] {
        fs::create_dir_all(home.join("tools/node").join(version)).unwrap();
    }
    let repo = p.0.join("project");
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
    let out = stdout(p.run(&["prune", "--dry-run", "--include-unrecorded"]));
    assert!(out.contains("node 24.1.0"));
    assert!(!out.contains("node 20.1.0") && !out.contains("node 22.1.0"));
    fs::write(repo.join(".avm.ci.json"), "broken profile").unwrap();
    assert!(!p
        .run(&["prune", "--yes", "--include-unrecorded"])
        .status
        .success());
    assert!(home.join("tools/node/24.1.0").exists());
}

#[cfg(unix)]
#[test]
fn shell_wrapper_forwards_profile_options() {
    let p = Project::new();
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"root":true,"aliases":{"build":"echo base"}}"#,
    )
    .unwrap();
    fs::write(
        p.0.join("project/.avm.ci.json"),
        r#"{"aliases":{"build":"echo ci"}}"#,
    )
    .unwrap();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_avm-bin"))
        .parent()
        .unwrap();
    let script = format!(
        r#"
export PATH={}:$PATH
eval "$(avm-bin shell-init)"
case "$(avm --env ci which build)" in *"echo ci"*) ;; *) exit 10 ;; esac
case "$(avm --env=ci which build)" in *"echo ci"*) ;; *) exit 11 ;; esac
[ "$(avm --env ci run build)" = ci ] || exit 12
[ "$(avm run build)" = base ] || exit 13
"#,
        avm_plugin_api::sh_quote(&bin.to_string_lossy())
    );
    for shell in ["bash", "zsh"] {
        let mut settings = p.command(&[]);
        settings.env("AVM_TRUST_ALL", "1");
        let mut cmd = Command::new(shell);
        cmd.args(["-c", &script])
            .current_dir(settings.get_current_dir().unwrap())
            .envs(settings.get_envs().filter_map(|(k, v)| v.map(|v| (k, v))));
        cmd.env_remove("AVM_ENV");
        let output = cmd.output();
        if shell == "zsh"
            && output
                .as_ref()
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        stdout(output.unwrap());
    }
}

#[cfg(unix)]
#[test]
fn upgrade_keeps_inherited_pins_at_their_source() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let plugin = p.0.join("home/.avm/plugins/avm-plugin-node/bin");
    fs::create_dir_all(&plugin).unwrap();
    let executable = plugin.join("avm-plugin");
    fs::write(&executable, "#!/bin/sh\necho '{\"versions\":[{\"version\":\"22.1.0\",\"label\":\"22\",\"is_lts\":true,\"is_security\":false},{\"version\":\"20.2.0\",\"label\":\"20\",\"is_lts\":true,\"is_security\":false}]}'\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"root":true,"tools":{"node":"20.1.0"}}"#,
    )
    .unwrap();
    let local = stdout(p.run(&["upgrade", "--bump", "--dry-run"]));
    assert!(local.contains("20.1.0 → 22.1.0 (local pin)"), "{local}");
    let child = p.0.join("project/child");
    fs::create_dir(&child).unwrap();
    let inherited = stdout(
        p.command(&["upgrade", "--bump", "--dry-run"])
            .current_dir(&child)
            .output()
            .unwrap(),
    );
    assert!(
        inherited.contains("edit it to move to 22.1.0"),
        "{inherited}"
    );
    assert!(!child.join(".avm.json").exists());
}

#[test]
fn shims_activate_appends_without_rewriting_non_utf8_rc_files() {
    let p = Project::new();
    let bashrc = p.0.join("home/.bashrc");
    let original = b"# caf\xe9 latin-1 comment\nalias ll='ls -l'\n".to_vec();
    fs::write(&bashrc, &original).unwrap();
    assert!(p.run(&["shims", "activate"]).status.success());
    let after = fs::read(&bashrc).unwrap();
    assert!(
        after.starts_with(&original),
        "existing bytes must be preserved"
    );
    assert!(String::from_utf8_lossy(&after).contains("# >>> avm shims >>>"));
    assert!(p.run(&["shims", "activate"]).status.success());
    assert_eq!(
        fs::read(&bashrc).unwrap(),
        after,
        "second run must not append again"
    );
}

#[cfg(unix)]
#[test]
fn shims_install_leaves_unchanged_shims_untouched() {
    use std::os::unix::fs::MetadataExt;
    let p = Project::new();
    assert!(p.run(&["shims", "install"]).status.success());
    let node = p.0.join("home/.avm/shims/node");
    let inode = fs::metadata(&node).unwrap().ino();
    assert!(p.run(&["shims", "install"]).status.success());
    assert_eq!(fs::metadata(&node).unwrap().ino(), inode);
}

#[test]
fn broken_package_json_never_blocks_env_or_shims() {
    let p = Project::new();
    fs::write(p.0.join("project/package.json"), "{ not json").unwrap();
    assert!(p.run(&["env"]).status.success());
    assert!(!String::from_utf8_lossy(
        &p.run(&["exec-shim", "definitely-not-a-command", "--"])
            .stderr
    )
    .contains("package.json"));
    // Alias commands warn and carry on instead of failing.
    let out = p.run(&["list"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ignoring package.json"));
}

#[test]
fn package_json_with_bom_still_provides_script_aliases() {
    let p = Project::new();
    fs::write(
        p.0.join("project/package.json"),
        b"\xef\xbb\xbf{\"scripts\":{\"dev\":\"echo hi\"}}",
    )
    .unwrap();
    let out = p.run(&["resolve", "dev"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A protocol plugin whose `env-vars` call is counted in `<root>/spawns`.
#[cfg(unix)]
fn fake_env_plugins(p: &Project, tools: &[&str]) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let counter = p.0.join("spawns");
    for tool in tools {
        let bin = p.0.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
        fs::create_dir_all(&bin).unwrap();
        let script = format!("#!/bin/sh\nif [ \"$1\" = env-vars ]; then echo x >> '{}'; echo '{{\"env\":{{\"{}_HOME\":\"/opt/{tool}\"}}}}'; fi\n", counter.display(), tool.to_uppercase());
        fs::write(bin.join("avm-plugin"), script).unwrap();
        fs::set_permissions(bin.join("avm-plugin"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let pins: serde_json::Map<_, _> = tools
        .iter()
        .map(|t| (t.to_string(), serde_json::json!("1")))
        .collect();
    fs::write(
        p.0.join("project/.avm.json"),
        serde_json::json!({"tools": pins}).to_string(),
    )
    .unwrap();
    counter
}

#[cfg(unix)]
#[test]
fn plugin_env_vars_are_cached_after_the_first_prompt() {
    let p = Project::new();
    let counter = fake_env_plugins(&p, &["node", "java", "android"]);
    for _ in 0..3 {
        let out = stdout(p.run(&["env"]));
        assert!(
            out.contains("JAVA_HOME") && out.contains("ANDROID_HOME"),
            "{out}"
        );
    }
    assert_eq!(
        fs::read_to_string(counter).unwrap().lines().count(),
        3,
        "one plugin spawn per tool, ever"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "benchmark: run with cargo test -p avm-cli --release pinned_tools_prompt_benchmark -- --ignored --nocapture"]
fn pinned_tools_prompt_benchmark() {
    let p = Project::new();
    fake_env_plugins(&p, &["node", "java", "android"]);
    let run = || {
        let start = std::time::Instant::now();
        stdout(p.run(&["env"]));
        start.elapsed().as_secs_f64() * 1000.0
    };
    for _ in 0..10 {
        run();
    }
    let mut ms: Vec<_> = (0..100).map(|_| run()).collect();
    ms.sort_by(f64::total_cmp);
    println!(
        "avm env, 3 pinned tools, 100 subprocesses: median {:.2} ms, p95 {:.2} ms",
        ms[50], ms[95]
    );
    assert!(
        ms[50] < 10.0,
        "median prompt overhead {:.2} ms exceeds 10 ms",
        ms[50]
    );
}

#[test]
fn bare_avm_without_a_terminal_prints_help_instead_of_failing() {
    let p = Project::new();
    let out = p.run(&[]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage: avm"));
}

#[cfg(unix)]
#[test]
fn list_shows_scope_active_and_numerically_sorted_installs() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let plugin = p.0.join("home/.avm/plugins/avm-plugin-node/bin/avm-plugin");
    fs::create_dir_all(plugin.parent().unwrap()).unwrap();
    fs::write(
        &plugin,
        "#!/bin/sh\ncase \"$1\" in\nmanifest) echo '{\"name\":\"node\",\"version\":\"1.0.0\",\"api_version\":1}';;\ninstalled-versions) echo '{\"versions\":[\"10.2.0\",\"18.3.0\",\"9.1.0\"]}';;\n*) exit 1;;\nesac\n",
    )
    .unwrap();
    fs::set_permissions(&plugin, fs::Permissions::from_mode(0o755)).unwrap();
    for v in ["9.1.0", "10.2.0", "18.3.0"] {
        fs::create_dir_all(p.0.join("home/.avm/tools/node").join(v).join("bin")).unwrap();
    }
    fs::write(p.0.join("home/.avm.json"), r#"{"tools":{"node":"10"}}"#).unwrap();
    let out = stdout(p.run(&["list"]));
    assert!(out.contains("Project: none here"), "{out}");
    assert!(out.contains("~/.avm.json"), "{out}");
    assert!(out.contains("9.1.0  10.2.0*  18.3.0"), "{out}");
    fs::write(p.0.join("project/.avm.json"), r#"{"tools":{"node":"99"}}"#).unwrap();
    let out = stdout(p.run(&["list"]));
    assert!(
        out.contains("local") && out.contains("./.avm.json"),
        "{out}"
    );
    assert!(out.contains("(not installed)"), "{out}");
    assert!(!out.contains(p.0.to_str().unwrap()), "{out}");
}

#[test]
fn prune_removes_only_stale_config_cache_files() {
    let p = Project::new();
    let cache = p.0.join("home/.avm/cache");
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
    fs::write(p.0.join("home/.avm.json"), "{}").unwrap();
    assert!(p.run(&["prune", "--yes"]).status.success());
    assert!(!old.exists() && fresh.exists());
}

#[test]
fn pinned_tool_without_plugin_makes_no_network_call() {
    let p = Project::new();
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"tools":{"ruby":"3.2.0"}}"#,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/registry.json", listener.local_addr().unwrap());
    let start = std::time::Instant::now();
    p.command(&["env"])
        .env("AVM_MARKETPLACE_URL", url)
        .output()
        .unwrap();
    assert!(listener.accept().is_err(), "marketplace was contacted");
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn list_starts_each_plugin_once() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let calls = p.0.join("calls");
    for tool in ["node", "java"] {
        let bin = p.0.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
        fs::create_dir_all(&bin).unwrap();
        let script = format!("#!/bin/sh\necho \"{tool} $1\" >> '{}'\nif [ \"$1\" = installed-versions ]; then echo '{{\"versions\":[\"1.0.0\"]}}'; fi\n", calls.display());
        fs::write(bin.join("avm-plugin"), script).unwrap();
        fs::set_permissions(bin.join("avm-plugin"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = stdout(p.run(&["list"]));
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

#[cfg(unix)]
#[test]
fn outdated_lists_versions_for_all_tools_concurrently() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    for tool in ["node", "java"] {
        let bin = p.0.join(format!("home/.avm/plugins/avm-plugin-{tool}/bin"));
        fs::create_dir_all(&bin).unwrap();
        let script = "#!/bin/sh\nif [ \"$1\" = versions ]; then sleep 2; echo '{\"versions\":[{\"version\":\"2.0.0\",\"label\":\"2.0.0\",\"is_lts\":false,\"channel\":null,\"is_security\":false}]}'; fi\n";
        fs::write(bin.join("avm-plugin"), script).unwrap();
        fs::set_permissions(bin.join("avm-plugin"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(
        p.0.join("project/.avm.json"),
        r#"{"tools":{"node":"1.0.0","java":"1.0.0"}}"#,
    )
    .unwrap();
    let start = std::time::Instant::now();
    let out = p
        .command(&["outdated"])
        .env("AVM_TRUST_ALL", "1")
        .output()
        .unwrap();
    let text = stdout(out);
    assert!(
        text.contains("node") && text.contains("java") && text.contains("2.0.0"),
        "{text}"
    );
    assert!(
        start.elapsed() < std::time::Duration::from_millis(3500),
        "{:?}",
        start.elapsed()
    );
}
