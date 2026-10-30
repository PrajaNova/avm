use crate::{env_timeout_ms, protocol::LockedAsset, run_timed, sha256_file};
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::path::Path;
use std::time::Duration;

/// Rejects empty, `.`/`..` or otherwise unsafe version strings so they cannot escape the tools directory when used as a path component.
pub fn validate_version(version: &str) -> Result<()> {
    if version.is_empty()
        || version == "."
        || version == ".."
        || !version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_+".contains(&c))
    {
        bail!("invalid resolved version '{version}'");
    }
    Ok(())
}

/// Checks a locked asset has a 64-char hex sha256 and an HTTP(S) URL or absolute local mirror path; errors otherwise.
pub fn validate_asset(asset: &LockedAsset) -> Result<()> {
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
        bail!("invalid sha256 for {}", asset.url);
    }
    if !asset.url.starts_with("https://")
        && !asset.url.starts_with("http://")
        && !Path::new(&asset.url).is_absolute()
    {
        bail!("locked asset must use HTTP(S) or an absolute local mirror path");
    }
    Ok(())
}

/// curl flags every request shares: fail on HTTP errors, follow redirects, and
/// allow only http(s) on the first request and on every redirect.
pub(crate) const CURL_SAFE_ARGS: [&str; 8] = [
    "-fL",
    "--proto",
    "=http,https",
    "--proto-redir",
    "=http,https",
    "--connect-timeout",
    "10",
    "--globoff",
];

/**
 * Runs `work` under a stderr spinner labelled `label`.
 *
 * indicatif hides the spinner when stderr is not a terminal, so CI logs and
 * plugin JSON on stdout stay clean.
 */
fn with_spinner<T>(label: &str, work: impl FnOnce() -> Result<T>) -> Result<T> {
    let spinner = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
    // The template is a constant, so a parse failure falls back to the default style.
    if let Ok(style) = ProgressStyle::with_template("{spinner} {msg} ({elapsed})") {
        spinner.set_style(style);
    }
    spinner.set_message(label.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    let result = work();
    spinner.finish_and_clear();
    result
}

/// `curl` `url` into `destination`. `$env_var` (seconds) overrides `default_ms`
/// for both curl's own limit and the outer kill timer.
pub fn download(
    url: &str,
    destination: &Path,
    default_ms: u64,
    env_var: &str,
    label: &str,
) -> Result<()> {
    let secs = env_timeout_ms(env_var, default_ms) / 1000;
    let mut curl = std::process::Command::new("curl");
    curl.args(CURL_SAFE_ARGS)
        .args(["--max-time", &secs.to_string()])
        .arg(url)
        .arg("-o")
        .arg(destination);
    with_spinner(label, || {
        run_timed(curl, default_ms + 30_000, label, env_var)
    })
}

/// Unpack a `.tar.gz` / `.zip` into `destination` with the system `tar` (Windows
/// 10+ ships bsdtar, which reads both), dropping `strip` leading path components.
pub fn extract_archive(
    archive: &Path,
    destination: &Path,
    strip: u32,
    default_ms: u64,
    env_var: &str,
    label: &str,
) -> Result<()> {
    let zip = archive.extension().is_some_and(|e| e == "zip");
    let mut tar = std::process::Command::new("tar");
    tar.arg(if zip || cfg!(windows) { "-xf" } else { "-xzf" })
        .arg(archive);
    if strip > 0 {
        tar.arg(format!("--strip-components={strip}"));
    }
    tar.arg("-C").arg(destination);
    with_spinner(label, || run_timed(tar, default_ms, label, env_var))
}

/// Download the locked bytes, with no unverified escape hatch.
pub fn download_verified(asset: &LockedAsset, destination: &Path) -> Result<()> {
    validate_asset(asset)?;
    let result = (|| {
        if Path::new(&asset.url).is_file() {
            fs_err::copy(&asset.url, destination).context("copy locked archive")?;
        } else {
            download(
                &asset.url,
                destination,
                300_000,
                "AVM_CURL_TIMEOUT",
                "locked archive download",
            )?;
        }
        let actual = sha256_file(destination)?;
        if actual != asset.sha256.to_ascii_lowercase() {
            bail!(
                "checksum mismatch for {}\n  expected {}\n  got      {actual}",
                asset.url,
                asset.sha256
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}

/// Returns the host platform as `<os>_<arch>` (e.g. `darwin_arm64`); errors on unsupported OS or architecture.
pub fn host_platform() -> Result<String> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => bail!("unsupported platform {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => bail!("unsupported architecture {other}"),
    };
    Ok(format!("{os}_{arch}"))
}

/// Errors unless `platform` is one of the supported `<os>_<arch>` identifiers.
pub fn validate_platform(platform: &str) -> Result<()> {
    if !matches!(
        platform,
        "darwin_amd64"
            | "darwin_arm64"
            | "linux_amd64"
            | "linux_arm64"
            | "windows_amd64"
            | "windows_arm64"
    ) {
        bail!("unsupported lock platform '{platform}' (use e.g. linux_amd64 or darwin_arm64)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_download_verifies_and_removes_tampered_bytes() {
        let root = std::env::temp_dir().join(format!("avm-locked-download-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source");
        let dest = root.join("dest");
        std::fs::write(&source, b"abc").unwrap();
        let asset = LockedAsset {
            url: source.display().to_string(),
            sha256: sha256_file(&source).unwrap(),
        };
        download_verified(&asset, &dest).unwrap();
        std::fs::write(&source, b"tampered").unwrap();
        assert!(download_verified(&asset, &dest)
            .unwrap_err()
            .to_string()
            .contains("checksum mismatch"));
        assert!(!dest.exists());
        assert!(validate_version("../escape").is_err());
        assert!(validate_asset(&LockedAsset {
            url: "file:///secret".into(),
            sha256: asset.sha256
        })
        .is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
