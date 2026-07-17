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
        [] => interactive_provider_menu(provider_name, provider.as_ref(), cfg),
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
            let resolved = resolve_version_spec(provider.as_ref(), version)?;
            install_and_pin(provider_name, provider.as_ref(), &resolved, PinScope::Auto)
        }
        [cmd, version, flag] if cmd == "install" && (flag == "--global" || flag == "-g") => {
            let resolved = resolve_version_spec(provider.as_ref(), version)?;
            install_and_pin(provider_name, provider.as_ref(), &resolved, PinScope::GlobalOnly)
        }
        [cmd, version, flag] if cmd == "install" && flag == "--no-pin" => {
            let resolved = resolve_version_spec(provider.as_ref(), version)?;
            install_and_pin(provider_name, provider.as_ref(), &resolved, PinScope::None)
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
            "unknown {provider_name} command. Run `avm {provider_name}` for an interactive menu, or `avm {provider_name} --help`"
        )),
    }
}

/// Bare `avm <plugin>` with no subcommand. In a TTY, show the next actions as a
/// picker and chain into the chosen one by re-dispatching arg strings through
/// the same parser — so this works identically for every plugin. Outside a TTY
/// it falls back to the classic status printout.
fn interactive_provider_menu(
    provider_name: &str,
    provider: &dyn ToolProvider,
    cfg: &ResolvedConfig,
) -> Result<()> {
    if !ui::can_select() {
        return print_provider_status(provider_name, provider, cfg);
    }

    // (label, subcommand to re-dispatch). "__uninstall" needs a version picker
    // first, so it is handled inline rather than re-dispatched.
    let actions: [(&str, &[&str]); 5] = [
        ("Show selected & installed versions", &["list"]),
        ("Browse & install a version", &["versions"]),
        ("Install the latest version", &["install", "latest"]),
        ("Uninstall an installed version", &["__uninstall"]),
        ("Show all commands", &["help"]),
    ];

    let items: Vec<ui::SelectItem> = actions
        .iter()
        .map(|(label, _)| ui::SelectItem {
            label: (*label).to_string(),
        })
        .collect();

    let title = format!("avm {provider_name} — what next?");
    let help = "Up/Down to move, Enter to select, q to cancel.";
    let Some(idx) = ui::select(&title, help, &items, 10)? else {
        println!("Cancelled.");
        return Ok(());
    };

    let sub = actions[idx].1;
    if sub.first() == Some(&"__uninstall") {
        return interactive_uninstall(provider_name, provider);
    }

    let mut args = vec![provider_name.to_string()];
    args.extend(sub.iter().map(|s| s.to_string()));
    cmd_provider_tool(args, cfg)
}

fn interactive_uninstall(provider_name: &str, provider: &dyn ToolProvider) -> Result<()> {
    let installed = provider.installed_versions()?;
    if installed.is_empty() {
        println!("No installed {provider_name} versions to remove.");
        return Ok(());
    }
    let items: Vec<ui::SelectItem> = installed
        .iter()
        .map(|v| ui::SelectItem { label: v.clone() })
        .collect();
    let help = "Up/Down to move, Enter to select, q to cancel.";
    match ui::select(&format!("Uninstall which {provider_name} version?"), help, &items, 10)? {
        Some(i) => {
            provider.uninstall(&installed[i])?;
            println!("✓ Removed {provider_name} {}", installed[i]);
            Ok(())
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
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

#[derive(Clone, Copy)]
enum PinScope {
    /// Pin locally; also globally if no global pin exists for this tool yet.
    Auto,
    /// Pin globally only.
    GlobalOnly,
    /// Do not pin.
    None,
}

fn install_and_pin(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
    scope: PinScope,
) -> Result<()> {
    if !provider.is_installed(version) {
        provider.install(version)?;
    }
    reshim()?;
    match scope {
        PinScope::None => {
            println!("✓ Installed {provider_name} {version}");
        }
        PinScope::GlobalOnly => {
            set_tool_version(provider_name, version, true)?;
            println!("✓ Installed {provider_name} {version} (global pin set)");
        }
        PinScope::Auto => {
            set_tool_version(provider_name, version, false)?;
            if global_pin(provider_name)?.is_none() {
                set_tool_version(provider_name, version, true)?;
                println!(
                    "✓ Installed {provider_name} {version} (local + global pin set)"
                );
            } else {
                println!("✓ Installed {provider_name} {version} (local pin set)");
            }
        }
    }
    warn_if_shim_is_not_preferred(provider_name);
    Ok(())
}

/// Translate a user-supplied version spec into a concrete version string.
/// Supports `latest`, `<major>` (e.g. `20`), or a literal version like `20.11.0`.
fn resolve_version_spec(provider: &dyn ToolProvider, spec: &str) -> Result<String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("version required"));
    }
    if trimmed == "latest" {
        let versions = provider
            .available_versions(avm_plugin_api::ToolVersionQuery::Latest)
            .context("failed to fetch latest version")?;
        let pick = versions
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no remote versions available"))?;
        println!("Resolved latest → {}", pick.version);
        return Ok(pick.version);
    }
    // Bare major like "20"
    if let Ok(major) = trimmed.trim_start_matches('v').parse::<u64>() {
        let versions = provider
            .available_versions(avm_plugin_api::ToolVersionQuery::Major(major))
            .context("failed to fetch versions for major")?;
        if let Some(pick) = versions.into_iter().next() {
            println!("Resolved {trimmed} → {}", pick.version);
            return Ok(pick.version);
        }
        // Fall through with the literal value so the provider can complain
        // with its own error.
    }
    Ok(trimmed.to_string())
}

fn global_pin(tool: &str) -> Result<Option<String>> {
    let root = home_dir()?;
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let parsed = load_config_for_root(&root)?;
    Ok(parsed.tools.get(tool).cloned())
}

fn set_tool_version(tool: &str, version: &str, global: bool) -> Result<()> {
    let root = if global {
        home_dir()?
    } else {
        std::env::current_dir().context("failed to read current directory")?
    };
    // Auto-create the config on either scope. We used to bail with
    // "run `avm init` first" for the local case, but that just added a step
    // for users who clearly want the tool pinned in this directory.
    let path = root.join(CONFIG_FILE);
    if !path.exists() {
        avm_core::write_default_config(root.as_path(), CONFIG_FILE)?;
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
    reshim()?;
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
    println!("  install <version|latest|N>   Install + auto-pin (local; global if unpinned)");
    println!("  install <version> --global   Install + pin globally only");
    println!("  install <version> --no-pin   Install without pinning");
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
