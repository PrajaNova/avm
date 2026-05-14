use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn install_node(version: &str) -> Result<()> {
    let version = version.trim_start_matches('v');
    let platform = platform_name()?;
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME not set"))?;
    let home = PathBuf::from(home);
    let tools_root = home.join(".avm").join("tools").join("node");
    let target = tools_root.join(version);
    if target.join("bin").join(binary_name("node")).exists() {
        return Ok(());
    }

    let archive_name = format!("node-v{version}-{platform}.tar.gz");
    let extract_name = format!("node-v{version}-{platform}");
    let tmp_root = home.join(".avm").join("tmp").join("node").join(version);
    let archive_path = tmp_root.join(&archive_name);
    let extract_path = tmp_root.join(&extract_name);

    fs::create_dir_all(&tmp_root).context("failed to create node install temp dir")?;
    fs::create_dir_all(&tools_root).context("failed to create node tools dir")?;
    fetch_archive(version, &archive_name, &archive_path)?;

    if extract_path.exists() {
        fs::remove_dir_all(&extract_path).context("failed to clean previous node extraction")?;
    }
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&tmp_root)
        .status()
        .context("failed to extract node archive")?;
    if !status.success() {
        return Err(anyhow!("failed to extract node archive: {status}"));
    }

    if target.exists() {
        fs::remove_dir_all(&target).context("failed to replace existing node install")?;
    }
    fs::rename(&extract_path, &target).context("failed to move node install into place")?;
    Ok(())
}

fn fetch_archive(version: &str, archive_name: &str, destination: &Path) -> Result<()> {
    let mirror = std::env::var("AVM_NODE_DIST_URL")
        .unwrap_or_else(|_| "https://nodejs.org/dist".to_string());
    let local = Path::new(&mirror)
        .join(format!("v{version}"))
        .join(archive_name);
    if local.exists() {
        fs::copy(&local, destination)
            .with_context(|| format!("failed to copy node archive from {}", local.display()))?;
        return Ok(());
    }

    let url = format!("{}/v{version}/{archive_name}", mirror.trim_end_matches('/'));
    let status = Command::new("curl")
        .arg("-fL")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg("120")
        .arg(&url)
        .arg("-o")
        .arg(destination)
        .status()
        .with_context(|| format!("failed to download Node.js from {url}"))?;
    if !status.success() {
        return Err(anyhow!("failed to download Node.js from {url}: {status}"));
    }
    Ok(())
}

fn platform_name() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("linux", "x86_64") => Ok("linux-x64"),
        (os, arch) => Err(anyhow!("unsupported Node.js platform: {os}-{arch}")),
    }
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
