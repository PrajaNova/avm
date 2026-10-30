//! Project manager: config and install lifecycle (`init`, `trust`).
use crate::common::manager::Manager;
use crate::project::{cmd_init, cmd_trust};
use crate::run::task::{Cmd, Task};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ProjectManager {
    /// Create a local .avm.json config file.
    Init,
    /// Trust a project's .avm.json and .env so their aliases and env apply.
    Trust(TrustArgs),
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
            Self::Trust(args) => Task::cmd(move || cmd_trust(args)),
        }
    }
}
