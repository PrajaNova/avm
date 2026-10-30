//! Global manager, the wildcard: lookups across every layer (`which`, `resolve`,
//! `run`, `outdated`, `upgrade`) and `avm <name> ...` for anything no other
//! manager claims. That is an alias first, then an installed plugin's command.
use crate::args::{Cli, Commands};
use crate::common::manager::Manager;
use crate::global::{cmd_outdated, cmd_resolve, cmd_run, cmd_upgrade, cmd_which};
use crate::run::task::{Cmd, Task};
use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};

#[derive(Subcommand)]
pub enum GlobalManager {
    /// Show where an alias or version selection comes from.
    Which { key: String },
    /// Print the command that an alias expands to.
    Resolve(ResolveArgs),
    /// Run an alias with optional arguments.
    Run(RunArgs),
    /// Show pinned tools with newer versions (in range and overall).
    Outdated {
        #[arg(long)]
        json: bool,
    },
    /// Install the newest version within each pin's range and update .avm.json pins.
    Upgrade {
        /// Only these tools (default: every pinned tool).
        tools: Vec<String>,
        /// Move to the latest release even across a major version.
        #[arg(long)]
        bump: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

// `disable_help_flag`: these forward trailing args verbatim to whatever the
// key resolves to, which can legitimately include `--help`/`-h` meant for
// the *target* (an alias command, or `avm <tool> --help`) — without this,
// clap's auto-inserted `--help` intercepts it here instead, always exiting
// 0 regardless of whether `key` is valid. The shell wrapper's `avm` function
// relies on `resolve`'s exit code to tell a real alias from a plugin/tool
// subcommand name, so a `--help` anywhere in the args must not silently
// force that check to succeed.
#[derive(Args)]
#[command(disable_help_flag = true)]
pub struct ResolveArgs {
    pub key: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
#[command(disable_help_flag = true)]
pub struct RunArgs {
    pub key: String,
    /// Temporary tool version; repeat for multiple tools, before alias arguments.
    #[arg(long = "with")]
    pub tools: Vec<String>,
    /// Fail if a requested tool version is not installed.
    #[arg(long)]
    pub no_install: bool,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl Manager for GlobalManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Which { key } => Task::cmd(move || cmd_which(&key)),
            Self::Resolve(args) => Task::quiet(move || cmd_resolve(args)),
            Self::Run(args) => Task::quiet(move || cmd_run(args)),
            Self::Outdated { json } => Task::cmd(move || cmd_outdated(json)),
            Self::Upgrade {
                tools,
                bump,
                dry_run,
                yes,
            } => Task::cmd(move || cmd_upgrade(tools, bump, dry_run, yes)),
        }
    }
}

/// The wildcard: runs `avm <name> ...` as `avm-bin run <name> ...` (the shell
/// wrapper's path too), by re-parsing; errors if it does not parse to a run command.
pub fn run_external(args: Vec<String>) -> Result<()> {
    let argv = ["avm-bin", "run"].into_iter().map(String::from).chain(args);
    match Cli::try_parse_from(argv)?.command {
        Some(Commands::Global(GlobalManager::Run(run_args))) => cmd_run(run_args),
        _ => Err(anyhow!("invalid alias invocation")),
    }
}
