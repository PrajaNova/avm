use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn avm_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_avm-bin"))
}

fn temp_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("avm-{name}-{}-{suffix}", std::process::id()));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, contents).expect("write test file");
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .expect("executable metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("chmod executable");
    }
}

fn run_avm(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(avm_bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("AVM_PLUGIN_DIR", home.join(".avm").join("plugins"))
        .output()
        .expect("run avm-bin")
}

fn run_avm_with_env(cwd: &Path, home: &Path, args: &[&str], envs: &[(&str, &Path)]) -> Output {
    let mut command = Command::new(avm_bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env("AVM_PLUGIN_DIR", home.join(".avm").join("plugins"));
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run avm-bin")
}

fn node_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "x86_64") => "linux-x64",
        other => panic!("unsupported test platform: {other:?}"),
    }
}

fn create_node_archive(dist: &Path, version: &str) {
    let platform = node_platform();
    let release_dir = dist.join(format!("v{version}"));
    let package = release_dir.join(format!("node-v{version}-{platform}"));
    let bin = package.join("bin");
    fs::create_dir_all(&bin).expect("create fake node bin");
    write_file(&bin.join("node"), "#!/usr/bin/env sh\necho fake-node\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(bin.join("node"))
            .expect("fake node metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(bin.join("node"), perms).expect("chmod fake node");
    }
    let archive = format!("node-v{version}-{platform}.tar.gz");
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg(format!("node-v{version}-{platform}"))
        .current_dir(&release_dir)
        .status()
        .expect("create fake node archive");
    assert!(status.success(), "tar fake node archive");
}

fn create_asdf_java_plugin(root: &Path) -> PathBuf {
    let plugin = root.join("asdf-java");
    let bin = plugin.join("bin");
    fs::create_dir_all(&bin).expect("create asdf plugin bin");
    write_file(
        &bin.join("list-all"),
        "#!/usr/bin/env sh\nprintf 'temurin-22.0.0+1 temurin-21.0.1+1\\n'\n",
    );
    write_file(
        &bin.join("install"),
        r#"#!/usr/bin/env sh
set -eu
mkdir -p "$ASDF_INSTALL_PATH/bin"
cat > "$ASDF_INSTALL_PATH/bin/java" <<'EOF'
#!/usr/bin/env sh
echo asdf-java-runtime
EOF
chmod +x "$ASDF_INSTALL_PATH/bin/java"
"#,
    );
    write_file(
        &bin.join("uninstall"),
        "#!/usr/bin/env sh\nrm -rf \"$ASDF_INSTALL_PATH\"\n",
    );
    make_executable(&bin.join("list-all"));
    make_executable(&bin.join("install"));
    make_executable(&bin.join("uninstall"));
    plugin
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

#[test]
fn resolves_and_runs_local_aliases() {
    let root = temp_root("basic-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {
    "dev": "echo local-dev:$1",
    "check": "echo check:$1:$2"
  },
  "env": {
    "AVM_TEST_ENV": "local"
  },
  "tools": {}
}"#,
    );

    let output = run_avm(&work, &home, &["resolve", "dev", "web"]);
    assert_success(&output);
    assert_eq!(stdout(&output).trim(), "'echo' 'local-dev:web'");

    let output = run_avm(&work, &home, &["run", "dev", "web"]);
    assert_success(&output);
    assert!(stdout(&output).contains("local-dev:web"));

    let output = run_avm(&work, &home, &["which", "dev"]);
    assert_success(&output);
    assert!(stdout(&output).contains("local alias 'dev': echo local-dev:$1"));

    let output = run_avm(&work, &home, &["env"]);
    assert_success(&output);
    assert!(stdout(&output).contains("export AVM_TEST_ENV='local'"));
}

