enum NodeVersionFilter {
    Recent,
    Major(u64),
    Latest,
}

fn parse_node_version_filter(value: &str) -> Result<NodeVersionFilter> {
    if value == "latest" || value == "latets" {
        return Ok(NodeVersionFilter::Latest);
    }

    let major = value
        .trim_start_matches('v')
        .parse::<u64>()
        .with_context(|| format!("unknown node version filter: {value}"))?;
    Ok(NodeVersionFilter::Major(major))
}

fn print_node_tool_help() {
    println!("Usage: avm tool node [COMMAND]");
    println!();
    println!("Commands:");
    println!("  list                         Show selected and installed Node.js versions");
    println!("  versions                     Pick from available Node.js versions");
    println!("  <major> versions             Pick from one major line, for example `avm tool node 19 versions`");
    println!("  latest versions              Pick the latest available Node.js version");
    println!("  use <version> [-g|--global]  Set Node.js version locally or globally");
    println!("  install <version>            Install Node.js version");
    println!("  uninstall <version>          Remove managed Node.js version");
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

fn print_available_node_versions(node: &NodeProvider, filter: NodeVersionFilter) -> Result<()> {
    let query = match filter {
        NodeVersionFilter::Recent => avm_plugin_api::ToolVersionQuery::Recent,
        NodeVersionFilter::Latest => avm_plugin_api::ToolVersionQuery::Latest,
        NodeVersionFilter::Major(major) => avm_plugin_api::ToolVersionQuery::Major(major),
    };
    let versions = node.available_versions(query)?;
    if versions.is_empty() {
        println!("Available node versions: none");
        return Ok(());
    }

    if ui::can_select() {
        return select_tool_version("Available node versions", versions);
    }

    println!("Available node versions:");
    for version in &versions {
        println!("  {}", version.label);
    }

    println!();
    println!("Use:");
    println!("  avm tool node install <version>");
    println!("  avm tool node use <version>");
    Ok(())
}
