use crate::versions::{find_package, strip_avm_prefix};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

const CURL_TIMEOUT_MS: u64 = 300_000;
const TAR_TIMEOUT_MS: u64 = 120_000;

fn env_timeout_ms(var: &str, default_ms: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
        .unwrap_or(default_ms)
}

fn run_with_timeout(mut cmd: Command, ms: u64, label: &str, env_var: &str) -> Result<()> {
    let mut child = cmd.spawn().with_context(|| format!("failed to spawn {label}"))?;
    let status = child
        .wait_timeout(Duration::from_millis(ms))
        .with_context(|| format!("failed while waiting for {label}"))?;
    let status = match status {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!(
                "{label} timed out after {}s — set {env_var}=<seconds> to extend",
                ms / 1000
            ));
        }
    };
    if !status.success() {
        return Err(anyhow!("{label} failed: {status}"));
    }
    Ok(())
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("HOME not set"))
}

pub fn tools_root() -> Result<PathBuf> {
    Ok(home_dir()?.join(".avm").join("tools").join("java"))
}

pub fn install_java(version: &str) -> Result<()> {
    let target = tools_root()?.join(version);
    if target.join("bin").join(binary_name("java")).exists() {
        return Ok(());
    }

    let java_version = strip_avm_prefix(version);
    let package = find_package(java_version)?;

    let tmp = home_dir()?.join(".avm").join("tmp").join("java").join(version);
    fs::create_dir_all(&tmp).context("failed to create java install temp dir")?;
    fs::create_dir_all(target.parent().unwrap()).context("failed to create java tools dir")?;

    let archive = tmp.join("jdk.tar.gz");
    download(&package.links.pkg_download_redirect, &archive)?;
    let extracted_root = extract(&archive, &tmp)?;
    let _ = fs::remove_file(&archive);

    // Temurin's macOS archives bundle a full `Contents/Home` app layout so
    // the JDK can register with `/usr/libexec/java_home`; avm just wants a
    // flat `bin/`, `lib/`, ... directly under the version dir, same as Linux.
    let source = if extracted_root.join("Contents").join("Home").exists() {
        extracted_root.join("Contents").join("Home")
    } else {
        extracted_root.clone()
    };

    if target.exists() {
        fs::remove_dir_all(&target).context("failed to replace existing java install")?;
    }
    fs::rename(&source, &target).context("failed to move JDK into place")?;
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

fn download(redirect_url: &str, destination: &Path) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.arg("-fL")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg((env_timeout_ms("AVM_JAVA_CURL_TIMEOUT", CURL_TIMEOUT_MS) / 1000).to_string())
        .arg(redirect_url)
        .arg("-o")
        .arg(destination);
    run_with_timeout(
        cmd,
        env_timeout_ms("AVM_JAVA_CURL_TIMEOUT", CURL_TIMEOUT_MS),
        "OpenJDK download",
        "AVM_JAVA_CURL_TIMEOUT",
    )
    .with_context(|| format!("failed to download OpenJDK from {redirect_url}"))
}

/// Extract into `tmp` and return the single top-level directory the archive
/// produced (Temurin tarballs contain exactly one root dir, e.g. `jdk-21.0.12+7`).
fn extract(archive: &Path, tmp: &Path) -> Result<PathBuf> {
    let before: std::collections::HashSet<PathBuf> = fs::read_dir(tmp)
        .context("failed to read java install temp dir")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();

    let mut cmd = Command::new("tar");
    cmd.arg("-xzf").arg(archive).arg("-C").arg(tmp);
    run_with_timeout(
        cmd,
        env_timeout_ms("AVM_JAVA_TAR_TIMEOUT", TAR_TIMEOUT_MS),
        "OpenJDK archive extraction",
        "AVM_JAVA_TAR_TIMEOUT",
    )?;

    fs::read_dir(tmp)
        .context("failed to read java install temp dir")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.is_dir() && !before.contains(p))
        .ok_or_else(|| anyhow!("OpenJDK archive did not produce a top-level directory"))
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
