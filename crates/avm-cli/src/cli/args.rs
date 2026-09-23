use super::*;

#[derive(Parser)]
#[command(
    name = "avm",
    version,
    about = "Any Version Manager",
    long_about = "Any Version Manager: aliases, plugin commands, runtime versions, and shims."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a local .avm.json config file.
    Init,
    /// Add an alias command to local or global config.
    #[command(alias = "aa")]
    Add(AddArgs),
    /// Remove an alias command from local or global config.
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// List aliases, env, selected versions, and plugin aliases.
    #[command(alias = "ls")]
    List,
    /// Show where an alias or version selection comes from.
    Which {
        key: String,
    },
    /// Manage alias commands — same shape as `avm plugin` (add/remove/list).
    /// `avm add`/`avm remove` above still work too, unchanged.
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },
    /// Print shell export statements for merged env and PATH (default), or
    /// manage custom env vars — same shape as `avm plugin` (add/remove/list).
    Env {
        #[command(subcommand)]
        command: Option<EnvCommands>,
    },
    /// Print the command that an alias expands to.
    Resolve(ResolveArgs),
    /// Run an alias with optional arguments.
    Run(RunArgs),
    /// Install, list, update, or remove avm plugins.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Show how to start a new avm plugin from the template repo.
    Create {
        name: String,
    },
    /// Print shell setup for avm aliases and shims.
    ShellInit,
    /// Manage executable shims used for plain commands like node and java.
    Shims {
        #[command(subcommand)]
        command: ShimsCommands,
    },
    /// Internal shim dispatch command.
    #[command(hide = true)]
    #[command(name = "exec-shim")]
    ExecShim(ExecShimArgs),
    /// Shortcut for `avm plugin add <source>`.
    #[command(hide = true)]
    Pa {
        source: String,
    },
    /// Shortcut for `avm env add <key> <value>`.
    #[command(hide = true)]
    Ea {
        key: String,
        value: String,
        #[arg(short = 'g', long)]
        global: bool,
    },
    /// Run an installed plugin command, for example `avm node versions` or `avm java versions`.
    #[command(external_subcommand)]
    PluginCommand(Vec<String>),
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Install a plugin by name, path, or URL.
    Add {
        source: String,
    },
    /// List installed plugins.
    List {
        #[arg(short, long)]
        all: bool,
    },
    /// Show plugins available to install.
    #[command(alias = "all", alias = "marketplace")]
    Available,
    /// Remove an installed plugin.
    Remove {
        name: String,
    },
    /// Update one plugin or all plugins.
    Update {
        #[arg(short, long)]
        all: bool,
        name: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ShimsCommands {
    /// (Re)generate shims in ~/.avm/shims for all installed versions, including global package binaries.
    #[command(alias = "reshim")]
    Install,
    /// Add ~/.avm/shims to PATH in shell startup files so it works everywhere, incl. closed envs.
    Activate,
    /// Remove one shim.
    Remove {
        tool: String,
    },
    /// Print the shim directory path.
    Path,
}

#[derive(Args)]
pub struct AddArgs {
    pub key: String,
    pub value: Vec<String>,
    #[arg(short = 'g', long)]
    pub global: bool,
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
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Args)]
pub struct ExecShimArgs {
    pub tool: String,
    #[arg(last = true)]
    pub args: Vec<String>,
}
