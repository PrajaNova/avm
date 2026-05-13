use anyhow::{anyhow, Context, Result};
use avm_core::{load_with_env, AliasSource, ConfigLoadResult, Resolver, ResolvedConfig};
use avm_plugin_api::ResolvedAlias;
use avm_plugin_api::ToolProvider;
use avm_plugin_node::{NodeAlias, NodeProvider};
use avm_runtime::PluginManager;
use avm_shims::{install_shims, remove_shim, shim_path_env};
use clap::{Args, Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG_FILE: &str = ".avm.json";
const BUILTIN_PLUGIN_DIR: &str = ".builtins";
const BUILTIN_NODE_PLUGIN_MARKER: &str = "node";

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
}

#[derive(Subcommand)]
enum ToolCommands {
    #[command(alias = "ls")]
    List,
    Use(ToolUseArgs),
    Install(ToolInstallArgs),
    Uninstall(ToolUninstallArgs),
    Node {
        #[command(subcommand)]
        command: Option<NodeToolCommands>,
    },
}

#[derive(Subcommand)]
enum NodeToolCommands {
    #[command(alias = "ls")]
    List,
    #[command(alias = "available")]
    Versions,
    Use(NodeToolUseArgs),
    Install(NodeToolVersionArgs),
    Uninstall(NodeToolVersionArgs),
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
struct NodeToolUseArgs {
    version: String,
    #[arg(short = 'g', long)]
    global: bool,
}

#[derive(Args)]
struct NodeToolVersionArgs {
    version: String,
}

#[derive(Args)]
struct ExecShimArgs {
    tool: String,
    #[arg(last = true)]
    args: Vec<String>,
}

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
    }
}

fn load_state() -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let home = home_dir()?;
    let mut plugin_aliases = load_runtime_aliases(&cwd)?;
    for (name, alias) in load_node_aliases(&cwd)? {
        plugin_aliases.entry(name).or_insert(alias);
    }
    let resolver = Resolver::new(cwd, home);
    resolver.load(plugin_aliases)
}

fn home_dir() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(home));
    }
    Err(anyhow!("HOME not set"))
}

fn load_config_for_root(root: &Path) -> Result<ConfigLoadResult> {
    load_with_env(root, CONFIG_FILE)
}

fn load_runtime_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let plugin_manager = PluginManager::new(None)?;
    plugin_manager.list_aliases(cwd)
}

fn load_node_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let node = NodeProvider::new();
    let aliases = node.aliases_from_package_json(cwd)?;
    let mut resolved = HashMap::new();
    for (name, alias) in aliases {
        resolved.insert(
            name,
            resolve_node_alias(alias),
        );
    }
    Ok(resolved)
}

fn resolve_node_alias(alias: NodeAlias) -> ResolvedAlias {
    let manager = if alias.manager.is_empty() {
        "npm".to_string()
    } else {
        alias.manager.clone()
    };
    ResolvedAlias {
        command: alias.command,
        description: alias.description,
        plugin_name: "node".to_string(),
        section_name: "Node Scripts".to_string(),
        source: Some(manager),
    }
}

fn save_config_for_root(
    root: &Path,
    aliases: &HashMap<String, String>,
    env: &HashMap<String, String>,
    tools: &HashMap<String, String>,
    structured: bool,
) -> Result<()> {
    avm_core::save_config(root, CONFIG_FILE, aliases, env, tools, structured)
}

fn cmd_init() -> Result<()> {
    let root = std::env::current_dir().context("failed to read current directory")?;
    let path = root.join(CONFIG_FILE);
    if path.exists() {
        return Err(anyhow!("{CONFIG_FILE} already exists"));
    }
    avm_core::write_default_config(root, CONFIG_FILE)?;
    println!("✓ Created {CONFIG_FILE} in current directory");
    Ok(())
}

