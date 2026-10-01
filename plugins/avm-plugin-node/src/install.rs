use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{fetch, run_timed, tool_dir, verify_sha256};
use std::fs;
use std::path::Path;
use std::process::Command;

const TAR_TIMEOUT_MS: u64 = 300_000;
const CURL_OUTER_TIMEOUT_MS: u64 = 180_000;

pub fn install_node(version: &str) -> Result<()> {
    let version = version.trim_start_matches('v');
    let platform = platform_name()?;
    let tools_root = tool_dir("node")?;
    let target = tools_root.join(version);
    if target.join("bin").join(crate::NODE_BIN).exists() {
        return Ok(());
    }

    // Download + extract inside tools_root: the half-done dirs never count as
    // installed (no `<version>/bin/node`), and the final rename stays on one fs.
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let archive_name = format!("node-v{version}-{platform}.{ext}");
    let tmp_root = tools_root.join(format!(".tmp-{version}"));
    let archive_path = tmp_root.join(&archive_name);
    let extract_path = tmp_root.join(format!("node-v{version}-{platform}"));

    if tmp_root.exists() {
        fs::remove_dir_all(&tmp_root).context("failed to clean previous node install temp dir")?;
    }
    fs::create_dir_all(&tmp_root).context("failed to create node install temp dir")?;
    fetch_archive(version, &archive_name, &archive_path)?;
    if let Err(e) = verify_archive(&mirror(), version, &archive_name, &archive_path) {
        let _ = fs::remove_dir_all(&tmp_root);
        return Err(e);
    }

    let mut tar = Command::new("tar");
    // Windows 10+ ships bsdtar, which also reads zip.
    tar.arg(if cfg!(windows) { "-xf" } else { "-xzf" }).arg(&archive_path).arg("-C").arg(&tmp_root);
    run_timed(tar, TAR_TIMEOUT_MS, "node archive extraction", "AVM_TAR_TIMEOUT")?;

    if target.exists() {
        fs::remove_dir_all(&target).context("failed to replace existing node install")?;
    }
    // The Windows zip has node.exe at its root (and npm installs global
    // packages next to it), so it becomes `<version>\\bin` to keep one layout.
    if cfg!(windows) {
        fs::create_dir_all(&target).context("failed to create node install dir")?;
        let bin = target.join("bin");
        fs::rename(&extract_path, &bin).context("failed to move node install into place")?;
        // Node for Windows points npm's global prefix at %APPDATA%\npm, shared
        // by every version. Point it at this version's dir, as on Unix, so
        // `npm i -g` lands where avm's shims look (and per-version pins hold).
        let npmrc = bin.join("node_modules").join("npm").join("npmrc");
        fs::write(&npmrc, format!("prefix={}\n", bin.display().to_string().replace('\\', "/")))
            .context("failed to set npm's global prefix")?;
    } else {
        fs::rename(&extract_path, &target).context("failed to move node install into place")?;
    }
    let _ = fs::remove_dir_all(&tmp_root);
    Ok(())
}

pub(crate) fn mirror() -> String {
    std::env::var("AVM_NODE_DIST_URL").unwrap_or_else(|_| "https://nodejs.org/dist".to_string())
}

/// Check the archive against the release's `SHASUMS256.txt` on the same
/// mirror. Fails closed unless `AVM_ALLOW_UNVERIFIED=1`.
fn verify_archive(mirror: &str, version: &str, archive_name: &str, archive: &Path) -> Result<()> {
    let url = format!("{}/v{version}/SHASUMS256.txt", mirror.trim_end_matches('/'));
    let sums = match fetch(&url, 20) {
        Ok(sums) => sums,
        Err(_) if std::env::var("AVM_ALLOW_UNVERIFIED").as_deref() == Ok("1") => {
            eprintln!("warning: installing UNVERIFIED {archive_name} (no {url}; AVM_ALLOW_UNVERIFIED=1)");
            return Ok(());
        }
        Err(e) => return Err(e.context("can't verify Node.js download; set AVM_ALLOW_UNVERIFIED=1 to skip")),
    };
    verify_sha256(archive, &String::from_utf8_lossy(&sums), archive_name)
        .map(|_| ())
        .context("refusing to install Node.js")
}

fn fetch_archive(version: &str, archive_name: &str, destination: &Path) -> Result<()> {
    let mirror = mirror();
    let local = Path::new(&mirror).join(format!("v{version}")).join(archive_name);
    if local.exists() {
        fs::copy(&local, destination)
            .with_context(|| format!("failed to copy node archive from {}", local.display()))?;
        return Ok(());
    }

    let url = format!("{}/v{version}/{archive_name}", mirror.trim_end_matches('/'));
    let mut curl = Command::new("curl");
    curl.args(["-fL", "--connect-timeout", "10", "--max-time", "120", &url, "-o"])
        .arg(destination);
    run_timed(curl, CURL_OUTER_TIMEOUT_MS, "node download", "AVM_CURL_TIMEOUT")
        .with_context(|| format!("failed to download Node.js from {url}"))
}

fn platform_name() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("linux", "x86_64") => Ok("linux-x64"),
        ("windows", "aarch64") => Ok("win-arm64"),
        ("windows", "x86_64") => Ok("win-x64"),
        (os, arch) => Err(anyhow!("unsupported Node.js platform: {os}-{arch}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_archive_checks_shasums() {
        let mirror = std::env::temp_dir().join(format!("avm-node-sums-{}", std::process::id()));
        let release = mirror.join("v1.0.0");
        fs::create_dir_all(&release).unwrap();
        let archive = release.join("node.tar.gz");
        fs::write(&archive, b"abc").unwrap();
        let mirror_str = mirror.to_str().unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

        fs::write(release.join("SHASUMS256.txt"), format!("{abc}  node.tar.gz\n")).unwrap();
        assert!(verify_archive(mirror_str, "1.0.0", "node.tar.gz", &archive).is_ok());
        fs::write(&archive, b"tampered").unwrap();
        let err = format!("{:#}", verify_archive(mirror_str, "1.0.0", "node.tar.gz", &archive).unwrap_err());
        assert!(err.contains("checksum mismatch"), "{err}");
        fs::remove_file(release.join("SHASUMS256.txt")).unwrap();
        assert!(verify_archive(mirror_str, "1.0.0", "node.tar.gz", &archive).is_err());
        let _ = fs::remove_dir_all(mirror);
    }
}
