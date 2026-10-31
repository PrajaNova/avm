//! Plugins: routing, discovery, discovers, installs, updates and talks to plugins.
//! Tiers: protocol executables (`process`), then the legacy asdf adapter (`asdf`).
mod asdf;
mod discovery;
mod marketplace;
mod plugin_handler;
mod plugin_list_handler;
mod plugin_manager;
mod process;
mod registry_service;
mod route;
mod timeout;

use crate::constants::msg::{self, fill};
use crate::constants::*;
use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::{
    env_timeout_ms, fetch, fetch_with_headers, list_installed, remove_version, run_timed,
    sha256_file, tool_dir, verify_sha256, wait_deadline, Manifest, ToolProvider, ToolVersion,
    ToolVersionQuery,
};
use fs_err as fs;
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub use asdf::AsdfToolProvider;
pub use discovery::PluginRegistry;
use discovery::*;
pub use marketplace::*;
#[allow(unused_imports)]
pub(crate) use plugin_handler::*;
#[allow(unused_imports)]
pub(crate) use plugin_list_handler::*;
pub use plugin_manager::*;
pub use process::PluginProcess;
use registry_service::*;
pub use route::{is_installed_plugin, route, Route};
use timeout::*;
