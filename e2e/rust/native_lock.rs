use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::{Command, Output},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct Disco {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Disco {
    fn new(mut package: Value, checksum: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        package["links"]["pkg_info_uri"] = json!(format!("{url}/info"));
        let packages = json!({"result":[package]}).to_string();
        let info = json!({"result":[{"checksum":checksum,"checksum_type":"sha256"}]}).to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    // Windows sockets inherit the listener's non-blocking mode; a
                    // read before the request lands would drop the connection.
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut request = [0; 8192];
                    if let Ok(n) = stream.read(&mut request) {
                        let body = if request[..n].starts_with(b"GET /info ") {
                            &info
                        } else {
                            &packages
                        };
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Disco {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn archive(root: &Path, name: &str, destination: &Path) {
    let mut cmd = Command::new("tar");
    if cfg!(windows) {
        cmd.args(["-a", "-cf"]);
    } else {
        cmd.arg("-czf");
    }
    assert!(cmd
        .arg(destination)
        .arg("-C")
        .arg(root)
        .arg(name)
        .status()
        .unwrap()
        .success());
}

fn success(out: Output) -> String {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

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
    success(run(&["lock"]));
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
    assert!(!root.join("home/.avm/tools/java/openjdk-21.0.1+12").exists());
    fs::write(&lock_file, &lock_bytes).unwrap();
    fs::write(&java_archive, "tampered ZIP or tarball").unwrap();
    assert!(!run(&["-p", "install", "--frozen"]).status.success());
    assert!(!root.join("home/.avm/tools/java/openjdk-21.0.1+12").exists());
    fs::remove_dir_all(root).unwrap();
}
