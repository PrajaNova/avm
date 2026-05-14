enum VersionFilter {
    Recent,
    Major(u64),
    Latest,
}

fn cmd_provider_tool(
    args: Vec<String>,
    cfg: &ResolvedConfig,
) -> Result<()> {
    let (provider_name, parts) = args
        .split_first()
        .ok_or_else(|| anyhow!("plugin name required"))?;
    let provider = provider_by_name(provider_name)?;

    match parts {
        [] => {
            print_provider_status(provider_name, provider.as_ref(), cfg)?;
            Ok(())
        }
        [cmd] if cmd == "list" || cmd == "ls" => {
            print_provider_status(provider_name, provider.as_ref(), cfg)?;
            Ok(())
        }
        [cmd] if cmd == "versions" || cmd == "available" => {
            print_available_versions(provider_name, provider.as_ref(), VersionFilter::Recent)?;
            Ok(())
        }
        [filter, cmd] if cmd == "versions" || cmd == "available" => {
            let filter = parse_version_filter(filter)?;
            print_available_versions(provider_name, provider.as_ref(), filter)?;
            Ok(())
        }
        [cmd, version] if cmd == "use" || cmd == "set" => {
            use_provider_version(provider_name, provider.as_ref(), version, false)
        }
        [cmd, version, flag]
            if (cmd == "use" || cmd == "set") && (flag == "--global" || flag == "-g") =>
        {
            use_provider_version(provider_name, provider.as_ref(), version, true)
        }
        [cmd, version] if cmd == "install" => {
            provider.install(version)?;
            install_shims()?;
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
            "unknown {provider_name} command. Try `avm {provider_name} --help`"
        )),
    }
}

fn cmd_plugin_command(args: Vec<String>) -> Result<()> {
    let Some(provider_name) = args.first() else {
        return Err(anyhow!("plugin name required"));
    };
    let _ = provider_name;
    let cfg = load_state()?;
    cmd_provider_tool(args, &cfg)
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

fn use_provider_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
    global: bool,
) -> Result<()> {
    ensure_provider_version_installed(provider_name, provider, version)?;
    install_shims()?;
    set_tool_version(provider_name, version, global)?;
    warn_if_shim_is_not_preferred(provider_name);
    Ok(())
}

fn ensure_provider_version_installed(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
) -> Result<()> {
    if provider.is_installed(version) {
        return Ok(());
    }

    println!("Installing {provider_name} {version}...");
    provider.install(version)?;
    println!("✓ Installed {provider_name} {version}");
    Ok(())
}

fn print_provider_help(provider_name: &str) {
    println!("Usage: avm {provider_name} [COMMAND]");
    println!();
    println!("Commands:");
    println!("  list                         Show selected and installed versions");
    println!("  versions                     Pick from available versions");
    println!("  <major> versions             Pick from one major line");
    println!("  latest versions              Pick the latest available version");
    println!("  use <version> [-g|--global]  Set version locally or globally");
    println!("  set <version> [-g|--global]  Alias for use");
    println!("  install <version>            Install version");
    println!("  uninstall <version>          Remove managed version");
}

fn warn_if_shim_is_not_preferred(tool: &str) {
    let Ok(shim_dir) = avm_shims::shim_dir() else {
        return;
    };
    let Some(first_match) = first_path_match(tool) else {
        return;
    };

    if path_starts_with(&first_match, &shim_dir) {
        return;
    }

    eprintln!(
        "warning: plain `{tool}` currently resolves to {} before AVM shims",
        first_match.display()
    );
    eprintln!("warning: run `eval \"$(avm shell-init)\"` and then `rehash` or `hash -r`");
}

fn first_path_match(binary: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&paths) {
        let candidate = entry.join(binary_name(binary));
        if candidate.exists() && candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn path_starts_with(candidate: &Path, root: &Path) -> bool {
    if let (Ok(candidate), Ok(root)) = (candidate.canonicalize(), root.canonicalize()) {
        return candidate.starts_with(root);
    }
    candidate.starts_with(root)
}

fn provider_query(filter: VersionFilter) -> avm_plugin_api::ToolVersionQuery {
    match filter {
        VersionFilter::Recent => avm_plugin_api::ToolVersionQuery::Recent,
        VersionFilter::Latest => avm_plugin_api::ToolVersionQuery::Latest,
        VersionFilter::Major(major) => avm_plugin_api::ToolVersionQuery::Major(major),
    }
}
