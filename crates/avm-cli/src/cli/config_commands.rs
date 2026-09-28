use super::*;

/// `.avm.json`'s directory: `$HOME` for global, else the current directory.
pub fn config_root(global: bool) -> Result<PathBuf> {
    if global {
        home_dir()
    } else {
        std::env::current_dir().context("failed to read current directory")
    }
}

/// Load the local/global config, apply `f`, save it back. A missing file
/// starts empty when `create`, otherwise it's an error.
pub fn edit_config(
    global: bool,
    create: bool,
    f: impl FnOnce(&mut ConfigLoadResult) -> Result<()>,
) -> Result<()> {
    let root = config_root(global)?;
    if !create && !root.join(CONFIG_FILE).exists() {
        return Err(if global {
            anyhow!("no {CONFIG_FILE} found in {}", root.display())
        } else {
            anyhow!("no {CONFIG_FILE} found in current directory. Run `avm init` first")
        });
    }
    // Editing through avm keeps an already-trusted (or brand new) local file
    // trusted; it never promotes an untrusted one.
    let file = root.join(CONFIG_FILE);
    let keep_trusted = !global && (!file.exists() || crate::trust::is_trusted(&file, &[]));
    let mut cfg = config::load(&root)?;
    f(&mut cfg)?;
    config::save(&root, &cfg)?;
    if keep_trusted {
        crate::trust::trust(&file)?;
    }
    Ok(())
}

fn scope(global: bool) -> &'static str {
    if global {
        "global"
    } else {
        "local"
    }
}

/// Print local ∪ global entries sorted by key, local winning.
fn print_layered(
    local: &HashMap<String, String>,
    global: &HashMap<String, String>,
    indent: &str,
    sep: &str,
) {
    let keys: BTreeSet<&String> = local.keys().chain(global.keys()).collect();
    for key in keys {
        match local.get(key) {
            Some(value) if global.contains_key(key) => {
                println!("{indent}{key}{sep}{value} [override global]")
            }
            Some(value) => println!("{indent}{key}{sep}{value}"),
            None => println!("{indent}{key}{sep}{}", global[key]),
        }
    }
}

pub fn cmd_init() -> Result<()> {
    let root = std::env::current_dir().context("failed to read current directory")?;
    if root.join(CONFIG_FILE).exists() {
        return Err(anyhow!("{CONFIG_FILE} already exists"));
    }
    config::save(&root, &ConfigLoadResult::default())?;
    crate::trust::trust(&root.join(CONFIG_FILE))?;
    println!("✓ Created {CONFIG_FILE} in current directory");
    Ok(())
}

