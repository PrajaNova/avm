use crate::versions::{find_package, package_sha256, strip_avm_prefix};
use anyhow::{Context, Result};
use avm_plugin_api::{run_timed, tool_dir};
use std::fs;
use std::process::Command;

const CURL_TIMEOUT_MS: u64 = 300_000;
const TAR_TIMEOUT_MS: u64 = 120_000;

pub fn install_java(version: &str) -> Result<()> {
    let tools_root = tool_dir("java")?;
    let target = tools_root.join(version);
    if target.join("bin").join(crate::JAVA_BIN).exists() {
        return Ok(());
    }

    let package = find_package(strip_avm_prefix(version))?;

    // Staged inside tools_root: never counts as installed (no `<version>/bin/java`)
    // and the final rename stays on one filesystem.
    let tmp = tools_root.join(format!(".tmp-{version}"));
    let jdk = tmp.join("jdk");
    if tmp.exists() {
        fs::remove_dir_all(&tmp).context("failed to clean previous java install temp dir")?;
    }
    fs::create_dir_all(&jdk).context("failed to create java install temp dir")?;

    let archive = tmp.join(format!("jdk.{}", crate::versions::ARCHIVE_TYPE));
    let url = &package.links.pkg_download_redirect;
    let max_secs = avm_plugin_api::env_timeout_ms("AVM_JAVA_CURL_TIMEOUT", CURL_TIMEOUT_MS) / 1000;
    let mut curl = Command::new("curl");
    curl.args(["-fL", "--connect-timeout", "10", "--max-time", &max_secs.to_string(), url, "-o"])
        .arg(&archive);
    run_timed(curl, CURL_TIMEOUT_MS, "OpenJDK download", "AVM_JAVA_CURL_TIMEOUT")
        .with_context(|| format!("failed to download OpenJDK from {url}"))?;
    if let Err(e) = verify_archive(&package, &archive) {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }

    // Temurin tarballs have exactly one root dir (e.g. `jdk-21.0.12+7`) — strip it.
    let mut tar = Command::new("tar");
    // Windows 10+ ships bsdtar, which also reads zip (and --strip-components).
    tar.arg(if cfg!(windows) { "-xf" } else { "-xzf" }).arg(&archive).arg("--strip-components=1").arg("-C").arg(&jdk);
    run_timed(tar, TAR_TIMEOUT_MS, "OpenJDK archive extraction", "AVM_JAVA_TAR_TIMEOUT")?;

    // Temurin's macOS archives are `<root>/Contents/Home/{bin,lib,...}` (an app
    // bundle layout for `/usr/libexec/java_home`); avm wants a flat JDK dir.
    let mac_home = jdk.join("Contents").join("Home");
    let source = if mac_home.exists() { mac_home } else { jdk };

    if target.exists() {
        fs::remove_dir_all(&target).context("failed to replace existing java install")?;
    }
    fs::rename(&source, &target).context("failed to move JDK into place")?;
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

/// Check the JDK archive against the sha256 foojay reports for it. Fails
/// closed unless `AVM_ALLOW_UNVERIFIED=1`.
fn verify_archive(package: &crate::versions::Package, archive: &std::path::Path) -> Result<()> {
    let expected = match package_sha256(package) {
        Ok(hash) => hash,
        Err(_) if std::env::var("AVM_ALLOW_UNVERIFIED").as_deref() == Ok("1") => {
            eprintln!("warning: installing UNVERIFIED OpenJDK (no sha256 available; AVM_ALLOW_UNVERIFIED=1)");
            return Ok(());
        }
        Err(e) => return Err(e.context("can't verify OpenJDK download; set AVM_ALLOW_UNVERIFIED=1 to skip")),
    };
    let name = format!("jdk.{}", crate::versions::ARCHIVE_TYPE);
    avm_plugin_api::verify_sha256(archive, &format!("{expected}  {name}"), &name)
        .map(|_| ())
        .context("refusing to install OpenJDK")
}
