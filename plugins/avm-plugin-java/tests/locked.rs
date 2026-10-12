#![cfg(unix)]

use avm_plugin_api::{
    protocol::{LockedAsset, ResolveRequest, ResolveResponse},
    sha256_file,
};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn resolves_cross_platform_java_and_installs_without_the_live_index() {
    let root = std::env::temp_dir().join(format!("avm-java-lock-test-{}", std::process::id()));
    let source = root.join("jdk/Contents/Home/bin");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(root.join("home")).unwrap();
    fs::create_dir_all(root.join("disco")).unwrap();
    fs::write(source.join("java"), "#!/bin/sh\necho locked-java\n").unwrap();
    fs::set_permissions(source.join("java"), fs::Permissions::from_mode(0o755)).unwrap();
    let archive = root.join("jdk.tar.gz");
    assert!(Command::new("tar")
        .args(["-czf"])
        .arg(&archive)
        .arg("-C")
        .arg(&root)
        .arg("jdk")
        .status()
        .unwrap()
        .success());
    let sha256 = sha256_file(&archive).unwrap();
    let info = root.join("info.json");
    fs::write(
        &info,
        serde_json::to_vec(
            &serde_json::json!({"result":[{"checksum":sha256,"checksum_type":"sha256"}]}),
        )
        .unwrap(),
    )
    .unwrap();
    for (os, arch, ext, libc) in [
        ("linux", "x64", "tar.gz", "&lib_c_type=glibc"),
        ("macos", "aarch64", "tar.gz", ""),
        ("windows", "x64", "zip", ""),
    ] {
        let index = root.join(format!("disco/packages?distribution=temurin&operating_system={os}&architecture={arch}&archive_type={ext}&package_type=jdk&release_status=ga{libc}"));
        let package = serde_json::json!({"java_version":"21.0.1+12","distribution_version":"21.0.1+12","major_version":21,"term_of_support":"lts","links":{"pkg_download_redirect":archive,"pkg_info_uri":info}});
        fs::write(
            index,
            serde_json::to_vec(&serde_json::json!({"result":[package]})).unwrap(),
        )
        .unwrap();
    }
    let command = |args: &[String]| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_avm-plugin-java"));
        cmd
            .args(args)
            .env("HOME", root.join("home"))
            .env("AVM_JAVA_DISCO_URL", root.join("disco"))
            .env("AVM_ALLOW_UNVERIFIED", "1");
        cmd
    };
    let run = |args: &[String]| command(args).output().unwrap();
    let request = ResolveRequest {
        version: "openjdk-21.0.1+12".into(),
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
    fs::remove_dir_all(root.join("disco")).unwrap();
    let mut asset = LockedAsset {
        url: archive.display().to_string(),
        sha256: "0".repeat(64),
    };
    let out = run(&[
        "install-locked".into(),
        request.version.clone(),
        serde_json::to_string(&asset).unwrap(),
    ]);
    assert!(!out.status.success());
    assert!(!root.join("home/.avm/tools/java/openjdk-21.0.1+12").exists());
    asset.sha256 = sha256;
    let args = ["install-locked".into(), "openjdk-21.0.1+12".into(), serde_json::to_string(&asset).unwrap()];
    let guard = avm_plugin_api::storage::lock(&root.join("home/.avm/tools/java/.install-openjdk-21.0.1+12")).unwrap();
    let mut children: Vec<_> = (0..2).map(|_| command(&args).spawn().unwrap()).collect();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(!root.join("home/.avm/tools/java/openjdk-21.0.1+12").exists());
    drop(guard);
    for child in &mut children { assert!(child.wait().unwrap().success()); }
    let java = root.join("home/.avm/tools/java/openjdk-21.0.1+12/bin/java");
    assert_eq!(
        Command::new(java).output().unwrap().stdout,
        b"locked-java\n"
    );
    fs::remove_dir_all(root).unwrap();
}
