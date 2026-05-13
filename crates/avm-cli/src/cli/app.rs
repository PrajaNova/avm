fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("avm: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Add(args) => cmd_add(args),
        Commands::Remove(args) => cmd_remove(args),
        Commands::List => cmd_list(),
        Commands::Which { key } => cmd_which(&key),
        Commands::Env(args) => cmd_env(args),
        Commands::Resolve(args) => cmd_resolve(args),
        Commands::Run(args) => cmd_run(args),
        Commands::Tool { command } => cmd_tool(command),
        Commands::Plugin { command } => cmd_plugin(command),
        Commands::ShellInit => {
            println!("{}", shell_init_script());
            Ok(())
        }
        Commands::Shims { command } => cmd_shims(command),
        Commands::ExecShim(args) => cmd_exec_shim(args),
        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::PluginCommand(args) => cmd_plugin_command(args),
    }
}
