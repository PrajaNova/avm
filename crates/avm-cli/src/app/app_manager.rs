//! App manager: avm itself (`self-update`, shell setup, plugin template).
use crate::common::manager::Manager;
use crate::common::shell::{pwsh_init_script, shell_init_script, Shell};
use crate::constants::msg;
use crate::run::task::{Cmd, Task};
use anyhow::{anyhow, Result};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AppManager {
    /// Show how to start a new avm plugin from the template repo.
    Create { name: String },
    /// Print shell setup for avm aliases and shims (sh/bash/zsh, or pwsh).
    ShellInit {
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
    /// Update avm to the latest release (or --version X). Homebrew, npm,
    /// cargo and Scoop installs print their own upgrade command instead.
    SelfUpdate {
        #[arg(long)]
        version: Option<String>,
    },
    /// Internal: background refresh for the update notice.
    #[command(hide = true, name = "__update-check")]
    UpdateCheck,
}

impl Manager for AppManager {
    fn into_command(self) -> Cmd {
        match self {
            Self::Create { name } => Task::cmd(move || {
                println!("{}", msg::PLUGIN_TEMPLATE_INTRO);
                println!(
                    "{}",
                    msg::fill(msg::PLUGIN_TEMPLATE_CMD, &[("name", &name)])
                );
                Ok(())
            }),
            Self::ShellInit { shell } => Task::quiet(move || shell_init(shell.unwrap_or_default())),
            Self::SelfUpdate { version } => {
                Task::quiet(move || crate::update::self_update(version))
            }
            Self::UpdateCheck => Task::quiet(crate::update::refresh_cache),
        }
    }
}

/// Prints the shell integration script for `shell`; errors for `cmd`, which has no init hook.
fn shell_init(shell: Shell) -> Result<()> {
    match shell {
        Shell::Pwsh => println!("{}", pwsh_init_script()),
        Shell::Cmd => {
            return Err(anyhow!(
                "cmd has no init hook; run `avm-bin env --shell cmd` and use `avm-bin` directly"
            ))
        }
        Shell::Sh => println!("{}", shell_init_script()),
    }
    Ok(())
}
