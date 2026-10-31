//! The interactive pickers behind bare `avm <plugin>`, uninstall and version selection.
use crate::common::state::load_state;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use crate::global::{cmd_provider_tool, print_provider_status, use_provider_version};
use crate::resolver::ResolvedConfig;
use anyhow::Result;
use avm_plugin_sdk::ToolProvider;
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
    let actions: [&[&str]; 5] = [
        &["list"],
        &["versions"],
        &["install", "latest"],
        &["__uninstall"],
        &["help"],
    ];

    let labels: Vec<String> = msg::PROVIDER_MENU_ACTIONS.map(String::from).to_vec();
    let title = fill(msg::PROVIDER_MENU_TITLE, &[("name", provider_name)]);
    let Some(idx) = ui::select(&title, &labels)? else {
        ui::cancelled();
        return Ok(());
    };

    let sub = actions[idx];
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
        ui::info(fill(
            msg::PROVIDER_UNINSTALL_NONE,
            &[("name", provider_name)],
        ));
        return Ok(());
    }
    let title = fill(msg::PROVIDER_UNINSTALL_TITLE, &[("name", provider_name)]);
    match ui::select(&title, &installed)? {
        Some(i) => {
            provider.uninstall(&installed[i])?;
            ui::success(fill(
                msg::PROVIDER_UNINSTALLED,
                &[("name", provider_name), ("version", &installed[i])],
            ));
            Ok(())
        }
        None => {
            ui::cancelled();
            Ok(())
        }
    }
}

/// Shows a picker of available versions (installed ones marked) and asks where to apply the chosen one.
pub fn select_tool_version(
    provider_name: &str,
    provider: &dyn ToolProvider,
    versions: Vec<avm_plugin_sdk::ToolVersion>,
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
    let title = fill(msg::PROVIDER_VERSIONS_TITLE, &[("name", provider_name)]);
    match ui::select(&title, &labels)? {
        Some(selected) => {
            confirm_tool_version_selection(provider_name, provider, &versions[selected].version)
        }
        None => {
            ui::cancelled();
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
    let scopes = [msg::PROVIDER_SCOPE_LOCAL, msg::PROVIDER_SCOPE_GLOBAL].map(String::from);
    let title = fill(
        msg::PROVIDER_SCOPE_TITLE,
        &[("name", provider_name), ("version", version)],
    );
    match ui::select(&title, &scopes)? {
        Some(choice) => use_provider_version(provider_name, provider, version, choice == 1),
        None => {
            ui::cancelled();
            Ok(())
        }
    }
}
