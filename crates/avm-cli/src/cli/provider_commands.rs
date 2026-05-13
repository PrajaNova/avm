enum VersionFilter {
    Recent,
    Major(u64),
    Latest,
}

fn cmd_provider_tool(
    args: Vec<String>,
    cfg: &ResolvedConfig,
    node: &NodeProvider,
) -> Result<()> {
    let (provider_name, parts) = args
        .split_first()
        .ok_or_else(|| anyhow!("tool provider required"))?;
    let provider = provider_by_name(provider_name, node)?;

    match parts {
        [] => {
            print_provider_status(provider_name, provider, cfg)?;
            Ok(())
        }
        [cmd] if cmd == "list" || cmd == "ls" => {
            print_provider_status(provider_name, provider, cfg)?;
            Ok(())
        }
        [cmd] if cmd == "versions" || cmd == "available" => {
            print_available_versions(provider_name, provider, VersionFilter::Recent)?;
            Ok(())
        }
        [filter, cmd] if cmd == "versions" || cmd == "available" => {
            let filter = parse_version_filter(filter)?;
            print_available_versions(provider_name, provider, filter)?;
            Ok(())
        }
        [cmd, version] if cmd == "use" => set_tool_version(provider_name, version, false),
        [cmd, version, flag] if cmd == "use" && (flag == "--global" || flag == "-g") => {
            set_tool_version(provider_name, version, true)
        }
        [cmd, version] if cmd == "install" => {
            provider.install(version)?;
            println!("✓ Installed {provider_name} {version}");
            Ok(())
        }
        [cmd, version] if cmd == "uninstall" => {
            provider.uninstall(version)?;
            println!("✓ Removed {provider_name} {version}");
            Ok(())
        }
        [cmd] if cmd == "--help" || cmd == "-h" || cmd == "help" => {
            print_provider_help(provider_name);
            Ok(())
        }
        _ => Err(anyhow!(
            "unknown {provider_name} tool command. Try `avm tool {provider_name} --help`"
        )),
    }
}

fn parse_version_filter(value: &str) -> Result<VersionFilter> {
    if value == "latest" || value == "latets" {
        return Ok(VersionFilter::Latest);
    }

    let major = value
        .trim_start_matches('v')
        .parse::<u64>()
        .with_context(|| format!("unknown version filter: {value}"))?;
    Ok(VersionFilter::Major(major))
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

fn print_provider_help(provider_name: &str) {
    println!("Usage: avm tool {provider_name} [COMMAND]");
    println!();
    println!("Commands:");
    println!("  list                         Show selected and installed versions");
    println!("  versions                     Pick from available versions");
    println!("  <major> versions             Pick from one major line");
    println!("  latest versions              Pick the latest available version");
    println!("  use <version> [-g|--global]  Set version locally or globally");
    println!("  install <version>            Install version");
    println!("  uninstall <version>          Remove managed version");
}

fn provider_query(filter: VersionFilter) -> avm_plugin_api::ToolVersionQuery {
    match filter {
        VersionFilter::Recent => avm_plugin_api::ToolVersionQuery::Recent,
        VersionFilter::Latest => avm_plugin_api::ToolVersionQuery::Latest,
        VersionFilter::Major(major) => avm_plugin_api::ToolVersionQuery::Major(major),
    }
}
