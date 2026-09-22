fn cmd_tool(cmd: Option<ToolCommands>) -> Result<()> {
    let cmd = cmd.unwrap_or(ToolCommands::List);
    let cfg = load_state()?;
    match cmd {
        ToolCommands::List => {
            let node = provider_by_name("node")?;
            print_tool_list(&cfg, node.as_ref())?;
            Ok(())
        }
        ToolCommands::Use(args) => {
            let provider = provider_by_name(&args.tool)?;
            use_provider_version(&args.tool, provider.as_ref(), &args.version, args.global)
        }
        ToolCommands::Install(args) => {
            let provider = provider_by_name(&args.tool)?;
            provider.install(&args.version)?;
            println!("✓ Installed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Uninstall(args) => {
            let provider = provider_by_name(&args.tool)?;
            provider.uninstall(&args.version)?;
            println!("✓ Removed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Provider(args) => cmd_provider_tool(args, &cfg),
    }
}

/// Every provider — first-party or third-party — speaks the same plugin
/// protocol and is discovered the same way, no exceptions and nothing
/// compiled into `avm-bin`:
///   1. installed via the marketplace (`avm plugin add <name>` fetched a
///      compiled release into `~/.avm/plugins/avm-plugin-<name>`)
///   2. the legacy asdf-compatible adapter, kept for community asdf plugins
///      that haven't adopted the native protocol (AGENTS.md: "legacy
///      executable plugins remain supported until v1.1 parity is proven")
/// Nothing is available out of the box — `avm plugin add node` (etc.) is
/// required, same as `brew install` or a Claude Code marketplace install.
fn provider_by_name(name: &str) -> Result<Box<dyn ToolProvider>> {
    let plugin_manager = PluginManager::new(None)?;
    if let Some(provider) = plugin_manager.protocol_provider(name)? {
        return Ok(Box::new(provider));
    }
    if let Some(provider) = plugin_manager.asdf_provider(name)? {
        return Ok(Box::new(provider));
    }

    if let Ok(Some(entry)) = avm_runtime::marketplace_lookup(name) {
        return Err(anyhow!(
            "'{name}' is available in the marketplace but not installed — run `avm plugin add {name}` ({})",
            entry.description
        ));
    }
    Err(anyhow!("unknown plugin '{name}'"))
}

fn print_tool_list(cfg: &ResolvedConfig, node: &dyn ToolProvider) -> Result<()> {
    println!("Tool providers:");
    println!("  node");
    let plugin_manager = PluginManager::new(None)?;
    for provider in plugin_manager.list_asdf_provider_names()? {
        println!("  {provider}");
    }
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

    print_installed_versions(node)
}
