use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::protocol::{LockedAsset, ResolveResponse};
use avm_plugin_sdk::{tool_dir, ToolVersion, ToolVersionQuery};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

/// Astral's index of every python-build-standalone build
/// (https://github.com/astral-sh/python-build-standalone): one JSON object per
/// line with each artifact's URL and sha256, back to CPython 3.7. It's ~20 MB,
/// so it's cached on disk for `CACHE_TTL`.
const DEFAULT_INDEX: &str =
    "https://raw.githubusercontent.com/astral-sh/versions/main/v1/python-build-standalone.ndjson";
const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);
/// Relocatable archives; newer releases ship the stripped one, older only the plain one.
const VARIANTS: [&str; 2] = ["install_only_stripped", "install_only"];

type Triple = (u64, u64, u64);

#[derive(Deserialize)]
struct Line {
    version: String,
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    platform: String,
    variant: String,
    url: String,
    archive_format: String,
    sha256: String,
}

/// Best build per CPython version, per target triple.
type Index = BTreeMap<Triple, BTreeMap<String, Build>>;

struct Build {
    /// `(release tag, prefers stripped)`; the larger key wins.
    rank: (String, bool),
    asset: LockedAsset,
}

/// ponytail: no ETag revalidation, so a brand-new release can take up to
/// `CACHE_TTL` to show up; delete `~/.avm/tools/python/.index.ndjson` to force it.
fn index_bytes() -> Result<Vec<u8>> {
    if let Ok(src) = std::env::var("AVM_PYTHON_INDEX") {
        return avm_plugin_sdk::fetch(&src, 120);
    }
    let cache = tool_dir("python")?.join(".index.ndjson");
    let age = std::fs::metadata(&cache)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok());
    if age.is_some_and(|a| a < CACHE_TTL) {
        if let Ok(bytes) = std::fs::read(&cache) {
            return Ok(bytes);
        }
    }
    match avm_plugin_sdk::fetch(DEFAULT_INDEX, 120) {
        Ok(bytes) => {
            if let Some(dir) = cache.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = avm_plugin_sdk::storage::atomic_write(&cache, &bytes);
            Ok(bytes)
        }
        // Offline: a stale index still beats no index.
        Err(e) => std::fs::read(&cache).map_err(|_| e),
    }
}

/// Parses the JSON-lines build index into versions mapped to their builds; errors on malformed lines and skips entries without a `+tag`.
fn parse_index(raw: &[u8]) -> Result<Index> {
    let mut index = Index::new();
    for line in raw.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let line: Line = serde_json::from_slice(line).context("failed to parse Python index")?;
        let Some((version, tag)) = line.version.split_once('+') else {
            continue;
        };
        // GA only: alphas/betas/rcs don't parse.
        let Some(triple) = parse_triple(version) else {
            continue;
        };
        for a in line.artifacts {
            let Some(pref) = VARIANTS.iter().position(|v| *v == a.variant) else {
                continue;
            };
            if a.archive_format != "tar.gz" {
                continue;
            }
            let rank = (tag.to_string(), pref == 0);
            let slot = index.entry(triple).or_default();
            if slot.get(&a.platform).is_none_or(|b| b.rank < rank) {
                let asset = LockedAsset {
                    sha256: a.sha256,
                    url: a.url,
                };
                slot.insert(a.platform, Build { rank, asset });
            }
        }
    }
    index.retain(|_, builds| !builds.is_empty());
    Ok(index)
}

/// Parses `major.minor.patch` into a triple, or `None` if it is not exactly three numbers.
fn parse_triple(version: &str) -> Option<Triple> {
    let mut parts = version.split('.').map(|p| p.parse().ok());
    let triple = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(triple)
}

/// Formats a version triple as `major.minor.patch`.
fn fmt(t: Triple) -> String {
    format!("{}.{}.{}", t.0, t.1, t.2)
}

/// Lists available Python versions, newest first, filtered by `query`; errors if the index cannot be fetched or parsed.
pub fn available_versions(query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
    let versions = parse_index(&index_bytes()?)?
        .into_keys()
        .rev()
        .map(|t| ToolVersion {
            version: fmt(t),
            label: fmt(t),
            channel: None,
            is_lts: false,
            is_security: false,
        })
        .collect();
    // Every CPython is 3.x, so the "major" a query selects is the minor line:
    // `avm python 13 versions` lists 3.13.*.
    Ok(query.filter(versions, |v| parse_triple(&v.version).map_or(0, |t| t.1)))
}

