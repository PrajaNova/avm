//! Project manager: config and install lifecycle (`init`, `lock`, `trust`, `prune`).
use crate::common::manager::Manager;
use crate::project::{cmd_init, cmd_lock, cmd_prune, cmd_trust};
use crate::run::task::{Cmd, Task};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ProjectManager {
    /// Create a local .avm.json config file.
    Init,
    /// Resolve selected tools and write a reproducible .avm.lock.
    Lock {
        /// Platforms to lock (default: this host), comma-separated.
        #[arg(long, value_delimiter = ',')]
        platform: Vec<String>,
    },
    /// Trust a project's .avm.json and .env so their aliases and env apply.
    Trust(TrustArgs),
    /// Remove installed versions no project or global pin uses.
    Prune {
        /// Only versions not used in this long, e.g. 90d.
        #[arg(long)]
        older_than: Option<String>,
        /// Also consider versions avm has never seen used (e.g. installed before avm recorded use).
        #[arg(long)]
        include_unrecorded: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Args)]
pub struct TrustArgs {
    /// Directory (or a specific .avm.json/.env file). Defaults to the current directory.
    pub path: Option<PathBuf>,
    /// Remove trust instead.
    #[arg(long, conflicts_with = "list")]
    pub revoke: bool,
    /// List trusted files.
    #[arg(long)]
    pub list: bool,
}

impl Manager for ProjectManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Init => Task::cmd(cmd_init),
            Self::Lock { platform } => Task::cmd(move || cmd_lock(platform)),
            Self::Trust(args) => Task::cmd(move || cmd_trust(args)),
            Self::Prune {
                older_than,
                include_unrecorded,
                dry_run,
                yes,
            } => Task::cmd(move || cmd_prune(older_than, include_unrecorded, dry_run, yes)),
        }
    }
}