#[test]
fn local_config_overrides_global_config() {
    let root = temp_root("precedence");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &home.join(".avm.json"),
        r#"{
  "aliases": {
    "build": "echo global-build",
    "deploy": "echo global-deploy"
  },
  "env": {
    "SCOPE": "global",
    "SHARED": "yes"
  },
  "tools": {
    "node": "20.11.1"
  }
}"#,
    );
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {
    "build": "echo local-build"
  },
  "env": {
    "SCOPE": "local"
  },
  "tools": {}
}"#,
    );

    let output = run_avm(&work, &home, &["which", "build"]);
    assert_success(&output);
    assert!(stdout(&output).contains("local alias 'build': echo local-build"));

    let output = run_avm(&work, &home, &["which", "deploy"]);
    assert_success(&output);
    assert!(stdout(&output).contains("global alias 'deploy': echo global-deploy"));

    let output = run_avm(&work, &home, &["env"]);
    assert_success(&output);
    assert!(stdout(&output).contains("export SCOPE='local'"));
    assert!(stdout(&output).contains("export SHARED='yes'"));
}

#[test]
fn node_provider_exposes_package_scripts_with_lockfile_manager() {
    let root = temp_root("node-provider");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{},"env":{},"tools":{}}"#,
    );
    write_file(
        &work.join("package.json"),
        r#"{
  "name": "avm-node-provider-test",
  "scripts": {
    "start": "vite --host 0.0.0.0",
    "build": "vite build"
  }
}"#,
    );
    write_file(&work.join("pnpm-lock.yaml"), "");

    let output = run_avm(&work, &home, &["which", "start"]);
    assert_success(&output);
    assert!(stdout(&output).contains("plugin alias 'start' from node"));
    assert!(stdout(&output).contains("pnpm run start"));

    let output = run_avm(&work, &home, &["which", "build"]);
    assert_success(&output);
    assert!(stdout(&output).contains("plugin alias 'build' from node"));
}

#[test]
fn fuzzy_alias_suggestion_matches_reordered_words() {
    let root = temp_root("fuzzy-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {
    "tv:run": "echo tv-run"
  },
  "env": {},
  "tools": {}
}"#,
    );

    let output = run_avm(&work, &home, &["run", "runtv"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("alias 'runtv' not found"));
    assert!(stderr(&output).contains("Did you mean?"));
    assert!(stderr(&output).contains("avm tv:run"));
}

#[test]
fn plugin_first_node_command_sets_and_lists_versions() {
    let root = temp_root("plugin-first-node");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{},"env":{},"tools":{}}"#,
    );
    write_file(
        &dist.join("index.json"),
        r#"[
  {"version":"v21.1.0","lts":false,"security":false},
  {"version":"v20.11.1","lts":"Iron","security":true},
  {"version":"v19.9.0","lts":false,"security":false}
]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "use", "20.11.1"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    assert!(stdout(&output).contains("Installing node 20.11.1"));
    assert!(stdout(&output).contains("✓ Installed node 20.11.1"));
    assert!(stdout(&output).contains("✓ Set local node version to 20.11.1"));
    assert!(home.join(".avm/tools/node/20.11.1/bin/node").exists());
    assert!(home.join(".avm/shims/node").exists());

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "latest", "versions"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    assert!(stdout(&output).contains("Available node versions:"));
    assert!(stdout(&output).contains("21.1.0"));
    assert!(!stdout(&output).contains("20.11.1"));
}

#[test]
fn dotenv_file_supplies_node_dist_url() {
    let root = temp_root("dotenv-dist");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &work.join(".env"),
        &format!("AVM_NODE_DIST_URL={}\n", dist.display()),
    );
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{},"env":{},"tools":{}}"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm(&work, &home, &["node", "use", "20.11.1"]);
    assert_success(&output);
    assert!(stdout(&output).contains("Installing node 20.11.1"));
    assert!(stdout(&output).contains("✓ Installed node 20.11.1"));
}

