use crate::cli::args::{Cli, Commands};
use crate::cli::commands::{
    cmd_add, cmd_alias, cmd_env, cmd_env_add, cmd_exec, cmd_exec_shim, cmd_home, cmd_init,
    cmd_list, cmd_lock, cmd_outdated, cmd_prune, cmd_remove, cmd_resolve, cmd_run, cmd_shims,
    cmd_trust, cmd_upgrade, cmd_which,
};
use crate::cli::run::task::{Cmd, Task};
use crate::cli::shell::{pwsh_init_script, shell_init_script, Shell};
use crate::constants::msg;
use anyhow::{anyhow, Result};
use clap::Parser;

/// Builds the command to run from what was parsed. Each arm picks the type
/// that handles it; `Task::quiet` marks commands whose output feeds a shell.
pub fn build(command: Option<Commands>) -> Cmd {
    let Some(command) = command else {
        return Task::cmd(cmd_home);
    };
    match command {
        Commands::Init => Task::cmd(cmd_init),
        Commands::Plugin { command } => command.into_command(),
        Commands::Lock { platform } => Task::cmd(move || cmd_lock(platform)),
        Commands::Add(args) => Task::cmd(move || cmd_add(args)),
        Commands::Remove(args) => Task::cmd(move || cmd_remove(args)),
        Commands::List => Task::cmd(cmd_list),
        Commands::Which { key } => Task::cmd(move || cmd_which(&key)),
        Commands::Alias { command } => Task::cmd(move || cmd_alias(command)),
        Commands::Env { command, shell } => {
            Task::quiet(move || cmd_env(command, shell.unwrap_or_default()))
        }
        Commands::Resolve(args) => Task::quiet(move || cmd_resolve(args)),
        Commands::Run(args) => Task::quiet(move || cmd_run(args)),
        Commands::Exec(args) => Task::quiet(move || cmd_exec(args)),
        Commands::Create { name } => Task::cmd(move || {
            println!("{}", msg::PLUGIN_TEMPLATE_INTRO);
            println!(
                "{}",
                msg::fill(msg::PLUGIN_TEMPLATE_CMD, &[("name", &name)])
            );
            Ok(())
        }),
        Commands::ShellInit { shell } => Task::quiet(move || shell_init(shell.unwrap_or_default())),
        Commands::Shims { command } => Task::cmd(move || cmd_shims(command)),
        Commands::ExecShim(args) => Task::quiet(move || cmd_exec_shim(args)),
        // Same path as the shell wrapper's `avm-bin run <key> ...`: alias first.
        Commands::External(args) => Task::quiet(move || run_external(args)),
        Commands::Ea { key, value, global } => Task::cmd(move || cmd_env_add(key, value, global)),
        Commands::Trust(args) => Task::cmd(move || cmd_trust(args)),
        Commands::SelfUpdate { version } => {
            Task::quiet(move || crate::update::self_update(version))
        }
        Commands::UpdateCheck => Task::quiet(crate::update::refresh_cache),
        Commands::Outdated { json } => Task::cmd(move || cmd_outdated(json)),
        Commands::Upgrade {
            tools,
            bump,
            dry_run,
            yes,
        } => Task::cmd(move || cmd_upgrade(tools, bump, dry_run, yes)),
        Commands::Prune {
            older_than,
            include_unrecorded,
            dry_run,
            yes,
        } => Task::cmd(move || cmd_prune(older_than, include_unrecorded, dry_run, yes)),
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

/// Runs an alias given as a bare command by re-parsing it as `avm-bin run <args>`; errors if it does not parse to a run command.
fn run_external(args: Vec<String>) -> Result<()> {
    let argv = ["avm-bin", "run"].into_iter().map(String::from).chain(args);
    match Cli::try_parse_from(argv)?.command {
        Some(Commands::Run(run_args)) => cmd_run(run_args),
        _ => Err(anyhow!("invalid alias invocation")),
    }
}
