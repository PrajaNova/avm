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

fn cmd_list(global_only: bool) -> Result<()> {
    let cfg = load_state()?;
    let show_local = !global_only;
    let mut printed = false;

    if !cfg.global_aliases.is_empty() || (show_local && !cfg.local_aliases.is_empty()) {
        printed = true;
        println!("Aliases:");
        let mut keys: Vec<&String> = cfg.global_aliases.keys().collect();
        if show_local {
            keys.extend(cfg.local_aliases.keys());
        }
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            if show_local {
                if let Some(value) = cfg.local_aliases.get(key) {
                    if cfg.global_aliases.contains_key(key) {
                        println!("  {key} → {value} [override global]");
                    } else {
                        println!("  {key} → {value}");
                    }
                    continue;
                }
            }
            if let Some(value) = cfg.global_aliases.get(key) {
                println!("  {key} → {value}");
            }
        }
    }

    let merged_env = if global_only {
        cfg.global_env.clone()
    } else {
        merge_env(&cfg)
    };
    if !merged_env.is_empty() {
        printed = true;
        println!("Environment:");
        let mut keys: Vec<_> = merged_env.keys().collect();
        keys.sort();
        for key in keys {
            println!("  {key}={}", merged_env[key]);
        }
    }

    if !cfg.global_tools.is_empty() || (show_local && !cfg.local_tools.is_empty()) {
        printed = true;
        println!("Tools:");
        let mut keys: Vec<&String> = cfg.global_tools.keys().collect();
        if show_local {
            keys.extend(cfg.local_tools.keys());
        }
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            if show_local {
                if let Some(version) = cfg.local_tools.get(key) {
                    if cfg.global_tools.contains_key(key) {
                        println!("  {key} = {version} [override global]");
                    } else {
                        println!("  {key} = {version}");
                    }
                    continue;
                }
            }
            if let Some(version) = cfg.global_tools.get(key) {
                println!("  {key} = {version}");
            }
        }
    }

    // Plugin aliases come from plugins, not local/global config — skip under -g.
    if show_local && !cfg.plugin_aliases.is_empty() {
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
