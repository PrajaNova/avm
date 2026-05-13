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
