//! Env manager: `avm env ...` and the hidden `avm ea` shortcut.
use crate::common::manager::Manager;
use crate::common::shell::Shell;
use crate::env::{cmd_env, cmd_env_add};
use crate::run::task::{Cmd, Task};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum EnvManager {
    /// Print shell export statements for merged env and PATH (default), or
    /// manage custom env vars — same shape as `avm plugin` (add/remove/list).
    Env {
        #[command(subcommand)]
        command: Option<EnvCommands>,
        /// Output syntax for the export list (default: sh on Unix, pwsh on Windows).
        #[arg(long, value_enum)]
        shell: Option<Shell>,
    },
    /// Shortcut for `avm env add <key> <value>`.
    #[command(hide = true)]
    Ea {
        key: String,
        value: String,
        #[arg(short = 'g', long)]
        global: bool,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    /// Add a custom env var to local or global config.
    Add {
        key: String,
        value: String,
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Remove a custom env var from local or global config.
    #[command(alias = "rm")]
    Remove {
        key: String,
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// List configured (merged) env vars — not provider-contributed ones
    /// like `ANDROID_HOME`; use bare `avm env` for the full export list.
    #[command(alias = "ls")]
    List,
}

impl Manager for EnvManager {
    fn into_command(self) -> Cmd {
        match self {
            // The export list feeds a shell, so no update notice.
            Self::Env { command, shell } => {
                Task::quiet(move || cmd_env(command, shell.unwrap_or_default()))
            }
            Self::Ea { key, value, global } => Task::cmd(move || cmd_env_add(key, value, global)),
        }
    }
}
