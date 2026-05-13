fn cmd_tool(cmd: Option<ToolCommands>) -> Result<()> {
    let cmd = cmd.unwrap_or(ToolCommands::List);
    let node = NodeProvider::new();
    let cfg = load_state()?;
    match cmd {
        ToolCommands::List => {
            print_tool_list(&cfg, &node)?;
            Ok(())
        }
        ToolCommands::Use(args) => set_tool_version(&args.tool, &args.version, args.global),
        ToolCommands::Install(args) => {
            let provider = provider_by_name(&args.tool, &node)?;
            provider.install(&args.version)?;
            println!("✓ Installed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Uninstall(args) => {
            let provider = provider_by_name(&args.tool, &node)?;
            provider.uninstall(&args.version)?;
            println!("✓ Removed {} {}", args.tool, args.version);
            Ok(())
        }
        ToolCommands::Provider(args) => cmd_provider_tool(args, &cfg, &node),
    }
}

fn provider_by_name<'a>(name: &str, node: &'a NodeProvider) -> Result<&'a dyn ToolProvider> {
    match name {
        "node" => Ok(node),
        _ => Err(anyhow!("unsupported tool provider '{name}'")),
    }
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

    print_installed_versions(node)
}