pub fn cmd_add(args: AddArgs) -> Result<()> {
    edit_config(args.global, args.global, |cfg| {
        cfg.aliases.insert(args.key.clone(), args.value.join(" "));
        Ok(())
    })?;
    println!("✓ Added {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

pub fn cmd_remove(args: RemoveArgs) -> Result<()> {
    edit_config(args.global, false, |cfg| match cfg.aliases.remove(&args.key) {
        Some(_) => Ok(()),
        None => Err(anyhow!("alias '{}' not found", args.key)),
    })?;
    println!("✓ Removed {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

/// `avm alias add/remove/list` — same shape as `avm plugin`. `add`/`remove`
/// reuse the exact same handlers as the top-level `avm add`/`avm remove`.
pub fn cmd_alias(command: AliasCommands) -> Result<()> {
    match command {
        AliasCommands::Add(args) => cmd_add(args),
        AliasCommands::Remove(args) => cmd_remove(args),
        AliasCommands::List => cmd_alias_list(),
    }
}

fn cmd_alias_list() -> Result<()> {
    let cfg = load_state()?;
    if cfg.local_aliases.is_empty() && cfg.global_aliases.is_empty() {
        println!("No aliases configured.");
        return Ok(());
    }
    print_layered(&cfg.local_aliases, &cfg.global_aliases, "", " → ");
    Ok(())
}

/// `avm env add/remove/list` — same shape as `avm plugin`, targeting the
/// custom `env` map in `.avm.json` rather than `aliases`.
pub fn cmd_env_add(key: String, value: String, global: bool) -> Result<()> {
    edit_config(global, global, |cfg| {
        cfg.env.insert(key.clone(), value.clone());
        Ok(())
    })?;
    println!("✓ Added {} env var '{key}={value}'", scope(global));
    Ok(())
}

pub fn cmd_env_remove(key: String, global: bool) -> Result<()> {
    edit_config(global, false, |cfg| match cfg.env.remove(&key) {
        Some(_) => Ok(()),
        None => Err(anyhow!("env var '{key}' not found")),
    })?;
    println!("✓ Removed {} env var '{key}'", scope(global));
    Ok(())
}

pub fn cmd_env_list() -> Result<()> {
    let cfg = load_state()?;
    let merged = merge_env(&cfg);
    if merged.is_empty() {
        println!("No env vars configured.");
        return Ok(());
    }
    let mut keys: Vec<_> = merged.keys().collect();
    keys.sort();
    for key in keys {
        println!("{key}={}", merged[key]);
    }
    Ok(())
}

pub fn cmd_trust(args: TrustArgs) -> Result<()> {
    if args.list {
        for path in crate::trust::list().keys() {
            println!("{path}");
        }
        return Ok(());
    }
    let target = match args.path {
        Some(path) => path,
        None => std::env::current_dir().context("failed to read current directory")?,
    };
    let files: Vec<PathBuf> = if target.is_dir() {
        [CONFIG_FILE, ".env"].iter().map(|f| target.join(f)).filter(|f| f.exists()).collect()
    } else {
        vec![target.clone()]
    };
    if files.is_empty() {
        return Err(anyhow!("no {CONFIG_FILE} or .env in {}", target.display()));
    }
    for file in files {
        if args.revoke {
            let removed = crate::trust::revoke(&file)?;
            println!("{} {}", if removed { "✓ Revoked trust for" } else { "Was not trusted:" }, file.display());
            continue;
        }
        // Show what's being trusted: the commands and env it will run with.
        if file.file_name().is_some_and(|n| n == CONFIG_FILE) {
            let cfg = config::load(file.parent().unwrap_or(Path::new(".")))?;
            for (key, value) in cfg.aliases.iter().collect::<BTreeMap<_, _>>() {
                println!("  alias {key} → {value}");
            }
            for (key, value) in cfg.env.iter().collect::<BTreeMap<_, _>>() {
                println!("  env   {key}={value}");
            }
        } else {
            for line in fs::read_to_string(&file)?.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#')) {
                println!("  {line}");
            }
        }
        crate::trust::trust(&file)?;
        println!("✓ Trusted {}", file.display());
    }
    Ok(())
}

pub fn cmd_list() -> Result<()> {
    let cfg = load_state()?;
    let mut printed = false;
    let untrusted = untrusted_files(&cfg);
    if !untrusted.is_empty() {
        eprintln!("{}", untrusted_notice(&untrusted));
    }

    if !cfg.local_aliases.is_empty() || !cfg.global_aliases.is_empty() {
        printed = true;
        println!("Aliases:");
        print_layered(&cfg.local_aliases, &cfg.global_aliases, "  ", " → ");
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
        print_layered(&cfg.local_tools, &cfg.global_tools, "  ", " = ");
    }

    if let Ok(plugin_manager) = PluginManager::new() {
        // list_plugins() keys are plugin *directory* names
        // (e.g. "avm-plugin-android"); the short tool name
        // provider_by_name expects lives on the manifest value.
        let names: BTreeSet<String> =
            plugin_manager.list_plugins().into_values().map(|m| m.name).collect();
        let mut lines = Vec::new();
        for name in names {
            if let Ok(provider) = provider_by_name(&name) {
                if let Ok(versions) = provider.installed_versions() {
                    if !versions.is_empty() {
                        lines.push((name, versions));
                    }
                }
            }
        }
        if !lines.is_empty() {
            printed = true;
            println!("Installed versions:");
            for (name, versions) in lines {
                println!("  {name}: {}", versions.join(", "));
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
        println!("  avm node use 20.11.1");
        println!("  avm plugin add <url-or-path>");
    }

    Ok(())
}