fn cmd_add(args: AddArgs) -> Result<()> {
    let root = if args.global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };

    if !args.global {
        let path = root.join(CONFIG_FILE);
        if !path.exists() {
            return Err(anyhow!(
                "no {CONFIG_FILE} found in current directory. Run `avm init` first"
            ));
        }
    } else {
        let path = root.join(CONFIG_FILE);
        if !path.exists() {
            avm_core::write_default_config(root.as_path(), CONFIG_FILE)?;
        }
    }

    let parsed = load_config_for_root(&root)?;
    let mut aliases = parsed.aliases;
    aliases.insert(args.key, args.value.join(" "));

    save_config_for_root(&root, &aliases, &parsed.env, &parsed.tools, parsed.is_structured)
}

fn cmd_remove(args: RemoveArgs) -> Result<()> {
    let root = if args.global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Err(anyhow!("no {CONFIG_FILE} found"));
    }

    let mut parsed = load_config_for_root(&root)?;
    let existing = parsed.aliases.remove(&args.key);
    if existing.is_none() {
        return Err(anyhow!("alias '{}' not found", args.key));
    }
    save_config_for_root(
        &root,
        &parsed.aliases,
        &parsed.env,
        &parsed.tools,
        parsed.is_structured,
    )?;

    if args.global {
        println!("✓ Removed global alias '{}'", args.key);
    } else {
        println!("✓ Removed local alias '{}'", args.key);
    }
    Ok(())
}

fn cmd_list() -> Result<()> {
    let cfg = load_state()?;
    let mut printed = false;

    if !cfg.local_aliases.is_empty() || !cfg.global_aliases.is_empty() {
        printed = true;
        println!("Aliases:");
        let mut keys: Vec<&String> = cfg
            .local_aliases
            .keys()
            .chain(cfg.global_aliases.keys())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            if let Some(value) = cfg.local_aliases.get(key) {
                if cfg.global_aliases.contains_key(key) {
                    println!("  {key} → {value} [override global]");
                } else {
                    println!("  {key} → {value}");
                }
            } else if let Some(value) = cfg.global_aliases.get(key) {
                println!("  {key} → {value}");
            }
        }
    }

    let merged_env = merge_env(&cfg);
    if !merged_env.is_empty() {
        printed = true;
        println!("Environment:");
        let mut keys: Vec<_> = merged_env.keys().collect();
        keys.sort();
        for key in keys {
            println!("  {key}={}", merged_env[key]);
        }
    }

    if !cfg.local_tools.is_empty() || !cfg.global_tools.is_empty() {
        printed = true;
        println!("Tools:");
        let mut keys: Vec<&String> = cfg
            .local_tools
            .keys()
            .chain(cfg.global_tools.keys())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            if let Some(version) = cfg.local_tools.get(key) {
                if cfg.global_tools.contains_key(key) {
                    println!("  {key} = {version} [override global]");
                } else {
                    println!("  {key} = {version}");
                }
            } else if let Some(version) = cfg.global_tools.get(key) {
                println!("  {key} = {version}");
            }
        }
    }

    if !cfg.plugin_aliases.is_empty() {
        printed = true;
        println!("Plugin aliases:");
        let mut section_map: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        for (name, alias) in &cfg.plugin_aliases {
            if cfg.local_aliases.contains_key(name) || cfg.global_aliases.contains_key(name) {
                continue;
            }

            section_map
                .entry(alias.section_name.clone())
                .or_default()
                .push((
                    name.clone(),
                    alias.command.clone(),
                    alias.source.clone().unwrap_or_default(),
                ));
        }

        let mut sections: Vec<_> = section_map.keys().collect();
        sections.sort();
        for section in sections {
            println!("  {section}:");
            if let Some(items) = section_map.get(section) {
                let mut sorted = items.clone();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                for (name, command, source) in sorted {
                    if source.is_empty() {
                        println!("    {name} → {command}");
                    } else {
                        println!("    {name} → {command} ({source})");
                    }
                }
            }
        }
    }

    if !printed {
        println!("No aliases configured.");
        println!();
        println!("Get started:");
        println!("  avm init");
        println!("  avm add start \"npm run dev\"");
        println!("  avm tool use node 20.11.1");
        println!("  avm plugin add <url-or-path>");
    }

    Ok(())
}

