mod aliases;
mod install;
mod versions;

use anyhow::{anyhow, Context};
use avm_plugin_api::{ToolProvider, ToolVersion, ToolVersionQuery};
use std::fs;
use std::path::{Path, PathBuf};

pub use aliases::NodeAlias;

#[derive(Debug)]
pub struct NodeProvider;

impl NodeProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn aliases_from_package_json(
        &self,
        cwd: &Path,
    ) -> anyhow::Result<std::collections::HashMap<String, NodeAlias>> {
        aliases::aliases_from_package_json(cwd)
    }

    pub fn bin_path_for(&self, version: &str, binary: &str) -> anyhow::Result<Option<PathBuf>> {
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

    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>> {
        versions::available_versions(query)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.bin_path_for(version, "node")
    }

    fn install(&self, _version: &str) -> anyhow::Result<()> {
        install::install_node(_version)
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        let root = self.install_root().ok_or_else(|| anyhow!("HOME not set"))?;
        let path = root.join(".avm").join("tools").join("node").join(version);
        if path.exists() {
            fs::remove_dir_all(path).context("failed to remove managed node version")?;
        }
        Ok(())
    }
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
