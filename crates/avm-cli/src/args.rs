use crate::alias::AliasManager;
use crate::app::AppManager;
use crate::env::EnvManager;
use crate::global::GlobalManager;
use crate::plugins::PluginManager;
use crate::project::ProjectManager;
use crate::shims::ShimsManager;
use crate::tool::ToolManager;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "avm",
    version,
    // `-v`/`--version` like most CLIs, instead of clap's default `-V`.
    disable_version_flag = true,
    about = "Any Version Manager",
    long_about = "Any Version Manager: aliases, plugin commands, runtime versions, and shims."
)]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    pub version: Option<bool>,

    /// Config profiles, comma-separated; overrides AVM_ENV.
    #[arg(long = "env", global = true)]
    pub profiles: Option<String>,

    /// Without a command, a terminal gets a picker of this project's aliases and tools.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Every top-level command, grouped by the manager that owns it (see `managers`).
#[derive(Subcommand)]
pub enum Commands {
    #[command(flatten)]
    Alias(AliasManager),
    #[command(flatten)]
    Plugin(PluginManager),
    #[command(flatten)]
    Env(EnvManager),
    #[command(flatten)]
    Shims(ShimsManager),
    #[command(flatten)]
    Project(ProjectManager),
    #[command(flatten)]
    Tool(ToolManager),
    #[command(flatten)]
    Global(GlobalManager),
    #[command(flatten)]
    App(AppManager),
    /// Anything else is the global manager's wildcard: an alias, or a plugin command like `avm node versions`.
    #[command(external_subcommand)]
    External(Vec<String>),
}
