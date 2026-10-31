use crate::common::config_utils::{alias_source_label, config_root, edit_config};
use crate::common::state::load_state;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use crate::global::{interactive_provider_menu, select_tool_version};
use crate::plugins::installed_cell;
use crate::plugins::PluginRegistry;
use crate::project::install_provider_version;
use crate::resolver::ResolvedConfig;
use crate::{config, shims};
use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::{ToolProvider, ToolVersionQuery};

/// Handles `avm <plugin> [command]`: with no command opens the interactive menu, otherwise runs list, versions, use, install, uninstall and similar subcommands.
/// Errors when the plugin is missing or arguments are invalid.
pub fn cmd_provider_tool(args: Vec<String>, cfg: &ResolvedConfig) -> Result<()> {
    let (provider_name, parts) = args
        .split_first()
        .ok_or_else(|| anyhow!(msg::PROVIDER_NAME_REQUIRED))?;
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
            ui::success(fill(
                msg::PROVIDER_UNINSTALLED,
                &[("name", provider_name), ("version", version)],
            ));
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
    let plugin_manager = PluginRegistry::new()?;
    if let Some(process) = plugin_manager.protocol_provider(provider_name) {
        let mut cmd = std::process::Command::new(process.executable());
        cmd.args(parts);
        if let Some((version, _)) = cfg.resolve_tool(provider_name) {
            cmd.env("AVM_RESOLVED_VERSION", version);
        }
        let status = cmd.status().with_context(|| {
            fill(
                msg::PROVIDER_PASSTHROUGH_FAILED,
                &[("name", provider_name), ("args", &parts.join(" "))],
            )
        })?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }
    Err(anyhow!(fill(
        msg::PROVIDER_UNKNOWN_COMMAND,
        &[("name", provider_name)]
    )))
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
        .with_context(|| fill(msg::PROVIDER_FILTER_UNKNOWN, &[("value", value)]))?;
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
    let vars = [("name", provider_name), ("version", version)];
    with_fresh_shims(provider_name, || {
        let done = match scope {
            PinScope::None => msg::PROVIDER_INSTALLED,
            PinScope::GlobalOnly => {
                set_tool_version(provider_name, version, true)?;
                msg::PROVIDER_INSTALLED_GLOBAL
            }
            PinScope::Auto => {
                set_tool_version(provider_name, version, false)?;
                if global_pin(provider_name)?.is_none() {
                    set_tool_version(provider_name, version, true)?;
                    msg::PROVIDER_INSTALLED_BOTH
                } else {
                    msg::PROVIDER_INSTALLED_LOCAL
                }
            }
        };
        ui::success(fill(done, &vars));
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
    let template = if global {
        msg::PROVIDER_SET_GLOBAL
    } else {
        msg::PROVIDER_SET_LOCAL
    };
    ui::success(fill(template, &[("tool", tool), ("version", version)]));
    if !global {
        crate::project::refresh_lock();
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

    let vars = [("name", provider_name), ("version", version)];
    ui::info(fill(msg::PROVIDER_INSTALLING, &vars));
    install_provider_version(provider, version)?;
    ui::success(fill(msg::PROVIDER_INSTALLED, &vars));
    Ok(())
}

/// Prints usage for `avm <plugin>` subcommands.
fn print_provider_help(provider_name: &str) {
    ui::info(fill(msg::PROVIDER_HELP, &[("name", provider_name)]));
}

/// Fills `{name}` in each line template.
fn fill_lines(lines: &[&str], name: &str) -> Vec<String> {
    lines
        .iter()
        .map(|line| fill(line, &[("name", name)]))
        .collect()
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

    ui::notice(fill(
        msg::PROVIDER_SHIM_SHADOWED,
        &[("tool", tool), ("path", &first_match.display().to_string())],
    ));
    ui::warn(msg::PROVIDER_SHIM_HINT);
}

/// Looks up an installed plugin by name; errors with the install hint when it is not found.
pub fn provider_by_name(name: &str) -> Result<Box<dyn ToolProvider>> {
    PluginRegistry::new()?
        .provider(name)
        .ok_or_else(|| anyhow!(fill(msg::PLUGIN_NOT_INSTALLED, &[("name", name)])))
}

/// Prints the plugin's selected version and its installed versions.
/// Errors if the plugin cannot list its versions.
pub fn print_provider_status(
    provider_name: &str,
    provider: &dyn ToolProvider,
    cfg: &ResolvedConfig,
) -> Result<()> {
    ui::info(fill(
        msg::PROVIDER_STATUS_PLUGIN,
        &[("name", provider_name)],
    ));
    if let Some((version, source)) = cfg.resolve_tool(provider_name) {
        ui::info(fill(
            msg::PROVIDER_STATUS_SELECTED,
            &[
                ("version", &version),
                ("source", alias_source_label(&source)),
            ],
        ));
    } else {
        ui::info(msg::PROVIDER_STATUS_SELECTED_NONE);
    }
    print_installed_versions(provider, cfg.resolve_tool(provider_name).map(|(v, _)| v))?;
    ui::blank();
    ui::section(
        msg::PROVIDER_COMMANDS_HEADING,
        &fill_lines(&msg::PROVIDER_STATUS_COMMANDS, provider_name),
    );
    Ok(())
}

/// Prints installed versions in version order, marking the `active` one.
fn print_installed_versions(provider: &dyn ToolProvider, active: Option<String>) -> Result<()> {
    let mut installed = provider.installed_versions()?;
    crate::version_files::sort_versions(&mut installed);
    let active = active.as_deref();
    if installed.is_empty() && active.is_none() {
        ui::info(fill(
            msg::PROVIDER_INSTALLED_NONE,
            &[("name", provider.name())],
        ));
    } else {
        ui::info(fill(
            msg::PROVIDER_INSTALLED_LIST,
            &[
                ("name", provider.name()),
                (
                    "versions",
                    &installed_cell(&installed, active).replace("  ", ", "),
                ),
            ],
        ));
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
        ui::info(fill(
            msg::PROVIDER_AVAILABLE_NONE,
            &[("name", provider_name)],
        ));
        return Ok(());
    }

    if ui::can_select() {
        return select_tool_version(provider_name, provider, versions);
    }

    ui::info(fill(
        msg::PROVIDER_AVAILABLE_HEADING,
        &[("name", provider_name)],
    ));
    for version in &versions {
        ui::info(format!("  {}", version.label));
    }

    ui::blank();
    ui::section(
        msg::PROVIDER_USE_HEADING,
        &fill_lines(&msg::PROVIDER_USE_COMMANDS, provider_name),
    );
    Ok(())
}
