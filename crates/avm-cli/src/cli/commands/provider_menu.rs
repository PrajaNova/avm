//! The interactive pickers behind bare `avm <plugin>`, uninstall and version selection.
use crate::cli::commands::provider::{
    cmd_provider_tool, print_provider_status, use_provider_version,
};
use crate::cli::state::load_state;
use crate::cli::ui;
use crate::resolver::ResolvedConfig;
use anyhow::Result;
use avm_plugin_api::ToolProvider;
use std::collections::HashSet;

/// Bare `avm <plugin>` with no subcommand. In a TTY, show the next actions as a
/// picker and chain into the chosen one by re-dispatching arg strings through
/// the same parser — so this works identically for every plugin. Outside a TTY
/// it falls back to the classic status printout.
pub fn interactive_provider_menu(
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

/// Lets the user pick an installed version to remove, then uninstalls it; does nothing when none are installed or the picker is cancelled.
pub fn interactive_uninstall(provider_name: &str, provider: &dyn ToolProvider) -> Result<()> {
    let installed = provider.installed_versions()?;
    if installed.is_empty() {
        println!("No installed {provider_name} versions to remove.");
        return Ok(());
    }
    match ui::select(
        &format!("Uninstall which {provider_name} version?"),
        &installed,
    )? {
        Some(i) => {
            provider.uninstall(&installed[i])?;
            done!("Removed {provider_name} {}", installed[i]);
            Ok(())
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

/// Shows a picker of available versions (installed ones marked) and asks where to apply the chosen one.
pub fn select_tool_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    versions: Vec<avm_plugin_api::ToolVersion>,
) -> Result<()> {
    // One listing for the ✓ marks; asking `is_installed` per row would spawn a plugin per version.
    let installed: HashSet<String> = provider
        .installed_versions()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let selected_version = load_state()
        .ok()
        .and_then(|cfg| cfg.resolve_tool(provider_name))
        .map(|(version, _)| version);
    let labels: Vec<String> = versions
        .iter()
        .map(|v| {
            let mark = if selected_version.as_deref() == Some(v.version.as_str()) {
                "●"
            } else if installed.contains(&v.version) {
                "✓"
            } else {
                " "
            };
            format!("{mark} {}", v.label)
        })
        .collect();
    match ui::select(
        &format!("Available {provider_name} versions (✓ installed, ● selected)"),
        &labels,
    )? {
        Some(selected) => {
            confirm_tool_version_selection(provider_name, provider, &versions[selected].version)
        }
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}

/// Asks whether to use `version` locally or globally and applies it; does nothing when cancelled.
pub fn confirm_tool_version_selection(
    provider_name: &str,
    provider: &dyn ToolProvider,
    version: &str,
) -> Result<()> {
    // Local first: it creates `.avm.json` when the project has none, so a project
    // that only has `.nvmrc` can be pinned from here without `avm init`.
    let scopes = [
        "Local: this project (writes .avm.json)".to_string(),
        "Global: every project".to_string(),
    ];
    match ui::select(&format!("Use {provider_name} {version} where?"), &scopes)? {
        Some(choice) => use_provider_version(provider_name, provider, version, choice == 1),
        None => {
            println!("Cancelled.");
            Ok(())
        }
    }
}