#[test]
fn asdf_java_plugin_can_be_installed_and_used_as_provider() {
    let root = temp_root("asdf-java-provider");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{},"env":{},"tools":{}}"#,
    );
    let plugin = create_asdf_java_plugin(&root);

    let output = run_avm(&work, &home, &["plugin", "add", plugin.to_str().unwrap()]);
    assert_success(&output);
    assert!(stdout(&output).contains("✓ Installed plugin"));

    let output = run_avm(&work, &home, &["java", "latest", "versions"]);
    assert_success(&output);
    assert!(stdout(&output).contains("Available java versions:"));
    assert!(stdout(&output).contains("temurin-22.0.0+1"));
    assert!(!stdout(&output).contains("temurin-21.0.1+1"));

    let output = run_avm(&work, &home, &["java", "use", "temurin-21.0.1+1"]);
    assert_success(&output);
    assert!(stdout(&output).contains("Installing java temurin-21.0.1+1"));
    assert!(stdout(&output).contains("✓ Installed java temurin-21.0.1+1"));
    assert!(stdout(&output).contains("✓ Set local java version to temurin-21.0.1+1"));
    assert!(home
        .join(".avm/tools/java/temurin-21.0.1+1/bin/java")
        .exists());

    let output = run_avm(&work, &home, &["shims", "install"]);
    assert_success(&output);
    let shim_dir = home.join(".avm").join("shims");
    let shim = shim_dir.join("java");
    let avm_dir = avm_bin().parent().expect("avm bin parent").to_path_buf();
    let path = std::env::join_paths([
        shim_dir.as_path(),
        avm_dir.as_path(),
        Path::new("/usr/bin"),
        Path::new("/bin"),
    ])
    .expect("join path");
    let output = Command::new(shim)
        .current_dir(&work)
        .env("HOME", &home)
        .env("PATH", path)
        .output()
        .expect("run java shim");

    assert_success(&output);
    assert!(stdout(&output).contains("asdf-java-runtime"));
}

#[test]
fn shim_falls_back_to_system_binary_when_managed_node_is_missing() {
    let root = temp_root("shim-fallback");
    let home = root.join("home");
    let work = root.join("work");
    let fake_bin = root.join("fake-bin");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&fake_bin).expect("create fake bin");
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {},
  "env": {},
  "tools": {
    "node": "99.9.9-missing"
  }
}"#,
    );
    write_file(
        &fake_bin.join("node"),
        "#!/usr/bin/env sh\nprintf 'system-node:%s\\n' \"$*\"\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(fake_bin.join("node"))
            .expect("fake node metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(fake_bin.join("node"), perms).expect("chmod fake node");
    }

    let output = run_avm(&work, &home, &["shims", "install"]);
    assert_success(&output);

    let shim_dir = home.join(".avm").join("shims");
    let shim = shim_dir.join("node");
    let avm_dir = avm_bin().parent().expect("avm bin parent").to_path_buf();
    let system_path = Path::new("/usr/bin");
    let system_bin = Path::new("/bin");
    let path = std::env::join_paths([
        fake_bin.as_path(),
        shim_dir.as_path(),
        avm_dir.as_path(),
        system_path,
        system_bin,
    ])
    .expect("join path");
    let output = Command::new(shim)
        .arg("-v")
        .current_dir(&work)
        .env("HOME", &home)
        .env("PATH", path)
        .output()
        .expect("run node shim");

    assert_success(&output);
    assert!(stderr(&output).contains(
        "warning: managed node 99.9.9-missing is not installed; falling back to system node"
    ));
    assert!(stdout(&output).contains("system-node:-v"));
}

#[test]
fn alias_with_chained_command_runs_both_sides() {
    let root = temp_root("chained-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {
    "chain": "echo first && echo second"
  },
  "env": {},
  "tools": {}
}"#,
    );

    let output = run_avm(&work, &home, &["run", "chain"]);
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("first"), "missing first half: {out}");
    assert!(out.contains("second"), "missing second half: {out}");
}

#[test]
fn alias_with_pipe_runs_through_shell() {
    let root = temp_root("piped-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{
  "aliases": {
    "p": "echo hello | tr a-z A-Z"
  },
  "env": {},
  "tools": {}
}"#,
    );

    let output = run_avm(&work, &home, &["run", "p"]);
    assert_success(&output);
    assert!(stdout(&output).contains("HELLO"));
}

#[test]
fn install_auto_pins_local_and_global_when_no_global() {
    let root = temp_root("install-auto-pin");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &dist.join("index.json"),
        r#"[{"version":"v20.11.1","lts":"Iron","security":false}]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("✓ Installed node 20.11.1"), "got: {out}");
    assert!(out.contains("local + global"), "expected dual pin: {out}");

    let local = fs::read_to_string(work.join(".avm.json")).expect("read local");
    assert!(local.contains("\"node\""), "local pin missing: {local}");
    assert!(local.contains("20.11.1"));
    let global = fs::read_to_string(home.join(".avm.json")).expect("read global");
    assert!(global.contains("\"node\""), "global pin missing: {global}");
    assert!(global.contains("20.11.1"));
}

