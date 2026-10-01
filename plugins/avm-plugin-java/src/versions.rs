use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{ToolVersion, ToolVersionQuery};
use serde::Deserialize;

/// The foojay Disco API (https://api.foojay.io) aggregates JDK builds across
/// vendors in one place; `temurin` is Eclipse Adoptium's build of OpenJDK —
/// the reference "just OpenJDK, nothing vendor-specific" distribution, and
/// unlike some vendors it keeps full historical patch releases rather than
/// pruning old ones. One request returns the whole patch history already
/// sorted newest-first, so there's no need for one call per major version.
const DISCO_BASE_URL: &str = "https://api.foojay.io/disco/v3.0";
const DISTRIBUTION: &str = "temurin";

pub fn available_versions(query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
    let releases = query.filter(package_index()?, |pkg| pkg.major_version);
    Ok(releases
        .into_iter()
        .map(|pkg| ToolVersion {
            version: avm_version(&pkg.java_version),
            label: pkg.java_version,
            channel: Some(pkg.term_of_support.clone()),
            is_lts: pkg.term_of_support == "lts",
            is_security: false,
        })
        .collect())
}

/// avm's on-disk version string. Prefixed so `~/.avm/tools/java/<version>`
/// stays self-describing and matches the naming an existing asdf-java
/// install already used (`openjdk-17.0.2`) — no forced reinstall on cutover.
fn avm_version(java_version: &str) -> String {
    format!("openjdk-{java_version}")
}

pub fn strip_avm_prefix(version: &str) -> &str {
    version.strip_prefix("openjdk-").unwrap_or(version)
}

/// Find the download package for an exact `java_version` (e.g. `17.0.9+9`,
/// with or without avm's `openjdk-` prefix already stripped by the caller).
pub fn find_package(java_version: &str) -> Result<Package> {
    package_index()?
        .into_iter()
        .find(|pkg| pkg.java_version == java_version || pkg.distribution_version == java_version)
        .ok_or_else(|| {
            anyhow!("no Temurin OpenJDK build found for version '{java_version}' ({} {})", host_os_param().unwrap_or("?"), host_arch_param().unwrap_or("?"))
        })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    java_version: String,
    distribution_version: String,
    major_version: u64,
    #[serde(default)]
    term_of_support: String,
    pub links: PackageLinks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageLinks {
    pub pkg_download_redirect: String,
    pub pkg_info_uri: String,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    checksum: String,
    checksum_type: String,
}

#[derive(Debug, Deserialize)]
struct PackageInfoResponse {
    result: Vec<PackageInfo>,
}

/// The sha256 foojay reports for `package` (Adoptium's published checksum).
pub fn package_sha256(package: &Package) -> Result<String> {
    parse_sha256(&avm_plugin_api::fetch(&package.links.pkg_info_uri, 20)?)
}

fn parse_sha256(raw: &[u8]) -> Result<String> {
    let parsed: PackageInfoResponse =
        serde_json::from_slice(raw).context("failed to parse foojay package info")?;
    parsed
        .result
        .into_iter()
        .find(|info| info.checksum_type == "sha256" && !info.checksum.is_empty())
        .map(|info| info.checksum)
        .ok_or_else(|| anyhow!("foojay package info has no sha256 checksum"))
}

#[derive(Debug, Deserialize)]
struct PackagesResponse {
    result: Vec<Package>,
}

fn package_index() -> Result<Vec<Package>> {
    let url = format!(
        "{DISCO_BASE_URL}/packages?distribution={DISTRIBUTION}&operating_system={}&architecture={}&archive_type={}&package_type=jdk&release_status=ga{}",
        host_os_param()?,
        host_arch_param()?,
        ARCHIVE_TYPE,
        libc_param(),
    );

    let raw = avm_plugin_api::fetch(&url, 20)?;
    let parsed: PackagesResponse =
        serde_json::from_slice(&raw).context("failed to parse foojay Disco API response")?;
    Ok(parsed.result)
}

/// Temurin ships zips for Windows, tarballs elsewhere.
pub const ARCHIVE_TYPE: &str = if cfg!(windows) { "zip" } else { "tar.gz" };

fn host_os_param() -> Result<&'static str> {
    match std::env::consts::OS {
        "macos" => Ok("macos"),
        "linux" => Ok("linux"),
        "windows" => Ok("windows"),
        other => Err(anyhow!("unsupported OpenJDK platform: {other}")),
    }
}

/// Linux results include Alpine (musl) builds, listed first for some
/// versions; they can't run on a glibc distro, so ask for glibc only.
fn libc_param() -> &'static str {
    if std::env::consts::OS == "linux" {
        "&lib_c_type=glibc"
    } else {
        ""
    }
}

fn host_arch_param() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Ok("aarch64"),
        "x86_64" => Ok("x64"),
        other => Err(anyhow!("unsupported OpenJDK architecture: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256_from_package_info() {
        let ok = br#"{"result":[{"checksum":"ab12","checksum_type":"sha256","filename":"x"}]}"#;
        assert_eq!(parse_sha256(ok).unwrap(), "ab12");
        assert!(parse_sha256(br#"{"result":[{"checksum":"ab12","checksum_type":"md5"}]}"#).is_err());
        assert!(parse_sha256(br#"{"result":[{"checksum":"","checksum_type":"sha256"}]}"#).is_err());
    }
}
