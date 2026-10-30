//! Plugins: routing, discovery, discovers, installs, updates and talks to plugins.
//! Tiers: protocol executables (`process`), then the legacy asdf adapter (`asdf`).
mod asdf;
mod discovery;
mod marketplace;
mod process;
mod route;
mod timeout;

use crate::constants::*;
use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{
    env_timeout_ms, fetch, fetch_with_headers, list_installed, remove_version, run_timed,
    sha256_file, tool_dir, verify_sha256, wait_deadline, Manifest, ToolProvider, ToolVersion,
    ToolVersionQuery,
};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub use asdf::AsdfToolProvider;
pub use discovery::PluginManager;
use discovery::*;
pub use marketplace::*;
pub use process::PluginProcess;
pub use route::{is_installed_plugin, route, Route};
use timeout::*;
