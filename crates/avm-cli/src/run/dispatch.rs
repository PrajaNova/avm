use crate::args::Commands;
use crate::common::manager::Manager;
use crate::global;
use crate::global::cmd_home;
use crate::run::task::{Cmd, Task};

/// Builds the command to run from what was parsed: each group goes to the
/// manager that owns it, and a bare `avm` opens the home picker.
pub fn build(command: Option<Commands>) -> Cmd {
    let Some(command) = command else {
        return Task::cmd(cmd_home);
    };
    match command {
        Commands::Alias(m) => m.into_command(),
        Commands::Plugin(m) => m.into_command(),
        Commands::Env(m) => m.into_command(),
        Commands::Shims(m) => m.into_command(),
        Commands::Project(m) => m.into_command(),
        Commands::Tool(m) => m.into_command(),
        Commands::Global(m) => m.into_command(),
        Commands::App(m) => m.into_command(),
        // Same path as the shell wrapper's `avm-bin run <key> ...`: alias first.
        Commands::External(args) => Task::quiet(move || global::run_external(args)),
    }
}
