use anyhow::{anyhow, Context, Result};
use avm_plugin_api::ToolProvider;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct NodeProvider;

impl NodeProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn aliases_from_package_json(&self, cwd: &Path) -> Result<HashMap<String, NodeAlias>> {
        let package_json = cwd.join("package.json");
        if !package_json.exists() {
            return Ok(HashMap::new());
        }

        let raw = fs::read_to_string(&package_json).context("failed to read package.json")?;
        let parsed: Value = serde_json::from_str(&raw).context("failed to parse package.json")?;
        let scripts = parsed
            .get("scripts")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let run_prefix = detect_manager(cwd);

        let mut aliases = HashMap::new();
        for (name, value) in scripts {
            let Some(cmd) = value.as_str() else {
                continue;
            };
            let command = format!("{run_prefix} {name}");
            aliases.insert(
                name,
                NodeAlias {
                    command,
                    description: Some(cmd.to_string()),
                    manager: run_prefix.to_string(),
                },
            );
        }
        Ok(aliases)
    }

    pub fn bin_path_for(
        &self,
        version: &str,
        binary: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        let root = self
            .install_root()
            .map(|home| home.join(".avm").join("tools").join("node").join(version));
        let root = match root {
            Some(root) => root,
            None => return Ok(None),
        };

        let candidate = root.join("bin").join(binary_name(binary));
        if candidate.exists() {
            return Ok(Some(candidate));
        }

        if binary == "node" {
            let alt = root.join(format!("{}.exe", binary));
            if alt.exists() {
                return Ok(Some(alt));
            }
        }

        Ok(None)
    }

    pub fn available_versions(&self) -> Result<Vec<NodeVersion>> {
        let mirror = std::env::var("AVM_NODE_DIST_URL")
            .unwrap_or_else(|_| "https://nodejs.org/dist".to_string());
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

        let raw: Vec<NodeReleaseIndexEntry> = serde_json::from_slice(&output.stdout)
            .context("failed to parse Node.js version index")?;
        Ok(raw.into_iter().map(NodeVersion::from).collect())
    }

    fn install_root(&self) -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

impl ToolProvider for NodeProvider {
    fn name(&self) -> &str {
        "node"
    }

    fn is_installed(&self, version: &str) -> bool {
        self.bin_path_for(version, "node").ok().flatten().is_some()
    }

    fn installed_versions(&self) -> anyhow::Result<Vec<String>> {
        let root = match self.install_root() {
            Some(root) => root.join(".avm").join("tools").join("node"),
            None => return Ok(Vec::new()),
        };

        let mut versions = Vec::new();
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };

        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    versions.push(name.to_string());
                }
            }
        }

        versions.sort_unstable();
        Ok(versions)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.bin_path_for(version, "node")
    }

    fn install(&self, _version: &str) -> anyhow::Result<()> {
        Err(anyhow!(
            "node install is not supported in this baseline; avm will not auto-install while resolving binaries",
        ))
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        let root = self
            .install_root()
            .ok_or_else(|| anyhow!("HOME not set"))?;
        let path = root.join(".avm").join("tools").join("node").join(version);
        if path.exists() {
            fs::remove_dir_all(path).context("failed to remove managed node version")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NodeAlias {
    pub command: String,
    pub description: Option<String>,
    pub manager: String,
}

#[derive(Debug, Clone)]
pub struct NodeVersion {
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

impl From<NodeReleaseIndexEntry> for NodeVersion {
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

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn detect_manager(cwd: &Path) -> String {
    if cwd.join("bun.lockb").exists() || cwd.join("bun.lock").exists() {
        return "bun run".to_string();
    }
    if cwd.join("pnpm-lock.yaml").exists() {
        return "pnpm run".to_string();
    }
    if cwd.join("yarn.lock").exists() {
        return "yarn".to_string();
    }
    "npm run".to_string()
}
