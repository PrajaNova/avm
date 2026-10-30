//! Plugin manager: `avm plugin ...` (short form `avm -p ...`).
use crate::common::manager::Manager;
use crate::run::task::Cmd;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum PluginManager {
    /// Install tools and plugins, uninstall or search plugins (short form: `avm -p ...`).
    #[command(short_flag = 'p')]
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Install plugins (a first-party name, `owner/repo`, a git URL or a path; re-run to
    /// update), or with no names every selected tool without changing its pin.
    #[command(short_flag = 'i')]
    Install {
        sources: Vec<String>,
        /// Require an up-to-date lockfile with downloads for this platform.
        #[arg(long, conflicts_with = "sources")]
        frozen: bool,
    },
    /// Remove installed plugins.
    Uninstall { names: Vec<String> },
    /// List tools with their active and installed versions, and plugin aliases.
    #[command(alias = "ls")]
    List,
    /// Search first-party plugins and GitHub repos tagged `avm-plugin`.
    Search { keyword: String },
}

impl Manager for PluginManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Plugin { command } => command.into_command(),
        }
    }
}