fn cmd_which(key: &str) -> Result<()> {
    let cfg = load_state()?;
    if let Some(alias) = cfg.resolve_alias(key, &cfg) {
        match alias.source {
            AliasSource::Local => println!("local alias '{key}': {}", alias.command),
            AliasSource::Global => println!("global alias '{key}': {}", alias.command),
            AliasSource::Plugin => {
                let plugin = alias.plugin_name.unwrap_or_else(|| "plugin".to_string());
                println!("plugin alias '{key}' from {plugin}: {}", alias.command);
            }
        }
        return Ok(());
    }

    if let Some((version, source)) = cfg.resolve_tool(key, &cfg) {
        println!("tool '{key}': {version} ({})", alias_source_label(&source));
        return Ok(());
    }

    println!("No mapping found for '{key}'.");
    Ok(())
}

fn cmd_env(args: EnvArgs) -> Result<()> {
    if args.format != "export" {
        return Err(anyhow!("unknown env format: {}", args.format));
    }

    let cfg = load_state()?;
    let mut env = merge_env(&cfg);
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }

    let mut keys: Vec<_> = env.keys().collect();
    keys.sort();
    for key in keys {
        println!("export {key}={}", shell_quote(&env[key]));
    }
    Ok(())
}

fn cmd_resolve(args: ResolveArgs) -> Result<()> {
    let cfg = load_state()?;
    let alias = cfg
        .resolve_alias(&args.key, &cfg)
        .ok_or_else(|| anyhow!("alias '{}' not found", args.key))?;
    let command = build_alias_command(&alias.command, &args.args)?;
    println!("{}", shell_quote_command(&command));
    Ok(())
}

fn cmd_run(args: RunArgs) -> Result<()> {
    if args.args.is_empty() {
        return Err(anyhow!("run requires a command name"));
    }

    let cfg = load_state()?;
    let alias = cfg
        .resolve_alias(&args.args[0], &cfg)
        .ok_or_else(|| anyhow!("alias '{}' not found", args.args[0]))?;
    let command = build_alias_command(&alias.command, &args.args[1..])?;
    if command.is_empty() {
        return Err(anyhow!("alias '{}' resolved to empty command", args.args[0]));
    }

    let mut env = std::env::vars().collect::<HashMap<String, String>>();
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }
    for (key, value) in merge_env(&cfg) {
        env.insert(key, value);
    }

    let status = Command::new(&command[0])
        .args(&command[1..])
        .envs(env)
        .status()
        .context("failed to run alias")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn shell_quote_command(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_alias_command(template: &str, args: &[String]) -> Result<Vec<String>> {
    let mut tokens = split_command_template(template)?;
    if tokens.is_empty() {
        return Err(anyhow!("invalid empty command"));
    }

    let mut contains_placeholder = false;
    let mut expanded = Vec::with_capacity(tokens.len());
    for token in tokens.drain(..) {
        let replaced = expand_template_placeholders(&token, args)?;
        if replaced != token {
            contains_placeholder = true;
        }
        expanded.push(replaced);
    }

    if !contains_placeholder && template.contains('$') {
        // Preserve literal $ when no placeholder syntax was actually used.
        expanded = split_command_template(template)?;
    }

    if !contains_placeholder {
        expanded.extend_from_slice(args);
    }

    Ok(expanded)
}

fn expand_template_placeholders(value: &str, args: &[String]) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        let mut digits = String::new();
        while let Some(next) = chars.peek() {
            if next.is_ascii_digit() {
                if let Some(digit) = chars.next() {
                    digits.push(digit);
                }
            } else {
                break;
            }
        }

        if digits.is_empty() {
            output.push('$');
            continue;
        }

        let index: usize = digits
            .parse()
            .with_context(|| format!("invalid placeholder ${digits}"))?;
        if index == 0 || index > args.len() {
            return Err(anyhow!("placeholder ${index} out of bounds"));
        }
        output.push_str(&args[index - 1]);
    }

    Ok(output)
}

