pub(crate) mod dispatch;
pub mod task;
mod wrappers;

use crate::args::Cli;
use crate::bootstrap::bootstrap;
use crate::run::dispatch::build;
use crate::run::task::{Cmd, Runnable};
use crate::run::wrappers::{WithProfiles, WithUpdateNotice};
use crate::shims;
use crate::shims::cmd_exec_shim;
use crate::shims::ExecShimArgs;
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
    let cli = Cli::parse_from(expand_alias_flag(std::env::args()));
    // Behavior is layered on, innermost first: the command, then the update
    // notice (when the command wants one), then the `--env` profile check.
    let command = build(cli.command);
    let command: Cmd = Box::new(WithUpdateNotice::new(command));
    exit_on_error(Box::new(WithProfiles::new(command, cli.profiles)).run());
}

/**
 * Rewrites a leading `-al` to `alias`, since clap short flags are one character.
 * Only the first argument counts, so `-al` stays a plain value anywhere else.
 */
fn expand_alias_flag(args: impl Iterator<Item = String>) -> Vec<String> {
    args.enumerate()
        .map(|(i, arg)| {
            if i == 1 && arg == "-al" {
                "alias".into()
            } else {
                arg
            }
        })
        .collect()
}

/// Prints the error to stderr and exits with status 1; does nothing on `Ok`.
fn exit_on_error(result: Result<()>) {
    if let Err(err) = result {
        eprintln!("avm: {err:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::expand_alias_flag;

    fn expand(args: &[&str]) -> Vec<String> {
        expand_alias_flag(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn leading_al_becomes_alias() {
        assert_eq!(
            expand(&["avm", "-al", "add", "x"]),
            ["avm", "alias", "add", "x"]
        );
    }

    #[test]
    fn al_elsewhere_is_left_alone() {
        assert_eq!(expand(&["avm", "run", "-al"]), ["avm", "run", "-al"]);
    }
}
