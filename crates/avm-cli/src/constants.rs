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
pub const REGISTRY_URL: &str = "https://raw.githubusercontent.com/PrajaNova/avm/main/registry.json";
/// A repo that ships several plugins tags each release `avm-plugin-<name>-v<version>`.
pub const PLUGIN_TAG_PREFIX: &str = "avm-plugin-";

pub const PLUGIN_PROCESS_READ_TIMEOUT_MS: u64 = 30_000;
pub const PLUGIN_PROCESS_INSTALL_TIMEOUT_MS: u64 = 1_800_000;

/// User-facing console and error text, one file per feature.
pub mod msg;
