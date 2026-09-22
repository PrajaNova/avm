use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{ToolVersion, ToolVersionQuery};
use serde::Deserialize;
use std::process::Command;

/// The foojay Disco API (https://api.foojay.io) aggregates JDK builds across
/// vendors in one place; `temurin` is Eclipse Adoptium's build of OpenJDK —
/// the reference "just OpenJDK, nothing vendor-specific" distribution, and
/// unlike some vendors it keeps full historical patch releases rather than
/// pruning old ones. One request returns the whole patch history already
/// sorted newest-first, so there's no need for one call per major version.
const DISCO_BASE_URL: &str = "https://api.foojay.io/disco/v3.0";
const DISTRIBUTION: &str = "temurin";

pub fn available_versions(query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
    let releases = package_index()?;

    let filtered: Vec<&Package> = match query {
        ToolVersionQuery::Recent => releases.iter().take(10).collect(),
        ToolVersionQuery::Latest => releases.iter().take(1).collect(),
        ToolVersionQuery::Major(major) => releases
            .iter()
            .filter(|pkg| pkg.major_version == major)
            .collect(),
    };

    Ok(filtered
        .into_iter()
        .map(|pkg| ToolVersion {
            version: avm_version(&pkg.java_version),
            label: pkg.java_version.clone(),
            channel: Some(pkg.term_of_support.clone()),
            is_lts: pkg.term_of_support == "lts",
            is_security: false,
        })
        .collect())
}

/// avm's on-disk version string. Prefixed so `~/.avm/tools/java/<version>`
/// stays self-describing and matches the naming an existing asdf-java
/// install already used (`openjdk-17.0.2`) — no forced reinstall on cutover.
pub fn avm_version(java_version: &str) -> String {
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
    pub java_version: String,
    pub distribution_version: String,
    pub major_version: u64,
    #[serde(default)]
    pub term_of_support: String,
    pub links: PackageLinks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageLinks {
    pub pkg_download_redirect: String,
}

#[derive(Debug, Deserialize)]
struct PackagesResponse {
    result: Vec<Package>,
}

fn package_index() -> Result<Vec<Package>> {
    let url = format!(
        "{DISCO_BASE_URL}/packages?distribution={DISTRIBUTION}&operating_system={}&architecture={}&archive_type=tar.gz&package_type=jdk&release_status=ga",
        host_os_param()?,
        host_arch_param()?,
    );

    let output = Command::new("curl")
        .arg("-fsSL")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg("20")
        .arg(&url)
        .output()
        .with_context(|| format!("failed to fetch OpenJDK version index from {url}"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "failed to fetch OpenJDK version index from {url}: curl exited with {}",
            output.status
        ));
    }

    let parsed: PackagesResponse =
        serde_json::from_slice(&output.stdout).context("failed to parse foojay Disco API response")?;
    Ok(parsed.result)
}

fn host_os_param() -> Result<&'static str> {
    match std::env::consts::OS {
        "macos" => Ok("macos"),
        "linux" => Ok("linux"),
        other => Err(anyhow!("unsupported OpenJDK platform: {other}")),
    }
}

fn host_arch_param() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Ok("aarch64"),
        "x86_64" => Ok("x64"),
        other => Err(anyhow!("unsupported OpenJDK architecture: {other}")),
    }
}
