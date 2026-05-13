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
        Commands::All => {
            print_grouped_help();
            Ok(())
        }
        Commands::PluginCommand(args) => cmd_plugin_command(args),
    }
}

fn print_grouped_help() {
    println!("avm command groups");
    println!();
    println!("Aliases:");
    println!("  avm init                         Create .avm.json");
    println!("  avm add <name> <command>         Add alias");
    println!("  avm remove <name>                Remove alias");
    println!("  avm list                         List config and aliases");
    println!("  avm run <name> [args...]         Run alias");
    println!("  avm resolve <name> [args...]     Print expanded alias command");
    println!("  avm which <name>                 Show alias/plugin origin");
    println!();
    println!("Plugins:");
    println!("  avm plugin available             Show installable plugins");
    println!("  avm plugin add node              Install built-in node plugin");
    println!("  avm plugin list --all            List installed and available plugins");
    println!("  avm plugin remove <name>         Remove plugin");
    println!();
    println!("Plugin commands:");
    println!("  avm node versions                Pick from recent Node.js versions");
    println!("  avm node <major> versions        Pick from one major version line");
    println!("  avm node latest versions         Show latest Node.js version");
    println!("  avm node use <version>           Set local Node.js version");
    println!("  avm node use <version> --global  Set global Node.js version");
    println!();
    println!("Shell and shims:");
    println!("  avm shell-init                   Print shell integration");
    println!("  avm shims install                Install shims");
    println!("  avm shims path                   Print shim directory");
}
