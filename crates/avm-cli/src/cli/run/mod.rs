mod dispatch;
pub mod task;
mod wrappers;

use crate::cli::args::{Cli, ExecShimArgs};
use crate::cli::bootstrap::bootstrap;
use crate::cli::commands::cmd_exec_shim;
use crate::cli::run::dispatch::build;
use crate::cli::run::task::{Cmd, Runnable};
use crate::cli::run::wrappers::{WithProfiles, WithUpdateNotice};
use crate::shims;
use anyhow::Result;
use clap::Parser;

/// CLI entry point: loads `.env`, handles Windows shim invocations, parses arguments and runs the command, exiting non-zero on error.
pub fn main() {
    if let Err(err) = bootstrap() {
        eprintln!("avm: failed to load .env: {err}");
        std::process::exit(1);
    }
    // Windows shims are avm-bin started as `node.exe`, `npm.exe`, ...
    if let Some(tool) = shims::invoked_as_shim() {
        let args = std::env::args().skip(1).collect();
        exit_on_error(cmd_exec_shim(ExecShimArgs { tool, args }));
        return;
    }
    let cli = Cli::parse();
    // Behavior is layered on, innermost first: the command, then the update
    // notice (when the command wants one), then the `--env` profile check.
    let command = build(cli.command);
    let command: Cmd = Box::new(WithUpdateNotice::new(command));
    exit_on_error(Box::new(WithProfiles::new(command, cli.profiles)).run());
}

/// Prints the error to stderr and exits with status 1; does nothing on `Ok`.
fn exit_on_error(result: Result<()>) {
    if let Err(err) = result {
        eprintln!("avm: {err:#}");
        std::process::exit(1);
    }
}
