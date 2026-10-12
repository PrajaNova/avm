#![cfg(unix)]

use avm_plugin_api::{
    download::host_platform,
    protocol::{LockedAsset, ResolveRequest, ResolveResponse},
    sha256_file,
};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn resolves_platform_checksums_and_installs_the_locked_archive() {
    let root = std::env::temp_dir().join(format!("avm-node-lock-test-{}", std::process::id()));
    fs::create_dir_all(root.join("home")).unwrap();
    let release = root.join("mirror/v20.1.0");
    fs::create_dir_all(&release).unwrap();
    let host = host_platform().unwrap();
    let os = if host.starts_with("darwin") {
        "darwin"
    } else {
        "linux"
    };
    let arch = if host.ends_with("amd64") {
        "x64"
    } else {
        "arm64"
    };
    let name = format!("node-v20.1.0-{os}-{arch}");
    let source = root.join(&name).join("bin");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("node"), "#!/bin/sh\necho locked-node\n").unwrap();
    fs::set_permissions(source.join("node"), fs::Permissions::from_mode(0o755)).unwrap();
    let archive = release.join(format!("{name}.tar.gz"));
    assert!(Command::new("tar")
        .args(["-czf"])
        .arg(&archive)
        .arg("-C")
        .arg(&root)
        .arg(&name)
        .status()
        .unwrap()
        .success());
    let sha256 = sha256_file(&archive).unwrap();
    fs::write(
        release.join("SHASUMS256.txt"),
        format!(
            "{sha256}  {name}.tar.gz\n{}  node-v20.1.0-win-x64.zip\n",
            "a".repeat(64)
        ),
    )
    .unwrap();

    let run = |args: &[String]| {
        Command::new(env!("CARGO_BIN_EXE_avm-plugin-node"))
            .args(args)
            .env("HOME", root.join("home"))
            .env("AVM_NODE_DIST_URL", root.join("mirror"))
            .env("AVM_ALLOW_UNVERIFIED", "1")
            .output()
            .unwrap()
    };
    let request = ResolveRequest {
        version: "20.1.0".into(),
        platforms: vec![host.clone(), "windows_amd64".into()],
    };
    let output = run(&["resolve".into(), serde_json::to_string(&request).unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: ResolveResponse = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response.assets[&host].sha256, sha256);
    assert!(response.assets["windows_amd64"]
        .url
        .ends_with("win-x64.zip"));
    let mut asset = LockedAsset {
        url: archive.display().to_string(),
        sha256: "0".repeat(64),
    };
    let out = run(&[
        "install-locked".into(),
        "20.1.0".into(),
        serde_json::to_string(&asset).unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(!root.join("home/.avm/tools/node/20.1.0").exists());
    asset.sha256 = sha256;
    // Upstream checksums can disappear after locking; installation uses the lock.
    fs::remove_file(release.join("SHASUMS256.txt")).unwrap();
    let out = run(&[
        "install-locked".into(),
        "20.1.0".into(),
        serde_json::to_string(&asset).unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let node = root.join("home/.avm/tools/node/20.1.0/bin/node");
    assert_eq!(
        Command::new(node).output().unwrap().stdout,
        b"locked-node\n"
    );
    fs::remove_dir_all(root).unwrap();
}
