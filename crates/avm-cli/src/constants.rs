//! Shared constants: plugin discovery, timeouts and release sources.

pub const ASDF_LIST_TIMEOUT_MS: u64 = 20_000;
pub const ASDF_ENV_TIMEOUT_MS: u64 = 2_000;
pub const ASDF_INSTALL_TIMEOUT_MS: u64 = 120_000;
pub const GIT_CLONE_TIMEOUT_MS: u64 = 120_000;
pub const PROTOCOL_PREFIX: &str = "avm-plugin-";
pub use crate::platform::{ARCHIVE_EXT as PLUGIN_ARCHIVE_EXT, PLUGIN_BIN};
pub const ASDF_PREFIX: &str = "asdf-";

pub const MARKETPLACE_TIMEOUT_SECS: u32 = 20;
pub const MARKETPLACE_INSTALL_TIMEOUT_MS: u64 = 300_000;
pub const DEFAULT_PLUGIN_PATH_ENV: &str =
    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
/// Plugin registry: a list of plugins with their repo and description. `AVM_REGISTRY_URL` overrides it.
pub const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/PrajaNova/avm/main/registry.json";
/// A repo that ships several plugins tags each release `avm-plugin-<name>-v<version>`.
pub const PLUGIN_TAG_PREFIX: &str = "avm-plugin-";

pub const PLUGIN_PROCESS_READ_TIMEOUT_MS: u64 = 30_000;
pub const PLUGIN_PROCESS_INSTALL_TIMEOUT_MS: u64 = 1_800_000;

/// User-facing console text for plugin and alias commands.
pub mod msg {
    pub const PLUGIN_FETCHING: &str = "Fetching '{name}' from {repo}...";
    pub const PLUGIN_INSTALLED: &str = "✓ Installed {name} {version}";
    pub const PLUGIN_INSTALLING_FROM: &str = "Installing plugin from {source}...";
    pub const PLUGIN_INSTALLED_PLAIN: &str = "✓ Installed plugin";
    pub const PLUGIN_REMOVED: &str = "Plugin '{name}' removed.";
    pub const PLUGIN_SEARCH_EMPTY: &str = "No plugins match '{keyword}'.";
    pub const PLUGIN_SEARCH_HINT: &str = "Install with: avm plugin install <name>";
    pub const PLUGIN_NOT_INSTALLED: &str =
        "plugin '{name}' is not installed — run `avm plugin install {name}`";
    pub const PLUGIN_TEMPLATE_INTRO: &str = "Start a new plugin from the template repo:";
    pub const PLUGIN_TEMPLATE_CMD: &str = "  gh repo create avm-plugin-{name} --template PrajaNova/avm-plugin-template --public --clone";

    /// Fills `{key}` placeholders: `fill(msg::PLUGIN_REMOVED, &[("name", "node")])`.
    pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
        vars.iter().fold(template.to_string(), |out, (key, value)| {
            out.replace(&format!("{{{key}}}"), value)
        })
    }
}