fn split_command_template(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    enum State {
        Normal,
        SingleQuote,
        DoubleQuote,
        Escape,
        DoubleEscape,
    }
    let mut state = State::Normal;

    for ch in input.chars() {
        state = match state {
            State::Normal => match ch {
                '\'' => State::SingleQuote,
                '"' => State::DoubleQuote,
                '\\' => State::Escape,
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                    State::Normal
                }
                _ => {
                    current.push(ch);
                    State::Normal
                }
            },
            State::SingleQuote => {
                if ch == '\'' {
                    State::Normal
                } else {
                    current.push(ch);
                    State::SingleQuote
                }
            }
            State::DoubleQuote => {
                if ch == '"' {
                    State::Normal
                } else if ch == '\\' {
                    State::DoubleEscape
                } else {
                    current.push(ch);
                    State::DoubleQuote
                }
            }
            State::Escape => {
                current.push(ch);
                State::Normal
            }
            State::DoubleEscape => {
                match ch {
                    '"' | '\\' | '$' | '`' => current.push(ch),
                    _ => {
                        current.push('\\');
                        current.push(ch);
                    }
                }
                State::DoubleQuote
            }
        };
    }

    match state {
        State::Normal | State::Escape | State::DoubleEscape => {
            if !current.is_empty() {
                tokens.push(current);
            }
            Ok(tokens)
        }
        State::SingleQuote | State::DoubleQuote => Err(anyhow!("unterminated quoted command")),
    }
}

