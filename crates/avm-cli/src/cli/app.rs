use super::*;

pub fn main() {
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

    let dirs: Vec<&Path> = cwd.ancestors().collect();
    for dir in dirs.into_iter().rev() {
        let env_file = dir.join(".env");
        if env_file.exists() {
            load_env_file(&env_file, &protected)?;
        }
    }

    Ok(())
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
        Commands::List => cmd_list(),
        Commands::Which { key } => cmd_which(&key),
        Commands::Alias { command } => cmd_alias(command),
        Commands::Env { command } => cmd_env(command),
        Commands::Resolve(args) => cmd_resolve(args),
        Commands::Run(args) => cmd_run(args),
        Commands::Plugin { command } => cmd_plugin(command),
        Commands::Create { name } => {
            println!("Start a new plugin from the template repo:");
            println!("  gh repo create avm-plugin-{name} --template PrajaNova/avm-plugin-template --public --clone");
            Ok(())
        }
        Commands::ShellInit => {
            println!("{}", shell_init_script());
            Ok(())
        }
        Commands::Shims { command } => cmd_shims(command),
        Commands::ExecShim(args) => cmd_exec_shim(args),
        Commands::PluginCommand(args) => cmd_plugin_command(args),
        Commands::Pa { source } => cmd_plugin(PluginCommands::Add { source }),
        Commands::Ea { key, value, global } => cmd_env_add(key, value, global),
    }
}
