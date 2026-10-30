use crate::cli::commands::provider_menu::{interactive_provider_menu, select_tool_version};
use crate::cli::commands::{
    alias_source_label, config_root, edit_config, install_provider_version, installed_cell,
};
use crate::cli::state::load_state;
use crate::cli::ui;
use crate::plugins::PluginManager;
use crate::resolver::ResolvedConfig;
use crate::{config, shims};
use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{ToolProvider, ToolVersionQuery};

/// Handles `avm <plugin> [command]`: with no command opens the interactive menu, otherwise runs list, versions, use, install, uninstall and similar subcommands.
/// Errors when the plugin is missing or arguments are invalid.
pub fn cmd_provider_tool(args: Vec<String>, cfg: &ResolvedConfig) -> Result<()> {
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
            let resolved = resolve_install_version(provider.as_ref(), version)?;
            install_and_pin(provider_name, provider.as_ref(), &resolved, PinScope::Auto)
        }
        [cmd, version, flag] if cmd == "install" && (flag == "--global" || flag == "-g") => {
            let resolved = resolve_install_version(provider.as_ref(), version)?;
            install_and_pin(
                provider_name,
                provider.as_ref(),
                &resolved,
                PinScope::GlobalOnly,
            )
        }
        [cmd, version, flag] if cmd == "install" && flag == "--no-pin" => {
            let resolved = resolve_install_version(provider.as_ref(), version)?;
            install_and_pin(provider_name, provider.as_ref(), &resolved, PinScope::None)
        }
        [cmd, version] if cmd == "uninstall" => {
            provider.uninstall(version)?;
            done!("Removed {provider_name} {version}");
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

/// Entry point for plugin commands: loads the current state and forwards to `cmd_provider_tool`.
pub fn cmd_plugin_command(args: Vec<String>) -> Result<()> {
    let cfg = load_state()?;
    cmd_provider_tool(args, &cfg)
}

/// Parses a version filter: `latest` or a major number, with an optional leading `v`.
/// Errors on anything else.
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

/// Installs `version` if missing and pins it according to `scope`, refreshing shims around the change.
fn install_and_pin(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
    scope: PinScope,
) -> Result<()> {
    if !provider.is_installed(version) {
        install_provider_version(provider, version)?;
    }
    with_fresh_shims(provider_name, || {
        match scope {
            PinScope::None => {
                done!("Installed {provider_name} {version}");
            }
            PinScope::GlobalOnly => {
                set_tool_version(provider_name, version, true)?;
                done!("Installed {provider_name} {version} (global pin set)");
            }
            PinScope::Auto => {
                set_tool_version(provider_name, version, false)?;
                if global_pin(provider_name)?.is_none() {
                    set_tool_version(provider_name, version, true)?;
                    done!("Installed {provider_name} {version} (local + global pin set)");
                } else {
                    done!("Installed {provider_name} {version} (local pin set)");
                }
            }
        }
        Ok(())
    })
}

/// Rebuild the shims, run `pin`, then warn if a shim is not first on PATH.
fn with_fresh_shims(tool: &str, pin: impl FnOnce() -> Result<()>) -> Result<()> {
    shims::reshim()?;
    pin()?;
    warn_if_shim_is_not_preferred(tool);
    Ok(())
}

/// Resolves a version spec to a concrete version, preferring the matching `.avm.lock` entry over a fresh lookup.
fn resolve_install_version(provider: &dyn ToolProvider, spec: &str) -> Result<String> {
    let spec = spec.trim();
    match crate::lockfile::entry_for(provider.name(), |entry| entry.requested == spec)? {
        Some(entry) => Ok(entry.resolved),
        None => crate::lockfile::resolve_spec(provider, spec),
    }
}

/// The version of `tool` pinned in the global config, if any.
fn global_pin(tool: &str) -> Result<Option<String>> {
    Ok(config::load(&config_root(true)?)?.tools.get(tool).cloned())
}

/// Pins `tool` to `version` in the local or global config, creating the config if needed.
/// A local pin also refreshes the lockfile.
pub fn set_tool_version(tool: &str, version: &str, global: bool) -> Result<()> {
    // Auto-create the config on either scope: a user pinning a tool here
    // clearly wants it pinned without an `avm init` step first.
    edit_config(global, true, |cfg| {
        cfg.tools.insert(tool.to_string(), version.to_string());
        Ok(())
    })?;
    if global {
        done!("Set global {tool} version to {version}");
    } else {
        done!("Set local {tool} version to {version}");
        super::install::refresh_lock();
    }
    Ok(())
}

/// Resolves `version`, installs it if missing, and pins it locally or globally, refreshing shims.
pub fn use_provider_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
    global: bool,
) -> Result<()> {
    let version = resolve_install_version(provider, version)?;
    ensure_provider_version_installed(provider_name, provider, &version)?;
    with_fresh_shims(provider_name, || {
        set_tool_version(provider_name, &version, global)
    })
}

/// Installs `version` of the provider unless it is already installed.
pub fn ensure_provider_version_installed(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
) -> Result<()> {
    if provider.is_installed(version) {
        return Ok(());
    }

    println!("Installing {provider_name} {version}...");
    install_provider_version(provider, version)?;
    done!("Installed {provider_name} {version}");
    Ok(())
}

/// Prints usage for `avm <plugin>` subcommands.
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

/// Warns when a non-shim binary for `tool` comes first on PATH and would bypass AVM shims.
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
    warn!("run `eval \"$(avm shell-init)\"` and then `rehash` or `hash -r`");
}

/// Looks up an installed plugin by name; errors with the install hint when it is not found.
pub fn provider_by_name(name: &str) -> Result<Box<dyn ToolProvider>> {
    PluginManager::new()?.provider(name).ok_or_else(|| {
        anyhow!(crate::constants::msg::fill(
            crate::constants::msg::PLUGIN_NOT_INSTALLED,
            &[("name", name)]
        ))
    })
}

/// Prints the plugin's selected version and its installed versions.
/// Errors if the plugin cannot list its versions.
pub fn print_provider_status(
    provider_name: &str,
    provider: &dyn ToolProvider,
    cfg: &ResolvedConfig,
) -> Result<()> {
    println!("Plugin: {provider_name}");
    if let Some((version, source)) = cfg.resolve_tool(provider_name) {
        println!(
            "Selected version: {version} ({})",
            alias_source_label(&source)
        );
    } else {
        println!("Selected version: none");
    }
    print_installed_versions(provider, cfg.resolve_tool(provider_name).map(|(v, _)| v))?;
    println!();
    println!("Commands:");
    println!("  avm {provider_name} versions");
    println!("  avm {provider_name} use <version>");
    println!("  avm {provider_name} install <version>");
    println!("  avm {provider_name} uninstall <version>");
    Ok(())
}

/// Prints installed versions in version order, marking the `active` one.
fn print_installed_versions(provider: &dyn ToolProvider, active: Option<String>) -> Result<()> {
    let mut installed = provider.installed_versions()?;
    crate::version_files::sort_versions(&mut installed);
    let active = active.as_deref();
    if installed.is_empty() && active.is_none() {
        println!("Installed {} versions: none", provider.name());
    } else {
        println!(
            "Installed {} versions: {}",
            provider.name(),
            installed_cell(&installed, active).replace("  ", ", ")
        );
    }
    Ok(())
}

/// Lists versions matching `query`: an interactive picker on a terminal, plain output otherwise.
/// Errors if the plugin cannot list versions.
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
