use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{ToolVersion, ToolVersionQuery};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn available_versions(query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
    let versions = filter_node_versions(release_index()?, &query);
    Ok(versions.into_iter().map(ToolVersion::from).collect())
}

fn release_index() -> Result<Vec<NodeRelease>> {
    let mirror = std::env::var("AVM_NODE_DIST_URL")
        .unwrap_or_else(|_| "https://nodejs.org/dist".to_string());
    let local_index = Path::new(&mirror).join("index.json");
    if local_index.exists() {
        let raw = fs::read(&local_index).with_context(|| {
            format!(
                "failed to read Node.js versions from {}",
                local_index.display()
            )
        })?;
        let parsed: Vec<NodeReleaseIndexEntry> =
            serde_json::from_slice(&raw).context("failed to parse Node.js version index")?;
        return Ok(parsed.into_iter().map(NodeRelease::from).collect());
    }

    let url = format!("{}/index.json", mirror.trim_end_matches('/'));
    let output = Command::new("curl")
        .arg("-fsSL")
        .arg("--connect-timeout")
        .arg("5")
        .arg("--max-time")
        .arg("20")
        .arg(&url)
        .output()
        .with_context(|| format!("failed to fetch Node.js versions from {url}"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "failed to fetch Node.js versions from {url}: curl exited with {}",
            output.status
        ));
    }

    let raw: Vec<NodeReleaseIndexEntry> =
        serde_json::from_slice(&output.stdout).context("failed to parse Node.js version index")?;
    Ok(raw.into_iter().map(NodeRelease::from).collect())
}

#[derive(Debug, Clone)]
struct NodeRelease {
    pub version: String,
    pub lts: Option<String>,
    pub security: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct NodeReleaseIndexEntry {
    version: String,
    #[serde(default)]
    lts: Value,
    #[serde(default)]
    security: bool,
}

impl From<NodeReleaseIndexEntry> for NodeRelease {
    fn from(value: NodeReleaseIndexEntry) -> Self {
        let lts = match value.lts {
            Value::String(name) => Some(name),
            _ => None,
        };

        Self {
            version: value.version,
            lts,
            security: value.security,
        }
    }
}

impl From<NodeRelease> for ToolVersion {
    fn from(value: NodeRelease) -> Self {
        let clean_version = value.version.trim_start_matches('v').to_string();
        let label = match (&value.lts, value.security) {
            (Some(lts), true) => format!("{clean_version}  LTS {lts}  security"),
            (Some(lts), false) => format!("{clean_version}  LTS {lts}"),
            (None, true) => format!("{clean_version}  security"),
            (None, false) => clean_version.clone(),
        };

        Self {
            version: clean_version,
            label,
            channel: value.lts.clone(),
            is_lts: value.lts.is_some(),
            is_security: value.security,
        }
    }
}

fn filter_node_versions(versions: Vec<NodeRelease>, query: &ToolVersionQuery) -> Vec<NodeRelease> {
    match query {
        ToolVersionQuery::Recent => versions.into_iter().take(10).collect(),
        ToolVersionQuery::Latest => versions.into_iter().take(1).collect(),
        ToolVersionQuery::Major(major) => versions
            .into_iter()
            .filter(|version| node_major(&version.version) == Some(*major))
            .collect(),
    }
}

fn node_major(version: &str) -> Option<u64> {
    version
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
}
