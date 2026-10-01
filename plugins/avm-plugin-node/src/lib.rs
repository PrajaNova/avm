mod aliases;
mod install;
mod versions;

use avm_plugin_api::{tool_dir, ToolProvider, ToolVersion, ToolVersionQuery};
use std::collections::HashMap;
use std::path::PathBuf;

pub use aliases::aliases_from_package_json;

/// `bin/node` (or `bin\\node.exe`) inside each installed version.
pub(crate) const NODE_BIN: &str = if cfg!(windows) { "node.exe" } else { "node" };

#[derive(Debug)]
pub struct NodeProvider;

impl NodeProvider {
    pub fn bin_path_for(&self, version: &str, binary: &str) -> anyhow::Result<Option<PathBuf>> {
        let candidate = tool_dir("node")?.join(version).join("bin").join(binary);
        Ok(candidate.exists().then_some(candidate))
    }
}

impl ToolProvider for NodeProvider {
    fn name(&self) -> &str {
        "node"
    }

    fn is_installed(&self, version: &str) -> bool {
        self.bin_path_for(version, NODE_BIN).ok().flatten().is_some()
    }

    fn installed_versions(&self) -> anyhow::Result<Vec<String>> {
        avm_plugin_api::list_installed("node", |v| self.is_installed(v))
    }

    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>> {
        versions::available_versions(query)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.bin_path_for(version, NODE_BIN)
    }

    /// Windows: a user or global npmrc prefix (nvm-windows, CI images'
    /// `C:\npm\prefix`) outranks the builtin one set at install, so pin npm's
    /// global prefix to this version for everything avm runs.
    fn env_vars(&self, version: &str) -> anyhow::Result<HashMap<String, String>> {
        let mut env = HashMap::new();
        if cfg!(windows) {
            let bin = tool_dir("node")?.join(version).join("bin");
            env.insert("npm_config_prefix".to_string(), bin.display().to_string());
        }
        Ok(env)
    }

    fn install(&self, version: &str) -> anyhow::Result<()> {
        install::install_node(version)
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        avm_plugin_api::remove_version("node", version)
    }
}
