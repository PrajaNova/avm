#![cfg(unix)]

use avm_plugin_sdk::{
    protocol::{LockedAsset, ResolveRequest, ResolveResponse},
    sha256_file,
};
use fs_err as fs;
use std::{os::unix::fs::PermissionsExt, process::Command};

const TRIPLES: [&str; 6] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "aarch64-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
];

#[test]
fn resolves_cross_platform_python_and_installs_from_a_local_index() {
    let root = std::env::temp_dir().join(format!("avm-python-lock-test-{}", std::process::id()));
    let source = root.join("python/bin");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(root.join("home")).unwrap();
    fs::write(source.join("python3"), "#!/bin/sh\necho locked-python\n").unwrap();
    fs::set_permissions(
        source.join("python3"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let archive = root.join("python.tar.gz");
    assert!(Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&root)
        .arg("python")
        .status()
        .unwrap()
        .success());
    let sha256 = sha256_file(&archive).unwrap();
    let artifacts: Vec<_> = TRIPLES
        .iter()
        .map(|platform| {
            serde_json::json!({"platform":platform,"variant":"install_only_stripped","archive_format":"tar.gz","url":archive,"sha256":sha256})
        })
        .collect();
    let index = root.join("index.ndjson");
    fs::write(
        &index,
        serde_json::json!({"version":"3.12.3+20250101","artifacts":artifacts}).to_string(),
    )
    .unwrap();

    let command = |args: &[String]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_avm-plugin-python"));
        cmd.args(args)
            .env("HOME", root.join("home"))
            .env("AVM_PYTHON_INDEX", &index);
        cmd
    };
    let run = |args: &[String]| command(args).output().unwrap();
    let request = ResolveRequest {
        version: "3.12.3".into(),
        platforms: vec![
            "linux_amd64".into(),
            "darwin_arm64".into(),
            "windows_amd64".into(),
        ],
    };
    let out = run(&["resolve".into(), serde_json::to_string(&request).unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let response: ResolveResponse = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(response.assets.len(), 3);
    assert_eq!(response.assets["darwin_arm64"].sha256, sha256);
    assert!(!run(&[
        "resolve".into(),
        r#"{"version":"3.1.1","platforms":[]}"#.into()
    ])
    .status
    .success());

    let mut asset = LockedAsset {
        url: archive.display().to_string(),
        sha256: "0".repeat(64),
    };
    let out = run(&[
        "install-locked".into(),
        "3.12.3".into(),
        serde_json::to_string(&asset).unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(!root.join("home/.avm/tools/python/3.12.3").exists());

    asset.sha256 = sha256;
    let args = [
        "install-locked".into(),
        "3.12.3".into(),
        serde_json::to_string(&asset).unwrap(),
    ];
    let guard = avm_plugin_sdk::storage::lock(&root.join("home/.avm/tools/python/.install-3.12.3"))
        .unwrap();
    let mut children: Vec<_> = (0..2).map(|_| command(&args).spawn().unwrap()).collect();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(!root.join("home/.avm/tools/python/3.12.3").exists());
    drop(guard);
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let python = root.join("home/.avm/tools/python/3.12.3/bin/python3");
    assert_eq!(
        Command::new(python).output().unwrap().stdout,
        b"locked-python\n"
    );

    // Unlocked install resolves the host platform from the same index.
    assert!(run(&["uninstall".into(), "3.12.3".into()]).status.success());
    assert!(run(&["install".into(), "3.12.3".into()]).status.success());
    assert!(root
        .join("home/.avm/tools/python/3.12.3/bin/python3")
        .exists());
    fs::remove_dir_all(root).unwrap();
}
