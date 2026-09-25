use super::*;

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
            print_available_versions(provider_name, provider.as_ref(), ToolVersionQuery::Recent)?;
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
        _ => plugin_passthrough(provider_name, parts, cfg),
    }
}

/// Anything that isn't one of the fixed protocol verbs above (list/versions/
/// use/install/uninstall) gets forwarded straight to the plugin's own
/// executable as raw argv, stdio inherited — this is what lets a plugin add
/// bespoke subcommands (e.g. `avm android avd list`) without avm-cli's core
/// ever needing to know they exist. Only protocol (marketplace) plugins have
/// a real executable to forward to; asdf-compat providers fall through to
/// the same "unknown command" error as before.
///
/// The resolved (pinned) version is passed via `AVM_RESOLVED_VERSION` so a
/// plugin's custom subcommands can act on "whichever version avm would use
/// here" without needing avm's resolution logic (local pin walking up
/// from cwd, then global) duplicated inside every plugin.
fn plugin_passthrough(provider_name: &str, parts: &[String], cfg: &ResolvedConfig) -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    if let Some(process) = plugin_manager.protocol_provider(provider_name) {
        let mut cmd = std::process::Command::new(process.executable());
        cmd.args(parts);
        if let Some((version, _)) = cfg.resolve_tool(provider_name) {
            cmd.env("AVM_RESOLVED_VERSION", version);
        }
        let status = cmd
            .status()
            .with_context(|| format!("failed to run `avm {provider_name} {}`", parts.join(" ")))?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }
    Err(anyhow!(
        "unknown {provider_name} command. Run `avm {provider_name}` for an interactive menu, or `avm {provider_name} --help`"
    ))
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

    let labels: Vec<String> = actions.iter().map(|(label, _)| label.to_string()).collect();
    let Some(idx) = ui::select(&format!("avm {provider_name} — what next?"), &labels)? else {
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
    match ui::select(&format!("Uninstall which {provider_name} version?"), &installed)? {
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

pub fn cmd_plugin_command(args: Vec<String>) -> Result<()> {
    let cfg = load_state()?;
    cmd_provider_tool(args, &cfg)
}

fn parse_version_filter(value: &str) -> Result<ToolVersionQuery> {
    if value == "latest" {
        return Ok(ToolVersionQuery::Latest);
    }

    let major = value
        .trim_start_matches('v')
        .parse::<u64>()
        .with_context(|| format!("unknown version filter: {value}"))?;
    Ok(ToolVersionQuery::Major(major))
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
    shims::reshim()?;
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
            .available_versions(ToolVersionQuery::Latest)
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
            .available_versions(ToolVersionQuery::Major(major))
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
    Ok(config::load(&config_root(true)?)?.tools.get(tool).cloned())
}

fn set_tool_version(tool: &str, version: &str, global: bool) -> Result<()> {
    // Auto-create the config on either scope: a user pinning a tool here
    // clearly wants it pinned without an `avm init` step first.
    edit_config(global, true, |cfg| {
        cfg.tools.insert(tool.to_string(), version.to_string());
        Ok(())
    })?;
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
    shims::reshim()?;
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
    let Some(first_match) = shims::which(tool, false) else {
        return;
    };
    // Shims are preferred unless the first PATH hit is also the first non-shim hit.
    if shims::which(tool, true).as_ref() != Some(&first_match) {
        return;
    }

    eprintln!(
        "warning: plain `{tool}` currently resolves to {} before AVM shims",
        first_match.display()
    );
    eprintln!("warning: run `eval \"$(avm shell-init)\"` and then `rehash` or `hash -r`");
}

/// Every provider — first-party or third-party — speaks the same plugin
/// protocol and is discovered the same way, nothing compiled into `avm-bin`:
///   1. installed via the marketplace (`avm plugin add <name>` fetched a
///      compiled release into `~/.avm/plugins/avm-plugin-<name>`)
///   2. the legacy asdf-compatible adapter, kept for community asdf plugins
///      that haven't adopted the native protocol
///
/// Nothing is available out of the box — `avm plugin add node` (etc.) is
/// required, same as `brew install` or a Claude Code marketplace install.
pub fn provider_by_name(name: &str) -> Result<Box<dyn ToolProvider>> {
    let plugin_manager = PluginManager::new()?;
    if let Some(provider) = plugin_manager.protocol_provider(name) {
        return Ok(Box::new(provider));
    }
    if let Some(provider) = plugin_manager.asdf_provider(name) {
        return Ok(Box::new(provider));
    }

    if let Ok(Some(entry)) = runtime::marketplace_lookup(name) {
        return Err(anyhow!(
            "'{name}' is available in the marketplace but not installed — run `avm plugin add {name}` ({})",
            entry.description
        ));
    }
    Err(anyhow!("unknown plugin '{name}'"))
}

fn print_provider_status(
    provider_name: &str,
    provider: &dyn ToolProvider,
    cfg: &ResolvedConfig,
) -> Result<()> {
    println!("Plugin: {provider_name}");
    if let Some((version, source)) = cfg.resolve_tool(provider_name) {
        println!("Selected version: {version} ({})", alias_source_label(&source));
    } else {
        println!("Selected version: none");
    }
    print_installed_versions(provider)?;
    println!();
    println!("Commands:");
    println!("  avm {provider_name} versions");
    println!("  avm {provider_name} use <version>");
    println!("  avm {provider_name} install <version>");
    println!("  avm {provider_name} uninstall <version>");
    Ok(())
}

fn print_installed_versions(provider: &dyn ToolProvider) -> Result<()> {
    let installed = provider.installed_versions()?;
    if installed.is_empty() {
        println!("Installed {} versions: none", provider.name());
    } else {
        println!(
            "Installed {} versions: {}",
            provider.name(),
            installed.join(", ")
        );
    }
    Ok(())
}

fn print_available_versions(
    provider_name: &str,
    provider: &dyn ToolProvider,
    query: ToolVersionQuery,
) -> Result<()> {
    let versions = provider.available_versions(query)?;
    if versions.is_empty() {
        println!("Available {provider_name} versions: none");
        return Ok(());
    }

    if ui::can_select() {
        return select_tool_version(provider_name, provider, versions);
    }

    println!("Available {provider_name} versions:");
    for version in &versions {
        println!("  {}", version.label);
    }

    println!();
    println!("Use:");
    println!("  avm {provider_name} install <version>");
    println!("  avm {provider_name} use <version>");
    Ok(())
}

fn select_tool_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    versions: Vec<avm_plugin_api::ToolVersion>,
) -> Result<()> {
    let labels: Vec<String> = versions.iter().map(|v| v.label.clone()).collect();
    match ui::select(&format!("Available {provider_name} versions"), &labels)? {
        Some(selected) => {
            confirm_tool_version_selection(provider_name, provider, &versions[selected].version)
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

fn confirm_tool_version_selection(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let has_local_config = cwd.join(CONFIG_FILE).exists();

    if has_local_config {
        print!("Use {provider_name} {version} locally or globally? [l/g/c]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "l" | "local" => use_provider_version(provider_name, provider, version, false),
            "g" | "global" => use_provider_version(provider_name, provider, version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    } else {
        print!("No local .avm.json found. Set {provider_name} {version} globally? [y/N]: ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => use_provider_version(provider_name, provider, version, true),
            _ => {
                println!("Cancelled.");
                Ok(())
            }
        }
    }
}
