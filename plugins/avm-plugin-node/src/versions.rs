use anyhow::{Context, Result};
use avm_plugin_api::{ToolVersion, ToolVersionQuery};
use serde_json::Value;

#[derive(serde::Deserialize)]
struct IndexEntry {
    version: String,
    #[serde(default)]
    lts: Value,
    #[serde(default)]
    security: bool,
}

pub fn available_versions(query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
    let mirror = crate::install::mirror();
    let raw = avm_plugin_api::fetch(&format!("{}/index.json", mirror.trim_end_matches('/')), 20)?;
    let index: Vec<IndexEntry> = serde_json::from_slice(&raw).context("failed to parse Node.js version index")?;
    let versions = index.into_iter().map(to_tool_version).collect();
    Ok(query.filter(versions, |v: &ToolVersion| major(&v.version)))
}

fn to_tool_version(entry: IndexEntry) -> ToolVersion {
    let version = entry.version.trim_start_matches('v').to_string();
    let lts = entry.lts.as_str().map(str::to_string);
    let mut label = version.clone();
    if let Some(name) = &lts {
        label.push_str(&format!("  LTS {name}"));
    }
    if entry.security {
        label.push_str("  security");
    }
    ToolVersion {
        version,
        label,
        is_lts: lts.is_some(),
        channel: lts,
        is_security: entry.security,
    }
}

fn major(version: &str) -> u64 {
    version.split('.').next().and_then(|m| m.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_major_filter() {
        let entry: IndexEntry =
            serde_json::from_str(r#"{"version":"v20.1.0","lts":"Iron","security":true}"#).unwrap();
        let v = to_tool_version(entry);
        assert_eq!((v.version.as_str(), v.label.as_str()), ("20.1.0", "20.1.0  LTS Iron  security"));
        assert!(v.is_lts && v.is_security);
        let entry: IndexEntry = serde_json::from_str(r#"{"version":"v21.0.0","lts":false}"#).unwrap();
        let versions = vec![to_tool_version(entry), v];
        let picked = ToolVersionQuery::Major(20).filter(versions, |v| major(&v.version));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].label, "20.1.0  LTS Iron  security");
    }
}
