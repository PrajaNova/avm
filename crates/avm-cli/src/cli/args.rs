#[derive(Parser)]
#[command(name = "avm", version, about = "Any Version Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Add(AddArgs),
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    #[command(alias = "ls")]
    List,
    Which {
        key: String,
    },
    Env(EnvArgs),
    Resolve(ResolveArgs),
    Run(RunArgs),
    #[command(alias = "tools")]
    Tool {
        #[command(subcommand)]
        command: Option<ToolCommands>,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    ShellInit,
    Shims {
        #[command(subcommand)]
        command: ShimsCommands,
    },
    #[command(name = "exec-shim")]
    ExecShim(ExecShimArgs),
    Version,
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
    Add {
        source: String,
    },
    List {
        #[arg(short, long)]
        all: bool,
    },
    #[command(alias = "all", alias = "marketplace")]
    Available,
    Remove {
        name: String,
    },
    Update {
        #[arg(short, long)]
        all: bool,
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum ShimsCommands {
    Install,
    Remove {
        tool: String,
    },
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
struct EnvArgs {
    #[arg(short, long, default_value = "export")]
    format: String,
}

#[derive(Args)]
struct ResolveArgs {
    key: String,
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct RunArgs {
    #[arg(required = true, trailing_var_arg = true)]
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
