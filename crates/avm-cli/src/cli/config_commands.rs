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
    aliases.insert(args.key.clone(), args.value.join(" "));

    save_config_for_root(&root, &aliases, &parsed.env, &parsed.tools, parsed.is_structured)?;
    if args.global {
        println!("✓ Added global alias '{}'", args.key);
    } else {
        println!("✓ Added local alias '{}'", args.key);
    }
    Ok(())
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

/// `avm alias add/remove/list` — same shape as `avm plugin`. `add`/`remove`
/// reuse the exact same handlers as the still-working top-level `avm add`/
/// `avm remove` shortcuts; only `list` is new here.
fn cmd_alias(command: AliasCommands) -> Result<()> {
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
                println!("{key} → {value} [override global]");
            } else {
                println!("{key} → {value}");
            }
        } else if let Some(value) = cfg.global_aliases.get(key) {
            println!("{key} → {value}");
        }
    }
    Ok(())
}

/// `avm env add/remove/list` — same shape as `avm plugin`, targeting the
/// custom `env` map in `.avm.json` rather than `aliases`. Previously the
/// only way to set a custom env var was hand-editing the file directly.
fn cmd_env_add(key: String, value: String, global: bool) -> Result<()> {
    let root = if global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };

    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        if global {
            avm_core::write_default_config(root.as_path(), CONFIG_FILE)?;
        } else {
            return Err(anyhow!(
                "no {CONFIG_FILE} found in current directory. Run `avm init` first"
            ));
        }
    }

    let parsed = load_config_for_root(&root)?;
    let mut env = parsed.env;
    env.insert(key.clone(), value.clone());

    save_config_for_root(&root, &parsed.aliases, &env, &parsed.tools, parsed.is_structured)?;
    if global {
        println!("✓ Added global env var '{key}={value}'");
    } else {
        println!("✓ Added local env var '{key}={value}'");
    }
    Ok(())
}

fn cmd_env_remove(key: String, global: bool) -> Result<()> {
    let root = if global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Err(anyhow!("no {CONFIG_FILE} found"));
    }

    let mut parsed = load_config_for_root(&root)?;
    let existing = parsed.env.remove(&key);
    if existing.is_none() {
        return Err(anyhow!("env var '{key}' not found"));
    }
    save_config_for_root(
        &root,
        &parsed.aliases,
        &parsed.env,
        &parsed.tools,
        parsed.is_structured,
    )?;

    if global {
        println!("✓ Removed global env var '{key}'");
    } else {
        println!("✓ Removed local env var '{key}'");
    }
    Ok(())
}

fn cmd_env_list() -> Result<()> {
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

    if let Ok(plugin_manager) = PluginManager::new(None) {
        if let Ok(plugins) = plugin_manager.list_plugins() {
            // list_plugins() keys are plugin *directory* names
            // (e.g. "avm-plugin-android"); the short tool name
            // provider_by_name expects lives on the manifest value.
            let mut names: Vec<_> = plugins.values().map(|m| m.name.clone()).collect();
            names.sort();
            names.dedup();
            let mut lines = Vec::new();
            for name in &names {
                if let Ok(provider) = provider_by_name(name) {
                    if let Ok(versions) = provider.installed_versions() {
                        if !versions.is_empty() {
                            lines.push((name.clone(), versions));
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
