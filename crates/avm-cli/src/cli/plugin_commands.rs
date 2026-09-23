use super::*;

pub fn cmd_plugin(cmd: PluginCommands) -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    match cmd {
        PluginCommands::Add { source } => {
            // A bare name (no "/", no scheme) is looked up in the
            // marketplace first — that's the common case ("avm plugin add
            // node"). Anything else (an org/repo, a full URL) goes straight
            // to the existing git-clone install path, which still works for
            // asdf-style plugins or a third party building from source.
            if !source.contains('/') {
                if let Some(entry) = runtime::marketplace_lookup(&source)? {
                    println!("Fetching '{source}' from {}...", entry.repo);
                    let version =
                        runtime::install_from_marketplace(&source, &entry.repo, &plugin_manager.plugin_dir())?;
                    println!("✓ Installed {source} {version}");
                    return Ok(());
                }
            }
            println!("Installing plugin from {source}...");
            plugin_manager.install_plugin(&source)?;
            println!("✓ Installed plugin");
            Ok(())
        }
        PluginCommands::List { all } => {
            print_installed_plugins(&plugin_manager)?;

            if all {
                println!();
                print_available_plugins()?;
            }
            Ok(())
        }
        PluginCommands::Available => {
            print_available_plugins()?;
            Ok(())
        }
        PluginCommands::Remove { name } => {
            plugin_manager.remove_plugin(&name)?;
            println!("Plugin '{name}' removed.");
            Ok(())
        }
        PluginCommands::Update { all, name } => {
            if all {
                for name in plugin_manager.list_plugins().into_keys() {
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
/// Everything installed on disk — marketplace plugins (`avm-plugin-<name>`
/// dirs) and legacy asdf plugins alike.
/// Nothing is compiled into `avm-bin`; this is a plain directory listing.
fn print_installed_plugins(plugin_manager: &PluginManager) -> Result<()> {
    let on_disk = plugin_manager.list_plugins();
    let mut names: Vec<_> = on_disk.keys().collect();
    names.sort();

    if names.is_empty() {
        println!("No plugins installed. Run `avm plugin available` to see the marketplace.");
        return Ok(());
    }

    println!("Installed plugins:");
    for name in names {
        let manifest = &on_disk[name];
        println!(
            "  {} ({}) - {}",
            manifest.name,
            manifest.version,
            manifest.description.clone().unwrap_or_default()
        );
    }
    Ok(())
}

fn print_available_plugins() -> Result<()> {
    println!("Marketplace (github.com/PrajaNova/avm-marketplace):");
    match runtime::marketplace_registry() {
        Ok(mut entries) => {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            for entry in entries {
                println!("  {} - {}", entry.name, entry.description);
            }
        }
        Err(err) => println!("  (couldn't reach the marketplace: {err})"),
    }
    println!();
    println!("Install with:");
    println!("  avm plugin add <name>");
    println!();
    println!("Install anything else (asdf-style plugin, or a plugin not yet in the marketplace) with a direct source:");
    println!("  avm plugin add <path-or-url>");
    println!("  avm plugin add https://github.com/asdf-community/asdf-kotlin.git");
    Ok(())
}