/// Maps an avm platform id to the python-build-standalone target triple; errors on unsupported platforms.
fn target_triple(platform: &str) -> Result<&'static str> {
    Ok(match platform {
        "darwin_arm64" => "aarch64-apple-darwin",
        "darwin_amd64" => "x86_64-apple-darwin",
        "linux_arm64" => "aarch64-unknown-linux-gnu",
        "linux_amd64" => "x86_64-unknown-linux-gnu",
        "windows_arm64" => "aarch64-pc-windows-msvc",
        "windows_amd64" => "x86_64-pc-windows-msvc",
        other => return Err(anyhow!("unsupported Python platform: {other}")),
    })
}

/// Returns the host platform as `<os>_<arch>` (e.g. `darwin_arm64`); errors on unsupported architectures.
pub fn host_platform() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => return Err(anyhow!("unsupported Python architecture: {other}")),
    };
    Ok(format!("{}_{arch}", std::env::consts::OS).replace("macos", "darwin"))
}

/// Resolves download URLs and checksums for `version` on each requested platform; errors on an unknown version or unsupported platform.
pub fn resolve_downloads(version: &str, platforms: &[String]) -> Result<ResolveResponse> {
    avm_plugin_sdk::download::validate_version(version)?;
    let index = parse_index(&index_bytes()?)?;
    let builds = parse_triple(version)
        .and_then(|t| index.get(&t))
        .ok_or_else(|| anyhow!("unknown Python version '{version}' (run `avm python versions`)"))?;
    let mut assets = BTreeMap::new();
    for platform in platforms {
        avm_plugin_sdk::download::validate_platform(platform)?;
        let build = builds
            .get(target_triple(platform)?)
            .ok_or_else(|| anyhow!("no Python {version} build for {platform}"))?;
        avm_plugin_sdk::download::validate_asset(&build.asset)?;
        assets.insert(platform.clone(), build.asset.clone());
    }
    Ok(ResolveResponse {
        version: version.to_string(),
        assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(version: &str, platform: &str, variant: &str, format: &str) -> String {
        let a = format!(
            r#"{{"platform":"{platform}","variant":"{variant}","url":"https://x/{version}-{variant}","archive_format":"{format}","sha256":"{}"}}"#,
            "a".repeat(64)
        );
        format!(r#"{{"version":"{version}","artifacts":[{a}]}}"#)
    }

    #[test]
    fn indexes_ga_installable_builds_preferring_newest_stripped() {
        let raw = [
            line(
                "3.12.3+20240415",
                "aarch64-apple-darwin",
                "install_only",
                "tar.gz",
            ),
            line(
                "3.12.3+20240415",
                "aarch64-apple-darwin",
                "debug+full",
                "tar.zst",
            ),
            line(
                "3.12.3+20250101",
                "aarch64-apple-darwin",
                "install_only",
                "tar.gz",
            ),
            line(
                "3.12.3+20250101",
                "aarch64-apple-darwin",
                "install_only_stripped",
                "tar.gz",
            ),
            line(
                "3.16.0a1+20250101",
                "aarch64-apple-darwin",
                "install_only",
                "tar.gz",
            ),
            line(
                "3.9.1+20210101",
                "aarch64-apple-darwin",
                "pgo+lto+full",
                "tar.zst",
            ),
        ]
        .join("\n");
        let index = parse_index(raw.as_bytes()).unwrap();
        assert_eq!(
            index.keys().copied().map(fmt).collect::<Vec<_>>(),
            ["3.12.3"]
        );
        let build = &index[&(3, 12, 3)]["aarch64-apple-darwin"];
        assert_eq!(
            build.asset.url,
            "https://x/3.12.3+20250101-install_only_stripped"
        );
    }

    #[test]
    fn query_by_minor_line() {
        let versions = ["3.13.2", "3.12.9", "3.13.1"].map(String::from).to_vec();
        let picked = ToolVersionQuery::Major(13)
            .filter(versions, |v: &String| parse_triple(v).map_or(0, |t| t.1));
        assert_eq!(picked, ["3.13.2", "3.13.1"]);
    }
}