fn cmd_tool(cmd: Option<ToolCommands>) -> Result<()> {
    let cmd = cmd.unwrap_or(ToolCommands::List);
    let node = NodeProvider::new();
    let cfg = load_state()?;
    match cmd {
        ToolCommands::List => {
            print_tool_list(&cfg, &node)?;
            Ok(())
        }
        ToolCommands::Use(args) => {
            set_tool_version(&args.tool, &args.version, args.global)
        }
        ToolCommands::Install(args) => {
            if args.tool != "node" {
                return Err(anyhow!("unsupported tool '{}'", args.tool));
            }
            node.install(&args.version)?;
            println!("✓ Installed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Uninstall(args) => {
            if args.tool != "node" {
                return Err(anyhow!("unsupported tool '{}'", args.tool));
            }
            node.uninstall(&args.version)?;
            println!("✓ Removed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Node { command } => cmd_node_tool(command, &cfg, &node),
    }
}

fn cmd_node_tool(
    command: Option<NodeToolCommands>,
    cfg: &ResolvedConfig,
    node: &NodeProvider,
) -> Result<()> {
    match command.unwrap_or(NodeToolCommands::List) {
        NodeToolCommands::List => {
            print_node_tool_status(cfg, node)?;
            Ok(())
        }
        NodeToolCommands::Versions => {
            println!("Available node versions: not implemented yet");
            println!("This build can list installed managed versions and select a version.");
            println!();
            print_installed_node_versions(node)?;
            println!();
            println!("Use:");
            println!("  avm tool node use <version>");
            println!("  avm tool node install <version>");
            Ok(())
        }
        NodeToolCommands::Use(args) => set_tool_version("node", &args.version, args.global),
        NodeToolCommands::Install(args) => {
            node.install(&args.version)?;
            println!("✓ Installed node {}", args.version);
            Ok(())
        }
        NodeToolCommands::Uninstall(args) => {
            node.uninstall(&args.version)?;
            println!("✓ Removed node {}", args.version);
            Ok(())
        }
    }
}

fn set_tool_version(tool: &str, version: &str, global: bool) -> Result<()> {
    let root = if global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };
    if !global {
        let path = root.join(CONFIG_FILE);
        if !path.exists() {
            return Err(anyhow!(
                "no {CONFIG_FILE} found in current directory. Run `avm init` first"
            ));
        }
    } else {
        let path = root.join(CONFIG_FILE);
        if !path.exists() {
            avm_core::write_default_config(root.as_path(), CONFIG_FILE)?;
        }
    }

    let mut parsed = load_config_for_root(&root)?;
    parsed.tools.insert(tool.to_string(), version.to_string());
    save_config_for_root(&root, &parsed.aliases, &parsed.env, &parsed.tools, true)?;
    if global {
        println!("✓ Set global {tool} version to {version}");
    } else {
        println!("✓ Set local {tool} version to {version}");
    }
    Ok(())
}

fn print_tool_list(cfg: &ResolvedConfig, node: &NodeProvider) -> Result<()> {
    println!("Tool providers:");
    println!("  node");
    let resolved = cfg.resolve_tools_with_source(cfg);
    if resolved.is_empty() {
        println!("Resolved tools: none");
    } else {
        println!("Resolved tools:");
        let mut keys: Vec<_> = resolved.keys().collect();
        keys.sort_unstable();
        for key in keys {
            if let Some((version, source)) = resolved.get(key) {
                println!("  {key}: {version} ({})", alias_source_label(source));
            }
        }
    }

    print_installed_node_versions(node)
}

fn print_node_tool_status(cfg: &ResolvedConfig, node: &NodeProvider) -> Result<()> {
    println!("Tool provider: node");
    if let Some((version, source)) = cfg.resolve_tool("node", cfg) {
        println!("Selected version: {version} ({})", alias_source_label(&source));
    } else {
        println!("Selected version: none");
    }
    print_installed_node_versions(node)?;
    println!();
    println!("Commands:");
    println!("  avm tool node versions");
    println!("  avm tool node use <version>");
    println!("  avm tool node install <version>");
    println!("  avm tool node uninstall <version>");
    Ok(())
}

fn print_installed_node_versions(node: &NodeProvider) -> Result<()> {
    let installed = node.installed_versions()?;
    if installed.is_empty() {
        println!("Installed node versions: none");
    } else {
        println!("Installed node versions: {}", installed.join(", "));
    }
    Ok(())
}

fn cmd_plugin(cmd: PluginCommands) -> Result<()> {
    let plugin_manager = PluginManager::new(None)?;
    match cmd {
        PluginCommands::Add { source } => {
            if source == "node" {
                install_builtin_node_plugin(&plugin_manager)?;
                println!("✓ Installed built-in plugin node");
                return Ok(());
            }
            println!("Installing plugin from {source}...");
            plugin_manager.install_plugin(&source)?;
            println!("✓ Installed plugin");
            Ok(())
        }
        PluginCommands::List { all } => {
            let installed = installed_plugins(&plugin_manager)?;
            if installed.is_empty() {
                println!("No plugins installed.");
            } else {
                println!("Installed plugins:");
                let mut names: Vec<_> = installed.keys().collect();
                names.sort();
                for name in names {
                    let manifest = &installed[name];
                    println!(
                        "  {} ({}) - {}",
                        manifest.name,
                        manifest.version,
                        manifest.description.clone().unwrap_or_default()
                    );
                }
            }

            if all {
                println!();
                print_available_plugins();
            }
            Ok(())
        }
        PluginCommands::Available => {
            print_available_plugins();
            Ok(())
        }
        PluginCommands::Remove { name } => {
            if name == "node" {
                remove_builtin_node_plugin(&plugin_manager)?;
                println!("Plugin 'node' removed.");
                return Ok(());
            }
            plugin_manager.remove_plugin(&name)?;
            println!("Plugin '{name}' removed.");
            Ok(())
        }
        PluginCommands::Update { all, name } => {
            if all {
                let names: Vec<String> = plugin_manager
                    .list_plugins()?
                    .keys()
                    .cloned()
                    .collect();
                for name in names {
                    plugin_manager.update_plugin(&name)?;
                }
                return Ok(());
            }
            let name = name.ok_or_else(|| anyhow!("plugin name required unless --all"))?;
            plugin_manager.update_plugin(&name)?;
            println!("Plugin '{name}' updated.");
            Ok(())
        }
    }
}

fn installed_plugins(plugin_manager: &PluginManager) -> Result<HashMap<String, avm_plugin_api::Manifest>> {
    let mut installed = plugin_manager.list_plugins()?;
    if is_builtin_node_plugin_installed(plugin_manager) {
        installed.insert("node".to_string(), builtin_node_manifest());
    }
    Ok(installed)
}

fn install_builtin_node_plugin(plugin_manager: &PluginManager) -> Result<()> {
    let dir = plugin_manager.plugin_dir().join(BUILTIN_PLUGIN_DIR);
    fs::create_dir_all(&dir).with_context(|| {
        format!(
            "failed to create built-in plugin marker directory: {}",
            dir.display()
        )
    })?;
    fs::write(dir.join(BUILTIN_NODE_PLUGIN_MARKER), "builtin\n")
        .context("failed to write built-in node plugin marker")?;
    Ok(())
}

fn remove_builtin_node_plugin(plugin_manager: &PluginManager) -> Result<()> {
    let marker = plugin_manager
        .plugin_dir()
        .join(BUILTIN_PLUGIN_DIR)
        .join(BUILTIN_NODE_PLUGIN_MARKER);
    if marker.exists() {
        fs::remove_file(marker).context("failed to remove built-in node plugin marker")?;
    }
    Ok(())
}

fn is_builtin_node_plugin_installed(plugin_manager: &PluginManager) -> bool {
    plugin_manager
        .plugin_dir()
        .join(BUILTIN_PLUGIN_DIR)
        .join(BUILTIN_NODE_PLUGIN_MARKER)
        .exists()
}

fn builtin_node_manifest() -> avm_plugin_api::Manifest {
    avm_plugin_api::Manifest {
        name: "node".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: Some(1),
        description: Some("Built-in Node.js provider for package.json scripts and node tool resolution".to_string()),
        section_label: Some("Node Scripts".to_string()),
        homepage: Some("https://github.com/prajanova/avm".to_string()),
    }
}

fn print_available_plugins() {
    println!("Available plugins:");
    println!("  node - built-in Node.js provider for package.json scripts and node tool resolution");
    println!();
    println!("Install with:");
    println!("  avm plugin add node");
    println!();
    println!("Install external plugins with:");
    println!("  avm plugin add <path-or-url>");
}

fn cmd_shims(command: ShimsCommands) -> Result<()> {
    match command {
        ShimsCommands::Install => {
            install_shims()?;
            println!("avm shims installed.");
            Ok(())
        }
        ShimsCommands::Remove { tool } => {
            remove_shim(&tool)?;
            println!("Removed shim for {tool}");
            Ok(())
        }
        ShimsCommands::Path => {
            println!("{}", shim_path_env()?);
            Ok(())
        }
    }
}

fn cmd_exec_shim(args: ExecShimArgs) -> Result<()> {
    let cfg = load_state()?;
    let node = NodeProvider::new();
    let effective_tool = normalize_shim_tool(&args.tool);
    let configured_version = cfg.resolve_tool("node", &cfg).map(|(version, _)| version);
    let selected = configured_version.as_deref().and_then(|version| {
        node.bin_path_for(version, &args.tool)
            .ok()
            .flatten()
            .or_else(|| {
                if effective_tool == "node" {
                    node.bin_path_for(version, "node").ok().flatten()
                } else {
                    None
                }
            })
    });

    let executable = if let Some(executable) = selected {
        executable
    } else {
        if let Some(version) = configured_version {
            eprintln!(
                "warning: managed node {version} is not installed; falling back to system {}",
                args.tool
            );
        }
        which_in_path_excluding_shims(&args.tool).ok_or_else(|| anyhow!("command '{}' not found in PATH", args.tool))?
    };

    let mut env = std::env::vars().collect::<HashMap<String, String>>();
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }
    for (key, value) in merge_env(&cfg) {
        env.insert(key, value);
    }

    let status = Command::new(&executable)
        .args(&args.args)
        .envs(env)
        .status()
        .context("failed to run shim target")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn merge_env(cfg: &ResolvedConfig) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    for (key, value) in &cfg.global_env {
        merged.insert(key.clone(), value.clone());
    }
    for (key, value) in &cfg.local_env {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

fn resolved_tool_path_prefix(cfg: &ResolvedConfig) -> Result<Option<String>> {
    let node = NodeProvider::new();
    let selections = cfg.resolve_tools_with_source(&cfg);
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    let sep = if cfg!(windows) { ";" } else { ":" };

    if let Some((version, _)) = selections.get("node") {
        if let Some(path) = node.bin_path_for(version, "node").ok().flatten() {
            if let Some(bin) = path.parent() {
                let candidate = bin.to_string_lossy().to_string();
                if seen.insert(candidate.clone()) {
                    paths.push(candidate);
                }
            }
        }
    }

    if paths.is_empty() {
        return Ok(None);
    }

    let existing = std::env::var("PATH").unwrap_or_else(|_| String::new());
    let injected = paths.join(sep);
    if existing.is_empty() {
        Ok(Some(injected))
    } else {
        Ok(Some(format!("{injected}{sep}{existing}")))
    }
}

fn which_in_path_excluding_shims(binary: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    let shim_dir = avm_shims::shim_dir().ok();

    for entry in std::env::split_paths(&paths) {
        let primary = entry.join(binary);
        if is_safe_executable(&primary).is_some() && !is_shim_path(&primary, &shim_dir) {
            return Some(primary);
        }

        if cfg!(windows) {
            let exe = entry.join(format!("{binary}.exe"));
            if is_safe_executable(&exe).is_some() && !is_shim_path(&exe, &shim_dir) {
                return Some(exe);
            }
        }
    }

    None
}

#[cfg(unix)]
fn is_safe_executable(path: &Path) -> Option<()> {
    if !path.exists() || !path.is_file() {
        return None;
    }

    use std::os::unix::fs::PermissionsExt;
    let meta = path.metadata().ok()?;
    let mode = meta.permissions().mode();
    if mode & 0o002 != 0 {
        return None;
    }
    if mode & 0o111 == 0 {
        return None;
    }
    Some(())
}

#[cfg(not(unix))]
fn is_safe_executable(path: &Path) -> Option<()> {
    if path.exists() && path.is_file() {
        Some(())
    } else {
        None
    }
}

fn is_shim_path(candidate: &Path, shim_dir: &Option<PathBuf>) -> bool {
    let Some(shim_dir) = shim_dir.as_ref() else {
        return false;
    };

    if let (Ok(candidate), Ok(shim_dir)) = (candidate.canonicalize(), shim_dir.canonicalize()) {
        return candidate.starts_with(shim_dir);
    }

    false
}

fn alias_source_label(source: &AliasSource) -> &'static str {
    match source {
        AliasSource::Local => "local",
        AliasSource::Global => "global",
        AliasSource::Plugin => "plugin",
    }
}

fn normalize_shim_tool(tool: &str) -> &str {
    match tool {
        "npm" | "npx" | "pnpm" | "yarn" | "bun" => "node",
        value => value,
    }
}

fn shell_init_script() -> String {
    let shim_dir = "$HOME/.avm/shims";
    format!(
        r#"
if [ -n "{shim_dir}" ] && [ -d "{shim_dir}" ]; then
  case ":$PATH:" in
    *":{shim_dir}:"*) ;;
    *) export PATH="{shim_dir}:$PATH" ;;
  esac
fi

avm() {{
  if [ $# -eq 0 ]; then
    command avm-bin "$@"
    return $?
  fi

  local _avm_key="$1"
  case "$_avm_key" in
    init|add|list|ls|remove|rm|which|env|tool|tools|version|help|shell-init|plugin|completion|--help|-h|--version|-v|resolve|run|shims|exec-shim)
      command avm-bin "$@"
      return $?
      ;;
  esac

  if command avm-bin resolve "$@" >/dev/null 2>&1; then
    command avm-bin run "$@"
    return $?
  fi
  command "$@"
}}
"#
    )
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