#[test]
fn install_keeps_existing_global_pin() {
    let root = temp_root("install-keep-global");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    // Pre-existing global pin for node.
    write_file(
        &home.join(".avm.json"),
        r#"{"aliases":{},"env":{},"tools":{"node":"18.0.0"}}"#,
    );
    write_file(
        &dist.join("index.json"),
        r#"[{"version":"v20.11.1","lts":"Iron","security":false}]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    let out = stdout(&output);
    assert!(
        out.contains("(local pin set)"),
        "expected local-only: {out}"
    );

    let global = fs::read_to_string(home.join(".avm.json")).expect("read global");
    assert!(
        global.contains("18.0.0"),
        "global pin overwritten: {global}"
    );
    let local = fs::read_to_string(work.join(".avm.json")).expect("read local");
    assert!(local.contains("20.11.1"));
}

#[test]
fn corrupt_local_config_is_backed_up_and_recovered() {
    let root = temp_root("corrupt-config");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(&work.join(".avm.json"), "{this is not json");

    // Any avm command should now succeed; the corrupt file gets backed up.
    let output = run_avm(&work, &home, &["which", "anything"]);
    assert_success(&output);
    assert!(stderr(&output).contains("was malformed"));

    // The original file should be gone and replaced by a .broken-*.json backup.
    let entries: Vec<_> = fs::read_dir(&work)
        .expect("read work")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        entries.iter().any(|n| n.starts_with(".avm.broken-")),
        "no backup file in: {entries:?}"
    );
}

// ---------------------------------------------------------------------------
// Deeper coverage for shell-mode alias execution
// ---------------------------------------------------------------------------

#[test]
fn alias_with_semicolon_runs_through_shell() {
    let root = temp_root("semicolon-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"two":"echo one; echo two"},"env":{},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "two"]);
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("one") && out.contains("two"), "got: {out}");
}

#[test]
fn alias_with_env_var_expands_via_shell() {
    let root = temp_root("envvar-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"greet":"echo hello $WHO"},"env":{"WHO":"world"},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "greet"]);
    assert_success(&output);
    assert!(stdout(&output).contains("hello world"));
}

#[test]
fn alias_with_command_substitution_runs_through_shell() {
    let root = temp_root("subshell-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"now":"echo got=$(echo inside)"},"env":{},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "now"]);
    assert_success(&output);
    assert!(stdout(&output).contains("got=inside"));
}

#[test]
fn alias_positional_placeholder_in_shell_mode() {
    // `$1` is replaced by AVM *before* the shell sees it (so the user can
    // pass arguments safely). Shell semantics still apply for `&&`.
    let root = temp_root("placeholder-shell");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"p":"echo start && echo got=$1"},"env":{},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "p", "value with spaces"]);
    assert_success(&output);
    let out = stdout(&output);
    assert!(out.contains("start"), "missing first half: {out}");
    assert!(out.contains("got=value with spaces"), "got: {out}");
}

#[test]
fn alias_exit_code_propagates_in_shell_mode() {
    let root = temp_root("exit-code-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"ok":"false || true","bad":"true && false"},"env":{},"tools":{}}"#,
    );

    let ok = run_avm(&work, &home, &["run", "ok"]);
    assert_eq!(ok.status.code(), Some(0));

    let bad = run_avm(&work, &home, &["run", "bad"]);
    assert_eq!(bad.status.code(), Some(1));
}

#[test]
fn alias_quoted_metacharacters_stay_in_direct_mode() {
    // `;` inside double quotes is literal; needs_shell() must not promote.
    // We can't directly observe direct vs shell mode, but we can ensure the
    // literal semicolon survives intact in the output.
    let root = temp_root("quoted-meta-alias");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"q":"printf %s \"a;b\""},"env":{},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "q"]);
    assert_success(&output);
    assert_eq!(stdout(&output).trim(), "a;b");
}

