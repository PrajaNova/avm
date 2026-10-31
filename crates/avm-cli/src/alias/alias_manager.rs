//! Alias manager: `avm alias ...` (short form `avm -al ...`).
use crate::alias::cmd_alias;
use crate::common::manager::Manager;
use crate::run::task::{Cmd, Task};
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum AliasManager {
    /// Manage alias commands (add/remove/list), same shape as `avm plugin`.
    /// Short form: `avm -al ...`.
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },
}

#[derive(Args)]
pub struct AddArgs {
    pub key: String,
    pub value: Vec<String>,
    #[arg(short = 'g', long)]
    pub global: bool,
    /// Overwrite an existing alias without asking.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args)]
pub struct RemoveArgs {
    pub key: String,
    #[arg(short = 'g', long)]
    pub global: bool,
}

#[derive(Subcommand)]
pub enum AliasCommands {
    /// Add an alias command to local or global config.
    Add(AddArgs),
    /// Remove an alias command from local or global config.
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// List configured aliases.
    #[command(alias = "ls")]
    List,
}

impl Manager for AliasManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Alias { command } => Task::cmd(move || cmd_alias(command)),
        }
    }
}
