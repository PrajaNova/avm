//! Shims manager: `avm shims ...` and the internal shim dispatch.
use crate::common::manager::Manager;
use crate::run::task::{Cmd, Task};
use crate::shims::{cmd_exec_shim, cmd_shims};
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ShimsManager {
    /// Manage executable shims used for plain commands like node and java.
    Shims {
        #[command(subcommand)]
        command: ShimsCommands,
    },
    /// Internal shim dispatch command.
    #[command(hide = true)]
    #[command(name = "exec-shim")]
    ExecShim(ExecShimArgs),
}

#[derive(Subcommand)]
pub enum ShimsCommands {
    /// (Re)generate shims in ~/.avm/shims for all installed versions, including global package binaries.
    #[command(alias = "reshim")]
    Install,
    /// Add ~/.avm/shims to PATH in shell startup files so it works everywhere, incl. closed envs.
    Activate,
    /// Remove one shim.
    Remove { tool: String },
    /// Print the shim directory path.
    Path,
}

#[derive(Args)]
pub struct ExecShimArgs {
    pub tool: String,
    #[arg(last = true)]
    pub args: Vec<String>,
}

impl Manager for ShimsManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Shims { command } => Task::cmd(move || cmd_shims(command)),
            Self::ExecShim(args) => Task::quiet(move || cmd_exec_shim(args)),
        }
    }
}
