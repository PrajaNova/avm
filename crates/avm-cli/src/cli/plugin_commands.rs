fn cmd_plugin(cmd: PluginCommands) -> Result<()> {
    let plugin_manager = PluginManager::new(None)?;
    match cmd {
        PluginCommands::Add { source } => {
            if is_builtin_plugin_name(&source) {
                println!("'{source}' ships with avm itself — nothing to install.");
                return Ok(());
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
                print_available_plugins();
            }
            Ok(())
        }
        PluginCommands::Available => {
            print_available_plugins();
            Ok(())
        }
        PluginCommands::Remove { name } => {
            if is_builtin_plugin_name(&name) {
                return Err(anyhow!(
                    "'{name}' ships with avm itself and can't be removed with `avm plugin remove`"
                ));
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
/// Native, host-first providers (see `provider_by_name` in tool_commands.rs)
/// that ship with avm itself rather than being installed as asdf-compatible
/// plugins. Listed here so `avm plugin add/remove/list` can treat them
/// uniformly instead of one hand-written function pair per tool.
const BUILTIN_PLUGINS: &[(&str, &str, &str)] = &[
    (
        "node",
        "built-in Node.js provider for package.json scripts and node tool resolution",
        "Node Scripts",
    ),
    (
        "android",
        "built-in Android SDK provider (cmdline-tools, platform-tools, sdkmanager)",
        "Android SDK",
    ),
    (
        "java",
        "built-in OpenJDK provider (Eclipse Temurin builds via the foojay Disco API)",
        "OpenJDK",
    ),
];

fn is_builtin_plugin_name(name: &str) -> bool {
    BUILTIN_PLUGINS.iter().any(|(n, _, _)| *n == name)
}

/// Tier-1 builtins are the `avm-plugin-<name>` executables actually shipped
/// next to `avm-bin` (see `avm_runtime::builtin_plugin_process`) — no marker
/// file needed, since "the binary exists" already answers "is it installed."
/// Any tier-2/3 plugin directory sharing a builtin's name is real on disk
/// but permanently shadowed by `provider_by_name`'s tier order, so it's
/// called out explicitly instead of silently printed as if it were active.
fn print_installed_plugins(plugin_manager: &PluginManager) -> Result<()> {
    let on_disk = plugin_manager.list_plugins()?;

    let mut lines: Vec<String> = Vec::new();
    for (name, description, _) in BUILTIN_PLUGINS {
        if avm_runtime::builtin_plugin_process(name)?.is_some() {
            lines.push(format!("  {name} (built-in) - {description}"));
        }
    }
    let mut other_names: Vec<_> = on_disk.keys().collect();
    other_names.sort();
    for name in other_names {
        let manifest = &on_disk[name];
        let shadowed = if is_builtin_plugin_name(&manifest.name) {
            format!(
                " [shadowed by the built-in '{}' provider; safe to delete this directory]",
                manifest.name
            )
        } else {
            String::new()
        };
        lines.push(format!(
            "  {} ({}) - {}{shadowed}",
            manifest.name,
            manifest.version,
            manifest.description.clone().unwrap_or_default()
        ));
    }

    if lines.is_empty() {
        println!("No plugins installed.");
    } else {
        println!("Installed plugins:");
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}

fn print_available_plugins() {
    println!("Available plugins:");
    for (name, description, _) in BUILTIN_PLUGINS {
        println!("  {name} - {description}");
    }
    println!();
    println!("Install with:");
    println!("  avm plugin add <name>");
    println!();
    println!("Install external AVM or compatible asdf plugins with:");
    println!("  avm plugin add <path-or-url>");
    println!("  avm plugin add https://github.com/asdf-community/asdf-kotlin.git");
}
