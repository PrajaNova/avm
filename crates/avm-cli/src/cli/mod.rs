mod alias_commands;
mod app;
mod args;
mod config_commands;
mod plugin_commands;
mod provider_commands;
mod shell;
mod shim_commands;
mod state;

pub use app::main;
pub use shell::sh_quote;

use alias_commands::*;
use args::*;
use config_commands::*;
use plugin_commands::*;
use provider_commands::*;
use shell::*;
use shim_commands::*;
use state::*;

use crate::config::{self, ConfigLoadResult, CONFIG_FILE};
use crate::resolver::{AliasSource, ResolvedConfig};
use crate::runtime::{self, PluginManager};
use crate::shims;
use crate::ui;
use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{ResolvedAlias, ToolProvider, ToolVersionQuery};
use clap::{Args, Parser, Subcommand};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
