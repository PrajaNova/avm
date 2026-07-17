use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(rename = "api_version")]
    pub api_version: Option<u32>,
    pub description: Option<String>,
    pub section_label: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasDetail {
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AliasValue {
    Simple(String),
    Detailed(AliasDetail),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub api_version: Option<u32>,
    pub aliases: HashMap<String, AliasValue>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    pub command: String,
    pub description: Option<String>,
    pub plugin_name: String,
    pub section_name: String,
    pub source: Option<String>,
}

impl From<(&str, AliasValue, &Manifest)> for ResolvedAlias {
    fn from((plugin_name, value, manifest): (&str, AliasValue, &Manifest)) -> Self {
        match value {
            AliasValue::Simple(command) => Self {
                command,
                description: None,
                plugin_name: plugin_name.to_string(),
                section_name: manifest
                    .section_label
                    .clone()
                    .unwrap_or_else(|| plugin_name.to_string()),
                source: Some("script".to_string()),
            },
            AliasValue::Detailed(detail) => Self {
                command: detail.command,
                description: detail.description,
                plugin_name: plugin_name.to_string(),
                section_name: manifest
                    .section_label
                    .clone()
                    .unwrap_or_else(|| plugin_name.to_string()),
                source: detail.source,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolResolvedPath {
    pub path: PathBuf,
    pub version: String,
}

#[derive(Debug, Clone)]
pub enum ToolVersionQuery {
    Recent,
    Latest,
    Major(u64),
}

#[derive(Debug, Clone)]
pub struct ToolVersion {
    pub version: String,
    pub label: String,
    pub channel: Option<String>,
    pub is_lts: bool,
    pub is_security: bool,
}

pub trait ToolProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_installed(&self, version: &str) -> bool;
    fn installed_versions(&self) -> anyhow::Result<Vec<String>>;
    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>>;
    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>>;
    fn install(&self, version: &str) -> anyhow::Result<()>;
    fn uninstall(&self, version: &str) -> anyhow::Result<()>;
}
