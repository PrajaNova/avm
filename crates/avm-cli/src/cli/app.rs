use super::*;

pub fn main() {
    // Windows often has only USERPROFILE; plugins (including ones built on an
    // older avm-plugin-api) find ~/.avm through HOME, so pass ours down.
    if std::env::var_os("HOME").is_none() {
        if let Ok(home) = home_dir() {
            std::env::set_var("HOME", home);
        }
    }
    if let Err(err) = load_dotenv_env() {
        eprintln!("avm: failed to load .env: {err}");
        std::process::exit(1);
    }
    // Windows shims are avm-bin started as `node.exe`, `npm.exe`, ...
    if let Some(tool) = shims::invoked_as_shim() {
        let args = std::env::args().skip(1).collect();
        if let Err(err) = cmd_exec_shim(ExecShimArgs { tool, args }) {
            eprintln!("avm: {err}");
            std::process::exit(1);
        }
        return;
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
    let trusted_paths = home_dir().and_then(|home| config::load(&home)).map(|c| c.trusted_paths).unwrap_or_default();

    let dirs: Vec<&Path> = cwd.ancestors().collect();
    let mut skipped = Vec::new();
    for dir in dirs.into_iter().rev() {
        let env_file = dir.join(".env");
        if !env_file.exists() {
            continue;
        }
        // A project's .env reaches every shimmed tool's environment, so it
        // needs the same trust as its .avm.json (#20).
        if crate::trust::is_trusted(&env_file, &trusted_paths) {
            load_env_file(&env_file, &protected)?;
        } else {
            skipped.push(env_file);
        }
    }
    let _ = UNTRUSTED_DOTENV.set(skipped);

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
        Commands::Env { command, shell } => cmd_env(command, shell.unwrap_or_default()),
        Commands::Resolve(args) => cmd_resolve(args),
        Commands::Run(args) => cmd_run(args),
        Commands::Plugin { command } => cmd_plugin(command),
        Commands::Create { name } => {
            println!("Start a new plugin from the template repo:");
            println!("  gh repo create avm-plugin-{name} --template PrajaNova/avm-plugin-template --public --clone");
            Ok(())
        }
        Commands::ShellInit { shell } => {
            match shell.unwrap_or_default() {
                Shell::Pwsh => println!("{}", pwsh_init_script()),
                Shell::Cmd => return Err(anyhow!("cmd has no init hook; run `avm-bin env --shell cmd` and use `avm-bin` directly")),
                Shell::Sh => println!("{}", shell_init_script()),
            }
            Ok(())
        }
        Commands::Shims { command } => cmd_shims(command),
        Commands::ExecShim(args) => cmd_exec_shim(args),
        Commands::PluginCommand(args) => cmd_plugin_command(args),
        Commands::Pa { source } => cmd_plugin(PluginCommands::Add { source }),
        Commands::Ea { key, value, global } => cmd_env_add(key, value, global),
        Commands::Trust(args) => cmd_trust(args),
    }
}
