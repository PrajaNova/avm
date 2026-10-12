use crate::{protocol::LockedAsset, run_timed, sha256_file};
use anyhow::{bail, Context, Result};
use std::path::Path;

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

/// Download the locked bytes, with no unverified escape hatch.
pub fn download_verified(asset: &LockedAsset, destination: &Path) -> Result<()> {
    validate_asset(asset)?;
    let result = (|| {
        if Path::new(&asset.url).is_file() {
            std::fs::copy(&asset.url, destination).context("copy locked archive")?;
        } else {
            let mut curl = std::process::Command::new("curl");
            curl.args([
                "-fL",
                "--proto",
                "=http,https",
                "--proto-redir",
                "=http,https",
                "--connect-timeout",
                "10",
                "--max-time",
                "300",
                &asset.url,
                "-o",
            ])
            .arg(destination);
            run_timed(curl, 330_000, "locked archive download", "AVM_CURL_TIMEOUT")?;
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
