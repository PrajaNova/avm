//! What every feature manager provides, and the reserved-word check built from the CLI definition.
use crate::args::Cli;
use crate::run::task::Cmd;
use clap::CommandFactory;

/// A feature's commands, ready to run.
pub trait Manager {
    /// Converts the parsed commands into the command to run, consuming them.
    fn into_command(self) -> Cmd;
}

/// Whether `name` is a built-in command word (or alias of one), so an alias cannot use it.
/// Derived from the clap definition, so a new command is reserved without extra work.
pub fn is_reserved(name: &str) -> bool {
    // clap adds `help` only when it builds the command, so it is not a listed subcommand.
    name == "help"
        || Cli::command()
            .get_subcommands()
            .any(|c| c.get_name() == name || c.get_all_aliases().any(|a| a == name))
}

#[cfg(test)]
mod tests {
    use super::is_reserved;

    #[test]
    fn built_in_words_are_reserved() {
        for word in ["alias", "plugin", "env", "run", "x", "help", "self-update"] {
            assert!(is_reserved(word), "{word}");
        }
        assert!(!is_reserved("start"));
        assert!(!is_reserved("list"));
    }
}
