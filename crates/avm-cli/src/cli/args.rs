#[derive(Parser)]
#[command(
    name = "avm",
    version,
    about = "Any Version Manager",
    long_about = "Any Version Manager: aliases, plugin commands, runtime versions, and shims."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a local .avm.json config file.
    Init,
    /// Add an alias command to local or global config.
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
    /// Print shell export statements for merged env and PATH.
    Env(EnvArgs),
    /// Print the command that an alias expands to.
    Resolve(ResolveArgs),
    /// Run an alias with optional arguments.
    Run(RunArgs),
    /// Compatibility command for older scripts. Prefer `avm <plugin> ...`.
    #[command(hide = true)]
    #[command(alias = "tools")]
    Tool {
        #[command(subcommand)]
        command: Option<ToolCommands>,
    },
    /// Install, list, update, or remove avm plugins.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Scaffold a new avm plugin project, ready to build and publish.
    Create(CreateArgs),
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
    /// Print avm version.
    Version,
    /// Show grouped command help.
    All,
    /// Run an installed plugin command, for example `avm node versions` or `avm java versions`.
    #[command(external_subcommand)]
    PluginCommand(Vec<String>),
}

#[derive(Subcommand)]
enum ToolCommands {
    #[command(alias = "ls")]
    List,
    Use(ToolUseArgs),
    Install(ToolInstallArgs),
    Uninstall(ToolUninstallArgs),
    #[command(external_subcommand)]
    Provider(Vec<String>),
}

#[derive(Subcommand)]
enum PluginCommands {
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
enum ShimsCommands {
    /// Install avm shims into ~/.avm/shims.
    Install,
    /// Regenerate shims for all installed versions, including global package binaries.
    Reshim,
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
struct AddArgs {
    key: String,
    value: Vec<String>,
    #[arg(short = 'g', long)]
    global: bool,
}

#[derive(Args)]
struct RemoveArgs {
    key: String,
    #[arg(short = 'g', long)]
    global: bool,
}

#[derive(Args)]
struct CreateArgs {
    /// Tool name, e.g. "kotlin" — scaffolds ./avm-plugin-kotlin
    name: String,
    /// Directory to create the plugin project in (default: current directory)
    #[arg(long)]
    path: Option<PathBuf>,
}

#[derive(Args)]
struct EnvArgs {
    #[arg(short, long, default_value = "export")]
    format: String,
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
struct ResolveArgs {
    key: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
#[command(disable_help_flag = true)]
struct RunArgs {
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct ToolUseArgs {
    tool: String,
    version: String,
    #[arg(short = 'g', long)]
    global: bool,
}

#[derive(Args)]
struct ToolInstallArgs {
    tool: String,
    version: String,
}
#[derive(Args)]
struct ToolUninstallArgs {
    tool: String,
    version: String,
}

#[derive(Args)]
struct ExecShimArgs {
    tool: String,
    #[arg(last = true)]
    args: Vec<String>,
}
