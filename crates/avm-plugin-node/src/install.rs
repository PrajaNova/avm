use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

const TAR_TIMEOUT_MS: u64 = 300_000;
const CURL_OUTER_TIMEOUT_MS: u64 = 180_000;

fn env_timeout_ms(var: &str, default_ms: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
        .unwrap_or(default_ms)
}

fn run_with_timeout(
    mut child: std::process::Child,
    ms: u64,
    label: &str,
    env_var: &str,
) -> Result<std::process::ExitStatus> {
    match child
        .wait_timeout(Duration::from_millis(ms))
        .with_context(|| format!("failed while waiting for {label}"))?
    {
        Some(s) => Ok(s),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err(anyhow!(
                "{label} timed out after {}s — set {env_var}=<seconds> to extend",
                ms / 1000
            ))
        }
    }
}

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
    let child = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&tmp_root)
        .spawn()
        .context("failed to spawn tar")?;
    let status = run_with_timeout(
        child,
        env_timeout_ms("AVM_TAR_TIMEOUT", TAR_TIMEOUT_MS),
        "tar extraction",
        "AVM_TAR_TIMEOUT",
    )?;
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
    let child = Command::new("curl")
        .arg("-fL")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg("120")
        .arg(&url)
        .arg("-o")
        .arg(destination)
        .spawn()
        .with_context(|| format!("failed to spawn curl for {url}"))?;
    let status = run_with_timeout(
        child,
        env_timeout_ms("AVM_CURL_TIMEOUT", CURL_OUTER_TIMEOUT_MS),
        "node download",
        "AVM_CURL_TIMEOUT",
    )?;
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