#[test]
fn alias_extra_args_are_shell_quoted_when_no_placeholder() {
    // Extra positional args passed in shell mode must be safely quoted so
    // metacharacters in arg values don't reinterpret the alias body.
    let root = temp_root("safe-quoting");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(
        &work.join(".avm.json"),
        r#"{"aliases":{"echoer":"echo a && echo"},"env":{},"tools":{}}"#,
    );
    let output = run_avm(&work, &home, &["run", "echoer", "rm -rf /tmp/x; echo BAD"]);
    assert_success(&output);
    let out = stdout(&output);
    // The arg must round-trip as a single literal line, not be re-parsed by sh.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines,
        vec!["a", "rm -rf /tmp/x; echo BAD"],
        "shell parse leak: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Deeper coverage for install pinning flags
// ---------------------------------------------------------------------------

#[test]
fn install_global_flag_pins_only_globally() {
    let root = temp_root("install-global-only");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &dist.join("index.json"),
        r#"[{"version":"v20.11.1","lts":"Iron","security":false}]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1", "--global"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    assert!(stdout(&output).contains("global pin set"));

    let global = fs::read_to_string(home.join(".avm.json")).expect("read global");
    assert!(global.contains("20.11.1"), "global pin missing: {global}");
    assert!(
        !work.join(".avm.json").exists(),
        "should not have created local config"
    );
}

#[test]
fn install_no_pin_flag_skips_pinning() {
    let root = temp_root("install-no-pin");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &dist.join("index.json"),
        r#"[{"version":"v20.11.1","lts":"Iron","security":false}]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1", "--no-pin"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    assert!(
        !work.join(".avm.json").exists(),
        "local config created despite --no-pin"
    );
    assert!(
        !home.join(".avm.json").exists(),
        "global config created despite --no-pin"
    );
}

#[test]
fn install_latest_resolves_and_pins() {
    let root = temp_root("install-latest");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    // Index has multiple versions; "latest" should pick the newest stable.
    write_file(
        &dist.join("index.json"),
        r#"[
  {"version":"v21.1.0","lts":false,"security":false},
  {"version":"v20.11.1","lts":"Iron","security":false}
]"#,
    );
    create_node_archive(&dist, "21.1.0");

    let output = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "latest"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&output);
    let out = stdout(&output);
    assert!(
        out.contains("Resolved latest"),
        "missing resolve banner: {out}"
    );
    assert!(out.contains("21.1.0"), "wrong version installed: {out}");
    let global = fs::read_to_string(home.join(".avm.json")).expect("read global");
    assert!(global.contains("21.1.0"));
}

#[test]
fn install_existing_version_still_updates_pin() {
    // Reinstalling the same version (already on disk) should still update
    // the local pin. Catches a regression where short-circuiting "already
    // installed" skips pinning too.
    let root = temp_root("reinstall-pins");
    let home = root.join("home");
    let work = root.join("work");
    let dist = root.join("dist");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    fs::create_dir_all(&dist).expect("create dist");
    write_file(
        &dist.join("index.json"),
        r#"[{"version":"v20.11.1","lts":"Iron","security":false}]"#,
    );
    create_node_archive(&dist, "20.11.1");

    let first = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1", "--no-pin"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&first);

    let second = run_avm_with_env(
        &work,
        &home,
        &["node", "install", "20.11.1"],
        &[("AVM_NODE_DIST_URL", dist.as_path())],
    );
    assert_success(&second);
    let local = fs::read_to_string(work.join(".avm.json")).expect("read local");
    assert!(
        local.contains("20.11.1"),
        "pin missing after reinstall: {local}"
    );
}

// ---------------------------------------------------------------------------
// Corrupt-config recovery: global scope
// ---------------------------------------------------------------------------

#[test]
fn corrupt_global_config_is_backed_up_and_recovered() {
    let root = temp_root("corrupt-global");
    let home = root.join("home");
    let work = root.join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_file(&home.join(".avm.json"), "not-json-at-all");

    let output = run_avm(&work, &home, &["which", "anything"]);
    assert_success(&output);
    assert!(stderr(&output).contains("was malformed"));
    let entries: Vec<_> = fs::read_dir(&home)
        .expect("read home")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        entries.iter().any(|n| n.starts_with(".avm.broken-")),
        "no backup in: {entries:?}"
    );
}
