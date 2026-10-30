//! Tool manager: running commands with specific tool versions (`exec`).
use crate::common::manager::Manager;
use crate::run::task::{Cmd, Task};
use crate::tool::cmd_exec;
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ToolManager {
    /// Run a command with temporary tool versions, without changing pins.
    #[command(alias = "x")]
    Exec(ExecArgs),
}

#[derive(Args)]
pub struct ExecArgs {
    /// Tool specifications, e.g. node@18 java@21.
    #[arg(required = true)]
    pub tools: Vec<String>,
    /// Fail if a requested tool version is not installed.
    #[arg(long)]
    pub no_install: bool,
    /// Command and arguments after --, forwarded verbatim.
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

impl Manager for ToolManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Exec(args) => Task::quiet(move || cmd_exec(args)),
        }
    }
}
