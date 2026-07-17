fn main() {
    if let Err(err) = load_dotenv_env() {
        eprintln!("avm: failed to load .env: {err}");
        std::process::exit(1);
    }
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("avm: {err}");
        std::process::exit(1);
    }
}

fn load_dotenv_env() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let protected: HashSet<String> = std::env::vars().map(|(key, _)| key).collect();

    for dir in ancestor_dirs(&cwd).into_iter().rev() {
        let env_file = dir.join(".env");
        if env_file.exists() {
            load_env_file(&env_file, &protected)?;
        }
    }

    Ok(())
}

fn ancestor_dirs(start: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        dirs.push(dir.clone());
        current = dir.parent().map(Path::to_path_buf);
    }
    dirs
}

fn load_env_file(path: &Path, protected: &HashSet<String>) -> Result<()> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read env file {}", path.display()))?;

    for (line_no, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            return Err(anyhow!(
                "invalid env assignment at {}:{}",
                path.display(),
                line_no + 1
            ));
        };

        let key = key.trim();
        if key.is_empty() {
            return Err(anyhow!(
                "invalid env key at {}:{}",
                path.display(),
                line_no + 1
            ));
        }
        if protected.contains(key) {
            continue;
        }

        std::env::set_var(key, parse_env_value(value.trim()));
    }

    Ok(())
}

fn parse_env_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Add(args) => cmd_add(args),
        Commands::Remove(args) => cmd_remove(args),
        Commands::List { global } => cmd_list(global),
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
    println!("  avm plugin add <path-or-url>     Install AVM or compatible asdf plugin");
    println!("  avm plugin list --all            List installed and available plugins");
    println!("  avm plugin remove <name>         Remove plugin");
    println!();
    println!("Plugin commands:");
    println!("  avm node versions                Pick from recent Node.js versions");
    println!("  avm node <major> versions        Pick from one major version line");
    println!("  avm node latest versions         Show latest Node.js version");
    println!("  avm node use <version>           Set local Node.js version");
    println!("  avm node use <version> --global  Set global Node.js version");
    println!("  avm java versions                Use after installing asdf-java");
    println!("  avm java latest versions         Show latest asdf-java version");
    println!("  avm java use <version>           Install if missing and set local Java version");
    println!();
    println!("Shell and shims:");
    println!("  avm shell-init                   Print shell integration");
    println!("  avm shims install                Install shims");
    println!("  avm shims path                   Print shim directory");
}
